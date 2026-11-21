//! The Adjustments group in the session (`specs/0135` criteria 1, 2, 9, 11, 12, 18, 25 to 30, 59
//! to 61): the view, the edits as one undo step each, the job reading the adjusted picture, the
//! clone override and the Before and after pane.

use std::sync::Arc;

use curvyo_document_core::{Affine, Direction, NewImage, NodeId, Vec2};
use curvyo_image_core::{WorkingImage, encode_png};
use curvyo_laser_core::adjust::StepKind;
use curvyo_laser_core::job::MoveKind;

use crate::session::Session;

/// A `width` x `height` ramp, light to dark.
pub(in crate::session) fn ramp_png(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let g = u8::try_from((x * 200 / width + y * 3) % 230 + 20).unwrap();
            pixels.extend_from_slice(&[g, g, g, 255]);
        }
    }
    encode_png(&WorkingImage {
        width,
        height,
        pixels,
        original_size: (width, height),
        truncated: false,
        animated: false,
    })
    .unwrap()
}

pub(super) fn laser_page() -> Session {
    let mut s = Session::new(290_135);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    s
}

pub(super) fn place(s: &Session, w: f64, h: f64) -> NodeId {
    let origin = s.active_manufacturing().unwrap().page.unwrap().origin;
    s.document
        .add_images(&[NewImage {
            bytes: Arc::from(ramp_png(40, 20).as_slice()),
            placement: Affine::new(w, 0.0, 0.0, h, origin.x + 10.0, origin.y + 10.0),
            name: "Photo 1".to_owned(),
        }])
        .unwrap()[0]
}

pub(super) fn image_op(s: &mut Session, items: &[NodeId], interval: f64) -> NodeId {
    s.selection.set(items);
    let id = s.press_operation_kind("image", true).unwrap();
    s.selection.set(&[]);
    s.edit_operation_setting(id, "main", "power", 50.0).unwrap();
    s.edit_operation_setting(id, "main", "speed", 6000.0)
        .unwrap();
    let _ = s.edit_operation_option(id, "interval", interval);
    id
}

/// The burned dots of the job, per sweep, as one list.
pub(super) fn dots(s: &Session) -> Vec<Vec<u8>> {
    let job = s.laser_job().unwrap();
    job.built
        .job
        .sweeps
        .iter()
        .map(|sweep| {
            (0..sweep.levels.len())
                .map(|i| sweep.levels.get(i))
                .collect()
        })
        .collect()
}

pub(super) fn undo_label(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

pub(super) fn engraved() -> (Session, NodeId, NodeId) {
    let mut s = laser_page();
    let image = place(&s, 8.0, 4.0);
    let op = image_op(&mut s, &[image], 0.1);
    s.selection.set(&[image]);
    (s, image, op)
}

/// Criterion 1, 25, 59, 60: the group of a new picture.
#[test]
fn c1_a_new_picture_shows_expert_and_an_empty_stack() {
    let mut s = laser_page();
    assert!(s.adjustments_view().is_none(), "nothing selected");
    let image = place(&s, 8.0, 4.0);
    s.selection.set(&[image]);
    let view = s.adjustments_view().unwrap();
    assert_eq!(view.presets.len(), 7);
    assert!(view.presets[6].pressed, "Expert");
    assert_eq!(view.presets[0].tooltip, "Photo (detailed) · Dither: Jarvis");
    assert_eq!(view.presets[4].cell, "Dots");
    assert!(view.strength.is_none() && view.derived.is_none());
    // Slice 2: seven tone and detail kinds, Crop and Resize in row 2.
    assert_eq!(view.add_tone.len(), 7);
    assert_eq!(view.add_tone[2].name, "Add Levels");
    assert_eq!(view.add_geometry.len(), 2);
    assert!(!view.restore_enabled);
    assert_eq!(view.state_line.as_deref(), Some("Not engraved yet"));
    assert_eq!(view.clone_row, None);
}

/// Criteria 25 to 28, 19: a preset pick writes the Dither, derives read-only rows, hides the tone
/// add row, changes the job's dots and is one undo step. Superseded by the Dither on the picture
/// (2026-11-20): the pick writes the picture's Dither, the operation is untouched, no notice.
#[test]
fn c27_a_preset_pick_is_one_step_and_the_job_burns_the_adjusted_picture() {
    let (mut s, _, op) = engraved();
    let plain = dots(&s);
    assert!(s.pick_adjust_preset("photo"));
    assert_eq!(undo_label(&mut s), "Edit adjustments");
    let view = s.adjustments_view().unwrap();
    assert!(view.presets[0].pressed);
    assert_eq!(view.strength.as_ref().unwrap().value, 50);
    let derived = view.derived.unwrap();
    assert_eq!(derived.line, "From preset Photo, Strength 50 %");
    assert!(derived.rows.iter().any(|r| r.kind == "levels"));
    assert!(view.add_tone.is_empty(), "P8");
    assert!(view.restore_enabled);
    assert_eq!(view.state_line, None, "one operation");
    assert_eq!(view.dither.dither, "jarvis", "the picture's Dither strip");
    let options = s
        .document
        .operations(s.active_manufacturing().unwrap().id)
        .unwrap();
    let options = options.iter().find(|o| o.id == op).unwrap();
    assert_eq!(options.options, None, "the operation is untouched");
    assert_eq!(s.take_adjust_notice(), None, "no notice");
    let adjusted = dots(&s);
    assert_ne!(adjusted, plain, "criterion 19");
    s.undo();
    assert_eq!(dots(&s), plain, "undo gives the 0123 dots back");
}

/// Criterion 26: a drag shows the held Strength, the release writes one step.
#[test]
fn c26_a_strength_drag_is_one_step() {
    let (mut s, _, _) = engraved();
    s.pick_adjust_preset("graphics");
    let before = s.document.version();
    s.hold_adjust_strength(80.4);
    s.hold_adjust_strength(90.0);
    assert_eq!(s.document.version(), before, "nothing written while held");
    assert_eq!(s.adjustments_view().unwrap().strength.unwrap().value, 90);
    let held = dots(&s);
    assert!(s.release_adjust_strength());
    assert_eq!(dots(&s), held, "the held dots are the written ones");
    s.undo();
    assert_eq!(s.adjustments_view().unwrap().strength.unwrap().value, 50);
    assert_eq!(s.set_adjust_strength(101.0), Err("Strength is 0 to 100 %."));
    s.hold_adjust_strength(30.0);
    s.cancel_adjust_strength();
    assert!(!s.release_adjust_strength());
}

/// Criteria 2, 3, 5, 30: add, edit, switch, move and remove manual rows.
#[test]
fn c2_manual_rows_edit_in_place_and_show_preset_in_use() {
    let (mut s, _, _) = engraved();
    assert!(s.add_adjust_step("sharpen"));
    assert!(s.add_adjust_step("levels"));
    let rows = s.adjustments_view().unwrap().steps;
    assert_eq!(
        rows.iter().map(|r| r.kind).collect::<Vec<_>>(),
        ["levels", "sharpen"]
    );
    assert!(!rows[0].up && rows[0].down);
    assert_eq!(
        s.edit_adjust_field(1, "amount_pct", 501.0),
        Err("Amount is 0 to 500 %.".to_owned())
    );
    assert_eq!(s.edit_adjust_field(1, "amount_pct", 150.0), Ok(true));
    assert!(s.set_adjust_step_enabled(0, false));
    let rows = s.adjustments_view().unwrap().steps;
    assert!(rows[0].line.starts_with("Off · "), "{}", rows[0].line);
    assert!(rows[0].accessible_name.ends_with(", off"));
    assert!(rows[1].line.contains("Amount 150 %"), "{}", rows[1].line);
    assert_eq!(rows[1].fields.len(), 3);
    assert!(s.move_adjust_step(1, true));
    assert!(s.pick_adjust_preset("photo"));
    let rows = s.adjustments_view().unwrap().steps;
    assert!(rows.iter().all(|r| r.preset_in_use));
    assert!(rows[0].line.starts_with("Preset in use · "));
    assert!(s.remove_adjust_step(0));
    assert_eq!(s.adjustments_view().unwrap().steps.len(), 1);
    assert_eq!(undo_label(&mut s), "Edit adjustments");
}

/// Criterion 2: a drag over two places is one step "Edit adjustments", and one undo takes it back.
#[test]
fn c2_a_drag_is_one_step() {
    let (mut s, _, _) = engraved();
    for kind in ["denoise", "levels", "sharpen"] {
        assert!(s.add_adjust_step(kind));
    }
    let kinds = |s: &Session| -> Vec<&'static str> {
        s.adjustments_view()
            .unwrap()
            .steps
            .iter()
            .map(|r| r.kind)
            .collect()
    };
    let before = kinds(&s);
    assert!(s.move_adjust_step_to(2, 0));
    assert_eq!(kinds(&s), ["sharpen", "denoise", "levels"]);
    assert_eq!(undo_label(&mut s), "Edit adjustments");
    assert!(!s.move_adjust_step_to(1, 1), "its own place writes nothing");
    s.undo();
    assert_eq!(kinds(&s), before, "one undo takes the whole drag back");
}

/// Criteria 5, 16: a Levels row shows the histogram of the picture after the rows above it, and
/// Stretch sets its Black and White point from that histogram in one step.
#[test]
fn c16_levels_shows_its_input_histogram_and_stretch_is_one_step() {
    let (mut s, _, _) = engraved();
    assert!(s.add_adjust_step("levels"));
    assert!(s.add_adjust_step("sharpen"));
    let rows = s.adjustments_view().unwrap().steps;
    assert_eq!(rows[1].histogram, None, "only Levels has a histogram");
    let bars = rows[0].histogram.clone().unwrap();
    assert_eq!(bars.len(), 256);
    // The ramp's greys are 20 to 249.
    assert!(bars[..16].iter().all(|&b| b == 0), "{bars:?}");
    assert!(bars[252..].iter().all(|&b| b == 0), "{bars:?}");
    assert_eq!(bars.iter().max(), Some(&255));
    let fields_of = |s: &Session| s.adjustments_view().unwrap().steps[0].fields.clone();
    let unstretched = fields_of(&s);
    assert!(s.stretch_adjust_levels(0));
    assert_eq!(undo_label(&mut s), "Edit adjustments");
    let stretched = fields_of(&s);
    s.undo();
    assert_eq!(fields_of(&s), unstretched, "one step");
    s.redo();
    assert_eq!(fields_of(&s), stretched);
    let fields = &stretched;
    let value = |key: &str| fields.iter().find(|f| f.key == key).unwrap().value;
    assert!(
        value("black_pct") > 5.0 && value("white_pct") < 99.0,
        "{fields:?}"
    );
    assert!(!s.stretch_adjust_levels(0), "stretched already");
    assert!(!s.stretch_adjust_levels(1), "no Levels row");
    // A second Levels reads the first one's output: Black 50 % puts the dark half on black.
    assert_eq!(s.edit_adjust_field(0, "black_pct", 50.0), Ok(true));
    assert!(s.add_adjust_step("levels"));
    let rows = s.adjustments_view().unwrap().steps;
    assert_eq!(rows[1].kind, "levels");
    assert_eq!(rows[0].histogram.as_ref().unwrap()[0], 0);
    assert!(
        rows[1].histogram.as_ref().unwrap()[0] > 0,
        "after the rows above"
    );
}

/// Criterion 56: a step that does not fit the memory of its run is not run, and the group says
/// which ran last; nothing aborts.
#[test]
fn c56_a_step_past_the_memory_budget_warns_and_names_the_last_step_that_ran() {
    let (mut s, _, _) = engraved();
    assert!(s.add_adjust_step("levels"));
    assert!(s.add_adjust_step("sharpen"));
    let _ = dots(&s);
    assert_eq!(s.adjustments_view().unwrap().memory_line, None);
    // The 40 x 20 picture: Levels and Sharpen each need 6 x 800 bytes (a later step names the
    // last that ran once a step changes the size, slice 2).
    s.adjust_pictures().set_run_bytes(5 * 800);
    assert!(s.edit_adjust_field(0, "black_pct", 10.0).unwrap());
    let _ = dots(&s);
    assert_eq!(
        s.adjustments_view().unwrap().memory_line.as_deref(),
        Some(
            "The adjustments of this picture need more memory than is free. Its steps were not run."
        )
    );
    assert_eq!(
        memory_line_of(&[StepKind::Levels, StepKind::Sharpen], 1),
        "The adjustments of this picture need more memory than is free. The steps after Levels \
         were not run."
    );
}

fn memory_line_of(kinds: &[StepKind], not_run: usize) -> String {
    let steps: Vec<_> = kinds.iter().map(|k| k.default_step(100)).collect();
    super::view::memory_line(&steps, not_run)
}

/// Criterion 29: Make editable keeps the dots byte for byte (superseded by the Dither on the
/// picture, 2026-11-20: and keeps the picture's Dither).
#[test]
fn c29_make_editable_burns_the_same_dots() {
    let (mut s, _, _) = engraved();
    s.pick_adjust_preset("black_white");
    s.add_adjust_step("levels");
    let with_preset = dots(&s);
    assert!(s.make_adjust_editable());
    let view = s.adjustments_view().unwrap();
    assert!(view.presets[6].pressed, "Expert");
    assert!(!view.steps.is_empty() && view.steps.iter().all(|r| !r.preset_in_use));
    assert_eq!(view.dither.dither, "threshold", "the Dither is kept");
    assert_eq!(dots(&s), with_preset);
    assert_eq!(s.take_adjust_notice(), None, "one operation");
}

/// Criterion 59: Restore original gives the `0123` dots back in one step. Superseded by the
/// Dither on the picture (2026-11-20): the picture's Dither goes back to Floyd-Steinberg, the
/// operation is untouched (its Threshold applies to gradient fills only); a stored Dither alone
/// enables it.
#[test]
fn c59_restore_original_is_the_imported_picture() {
    let (mut s, image, op) = engraved();
    assert!(s.set_operation_dither(op, "threshold"));
    s.selection.set(&[image]);
    let plain = dots(&s);
    s.pick_adjust_preset("line_art");
    s.add_adjust_step("denoise");
    assert_ne!(dots(&s), plain);
    assert!(s.restore_original_adjustments());
    assert_eq!(undo_label(&mut s), "Restore original");
    assert!(dots(&s) == plain, "the 0123 dots at Floyd-Steinberg");
    let view = s.adjustments_view().unwrap();
    assert_eq!(view.dither.dither, "floyd_steinberg");
    assert!(!view.restore_enabled);
    assert!(!s.restore_original_adjustments(), "nothing to restore");
    assert!(s.set_picture_dither("jarvis"));
    assert!(
        s.adjustments_view().unwrap().restore_enabled,
        "a Dither alone is something to restore"
    );
}

/// UX notes 3.10: the picture's Dither strip and its fields are one step each, keep Preset and
/// Strength, change the dots, and a field out of range is refused.
#[test]
fn the_pictures_dither_strip_and_fields_are_one_step_each() {
    let (mut s, _, _) = engraved();
    s.pick_adjust_preset("photo");
    assert!(s.set_adjust_strength(70.0).unwrap());
    let jarvis = dots(&s);
    assert!(s.set_picture_dither("halftone"));
    assert_eq!(undo_label(&mut s), "Edit adjustments");
    let view = s.adjustments_view().unwrap();
    assert_eq!(view.dither.dither, "halftone");
    assert!(view.presets[0].pressed, "Preset kept");
    assert_eq!(view.strength.unwrap().value, 70, "Strength kept");
    let fields: Vec<_> = view.dither.screen_fields.iter().map(|f| f.field).collect();
    assert_eq!(fields, ["screen_size", "screen_angle"]);
    assert_ne!(dots(&s), jarvis);
    assert_eq!(s.edit_picture_dither_field("screen_size", 1.2), Ok(true));
    assert_eq!(
        s.adjustments_view().unwrap().dither.screen_fields[0].value,
        1.2
    );
    assert_eq!(
        s.edit_picture_dither_field("screen_size", 9.0),
        Err("Cell size is 0.20 to 5.00 mm.".to_owned())
    );
    assert!(
        s.edit_picture_dither_field("lowest_power", 5.0).is_err(),
        "the operation's"
    );
    assert!(!s.set_picture_dither("halftone"), "the same mode");
    assert!(!s.set_picture_dither("halftone_x"));
    s.undo();
    assert_eq!(
        s.adjustments_view().unwrap().dither.screen_fields[0].value,
        0.8
    );
}

/// Criterion 67: two pictures with different presets in one Image operation keep different
/// dots; each burns the dots it burns alone.
#[test]
fn c67_two_pictures_in_one_operation_keep_their_own_dither() {
    let mut s = laser_page();
    let a = place(&s, 8.0, 4.0);
    let b = place(&s, 8.0, 4.0);
    s.document
        .translate_objects(&[b], Vec2::new(0.0, 20.0))
        .unwrap();
    image_op(&mut s, &[a, b], 0.1);
    s.selection.set(&[a]);
    assert!(s.pick_adjust_preset("photo"));
    s.selection.set(&[b]);
    assert!(s.pick_adjust_preset("dot_raster"));
    let modes: Vec<String> = [a, b]
        .iter()
        .map(|&id| {
            s.selection.set(&[id]);
            s.adjustments_view().unwrap().dither.dither.to_owned()
        })
        .collect();
    assert_eq!(modes, ["jarvis", "halftone"]);
    let job = s.laser_job().unwrap();
    let levels_of = |node: NodeId| -> Vec<Vec<u8>> {
        job.built
            .job
            .moves
            .iter()
            .filter_map(|m| match m.kind {
                curvyo_laser_core::job::MoveKind::Sweep {
                    node: at, sweep, ..
                } if at == node => {
                    let sweep = job.built.job.sweep(sweep).unwrap();
                    Some(
                        (0..sweep.levels.len())
                            .map(|i| sweep.levels.get(i))
                            .collect(),
                    )
                }
                _ => None,
            })
            .collect()
    };
    let (dots_a, dots_b) = (levels_of(a), levels_of(b));
    assert!(!dots_a.is_empty() && !dots_b.is_empty());
    assert_ne!(dots_a, dots_b, "the same pixels, two dithers, two dot sets");
}

/// Criteria 9, 61: a clone shows Original's until its first edit; then Own, the original's dots
/// unchanged; Original's is Follow original.
#[test]
fn c9_a_clone_gets_own_adjustments_on_its_first_edit() {
    let (mut s, image, op) = engraved();
    s.pick_adjust_preset("photo");
    s.selection.set(&[image]);
    s.linked_clone();
    let clone = s.selection.ids()[0];
    s.document
        .translate_objects(&[clone], Vec2::new(20.0, 0.0))
        .unwrap();
    s.selection.set(&[clone]);
    let _ = op;
    assert_eq!(s.adjustments_view().unwrap().clone_row, Some("original"));
    let original_leaf = s.picture_leaf(image).unwrap().adjustments;
    assert!(s.set_adjust_strength(90.0).unwrap());
    assert_eq!(s.adjustments_view().unwrap().clone_row, Some("own"));
    assert_eq!(s.picture_leaf(image).unwrap().adjustments, original_leaf);
    assert_eq!(s.picture_leaf(clone).unwrap().adjustments.strength, 90);
    assert!(s.follow_original_adjustments());
    assert_eq!(undo_label(&mut s), "Follow original");
    assert_eq!(s.adjustments_view().unwrap().clone_row, Some("original"));
}

/// Criterion 9, UX review U2: Own on the clone's override row copies the original's adjustments
/// whole (Preset, Strength and stack), one step; the dots stay what the clone showed.
#[test]
fn c9_own_copies_the_originals_adjustments_whole() {
    let (mut s, image, _) = engraved();
    s.pick_adjust_preset("photo");
    assert!(s.set_adjust_strength(70.0).unwrap());
    s.selection.set(&[image]);
    s.linked_clone();
    let clone = s.selection.ids()[0];
    s.selection.set(&[clone]);
    let shown = s.picture_leaf(clone).unwrap().adjustments;
    assert!(s.own_adjustments());
    assert_eq!(undo_label(&mut s), "Edit adjustments");
    let view = s.adjustments_view().unwrap();
    assert_eq!(view.clone_row, Some("own"));
    let own = s.picture_leaf(clone).unwrap().adjustments;
    assert_eq!(own, shown, "Preset, Strength and stack copied");
    assert_eq!(own.preset.as_deref(), Some("photo"));
    assert_eq!(own.strength, 70);
    assert!(!s.own_adjustments(), "already own");
    s.selection.set(&[image]);
    assert!(!s.own_adjustments(), "an image has no override");
}

/// Criteria 11, 12, 60: the pane's subject, the operation pick and the After dots are the job's.
#[test]
fn c12_the_after_half_is_the_job_dots_of_the_item() {
    let (mut s, image, op) = engraved();
    let second = image_op(&mut s, &[image], 0.2);
    s.selection.set(&[image]);
    s.pick_adjust_preset("photo");
    assert!(s.toggle_before_after());
    let view = s.before_after_view();
    assert!(view.open && view.view == "split");
    assert_eq!(view.operations.len(), 2);
    assert_eq!(view.subject, "Photo 1 · Image 1 · Jarvis · 0.10 mm");
    assert_eq!(view.before.as_ref().unwrap().width, 40);
    let after = view.after.unwrap();
    assert!(!after.reduced);
    let job = s.laser_job().unwrap();
    let block = job.input.blocks.iter().position(|b| b.id == op).unwrap();
    let block = job.built.job_block[block].unwrap();
    let mut seen = std::collections::HashSet::new();
    let burned: usize = job
        .built
        .job
        .moves
        .iter()
        .filter(|m| m.block == block)
        .filter_map(|m| match m.kind {
            MoveKind::Sweep { sweep, node, .. } if node == image && seen.insert(sweep) => {
                Some(job.built.job.sweeps[usize::try_from(sweep).unwrap()].burned_dots())
            }
            _ => None,
        })
        .sum();
    let pixels = s.before_after_pixels("after");
    let lit = pixels.chunks(4).filter(|p| p[3] > 0).count();
    assert!(burned > 0);
    assert_eq!(lit, burned, "one texel per burned dot");
    assert_eq!(
        s.adjustments_view().unwrap().state_line.as_deref(),
        Some("Numbers for Image 1, 0.10 mm")
    );
    assert!(s.pick_before_after_operation(second));
    assert!(s.before_after_view().subject.ends_with("0.20 mm"));
    assert!(s.set_before_after_view("side_by_side"));
    s.selection.set(&[]);
    assert_eq!(s.before_after_view().empty, Some("No picture selected."));
}

/// Criterion 13: while the exact dots are worked the After half is a live picture of the current
/// stack at a reduced level, then the job's own dots.
#[test]
fn c13_the_after_half_is_live_while_the_dots_are_worked() {
    let mut s = laser_page();
    let image = place(&s, 80.0, 40.0);
    let _op = image_op(&mut s, &[image], 0.1);
    s.selection.set(&[image]);
    let _ = s.laser_job();
    assert!(s.toggle_before_after());
    s.pick_adjust_preset("black_white");
    assert!(s.job_pending(), "800 x 400 dots are worked in ticks");
    let live = s.before_after_view().after.unwrap();
    assert!(live.reduced);
    assert!(live.image.width < 800, "a coarser dot grid");
    let first = s.before_after_pixels("after");
    assert_eq!(
        first.len(),
        (live.image.width * live.image.height * 4) as usize
    );
    s.pick_adjust_preset("photo");
    assert!(s.job_pending());
    let second = s.before_after_pixels("after");
    assert_ne!(first, second, "the current stack, not the last job's dots");
    let mut ticks = 0;
    while s.job_pending() {
        s.advance_job();
        ticks += 1;
        assert!(ticks < 1_000);
    }
    let exact = s.before_after_view().after.unwrap();
    assert!(!exact.reduced);
    assert_eq!(exact.image.width, 800);
}

/// The image quads of a frame with their pictures.
pub(super) fn drawn_pictures(
    s: &mut Session,
) -> Vec<(curvyo_document_core::AssetHash, Arc<WorkingImage>)> {
    let mut list = s.frame_draw_list();
    s.frame_images(&mut list)
}

/// Criterion 15: the canvas draws the adjusted picture, made in the host's ticks at the display
/// level and kept by content; an image without adjustments draws as imported.
#[test]
fn c15_the_canvas_shows_the_adjusted_picture() {
    let mut s = Session::new(290_136);
    let image = place_on_page(&s);
    let asset = s.picture_leaf(image).unwrap().asset;
    s.selection.set(&[image]);
    let plain = drawn_pictures(&mut s);
    assert_eq!(plain.len(), 1);
    assert_eq!(
        plain[0].0, asset,
        "empty adjustments: the picture as imported"
    );
    assert!(!s.ticks_pending());
    assert!(s.add_adjust_step("levels"));
    assert!(s.edit_adjust_field(0, "black_pct", 40.0).unwrap());
    let first = drawn_pictures(&mut s);
    assert_eq!(first[0].0, asset, "the edit's frame never adjusts");
    assert!(s.ticks_pending(), "the look is worked in ticks");
    let mut ticks = 0;
    while s.ticks_pending() {
        s.advance_tick();
        ticks += 1;
        assert!(ticks < 100);
    }
    let looked = drawn_pictures(&mut s);
    assert_ne!(looked[0].0, asset);
    let picture = &looked[0].1;
    assert!(
        picture
            .pixels
            .chunks(4)
            .all(|p| p[0] == p[1] && p[1] == p[2] && p[3] == 255),
        "grey after Levels"
    );
    assert_ne!(picture.pixels, plain[0].1.pixels);
    let again = drawn_pictures(&mut s);
    assert!(
        Arc::ptr_eq(&again[0].1, picture),
        "kept, not made per frame"
    );
    assert!(s.set_adjust_step_enabled(0, false));
    assert_eq!(
        drawn_pictures(&mut s)[0].0,
        asset,
        "a step that is off changes nothing"
    );
    assert!(!s.ticks_pending());
}

/// An image placed on the first page, 8 x 4 mm.
pub(super) fn place_on_page(s: &Session) -> NodeId {
    s.document
        .add_images(&[NewImage {
            bytes: Arc::from(ramp_png(40, 20).as_slice()),
            placement: Affine::new(8.0, 0.0, 0.0, 4.0, 10.0, 10.0),
            name: "Photo 1".to_owned(),
        }])
        .unwrap()[0]
}

/// UX notes E1: the Style tab of a selected picture is not dimmed; it holds the Adjustments group.
#[test]
fn a_selected_picture_has_a_style_tab() {
    let mut s = laser_page();
    let image = place(&s, 8.0, 4.0);
    s.selection.set(&[image]);
    let record = crate::session::panel_tabs::PanelTabsRecord::new(&s.panel_view());
    let style = record
        .tab_names
        .iter()
        .position(|name| name == "style")
        .unwrap();
    assert_eq!(record.tab_enabled[style], 1, "the Style tab is enabled");
}
