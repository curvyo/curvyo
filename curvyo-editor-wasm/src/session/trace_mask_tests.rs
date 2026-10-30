//! The Trace tool's Mask view (`specs/0089-trace-quality-and-colour` criterion 21, `ux-notes.md`
//! §4): the fourth view, the mask drawn in place of the shown image, pixel-equal with the mask the
//! worker handed in.

use curvyo_document_core::{Length, NodeId, ObjectSnapshot};
use curvyo_ui_core::{TraceParams, trace_mask_job};

use super::{Session, Tool, TraceViewMode};

const PNG: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");

fn traced_image() -> (Session, NodeId) {
    let mut session = Session::new(1);
    session.import_begin(None);
    session.import_add(PNG, "logo.png");
    session.import_finish(false);
    let image = session
        .objects()
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.id),
            _ => None,
        })
        .unwrap();
    session.set_tool(Tool::Trace);
    (session, image)
}

/// The mask the worker makes for `image` (3 x 1 px: transparent red, half green, blue) with the
/// first-use settings at Threshold 128 and no Specks: the blue pixel.
fn worker_mask(session: &mut Session, image: NodeId) -> curvyo_ui_core::TraceMask {
    let source = session.trace_source(image, false).unwrap();
    trace_mask_job(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        &TraceParams {
            specks: Length::from_mm(0.0),
            ..TraceParams::with_threshold(128)
        },
    )
    .unwrap()
}

/// Shift+`\` cycles the four views in pill order; the peek still shows Image.
#[test]
fn the_views_cycle_overlay_mask_trace_image() {
    let (mut session, _) = traced_image();
    let mut seen = vec![session.trace_view_mode()];
    for _ in 0..4 {
        session.cycle_trace_view_mode();
        seen.push(session.trace_view_mode());
    }
    assert_eq!(
        seen,
        [
            TraceViewMode::Overlay,
            TraceViewMode::Mask,
            TraceViewMode::Trace,
            TraceViewMode::Image,
            TraceViewMode::Overlay
        ]
    );
    session.set_trace_view_mode(TraceViewMode::Mask);
    session.set_trace_peek(true);
    assert_eq!(session.shown_trace_view_mode(), TraceViewMode::Image);
    assert_eq!(session.trace_view_mode(), TraceViewMode::Mask);
}

/// Criterion 21: the Mask view draws the worker's mask, content black and the rest white, pixel
/// for pixel, at the image's frame, unwashed, nearest-neighbour when zoomed in; the document is
/// untouched.
#[test]
fn the_mask_view_draws_exactly_the_mask() {
    let (mut session, image) = traced_image();
    let mask = worker_mask(&mut session, image);
    assert_eq!((mask.width, mask.height), (3, 1));
    assert_eq!(mask.bits, [0, 0, 255]);
    let before = session.objects().to_vec();
    assert!(session.set_trace_mask(image, mask.width, mask.height, &mask.bits));
    session.set_trace_view_mode(TraceViewMode::Mask);

    let mut list = session.draw_list();
    assert_eq!(list.images.len(), 1, "one picture: the mask");
    let ObjectSnapshot::Image(original) = &before[0] else {
        panic!("the image");
    };
    assert_ne!(
        list.images[0].asset, original.asset,
        "not the image's picture"
    );
    assert_eq!(
        list.images[0].corners,
        original.corners(),
        "at the image's frame"
    );
    assert!(list.images[0].opacity.get() > 0.999);
    let pictures = session.frame_images(&mut list);
    assert_eq!(pictures.len(), 1);
    let (_, picture, pixelated) = &pictures[0];
    assert!(*pixelated, "pixels stay pixels when zoomed in");
    assert_eq!((picture.width, picture.height), (mask.width, mask.height));
    let drawn: Vec<u8> = picture
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|px| match px {
            [0, 0, 0, 255] => 255,
            [255, 255, 255, 255] => 0,
            other => panic!("not black or white: {other:?}"),
        })
        .collect();
    assert_eq!(drawn, mask.bits, "pixel-equal with the mask handed in");

    let mut objects = before.clone();
    session.trace_artwork(&mut objects);
    assert_eq!(
        objects.len(),
        1,
        "the image's wash is gone in the Mask view"
    );
    assert_eq!(
        &session.objects()[..],
        &before[..],
        "the document is untouched"
    );
    assert_eq!(
        session.take_bad_asset_notice(),
        "",
        "the mask is no unreadable asset"
    );
}

/// A stale mask is drawn at half strength; a new one is current again.
#[test]
fn a_stale_mask_is_half_strength() {
    let (mut session, image) = traced_image();
    let mask = worker_mask(&mut session, image);
    session.set_trace_mask(image, mask.width, mask.height, &mask.bits);
    session.set_trace_view_mode(TraceViewMode::Mask);
    session.set_trace_preview_stale(true);
    assert!((session.draw_list().images[0].opacity.get() - 0.5).abs() < 1e-9);
    session.set_trace_mask(image, mask.width, mask.height, &mask.bits);
    assert!(session.draw_list().images[0].opacity.get() > 0.999);
}

/// Without a mask of the shown image the Mask view leaves it washed; bits of the wrong size are
/// refused.
#[test]
fn no_mask_leaves_the_image_washed() {
    let (mut session, image) = traced_image();
    session.set_trace_view_mode(TraceViewMode::Mask);
    let objects = session.objects().to_vec();
    let mut drawn = objects.clone();
    session.trace_artwork(&mut drawn);
    assert_eq!(drawn.len(), 2, "the image and its wash");
    assert_eq!(drawn[0], objects[0]);
    assert!(!session.set_trace_mask(image, 4, 4, &[0; 15]));
    assert!(!session.set_trace_mask(image, 0, 0, &[]));
    let mut again = objects.clone();
    session.trace_artwork(&mut again);
    assert_eq!(again, drawn);
}
