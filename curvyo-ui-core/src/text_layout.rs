//! A text object laid out through the engine into what the canvas draws, hits and converts:
//! its glyphs as one compound path in the document and its line box (`specs/0074-text-objects`
//! criteria 4, 22, 23, 25).

use curvyo_document_core::{
    AnchorId, AnchorKind, NewAnchor, PathSnapshot, Point, SubpathSnapshot, TextOutline,
    TextSnapshot, Vec2,
};
use curvyo_text_core::{Face, Mm, TextLayout, layout};
use kurbo::{BezPath, PathEl};

/// Two points closer than this (mm) are one: a contour whose last point returns to its start
/// closes there.
const SAME_POINT_MM: f64 = 1e-9;

/// The engine's layout of `text` in `face` at the text's size, in text space; `None` when the
/// string is beyond the engine's bound.
#[must_use]
pub fn text_layout(text: &TextSnapshot, face: &Face) -> Option<TextLayout> {
    layout(face, &text.string, Mm(text.size.as_mm())).ok()
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
    for glyph in laid.glyph_paths(face) {
        let contours = contours_of(&glyph, &map, &mut next_id);
        glyph_outlines.push(contours.len());
        outlines.extend(contours);
    }
    let line_box = laid.line_box();
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
    let glyphs = PathSnapshot {
        id: text.id,
        closed: first.closed,
        style: text.style.clone(),
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

/// The contours of one glyph path (cubics and lines only) as closed outlines of anchors in the
/// document, with placeholder anchor ids (Object to path gives fresh ones).
fn contours_of(
    path: &BezPath,
    map: &impl Fn(kurbo::Point) -> Point,
    next_id: &mut u64,
) -> Vec<SubpathSnapshot> {
    let mut contours = Vec::new();
    let mut anchors: Vec<NewAnchor> = Vec::new();
    let mut finish = |anchors: &mut Vec<NewAnchor>| {
        if anchors.len() > 1 {
            let (first, last) = (anchors[0].point, anchors[anchors.len() - 1].point);
            if first.vector_to(last).length() <= SAME_POINT_MM {
                // invariant: at least two anchors, checked above.
                #[allow(clippy::unwrap_used)]
                let last = anchors.pop().unwrap();
                anchors[0].handle_in = last.handle_in;
            }
        }
        if !anchors.is_empty() {
            contours.push(SubpathSnapshot {
                closed: true,
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
                finish(&mut anchors);
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
            PathEl::ClosePath => finish(&mut anchors),
        }
    }
    finish(&mut anchors);
    contours
}

#[cfg(test)]
mod tests {
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
