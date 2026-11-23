//! The commands and reads of a marks object (ADR 0081 §2, `specs/0121-print-and-cut` criteria 1
//! to 3): where new marks go, adding them, sizing them, and the rule that keeps two.

use loro::{LoroTree, TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{children_of, is_group_node, node_id};
use crate::objects::ObjectEditError;
use crate::pages::Page;
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::registration_mark_codec::{
    is_marks_group, is_registration_mark_meta, read_mark_snapshot, write_mark, write_marks_tag,
    write_size,
};
use crate::registration_mark_model::{
    ADD_REGISTRATION_MARK_COMMIT_LABEL, INSERT_REGISTRATION_MARKS_COMMIT_LABEL,
    MARK_DEFAULT_DIAMETER_MM, MARK_DEFAULT_LINE_MM, MARK_INSET_MM, MARK_MAX_COUNT, MARK_MIN_COUNT,
    REGISTRATION_MARK_SIZE_COMMIT_LABEL, RegistrationMarkSnapshot, mark_diameter_is_valid,
    mark_line_is_valid,
};
use crate::units::{Length, Point};

/// One marks object: the group and its marks in tree order.
#[derive(Debug, Clone, PartialEq)]
pub struct MarksObject {
    /// The group.
    pub id: NodeId,
    /// Its marks, in tree order.
    pub marks: Vec<RegistrationMarkSnapshot>,
}

impl MarksObject {
    /// The lowest number 1 to 4 no mark carries, `None` with all four.
    #[must_use]
    pub fn lowest_free_number(&self) -> Option<u8> {
        (1..=MARK_MAX_COUNT).find(|n| self.marks.iter().all(|m| m.number != *n))
    }
}

/// Where marks 1 to 4 go on a page (criterion 1, clockwise from the top-left): the page corners
/// inset [`MARK_INSET_MM`] when each lies at least that far outside the artwork box (or there is
/// no artwork); otherwise the corners of the artwork box grown by it, clamped to that inset of the
/// page edges. `page` and `artwork` are `(min, max)` in document mm.
#[must_use]
pub fn registration_mark_corners(
    page: (Point, Point),
    artwork: Option<(Point, Point)>,
) -> [Point; 4] {
    let inset = MARK_INSET_MM;
    let corners = |(min, max): (Point, Point)| {
        [
            Point::new(min.x, min.y),
            Point::new(max.x, min.y),
            Point::new(max.x, max.y),
            Point::new(min.x, max.y),
        ]
    };
    let inner = (
        Point::new(page.0.x + inset, page.0.y + inset),
        Point::new(page.1.x - inset, page.1.y - inset),
    );
    let Some((art_min, art_max)) = artwork else {
        return corners(inner);
    };
    let grown = (
        Point::new(art_min.x - inset, art_min.y - inset),
        Point::new(art_max.x + inset, art_max.y + inset),
    );
    let clear =
        |p: &Point| p.x <= grown.0.x || p.x >= grown.1.x || p.y <= grown.0.y || p.y >= grown.1.y;
    let candidates = corners(inner);
    if candidates.iter().all(clear) {
        return candidates;
    }
    corners(grown).map(|p| {
        Point::new(
            p.x.clamp(inner.0.x.min(inner.1.x), inner.1.x.max(inner.0.x)),
            p.y.clamp(inner.0.y.min(inner.1.y), inner.1.y.max(inner.0.y)),
        )
    })
}

/// The marks under the group `group`, in tree order.
fn marks_under(tree: &LoroTree, group: TreeID) -> Vec<RegistrationMarkSnapshot> {
    children_of(tree, Some(group))
        .into_iter()
        .filter_map(|child| {
            let meta = tree.get_meta(child).ok()?;
            is_registration_mark_meta(&meta)
                .then(|| read_mark_snapshot(node_id(child), &meta))
                .flatten()
        })
        .collect()
}

/// Every marks group in tree order (depth first, bottom first).
fn marks_groups(tree: &LoroTree) -> Vec<TreeID> {
    let mut found = Vec::new();
    let mut stack: Vec<TreeID> = children_of(tree, None).into_iter().rev().collect();
    while let Some(node) = stack.pop() {
        if is_marks_group(tree, node) {
            found.push(node);
        } else if is_group_node(tree, node) {
            stack.extend(children_of(tree, Some(node)).into_iter().rev());
        }
    }
    found
}

/// Whether deleting `doomed` would leave a marks object with one mark (fewer than
/// [`MARK_MIN_COUNT`] but not none: criterion 3, refused; all of its marks or the object itself
/// delete it as a whole).
pub(crate) fn delete_leaves_marks_below_two(tree: &LoroTree, doomed: &[TreeID]) -> bool {
    marks_groups(tree).into_iter().any(|group| {
        if doomed.contains(&group) || crate::group_tree::has_ancestor_in(tree, group, doomed) {
            return false;
        }
        let marks = marks_under(tree, group);
        let gone = marks
            .iter()
            .filter(|m| doomed.contains(&tree_id_of(m.id)))
            .count();
        gone > 0 && (1..MARK_MIN_COUNT).contains(&(marks.len() - gone))
    })
}

impl Document {
    /// Whether deleting `ids` is refused because a marks object would keep one mark
    /// ("Registration marks need at least 2 marks.", criterion 3).
    #[must_use]
    pub fn delete_leaves_one_mark(&self, ids: &[NodeId]) -> bool {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let doomed: Vec<TreeID> = ids.iter().map(|&id| tree_id_of(id)).collect();
        delete_leaves_marks_below_two(&tree, &doomed)
    }

    /// The marks object `id`, `None` when it is not a live marks group.
    #[must_use]
    pub fn marks_object(&self, id: NodeId) -> Option<MarksObject> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let group = tree_id_of(id);
        (node_exists(&tree, group) && is_marks_group(&tree, group)).then(|| MarksObject {
            id,
            marks: marks_under(&tree, group),
        })
    }

    /// The marks object `id` is or belongs to: the group itself, or the group of a mark.
    #[must_use]
    pub fn marks_object_of(&self, id: NodeId) -> Option<MarksObject> {
        if let Some(object) = self.marks_object(id) {
            return Some(object);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let parent = crate::group_tree::group_parent(&tree, tree_id_of(id))?;
        self.marks_object(node_id(parent))
    }

    /// Every marks object whose marks lie on `page` (the first mark's centre inside the frame),
    /// in tree order (ADR 0081 §5: the Print and cut group reads the first).
    #[must_use]
    pub fn marks_objects_on(&self, page: &Page) -> Vec<MarksObject> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        marks_groups(&tree)
            .into_iter()
            .map(|group| MarksObject {
                id: node_id(group),
                marks: marks_under(&tree, group),
            })
            .filter(|object| {
                object
                    .marks
                    .first()
                    .is_some_and(|m| page.contains(m.centre))
            })
            .collect()
    }

    /// Adds a marks object of marks 1 to 4 at `corners` (from [`registration_mark_corners`]) at
    /// the default size, under the draw parent, in one commit labelled
    /// [`INSERT_REGISTRATION_MARKS_COMMIT_LABEL`] (criterion 1). Returns the group.
    ///
    /// # Errors
    /// [`ObjectEditError::NonFiniteGeometry`] for a corner that is not finite; nothing is written.
    ///
    /// # Panics
    /// Does not panic in practice: every write goes to the nodes this call creates.
    pub fn insert_registration_marks(
        &self,
        corners: [Point; 4],
    ) -> Result<NodeId, ObjectEditError> {
        if corners.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
            return Err(ObjectEditError::NonFiniteGeometry);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        // invariant: creating a node under the root or a live group cannot fail.
        #[allow(clippy::unwrap_used)]
        let group = tree.create(self.drawing_parent(&tree)).unwrap();
        // invariant: the meta map of a node just created can be read.
        #[allow(clippy::unwrap_used)]
        let meta = tree.get_meta(group).unwrap();
        // invariant: as above.
        #[allow(clippy::unwrap_used)]
        meta.insert(crate::group_tree::KEY_GROUP, true).unwrap();
        write_marks_tag(&meta);
        // invariant: as above.
        #[allow(clippy::unwrap_used)]
        meta.insert(crate::node_registers::KEY_NAME, "Registration marks")
            .unwrap();
        for (number, centre) in (1..=MARK_MAX_COUNT).zip(corners) {
            write_new_mark(&tree, group, number, centre, default_size());
        }
        self.commit_with_label(INSERT_REGISTRATION_MARKS_COMMIT_LABEL);
        Ok(node_id(group))
    }

    /// Adds mark n, the lowest free number, at `corners[n − 1]` with the size of the object's
    /// first mark, in one commit labelled [`ADD_REGISTRATION_MARK_COMMIT_LABEL`] (criterion 3).
    ///
    /// # Errors
    /// [`ObjectEditError::NotAGroup`] when `group` is no marks object;
    /// [`ObjectEditError::TooDeep`] when it holds all four numbers.
    pub fn add_registration_mark(
        &self,
        group: NodeId,
        corners: [Point; 4],
    ) -> Result<NodeId, ObjectEditError> {
        let object = self.marks_object(group).ok_or(ObjectEditError::NotAGroup)?;
        let number = object
            .lowest_free_number()
            .ok_or(ObjectEditError::TooDeep)?;
        let size = object
            .marks
            .first()
            .map_or_else(default_size, |m| (m.diameter, m.line));
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let centre = corners[usize::from(number - 1)];
        let mark = write_new_mark(&tree, tree_id_of(group), number, centre, size);
        self.commit_with_label(ADD_REGISTRATION_MARK_COMMIT_LABEL);
        Ok(mark)
    }

    /// Sets the diameter and line width of every mark of `group` in one commit labelled
    /// [`REGISTRATION_MARK_SIZE_COMMIT_LABEL`] (criterion 2).
    ///
    /// # Errors
    /// [`ObjectEditError::NotAGroup`] when `group` is no marks object;
    /// [`ObjectEditError::InvalidStrokeWidth`] for a diameter or line out of range.
    pub fn set_registration_mark_size(
        &self,
        group: NodeId,
        diameter: Length,
        line: Length,
    ) -> Result<(), ObjectEditError> {
        let object = self.marks_object(group).ok_or(ObjectEditError::NotAGroup)?;
        if !mark_diameter_is_valid(diameter.as_mm()) || !mark_line_is_valid(line.as_mm()) {
            return Err(ObjectEditError::InvalidStrokeWidth);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for mark in &object.marks {
            if let Ok(meta) = tree.get_meta(tree_id_of(mark.id)) {
                write_size(&meta, diameter, line);
            }
        }
        self.commit_with_label(REGISTRATION_MARK_SIZE_COMMIT_LABEL);
        Ok(())
    }
}

fn default_size() -> (Length, Length) {
    (
        Length::from_mm(MARK_DEFAULT_DIAMETER_MM),
        Length::from_mm(MARK_DEFAULT_LINE_MM),
    )
}

/// Writes one mark under `group` without committing.
fn write_new_mark(
    tree: &LoroTree,
    group: TreeID,
    number: u8,
    centre: Point,
    (diameter, line): (Length, Length),
) -> NodeId {
    // invariant: the group is live (created or confirmed by the caller).
    #[allow(clippy::unwrap_used)]
    let node = tree.create(TreeParentId::Node(group)).unwrap();
    // invariant: the meta map of a node just created can be read.
    #[allow(clippy::unwrap_used)]
    let meta = tree.get_meta(node).unwrap();
    let id = node_id(node);
    write_mark(
        &meta,
        &RegistrationMarkSnapshot::new(id, number, centre, diameter, line),
    );
    id
}

#[cfg(test)]
#[path = "registration_mark_commands_tests.rs"]
mod tests;
