//! The operations of a CNC page (`specs/0161-cnc-output` criterion 5, UX 6): Profile outside and
//! Profile inside take the selection into the last Profile of that side or a new one with the
//! first tool and the whole stock thickness, and its numbers are edited one at a time, each one
//! history step.

use curvyo_cnc_core::{PROFILE_KIND, ProfileSettings, Side};
use curvyo_document_core::{NewOperation, NodeId, OperationSnapshot, SettingsStep, SettingsWrite};
use serde_json::{Map, Value};

use super::super::Session;
use super::CncPage;

/// The notice of a selection with nothing to assign.
const NOTHING_TO_ASSIGN: &str = "Select items to assign.";

fn object(value: Option<&Value>) -> Map<String, Value> {
    value
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

impl CncPage {
    /// The numbers of operation `op` as read, with how many were damaged.
    pub(in crate::session) fn settings_of(
        &self,
        op: &OperationSnapshot,
    ) -> (ProfileSettings, usize) {
        ProfileSettings::read(
            &object(op.options.as_ref()),
            &object(op.settings.as_ref()),
            &self.tools,
            self.stock.thickness.as_mm(),
        )
    }

    /// The next free name "Profile N".
    pub(in crate::session) fn next_name(&self) -> String {
        let taken: Vec<&str> = self.operations.iter().map(|op| op.name.as_str()).collect();
        (1..=taken.len() + 1)
            .map(|n| format!("Profile {n}"))
            .find(|name| !taken.contains(&name.as_str()))
            .unwrap_or_default()
    }

    /// The last Profile operation of `side`.
    pub(in crate::session) fn last_of(&self, side: Side) -> Option<&OperationSnapshot> {
        self.operations
            .iter()
            .rev()
            .find(|op| Self::is_profile(op) && self.settings_of(op).0.side == side)
    }
}

impl Session {
    /// A press on Profile outside or Profile inside (`side`: `outside`, `inside`; criterion 5,
    /// UX 6.1): the selection goes into the last Profile of that side, or a new "Profile N" at
    /// the end of the list when there is none or `shift` is held, in one step "Assign to
    /// `<name>`". The operation written to.
    pub fn press_cnc_profile(&mut self, side: &str, shift: bool) -> Option<NodeId> {
        let side = Side::parse(side)?;
        let active = self.active_cnc()?;
        let selection = self.selection_targets();
        if selection.targets.is_empty() {
            self.operations.notice = Some(NOTHING_TO_ASSIGN.to_owned());
            return None;
        }
        let keys: Vec<Vec<NodeId>> = selection.targets.iter().map(|&id| vec![id]).collect();
        if let Some(op) = active.last_of(side).filter(|_| !shift) {
            let (id, name) = (op.id, op.name.clone());
            let changed =
                self.recording_named_counting(format!("Assign to {name}"), None, |this| {
                    this.document
                        .set_operation_members(id, &keys, true)
                        .unwrap_or(false)
                });
            return changed.then_some(id);
        }
        let name = active.next_name();
        let mut settings = ProfileSettings::new(&active.tools[0], active.stock.thickness.as_mm());
        settings.side = side;
        let (options, feeds) = (settings.options(), settings.settings());
        let index = active.operations.len();
        self.recording_named_counting(format!("Assign to {name}"), None, |this| {
            this.document.batch("add_operation", |d| {
                let new = NewOperation {
                    kind: PROFILE_KIND,
                    name: &name,
                    settings: Some(&feeds),
                };
                let id = d.add_operation(active.id, &new, index, &keys).ok()?;
                d.write_operation_options(id, &options).ok()?;
                Some(id)
            })
        })
    }

    /// Takes the selection out of operation `id` of the active CNC page, one step.
    pub fn remove_selection_from_cnc_operation(&mut self, id: NodeId) -> bool {
        let Some(active) = self.active_cnc() else {
            return false;
        };
        let Some(op) = active.operations.iter().find(|op| op.id == id) else {
            return false;
        };
        let keys: Vec<Vec<NodeId>> = self
            .selection_targets()
            .targets
            .iter()
            .map(|&t| vec![t])
            .collect();
        let name = op.name.clone();
        self.recording_named_counting(format!("Remove from {name}"), None, |this| {
            this.document
                .set_operation_members(id, &keys, false)
                .unwrap_or(false)
        })
    }

    /// Deletes operation `id` of the active CNC page with its memberships, one step.
    pub fn delete_cnc_operation(&mut self, id: NodeId) -> bool {
        let Some(active) = self.active_cnc() else {
            return false;
        };
        active.operations.iter().any(|op| op.id == id)
            && self.recording_unguarded(|this| this.document.delete_operation(id).is_ok())
    }

    /// Sets the field `field` (`side`, `direction`, `depth_mm`, `tool`, `feed_mm_min`,
    /// `plunge_mm_min`) of Profile `id` to the typed `text` (criterion 5, UX 6.3), one step
    /// "Edit settings"; an empty feed follows the tool again.
    ///
    /// # Errors
    /// The field message for a value that does not read or is out of its range.
    pub fn edit_cnc_setting(
        &mut self,
        id: NodeId,
        field: &str,
        text: &str,
    ) -> Result<bool, String> {
        let active = self.active_cnc().ok_or_else(String::new)?;
        let op = active
            .operations
            .iter()
            .find(|op| op.id == id && CncPage::is_profile(op))
            .ok_or_else(String::new)?;
        let (settings, _) = active.settings_of(op);
        let edited = settings.edit(field, text, &active.tools, active.stock.thickness.as_mm())?;
        let (options, feeds) = (edited.options(), edited.settings());
        Ok(self.recording_unguarded(|this| {
            this.document.batch("edit_operation_settings", |d| {
                let a = d.write_operation_options(id, &options).unwrap_or(false);
                let write = SettingsWrite {
                    operation: id,
                    value: Some(feeds),
                };
                let b = d
                    .write_operation_settings(SettingsStep::EditSettings, &[write])
                    .unwrap_or(false);
                a || b
            })
        }))
    }
}
