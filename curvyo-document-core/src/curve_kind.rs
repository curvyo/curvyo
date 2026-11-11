//! Stored editable curve kinds for single-outline paths.

use crate::{Document, NewAnchor, NodeId, PathEditError};
use loro::{LoroMap, LoroValue};
use serde::{Deserialize, Serialize};

pub(crate) const KEY: &str = "curve_kind";

pub(crate) fn write(meta: &LoroMap, kind: CurveKind) {
    let code = match kind {
        CurveKind::Bezier => return,
        CurveKind::Spiro => "spiro",
        CurveKind::BSpline => "bspline",
    };
    // invariant: inserting a plain value into an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY, code).unwrap();
}

/// The editable interpretation of a path's control points.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CurveKind {
    /// Ordinary lines and cubic Bézier segments.
    #[default]
    Bezier,
    /// Spiro control points.
    Spiro,
    /// Non-rational B-spline control points.
    BSpline,
}

impl CurveKind {
    /// Whether this is the implicit ordinary path representation.
    #[must_use]
    pub const fn is_bezier(&self) -> bool {
        matches!(self, Self::Bezier)
    }
}

/// A refused editable-curve operation.
#[derive(Debug, thiserror::Error)]
pub enum CurveKindError {
    /// The path does not exist.
    #[error(transparent)]
    Path(#[from] PathEditError),
    /// This change needs explicit conversion or is not supported for the path.
    #[error("this curve change needs an explicit conversion")]
    NeedsConversion,
    /// A dimension or text depends on the current controls.
    #[error("dimensions or text depend on this curve")]
    Referenced,
    /// Older participants cannot safely interpret these curves.
    #[error("editable curves are not available in shared documents yet")]
    Shared,
}

pub(crate) fn read(meta: &LoroMap) -> Option<CurveKind> {
    match meta.get(KEY).map(|value| value.get_deep_value()) {
        None => Some(CurveKind::Bezier),
        Some(LoroValue::String(value)) if value.as_str() == "spiro" => Some(CurveKind::Spiro),
        Some(LoroValue::String(value)) if value.as_str() == "bspline" => Some(CurveKind::BSpline),
        _ => None,
    }
}

pub(crate) fn has_extended(meta: &LoroMap) -> bool {
    meta.get(KEY).is_some()
}

impl Document {
    /// Creates an editable curve and its controls in one history step.
    ///
    /// # Errors
    /// Rejects shared documents, ordinary paths and insufficient or invalid controls.
    ///
    /// # Panics
    /// Does not panic in practice: creating a node and writing values into attached maps cannot fail.
    pub fn create_editable_curve(
        &self,
        controls: &[NewAnchor],
        closed: bool,
        kind: CurveKind,
    ) -> Result<NodeId, CurveKindError> {
        if self.extended_edits_blocked() {
            return Err(CurveKindError::Shared);
        }
        let code = match kind {
            CurveKind::Spiro => "spiro",
            CurveKind::BSpline => "bspline",
            CurveKind::Bezier => return Err(CurveKindError::NeedsConversion),
        };
        let mut ids = std::collections::HashSet::new();
        if controls.len() < if closed { 3 } else { 2 }
            || controls.iter().any(|point| {
                !point.point.x.is_finite()
                    || !point.point.y.is_finite()
                    || point.handle_in != crate::Vec2::ZERO
                    || point.handle_out != crate::Vec2::ZERO
                    || !ids.insert(point.id)
            })
        {
            return Err(CurveKindError::NeedsConversion);
        }
        let tree = self.loro().get_tree(crate::document::OBJECTS_TREE);
        let path = self.create_path_uncommitted(
            self.drawing_parent(&tree),
            controls,
            closed,
            &crate::Style::default(),
            crate::Angle::from_radians(0.0),
        );
        // invariant: the path was just created and both attached maps accept these values.
        #[allow(clippy::unwrap_used)]
        {
            let meta = tree.get_meta(crate::paths::tree_id_of(path)).unwrap();
            meta.insert(KEY, code).unwrap();
            let root = self.loro().get_map(crate::document::ROOT_MAP);
            if root.get(crate::width_profile::HISTORY_MARK).is_none() {
                root.insert(crate::width_profile::HISTORY_MARK, true)
                    .unwrap();
            }
        }
        self.commit_with_label("create_path");
        Ok(path)
    }

    /// Replaces an evaluated B-spline with explicit Bézier controls in one history step.
    /// The caller must evaluate the exact supplied snapshot; style, identity and width points remain.
    ///
    /// # Errors
    /// Rejects stale snapshots, shared documents and invalid replacement controls.
    ///
    /// # Panics
    /// Attached Loro containers accept validated values.
    pub fn convert_bspline_to_bezier(
        &self,
        source: &crate::PathSnapshot,
        anchors: &[NewAnchor],
    ) -> Result<(), CurveKindError> {
        self.convert_editable_to_bezier(source, anchors, CurveKind::BSpline)
    }

    /// Converts an evaluated Spiro snapshot to its document-space Bézier geometry in one step.
    ///
    /// # Errors
    /// Rejects stale snapshots, shared documents, dependencies and invalid replacement controls.
    ///
    /// # Panics
    /// Attached Loro containers accept validated values.
    pub fn convert_spiro_to_bezier(
        &self,
        source: &crate::PathSnapshot,
        anchors: &[NewAnchor],
    ) -> Result<(), CurveKindError> {
        self.convert_editable_to_bezier(source, anchors, CurveKind::Spiro)
    }

    fn convert_editable_to_bezier(
        &self,
        source: &crate::PathSnapshot,
        anchors: &[NewAnchor],
        expected: CurveKind,
    ) -> Result<(), CurveKindError> {
        if self.extended_edits_blocked() {
            return Err(CurveKindError::Shared);
        }
        let current = self.path(source.id).ok_or(PathEditError::NoSuchPath)?;
        let mut ids = std::collections::HashSet::new();
        if &current != source
            || current.curve_kind != expected
            || current.is_compound()
            || anchors.len() < 2
            || anchors.iter().any(|a| {
                !a.point.x.is_finite()
                    || !a.point.y.is_finite()
                    || !a.handle_in.x.is_finite()
                    || !a.handle_in.y.is_finite()
                    || !a.handle_out.x.is_finite()
                    || !a.handle_out.y.is_finite()
                    || !ids.insert(a.id)
            })
        {
            return Err(CurveKindError::NeedsConversion);
        }
        let read = self.read_objects();
        let mut dependent_ids = std::collections::HashSet::from([source.id]);
        for object in read.objects.iter() {
            if let crate::ObjectSnapshot::Clone(clone) = object
                && clone.chain.root == source.id
            {
                dependent_ids.insert(clone.id());
            }
        }
        if read.objects.iter().any(|object| match object {
            crate::ObjectSnapshot::Dimension(dimension) => dimension
                .slots
                .nodes()
                .iter()
                .any(|id| dependent_ids.contains(id)),
            crate::ObjectSnapshot::Text(text) => text
                .on_path
                .as_ref()
                .is_some_and(|on| dependent_ids.contains(&on.carrier.node)),
            _ => false,
        }) {
            return Err(CurveKindError::Referenced);
        }
        let (meta, _) = self.path_parts(source.id)?;
        let list = crate::path_codec::insert_anchors_container(&meta);
        for anchor in anchors {
            crate::path_codec::push_anchor(&list, anchor);
        }
        // invariant: deleting a key on the attached path metadata succeeds.
        #[allow(clippy::unwrap_used)]
        meta.delete(KEY).unwrap();
        if expected == CurveKind::Spiro && meta.get(crate::spiro_transform::KEY).is_some() {
            // invariant: deleting a present key on an attached map succeeds.
            #[allow(clippy::unwrap_used)]
            meta.delete(crate::spiro_transform::KEY).unwrap();
        }
        self.commit_with_label("convert_curve_to_bezier");
        Ok(())
    }

    /// Sets the editable kind on an ordinary single-outline path.
    ///
    /// # Errors
    /// Rejects missing paths, shared documents and changes requiring conversion.
    ///
    /// # Panics
    /// Does not panic in practice: inserting values into attached maps cannot fail.
    pub fn set_curve_kind(&self, path: NodeId, kind: CurveKind) -> Result<(), CurveKindError> {
        if self.extended_edits_blocked() {
            return Err(CurveKindError::Shared);
        }
        let (meta, _) = self.path_parts(path)?;
        let current = read(&meta).ok_or(CurveKindError::NeedsConversion)?;
        if current == kind {
            return Ok(());
        }
        if !current.is_bezier()
            || kind.is_bezier()
            || self.path(path).is_none_or(|p| {
                p.is_compound()
                    || p.anchors.len() < if p.closed { 3 } else { 2 }
                    || p.anchors.iter().any(|anchor| {
                        anchor.handle_in != crate::Vec2::ZERO
                            || anchor.handle_out != crate::Vec2::ZERO
                    })
            })
        {
            return Err(CurveKindError::NeedsConversion);
        }
        let code = match kind {
            CurveKind::Spiro => "spiro",
            CurveKind::BSpline => "bspline",
            CurveKind::Bezier => return Err(CurveKindError::NeedsConversion),
        };
        let read = self.read_objects();
        let mut carriers = std::collections::HashSet::from([path]);
        for object in read.objects.iter() {
            if let crate::ObjectSnapshot::Clone(clone) = object
                && clone.chain.root == path
            {
                carriers.insert(clone.id());
            }
        }
        if read.objects.iter().any(|object| matches!(object, crate::ObjectSnapshot::Text(text) if text.on_path.as_ref().is_some_and(|on| carriers.contains(&on.carrier.node)))) {
            return Err(CurveKindError::Referenced);
        }
        let root = self.loro().get_map(crate::document::ROOT_MAP);
        // invariant: writing values to attached maps cannot fail.
        #[allow(clippy::unwrap_used)]
        {
            if root.get(crate::width_profile::HISTORY_MARK).is_none() {
                root.insert(crate::width_profile::HISTORY_MARK, true)
                    .unwrap();
            }
            meta.insert(KEY, code).unwrap();
        }
        self.commit_with_label("set_curve_kind");
        Ok(())
    }
}
