//! The commands of a laser page's output (ADR 0065 §1, ADR 0064 §11, `specs/0029` criteria 2,
//! 27, 28, 38; `specs/0118` O1, O2): its three process registers and the export step.

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
/// The label of Start set to User origin (`specs/0118` criterion 30).
pub const SET_START_USER_ORIGIN_COMMIT_LABEL: &str = "set_start_user_origin";
/// The label of the user origin stored with Start User origin (`specs/0118` criterion 29).
pub const SET_USER_ORIGIN_COMMIT_LABEL: &str = "set_user_origin";
/// The label of the user origin typed in the X, Y row (`specs/0118` W43), its own step "User origin".
pub const TYPE_USER_ORIGIN_COMMIT_LABEL: &str = "type_user_origin";
/// The largest distance of a user origin from the machine origin on either axis, in µm (10 m).
pub const USER_ORIGIN_LIMIT_UM: i64 = 10_000_000;
/// The label of the export step (criterion 28).
pub const EXPORT_JOB_COMMIT_LABEL: &str = "export_job";
/// The label of the send step (`specs/0125` criterion S10).
pub const SEND_JOB_COMMIT_LABEL: &str = "send_job";

/// The register field of the fixture jobs' material id.
const FIXTURE_MATERIAL_FIELD: &str = "fixture_material";
/// The register field of Start.
const START_FIELD: &str = "start";
/// The register field of the user origin (`specs/0118` O1).
const USER_ORIGIN_FIELD: &str = "user_origin_um";
/// The spelling of Start User origin.
const USER_ORIGIN_START: &str = "user_origin";
/// The stored spellings of Start with their labels, in the pick's order (ADR 0065 §2).
const STARTS: [(&str, &str); 4] = [
    ("machine_origin", SET_START_MACHINE_ORIGIN_COMMIT_LABEL),
    ("head_position", SET_START_HEAD_POSITION_COMMIT_LABEL),
    ("home_then_origin", SET_START_HOME_THEN_ORIGIN_COMMIT_LABEL),
    (USER_ORIGIN_START, SET_START_USER_ORIGIN_COMMIT_LABEL),
];

/// The stored form of the user origin `point`.
fn origin_value(point: [i64; 2]) -> LoroValue {
    LoroValue::from(vec![LoroValue::I64(point[0]), LoroValue::I64(point[1])])
}

/// Whether `value` is a readable user origin: two integers within the limit.
fn readable_origin(value: Option<&ValueOrContainer>) -> bool {
    let Some(ValueOrContainer::Value(LoroValue::List(items))) = value else {
        return false;
    };
    items.len() == 2
        && items
            .iter()
            .all(|item| matches!(item, LoroValue::I64(n) if n.abs() <= USER_ORIGIN_LIMIT_UM))
}

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

    /// Writes Start `x.<process>.start` (`"machine_origin"`, `"head_position"`,
    /// `"home_then_origin"` or `"user_origin"`) in one step "Start: `<option>`"; `Ok(false)` when
    /// it already holds that spelling. A damaged stored value differs from every spelling, so a
    /// pick rewrites it.
    ///
    /// # Errors
    /// [`OperationError::BadStart`] for another spelling, [`OperationError::NoUserOrigin`] for
    /// `"user_origin"` without a readable stored point, [`OperationError::NoSuchManufacturing`].
    pub fn set_start(&self, manufacturing: NodeId, start: &str) -> Result<bool, OperationError> {
        let (value, label) = STARTS
            .into_iter()
            .find(|(spelling, _)| *spelling == start)
            .ok_or(OperationError::BadStart)?;
        if value == USER_ORIGIN_START {
            let (meta, key) = self.output_register(manufacturing, USER_ORIGIN_FIELD)?;
            if !readable_origin(meta.get(&key).as_ref()) {
                return Err(OperationError::NoUserOrigin);
            }
        }
        self.write_output_register(
            manufacturing,
            START_FIELD,
            Some(LoroValue::from(value)),
            label,
        )
    }

    /// Stores `point` (µm of the job file's coordinates) as the page's user origin
    /// `x.<process>.user_origin_um` and Start `"user_origin"` in one step "Set origin here"
    /// (`specs/0118` criterion 29, O2), so one undo takes both back; `Ok(false)` when both are
    /// already stored.
    ///
    /// # Errors
    /// [`OperationError::BadUserOrigin`] beyond [`USER_ORIGIN_LIMIT_UM`] on either axis,
    /// [`OperationError::NoSuchManufacturing`].
    pub fn set_user_origin(
        &self,
        manufacturing: NodeId,
        point: [i64; 2],
    ) -> Result<bool, OperationError> {
        self.store_user_origin(manufacturing, point, SET_USER_ORIGIN_COMMIT_LABEL)
    }

    /// The typed X, Y pair of the User origin row: what [`Self::set_user_origin`] stores, as its
    /// own step "User origin" (`specs/0118` W43).
    ///
    /// # Errors
    /// As [`Self::set_user_origin`].
    pub fn type_user_origin(
        &self,
        manufacturing: NodeId,
        point: [i64; 2],
    ) -> Result<bool, OperationError> {
        self.store_user_origin(manufacturing, point, TYPE_USER_ORIGIN_COMMIT_LABEL)
    }

    fn store_user_origin(
        &self,
        manufacturing: NodeId,
        point: [i64; 2],
        label: &str,
    ) -> Result<bool, OperationError> {
        if point.iter().any(|n| n.abs() > USER_ORIGIN_LIMIT_UM) {
            return Err(OperationError::BadUserOrigin);
        }
        let (meta, origin_key) = self.output_register(manufacturing, USER_ORIGIN_FIELD)?;
        let (_, start_key) = self.output_register(manufacturing, START_FIELD)?;
        let value = origin_value(point);
        let stored = |key: &str| match meta.get(key) {
            Some(ValueOrContainer::Value(value)) => Some(value),
            _ => None,
        };
        let start = LoroValue::from(USER_ORIGIN_START);
        if stored(&origin_key) == Some(value.clone()) && stored(&start_key) == Some(start.clone()) {
            return Ok(false);
        }
        meta.insert(&origin_key, value)
            .and_then(|()| meta.insert(&start_key, start))
            .map_err(|_| OperationError::NoSuchManufacturing)?;
        self.commit_with_label(label);
        Ok(true)
    }

    /// The export or send step (criterion 28 a, b; ADR 0064 §11; `0125` S10), labelled `label`
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
