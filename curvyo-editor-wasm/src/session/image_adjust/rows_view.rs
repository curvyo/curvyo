//! The step rows of the Adjustments group as the host reads them (`specs/0135` criteria 5, 13,
//! 30): each row's fields, numbers and body.

use curvyo_document_core::AdjustEntry;
use curvyo_laser_core::adjust::{Band, CHANNEL_PRESETS, FieldSpec, ManualStep};
use curvyo_ui_core::move_step;

use super::view::{ChannelPresetView, StepBodyView, StepFieldView, StepRowView, resize_range_text};

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

pub(super) fn field_views(step: ManualStep) -> Vec<StepFieldView> {
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
                range: spec.range_text.to_owned(),
            })
        })
        .collect()
}

/// A derived, read-only row.
pub(super) fn derived_row(step: ManualStep) -> StepRowView {
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
        body: None,
    }
}

/// A number with a comma between thousands ("2,400").
fn thousands(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The body of `step` beyond its fields, for a picture of `size` pixels when known.
fn step_body(step: ManualStep, size: Option<(u32, u32)>) -> Option<StepBodyView> {
    match step {
        ManualStep::Curves { points, count } => Some(StepBodyView::Curve {
            points: points[..usize::from(count)]
                .iter()
                .map(|&(i, o)| [i, o])
                .collect(),
        }),
        ManualStep::ChannelMix { weights } => Some(StepBodyView::ChannelMix {
            presets: CHANNEL_PRESETS
                .iter()
                .map(|&(name, w)| ChannelPresetView {
                    cell: name.split(' ').next().unwrap_or(name),
                    name,
                    pressed: w == weights,
                })
                .collect(),
        }),
        ManualStep::Resize { width_px } => {
            let (w, h) = size?;
            let (rw, rh) = curvyo_image_core::resized_size(w, h, width_px);
            Some(StepBodyView::Resize {
                height_px: rh,
                // The size this Resize gives, so it follows a Width edit (UX review).
                line: format!("Working image {} × {} px", thousands(rw), thousands(rh)),
            })
        }
        _ => None,
    }
}

/// The manual row `index` of `entries` with `preset` active or not, for a picture of `size`
/// pixels when known; a preset runs instead of the tone rows only (criterion 30).
pub(super) fn manual_row(
    entries: &[AdjustEntry],
    index: usize,
    preset: bool,
    size: Option<(u32, u32)>,
) -> StepRowView {
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
            body: None,
        };
    };
    let preset = preset && step.kind().band() == Band::Tone;
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
        fields: {
            let mut fields = field_views(step);
            if let (ManualStep::Resize { .. }, Some((width, _)), Some(field)) =
                (step, size, fields.first_mut())
            {
                field.range = resize_range_text(width.clamp(16, 8_192));
            }
            // A Crop field's range as it stands (UX review UX-2).
            if let ManualStep::Crop { pct } = step {
                for field in &mut fields {
                    if let Some(text) = curvyo_ui_core::crop_range_text(pct, field.key) {
                        field.range = text;
                    }
                }
            }
            fields
        },
        up: move_step(entries, index, true).is_some(),
        down: move_step(entries, index, false).is_some(),
        accessible_name: format!("{}, {numbers}{suffix}", step.kind().name()),
        histogram: None,
        body: step_body(step, size),
    }
}
