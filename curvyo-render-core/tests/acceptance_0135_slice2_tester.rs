//! Tester acceptance case for `specs/0135-image-optimisation` slice 2 in `curvyo-render-core`:
//! the frame drawn for a cropped picture's operation look (the dashed Not assigned frame and the
//! coloured frame under "View: Operations") is the visible part, the frame of ADR 0072 §3, not
//! the whole picture (criterion 5: "the crop makes the image object's frame the cropped
//! rectangle").
#![allow(clippy::unwrap_used, missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    AdjustEntry, AdjustStep, AdjustmentsWrite, Affine, Document, NewImage, ObjectSnapshot, Point,
    ViewTransform,
};
use curvyo_render_core::{OperationLook, restyled};

#[test]
fn c5_the_operation_look_frame_of_a_cropped_picture_is_its_visible_part() {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png"),
    )
    .unwrap();
    let d = Document::new(1);
    let id = d
        .add_images(&[NewImage {
            bytes: Arc::from(bytes.as_slice()),
            placement: Affine::new(40.0, 0.0, 0.0, 20.0, 10.0, 10.0),
            name: "P".to_owned(),
        }])
        .unwrap()[0];
    let fields = [
        ("left_pct", 0),
        ("top_pct", 0),
        ("width_pct", 50),
        ("height_pct", 100),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    d.edit_image_adjustments(
        id,
        &AdjustmentsWrite {
            preset: None,
            strength: None,
            steps: Some(vec![AdjustEntry::Step(AdjustStep {
                kind: "crop".to_owned(),
                enabled: true,
                fields,
            })]),
            dither: None,
        },
    )
    .unwrap();
    let read = d.read_objects();
    let image = read
        .objects
        .iter()
        .find(|o| matches!(o, ObjectSnapshot::Image(i) if i.id == id))
        .unwrap();
    let ObjectSnapshot::Image(snap) = image else {
        unreachable!()
    };
    let visible = snap.corners();
    assert!(
        (visible[1].x - 30.0).abs() < 1e-9,
        "the visible part ends at 30 mm"
    );
    let view = ViewTransform::new(4.0, Point::new(0.0, 0.0));
    let Some(ObjectSnapshot::Path(frame)) = restyled(image, OperationLook::MutedDashed, view)
    else {
        panic!("an image becomes its frame");
    };
    let xs: Vec<f64> = frame.anchors.iter().map(|a| a.point.x).collect();
    let max = xs.iter().copied().fold(f64::MIN, f64::max);
    assert!(
        (max - 30.0).abs() < 1e-9,
        "the drawn frame is the visible part (max x 30), got {xs:?}"
    );
}
