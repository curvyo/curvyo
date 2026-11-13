//! `Session`'s Spiro objects as the Bézier paths they draw, for hit tests, bounds, transforms
//! and Curve to Bézier.

use curvyo_document_core::CurveKind;

use super::spiro_cache::{
    CONVERT_FAILED_NOTICE, CONVERT_REFERENCED_NOTICE, CONVERT_SHARED_NOTICE, SPIRO_UNSOLVED_NOTICE,
};

impl super::Session {
    /// Whether one selected path can be offered for Spiro-to-Bézier conversion.
    #[must_use]
    pub fn can_convert_selected_spiro_to_bezier(&self) -> bool {
        let [id] = self.selection.ids() else {
            return false;
        };
        self.document
            .path(*id)
            .is_some_and(|path| path.curve_kind == CurveKind::Spiro)
            && self.selected_group_leaves().is_empty()
    }

    /// Converts one selected Spiro path into the Bézier path it draws.
    pub fn convert_selected_spiro_to_bezier(&mut self) -> bool {
        self.recording(|this| {
            let [id] = this.selection.ids() else {
                return false;
            };
            let Some(source) = this.document.path(*id) else {
                return false;
            };
            if source.curve_kind != CurveKind::Spiro || !this.selected_group_leaves().is_empty() {
                return false;
            }
            let Some(mut evaluated) =
                this.spiro_cache
                    .borrow_mut()
                    .ready(&this.document, &source, 0.01)
            else {
                this.transform_notice = Some(SPIRO_UNSOLVED_NOTICE);
                return false;
            };
            for anchor in &mut evaluated.anchors {
                anchor.id = this.minter.mint();
            }
            match this
                .document
                .convert_spiro_to_bezier(&source, &evaluated.anchors)
            {
                Ok(()) => true,
                Err(error) => {
                    this.transform_notice = Some(match error {
                        curvyo_document_core::CurveKindError::Referenced => {
                            CONVERT_REFERENCED_NOTICE
                        }
                        curvyo_document_core::CurveKindError::Shared => CONVERT_SHARED_NOTICE,
                        _ => CONVERT_FAILED_NOTICE,
                    });
                    false
                }
            }
        })
    }
}

impl super::Session {
    /// `objects` with each Spiro path and each clone of one replaced by the Bézier path it
    /// draws, for hit tests, bounds and transforms; a Spiro without a curve stays as it is.
    pub(super) fn spiro_selection_objects(
        &self,
        objects: &[curvyo_document_core::ObjectSnapshot],
    ) -> Vec<curvyo_document_core::ObjectSnapshot> {
        use curvyo_document_core::ObjectSnapshot;
        objects
            .iter()
            .map(|object| match object {
                ObjectSnapshot::Path(path) if path.curve_kind == CurveKind::Spiro => {
                    let mut cache = self.spiro_cache.borrow_mut();
                    let result = cache.ready(&self.document, path, 0.01);
                    result.map_or_else(|| object.clone(), ObjectSnapshot::Path)
                }
                ObjectSnapshot::Clone(clone)
                    if clone.chain.root_outline.curve_kind == CurveKind::Spiro =>
                {
                    let matrix = clone.chain.composed;
                    let stretch = matrix
                        .a
                        .hypot(matrix.b)
                        .hypot(matrix.c.hypot(matrix.d))
                        .max(1.0);
                    if let Some(root) = self.spiro_cache.borrow_mut().ready(
                        &self.document,
                        &clone.chain.root_outline,
                        0.01 / stretch,
                    ) {
                        let mut clone = clone.clone();
                        let mut mapped = root.mapped(matrix);
                        mapped.id = clone.resolved.id;
                        mapped.style = clone.resolved.style.clone();
                        mapped.rotation = clone.resolved.rotation;
                        clone.resolved = Box::new(mapped);
                        clone.chain.root_outline = std::sync::Arc::new(root);
                        ObjectSnapshot::Clone(clone)
                    } else {
                        object.clone()
                    }
                }
                _ => object.clone(),
            })
            .collect()
    }
    /// `objects` as a press or hover picks them: as [`Session::spiro_selection_objects`], and a
    /// Spiro without a curve as its control polygon, so it can be clicked and selected. A
    /// transform of it is still refused on release.
    pub(super) fn spiro_pick_objects(
        &self,
        objects: &[curvyo_document_core::ObjectSnapshot],
    ) -> Vec<curvyo_document_core::ObjectSnapshot> {
        use curvyo_document_core::ObjectSnapshot;
        self.spiro_selection_objects(objects)
            .into_iter()
            .map(|object| match object {
                ObjectSnapshot::Path(path) if path.curve_kind == CurveKind::Spiro => self
                    .spiro_cache
                    .borrow_mut()
                    .lookup(&self.document, &path)
                    .map_or(ObjectSnapshot::Path(path), ObjectSnapshot::Path),
                other => other,
            })
            .collect()
    }

    pub(super) fn spiro_placement_objects(
        &self,
        objects: &[curvyo_document_core::ObjectSnapshot],
        tolerance: f64,
    ) -> Vec<curvyo_document_core::ObjectSnapshot> {
        use curvyo_document_core::ObjectSnapshot;
        objects
            .iter()
            .map(|object| match object {
                ObjectSnapshot::Path(path) if path.curve_kind == CurveKind::Spiro => self
                    .spiro_cache
                    .borrow_mut()
                    .lookup_with_tolerance(&self.document, path, tolerance)
                    .map_or_else(|| object.clone(), ObjectSnapshot::Path),
                ObjectSnapshot::Clone(clone)
                    if clone.chain.root_outline.curve_kind == CurveKind::Spiro =>
                {
                    let matrix = clone.chain.composed;
                    let stretch = matrix
                        .a
                        .hypot(matrix.b)
                        .hypot(matrix.c.hypot(matrix.d))
                        .max(1.0);
                    if let Some(root) = self.spiro_cache.borrow_mut().lookup_with_tolerance(
                        &self.document,
                        &clone.chain.root_outline,
                        tolerance / stretch,
                    ) {
                        let mut clone = clone.clone();
                        let mut mapped = root.mapped(matrix);
                        mapped.id = clone.resolved.id;
                        mapped.style = clone.resolved.style.clone();
                        mapped.rotation = clone.resolved.rotation;
                        *clone.resolved = mapped;
                        clone.chain.root_outline = std::sync::Arc::new(root);
                        ObjectSnapshot::Clone(clone)
                    } else {
                        object.clone()
                    }
                }
                _ => object.clone(),
            })
            .collect()
    }

    pub(super) fn selection_contains_spiro(&self) -> bool {
        self.transform_selection_ids().iter().any(|id| {
            self.document
                .object(*id)
                .and_then(|object| object.drawn_path().map(|path| path.curve_kind))
                .is_some_and(|kind| kind == CurveKind::Spiro)
        })
    }
}
