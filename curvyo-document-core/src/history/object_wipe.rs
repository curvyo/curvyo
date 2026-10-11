//! Wiping the history of one object (`specs/0041-object-history` criteria 29 to 34, 38): the
//! step hides, it does not delete. The object gets a `history_floor` register, and its timeline,
//! its object undo and the copies made later start there; the operations stay in the file and in
//! the document list (ADR 0014 §10).

use super::error::HistoryError;
use super::lineage::{KEY_HISTORY_FLOOR, encode_node};
use super::object_rows::tree_id;
use super::restore_plan::alive;
use super::step_header::{KEY_OBJECT, LABEL_WIPE_OBJECT};
use super::wipe::encode_frontiers;
use crate::document::{Document, OBJECTS_TREE};
use crate::path_model::NodeId;

impl Document {
    /// Hides everything before now from the history of `id`. Writes one step, "Wipe history",
    /// that is on no stack and cannot be undone; returns how many rows of the object's timeline
    /// it hid. The caller drops the stack entries that touched only this object.
    ///
    /// # Errors
    /// [`HistoryError::NoSuchObject`] for an object that does not exist,
    /// [`HistoryError::NoSteps`] when nothing would be hidden.
    pub fn wipe_object_history(&self, id: NodeId) -> Result<usize, HistoryError> {
        self.assert_outside_batch();
        let object = tree_id(id);
        let doc = self.loro();
        if !alive(doc, object) {
            return Err(HistoryError::NoSuchObject);
        }
        let hidden = self.object_wipe_count(id);
        if hidden == 0 {
            return Err(HistoryError::NoSteps);
        }
        doc.commit();
        let meta = doc
            .get_tree(OBJECTS_TREE)
            .get_meta(object)
            .map_err(|_| HistoryError::NoSuchObject)?;
        // The floor is the version right before the wipe step, which is also where the step
        // starts: the timeline reads it from the step.
        let floor = encode_frontiers(&doc.oplog_frontiers());
        meta.insert(KEY_HISTORY_FLOOR, floor)
            .map_err(|_| HistoryError::Wipe)?;
        let header = self.next_header(LABEL_WIPE_OBJECT, &[(KEY_OBJECT, encode_node(object))]);
        self.commit_with_header(&header);
        Ok(hidden)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::step_log::find_step;
    use crate::{CopySource, Point, RectBounds, Vec2};

    fn rect(document: &Document) -> NodeId {
        document.create_rect(RectBounds::from_corners(
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
        ))
    }

    fn register(document: &Document, id: NodeId, key: &str) -> Option<String> {
        let meta = document
            .loro()
            .get_tree(OBJECTS_TREE)
            .get_meta(tree_id(id))
            .ok()?;
        match meta.get(key)?.get_deep_value() {
            loro::LoroValue::String(text) => Some(text.to_string()),
            _ => None,
        }
    }

    #[test]
    fn the_floor_register_holds_the_version_the_wipe_step_started_from() {
        let document = Document::new(1);
        let a = rect(&document);
        document
            .translate_objects(&[a], Vec2::new(1.0, 0.0))
            .unwrap();
        document.wipe_object_history(a).unwrap();
        let step = document.history_page(0, 1)[0].step;
        let log = find_step(document.loro(), step).unwrap();
        assert_eq!(
            register(&document, a, KEY_HISTORY_FLOOR),
            Some(encode_frontiers(&log.before))
        );
    }

    #[test]
    fn an_unreadable_clone_origin_means_no_origin_and_the_file_stays_readable() {
        let document = Document::new(1);
        let a = rect(&document);
        let clone = document
            .duplicate_objects(
                &[CopySource {
                    id: a,
                    anchor_ids: vec![],
                }],
                Vec2::new(1.0, 1.0),
            )
            .unwrap()[0];
        assert!(register(&document, clone, "clone_of").is_some());
        assert_eq!(document.object_history_len(clone), 2);
        document
            .loro()
            .get_tree(OBJECTS_TREE)
            .get_meta(tree_id(clone))
            .unwrap()
            .insert("clone_of", "garbage")
            .unwrap();
        document.loro().commit();
        // The write above is a step of its own; nothing is inherited.
        let rows = document.object_history_page(clone, 0, 5);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.inherited_from.is_none()));
        assert_eq!(rows[1].class, crate::ObjectRowClass::Step);
    }
}
