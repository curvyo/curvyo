//! Small stick text with umlauts or ß for the reference set (`specs/0089-trace-quality-and-colour`
//! criterion 2): glyphs on a grid, drawn as round strokes with dots.

use std::sync::Arc;

use super::metrics::to_segment;
use super::reference::{Built, Source};
use super::render::{Layer, Scene, Shape};
use super::scenes::{BLACK, WHITE};

type Strokes = Vec<((f64, f64), (f64, f64))>;

/// Whether `q` is within `radius` of one of `strokes` (bounding boxes first).
fn near(q: (f64, f64), strokes: &[((f64, f64), (f64, f64))], radius: f64) -> bool {
    strokes.iter().any(|&(a, b)| {
        q.0 >= a.0.min(b.0) - radius
            && q.0 <= a.0.max(b.0) + radius
            && q.1 >= a.1.min(b.1) - radius
            && q.1 <= a.1.max(b.1) + radius
            && to_segment(q, a, b) <= radius
    })
}

/// Stick glyphs on a grid of 4 x 6 units (lower case from 2 to 6); `Some(dots)` marks an umlaut.
fn glyph(c: char) -> (&'static [((f64, f64), (f64, f64))], bool) {
    const A: &[((f64, f64), (f64, f64))] = &[
        ((0., 6.), (2., 0.)),
        ((2., 0.), (4., 6.)),
        ((1., 3.), (3., 3.)),
    ];
    const O: &[((f64, f64), (f64, f64))] = &[
        ((0., 0.), (4., 0.)),
        ((4., 0.), (4., 6.)),
        ((4., 6.), (0., 6.)),
        ((0., 6.), (0., 0.)),
    ];
    const U: &[((f64, f64), (f64, f64))] = &[
        ((0., 0.), (0., 6.)),
        ((0., 6.), (4., 6.)),
        ((4., 6.), (4., 0.)),
    ];
    const SZ: &[((f64, f64), (f64, f64))] = &[
        ((0., 6.), (0., 1.)),
        ((0., 1.), (1.5, 0.)),
        ((1.5, 0.), (3., 1.)),
        ((3., 1.), (1.5, 3.)),
        ((1.5, 3.), (3.5, 4.5)),
        ((3.5, 4.5), (2., 6.)),
        ((2., 6.), (1., 6.)),
    ];
    const LO: &[((f64, f64), (f64, f64))] = &[
        ((0., 2.), (3., 2.)),
        ((3., 2.), (3., 6.)),
        ((3., 6.), (0., 6.)),
        ((0., 6.), (0., 2.)),
    ];
    const LA: &[((f64, f64), (f64, f64))] = &[
        ((0., 2.), (3., 2.)),
        ((3., 2.), (3., 6.)),
        ((3., 6.), (0., 6.)),
        ((0., 6.), (0., 4.)),
        ((0., 4.), (3., 4.)),
    ];
    const LU: &[((f64, f64), (f64, f64))] = &[
        ((0., 2.), (0., 6.)),
        ((0., 6.), (3., 6.)),
        ((3., 6.), (3., 2.)),
    ];
    const G: &[((f64, f64), (f64, f64))] = &[
        ((4., 0.), (0., 0.)),
        ((0., 0.), (0., 6.)),
        ((0., 6.), (4., 6.)),
        ((4., 6.), (4., 3.)),
        ((4., 3.), (2., 3.)),
    ];
    const R: &[((f64, f64), (f64, f64))] = &[
        ((0., 6.), (0., 0.)),
        ((0., 0.), (4., 0.)),
        ((4., 0.), (4., 3.)),
        ((4., 3.), (0., 3.)),
        ((1., 3.), (4., 6.)),
    ];
    const E: &[((f64, f64), (f64, f64))] = &[
        ((4., 0.), (0., 0.)),
        ((0., 0.), (0., 6.)),
        ((0., 6.), (4., 6.)),
        ((0., 3.), (3., 3.)),
    ];
    match c {
        'A' => (A, false),
        'Ä' => (A, true),
        'O' => (O, false),
        'Ö' => (O, true),
        'U' => (U, false),
        'Ü' => (U, true),
        'ß' => (SZ, false),
        'ä' => (LA, true),
        'ö' => (LO, true),
        'ü' => (LU, true),
        'G' => (G, false),
        'R' => (R, false),
        'E' => (E, false),
        _ => (&[], false),
    }
}

/// The strokes and umlaut dots of `text` at `(x, y)` with `unit` px per grid unit.
fn text(text: &str, x: f64, y: f64, unit: f64) -> (Strokes, Vec<(f64, f64)>) {
    let (mut strokes, mut dots) = (Vec::new(), Vec::new());
    for (i, c) in text.chars().enumerate() {
        let left = x + i as f64 * 6.0 * unit;
        let (lines, umlaut) = glyph(c);
        let top = if c.is_lowercase() { 2.0 } else { 0.0 };
        for &((ax, ay), (bx, by)) in lines {
            strokes.push((
                (left + ax * unit, y + ay * unit),
                (left + bx * unit, y + by * unit),
            ));
        }
        if umlaut {
            for dx in [1.0, 3.0] {
                dots.push((left + dx * unit, y + (top - 1.6) * unit));
            }
        }
    }
    (strokes, dots)
}

pub fn text_umlauts() -> Built {
    let mut parts = Vec::new();
    for &(line, y, unit) in &[
        ("ÄÖÜ ß äöü", 20.0, 3.0),
        ("GRÜßE ÄRGER", 70.0, 1.6),
        ("äöü ÄÖÜ ß", 100.0, 1.2),
    ] {
        let (strokes, dots) = text(line, 12.0, y, unit);
        parts.push((strokes, dots, unit));
    }
    let inside = Arc::new(move |x: f64, y: f64| {
        parts.iter().any(|(strokes, dots, unit)| {
            let r = 0.35 * unit;
            near((x, y), strokes, r)
                || dots
                    .iter()
                    .any(|&(dx, dy)| (x - dx).hypot(y - dy) <= 0.55 * unit)
        })
    });
    Built {
        source: Source::Scene(Scene {
            width: 340.0,
            height: 120.0,
            background: WHITE,
            layers: vec![Layer::ink(Shape::Implicit(inside), BLACK)],
        }),
        artefacts: Vec::new(),
        corners: Vec::new(),
        edges: Vec::new(),
    }
}
