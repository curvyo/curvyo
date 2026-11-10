//! The Trace tool with mask paths (`specs/0094-masked-composite-trace` slice 1, criteria 1, 5,
//! 6, 7, 12, 13, 14 and 17).

#![allow(clippy::many_single_char_names)]

use curvyo_document_core::{Length, NodeId, ObjectSnapshot, Point, RectBounds};
use curvyo_ui_core::{
    TraceJob, TraceMode, TraceParams, TracePixelMap, TraceRaster, read_trace_wire, run_trace_job,
};

use super::{Session, Tool, TraceCreateOutcome, TraceSelection};

const PNG: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");

fn import_images(session: &mut Session, count: usize) -> Vec<NodeId> {
    session.import_begin(None);
    for i in 0..count {
        session.import_add(PNG, &format!("logo-{i}.png"));
    }
    session.import_finish(false);
    session
        .objects()
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.id),
            _ => None,
        })
        .collect()
}

fn corners(session: &Session, image: NodeId) -> [Point; 4] {
    session
        .objects()
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Image(i) if i.id == image => Some(i.corners()),
            _ => None,
        })
        .unwrap()
}

fn rect(session: &Session, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    session.document.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

/// One image and a rectangle over its left half.
fn masked() -> (Session, NodeId, NodeId) {
    let mut session = Session::new(1);
    let image = import_images(&mut session, 1)[0];
    let c = corners(&session, image);
    let (w, h) = (c[2].x - c[0].x, c[2].y - c[0].y);
    let mask = rect(&session, c[0].x - 1.0, c[0].y - 1.0, w / 2.0 + 1.0, h + 2.0);
    session.selection.set(&[image, mask]);
    (session, image, mask)
}

#[test]
fn c1_an_image_and_a_path_are_a_masked_trace() {
    let (mut session, image, mask) = masked();
    assert!(session.trace_available(), "the Select bar's Trace button");
    session.set_tool(Tool::Trace);
    let view = session.trace_view();
    assert_eq!(view.selection, TraceSelection::Images);
    assert_eq!((view.images, view.shown), (vec![image], Some(image)));
    assert_eq!(view.mask, [mask]);
    assert!(!view.mask_outside_image);
    assert!(session.trace_source(image, false).is_some());
    let json: serde_json::Value = serde_json::from_str(&session.trace_mask_json()).unwrap();
    assert_eq!(json["outside"], false);
    // Only images: no mask, the settings as before.
    session.selection.set(&[image]);
    assert_eq!(session.trace_view().mask, []);
    assert_eq!(session.trace_mask_json(), "");
}

#[test]
fn c14_text_or_a_second_image_with_mask_paths_is_refused() {
    let mut session = super::text_tests::with_text("Name");
    let images = import_images(&mut session, 2);
    let text = session
        .objects()
        .iter()
        .find_map(|o| matches!(o, ObjectSnapshot::Text(_)).then(|| o.id()))
        .unwrap();
    let mask = rect(&session, 0.0, 0.0, 5.0, 5.0);
    session.set_tool(Tool::Trace);
    session.selection.set(&[images[0], text]);
    assert_eq!(session.trace_view().selection, TraceSelection::Mixed);
    session.selection.set(&[images[0], images[1], mask]);
    let view = session.trace_view();
    assert_eq!(view.selection, TraceSelection::Mixed);
    assert_eq!(view.images, []);
    assert_eq!(
        session.create_traced_paths(),
        TraceCreateOutcome::Ignored,
        "the Trace button is dimmed and creates nothing"
    );
    // The images alone still trace (0025 criterion 46).
    session.selection.set(&images);
    assert_eq!(session.trace_view().selection, TraceSelection::Images);
    session.selection.set(&[mask]);
    assert_eq!(session.trace_view().selection, TraceSelection::Other);
}

#[test]
fn c7_a_mask_away_from_the_image_meets_nothing() {
    let mut session = Session::new(1);
    let image = import_images(&mut session, 1)[0];
    let c = corners(&session, image);
    let away = rect(&session, c[2].x + 10.0, c[2].y + 10.0, 5.0, 5.0);
    session.set_tool(Tool::Trace);
    session.selection.set(&[image, away]);
    let view = session.trace_view();
    assert!(view.mask_outside_image);
    assert_eq!(session.trace_mask_json(), "", "no job is started");
}

#[test]
fn c17_outside_is_tool_state_and_inside_again_on_entry() {
    let (mut session, _, _) = masked();
    session.set_tool(Tool::Trace);
    session.set_trace_mask_outside(true);
    assert!(session.trace_mask_outside());
    let json: serde_json::Value = serde_json::from_str(&session.trace_mask_json()).unwrap();
    assert_eq!(json["outside"], true);
    session.set_tool(Tool::Select);
    session.set_tool(Tool::Trace);
    assert!(!session.trace_mask_outside());
}

#[test]
fn c13_shift_click_adds_and_removes_a_mask_path() {
    let mut session = Session::new(1);
    let image = import_images(&mut session, 1)[0];
    let c = corners(&session, image);
    let mask = rect(&session, c[0].x - 20.0, c[0].y, 10.0, 10.0);
    session.set_tool(Tool::Trace);
    session.selection.set(&[image]);
    // The rectangle's edge, outside the image.
    let on_edge = Point::new(c[0].x - 20.0, c[0].y + 5.0);
    session.pointer_down(on_edge, true);
    session.pointer_up(on_edge, true, false);
    assert_eq!(session.trace_view().mask, [mask]);
    let nowhere = Point::new(c[0].x - 200.0, c[0].y - 200.0);
    session.pointer_down(nowhere, true);
    session.pointer_up(nowhere, true, false);
    assert_eq!(
        session.trace_view().mask,
        [mask],
        "empty canvas changes nothing"
    );
    session.pointer_down(on_edge, true);
    session.pointer_up(on_edge, true, false);
    assert_eq!(session.trace_view().mask, []);
    assert_eq!(session.trace_view().images, [image]);
}

#[test]
fn c12_create_paths_leaves_the_mask_paths_and_undoes_in_one_step() {
    let (mut session, image, mask) = masked();
    session.set_tool(Tool::Trace);
    let source = session.trace_source(image, false).unwrap();
    let region = session.trace_region().unwrap();
    // A black square over the middle of the image, as a 16 x 16 px picture over its frame.
    let picture: Vec<u8> = (0..16 * 16)
        .flat_map(|i| {
            let (x, y) = (i % 16, i / 16);
            let v = if (4..12).contains(&x) && (4..12).contains(&y) {
                0
            } else {
                255
            };
            [v, v, v, 255]
        })
        .collect();
    let map = curvyo_document_core::Affine::from_array(source.map)
        .compose(curvyo_document_core::Affine::scale(
            f64::from(source.width) / 16.0,
            f64::from(source.height) / 16.0,
        ))
        .to_array();
    let wire = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 16,
                height: 16,
                rgba: &picture,
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Mono(
                // The image is under a millimetre wide: no speck is ignored.
                &TraceParams {
                    specks: curvyo_ui_core::TraceMm::from_mm(0.0),
                    ..TraceParams::with_threshold(128)
                },
            ),
            lines: &[],
            mask: Some(&region),
            shapes: false,
        },
        &mut |_| {},
    );
    let read = read_trace_wire(&wire).unwrap();
    assert_ne!(read.outlines.len(), 0);
    let c = corners(&session, image);
    let half = f64::midpoint(c[0].x, c[2].x);
    for b in read.outlines.iter().map(kurbo::Shape::bounding_box) {
        assert!(b.x1 <= half + 0.01, "the result lies in the mask");
    }
    let before = session.objects();
    let mask_before = before.iter().find(|o| o.id() == mask).cloned();
    assert!(session.set_trace_result(image, &wire));
    assert!(matches!(
        session.create_traced_paths(),
        TraceCreateOutcome::Created { images: 1, .. }
    ));
    let after = session.objects();
    assert_eq!(after.iter().find(|o| o.id() == mask).cloned(), mask_before);
    assert!(!session.selection.ids().contains(&mask));
    session.undo();
    assert_eq!(session.objects().len(), before.len());
}

#[test]
fn c5_c6_overlapping_and_open_paths_make_one_region() {
    let mut session = Session::new(1);
    let image = import_images(&mut session, 1)[0];
    let c = corners(&session, image);
    let (w, h) = (c[2].x - c[0].x, c[2].y - c[0].y);
    let a = rect(&session, c[0].x, c[0].y, w * 0.6, h);
    let b = rect(&session, c[0].x + w * 0.4, c[0].y, w * 0.6, h);
    session.set_tool(Tool::Trace);
    session.selection.set(&[image, a, b]);
    assert_eq!(session.trace_region().unwrap().polygons().len(), 1);
    // A straight open path encloses nothing: it is dropped, the rectangle stays the region.
    let line = session.document.create_path(
        &[c[0], c[2]].map(|point| curvyo_document_core::NewAnchor {
            id: curvyo_document_core::AnchorId::new(9, 1),
            point,
            handle_in: curvyo_document_core::Vec2::ZERO,
            handle_out: curvyo_document_core::Vec2::ZERO,
            kind: curvyo_document_core::AnchorKind::Corner,
        }),
        false,
    );
    session.selection.set(&[image, a, line]);
    assert_eq!(session.trace_view().mask.len(), 2);
    assert!(session.trace_region().is_some());
    session.selection.set(&[image, line]);
    assert!(session.trace_view().mask_outside_image);
}

#[test]
fn c2_overlay_washes_the_image_outside_the_region() {
    let (mut session, image, _) = masked();
    session.set_tool(Tool::Trace);
    let mut objects: Vec<ObjectSnapshot> = session.objects().to_vec();
    session.trace_artwork(&mut objects);
    let carrying = objects.iter().filter(|o| o.id() == image).count();
    assert_eq!(
        carrying, 3,
        "the image, its wash and the wash outside the region"
    );
    session.selection.set(&[image]);
    let mut objects: Vec<ObjectSnapshot> = session.objects().to_vec();
    session.trace_artwork(&mut objects);
    assert_eq!(objects.iter().filter(|o| o.id() == image).count(), 2);
}

/// Criterion 14 and Terms ("Mask path"): a linked clone of an image or a text does not exist (Linked
/// clone refuses both, `specs/0058` criterion 38; a text clone of `0080` is a text, refused with
/// the criterion 14 row); a linked clone of a path is a mask path.
#[test]
fn c14_only_a_clone_of_a_path_is_a_mask_path() {
    let mut session = super::text_tests::with_text("Name");
    let image = import_images(&mut session, 1)[0];
    let text = session
        .objects()
        .iter()
        .find_map(|o| matches!(o, ObjectSnapshot::Text(_)).then(|| o.id()))
        .unwrap();
    let path = rect(&session, 0.0, 0.0, 5.0, 5.0);
    let clone = |session: &mut Session, id: NodeId| {
        session.set_tool(Tool::Select);
        session.selection.set(&[id]);
        let kind = session.linked_clone().kind;
        (kind, session.selection.ids()[0])
    };
    // Since `specs/0100` an image and a text are originals too; their clones are no mask path.
    let (kind, image_clone) = clone(&mut session, image);
    assert_eq!(kind, super::clones::CloneKind::Done);
    let (kind, _) = clone(&mut session, text);
    assert_eq!(kind, super::clones::CloneKind::Done);
    let (kind, path_clone) = clone(&mut session, path);
    assert_eq!(kind, super::clones::CloneKind::Done);
    assert!(matches!(
        session.objects().iter().find(|o| o.id() == path_clone),
        Some(ObjectSnapshot::Clone(_))
    ));
    session.set_tool(Tool::Trace);
    session.selection.set(&[image, text]);
    assert_eq!(session.trace_view().selection, TraceSelection::Mixed);
    session.selection.set(&[image, path_clone]);
    let view = session.trace_view();
    assert_eq!(view.selection, TraceSelection::Images);
    assert_eq!(view.mask, [path_clone]);
    session.selection.set(&[image, image_clone]);
    assert!(!session.trace_view().mask.contains(&image_clone));
}

/// Terms ("Mask path"): a group counts as the paths in it (architect review 2026-11-04).
#[test]
fn a_grouped_mask_path_counts_as_its_paths() {
    let mut session = Session::new(1);
    let image = import_images(&mut session, 1)[0];
    let c = corners(&session, image);
    let (w, h) = (c[2].x - c[0].x, c[2].y - c[0].y);
    // `a` reaches 5 mm left of the picture, so its left edge is hit outside the image.
    let a = rect(&session, c[0].x - 5.0, c[0].y, w * 0.3 + 5.0, h);
    let b = rect(&session, c[0].x + w * 0.6, c[0].y, w * 0.3, h);
    session.selection.set(&[a, b]);
    assert_eq!(
        session.group_selection().kind,
        super::grouping::GroupingKind::Grouped
    );
    let leaves = session.selection.ids().to_vec();
    assert_eq!(leaves.len(), 2);
    session.set_tool(Tool::Trace);
    // A Shift+click on the group in the Trace tool adds its leaves.
    session.selection.set(&[image]);
    let on_edge = Point::new(c[0].x - 5.0, f64::midpoint(c[0].y, c[2].y));
    session.pointer_down(on_edge, true);
    session.pointer_up(on_edge, true, false);
    let view = session.trace_view();
    assert_eq!(view.selection, TraceSelection::Images);
    assert_eq!(view.mask.len(), 2, "both paths of the group");
    assert_eq!(session.trace_region().unwrap().polygons().len(), 2);
}

/// A preview whose clip the kernel refused names the shapes kept whole (tester 2026-11-04).
#[test]
fn a_refused_clip_shows_its_muted_notice() {
    let (mut session, image, _) = masked();
    session.set_tool(Tool::Trace);
    let mut wire = vec![0.0; curvyo_ui_core::TRACE_WIRE_HEADER];
    assert!(session.set_trace_preview(image, &wire));
    assert_eq!(session.trace_unclipped_notice(), "");
    wire.extend([-5.0, 2.0]);
    assert!(session.set_trace_preview(image, &wire));
    assert_eq!(
        session.trace_unclipped_notice(),
        "2 shapes cross the mask edge unclipped."
    );
}
