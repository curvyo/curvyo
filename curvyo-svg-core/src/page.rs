//! The root of an SVG: the page, the map from its user units to millimetres, the pixel size of
//! known generators, and the Inkscape document settings (`specs/0024-svg-import-export`
//! criteria 15 to 20).

use kurbo::Affine;
use roxmltree::{Document, Node};

use crate::drawing::{DisplayUnitName, PageBackground};
use crate::limits::{MAX_COORDINATE_MM, MAX_PAGES};
use crate::units::{MM_PER_INCH, Units, aspect, root_size_mm, view_box, view_box_map};

/// The Inkscape namespace.
pub(crate) const INKSCAPE_NS: &str = "http://www.inkscape.org/namespaces/inkscape";
/// The Sodipodi namespace.
pub(crate) const SODIPODI_NS: &str = "http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd";
/// The namespace Adobe Illustrator declares on its root.
const ILLUSTRATOR_NS: &str = "http://ns.adobe.com/AdobeIllustrator/10.0/";

/// The root's page and map.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RootView {
    /// User units to millimetres.
    pub(crate) matrix: Affine,
    /// Lengths inside the content.
    pub(crate) units: Units,
    /// The page from the file; `None` when it gives no size and no viewBox.
    pub(crate) page: Option<(f64, f64)>,
}

/// Millimetres per pixel for this file: 1/72 inch for Illustrator, 1/90 inch for Inkscape 0.91
/// and earlier without a viewBox, else 1/96 inch (criterion 19).
fn px_mm(doc: &Document<'_>, root: Node<'_, '_>, has_view_box: bool) -> f64 {
    let illustrator = root.namespaces().any(|ns| ns.uri() == ILLUSTRATOR_NS)
        || doc.root().children().any(|n| {
            n.is_comment()
                && n.text()
                    .is_some_and(|t| t.contains("Generator: Adobe Illustrator"))
        });
    if illustrator {
        return MM_PER_INCH / 72.0;
    }
    let legacy = !has_view_box
        && root
            .attribute((INKSCAPE_NS, "version"))
            .and_then(inkscape_version)
            .is_some_and(|version| version < (0, 92));
    if legacy {
        MM_PER_INCH / 90.0
    } else {
        MM_PER_INCH / 96.0
    }
}

/// The major and minor number of an `inkscape:version` such as "0.48.4 r9939".
fn inkscape_version(text: &str) -> Option<(u32, u32)> {
    let mut parts = text.trim().split(|c: char| !c.is_ascii_digit());
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

/// The root's page, map and units (criteria 15 to 19, 20 (d)).
pub(crate) fn root_view(doc: &Document<'_>, root: Node<'_, '_>) -> RootView {
    let vb = view_box(root.attribute("viewBox"));
    let px = px_mm(doc, root, vb.is_some());
    let width = root_size_mm(root.attribute("width"), px);
    let height = root_size_mm(root.attribute("height"), px);
    let sizes = match (width, height, vb) {
        (Some(w), Some(h), _) => Some((w, h)),
        (Some(w), None, Some(vb)) => Some((w, w * vb.h / vb.w)),
        (None, Some(h), Some(vb)) => Some((h * vb.w / vb.h, h)),
        _ => None,
    };
    match (sizes, vb) {
        (Some((w, h)), Some(vb)) => RootView {
            matrix: view_box_map(vb, aspect(root.attribute("preserveAspectRatio")), w, h),
            units: Units {
                px_mm: px,
                viewport: (vb.w, vb.h),
            },
            page: Some((w, h)),
        },
        (Some((w, h)), None) => RootView {
            matrix: Affine::scale(px),
            units: Units {
                px_mm: px,
                viewport: (w / px, h / px),
            },
            page: Some((w, h)),
        },
        (None, Some(vb)) => RootView {
            matrix: Affine::scale(px) * Affine::translate((-vb.x, -vb.y)),
            units: Units {
                px_mm: px,
                viewport: (vb.w, vb.h),
            },
            page: Some((vb.w * px, vb.h * px)),
        },
        (None, None) => {
            let page = first_inkscape_page(root, px);
            RootView {
                matrix: Affine::scale(px),
                units: Units {
                    px_mm: px,
                    viewport: page.map_or((0.0, 0.0), |(w, h)| (w / px, h / px)),
                },
                page,
            }
        }
    }
}

/// `sodipodi:namedview`, the first one.
fn named_view<'a, 'input>(root: Node<'a, 'input>) -> Option<Node<'a, 'input>> {
    root.children()
        .find(|n| n.has_tag_name((SODIPODI_NS, "namedview")))
}

/// The size of the first `inkscape:page` (Inkscape 1.2), in millimetres (criterion 20 (d)).
fn first_inkscape_page(root: Node<'_, '_>, px: f64) -> Option<(f64, f64)> {
    let page = named_view(root)?
        .children()
        .find(|n| n.has_tag_name((INKSCAPE_NS, "page")))?;
    let side = |name| {
        page.attribute(name)
            .and_then(|t| t.trim().parse::<f64>().ok())
            .filter(|n| n.is_finite() && *n > 0.0)
    };
    Some((side("width")? * px, side("height")? * px))
}

/// Every `inkscape:page` of the named view in file order, its rectangle in user units mapped
/// to millimetres by `matrix` (criterion 129), and whether pages were left out for the count
/// limit. A page without a positive size, or whose mapped rectangle is not finite or reaches
/// beyond [`MAX_COORDINATE_MM`], is skipped (ADR 0018 point 6); at most [`MAX_PAGES`] are read.
pub(crate) fn inkscape_pages(
    root: Node<'_, '_>,
    matrix: Affine,
) -> (Vec<crate::drawing::SvgPage>, bool) {
    let Some(view) = named_view(root) else {
        return (Vec::new(), false);
    };
    let mut pages = Vec::new();
    for page in view
        .children()
        .filter(|n| n.has_tag_name((INKSCAPE_NS, "page")))
    {
        if pages.len() == MAX_PAGES {
            return (pages, true);
        }
        pages.extend(inkscape_page(page, matrix));
    }
    (pages, false)
}

/// One `inkscape:page` mapped by `matrix`; `None` when it has no positive size or lies outside
/// the coordinate limit.
fn inkscape_page(page: Node<'_, '_>, matrix: Affine) -> Option<crate::drawing::SvgPage> {
    let number = |name| {
        page.attribute(name)
            .and_then(|t| t.trim().parse::<f64>().ok())
            .filter(|n| n.is_finite())
    };
    let (w, h) = (number("width")?, number("height")?);
    if !(w > 0.0 && h > 0.0) {
        return None;
    }
    let (x, y) = (number("x").unwrap_or(0.0), number("y").unwrap_or(0.0));
    let rect = matrix.transform_rect_bbox(kurbo::Rect::new(x, y, x + w, y + h));
    let inside = [rect.x0, rect.y0, rect.x1, rect.y1]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE_MM);
    inside.then(|| crate::drawing::SvgPage {
        name: page.attribute((INKSCAPE_NS, "label")).map(str::to_owned),
        rect,
    })
}

/// `inkscape:document-units` when it is mm, cm or in (criterion 20 (b)).
pub(crate) fn display_unit(root: Node<'_, '_>) -> Option<DisplayUnitName> {
    match named_view(root)?
        .attribute((INKSCAPE_NS, "document-units"))?
        .trim()
    {
        "mm" => Some(DisplayUnitName::Mm),
        "cm" => Some(DisplayUnitName::Cm),
        "in" => Some(DisplayUnitName::In),
        _ => None,
    }
}

/// `pagecolor` with `inkscape:pageopacity` (criterion 20 (c)): the colour at that opacity, none
/// when the opacity is 0 or missing; `None` without a readable page colour.
pub(crate) fn background(root: Node<'_, '_>) -> Option<PageBackground> {
    let view = named_view(root)?;
    let colour = crate::paint::colour(view.attribute("pagecolor")?)?;
    let opacity = view
        .attribute((INKSCAPE_NS, "pageopacity"))
        .and_then(|t| t.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite())
        .map_or(0.0, |n| n.clamp(0.0, 1.0));
    Some(if opacity > 0.0 {
        PageBackground::Colour(colour.rgb, opacity)
    } else {
        PageBackground::None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inkscape_versions_read_major_and_minor() {
        assert_eq!(inkscape_version("0.48.4 r9939"), Some((0, 48)));
        assert_eq!(
            inkscape_version("1.3.2 (091e20e, 2023-11-25)"),
            Some((1, 3))
        );
        assert_eq!(inkscape_version("x"), None);
    }
}
