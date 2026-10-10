//! The commit path: how every edit becomes one commit with a step header, and how a gesture that
//! needs several writes still becomes one step (ADR 0014 §2; `specs/0020-undo-redo` criteria 13,
//! 14, 54 and 58).

use std::cell::{Cell, RefCell};

use loro::{CommitOptions, Frontiers};

use super::step_header::{LABEL_NEW_DOCUMENT, StepHeader};
use super::step_id::StepRef;
use crate::document::Document;
use crate::units::Timestamp;

/// The newest commit of this session, which a continuation may join.
#[derive(Debug, Clone)]
struct LastCommit {
    step: StepRef,
    header: String,
    /// The committed frontiers right after the commit.
    version: Frontiers,
}

/// A step this session started, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnStep {
    /// The step's identity.
    pub step: StepRef,
    /// The commit label of the step, as written (`translate_objects`).
    pub label: String,
}

/// The session state of the commit path.
#[derive(Debug, Default)]
pub(crate) struct CommitState {
    /// The counter of the last step this session started.
    seq: Cell<u64>,
    time: Cell<Timestamp>,
    continue_next: Cell<bool>,
    batch_depth: Cell<u32>,
    last: RefCell<Option<LastCommit>>,
    own: RefCell<Vec<OwnStep>>,
    /// The history was read since the document was opened (see `Document::warm_history`).
    warm: Cell<bool>,
}

/// Ends a [`Document::batch`] even when the closure panics.
struct BatchGuard<'a>(&'a CommitState);

impl Drop for BatchGuard<'_> {
    fn drop(&mut self) {
        self.0.batch_depth.set(self.0.batch_depth.get() - 1);
    }
}

impl CommitState {
    pub(super) fn is_warm(&self) -> bool {
        self.warm.get()
    }

    pub(crate) fn set_warm(&self) {
        self.warm.set(true);
    }
}

impl Document {
    /// Sets the wall-clock time the next steps carry. The editor passes it in; this crate reads no
    /// clock (`CLAUDE.md` §6).
    pub fn set_step_time(&self, time: Timestamp) {
        self.commit_state.time.set(time);
    }

    /// Makes the next commit repeat the header of this session's last step and join it, when
    /// nothing was committed or merged since (a held nudge is one step). Does nothing when the
    /// document version moved: the next commit then starts a new step.
    pub fn continue_step(&self) {
        let allowed = self
            .commit_state
            .last
            .borrow()
            .as_ref()
            .is_some_and(|last| last.version == self.loro().oplog_frontiers());
        self.commit_state.continue_next.set(allowed);
    }

    /// Runs `write` and commits everything it wrote as one step labelled `label`. Commits the
    /// closure's inner methods would make are deferred to the end; a closure that writes nothing
    /// adds no step.
    pub fn batch<R>(&self, label: &str, write: impl FnOnce(&Self) -> R) -> R {
        let outermost = self.commit_state.batch_depth.get() == 0;
        self.commit_state
            .batch_depth
            .set(self.commit_state.batch_depth.get() + 1);
        let result = {
            let _guard = BatchGuard(&self.commit_state);
            write(self)
        };
        if outermost {
            self.commit_step(label, &[]);
        }
        result
    }

    /// The steps this session started, oldest first. A continuation joins its step and is not
    /// listed again.
    #[must_use]
    pub fn own_steps(&self) -> Vec<OwnStep> {
        self.commit_state.own.borrow().clone()
    }

    /// The steps this session started from index `from` on (a cheap way to ask for the new ones).
    #[must_use]
    pub fn own_steps_from(&self, from: usize) -> Vec<OwnStep> {
        self.commit_state
            .own
            .borrow()
            .get(from..)
            .map(<[OwnStep]>::to_vec)
            .unwrap_or_default()
    }

    /// The number of steps this session started.
    #[must_use]
    pub fn own_step_count(&self) -> usize {
        self.commit_state.own.borrow().len()
    }

    /// Commits the pending writes with `label` as the (persisted) commit message of one step: the
    /// header `<label>;s=<seq>;t=<seconds>`. Each editing method ends with this call, so one
    /// interaction is one commit (ADR 0002 §9). Inside a [`Document::batch`] it defers to the
    /// batch's commit.
    pub(crate) fn commit_with_label(&self, label: &str) {
        if self.commit_state.batch_depth.get() == 0 {
            self.commit_step(label, &[]);
        }
    }

    /// Commits the pending writes as one step with the given extra header keys. Returns the step,
    /// or `None` when nothing was pending (no step is added).
    pub(crate) fn commit_step(&self, label: &str, extra: &[(&str, String)]) -> Option<StepRef> {
        let state = &self.commit_state;
        let pending = self.loro().get_pending_txn_len();
        if pending == 0 {
            state.continue_next.set(false);
            return None;
        }
        // `continue_step` checked that nothing else was committed or merged since the last step.
        let joined = state
            .continue_next
            .replace(false)
            .then(|| state.last.borrow().clone())
            .flatten();
        if let Some(last) = joined {
            let step = self.finish_commit(&last.header, Some(last.step), pending);
            return Some(step);
        }
        let header = self.next_header(label, extra);
        let step = self.finish_commit(&header.format(), None, pending);
        state.own.borrow_mut().push(OwnStep {
            step,
            label: label.to_owned(),
        });
        Some(step)
    }

    /// The header of the next step this session starts; the step counter moves on.
    pub(crate) fn next_header(&self, label: &str, extra: &[(&str, String)]) -> StepHeader {
        let state = &self.commit_state;
        state.seq.set(state.seq.get() + 1);
        let mut header = StepHeader::new(label, state.seq.get(), state.time.get());
        header.extra = extra
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.clone()))
            .collect();
        header
    }

    /// Commits the pending writes with the given header and remembers the commit as the one a
    /// continuation may join. Not listed among [`Document::own_steps`]: undo and redo use it.
    pub(crate) fn commit_with_header(&self, header: &StepHeader) {
        let pending = self.loro().get_pending_txn_len();
        if pending > 0 {
            self.finish_commit(&header.format(), None, pending);
        } else {
            self.note_last_commit(header);
        }
    }

    fn finish_commit(&self, header: &str, existing: Option<StepRef>, pending: usize) -> StepRef {
        let peer = self.loro().peer_id();
        self.loro()
            .commit_with(CommitOptions::new().commit_msg(header));
        let end = self.loro().oplog_vv().get(&peer).copied().unwrap_or(0);
        let first = end - i32::try_from(pending).unwrap_or(i32::MAX);
        let step = existing.unwrap_or_else(|| StepRef::new(peer, first));
        *self.commit_state.last.borrow_mut() = Some(LastCommit {
            step,
            header: header.to_owned(),
            version: self.loro().oplog_frontiers(),
        });
        step
    }

    /// Remembers that the newest commit of this peer carries `header` (after a change that was
    /// imported instead of committed).
    pub(crate) fn note_last_commit(&self, header: &StepHeader) {
        let peer = self.loro().peer_id();
        let end = self.loro().oplog_vv().get(&peer).copied().unwrap_or(0);
        *self.commit_state.last.borrow_mut() = Some(LastCommit {
            step: StepRef::new(peer, end.saturating_sub(1)),
            header: header.format(),
            version: self.loro().oplog_frontiers(),
        });
    }

    /// Commits the root setup of a new document as its own change, so that the first user step
    /// does not carry it and an undo of that step leaves the size and version in place.
    pub(crate) fn commit_new_document(&self) {
        let header = StepHeader::new(LABEL_NEW_DOCUMENT, 0, self.commit_state.time.get());
        self.loro()
            .commit_with(CommitOptions::new().commit_msg(&header.format()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(document: &Document, x: f64) -> crate::NodeId {
        use crate::{Point, RectBounds};
        document.create_rect(RectBounds::from_corners(
            Point::new(x, 0.0),
            Point::new(x + 10.0, 10.0),
        ))
    }

    #[test]
    fn a_new_document_has_its_setup_as_its_own_change_and_no_step() {
        let document = Document::new(5);
        assert_eq!(document.loro().len_changes(), 1);
        assert_eq!(document.own_step_count(), 0);
        let change = document.loro().get_change(loro::ID::new(5, 0)).unwrap();
        assert_eq!(change.message.as_deref(), Some("new_document;s=0;t=0"));
        assert_eq!(change.len, 3);
    }

    #[test]
    fn an_edit_is_one_step_with_a_header() {
        let document = Document::new(5);
        document.set_step_time(Timestamp::from_secs(1_700_000_000));
        let _ = rect(&document, 0.0);
        let steps = document.own_steps();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].label, "create_rect");
        assert_eq!(steps[0].step, StepRef::new(5, 3));
        let change = document.loro().get_change(steps[0].step.id()).unwrap();
        assert_eq!(
            change.message.as_deref(),
            Some("create_rect;s=1;t=1700000000")
        );
    }

    #[test]
    fn two_identical_edits_are_two_steps() {
        let document = Document::new(5);
        let id = rect(&document, 0.0);
        for _ in 0..2 {
            document
                .translate_objects(&[id], crate::Vec2::new(1.0, 0.0))
                .unwrap();
        }
        assert_eq!(document.own_step_count(), 3);
        let moves: Vec<_> = document.own_steps().into_iter().skip(1).collect();
        assert_ne!(moves[0].step, moves[1].step);
    }

    #[test]
    fn a_batch_of_several_edits_is_one_step() {
        let document = Document::new(5);
        document.batch("combo", |d| {
            let _ = rect(d, 0.0);
            let _ = rect(d, 20.0);
        });
        let steps = document.own_steps();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].label, "combo");
        assert_eq!(document.object_ids().len(), 2);
    }

    #[test]
    fn a_batch_that_writes_nothing_adds_no_step() {
        let document = Document::new(5);
        document.batch("nothing", |_| {});
        assert_eq!(document.own_step_count(), 0);
    }

    #[test]
    fn continuations_join_the_step_until_something_else_is_committed() {
        let document = Document::new(5);
        let id = rect(&document, 0.0);
        let offset = crate::Vec2::new(1.0, 0.0);
        document.translate_objects(&[id], offset).unwrap();
        for _ in 0..10 {
            document.continue_step();
            document.translate_objects(&[id], offset).unwrap();
        }
        assert_eq!(document.own_step_count(), 2);
        // A write from elsewhere between two commits refuses the continuation.
        let other = Document::new(9);
        let _ = rect(&other, 50.0);
        let updates = other
            .loro()
            .export(loro::ExportMode::all_updates())
            .unwrap();
        document.loro().import(&updates).unwrap();
        document.continue_step();
        document.translate_objects(&[id], offset).unwrap();
        assert_eq!(document.own_step_count(), 3);
    }
}
