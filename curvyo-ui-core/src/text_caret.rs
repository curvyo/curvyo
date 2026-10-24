//! Where the caret, the selected range and the IME composition of a text in edit mode are drawn
//! (`specs/0074-text-objects` criterion 3; UX notes §2, "Look in edit mode"), in document mm,
//! turned and skewed with the text's placement; across the column in vertical text
//! (`specs/0079-text-layout-controls` criterion 24).

use std::ops::Range;

use curvyo_document_core::{Affine, Point};
use curvyo_text_core::TextLayout;

/// A selected line break is shown as a band this share of the em wide (UX notes §2).
const LINE_BREAK_BAND_EM: f64 = 0.3;

/// The geometry of a text in edit mode.
#[derive(Debug, Clone, PartialEq)]
pub struct TextEditGeometry {
    /// The line box corners: top-left, top-right, bottom-right, bottom-left.
    pub line_box: [Point; 4],
    /// The caret from the ascender to the descender, when there is no selected range.
    pub caret: Option<[Point; 2]>,
    /// One quad per line of the selected range.
    pub selection: Vec<[Point; 4]>,
    /// The underline of the composition, per line.
    pub preedit: Vec<[Point; 2]>,
}

fn x_at(layout: &TextLayout, line: usize, byte: usize) -> f64 {
    layout.lines.get(line).map_or(0.0, |l| {
        l.boundaries
            .iter()
            .rev()
            .find(|(at, _)| *at <= byte)
            .map_or(0.0, |(_, x)| x.0)
    })
}

/// The edit geometry of `layout` (the shown string, composition included) under `placement`:
/// the caret at byte `caret`, the quads of `selection`, the underline of `preedit`.
#[must_use]
pub fn edit_geometry(
    layout: &TextLayout,
    placement: Affine,
    caret: usize,
    selection: &Range<usize>,
    preedit: Option<&Range<usize>>,
) -> TextEditGeometry {
    // Caret and ranges are found in line space; the frame turns them into a column's.
    let frame = layout.frame();
    let map = |x: f64, y: f64| {
        let p = frame * kurbo::Point::new(x, y);
        placement.apply_point(Point::new(p.x, p.y))
    };
    let corner = |x: f64, y: f64| placement.apply_point(Point::new(x, y));
    let (ascender, descender) = (layout.ascender.0, layout.descender.0);
    let line_box = layout.line_box();
    let mut geometry = TextEditGeometry {
        line_box: [
            corner(line_box.x0, line_box.y0),
            corner(line_box.x1, line_box.y0),
            corner(line_box.x1, line_box.y1),
            corner(line_box.x0, line_box.y1),
        ],
        caret: None,
        selection: Vec::new(),
        preedit: Vec::new(),
    };
    if selection.is_empty() {
        let line = layout.line_of(caret);
        let x = x_at(layout, line, caret);
        let baseline = layout.lines.get(line).map_or(0.0, |l| l.baseline.0);
        geometry.caret = Some([map(x, baseline - ascender), map(x, baseline + descender)]);
    }
    let band = LINE_BREAK_BAND_EM * layout.mm_per_em;
    for (index, line) in layout.lines.iter().enumerate() {
        let baseline = line.baseline.0;
        let span = |range: &Range<usize>| -> Option<(f64, f64, bool)> {
            let start = range.start.max(line.range.start);
            let end = range.end.min(line.range.end);
            let breaks = range.end > line.range.end && range.start <= line.range.end;
            (start < end || breaks)
                .then(|| (x_at(layout, index, start), x_at(layout, index, end), breaks))
        };
        if let Some((x0, x1, breaks)) = span(selection) {
            let x1 = if breaks { x1 + band } else { x1 };
            geometry.selection.push([
                map(x0, baseline - ascender),
                map(x1, baseline - ascender),
                map(x1, baseline + descender),
                map(x0, baseline + descender),
            ]);
        }
        if let Some(range) = preedit
            && let Some((x0, x1, _)) = span(range)
        {
            geometry
                .preedit
                .push([map(x0, baseline), map(x1, baseline)]);
        }
    }
    geometry
}
