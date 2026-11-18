//! The commands of a laser page's output (ADR 0065 §1, ADR 0064 §11, `specs/0029` criteria 2,
//! 27, 28, 38): its two process registers and the export step.

use loro::{LoroMap, LoroValue, ValueOrContainer};

use crate::document::{Document, OBJECTS_TREE};
use crate::manufacturing_codec::{KEY_PROCESS, string};
use crate::operation::OperationError;
use crate::operation_codec::KEY_OP_OUTPUT;
use crate::operation_settings_commands::SettingsWrite;
use crate::path_model::NodeId;

/// The label of the fixture jobs' material pick (criterion 2, Frage 221 B).
pub const SET_FIXTURE_MATERIAL_COMMIT_LABEL: &str = "set_fixture_material";
/// The label of Start set to Machine origin (criterion 38).
pub const SET_START_MACHINE_ORIGIN_COMMIT_LABEL: &str = "set_start_machine_origin";
/// The label of Start set to Head position (criterion 38).
pub const SET_START_HEAD_POSITION_COMMIT_LABEL: &str = "set_start_head_position";
/// The label of Start set to Home, then machine origin (criterion 38).
pub const SET_START_HOME_THEN_ORIGIN_COMMIT_LABEL: &str = "set_start_home_then_origin";
/// The label of the export step (criterion 28).
pub const EXPORT_JOB_COMMIT_LABEL: &str = "export_job";
/// The label of the send step (`specs/0124` criterion S10).
pub const SEND_JOB_COMMIT_LABEL: &str = "send_job";

/// The register field of the fixture jobs' material id.
const FIXTURE_MATERIAL_FIELD: &str = "fixture_material";
/// The register field of Start.
const START_FIELD: &str = "start";
/// The stored spellings of Start with their labels, in the pick's order (ADR 0065 §2).
const STARTS: [(&str, &str); 3] = [
    ("machine_origin", SET_START_MACHINE_ORIGIN_COMMIT_LABEL),
    ("head_position", SET_START_HEAD_POSITION_COMMIT_LABEL),
    ("home_then_origin", SET_START_HOME_THEN_ORIGIN_COMMIT_LABEL),
];

impl Document {
    /// The meta map of the manufacturing `id` and the key `x.<process>.<field>` of its register.
    fn output_register(
        &self,
        id: NodeId,
        field: &str,
    ) -> Result<(LoroMap, String), OperationError> {
        let node = self.manufacturing_node(id)?;
        let meta = self
            .loro()
            .get_tree(OBJECTS_TREE)
            .get_meta(node)
            .map_err(|_| OperationError::NoSuchManufacturing)?;
        let key = format!(
            "x.{}.{field}",
            string(&meta, KEY_PROCESS).unwrap_or_default()
        );
        Ok((meta, key))
    }

    /// Writes `value` (`None` deletes the register) in one step `label`; `Ok(false)` when it is
    /// already stored.
    fn write_output_register(
        &self,
        id: NodeId,
        field: &str,
        value: Option<LoroValue>,
        label: &str,
    ) -> Result<bool, OperationError> {
        let (meta, key) = self.output_register(id, field)?;
        let stored = match meta.get(&key) {
            Some(ValueOrContainer::Value(value)) => Some(value),
            Some(ValueOrContainer::Container(_)) => Some(LoroValue::Null),
            None => None,
        };
        if stored == value {
            return Ok(false);
        }
        match value {
            Some(value) => meta.insert(&key, value),
            None => meta.delete(&key),
        }
        .map_err(|_| OperationError::NoSuchManufacturing)?;
        self.commit_with_label(label);
        Ok(true)
    }

    /// Writes the fixture jobs' material pick `x.<process>.fixture_material` of the laser page
    /// `manufacturing` (a material id; `None` removes the pick) in one step "Pick fixture
    /// material". The id is opaque here; the laser extension reads it (ADR 0065 §1).
    ///
    /// # Errors
    /// [`OperationError::NoSuchManufacturing`].
    pub fn set_fixture_material(
        &self,
        manufacturing: NodeId,
        material: Option<&str>,
    ) -> Result<bool, OperationError> {
        self.write_output_register(
            manufacturing,
            FIXTURE_MATERIAL_FIELD,
            material.map(LoroValue::from),
            SET_FIXTURE_MATERIAL_COMMIT_LABEL,
        )
    }

    /// Writes Start `x.<process>.start` (`"machine_origin"`, `"head_position"` or
    /// `"home_then_origin"`) in one step "Start: `<option>`"; `Ok(false)` when it already holds
    /// that spelling. A damaged stored value differs from every spelling, so a pick rewrites it.
    ///
    /// # Errors
    /// [`OperationError::BadStart`] for another spelling, [`OperationError::NoSuchManufacturing`].
    pub fn set_start(&self, manufacturing: NodeId, start: &str) -> Result<bool, OperationError> {
        let (value, label) = STARTS
            .into_iter()
            .find(|(spelling, _)| *spelling == start)
            .ok_or(OperationError::BadStart)?;
        self.write_output_register(
            manufacturing,
            START_FIELD,
            Some(LoroValue::from(value)),
            label,
        )
    }

    /// The export or send step (criterion 28 a, b; ADR 0064 §11; `0124` S10), labelled `label`
    /// ([`EXPORT_JOB_COMMIT_LABEL`] or [`SEND_JOB_COMMIT_LABEL`]): switches Output off
    /// on every fixture job of `fixture_jobs_off` and writes the refreshed stored numbers
    /// `refreshed`, in one history step. An id that names no live fixture job or operation (gone
    /// while the save dialog was open) is skipped. Whether anything was written; nothing is
    /// committed when nothing differs.
    pub fn export_job(
        &self,
        label: &str,
        fixture_jobs_off: &[NodeId],
        refreshed: &[SettingsWrite],
    ) -> bool {
        let jobs: Vec<LoroMap> = fixture_jobs_off
            .iter()
            .filter_map(|&id| self.job_meta(id).ok().map(|(meta, _)| meta))
            .filter(|meta| {
                matches!(
                    meta.get(KEY_OP_OUTPUT),
                    Some(ValueOrContainer::Value(LoroValue::Bool(true)))
                )
            })
            .collect();
        let (metas, writes): (Vec<LoroMap>, Vec<SettingsWrite>) = refreshed
            .iter()
            .filter_map(|write| {
                self.writable_operation_meta(write.operation)
                    .ok()
                    .map(|meta| (meta, write.clone()))
            })
            .unzip();
        let mut changed = false;
        for meta in &jobs {
            changed |= meta.insert(KEY_OP_OUTPUT, false).is_ok();
        }
        changed |= Self::write_settings(&metas, &writes);
        if changed {
            self.commit_with_label(label);
        }
        changed
    }
}

#[cfg(test)]
#[path = "laser_output_commands_tests.rs"]
mod tests;
