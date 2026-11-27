//! A test glyph table (ADR 0092 §3.5): every printable ASCII character a box with a hole, 0.6 mm
//! advance and 0.7 mm tall at a size of 1 mm, the space empty. Outers are clockwise, the hole the
//! other way, so the fill rule is exercised without a font.
#![allow(dead_code, clippy::unwrap_used)]

use curvyo_document_core::{Length, Point};
use curvyo_extension_core::{ClosedPath, Cubic, Glyph, GlyphTable};

fn line(from: Point, to: Point) -> Cubic {
    Cubic {
        c1: from,
        c2: to,
        to,
    }
}

fn square(x0: f64, y0: f64, x1: f64, y1: f64, clockwise: bool) -> ClosedPath {
    let mut corners = [
        Point::new(x0, y0),
        Point::new(x1, y0),
        Point::new(x1, y1),
        Point::new(x0, y1),
    ];
    if !clockwise {
        corners.swap(1, 3);
    }
    ClosedPath {
        start: corners[0],
        segments: (0..4)
            .map(|at| line(corners[at], corners[(at + 1) % 4]))
            .collect(),
    }
}

pub fn test_table() -> GlyphTable {
    let mut table = GlyphTable::default();
    for ch in ' '..='~' {
        let paths = if ch == ' ' {
            Vec::new()
        } else {
            vec![
                square(0.05, -0.7, 0.55, 0.0, true),
                square(0.2, -0.5, 0.4, -0.2, false),
            ]
        };
        table.glyphs.insert(
            ch,
            Glyph {
                paths,
                advance: Length::from_mm(0.6),
            },
        );
    }
    table
}
