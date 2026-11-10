//! The cloning drop of a Select move onto a laser page (`specs/0100-bring-content-in` criteria 1
//! to 6, `adrs.md` notes 3 and 4): what a release would do, decided per frame from the
//! selection, the page under the pointer and Alt and Ctrl; nothing is written before the release.

use curvyo_document_core::{NodeId, ObjectSnapshot, Page, PageKind, Point, Vec2};
use curvyo_ui_core::{Modifiers, Reach, object_bounds};

use super::Session;
use super::row_drag::{DropBadge, RowDragAnswer};

/// What a release of the move does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropMode {
    /// Linked clones of the selection on the laser page (criterion 1).
    Clone,
    /// The objects themselves move (Alt, criterion 3).
    Move,
    /// Independent copies (Ctrl, also with Alt, criterion 4).
    Copy,
    /// The move or Ctrl copy as built, with no badge (criterion 5).
    Plain,
    /// A selection of only guides and dimensions over a laser page: no badge, and the
    /// release writes nothing (criterion 6).
    Nothing,
}

/// The kind of place under the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// A laser page whose extension is on.
    LaserPage,
    /// Anything else: a drawing, sketch, "Laser (off)" or unknown page, or the pasteboard.
    Other,
}

/// The one decision of criteria 1 to 5.
#[must_use]
pub const fn drop_mode(
    selection_on_laser_page: bool,
    target: TargetKind,
    alt: bool,
    ctrl: bool,
) -> DropMode {
    if selection_on_laser_page || !matches!(target, TargetKind::LaserPage) {
        return DropMode::Plain;
    }
    if ctrl {
        DropMode::Copy
    } else if alt {
        DropMode::Move
    } else {
        DropMode::Clone
    }
}

/// The badge of `mode`.
const fn badge_of(mode: DropMode) -> DropBadge {
    match mode {
        DropMode::Clone => DropBadge::Clone,
        DropMode::Move => DropBadge::Move,
        DropMode::Copy => DropBadge::Copy,
        DropMode::Plain | DropMode::Nothing => DropBadge::None,
    }
}

/// A Select move that may end as a cloning drop.
#[derive(Debug, Clone)]
pub(super) struct CloneDrop {
    /// The selected objects that are cloned: everything but guides and dimensions.
    originals: Vec<NodeId>,
    /// How many guides and dimensions the selection holds (criterion 6's notice).
    left_out: (usize, usize),
    /// Whether the selection holds a member of a laser page (criterion 5).
    on_laser: bool,
    mode: DropMode,
    /// The laser page under the pointer.
    target: Option<String>,
}

impl CloneDrop {
    /// The drop of a move of `selection` among `objects`, `guides` the ids drawn as guides,
    /// `on_laser` whether one of them is a member of a laser page.
    pub(super) fn begin(
        units: &[curvyo_ui_core::SelectedUnit],
        objects: &[ObjectSnapshot],
        guides: &std::collections::HashSet<NodeId>,
        on_laser: bool,
    ) -> Self {
        let mut originals = Vec::new();
        let mut left_out = (0, 0);
        for unit in units {
            // A group is cloned whole, with everything in it (criterion 6).
            if unit.is_group {
                originals.push(unit.id);
                continue;
            }
            let Some(object) = objects.iter().find(|o| o.id() == unit.id) else {
                continue;
            };
            if matches!(object, ObjectSnapshot::Dimension(_)) {
                left_out.1 += 1;
            } else if guides.contains(&object.id()) {
                left_out.0 += 1;
            } else {
                originals.push(object.id());
            }
        }
        Self {
            originals,
            left_out,
            on_laser,
            mode: DropMode::Plain,
            target: None,
        }
    }

    /// One frame: the page `under` the pointer (with whether it is a laser page with its
    /// extension on) and the modifiers. A selection of only guides and dimensions gets no badge.
    pub(super) fn update(
        &mut self,
        under: Option<(&Page, bool)>,
        modifiers: Modifiers,
    ) -> RowDragAnswer {
        let target = match under {
            Some((_, true)) if !self.originals.is_empty() => TargetKind::LaserPage,
            _ => TargetKind::Other,
        };
        self.mode = drop_mode(self.on_laser, target, modifiers.alt, modifiers.ctrl);
        // Guides and dimensions never run: alone they are not cloned, moved or copied onto a
        // laser page (criterion 6).
        let over_laser = matches!(under, Some((_, true)));
        if self.originals.is_empty() && self.left_out != (0, 0) && over_laser && !self.on_laser {
            self.mode = DropMode::Nothing;
        }
        self.target = (self.mode != DropMode::Plain)
            .then(|| under.map(|(page, _)| page.id.clone()))
            .flatten();
        self.answer()
    }

    /// The answer of the last frame.
    pub(super) fn answer(&self) -> RowDragAnswer {
        RowDragAnswer {
            badge: badge_of(self.mode),
            target: self.target.clone(),
            faint: false,
            fit: None,
        }
    }

    /// What a release does now.
    pub(super) const fn mode(&self) -> DropMode {
        self.mode
    }

    /// The notice of the guides and dimensions a clone drop leaves out (criterion 6), at most
    /// 70 characters; `None` when there are none.
    pub(super) fn left_out_notice(&self) -> Option<String> {
        let (guides, dimensions) = self.left_out;
        let noun = match (guides, dimensions) {
            (0, 0) => return None,
            (1, 0) => "1 guide was".to_owned(),
            (n, 0) => format!("{n} guides were"),
            (0, 1) => "1 dimension was".to_owned(),
            (0, n) => format!("{n} dimensions were"),
            (g, d) => format!("{} guides and dimensions were", g + d),
        };
        Some(format!("{noun} not cloned. Guides never run."))
    }
}

/// The session's state of the cloning drop: the drop in flight and the notice of the last one.
#[derive(Debug, Default)]
pub(super) struct CloneDropState {
    drop: Option<CloneDrop>,
    notice: Option<String>,
    /// A Select drag Escape cancelled while the button was down: its release changes nothing,
    /// the active page included (`specs/0100` UX review).
    pub(super) escaped: bool,
}

impl Session {
    /// A Select move began: it may end as a cloning drop.
    pub(super) fn clone_drop_begin(&mut self) {
        if !self.select.is_moving() {
            self.clone_drops.drop = None;
            return;
        }
        let read = self.document_read();
        let pages = self.document.pages();
        let membership = self.page_membership();
        let on_laser = read
            .leaves
            .iter()
            .filter(|o| self.selection.ids().contains(&o.id()))
            .any(|o| {
                let bounds = object_bounds(o);
                pages.iter().any(|page| {
                    matches!(page.kind, PageKind::Output { .. })
                        && membership.is_member(page, o.id(), bounds, Reach::Touches)
                })
            });
        let units = self.units().selected(self.selection.ids());
        self.clone_drops.drop = Some(CloneDrop::begin(
            &units,
            &read.leaves,
            &read.guides,
            on_laser,
        ));
    }

    /// One frame of the move at `point` with the held modifiers.
    pub(super) fn clone_drop_update(&mut self, point: Point) {
        // The move is known once the press left the dead zone.
        if self.clone_drops.drop.is_none() && self.select.is_moving() {
            self.clone_drop_begin();
        }
        let pages = self.document.pages();
        let under = pages.iter().rev().find(|p| p.contains(point));
        let under = under.map(|page| {
            let on = match &page.kind {
                PageKind::Output { process, .. } => self.process_enabled(process),
                _ => false,
            };
            (page, on)
        });
        let held = self.held;
        if let Some(drop) = self.clone_drops.drop.as_mut() {
            drop.update(under, held);
        }
    }

    /// The answer of the move in flight, `None` while no cloning drop is possible.
    #[must_use]
    pub fn clone_drop_answer(&self) -> Option<RowDragAnswer> {
        self.clone_drops.drop.as_ref().map(CloneDrop::answer)
    }

    /// What a release of the move in flight does now.
    pub(super) fn clone_drop_mode(&self) -> DropMode {
        self.clone_drops
            .drop
            .as_ref()
            .map_or(DropMode::Plain, CloneDrop::mode)
    }

    /// Escape or a cancel: the drop is gone, nothing is written.
    pub(super) fn clone_drop_cancel(&mut self) {
        self.clone_drops.drop = None;
    }

    /// The notice of the last cloning drop, once.
    pub fn take_clone_drop_notice(&mut self) -> String {
        self.clone_drops.notice.take().unwrap_or_default()
    }

    /// The release of the move at `point` (`shift`, `ctrl` and the held Alt are the release's
    /// flags): a Linked clone drop places the clones in one step and returns `true`; a Move or
    /// Copy drop over a laser page (and a plain move) is left to the Select tool, the page is
    /// activated afterwards by the caller through [`Session::clone_drop_target`]. `false` when
    /// the release is not a Linked clone drop.
    pub(super) fn clone_drop_release(&mut self, point: Point, snap_offset: Vec2) -> bool {
        self.clone_drop_update(point);
        let Some(drop) = self.clone_drops.drop.clone() else {
            return false;
        };
        if drop.mode() == DropMode::Nothing {
            self.clone_drops.drop = None;
            let units = self.units();
            let _ = self.select.end_move_uncommitted(
                &mut self.selection,
                &units,
                point,
                Modifiers::NONE,
                snap_offset,
            );
            return true;
        }
        if drop.mode() != DropMode::Clone {
            return false;
        }
        self.clone_drops.drop = None;
        let modifiers = Modifiers::new(self.held.shift, false).with_alt(self.held.alt);
        let units = self.units();
        let Some(offset) = self.select.end_move_uncommitted(
            &mut self.selection,
            &units,
            point,
            modifiers,
            snap_offset,
        ) else {
            return true;
        };
        // The clones go on top of the active layer, outside any entered group; undo leaves an
        // empty selection (criterion 1 c, e).
        self.leave_to_level(0);
        self.document.set_draw_parent(self.draw_parent(None));
        match self.document.place_linked_clones(&drop.originals, offset) {
            Ok(clones) => {
                if let Some(page) = &drop.target {
                    self.activate_page(page);
                }
                self.node.clear_selection();
                self.selection.set(&clones);
                self.hovered_object = None;
                self.clone_drops.notice = drop.left_out_notice();
            }
            Err(curvyo_document_core::CloneError::TooDeep(offenders)) => {
                self.command_refusal = Some(super::refusal::RefusalMarks::new(
                    offenders,
                    &self.selection,
                ));
                self.clone_drops.notice = Some(super::clones::TOO_DEEP_NOTICE.to_owned());
            }
            Err(_) => {}
        }
        true
    }

    /// The laser page a Move or Copy drop over it activates, read before the release.
    pub(super) fn clone_drop_target(&self) -> Option<String> {
        let drop = self.clone_drops.drop.as_ref()?;
        matches!(drop.mode(), DropMode::Move | DropMode::Copy)
            .then(|| drop.target.clone())
            .flatten()
    }

    /// The ghost of a Linked clone drop: the selection's outlines at the pointer in the "Drag
    /// ghost" look, over the originals drawn where they started.
    pub(super) fn clone_drop_ghost_list(&self) -> curvyo_render_core::DrawList {
        if self.clone_drop_mode() != DropMode::Clone {
            return curvyo_render_core::DrawList::default();
        }
        let objects = self.objects();
        let Some(live) = self.select_live_edit_in(&objects) else {
            return curvyo_render_core::DrawList::default();
        };
        curvyo_render_core::build_drag_ghost(&live.objects, self.view())
    }
}

#[cfg(test)]
#[path = "clone_drop_tests.rs"]
mod session_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_drop_mode_table_of_criteria_1_to_5() {
        use DropMode::{Clone, Copy, Move, Plain};
        use TargetKind::{LaserPage, Other};
        let table = [
            // (selection on a laser page, target, alt, ctrl) -> mode
            ((false, LaserPage, false, false), Clone),
            ((false, LaserPage, true, false), Move),
            ((false, LaserPage, false, true), Copy),
            ((false, LaserPage, true, true), Copy),
            ((true, LaserPage, false, false), Plain),
            ((true, LaserPage, true, true), Plain),
            ((false, Other, false, false), Plain),
            ((false, Other, true, false), Plain),
            ((false, Other, false, true), Plain),
        ];
        for ((on, target, alt, ctrl), want) in table {
            assert_eq!(
                drop_mode(on, target, alt, ctrl),
                want,
                "{on} {target:?} {alt} {ctrl}"
            );
        }
    }

    #[test]
    fn the_left_out_notice_counts_guides_and_dimensions() {
        let drop = |guides, dimensions| CloneDrop {
            originals: Vec::new(),
            left_out: (guides, dimensions),
            on_laser: false,
            mode: DropMode::Plain,
            target: None,
        };
        assert_eq!(drop(0, 0).left_out_notice(), None);
        assert_eq!(
            drop(2, 0).left_out_notice().unwrap(),
            "2 guides were not cloned. Guides never run."
        );
        assert_eq!(
            drop(1, 0).left_out_notice().unwrap(),
            "1 guide was not cloned. Guides never run."
        );
        assert_eq!(
            drop(2, 1).left_out_notice().unwrap(),
            "3 guides and dimensions were not cloned. Guides never run."
        );
        assert!(drop(99, 99).left_out_notice().unwrap().len() <= 70);
    }
}
