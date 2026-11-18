//! The gradient commands (`specs/0124-gradient-fill` criteria 2, 3, 6, 8).
//!
//! It holds how a gradient `StyleEdit` changes a fill, `Document::set_gradient_geometries`
//! (each shape its own geometry in one commit, for an Angle or Radius typed over shapes whose
//! boxes differ) and `Document::write_carried_gradient`.

use crate::document::Document;
use crate::gradient_codec::write_carried_geometry;
use crate::gradient_model::{GradientFill, GradientGeometry};
use crate::path_model::NodeId;
use crate::primitive_model::ObjectSnapshot;
use crate::style_codec::{read_style, write_changes};
use crate::style_model::Fill;
use crate::styles::{StyleEdit, StyleEditError};

/// The commit label of every gradient edit that is not a stop added or removed, a reverse or a
/// kind switch ("Edit gradient").
pub const EDIT_GRADIENT_LABEL: &str = "edit_gradient";

/// The gradient a fill gets when Gradient is first chosen: from its Colour at its opacity
/// (criterion 2).
pub(crate) fn first_gradient(fill: &Fill) -> GradientFill {
    // invariant: an opacity lies in [0, 1], so the rounded product lies in [0, 255].
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let alpha = (fill.opacity.get() * 255.0).round() as u8;
    GradientFill::first_for(fill.color, alpha)
}

impl StyleEdit {
    /// Applies a gradient edit to `fill`: the Kind switch gives a fill without a gradient its
    /// first one; every other gradient edit leaves such a fill alone, and an index past the list
    /// changes nothing.
    pub(crate) fn apply_to_gradient(&self, fill: &mut Fill) {
        if let Self::GradientKind(kind) = self {
            if fill.gradient.is_none() {
                fill.gradient = Some(first_gradient(fill));
            }
            if let Some(gradient) = fill.gradient.as_mut()
                && gradient.geometry.kind() != *kind
            {
                gradient.geometry = GradientGeometry::default_for(*kind);
            }
            return;
        }
        let Some(gradient) = fill.gradient.as_mut() else {
            return;
        };
        let stops = &gradient.stops;
        let new_stops = match self {
            Self::GradientGeometry(geometry) => {
                gradient.geometry = *geometry;
                None
            }
            Self::GradientStopColour(index, colour, alpha) => {
                stops.with_stop_colour(*index, *colour, *alpha)
            }
            Self::GradientStopOffset(index, offset) => stops.with_stop_offset(*index, *offset),
            Self::GradientStops(list) => Some(list.clone()),
            Self::AddGradientStop(at) => Some(
                stops
                    .with_stop_added(at.unwrap_or_else(|| stops.widest_gap_middle()))
                    .0,
            ),
            Self::RemoveGradientStop(index) => stops.without_stop(*index),
            Self::ReverseGradient => Some(stops.reversed()),
            _ => None,
        };
        if let Some(new_stops) = new_stops {
            gradient.stops = new_stops;
        }
    }
}

impl Document {
    /// Writes each object's new gradient geometry, all in **one commit** labelled
    /// [`EDIT_GRADIENT_LABEL`]. Only the `fill_gradient` register is written, and only where
    /// the geometry changes; an object without a stored gradient, and a linked clone without own
    /// style, is left alone. A batch that changes nothing makes no commit.
    ///
    /// # Errors
    /// [`StyleEditError::NoSuchObject`] if any id is gone; nothing is written then.
    pub fn set_gradient_geometries(
        &self,
        edits: &[(NodeId, GradientGeometry)],
    ) -> Result<(), StyleEditError> {
        let ids: Vec<NodeId> = edits.iter().map(|(id, _)| *id).collect();
        let metas = self.owned_style_metas(&ids)?;
        let mut wrote = false;
        for (id, meta) in &metas {
            // The last geometry named for an id wins.
            let Some((_, geometry)) = edits.iter().rev().find(|(target, _)| target == id) else {
                continue;
            };
            let old = read_style(meta);
            let mut new = old.clone();
            if let Some(gradient) = new.fill.gradient.as_mut() {
                gradient.geometry = *geometry;
            }
            wrote |= write_changes(meta, &old, &new);
        }
        if wrote {
            self.commit_with_label(EDIT_GRADIENT_LABEL);
        }
        Ok(())
    }

    /// Writes the gradient geometry `result` carries (a transform's result whose gradient
    /// [`crate::carry_gradient`] mapped, ADR 0066 §5) where it differs from the stored one, and
    /// commits with `transform_objects`; inside a [`Document::batch`] it joins the batch's step.
    /// Only `fill_gradient` is written; a clone without own style, an object without a stored
    /// gradient or an id that is gone is left alone.
    pub fn write_carried_gradient(&self, result: &ObjectSnapshot) {
        let Ok(metas) = self.owned_style_metas(&[result.id()]) else {
            return;
        };
        let mut wrote = false;
        for (_, meta) in &metas {
            wrote |= write_carried_geometry(meta, result.style().fill.gradient.as_ref());
        }
        if wrote {
            self.commit_with_label(crate::transform_objects::TRANSFORM_OBJECTS_COMMIT_LABEL);
        }
    }
}
