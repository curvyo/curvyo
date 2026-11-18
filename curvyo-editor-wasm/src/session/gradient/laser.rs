//! The grey view of laser-page gradients (`specs/0124-gradient-fill` criterion 10, ADR 0067 §3):
//! which leaves a laser page shows, read once per document version so a frame adds no work.

use std::collections::HashSet;
use std::sync::Arc;

use curvyo_document_core::{NodeId, ObjectSnapshot};
use curvyo_render_core::DrawList;
use curvyo_ui_core::{Home, object_bounds};

use super::super::Session;

impl Session {
    /// The ids among `objects` whose home is a laser page; empty without an output page or a
    /// gradient, which costs nothing.
    pub(in crate::session) fn laser_gradient_ids(
        &self,
        objects: &[ObjectSnapshot],
    ) -> Arc<HashSet<NodeId>> {
        let paints = |o: &ObjectSnapshot| match o {
            ObjectSnapshot::Clone(clone) if clone.is_picture() => true,
            _ => o.style().fill.painted_gradient().is_some(),
        };
        if !objects.iter().any(paints) || self.document.manufacturings().is_empty() {
            return Arc::default();
        }
        let version = self.document.version();
        if let Some((held, ids)) = self.gradient.laser.borrow().as_ref()
            && *held == version
        {
            return Arc::clone(ids);
        }
        let (pages, membership) = self.context_read();
        let ids: Arc<HashSet<NodeId>> = Arc::new(
            self.document_read()
                .leaves
                .iter()
                .filter(|leaf| {
                    matches!(
                        pages.home_of(&membership, leaf.id(), object_bounds(leaf)),
                        Home::Laser(_)
                    )
                })
                .map(ObjectSnapshot::id)
                .collect(),
        );
        *self.gradient.laser.borrow_mut() = Some((version, Arc::clone(&ids)));
        ids
    }

    /// The artwork of `objects` with the document's guides marked and the gradients of the
    /// leaves on a laser page drawn as grey: every canvas draw of document objects goes through
    /// it (the History previews and the sketch views included, criterion 10).
    pub(in crate::session) fn shown_artwork(&self, objects: &[ObjectSnapshot]) -> DrawList {
        let laser = self.laser_gradient_ids(objects);
        curvyo_render_core::build_artwork_greyed(
            objects,
            &self.document_read().guides,
            &|id| laser.contains(&id),
            self.view(),
        )
    }

    /// The History ghost of `objects` (`build_history_preview`), laser-page gradients as grey.
    pub(in crate::session) fn shown_history_preview(&self, objects: &[ObjectSnapshot]) -> DrawList {
        let laser = self.laser_gradient_ids(objects);
        curvyo_render_core::build_history_preview_greyed(
            objects,
            &|id| laser.contains(&id),
            self.view(),
        )
    }
}
