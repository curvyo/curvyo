//! One drawn leaf as written (criteria 54 to 59, 139 to 141): path data in absolute millimetres
//! with `M L C Z`, primitives as their SVG elements with Inkscape's data, `use`, the style as
//! presentation attributes, the `marker-*` references and the `curvyo:` shadow data.

use kurbo::{BezPath, PathEl};

use crate::drawing::{Cap, Join};
use crate::export::{
    ExportKind, ExportLeaf, ExportState, ExportStyle, MarkerShape, Shadow, ShadowMarkers,
};
use crate::write::{Writer, hex, marker_def};
use crate::write_number::{coord, exact};

/// `d` of `path`: absolute `M L C Z`, numbers by [`coord`], one space between all tokens
/// (criterion 54). Quadratic segments are raised to cubics.
pub(crate) fn path_data(path: &BezPath, out: &mut String) {
    let mut last = kurbo::Point::ZERO;
    let mut first = true;
    let mut push = |out: &mut String, letter: char, points: &[kurbo::Point]| {
        if !first {
            out.push(' ');
        }
        first = false;
        out.push(letter);
        for p in points {
            out.push(' ');
            coord(p.x, out);
            out.push(' ');
            coord(p.y, out);
        }
    };
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                push(out, 'M', &[p]);
                last = p;
            }
            PathEl::LineTo(p) => {
                push(out, 'L', &[p]);
                last = p;
            }
            PathEl::QuadTo(c, p) => {
                let c1 = last + (c - last) * (2.0 / 3.0);
                let c2 = p + (c - p) * (2.0 / 3.0);
                push(out, 'C', &[c1, c2, p]);
                last = p;
            }
            PathEl::CurveTo(c1, c2, p) => {
                push(out, 'C', &[c1, c2, p]);
                last = p;
            }
            PathEl::ClosePath => push(out, 'Z', &[]),
        }
    }
}

impl Writer<'_> {
    pub(crate) fn leaf(&mut self, state: &ExportState, leaf: &ExportLeaf, depth: usize) {
        let id = self.take_id();
        self.indent(depth);
        let open = |w: &mut Self, element: &str| {
            w.out.push('<');
            w.out.push_str(element);
            w.attr("id", &id);
            if let Some(name) = &state.name {
                w.attr_name("inkscape:label", name);
            }
        };
        match &leaf.kind {
            ExportKind::Path {
                path, node_types, ..
            } => {
                open(self, "path");
                self.d(path);
                self.node_types(node_types);
            }
            ExportKind::Rect {
                rect,
                radius,
                rotation_deg,
            } => {
                open(self, "rect");
                self.attr_coord("x", rect.x0);
                self.attr_coord("y", rect.y0);
                self.attr_coord("width", rect.width());
                self.attr_coord("height", rect.height());
                if *radius > 0.0 {
                    self.attr_coord("rx", *radius);
                    self.attr_coord("ry", *radius);
                }
                self.rotate(*rotation_deg, rect.center());
            }
            ExportKind::Ellipse {
                center,
                rx,
                ry,
                rotation_deg,
            } => {
                let circle = (rx - ry).abs() <= crate::export::CIRCLE_TOLERANCE_MM;
                open(self, if circle { "circle" } else { "ellipse" });
                self.attr_coord("cx", center.x);
                self.attr_coord("cy", center.y);
                if circle {
                    self.attr_coord("r", *rx);
                } else {
                    self.attr_coord("rx", *rx);
                    self.attr_coord("ry", *ry);
                }
                self.rotate(*rotation_deg, *center);
            }
            ExportKind::Star {
                star,
                path,
                node_types,
            } => {
                open(self, "path");
                self.star_numbers(star);
                self.d(path);
                self.node_types(node_types);
            }
            ExportKind::Arc {
                arc,
                path,
                rotation_deg,
            } => {
                open(self, "path");
                self.arc_numbers(arc);
                self.d(path);
                self.rotate(*rotation_deg, arc.center);
            }
            ExportKind::Use { target, matrix } => {
                open(self, "use");
                debug_assert!(
                    self.ids.keys.contains_key(target),
                    "the mapping gives every `use` target a key"
                );
                let href = format!(
                    "#{}",
                    self.ids.keys.get(target).cloned().unwrap_or_default()
                );
                self.attr("xlink:href", &href);
                self.matrix(*matrix);
                self.state_flags(state);
                self.out.push_str("/>\n");
                return;
            }
        }
        self.state_flags(state);
        self.paint(&leaf.style, true);
        self.opacity(state);
        self.shadow(&leaf.shadow);
        self.out.push_str("/>\n");
    }

    /// Inkscape's star data (criterion 55).
    fn star_numbers(&mut self, star: &crate::drawing::StarData) {
        self.attr("sodipodi:type", "star");
        self.attr("sodipodi:sides", &star.sides.to_string());
        self.attr_exact("sodipodi:cx", star.center.x);
        self.attr_exact("sodipodi:cy", star.center.y);
        self.attr_exact("sodipodi:r1", star.r1);
        self.attr_exact("sodipodi:r2", star.r2.unwrap_or(star.r1));
        self.attr_exact("sodipodi:arg1", star.arg1);
        self.attr_exact("sodipodi:arg2", star.arg2.unwrap_or(star.arg1));
        self.attr(
            "inkscape:flatsided",
            if star.flat { "true" } else { "false" },
        );
        self.attr("inkscape:rounded", "0");
        self.attr("inkscape:randomized", "0");
    }

    /// Inkscape's arc data (criterion 139).
    fn arc_numbers(&mut self, arc: &crate::drawing::ArcData) {
        self.attr("sodipodi:type", "arc");
        self.attr_exact("sodipodi:cx", arc.center.x);
        self.attr_exact("sodipodi:cy", arc.center.y);
        self.attr_exact("sodipodi:rx", arc.rx);
        self.attr_exact("sodipodi:ry", arc.ry);
        self.attr_exact("sodipodi:start", arc.start.unwrap_or(0.0));
        self.attr_exact("sodipodi:end", arc.end.unwrap_or(std::f64::consts::TAU));
        let arc_type = match arc.arc_type.as_deref() {
            Some("arc") => "arc",
            Some("chord") => "chord",
            _ => "slice",
        };
        self.attr("sodipodi:arc-type", arc_type);
        if arc.open {
            self.attr("sodipodi:open", "true");
        }
    }

    fn d(&mut self, path: &BezPath) {
        self.out.push_str(" d=\"");
        path_data(path, &mut self.out);
        self.out.push('"');
    }

    fn node_types(&mut self, node_types: &str) {
        if !node_types.is_empty() {
            self.attr("sodipodi:nodetypes", node_types);
        }
    }

    /// `transform="rotate(deg cx cy)"` when `deg` is not 0.
    pub(crate) fn rotate(&mut self, deg: f64, center: kurbo::Point) {
        if deg == 0.0 {
            return;
        }
        self.out.push_str(" transform=\"rotate(");
        exact(deg, &mut self.out);
        self.out.push(' ');
        coord(center.x, &mut self.out);
        self.out.push(' ');
        coord(center.y, &mut self.out);
        self.out.push_str(")\"");
    }

    /// `transform="matrix(a b c d e f)"`, exact numbers.
    pub(crate) fn matrix(&mut self, m: kurbo::Affine) {
        self.out.push_str(" transform=\"matrix(");
        for (i, v) in m.as_coeffs().iter().enumerate() {
            if i > 0 {
                self.out.push(' ');
            }
            exact(*v, &mut self.out);
        }
        self.out.push_str(")\"");
    }

    /// The presentation attributes of `style` (criterion 56), the marker references with
    /// `markers` (criterion 141 (b)).
    pub(crate) fn paint(&mut self, style: &ExportStyle, markers: bool) {
        match style.fill {
            Some(fill) => {
                self.attr("fill", &hex(fill.rgb));
                if fill.opacity < 1.0 {
                    self.attr_exact("fill-opacity", fill.opacity);
                }
            }
            None => self.attr("fill", "none"),
        }
        match &style.stroke {
            None => self.attr("stroke", "none"),
            Some(stroke) => {
                self.attr("stroke", &hex(stroke.paint.rgb));
                self.attr_coord("stroke-width", stroke.width_mm);
                if stroke.paint.opacity < 1.0 {
                    self.attr_exact("stroke-opacity", stroke.paint.opacity);
                }
                match stroke.cap {
                    Cap::Butt => {}
                    Cap::Round => self.attr("stroke-linecap", "round"),
                    Cap::Square => self.attr("stroke-linecap", "square"),
                }
                match stroke.join {
                    Join::Miter => {}
                    Join::Round => self.attr("stroke-linejoin", "round"),
                    Join::Bevel => self.attr("stroke-linejoin", "bevel"),
                }
                if !stroke.dash_mm.is_empty() {
                    self.attr_list("stroke-dasharray", &stroke.dash_mm, coord);
                }
            }
        }
        if style.fill.is_some() {
            self.attr("fill-rule", "nonzero");
        }
        if !markers {
            return;
        }
        if let Some(stroke) = &style.stroke {
            for (name, shape, reversed) in [
                ("marker-start", stroke.start, true),
                ("marker-mid", stroke.mid, false),
                ("marker-end", stroke.end, false),
            ] {
                if let Some(shape) = shape {
                    let n = self.ids.marker(marker_def(shape, reversed, stroke.paint));
                    self.attr(name, &format!("url(#marker{n})"));
                }
            }
        }
    }

    /// The `curvyo:` attributes in their fixed order (criteria 57, 140, 141, 149).
    pub(crate) fn shadow(&mut self, shadow: &Shadow) {
        if let Some(kind) = shadow.kind {
            self.attr("curvyo:kind", kind.name());
        }
        if let Some(frame) = &shadow.frame {
            self.attr_list("curvyo:frame", frame, exact);
        }
        if let Some(radii) = &shadow.radii {
            self.attr_list("curvyo:radii", radii, exact);
        }
        if let Some(width) = shadow.stroke_width {
            self.attr_exact("curvyo:stroke-width", width);
        }
        if let Some(dash) = &shadow.dash {
            self.attr_list("curvyo:dash", dash, exact);
        }
        if let Some(markers) = shadow.markers {
            self.attr("curvyo:markers", &markers_text(markers));
        }
    }
}

/// The value of `curvyo:markers` (criterion 141 (d)).
pub(crate) fn markers_text(markers: ShadowMarkers) -> String {
    let name = |shape: Option<MarkerShape>| match shape {
        None => "none",
        Some(MarkerShape::Arrow) => "arrow",
        Some(MarkerShape::Dot) => "dot",
    };
    format!(
        "start={} middle={} place={} count={} end={}",
        name(markers.start),
        name(markers.middle),
        if markers.at_nodes { "nodes" } else { "spaced" },
        markers.count,
        name(markers.end)
    )
}
