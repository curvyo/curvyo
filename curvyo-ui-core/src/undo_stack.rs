//! The maker's own undo and redo stacks of this session, with the selection each step had before
//! and after it (`specs/0020-undo-redo` criteria 15, 23, 28). The stacks come from the pure
//! stack machine of `curvyo-document-core`; the selection is view state and lives only here, in
//! memory (ADR 0009 §2).

use std::collections::HashMap;

use curvyo_document_core::{AnchorId, Event, NodeId, STACK_LIMIT, StackMachine, StepRef};

/// What was selected at one moment: the objects, and in the Node tool the selected nodes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectionSnapshot {
    /// The selected objects.
    pub objects: Vec<NodeId>,
    /// The Node tool's selected nodes.
    pub nodes: Vec<(NodeId, AnchorId)>,
}

/// A step on a stack: its label and the selection around it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    label: String,
    before: SelectionSnapshot,
    after: SelectionSnapshot,
}

/// The entry a press acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The step.
    pub step: StepRef,
    /// Its commit label.
    pub label: String,
    /// The selection to set when the step is taken back.
    pub before: SelectionSnapshot,
    /// The selection to set when the step is applied again.
    pub after: SelectionSnapshot,
}

/// The stacks of this session.
#[derive(Debug, Clone)]
pub struct UndoStack {
    machine: StackMachine,
    entries: HashMap<StepRef, Entry>,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoStack {
    /// Empty stacks.
    #[must_use]
    pub fn new() -> Self {
        Self {
            machine: StackMachine::new(STACK_LIMIT),
            entries: HashMap::new(),
        }
    }

    /// A new step of this session: it goes on the undo stack and empties the redo stack.
    pub fn record(
        &mut self,
        step: StepRef,
        label: &str,
        before: SelectionSnapshot,
        after: SelectionSnapshot,
    ) {
        self.entries.insert(
            step,
            Entry {
                label: label.to_owned(),
                before,
                after,
            },
        );
        self.machine.apply(Event::Do(step));
        self.prune();
    }

    /// The step an undo press acts on.
    #[must_use]
    pub fn undo_target(&self) -> Option<Target> {
        self.target(self.machine.top_undo()?)
    }

    /// The step a redo press acts on.
    #[must_use]
    pub fn redo_target(&self) -> Option<Target> {
        self.target(self.machine.top_redo()?)
    }

    fn target(&self, step: StepRef) -> Option<Target> {
        let entry = self.entries.get(&step)?;
        Some(Target {
            step,
            label: entry.label.clone(),
            before: entry.before.clone(),
            after: entry.after.clone(),
        })
    }

    /// The press took the step back (it wrote something).
    pub fn undone(&mut self, step: StepRef) {
        self.machine.apply(Event::Undo(step));
    }

    /// The press applied the step again.
    pub fn redone(&mut self, step: StepRef) {
        self.machine.apply(Event::Redo(step));
    }

    /// The press skipped the whole step: the entry is consumed and goes nowhere.
    pub fn consume_undo(&mut self) {
        self.machine.apply(Event::ConsumeUndo);
        self.prune();
    }

    /// As [`UndoStack::consume_undo`], for the redo stack.
    pub fn consume_redo(&mut self) {
        self.machine.apply(Event::ConsumeRedo);
        self.prune();
    }

    /// The number of steps on the undo stack.
    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.machine.undo_stack().len()
    }

    /// The number of steps on the redo stack.
    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.machine.redo_stack().len()
    }

    /// Forgets the selections of steps that left both stacks.
    fn prune(&mut self) {
        let machine = &self.machine;
        self.entries.retain(|step, _| {
            machine.undo_stack().contains(step) || machine.redo_stack().contains(step)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(n: i32) -> StepRef {
        StepRef::new(1, n)
    }

    fn selection(n: i32) -> SelectionSnapshot {
        let document = curvyo_document_core::Document::new(1);
        let id = document.create_rect(curvyo_document_core::RectBounds::from_corners(
            curvyo_document_core::Point::new(f64::from(n), 0.0),
            curvyo_document_core::Point::new(f64::from(n) + 1.0, 1.0),
        ));
        SelectionSnapshot {
            objects: vec![id],
            nodes: Vec::new(),
        }
    }

    #[test]
    fn a_step_goes_on_the_undo_stack_and_comes_back_through_the_redo_stack() {
        let mut stack = UndoStack::new();
        stack.record(
            step(1),
            "create_rect",
            SelectionSnapshot::default(),
            selection(1),
        );
        stack.record(step(2), "translate_objects", selection(1), selection(1));
        let target = stack.undo_target().unwrap();
        assert_eq!(target.step, step(2));
        assert_eq!(target.label, "translate_objects");
        stack.undone(step(2));
        assert_eq!(stack.undo_len(), 1);
        assert_eq!(stack.redo_target().unwrap().step, step(2));
        stack.redone(step(2));
        assert_eq!(stack.redo_len(), 0);
        assert_eq!(stack.undo_len(), 2);
    }

    #[test]
    fn a_new_step_forgets_the_redo_entries_and_their_selections() {
        let mut stack = UndoStack::new();
        stack.record(step(1), "a", SelectionSnapshot::default(), selection(1));
        stack.undone(step(1));
        stack.record(step(2), "b", SelectionSnapshot::default(), selection(2));
        assert!(stack.redo_target().is_none());
        assert_eq!(stack.entries.len(), 1);
    }

    #[test]
    fn five_hundred_and_one_steps_keep_five_hundred() {
        let mut stack = UndoStack::new();
        for n in 0..501 {
            stack.record(
                step(n),
                "a",
                SelectionSnapshot::default(),
                SelectionSnapshot::default(),
            );
        }
        assert_eq!(stack.undo_len(), 500);
        assert_eq!(stack.entries.len(), 500);
        for _ in 0..500 {
            let target = stack.undo_target().unwrap();
            stack.undone(target.step);
        }
        assert!(stack.undo_target().is_none());
    }

    #[test]
    fn a_consumed_press_leaves_no_redo_entry() {
        let mut stack = UndoStack::new();
        stack.record(
            step(1),
            "a",
            SelectionSnapshot::default(),
            SelectionSnapshot::default(),
        );
        stack.consume_undo();
        assert!(stack.undo_target().is_none());
        assert!(stack.redo_target().is_none());
    }
}
