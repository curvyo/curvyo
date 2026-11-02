//! A document as the writer's SVG drawing (`specs/0024-svg-import-export` slice 4, `adrs.md`
//! export §4): the scope of the four options, the export origin and the pages, the tree of
//! layers and groups in z-order, the guides moved to their own layer, and every leaf as the
//! element it is written as (linked clones as `<use>` where the import gets them back).

use std::collections::{HashMap, HashSet};

use curvyo_document_core::{
    BackgroundPaint, DisplayUnit, Document, NodeId, NodeKind, NodeRead, ObjectRead, ObjectSnapshot,
    Point,
};
use curvyo_svg_core::{
    DisplayUnitName, ExportKind, ExportLeaf, ExportNode, ExportPage, ExportState, ExportStyle,
    PageBackground, Shadow, SvgExport, is_isometry,
};
use kurbo::{Affine, Rect};

use crate::svg_export_leaf::{export_style, path_kind, svg_draws_markers};
use crate::svg_export_parts::Parts;
use crate::svg_export_shapes::primitive_kind;
use crate::svg_text_layout::ImportFaces;

/// The export options of the File tab (SVG-OPTIONS A, criterion 147).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)] // the four switches of the File tab
pub struct SvgExportOptions {
    /// Text as paths (criterion 136).
    pub text_as_paths: bool,
    /// Strokes as paths (criterion 146).
    pub strokes_as_paths: bool,
    /// Selection only (criterion 131).
    pub selection_only: bool,
    /// Active page only (criterion 130).
    pub active_page_only: bool,
}

/// What the session knows beyond the document: the selection and the active page.
#[derive(Debug, Clone, Copy)]
pub struct ExportScope<'a> {
    /// The selected nodes.
    pub selection: &'a [NodeId],
    /// The index of the active page in the page list.
    pub active_page: usize,
}

/// The drawing to write and the counts the notices need.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgExportOutcome {
    /// The drawing.
    pub svg: SvgExport,
    /// How many objects are written (0: the drawing is empty, criterion 148).
    pub objects: usize,
    /// Texts written as live text although outlines were asked for: their face is missing
    /// (criteria 136, 148).
    pub texts_kept_live: usize,
}

impl SvgExportOutcome {
    /// The SVG text of the drawing (`curvyo_svg_core::write_svg`).
    #[must_use]
    pub fn text(&self) -> String {
        curvyo_svg_core::write_svg(&self.svg)
    }
}

/// How close a box may come to the active page and still count (criterion 130 (b)).
const PAGE_TOLERANCE_MM: f64 = 1e-6;

/// The longest chain of clones the importer follows (`curvyo_svg_core::limits`).
const MAX_USE_CHAIN: usize = curvyo_svg_core::limits::MAX_CLONE_CHAIN;

/// The document `document` as an SVG drawing: `laid` holds every object as the session laid it
/// out (texts with their glyphs), `fonts` the faces texts are set in. Reads the node tree; a
/// caller that holds one already passes it to [`svg_export_of`].
#[must_use]
pub fn svg_export(
    document: &Document,
    laid: &ObjectRead,
    fonts: &ImportFaces<'_>,
    scope: &ExportScope<'_>,
    options: SvgExportOptions,
) -> SvgExportOutcome {
    svg_export_of(
        document,
        &document.read_nodes(),
        laid,
        fonts,
        scope,
        options,
    )
}

/// [`svg_export`] with the node tree `nodes` of `document` already read (the session's cached
/// read), so the export does not read the document a second time (criterion 71).
#[must_use]
pub fn svg_export_of(
    document: &Document,
    nodes: &[NodeRead],
    laid: &ObjectRead,
    fonts: &ImportFaces<'_>,
    scope: &ExportScope<'_>,
    options: SvgExportOptions,
) -> SvgExportOutcome {
    let pages = document.pages();
    let active = pages
        .get(scope.active_page)
        .or_else(|| pages.first())
        .cloned();
    let page_rect = |p: &curvyo_document_core::Page| {
        Rect::new(
            p.origin.x,
            p.origin.y,
            p.origin.x + p.size.width.as_mm(),
            p.origin.y + p.size.height.as_mm(),
        )
    };
    let shown: Vec<(String, Rect)> = if options.active_page_only {
        active
            .iter()
            .map(|p| (p.name.clone(), page_rect(p)))
            .collect()
    } else {
        pages
            .iter()
            .map(|p| (p.name.clone(), page_rect(p)))
            .collect()
    };
    // The root's size and viewBox are the first written page, as Inkscape reads its first page;
    // the other pages sit beside it in the same user units.
    let first = shown
        .first()
        .map_or(Rect::new(0.0, 0.0, 210.0, 297.0), |(_, r)| *r);
    let t = Affine::translate((-first.x0, -first.y0));
    let tree = Tree::new(nodes, laid);
    let in_scope = tree.scope(scope.selection, options, active.as_ref().map(page_rect));
    let mut parts = Parts::new(&tree, document, fonts, options, t, &in_scope);
    let mut top = Vec::new();
    for &root in &tree.roots {
        parts.node(root, &mut top);
    }
    let svg = SvgExport {
        size: (first.width(), first.height()),
        pages: shown
            .into_iter()
            .map(|(name, rect)| ExportPage {
                name,
                rect: t.transform_rect_bbox(rect),
            })
            .collect(),
        display_unit: match document.display_unit() {
            DisplayUnit::Mm => DisplayUnitName::Mm,
            DisplayUnit::Cm => DisplayUnitName::Cm,
            DisplayUnit::In => DisplayUnitName::In,
        },
        background: {
            let b = document.background();
            match b.paint {
                BackgroundPaint::None => PageBackground::None,
                BackgroundPaint::Solid => {
                    PageBackground::Colour([b.color.r, b.color.g, b.color.b], b.opacity.get())
                }
            }
        },
        nodes: top,
        guides: std::mem::take(&mut parts.guides),
    };
    SvgExportOutcome {
        svg,
        objects: parts.objects,
        texts_kept_live: parts.texts_kept_live,
    }
}

/// The node tree of one read, with each leaf's laid-out snapshot.
pub(crate) struct Tree<'a> {
    pub(crate) nodes: &'a [NodeRead],
    pub(crate) children: Vec<Vec<usize>>,
    pub(crate) roots: Vec<usize>,
    pub(crate) index: HashMap<NodeId, usize>,
    laid: &'a ObjectRead,
}

impl<'a> Tree<'a> {
    fn new(nodes: &'a [NodeRead], laid: &'a ObjectRead) -> Self {
        let index: HashMap<NodeId, usize> =
            nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let mut children = vec![Vec::new(); nodes.len()];
        let mut roots = Vec::new();
        for (i, node) in nodes.iter().enumerate() {
            match node.parent.and_then(|p| index.get(&p)) {
                Some(&parent) => children[parent].push(i),
                None => roots.push(i),
            }
        }
        Self {
            nodes,
            children,
            roots,
            index,
            laid,
        }
    }

    /// The snapshot of leaf `i` as laid out, `None` for a container or an orphan clone.
    pub(crate) fn object(&self, i: usize) -> Option<&'a ObjectSnapshot> {
        let node = &self.nodes[i];
        match self.laid.index.get(&node.id) {
            Some(&at) => self.laid.objects.get(at),
            None => match &node.kind {
                NodeKind::Leaf(ObjectSnapshot::Clone(_)) | NodeKind::Layer | NodeKind::Group => {
                    None
                }
                NodeKind::Leaf(object) => Some(object),
            },
        }
    }

    fn leaves_under(&self, i: usize, out: &mut Vec<usize>) {
        if self.nodes[i].is_container() {
            for &c in &self.children[i] {
                self.leaves_under(c, out);
            }
        } else {
            out.push(i);
        }
    }

    /// Which leaves are in scope (criteria 130, 131): selected (or below a selected node) and
    /// overlapping the active page by the bounds of their unit (a layer's child or a loose
    /// top-level node); the carriers of texts in scope with them.
    fn scope(
        &self,
        selection: &[NodeId],
        options: SvgExportOptions,
        page: Option<Rect>,
    ) -> Vec<bool> {
        let n = self.nodes.len();
        let mut keep = vec![true; n];
        if options.selection_only {
            let selected: HashSet<NodeId> = selection.iter().copied().collect();
            let mut covered = vec![false; n];
            for (i, node) in self.nodes.iter().enumerate() {
                let parent = node.parent.and_then(|p| self.index.get(&p)).copied();
                covered[i] = selected.contains(&node.id) || parent.is_some_and(|p| covered[p]);
            }
            keep = covered;
        }
        if let (true, Some(page)) = (options.active_page_only, page) {
            for (i, node) in self.nodes.iter().enumerate() {
                let parent = node.parent.and_then(|p| self.index.get(&p)).copied();
                let is_unit = parent.is_none_or(|p| matches!(self.nodes[p].kind, NodeKind::Layer));
                if !is_unit || matches!(node.kind, NodeKind::Layer) {
                    continue;
                }
                let mut leaves = Vec::new();
                self.leaves_under(i, &mut leaves);
                let bounds = leaves
                    .iter()
                    .filter_map(|&l| self.object(l).map(bounds_of))
                    .reduce(|a, b| a.union(b));
                let on = bounds.is_some_and(|b| overlaps(page, b));
                if !on {
                    for l in leaves {
                        keep[l] = false;
                    }
                }
            }
        }
        // A text on path keeps its carrier (criterion 130 (e)).
        for i in 0..n {
            if keep[i]
                && let Some(ObjectSnapshot::Text(text)) = self.object(i)
                && let Some(on) = &text.on_path
                && let Some(&carrier) = self.index.get(&on.carrier.node)
            {
                keep[carrier] = true;
            }
        }
        keep
    }

    /// Whether the clone at leaf `i` is written as a `<use>` of its origin (criterion 59) given
    /// which leaves are written.
    pub(crate) fn as_use(&self, i: usize, written: &[bool]) -> Option<(usize, Affine)> {
        let Some(ObjectSnapshot::Clone(clone)) = self.object(i) else {
            return None;
        };
        let chain = &clone.chain;
        let origin = *self.index.get(&chain.origin)?;
        let m = Affine::new(chain.matrix.to_array());
        let origin_node = &self.nodes[origin];
        let fits = !chain.own_placement
            && !chain.own_style
            && chain.depth <= MAX_USE_CHAIN
            && is_isometry(m)
            && written[origin]
            && origin_node.visible
            && !origin_node.guide
            && !self.nodes[i].guide;
        fits.then_some((origin, m))
    }
}

/// The bounds that decide page membership: a text's line box, else the drawn outline's.
fn bounds_of(object: &ObjectSnapshot) -> Rect {
    let (min, max): (Point, Point) = match object {
        ObjectSnapshot::Text(text) => crate::text_box::line_box_bounds(text),
        other => crate::object_bounds::object_outline_bounds(other),
    };
    Rect::new(min.x, min.y, max.x, max.y)
}

/// Whether the box `b` overlaps the page (criterion 130 (b)): an area in common, or a box
/// without width or height with any part inside the closed page.
fn overlaps(page: Rect, b: Rect) -> bool {
    let e = PAGE_TOLERANCE_MM;
    let flat = b.width() <= e || b.height() <= e;
    if flat {
        b.x1 >= page.x0 - e && b.x0 <= page.x1 + e && b.y1 >= page.y0 - e && b.y0 <= page.y1 + e
    } else {
        b.x1 > page.x0 + e && b.x0 < page.x1 - e && b.y1 > page.y0 + e && b.y0 < page.y1 - e
    }
}

/// The registers of `node` as written.
pub(crate) fn state_of(node: &NodeRead) -> ExportState {
    ExportState {
        name: node.registers.name.clone(),
        hidden: !node.registers.visible,
        locked: node.registers.locked,
        opacity: node.registers.opacity.get(),
        key: None,
    }
}

/// A leaf of `kind` with `style` and `shadow`.
pub(crate) fn leaf_node(
    state: ExportState,
    kind: ExportKind,
    style: ExportStyle,
    shadow: Shadow,
) -> ExportNode {
    ExportNode::Leaf {
        state,
        leaf: Box::new(ExportLeaf {
            kind,
            style,
            shadow,
        }),
    }
}

/// The element, style and shadow of a path, primitive, clone or sketch outline mapped by `t`.
pub(crate) fn shape_leaf(
    object: &ObjectSnapshot,
    t: Affine,
) -> Option<(ExportKind, ExportStyle, Shadow)> {
    match object {
        ObjectSnapshot::Path(path) => {
            let (style, shadow) = export_style(&path.style, svg_draws_markers(path));
            Some((path_kind(path, t), style, shadow))
        }
        ObjectSnapshot::Clone(clone) => {
            let path = &clone.resolved;
            let (style, shadow) = export_style(&path.style, svg_draws_markers(path));
            Some((path_kind(path, t), style, shadow))
        }
        ObjectSnapshot::Primitive(p) => {
            let (kind, mut shadow) = primitive_kind(&p.shape, p.rotation, t);
            let (style, stroke_shadow) = export_style(&p.style, false);
            shadow.stroke_width = stroke_shadow.stroke_width;
            shadow.dash = stroke_shadow.dash;
            // A turned element's width is read through its transform: keep it exact.
            if p.rotation.as_radians() != 0.0
                && let Some(stroke) = &style.stroke
                && !matches!(kind, ExportKind::Path { .. })
            {
                shadow.stroke_width = Some(stroke.width_mm);
                if !p.style.stroke.dash.is_solid() {
                    shadow.dash = Some(p.style.stroke.dash.as_slice().to_vec());
                }
            }
            shadow.markers = stroke_shadow.markers;
            Some((kind, style, shadow))
        }
        // A dimension is never output (ADR 0027).
        ObjectSnapshot::Sketch(_)
        | ObjectSnapshot::Text(_)
        | ObjectSnapshot::Image(_)
        | ObjectSnapshot::Dimension(_) => None,
    }
}
