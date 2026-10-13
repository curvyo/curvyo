//! Computing an offset of the selected objects (`specs/0038-path-offset` criteria 20 to 23, 29 to
//! 32, 40 to 43): which objects are operands, one result per operand and side (a primitive where
//! it stays one, otherwise the curve kernel's path), why the whole offset is refused, and the
//! writes that put the results into the document.
//!
//! Pure: it reads snapshots and the selection, so the preview draws exactly what the release or
//! Apply writes; [`OffsetPlan::into_writes`] mints the anchor ids only for the write. Groups are
//! refused by the session, which knows them.

use curvyo_document_core::{
    AnchorKind, Angle, Length, NewAnchor, NodeId, ObjectSnapshot, OffsetGeometry, OffsetWrite,
    PieceOutlines, Shape, Tolerance,
};
use curvyo_geometry_core::{
    OffsetAnchor, OffsetError, OffsetOutlines, Outline, OutlineTriple, offset,
    offset_distance_is_zero,
};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::{is_open, operands_in_order, outlines_of};
use crate::object_selection::ObjectSelection;
use crate::offset_entry::{OffsetCap, OffsetJoin, OffsetRole, OffsetSide, OffsetSides};
use crate::offset_primitive::{PrimitiveOffset, offset_primitive};

/// The kernel tolerance (criterion 34).
const OFFSET_TOLERANCE: Tolerance = Tolerance::from_mm(0.01);

/// What an offset is asked to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetRequest {
    /// The unsigned distance in millimetres.
    pub distance_mm: f64,
    /// The sides written and the role of each.
    pub sides: OffsetSides,
    /// The corner join.
    pub join: OffsetJoin,
    /// The end cap of open paths.
    pub cap: OffsetCap,
}

/// Why an offset was refused. Nothing is written in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OffsetRefusal {
    /// No operand is selected.
    NoOperands,
    /// The distance is zero: nothing is written.
    ZeroDistance,
    /// Only an inset is asked for and every operand is an open path (criterion 30).
    InsetOfOpenPaths {
        /// The open paths, to outline.
        offenders: Vec<NodeId>,
        /// How many objects are selected.
        of: usize,
    },
    /// `offenders` of the `of` selected objects, or their results, have a coordinate the kernel
    /// cannot work with (criterion 32).
    OutOfRange {
        /// The objects to outline.
        offenders: Vec<NodeId>,
        /// How many objects are selected.
        of: usize,
    },
    /// `offenders` of the `of` selected objects are closed and enclose no area (criterion 32).
    NoArea {
        /// The objects to outline.
        offenders: Vec<NodeId>,
        /// How many objects are selected.
        of: usize,
    },
    /// Every result vanished (criterion 29).
    Empty,
}

/// One result: the geometry of one side of one operand.
#[derive(Debug, Clone, PartialEq)]
pub enum OffsetShape {
    /// The operand's kind of primitive with these fields and rotation (criterion 40).
    Primitive {
        /// Kind and fields, in the primitive's own frame.
        shape: Shape,
        /// The operand's rotation.
        rotation: Angle,
    },
    /// Closed outlines of lines and cubics, rotation 0.
    Path(Vec<Vec<OffsetAnchor>>),
}

/// One side of one operand and what becomes of it.
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetItem {
    /// The operand.
    pub operand: NodeId,
    /// Outset or inset.
    pub side: OffsetSide,
    /// A new object or the changed original.
    pub role: OffsetRole,
    /// The result.
    pub shape: OffsetShape,
}

/// Whether the results of primitive operands stay primitives (the bar's "Stays a rectangle." or
/// "Becomes a path.", criterion 15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindNote {
    /// Every operand is this kind of primitive and every result stays one.
    Stays(PrimitiveKind),
    /// Every operand is a primitive and some result becomes a path.
    BecomesPath,
}

/// The kinds of primitives, for [`KindNote`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveKind {
    /// A rectangle.
    Rectangle,
    /// A circle.
    Circle,
    /// An ellipse.
    Ellipse,
    /// A polygon.
    Polygon,
    /// A star.
    Star,
}

/// The computed offset: every result, bottom-most operand first.
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetPlan {
    /// The results, per operand outer side first.
    pub items: Vec<OffsetItem>,
    /// Operands whose inset vanished (criterion 29, "1 of 3 shapes vanished").
    pub vanished: usize,
    /// Open paths left out of an inset (criterion 30).
    pub open_left_out: Vec<NodeId>,
    /// How many operands there were.
    pub of: usize,
    /// Stays a primitive or becomes a path, for an all-primitive selection.
    pub kind_note: Option<KindNote>,
}

impl OffsetPlan {
    /// How many new objects the plan adds.
    #[must_use]
    pub fn added(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.role == OffsetRole::Copy)
            .count()
    }

    /// How many originals the plan changes.
    #[must_use]
    pub fn changed(&self) -> usize {
        self.items.len() - self.added()
    }

    /// Every outline of every result as anchors, for the preview.
    #[must_use]
    pub fn preview_outlines(&self) -> Vec<Vec<OutlineTriple>> {
        self.items
            .iter()
            .flat_map(|item| match &item.shape {
                OffsetShape::Primitive { shape, rotation } => {
                    vec![
                        curvyo_document_core::outline_of_rotated(shape, *rotation)
                            .iter()
                            .map(|a| (a.point, a.handle_in, a.handle_out))
                            .collect(),
                    ]
                }
                OffsetShape::Path(outlines) => outlines
                    .iter()
                    .map(|outline| {
                        outline
                            .iter()
                            .map(|a| (a.point, a.handle_in, a.handle_out))
                            .collect()
                    })
                    .collect(),
            })
            .collect()
    }

    /// The writes for `Document::write_offset`: an outset copy below its operand, an inset copy
    /// above, a changed original in place; paths get fresh anchor ids.
    #[must_use]
    pub fn into_writes(self, minter: &mut AnchorIdMinter) -> Vec<OffsetWrite> {
        self.items
            .into_iter()
            .map(|item| {
                let geometry = match item.shape {
                    OffsetShape::Primitive { shape, rotation } => {
                        OffsetGeometry::Primitive { shape, rotation }
                    }
                    OffsetShape::Path(outlines) => {
                        OffsetGeometry::Path(path_outlines(&outlines, minter))
                    }
                };
                match item.role {
                    OffsetRole::Copy => OffsetWrite::Add {
                        source: item.operand,
                        below: item.side == OffsetSide::Out,
                        geometry,
                    },
                    OffsetRole::Original => OffsetWrite::Change {
                        target: item.operand,
                        geometry,
                    },
                }
            })
            .collect()
    }
}

/// Closed outlines with fresh anchor ids; a smooth node is asymmetric (tangent, own lengths).
fn path_outlines(outlines: &[Vec<OffsetAnchor>], minter: &mut AnchorIdMinter) -> PieceOutlines {
    outlines
        .iter()
        .map(|outline| {
            let anchors = outline
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
        .collect()
}

/// The operands of `selection` in stacking order: the objects the Offset tool works on.
#[must_use]
pub fn offset_operands<'a>(
    objects: &'a [ObjectSnapshot],
    selection: &ObjectSelection,
) -> Vec<&'a ObjectSnapshot> {
    operands_in_order(objects, selection)
}

/// Offsets every selected operand on its own (criterion 22: results are not merged), in stacking
/// order, so the selection order does not matter (criterion 44).
///
/// # Errors
/// An [`OffsetRefusal`]; the checks run in the order: no operand, zero distance, an inset of
/// open paths only, then per operand out of range and no area, then everything vanished.
pub fn plan_offset(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    request: &OffsetRequest,
) -> Result<OffsetPlan, OffsetRefusal> {
    let operands = operands_in_order(objects, selection);
    let of = operands.len();
    if operands.is_empty() {
        return Err(OffsetRefusal::NoOperands);
    }
    if offset_distance_is_zero(Length::from_mm(request.distance_mm)) {
        return Err(OffsetRefusal::ZeroDistance);
    }
    let inset_only = request.sides.out.is_none();
    let open: Vec<NodeId> = operands
        .iter()
        .filter(|object| is_open(object))
        .map(|object| object.id())
        .collect();
    if inset_only && open.len() == of {
        return Err(OffsetRefusal::InsetOfOpenPaths {
            offenders: open,
            of,
        });
    }
    let mut results = Results::default();
    for object in &operands {
        results.add(object, request);
    }
    let Results {
        items,
        vanished,
        out_of_range,
        no_area,
    } = results;
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
    if items.is_empty() {
        return Err(OffsetRefusal::Empty);
    }
    let kind_note = kind_note(&operands, &items);
    Ok(OffsetPlan {
        items,
        vanished,
        open_left_out: if inset_only { open } else { Vec::new() },
        of,
        kind_note,
    })
}

/// The results of the operands, collected one operand after the other.
#[derive(Default)]
struct Results {
    items: Vec<OffsetItem>,
    vanished: usize,
    out_of_range: Vec<NodeId>,
    no_area: Vec<NodeId>,
}

impl Results {
    /// Offsets `object` on every side `request` asks for. An open path has no inside: its Both is
    /// its Out, its In is left out.
    fn add(&mut self, object: &ObjectSnapshot, request: &OffsetRequest) {
        let id = object.id();
        let sides = [
            (OffsetSide::Out, request.sides.out),
            (OffsetSide::In, request.sides.inside),
        ];
        for (side, role) in sides {
            let Some(role) = role else { continue };
            if side == OffsetSide::In && is_open(object) {
                continue;
            }
            let d = match side {
                OffsetSide::Out => request.distance_mm,
                OffsetSide::In => -request.distance_mm,
            };
            match offset_one(object, d, request) {
                Ok(shape) => self.items.push(OffsetItem {
                    operand: id,
                    side,
                    role,
                    shape,
                }),
                Err(OffsetError::Empty) => self.vanished += 1,
                Err(OffsetError::NoArea) => {
                    if !self.no_area.contains(&id) {
                        self.no_area.push(id);
                    }
                }
                Err(_) => {
                    if !self.out_of_range.contains(&id) {
                        self.out_of_range.push(id);
                    }
                }
            }
        }
    }
}

/// One side of one operand: a primitive's own offset where it stays one, otherwise the kernel.
fn offset_one(
    object: &ObjectSnapshot,
    d: f64,
    request: &OffsetRequest,
) -> Result<OffsetShape, OffsetError> {
    if let ObjectSnapshot::Primitive(primitive) = object {
        match offset_primitive(&primitive.shape, Length::from_mm(d), request.join) {
            PrimitiveOffset::Stays(shape) => {
                return Ok(OffsetShape::Primitive {
                    shape,
                    rotation: primitive.rotation,
                });
            }
            PrimitiveOffset::Vanishes => return Err(OffsetError::Empty),
            PrimitiveOffset::Path => {}
        }
    }
    let owned: Vec<(Vec<OutlineTriple>, bool)> = outlines_of(object);
    let outlines: Vec<Outline<'_>> = owned
        .iter()
        .map(|(anchors, closed)| Outline::new(anchors, *closed))
        .collect();
    offset(
        &outlines,
        Length::from_mm(d),
        request.join,
        request.cap,
        OFFSET_TOLERANCE,
    )
    .map(|result| OffsetShape::Path(OffsetOutlines::into_outlines(result)))
}

/// The bar's note for a selection of primitives of one kind.
fn kind_note(operands: &[&ObjectSnapshot], items: &[OffsetItem]) -> Option<KindNote> {
    let mut kind = None;
    for object in operands {
        let ObjectSnapshot::Primitive(primitive) = object else {
            return None;
        };
        let this = primitive_kind(&primitive.shape);
        if kind.is_some_and(|k| k != this) {
            return None;
        }
        kind = Some(this);
    }
    let all_stay = items
        .iter()
        .all(|item| matches!(item.shape, OffsetShape::Primitive { .. }));
    kind.map(|kind| {
        if all_stay {
            KindNote::Stays(kind)
        } else {
            KindNote::BecomesPath
        }
    })
}

fn primitive_kind(shape: &Shape) -> PrimitiveKind {
    match shape {
        Shape::Rect { .. } => PrimitiveKind::Rectangle,
        Shape::Ellipse { frame } if (frame.rx.as_mm() - frame.ry.as_mm()).abs() <= 1e-9 => {
            PrimitiveKind::Circle
        }
        Shape::Ellipse { .. } => PrimitiveKind::Ellipse,
        Shape::Polygon { .. } => PrimitiveKind::Polygon,
        Shape::Star { .. } => PrimitiveKind::Star,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{AnchorId, Document, EllipseFrame, Point, RectBounds};

    fn rect(document: &Document, x: f64, side: f64) -> NodeId {
        document.create_rect(RectBounds {
            origin: Point::new(x, 0.0),
            width: Length::from_mm(side),
            height: Length::from_mm(side),
        })
    }

    fn square_path(document: &Document, x: f64, side: f64, first: u64) -> NodeId {
        let corners = [(x, 0.0), (x + side, 0.0), (x + side, side), (x, side)];
        let anchors: Vec<NewAnchor> = corners
            .into_iter()
            .zip(first..)
            .map(|((cx, cy), n)| NewAnchor::corner(AnchorId::new(1, n), Point::new(cx, cy)))
            .collect();
        document.create_path(&anchors, true)
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

    fn request(distance_mm: f64, sides: OffsetSides) -> OffsetRequest {
        OffsetRequest {
            distance_mm,
            sides,
            join: OffsetJoin::Round,
            cap: OffsetCap::Round,
        }
    }

    const OUT_COPY: OffsetSides = OffsetSides {
        out: Some(OffsetRole::Copy),
        inside: None,
    };
    const IN_COPY: OffsetSides = OffsetSides {
        out: None,
        inside: Some(OffsetRole::Copy),
    };
    const BOTH: OffsetSides = OffsetSides {
        out: Some(OffsetRole::Copy),
        inside: Some(OffsetRole::Copy),
    };

    #[test]
    fn ac22_every_operand_gets_its_own_result_in_stacking_order() {
        let document = Document::new(1);
        let a = square_path(&document, 0.0, 10.0, 1);
        let b = square_path(&document, 5.0, 10.0, 10);
        let all = objects(&document);
        let forward = plan_offset(&all, &selection_of(&[a, b]), &request(2.0, OUT_COPY)).unwrap();
        let backward = plan_offset(&all, &selection_of(&[b, a]), &request(2.0, OUT_COPY)).unwrap();
        assert_eq!(forward, backward, "the selection order does not matter");
        assert_eq!(forward.items.len(), 2, "overlapping results are not merged");
        assert_eq!(forward.items[0].operand, a);
        assert!(matches!(forward.items[0].shape, OffsetShape::Path(_)));
        assert_eq!(forward.added(), 2);
    }

    #[test]
    fn ac40_ac43_primitives_stay_primitives_on_both_sides() {
        let document = Document::new(1);
        let r = rect(&document, 0.0, 20.0);
        let all = objects(&document);
        let plan = plan_offset(&all, &selection_of(&[r]), &request(2.0, BOTH)).unwrap();
        assert_eq!(plan.items.len(), 2);
        assert!(
            plan.items
                .iter()
                .all(|item| matches!(item.shape, OffsetShape::Primitive { .. }))
        );
        assert_eq!(
            plan.kind_note,
            Some(KindNote::Stays(PrimitiveKind::Rectangle))
        );
        let oval = document.create_ellipse(EllipseFrame {
            center: Point::new(50.0, 50.0),
            rx: Length::from_mm(20.0),
            ry: Length::from_mm(10.0),
        });
        let all = objects(&document);
        let plan = plan_offset(&all, &selection_of(&[oval]), &request(2.0, OUT_COPY)).unwrap();
        assert!(matches!(plan.items[0].shape, OffsetShape::Path(_)));
        assert_eq!(plan.kind_note, Some(KindNote::BecomesPath));
    }

    #[test]
    fn ac20_ac23_writes_place_copies_and_change_originals() {
        let document = Document::new(1);
        let r = square_path(&document, 0.0, 20.0, 1);
        let all = objects(&document);
        let sides = OffsetSides {
            out: Some(OffsetRole::Original),
            inside: Some(OffsetRole::Copy),
        };
        let plan = plan_offset(&all, &selection_of(&[r]), &request(2.0, sides)).unwrap();
        assert_eq!((plan.added(), plan.changed()), (1, 1));
        let writes = plan.into_writes(&mut AnchorIdMinter::new(7));
        assert!(matches!(writes[0], OffsetWrite::Change { target, .. } if target == r));
        assert!(matches!(writes[1], OffsetWrite::Add { below: false, .. }));
    }

    #[test]
    fn ac29_vanishing_operands_are_counted_and_all_vanishing_is_refused() {
        let document = Document::new(1);
        let small = square_path(&document, 0.0, 4.0, 1);
        let big = square_path(&document, 10.0, 40.0, 10);
        let all = objects(&document);
        let partly =
            plan_offset(&all, &selection_of(&[small, big]), &request(3.0, IN_COPY)).unwrap();
        assert_eq!((partly.vanished, partly.of, partly.items.len()), (1, 2, 1));
        assert_eq!(
            plan_offset(&all, &selection_of(&[small, big]), &request(30.0, IN_COPY)),
            Err(OffsetRefusal::Empty)
        );
    }

    #[test]
    fn ac30_open_paths_grow_and_are_left_out_of_an_inset() {
        let document = Document::new(1);
        let a = square_path(&document, 0.0, 10.0, 1);
        let open = line(&document, 20);
        let all = objects(&document);
        let grown = plan_offset(&all, &selection_of(&[open]), &request(5.0, BOTH)).unwrap();
        assert_eq!(grown.items.len(), 1, "Both on an open path is its Out");
        assert_eq!(
            plan_offset(&all, &selection_of(&[open]), &request(1.0, IN_COPY)),
            Err(OffsetRefusal::InsetOfOpenPaths {
                offenders: vec![open],
                of: 1
            })
        );
        let mixed = plan_offset(&all, &selection_of(&[a, open]), &request(1.0, IN_COPY)).unwrap();
        assert_eq!(mixed.items.len(), 1);
        assert_eq!(mixed.open_left_out, vec![open]);
    }

    #[test]
    fn ac32_zero_no_operand_and_no_area_are_refused() {
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
            plan_offset(&all, &selection_of(&[a]), &request(0.0, OUT_COPY)),
            Err(OffsetRefusal::ZeroDistance)
        );
        assert_eq!(
            plan_offset(&all, &ObjectSelection::new(), &request(1.0, OUT_COPY)),
            Err(OffsetRefusal::NoOperands)
        );
        assert_eq!(
            plan_offset(&all, &selection_of(&[a, flat]), &request(1.0, OUT_COPY)),
            Err(OffsetRefusal::NoArea {
                offenders: vec![flat],
                of: 2
            })
        );
    }
}
