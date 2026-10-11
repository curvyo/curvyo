//! The group the Select tool works in, and what the tools see and draw there (`specs/0023-groups`).
//!
//! The context is a stack of entered groups, view state like the selection: not saved and not
//! undone. It is read lazily, so a group a peer deleted or an undo removed falls back to the
//! nearest group that still exists (criterion 20). Everything the tools work on
//! ([`Session::objects`]) is the leaves inside the context; the whole document is only drawn
//! ([`Session::render_objects`]), with the outside dimmed. The Group and Ungroup commands are in
//! `grouping.rs`.

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

use curvyo_document_core::{NodeId, ObjectSnapshot};
use curvyo_render_core::dim_outside;
use curvyo_ui_core::{Units, context_announcement};

use super::{Session, Tool};

/// The objects and units of one context at one document read.
#[derive(Debug, Clone)]
struct Held {
    /// The whole document's objects this entry was made from (compared by identity).
    all: Arc<[ObjectSnapshot]>,
    context: Option<NodeId>,
    objects: Arc<[ObjectSnapshot]>,
    units: Arc<Units>,
    /// The whole document with the outside of the context dimmed, made when first drawn.
    dimmed: Option<Arc<[ObjectSnapshot]>>,
}

/// The cache of [`Held`]: valid while the document read and the context are the ones it was
/// made for, so a frame and the events between two frames build it once.
#[derive(Debug, Default)]
pub(super) struct ContextCache {
    held: RefCell<Option<Held>>,
}

impl Session {
    /// Every object of the document as the document held it at its current version: what is
    /// drawn. A Select or Node-tool drag keeps its first read for its whole life
    /// (`specs/0031` criterion 17).
    pub(super) fn all_objects(&self) -> Arc<[ObjectSnapshot]> {
        let drag = (self.tool == Tool::Select && self.select.drag_in_flight())
            || (self.tool == Tool::Node && self.node.drag_in_flight());
        self.object_cache.get(&self.document, drag)
    }

    /// The objects the tools work on: the leaves inside the entered group, or every leaf at the
    /// root. Everything outside the entered group is inert (criterion 15).
    pub(super) fn objects(&self) -> Arc<[ObjectSnapshot]> {
        self.held_context().objects
    }

    /// The grouping of the leaves under the context.
    pub(super) fn units(&self) -> Arc<Units> {
        self.held_context().units
    }

    /// The objects to draw: the whole document, with everything outside the entered group dimmed
    /// (criterion 15). Without an entered group, [`Session::all_objects`].
    pub(super) fn render_objects(&self) -> Arc<[ObjectSnapshot]> {
        let held = self.held_context();
        if held.context.is_none() {
            return held.all;
        }
        if let Some(dimmed) = held.dimmed {
            return dimmed;
        }
        let inside: HashSet<NodeId> = held.objects.iter().map(ObjectSnapshot::id).collect();
        let dimmed: Arc<[ObjectSnapshot]> =
            dim_outside(&held.all, &|id| inside.contains(&id)).into();
        if let Some(entry) = self.context_cache.held.borrow_mut().as_mut() {
            entry.dimmed = Some(Arc::clone(&dimmed));
        }
        dimmed
    }

    fn held_context(&self) -> Held {
        let all = self.all_objects();
        let context = self.context_group();
        if let Some(held) = self.context_cache.held.borrow().as_ref()
            && Arc::ptr_eq(&held.all, &all)
            && held.context == context
        {
            return held.clone();
        }
        let units = Arc::new(Units::build(&self.document, context));
        let objects = match context {
            None => Arc::clone(&all),
            Some(group) => {
                let leaves: HashSet<NodeId> = self.document.leaves_of(group).into_iter().collect();
                all.iter()
                    .filter(|object| leaves.contains(&object.id()))
                    .cloned()
                    .collect()
            }
        };
        let held = Held {
            all,
            context,
            objects,
            units,
            dimmed: None,
        };
        *self.context_cache.held.borrow_mut() = Some(held.clone());
        held
    }

    /// The group the Select tool works in: the innermost entered group that still exists, `None`
    /// at the document root (criterion 20).
    pub(super) fn context_group(&self) -> Option<NodeId> {
        self.context
            .iter()
            .rev()
            .copied()
            .find(|&group| self.document.is_group(group))
    }

    /// The entered groups from the outermost to the innermost, as the document holds them now.
    fn context_chain(&self) -> Vec<NodeId> {
        let Some(group) = self.context_group() else {
            return Vec::new();
        };
        let mut chain = self.document.ancestor_groups(group);
        chain.reverse();
        chain.push(group);
        chain
    }

    /// How many groups deep the Select tool is: 0 at the document root. The host shows the
    /// indicator "Open groups" with one crumb per level (criterion 16).
    #[must_use]
    pub fn context_depth(&self) -> usize {
        self.context_chain().len()
    }

    /// Brings the session in line with the context before an event is dispatched: a group that is
    /// gone is left (the context falls back to the nearest one that exists and a screen reader is
    /// told) and new objects go into the entered group. Reads nothing from the document at the
    /// root, so a drawing tool pays nothing for groups.
    pub(super) fn sync_context(&mut self) {
        let live = self.context_group();
        if self.context.last().copied() != live {
            let chain = self.context_chain();
            let announce = !self.context.is_empty();
            self.context = chain;
            if announce {
                self.context_announcement = Some(context_announcement(self.context.len(), false));
            }
        }
        self.document.set_draw_parent(live);
    }

    /// Tells the Select tool which leaves belong together in the current context. Called before
    /// every Select-tool event that selects, toggles, copies or opens by object.
    pub(super) fn sync_select_units(&mut self) {
        self.sync_context();
        let units = self.units();
        self.select.set_units(units);
    }

    /// Makes `chain` the entered groups and starts from an empty selection and no hover.
    fn set_context(&mut self, chain: Vec<NodeId>, entered: bool) {
        self.context = chain;
        self.document.set_draw_parent(self.context.last().copied());
        self.selection.clear();
        self.select.forget_press();
        self.hovered_object = None;
        self.command_refusal = None;
        self.node.cancel_drag();
        self.node.clear_selection();
        let units = self.units();
        self.select.set_units(units);
        self.context_announcement = Some(context_announcement(self.context.len(), entered));
    }

    /// Enters `group` (a double-click on it, or Enter with it selected): it becomes the context,
    /// and `leaf`, the leaf under the pointer, selects the object of the new context that holds
    /// it; without `leaf` nothing is selected (criterion 15).
    pub(super) fn enter_group(&mut self, group: NodeId, leaf: Option<NodeId>) -> bool {
        if !self.document.is_group(group) {
            return false;
        }
        let mut chain = self.document.ancestor_groups(group);
        chain.reverse();
        chain.push(group);
        self.set_context(chain, true);
        if let Some(leaf) = leaf {
            let members = self.units().leaves_with(&leaf).to_vec();
            self.selection.set(&members);
        }
        true
    }

    /// Enter with exactly one group selected: enters it with nothing selected (criterion 15).
    /// Returns whether it did.
    pub(super) fn enter_selected_group(&mut self) -> bool {
        let units = self.units();
        let selected = units.selected(self.selection.ids());
        match selected.as_slice() {
            [unit] if unit.is_group => self.enter_group(unit.id, None),
            _ => false,
        }
    }

    /// Leaves the innermost entered group (Escape with nothing selected, a double-click on empty
    /// canvas): the context moves up one level. With `select_left` the group just left becomes
    /// the selection (Escape, criterion 17). Returns whether there was a group to leave.
    pub(super) fn leave_group(&mut self, select_left: bool) -> bool {
        let Some(left) = self.context_group() else {
            return false;
        };
        let mut chain = self.context_chain();
        chain.pop();
        self.set_context(chain, false);
        if select_left {
            let members = self.units().leaves_of_unit(left);
            self.selection.set(&members);
        }
        true
    }

    /// A crumb of the indicator: makes the context the first `level` entered groups (0 is the
    /// document root, which leaves every level at once). Returns whether the context changed.
    pub fn leave_to_level(&mut self, level: usize) -> bool {
        let mut chain = self.context_chain();
        if level >= chain.len() {
            return false;
        }
        chain.truncate(level);
        self.set_context(chain, false);
        true
    }

    /// What a screen reader should hear about the context, once (entering and leaving a group,
    /// and a context that fell back).
    pub fn take_context_announcement(&mut self) -> String {
        self.context_announcement.take().unwrap_or_default()
    }
}
