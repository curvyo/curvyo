//! The linked-clone commands of the session with their notices (`specs/0049-linked-clone`):
//! Linked clone, Unlink clone, Select original, Select root original and Select clones, the
//! clone block of the panel, and the orphan repair after an Open.

use std::sync::Arc;

use curvyo_document_core::{CloneError, CopySource, NodeId, ObjectSnapshot};
use curvyo_ui_core::{
    CloneBlock, LinkedClonePlan, clone_block, followers_of, linked_clone_shown, plan_linked_clone,
    select_clones, select_originals, select_root_originals,
};

use super::refusal::RefusalMarks;
use super::{Session, Tool};

/// The refusal of a ninth level (criterion 52).
pub const TOO_DEEP_NOTICE: &str = "Linked clones can follow each other 8 levels deep. Clone the root original instead. Nothing was changed.";

/// What a clone command did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneKind {
    /// Nothing: not available now.
    Ignored,
    /// The command ran.
    Done,
    /// Refused; nothing was changed.
    Refused,
}

/// The outcome of a clone command with its one-line notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneResult {
    /// What happened.
    pub kind: CloneKind,
    /// The sentence to show; empty when ignored.
    pub notice: String,
}

impl CloneResult {
    pub(super) fn ignored() -> Self {
        Self {
            kind: CloneKind::Ignored,
            notice: String::new(),
        }
    }

    pub(super) fn done(notice: String) -> Self {
        Self {
            kind: CloneKind::Done,
            notice,
        }
    }

    /// The code the host keys on: `"ignored"`, `"done"` or `"refused"`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self.kind {
            CloneKind::Ignored => "ignored",
            CloneKind::Done => "done",
            CloneKind::Refused => "refused",
        }
    }
}

/// What the Style tab shows about linked clones (criteria 22, 23, 56).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClonePanel {
    /// The clone block, when the selection holds only clones.
    pub block: Option<CloneBlock>,
    /// How many clones follow the selected objects directly (the origin line from 1 on).
    pub followers: usize,
    /// How many selected objects clones follow (for "these objects").
    pub followed: usize,
}

/// "1 linked clone" / "3 linked clones".
fn clones_word(count: usize) -> String {
    format!("{count} {}", if count == 1 { "clone" } else { "clones" })
}

impl Session {
    /// Whether the Linked clone button is in the Select bar (criterion 4).
    #[must_use]
    pub fn linked_clone_shown(&self) -> bool {
        self.tool == Tool::Select && linked_clone_shown(&self.selection, &self.units())
    }

    /// Linked clone (criteria 1 to 3, 5, 52): one clone above each selected object in one commit,
    /// the clones selected; shapes are converted to paths first. A selected clone eight levels
    /// deep refuses all of it and outlines the offenders.
    pub fn linked_clone(&mut self) -> CloneResult {
        self.recording(|this| {
            if !this.rail_command_ready() {
                return CloneResult::ignored();
            }
            this.flush_select_bar_preview();
            this.end_colour_pick();
            let objects = this.objects();
            let units = this.units();
            let plan = plan_linked_clone(
                &this.document,
                &objects,
                &this.selection,
                &units,
                &mut this.minter,
            );
            match plan {
                None => CloneResult::ignored(),
                Some(LinkedClonePlan::TooDeep(offenders)) => {
                    this.command_refusal = Some(RefusalMarks::new(offenders, &this.selection));
                    CloneResult {
                        kind: CloneKind::Refused,
                        notice: TOO_DEEP_NOTICE.to_owned(),
                    }
                }
                Some(LinkedClonePlan::Ready {
                    sources,
                    conversions,
                }) => match this.document.linked_clone(&sources, &conversions) {
                    Ok(clones) => {
                        this.command_refusal = None;
                        this.node.clear_selection();
                        this.selection.set(&clones);
                        this.hovered_object = None;
                        CloneResult::done(format!("Created {}.", linked(clones.len())))
                    }
                    Err(CloneError::TooDeep(offenders)) => {
                        this.command_refusal = Some(RefusalMarks::new(offenders, &this.selection));
                        CloneResult {
                            kind: CloneKind::Refused,
                            notice: TOO_DEEP_NOTICE.to_owned(),
                        }
                    }
                    Err(_) => CloneResult::ignored(),
                },
            }
        })
    }

    /// Unlink clone (criteria 28, 55): every selected clone becomes a path in one commit, still
    /// selected; the anchor ids are minted here.
    pub fn unlink_clones(&mut self) -> CloneResult {
        self.recording(|this| {
            if this.tool != Tool::Select && this.tool != Tool::Node {
                return CloneResult::ignored();
            }
            let objects = this.objects();
            let unlinks: Vec<CopySource> = objects
                .iter()
                .filter_map(|object| match object {
                    ObjectSnapshot::Clone(clone) if this.selection.ids().contains(&clone.id()) => {
                        Some(CopySource {
                            id: clone.id(),
                            anchor_ids: clone
                                .resolved
                                .all_anchors()
                                .map(|_| this.minter.mint())
                                .collect(),
                        })
                    }
                    _ => None,
                })
                .collect();
            // Text clones become texts (`specs/0080-text-on-path` criterion 41).
            let texts = this.selected_text_clones();
            if unlinks.is_empty() && texts.is_empty() {
                return CloneResult::ignored();
            }
            // One step for both kinds (lead decision 2).
            match this.document.unlink_clones_and_texts(&unlinks, &texts) {
                Ok(n) => CloneResult::done(format!("Unlinked {}.", clones_word(n))),
                Err(_) => CloneResult::ignored(),
            }
        })
    }

    /// Select original (criterion 25): the direct originals of the selected clones. View state.
    pub fn select_original(&mut self) -> CloneResult {
        let targets = select_originals(&self.object_read(), self.selection.ids());
        self.select_targets(&targets, "original")
    }

    /// Select root original (criterion 56): the roots of the selected clones. View state.
    pub fn select_root_original(&mut self) -> CloneResult {
        let targets = select_root_originals(&self.object_read(), self.selection.ids());
        self.select_targets(&targets, "original")
    }

    /// Select clones (criterion 25): the clones that follow the selected objects directly.
    pub fn select_clones(&mut self) -> CloneResult {
        let targets = select_clones(&self.object_read(), self.selection.ids());
        self.select_targets(&targets, "clone")
    }

    /// Selects `targets` in the context of the group that holds the first one (`0023`
    /// criterion 15). No commit.
    ///
    /// A hidden or locked target is left out (`0050` criterion 14); with none left the selection
    /// stays and a refusal says why, with some left a status says how many (`0049` criterion 25).
    fn select_targets(&mut self, all: &[NodeId], what: &str) -> CloneResult {
        if all.is_empty() {
            return CloneResult::ignored();
        }
        let inert = Arc::clone(&self.document_read().inert);
        let targets: Vec<NodeId> = all
            .iter()
            .copied()
            .filter(|id| !inert.contains(id))
            .collect();
        let Some(&first) = targets.first() else {
            let notice = if all.len() == 1 {
                format!(
                    "The {what} is hidden or locked. Show or unlock it in the Layers tab. Nothing was changed."
                )
            } else {
                format!(
                    "The {what}s are hidden or locked. Show or unlock them in the Layers tab. Nothing was changed."
                )
            };
            return CloneResult {
                kind: CloneKind::Refused,
                notice,
            };
        };
        let partial = (targets.len() < all.len()).then(|| {
            format!(
                "Selected {} of {} {what}s. The rest are hidden or locked.",
                targets.len(),
                all.len()
            )
        });
        let targets = &targets[..];
        let group = self.document.parent_group(first);
        if group != self.context_group() {
            match group {
                Some(group) => {
                    self.enter_group(group, None);
                }
                None => {
                    self.leave_to_level(0);
                }
            }
        }
        self.node.clear_selection();
        self.selection.set(targets);
        self.hovered_object = None;
        CloneResult::done(partial.unwrap_or_default())
    }

    /// The subject line for a selection of clones only (criterion 21): "Clone of Path 3f9a1c"
    /// for one, "3 clones" for several; `None` for any other selection.
    pub(super) fn clone_subject(&self) -> Option<String> {
        let ids = self.selection.ids();
        if ids.is_empty() {
            return None;
        }
        let read = self.object_read();
        let clone = |id: NodeId| match read.get(id) {
            Some(ObjectSnapshot::Clone(_)) => true,
            Some(ObjectSnapshot::Text(text)) => text.source.is_some(),
            _ => false,
        };
        if !ids.iter().all(|&id| clone(id)) {
            return None;
        }
        Some(match ids {
            [one] => curvyo_ui_core::object_name(&read, *one),
            many => format!("{} clones", many.len()),
        })
    }

    /// What the Style tab shows about linked clones for the selection.
    #[must_use]
    pub fn clone_panel(&self) -> ClonePanel {
        let read = self.object_read();
        let selected = self.selection.ids();
        let (followers, followed) = followers_of(&read, selected);
        ClonePanel {
            block: clone_block(&read, selected),
            followers,
            followed,
        }
    }

    /// The sentence the clone repair of the last command adds to its notice (criterion 31):
    /// "3 clones became paths.", "2 clones now follow their next original.", both, or empty;
    /// before them "1 text kept its path." when a removed carrier left its outline to texts
    /// (`specs/0080-text-on-path` criterion 25).
    /// Read it once after a command that removes objects.
    pub fn take_clone_repair_notice(&mut self) -> String {
        let repair = self.document.take_clone_repair();
        let kept = self.document.take_kept_carriers();
        let mut parts = Vec::new();
        // The texts on a removed carrier keep its outline (`specs/0080-text-on-path` criterion 25).
        if kept > 0 {
            parts.push(if kept == 1 {
                "1 text kept its path.".to_owned()
            } else {
                format!("{kept} texts kept their paths.")
            });
        }
        if repair.unlinked > 0 {
            parts.push(if repair.unlinked == 1 {
                "1 clone became a path.".to_owned()
            } else {
                format!("{} clones became paths.", repair.unlinked)
            });
        }
        if let Some(sentence) = self.text_clones_unlinked_sentence() {
            parts.push(sentence);
        }
        if repair.repointed > 0 {
            parts.push(if repair.repointed == 1 {
                "1 clone now follows its next original.".to_owned()
            } else {
                format!(
                    "{} clones now follow their next original.",
                    repair.repointed
                )
            });
        }
        parts.join(" ")
    }

    /// The notice of the last Delete key when it repaired clones (criterion 31), once: "Deleted
    /// 1 object. 1 clone became a path."; empty when no clone was repaired.
    pub fn take_delete_notice(&mut self) -> String {
        let deleted = std::mem::take(&mut self.deleted_objects);
        let repaired = self.take_clone_repair_notice();
        if repaired.is_empty() {
            return repaired;
        }
        let what = curvyo_ui_core::objects_count_text(deleted.max(1));
        format!("Deleted {what}. {repaired}")
    }

    /// Removes the clones whose original cannot be found, after an Open (criterion 35), as one
    /// step, turns a clone with no link but an outline into that path (criterion 38), and keeps
    /// the notice for the host ([`Session::take_orphan_notice`]).
    pub(super) fn remove_orphans_after_open(&mut self) {
        let _ = self.document.take_clone_repair();
        let removed = self.recording(|this| this.document.remove_orphan_clones());
        let mut parts = Vec::new();
        match removed {
            0 => {}
            1 => parts.push("Removed 1 clone whose original was not found.".to_owned()),
            n => parts.push(format!("Removed {n} clones whose original was not found.")),
        }
        // A clone with no link but an outline became that path (criterion 38).
        let converted = self.take_clone_repair_notice();
        if !converted.is_empty() {
            parts.push(converted);
        }
        self.orphan_notice = (!parts.is_empty()).then(|| parts.join(" "));
    }

    /// The notice of the orphan repair after the Open, once (empty when nothing was removed).
    pub fn take_orphan_notice(&mut self) -> String {
        self.orphan_notice.take().unwrap_or_default()
    }

    /// The refusal of the last command because a linked clone would pass the limit (`0049`
    /// criterion 17), once: "Nothing was changed: Clone of Path 3f9a1c would move beyond the
    /// limit of 10 km." Empty when nothing was refused.
    pub fn take_clone_limit_notice(&mut self) -> String {
        let Some(clone) = self.document.take_clone_refusal() else {
            return String::new();
        };
        let name = curvyo_ui_core::object_name(&self.object_read(), clone);
        format!("Nothing was changed: {name} would move beyond the limit of 10 km.")
    }

    /// What a screen reader hears when one linked clone is selected (`0049` criterion 62): "Linked
    /// clone of Path 3f9a1c selected, 12.0 by 8.0 mm. Its original is Path 3f9a1c." Empty for any
    /// other single object (its announcement is the host's).
    pub(super) fn clone_announcement(&self, id: NodeId) -> String {
        let read = self.object_read();
        let Some(ObjectSnapshot::Clone(clone)) = read.get(id) else {
            return String::new();
        };
        let original = curvyo_ui_core::object_name(&read, clone.chain.origin);
        let (min, max) =
            curvyo_ui_core::object_outline_bounds(&ObjectSnapshot::Clone(clone.clone()));
        let mut text = format!(
            "Linked clone of {original} selected, {:.1} by {:.1} mm. Its original is {original}.",
            max.x - min.x,
            max.y - min.y
        );
        // A chain names its root too (design system, "Screen reader").
        if clone.chain.root != clone.chain.origin {
            text.push_str(" Root original: ");
            text.push_str(&curvyo_ui_core::object_name(&read, clone.chain.root));
            text.push('.');
        }
        text
    }

    /// Whether the Node tool's selection is linked clones only: the Node bar then holds the
    /// sentence that a clone has no nodes (criterion 26).
    #[must_use]
    pub fn node_selection_is_clones(&self) -> bool {
        let read = self.object_read();
        let held: Vec<&ObjectSnapshot> = self
            .selection
            .ids()
            .iter()
            .filter_map(|id| read.get(*id))
            .collect();
        // A compound path has nodes since `specs/0055-compound-path-node-editing`, so the clone
        // sentence stands only for a selection of clones alone.
        !held.is_empty()
            && held
                .iter()
                .all(|object| matches!(object, ObjectSnapshot::Clone(_)))
    }
}

/// "1 linked clone" / "3 linked clones".
fn linked(count: usize) -> String {
    format!(
        "{count} linked {}",
        if count == 1 { "clone" } else { "clones" }
    )
}
