//! `specs/0121-print-and-cut` criterion 4: a registration mark is drawn as artwork (the canvas
//! and the PNG export read the same lines), black, at its centre.
#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_document_core::{Document, ObjectSnapshot, Point, ViewTransform};
use curvyo_render_core::{RgbaColor, build_artwork};

#[test]
fn a_mark_draws_black_triangles_around_its_centre() {
    let d = Document::new(1);
    d.insert_registration_marks([Point::new(50.0, 40.0); 4])
        .unwrap();
    let mark = d
        .read_objects()
        .objects
        .iter()
        .find(|o| matches!(o, ObjectSnapshot::RegistrationMark(_)))
        .cloned()
        .unwrap();
    let list = build_artwork(&[mark], ViewTransform::new(4.0, Point::new(0.0, 0.0)));
    assert!(
        list.triangles
            .iter()
            .all(|v| v.color.r == RgbaColor::BLACK.r && v.color.a == 255)
    );
    let min = list
        .triangles
        .iter()
        .map(|v| v.position.x)
        .fold(f64::MAX, f64::min);
    assert!(
        min < 46.1 && min > 45.8,
        "the cross reaches 4 mm left: {min}"
    );
}
