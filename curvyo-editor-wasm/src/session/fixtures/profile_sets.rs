//! Keeps a page's sets in step with its profile record (ADR 0059 §10, ADR 0060 §5a;
//! `specs/0113-fixture-sets-and-pins` criteria 3, 44 to 47).

use curvyo_document_core::{
    AlignCommit, Direction, FixtureSetRecord, ManufacturingSnapshot, NodeId,
};
use curvyo_laser_core::{
    FixtureKind, GENERIC_LASER_ID, align_due, new_record_id, read_copy, sets_differ,
};

use super::super::Session;
use super::super::library_journal::{RecordKind, RecordTexts};
use super::super::manufacturing::saved_jobs;

/// Where the sets of a laser page live on this computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::session) enum SetsHome {
    /// Generic laser: it holds no sets (criterion 3).
    Generic,
    /// A user profile of this library, by id: its sets are edited here and written to it.
    Editable(String),
    /// A profile this computer does not have, or has damaged, older or newer: the stored mirror
    /// is shown read only (criterion 46). The profile's name.
    ReadOnly(String),
}

/// The source of the profile ids a set edit gives new nodes (ADR 0060 §1: lowercase UUID v4 from
/// 16 bytes): a splitmix64 stream seeded from the session's peer and the clock, mixed with the
/// host's random bytes when it hands them in (Proposal: the record ids need only be unique within
/// one record).
#[derive(Debug, Clone, Copy, Default)]
pub(in crate::session) struct RecordIds {
    state: Option<u64>,
}

impl RecordIds {
    fn step(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// The next id; `seed` starts the stream on first use.
    fn next(&mut self, seed: u64) -> String {
        let state = self.state.get_or_insert(seed);
        let mut bytes = [0_u8; 16];
        bytes[..8].copy_from_slice(&Self::step(state).to_le_bytes());
        bytes[8..].copy_from_slice(&Self::step(state).to_le_bytes());
        new_record_id(bytes)
    }

    /// Mixes the host's random bytes into the stream.
    fn mix(&mut self, random: &[u8], seed: u64) {
        let state = self.state.get_or_insert(seed);
        for chunk in random.chunks(8) {
            let mut word = [0_u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            *state ^= u64::from_le_bytes(word);
            Self::step(state);
        }
    }
}

/// The page's sets as the record keeps them: fixtures of a kind this build knows, their jobs as a
/// profile write keeps them (ADR 0052 §7).
pub(in crate::session) fn record_sets(sets: Vec<FixtureSetRecord>) -> Vec<FixtureSetRecord> {
    sets.into_iter()
        .map(|mut set| {
            set.fixtures = set
                .fixtures
                .into_iter()
                .filter(|f| FixtureKind::from_stored(&f.kind) != FixtureKind::Unknown)
                .map(saved_jobs)
                .collect();
            set
        })
        .collect()
}

/// The notice of an edit whose profile write failed (criterion 45).
fn write_failed_notice(name: &str) -> String {
    format!("Could not save {name}. The change was undone.")
}

impl Session {
    /// Where the sets of `manufacturing` live on this computer.
    pub(in crate::session) fn sets_home(&self, manufacturing: &ManufacturingSnapshot) -> SetsHome {
        let read = read_copy(
            manufacturing.profile.as_ref(),
            manufacturing.legacy_machine_name.as_deref(),
        );
        match read.copy.id.as_deref() {
            Some(GENERIC_LASER_ID) => SetsHome::Generic,
            Some(id) if self.machine_profiles().is_user(id) => SetsHome::Editable(id.to_owned()),
            _ => SetsHome::ReadOnly(read.copy.values.name),
        }
    }

    /// Whether the active laser page's sets are edited here: its profile is a user profile of
    /// this library (criteria 3, 46).
    #[must_use]
    pub fn fixture_sets_editable(&self) -> bool {
        self.active_manufacturing()
            .is_some_and(|m| matches!(self.sets_home(&m), SetsHome::Editable(_)))
    }

    /// Mixes the host's random bytes (`crypto.getRandomValues`) into the profile ids of new sets,
    /// fixtures and jobs.
    pub fn seed_record_ids(&mut self, random: &[u8]) {
        let seed = self.record_id_seed();
        self.fixtures.ids.mix(random, seed);
    }

    fn record_id_seed(&self) -> u64 {
        self.peer ^ (self.history_clock())().as_secs().rotate_left(32)
    }

    /// A new profile id.
    pub(in crate::session) fn new_record_id(&mut self) -> String {
        let seed = self.record_id_seed();
        self.fixtures.ids.next(seed)
    }

    /// Whether `id` is part of the active page's set mirror: a set, a set fixture, or a slot or
    /// job of one.
    fn in_set_mirror(&self, id: NodeId) -> bool {
        let Some(m) = self.active_manufacturing() else {
            return false;
        };
        m.sets.iter().any(|set| set.id == id)
            || m.fixtures.iter().filter(|f| f.set.is_some()).any(|f| {
                f.id == id
                    || f.slots.iter().any(|s| s.id == id)
                    || f.jobs.iter().any(|j| j.id == id)
            })
    }

    /// Runs `body`, a command on `target`, as one step: refused (`R::default()`) when `target`
    /// is part of a set mirror that is read only here; a content edit of the mirror writes the
    /// profile record in the same step (ADR 0060 §5a).
    pub(super) fn set_content<R: Default>(
        &mut self,
        target: NodeId,
        body: impl FnOnce(&mut Self) -> R,
    ) -> R {
        if !self.in_set_mirror(target) {
            return self.recording_unguarded(body);
        }
        if !self.fixture_sets_editable() {
            return R::default();
        }
        self.set_edit(body)
    }

    /// Runs `body` as one step of the active laser page; when it changed the contents of the
    /// page's set mirror, the same step gives every new node its profile id, writes the profile
    /// record, aligns every other page of the project on that profile and keeps the record part
    /// in the journal (ADR 0060 §5a). A drag commits at release only, so it writes once.
    pub(super) fn set_edit<R>(&mut self, body: impl FnOnce(&mut Self) -> R) -> R {
        let before = self.active_manufacturing().map(|m| (m.id, m.set_records()));
        self.recording_unguarded(|this| {
            let result = body(this);
            if let Some((id, before)) = before
                && this.has_unrecorded_steps()
            {
                this.write_set_edit(id, &before);
            }
            result
        })
    }

    /// The record part of a set edit of the manufacturing `id`, joined to the step just
    /// committed; nothing when its mirror's contents equal `before`.
    fn write_set_edit(&mut self, id: NodeId, before: &[FixtureSetRecord]) {
        let Some(m) = self.document.manufacturing(id) else {
            return;
        };
        let SetsHome::Editable(profile) = self.sets_home(&m) else {
            return;
        };
        if m.set_records() == before {
            return;
        }
        let Some(step) = self.document.own_steps().last().cloned() else {
            return;
        };
        let pages: Vec<NodeId> = self
            .document
            .manufacturings()
            .into_iter()
            .filter(|other| self.sets_home(other) == SetsHome::Editable(profile.clone()))
            .map(|other| other.id)
            .collect();
        let text_before = self.manufacturing.machine.profiles.text(&profile);
        let now = i64::try_from(super::super::materials::now_ms(self)).unwrap_or(i64::MAX);
        let seed = self.record_id_seed();
        let mut ids = self.fixtures.ids;
        let label = step.label.clone();
        self.document.continue_step();
        let profiles = &mut self.manufacturing.machine.profiles;
        let write = self.document.batch(&label, |d| {
            let _ = d.fill_fixture_set_uids(id, &mut || ids.next(seed), &label);
            let records = record_sets(d.manufacturing(id)?.set_records());
            let write = profiles.set_fixture_sets(&profile, records.clone(), now)?;
            let modified = profiles.find(&profile)?.modified;
            for page in &pages {
                let _ = d.align_fixture_sets(*page, &records, modified, AlignCommit::Step(&label));
            }
            Some(write)
        });
        self.fixtures.ids = ids;
        let Some(write) = write else {
            return;
        };
        let machine = &mut self.manufacturing.machine;
        let name = machine
            .profiles
            .find(&profile)
            .map(|p| p.values.name.clone())
            .unwrap_or_default();
        let after = machine.profiles.text(&profile);
        machine.push_library_write(write);
        self.library_journal.record(
            step.step,
            RecordKind::Machine,
            vec![RecordTexts {
                id: profile,
                name,
                before: text_before,
                after,
            }],
            Vec::new(),
        );
    }

    /// The machine record of an undone or redone set edit written back when it still is what the
    /// step left (ADR 0060 §5a; `name` is the short name for the notice); else the notice.
    pub(in crate::session) fn revert_machine_records(
        &mut self,
        direction: Direction,
        name: &str,
        records: &mut [RecordTexts],
    ) -> Option<String> {
        let undo = direction == Direction::Undo;
        let now = i64::try_from(super::super::materials::now_ms(self)).unwrap_or(i64::MAX);
        let machine = &mut self.manufacturing.machine;
        for record in records.iter_mut() {
            let (expected, earlier) = if undo {
                (&record.after, &record.before)
            } else {
                (&record.before, &record.after)
            };
            if machine.profiles.text(&record.id).as_deref() != expected.as_deref() {
                let done = if undo { "undone" } else { "redone" };
                return Some(format!("{name} changed since, its sets were not {done}."));
            }
            let Some(earlier) = earlier.clone() else {
                continue;
            };
            let Some(write) = machine.profiles.restore(&record.id, &earlier, now) else {
                continue;
            };
            // The text now in the library is what the next undo or redo of this step expects.
            let current = machine.profiles.text(&record.id);
            if undo {
                record.before = current;
            } else {
                record.after = current;
            }
            machine.push_library_write(write);
        }
        None
    }

    /// The host could not write the profile record `id` (ADR 0055 §9 (f), `0113` criterion 45):
    /// when the newest undo step is a set edit that wrote it, its document part is undone, the
    /// library takes the record back as it was and the notice says so. `None` otherwise.
    pub fn machine_record_write_failed(&mut self, id: &str) -> Option<String> {
        self.sync_history();
        let step = self.history.stack.undo_target()?.step;
        let record = self.library_journal.machine_record(step)?.clone();
        if record.id != id {
            return None;
        }
        self.library_journal.forget(step);
        self.undo();
        if let Some(before) = &record.before
            && let Ok(profile) =
                curvyo_laser_core::read_record(&curvyo_laser_core::record_file_name(id), before)
        {
            // The record on disk is still the one before: the library takes it back unwritten.
            let _ = self.manufacturing.machine.profiles.add(profile);
        }
        let notice = write_failed_notice(&super::super::library_journal::short_name(&record.name));
        self.post_fixture_notice(&notice);
        Some(notice)
    }

    /// The set part of Save as profile on the manufacturing `id` (ADR 0060 §6 amended): every
    /// node of the mirror gets its profile id and the page is aligned to the new record's time
    /// `modified`, joined to the Save as step when `joined` (else as its own step). The sets the
    /// new record holds.
    pub(in crate::session) fn save_set_mirror(
        &mut self,
        id: NodeId,
        modified: i64,
        joined: bool,
    ) -> Vec<FixtureSetRecord> {
        let seed = self.record_id_seed();
        let mut ids = self.fixtures.ids;
        let label = curvyo_document_core::SAVE_AS_PROFILE_COMMIT_LABEL;
        if joined {
            self.document.continue_step();
        }
        let sets = self.recording_unguarded(|this| {
            this.document.batch(label, |d| {
                let _ = d.fill_fixture_set_uids(id, &mut || ids.next(seed), label);
                let records = record_sets(d.manufacturing(id)?.set_records());
                let _ = d.align_fixture_sets(id, &records, modified, AlignCommit::Step(label));
                Some(records)
            })
        });
        self.fixtures.ids = ids;
        sets.unwrap_or_default()
    }

    /// The guarded alignment (ADR 0059 §10 a; criterion 47): every laser page (or only `only`)
    /// whose profile record here is newer than its mirror and differs from it takes the record's
    /// sets in one reserved commit each, no undo step. Nothing is written for a record that is
    /// not newer.
    pub(in crate::session) fn align_fixture_pages(&mut self, only: Option<NodeId>) -> bool {
        self.align_pages(only, true)
    }

    /// The alignment after an undo or redo of a set edit, done or refused (ADR 0060 §5a): every
    /// laser page whose mirror's contents differ from its loaded record takes the record's sets,
    /// whatever the two times say, in one reserved commit each.
    pub(in crate::session) fn align_fixture_pages_after_history(&mut self) -> bool {
        self.align_pages(None, false)
    }

    fn align_pages(&mut self, only: Option<NodeId>, guarded: bool) -> bool {
        let mut changed = false;
        for m in self.document.manufacturings() {
            if only.is_some_and(|id| id != m.id) {
                continue;
            }
            let SetsHome::Editable(profile) = self.sets_home(&m) else {
                continue;
            };
            let Some(record) = self.machine_profiles().find(&profile) else {
                continue;
            };
            let due = if guarded {
                align_due(&m.set_records(), m.sets_modified, record)
            } else {
                sets_differ(&m.set_records(), &record.fixture_sets)
            };
            if !due {
                continue;
            }
            let (sets, modified) = (record.fixture_sets.clone(), record.modified);
            changed |= self
                .document
                .align_fixture_sets(m.id, &sets, modified, AlignCommit::Reserved)
                .unwrap_or(false);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_ids_are_uuid_v4_and_differ() {
        let mut ids = RecordIds::default();
        let a = ids.next(7);
        let b = ids.next(7);
        assert!(curvyo_laser_core::is_record_id(&a), "{a}");
        assert_ne!(a, b);
        let mut mixed = RecordIds::default();
        mixed.mix(&[1, 2, 3], 7);
        assert_ne!(mixed.next(7), a, "the host's bytes change the stream");
    }

    #[test]
    fn the_write_failed_notice_fits_seventy_characters() {
        let name = super::super::super::library_journal::short_name(&"n".repeat(60));
        assert!(write_failed_notice(&name).chars().count() <= 70);
    }
}
