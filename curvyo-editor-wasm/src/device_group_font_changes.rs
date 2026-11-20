//! What an import changed in the font entries (ADR 0062 decision 5, `specs/0116-library-sync`
//! criteria 11 and 17): switch-off and deletion notices, metadata changes for the page, and
//! the notice naming a field whose local edit lost.

use curvyo_document_core::{AssetHash, FontEntry};
use curvyo_ui_core::library_text as texts;

use super::DeviceGroupGlue;
use super::fonts::{FontArrival, FontChange, FontState, state_of, stem};

impl DeviceGroupGlue {
    /// The state of the font entries, before an import.
    pub(super) fn font_state(&self) -> FontState {
        state_of(self.library.font_entries())
    }

    /// Queues what an import changed in the font entries (switch-off and deletion notices,
    /// metadata changes, and a conflict notice naming a field changed here too). `replaced`:
    /// the import was a room snapshot that replaced the whole list, so an entry missing from
    /// it is not a deletion (a font switched on offline looks the same): the entry is gone but
    /// nothing is deleted here. A snapshot that merged reports a deletion like an update.
    pub(super) fn queue_font_changes(&mut self, before: &FontState, seen: bool, replaced: bool) {
        let after = self.font_state();
        for (hash, old) in before {
            let name = old.details.as_ref().map(|d| stem(&d.file_name).to_owned());
            match after.get(hash) {
                None if old.sync && replaced => {
                    self.fonts.delivered.remove(hash);
                    self.forget_blobs();
                }
                None if old.sync => {
                    self.fonts.delivered.remove(hash);
                    self.fonts.queued.push_back(FontArrival {
                        hash: hash.clone(),
                        change: FontChange::Removed,
                    });
                    self.notices
                        .push_back(texts::deleted_notice(name.as_deref().unwrap_or("A font")));
                    self.forget_blobs();
                }
                Some(new) if old.sync && !new.sync => {
                    self.fonts.delivered.remove(hash);
                    self.notices.push_back(texts::SYNC_OFF_NOTICE.to_owned());
                    self.forget_blobs();
                }
                Some(new) if old.sync && new.sync && old != new => {
                    self.queue_meta_change(hash, old, new, seen);
                }
                _ => {}
            }
        }
        if seen {
            self.fonts.dirty.clear();
        }
    }

    fn queue_meta_change(
        &mut self,
        hash: &AssetHash,
        old: &FontEntry,
        new: &FontEntry,
        seen: bool,
    ) {
        let (Some(was), Some(now)) = (&old.details, &new.details) else {
            return;
        };
        if !seen {
            for (field, differs) in [
                ("tags", was.tags != now.tags),
                ("star", was.favourite != now.favourite),
                ("Cuttable", was.cuttable != now.cuttable),
            ] {
                if differs && self.fonts.dirty.contains(&(hash.clone(), field)) {
                    self.notices
                        .push_back(texts::font_field_notice(stem(&now.file_name), field));
                }
            }
        }
        self.fonts.queued.push_back(FontArrival {
            hash: hash.clone(),
            change: FontChange::Meta(now.clone()),
        });
    }
}
