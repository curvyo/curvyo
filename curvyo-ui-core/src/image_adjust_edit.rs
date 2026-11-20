//! The writes of each edit of a picture's adjustments (`specs/0135` criteria 2, 3, 5, 26, 27,
//! 29; `ux-notes.md` §3.5 to §3.7, §4, §5): what a preset pick, Strength, a step added, edited,
//! switched, moved or removed, a Dither change and Make editable store, for one step "Edit
//! adjustments".

use curvyo_document_core::{AdjustEntry, AdjustmentsWrite, EXPERT_PRESET};
use curvyo_laser_core::adjust::{DitherSettings, ManualStep, Preset, StepKind};
use curvyo_laser_core::settings::ImageEdit;

/// The refusal of a typed Strength out of range (criterion 26, as `0123` criterion 12).
pub const STRENGTH_RANGE_TEXT: &str = "Strength is 0 to 100 %.";
/// The refusal of a Black point that is not at least 2 % below the White point (criterion 5).
pub const LEVELS_ORDER_TEXT: &str = "Black point is at least 2 % below White point.";

/// The Interval a preset pick sizes the screens by when the picture is in no Image operation
/// (ADR 0072 amendment A3).
pub const PICK_INTERVAL_UM: u32 = 100;

/// A preset pick: the Preset register and, in the same write, the picture's dither `current`
/// with the preset's mode and fields folded in (criterion 27, ADR 0072 amendment A3): a field the
/// preset does not name keeps its value; Cell size and Line pitch are a multiple of `interval_um`,
/// the Interval of the first Image operation the picture is in. Expert writes the Preset only.
/// No operation is written.
#[must_use]
pub fn pick_preset(preset: Preset, current: DitherSettings, interval_um: u32) -> AdjustmentsWrite {
    let edits = preset.dither_edits(interval_um);
    let dither = (!edits.is_empty()).then(|| {
        edits
            .into_iter()
            .fold(current, |d, edit| d.with(edit).unwrap_or(d))
            .stored()
    });
    AdjustmentsWrite {
        preset: Some(preset.key().to_owned()),
        dither,
        ..AdjustmentsWrite::default()
    }
}

/// An edit of the picture's Dither strip or its fields (amendment A3): the whole register with
/// `edit` applied, Preset and Strength kept; `None` for an edit that is not the dither's.
#[must_use]
pub fn set_dither(current: DitherSettings, edit: ImageEdit) -> Option<AdjustmentsWrite> {
    current.with(edit).map(|next| AdjustmentsWrite {
        dither: Some(next.stored()),
        ..AdjustmentsWrite::default()
    })
}

/// A typed or dragged Strength (criterion 26), rounded to a whole percent.
///
/// # Errors
/// [`STRENGTH_RANGE_TEXT`] for a value that is no number or out of 0 to 100.
pub fn set_strength(value: f64) -> Result<AdjustmentsWrite, &'static str> {
    if !value.is_finite() || !(0.0..=100.0).contains(&value) {
        return Err(STRENGTH_RANGE_TEXT);
    }
    // invariant: 0 to 100 after the check, exact in a u8.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let strength = value.round() as u8;
    Ok(AdjustmentsWrite {
        strength: Some(strength),
        ..AdjustmentsWrite::default()
    })
}

/// The kind of a stored entry this build knows; `None` for an unknown or unreadable one.
fn known_kind(entry: &AdjustEntry) -> Option<StepKind> {
    entry.step().and_then(|step| StepKind::from_key(&step.kind))
}

/// The stack with a new step of `kind` with its defaults at `interval_um`, switched on, at its
/// pipeline place: after the last row whose kind comes earlier or is the same (criterion 3,
/// `ux-notes.md` §3.5); rows this build does not know do not count.
#[must_use]
pub fn add_step(entries: &[AdjustEntry], kind: StepKind, interval_um: u32) -> Vec<AdjustEntry> {
    let at = entries
        .iter()
        .rposition(|entry| known_kind(entry).is_some_and(|k| k <= kind))
        .map_or(0, |i| i + 1);
    let mut next = entries.to_vec();
    next.insert(at, kind.default_step(interval_um).entry(true));
    next
}

/// The step of row `index` when it reads, with its on/off state.
fn manual(entries: &[AdjustEntry], index: usize) -> Option<(ManualStep, bool)> {
    let step = entries.get(index)?.step()?;
    let read = ManualStep::read(&step.kind, |key| step.fields.get(key).copied()).ok()?;
    Some((read, step.enabled))
}

/// The stack with field `key` of row `index` set to `typed` (in the field's shown unit), rounded
/// to the field's step (criterion 5). `Ok(None)` when the row does not read or has no such field.
///
/// # Errors
/// The field's range text for a value out of range; [`LEVELS_ORDER_TEXT`] for Levels whose Black
/// point would not be at least 2 % below its White point.
pub fn edit_field(
    entries: &[AdjustEntry],
    index: usize,
    key: &str,
    typed: f64,
) -> Result<Option<Vec<AdjustEntry>>, &'static str> {
    let Some((step, enabled)) = manual(entries, index) else {
        return Ok(None);
    };
    let Some(spec) = step.kind().fields().iter().find(|f| f.key == key) else {
        return Ok(None);
    };
    let value = spec.typed(typed)?;
    let fields: Vec<(&str, i64)> = step
        .fields()
        .into_iter()
        .map(|(k, v)| (k, if k == key { value } else { v }))
        .collect();
    let edited = ManualStep::read(step.kind().key(), |k| {
        fields.iter().find(|(f, _)| *f == k).map(|(_, v)| *v)
    })
    .map_err(|_| LEVELS_ORDER_TEXT)?;
    let mut next = entries.to_vec();
    next[index] = edited.entry(enabled);
    Ok(Some(next))
}

/// The stack with the Levels row `index` set to Black and White point `points` (in %, from the
/// Stretch button, criterion 5), Gamma and the switch kept; `None` when the row is no Levels row
/// or already has them.
#[must_use]
pub fn stretch_levels(
    entries: &[AdjustEntry],
    index: usize,
    (black_pct, white_pct): (u8, u8),
) -> Option<Vec<AdjustEntry>> {
    let (step, enabled) = manual(entries, index)?;
    let ManualStep::Levels { gamma_centi, .. } = step else {
        return None;
    };
    let stretched = ManualStep::Levels {
        black_pct,
        white_pct,
        gamma_centi,
    };
    if stretched == step {
        return None;
    }
    let mut next = entries.to_vec();
    next[index] = stretched.entry(enabled);
    Some(next)
}

/// The stack with row `index` switched on or off, its fields kept (criterion 2); `None` when the
/// row is not of the step shape or already in that state.
#[must_use]
pub fn set_enabled(entries: &[AdjustEntry], index: usize, on: bool) -> Option<Vec<AdjustEntry>> {
    let step = entries.get(index)?.step()?;
    if step.enabled == on {
        return None;
    }
    let mut changed = step.clone();
    changed.enabled = on;
    let mut next = entries.to_vec();
    next[index] = AdjustEntry::Step(changed);
    Some(next)
}

/// The stack without row `index` (any row, also one this build does not know); `None` past the
/// end.
#[must_use]
pub fn remove_step(entries: &[AdjustEntry], index: usize) -> Option<Vec<AdjustEntry>> {
    (index < entries.len()).then(|| {
        let mut next = entries.to_vec();
        next.remove(index);
        next
    })
}

/// The stack with row `index` moved one row up or down inside its band (criterion 3,
/// `ux-notes.md` §3.7; every slice-1 kind is in the tone and detail band); `None` at the band's
/// end or for a row this build does not know.
#[must_use]
pub fn move_step(entries: &[AdjustEntry], index: usize, up: bool) -> Option<Vec<AdjustEntry>> {
    known_kind(entries.get(index)?)?;
    let other = if up { index.checked_sub(1)? } else { index + 1 };
    known_kind(entries.get(other)?)?;
    let mut next = entries.to_vec();
    next.swap(index, other);
    Some(next)
}

/// The stack with row `from` dropped at index `to` inside its band, in one write (criterion 2,
/// a drag of `ux-notes.md` §3.7); `None` for its own place, past the end, or when it or a row it
/// passes is one this build does not know.
#[must_use]
pub fn move_step_to(entries: &[AdjustEntry], from: usize, to: usize) -> Option<Vec<AdjustEntry>> {
    if from == to || to >= entries.len() {
        return None;
    }
    let passed = &entries[from.min(to)..=from.max(to)];
    if passed.iter().any(|entry| known_kind(entry).is_none()) {
        return None;
    }
    let mut next = entries.to_vec();
    let row = next.remove(from);
    next.insert(to, row);
    Some(next)
}

/// Make editable (criterion 29): Preset Expert and, in one write, the steps that ran (`resolved`,
/// the derived steps at the shown operation's Interval) as ordinary switched-on rows in place of
/// the manual tone steps that were "Preset in use"; the rows this build does not know stay after
/// them. Strength is kept.
#[must_use]
pub fn make_editable(entries: &[AdjustEntry], resolved: &[ManualStep]) -> AdjustmentsWrite {
    let steps = resolved
        .iter()
        .map(|step| step.entry(true))
        .chain(
            entries
                .iter()
                .filter(|entry| known_kind(entry).is_none())
                .cloned(),
        )
        .collect();
    AdjustmentsWrite {
        preset: Some(EXPERT_PRESET.to_owned()),
        steps: Some(steps),
        ..AdjustmentsWrite::default()
    }
}

/// The notice of Make editable on a picture in more than one operation (`ux-notes.md` §5, P15):
/// "Numbers fixed for Image 1, 0.10 mm."; `None` for one operation.
#[must_use]
pub fn make_editable_notice(operations: usize, shown: &str, interval_um: u32) -> Option<String> {
    (operations > 1).then(|| {
        let mm = f64::from(interval_um) / 1000.0;
        format!("Numbers fixed for {shown}, {mm:.2} mm.")
    })
}

#[cfg(test)]
mod tests {
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
            Err("Amount is 0 to 500 %.")
        );
        assert_eq!(edit_field(&stack, 0, "nothing", 1.0), Ok(None));
        let levels = add_step(&[], StepKind::Levels, 100);
        let up = edit_field(&levels, 0, "black_pct", 50.0).unwrap().unwrap();
        assert_eq!(
            edit_field(&up, 0, "white_pct", 51.0),
            Err(LEVELS_ORDER_TEXT)
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
}
