//! The No solution outline of a generated object whose stored parameters cannot be drawn
//! (`specs/0144`, ADR 0091 §3), and the memo of which objects those are.

use std::sync::Arc;

use curvyo_document_core::{NodeId, ObjectSnapshot, ViewTransform};
use curvyo_render_core::DrawList;
use curvyo_ui_core::instance;

use crate::session::Session;

impl Session {
    /// The "No solution" outline of every generated object whose stored parameters cannot be drawn,
    /// and of the one being previewed when its preview cannot (ADR 0091 §3): screen only, never
    /// exported or cut. The set is worked out once per read of the document.
    pub(in crate::session) fn generated_no_solution_overlay(
        &self,
        objects: &[ObjectSnapshot],
        view: ViewTransform,
    ) -> DrawList {
        let read = self.document_read();
        let mut memo = self.generators.unsolved.borrow_mut();
        if !memo
            .as_ref()
            .is_some_and(|(held, _)| Arc::ptr_eq(held, &read))
        {
            let ids = read
                .leaves
                .iter()
                .filter_map(|object| match object {
                    ObjectSnapshot::Path(path) => {
                        self.is_refused_object(path.id).then_some(path.id)
                    }
                    _ => None,
                })
                .collect();
            *memo = Some((Arc::clone(&read), ids));
        }
        let mut marked: Vec<NodeId> = memo
            .as_ref()
            .map(|(_, ids)| ids.clone())
            .unwrap_or_default();
        if let Some(preview) = self.generators.preview.as_ref().filter(|p| !p.overlay) {
            marked.retain(|id| *id != preview.id);
            if preview.refused {
                marked.push(preview.id);
            }
        }
        if marked.is_empty() {
            return DrawList::default();
        }
        curvyo_render_core::build_no_solution_outlines(objects, &marked, view)
    }

    /// Whether the generated object `id` has parameters its generator cannot draw. Only a generator
    /// with a text parameter can refuse, so a box or a gear costs nothing; the answer is kept per
    /// parameter set, and the parameters are read without the outline.
    fn is_refused_object(&self, id: NodeId) -> bool {
        let Some(found) = self.document.generated_params(id) else {
            return false;
        };
        let (kind, seed, params) = (found.kind, found.seed, found.params);
        if !instance::can_refuse(&self.manufacturing.extensions, &kind) {
            return false;
        }
        let mut refusals = self.generators.refusals.borrow_mut();
        if let Some((known, refused)) = refusals.get(&id)
            && *known == params
        {
            return *refused;
        }
        let refused = instance::is_refused(&self.manufacturing.extensions, &kind, &params, seed);
        refusals.insert(id, (params, refused));
        refused
    }
}
