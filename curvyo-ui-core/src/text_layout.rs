//! A text object laid out through the engine into what the canvas draws, hits and converts:
//! its glyphs as one compound path in the document and its line box (`specs/0074-text-objects`
//! criteria 4, 22, 23, 25). A single-line text's glyphs are open centre lines drawn with the
//! stroke and never filled (`specs/0076-single-line-fonts` criteria 7, 16, 17). The text's layout
//! values become the engine's options here, once, for drawing, editing, hit test and Object to
//! path alike (`specs/0079-text-layout-controls/adrs.md` decision 4).

use curvyo_document_core::{
    Affine, AnchorId, AnchorKind, FontKind, Length, NewAnchor, PathSnapshot, Point,
    SubpathSnapshot, TextAlign, TextChars, TextGlyphOrientation, TextLayoutValues, TextOutline,
    TextSnapshot, TextWritingMode, Vec2,
};
use curvyo_text_core::{
    Align, CharOffset, Face, GlyphOrientation, LayoutOptions, Mm, TAPER_ACCURACY, TextLayout,
    WritingMode, layout,
};
use kurbo::{BezPath, PathEl, Shape};

/// Two points closer than this (mm) are one: a contour whose last point returns to its start
/// closes there.
const SAME_POINT_MM: f64 = 1e-9;

/// The engine's options for a text of `size` with the layout values `values` and the
/// per-character values `chars`: every relative value times the size, in mm (`0079` decision 4,
/// ADR 0021 §6).
#[must_use]
pub fn layout_options(size: Length, values: &TextLayoutValues, chars: &TextChars) -> LayoutOptions {
    let size = size.as_mm();
    LayoutOptions {
        char_offsets: chars
            .entries()
            .iter()
            .map(|(index, values)| CharOffset {
                index: *index,
                along: Mm(values.kern_x * size),
                across: Mm(values.kern_y * size),
                rotation_rad: values.rotation.to_radians(),
            })
            .collect(),
        align: match values.align {
            TextAlign::Left => Align::Left,
            TextAlign::Centre => Align::Centre,
            TextAlign::Right => Align::Right,
            TextAlign::Justify => Align::Justify,
        },
        letter_spacing: Mm(values.letter_spacing * size),
        word_spacing: Mm(values.word_spacing * size),
        line_spacing: values.line_spacing.map(|factor| Mm(factor * size)),
        rotation_rad: values.rotation.as_radians(),
        writing_mode: match values.writing_mode {
            TextWritingMode::Horizontal => WritingMode::Horizontal,
            TextWritingMode::VerticalRl => WritingMode::VerticalRl,
            TextWritingMode::VerticalLr => WritingMode::VerticalLr,
        },
        orientation: match values.orientation {
            TextGlyphOrientation::Auto => GlyphOrientation::Auto,
            TextGlyphOrientation::Upright => GlyphOrientation::Upright,
            TextGlyphOrientation::Sideways => GlyphOrientation::Sideways,
        },
        taper: values.taper,
    }
}

/// How close a tapered curve must stay to the exact map in text space so that it stays within
/// [`TAPER_ACCURACY`] in the document under `placement`: the accuracy divided by the placement's
/// largest scale (`0079` `adrs.md` decision 9).
#[must_use]
pub(crate) fn taper_accuracy(placement: Affine) -> Mm {
    let [a, b, c, d, _, _] = placement.to_array();
    let squares = a * a + b * b + c * c + d * d;
    let det = a * d - b * c;
    let largest = f64::midpoint(
        squares,
        (squares * squares - 4.0 * det * det).max(0.0).sqrt(),
    )
    .sqrt();
    if largest.is_finite() && largest > 1.0 {
        Mm(TAPER_ACCURACY.0 / largest)
    } else {
        TAPER_ACCURACY
    }
}

/// The engine's layout of `text` in `face` at the text's size with its layout values, in text
/// space; `None` when the string is beyond the engine's bound.
#[must_use]
pub fn text_layout(text: &TextSnapshot, face: &Face) -> Option<TextLayout> {
    let mut values = text.layout;
    if text.on_path.as_ref().is_some_and(|p| p.curve.is_some()) {
        // A vertical text is set horizontally along a path; its stored mode is kept
        // (`specs/0080-text-on-path` criterion 48).
        values.writing_mode = TextWritingMode::Horizontal;
        values.orientation = TextGlyphOrientation::Auto;
    }
    layout(
        face,
        &text.string,
        Mm(text.size.as_mm()),
        &layout_options(text.size, &values, &text.chars),
    )
    .ok()
}

/// The placement that keeps `text` where it is on the canvas when its alignment becomes `align`
/// (`0079` criterion 4): the anchor moves along the baseline so that the widest line's extent
/// keeps its position. The text's own placement when it cannot be laid out.
#[must_use]
pub fn realigned_placement(text: &TextSnapshot, face: &Face, align: TextAlign) -> Affine {
    let mut moved = text.clone();
    moved.layout.align = align;
    let (Some(before), Some(after)) = (text_layout(text, face), text_layout(&moved, face)) else {
        return text.placement;
    };
    let widest = before
        .lines
        .iter()
        .enumerate()
        .fold((0, f64::NEG_INFINITY), |(at, width), (index, line)| {
            let this = line.extent.end.0 - line.extent.start.0;
            if this > width {
                (index, this)
            } else {
                (at, width)
            }
        })
        .0;
    let (Some(old), Some(new)) = (before.lines.get(widest), after.lines.get(widest)) else {
        return text.placement;
    };
    let dx = old.extent.start.0 - new.extent.start.0;
    // Along the line: down the column for vertical text.
    let along = after.frame() * kurbo::Point::new(dx, 0.0);
    text.placement
        .compose(Affine::translation(Vec2::new(along.x, along.y)))
}

/// The placement that keeps `text` where it is on the canvas when its layout values become
/// `values` (writing mode or glyph orientation, `0079` criterion 20): the first column's start
/// stays where the first line's start was, in text space. The text's own placement when it
/// cannot be laid out.
#[must_use]
pub fn kept_placement(text: &TextSnapshot, face: &Face, values: &TextLayoutValues) -> Affine {
    let mut moved = text.clone();
    moved.layout = *values;
    let start = |laid: &TextLayout| {
        laid.lines
            .first()
            .map(|line| laid.frame() * kurbo::Point::new(line.extent.start.0, line.baseline.0))
    };
    let (Some(before), Some(after)) = (
        text_layout(text, face).as_ref().and_then(start),
        text_layout(&moved, face).as_ref().and_then(start),
    ) else {
        return text.placement;
    };
    text.placement.compose(Affine::translation(Vec2::new(
        before.x - after.x,
        before.y - after.y,
    )))
}

/// `text` laid out in `face` and mapped to the document by its placement. `substitute` says the
/// face stands in for a missing one (criterion 29).
#[must_use]
pub fn lay_out(text: &TextSnapshot, face: &Face, substitute: bool) -> Option<TextOutline> {
    let laid = text_layout(text, face)?;
    if let Some(placement) = text
        .on_path
        .as_ref()
        .and_then(|on_path| crate::text_on_path::PathPlacement::new(&laid, on_path))
    {
        return Some(crate::text_on_path::lay_out_on_path(
            text, face, &laid, &placement, substitute,
        ));
    }
    let placement = text.placement;
    let map = |p: kurbo::Point| placement.apply_point(Point::new(p.x, p.y));
    let mut outlines: Vec<SubpathSnapshot> = Vec::new();
    let mut glyph_outlines = Vec::new();
    let mut next_id = 0_u64;
    let paths = laid.glyph_paths_within(face, taper_accuracy(placement));
    for glyph in &paths {
        let contours = contours_of(glyph, &map, &mut next_id);
        glyph_outlines.push(contours.len());
        outlines.extend(contours);
    }
    // The box of the letters as drawn, raised by their vertical kerns (`0079` criterion 26).
    let mut line_box = crate::text_caret::letters_box(&laid);
    let turned = laid.rotation_rad != 0.0
        || laid.taper != 0.0
        || laid
            .lines
            .iter()
            .flat_map(|line| &line.glyphs)
            .any(|glyph| glyph.rotation_rad != 0.0);
    if turned {
        // Turned or tapered glyphs may stand out of their boxes: the box covers them (`0079`
        // criteria 15, 17).
        line_box = paths
            .iter()
            .map(Shape::bounding_box)
            .fold(line_box, |union, glyph| union.union(glyph));
    }
    let corners = [
        (line_box.x0, line_box.y0),
        (line_box.x1, line_box.y0),
        (line_box.x1, line_box.y1),
        (line_box.x0, line_box.y1),
    ]
    .map(|(x, y)| placement.apply_point(Point::new(x, y)));
    let glyphs = glyph_path(text, outlines, text.rotation());
    Some(TextOutline {
        glyphs,
        line_box: corners,
        glyph_outlines,
        letter_boxes: Vec::new(),
        substitute,
    })
}

/// The glyphs of `text` as one compound path with its id and style: a single-line text keeps its
/// fill register but does not apply it (`0076` criterion 8).
pub(crate) fn glyph_path(
    text: &TextSnapshot,
    outlines: Vec<SubpathSnapshot>,
    rotation: curvyo_document_core::Angle,
) -> PathSnapshot {
    let mut outlines = outlines.into_iter();
    let first = outlines.next().unwrap_or(SubpathSnapshot {
        closed: true,
        anchors: Vec::new(),
    });
    let mut style = text.style.clone();
    if text.font.kind == FontKind::SingleLine {
        style.fill.enabled = false;
    }
    PathSnapshot {
        id: text.id,
        closed: first.closed,
        style,
        anchors: first.anchors,
        extra_subpaths: outlines.collect(),
        rotation,
    }
}

/// The contours of one glyph path (cubics and lines only) as outlines of anchors in the
/// document, with placeholder anchor ids (Object to path gives fresh ones). A contour is closed
/// only when it ends with `ClosePath` (every OpenType contour does); an open one is kept open even
/// when its ends meet (`0076` criterion 16).
pub(crate) fn contours_of(
    path: &BezPath,
    map: &impl Fn(kurbo::Point) -> Point,
    next_id: &mut u64,
) -> Vec<SubpathSnapshot> {
    let mut contours = Vec::new();
    let mut anchors: Vec<NewAnchor> = Vec::new();
    let mut finish = |anchors: &mut Vec<NewAnchor>, closed: bool| {
        if closed && anchors.len() > 1 {
            let (first, last) = (anchors[0].point, anchors[anchors.len() - 1].point);
            if first.vector_to(last).length() <= SAME_POINT_MM {
                // invariant: at least two anchors, checked above.
                #[allow(clippy::unwrap_used)]
                let last = anchors.pop().unwrap();
                anchors[0].handle_in = last.handle_in;
            }
        }
        // A lone point of an open contour draws nothing and is no path.
        if !closed && anchors.len() < 2 {
            anchors.clear();
        }
        if !anchors.is_empty() {
            contours.push(SubpathSnapshot {
                closed,
                anchors: std::mem::take(anchors),
            });
        }
    };
    let anchor = |point: Point, next_id: &mut u64| {
        *next_id += 1;
        NewAnchor {
            id: AnchorId::new(0, *next_id),
            point,
            handle_in: Vec2::ZERO,
            handle_out: Vec2::ZERO,
            kind: AnchorKind::Corner,
        }
    };
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) => {
                finish(&mut anchors, false);
                anchors.push(anchor(map(p), next_id));
            }
            // The engine writes cubics only; a quadratic never arrives, its end is kept.
            PathEl::LineTo(p) | PathEl::QuadTo(_, p) => anchors.push(anchor(map(p), next_id)),
            PathEl::CurveTo(c1, c2, p) => {
                let end = map(p);
                if let Some(last) = anchors.last_mut() {
                    last.handle_out = last.point.vector_to(map(c1));
                }
                let mut new = anchor(end, next_id);
                new.handle_in = end.vector_to(map(c2));
                anchors.push(new);
            }
            PathEl::ClosePath => finish(&mut anchors, true),
        }
    }
    finish(&mut anchors, false);
    contours
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use curvyo_document_core::{
        Angle, FontKind, FontRef, Length, Style, TextAlign, TextLayoutValues, TextSnapshot,
        TextWritingMode,
    };
    use curvyo_text_core::FaceStore;

    use super::*;

    fn sans() -> std::sync::Arc<curvyo_text_core::Face> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf");
        let mut store = FaceStore::new();
        let key = store.add(std::fs::read(path).unwrap(), 0).unwrap();
        store.get(&key).unwrap().clone()
    }

    fn text(string: &str, layout: TextLayoutValues) -> TextSnapshot {
        TextSnapshot {
            id: curvyo_document_core::UNCREATED_TEXT_ID,
            string: string.to_owned(),
            font: FontRef {
                family: "Source Sans 3".to_owned(),
                style: "Regular".to_owned(),
                face_hash: "0".repeat(64),
                face_index: 0,
                kind: FontKind::Outline,
            },
            size: Length::from_mm(10.0),
            layout,
            chars: curvyo_document_core::TextChars::default(),
            placement: Affine::translation(Vec2::new(30.0, 40.0)),
            style: Style::default(),
            outline: None,
            on_path: None,
            source: None,
        }
    }

    /// `specs/0079-text-layout-controls` criterion 4: an alignment change moves the anchor so the
    /// widest line keeps its place; the other lines take the new alignment.
    #[test]
    fn ac04_the_widest_line_keeps_its_place() {
        let face = &sans();
        let left = text("A\nAAAA\nAA", TextLayoutValues::DEFAULT);
        let placement = realigned_placement(&left, face, TextAlign::Centre);
        let mut centred = left.clone();
        centred.layout.align = TextAlign::Centre;
        centred.placement = placement;
        let start = |t: &TextSnapshot, line: usize| {
            let laid = text_layout(t, face).unwrap();
            t.placement
                .apply_point(Point::new(laid.lines[line].extent.start.0, 0.0))
                .x
        };
        assert!((start(&left, 1) - start(&centred, 1)).abs() < 0.01);
        assert!(
            start(&centred, 0) > start(&left, 0) + 1.0,
            "the short line moved"
        );
    }

    /// Criterion 15: the box of a text with turned glyphs covers their outlines.
    #[test]
    fn ac15_the_box_covers_turned_glyphs() {
        let face = &sans();
        let turned = text(
            "I",
            TextLayoutValues {
                rotation: Angle::from_radians(90_f64.to_radians()),
                letter_spacing: 0.1,
                ..TextLayoutValues::DEFAULT
            },
        );
        let outline = lay_out(&turned, face, false).unwrap();
        let (min_x, max_x) = outline
            .line_box
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p.x), hi.max(p.x))
            });
        for anchor in &outline.glyphs.anchors {
            assert!(anchor.point.x >= min_x - 1e-9 && anchor.point.x <= max_x + 1e-9);
        }
    }
    /// `specs/0079-text-layout-controls` criteria 20 and 24: in a column the caret lies across it
    /// at the boundary's height, and a writing-mode change keeps the first line's start.
    #[test]
    fn a_column_has_a_crosswise_caret_and_keeps_its_start() {
        let face = &sans();
        let flat = text("AB", TextLayoutValues::DEFAULT);
        let values = TextLayoutValues {
            writing_mode: TextWritingMode::VerticalRl,
            ..TextLayoutValues::DEFAULT
        };
        let mut column = flat.clone();
        column.layout = values;
        column.placement = kept_placement(&flat, face, &values);
        let laid = text_layout(&column, face).unwrap();
        let geometry = crate::edit_geometry(&laid, column.placement, 1, &(1..1), None);
        let [a, b] = geometry.caret.unwrap();
        let down = laid.lines[0].boundaries[1].1.0;
        assert!(
            (a.y - b.y).abs() < 1e-9
                && (a.y - column.placement.apply_point(Point::new(0.0, down)).y).abs() < 1e-9
        );
        assert!((a.x - b.x).abs() > 1.0, "across the column");
        let start = |t: &TextSnapshot| {
            let laid = text_layout(t, face).unwrap();
            let p = laid.frame() * kurbo::Point::new(laid.lines[0].extent.start.0, 0.0);
            t.placement.apply_point(Point::new(p.x, p.y))
        };
        assert!(start(&flat).vector_to(start(&column)).length() < 0.01);
    }

    /// The engine lays out every string a file may hold (`MAX_SHAPE_CHARS` is text-core's own
    /// copy of the document's bound, architect review m2, must-fix 1).
    #[test]
    fn the_engine_bound_is_the_stored_bound() {
        assert_eq!(
            curvyo_text_core::limits::MAX_SHAPE_CHARS,
            curvyo_document_core::MAX_STORED_TEXT_CHARS
        );
    }
}
