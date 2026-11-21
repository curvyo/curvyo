use curvyo_laser_core::settings::Dither;
use serde_json::json;

use super::*;

fn kinds(entries: &[AdjustEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|e| e.step().map_or("?".to_owned(), |s| s.kind.clone()))
        .collect()
}

/// Criterion 27, superseded by the Dither on the picture (2026-11-20): a pick writes the
/// Preset and the picture's Dither, no operation and no notice (the old test wrote the Dither
/// of every operation and said so).
#[test]
fn a_preset_pick_writes_the_pictures_dither() {
    let current = DitherSettings {
        threshold_pct: 30,
        screen_angle_deg: 15,
        ..DitherSettings::DEFAULT
    };
    let write = pick_preset(Preset::DotRaster, current, 200);
    assert_eq!(write.preset.as_deref(), Some("dot_raster"));
    assert_eq!((write.strength, write.steps), (None, None));
    let (dither, damaged) = DitherSettings::read(write.dither.as_ref());
    assert!(!damaged);
    assert_eq!(
        dither,
        DitherSettings {
            mode: Dither::Halftone,
            threshold_pct: 30,
            screen_size_um: 1_600,
            screen_angle_deg: 45,
        },
        "8 x Interval 0.2, Angle 45, Threshold kept"
    );
    let expert = pick_preset(Preset::Expert, current, 200);
    assert_eq!(expert.preset.as_deref(), Some("expert"));
    assert_eq!(expert.dither, None, "Expert writes the Preset only");
    let photo = pick_preset(Preset::Photo, current, PICK_INTERVAL_UM);
    assert_eq!(
        DitherSettings::read(photo.dither.as_ref()).0,
        DitherSettings {
            mode: Dither::Jarvis,
            ..current
        }
    );
    let fine = pick_preset(Preset::Newspaper, current, 10);
    assert_eq!(
        DitherSettings::read(fine.dither.as_ref()).0.screen_size_um,
        200,
        "clamped to 0.20 mm"
    );
}

#[test]
fn a_dither_edit_writes_the_whole_register_and_nothing_else() {
    let write = set_dither(DitherSettings::DEFAULT, ImageEdit::Dither(Dither::Grey)).unwrap();
    assert_eq!(
        (write.preset, write.strength, write.steps),
        (None, None, None)
    );
    assert_eq!(write.dither.unwrap().fields.len(), 3);
    assert_eq!(
        set_dither(DitherSettings::DEFAULT, ImageEdit::Gamma(120)),
        None
    );
    assert_eq!(
        set_dither(DitherSettings::DEFAULT, ImageEdit::LowestPower(5)),
        None
    );
}

#[test]
fn strength_is_whole_percent_from_0_to_100() {
    assert_eq!(set_strength(42.4).unwrap().strength, Some(42));
    assert_eq!(set_strength(100.0).unwrap().strength, Some(100));
    assert_eq!(set_strength(100.5), Err(STRENGTH_RANGE_TEXT));
    assert_eq!(set_strength(-1.0), Err(STRENGTH_RANGE_TEXT));
    assert_eq!(set_strength(f64::NAN), Err(STRENGTH_RANGE_TEXT));
}

/// Criterion 3: Sharpen then Levels, or Levels then Sharpen, ends in the pipeline order; a
/// second step of a kind goes after the first.
#[test]
fn a_new_step_goes_to_its_pipeline_place() {
    let a = add_step(
        &add_step(&[], StepKind::Sharpen, 100),
        StepKind::Levels,
        100,
    );
    let b = add_step(
        &add_step(&[], StepKind::Levels, 100),
        StepKind::Sharpen,
        100,
    );
    assert_eq!(kinds(&a), ["levels", "sharpen"]);
    assert_eq!(a, b);
    let mut c = add_step(&a, StepKind::Levels, 100);
    c = add_step(&c, StepKind::Denoise, 100);
    assert_eq!(kinds(&c), ["denoise", "levels", "levels", "sharpen"]);
    let sharpen = c[3].step().unwrap();
    assert_eq!(
        sharpen.fields["radius_um"], 100,
        "the Interval as its radius"
    );
}

#[test]
fn a_field_edit_rounds_to_the_step_and_refuses_out_of_range() {
    let stack = add_step(&[], StepKind::Sharpen, 100);
    let edited = edit_field(&stack, 0, "radius_um", 0.234).unwrap().unwrap();
    assert_eq!(edited[0].step().unwrap().fields["radius_um"], 230);
    assert_eq!(
        edit_field(&stack, 0, "amount_pct", 501.0),
        Err("Amount is 0 to 500 %.".to_owned())
    );
    assert_eq!(edit_field(&stack, 0, "nothing", 1.0), Ok(None));
    let levels = add_step(&[], StepKind::Levels, 100);
    let up = edit_field(&levels, 0, "black_pct", 50.0).unwrap().unwrap();
    assert_eq!(
        edit_field(&up, 0, "white_pct", 51.0),
        Err(LEVELS_ORDER_TEXT.to_owned())
    );
}

/// Criterion 2: off keeps the fields; remove; moves stay inside the band and skip nothing
/// this build does not know.
#[test]
fn switch_remove_and_move() {
    let mut stack = add_step(&[], StepKind::Levels, 100);
    stack = add_step(&stack, StepKind::Sharpen, 100);
    stack.push(AdjustEntry::Unreadable(json!(1)));
    let off = set_enabled(&stack, 0, false).unwrap();
    assert!(!off[0].step().unwrap().enabled);
    assert_eq!(
        off[0].step().unwrap().fields,
        stack[0].step().unwrap().fields
    );
    assert_eq!(set_enabled(&off, 0, false), None);
    assert_eq!(set_enabled(&stack, 2, true), None);
    assert_eq!(
        kinds(&move_step(&stack, 1, true).unwrap()),
        ["sharpen", "levels", "?"]
    );
    assert_eq!(move_step(&stack, 0, true), None, "the band's top");
    assert_eq!(move_step(&stack, 1, false), None, "not past an unknown row");
    assert_eq!(
        move_step(&stack, 2, true),
        None,
        "an unknown row does not move"
    );
    assert_eq!(
        kinds(&remove_step(&stack, 2).unwrap()),
        ["levels", "sharpen"]
    );
    assert_eq!(remove_step(&stack, 3), None);
}

/// Criterion 5: Stretch writes Black and White of a Levels row, keeps Gamma and the switch.
#[test]
fn stretch_sets_the_two_points_of_a_levels_row() {
    let mut stack = add_step(&[], StepKind::Levels, 100);
    stack = edit_field(&stack, 0, "gamma_centi", 1.5).unwrap().unwrap();
    stack = set_enabled(&stack, 0, false).unwrap();
    stack = add_step(&stack, StepKind::Sharpen, 100);
    let next = stretch_levels(&stack, 0, (3, 97)).unwrap();
    let step = next[0].step().unwrap();
    assert!(!step.enabled);
    assert_eq!(
        (
            step.fields["black_pct"],
            step.fields["white_pct"],
            step.fields["gamma_centi"]
        ),
        (3, 97, 150)
    );
    assert_eq!(stretch_levels(&next, 0, (3, 97)), None, "nothing to write");
    assert_eq!(stretch_levels(&stack, 1, (3, 97)), None, "no Levels row");
}

/// Criterion 2: a drag lands a row anywhere inside its band in one write; never past a row
/// this build does not know, never onto its own place.
#[test]
fn a_drag_moves_a_row_to_its_landing_place_in_one_write() {
    let mut stack = add_step(&[], StepKind::Denoise, 100);
    stack = add_step(&stack, StepKind::Levels, 100);
    stack = add_step(&stack, StepKind::Sharpen, 100);
    stack.push(AdjustEntry::Unreadable(json!(1)));
    assert_eq!(
        kinds(&move_step_to(&stack, 2, 0).unwrap()),
        ["sharpen", "denoise", "levels", "?"]
    );
    assert_eq!(
        kinds(&move_step_to(&stack, 0, 2).unwrap()),
        ["levels", "sharpen", "denoise", "?"]
    );
    assert_eq!(move_step_to(&stack, 1, 1), None, "its own place");
    assert_eq!(move_step_to(&stack, 0, 3), None, "not past an unknown row");
    assert_eq!(
        move_step_to(&stack, 3, 0),
        None,
        "an unknown row does not move"
    );
    assert_eq!(move_step_to(&stack, 0, 9), None, "past the end");
}

/// Criterion 29: Expert and the resolved steps in place of the manual tone steps; unknown
/// rows stay; Strength is not written.
#[test]
fn make_editable_replaces_the_tone_steps_with_the_resolved_ones() {
    let mut stack = add_step(&[], StepKind::Contours, 100);
    stack.push(AdjustEntry::Unreadable(json!("later")));
    let resolved = [
        StepKind::Denoise.default_step(100),
        StepKind::Levels.default_step(100),
    ];
    let write = make_editable(&stack, &resolved);
    assert_eq!(write.preset.as_deref(), Some("expert"));
    assert_eq!(write.strength, None);
    assert_eq!(write.dither, None, "the Dither is kept (amendment A3)");
    assert_eq!(
        kinds(write.steps.as_ref().unwrap()),
        ["denoise", "levels", "?"]
    );
    assert_eq!(make_editable_notice(1, "Image 1", 100), None);
    assert_eq!(
        make_editable_notice(2, "Image 1", 100).as_deref(),
        Some("Numbers fixed for Image 1, 0.10 mm.")
    );
}

/// Criteria 3, 7: the slice-2 kinds go to their pipeline place (Crop, Resize, then Channel
/// mix first of the tone steps, Curves after Levels); Denoise before and after Channel mix
/// are both kept.
#[test]
fn slice_2_kinds_go_to_their_pipeline_place() {
    let mut stack = add_step(&[], StepKind::Sharpen, 100);
    for kind in [
        StepKind::Curves,
        StepKind::Resize,
        StepKind::Levels,
        StepKind::ChannelMix,
        StepKind::Crop,
    ] {
        stack = add_step(&stack, kind, 100);
    }
    assert_eq!(
        kinds(&stack),
        [
            "crop",
            "resize",
            "channel_mix",
            "levels",
            "curves",
            "sharpen"
        ]
    );
    let denoise_after = move_step(&add_step(&stack, StepKind::Denoise, 100), 3, true);
    let after = denoise_after.unwrap();
    assert_eq!(kinds(&after)[2..4], ["denoise", "channel_mix"]);
}

/// Criterion 3, `ux-notes.md` §3.7: a row moves only inside its band; the geometry band
/// keeps its fixed order (a Crop passes only another Crop).
#[test]
fn a_move_stays_inside_its_band() {
    let mut stack = add_step(&[], StepKind::Crop, 100);
    stack = add_step(&stack, StepKind::Resize, 100);
    stack = add_step(&stack, StepKind::Levels, 100);
    assert!(
        move_step(&stack, 1, false).is_none(),
        "Resize does not pass Levels"
    );
    assert!(
        move_step(&stack, 2, true).is_none(),
        "Levels does not pass Resize"
    );
    assert!(
        move_step(&stack, 0, false).is_none(),
        "Crop does not pass Resize"
    );
    assert!(move_step_to(&stack, 2, 0).is_none());
    let two_crops = add_step(&stack, StepKind::Crop, 100);
    assert!(move_step(&two_crops, 0, false).is_some(), "two crops swap");
}

/// Criterion 29 with slice 2: Make editable keeps the Crop and Resize rows (also one that is
/// off) and writes the tone steps that ran.
#[test]
fn make_editable_keeps_the_geometry_rows() {
    let mut stack = add_step(&[], StepKind::Crop, 100);
    stack = add_step(&stack, StepKind::Resize, 100);
    stack = set_enabled(&stack, 1, false).unwrap();
    stack = add_step(&stack, StepKind::Curves, 100);
    let crop = ManualStep::Crop {
        pct: [0, 0, 100, 100],
    };
    let resolved = [crop, StepKind::Levels.default_step(100)];
    let write = make_editable(&stack, &resolved);
    let steps = write.steps.unwrap();
    assert_eq!(kinds(&steps), ["crop", "resize", "levels"]);
    assert!(!steps[1].step().unwrap().enabled, "kept as it was");
}

/// Criterion 5 (UX review UX-2): a crop past the edge is refused with the range as it stands
/// (Left 0 to 100 minus Width, Width 1 to 100 minus Left), a curve out of order with its text.
#[test]
fn crop_and_curve_edits_are_refused_past_their_rules() {
    let crop = add_step(&[], StepKind::Crop, 100);
    assert_eq!(
        edit_field(&crop, 0, "left_pct", 10.0),
        Err("Left is 0 to 0 %.".to_owned())
    );
    let narrower = edit_field(&crop, 0, "width_pct", 50.0).unwrap().unwrap();
    assert_eq!(
        edit_field(&narrower, 0, "left_pct", 60.0),
        Err("Left is 0 to 50 %.".to_owned())
    );
    let moved = edit_field(&narrower, 0, "left_pct", 30.0).unwrap().unwrap();
    assert_eq!(
        edit_field(&moved, 0, "width_pct", 80.0),
        Err("Width is 1 to 70 %.".to_owned())
    );
    assert_eq!(
        edit_field(&moved, 0, "height_pct", 0.0),
        Err("Height is 1 to 100 %.".to_owned())
    );
    assert_eq!(
        crop_range_text([30, 0, 50, 100], "top_pct").as_deref(),
        Some("Top is 0 to 0 %.")
    );
    let curve = add_step(&[], StepKind::Curves, 100);
    assert_eq!(
        edit_field(&curve, 0, "in_0", 10.0),
        Err(CURVE_ORDER_TEXT.to_owned())
    );
    assert!(edit_field(&curve, 0, "out_0", 10.0).unwrap().is_some());
}
