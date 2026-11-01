//! The SVG writer (`specs/0024-svg-import-export` criteria 52, 53, 128, 141 and 143): the root,
//! the named view with its pages, the marker definitions, the tree in z-order and the Guides
//! layer, with ids numbered per kind in document order in one pre-pass.

use std::collections::BTreeMap;

use crate::drawing::{DisplayUnitName, PageBackground, Rgb};
use crate::export::{
    CIRCLE_TOLERANCE_MM, ExportKind, ExportNode, ExportState, MarkerShape, PathId, SvgExport,
};
use crate::write_number::{coord, escape_name, exact};

/// The namespace of the `curvyo:` profile (criterion 52).
pub(crate) const CURVYO_NS: &str = "urn:curvyo:svg:1";

/// The version of the `curvyo:` profile the writer writes and the reader reads.
pub(crate) const CURVYO_FORMAT: &str = "1";

/// A `<marker>` definition: shape, whether it is drawn reversed (a start arrow), colour and
/// opacity (criterion 141 (a)).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MarkerDef {
    pub(crate) shape: MarkerShape,
    pub(crate) reversed: bool,
    pub(crate) rgb: Rgb,
    pub(crate) opacity: f64,
}

/// The ids of one file, assigned in document order before anything is written.
pub(crate) struct Ids {
    /// Per node in walk order (guides layer last), its id.
    pub(crate) nodes: Vec<String>,
    /// The id of each key.
    pub(crate) keys: BTreeMap<usize, String>,
    /// The marker definitions in first-use order; `marker1` is the first.
    pub(crate) markers: Vec<MarkerDef>,
    /// Whether anything needs `xlink:href`.
    pub(crate) xlink: bool,
    counters: BTreeMap<&'static str, usize>,
}

impl Ids {
    fn assign(drawing: &SvgExport) -> Self {
        let mut ids = Self {
            nodes: Vec::new(),
            keys: BTreeMap::new(),
            markers: Vec::new(),
            xlink: false,
            counters: BTreeMap::new(),
        };
        ids.walk(&drawing.nodes);
        if !drawing.guides.is_empty() {
            let id = ids.next("layer");
            ids.nodes.push(id);
            ids.walk(&drawing.guides);
        }
        ids
    }

    fn next(&mut self, prefix: &'static str) -> String {
        let n = self.counters.entry(prefix).or_insert(0);
        *n += 1;
        format!("{prefix}{n}")
    }

    fn walk(&mut self, nodes: &[ExportNode]) {
        for node in nodes {
            let id = self.next(prefix(node));
            if let Some(key) = node.state().key {
                self.keys.insert(key, id.clone());
            }
            self.nodes.push(id);
            match node {
                ExportNode::Group { children, .. } => self.walk(children),
                ExportNode::Leaf { leaf, .. } => {
                    if matches!(leaf.kind, ExportKind::Use { .. }) {
                        self.xlink = true;
                    }
                    if let Some(stroke) = &leaf.style.stroke {
                        for (shape, reversed) in [
                            (stroke.start, true),
                            (stroke.mid, false),
                            (stroke.end, false),
                        ] {
                            if let Some(shape) = shape {
                                let def = marker_def(shape, reversed, stroke.paint);
                                if !self.markers.contains(&def) {
                                    self.markers.push(def);
                                }
                            }
                        }
                    }
                }
                ExportNode::Text { text, .. } => {
                    if text.lines.iter().any(|l| l.on_path.is_some()) {
                        self.xlink = true;
                    }
                }
                ExportNode::Image { .. } => self.xlink = true,
            }
        }
    }

    /// The id of marker `def` (assigned in the pre-pass).
    pub(crate) fn marker(&self, def: MarkerDef) -> usize {
        self.markers
            .iter()
            .position(|m| *m == def)
            .map_or(0, |i| i + 1)
    }
}

/// The definition a stroke's marker slot uses.
pub(crate) fn marker_def(
    shape: MarkerShape,
    reversed: bool,
    paint: crate::drawing::Paint,
) -> MarkerDef {
    MarkerDef {
        shape,
        // A dot looks the same both ways.
        reversed: reversed && shape == MarkerShape::Arrow,
        rgb: paint.rgb,
        opacity: paint.opacity,
    }
}

/// The id prefix of `node` (criterion 53).
fn prefix(node: &ExportNode) -> &'static str {
    match node {
        ExportNode::Group { layer: true, .. } => "layer",
        ExportNode::Group { .. } => "g",
        ExportNode::Text { .. } => "text",
        ExportNode::Image { .. } => "image",
        ExportNode::Leaf { leaf, .. } => match &leaf.kind {
            ExportKind::Path { id, .. } => match id {
                PathId::Path => "path",
                PathId::Compound => "compound",
                PathId::Rect => "rect",
                PathId::Ellipse => "ellipse",
                PathId::Polygon => "polygon",
                PathId::Star => "star",
            },
            ExportKind::Rect { .. } => "rect",
            ExportKind::Ellipse { rx, ry, .. } if (rx - ry).abs() <= CIRCLE_TOLERANCE_MM => {
                "circle"
            }
            ExportKind::Ellipse { .. } => "ellipse",
            ExportKind::Star { star, .. } if star.flat => "polygon",
            ExportKind::Star { .. } => "star",
            ExportKind::Arc { .. } => "path",
            ExportKind::Use { .. } => "use",
        },
    }
}

/// The writer's state while it writes one file.
pub(crate) struct Writer<'a> {
    pub(crate) out: String,
    pub(crate) ids: &'a Ids,
    /// The next node in walk order.
    pub(crate) at: usize,
}

/// The SVG text of `drawing`: UTF-8 XML, the same bytes for the same input on every target
/// (criteria 52 to 61, 72, 128 to 146).
#[must_use]
pub fn write_svg(drawing: &SvgExport) -> String {
    let ids = Ids::assign(drawing);
    let mut w = Writer {
        out: String::with_capacity(4096),
        ids: &ids,
        at: 0,
    };
    w.root(drawing);
    w.named_view(drawing);
    w.defs();
    for node in &drawing.nodes {
        w.node(node, 1);
    }
    if !drawing.guides.is_empty() {
        w.guides_layer(&drawing.guides);
    }
    w.out.push_str("</svg>\n");
    w.out
}

impl Writer<'_> {
    /// The id of the next node in walk order.
    pub(crate) fn take_id(&mut self) -> String {
        let id = self.ids.nodes.get(self.at).cloned().unwrap_or_default();
        self.at += 1;
        id
    }

    pub(crate) fn indent(&mut self, depth: usize) {
        for _ in 0..depth {
            self.out.push_str("  ");
        }
    }

    /// ` name="value"` with `value` already safe.
    pub(crate) fn attr(&mut self, name: &str, value: &str) {
        self.out.push(' ');
        self.out.push_str(name);
        self.out.push_str("=\"");
        self.out.push_str(value);
        self.out.push('"');
    }

    /// ` name="<coordinate>"`.
    pub(crate) fn attr_coord(&mut self, name: &str, value: f64) {
        self.out.push(' ');
        self.out.push_str(name);
        self.out.push_str("=\"");
        coord(value, &mut self.out);
        self.out.push('"');
    }

    /// ` name="<exact number>"`.
    pub(crate) fn attr_exact(&mut self, name: &str, value: f64) {
        self.out.push(' ');
        self.out.push_str(name);
        self.out.push_str("=\"");
        exact(value, &mut self.out);
        self.out.push('"');
    }

    /// ` name="<escaped name>"`.
    pub(crate) fn attr_name(&mut self, name: &str, value: &str) {
        self.out.push(' ');
        self.out.push_str(name);
        self.out.push_str("=\"");
        escape_name(value, &mut self.out);
        self.out.push('"');
    }

    /// ` name="<numbers separated by spaces>"` with `number` writing each.
    pub(crate) fn attr_list(&mut self, name: &str, values: &[f64], number: fn(f64, &mut String)) {
        self.out.push(' ');
        self.out.push_str(name);
        self.out.push_str("=\"");
        for (i, v) in values.iter().enumerate() {
            if i > 0 {
                self.out.push(' ');
            }
            number(*v, &mut self.out);
        }
        self.out.push('"');
    }

    /// The registers after the geometry: hidden and locked (criterion 53).
    pub(crate) fn state_flags(&mut self, state: &ExportState) {
        if state.hidden {
            self.attr("style", "display:none");
        }
        if state.locked {
            self.attr("sodipodi:insensitive", "true");
        }
    }

    /// `opacity` below 1.
    pub(crate) fn opacity(&mut self, state: &ExportState) {
        if state.opacity < 1.0 {
            self.attr_exact("opacity", state.opacity);
        }
    }

    fn root(&mut self, drawing: &SvgExport) {
        let (w, h) = drawing.size;
        self.out.push_str(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"",
        );
        if self.ids.xlink {
            self.attr("xmlns:xlink", "http://www.w3.org/1999/xlink");
        }
        self.attr("xmlns:inkscape", crate::page::INKSCAPE_NS);
        self.out.push_str("\n    ");
        self.attr("xmlns:sodipodi", crate::page::SODIPODI_NS);
        self.attr("xmlns:curvyo", CURVYO_NS);
        self.out.push_str("\n    ");
        let mut size = String::new();
        coord(w, &mut size);
        size.push_str("mm");
        self.attr("width", &size);
        size.clear();
        coord(h, &mut size);
        size.push_str("mm");
        self.attr("height", &size);
        let mut view = String::from("0 0 ");
        coord(w, &mut view);
        view.push(' ');
        coord(h, &mut view);
        self.attr("viewBox", &view);
        self.attr("curvyo:format", CURVYO_FORMAT);
        self.out.push_str(">\n");
    }

    fn named_view(&mut self, drawing: &SvgExport) {
        self.indent(1);
        self.out.push_str("<sodipodi:namedview");
        self.attr("id", "namedview1");
        let unit = match drawing.display_unit {
            DisplayUnitName::Mm => "mm",
            DisplayUnitName::Cm => "cm",
            DisplayUnitName::In => "in",
        };
        self.attr("inkscape:document-units", unit);
        let (rgb, opacity) = match drawing.background {
            PageBackground::None => ([255, 255, 255], 0.0),
            PageBackground::Colour(rgb, opacity) => (rgb, opacity),
        };
        self.attr("pagecolor", &hex(rgb));
        self.attr_exact("inkscape:pageopacity", opacity);
        if drawing.pages.len() < 2 {
            self.out.push_str("/>\n");
            return;
        }
        self.out.push_str(">\n");
        for (i, page) in drawing.pages.iter().enumerate() {
            self.indent(2);
            self.out.push_str("<inkscape:page");
            self.attr("id", &format!("page{}", i + 1));
            self.attr_coord("x", page.rect.x0);
            self.attr_coord("y", page.rect.y0);
            self.attr_coord("width", page.rect.width());
            self.attr_coord("height", page.rect.height());
            self.attr_name("inkscape:label", &page.name);
            self.out.push_str("/>\n");
        }
        self.indent(1);
        self.out.push_str("</sodipodi:namedview>\n");
    }

    fn defs(&mut self) {
        if self.ids.markers.is_empty() {
            return;
        }
        self.indent(1);
        self.out.push_str("<defs>\n");
        let markers = self.ids.markers.clone();
        for (i, def) in markers.iter().enumerate() {
            self.indent(2);
            self.out.push_str("<marker");
            self.attr("id", &format!("marker{}", i + 1));
            self.attr("markerUnits", "strokeWidth");
            self.attr("orient", "auto");
            self.attr("overflow", "visible");
            let (size, view) = match def.shape {
                MarkerShape::Arrow => (("4", "3"), "-2 -1.5 4 3"),
                MarkerShape::Dot => (("3", "3"), "-1.5 -1.5 3 3"),
            };
            self.attr("markerWidth", size.0);
            self.attr("markerHeight", size.1);
            self.attr("viewBox", view);
            self.attr("refX", "0");
            self.attr("refY", "0");
            self.out.push_str(">\n");
            self.indent(3);
            match (def.shape, def.reversed) {
                (MarkerShape::Arrow, false) => {
                    self.out.push_str("<path");
                    self.attr("d", "M -2 -1.5 L 2 0 L -2 1.5 Z");
                }
                (MarkerShape::Arrow, true) => {
                    self.out.push_str("<path");
                    self.attr("d", "M 2 -1.5 L -2 0 L 2 1.5 Z");
                }
                (MarkerShape::Dot, _) => {
                    self.out.push_str("<circle");
                    self.attr("cx", "0");
                    self.attr("cy", "0");
                    self.attr("r", "1.5");
                }
            }
            self.attr("fill", &hex(def.rgb));
            if def.opacity < 1.0 {
                self.attr_exact("fill-opacity", def.opacity);
            }
            self.attr("stroke", "none");
            self.out.push_str("/>\n");
            self.indent(2);
            self.out.push_str("</marker>\n");
        }
        self.indent(1);
        self.out.push_str("</defs>\n");
    }

    fn guides_layer(&mut self, guides: &[ExportNode]) {
        let id = self.take_id();
        self.indent(1);
        self.out.push_str("<g");
        self.attr("id", &id);
        self.attr("inkscape:groupmode", "layer");
        self.attr("inkscape:label", "Guides");
        self.attr("curvyo:guides", "1");
        self.attr("style", "display:none");
        self.attr("sodipodi:insensitive", "true");
        self.out.push_str(">\n");
        for node in guides {
            self.node(node, 2);
        }
        self.indent(1);
        self.out.push_str("</g>\n");
    }

    /// One node at `depth`.
    pub(crate) fn node(&mut self, node: &ExportNode, depth: usize) {
        match node {
            ExportNode::Group {
                state,
                layer,
                children,
            } => {
                let id = self.take_id();
                self.indent(depth);
                self.out.push_str("<g");
                self.attr("id", &id);
                if *layer {
                    self.attr("inkscape:groupmode", "layer");
                }
                if let Some(name) = &state.name {
                    self.attr_name("inkscape:label", name);
                }
                self.state_flags(state);
                self.opacity(state);
                if children.is_empty() {
                    self.out.push_str("/>\n");
                    return;
                }
                self.out.push_str(">\n");
                for child in children {
                    self.node(child, depth + 1);
                }
                self.indent(depth);
                self.out.push_str("</g>\n");
            }
            ExportNode::Leaf { state, leaf } => self.leaf(state, leaf, depth),
            ExportNode::Text { state, text } => self.text(state, text, depth),
            ExportNode::Image { state, image } => self.image(state, image, depth),
        }
    }
}

/// `#rrggbb`.
pub(crate) fn hex(rgb: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}
