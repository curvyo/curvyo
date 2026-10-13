//! The session's Offset entry (`specs/0038-path-offset`): opening and closing it, the remembered
//! values, the preview it shows and the one write of Apply.
//!
//! The entry belongs to the selection and the tool it was opened for: once either changes it is
//! closed (criterion 4). The preview is computed when a value changes and again whenever the
//! document has changed under it, and Apply writes the preview's own result, so what is written
//! is what was last shown (criterion 3).

use std::cell::RefCell;

use curvyo_document_core::{DocumentVersion, NodeId, Point};
use curvyo_ui_core::{
    OffsetAvailability, OffsetCap, OffsetJoin, OffsetRefusal, OffsetResult, OffsetSettings,
    compute_offset, offset_availability,
};

use super::refusal::RefusalMarks;
use super::{Session, Tool};

/// The label of the commit Apply makes (criterion 13).
pub const OFFSET_COMMIT_LABEL: &str = "offset_path";

/// The open entry: the selection it was opened for and its last preview.
#[derive(Debug, Default)]
pub(super) struct OffsetEntry {
    open_for: Option<Vec<NodeId>>,
    preview: RefCell<Option<Preview>>,
}

/// One computed preview and what it was computed from.
#[derive(Debug, Clone)]
struct Preview {
    version: DocumentVersion,
    settings: OffsetSettings,
    result: Result<OffsetResult, OffsetRefusal>,
}

/// What the entry's caption and the preview show now.
#[derive(Debug, Clone, PartialEq)]
pub enum OffsetPreviewState {
    /// The entry is closed.
    Closed,
    /// A result is previewed: `vanished` of the `of` operands are gone in it.
    Result {
        /// How many operands vanished.
        vanished: usize,
        /// How many operands there are.
        of: usize,
    },
    /// Apply would be refused, or the distance is zero.
    Refused(OffsetRefusal),
    /// Apply would be refused: the selection holds a group (`0023` criterion 28).
    RefusedGroups {
        /// How many leaves the selected groups hold.
        leaves: usize,
    },
}

/// What [`Session::apply_offset`] did.
#[derive(Debug, Clone, PartialEq)]
pub enum OffsetOutcome {
    /// Nothing: the entry is not open, a drag is in flight or the write could not be made.
    Ignored,
    /// The distance is zero: the entry closed and nothing was written (criterion 5).
    ClosedAtZero,
    /// The results were added in one commit and selected; the entry closed.
    Applied {
        /// How many shapes were added.
        added: usize,
        /// How many operands vanished.
        vanished: usize,
        /// How many operands there were.
        of: usize,
        /// The signed distance in millimetres.
        distance_mm: f64,
    },
    /// Refused; nothing was written and the entry stays open. Offenders are outlined.
    Refused(OffsetRefusal),
    /// The selection holds a group (`0023` criterion 28); its leaves are outlined.
    RefusedGroups {
        /// How many leaves the selected groups hold.
        leaves: usize,
    },
}

impl Session {
    /// What the Offset button and section show: nothing is offered with another tool than
    /// Select (criterion 1).
    #[must_use]
    pub fn offset_availability(&self) -> OffsetAvailability {
        offset_availability(&self.objects(), &self.offset_selection())
    }

    /// The selection without the leaves of selected groups: a group is no operand, so a group
    /// alone offers nothing (criterion 1, `0023` criterion 28).
    fn offset_selection(&self) -> curvyo_ui_core::ObjectSelection {
        let in_groups = self.selected_group_leaves();
        let mut selection = self.boolean_selection();
        if !in_groups.is_empty() {
            let ids: Vec<NodeId> = selection
                .ids()
                .iter()
                .copied()
                .filter(|id| !in_groups.contains(id))
                .collect();
            selection.set(&ids);
        }
        selection
    }

    /// The values the entry shows, remembered for the session.
    #[must_use]
    pub fn offset_settings(&self) -> OffsetSettings {
        self.offset_settings
    }

    /// Whether the entry is open: it was opened for this selection, the Select tool is still
    /// active, and nothing has closed it.
    #[must_use]
    pub fn offset_open(&self) -> bool {
        self.offset
            .open_for
            .as_ref()
            .is_some_and(|ids| self.tool == Tool::Select && self.selection.ids() == ids.as_slice())
    }

    /// Opens the entry for the current selection and computes its first preview with the
    /// remembered values. Returns whether it opened (it does not without an operand, with another
    /// tool than Select, or during a drag or an open typed entry).
    pub fn open_offset(&mut self) -> bool {
        if !self.rail_command_ready() || !self.offset_availability().offered {
            return false;
        }
        self.flush_select_bar_preview();
        self.end_colour_pick();
        self.command_refusal = None;
        self.offset.open_for = Some(self.selection.ids().to_vec());
        self.offset.preview.replace(None);
        true
    }

    /// Closes the entry; nothing is written (criterion 4).
    pub fn close_offset(&mut self) {
        self.offset.open_for = None;
        self.offset.preview.replace(None);
    }

    /// Sets the distance (criterion 1's typed range); returns whether it was taken. The preview
    /// follows on the next read.
    pub fn set_offset_distance(&mut self, distance_mm: f64) -> bool {
        self.offset_settings.set_distance_mm(distance_mm)
    }

    /// Sets the join.
    pub fn set_offset_join(&mut self, join: OffsetJoin) {
        self.offset_settings.set_join(join);
    }

    /// Sets the cap.
    pub fn set_offset_cap(&mut self, cap: OffsetCap) {
        self.offset_settings.set_cap(cap);
    }

    /// What the caption shows: the previewed result or why Apply would be refused.
    #[must_use]
    pub fn offset_preview_state(&self) -> OffsetPreviewState {
        if !self.offset_open() {
            return OffsetPreviewState::Closed;
        }
        let leaves = self.selected_group_leaves().len();
        if leaves > 0 {
            return OffsetPreviewState::RefusedGroups { leaves };
        }
        match self.current_offset_preview().result {
            Ok(result) => OffsetPreviewState::Result {
                vanished: result.vanished,
                of: result.of,
            },
            Err(refusal) => OffsetPreviewState::Refused(refusal),
        }
    }

    /// The outlines the preview draws (criterion 2): every outline of every result, empty when
    /// the entry is closed or Apply would be refused.
    #[must_use]
    pub(super) fn offset_preview_outlines(&self) -> Vec<Vec<Point>> {
        // Apply refuses a selection with a group, so nothing is previewed (criterion 2).
        if !self.offset_open() || !self.selected_group_leaves().is_empty() {
            return Vec::new();
        }
        match self.current_offset_preview().result {
            Ok(result) => result
                .pieces
                .iter()
                .flat_map(|(_, outlines)| outlines)
                .map(|outline| {
                    outline
                        .iter()
                        .map(|anchor| anchor.point)
                        .collect()
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// The blue hollow outline of the result over the unchanged originals (criterion 2).
    pub(super) fn offset_preview_list(
        &self,
        view: curvyo_document_core::ViewTransform,
    ) -> curvyo_render_core::DrawList {
        let outlines = self.offset_preview_outlines();
        if outlines.is_empty() {
            return curvyo_render_core::DrawList::default();
        }
        curvyo_render_core::build_offset_preview(&outlines, view)
    }

    /// The preview for the current document and values, computed if the cached one is stale.
    fn current_offset_preview(&self) -> Preview {
        let version = self.document.version();
        if let Some(preview) = self.offset.preview.borrow().as_ref()
            && preview.version == version
            && preview.settings == self.offset_settings
        {
            return preview.clone();
        }
        let preview = Preview {
            version,
            settings: self.offset_settings,
            result: compute_offset(
                &self.objects(),
                &self.boolean_selection(),
                &self.offset_settings,
            ),
        };
        self.offset.preview.replace(Some(preview.clone()));
        preview
    }

    /// Applies the previewed result (criteria 5, 10, 13): one commit labelled `offset_path` that
    /// adds every result directly above its operand, then selects the results and closes the
    /// entry. A refusal writes nothing, outlines its offenders and leaves the entry open.
    pub fn apply_offset(&mut self) -> OffsetOutcome {
        self.recording(|this| -> OffsetOutcome {
            if !this.offset_open() || !this.rail_command_ready() {
                return OffsetOutcome::Ignored;
            }
            let selection = this.boolean_selection();
            let in_groups = this.selected_group_leaves();
            if !in_groups.is_empty() {
                let leaves = in_groups.len();
                this.command_refusal = Some(RefusalMarks::new(in_groups, &selection));
                return OffsetOutcome::RefusedGroups { leaves };
            }
            let distance_mm = this.offset_settings.distance_mm();
            let result = match this.current_offset_preview().result {
                Ok(result) => result,
                Err(OffsetRefusal::ZeroDistance) => {
                    this.close_offset();
                    return OffsetOutcome::ClosedAtZero;
                }
                Err(refusal) => {
                    this.command_refusal = match &refusal {
                        OffsetRefusal::InsetOfOpenPaths { offenders, .. }
                        | OffsetRefusal::OutOfRange { offenders, .. }
                        | OffsetRefusal::NoArea { offenders, .. } => {
                            Some(RefusalMarks::new(offenders.clone(), &selection))
                        }
                        OffsetRefusal::NoOperands
                        | OffsetRefusal::ZeroDistance
                        | OffsetRefusal::Empty => None,
                    };
                    return OffsetOutcome::Refused(refusal);
                }
            };
            let (vanished, of) = (result.vanished, result.of);
            let pieces = result.into_pieces(&mut this.minter);
            let Ok(added) = this.document.add_paths_above(&pieces, OFFSET_COMMIT_LABEL) else {
                // invariant: the result was computed from the objects of this document version,
                // each with an outline and fresh anchor ids, so the write cannot fail.
                debug_assert!(false, "a computed offset could not be written");
                return OffsetOutcome::Ignored;
            };
            this.command_refusal = None;
            this.selection.set(&added);
            this.hovered_object = None;
            this.close_offset();
            OffsetOutcome::Applied {
                added: added.len(),
                vanished,
                of,
                distance_mm,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{AnchorId, Length, NewAnchor, ObjectSnapshot, RectBounds};

    fn rect(session: &Session, x: f64, side: f64) -> NodeId {
        session.document.create_rect(RectBounds {
            origin: Point::new(x, 0.0),
            width: Length::from_mm(side),
            height: Length::from_mm(side),
        })
    }

    fn session_with_square() -> (Session, NodeId) {
        let mut session = Session::new(1);
        let square = rect(&session, 0.0, 20.0);
        session.selection.select_single(square);
        session.set_tool(Tool::Select);
        (session, square)
    }

    #[test]
    fn opening_shows_the_preview_with_the_remembered_values() {
        let (mut session, _) = session_with_square();
        assert!(session.offset_availability().offered);
        assert!(session.open_offset());
        assert!(session.offset_open());
        assert_eq!(
            session.offset_preview_state(),
            OffsetPreviewState::Result { vanished: 0, of: 1 }
        );
        assert_eq!(session.offset_preview_outlines().len(), 1);
        assert!(session.set_offset_distance(-4.0));
        session.close_offset();
        assert!(session.open_offset());
        assert!((session.offset_settings().distance_mm() + 4.0).abs() < 1e-12);
    }

    #[test]
    fn apply_adds_the_result_above_the_original_in_one_commit_and_selects_it() {
        let (mut session, square) = session_with_square();
        let other = rect(&session, 50.0, 10.0);
        session.open_offset();
        session.set_offset_distance(5.0);
        session.set_offset_join(OffsetJoin::Miter);
        let shown = session.offset_preview_outlines();
        let before = session.document.version();
        let outcome = session.apply_offset();
        assert_eq!(
            outcome,
            OffsetOutcome::Applied {
                added: 1,
                vanished: 0,
                of: 1,
                distance_mm: 5.0
            }
        );
        assert_ne!(session.document.version(), before);
        let ids = session.document.object_ids();
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[0], square, "the original is kept");
        assert_eq!(ids[2], other);
        assert_eq!(session.selection.ids(), &[ids[1]]);
        assert!(!session.offset_open());
        let Some(ObjectSnapshot::Path(result)) = session.document.object(ids[1]) else {
            panic!("a path");
        };
        let points: Vec<Point> = result.all_anchors().map(|a| a.point).collect();
        assert_eq!(vec![points], shown, "what is written is what was shown");
    }

    #[test]
    fn a_change_of_selection_or_tool_or_escape_closes_the_entry() {
        let (mut session, square) = session_with_square();
        session.open_offset();
        session.selection.clear();
        assert!(!session.offset_open());
        assert_eq!(session.offset_preview_state(), OffsetPreviewState::Closed);
        assert_eq!(session.offset_preview_outlines(), Vec::<Vec<Point>>::new());

        session.selection.select_single(square);
        session.open_offset();
        session.set_tool(Tool::Pen);
        assert!(!session.offset_open());

        session.set_tool(Tool::Select);
        session.selection.select_single(square);
        session.open_offset();
        assert_eq!(
            session.escape(),
            super::super::keys::EscapeStep::ClosedEntry
        );
        assert!(!session.offset_open());
        assert_eq!(session.selection.ids(), &[square], "the selection stays");
    }

    #[test]
    fn zero_closes_and_a_refusal_keeps_the_entry_open() {
        let (mut session, _) = session_with_square();
        session.open_offset();
        session.set_offset_distance(0.0);
        let before = session.document.object_ids();
        assert_eq!(session.apply_offset(), OffsetOutcome::ClosedAtZero);
        assert!(!session.offset_open());

        session.open_offset();
        session.set_offset_distance(-10.0);
        assert_eq!(
            session.offset_preview_state(),
            OffsetPreviewState::Refused(OffsetRefusal::Empty)
        );
        assert_eq!(
            session.apply_offset(),
            OffsetOutcome::Refused(OffsetRefusal::Empty)
        );
        assert!(session.offset_open());
        assert_eq!(session.document.object_ids(), before, "nothing written");
    }

    #[test]
    fn an_inset_of_an_open_path_outlines_it() {
        let mut session = Session::new(1);
        let line = session.document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, 2), Point::new(20.0, 0.0)),
            ],
            false,
        );
        session.selection.select_single(line);
        assert!(session.offset_availability().has_open);
        session.open_offset();
        session.set_offset_distance(-1.0);
        assert!(matches!(
            session.apply_offset(),
            OffsetOutcome::Refused(OffsetRefusal::InsetOfOpenPaths { .. })
        ));
        assert_eq!(session.refusal_objects(&session.objects()).len(), 1);
    }

    #[test]
    fn the_preview_follows_a_change_of_the_document() {
        let (mut session, square) = session_with_square();
        session.open_offset();
        assert_eq!(session.offset_preview_outlines().len(), 1);
        session.document.delete_objects(&[square]).unwrap();
        assert!(session.offset_open(), "the selection still names it");
        assert_eq!(
            session.offset_preview_state(),
            OffsetPreviewState::Refused(OffsetRefusal::NoOperands)
        );
        assert_eq!(session.offset_preview_outlines(), Vec::<Vec<Point>>::new());
    }
}
