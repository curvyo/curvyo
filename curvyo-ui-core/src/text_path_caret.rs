//! The caret, the selected range and the click of a text in edit mode along its carrier
//! (`specs/0080-text-on-path` criterion 27, UX notes §4): each piece is turned with the letter it
//! stands at, through the frame of `text_on_path`.

use std::ops::Range;

use curvyo_document_core::{Point, Vec2};
use curvyo_text_core::{LineLayout, TextLayout};

use crate::text_caret::TextEditGeometry;
use crate::text_on_path::PathPlacement;

/// The x of caret position `byte` on `line`.
fn x_at(line: &LineLayout, byte: usize) -> f64 {
    line.boundaries
        .iter()
        .rev()
        .find(|(at, _)| *at <= byte)
        .map_or(0.0, |(_, x)| x.0)
}

/// The clusters of `line`: (start byte, end byte, x0, x1, centre).
fn clusters(line: &LineLayout) -> impl Iterator<Item = (usize, usize, f64, f64, f64)> + '_ {
    line.boundaries.windows(2).map(|pair| {
        let ((start, x0), (end, x1)) = (pair[0], pair[1]);
        let centre = line
            .glyphs
            .iter()
            .find(|g| g.cluster == start)
            .map_or(f64::midpoint(x0.0, x1.0), |g| g.centre_x.0);
        (start, end, x0.0, x1.0, centre)
    })
}

/// The edit geometry of `layout` (the shown string) on its carrier: the caret at `caret` across
/// the letter it stands at, one turned quad per selected letter, the composition underline per
/// letter, and the axis-parallel box of the letters as the line box.
#[must_use]
pub fn edit_geometry_on_path(
    layout: &TextLayout,
    placement: &PathPlacement,
    caret: usize,
    selection: &Range<usize>,
    preedit: Option<&Range<usize>>,
) -> TextEditGeometry {
    let (ascender, descender) = (layout.ascender.0, layout.descender.0);
    let mut geometry = TextEditGeometry {
        line_box: [Point::new(0.0, 0.0); 4],
        caret: None,
        selection: Vec::new(),
        preedit: Vec::new(),
    };
    let mut corners = Vec::new();
    for (index, line) in layout.lines.iter().enumerate() {
        let baseline = line.baseline.0;
        for (start, end, x0, x1, centre) in clusters(line) {
            let Some(frame) = placement.frame(index, centre) else {
                continue;
            };
            let map = |x: f64, y: f64| frame.apply_point(Point::new(x, y));
            let quad = [
                map(x0, baseline - ascender),
                map(x1, baseline - ascender),
                map(x1, baseline + descender),
                map(x0, baseline + descender),
            ];
            corners.extend(quad);
            if start >= selection.start && end <= selection.end && selection.start < selection.end {
                geometry.selection.push(quad);
            }
            if let Some(range) = preedit
                && start >= range.start
                && end <= range.end
            {
                geometry
                    .preedit
                    .push([map(x0, baseline), map(x1, baseline)]);
            }
        }
    }
    if selection.is_empty() {
        geometry.caret = caret_on_path(layout, placement, caret);
    }
    geometry.line_box = letters_box(&corners, placement);
    geometry
}

/// The caret at `caret` across the letter it stands at, or `None` off the carrier.
fn caret_on_path(
    layout: &TextLayout,
    placement: &PathPlacement,
    caret: usize,
) -> Option<[Point; 2]> {
    let (ascender, descender) = (layout.ascender.0, layout.descender.0);
    let index = layout.line_of(caret);
    let line = layout.lines.get(index)?;
    let x = x_at(line, caret);
    // The caret turns with the letter after it, or with the last one at the line's end.
    let centre = clusters(line)
        .find(|c| c.0 >= caret)
        .or_else(|| clusters(line).last())
        .map_or(x, |c| c.4);
    let frame = placement.frame(index, centre)?;
    let baseline = line.baseline.0;
    Some([
        frame.apply_point(Point::new(x, baseline - ascender)),
        frame.apply_point(Point::new(x, baseline + descender)),
    ])
}

/// The axis-parallel box of the letters' `corners`, or the start of the carrier without letters.
fn letters_box(corners: &[Point], placement: &PathPlacement) -> [Point; 4] {
    if let Some(first) = corners.first() {
        let (mut min, mut max) = (*first, *first);
        for p in corners {
            min = Point::new(min.x.min(p.x), min.y.min(p.y));
            max = Point::new(max.x.max(p.x), max.y.max(p.y));
        }
        [min, Point::new(max.x, min.y), max, Point::new(min.x, max.y)]
    } else {
        placement
            .point(0, 0.0, 0.0)
            .map_or([Point::new(0.0, 0.0); 4], |p| [p; 4])
    }
}

/// The caret position nearest to the document point `point` on a text laid out along its carrier
/// (criterion 27): the letter whose turned box is nearest, then its nearer boundary.
#[must_use]
pub fn nearest_boundary_on_path(
    layout: &TextLayout,
    placement: &PathPlacement,
    point: Point,
) -> usize {
    let (ascender, descender) = (layout.ascender.0, layout.descender.0);
    let mut best = (f64::INFINITY, 0_usize);
    for (index, line) in layout.lines.iter().enumerate() {
        let (top, bottom) = (line.baseline.0 - ascender, line.baseline.0 + descender);
        for (start, end, x0, x1, centre) in clusters(line) {
            let Some(inverse) = placement
                .frame(index, centre)
                .and_then(curvyo_document_core::Affine::inverse)
            else {
                continue;
            };
            let q = inverse.apply_point(point);
            let outside = Vec2::new(
                (x0 - q.x).max(q.x - x1).max(0.0),
                (top - q.y).max(q.y - bottom).max(0.0),
            )
            .length();
            if outside < best.0 {
                let byte = if (q.x - x0).abs() <= (q.x - x1).abs() {
                    start
                } else {
                    end
                };
                best = (outside, byte);
            }
        }
        if line.boundaries.len() < 2 && best.0.is_infinite() {
            best = (f64::MAX, line.range.start);
        }
    }
    best.1
}
