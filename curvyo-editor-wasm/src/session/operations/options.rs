//! The edits of an unfolded row that no material touches (`specs/0111-laser-operation-tools`
//! criteria 19 to 24; ADR 0056 §5, §6): Interval in `op_settings.main` and the operation-only
//! settings in `op_options`, each typed value or toggle one step "Edit settings".

use curvyo_document_core::{NodeId, ProfileChange, SettingsStep, SettingsWrite};
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::{
    Interval, OptionEdit, Overscan, ScanAngle, SettingsRead, ZOffset, edit_interval, hatches,
    read_options,
};

use super::super::Session;

/// One field or toggle of the lower part, or the Interval of the upper part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionField {
    Interval,
    Crosshatch,
    Bidirectional,
    ScanAngle,
    Overscan,
    ZOffset,
}

impl OptionField {
    /// The field of its host name.
    fn of(name: &str) -> Option<Self> {
        Some(match name {
            "interval" => Self::Interval,
            "crosshatch" => Self::Crosshatch,
            "bidirectional" => Self::Bidirectional,
            "scan_angle" => Self::ScanAngle,
            "overscan" => Self::Overscan,
            "z_offset" => Self::ZOffset,
            _ => return None,
        })
    }

    /// The `op_options` edit of `value` typed into (or, for a toggle, any non-zero `value`
    /// pressing) this field; `Err` is the chip text of UX notes §10. `None` for Interval, which is
    /// no option.
    fn edit(self, value: f64) -> Option<Result<OptionEdit, &'static str>> {
        Some(match self {
            Self::Interval => return None,
            Self::Crosshatch => Ok(OptionEdit::Crosshatch(value != 0.0)),
            Self::Bidirectional => Ok(OptionEdit::Bidirectional(value != 0.0)),
            Self::ScanAngle => ScanAngle::typed(value).map(OptionEdit::ScanAngle),
            Self::Overscan => Overscan::typed(value).map(OptionEdit::Overscan),
            Self::ZOffset => ZOffset::typed(value).map(OptionEdit::ZOffset),
        })
    }
}

impl Session {
    /// Whether the active laser page's machine has a motorised Z axis (its profile copy,
    /// `specs/0111` criteria 21, 22).
    pub(in crate::session) fn active_motorised_z(&self) -> bool {
        self.active_copy()
            .is_some_and(|copy| copy.read.copy.values.motorised_z)
    }

    /// The Motorised Z axis switch of the page's profile copy (`specs/0111` criterion 22): one
    /// step "Motorised Z axis on" or "off". Whether it changed.
    pub fn set_machine_motorised_z(&mut self, on: bool) -> bool {
        let before = self.document.version();
        self.edit_copy(ProfileChange::MotorisedZ { on }, |copy| {
            if copy.values.motorised_z == on {
                return None;
            }
            copy.values.motorised_z = on;
            Some(None)
        });
        self.document.version() != before
    }

    /// Types `value` into the field `option` of operation `id`'s unfolded row (`interval`,
    /// `scan_angle`, `overscan`, `z_offset`), or presses the toggle `option` (`crosshatch`,
    /// `bidirectional`) on (`value` non-zero) or off (criteria 19, 20, 24). One step "Edit
    /// settings"; the whole `op_options` map is written. `Ok(false)` when nothing is written: the
    /// same value, a field the row does not show (a hatch option on a Cut, Z offset without a
    /// motorised Z axis, Interval with No settings, which waits in the draft), damaged settings or another page.
    ///
    /// # Errors
    /// The chip text of a value out of range (UX notes §10); nothing is written.
    pub fn edit_operation_option(
        &mut self,
        id: NodeId,
        option: &str,
        value: f64,
    ) -> Result<bool, String> {
        self.operations.notice = None;
        let field = OptionField::of(option).ok_or_else(|| "Enter a number.".to_owned())?;
        let Some(edit) = field.edit(value) else {
            let interval = Interval::typed(value).map_err(str::to_owned)?;
            return Ok(self.edit_interval(id, interval));
        };
        let edit = edit.map_err(str::to_owned)?;
        if field == OptionField::ZOffset && !self.active_motorised_z() {
            return Ok(false);
        }
        let Some(active) = self.active_operations() else {
            return Ok(false);
        };
        let Some(op) = active.operations.iter().find(|op| op.id == id && !op.grid) else {
            return Ok(false);
        };
        let Some(kind) = op.kind.as_deref().and_then(OperationKind::parse) else {
            return Ok(false);
        };
        let Ok(options) = read_options(op.options.as_ref(), kind) else {
            return Ok(false);
        };
        let Some(next) = options.with(kind, edit) else {
            return Ok(false);
        };
        let serde_json::Value::Object(map) = next.to_value(kind) else {
            return Ok(false);
        };
        Ok(self.recording_unguarded(|this| {
            this.document
                .write_operation_options(id, &map)
                .unwrap_or(false)
        }))
    }

    /// A typed Interval (criterion 19): written to `main.interval_mm` of the operation, never as
    /// an override, one step "Edit settings"; with No settings it waits in the draft and is
    /// written with power and speed.
    fn edit_interval(&mut self, id: NodeId, interval: Interval) -> bool {
        let Some(target) = self.settings_target(id) else {
            return false;
        };
        if target.read == SettingsRead::None {
            return hatches(target.kind) && self.type_interval_into_draft(id, interval);
        }
        let Some(next) = edit_interval(&target.read, target.kind, interval) else {
            return false;
        };
        let serde_json::Value::Object(map) = next.to_value() else {
            return false;
        };
        let write = SettingsWrite {
            operation: id,
            value: Some(map),
        };
        self.recording_unguarded(|this| {
            this.document
                .write_operation_settings(SettingsStep::EditSettings, &[write])
                .unwrap_or(false)
        })
    }
}
