//! The rigid map of each cluster of a laid-out text onto its carrier
//! (`specs/0080-text-on-path` criteria 8 to 12, 18, 19, 26, 47, 48; `adrs.md` decision 5).
//!
//! text-core lays the lines out in line space as for a straight text (kerns, spacing and
//! alignment included); here each cluster is put on the carrier at the arc length of its centre
//! and turned to the tangent there. Position is the arc length of the text's middle `x_m` (the
//! middle of all its lines' extent, customer 2026-10-24). With `σ = +1` when the text reads in
//! carrier direction (Above with Baseline, Below with Top edge) and `-1` otherwise, the
//! line-space point `(x, y)` of a cluster with centre `x_c` lands at
//! `P + σ(x - x_c)τ - σ(y - ℓ)n + d·m`, where `(P, τ)` is the carrier point and unit tangent at
//! `position + σ(x_c - x_m)`, `n` its left normal (y down), `ℓ` the
//! touching line (0, or `-size` for the top of the H), `d` the Distance and `m` the normal towards
//! the letters (`n` Above, `-n` Below). Mirror letters reflects each line about the middle of its
//! extent first, so every letter is mirrored about its own axis and the order reverses.

use curvyo_document_core::{
    Affine, Angle, CarrierCurve, Length, LineOnPath, OnPathValues, PathSide, Point, TextOnPath,
    TextOutline, TextSnapshot, Vec2,
};
use curvyo_geometry_core::PathMeasure;
use curvyo_text_core::{Face, LineLayout, TextLayout};

/// Letters whose frame lands beyond this (mm) are left out (criterion 45).
const LIMIT_MM: f64 = 1e7;

/// The measure of a carrier curve.
#[must_use]
pub fn measure_of(curve: &CarrierCurve) -> PathMeasure {
    let anchors: Vec<(Point, Vec2, Vec2)> = curve
        .anchors
        .iter()
        .map(|a| (a.point, a.handle_in, a.handle_out))
        .collect();
    PathMeasure::new(&anchors, curve.closed)
}

/// A laid-out text placed on its carrier: the frame of every cluster.
#[derive(Debug, Clone)]
pub struct PathPlacement {
    measure: PathMeasure,
    values: OnPathValues,
    /// +1 when the text reads in carrier direction, else -1.
    sigma: f64,
    /// The touching line's line-space y offset (0 or `-size`).
    line_y: f64,
    /// Each line's extent, for Mirror letters.
    extents: Vec<(f64, f64)>,
    /// The line-space x of the text's middle: what Position places.
    middle: f64,
}

/// The line-space x of the middle of `laid`: the middle of the extent of all its lines (0 for
/// no line).
#[must_use]
pub(crate) fn text_middle(laid: &TextLayout) -> f64 {
    let (start, end) = text_extent(laid);
    f64::midpoint(start, end)
}

/// The line-space start and end of all lines of `laid` together.
fn text_extent(laid: &TextLayout) -> (f64, f64) {
    laid.lines
        .iter()
        .map(|line| (line.extent.start.0, line.extent.end.0))
        .reduce(|(a, b), (c, d)| (a.min(c), b.max(d)))
        .unwrap_or((0.0, 0.0))
}

/// Half the width of `laid` along its lines: the default Position on an open carrier, which
/// puts the text's start at the carrier's start (criterion 5).
#[must_use]
pub(crate) fn text_half_width(laid: &TextLayout) -> f64 {
    let (start, end) = text_extent(laid);
    (end - start) / 2.0
}

impl PathPlacement {
    /// The placement of `laid` (laid out horizontally, at the text's size) on `on_path`'s curve
    /// with its values; `None` when the text has no curve.
    #[must_use]
    pub fn new(laid: &TextLayout, on_path: &TextOnPath) -> Option<Self> {
        let curve = on_path.curve.as_ref()?;
        Some(Self::on_measure(measure_of(curve), laid, on_path.values))
    }

    /// The placement of `laid` on `measure` with `values`.
    #[must_use]
    pub fn on_measure(measure: PathMeasure, laid: &TextLayout, values: OnPathValues) -> Self {
        let sigma = if values.reads_with_carrier() {
            1.0
        } else {
            -1.0
        };
        let line_y = match values.line {
            LineOnPath::Baseline => 0.0,
            LineOnPath::TopEdge => -laid.size.0,
        };
        Self {
            measure,
            values,
            sigma,
            line_y,
            extents: laid
                .lines
                .iter()
                .map(|line| (line.extent.start.0, line.extent.end.0))
                .collect(),
            middle: text_middle(laid),
        }
    }

    /// The measured carrier.
    #[must_use]
    pub const fn measure(&self) -> &PathMeasure {
        &self.measure
    }

    /// The values the text sits with.
    #[must_use]
    pub const fn values(&self) -> &OnPathValues {
        &self.values
    }

    /// The line-space x of `x` on line `line` after Mirror letters.
    #[must_use]
    pub fn shown_x(&self, line: usize, x: f64) -> f64 {
        if self.values.mirror_letters {
            let (a, b) = self.extents.get(line).copied().unwrap_or((0.0, 0.0));
            a + b - x
        } else {
            x
        }
    }

    /// The map from line space to the document of a cluster of line `line` with centre `x_c`
    /// (before Mirror letters): Mirror letters, then the rigid frame at the cluster's arc length.
    /// `None` when the frame is not finite or lands beyond ±1e7 mm (criterion 45).
    #[must_use]
    pub fn frame(&self, line: usize, x_c: f64) -> Option<Affine> {
        let shown = self.shown_x(line, x_c);
        let arc = self
            .sigma
            .mul_add(shown - self.middle, self.values.position.as_mm());
        let sample = self.measure.at(Length::from_mm(arc));
        let (tangent, point) = (sample.tangent, sample.point);
        let normal = Vec2::new(tangent.y, -tangent.x);
        let towards = match self.values.side {
            PathSide::Above => normal,
            PathSide::Below => normal.scaled(-1.0),
        };
        let sigma = self.sigma;
        let gap = self.values.distance.as_mm();
        let offset_x =
            point.x - sigma * shown * tangent.x + sigma * self.line_y * normal.x + gap * towards.x;
        let offset_y =
            point.y - sigma * shown * tangent.y + sigma * self.line_y * normal.y + gap * towards.y;
        let frame = Affine::new(
            sigma * tangent.x,
            sigma * tangent.y,
            -sigma * normal.x,
            -sigma * normal.y,
            offset_x,
            offset_y,
        );
        let frame = if self.values.mirror_letters {
            let (start, end) = self.extents.get(line).copied().unwrap_or((0.0, 0.0));
            frame.compose(Affine::new(-1.0, 0.0, 0.0, 1.0, start + end, 0.0))
        } else {
            frame
        };
        (frame.is_within_limit() && offset_x.abs() <= LIMIT_MM && offset_y.abs() <= LIMIT_MM)
            .then_some(frame)
    }

    /// The document point of line-space `(x, y)` on line `line`, turned with the cluster whose
    /// centre is `x` (a caret at a boundary).
    #[must_use]
    pub fn point(&self, line: usize, x: f64, y: f64) -> Option<Point> {
        self.frame(line, x)
            .map(|frame| frame.apply_point(Point::new(x, y)))
    }
}

/// `text` laid out along its carrier: every glyph rigidly on the carrier by its cluster's frame,
/// a letter box per cluster (its advance by ascender to descender), the selection box around
/// them (criteria 8, 28). Letters whose frame is not finite or beyond ±1e7 mm are left out.
#[must_use]
pub fn lay_out_on_path(
    text: &TextSnapshot,
    face: &Face,
    laid: &TextLayout,
    placement: &PathPlacement,
    substitute: bool,
) -> TextOutline {
    let mut outlines = Vec::new();
    let mut glyph_outlines = Vec::new();
    let mut next_id = 0_u64;
    for (index, line) in laid.lines.iter().enumerate() {
        for glyph in &line.glyphs {
            let Some(frame) = placement.frame(index, glyph.centre_x.0) else {
                continue;
            };
            // Tapered and turned in the glyph's own frame before it is put on the carrier
            // (`0079` criterion 19); the frame is rigid, so the accuracy holds on the page.
            let Some(path) = laid.glyph_in_line(face, glyph, curvyo_text_core::TAPER_ACCURACY)
            else {
                continue;
            };
            let map = |p: kurbo::Point| frame.apply_point(Point::new(p.x, p.y));
            let contours = crate::text_layout::contours_of(&path, &map, &mut next_id);
            glyph_outlines.push(contours.len());
            outlines.extend(contours);
        }
    }
    let letter_boxes = letter_boxes(laid, placement);
    let corners: Vec<Point> = letter_boxes.iter().flatten().copied().collect();
    let line_box = bounding_corners(&corners).unwrap_or_else(|| {
        let p = placement
            .point(0, 0.0, 0.0)
            .unwrap_or(Point::new(text.placement.e, text.placement.f));
        [p; 4]
    });
    TextOutline {
        glyphs: crate::text_layout::glyph_path(text, outlines, Angle::from_radians(0.0)),
        line_box,
        glyph_outlines,
        letter_boxes,
        substitute,
    }
}

/// A cluster as it is drawn on the carrier: the map of its line-space points to the document
/// (its turn about its centre, then the frame of its centre) and its baseline with the vertical
/// kerns (criterion 47). Hit boxes, caret and selection go through it so they cover the letter.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Letter {
    /// Line space to the document.
    pub map: Affine,
    /// The line-space y of the cluster's baseline, raised by the kerns up to it.
    pub baseline: f64,
}

/// The cluster of line `index` (`line`) that starts at `byte` as drawn; `x0..x1` its caret
/// boundaries, for a cluster without a glyph. `None` off the carrier (criterion 45).
pub(crate) fn letter(
    laid: &TextLayout,
    placement: &PathPlacement,
    index: usize,
    line: &LineLayout,
    byte: usize,
    (x0, x1): (f64, f64),
) -> Option<Letter> {
    let Some(glyph) = line.glyphs.iter().find(|g| g.cluster == byte) else {
        return placement
            .frame(index, f64::midpoint(x0, x1))
            .map(|map| Letter {
                map,
                baseline: line.baseline.0,
            });
    };
    let frame = placement.frame(index, glyph.centre_x.0)?;
    let turn = Affine::rotation_about(
        Point::new(glyph.centre_x.0, glyph.centre_y.0),
        Angle::from_radians(laid.rotation_rad + glyph.rotation_rad),
    );
    Some(Letter {
        map: frame.compose(turn),
        // A horizontal glyph turns about half the size above its (raised) baseline.
        baseline: glyph.centre_y.0 + laid.size.0 / 2.0,
    })
}

/// One box per cluster of every line: from its caret boundary to the next, ascender to
/// descender of its raised baseline, turned and placed as the letter is drawn.
fn letter_boxes(laid: &TextLayout, placement: &PathPlacement) -> Vec<[Point; 4]> {
    let mut boxes = Vec::new();
    for (index, line) in laid.lines.iter().enumerate() {
        for pair in line.boundaries.windows(2) {
            let ((byte, x0), (_, x1)) = (pair[0], pair[1]);
            let Some(letter) = letter(laid, placement, index, line, byte, (x0.0, x1.0)) else {
                continue;
            };
            let top = letter.baseline - laid.ascender.0;
            let bottom = letter.baseline + laid.descender.0;
            boxes.push(
                [(x0.0, top), (x1.0, top), (x1.0, bottom), (x0.0, bottom)]
                    .map(|(x, y)| letter.map.apply_point(Point::new(x, y))),
            );
        }
    }
    boxes
}

/// The axis-parallel box of `points` as four corners, or `None` for no point.
fn bounding_corners(points: &[Point]) -> Option<[Point; 4]> {
    let first = points.first()?;
    let (mut min, mut max) = (*first, *first);
    for p in points {
        min = Point::new(min.x.min(p.x), min.y.min(p.y));
        max = Point::new(max.x.max(p.x), max.y.max(p.y));
    }
    Some([min, Point::new(max.x, min.y), max, Point::new(min.x, max.y)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{AnchorKind, CarrierRef, OutlineAnchor};
    use curvyo_text_core::{LayoutOptions, Mm, layout};

    fn sans() -> std::sync::Arc<curvyo_text_core::Face> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf");
        let mut store = curvyo_text_core::FaceStore::new();
        let key = store.add(std::fs::read(path).unwrap(), 0).unwrap();
        store.get(&key).unwrap().clone()
    }

    fn line_curve() -> CarrierCurve {
        let corner = |x: f64| OutlineAnchor {
            point: Point::new(x, 0.0),
            handle_in: Vec2::ZERO,
            handle_out: Vec2::ZERO,
            kind: AnchorKind::Corner,
        };
        CarrierCurve {
            closed: false,
            anchors: vec![corner(0.0), corner(100.0)],
            ellipse: None,
        }
    }

    /// Criterion 9's frame: on a straight carrier from (0, 0) to the right, Above with Baseline
    /// at half the text's width (its middle there) is the identity.
    #[test]
    fn a_straight_carrier_is_the_identity() {
        let face = sans();
        let laid = layout(&face, "AB", Mm(10.0), &LayoutOptions::default()).unwrap();
        let on_path = TextOnPath {
            carrier: CarrierRef {
                node: curvyo_document_core::UNCREATED_TEXT_ID,
                outline: 0,
            },
            values: OnPathValues::at(Length::from_mm(text_half_width(&laid))),
            curve: Some(std::sync::Arc::new(line_curve())),
            kept: false,
            kept_curve: None,
        };
        let placement = PathPlacement::new(&laid, &on_path).unwrap();
        let frame = placement.frame(0, 3.0).unwrap();
        let p = frame.apply_point(Point::new(7.0, -4.0));
        assert!(
            (p.x - 7.0).abs() < 1e-6 && (p.y + 4.0).abs() < 1e-6,
            "{p:?}"
        );
    }
}
