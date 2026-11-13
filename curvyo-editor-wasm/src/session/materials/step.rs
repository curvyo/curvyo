//! One library change as one document step (ADR 0055 §9, ADR 0058 §3, §8): the record edits and
//! photo writes are prepared on the library first, then the document part and the root marker are
//! committed in one batch; a refused commit takes the records back, a done one is journalled.

use curvyo_document_core::{Document, MaterialStep, SettingsWrite};
use curvyo_laser_core::grids::TestId;
use curvyo_laser_core::library::{Material, PhotoWrite, RecordRefusal, StoreRefusal};

use super::super::Session;
use super::undo::{PhotoBytes, RecordTexts};
use super::{MaterialWrite, now_ms};

/// A library change being prepared: what each record and photo was before it, and the writes.
#[derive(Debug, Default)]
pub(in crate::session) struct LibraryChange {
    records: Vec<RecordTexts>,
    photos: Vec<PhotoBytes>,
    writes: Vec<MaterialWrite>,
    photo_writes: Vec<PhotoWrite>,
    retire: Vec<TestId>,
}

impl LibraryChange {
    /// Whether nothing was changed yet.
    pub(in crate::session) fn is_empty(&self) -> bool {
        self.records.is_empty() && self.photos.is_empty()
    }

    /// Retires `ids` when the step is done (ADR 0058 §4: not undone with it).
    pub(in crate::session) fn retire(&mut self, ids: impl IntoIterator<Item = TestId>) {
        self.retire.extend(ids);
    }
}

fn refusal_text(refusal: &StoreRefusal) -> String {
    refusal.to_string()
}

impl Session {
    /// Notes the record `id` as it is before the first edit of `change`.
    fn note_record(&self, change: &mut LibraryChange, id: &str) {
        if change.records.iter().any(|r| r.id == id) {
            return;
        }
        let library = &self.materials.library;
        change.records.push(RecordTexts {
            id: id.to_owned(),
            name: library.name_of(id).unwrap_or_default().to_owned(),
            before: library.text(id).map(str::to_owned),
            after: None,
        });
    }

    /// Edits the readable material `id` as part of `change`.
    ///
    /// # Errors
    /// The refusal's text.
    pub(in crate::session) fn change_record(
        &mut self,
        change: &mut LibraryChange,
        id: &str,
        edit: impl FnOnce(&mut Material) -> Result<(), RecordRefusal>,
    ) -> Result<(), String> {
        self.note_record(change, id);
        let write = self
            .materials
            .library
            .edit(id, edit)
            .map_err(|r| refusal_text(&r))?;
        change.writes.push(write.into());
        Ok(())
    }

    /// Adds `material` as a new record as part of `change`.
    ///
    /// # Errors
    /// The refusal's text.
    pub(in crate::session) fn add_record(
        &mut self,
        change: &mut LibraryChange,
        material: Material,
    ) -> Result<(), String> {
        let id = material.id.clone();
        self.note_record(change, &id);
        let write = self
            .materials
            .library
            .add(material)
            .map_err(|r| refusal_text(&r))?;
        change.writes.push(write.into());
        if let Some(record) = change.records.iter_mut().find(|r| r.id == id) {
            self.materials
                .library
                .name_of(&id)
                .unwrap_or_default()
                .clone_into(&mut record.name);
        }
        Ok(())
    }

    /// Deletes the record `id` as part of `change`; the material it held.
    ///
    /// # Errors
    /// The refusal's text.
    pub(in crate::session) fn delete_record(
        &mut self,
        change: &mut LibraryChange,
        id: &str,
    ) -> Result<Option<Material>, String> {
        self.note_record(change, id);
        let (write, material) = self
            .materials
            .library
            .delete(id)
            .map_err(|r| refusal_text(&r))?;
        change.writes.push(write.into());
        Ok(material)
    }

    /// Writes or removes a photo file as part of `change`, with the bytes it had before when
    /// this session knows them.
    pub(in crate::session) fn change_photo(&self, change: &mut LibraryChange, write: PhotoWrite) {
        let (name, after) = match &write {
            PhotoWrite::Write { name, bytes } => (name.clone(), Some(bytes.clone())),
            PhotoWrite::Remove { name } => (name.clone(), None),
        };
        let before = self.materials.photos.get(&name).cloned();
        match change.photos.iter_mut().find(|p| p.name == name) {
            Some(photo) => photo.after = after,
            None => change.photos.push(PhotoBytes {
                name,
                before,
                after,
            }),
        }
        change.photo_writes.push(write);
    }

    /// Takes the records of `change` back to their texts before it (a refused step).
    fn undo_change(&mut self, change: &LibraryChange) {
        for record in change.records.iter().rev() {
            self.materials
                .library
                .restore(&record.id, record.before.as_deref());
        }
    }

    /// Commits `change` as one step: `document` writes the document part inside one batch
    /// labelled `label`, then the root marker of the last changed material and the refreshed
    /// numbers of every operation pointing to a changed material join it (ADR 0054 §3). A
    /// change of no record commits `document` alone. On a refusal of `document` or of the commit
    /// the records go back and nothing is written.
    ///
    /// # Errors
    /// The refusal's text, or "Nothing changed.".
    pub(in crate::session) fn commit_library_change<R>(
        &mut self,
        label: &str,
        mut change: LibraryChange,
        document: impl FnOnce(&Document) -> Result<R, String>,
    ) -> Result<R, String> {
        for record in &mut change.records {
            record.after = self.materials.library.text(&record.id).map(str::to_owned);
        }
        let marker = change.records.last().map(|record| {
            let modified = self
                .materials
                .library
                .find(&record.id)
                .map_or_else(|| now_ms(self), |m| m.modified);
            (record.id.clone(), modified)
        });
        let mut refreshed: Vec<SettingsWrite> = Vec::new();
        for record in &change.records {
            refreshed.extend(self.refreshed_settings(Some(&record.id)));
        }
        let steps = self.undo_len();
        let result = self.recording_unguarded(|this| {
            this.document.batch(label, |d| {
                let out = document(d)?;
                if let Some((id, modified)) = &marker {
                    d.commit_material_edit(MaterialStep::EditMaterial, id, *modified, &refreshed)
                        .map_err(|_| "Nothing changed.".to_owned())?;
                }
                Ok(out)
            })
        });
        let done = result.is_ok() && self.undo_len() > steps;
        if !done {
            self.undo_change(&change);
            return result.and(Err("Nothing changed.".to_owned()));
        }
        if let Some(target) = self.history.stack.undo_target()
            && !change.is_empty()
        {
            self.materials.journal.record(
                target.step,
                change.records.clone(),
                change.photos.clone(),
            );
        }
        self.materials.outbox.append(&mut change.writes);
        for write in change.photo_writes {
            self.materials.queue_photo(write);
        }
        if !change.retire.is_empty() {
            self.retire_test_ids(change.retire);
        }
        result
    }

    /// One library-only step labelled by `step`: `prepare` edits the library through `change`.
    ///
    /// # Errors
    /// The refusal's text.
    pub(in crate::session) fn library_step(
        &mut self,
        step: &str,
        prepare: impl FnOnce(&mut Self, &mut LibraryChange) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut change = LibraryChange::default();
        if let Err(text) = prepare(self, &mut change) {
            self.undo_change(&change);
            return Err(text);
        }
        self.commit_library_change(step, change, |_| Ok(()))
    }

    /// Adds `ids` to the retired ids and queues the file write (ADR 0058 §4); not a step.
    pub(in crate::session) fn retire_test_ids(&mut self, ids: Vec<TestId>) {
        match self.materials.retired.retire(ids) {
            Ok(true) => {
                let text = curvyo_laser_core::library::write_retired(&self.materials.retired);
                self.materials.outbox.push(MaterialWrite::Retired { text });
            }
            Ok(false) => {}
            Err(refused) => self.materials.notice = Some(refused.to_string()),
        }
    }
}
