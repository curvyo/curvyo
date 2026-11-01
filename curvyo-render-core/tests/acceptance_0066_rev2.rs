//! Independent acceptance test for 0066 revision 2 UI review item 10 (AC5, AC32): a redundant
//! dimension is drawn dashed, in its driving colour, so it reads apart from a conflicting one.
use curvyo_document_core::{Point, ViewTransform};
use curvyo_render_core::{DimensionLook, DimensionMarkDrawing, build_dimension_marks};

fn marks(line: &[(Point, Point)], reference: bool, redundant: bool) -> DimensionMarkDrawing<'_> {
    DimensionMarkDrawing {
        lines: line,
        arrows: &[],
        look: if reference {
            DimensionLook::Reference
        } else {
            DimensionLook::Driving
        },
        dashed: redundant,
        hovered: false,
        selected: false,
    }
}

#[test]
fn a_redundant_dimension_line_is_dashed_in_the_driving_colour_at_every_zoom() {
    for scale in [0.25, 1.0, 8.0] {
        let view = ViewTransform::new(scale, Point::new(0.0, 0.0));
        let line = [(Point::new(0.0, 0.0), Point::new(140.0 / scale, 0.0))];
        let driving = build_dimension_marks(&marks(&line, false, false), view);
        let redundant = build_dimension_marks(&marks(&line, false, true), view);
        assert_eq!(
            driving.triangles.len(),
            6,
            "a driving line is one solid quad"
        );
        assert!(
            redundant.triangles.len() >= 6 * 10,
            "{scale}: redundant is dashed ({} triangles)",
            redundant.triangles.len()
        );
        assert_eq!(
            driving.triangles[0].color, redundant.triangles[0].color,
            "same colour as a driving dimension"
        );
    }
}

#[test]
fn a_reference_dimension_stays_dashed_and_differs_in_colour_from_a_redundant_one() {
    let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
    let line = [(Point::new(0.0, 0.0), Point::new(70.0, 0.0))];
    let r = build_dimension_marks(&marks(&line, true, false), view);
    let x = build_dimension_marks(&marks(&line, false, true), view);
    assert!(r.triangles.len() > 6 && x.triangles.len() > 6);
    assert_ne!(r.triangles[0].color, x.triangles[0].color);
}

#[test]
fn a_selected_redundant_dimension_is_still_dashed() {
    let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
    let line = [(Point::new(0.0, 0.0), Point::new(70.0, 0.0))];
    let mut m = marks(&line, false, true);
    m.selected = true;
    assert!(build_dimension_marks(&m, view).triangles.len() > 6);
}
