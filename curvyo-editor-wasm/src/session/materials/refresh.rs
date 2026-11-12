//! The stored numbers beside material links across the open project (ADR 0054 §3; Frage 187
//! B+C): which operations point to a material, and the values that bring their stored numbers to
//! the loaded library, at a material edit (no guard) and at a save (with the guard).

use curvyo_document_core::{NodeId, SettingsWrite};
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::{SettingsRead, read_settings, refresh};
use curvyo_laser_core::{PROCESS_ID, read_copy};

use super::super::Session;

/// One operation of a laser page with its kind, its stored settings and its page's profile id.
pub(in crate::session) struct LaserOperation {
    /// The operation.
    pub(in crate::session) id: NodeId,
    /// Its known kind.
    pub(in crate::session) kind: OperationKind,
    /// Its `op_settings` as read.
    pub(in crate::session) read: SettingsRead,
    /// The id of its page's profile copy, empty when the copy names none.
    pub(in crate::session) profile_id: String,
}

impl LaserOperation {
    /// The id of the material it points to.
    pub(in crate::session) fn material_id(&self) -> Option<&str> {
        match &self.read {
            SettingsRead::Stored(stored) => stored.material.as_ref().map(|link| link.id.as_str()),
            SettingsRead::None | SettingsRead::Damaged(_) => None,
        }
    }
}

/// Plain values of a value the laser rule returned (always a map).
fn plain(value: serde_json::Value) -> Option<serde_json::Map<String, serde_json::Value>> {
    match value {
        serde_json::Value::Object(map) => Some(map),
        _ => None,
    }
}

impl Session {
    /// Every operation of a known kind on every laser page of the project, grid rows left out.
    pub(in crate::session) fn laser_operations(&self) -> Vec<LaserOperation> {
        let mut out = Vec::new();
        for manufacturing in self.document.manufacturings() {
            if manufacturing.process != PROCESS_ID {
                continue;
            }
            let copy = read_copy(
                manufacturing.profile.as_ref(),
                manufacturing.legacy_machine_name.as_deref(),
            );
            let profile_id = copy.copy.id.unwrap_or_default();
            let Ok(operations) = self.document.operations(manufacturing.id) else {
                continue;
            };
            for op in operations {
                let Some(kind) = op.kind.as_deref().and_then(OperationKind::parse) else {
                    continue;
                };
                if op.grid {
                    continue;
                }
                out.push(LaserOperation {
                    id: op.id,
                    kind,
                    read: read_settings(op.settings.as_ref(), kind),
                    profile_id: profile_id.clone(),
                });
            }
        }
        out
    }

    /// The values that refresh the stored numbers of the linked operations: of the material
    /// `only` (a material edit, no guard) or of every material (`None`, the save, with the guard).
    pub(in crate::session) fn refreshed_settings(&self, only: Option<&str>) -> Vec<SettingsWrite> {
        let reads = self.materials.library().reads();
        self.laser_operations()
            .into_iter()
            .filter(|op| op.material_id().is_some())
            .filter(|op| only.is_none_or(|id| op.material_id() == Some(id)))
            .filter_map(|op| {
                let value = refresh(&op.read, op.kind, reads, &op.profile_id, only.is_none())?;
                Some(SettingsWrite {
                    operation: op.id,
                    value: plain(value.to_value()),
                })
            })
            .collect()
    }

    /// The save refresh (Frage 187 C): one reserved commit, no step and no History row, before
    /// the file text is taken; nothing is committed when nothing differs.
    pub(in crate::session) fn refresh_material_numbers_for_save(&mut self) {
        let refreshed = self.refreshed_settings(None);
        if refreshed.is_empty() {
            return;
        }
        let _ = self.document.refresh_material_numbers(&refreshed);
    }
}
