//! Writes an object's state at a past version as a new step: go to version into the object itself,
//! clone version into a new object (`specs/0042-history-branches` criteria 10, 11, 15, 22 to 25
//! and 31). The version is read under the checkout guard (ADR 0014 §12) and written through the
//! restore write path (`merge.rs`), with no three-way check: a go to is a deliberate return, so
//! every field takes the version's value.

use std::collections::BTreeMap;

use super::checkout::Detached;
use super::error::HistoryError;
use super::lineage::{KEY_CLONE_OF, KEY_HISTORY_FLOOR, encode_clone_of, encode_node};
use super::merge::{Tally, restore_map};
use super::node_value::{Node, read_map};
use super::object_rows::{node_id, tree_id};
use super::object_timeline::EntryClass;
use super::stack_machine::Status;
use super::step_header::{KEY_OBJECT, KEY_VERSION, LABEL_CLONE_VERSION, LABEL_GO_TO_VERSION};
use super::step_id::StepRef;
use super::step_log::find_step;
use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::KEY_GROUP;
use crate::objects::renumber_anchors;
use crate::path_model::{AnchorId, NodeId};
use crate::shape_codec::read_shape_tag;
use crate::subpath_codec::total_anchor_count;

/// What a go to version did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoToReport {
    /// The step written or continued; `None` when the object already had that version.
    pub step: Option<StepRef>,
    /// Whether the press continued the exploration step instead of starting one.
    pub continued: bool,
    /// How many later steps of other authors on the object it went back over (criterion 15).
    pub others_reverted: usize,
}

/// An object's meta map without the keys that belong to the object and not to a version: its
/// lineage and its floor stay as they are.
fn own_fields(mut meta: BTreeMap<String, Node>) -> BTreeMap<String, Node> {
    meta.remove(KEY_CLONE_OF);
    meta.remove(KEY_HISTORY_FLOOR);
    meta
}

impl Document {
    /// Sets the fields of `id` to what they were right after `version`, read from `source` (the
    /// object an inherited row belongs to) or from `id` itself, as one step "Go to version"
    /// (criterion 10). With `explore`, a press that follows this session's go to of the same object
    /// with nothing committed in between continues that step (criterion 11).
    ///
    /// # Errors
    /// [`HistoryError::NoSuchStep`] if `version` is not in the log, [`HistoryError::NoSuchObject`]
    /// if `id` does not exist or the version holds no such object, [`HistoryError::Unreadable`] if
    /// the version cannot be checked out.
    pub fn go_to_version(
        &self,
        id: NodeId,
        version: StepRef,
        source: Option<NodeId>,
        explore: bool,
    ) -> Result<GoToReport, HistoryError> {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        let log = find_step(doc, version).ok_or(HistoryError::NoSuchStep)?;
        let subject = tree_id(id);
        let tree = doc.get_tree(OBJECTS_TREE);
        let meta = tree
            .get_meta(subject)
            .map_err(|_| HistoryError::NoSuchObject)?;
        if !matches!(tree.is_node_deleted(&subject), Ok(false)) {
            return Err(HistoryError::NoSuchObject);
        }
        let at = {
            let guard = Detached::at(doc, &log.after())?;
            guard.node(tree_id(source.unwrap_or(id)))
        };
        if at.meta.is_empty() {
            return Err(HistoryError::NoSuchObject);
        }
        let (now, to) = (own_fields(read_map(&meta)), own_fields(at.meta));
        if now == to {
            return Ok(GoToReport {
                step: None,
                continued: false,
                others_reverted: 0,
            });
        }
        let others_reverted = self.later_steps_of_others(id, version);
        let joinable = explore
            .then(|| self.last_step_if_current())
            .flatten()
            .filter(|(_, header)| {
                header.label == LABEL_GO_TO_VERSION
                    && header.extra_value(KEY_OBJECT) == Some(encode_node(subject).as_str())
            });
        let mut tally = Tally::default();
        restore_map(&meta, &now, &to, &mut tally);
        let (step, continued) = if let Some((step, mut header)) = joinable {
            header.set(KEY_VERSION, version.encode());
            (self.commit_joining(step, &header), true)
        } else {
            let extra = [
                (KEY_OBJECT, encode_node(subject)),
                (KEY_VERSION, version.encode()),
            ];
            (self.commit_step(LABEL_GO_TO_VERSION, &extra), false)
        };
        Ok(GoToReport {
            step,
            continued,
            others_reverted,
        })
    }

    /// Creates a new object with the state `source` had right after `version` (or right before
    /// it, for a step that deleted it), on top of the stack, as one step "Clone version"
    /// (criteria 22 and 24). A path's anchors get ids from `mint`; the clone's lineage names
    /// `source` at that version (criterion 23). Returns the clone.
    ///
    /// # Errors
    /// [`HistoryError::NoSuchStep`] if `version` is not in the log, [`HistoryError::NoSuchObject`]
    /// if the object exists at neither version, [`HistoryError::Group`] for a group (criterion
    /// 25), [`HistoryError::Unreadable`] if the version cannot be checked out.
    pub fn clone_version(
        &self,
        source: NodeId,
        version: StepRef,
        mint: &mut dyn FnMut() -> AnchorId,
    ) -> Result<NodeId, HistoryError> {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        let log = find_step(doc, version).ok_or(HistoryError::NoSuchStep)?;
        let src = tree_id(source);
        let (at, read_at) = {
            let guard = Detached::at(doc, &log.after())?;
            let after = guard.node(src);
            if after.meta.is_empty() {
                // The step deleted it: the clone is the object just before (criterion 24).
                guard.goto(&log.before)?;
                (guard.node(src), log.before.clone())
            } else {
                (after, log.after())
            }
        };
        if at.meta.is_empty() {
            return Err(HistoryError::NoSuchObject);
        }
        if at.meta.get(KEY_GROUP) == Some(&Node::Value(loro::LoroValue::Bool(true))) {
            return Err(HistoryError::Group);
        }
        let tree = doc.get_tree(OBJECTS_TREE);
        let created = tree
            .create(loro::TreeParentId::Root)
            .map_err(|_| HistoryError::Unreadable)?;
        let meta = tree
            .get_meta(created)
            .map_err(|_| HistoryError::Unreadable)?;
        let mut tally = Tally::default();
        restore_map(&meta, &BTreeMap::new(), &own_fields(at.meta), &mut tally);
        if read_shape_tag(&meta).is_none() {
            let ids: Vec<AnchorId> = (0..total_anchor_count(&meta)).map(|_| mint()).collect();
            renumber_anchors(&meta, &ids);
        }
        meta.insert(KEY_CLONE_OF, encode_clone_of(src, &read_at))
            .map_err(|_| HistoryError::Unreadable)?;
        let extra = [
            (KEY_OBJECT, encode_node(src)),
            (KEY_VERSION, version.encode()),
        ];
        self.commit_step(LABEL_CLONE_VERSION, &extra);
        Ok(node_id(created))
    }

    /// The steps of other authors on `id`, in effect, that came after `version`.
    fn later_steps_of_others(&self, id: NodeId, version: StepRef) -> usize {
        self.with_reader(|reader, me| {
            let Some(from) = reader.steps.position(version) else {
                return 0;
            };
            let timeline = reader.timeline(tree_id(id));
            timeline
                .entries
                .iter()
                .filter(|entry| entry.class == EntryClass::Do && entry.step.peer() != me)
                .filter(|entry| reader.steps.status(entry.step) == Status::Applied)
                .filter(|entry| reader.steps.position(entry.step).is_some_and(|p| p > from))
                .count()
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Document, Length, NodeId, Point, RectBounds, StepRef, StyleEdit};

    fn rect(d: &Document) -> NodeId {
        d.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        })
    }

    fn width(d: &Document, id: NodeId, mm: f64) -> StepRef {
        d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(mm)))
            .unwrap();
        d.own_steps().last().unwrap().step
    }

    fn stroke(d: &Document, id: NodeId) -> f64 {
        match d.object(id).unwrap() {
            crate::ObjectSnapshot::Primitive(p) => p.style.stroke.width.as_mm(),
            crate::ObjectSnapshot::Path(p) => p.style.stroke.width.as_mm(),
        }
    }

    #[test]
    fn going_to_a_version_writes_one_step_and_an_equal_object_writes_none() {
        let d = Document::new(1);
        let r = rect(&d);
        let s1 = width(&d, r, 1.0);
        width(&d, r, 3.0);
        let steps = d.own_step_count();
        let report = d.go_to_version(r, s1, None, false).unwrap();
        assert!(report.step.is_some());
        assert!(!report.continued);
        assert!((stroke(&d, r) - 1.0).abs() < 1e-9);
        assert_eq!(d.own_step_count(), steps + 1);
        let again = d.go_to_version(r, s1, None, false).unwrap();
        assert_eq!(again.step, None, "nothing written");
        assert_eq!(d.own_step_count(), steps + 1);
    }

    #[test]
    fn further_presses_of_an_exploration_are_one_step_that_undo_takes_back_whole() {
        let d = Document::new(1);
        let r = rect(&d);
        let s = [1.0, 2.0, 3.0, 4.0].map(|mm| width(&d, r, mm));
        let before = d.export_json().unwrap();
        let rows = d.history_len();
        let first = d.go_to_version(r, s[0], None, true).unwrap();
        for version in [s[1], s[2], s[0], s[1]] {
            let report = d.go_to_version(r, version, None, true).unwrap();
            assert!(report.continued);
            assert_eq!(report.step, first.step);
        }
        assert!((stroke(&d, r) - 2.0).abs() < 1e-9);
        assert_eq!(d.history_len(), rows + 1, "one row");
        let row = d.history_page(0, 1).pop().unwrap();
        assert_eq!(row.label, "go_to_version");
        assert_eq!(
            row.version.map(|v| v.step),
            Some(s[1]),
            "names the latest version"
        );
        d.undo_step(first.step.unwrap()).unwrap();
        assert_eq!(
            d.export_json().unwrap(),
            before,
            "back before the first press"
        );
    }

    #[test]
    fn without_explore_or_after_another_edit_a_press_starts_a_new_step() {
        let d = Document::new(1);
        let r = rect(&d);
        let s1 = width(&d, r, 1.0);
        let s2 = width(&d, r, 2.0);
        let first = d.go_to_version(r, s1, None, true).unwrap();
        let second = d.go_to_version(r, s2, None, false).unwrap();
        assert_ne!(first.step, second.step);
        d.translate_objects(&[r], crate::Vec2::new(1.0, 0.0))
            .unwrap();
        let third = d.go_to_version(r, s1, None, true).unwrap();
        assert!(!third.continued);
    }
}
