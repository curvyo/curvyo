//! The glyph outlines a code's human-readable text is set with (ADR 0092 §3): one table per face,
//! built from the text engine, and the per-thread registry the generation funnel reads, as it
//! reads the active extension settings. A table covers U+0020 to U+007E, one unit is the face's
//! cap height (a text's Size), winding normalised by nesting.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use curvyo_document_core::{FontRef, Length, Point};
use curvyo_extension_core::{ClosedPath, Cubic, Glyph, GlyphTable};
use curvyo_text_core::{Face, glyph_cell};
use kurbo::{BezPath, PathEl};

/// The tables the generation funnel can use right now, by the face a code's `Font` parameter names.
#[derive(Debug, Clone, Default)]
pub struct GlyphRegistry {
    /// The table of the included sans: what a `None` font and a face missing on this computer use.
    pub default: Option<Arc<GlyphTable>>,
    /// The table of a named face, by `(face hash, face index)`; a missing face maps to the table of
    /// the face that stands in for it.
    pub faces: HashMap<(String, u32), Arc<GlyphTable>>,
}

thread_local! {
    static ACTIVE: RefCell<GlyphRegistry> = RefCell::new(GlyphRegistry::default());
}

/// Makes `registry` the one the generation funnel of this thread reads (the session sets it before
/// every write that regenerates a code).
pub fn set_active_glyphs(registry: GlyphRegistry) {
    ACTIVE.with(|active| *active.borrow_mut() = registry);
}

/// The table for `font`: the named face's, else the included sans'; `None` while neither is loaded.
#[must_use]
pub fn glyphs_for(font: Option<&FontRef>) -> Option<Arc<GlyphTable>> {
    ACTIVE.with(|active| {
        let active = active.borrow();
        font.and_then(|font| {
            active
                .faces
                .get(&(font.face_hash.clone(), font.face_index))
                .cloned()
        })
        .or_else(|| active.default.clone())
    })
}

const FLATTEN_STEPS: usize = 8;

/// `contour` closed explicitly and kept when it has at least two segments.
fn close_contour(mut contour: ClosedPath, out: &mut Vec<ClosedPath>) {
    let end = contour.segments.last().map_or(contour.start, |s| s.to);
    if (end.x - contour.start.x).abs() > 1e-12 || (end.y - contour.start.y).abs() > 1e-12 {
        contour.segments.push(Cubic {
            c1: end,
            c2: contour.start,
            to: contour.start,
        });
    }
    if contour.segments.len() >= 2 {
        out.push(contour);
    }
}

/// The cubic of a path element that draws, from `at`: a line, a quadratic raised exactly (the text
/// engine never emits one) or a cubic; `None` for a move or a close.
fn segment_of(element: PathEl, at: Point, map: &impl Fn(kurbo::Point) -> Point) -> Option<Cubic> {
    match element {
        PathEl::LineTo(p) => {
            let to = map(p);
            Some(Cubic { c1: at, c2: to, to })
        }
        PathEl::CurveTo(c1, c2, p) => Some(Cubic {
            c1: map(c1),
            c2: map(c2),
            to: map(p),
        }),
        PathEl::QuadTo(c, p) => {
            let (c, to) = (map(c), map(p));
            let third = |a: f64, b: f64| a + 2.0 / 3.0 * (b - a);
            Some(Cubic {
                c1: Point::new(third(at.x, c.x), third(at.y, c.y)),
                c2: Point::new(third(to.x, c.x), third(to.y, c.y)),
                to,
            })
        }
        PathEl::MoveTo(_) | PathEl::ClosePath => None,
    }
}

/// The closed contours of a glyph path, scaled.
fn contours(path: &BezPath, scale: f64) -> Vec<ClosedPath> {
    let map = |p: kurbo::Point| Point::new(p.x * scale, p.y * scale);
    let mut out: Vec<ClosedPath> = Vec::new();
    let mut current: Option<ClosedPath> = None;
    let mut at = Point::new(0.0, 0.0);
    for element in path.elements() {
        if let PathEl::MoveTo(p) = *element {
            if let Some(done) = current.take() {
                close_contour(done, &mut out);
            }
            at = map(p);
            current = Some(ClosedPath {
                start: at,
                segments: Vec::new(),
            });
        } else if let Some(segment) = segment_of(*element, at, &map) {
            if let Some(contour) = current.as_mut() {
                at = segment.to;
                contour.segments.push(segment);
            }
        } else if let Some(done) = current.take() {
            close_contour(done, &mut out);
        }
    }
    if let Some(done) = current.take() {
        close_contour(done, &mut out);
    }
    out
}

/// The contour as a polygon, its cubics cut into pieces.
fn polygon(path: &ClosedPath) -> Vec<(f64, f64)> {
    let mut points = vec![(path.start.x, path.start.y)];
    let mut from = path.start;
    for cubic in &path.segments {
        for step in 1..=FLATTEN_STEPS {
            #[allow(clippy::cast_precision_loss)]
            let t = step as f64 / FLATTEN_STEPS as f64;
            let u = 1.0 - t;
            let at = |a: f64, b: f64, c: f64, d: f64| {
                u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d
            };
            points.push((
                at(from.x, cubic.c1.x, cubic.c2.x, cubic.to.x),
                at(from.y, cubic.c1.y, cubic.c2.y, cubic.to.y),
            ));
        }
        from = cubic.to;
    }
    points
}

/// Twice the signed area: positive for a contour that runs clockwise on the screen (y down).
fn area(polygon: &[(f64, f64)]) -> f64 {
    polygon
        .windows(2)
        .map(|pair| pair[0].0 * pair[1].1 - pair[1].0 * pair[0].1)
        .sum()
}

fn contains(polygon: &[(f64, f64)], (x, y): (f64, f64)) -> bool {
    let mut inside = false;
    for pair in polygon.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        if (y0 > y) != (y1 > y) && x < x0 + (y - y0) / (y1 - y0) * (x1 - x0) {
            inside = !inside;
        }
    }
    inside
}

/// `path` run the other way round from the same start.
fn reversed(path: &ClosedPath) -> ClosedPath {
    let mut points = vec![path.start];
    points.extend(path.segments.iter().map(|s| s.to));
    let n = path.segments.len();
    ClosedPath {
        start: path.start,
        segments: (0..n)
            .map(|j| {
                let source = &path.segments[n - 1 - j];
                Cubic {
                    c1: source.c2,
                    c2: source.c1,
                    to: points[n - 1 - j],
                }
            })
            .collect(),
    }
}

/// The contours with the winding of ink: those at an even depth of nesting clockwise on the
/// screen, those at an odd depth the other way (TrueType and CFF wind oppositely).
fn normalised(contours: Vec<ClosedPath>) -> Vec<ClosedPath> {
    let polygons: Vec<Vec<(f64, f64)>> = contours.iter().map(polygon).collect();
    contours
        .into_iter()
        .enumerate()
        .map(|(at, contour)| {
            // A point on the contour that is not a vertex shared with another contour.
            let probe = polygons[at][FLATTEN_STEPS / 2];
            let depth = polygons
                .iter()
                .enumerate()
                .filter(|(other, polygon)| *other != at && contains(polygon, probe))
                .count();
            let clockwise = area(&polygons[at]) > 1e-12;
            if (depth % 2 == 0) == clockwise {
                contour
            } else {
                reversed(&contour)
            }
        })
        .collect()
}

/// The glyph table of `face` for U+0020 to U+007E: a character the face lacks is absent.
#[must_use]
pub fn glyph_table(face: &Face) -> GlyphTable {
    let cap = face.metrics().cap_height;
    let mut table = GlyphTable::default();
    for ch in ' '..='~' {
        let Some(cell) = glyph_cell(face, ch) else {
            continue;
        };
        let outline = face.outline(face.glyph_for(ch));
        table.glyphs.insert(
            ch,
            Glyph {
                paths: normalised(contours(&outline.path, 1.0 / cap)),
                advance: Length::from_mm(cell.advance),
            },
        );
    }
    table
}
