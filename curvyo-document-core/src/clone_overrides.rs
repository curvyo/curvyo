//! The four switches of a linked clone's overrides (`specs/0057-clone-overrides` criteria 2, 11
//! and 14, ADR 0016 §1): own placement on and off, own style on and off, one commit each.

use loro::{LoroMap, LoroTree};

use crate::clone_codec::{KEY_OWN_PLACEMENT, KEY_OWN_STYLE, is_clone_meta, write_flag};
use crate::clones::CloneError;
use crate::document::{Document, OBJECTS_TREE};
use crate::node_registers::KEY_OPACITY;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::style_model::Style;

/// The commit label of the switch to own placement (ADR 0002 §9).
pub const OWN_PLACEMENT_COMMIT_LABEL: &str = "own_placement";
/// The commit label of the switch back to the original's placement.
pub const FOLLOW_PLACEMENT_COMMIT_LABEL: &str = "follow_placement";
/// The commit label of the switch to own style.
pub const OWN_STYLE_COMMIT_LABEL: &str = "own_style";
/// The commit label of the switch back to the original's style.
pub const ORIGINAL_STYLE_COMMIT_LABEL: &str = "original_style";
/// The commit label of Spread colours (`0057` criterion 42).
pub const SPREAD_COLOURS_COMMIT_LABEL: &str = "spread_colours";

/// The new paint colours of one clone for [`Document::recolour_clones`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloneColours {
    /// The clone.
    pub clone: NodeId,
    /// Its new fill colour.
    pub fill: crate::path_model::Color,
    /// Its new stroke colour, `None` to keep it.
    pub stroke: Option<crate::path_model::Color>,
}

impl CloneColours {
    /// `style` with these colours: the fill colour, and the stroke colour when given.
    fn apply(&self, style: &mut Style) {
        style.fill.color = self.fill;
        if let Some(stroke) = self.stroke {
            style.stroke.color = stroke;
        }
    }
}

impl Document {
    /// Sets the placement state of every clone of `clones`: `own` true holds it back when its
    /// original is transformed, false lets it follow again (`0057` criterion 2). Nothing moves:
    /// only the flag is written, for the clones whose state changes, in **one commit** labelled
    /// [`OWN_PLACEMENT_COMMIT_LABEL`] or [`FOLLOW_PLACEMENT_COMMIT_LABEL`]. Returns how many
    /// changed; with none, nothing is committed.
    ///
    /// # Errors
    /// [`CloneError::NoSuchObject`] for an id that does not exist; [`CloneError::NotAClone`] for
    /// one that is not a resolvable clone. Checked before the first write.
    pub fn set_own_placement(&self, clones: &[NodeId], own: bool) -> Result<usize, CloneError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let read = self.read_objects();
        let metas = live_clone_metas(&tree, &read, clones)?;
        let changed: Vec<&LoroMap> = metas
            .iter()
            .filter(|(meta, _)| crate::clone_codec::read_overrides(meta).own_placement != own)
            .map(|(meta, _)| meta)
            .collect();
        for meta in &changed {
            write_flag(meta, KEY_OWN_PLACEMENT, own);
        }
        if !changed.is_empty() {
            self.commit_with_label(if own {
                OWN_PLACEMENT_COMMIT_LABEL
            } else {
                FOLLOW_PLACEMENT_COMMIT_LABEL
            });
        }
        Ok(changed.len())
    }

    /// Sets the style state of every clone of `clones` (`0057` criteria 11 and 14). `own` true
    /// gives each clone that has none a complete style equal to the one it shows, with the
    /// `opacity` register of its style source; every effective style is read before the first
    /// write, so a chain switched together keeps its look. `own` false removes the marker, every
    /// style key and `opacity`, so the clone shows its original's again. Only the clones whose
    /// state changes are written, in **one commit** labelled [`OWN_STYLE_COMMIT_LABEL`] or
    /// [`ORIGINAL_STYLE_COMMIT_LABEL`]. Returns how many changed; with none, nothing is committed.
    ///
    /// # Errors
    /// [`CloneError::NoSuchObject`] for an id that does not exist; [`CloneError::NotAClone`] for
    /// one that is not a resolvable clone. Checked before the first write.
    pub fn set_own_style(&self, clones: &[NodeId], own: bool) -> Result<usize, CloneError> {
        self.set_own_style_and_texts(clones, &[], own)
    }

    /// [`Document::set_own_style`] for the linked clones of `clones` and, in the **same commit**,
    /// the text clones of `texts` (`specs/0080-text-on-path` criterion 40, lead decision 2): `own`
    /// true gives each a copy of the style it shows, false removes it; a text clone's opacity
    /// stays its own register. Returns how many changed.
    ///
    /// # Errors
    /// As [`Document::set_own_style`]; [`CloneError::NotAClone`] for an id of `texts` that is not
    /// a text clone with a live source. Checked before the first write.
    pub fn set_own_style_and_texts(
        &self,
        clones: &[NodeId],
        texts: &[NodeId],
        own: bool,
    ) -> Result<usize, CloneError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let read = self.read_objects();
        let metas = live_clone_metas(&tree, &read, clones)?;
        let mut planned: Vec<(&LoroMap, Option<(Style, f64)>)> = Vec::new();
        for (meta, clone) in &metas {
            if crate::clone_codec::read_overrides(meta).own_style == own {
                continue;
            }
            let copy = own.then(|| {
                let opacity = crate::clone_write::source_opacity(&tree, clone);
                (clone.resolved.style.clone(), opacity)
            });
            planned.push((meta, copy));
        }
        let text_plan = self
            .plan_text_clone_style(texts, own)
            .map_err(|_| CloneError::NotAClone)?;
        crate::text_clone_commands::write_text_clone_style(&text_plan, own);
        for (meta, copy) in &planned {
            if let Some((style, opacity)) = copy {
                write_own_style(meta, style, *opacity);
            } else {
                clear_style_keys(meta);
                write_flag(meta, KEY_OWN_STYLE, false);
            }
        }
        if !planned.is_empty() || !text_plan.is_empty() {
            self.commit_with_label(if own {
                OWN_STYLE_COMMIT_LABEL
            } else {
                ORIGINAL_STYLE_COMMIT_LABEL
            });
        }
        Ok(planned.len() + text_plan.len())
    }
}

impl Document {
    /// Spread colours (`0057` criterion 42): every clone of `colours` gets its own style if it has
    /// none (a copy of the style it shows, as [`Document::set_own_style`]) and the given fill and
    /// stroke colours, alpha and opacity kept, in **one commit** labelled
    /// [`SPREAD_COLOURS_COMMIT_LABEL`]. A text clone among them is treated the same way
    /// (`specs/0080-text-on-path` criterion 40). Returns how many clones changed; with none,
    /// nothing is committed.
    ///
    /// # Errors
    /// As [`Document::set_own_style`]; checked before the first write.
    pub fn recolour_clones(&self, colours: &[CloneColours]) -> Result<usize, CloneError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let read = self.read_objects();
        // Text clones take part too (`specs/0080-text-on-path`, lead decision 3).
        let (texts, linked): (Vec<CloneColours>, Vec<CloneColours>) =
            colours.iter().copied().partition(|c| {
                matches!(
                    read.get(c.clone),
                    Some(crate::primitive_model::ObjectSnapshot::Text(_))
                )
            });
        let text_ids: Vec<NodeId> = texts.iter().map(|c| c.clone).collect();
        let text_plan = self
            .plan_text_unlinks(&text_ids)
            .map_err(|_| CloneError::NotAClone)?;
        let ids: Vec<NodeId> = linked.iter().map(|c| c.clone).collect();
        let metas = live_clone_metas(&tree, &read, &ids)?;
        let mut changed = 0;
        for ((meta, shown), wanted) in text_plan.iter().zip(&texts) {
            let mut style = shown.style.clone();
            wanted.apply(&mut style);
            if crate::text_clone_commands::write_text_clone_colours(meta, shown, &style) {
                changed += 1;
            }
        }
        for ((meta, clone), wanted) in metas.iter().zip(&linked) {
            let mut style = clone.resolved.style.clone();
            wanted.apply(&mut style);
            if clone.chain.own_style {
                if crate::style_codec::write_changes(meta, &clone.resolved.style, &style) {
                    changed += 1;
                }
            } else {
                let opacity = crate::clone_write::source_opacity(&tree, clone);
                write_own_style(meta, &style, opacity);
                changed += 1;
            }
        }
        if changed > 0 {
            self.commit_with_label(SPREAD_COLOURS_COMMIT_LABEL);
        }
        Ok(changed)
    }
}

/// Gives the clone node `meta` a complete own style: the marker, `style` and the `opacity`
/// register value, any older style or opacity key removed first. Uncommitted.
pub(crate) fn write_own_style(meta: &LoroMap, style: &Style, opacity: f64) {
    clear_style_keys(meta);
    crate::style_codec::write_style(meta, style);
    crate::clone_write::write_opacity_register(meta, opacity);
    write_flag(meta, KEY_OWN_STYLE, true);
}

/// `ids` with every repeat after the first left out.
fn dedup(ids: &[NodeId]) -> Vec<NodeId> {
    let mut seen = std::collections::HashSet::with_capacity(ids.len());
    ids.iter().copied().filter(|id| seen.insert(*id)).collect()
}

/// The meta maps and resolved reads of `ids` (repeats dropped, in order), each a live clone leaf
/// that resolves in `read` (the one read of the command, review L4).
fn live_clone_metas<'r>(
    tree: &LoroTree,
    read: &'r crate::clone_read::ObjectRead,
    ids: &[NodeId],
) -> Result<Vec<(LoroMap, &'r crate::clone_model::CloneSnapshot)>, CloneError> {
    dedup(ids)
        .into_iter()
        .map(|id| {
            let tree_id = tree_id_of(id);
            if !crate::path_codec::leaf_exists(tree, tree_id) {
                return Err(CloneError::NoSuchObject);
            }
            let meta = tree
                .get_meta(tree_id)
                .map_err(|_| CloneError::NoSuchObject)?;
            match read.get(id) {
                Some(crate::primitive_model::ObjectSnapshot::Clone(clone))
                    if is_clone_meta(&meta) =>
                {
                    Ok((meta, clone))
                }
                _ => Err(CloneError::NotAClone),
            }
        })
        .collect()
}

/// Removes every style key and `opacity` from a clone node, residue included. Uncommitted.
fn clear_style_keys(meta: &LoroMap) {
    for key in crate::style_codec::ALL_STYLE_KEYS
        .iter()
        .copied()
        .chain([KEY_OPACITY])
    {
        if meta.get(key).is_some() {
            // invariant: deleting a key just confirmed present on an attached map cannot fail.
            #[allow(clippy::unwrap_used)]
            meta.delete(key).unwrap();
        }
    }
}
