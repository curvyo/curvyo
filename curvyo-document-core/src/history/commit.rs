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
    /// The version vector right after the commit.
    vv: loro::VersionVector,
}

/// A step this session started, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnStep {
    /// The step's identity.
    pub step: StepRef,
    /// The commit label of the step, as written (`translate_objects`).
    pub label: String,
    /// For an object undo or redo: the commit label of the step it names, which the stack entry
    /// is called after ("Undo Resize").
    pub reverts: Option<String>,
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
    /// The document has a keyring (see `Document::mark_shared`).
    shared: Cell<bool>,
    /// The version after this session's last commit, when that commit found no dimension
    /// (`crate::dimension_keep`): a commit right after it adds one only through the writers
    /// that forget this.
    pub(crate) no_dimensions_at: RefCell<Option<loro::Frontiers>>,
}

/// Ends a [`Document::batch`] even when the closure panics.
struct BatchGuard<'a>(&'a CommitState);

impl Drop for BatchGuard<'_> {
    fn drop(&mut self) {
        self.0.batch_depth.set(self.0.batch_depth.get() - 1);
    }
}

impl CommitState {
    pub(crate) fn is_shared(&self) -> bool {
        self.shared.get()
    }

    pub(crate) fn set_shared(&self) {
        self.shared.set(true);
    }

    /// A wipe removed the steps this session started: they are no longer in the log.
    pub(crate) fn forget_steps(&self) {
        self.own.borrow_mut().clear();
        *self.last.borrow_mut() = None;
        self.continue_next.set(false);
        self.warm.set(true);
    }

    pub(super) fn is_warm(&self) -> bool {
        self.warm.get()
    }

    pub(crate) fn set_warm(&self) {
        self.warm.set(true);
    }

    /// An object undo or redo is a step of this session like any other: it goes on the maker's
    /// stack (`specs/0041-object-history` criterion 8).
    pub(crate) fn register_own(&self, step: StepRef, label: &str, reverts: Option<String>) {
        self.own.borrow_mut().push(OwnStep {
            step,
            label: label.to_owned(),
            reverts,
        });
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

    /// Runs `write` with every commit its inner methods would make deferred, and commits
    /// nothing: the caller's next commit takes it all (a new document's setup change).
    pub(crate) fn uncommitted<R>(&self, write: impl FnOnce(&Self) -> R) -> R {
        self.commit_state
            .batch_depth
            .set(self.commit_state.batch_depth.get() + 1);
        let _guard = BatchGuard(&self.commit_state);
        write(self)
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
        if self.loro().get_pending_txn_len() == 0 {
            state.continue_next.set(false);
            return None;
        }
        // The last value of each linked dimension joins the step that changed its geometry
        // (ADR 0027 §4).
        let without_dimensions = self.refresh_dimension_kept();
        let pending = self.loro().get_pending_txn_len();
        // `continue_step` checked that nothing else was committed or merged since the last step.
        let joined = state
            .continue_next
            .replace(false)
            .then(|| state.last.borrow().clone())
            .flatten()
            .filter(|last| self.only_own_since(last));
        if let Some(last) = joined {
            let step = self.finish_commit(&last.header, Some(last.step), pending);
            return Some(step);
        }
        let header = self.next_header(label, extra);
        let step = self.finish_commit(&header.format(), None, pending);
        let version = state
            .last
            .borrow()
            .as_ref()
            .map(|last| last.version.clone());
        *state.no_dimensions_at.borrow_mut() = version.filter(|_| without_dimensions);
        state.own.borrow_mut().push(OwnStep {
            step,
            label: label.to_owned(),
            reverts: None,
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

    /// Whether this session's last commit found no dimension and nothing was committed or
    /// merged since: only the pending writes are new (`crate::dimension_keep`).
    pub(crate) fn dimension_free_since_last_commit(&self) -> bool {
        let state = &self.commit_state;
        let at = state.no_dimensions_at.borrow();
        let last = state.last.borrow();
        match (at.as_ref(), last.as_ref()) {
            (Some(at), Some(last)) => last.version == *at && self.only_own_since(last),
            _ => false,
        }
    }

    /// Whether nothing but this peer's own pending writes happened since `last`: no commit of
    /// another peer was merged in between.
    fn only_own_since(&self, last: &LastCommit) -> bool {
        let peer = self.loro().peer_id();
        let now = self.loro().oplog_vv();
        now.iter()
            .all(|(other, counter)| *other == peer || last.vv.get(other) == Some(counter))
            && last.vv.iter().all(|(other, _)| now.get(other).is_some())
    }

    /// Commits the pending writes of an undo or redo with the given header. Not listed among
    /// [`Document::own_steps`], and never joined: a nudge after Ctrl+Z is a new step.
    pub(crate) fn commit_with_header(&self, header: &StepHeader) {
        if self.loro().get_pending_txn_len() > 0 {
            self.loro()
                .commit_with(CommitOptions::new().commit_msg(&header.format()));
        }
        *self.commit_state.last.borrow_mut() = None;
    }

    /// History reads and restores commit pending writes without a header; inside a
    /// [`Document::batch`] that would split the gesture into an unlabelled step.
    pub(crate) fn assert_outside_batch(&self) {
        debug_assert_eq!(
            self.commit_state.batch_depth.get(),
            0,
            "a history call inside a batch"
        );
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
            vv: self.loro().oplog_vv(),
        });
        step
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
        assert_eq!(change.len, 7); // version, size and four initial page registers.
    }

    #[test]
    fn an_edit_is_one_step_with_a_header() {
        let document = Document::new(5);
        document.set_step_time(Timestamp::from_secs(1_700_000_000));
        let _ = rect(&document, 0.0);
        let steps = document.own_steps();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].label, "create_rect");
        assert_eq!(steps[0].step, StepRef::new(5, 7));
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

    #[test]
    fn a_nudge_after_an_undo_is_a_new_step_and_never_joins_the_undo() {
        let document = Document::new(5);
        let id = rect(&document, 0.0);
        let offset = crate::Vec2::new(1.0, 0.0);
        document.translate_objects(&[id], offset).unwrap();
        let moved = document.own_steps()[1].step;
        document.undo_step(moved).unwrap();
        document.continue_step();
        document.translate_objects(&[id], offset).unwrap();
        assert_eq!(document.own_step_count(), 3, "a new step, not a join");
        assert_ne!(document.own_steps()[2].step, moved);
    }
}
