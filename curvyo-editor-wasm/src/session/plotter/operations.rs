//! The operations of a plotter page (`specs/0159-cutting-plotter` criteria 7 to 9): Cut and Draw
//! take the selection into the last operation of their kind or a new one, Draw before Cut, and
//! the numbers are edited field by field, each one history step.

use curvyo_cutter_core::{CutterSettings, Field, OpKind, edit_setting};
use curvyo_document_core::{NewOperation, NodeId, SettingsStep, SettingsWrite};

use super::super::Session;
use super::PlotterPage;

/// The notice of a selection with nothing to assign.
pub(in crate::session) const NOTHING_TO_ASSIGN: &str = "Select items to assign.";

impl PlotterPage {
    /// The numbers of operation `op` of `kind` as read, with how many were damaged.
    pub(in crate::session) fn settings_of(
        &self,
        kind: OpKind,
        op: &curvyo_document_core::OperationSnapshot,
    ) -> (CutterSettings, usize) {
        let empty = serde_json::Map::new();
        let stored = op
            .settings
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .unwrap_or(&empty);
        CutterSettings::read(kind, stored, &self.copy.values)
    }

    /// The next free name "`<Kind> <N>`".
    pub(in crate::session) fn next_name(&self, kind: OpKind) -> String {
        let taken: Vec<&str> = self.operations.iter().map(|op| op.name.as_str()).collect();
        (1..=taken.len() + 1)
            .map(|n| format!("{} {n}", kind.label()))
            .find(|name| !taken.contains(&name.as_str()))
            .unwrap_or_default()
    }

    /// Where a new operation of `kind` goes: with Cuts last on Draw before every Cut, else and
    /// for Cut at the end (criterion 9, UX C12).
    fn insert_index(&self, kind: OpKind) -> usize {
        match kind {
            OpKind::Draw if self.cuts_last => self
                .operations
                .iter()
                .position(|op| Self::kind_of(op) == Some(OpKind::Cut))
                .unwrap_or(self.operations.len()),
            OpKind::Cut | OpKind::Draw => self.operations.len(),
        }
    }
}

impl Session {
    /// A press on Cut or Draw (`kind`) on a plotter page (criterion 7): the selection goes into
    /// the last operation of that kind, or a new "`<Kind> <N>`" with the profile's default
    /// numbers when there is none or `shift` is held, in one step "Assign to `<name>`". The
    /// operation written to; `None` when nothing changes.
    pub fn press_plotter_kind(&mut self, kind: &str, shift: bool) -> Option<NodeId> {
        let kind = OpKind::parse(kind)?;
        let active = self.active_plotter()?;
        let selection = self.selection_targets();
        if selection.targets.is_empty() {
            self.operations.notice = Some(NOTHING_TO_ASSIGN.to_owned());
            return None;
        }
        let keys: Vec<Vec<NodeId>> = selection.targets.iter().map(|&id| vec![id]).collect();
        let reuse = if shift {
            None
        } else {
            active
                .operations
                .iter()
                .rev()
                .find(|op| PlotterPage::kind_of(op) == Some(kind))
        };
        if let Some(op) = reuse {
            let (id, name) = (op.id, op.name.clone());
            let changed =
                self.recording_named_counting(format!("Assign to {name}"), None, |this| {
                    this.document
                        .set_operation_members(id, &keys, true)
                        .unwrap_or(false)
                });
            return changed.then_some(id);
        }
        let name = active.next_name(kind);
        let settings = CutterSettings::defaults(&active.copy.values).write(kind);
        let new = NewOperation {
            kind: kind.key(),
            name: &name,
            settings: Some(&settings),
        };
        let index = active.insert_index(kind);
        self.recording_named_counting(format!("Assign to {name}"), None, |this| {
            this.document
                .add_operation(active.id, &new, index, &keys)
                .ok()
        })
    }

    /// Sets the field `field` (`pressure`, `speed`, `passes`) of operation `id` to the typed
    /// `text` (criterion 8), one step "Edit settings".
    ///
    /// # Errors
    /// The field message for a value that does not read or is out of its range.
    pub fn edit_plotter_setting(
        &mut self,
        id: NodeId,
        field: &str,
        text: &str,
    ) -> Result<bool, String> {
        let active = self.active_plotter().ok_or_else(String::new)?;
        let op = active
            .operations
            .iter()
            .find(|op| op.id == id)
            .ok_or_else(String::new)?;
        let kind = PlotterPage::kind_of(op).ok_or_else(String::new)?;
        let field = Field::parse(field).ok_or_else(String::new)?;
        let (settings, _) = active.settings_of(kind, op);
        let edited = edit_setting(kind, settings, field, text, active.copy.values.max_speed)?;
        let write = SettingsWrite {
            operation: id,
            value: Some(edited.write(kind)),
        };
        Ok(self.recording_unguarded(|this| {
            this.document
                .write_operation_settings(SettingsStep::EditSettings, &[write])
                .unwrap_or(false)
        }))
    }
}
