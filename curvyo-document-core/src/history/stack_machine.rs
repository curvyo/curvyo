//! The undo and redo stacks and the status of every step, from one pure transition function
//! (ADR 0014 §6; `specs/0020-undo-redo` criterion 55).
//!
//! The session feeds [`StackMachine::apply`] its own actions, with a limit of
//! [`STACK_LIMIT`] entries. The history list feeds it the events of a peer's commits read from the
//! log, without a limit. Both give the same stacks and the same statuses for the same events.
//!
//! A new step that arrives while the redo side holds steps leaves them as a branch whose fork point
//! is the top of the undo side at that moment (`specs/0042-history-branches` criterion 28).

use std::collections::HashMap;

use super::step_id::StepRef;

/// The most steps a stack holds (`specs/0020-undo-redo` criterion 28).
pub const STACK_LIMIT: usize = 500;

/// What one step is, as the history list says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// In effect.
    Applied,
    /// Taken back with undo, and redo can bring it back.
    Undone,
    /// Taken back and then replaced by a new step: no longer on the redo stack.
    OnBranch,
}

/// The branch a step was left on (`specs/0042-history-branches` criteria 1 and 28).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BranchMark {
    /// The newest step of the run that was left together: the branch's identity in this machine.
    pub key: StepRef,
    /// The top of the undo side when the run was left; `None` when the undo side was empty.
    pub fork: Option<StepRef>,
}

/// One thing that happened to a peer's stacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The peer made a new step.
    Do(StepRef),
    /// The peer took the step back.
    Undo(StepRef),
    /// The peer applied the step again.
    Redo(StepRef),
    /// A press on the live undo stack that skipped the whole step: the entry is consumed and goes
    /// nowhere. Writes nothing to the log, so the log replay never sees it.
    ConsumeUndo,
    /// The same for the redo stack.
    ConsumeRedo,
}

/// The stacks of one peer.
#[derive(Debug, Clone)]
pub struct StackMachine {
    limit: usize,
    undo: Vec<StepRef>,
    redo: Vec<StepRef>,
    statuses: HashMap<StepRef, Status>,
    branches: HashMap<StepRef, BranchMark>,
}

impl StackMachine {
    /// A machine whose stacks hold at most `limit` entries.
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            undo: Vec::new(),
            redo: Vec::new(),
            statuses: HashMap::new(),
            branches: HashMap::new(),
        }
    }

    /// The transition function: the stacks and statuses after `event`.
    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Do(step) => {
                // The redo side holds the run newest first at its bottom: the first entry is the
                // newest step of the run.
                if let Some(&key) = self.redo.first() {
                    let mark = BranchMark {
                        key,
                        fork: self.undo.last().copied(),
                    };
                    for dropped in self.redo.drain(..) {
                        self.statuses.insert(dropped, Status::OnBranch);
                        self.branches.insert(dropped, mark);
                    }
                }
                self.push_undo(step);
            }
            Event::Undo(step) => {
                self.undo.retain(|entry| *entry != step);
                self.redo.push(step);
                self.statuses.insert(step, Status::Undone);
            }
            Event::Redo(step) => {
                self.redo.retain(|entry| *entry != step);
                self.statuses.remove(&step);
                self.branches.remove(&step);
                self.push_undo(step);
            }
            Event::ConsumeUndo => {
                self.undo.pop();
            }
            Event::ConsumeRedo => {
                self.redo.pop();
            }
        }
    }

    /// Drops the steps `keep` refuses from both stacks. The wipe of one object's history uses it:
    /// a step that touched only that object can no longer be taken back with Ctrl+Z.
    pub fn retain(&mut self, mut keep: impl FnMut(StepRef) -> bool) {
        self.undo.retain(|step| keep(*step));
        self.redo.retain(|step| keep(*step));
    }

    fn push_undo(&mut self, step: StepRef) {
        self.undo.push(step);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
    }

    /// The step an undo press takes back.
    #[must_use]
    pub fn top_undo(&self) -> Option<StepRef> {
        self.undo.last().copied()
    }

    /// The step a redo press applies again.
    #[must_use]
    pub fn top_redo(&self) -> Option<StepRef> {
        self.redo.last().copied()
    }

    /// The undo stack, oldest first.
    #[must_use]
    pub fn undo_stack(&self) -> &[StepRef] {
        &self.undo
    }

    /// The redo stack, the next redo last.
    #[must_use]
    pub fn redo_stack(&self) -> &[StepRef] {
        &self.redo
    }

    /// The status of a step.
    #[must_use]
    pub fn status(&self, step: StepRef) -> Status {
        self.statuses.get(&step).copied().unwrap_or(Status::Applied)
    }

    /// The branch a step was left on, while it is on one.
    #[must_use]
    pub fn branch_of(&self, step: StepRef) -> Option<BranchMark> {
        self.branches.get(&step).copied()
    }

    /// Every step that is not applied, with its status.
    pub fn non_applied(&self) -> impl Iterator<Item = (StepRef, Status)> + '_ {
        self.statuses.iter().map(|(step, status)| (*step, *status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(n: i32) -> StepRef {
        StepRef::new(1, n)
    }

    #[test]
    fn do_undo_redo_move_steps_between_the_stacks() {
        let mut machine = StackMachine::new(STACK_LIMIT);
        machine.apply(Event::Do(step(1)));
        machine.apply(Event::Do(step(2)));
        machine.apply(Event::Undo(step(2)));
        assert_eq!(machine.undo_stack(), [step(1)]);
        assert_eq!(machine.redo_stack(), [step(2)]);
        assert_eq!(machine.status(step(2)), Status::Undone);
        machine.apply(Event::Redo(step(2)));
        assert_eq!(machine.undo_stack(), [step(1), step(2)]);
        assert_eq!(machine.redo_stack().len(), 0);
        assert_eq!(machine.status(step(2)), Status::Applied);
    }

    #[test]
    fn a_new_step_empties_the_redo_stack_and_marks_the_dropped_steps_as_on_a_branch() {
        let mut machine = StackMachine::new(STACK_LIMIT);
        machine.apply(Event::Do(step(1)));
        machine.apply(Event::Do(step(2)));
        machine.apply(Event::Undo(step(2)));
        machine.apply(Event::Undo(step(1)));
        machine.apply(Event::Do(step(3)));
        assert_eq!(machine.redo_stack().len(), 0);
        assert_eq!(machine.undo_stack(), [step(3)]);
        assert_eq!(machine.status(step(1)), Status::OnBranch);
        assert_eq!(machine.status(step(2)), Status::OnBranch);
        assert_eq!(machine.status(step(3)), Status::Applied);
    }

    #[test]
    fn the_steps_a_new_step_leaves_are_one_branch_from_the_top_of_the_undo_side() {
        let mut machine = StackMachine::new(STACK_LIMIT);
        for n in 1..=3 {
            machine.apply(Event::Do(step(n)));
        }
        machine.apply(Event::Undo(step(3)));
        machine.apply(Event::Undo(step(2)));
        machine.apply(Event::Do(step(4)));
        let mark = BranchMark {
            key: step(3),
            fork: Some(step(1)),
        };
        assert_eq!(machine.branch_of(step(2)), Some(mark));
        assert_eq!(machine.branch_of(step(3)), Some(mark));
        assert_eq!(machine.branch_of(step(1)), None);
        assert_eq!(machine.branch_of(step(4)), None);
        assert_eq!(machine.top_redo(), None, "nothing to redo after the fork");
    }

    #[test]
    fn redo_at_a_fork_brings_back_the_branch_left_last() {
        let mut machine = StackMachine::new(STACK_LIMIT);
        for n in 1..=3 {
            machine.apply(Event::Do(step(n)));
        }
        machine.apply(Event::Undo(step(3)));
        machine.apply(Event::Undo(step(2)));
        machine.apply(Event::Do(step(4)));
        machine.apply(Event::Undo(step(4)));
        assert_eq!(machine.top_undo(), Some(step(1)));
        assert_eq!(machine.top_redo(), Some(step(4)));
        assert_eq!(machine.status(step(4)), Status::Undone);
        assert_eq!(machine.status(step(2)), Status::OnBranch);
    }

    #[test]
    fn a_fork_with_an_empty_undo_side_has_no_fork_point() {
        let mut machine = StackMachine::new(STACK_LIMIT);
        machine.apply(Event::Do(step(1)));
        machine.apply(Event::Undo(step(1)));
        machine.apply(Event::Do(step(2)));
        assert_eq!(
            machine.branch_of(step(1)),
            Some(BranchMark {
                key: step(1),
                fork: None
            })
        );
    }

    #[test]
    fn a_step_on_a_branch_can_fork_again_when_its_own_branch_is_left() {
        // s1 s2 s3, take back s3, s4: s3 forks from s2. Take back s4 and s2, s5: s2 and s4 are a
        // branch from s1, and s3 keeps its fork point s2.
        let mut machine = StackMachine::new(STACK_LIMIT);
        for n in 1..=3 {
            machine.apply(Event::Do(step(n)));
        }
        machine.apply(Event::Undo(step(3)));
        machine.apply(Event::Do(step(4)));
        machine.apply(Event::Undo(step(4)));
        machine.apply(Event::Undo(step(2)));
        machine.apply(Event::Do(step(5)));
        assert_eq!(machine.branch_of(step(3)).unwrap().fork, Some(step(2)));
        let outer = machine.branch_of(step(2)).unwrap();
        assert_eq!(outer.fork, Some(step(1)));
        assert_eq!(outer.key, step(4));
        assert_eq!(machine.branch_of(step(4)), Some(outer));
    }

    #[test]
    fn the_limit_drops_the_oldest_entry_from_the_stack() {
        let mut machine = StackMachine::new(STACK_LIMIT);
        for n in 0..=500 {
            machine.apply(Event::Do(step(n)));
        }
        assert_eq!(machine.undo_stack().len(), 500);
        assert_eq!(machine.undo_stack()[0], step(1));
    }

    #[test]
    fn retain_drops_the_refused_steps_from_both_stacks() {
        let mut machine = StackMachine::new(STACK_LIMIT);
        for n in 1..=4 {
            machine.apply(Event::Do(step(n)));
        }
        machine.apply(Event::Undo(step(4)));
        machine.retain(|s| s != step(2) && s != step(4));
        assert_eq!(machine.undo_stack(), [step(1), step(3)]);
        assert_eq!(machine.redo_stack().len(), 0);
    }

    #[test]
    fn a_consumed_press_goes_nowhere() {
        let mut machine = StackMachine::new(STACK_LIMIT);
        machine.apply(Event::Do(step(1)));
        machine.apply(Event::Do(step(2)));
        machine.apply(Event::ConsumeUndo);
        assert_eq!(machine.undo_stack(), [step(1)]);
        assert_eq!(machine.redo_stack().len(), 0);
        assert_eq!(machine.status(step(2)), Status::Applied);
    }
}
