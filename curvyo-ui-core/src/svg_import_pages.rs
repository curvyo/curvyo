//! The page settings of an imported drawing (`specs/0024-svg-import-export` criteria 20 and
//! 129): its size, display unit, background and Inkscape pages, and the closing-node rule of
//! the profile's outlines (ADR 0018 point 9).

use curvyo_document_core::{
    BackgroundPaint, Color, DisplayUnit, DocumentBackground, DocumentSize, DrawingPage,
    DrawingPageFrame, Opacity, Point, clean_name, validated_document_side,
};
use curvyo_svg_core::{DisplayUnitName, PageBackground, SvgDrawing};

/// The page settings of `drawing`: its page (a new document's size when it gives none), its
/// display unit and its background.
#[must_use]
pub fn svg_page(drawing: &SvgDrawing) -> DrawingPage {
    let size = drawing
        .page
        .and_then(|(w, h)| {
            Some(DocumentSize::new(
                validated_document_side(curvyo_document_core::Length::from_mm(w)).ok()?,
                validated_document_side(curvyo_document_core::Length::from_mm(h)).ok()?,
            ))
        })
        .unwrap_or_default();
    let display_unit = drawing.display_unit.map(|unit| match unit {
        DisplayUnitName::Mm => DisplayUnit::Mm,
        DisplayUnitName::Cm => DisplayUnit::Cm,
        DisplayUnitName::In => DisplayUnit::In,
    });
    let background = drawing.background.map(|paint| match paint {
        PageBackground::None => DocumentBackground {
            paint: BackgroundPaint::None,
            ..DocumentBackground::DEFAULT
        },
        PageBackground::Colour(rgb, opacity) => DocumentBackground {
            paint: BackgroundPaint::Solid,
            color: Color {
                r: rgb[0],
                g: rgb[1],
                b: rgb[2],
            },
            opacity: Opacity::new(opacity).unwrap_or(Opacity::OPAQUE),
        },
    });
    let page_side = |mm: f64| {
        validated_document_side(curvyo_document_core::Length::from_mm(mm.clamp(1.0, 1e5))).ok()
    };
    let pages = drawing
        .pages
        .iter()
        .take(curvyo_svg_core::limits::MAX_PAGES)
        .enumerate()
        .filter_map(|(index, page)| {
            Some(DrawingPageFrame {
                name: page
                    .name
                    .as_deref()
                    .and_then(clean_name)
                    .unwrap_or_else(|| format!("Page {}", index + 1)),
                origin: Point::new(page.rect.x0, page.rect.y0),
                size: DocumentSize::new(
                    page_side(page.rect.width())?,
                    page_side(page.rect.height())?,
                ),
            })
        })
        .collect();
    DrawingPage {
        size,
        display_unit,
        background,
        pages,
    }
}

/// Under the profile, a closed outline whose last node lies exactly on its first is closed by its
/// last segment: the two nodes are one (the writer's form of a curved closing segment).
pub(crate) fn merge_closing_nodes(outlines: &mut [curvyo_document_core::DrawingOutline]) {
    for outline in outlines.iter_mut().filter(|o| o.closed) {
        if outline.anchors.len() > 2
            && let (Some(first), Some(last)) = (outline.anchors.first(), outline.anchors.last())
            // exact: the writer repeats the first node's own numbers (ADR 0018 point 9).
            && first.point == last.point
        {
            let handle_in = last.handle_in;
            outline.anchors.pop();
            outline.anchors[0].handle_in = handle_in;
        }
    }
}
