//! What an undo or a redo did, per object, for the notice the maker reads.

use crate::path_model::NodeId;

/// Which way a restore went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Take the step back.
    Undo,
    /// Apply the step again.
    Redo,
}

/// What the step did to an object, from the step's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The step created the object.
    Created,
    /// The step deleted the object.
    Deleted,
    /// The step changed the object's values.
    Changed,
    /// The step moved the object in the stacking order or to another parent (grouping).
    Moved,
}

/// Why part of a step was left as it is (`specs/0020-undo-redo` criteria 16 to 18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// Someone else changed it after the step.
    ChangedLater,
    /// Someone else deleted it after the step.
    DeletedLater,
}

/// How much of one object the restore brought back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectResult {
    /// Everything the step did to the object was taken back (or applied again).
    Restored,
    /// Some fields were taken back, others kept later changes by someone else.
    Partial,
    /// Nothing of the object was changed.
    Skipped(SkipReason),
}

/// What kind of object a touched object is, for the summary "3 rectangles".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ObjectShape {
    /// A path.
    Path,
    /// A rectangle.
    Rectangle,
    /// An ellipse or circle.
    Ellipse,
    /// A polygon.
    Polygon,
    /// A star.
    Star,
    /// A group (`specs/0023-groups`).
    Group,
}

/// The outcome for one object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectReport {
    /// The object.
    pub id: NodeId,
    /// What the object is.
    pub shape: ObjectShape,
    /// What the step did to it.
    pub role: Role,
    /// What the restore did to it.
    pub result: ObjectResult,
    /// How many fields kept a later change by someone else.
    pub kept: usize,
}

/// What an undo or redo did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreReport {
    /// The direction.
    pub direction: Direction,
    /// One entry per object the step touched.
    pub objects: Vec<ObjectReport>,
    /// The result for the document's root registers (size, unit), when the step wrote them.
    pub root: Option<ObjectResult>,
    /// Whether anything was written. A step whose every part was skipped writes nothing.
    pub wrote: bool,
    /// The step an object undo or redo wrote; `None` for Ctrl+Z, Ctrl+Shift+Z and when nothing
    /// was written.
    pub step: Option<super::step_id::StepRef>,
}

impl RestoreReport {
    /// How many parts the step had: its objects, plus the root registers.
    #[must_use]
    pub fn total(&self) -> usize {
        self.objects.len() + usize::from(self.root.is_some())
    }

    /// How many parts kept later changes by someone else, in whole or in part.
    #[must_use]
    pub fn skipped(&self) -> usize {
        let kept = |result: &ObjectResult| !matches!(result, ObjectResult::Restored);
        self.objects.iter().filter(|o| kept(&o.result)).count()
            + usize::from(self.root.as_ref().is_some_and(kept))
    }

    /// Whether the step created an object that stays because someone else changed it
    /// (criterion 18).
    #[must_use]
    pub fn created_object_kept(&self) -> bool {
        self.objects.iter().any(|o| {
            o.role == Role::Created && o.result == ObjectResult::Skipped(SkipReason::ChangedLater)
        })
    }
}
