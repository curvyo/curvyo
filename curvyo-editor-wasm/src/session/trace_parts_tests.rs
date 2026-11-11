//! The parts of a composite trace in the Trace tool (`specs/0094-masked-composite-trace` slice 2,
//! criteria 18, 19, 24, 27 to 29 and 42).

use curvyo_document_core::{Affine, Length, NodeId, ObjectSnapshot, Point, RectBounds};
use curvyo_ui_core::{TracePartClip, read_trace_wire};
use vectrace::job::{Job as TraceJob, Mode as TraceMode, run as run_trace_job};
use vectrace::mask::Mask;
use vectrace::{PixelMap as TracePixelMap, Raster as TraceRaster, TraceParams};

use super::trace_parts::TRACE_PARTS_MAX;
use super::{Session, Tool, TraceCreateOutcome};

const PNG: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");

fn image_of(session: &mut Session) -> NodeId {
    session.import_begin(None);
    session.import_add(PNG, "logo.png");
    session.import_finish(false);
    session
        .objects()
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.id),
            _ => None,
        })
        .unwrap()
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

/// One image in the Trace tool, a rectangle over its left half (`left`) and one over its right
/// half (`right`), the image alone selected.
fn halves() -> (Session, NodeId, NodeId, NodeId) {
    let mut session = Session::new(1);
    let image = image_of(&mut session);
    let c = corners(&session, image);
    let (w, h) = (c[2].x - c[0].x, c[2].y - c[0].y);
    let left = rect(&session, c[0].x - 1.0, c[0].y - 1.0, w / 2.0 + 1.0, h + 2.0);
    let right = rect(
        &session,
        c[0].x + w / 2.0,
        c[0].y - 1.0,
        w / 2.0 + 1.0,
        h + 2.0,
    );
    session.set_tool(Tool::Trace);
    session.selection.set(&[image]);
    (session, image, left, right)
}

fn ids(session: &Session) -> Vec<NodeId> {
    sorted(session.selection.ids().to_vec())
}

fn sorted(mut ids: Vec<NodeId>) -> Vec<NodeId> {
    ids.sort_by_key(|id| format!("{id:?}"));
    ids
}

#[test]
fn c18_one_part_whole_image_and_add_stops_at_eight() {
    let (mut session, _, _, _) = halves();
    let parts = session.trace_parts();
    assert_eq!(parts.len(), 1);
    assert!(parts[0].active && parts[0].whole && !parts[0].muted);
    assert!(parts[0].contributes);
    for n in 2..=TRACE_PARTS_MAX {
        assert!(session.add_trace_part().is_some());
        assert_eq!(session.trace_part_count(), n);
    }
    assert_eq!(session.add_trace_part(), None, "Add part is dimmed at 8");
    assert_eq!(session.trace_part_count(), TRACE_PARTS_MAX);
}

#[test]
fn c19_a_new_part_is_active_without_paths_and_takes_the_selected_paths() {
    let (mut session, image, left, right) = halves();
    session.selection.set(&[image, left]);
    session.set_trace_mask_outside(true);
    let first = session.trace_parts()[0].id;
    let second = session.add_trace_part().unwrap();
    assert_ne!(first, second);
    let parts = session.trace_parts();
    assert_eq!(parts.len(), 2);
    assert!(parts[1].active && parts[1].muted && !parts[1].contributes);
    assert_eq!(parts[1].paths, 0);
    assert!(!parts[1].outside, "a new part is Inside");
    assert!(!session.trace_mask_outside());
    assert_eq!(
        session.selection.ids(),
        [image],
        "only the image stays selected"
    );
    // Selecting image plus paths assigns them to the active part.
    session.selection.set(&[image, right]);
    let parts = session.trace_parts();
    assert_eq!((parts[0].paths, parts[1].paths), (1, 1));
    assert!(parts[1].contributes && !parts[1].muted);
    // A chip pick loads its mode and selects its paths with the image.
    assert!(session.set_trace_active_part(0));
    assert_eq!(ids(&session), sorted(vec![image, left]));
    assert!(session.trace_mask_outside());
    assert!(session.trace_parts()[0].active);
    assert!(session.set_trace_active_part(1));
    assert_eq!(ids(&session), sorted(vec![image, right]));
    assert!(!session.trace_mask_outside());
    assert!(!session.set_trace_active_part(5));
}

/// Criterion 21 in the session: the earlier part's clip is cut by the later part.
#[test]
fn the_parts_clips_take_out_the_later_parts() {
    let (mut session, image, left, right) = halves();
    let c = corners(&session, image);
    let mid = Point::new(f64::midpoint(c[0].x, c[2].x), f64::midpoint(c[0].y, c[2].y));
    let quarter = Point::new(c[0].x + (c[2].x - c[0].x) * 0.25, mid.y);
    let three = Point::new(c[0].x + (c[2].x - c[0].x) * 0.75, mid.y);
    session.add_trace_part();
    session.selection.set(&[image, left]);
    let clips = session.trace_part_clip_list();
    let TracePartClip::Region(rest) = &clips[0] else {
        panic!("{clips:?}")
    };
    assert!(rest.keeps(three) && !rest.keeps(quarter));
    assert!(clips[1].region().is_some_and(|r| r.keeps(quarter)));
    assert_ne!(session.trace_part_mask_json(0), "");
    // A third part over the right half: the whole image keeps nothing.
    session.add_trace_part();
    session.selection.set(&[image, right]);
    let clips = session.trace_part_clip_list();
    assert_eq!(clips[0], TracePartClip::Nothing);
    assert!(!session.trace_parts()[0].contributes);
    assert_eq!(session.trace_part_mask_json(0), "");
}

/// Criterion 27 by the routes the Trace tool has: the Layers row's Delete (an inactive part's
/// path, not selected) and Undo (the active part's path, selected).
#[test]
fn c27_a_deleted_mask_path_leaves_its_part() {
    let (mut session, image, left, _) = halves();
    session.selection.set(&[image, left]);
    session.add_trace_part();
    let c = corners(&session, image);
    let (w, h) = (c[2].x - c[0].x, c[2].y - c[0].y);
    // A path drawn while the tool is open, then undone: the active part loses it.
    let drawn = session.document.batch("create_rect", |document| {
        Some(document.create_rect(RectBounds {
            origin: Point::new(c[0].x + w / 2.0, c[0].y - 1.0),
            width: Length::from_mm(w / 2.0 + 1.0),
            height: Length::from_mm(h + 2.0),
        }))
    });
    let drawn = drawn.unwrap();
    session.selection.set(&[image, drawn]);
    assert_eq!(session.trace_parts()[1].paths, 1);
    session.undo();
    assert!(
        session.objects().iter().all(|o| o.id() != drawn),
        "the undo took the path"
    );
    let parts = session.trace_parts();
    assert!(parts[1].muted && !parts[1].contributes, "{parts:?}");
    assert_eq!(
        session.trace_view().selection,
        super::TraceSelection::Images,
        "no error row"
    );
    let key = |id: NodeId| serde_json::to_string(&id).unwrap();
    session.delete_layer_row(&key(left));
    let parts = session.trace_parts();
    assert_eq!(parts[0].paths, 0);
    assert!(
        parts[0].whole,
        "the first part covers the whole image again"
    );
    assert_eq!(
        session.trace_view().selection,
        super::TraceSelection::Images,
        "no error row"
    );
    assert!(session.set_trace_active_part(0));
    assert_eq!(session.selection.ids(), [image]);
}

/// A selected id whose node is gone (a deletion that did not prune the selection) is passed
/// over: the image with its other mask paths still traces (criterion 27, no error appears).
#[test]
fn a_gone_selected_id_is_passed_over() {
    let (mut session, image, left, right) = halves();
    session.selection.set(&[image, left, right]);
    session.document.delete_objects(&[left]).unwrap();
    let view = session.trace_view();
    assert_eq!(view.selection, super::TraceSelection::Images);
    assert_eq!(view.mask, [right]);
}

#[test]
fn c28_removing_a_part_renumbers_and_activates_the_one_before() {
    let (mut session, image, left, right) = halves();
    assert!(!session.remove_trace_part(0), "one part stays");
    session.add_trace_part();
    session.selection.set(&[image, left]);
    let third = session.add_trace_part().unwrap();
    session.selection.set(&[image, right]);
    assert!(session.remove_trace_part(1));
    let parts = session.trace_parts();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[1].id, third, "the parts renumber, their ids stay");
    assert!(parts[1].active && parts[1].paths == 1);
    // Removing the active part activates the one before it with its selection.
    assert!(session.remove_trace_part(1));
    assert_eq!(session.trace_part_count(), 1);
    assert!(session.trace_parts()[0].active);
    assert_eq!(session.selection.ids(), [image]);
}

#[test]
fn c29_leaving_the_tool_leaves_one_part() {
    let (mut session, image, left, _) = halves();
    session.add_trace_part();
    session.selection.set(&[image, left]);
    session.set_tool(Tool::Select);
    session.set_tool(Tool::Trace);
    assert_eq!(session.trace_part_count(), 1);
    let parts = session.trace_parts();
    assert_eq!(parts[0].paths, 1, "the selected paths are part 1's mask");
    assert!(parts[0].active);
}

#[test]
fn c42_clear_mask_and_picks_change_the_active_part_only() {
    let (mut session, image, left, right) = halves();
    session.selection.set(&[image, left]);
    session.add_trace_part();
    session.set_trace_mask_pick(true);
    session.selection.set(&[image, right]);
    session.clear_trace_mask();
    let parts = session.trace_parts();
    assert_eq!((parts[0].paths, parts[1].paths), (1, 0));
    assert!(session.trace_mask_pick(), "the mode stays armed");
    session.set_trace_active_part(0);
    assert!(
        session.trace_mask_pick(),
        "a chip pick leaves the mode armed"
    );
    session.add_trace_part();
    assert!(session.trace_mask_pick(), "Add part leaves the mode armed");
}

/// A 16 x 16 px picture over the image's frame: a black square in the middle.
fn square_picture() -> Vec<u8> {
    (0..16 * 16)
        .flat_map(|i| {
            let (x, y) = (i % 16, i / 16);
            let v = if (4..12).contains(&x) && (4..12).contains(&y) {
                0
            } else {
                255
            };
            [v, v, v, 255]
        })
        .collect()
}

/// Criteria 24 and 25 in the session: the parts' wires joined into the preview and the result;
/// Create paths writes one group in one step, one undo restores the document.
#[test]
fn c24_the_joined_parts_are_one_group_in_one_step() {
    let (mut session, image, left, _) = halves();
    session.add_trace_part();
    session.selection.set(&[image, left]);
    let source = session.trace_source(image, false).unwrap();
    let map = Affine::from_array(source.map)
        .compose(Affine::scale(
            f64::from(source.width) / 16.0,
            f64::from(source.height) / 16.0,
        ))
        .to_array();
    let params = TraceParams {
        specks: vectrace::Mm::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    };
    let clips = session.trace_part_clip_list();
    let wires: Vec<Vec<f64>> = clips
        .iter()
        .map(|clip| {
            run_trace_job(
                &TraceJob {
                    raster: TraceRaster {
                        width: 16,
                        height: 16,
                        rgba: &square_picture(),
                    },
                    map: TracePixelMap::new(map),
                    mode: TraceMode::Mono(&params),
                    lines: &[],
                    mask: clip.region().map(|r| r as &dyn Mask),
                    shapes: false,
                },
                &mut |_| {},
            )
        })
        .collect();
    let refs: Vec<Option<&[f64]>> = wires.iter().map(|w| Some(w.as_slice())).collect();
    let header = session.set_trace_joined(image, &refs, true).unwrap();
    assert_eq!(header[0], 0.0, "traced");
    assert_eq!(header[1], 1.0, "the square's halves are one path");
    let whole = read_trace_wire(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 16,
                height: 16,
                rgba: &square_picture(),
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Mono(&params),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    ))
    .unwrap();
    assert_eq!(whole.paths, 1);
    let before = session.objects().len();
    let left_before = session.objects().iter().find(|o| o.id() == left).cloned();
    assert!(matches!(
        session.create_traced_paths(),
        TraceCreateOutcome::Created {
            images: 1,
            paths: 1,
            ..
        }
    ));
    assert_eq!(session.objects().len(), before + 1, "one group's path");
    assert_eq!(
        session.objects().iter().find(|o| o.id() == left).cloned(),
        left_before,
        "the mask path is unchanged"
    );
    session.undo();
    assert_eq!(session.objects().len(), before);
}

/// Criterion 31: a part with a region inside the picture is traced from its crop, with the map
/// moved so its outlines land where the whole picture's would; part 1 (the whole image minus
/// part 2) is the whole picture.
#[test]
fn c31_a_part_inside_the_picture_is_cropped() {
    let (width, height) = (100_u32, 80_u32);
    let rgba: Vec<u8> = (0..width * height)
        .flat_map(|i| {
            let v = u8::try_from(i % 251).unwrap();
            [v, v / 2, v / 3, 255]
        })
        .collect();
    let map = [0.5, 0.0, 0.0, 0.5, 10.0, 20.0];
    let source = super::TraceSource {
        width,
        height,
        rgba: rgba.clone(),
        map,
    };
    // Pixels 40..60 by 30..50 in mm.
    let px = |x: f64, y: f64| Point::new(10.0 + 0.5 * x, 20.0 + 0.5 * y);
    let polygon = vec![
        px(40.0, 30.0),
        px(60.0, 30.0),
        px(60.0, 50.0),
        px(40.0, 50.0),
    ];
    let cut = super::trace_parts::crop_to(source.clone(), &[polygon], 16.0);
    assert_eq!((cut.width, cut.height), (52, 52), "16 px of margin around");
    let moved = Affine::from_array(cut.map).apply_point(Point::new(0.0, 0.0));
    assert_eq!(moved, px(24.0, 14.0));
    let at = |rgba: &[u8], w: u32, x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        rgba[i..i + 4].to_vec()
    };
    assert_eq!(at(&cut.rgba, cut.width, 0, 0), at(&rgba, width, 24, 14));
    assert_eq!(at(&cut.rgba, cut.width, 51, 51), at(&rgba, width, 75, 65));
    // A box that reaches past the picture is clamped; one that is the picture is no crop.
    let all = vec![
        px(-5.0, -5.0),
        px(200.0, -5.0),
        px(200.0, 200.0),
        px(-5.0, 200.0),
    ];
    assert_eq!(
        super::trace_parts::crop_to(source.clone(), &[all], 16.0),
        source
    );
    // In the session: the whole image minus a later part is the whole picture.
    let (mut session, image, left, _) = halves();
    let whole = session.trace_source(image, false).unwrap();
    session.add_trace_part();
    session.selection.set(&[image, left]);
    assert_eq!(
        session
            .trace_part_source(image, 0, false, 0.1, false)
            .unwrap(),
        whole
    );
    assert!(
        session
            .trace_part_source(image, 1, false, 0.1, false)
            .is_some()
    );
}

/// A 1000 x 1000 px picture, 10 px per mm: black discs of 6 mm on a 10 mm grid and a thin bar of
/// 0.3 mm across the whole width at y = 50 mm.
fn grid_picture() -> Vec<u8> {
    (0..1000 * 1000)
        .flat_map(|i| {
            let (x, y) = (f64::from(i % 1000) + 0.5, f64::from(i / 1000) + 0.5);
            let (cx, cy) = (
                (x / 100.0).floor() * 100.0 + 50.0,
                (y / 100.0).floor() * 100.0 + 50.0,
            );
            let ink = (x - cx).hypot(y - cy) <= 30.0 || (497.0..500.0).contains(&y);
            let v = if ink { 0 } else { 255 };
            [v, v, v, 255]
        })
        .collect()
}

/// Criterion 31 (adrs.md slice 2 note): a part traced from its crop is the part traced from the
/// whole picture, within 0.1 % of its area: with Ignore below larger than the margin (the thin bar
/// enters the clip from outside the crop) and with Upscale Off; with Upscale Auto the crop that
/// would be upscaled while the picture is not is not cut.
#[test]
fn c31_a_cropped_part_traces_as_the_whole_picture_does() {
    use curvyo_document_core::Vec2;
    use curvyo_ui_core::TraceRegion;
    use vectrace::Upscale as TraceUpscale;
    let rgba = grid_picture();
    let map = [0.1, 0.0, 0.0, 0.1, 0.0, 0.0];
    let source = super::TraceSource {
        width: 1000,
        height: 1000,
        rgba: rgba.clone(),
        map,
    };
    let corner = |x: f64, y: f64| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO);
    let square = vec![
        corner(30.0, 30.0),
        corner(70.0, 30.0),
        corner(70.0, 70.0),
        corner(30.0, 70.0),
    ];
    let region = TraceRegion::of_operands(&[vec![(square, true)]], false).unwrap();
    // Ignore below 1 mm: 1 mm² is 100 px, more than the 16 px margin.
    let params = TraceParams {
        specks: vectrace::Mm::from_mm(1.0),
        upscale: TraceUpscale::Off,
        ..TraceParams::with_threshold(128)
    };
    let margin = (1.0_f64 / 0.01).ceil();
    let cut = super::trace_parts::crop_to(source, region.polygons(), margin);
    assert!(cut.width < 1000, "the part is cut: {}", cut.width);
    let read = |rgba: &[u8], w: u32, h: u32, map: [f64; 6]| {
        read_trace_wire(&run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: w,
                    height: h,
                    rgba,
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Mono(&params),
                lines: &[],
                mask: Some(&region),
                shapes: false,
            },
            &mut |_| {},
        ))
        .unwrap()
    };
    let whole = read(&rgba, 1000, 1000, map);
    let cropped = read(&cut.rgba, cut.width, cut.height, cut.map);
    // The painted area of both, sampled every 0.2 mm over the clip.
    let paints = |outlines: &[kurbo::BezPath], x: f64, y: f64| {
        outlines
            .iter()
            .map(|o| kurbo::Shape::winding(o, kurbo::Point::new(x, y)))
            .sum::<i32>()
            != 0
    };
    let (mut painted, mut differ) = (0_u32, 0_u32);
    for i in 0..200 {
        for j in 0..200 {
            let (x, y) = (30.1 + 0.2 * f64::from(i), 30.1 + 0.2 * f64::from(j));
            let a = paints(&whole.outlines, x, y);
            painted += u32::from(a);
            differ += u32::from(a != paints(&cropped.outlines, x, y));
        }
    }
    assert!(painted > 2500, "{painted}");
    assert!(
        f64::from(differ) <= 0.001 * f64::from(painted),
        "{differ} of {painted} samples differ"
    );
    assert!(
        whole.outlines.iter().any(|o| {
            let b = kurbo::Shape::bounding_box(o);
            b.width() > 30.0 && b.height() < 1.0
        }),
        "the thin bar is kept"
    );
    // Upscale Auto: the cut-out and the picture fall on the same side of 1 Mpx.
    let (mut session, image, left, _) = halves();
    session.add_trace_part();
    session.selection.set(&[image, left]);
    let whole_source = session.trace_source(image, false).unwrap();
    let auto = session
        .trace_part_source(image, 1, false, 0.1, true)
        .unwrap();
    let upscaled = |s: &super::TraceSource| TraceUpscale::Auto.applies(s.width, s.height);
    assert_eq!(upscaled(&auto), upscaled(&whole_source));
}

/// The Layers row's Delete on a selected mask path while Trace is open deletes that path: the
/// image stays selected and the part loses the path (coordinator 2026-11-06, point 4).
#[test]
fn deleting_a_selected_mask_path_from_its_row_removes_it() {
    let (mut session, image, left, right) = halves();
    session.selection.set(&[image, left, right]);
    let key = |id: NodeId| serde_json::to_string(&id).unwrap();
    session.delete_layer_row(&key(left));
    assert!(
        session.objects().iter().all(|o| o.id() != left),
        "the path is deleted"
    );
    assert!(
        session.objects().iter().any(|o| o.id() == image),
        "the image stays"
    );
    assert_eq!(ids(&session), sorted(vec![image, right]));
    assert_eq!(session.trace_parts()[0].paths, 1);
    assert_eq!(session.tool, Tool::Trace);
}

#[test]
fn unsupported_mask_in_inactive_part_stops_entire_composite() {
    use curvyo_document_core::{AnchorId, CurveKind, NewAnchor};
    let (mut session, image, _, right) = halves();
    // A Spiro through one point has no curve: it cannot be a mask.
    let nodes = [
        NewAnchor::corner(AnchorId::new(181, 1), Point::new(5., 5.)),
        NewAnchor::corner(AnchorId::new(181, 2), Point::new(5., 5.)),
        NewAnchor::corner(AnchorId::new(181, 3), Point::new(5., 5.)),
    ];
    let bad = session
        .document
        .create_editable_curve(&nodes, true, CurveKind::Spiro)
        .unwrap();
    session.selection.set(&[image, bad]);
    session.add_trace_part().unwrap();
    session.selection.set(&[image, right]);
    assert!(session.trace_view().mask_failed);
    assert!(
        session
            .trace_part_clip_list()
            .iter()
            .all(|clip| !clip.contributes())
    );
    assert!(session.set_trace_joined(image, &[], true).is_none());
    assert!(session.trace_source(image, false).is_none());
    assert_eq!(session.create_traced_paths(), TraceCreateOutcome::Ignored);
}
