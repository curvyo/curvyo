//! The writes of each edit of a picture's adjustments (`specs/0135` criteria 2, 3, 5, 26, 27,
//! 29; `ux-notes.md` §3.5 to §3.7, §4, §5): what a preset pick, Strength, a step added, edited,
//! switched, moved or removed, a Dither change and Make editable store, for one step "Edit
//! adjustments".

use curvyo_document_core::{AdjustEntry, AdjustmentsWrite, EXPERT_PRESET};
use curvyo_laser_core::adjust::{Band, DitherSettings, ManualStep, Preset, StepKind};
use curvyo_laser_core::settings::ImageEdit;

/// The refusal of a typed Strength out of range (criterion 26, as `0123` criterion 12).
pub const STRENGTH_RANGE_TEXT: &str = "Strength is 0 to 100 %.";
/// The refusal of a Black point that is not at least 2 % below the White point (criterion 5).
pub const LEVELS_ORDER_TEXT: &str = "Black point is at least 2 % below White point.";
/// The refusal of a Curves point that would pass its neighbour or move an end point's Input
/// (criterion 5).
pub const CURVE_ORDER_TEXT: &str = "Points stay in order; the end points stay at 0 and 100 %.";

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
    add_step_with(entries, kind.default_step(interval_um))
}

/// [`add_step`] of `step` with its fields as given (a Resize at the picture's width).
#[must_use]
pub fn add_step_with(entries: &[AdjustEntry], step: ManualStep) -> Vec<AdjustEntry> {
    let kind = step.kind();
    let at = entries
        .iter()
        .rposition(|entry| known_kind(entry).is_some_and(|k| k <= kind))
        .map_or(0, |i| i + 1);
    let mut next = entries.to_vec();
    next.insert(at, step.entry(true));
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
/// The field's range text for a value out of range (a Crop field's range as it stands, from
/// [`crop_range_text`]); [`LEVELS_ORDER_TEXT`] for Levels whose Black point would not be at least
/// 2 % below its White point; [`CURVE_ORDER_TEXT`] for a Curves point out of order.
pub fn edit_field(
    entries: &[AdjustEntry],
    index: usize,
    key: &str,
    typed: f64,
) -> Result<Option<Vec<AdjustEntry>>, String> {
    let Some((step, enabled)) = manual(entries, index) else {
        return Ok(None);
    };
    let Some(spec) = step.kind().fields().iter().find(|f| f.key == key) else {
        return Ok(None);
    };
    if let ManualStep::Crop { pct } = step
        && let Some((min, max)) = crop_range(pct, key)
        && !(f64::from(min)..=f64::from(max)).contains(&typed.round())
    {
        return Err(crop_range_text(pct, key).unwrap_or_default());
    }
    let value = spec.typed(typed).map_err(str::to_owned)?;
    let fields: Vec<(&str, i64)> = step
        .fields()
        .into_iter()
        .map(|(k, v)| (k, if k == key { value } else { v }))
        .collect();
    let edited = ManualStep::read(step.kind().key(), |k| {
        fields.iter().find(|(f, _)| *f == k).map(|(_, v)| *v)
    })
    .map_err(|_| match step.kind() {
        StepKind::Curves => CURVE_ORDER_TEXT.to_owned(),
        _ => LEVELS_ORDER_TEXT.to_owned(),
    })?;
    let mut next = entries.to_vec();
    next[index] = edited.entry(enabled);
    Ok(Some(next))
}

/// The range Crop field `key` takes in a crop of `pct` as it stands (criterion 5, UX review
/// UX-2): Left and Top 0 to 100 minus Width or Height, Width and Height 1 to 100 minus Left or Top.
fn crop_range([left, top, width, height]: [u8; 4], key: &str) -> Option<(u8, u8)> {
    Some(match key {
        "left_pct" => (0, 100 - width.min(100)),
        "top_pct" => (0, 100 - height.min(100)),
        "width_pct" => (1, 100 - left.min(99)),
        "height_pct" => (1, 100 - top.min(99)),
        _ => return None,
    })
}

/// The refusal of Crop field `key` in a crop of `pct`: "Left is 0 to 50 %."; `None` for a key
/// that is no Crop field.
#[must_use]
pub fn crop_range_text(pct: [u8; 4], key: &str) -> Option<String> {
    let (min, max) = crop_range(pct, key)?;
    let label = match key {
        "left_pct" => "Left",
        "top_pct" => "Top",
        "width_pct" => "Width",
        _ => "Height",
    };
    Some(format!("{label} is {min} to {max} %."))
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

/// Whether a row of kind `other` is in the same band as a moved row of kind `moved`, so the
/// move may pass it (criterion 3, `ux-notes.md` §3.7): tone steps among tone steps; the geometry
/// band runs in its fixed order, so a geometry row passes only rows of its own kind.
fn same_band(moved: StepKind, other: StepKind) -> bool {
    match moved.band() {
        Band::Tone => other.band() == Band::Tone,
        Band::Geometry => other == moved,
    }
}

/// The stack with row `index` moved one row up or down inside its band (criterion 3,
/// `ux-notes.md` §3.7); `None` at the band's end or for a row this build does not know.
#[must_use]
pub fn move_step(entries: &[AdjustEntry], index: usize, up: bool) -> Option<Vec<AdjustEntry>> {
    let moved = known_kind(entries.get(index)?)?;
    let other = if up { index.checked_sub(1)? } else { index + 1 };
    if !same_band(moved, known_kind(entries.get(other)?)?) {
        return None;
    }
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
    let moved = known_kind(entries.get(from)?)?;
    let passed = &entries[from.min(to)..=from.max(to)];
    if passed
        .iter()
        .any(|entry| known_kind(entry).is_none_or(|k| !same_band(moved, k)))
    {
        return None;
    }
    let mut next = entries.to_vec();
    let row = next.remove(from);
    next.insert(to, row);
    Some(next)
}

/// Make editable (criterion 29): Preset Expert and, in one write, the tone steps that ran
/// (`resolved`, the derived steps at the shown operation's Interval) as ordinary switched-on rows
/// in place of the manual tone steps that were "Preset in use"; the geometry rows (Crop, Resize)
/// stay as they were, before them, and the rows this build does not know after them. Strength is
/// kept.
#[must_use]
pub fn make_editable(entries: &[AdjustEntry], resolved: &[ManualStep]) -> AdjustmentsWrite {
    let steps = entries
        .iter()
        .filter(|entry| known_kind(entry).is_some_and(|k| k.band() == Band::Geometry))
        .cloned()
        .chain(
            resolved
                .iter()
                .filter(|step| step.kind().band() == Band::Tone)
                .map(|step| step.entry(true)),
        )
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
#[path = "image_adjust_edit_tests.rs"]
mod tests;
