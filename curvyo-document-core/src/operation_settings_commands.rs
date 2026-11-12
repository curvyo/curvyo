//! The commands that write the settings registers `op_settings` and `op_options` of laser
//! operations (ADR 0054 §3, ADR 0056 §6; `specs/0028` criteria 24 to 27, `specs/0111` criteria 20,
//! 24); the values are made by the laser extension, which decides every rule.

use loro::{LoroMap, LoroValue, ValueOrContainer};

use crate::document::{Document, ROOT_MAP};
use crate::history::LABEL_REFRESH_MATERIAL_NUMBERS;
use crate::manufacturing_codec::{json_of, write_plain_map};
use crate::operation::OperationError;
use crate::operation_codec::{KEY_OP_OPTIONS, KEY_OP_SETTINGS, grid_switch};
use crate::path_model::NodeId;

/// The label of a material pick (criteria 25, 27).
pub const USE_MATERIAL_COMMIT_LABEL: &str = "use_material";
/// The label of a typed number (criterion 24).
pub const EDIT_OPERATION_SETTINGS_COMMIT_LABEL: &str = "edit_operation_settings";
/// The label of Reset to material (criterion 24).
pub const RESET_TO_MATERIAL_COMMIT_LABEL: &str = "reset_to_material";
/// The label of Own numbers in the pick (criterion 25).
pub const USE_OWN_NUMBERS_COMMIT_LABEL: &str = "use_own_numbers";
/// The label of Save as material (criterion 15; ADR 0055 §9).
pub const SAVE_AS_MATERIAL_COMMIT_LABEL: &str = "save_as_material";
/// The label of a set edit, a set removal or a thickness change of a material (criterion 22).
pub const EDIT_MATERIAL_COMMIT_LABEL: &str = "edit_material";
/// The label of a material rename (criterion 20).
pub const RENAME_MATERIAL_COMMIT_LABEL: &str = "rename_material";
/// The label of a material delete (criterion 21).
pub const DELETE_MATERIAL_COMMIT_LABEL: &str = "delete_material";

/// The root register a material edit writes so its step exists also when no operation points to
/// the material (ADR 0054 §3): `{ id, modified }`. Nothing reads it; every build keeps it.
pub(crate) const KEY_MATERIAL_EDIT: &str = "x.curvyo.laser.material_edit";

/// A step that writes the settings of operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsStep {
    /// Use material.
    UseMaterial,
    /// Edit settings.
    EditSettings,
    /// Reset to material.
    ResetToMaterial,
    /// Use own numbers.
    UseOwnNumbers,
}

impl SettingsStep {
    const fn label(self) -> &'static str {
        match self {
            Self::UseMaterial => USE_MATERIAL_COMMIT_LABEL,
            Self::EditSettings => EDIT_OPERATION_SETTINGS_COMMIT_LABEL,
            Self::ResetToMaterial => RESET_TO_MATERIAL_COMMIT_LABEL,
            Self::UseOwnNumbers => USE_OWN_NUMBERS_COMMIT_LABEL,
        }
    }
}

/// A material edit made in this project (ADR 0055 §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialStep {
    /// Save as material.
    SaveAsMaterial,
    /// Edit material.
    EditMaterial,
    /// Rename material.
    RenameMaterial,
    /// Delete material.
    DeleteMaterial,
}

impl MaterialStep {
    const fn label(self) -> &'static str {
        match self {
            Self::SaveAsMaterial => SAVE_AS_MATERIAL_COMMIT_LABEL,
            Self::EditMaterial => EDIT_MATERIAL_COMMIT_LABEL,
            Self::RenameMaterial => RENAME_MATERIAL_COMMIT_LABEL,
            Self::DeleteMaterial => DELETE_MATERIAL_COMMIT_LABEL,
        }
    }
}

/// The new `op_settings` of one operation, as plain values; `None` removes it (No settings).
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsWrite {
    /// The operation.
    pub operation: NodeId,
    /// Its new value.
    pub value: Option<serde_json::Map<String, serde_json::Value>>,
}

/// The register `key` as plain values, `None` when absent or not a plain map.
fn stored_at(meta: &LoroMap, key: &str) -> Option<serde_json::Value> {
    match meta.get(key)? {
        ValueOrContainer::Value(value) => json_of(&value),
        ValueOrContainer::Container(_) => None,
    }
}

impl Document {
    /// The meta maps of the operations of `writes`, each a live operation that is no grid row.
    fn settings_targets(&self, writes: &[SettingsWrite]) -> Result<Vec<LoroMap>, OperationError> {
        writes
            .iter()
            .map(|write| self.writable_operation_meta(write.operation))
            .collect()
    }

    /// The meta map of the live operation `id`, refused for a grid row.
    fn writable_operation_meta(&self, id: NodeId) -> Result<LoroMap, OperationError> {
        let meta = self.operation_meta(id)?;
        if grid_switch(&meta) != Some(false) {
            return Err(OperationError::GridRow);
        }
        Ok(meta)
    }

    /// Writes each value that differs from the stored one, uncommitted; whether any did.
    fn write_settings(metas: &[LoroMap], writes: &[SettingsWrite]) -> bool {
        let mut changed = false;
        for (meta, write) in metas.iter().zip(writes) {
            let now = stored_at(meta, KEY_OP_SETTINGS);
            match &write.value {
                Some(value)
                    if now.as_ref().and_then(serde_json::Value::as_object) != Some(value) =>
                {
                    write_plain_map(meta, KEY_OP_SETTINGS, value);
                    changed = true;
                }
                None if meta.get(KEY_OP_SETTINGS).is_some() => {
                    let _ = meta.delete(KEY_OP_SETTINGS);
                    changed = true;
                }
                _ => {}
            }
        }
        changed
    }

    /// Writes the settings of one or several operations in one history step `step`. `Ok(false)`
    /// and nothing written when every value is already stored.
    ///
    /// # Errors
    /// [`OperationError::NoSuchOperation`] when an id names no live operation;
    /// [`OperationError::GridRow`] for a test grid row, which has no settings (ADR 0054 §4).
    pub fn write_operation_settings(
        &self,
        step: SettingsStep,
        writes: &[SettingsWrite],
    ) -> Result<bool, OperationError> {
        let metas = self.settings_targets(writes)?;
        if !Self::write_settings(&metas, writes) {
            return Ok(false);
        }
        self.commit_with_label(step.label());
        Ok(true)
    }

    /// Writes the whole `op_options` map `value` of the operation `operation` in one history step
    /// "Edit settings" (ADR 0056 §6: one commit per field or toggle). `Ok(false)` and nothing
    /// written when it is already stored.
    ///
    /// # Errors
    /// [`OperationError::NoSuchOperation`] when the id names no live operation;
    /// [`OperationError::GridRow`] for a test grid row.
    pub fn write_operation_options(
        &self,
        operation: NodeId,
        value: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<bool, OperationError> {
        let meta = self.writable_operation_meta(operation)?;
        let now = stored_at(&meta, KEY_OP_OPTIONS);
        if now.as_ref().and_then(serde_json::Value::as_object) == Some(value) {
            return Ok(false);
        }
        write_plain_map(&meta, KEY_OP_OPTIONS, value);
        self.commit_with_label(EDIT_OPERATION_SETTINGS_COMMIT_LABEL);
        Ok(true)
    }

    /// The document part of a material edit made in this project (ADR 0054 §3, ADR 0055 §9): the
    /// root marker `x.curvyo.laser.material_edit` = `{ id, modified }` and the refreshed settings of
    /// every operation that points to the material, in one history step `step`.
    ///
    /// # Errors
    /// As [`Document::write_operation_settings`]; nothing is written then.
    pub fn commit_material_edit(
        &self,
        step: MaterialStep,
        material_id: &str,
        modified: u64,
        refreshed: &[SettingsWrite],
    ) -> Result<(), OperationError> {
        let metas = self.settings_targets(refreshed)?;
        Self::write_settings(&metas, refreshed);
        let mut marker = serde_json::Map::new();
        marker.insert("id".to_owned(), material_id.into());
        marker.insert("modified".to_owned(), modified.into());
        write_plain_map(&self.loro().get_map(ROOT_MAP), KEY_MATERIAL_EDIT, &marker);
        self.commit_with_label(step.label());
        Ok(())
    }

    /// The save refresh (Frage 187 C, ADR 0054 §3): writes the refreshed settings as one commit
    /// with the reserved label `refresh_material_numbers`, which is no step of the stacks and no
    /// row of the history list. Nothing is committed when nothing differs; `Ok(false)` then.
    ///
    /// # Errors
    /// As [`Document::write_operation_settings`]; nothing is written then.
    pub fn refresh_material_numbers(
        &self,
        refreshed: &[SettingsWrite],
    ) -> Result<bool, OperationError> {
        let metas = self.settings_targets(refreshed)?;
        let mut changed = false;
        self.commit_reserved(LABEL_REFRESH_MATERIAL_NUMBERS, |_| {
            changed = Self::write_settings(&metas, refreshed);
        });
        Ok(changed)
    }

    /// The root marker of the last material edit, as plain values (for tests and the goldens).
    #[must_use]
    pub fn material_edit_marker(&self) -> Option<serde_json::Value> {
        match self.loro().get_map(ROOT_MAP).get(KEY_MATERIAL_EDIT)? {
            ValueOrContainer::Value(value @ LoroValue::Map(_)) => json_of(&value),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "operation_settings_commands_tests.rs"]
mod tests;
