//! The Machines tab's edits as undo steps (ADR 0098 decision 4, UX unify 4.2 and 4.4): each field
//! edit, add, duplicate, rename and delete is one document step with the generalised root marker,
//! and the record texts around it go into the library journal for the undo to write back.

use curvyo_document_core::{Direction, MachineStep};
use curvyo_laser_core::LibraryWrite;

use super::super::Session;
use super::super::library_journal::{RecordKind, RecordTexts};
use super::Family;

/// The History name of an edit of the field labelled `label`: "Edit max feed", "Edit Z travel".
fn edit_name(label: &str) -> String {
    let mut chars = label.chars();
    let lowered = match (chars.next(), chars.clone().next()) {
        // A one-letter word ("Z travel") keeps its capital.
        (Some(first), Some(' ')) => format!("{first}{}", chars.as_str()),
        (Some(first), _) => format!("{}{}", first.to_lowercase(), chars.as_str()),
        (None, _) => String::new(),
    };
    format!("Edit {lowered}")
}

impl Session {
    /// The ids and texts of every maker's machine record of every family.
    fn machine_texts(&self) -> Vec<(String, String)> {
        let m = &self.manufacturing;
        let ids = m
            .machine
            .profiles
            .listed()
            .into_iter()
            .map(|p| p.id.clone())
            .chain(m.plotter.profiles.iter().map(|p| p.id.clone()))
            .chain(m.embroidery.profiles.iter().map(|p| p.id.clone()))
            .chain(m.cnc.profiles.iter().map(|p| p.id.clone()));
        ids.filter_map(|id| self.machine_record_text(&id).map(|text| (id, text)))
            .collect()
    }

    /// Runs `body`, a record edit of the Machines tab, as one step `step` named `name` (the
    /// label's name when `None`): the root marker makes the step, the journal keeps every record
    /// text `body` changed. Nothing is recorded when no record changed.
    pub(super) fn machine_step<R>(
        &mut self,
        step: MachineStep,
        name: Option<String>,
        body: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let before = self.machine_texts();
        let result = body(self);
        let after = self.machine_texts();
        let mut records: Vec<RecordTexts> = Vec::new();
        for (id, text) in &before {
            let now = after.iter().find(|(a, _)| a == id).map(|(_, t)| t.clone());
            if now.as_ref() != Some(text) {
                records.push(RecordTexts {
                    id: id.clone(),
                    name: String::new(),
                    before: Some(text.clone()),
                    after: now,
                });
            }
        }
        for (id, text) in &after {
            if !before.iter().any(|(b, _)| b == id) {
                records.push(RecordTexts {
                    id: id.clone(),
                    name: String::new(),
                    before: None,
                    after: Some(text.clone()),
                });
            }
        }
        let Some(last) = records.last().map(|r| r.id.clone()) else {
            return result;
        };
        let names: Vec<String> = records
            .iter()
            .map(|r| {
                self.record_name(&r.id)
                    .or_else(|| r.before.as_deref().and_then(record_text_name))
                    .unwrap_or_default()
            })
            .collect();
        for (record, name) in records.iter_mut().zip(names) {
            record.name = name;
        }
        let modified = super::super::materials::now_ms(self);
        let commit = |this: &mut Self| this.document.commit_machine_edit(step, &last, modified);
        match name {
            Some(name) => self.recording_named(name, commit),
            None => self.recording_unguarded(commit),
        }
        if let Some(target) = self.history.stack.undo_target() {
            self.library_journal
                .record(target.step, RecordKind::MachineEdit, records, Vec::new());
        }
        result
    }

    /// Sets the field `key` of the maker's machine `id` from the typed `text` (`on`/`off` for a
    /// switch, the corner name for Origin): one step "Edit `field`", then the offer to update the
    /// pages that used the machine as it was (Frage 434 C). The connector address is a plain
    /// write without a step. `Ok` with the notice of a clamped value; `Ok(None)` when nothing
    /// changed.
    ///
    /// # Errors
    /// The field message; empty for a built-in, an unknown id or an unknown field.
    pub fn set_machine_value(
        &mut self,
        id: &str,
        key: &str,
        text: &str,
        now_ms: i64,
    ) -> Result<Option<String>, String> {
        if key == "connector" {
            return self.edit_machine_record(id, key, text, now_ms);
        }
        let label = self
            .family_of_record(id)
            .and_then(|family| {
                self.machine_rows(family)
                    .into_iter()
                    .find(|row| row.id == id)?
                    .fields
                    .into_iter()
                    .find(|field| field.key == key)
            })
            .map(|field| edit_name(field.label));
        self.machine_step(MachineStep::Edit, label, |this| {
            this.edit_machine_record(id, key, text, now_ms)
        })
    }

    /// Duplicate (UX unify 4.2): a copy of the machine `id` of any family, "`name` copy", in one
    /// step. The new id.
    pub fn duplicate_machine(&mut self, id: &str, random: [u8; 16], now_ms: i64) -> Option<String> {
        self.machine_step(MachineStep::Duplicate, None, |this| {
            this.duplicate_machine_record(id, random, now_ms)
        })
    }

    /// "+ Machine" of the family section `family` (`laser`, `plotter`, `embroidery`, `cnc`): a
    /// copy of the family's built-in, one step "Add machine". The new id.
    pub fn add_machine(&mut self, family: &str, random: [u8; 16], now_ms: i64) -> Option<String> {
        let builtin = match Family::of_key(family)? {
            Family::Laser => curvyo_laser_core::GENERIC_LASER_ID,
            Family::Plotter => curvyo_cutter_core::CRICUT_MAKER_3.id,
            Family::Embroidery => curvyo_embroidery_core::W6_EU7.id,
            Family::Cnc => curvyo_cnc_core::GENERIC_GRBL_ROUTER.id,
        };
        self.machine_step(MachineStep::Add, None, |this| {
            this.duplicate_machine_record(builtin, random, now_ms)
        })
    }

    /// Renames the maker's machine `id` of any family (1 to 60 characters) in one step; an empty
    /// name changes nothing. Whether it changed.
    pub fn rename_machine(&mut self, id: &str, typed: &str, now_ms: i64) -> bool {
        self.machine_step(MachineStep::Rename, None, |this| {
            this.rename_machine_record(id, typed, now_ms)
        })
    }

    /// Deletes the maker's machine `id` of any family in one step; pages keep their copy and show
    /// "Not in your machines" (ADR 0098 R2). Whether it was deleted.
    pub fn delete_machine(&mut self, id: &str) -> bool {
        self.machine_step(MachineStep::Delete, None, |this| {
            this.delete_machine_record(id)
        })
    }

    /// The machine records of an undone or redone Machines-tab step written back, each when it
    /// still is what the step left (the materials rule); else the notice. An undo ends the offer
    /// to update the pages (ADR 0098 R4).
    pub(in crate::session) fn revert_machine_edit_records(
        &mut self,
        direction: Direction,
        name: &str,
        records: &mut [RecordTexts],
    ) -> Option<String> {
        let undo = direction == Direction::Undo;
        if undo {
            self.machines_follow = None;
        }
        let current = records.iter().all(|r| {
            let expected = if undo { &r.after } else { &r.before };
            self.machine_record_text(&r.id) == *expected
        });
        if !current {
            let done = if undo { "undone" } else { "redone" };
            return Some(format!("{name} changed since; the machine was not {done}."));
        }
        let now = i64::try_from(super::super::materials::now_ms(self)).unwrap_or(i64::MAX);
        for record in records.iter_mut() {
            let earlier = if undo {
                record.before.clone()
            } else {
                record.after.clone()
            };
            let write = match &earlier {
                Some(text) => self.put_machine_text(&record.id, text, now),
                None => self.remove_machine_text(&record.id),
            };
            if let Some(write) = write {
                self.manufacturing.machine.queue(write);
            }
            // The text now in the library is what the next undo or redo of this step expects.
            let text = self.machine_record_text(&record.id);
            if undo {
                record.before = text;
            } else {
                record.after = text;
            }
        }
        None
    }

    /// Puts the record text `text` back as the maker's machine `id` of the family it names, with
    /// a `modified` above the one it replaces so other devices take it as newer.
    fn put_machine_text(&mut self, id: &str, text: &str, now: i64) -> Option<LibraryWrite> {
        let file = curvyo_laser_core::record_file_name(id);
        let newer = |old: Option<i64>| old.map_or(now, |old| now.max(old.saturating_add(1)));
        let m = &mut self.manufacturing;
        if let Ok(mut p) = curvyo_laser_core::read_record(&file, text) {
            if m.machine.profiles.is_user(id) {
                return m.machine.profiles.restore(id, text, now);
            }
            p.modified = now;
            return m.machine.profiles.add(p);
        }
        if let Ok(mut p) = curvyo_cutter_core::read_record(&file, text) {
            p.modified = newer(m.plotter.find(id).map(|o| o.modified));
            let text = curvyo_cutter_core::write_record(&p);
            m.plotter.put(p);
            return Some(LibraryWrite::Write {
                id: id.to_owned(),
                text,
            });
        }
        if let Ok(mut p) = curvyo_embroidery_core::read_record(&file, text) {
            p.modified = newer(m.embroidery.find(id).map(|o| o.modified));
            let text = curvyo_embroidery_core::write_record(&p);
            m.embroidery.put(p);
            return Some(LibraryWrite::Write {
                id: id.to_owned(),
                text,
            });
        }
        let mut p = curvyo_cnc_core::read_record(&file, text).ok()?;
        p.modified = newer(m.cnc.find(id).map(|o| o.modified));
        let text = curvyo_cnc_core::write_record(&p);
        m.cnc.put(p);
        Some(LibraryWrite::Write {
            id: id.to_owned(),
            text,
        })
    }

    /// Takes the maker's machine `id` of any family out of its library.
    fn remove_machine_text(&mut self, id: &str) -> Option<LibraryWrite> {
        let m = &mut self.manufacturing;
        if let Some(write) = m.machine.profiles.delete(id) {
            return Some(write);
        }
        let before = m.plotter.profiles.len() + m.embroidery.profiles.len() + m.cnc.profiles.len();
        m.plotter.profiles.retain(|p| p.id != id);
        m.embroidery.profiles.retain(|p| p.id != id);
        m.cnc.profiles.retain(|p| p.id != id);
        let after = m.plotter.profiles.len() + m.embroidery.profiles.len() + m.cnc.profiles.len();
        (after != before).then(|| LibraryWrite::Remove { id: id.to_owned() })
    }
}

/// The `name` a record text holds (a deleted record's name for the notices).
fn record_text_name(text: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    value["name"].as_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_history_name_lowers_the_label_but_keeps_a_one_letter_word() {
        assert_eq!(super::edit_name("Max feed"), "Edit max feed");
        assert_eq!(super::edit_name("Z travel"), "Edit Z travel");
    }
}
