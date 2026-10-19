//! A primitive as a path (`specs/0047-transform-polish/` criteria 2, 3, 7 and 12;
//! `specs/0019-multi-object-transform/` `adrs.md` decision 8) and "Object to path"
//! (acceptance criteria 17, 21, 22): which shapes a skew or a stretch turns into paths,
//! the one function that builds the path, and the resolved anchor geometry a
//! [`curvyo_document_core::Document::convert_to_paths`] call needs,
//! for a whole primitive selection at once. Lives in this crate, not
//! `curvyo-editor-wasm`'s facade (architect review: ADR 0001 §1's
//! "the facade has no editing logic of its own" rule) — `Session` just
//! calls [`build_primitive_conversions`] and passes the result straight
//! to `Document::convert_to_paths`.

use curvyo_document_core::{
    AnchorSnapshot, Angle, Curve, Document, EllipseFrame, NewAnchor, NodeId, ObjectSnapshot,
    PathSnapshot, PrimitiveSnapshot, Shape, outline_of_rotated,
};

use crate::AnchorIdMinter;
use crate::transform_handle_layout::EditHandle;

/// A rotation within this (radians) of a multiple of 90 degrees is one.
const QUARTER_TURN_TOLERANCE_RAD: f64 = 1e-9;

/// The geometric tolerance (millimetres) of the group and conversion rules: two radii
/// closer than this make a circle, and an extent below it is "zero".
pub(crate) const GEOMETRIC_TOLERANCE_MM: f64 = 1e-6;

/// Whether `rotation` is a multiple of 90 degrees.
fn is_quarter_turn(rotation: Angle) -> bool {
    let quarters = rotation.as_radians() / std::f64::consts::FRAC_PI_2;
    (quarters - quarters.round()).abs() * std::f64::consts::FRAC_PI_2 <= QUARTER_TURN_TOLERANCE_RAD
}

/// Whether an ellipse frame is a circle: the one test the handles and the scale of
/// a group share, so that a circle never converts yet stretches in its own turned
/// frame.
pub(crate) fn is_circle(frame: &EllipseFrame) -> bool {
    (frame.rx.as_mm() - frame.ry.as_mm()).abs() < GEOMETRIC_TOLERANCE_MM
}

/// Whether a stretch along axes at `axes` turns `object` into a path
/// (`specs/0047-transform-polish/` criterion 7, `adrs.md` point 2: the one predicate that
/// decides). A path never converts; a polygon and a star always do (they store one
/// radius); a rectangle or ellipse does unless its `rotation` minus `axes` is a quarter
/// turn; a circle never does (it becomes an ellipse); an ellipse with an arc never does
/// and a curved one converts like a rectangle. A group passes `axes` 0; a single
/// object passes its own box angle, which a rectangle or ellipse shares, so it never
/// converts.
#[must_use]
pub(crate) fn converts_on_stretch(object: &ObjectSnapshot, axes: Angle) -> bool {
    let ObjectSnapshot::Primitive(primitive) = object else {
        return false;
    };
    let relative = Angle::from_radians(primitive.rotation.as_radians() - axes.as_radians());
    match &primitive.shape {
        Shape::Polygon { .. } | Shape::Star { .. } => true,
        Shape::Rect { .. } => !is_quarter_turn(relative),
        // An arc is the image of an arc under any linear map: it stays
        // (`specs/0021-ellipse-arcs-and-shaping/` criterion 56). A curved ellipse's ring is
        // not, unless the stretch runs along its own axes.
        Shape::Ellipse { arc: Some(_), .. } => false,
        Shape::Ellipse { curve, .. } if *curve != Curve::ELLIPSE_DEFAULT => {
            !is_quarter_turn(relative)
        }
        Shape::Ellipse { frame, .. } => !is_quarter_turn(relative) && !is_circle(frame),
    }
}

/// The path a primitive becomes: the outline "Object to path" gives it, under the
/// same id and with the same style, its anchors minted by `minter`, one outline (open
/// only for an open Arc), and the shown angle (a polygon's or star's frame angle plus its
/// register) as `rotation`. The one function behind "Object to path" and the
/// conversion of a stretch, so the two cannot differ (`adrs.md` decision 8).
#[must_use]
pub fn primitive_as_path(
    primitive: &PrimitiveSnapshot,
    minter: &mut AnchorIdMinter,
) -> PathSnapshot {
    let outline = outline_of_rotated(&primitive.shape, primitive.rotation);
    let anchors = outline
        .iter()
        .map(|anchor| AnchorSnapshot {
            id: minter.mint(),
            point: anchor.point,
            handle_in: anchor.handle_in,
            handle_out: anchor.handle_out,
            kind: anchor.kind,
        })
        .collect();
    PathSnapshot {
        id: primitive.id,
        closed: outline.closed,
        style: primitive.style.clone(),
        anchors,
        extra_subpaths: Vec::new(),
        rotation: ObjectSnapshot::Primitive(primitive.clone()).orientation(),
    }
}

/// `path` with every anchor id replaced by a fresh one from `minter`: the ids a
/// conversion carries from the press (built before a minter was at hand) become the
/// session's own before anything is previewed or written.
pub(crate) fn with_fresh_anchor_ids(path: &mut PathSnapshot, minter: &mut AnchorIdMinter) {
    for anchor in path.all_anchors_mut() {
        anchor.id = minter.mint();
    }
}

/// Whether the gesture on `handle` turns `object` into a path (`specs/0047-transform-polish/`
/// criteria 2, 7 and 10), the one rule behind [`converted_for_object`] and
/// [`converted_for_handle`]. A skew converts every primitive; a resize converts what
/// [`converts_on_stretch`] says along `axes`; a rotate, a move and a parameter handle
/// never convert.
fn converts_for(object: &ObjectSnapshot, handle: EditHandle, axes: Angle) -> bool {
    match handle {
        EditHandle::Skew(_) => matches!(object, ObjectSnapshot::Primitive(_)),
        EditHandle::Resize(_) => converts_on_stretch(object, axes),
        EditHandle::Rotate(_) | EditHandle::Move | EditHandle::Param(_) => false,
    }
}

/// The path `object` becomes if the gesture on `handle` ends in a conversion, with the
/// ids of a local counter (see [`converted_for_handle`]); `None` for an object the
/// gesture leaves in its kind. A skew converts every primitive; a resize converts a
/// polygon or star (the box of a single rectangle or ellipse resizes in its own axes).
#[must_use]
pub(crate) fn converted_for_object(
    object: &ObjectSnapshot,
    handle: EditHandle,
) -> Option<PathSnapshot> {
    let ObjectSnapshot::Primitive(primitive) = object else {
        return None;
    };
    // The box of a single object has its own axes, which a rectangle or ellipse shares.
    converts_for(object, handle, object.rotation())
        .then(|| primitive_as_path(primitive, &mut AnchorIdMinter::new(0)))
}

/// The paths the shapes of `starts` become if the group gesture on `handle` ends in a
/// commit, one entry per start, `None` for an object that keeps its kind. A skew
/// converts every primitive; a resize the objects that [`converts_on_stretch`] along
/// the document axes. The anchors carry the ids of a local counter, unique within the
/// call and replaced by the session's own before the first preview
/// (`SelectTool::mint_conversion_ids`).
#[must_use]
pub(crate) fn converted_for_handle(
    starts: &[ObjectSnapshot],
    handle: EditHandle,
) -> Vec<Option<PathSnapshot>> {
    let mut placeholder = AnchorIdMinter::new(0);
    starts
        .iter()
        .map(|object| match object {
            ObjectSnapshot::Primitive(primitive)
                if converts_for(object, handle, Angle::from_radians(0.0)) =>
            {
                Some(primitive_as_path(primitive, &mut placeholder))
            }
            // A clone whose chain moves with it is unlinked by a singular scale or skew
            // (`specs/0049-linked-clone` criterion 16): its path is ready from the press.
            ObjectSnapshot::Clone(clone)
                if matches!(handle, EditHandle::Resize(_) | EditHandle::Skew(_))
                    && crate::clone_gesture::chain_moves(clone, starts) =>
            {
                Some(crate::clone_gesture::unlinked_placeholder(clone))
            }
            _ => None,
        })
        .collect()
}

/// For every id in `ids` that still names a live primitive, the outline
/// "Object to path" gives it ([`primitive_as_path`]) with that id, exactly the
/// shape `Document::convert_to_paths` takes. An id that no longer resolves to a
/// primitive (already deleted, or already converted) is silently skipped, the same
/// lazy-resolution stance `curvyo-ui-core`'s other multi-id operations take (ADR
/// 0009 §2); `convert_to_paths` itself still refuses the whole call on any id it
/// cannot resolve, so a genuinely stale id surviving to that point is reported
/// there, not swallowed twice. The path's `rotation` is written by
/// `convert_to_paths` itself, from the stored primitive.
#[must_use]
pub fn build_primitive_conversions(
    document: &Document,
    minter: &mut AnchorIdMinter,
    ids: &[NodeId],
) -> Vec<(NodeId, Vec<NewAnchor>)> {
    ids.iter()
        .filter_map(|&id| {
            let primitive = document.primitive(id)?;
            let path = primitive_as_path(&primitive, minter);
            let anchors = path
                .anchors
                .into_iter()
                .map(|anchor| NewAnchor {
                    id: anchor.id,
                    point: anchor.point,
                    handle_in: anchor.handle_in,
                    handle_out: anchor.handle_out,
                    kind: anchor.kind,
                })
                .collect();
            Some((id, anchors))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{
        AnchorKind, EllipseFrame, InnerRatio, Length, Point, PointCount, RectBounds, StarFrame,
    };

    fn turned(document: &Document, id: NodeId, degrees: f64) -> ObjectSnapshot {
        let rotated = document.object(id).expect("exists").rotated(
            Point::new(0.0, 0.0),
            Angle::from_radians(degrees.to_radians()),
        );
        document.rotate_object(&rotated).expect("rotates");
        document.object(id).expect("exists")
    }

    /// Decision 8: a path and a circle never convert; an aligned rectangle or ellipse
    /// keeps its kind; a polygon, a star and an off-axis rectangle or ellipse do.
    #[test]
    fn the_converting_kinds_follow_the_rule() {
        let document = Document::new(1);
        let rect = document.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(6.0),
        });
        let ellipse = document.create_ellipse(EllipseFrame {
            center: Point::new(0.0, 0.0),
            rx: Length::from_mm(5.0),
            ry: Length::from_mm(3.0),
        });
        let circle = document.create_ellipse(EllipseFrame {
            center: Point::new(0.0, 0.0),
            rx: Length::from_mm(4.0),
            ry: Length::from_mm(4.0),
        });
        let star = document.create_star(
            StarFrame {
                center: Point::new(0.0, 0.0),
                radius: Length::from_mm(5.0),
                angle: Angle::from_radians(0.0),
            },
            PointCount::new(5).expect("count"),
            InnerRatio::new(0.4).expect("ratio"),
        );
        let zero = Angle::from_radians(0.0);
        let converts = |object: &ObjectSnapshot| converts_on_stretch(object, zero);
        assert!(!converts(&document.object(rect).unwrap()));
        assert!(
            !converts(&turned(&document, rect, 90.0)),
            "a quarter turn is aligned"
        );
        assert!(converts(&turned(&document, rect, 30.0)));
        assert!(!converts(&document.object(ellipse).unwrap()));
        assert!(converts(&turned(&document, ellipse, 30.0)));
        assert!(
            !converts(&turned(&document, circle, 30.0)),
            "a circle never converts"
        );
        assert!(converts(&document.object(star).unwrap()));
        // Against its own box angle a turned rectangle keeps its kind (one object).
        let own = turned(&document, rect, 30.0);
        assert!(!converts_on_stretch(&own, own.rotation()));
    }

    /// `0019` criterion 53.8: a rotation within 1e-9 rad of a quarter turn is aligned (1e-10
    /// is, 1e-6 is not); a circle is |rx - ry| under 1e-6 mm.
    #[test]
    fn the_boundaries_of_aligned_and_circle() {
        let document = Document::new(1);
        let rect = document.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(6.0),
        });
        let ellipse = document.create_ellipse(EllipseFrame {
            center: Point::new(0.0, 0.0),
            rx: Length::from_mm(5.0),
            ry: Length::from_mm(5.0 + 1e-7),
        });
        let with_rotation = |id: NodeId, radians: f64| match document.object(id).unwrap() {
            ObjectSnapshot::Primitive(p) => ObjectSnapshot::Primitive(PrimitiveSnapshot {
                rotation: Angle::from_radians(radians),
                ..p
            }),
            ObjectSnapshot::Path(_) | ObjectSnapshot::Clone(_) => panic!("a primitive"),
        };
        let zero = Angle::from_radians(0.0);
        let quarter = std::f64::consts::FRAC_PI_2;
        for turns in [0.0, quarter, std::f64::consts::PI] {
            assert!(!converts_on_stretch(
                &with_rotation(rect, turns + 1e-10),
                zero
            ));
            assert!(converts_on_stretch(
                &with_rotation(rect, turns + 1e-6),
                zero
            ));
        }
        // 1e-7 apart is a circle, whatever its rotation; 1e-5 apart is an ellipse.
        assert!(!converts_on_stretch(&with_rotation(ellipse, 0.5), zero));
        let wider = match document.object(ellipse).unwrap() {
            ObjectSnapshot::Primitive(p) => ObjectSnapshot::Primitive(PrimitiveSnapshot {
                shape: Shape::Ellipse {
                    frame: EllipseFrame {
                        center: Point::new(0.0, 0.0),
                        rx: Length::from_mm(5.0),
                        ry: Length::from_mm(5.0 + 1e-5),
                    },
                    arc: None,
                    curve: curvyo_document_core::Curve::ELLIPSE_DEFAULT,
                },
                rotation: Angle::from_radians(0.5),
                ..p
            }),
            ObjectSnapshot::Path(_) | ObjectSnapshot::Clone(_) => panic!("a primitive"),
        };
        assert!(converts_on_stretch(&wider, zero));
    }

    /// Criterion 53.1/53.2 and the debt item: the converted path has the id, style,
    /// closed outline of "Object to path" and the shown angle as its rotation.
    #[test]
    fn a_star_becomes_the_path_object_to_path_gives_it() {
        let document = Document::new(1);
        let id = document.create_star(
            StarFrame {
                center: Point::new(10.0, 10.0),
                radius: Length::from_mm(5.0),
                angle: Angle::from_radians(0.5),
            },
            PointCount::new(5).expect("count"),
            InnerRatio::new(0.4).expect("ratio"),
        );
        let ObjectSnapshot::Primitive(star) = document.object(id).expect("exists") else {
            panic!("a star");
        };
        let mut minter = AnchorIdMinter::new(3);
        let path = primitive_as_path(&star, &mut minter);
        assert_eq!(path.id, id);
        assert_eq!(path.style, star.style);
        assert!(path.closed && path.extra_subpaths.is_empty());
        assert_eq!(path.anchors.len(), 10);
        assert!((path.rotation.as_radians() - 0.5).abs() < 1e-12);
        // The same anchors "Object to path" writes.
        let conversions = build_primitive_conversions(&document, &mut minter, &[id]);
        for (a, b) in path.anchors.iter().zip(&conversions[0].1) {
            assert_eq!(
                (a.point, a.handle_in, a.handle_out),
                (b.point, b.handle_in, b.handle_out)
            );
        }
        let mut again = path;
        let first = again.anchors[0].id;
        with_fresh_anchor_ids(&mut again, &mut minter);
        assert_ne!(again.anchors[0].id, first);
    }

    /// AC17: one primitive's conversion carries its outline's exact
    /// anchor count/kind, with freshly minted, distinct ids.
    #[test]
    fn build_primitive_conversions_mints_fresh_ids_for_one_rect() {
        let document = Document::new(1);
        let id = document.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        });
        let mut minter = AnchorIdMinter::new(1);
        let conversions = build_primitive_conversions(&document, &mut minter, &[id]);
        assert_eq!(conversions.len(), 1);
        let (converted_id, anchors) = &conversions[0];
        assert_eq!(*converted_id, id);
        assert_eq!(anchors.len(), 4);
        assert!(anchors.iter().all(|a| a.kind == AnchorKind::Corner));
        let mut ids: Vec<_> = anchors.iter().map(|a| a.id).collect();
        ids.sort_by_key(|id| id.to_hex());
        ids.dedup();
        assert_eq!(ids.len(), 4, "every minted anchor id is distinct");
    }

    /// AC22: multiple primitives convert independently in the returned
    /// list, each with its own outline.
    #[test]
    fn build_primitive_conversions_handles_a_multi_object_selection() {
        let document = Document::new(1);
        let rect_id = document.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        });
        let ellipse_id = document.create_ellipse(EllipseFrame {
            center: Point::new(0.0, 0.0),
            rx: Length::from_mm(5.0),
            ry: Length::from_mm(5.0),
        });
        let mut minter = AnchorIdMinter::new(1);
        let conversions =
            build_primitive_conversions(&document, &mut minter, &[rect_id, ellipse_id]);
        assert_eq!(conversions.len(), 2);
        assert_eq!(conversions[0].1.len(), 4, "rect: 4 corners");
        assert_eq!(conversions[1].1.len(), 4, "ellipse: 4 smooth nodes");
    }

    /// Acceptance criterion 17/21: converting a rotated primitive bakes
    /// the rotated outline into the new path's anchors, and the
    /// converted path keeps the original's `rotation` register.
    #[test]
    fn build_primitive_conversions_bakes_rotation_and_keeps_the_register() {
        let document = Document::new(1);
        let id = document.create_rect(RectBounds {
            origin: Point::new(-5.0, -5.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        });
        document
            .rotate_object(&document.object(id).expect("object exists").rotated(
                Point::new(0.0, 0.0),
                curvyo_document_core::Angle::from_radians(std::f64::consts::FRAC_PI_2),
            ))
            .expect("rotate");
        let mut minter = AnchorIdMinter::new(1);
        let conversions = build_primitive_conversions(&document, &mut minter, &[id]);
        document.convert_to_paths(&conversions).expect("convert");

        let path = document.path(id).expect("now a path, same id");
        assert!(
            (path.rotation.as_radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-9,
            "rotation register carried over"
        );
        // The unrotated top-left corner was (-5, -5); after a 90-degree
        // rotation about the origin it is now (5, -5).
        assert!(
            path.anchors
                .iter()
                .any(|a| (a.point.x - 5.0).abs() < 1e-6 && (a.point.y - (-5.0)).abs() < 1e-6),
            "anchors are baked into the rotated, absolute positions: {:?}",
            path.anchors
        );
    }

    /// A stale id (no longer a primitive) is skipped, not included.
    #[test]
    fn build_primitive_conversions_skips_a_stale_id() {
        let document = Document::new(1);
        let id = document.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(1.0),
            height: Length::from_mm(1.0),
        });
        let mut minter = AnchorIdMinter::new(1);
        document
            .convert_to_paths(&build_primitive_conversions(&document, &mut minter, &[id]))
            .expect("convert once");

        let conversions = build_primitive_conversions(&document, &mut minter, &[id]);
        assert!(conversions.is_empty(), "already a path, not a primitive");
    }
}
