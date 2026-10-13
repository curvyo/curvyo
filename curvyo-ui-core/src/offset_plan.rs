//! Computing an offset of the selected objects (`specs/0038-path-offset` criteria 1, 5 to 14):
//! which objects are operands, one kernel call per operand, and why the whole command is refused.
//!
//! Pure: it reads snapshots and the selection and returns the result outlines in millimetres, so
//! the preview draws exactly what Apply writes (criterion 3); [`OffsetResult::into_pieces`] mints
//! the anchor ids only for the write. Groups are refused by the session, which knows them.

use curvyo_document_core::{
    Length, NewAnchor, NodeId, ObjectSnapshot, PieceOutlines, Tolerance,
};
use curvyo_document_core::AnchorKind;
use curvyo_geometry_core::{
    OffsetAnchor, OffsetError, Outline, OutlineTriple, offset, offset_distance_is_zero,
};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::{is_open, operands_in_order, outlines_of};
use crate::object_selection::ObjectSelection;
use crate::offset_entry::OffsetSettings;

/// The kernel tolerance, as for the Boolean operations (criterion 11).
const OFFSET_TOLERANCE: Tolerance = Tolerance::from_mm(0.01);

/// What the Select bar and the Offset section show for the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetAvailability {
    /// At least one operand (a path or a primitive, not a group) is selected: the Offset button
    /// is shown.
    pub offered: bool,
    /// An open path is selected: the Cap choice is shown.
    pub has_open: bool,
}

/// What the Offset button and section show for `selection` (criterion 1). The caller passes an
/// empty selection when the Select tool is not active.
#[must_use]
pub fn offset_availability(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
) -> OffsetAvailability {
    let operands = operands_in_order(objects, selection);
    OffsetAvailability {
        offered: !operands.is_empty(),
        has_open: operands.iter().any(|object| is_open(object)),
    }
}

/// Why an offset was refused. Nothing is written in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OffsetRefusal {
    /// No operand is selected.
    NoOperands,
    /// The distance is zero: the entry closes and writes nothing (criterion 5).
    ZeroDistance,
    /// The distance is negative and `offenders` of the `of` selected objects are open paths
    /// (criterion 8).
    InsetOfOpenPaths {
        /// The open paths, to outline.
        offenders: Vec<NodeId>,
        /// How many objects are selected.
        of: usize,
    },
    /// `offenders` of the `of` selected objects, or their results, have a coordinate the kernel
    /// cannot work with (criterion 14).
    OutOfRange {
        /// The objects to outline.
        offenders: Vec<NodeId>,
        /// How many objects are selected.
        of: usize,
    },
    /// `offenders` of the `of` selected objects are closed and enclose no area (criterion 14).
    NoArea {
        /// The objects to outline.
        offenders: Vec<NodeId>,
        /// How many objects are selected.
        of: usize,
    },
    /// Every operand vanished under the inset (criterion 7).
    Empty,
}

/// The result of an offset: per surviving operand, its result outlines in millimetres, in the
/// kernel's canonical form.
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetResult {
    /// `(operand, outlines)`, bottom-most operand first.
    pub pieces: Vec<(NodeId, Vec<Vec<OffsetAnchor>>)>,
    /// How many operands vanished (criterion 7, "1 of 3 shapes vanished").
    pub vanished: usize,
    /// How many operands there were.
    pub of: usize,
}

impl OffsetResult {
    /// The pieces for `Document::add_paths_above`: closed corner outlines with fresh anchor ids.
    #[must_use]
    pub fn into_pieces(self, minter: &mut AnchorIdMinter) -> Vec<(NodeId, PieceOutlines)> {
        self.pieces
            .into_iter()
            .map(|(operand, outlines)| {
                let outlines = outlines
                    .iter()
                    .map(|points| {
                        let anchors = points
                            .iter()
                            .map(|anchor| NewAnchor {
                                id: minter.mint(),
                                point: anchor.point,
                                handle_in: anchor.handle_in,
                                handle_out: anchor.handle_out,
                                kind: if anchor.smooth {
                                    AnchorKind::Asymmetric
                                } else {
                                    AnchorKind::Corner
                                },
                            })
                            .collect();
                        (anchors, true)
                    })
                    .collect();
                (operand, outlines)
            })
            .collect()
    }
}

/// Offsets every selected operand by the settings, each on its own (criterion 10: results are
/// not merged), in stacking order, so the selection order does not matter (criterion 12).
///
/// # Errors
/// An [`OffsetRefusal`]; the checks run in the order: no operand, zero distance, inset of open
/// paths, then per operand out of range and no area, then everything vanished.
pub fn compute_offset(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    settings: &OffsetSettings,
) -> Result<OffsetResult, OffsetRefusal> {
    let operands = operands_in_order(objects, selection);
    let of = operands.len();
    if operands.is_empty() {
        return Err(OffsetRefusal::NoOperands);
    }
    let distance_mm = settings.distance_mm();
    if offset_distance_is_zero(Length::from_mm(distance_mm)) {
        return Err(OffsetRefusal::ZeroDistance);
    }
    if distance_mm < 0.0 {
        let open: Vec<NodeId> = operands
            .iter()
            .filter(|object| is_open(object))
            .map(|object| object.id())
            .collect();
        if !open.is_empty() {
            return Err(OffsetRefusal::InsetOfOpenPaths {
                offenders: open,
                of,
            });
        }
    }
    let mut pieces = Vec::with_capacity(of);
    let mut vanished = 0;
    let mut out_of_range = Vec::new();
    let mut no_area = Vec::new();
    for object in &operands {
        match offset_one(object, settings) {
            Ok(outlines) => pieces.push((object.id(), outlines)),
            Err(OffsetError::Empty) => vanished += 1,
            Err(OffsetError::NoArea) => no_area.push(object.id()),
            Err(
                OffsetError::OutOfRange
                | OffsetError::ZeroDistance
                | OffsetError::InsetOfOpenPath
                | OffsetError::ToleranceTooSmall,
            ) => out_of_range.push(object.id()),
        }
    }
    if !out_of_range.is_empty() {
        return Err(OffsetRefusal::OutOfRange {
            offenders: out_of_range,
            of,
        });
    }
    if !no_area.is_empty() {
        return Err(OffsetRefusal::NoArea {
            offenders: no_area,
            of,
        });
    }
    if pieces.is_empty() {
        return Err(OffsetRefusal::Empty);
    }
    Ok(OffsetResult {
        pieces,
        vanished,
        of,
    })
}

/// One kernel call. The zero distance and the open inset were refused before, so the kernel's
/// own refusals for them cannot come back; they are reported as out of range if they ever do.
fn offset_one(
    object: &ObjectSnapshot,
    settings: &OffsetSettings,
) -> Result<Vec<Vec<OffsetAnchor>>, OffsetError> {
    let owned: Vec<(Vec<OutlineTriple>, bool)> = outlines_of(object);
    let outlines: Vec<Outline<'_>> = owned
        .iter()
        .map(|(anchors, closed)| Outline::new(anchors, *closed))
        .collect();
    offset(
        &outlines,
        Length::from_mm(settings.distance_mm()),
        settings.join(),
        settings.cap(),
        OFFSET_TOLERANCE,
    )
    .map(curvyo_geometry_core::OffsetOutlines::into_outlines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{AnchorId, Document, Point, RectBounds};

    use crate::offset_entry::OffsetJoin;

    fn rect(document: &Document, x: f64, side: f64) -> NodeId {
        document.create_rect(RectBounds {
            origin: Point::new(x, 0.0),
            width: Length::from_mm(side),
            height: Length::from_mm(side),
        })
    }

    fn line(document: &Document, first: u64) -> NodeId {
        document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, first), Point::new(0.0, 50.0)),
                NewAnchor::corner(AnchorId::new(1, first + 1), Point::new(20.0, 50.0)),
            ],
            false,
        )
    }

    fn selection_of(ids: &[NodeId]) -> ObjectSelection {
        let mut selection = ObjectSelection::new();
        for &id in ids {
            selection.add(id);
        }
        selection
    }

    fn objects(document: &Document) -> Vec<ObjectSnapshot> {
        document
            .object_ids()
            .into_iter()
            .filter_map(|id| document.object(id))
            .collect()
    }

    fn settings(distance_mm: f64) -> OffsetSettings {
        let mut settings = OffsetSettings::default();
        assert!(settings.set_distance_mm(distance_mm));
        settings
    }

    #[test]
    fn availability_shows_the_button_for_operands_and_the_cap_for_open_paths() {
        let document = Document::new(1);
        let a = rect(&document, 0.0, 10.0);
        let open = line(&document, 1);
        let all = objects(&document);
        assert_eq!(
            offset_availability(&all, &ObjectSelection::new()),
            OffsetAvailability {
                offered: false,
                has_open: false
            }
        );
        assert_eq!(
            offset_availability(&all, &selection_of(&[a])),
            OffsetAvailability {
                offered: true,
                has_open: false
            }
        );
        assert_eq!(
            offset_availability(&all, &selection_of(&[a, open])),
            OffsetAvailability {
                offered: true,
                has_open: true
            }
        );
    }

    #[test]
    fn every_operand_gets_its_own_result_in_stacking_order() {
        let document = Document::new(1);
        let a = rect(&document, 0.0, 10.0);
        let b = rect(&document, 5.0, 10.0);
        let all = objects(&document);
        let forward = compute_offset(&all, &selection_of(&[a, b]), &settings(2.0)).unwrap();
        let backward = compute_offset(&all, &selection_of(&[b, a]), &settings(2.0)).unwrap();
        assert_eq!(forward, backward, "the selection order does not matter");
        assert_eq!(
            forward.pieces.len(),
            2,
            "overlapping results are not merged"
        );
        assert_eq!(forward.pieces[0].0, a);
        assert_eq!(forward.pieces[1].0, b);
        assert_eq!(forward.vanished, 0);
        let pieces = forward.into_pieces(&mut AnchorIdMinter::new(7));
        assert_eq!(pieces[0].1.len(), 1);
        assert!(pieces[0].1[0].1, "closed");
    }

    #[test]
    fn vanishing_operands_are_counted_and_all_vanishing_is_refused() {
        let document = Document::new(1);
        let small = rect(&document, 0.0, 4.0);
        let big = rect(&document, 10.0, 40.0);
        let all = objects(&document);
        let partly = compute_offset(&all, &selection_of(&[small, big]), &settings(-3.0)).unwrap();
        assert_eq!(partly.vanished, 1);
        assert_eq!(partly.of, 2);
        assert_eq!(partly.pieces.len(), 1);
        assert_eq!(partly.pieces[0].0, big);
        assert_eq!(
            compute_offset(&all, &selection_of(&[small, big]), &settings(-30.0)),
            Err(OffsetRefusal::Empty)
        );
    }

    #[test]
    fn an_open_path_grows_and_refuses_an_inset() {
        let document = Document::new(1);
        let a = rect(&document, 0.0, 10.0);
        let open = line(&document, 1);
        let all = objects(&document);
        let grown = compute_offset(&all, &selection_of(&[open]), &settings(5.0)).unwrap();
        assert_eq!(grown.pieces.len(), 1);
        assert_eq!(
            compute_offset(&all, &selection_of(&[a, open]), &settings(-1.0)),
            Err(OffsetRefusal::InsetOfOpenPaths {
                offenders: vec![open],
                of: 2
            })
        );
    }

    #[test]
    fn zero_no_operand_and_no_area_are_refused() {
        let document = Document::new(1);
        let a = rect(&document, 0.0, 10.0);
        let flat = document.create_path(
            &[
                NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, 2), Point::new(5.0, 0.0)),
                NewAnchor::corner(AnchorId::new(1, 3), Point::new(9.0, 0.0)),
            ],
            true,
        );
        let all = objects(&document);
        assert_eq!(
            compute_offset(&all, &selection_of(&[a]), &settings(0.0)),
            Err(OffsetRefusal::ZeroDistance)
        );
        assert_eq!(
            compute_offset(&all, &ObjectSelection::new(), &settings(1.0)),
            Err(OffsetRefusal::NoOperands)
        );
        assert_eq!(
            compute_offset(&all, &selection_of(&[a, flat]), &settings(1.0)),
            Err(OffsetRefusal::NoArea {
                offenders: vec![flat],
                of: 2
            })
        );
        let mut miter = settings(1.0);
        miter.set_join(OffsetJoin::Miter);
        let result = compute_offset(&all, &selection_of(&[a]), &miter).unwrap();
        assert_eq!(
            result.pieces[0].1[0].len(),
            4,
            "a mitred square has four corners"
        );
    }
}
