//! Where the caret, the selected range and the IME composition of a text in edit mode are drawn
//! (`specs/0074-text-objects` criterion 3; UX notes §2, "Look in edit mode"), in document mm,
//! turned and skewed with the text's placement; across the column in vertical text
//! (`specs/0079-text-layout-controls` criterion 24).

use std::ops::Range;

use curvyo_document_core::{Affine, Point};
use curvyo_text_core::{LineLayout, TextLayout};
use kurbo::Rect;

/// A selected line break is shown as a band this share of the em wide (UX notes §2).
const LINE_BREAK_BAND_EM: f64 = 0.3;

/// Two letters whose maps and baselines differ by less than this (mm) are drawn in one place.
const SAME_PLACE: f64 = 1e-9;

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

/// One cluster of a line as it is drawn (ADR 0021 §6): its bytes, its caret boundaries in line
/// space, the map of its line space to text space (turned about the cluster's centre) and its
/// baseline, raised by the vertical kerns up to it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Letter {
    bytes: (usize, usize),
    x: (f64, f64),
    map: kurbo::Affine,
    baseline: f64,
}

impl Letter {
    /// The letter's box, ascender to descender of its baseline, in text space.
    fn corners(&self, layout: &TextLayout) -> [kurbo::Point; 4] {
        let (top, bottom) = (
            self.baseline - layout.ascender.0,
            self.baseline + layout.descender.0,
        );
        let (x0, x1) = self.x;
        [(x0, top), (x1, top), (x1, bottom), (x0, bottom)]
            .map(|(x, y)| self.map * kurbo::Point::new(x, y))
    }

    /// Whether `other` is drawn in the same place as this one, so one quad covers both.
    fn same_place(&self, other: &Self) -> bool {
        let near = |a: f64, b: f64| (a - b).abs() <= SAME_PLACE;
        near(self.baseline, other.baseline)
            && self
                .map
                .as_coeffs()
                .iter()
                .zip(other.map.as_coeffs())
                .all(|(a, b)| near(*a, b))
    }
}

/// The letters of `line` in reading order.
fn letters(layout: &TextLayout, line: &LineLayout) -> Vec<Letter> {
    let frame = layout.frame();
    line.boundaries
        .windows(2)
        .map(|pair| {
            let ((start, x0), (end, x1)) = (pair[0], pair[1]);
            let (map, baseline) = line.glyphs.iter().find(|g| g.cluster == start).map_or(
                (frame, line.baseline.0),
                |glyph| {
                    let turn = layout.rotation_rad + glyph.rotation_rad;
                    let centre = kurbo::Point::new(glyph.centre_x.0, glyph.centre_y.0);
                    (
                        frame * kurbo::Affine::rotate_about(turn, centre),
                        glyph.baseline.0,
                    )
                },
            );
            Letter {
                bytes: (start, end),
                x: (x0.0, x1.0),
                map,
                baseline,
            }
        })
        .collect()
}

/// The box of `layout`'s letters as drawn, in text space: each letter's box turned and raised as
/// it is drawn; a line without letters gives its empty box at its start.
pub(crate) fn letters_box(layout: &TextLayout) -> Rect {
    let mut points = Vec::new();
    for line in &layout.lines {
        let letters = letters(layout, line);
        if letters.is_empty() {
            let (top, bottom) = (
                line.baseline.0 - layout.ascender.0,
                line.baseline.0 + layout.descender.0,
            );
            let frame = layout.frame();
            points.push(frame * kurbo::Point::new(line.start.0, top));
            points.push(frame * kurbo::Point::new(line.start.0 + line.width.0, bottom));
        }
        for letter in &letters {
            points.extend(letter.corners(layout));
        }
    }
    points.iter().skip(1).fold(
        points
            .first()
            .map_or(Rect::ZERO, |p| Rect::from_points(*p, *p)),
        |r, p| r.union_pt(*p),
    )
}

/// The caret position nearest to `point` (text space): the letter whose box as drawn is
/// nearest, then its nearer boundary (`0074` criterion 10, `0079` criteria 26-29).
#[must_use]
pub fn nearest_boundary(layout: &TextLayout, point: Point) -> usize {
    let point = kurbo::Point::new(point.x, point.y);
    let (ascender, descender) = (layout.ascender.0, layout.descender.0);
    let mut best = (f64::INFINITY, 0_usize);
    let mut consider = |letter: &Letter| {
        let q = letter.map.inverse() * point;
        let (x0, x1) = letter.x;
        let (top, bottom) = (letter.baseline - ascender, letter.baseline + descender);
        let outside = kurbo::Vec2::new(
            (x0 - q.x).max(q.x - x1).max(0.0),
            (top - q.y).max(q.y - bottom).max(0.0),
        )
        .hypot();
        if outside < best.0 {
            let byte = if (q.x - x0).abs() <= (q.x - x1).abs() {
                letter.bytes.0
            } else {
                letter.bytes.1
            };
            best = (outside, byte);
        }
    };
    for line in &layout.lines {
        let letters = letters(layout, line);
        if letters.is_empty() {
            consider(&Letter {
                bytes: (line.range.start, line.range.start),
                x: (line.start.0, line.start.0),
                map: layout.frame(),
                baseline: line.baseline.0,
            });
        }
        letters.iter().for_each(&mut consider);
    }
    best.1
}

/// The quads over `pieces` (letters, in reading order), one per run drawn in the same place:
/// the box from `top` to `bottom` below each run's baseline.
fn runs(pieces: &[Letter], top: f64, bottom: f64) -> Vec<[kurbo::Point; 4]> {
    let mut quads = Vec::new();
    let mut at = 0;
    while let Some(first) = pieces.get(at) {
        let mut end = at + 1;
        while pieces.get(end).is_some_and(|next| first.same_place(next)) {
            end += 1;
        }
        let (x0, x1) = (first.x.0, pieces[end - 1].x.1);
        let (y0, y1) = (first.baseline - top, first.baseline + bottom);
        quads.push(
            [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
                .map(|(x, y)| first.map * kurbo::Point::new(x, y)),
        );
        at = end;
    }
    quads
}

/// The letters of `line` inside `range`, with the line break as a band after the last when the
/// range holds it.
fn pieces_in(
    letters: &[Letter],
    line: &LineLayout,
    layout: &TextLayout,
    range: &Range<usize>,
    band: f64,
) -> Vec<Letter> {
    let mut pieces: Vec<Letter> = letters
        .iter()
        .filter(|l| l.bytes.0 >= range.start && l.bytes.1 <= range.end && l.bytes.0 < l.bytes.1)
        .copied()
        .collect();
    if band > 0.0 && range.end > line.range.end && range.start <= line.range.end {
        let end = line.boundaries.last().map_or(line.start.0, |(_, x)| x.0);
        let (map, baseline) = letters
            .last()
            .map_or((layout.frame(), line.baseline.0), |l| (l.map, l.baseline));
        pieces.push(Letter {
            bytes: (line.range.end, line.range.end),
            x: (end, end + band),
            map,
            baseline,
        });
    }
    pieces
}

/// The edit geometry of `layout` (the shown string, composition included) under `placement`:
/// the caret at byte `caret`, the quads of `selection`, the underline of `preedit`, each with the
/// letters as drawn (raised by the vertical kerns, turned about their centres); the line box is
/// the box of the letters.
#[must_use]
pub fn edit_geometry(
    layout: &TextLayout,
    placement: Affine,
    caret: usize,
    selection: &Range<usize>,
    preedit: Option<&Range<usize>>,
) -> TextEditGeometry {
    let place = |p: kurbo::Point| placement.apply_point(Point::new(p.x, p.y));
    let (ascender, descender) = (layout.ascender.0, layout.descender.0);
    let line_box = letters_box(layout);
    let mut geometry = TextEditGeometry {
        line_box: [
            (line_box.x0, line_box.y0),
            (line_box.x1, line_box.y0),
            (line_box.x1, line_box.y1),
            (line_box.x0, line_box.y1),
        ]
        .map(|(x, y)| place(kurbo::Point::new(x, y))),
        caret: None,
        selection: Vec::new(),
        preedit: Vec::new(),
    };
    if selection.is_empty() {
        let index = layout.line_of(caret);
        let x = x_at(layout, index, caret);
        let (map, baseline) = layout
            .lines
            .get(index)
            .map_or((layout.frame(), 0.0), |line| {
                // The caret stands with the letter after it, or with the last one at the end.
                let letters = letters(layout, line);
                letters
                    .iter()
                    .find(|l| l.bytes.0 >= caret)
                    .or_else(|| letters.last())
                    .map_or((layout.frame(), line.baseline.0), |l| (l.map, l.baseline))
            });
        geometry.caret = Some([
            place(map * kurbo::Point::new(x, baseline - ascender)),
            place(map * kurbo::Point::new(x, baseline + descender)),
        ]);
    }
    let band = LINE_BREAK_BAND_EM * layout.mm_per_em;
    for line in &layout.lines {
        let letters = letters(layout, line);
        let selected = pieces_in(&letters, line, layout, selection, band);
        geometry.selection.extend(
            runs(&selected, ascender, descender)
                .into_iter()
                .map(|quad| quad.map(place)),
        );
        if let Some(range) = preedit {
            let composed = pieces_in(&letters, line, layout, range, 0.0);
            geometry.preedit.extend(
                runs(&composed, 0.0, 0.0)
                    .into_iter()
                    .map(|[a, b, _, _]| [place(a), place(b)]),
            );
        }
    }
    geometry
}
