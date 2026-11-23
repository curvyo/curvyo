//! The Adjustments group of the Style tab as the host reads it (`specs/0135` criteria 1, 2, 9,
//! 25, 26, 28, 30, 59, 60; `ux-notes.md` §3, §9, §15): header, state line, Preset strip,
//! Strength, the derived block, the add toolbar, the manual rows and the clone override row.

use curvyo_document_core::AdjustOwner;
use curvyo_laser_core::adjust::{Band, ManualStep, Preset, StepKind};
use curvyo_ui_core::STRENGTH_RANGE_TEXT;
use serde::Serialize;

use super::picture::PictureOperation;
pub(super) use super::rows_view::field_views;
use super::rows_view::{derived_row, manual_row};
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
    pub range: String,
}

/// The refusal of a Resize Width above the working image's `width` (criteria 5, 14).
pub(super) fn resize_range_text(width: u32) -> String {
    format!("Width is 16 to {width} px.")
}

/// One cell of a Channel mix row's Channel preset strip (`ux-notes.md` §3.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChannelPresetView {
    /// The visible cell text ("Red").
    pub cell: &'static str,
    /// The accessible name ("Red filter").
    pub name: &'static str,
    /// Whether the row's three weights are this preset's.
    pub pressed: bool,
}

/// The extra body of a step row whose fields are not all plain numbers (`ux-notes.md` §3.8).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepBodyView {
    /// Curves: the points (Input %, Output %) the graph draws, in Input order.
    Curve {
        /// The points.
        points: Vec<[u8; 2]>,
    },
    /// Channel mix: the Channel preset strip.
    ChannelMix {
        /// The four cells.
        presets: Vec<ChannelPresetView>,
    },
    /// Resize: the height that follows and the working image's size.
    Resize {
        /// The height in pixels the width gives (the ratio kept, never enlarged).
        height_px: u32,
        /// "Working image 2,400 × 1,800 px".
        line: String,
    },
    /// An AI step: Run, the model's state, the stored result (`ux-notes.md` §8).
    Ai(Box<super::ai_view::AiBodyView>),
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
    /// The graph or strip of a Curves, Channel mix or Resize row; `None` for the others.
    pub body: Option<StepBodyView>,
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
    /// Whether Paste adjustments is enabled (greyed until a Copy, criterion 10).
    pub paste_enabled: bool,
    /// The Crop row the Crop bar edits (its "Crop on the canvas" pressed); `None` while closed.
    pub crop_bar_row: Option<usize>,
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
    /// Row 2 (Crop, Resize, Upscale, AI denoise, Remove background), shown with any preset.
    pub add_geometry: Vec<AddButtonView>,
    /// The manual stack.
    pub steps: Vec<StepRowView>,
    /// The clone's Adjustments override row: "original" or "own"; `None` for an image.
    pub clone_row: Option<&'static str>,
    /// "Stored results 120 MB of 512 MB" while the document holds any (criterion 66).
    pub stored_results: Option<super::ai_view::BudgetView>,
    /// The notice of the last edit, kept until read.
    pub notice: Option<String>,
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
        let size = self.picture_size(target);
        let stale = self.stale_rows(&registers);
        let add = |band: Band| {
            StepKind::ALL
                .into_iter()
                .filter(|kind| kind.band() == band)
                .map(|kind| AddButtonView {
                    kind: kind.key(),
                    name: format!("Add {}", kind.name()),
                })
                .collect::<Vec<_>>()
        };
        Some(AdjustmentsView {
            restore_enabled: shown.adjustments.is_restorable(),
            paste_enabled: self.can_paste_adjustments(),
            crop_bar_row: self.crop_bar_row(),
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
                    .map(|r| {
                        r.steps
                            .iter()
                            .copied()
                            .filter(|s| s.kind().band() == Band::Tone)
                            .map(derived_row)
                            .collect()
                    })
                    .unwrap_or_default(),
            }),
            add_tone: if active { Vec::new() } else { add(Band::Tone) },
            add_geometry: [add(Band::Geometry), add(Band::Mask)].concat(),
            steps: (0..entries.len())
                .map(|i| {
                    let mut row = manual_row(entries, i, active, size);
                    if row.kind == StepKind::Levels.key() {
                        row.histogram = self
                            .levels_input(target, i)
                            .map(|h| super::levels_input::bars(&h));
                    }
                    if let Some(step) = registers.steps.get(i).and_then(|s| s.step)
                        && let Some((body, folded)) = self.ai_body(entries, i, step, &stale)
                    {
                        row.line = format!("{folded}{}", row.line);
                        row.body = Some(StepBodyView::Ai(Box::new(body)));
                    }
                    row
                })
                .collect(),
            clone_row: match shown.owner {
                AdjustOwner::Image => None,
                AdjustOwner::CloneFollowing => Some("original"),
                AdjustOwner::CloneOwn => Some("own"),
            },
            stored_results: self.budget_view(),
            notice: self.image_adjust.notice.clone(),
        })
    }

    /// [`Self::adjustments_view`] as JSON, `null` when the group is absent.
    #[must_use]
    pub fn adjustments_view_json(&self) -> String {
        serde_json::to_string(&self.adjustments_view()).unwrap_or_else(|_| "null".to_owned())
    }
}
