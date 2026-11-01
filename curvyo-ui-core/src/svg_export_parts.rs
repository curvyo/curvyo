//! The walk of the SVG export over the node tree (`specs/0024-svg-import-export` criteria 53,
//! 59, 136, 138, 143 and 146): containers kept while they hold something written, guides moved
//! to the Guides layer, clones as `<use>` or paths, texts live or as outlines, Strokes as paths.

use std::collections::HashSet;

use curvyo_document_core::{
    Document, FontKind, NodeKind, ObjectSnapshot, PathSnapshot, TextSnapshot, single_line_style,
};
use curvyo_svg_core::{ExportKind, ExportNode, ExportState, ExportStyle, PathId, Shadow};
use kurbo::{Affine, ParamCurveArclen};

use crate::stroke_to_path_plan::{draws_stroke, stroke_outline_anchors, stroked_outlines_of};
use crate::svg_export::{SvgExportOptions, Tree, leaf_node, shape_leaf, state_of};
use crate::svg_export_leaf::{export_style, offset_data, paint, path_data, path_kind};
use crate::svg_export_sketch::sketch_objects;
use crate::svg_export_text::{OnCarrier, export_image, live_text};
use crate::svg_text_layout::ImportFaces;

/// The accuracy of a carrier's length, mm.
const LENGTH_ACCURACY_MM: f64 = 1e-7;

/// The state of one walk.
pub(crate) struct Parts<'t, 'a> {
    tree: &'t Tree<'a>,
    document: &'t Document,
    fonts: &'t ImportFaces<'t>,
    options: SvgExportOptions,
    t: Affine,
    /// Per node: a leaf that is written.
    written: Vec<bool>,
    /// The nodes a `use` or a `textPath` names.
    keys: HashSet<usize>,
    /// The leaves of the Guides layer, in document order.
    pub(crate) guides: Vec<ExportNode>,
    /// How many objects are written.
    pub(crate) objects: usize,
    /// Texts kept live for a missing face although outlines were asked for.
    pub(crate) texts_kept_live: usize,
}

impl<'t, 'a> Parts<'t, 'a> {
    pub(crate) fn new(
        tree: &'t Tree<'a>,
        document: &'t Document,
        fonts: &'t ImportFaces<'t>,
        options: SvgExportOptions,
        t: Affine,
        in_scope: &[bool],
    ) -> Self {
        let written: Vec<bool> = in_scope
            .iter()
            .enumerate()
            .map(|(i, keep)| *keep && tree.object(i).is_some())
            .collect();
        let mut keys = HashSet::new();
        for i in (0..written.len()).filter(|&i| written[i]) {
            if let Some((origin, _)) = tree.as_use(i, &written) {
                keys.insert(origin);
            }
            if let Some(ObjectSnapshot::Text(text)) = tree.object(i)
                && let Some(on) = &text.on_path
                && let Some(&carrier) = tree.index.get(&on.carrier.node)
            {
                keys.insert(carrier);
            }
        }
        Self {
            tree,
            document,
            fonts,
            options,
            t,
            written,
            keys,
            guides: Vec::new(),
            objects: 0,
            texts_kept_live: 0,
        }
    }

    fn filtered(&self) -> bool {
        self.options.selection_only || self.options.active_page_only
    }

    /// Writes node `i` into `out` (or a guide into the Guides layer).
    pub(crate) fn node(&mut self, i: usize, out: &mut Vec<ExportNode>) {
        let node = &self.tree.nodes[i];
        if node.is_container() {
            let mut inner = Vec::new();
            for &child in &self.tree.children[i] {
                self.node(child, &mut inner);
            }
            let empty_in_model = self.tree.children[i].is_empty();
            if !inner.is_empty() || (empty_in_model && !self.filtered()) {
                out.push(ExportNode::Group {
                    state: state_of(node),
                    layer: matches!(node.kind, NodeKind::Layer),
                    children: inner,
                });
            }
            return;
        }
        if !self.written[i] {
            return;
        }
        let mut state = state_of(node);
        if self.keys.contains(&i) {
            state.key = Some(i);
        }
        let Some(made) = self.leaf(i, state) else {
            return;
        };
        self.objects += 1;
        if node.guide {
            self.guides.push(made);
        } else {
            out.push(made);
        }
    }

    fn leaf(&mut self, i: usize, mut state: ExportState) -> Option<ExportNode> {
        let object = self.tree.object(i)?;
        let t = self.t;
        match object {
            ObjectSnapshot::Clone(clone) => {
                if let Some((origin, m)) = self.tree.as_use(i, &self.written) {
                    let kind = ExportKind::Use {
                        target: origin,
                        matrix: t * m * t.inverse(),
                    };
                    return Some(leaf_node(
                        state,
                        kind,
                        ExportStyle::default(),
                        Shadow::default(),
                    ));
                }
                if !clone.chain.own_style
                    && let Some(&source) = self.tree.index.get(&clone.chain.style_source)
                {
                    state.opacity = self.tree.nodes[source].registers.opacity.get();
                }
                self.shape(object, state)
            }
            ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_) => self.shape(object, state),
            ObjectSnapshot::Sketch(sketch) => {
                let children = sketch_objects(sketch)
                    .iter()
                    .filter_map(|one| self.shape(one, ExportState::default()))
                    .collect();
                Some(ExportNode::Group {
                    state,
                    layer: false,
                    children,
                })
            }
            ObjectSnapshot::Text(text) => self.text(text, state),
            ObjectSnapshot::Image(image) => Some(ExportNode::Image {
                state,
                image: Box::new(export_image(image, self.document, t)?),
            }),
        }
    }

    /// A path, primitive or clone, split into its fill and its stroke outline under Strokes as
    /// paths (criterion 146).
    fn shape(&self, object: &ObjectSnapshot, state: ExportState) -> Option<ExportNode> {
        let (kind, style, shadow) = shape_leaf(object, self.t)?;
        if !(self.options.strokes_as_paths && draws_stroke(object.style())) {
            return Some(leaf_node(state, kind, style, shadow));
        }
        let (outlines, markers) = stroked_outlines_of(object);
        let stroke = stroke_outline_anchors(&outlines, object.style(), markers)
            .map(|outline| offset_data(&outline, self.t));
        Some(Self::split(
            state,
            (kind, style, shadow),
            stroke,
            object.style(),
        ))
    }

    /// The group of a fill part and a stroke outline (criterion 146).
    fn split(
        state: ExportState,
        (kind, style, shadow): (ExportKind, ExportStyle, Shadow),
        stroke: Option<(kurbo::BezPath, String)>,
        model: &curvyo_document_core::Style,
    ) -> ExportNode {
        let mut children = Vec::new();
        if style.fill.is_some() {
            let fill_shadow = Shadow {
                dash: None,
                stroke_width: None,
                markers: None,
                ..shadow
            };
            let fill_only = ExportStyle {
                fill: style.fill,
                stroke: None,
            };
            children.push(leaf_node(
                ExportState::default(),
                kind,
                fill_only,
                fill_shadow,
            ));
        }
        if let Some((path, node_types)) = stroke {
            let compound = path
                .elements()
                .iter()
                .filter(|e| matches!(e, kurbo::PathEl::MoveTo(_)))
                .count()
                > 1;
            let s = &model.stroke;
            children.push(leaf_node(
                ExportState::default(),
                ExportKind::Path {
                    path,
                    node_types,
                    id: if compound {
                        PathId::Compound
                    } else {
                        PathId::Path
                    },
                },
                ExportStyle {
                    fill: Some(paint(s.color, s.opacity.get())),
                    stroke: None,
                },
                Shadow::default(),
            ));
        }
        ExportNode::Group {
            state,
            layer: false,
            children,
        }
    }

    /// A text: live, or its glyph outlines (criteria 132 to 136); `None` for outlines without a
    /// node (a text of spaces).
    fn text(&mut self, text: &TextSnapshot, state: ExportState) -> Option<ExportNode> {
        let single_line = text.font.kind == FontKind::SingleLine;
        let stroked = self.options.strokes_as_paths && draws_stroke(&text.style);
        let want_paths = single_line || self.options.text_as_paths || stroked;
        let glyphs = text
            .outline
            .as_deref()
            .filter(|outline| !outline.substitute)
            .map(|outline| &outline.glyphs);
        let Some(glyphs) = glyphs.filter(|_| want_paths) else {
            if want_paths && !single_line {
                self.texts_kept_live += 1;
            }
            let carrier = self.carrier_of(text);
            return Some(ExportNode::Text {
                state,
                text: Box::new(live_text(text, self.fonts, self.t, carrier)),
            });
        };
        if glyphs.subpaths().all(|s| s.anchors.is_empty()) {
            return None;
        }
        let mut style = text.style.clone();
        if single_line {
            style = single_line_style(&style).unwrap_or(style);
            style.fill.enabled = false;
        }
        let mut path = glyphs.clone();
        path.style = style.clone();
        let (export, shadow) = export_style(&style, false);
        // One outline is a `path`, more a `compound`, as the import gives them back (criterion 152).
        let kind = path_kind(&path, self.t);
        if !(self.options.strokes_as_paths && draws_stroke(&style)) {
            return Some(leaf_node(state, kind, export, shadow));
        }
        let outlines: Vec<_> = path
            .subpaths()
            .map(|s| {
                let triples = s
                    .anchors
                    .iter()
                    .map(|a| (a.point, a.handle_in, a.handle_out))
                    .collect();
                (triples, s.closed)
            })
            .collect();
        let stroke = stroke_outline_anchors(&outlines, &style, false)
            .map(|outline| offset_data(&outline, self.t));
        Some(Self::split(state, (kind, export, shadow), stroke, &style))
    }

    /// The carrier of a text on path: its key and its outline's length.
    fn carrier_of(&self, text: &TextSnapshot) -> Option<OnCarrier> {
        let on = text.on_path.as_ref()?;
        let key = *self.tree.index.get(&on.carrier.node)?;
        let path = match self.tree.object(key)? {
            ObjectSnapshot::Primitive(p) => {
                let outline =
                    curvyo_document_core::drawing_outline_of_rotated(&p.shape, p.rotation);
                let nodes: Vec<_> = outline
                    .anchors
                    .iter()
                    .map(|a| (a.point, a.handle_in, a.handle_out, a.kind))
                    .collect();
                let mut path = kurbo::BezPath::new();
                let mut letters = String::new();
                crate::svg_export_leaf::push_outline(
                    &nodes,
                    outline.closed,
                    Affine::IDENTITY,
                    &mut path,
                    &mut letters,
                );
                path
            }
            other => {
                let drawn = other.drawn_path()?;
                let subpath = drawn.subpaths().nth(on.carrier.outline as usize)?;
                let one = PathSnapshot {
                    closed: subpath.closed,
                    anchors: subpath.anchors.to_vec(),
                    extra_subpaths: Vec::new(),
                    ..drawn.clone()
                };
                path_data(&one, Affine::IDENTITY).0
            }
        };
        let length = path
            .segments()
            .map(|seg| seg.arclen(LENGTH_ACCURACY_MM))
            .sum();
        Some(OnCarrier { key, length })
    }
}
