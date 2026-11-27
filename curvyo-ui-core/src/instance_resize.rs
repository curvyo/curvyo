//! A gear's resize as the shell shows it while the corner is dragged: the whole-tooth gear nearest
//! the pointer and its readout (`specs/0141` UX notes §15.6).

use curvyo_document_core::{Document, GeneratedSnapshot, ObjectSnapshot, Style};
use curvyo_extension_core::{Bound, ParamValue};

use crate::extensions::ExtensionSettings;
use crate::instance::{find_kind, plan_resize};
use crate::instance_notice::shown;
use crate::instance_preview::outline_path;

/// The kind that steps by whole teeth.
const GEAR: &str = "curvyo.generators/gear";

/// What a gear shows while its corner is dragged.
#[derive(Debug, Clone)]
pub struct ResizeView {
    /// The regenerated gear, the object's own id and style, replacing the scaled outline.
    pub preview: ObjectSnapshot,
    /// "34 teeth · ⌀ 72 mm", "6 teeth min · ..." and "200 teeth max · ..." at the bounds.
    pub readout: String,
}

/// The teeth-by-teeth view of a gear resize `result` (the object scaled as the pointer asks);
/// `None` for any other generated object, for a move or a turn (no size change) and for one the
/// generator refuses.
#[must_use]
pub fn resize_view(
    document: &Document,
    settings: &ExtensionSettings,
    result: &ObjectSnapshot,
) -> Option<ResizeView> {
    let id = result.id();
    let kind = document.generated(id)?.record.kind;
    let free = find_kind(settings, &kind)
        .is_some_and(|found| found.kind.resize == curvyo_extension_core::Resize::Free);
    if kind != GEAR && !free {
        return None;
    }
    let plan = plan_resize(document, settings, result).ok()?;
    if (plan.factor - 1.0).abs() < 1e-9 {
        return None;
    }
    if free {
        return free_view(document, result, plan);
    }
    let value = |key: &str| ParamValue::from_stored(plan.record.params.get(key)?);
    let teeth = value("teeth")?.count()?;
    let module = value("module")?.length()?.as_mm();
    // The generator reports the teeth it had to hold at 6 or 200.
    let bound = match plan.clamps.iter().find(|clamp| clamp.key == "teeth") {
        Some(clamp) if clamp.bound == Bound::Minimum => " min",
        Some(clamp) if clamp.bound == Bound::Maximum => " max",
        _ => "",
    };
    let readout = format!(
        "{teeth} teeth{bound} · ⌀ {} mm",
        shown(module * f64::from(teeth + 2), 1)
    );
    let style = match result {
        ObjectSnapshot::Path(path) => path.style.clone(),
        _ => Style::default(),
    };
    let regenerated = GeneratedSnapshot {
        id,
        record: plan.record,
        frame: plan.frame,
    };
    Some(ResizeView {
        preview: ObjectSnapshot::Path(outline_path(&regenerated, style)?),
        readout,
    })
}

/// The regenerated outline of a freely resized code and the number a scanner depends on: Module
/// width for a side drag, Bar height for a top or bottom drag, the box for a corner (UX notes of
/// `0144` §11.10); `None` readout text means the Select tool's own box readout.
fn free_view(
    document: &Document,
    result: &ObjectSnapshot,
    plan: crate::instance::ResizePlan,
) -> Option<ResizeView> {
    let before = document.generated(result.id())?.record.params;
    let length = |params: &curvyo_extension_core::StoredParams, key: &str| {
        ParamValue::from_stored(params.get(key)?)?
            .length()
            .map(curvyo_document_core::Length::as_mm)
    };
    let (module, height) = (
        (
            length(&plan.record.params, "module_width")?,
            length(&before, "module_width")?,
        ),
        (
            length(&plan.record.params, "bar_height")?,
            length(&before, "bar_height")?,
        ),
    );
    let moved = |pair: (f64, f64)| (pair.0 - pair.1).abs() > 1e-9;
    let bound = |key: &str| match plan.clamps.iter().find(|clamp| clamp.key == key) {
        Some(clamp) if clamp.bound == Bound::Minimum => " min",
        Some(clamp) if clamp.bound == Bound::Maximum => " max",
        _ => "",
    };
    let readout = match (moved(module), moved(height)) {
        (true, false) => format!(
            "Module width {} mm{}",
            crate::instance_notice::shown(module.0, 3),
            bound("module_width")
        ),
        (false, true) => format!(
            "Bar height {} mm{}",
            shown(height.0, 1),
            bound("bar_height")
        ),
        _ => String::new(),
    };
    let style = match result {
        ObjectSnapshot::Path(path) => path.style.clone(),
        _ => Style::default(),
    };
    let regenerated = GeneratedSnapshot {
        id: result.id(),
        record: plan.record,
        frame: plan.frame,
    };
    Some(ResizeView {
        preview: ObjectSnapshot::Path(outline_path(&regenerated, style)?),
        readout,
    })
}
