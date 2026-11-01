//! The walk over the element tree (`specs/0024-svg-import-export` criteria 13, 31 to 35, 42, 44
//! to 49): containers become groups or layers, drawn elements leaves, every transform is composed
//! into the leaf's matrix, and everything the subset does not read is ignored without a trace.

use std::collections::BTreeMap;

use kurbo::Affine;
use roxmltree::{Document, Node, NodeId};

use crate::drawing::{InkscapeObject, LeafKind, NodeState, SvgLeaf, SvgNode};
use crate::page::{INKSCAPE_NS, SODIPODI_NS};
use crate::paint::Ids;
use crate::shapes::{ShapeError, attr, leaf_kind};
use crate::style::{Cascade, Computed};
use crate::units::{Axis, Units, aspect, view_box, view_box_map};

/// The SVG namespace.
pub(crate) const SVG_NS: &str = "http://www.w3.org/2000/svg";

/// Where an element is read: its parent's map, style and units.
#[derive(Debug, Clone)]
pub(crate) struct Ctx {
    /// The parent's user units to millimetres.
    pub(crate) matrix: Affine,
    /// The parent's computed style.
    pub(crate) style: Computed,
    /// The parent's units.
    pub(crate) units: Units,
    /// How many `use` copies this element is inside; 0 is its own place in the file.
    pub(crate) copy_depth: usize,
    /// The `use` elements being copied around it (a reference cycle is skipped).
    pub(crate) expanding: Vec<NodeId>,
    /// Whether the parent is the root `<svg>` (a layer may stand here).
    pub(crate) at_root: bool,
}

/// The state of one reading.
pub(crate) struct Reader<'a, 'd, 'input> {
    pub(crate) doc: &'d Document<'input>,
    pub(crate) cascade: Cascade<'a, 'd, 'input>,
    pub(crate) ids: &'a Ids<'d, 'input>,
    /// Whether elements without a namespace count as SVG (a root without `xmlns`).
    pub(crate) plain_names: bool,
    /// The path points left.
    pub(crate) points: usize,
    /// The `use` copies left.
    pub(crate) copies: usize,
    /// A limit stopped the reading.
    pub(crate) stopped: bool,
    /// The leaves made so far.
    pub(crate) leaves: usize,
    /// The leaf index of each element read at its own place.
    pub(crate) leaf_of: BTreeMap<u32, usize>,
    /// Clone leaves and the element their original is: resolved after the walk.
    pub(crate) pending: Vec<(usize, u32)>,
    /// The clone decisions of `use` elements at their own place (`None`: a copy).
    pub(crate) clone_depths: BTreeMap<u32, Option<usize>>,
    /// Layers made so far, for "Layer N".
    layers: usize,
    /// The text characters left in the file (criterion 98).
    pub(crate) text_chars: usize,
    /// The texts made so far.
    pub(crate) texts: usize,
    /// Text paths whose carrier is a drawn element: (text number, path slot, element).
    pub(crate) carriers: Vec<(usize, usize, u32)>,
    /// Nodes to place before the next node read (a text's new guide carrier, criterion 94).
    pub(crate) before: Vec<SvgNode>,
    /// Whether the root declares the `curvyo:` profile v1 (criterion 57).
    pub(crate) profile: bool,
}

impl<'a, 'd, 'input> Reader<'a, 'd, 'input> {
    /// A reader of `doc` with the point and copy budgets.
    pub(crate) fn new(
        doc: &'d Document<'input>,
        cascade: Cascade<'a, 'd, 'input>,
        ids: &'a Ids<'d, 'input>,
        plain_names: bool,
    ) -> Self {
        Self {
            doc,
            cascade,
            ids,
            plain_names,
            points: crate::limits::MAX_PATH_POINTS,
            copies: crate::limits::MAX_USE_COPIES,
            stopped: false,
            leaves: 0,
            leaf_of: BTreeMap::new(),
            pending: Vec::new(),
            clone_depths: BTreeMap::new(),
            layers: 0,
            text_chars: crate::limits::MAX_TEXT_CHARS,
            texts: 0,
            carriers: Vec::new(),
            before: Vec::new(),
            profile: false,
        }
    }

    /// Whether `node` is an SVG element named `name`.
    pub(crate) fn is(&self, node: Node<'_, '_>, name: &str) -> bool {
        node.is_element() && node.tag_name().name() == name && self.is_svg(node)
    }

    /// Whether `node` is an element of the SVG namespace.
    pub(crate) fn is_svg(&self, node: Node<'_, '_>) -> bool {
        node.is_element()
            && match node.tag_name().namespace() {
                Some(ns) => ns == SVG_NS,
                None => self.plain_names,
            }
    }

    /// The children of the container `node`, read under `ctx`.
    pub(crate) fn children(&mut self, node: Node<'_, '_>, ctx: &Ctx) -> Vec<SvgNode> {
        let mut out = Vec::new();
        for child in node.children() {
            if self.stopped {
                break;
            }
            if let Some(read) = self.element(child, ctx) {
                out.append(&mut self.before);
                out.push(read);
            }
        }
        out
    }

    /// One element under `ctx`, or `None` when it is not read.
    pub(crate) fn element(&mut self, node: Node<'_, '_>, ctx: &Ctx) -> Option<SvgNode> {
        if !self.is_svg(node) {
            return None;
        }
        let name = node.tag_name().name();
        let known = matches!(
            name,
            "g" | "a"
                | "switch"
                | "svg"
                | "use"
                | "path"
                | "rect"
                | "circle"
                | "ellipse"
                | "line"
                | "polyline"
                | "polygon"
                | "text"
                | "flowRoot"
                | "image"
        );
        if !known {
            return None;
        }
        let style = self.cascade.computed(node, &ctx.style, &ctx.units);
        let own = attr(node, "transform")
            .and_then(|t| t.parse::<svgtypes::Transform>().ok())
            .map_or(Affine::IDENTITY, |t| {
                Affine::new([t.a, t.b, t.c, t.d, t.e, t.f])
            });
        let matrix = ctx.matrix * own;
        let det = matrix.determinant();
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        let inner = Ctx {
            matrix,
            style,
            units: ctx.units,
            copy_depth: ctx.copy_depth,
            expanding: ctx.expanding.clone(),
            at_root: false,
        };
        match name {
            "g" | "a" => self.group(node, &inner, ctx.at_root && name == "g"),
            "switch" => {
                let first = node
                    .children()
                    .find(|child| child.is_element() && self.passes_switch(*child))?;
                self.element(first, &inner)
            }
            "text" => self.text(node, &inner),
            "flowRoot" => self.flow_root(node, &inner),
            "image" => crate::image::image(node, &inner),
            "svg" => self.nested_svg(node, inner),
            "use" => self.use_element(node, &inner),
            _ => self.leaf(node, &inner),
        }
    }

    /// Whether a child of `<switch>` is one SVG draws (criterion 99): no extension is supported
    /// (Illustrator's private data), only SVG 1.1 features, English as the language; a bare
    /// `foreignObject` draws nothing here, so it is passed over too.
    fn passes_switch(&self, child: Node<'_, '_>) -> bool {
        let extensions = child
            .attribute("requiredExtensions")
            .is_none_or(|ext| ext.trim().is_empty());
        let features = child.attribute("requiredFeatures").is_none_or(|list| {
            list.split_ascii_whitespace()
                .all(|f| f.starts_with("http://www.w3.org/TR/SVG11/feature#"))
        });
        let language = child.attribute("systemLanguage").is_none_or(|list| {
            list.split(',').any(|tag| {
                let tag = tag.trim().to_ascii_lowercase();
                tag == "en" || tag.starts_with("en-")
            })
        });
        let foreign = self.is(child, "foreignObject");
        extensions && features && language && !foreign
    }

    /// The registers of `node` under its computed style.
    pub(crate) fn state(node: Node<'_, '_>, style: &Computed, leaf: bool) -> NodeState {
        NodeState {
            name: node.attribute((INKSCAPE_NS, "label")).map(str::to_owned),
            hidden: style.display_none || (leaf && style.hidden_visibility),
            locked: node.attribute((SODIPODI_NS, "insensitive")).map(str::trim) == Some("true"),
            opacity: style.opacity,
            guide: false,
            inkscape: match node.attribute((SODIPODI_NS, "type")).map(str::trim) {
                Some("spiral") => Some(InkscapeObject::Spiral),
                Some("inkscape:box3d") => Some(InkscapeObject::Box3d),
                Some("inkscape:box3dside") => Some(InkscapeObject::Box3dSide),
                _ => None,
            },
            guides_layer: false,
        }
    }

    /// A `<g>` or `<a>`: a group, a layer at the root, nothing when empty (a layer is kept).
    fn group(&mut self, node: Node<'_, '_>, ctx: &Ctx, may_be_layer: bool) -> Option<SvgNode> {
        let layer = may_be_layer
            && node.attribute((INKSCAPE_NS, "groupmode")).map(str::trim) == Some("layer");
        let mut state = Self::state(node, &ctx.style, false);
        state.guides_layer = layer
            && self.profile
            && crate::shadow::curvyo(node, "guides").map(str::trim) == Some("1");
        if layer {
            self.layers += 1;
            if state.name.is_none() {
                state.name = Some(
                    node.attribute("id")
                        .map_or_else(|| format!("Layer {}", self.layers), str::to_owned),
                );
            }
        }
        let children = self.children(node, ctx);
        if children.is_empty() && !layer {
            return None;
        }
        Some(SvgNode::Group {
            state,
            layer,
            children,
        })
    }

    /// A nested `<svg>`: a group with its viewport map (criterion 20 (e)); its clip is ignored.
    fn nested_svg(&mut self, node: Node<'_, '_>, mut ctx: Ctx) -> Option<SvgNode> {
        let (map, viewport) = viewport_map(node, &ctx.units);
        ctx.matrix *= map;
        ctx.units.viewport = viewport;
        let state = Self::state(node, &ctx.style, false);
        let children = self.children(node, &ctx);
        (!children.is_empty()).then_some(SvgNode::Group {
            state,
            layer: false,
            children,
        })
    }

    /// A drawn element as a leaf.
    fn leaf(&mut self, node: Node<'_, '_>, ctx: &Ctx) -> Option<SvgNode> {
        let kind = match leaf_kind(node, ctx.matrix, &ctx.units, &mut self.points) {
            Ok(kind) => kind,
            Err(ShapeError::Skip) => return None,
            Err(ShapeError::Budget) => {
                self.stopped = true;
                return None;
            }
        };
        let scale = ctx.matrix.determinant().abs().sqrt();
        let style = ctx.style.leaf_style(scale);
        Some(self.push_leaf(node, ctx, kind, style))
    }

    /// Numbers a new leaf and records where it came from.
    pub(crate) fn push_leaf(
        &mut self,
        node: Node<'_, '_>,
        ctx: &Ctx,
        kind: LeafKind,
        style: crate::drawing::LeafStyle,
    ) -> SvgNode {
        let index = self.leaves;
        self.leaves += 1;
        if ctx.copy_depth == 0 {
            self.leaf_of.insert(node.id().get(), index);
        }
        SvgNode::Leaf {
            state: Self::state(node, &ctx.style, true),
            leaf: SvgLeaf {
                kind,
                style,
                shadow: if self.profile {
                    crate::shadow::read_shadow(node)
                } else {
                    crate::export::Shadow::default()
                },
                marker_refs: crate::shadow::marker_refs(node),
            },
        }
    }
}

/// The viewport map of a nested `<svg>` (`x`, `y`, `width`, `height`, `viewBox` and
/// `preserveAspectRatio`, in the parent's `units`) and the viewport its content sees.
pub(crate) fn viewport_map(node: Node<'_, '_>, units: &Units) -> (Affine, (f64, f64)) {
    let length = |name: &str, axis, default: f64| {
        attr(node, name)
            .and_then(|t| units.parse(t, axis))
            .unwrap_or(default)
    };
    let (vw, vh) = units.viewport;
    let (x, y) = (length("x", Axis::X, 0.0), length("y", Axis::Y, 0.0));
    let (w, h) = (length("width", Axis::X, vw), length("height", Axis::Y, vh));
    match view_box(node.attribute("viewBox")) {
        Some(vb) if w > 0.0 && h > 0.0 => (
            Affine::translate((x, y))
                * view_box_map(vb, aspect(node.attribute("preserveAspectRatio")), w, h),
            (vb.w, vb.h),
        ),
        _ => (Affine::translate((x, y)), (w, h)),
    }
}
