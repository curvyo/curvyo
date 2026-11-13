//! The Trace tool with stored regions (`specs/0094-masked-composite-trace` slice 1 criteria 2, 5,
//! 7, 12 and 17 as `specs/0114-image-trace-mask` keeps them; `0114` criteria 15 and 21): the
//! regions of the image limit the trace, other selected objects are passed over.

#![allow(clippy::many_single_char_names)]

use curvyo_document_core::{
    Length, NewTraceRegion, NodeId, ObjectSnapshot, Point, RectBounds, TracePartRef,
    TraceRegionKind, UnitPoint,
};
use curvyo_ui_core::read_trace_wire;
use vectrace::job::{Job as TraceJob, Mode as TraceMode, run as run_trace_job};
use vectrace::{PixelMap as TracePixelMap, Raster as TraceRaster, TraceParams};

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

/// Stores a region of `kind` on `image` with corners in the image's unit square, in its first
/// part (the implicit one on an untouched image).
fn region(session: &Session, image: NodeId, kind: TraceRegionKind, points: &[(f64, f64)]) {
    let mask = session.document.trace_mask(image).unwrap();
    let implicit = curvyo_document_core::ImplicitTracePart::default();
    let part = mask
        .parts
        .first()
        .map_or(TracePartRef::Implicit(&implicit), |p| {
            TracePartRef::Stored(&p.id)
        });
    session
        .document
        .draw_trace_region(
            image,
            part,
            &NewTraceRegion {
                kind,
                subtract: false,
                points: points.iter().map(|&(u, v)| UnitPoint::new(u, v)).collect(),
            },
        )
        .unwrap();
}

/// One image with a rectangle region over its left half, selected in the Trace tool.
fn masked() -> (Session, NodeId) {
    let mut session = Session::new(1);
    let image = import_images(&mut session, 1)[0];
    region(
        &session,
        image,
        TraceRegionKind::Rect,
        &[(-0.1, -0.1), (0.5, 1.1)],
    );
    session.selection.set(&[image]);
    (session, image)
}

/// `0094` criterion 1 as `0114` keeps it: the stored region limits the trace of the image; an
/// image without regions has no mask.
#[test]
fn c1_the_stored_region_masks_the_image() {
    let (mut session, image) = masked();
    assert!(session.trace_available(), "the Select bar's Trace button");
    session.set_tool(Tool::Trace);
    let view = session.trace_view();
    assert_eq!(view.selection, TraceSelection::Images);
    assert_eq!((view.images, view.shown), (vec![image], Some(image)));
    assert_eq!(view.regions, 1);
    assert!(!view.mask_outside_image);
    assert!(session.trace_source(image, false).is_some());
    let json: serde_json::Value = serde_json::from_str(&session.trace_mask_json()).unwrap();
    assert_eq!(json["outside"], false);
    let mut plain = Session::new(1);
    let other = import_images(&mut plain, 1)[0];
    plain.selection.set(&[other]);
    plain.set_tool(Tool::Trace);
    assert_eq!(plain.trace_view().regions, 0);
    assert_eq!(plain.trace_mask_json(), "");
}

/// `0114` criterion 21: an image with a text, a path or a second image traces the images as
/// for the images alone; no mask is read from the other objects and nothing is refused.
#[test]
fn c21_other_objects_beside_the_images_are_passed_over() {
    let mut session = super::text_tests::with_text("Name");
    let images = import_images(&mut session, 2);
    let text = session
        .objects()
        .iter()
        .find_map(|o| matches!(o, ObjectSnapshot::Text(_)).then(|| o.id()))
        .unwrap();
    let path = rect(&session, 0.0, 0.0, 5.0, 5.0);
    session.set_tool(Tool::Trace);
    session.selection.set(&[images[0], text]);
    let view = session.trace_view();
    assert_eq!(view.selection, TraceSelection::Images);
    assert_eq!(view.images, [images[0]]);
    assert_eq!(session.trace_mask_json(), "", "the text is no mask");
    session.selection.set(&[images[0], images[1], path]);
    let view = session.trace_view();
    assert_eq!(view.selection, TraceSelection::Images);
    assert_eq!(view.images, images);
    assert_eq!(session.trace_mask_json(), "", "the path is no mask");
    session.selection.set(&[path]);
    assert_eq!(session.trace_view().selection, TraceSelection::Other);
    session.set_tool(Tool::Select);
    session.selection.set(&[images[0], text]);
    assert!(session.trace_available(), "an image in the selection");
}

/// `0094` criterion 7 with a region: a region away from the image meets nothing, no job starts.
#[test]
fn c7_a_region_away_from_the_image_meets_nothing() {
    let mut session = Session::new(1);
    let image = import_images(&mut session, 1)[0];
    region(
        &session,
        image,
        TraceRegionKind::Rect,
        &[(1.5, 1.5), (2.0, 2.0)],
    );
    session.set_tool(Tool::Trace);
    session.selection.set(&[image]);
    let view = session.trace_view();
    assert!(view.mask_outside_image);
    assert_eq!(session.trace_mask_json(), "", "no job is started");
}

/// `0094` criterion 17 with ADR 0061 §3: Outside on an untouched image is tool state, Inside again
/// on entry; on a stored part it is the part's and comes back.
#[test]
fn c17_outside_is_tool_state_until_a_part_is_stored() {
    let mut session = Session::new(1);
    let image = import_images(&mut session, 1)[0];
    session.selection.set(&[image]);
    session.set_tool(Tool::Trace);
    session.set_trace_mask_outside(true);
    assert!(session.trace_mask_outside());
    session.set_tool(Tool::Select);
    session.set_tool(Tool::Trace);
    assert!(!session.trace_mask_outside());
    region(
        &session,
        image,
        TraceRegionKind::Rect,
        &[(0.0, 0.0), (0.5, 1.0)],
    );
    session.set_trace_mask_outside(true);
    let json: serde_json::Value = serde_json::from_str(&session.trace_mask_json()).unwrap();
    assert_eq!(json["outside"], true);
    session.set_tool(Tool::Select);
    session.set_tool(Tool::Trace);
    assert!(
        session.trace_mask_outside(),
        "the stored part keeps Outside"
    );
}

/// `0094` criterion 12 and `0114` criterion 19: Create paths cuts to the region, leaves it on
/// the image and undoes in one step.
#[test]
fn c12_create_paths_leaves_the_regions_and_undoes_in_one_step() {
    let (mut session, image) = masked();
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
                    specks: vectrace::Mm::from_mm(0.0),
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
    let mask_before = session.document.trace_mask(image);
    assert!(session.set_trace_result(image, &wire));
    assert!(matches!(
        session.create_traced_paths(),
        TraceCreateOutcome::Created { images: 1, .. }
    ));
    assert_eq!(
        session.document.trace_mask(image),
        mask_before,
        "the regions stay"
    );
    session.undo();
    assert_eq!(session.objects().len(), before.len());
}

/// `0094` criteria 5 and 6 with regions: overlapping regions make one region; a flat lasso
/// encloses nothing and is dropped, the other region stays the mask.
#[test]
fn c5_c6_overlapping_regions_make_one_region_and_a_flat_lasso_is_dropped() {
    let mut session = Session::new(1);
    let image = import_images(&mut session, 1)[0];
    region(
        &session,
        image,
        TraceRegionKind::Rect,
        &[(0.0, 0.0), (0.6, 1.0)],
    );
    region(
        &session,
        image,
        TraceRegionKind::Rect,
        &[(0.4, 0.0), (1.0, 1.0)],
    );
    session.set_tool(Tool::Trace);
    session.selection.set(&[image]);
    assert_eq!(session.trace_region().unwrap().polygons().len(), 1);
    let mut flat = Session::new(1);
    let alone = import_images(&mut flat, 1)[0];
    region(
        &flat,
        alone,
        TraceRegionKind::Lasso,
        &[(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)],
    );
    flat.set_tool(Tool::Trace);
    flat.selection.set(&[alone]);
    assert!(flat.trace_view().mask_outside_image, "nothing to keep");
    region(
        &flat,
        alone,
        TraceRegionKind::Rect,
        &[(0.0, 0.0), (0.5, 1.0)],
    );
    assert_eq!(flat.trace_view().regions, 2);
    assert!(flat.trace_region().is_some());
}

/// `0094` criterion 2 with a region: Overlay washes the image outside the region.
#[test]
fn c2_overlay_washes_the_image_outside_the_region() {
    let (mut session, image) = masked();
    session.set_tool(Tool::Trace);
    let mut objects: Vec<ObjectSnapshot> = session.objects().to_vec();
    session.trace_artwork(&mut objects);
    let carrying = objects.iter().filter(|o| o.id() == image).count();
    assert_eq!(
        carrying, 3,
        "the image, its wash and the wash outside the region"
    );
    let mut plain = Session::new(1);
    let other = import_images(&mut plain, 1)[0];
    plain.selection.set(&[other]);
    plain.set_tool(Tool::Trace);
    let mut objects: Vec<ObjectSnapshot> = plain.objects().to_vec();
    plain.trace_artwork(&mut objects);
    assert_eq!(objects.iter().filter(|o| o.id() == other).count(), 2);
}

/// `0114` criterion 15 (CLONE-MASK A): a linked clone of an image is traceable and shows its
/// root's regions under its own placement; a clone of a path is not traced.
#[test]
fn c15_an_image_clone_is_traced_with_its_root_s_mask() {
    let (mut session, image) = masked();
    let path = rect(&session, 0.0, 0.0, 5.0, 5.0);
    let clone = |session: &mut Session, id: NodeId| {
        session.set_tool(Tool::Select);
        session.selection.set(&[id]);
        let kind = session.linked_clone().kind;
        assert_eq!(kind, super::clones::CloneKind::Done);
        session.selection.ids()[0]
    };
    let image_clone = clone(&mut session, image);
    let path_clone = clone(&mut session, path);
    session.set_tool(Tool::Trace);
    session.selection.set(&[image_clone]);
    let view = session.trace_view();
    assert_eq!(view.selection, TraceSelection::Images);
    assert_eq!(view.images, [image_clone]);
    assert_eq!(view.regions, 1, "the root's region");
    assert!(session.trace_source(image_clone, false).is_some());
    assert!(session.trace_region().is_some());
    session.selection.set(&[path_clone]);
    assert_eq!(session.trace_view().selection, TraceSelection::Other);
}

/// A preview whose clip the kernel refused names the shapes kept whole (tester 2026-11-04).
#[test]
fn a_refused_clip_shows_its_muted_notice() {
    let (mut session, image) = masked();
    session.set_tool(Tool::Trace);
    let mut wire = vec![0.0; vectrace::wire::HEADER];
    assert!(session.set_trace_preview(image, &wire));
    assert_eq!(session.trace_unclipped_notice(), "");
    wire.extend([-5.0, 2.0]);
    assert!(session.set_trace_preview(image, &wire));
    assert_eq!(
        session.trace_unclipped_notice(),
        "2 shapes cross the mask edge unclipped."
    );
}
