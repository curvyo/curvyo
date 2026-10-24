//! A text object laid out through the engine into what the canvas draws, hits and converts:
//! its glyphs as one compound path in the document and its line box (`specs/0074-text-objects`
//! criteria 4, 22, 23, 25). A single-line text's glyphs are open centre lines drawn with the
//! stroke and never filled (`specs/0076-single-line-fonts` criteria 7, 16, 17). The text's layout
//! values become the engine's options here, once, for drawing, editing, hit test and Object to
//! path alike (`specs/0079-text-layout-controls/adrs.md` decision 4).

use curvyo_document_core::{
    Affine, AnchorId, AnchorKind, FontKind, Length, NewAnchor, PathSnapshot, Point,
    SubpathSnapshot, TextAlign, TextLayoutValues, TextOutline, TextSnapshot, Vec2,
};
use curvyo_text_core::{Align, Face, LayoutOptions, Mm, TextLayout, layout};
use kurbo::{BezPath, PathEl, Shape};

/// Two points closer than this (mm) are one: a contour whose last point returns to its start
/// closes there.
const SAME_POINT_MM: f64 = 1e-9;

/// The engine's options for a text of `size` with the layout values `values`: every relative
/// value times the size, in mm (`0079` decision 4).
#[must_use]
pub fn layout_options(size: Length, values: &TextLayoutValues) -> LayoutOptions {
    let size = size.as_mm();
    LayoutOptions {
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
    }
}

/// The engine's layout of `text` in `face` at the text's size with its layout values, in text
/// space; `None` when the string is beyond the engine's bound.
#[must_use]
pub fn text_layout(text: &TextSnapshot, face: &Face) -> Option<TextLayout> {
    layout(
        face,
        &text.string,
        Mm(text.size.as_mm()),
        &layout_options(text.size, &text.layout),
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
    text.placement
        .compose(Affine::translation(Vec2::new(dx, 0.0)))
}

/// `text` laid out in `face` and mapped to the document by its placement. `substitute` says the
/// face stands in for a missing one (criterion 29).
#[must_use]
pub fn lay_out(text: &TextSnapshot, face: &Face, substitute: bool) -> Option<TextOutline> {
    let laid = text_layout(text, face)?;
    let placement = text.placement;
    let map = |p: kurbo::Point| placement.apply_point(Point::new(p.x, p.y));
    let mut outlines: Vec<SubpathSnapshot> = Vec::new();
    let mut glyph_outlines = Vec::new();
    let mut next_id = 0_u64;
    let paths = laid.glyph_paths(face);
    for glyph in &paths {
        let contours = contours_of(glyph, &map, &mut next_id);
        glyph_outlines.push(contours.len());
        outlines.extend(contours);
    }
    let mut line_box = laid.line_box();
    if laid.rotation_rad != 0.0 {
        // Turned glyphs may stand out of the lines' box: the box covers them (`0079` criterion 15).
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
    let mut outlines = outlines.into_iter();
    let first = outlines.next().unwrap_or(SubpathSnapshot {
        closed: true,
        anchors: Vec::new(),
    });
    let mut style = text.style.clone();
    if text.font.kind == FontKind::SingleLine {
        // The text's own fill register is kept; it is not applied (criterion 8).
        style.fill.enabled = false;
    }
    let glyphs = PathSnapshot {
        id: text.id,
        closed: first.closed,
        style,
        anchors: first.anchors,
        extra_subpaths: outlines.collect(),
        rotation: text.rotation(),
    };
    Some(TextOutline {
        glyphs,
        line_box: corners,
        glyph_outlines,
        substitute,
    })
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
            placement: Affine::translation(Vec2::new(30.0, 40.0)),
            style: Style::default(),
            outline: None,
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
