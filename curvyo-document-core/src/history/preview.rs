//! Previews of earlier versions: plain copies of objects as they were right after a step, read
//! from the live document under the checkout guard (`specs/0042-history-branches` criteria 17, 19,
//! 20, 30 and 32; ADR 0014 §8 and §12). Nothing here writes or replicates.

use super::checkout::Detached;
use super::error::HistoryError;
use super::step_id::StepRef;
use super::step_log::find_step;
use crate::document::Document;
use crate::document_background::DocumentBackground;
use crate::path_model::NodeId;
use crate::primitive_model::ObjectSnapshot;
use crate::units::DocumentSize;

/// What a preview shows: the objects and the page as they were right after `step`, as plain data
/// that cannot be edited. The live document keeps committing and merging while a preview holds one.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoricalDocument {
    /// The step whose version this is.
    pub step: StepRef,
    /// The objects at that version, in paint order; empty when none existed.
    pub objects: Vec<ObjectSnapshot>,
    /// The page size at that version.
    pub size: DocumentSize,
    /// The background at that version.
    pub background: DocumentBackground,
}

impl Document {
    /// The object `id` as it was right after `step` (criterion 17). An object that did not exist
    /// at that version gives no object.
    ///
    /// # Errors
    /// [`HistoryError::NoSuchStep`] if the step is not in the log, [`HistoryError::Unreadable`] if
    /// its version cannot be checked out.
    pub fn preview_object_at(
        &self,
        step: StepRef,
        id: NodeId,
    ) -> Result<HistoricalDocument, HistoryError> {
        self.read_at(step, Some(id))
    }

    /// Every object and the page as they were right after `step` (criterion 19).
    ///
    /// # Errors
    /// As [`Document::preview_object_at`].
    pub fn preview_document_at(&self, step: StepRef) -> Result<HistoricalDocument, HistoryError> {
        self.read_at(step, None)
    }

    /// Checks the document out to the version right after `step`, reads plain copies of `object`
    /// (every object for `None`) through the guard, and attaches it again in the same call, also when the read panics (ADR 0014 §12).
    fn read_at(
        &self,
        step: StepRef,
        object: Option<NodeId>,
    ) -> Result<HistoricalDocument, HistoryError> {
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        let log = find_step(doc, step).ok_or(HistoryError::NoSuchStep)?;
        let guard = Detached::at(doc, &log.after())?;
        let objects = match object {
            Some(id) => guard.object(id).into_iter().collect(),
            None => guard.objects(),
        };
        let (size, background) = guard.page();
        drop(guard);
        Ok(HistoricalDocument {
            step,
            objects,
            size,
            background,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Document, Length, Point, RectBounds, StyleEdit};

    fn stroke_mm(d: &Document, id: crate::NodeId) -> f64 {
        match d.object(id).unwrap() {
            crate::ObjectSnapshot::Primitive(p) => p.style.stroke.width.as_mm(),
            crate::ObjectSnapshot::Path(p) => p.style.stroke.width.as_mm(),
            crate::ObjectSnapshot::Clone(c) => c.resolved.style.stroke.width.as_mm(),
        }
    }

    fn width_of(object: &crate::ObjectSnapshot) -> f64 {
        match object {
            crate::ObjectSnapshot::Primitive(p) => p.style.stroke.width.as_mm(),
            crate::ObjectSnapshot::Path(p) => p.style.stroke.width.as_mm(),
            crate::ObjectSnapshot::Clone(c) => c.resolved.style.stroke.width.as_mm(),
        }
    }

    fn rect(d: &Document, x: f64) -> crate::NodeId {
        d.create_rect(RectBounds {
            origin: Point::new(x, 10.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        })
    }

    #[test]
    fn an_object_preview_reads_the_version_and_changes_nothing() {
        let d = Document::new(1);
        let r = rect(&d, 0.0);
        d.edit_style(&[r], &StyleEdit::StrokeWidth(Length::from_mm(1.0)))
            .unwrap();
        let s1 = d.own_steps().last().unwrap().step;
        d.edit_style(&[r], &StyleEdit::StrokeWidth(Length::from_mm(3.0)))
            .unwrap();
        let before = d.export_json().unwrap();
        let version = d.version();
        let rows = d.history_len();
        let preview = d.preview_object_at(s1, r).unwrap();
        assert_eq!(preview.objects.len(), 1);
        assert!((width_of(&preview.objects[0]) - 1.0).abs() < 1e-9);
        assert_eq!(d.export_json().unwrap(), before);
        assert_eq!(d.version(), version, "nothing written");
        assert_eq!(d.history_len(), rows, "no step");
        assert!(
            (stroke_mm(&d, r) - 3.0).abs() < 1e-9,
            "the live object is as it was"
        );
    }

    #[test]
    fn a_document_preview_holds_every_object_that_existed_then() {
        let d = Document::new(1);
        rect(&d, 0.0);
        let first = d.own_steps().last().unwrap().step;
        rect(&d, 20.0);
        assert_eq!(d.preview_document_at(first).unwrap().objects.len(), 1);
        assert_eq!(d.object_ids().len(), 2);
    }

    #[test]
    fn an_object_that_did_not_exist_then_previews_as_nothing() {
        let d = Document::new(1);
        let a = rect(&d, 0.0);
        let first = d.own_steps().last().unwrap().step;
        let b = rect(&d, 20.0);
        assert_eq!(d.preview_object_at(first, b).unwrap().objects, []);
        assert_eq!(d.preview_object_at(first, a).unwrap().objects.len(), 1);
    }

    #[test]
    fn a_linked_clone_previews_as_a_clone_of_its_original_then() {
        let d = Document::new(1);
        let corner = |k: u64, x: f64, y: f64| {
            crate::NewAnchor::corner(crate::AnchorId::new(1, k), Point::new(x, y))
        };
        let original = d.create_path(
            &[
                corner(1, 0.0, 0.0),
                corner(2, 10.0, 0.0),
                corner(3, 10.0, 10.0),
            ],
            true,
        );
        let clone = d.linked_clone(&[original], &[]).unwrap()[0];
        let step = d.own_steps().last().unwrap().step;
        let preview = d.preview_object_at(step, clone).unwrap();
        let [crate::ObjectSnapshot::Clone(c)] = preview.objects.as_slice() else {
            panic!("a clone: {:?}", preview.objects);
        };
        assert_eq!(c.chain.origin, original);
        assert_eq!(c.resolved.id, clone);
    }

    #[test]
    fn an_unknown_step_is_refused_and_the_document_stays_editable() {
        let d = Document::new(1);
        let r = rect(&d, 0.0);
        assert!(d.preview_object_at(crate::StepRef::new(9, 0), r).is_err());
        d.edit_style(&[r], &StyleEdit::StrokeWidth(Length::from_mm(2.0)))
            .unwrap();
        assert!((stroke_mm(&d, r) - 2.0).abs() < 1e-9);
    }
}
