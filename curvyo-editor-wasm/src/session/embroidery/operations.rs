//! The operations of an embroidery page (`specs/0160-embroidery` criteria 5, 6): the three kinds
//! take the selection into the last operation of their kind or a new one with the thread of its
//! first item, and the numbers and the thread are edited one at a time, each one history step.

use curvyo_document_core::{NewOperation, NodeId, OperationSnapshot, SettingsStep, SettingsWrite};
use curvyo_embroidery_core::{EmbroiderySettings, OpKind, ThreadColour, edit_setting};

use super::super::Session;
use super::EmbroideryPage;

/// The notice of a selection with nothing to assign.
const NOTHING_TO_ASSIGN: &str = "Select items to assign.";

impl EmbroideryPage {
    /// The numbers and thread of operation `op` of `kind` as read, with how many were damaged.
    pub(in crate::session) fn settings_of(
        kind: OpKind,
        op: &OperationSnapshot,
    ) -> (EmbroiderySettings, ThreadColour, usize) {
        let empty = serde_json::Map::new();
        let stored = op
            .settings
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .unwrap_or(&empty);
        EmbroiderySettings::read(kind, stored)
    }

    /// The next free name "`<Kind> <N>`".
    pub(in crate::session) fn next_name(&self, kind: OpKind) -> String {
        let taken: Vec<&str> = self.operations.iter().map(|op| op.name.as_str()).collect();
        (1..=taken.len() + 1)
            .map(|n| format!("{} {n}", kind.label()))
            .find(|name| !taken.contains(&name.as_str()))
            .unwrap_or_default()
    }
}

impl Session {
    /// The thread of a new operation of `kind` (criterion 5, Proposal): the first selected item's
    /// fill colour for a Tatami fill and its stroke colour for a line, the other one when that
    /// paint is off, black without an item.
    fn first_thread(&self, kind: OpKind, targets: &[NodeId]) -> ThreadColour {
        targets
            .first()
            .and_then(|&id| self.document.object(id))
            .map(|object| {
                let style = object.style();
                let fill = if kind == OpKind::Tatami {
                    style.fill.enabled || !style.stroke.enabled
                } else {
                    !style.stroke.enabled && style.fill.enabled
                };
                let c = if fill {
                    style.fill.color
                } else {
                    style.stroke.color
                };
                ThreadColour::rgb(c.r, c.g, c.b)
            })
            .unwrap_or_default()
    }

    /// A press on Running stitch, Tatami fill or Satin (`kind`) on an embroidery page (criterion
    /// 5): the selection goes into the last operation of that kind, or a new "`<Kind> <N>`" at the
    /// end of the list with the defaults and its first item's thread when there is none or
    /// `shift` is held, in one step "Assign to `<name>`". The operation written to; `None` when
    /// nothing changes.
    pub fn press_embroidery_kind(&mut self, kind: &str, shift: bool) -> Option<NodeId> {
        let kind = OpKind::parse(kind)?;
        let active = self.active_embroidery()?;
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
                .find(|op| EmbroideryPage::kind_of(op) == Some(kind))
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
        let thread = self.first_thread(kind, &selection.targets);
        let settings = EmbroiderySettings::default().write(kind, thread);
        let new = NewOperation {
            kind: kind.key(),
            name: &name,
            settings: Some(&settings),
        };
        let index = active.operations.len();
        self.recording_named_counting(format!("Assign to {name}"), None, |this| {
            this.document
                .add_operation(active.id, &new, index, &keys)
                .ok()
        })
    }

    fn embroidery_op(
        &self,
        id: NodeId,
    ) -> Result<(OpKind, EmbroiderySettings, ThreadColour), String> {
        let active = self.active_embroidery().ok_or_else(String::new)?;
        let op = active
            .operations
            .iter()
            .find(|op| op.id == id)
            .ok_or_else(String::new)?;
        let kind = EmbroideryPage::kind_of(op).ok_or_else(String::new)?;
        let (settings, thread, _) = EmbroideryPage::settings_of(kind, op);
        Ok((kind, settings, thread))
    }

    fn write_embroidery_op(
        &mut self,
        id: NodeId,
        value: serde_json::Map<String, serde_json::Value>,
    ) -> bool {
        let write = SettingsWrite {
            operation: id,
            value: Some(value),
        };
        self.document
            .write_operation_settings(SettingsStep::EditSettings, &[write])
            .unwrap_or(false)
    }

    /// Sets the field `field` (`stitch_length_mm`, `repeats`, `angle_deg`, `row_spacing_mm`,
    /// `max_stitch_mm`, `zigzag_spacing_mm`) of operation `id` to the typed `text` (criterion 6),
    /// one step "Edit settings".
    ///
    /// # Errors
    /// The field message for a value that does not read or is out of its range.
    pub fn edit_embroidery_setting(
        &mut self,
        id: NodeId,
        field: &str,
        text: &str,
    ) -> Result<bool, String> {
        let (kind, settings, thread) = self.embroidery_op(id)?;
        let edited = edit_setting(kind, settings, field, text)?;
        let value = edited.write(kind, thread);
        Ok(self.recording_unguarded(|this| this.write_embroidery_op(id, value)))
    }

    /// Sets the thread of operation `id` to `hex` ("#RRGGBB"), one step "Set thread colour"
    /// (UX 5.3).
    pub fn set_embroidery_thread(&mut self, id: NodeId, hex: &str) -> bool {
        let (Ok((kind, settings, _)), Some(thread)) =
            (self.embroidery_op(id), ThreadColour::parse(hex))
        else {
            return false;
        };
        let value = settings.write(kind, thread);
        self.recording_named_counting("Set thread colour".to_owned(), None, |this| {
            this.write_embroidery_op(id, value)
        })
    }
}
