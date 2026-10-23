//! Writes a string laid out in a face as SVG path data: the name of a Fonts-tab row drawn in a
//! single-line face's own centre lines (`specs/0076-single-line-fonts` criterion 6, `adrs.md`
//! decision 6), since browsers draw no SVG fonts.

use std::fmt::Write as _;

use kurbo::{PathEl, Rect};

use crate::face::Face;
use crate::layout::layout;
use crate::units::Mm;

/// `text` in `face` at 1 em as path data (em units, y down, origin on the first baseline) and
/// its line box; `None` beyond the engine's bound.
#[must_use]
pub fn preview_path(face: &Face, text: &str) -> Option<(String, Rect)> {
    let laid = layout(face, text, Mm(face.metrics().cap_height)).ok()?;
    let mut d = String::new();
    for path in laid.glyph_paths(face) {
        for element in path.elements() {
            let _ = match *element {
                PathEl::MoveTo(p) => write!(d, "M{:.4} {:.4}", p.x, p.y),
                PathEl::LineTo(p) => write!(d, "L{:.4} {:.4}", p.x, p.y),
                PathEl::QuadTo(a, p) => write!(d, "Q{:.4} {:.4} {:.4} {:.4}", a.x, a.y, p.x, p.y),
                PathEl::CurveTo(a, b, p) => write!(
                    d,
                    "C{:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
                    a.x, a.y, b.x, b.y, p.x, p.y
                ),
                PathEl::ClosePath => write!(d, "Z"),
            };
        }
    }
    Some((d, laid.line_box()))
}
