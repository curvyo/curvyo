//! The Adjustments group of the Style tab as the host reads it (`specs/0135` criteria 1, 2, 9,
//! 25, 26, 28, 30, 59, 60; `ux-notes.md` §3, §9, §15): header, state line, Preset strip,
//! Strength, the derived block, the add toolbar, the manual rows and the clone override row.

use curvyo_document_core::{AdjustEntry, AdjustOwner};
use curvyo_laser_core::adjust::{FieldSpec, ManualStep, Preset, StepKind};
use curvyo_ui_core::{STRENGTH_RANGE_TEXT, move_step};
use serde::Serialize;

use super::picture::PictureOperation;
use crate::session::operations::image_view::{DitherView, dither_view};

/// One cell of the Preset strip (criterion 25, `ux-notes.md` §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PresetCellView {
    /// The stored key the pick sends back.
    pub key: &'static str,
    /// The visible cell text ("Dots").
    pub cell: &'static str,
    /// The accessible name, the full name.
    pub name: &'static str,
    /// "<full name> · Dither: <mode>".
    pub tooltip: &'static str,
    /// Whether it is the picture's preset.
    pub pressed: bool,
}

/// The Strength row (criterion 26).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StrengthView {
    /// The value in whole percent (the held one while dragged).
    pub value: u8,
    /// The refusal of a typed value out of range.
    pub range: &'static str,
}

/// One number field of a step row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepFieldView {
    /// The key `edit_adjust_field` takes.
    pub key: &'static str,
    /// The label inside the field.
    pub label: &'static str,
    /// The unit inside the field ("%", "mm", empty for Gamma).
    pub unit: &'static str,
    /// The value in the shown unit.
    pub value: f64,
    /// The decimals shown.
    pub decimals: u32,
    /// The refusal of a typed value out of range.
    pub range: &'static str,
}

/// One step row, manual or derived (`ux-notes.md` §3.4, §3.6).
#[allow(clippy::struct_excessive_bools)] // independent row states, read by name
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepRowView {
    /// The row's index in the stored stack (the commands take it); `None` for a derived row.
    pub index: Option<usize>,
    /// The kind key, also the glyph's name; "unknown" for a step this build cannot run.
    pub kind: &'static str,
    /// The name on line 1.
    pub name: &'static str,
    /// Whether the power toggle is on.
    pub enabled: bool,
    /// Whether the preset runs instead of it (criterion 30).
    pub preset_in_use: bool,
    /// Whether it cannot run (unknown kind, damaged field): trash only.
    pub unknown: bool,
    /// Line 2 folded: "Off · ", "Preset in use · " or "Not run · from a newer Curvyo" and the
    /// numbers.
    pub line: String,
    /// The fields of the unfolded body; empty for an unknown or derived row.
    pub fields: Vec<StepFieldView>,
    /// Whether the up arrow is there (inside its band).
    pub up: bool,
    /// Whether the down arrow is there.
    pub down: bool,
    /// The row's accessible name ("Levels, Black point 37 %, …, off").
    pub accessible_name: String,
    /// A Levels row's histogram of its input (criterion 16): 256 bars, black to white, each 0 to
    /// 255 of the tallest; `None` for other rows and when the picture cannot be read.
    pub histogram: Option<Vec<u8>>,
}

/// The derived block of a preset (criterion 28).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DerivedView {
    /// "From preset Photo, Strength 50 %".
    pub line: String,
    /// The read-only rows with the resolved numbers.
    pub rows: Vec<StepRowView>,
}

/// One button of the add toolbar (`ux-notes.md` §3.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddButtonView {
    /// The kind key `add_adjust_step` takes, also the glyph's name.
    pub kind: &'static str,
    /// The tooltip and accessible name, "Add Levels".
    pub name: String,
}

/// The Adjustments group of the selected picture.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AdjustmentsView {
    /// Whether Restore original is enabled (greyed, never hidden, criterion 59).
    pub restore_enabled: bool,
    /// Whether the Before and after pane is open (the header button's pressed state).
    pub before_after_open: bool,
    /// "Not engraved yet", "Numbers for Image 1, 0.10 mm" or `None` (`ux-notes.md` §3.1 row 2).
    pub state_line: Option<String>,
    /// The alert line under the header when steps did not fit the memory of a run (criterion
    /// 56, `ux-notes.md` §6 P17); `None` when every step ran or the picture is not made yet.
    pub memory_line: Option<String>,
    /// The seven cells in strip order.
    pub presets: Vec<PresetCellView>,
    /// The picture's own Dither strip and its mode's fields (`ux-notes.md` §3.10, ADR 0072
    /// amendment A1); Lowest power is the operation's (Frage 284 B).
    pub dither: DitherView,
    /// The Strength row; `None` with Expert (hidden, the value kept).
    pub strength: Option<StrengthView>,
    /// The derived block; `None` with Expert.
    pub derived: Option<DerivedView>,
    /// Row 1 of the add toolbar (tone and detail); empty while a preset is active (P8).
    pub add_tone: Vec<AddButtonView>,
    /// Row 2 (Crop, Resize, AI steps): none in slice 1.
    pub add_geometry: Vec<AddButtonView>,
    /// The manual stack.
    pub steps: Vec<StepRowView>,
    /// The clone's Adjustments override row: "original" or "own"; `None` for an image.
    pub clone_row: Option<&'static str>,
    /// The notice of the last edit, kept until read.
    pub notice: Option<String>,
}

/// The decimals of a field whose shown unit has `per_unit` steps.
fn decimals(per_unit: i64) -> u32 {
    per_unit.max(1).ilog10()
}

fn field_text(spec: &FieldSpec, value: i64) -> String {
    let shown = spec.shown(value);
    let digits = decimals(spec.per_unit) as usize;
    let number = format!("{shown:.digits$}");
    if spec.unit.is_empty() {
        format!("{} {number}", spec.label)
    } else {
        format!("{} {number} {}", spec.label, spec.unit)
    }
}

/// The numbers of `step` as line 2 shows them.
fn numbers(step: ManualStep) -> String {
    let fields = step.fields();
    step.kind()
        .fields()
        .iter()
        .filter_map(|spec| {
            let value = fields.iter().find(|(k, _)| *k == spec.key)?.1;
            Some(field_text(spec, value))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn field_views(step: ManualStep) -> Vec<StepFieldView> {
    let fields = step.fields();
    step.kind()
        .fields()
        .iter()
        .filter_map(|spec| {
            let value = fields.iter().find(|(k, _)| *k == spec.key)?.1;
            Some(StepFieldView {
                key: spec.key,
                label: spec.label,
                unit: spec.unit,
                value: spec.shown(value),
                decimals: decimals(spec.per_unit),
                range: spec.range_text,
            })
        })
        .collect()
}

/// A derived, read-only row.
fn derived_row(step: ManualStep) -> StepRowView {
    let numbers = numbers(step);
    StepRowView {
        index: None,
        kind: step.kind().key(),
        name: step.kind().name(),
        enabled: true,
        preset_in_use: false,
        unknown: false,
        accessible_name: format!("{}, {numbers}", step.kind().name()),
        line: numbers,
        fields: Vec::new(),
        up: false,
        down: false,
        histogram: None,
    }
}

/// The manual row `index` of `entries` with `preset` active or not.
fn manual_row(entries: &[AdjustEntry], index: usize, preset: bool) -> StepRowView {
    let read = entries[index].step().and_then(|stored| {
        let step = ManualStep::read(&stored.kind, |key| stored.fields.get(key).copied()).ok()?;
        Some((step, stored.enabled))
    });
    let Some((step, enabled)) = read else {
        return StepRowView {
            index: Some(index),
            kind: "unknown",
            name: "Unknown step",
            enabled: false,
            preset_in_use: false,
            unknown: true,
            line: "Not run · from a newer Curvyo".to_owned(),
            fields: Vec::new(),
            up: false,
            down: false,
            accessible_name: "Unknown step, not run".to_owned(),
            histogram: None,
        };
    };
    let numbers = numbers(step);
    let (prefix, suffix) = if preset {
        ("Preset in use · ", ", preset in use")
    } else if enabled {
        ("", "")
    } else {
        ("Off · ", ", off")
    };
    StepRowView {
        index: Some(index),
        kind: step.kind().key(),
        name: step.kind().name(),
        enabled,
        preset_in_use: preset,
        unknown: false,
        line: format!("{prefix}{numbers}"),
        fields: field_views(step),
        up: move_step(entries, index, true).is_some(),
        down: move_step(entries, index, false).is_some(),
        accessible_name: format!("{}, {numbers}{suffix}", step.kind().name()),
        histogram: None,
    }
}

/// The memory line of a picture whose steps from `not_run` on did not fit (criterion 56): it names
/// the last step that ran (P17).
pub(super) fn memory_line(steps: &[ManualStep], not_run: usize) -> String {
    let lead = "The adjustments of this picture need more memory than is free.";
    match not_run.checked_sub(1).and_then(|last| steps.get(last)) {
        Some(last) => format!(
            "{lead} The steps after {} were not run.",
            last.kind().name()
        ),
        None => format!("{lead} Its steps were not run."),
    }
}

/// The state line: "Not engraved yet" without an Image operation, "Numbers for <op>, <mm> mm"
/// with two or more (criterion 60).
fn state_line(operations: &[PictureOperation], shown: Option<&PictureOperation>) -> Option<String> {
    match (operations.len(), shown) {
        (0, _) => Some("Not engraved yet".to_owned()),
        (1, _) | (_, None) => None,
        (_, Some(op)) => Some(format!(
            "Numbers for {}, {:.2} mm",
            op.name,
            f64::from(op.interval_um) / 1000.0
        )),
    }
}

impl crate::session::Session {
    /// The Adjustments group of the selected picture; `None` without exactly one image or image
    /// clone selected (the group is absent).
    pub fn adjustments_view(&self) -> Option<AdjustmentsView> {
        let target = self.adjust_target()?;
        let shown = self.document.shown_adjustments(target).ok()?;
        let operations = self.picture_operations(target);
        let shown_op = self.shown_operation(&operations);
        let resolved = self.resolved_picture(target);
        let registers = resolved.as_ref().map_or_else(
            || curvyo_laser_core::adjust::AdjustRegisters::read(&shown.adjustments),
            |r| r.registers.clone(),
        );
        let preset = registers.preset;
        let active = preset != Preset::Expert;
        let entries = &shown.adjustments.steps;
        Some(AdjustmentsView {
            restore_enabled: shown.adjustments.is_restorable(),
            before_after_open: self.image_adjust.pane.open,
            state_line: state_line(&operations, shown_op),
            memory_line: resolved.as_ref().and_then(|r| {
                let held = self.adjust_pictures().held(r.key.as_ref()?)?;
                Some(memory_line(&r.steps, held.not_run?))
            }),
            presets: Preset::ALL
                .into_iter()
                .map(|p| PresetCellView {
                    key: p.key(),
                    cell: p.cell(),
                    name: p.name(),
                    tooltip: p.tooltip(),
                    pressed: p == preset,
                })
                .collect(),
            dither: dither_view(
                &curvyo_laser_core::settings::ImageSettings::DEFAULT.with_dither(registers.dither),
            ),
            strength: active.then_some(StrengthView {
                value: registers.strength,
                range: STRENGTH_RANGE_TEXT,
            }),
            derived: active.then(|| DerivedView {
                line: format!(
                    "From preset {}, Strength {} %",
                    preset.cell(),
                    registers.strength
                ),
                rows: resolved
                    .as_ref()
                    .map(|r| r.steps.iter().copied().map(derived_row).collect())
                    .unwrap_or_default(),
            }),
            add_tone: if active {
                Vec::new()
            } else {
                StepKind::ALL
                    .into_iter()
                    .map(|kind| AddButtonView {
                        kind: kind.key(),
                        name: format!("Add {}", kind.name()),
                    })
                    .collect()
            },
            add_geometry: Vec::new(),
            steps: (0..entries.len())
                .map(|i| {
                    let mut row = manual_row(entries, i, active);
                    if row.kind == StepKind::Levels.key() {
                        row.histogram = self
                            .levels_input(target, i)
                            .map(|h| super::levels_input::bars(&h));
                    }
                    row
                })
                .collect(),
            clone_row: match shown.owner {
                AdjustOwner::Image => None,
                AdjustOwner::CloneFollowing => Some("original"),
                AdjustOwner::CloneOwn => Some("own"),
            },
            notice: self.image_adjust.notice.clone(),
        })
    }

    /// [`Self::adjustments_view`] as JSON, `null` when the group is absent.
    #[must_use]
    pub fn adjustments_view_json(&self) -> String {
        serde_json::to_string(&self.adjustments_view()).unwrap_or_else(|_| "null".to_owned())
    }
}
