//! The adjustment commands of one picture, one commit each (ADR 0072 §2, §7; `specs/0135`
//! criteria 9, 24, 27, 59): an edit of its registers, Restore original, Own and Follow original.

use loro::LoroMap;

use crate::clone_codec::{KEY_OWN_ADJUSTMENTS, write_flag};
use crate::document::{Document, OBJECTS_TREE};
use crate::image_adjust_codec::{
    raw_registers, remove_all, remove_dither, write_adjustments, write_raw_registers,
};
use crate::image_adjust_model::{
    AdjustError, AdjustOwner, AdjustmentsWrite, EDIT_ADJUSTMENTS_COMMIT_LABEL, EXPERT_PRESET,
    FOLLOW_ORIGINAL_COMMIT_LABEL, ImageAdjustments, MAX_IMAGE_STRENGTH,
    RESTORE_ORIGINAL_COMMIT_LABEL, ShownAdjustments,
};
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::primitive_model::ObjectSnapshot;

/// A picture as an adjustment command finds it.
struct Target {
    /// The node the registers are written to: the image, or the clone.
    meta: LoroMap,
    /// What it shows now.
    shown: ShownAdjustments,
    /// For a clone without own adjustments, the node they are read from.
    source: Option<LoroMap>,
}

impl Document {
    /// The adjustments the image or image clone `target` shows, and whose they are
    /// (criterion 9).
    ///
    /// # Errors
    /// [`AdjustError::NotAnImage`] when `target` is neither a live image nor a resolvable clone
    /// whose root is an image.
    pub fn shown_adjustments(&self, target: NodeId) -> Result<ShownAdjustments, AdjustError> {
        self.adjust_target(target).map(|t| t.shown)
    }

    /// Writes `write` onto the picture `target` in **one commit** "Edit adjustments" (criteria 2,
    /// 24, 27); no operation is written (ADR 0072 amendment A3). On a clone that shows its
    /// original's, the first edit gives it its own adjustments: the marker and a copy of the four
    /// registers it showed, then the edit (ADR 0072 §2, A2). `Ok(false)` and nothing written when
    /// nothing would change.
    ///
    /// # Errors
    /// [`AdjustError::NotAnImage`]; [`AdjustError::StrengthOutOfRange`] above 100. Checked before
    /// the first write.
    pub fn edit_image_adjustments(
        &self,
        target: NodeId,
        write: &AdjustmentsWrite,
    ) -> Result<bool, AdjustError> {
        if write.strength.is_some_and(|s| s > MAX_IMAGE_STRENGTH) {
            return Err(AdjustError::StrengthOutOfRange);
        }
        let target = self.adjust_target(target)?;
        if applied(&target.shown.adjustments, write) == target.shown.adjustments {
            return Ok(false);
        }
        if let Some(source) = &target.source {
            own_copy(&target.meta, source);
        }
        write_adjustments(&target.meta, write);
        self.commit_with_label(EDIT_ADJUSTMENTS_COMMIT_LABEL);
        Ok(true)
    }

    /// Restore original (criterion 59): Preset Expert, an empty stack and no Dither (it reads as
    /// Floyd-Steinberg), Strength kept, in one commit "Restore original"; on a clone its own
    /// adjustments become empty (the marker, Expert, the Strength it showed, no stack, no Dither).
    /// The operations are not touched. `Ok(false)` and nothing written when the picture shows
    /// nothing to restore (amendment A3: enabled while adjusted or a Dither is stored).
    ///
    /// # Errors
    /// [`AdjustError::NotAnImage`].
    pub fn restore_original(&self, target: NodeId) -> Result<bool, AdjustError> {
        let target = self.adjust_target(target)?;
        if !target.shown.adjustments.is_restorable() {
            return Ok(false);
        }
        let strength = target.shown.adjustments.strength;
        if target.source.is_some() {
            write_flag(&target.meta, KEY_OWN_ADJUSTMENTS, true);
        }
        let empty = AdjustmentsWrite {
            preset: Some(EXPERT_PRESET.to_owned()),
            strength: (target.shown.owner != AdjustOwner::Image).then_some(strength),
            steps: Some(Vec::new()),
            dither: None,
        };
        if target.shown.owner == AdjustOwner::Image {
            remove_dither(&target.meta);
        } else {
            remove_all(&target.meta);
        }
        write_adjustments(&target.meta, &empty);
        self.commit_with_label(RESTORE_ORIGINAL_COMMIT_LABEL);
        Ok(true)
    }

    /// Own on a clone's Adjustments override row (criterion 9): writes the marker and a copy of
    /// the source's four registers as stored (Preset, Strength, stack and Dither, damage included) in one
    /// commit "Edit adjustments", so the clone shows what it showed. `Ok(false)` when it has its
    /// own already.
    ///
    /// # Errors
    /// [`AdjustError::NotAnImage`]; [`AdjustError::NotAClone`] for an image.
    pub fn own_adjustments(&self, clone: NodeId) -> Result<bool, AdjustError> {
        let target = self.adjust_target(clone)?;
        match (target.shown.owner, &target.source) {
            (AdjustOwner::Image, _) => Err(AdjustError::NotAClone),
            (AdjustOwner::CloneFollowing, Some(source)) => {
                own_copy(&target.meta, source);
                self.commit_with_label(EDIT_ADJUSTMENTS_COMMIT_LABEL);
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Original's on a clone's Adjustments override row (criterion 9): removes the marker and the
    /// clone's four registers in one commit "Follow original". `Ok(false)` when it already
    /// follows.
    ///
    /// # Errors
    /// [`AdjustError::NotAnImage`]; [`AdjustError::NotAClone`] for an image.
    pub fn follow_original_adjustments(&self, clone: NodeId) -> Result<bool, AdjustError> {
        let target = self.adjust_target(clone)?;
        match target.shown.owner {
            AdjustOwner::Image => Err(AdjustError::NotAClone),
            AdjustOwner::CloneFollowing => Ok(false),
            AdjustOwner::CloneOwn => {
                remove_all(&target.meta);
                write_flag(&target.meta, KEY_OWN_ADJUSTMENTS, false);
                self.commit_with_label(FOLLOW_ORIGINAL_COMMIT_LABEL);
                Ok(true)
            }
        }
    }

    /// The adjustment registers node `id` stores, as read: on an image when any is stored, on a
    /// clone with the `own_adjustments` marker always (the `document.json` view).
    pub(crate) fn stored_adjustments(&self, id: NodeId) -> Option<ImageAdjustments> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let meta = tree.get_meta(tree_id_of(id)).ok()?;
        let stored = if crate::image_codec::is_image_meta(&meta) {
            raw_registers(&meta).iter().any(Option::is_some)
        } else {
            crate::clone_codec::is_clone_meta(&meta)
                && crate::clone_codec::read_overrides(&meta).own_adjustments
        };
        stored.then(|| crate::image_adjust_codec::read_adjustments(&meta))
    }

    fn adjust_target(&self, id: NodeId) -> Result<Target, AdjustError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let meta_of = |id: NodeId| {
            let node = tree_id_of(id);
            crate::path_codec::node_exists(&tree, node)
                .then(|| tree.get_meta(node).ok())
                .flatten()
        };
        let meta = meta_of(id).ok_or(AdjustError::NotAnImage)?;
        match self.object(id) {
            Some(ObjectSnapshot::Image(image)) => Ok(Target {
                meta,
                shown: ShownAdjustments {
                    adjustments: image.adjustments,
                    owner: AdjustOwner::Image,
                },
                source: None,
            }),
            Some(ObjectSnapshot::Clone(clone)) => {
                let shown = match clone.chain.picture.as_deref() {
                    Some([ObjectSnapshot::Image(image)]) if image.id == clone.chain.root => {
                        image.adjustments.clone()
                    }
                    _ => return Err(AdjustError::NotAnImage),
                };
                let own = clone.chain.own_adjustments;
                let source = if own {
                    None
                } else {
                    Some(meta_of(clone.chain.adjust_source).ok_or(AdjustError::NotAnImage)?)
                };
                Ok(Target {
                    meta,
                    shown: ShownAdjustments {
                        adjustments: shown,
                        owner: if own {
                            AdjustOwner::CloneOwn
                        } else {
                            AdjustOwner::CloneFollowing
                        },
                    },
                    source,
                })
            }
            _ => Err(AdjustError::NotAnImage),
        }
    }
}

/// The registers `adjustments` with `write` applied, as they would read back.
fn applied(adjustments: &ImageAdjustments, write: &AdjustmentsWrite) -> ImageAdjustments {
    let mut next = adjustments.clone();
    if let Some(preset) = &write.preset {
        next.preset = Some(preset.clone());
        next.damage.preset = false;
    }
    if let Some(strength) = write.strength {
        next.strength = strength;
        next.damage.strength = false;
    }
    if let Some(steps) = &write.steps {
        next.steps.clone_from(steps);
        next.damage.stack = false;
    }
    if let Some(dither) = &write.dither {
        next.dither = Some(dither.clone());
        next.damage.dither = false;
    }
    next
}

/// Gives the clone node `meta` its own adjustments: the marker and the four registers of
/// `source` exactly as stored (damage included), uncommitted (ADR 0072 §2).
pub(crate) fn own_copy(meta: &LoroMap, source: &LoroMap) {
    write_raw_registers(meta, &raw_registers(source));
    write_flag(meta, KEY_OWN_ADJUSTMENTS, true);
}
