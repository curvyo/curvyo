//! `Document`'s primitive-shape command methods
//! (`specs/0003-primitive-shapes/adrs.md`, "the primitive schema" and "'object
//! to path' keeps the `NodeId`"). Mirrors [`crate::paths`]'s shape for
//! paths: each mutating method here ends in exactly one Loro commit
//! (ADR 0002 §9), and every id is resolved before the first write so a
//! refused multi-object call (`convert_to_paths`) never leaves a partial
//! edit behind.

use crate::corner_radii::CornerRadii;
use crate::corner_radii_codec::write_corner_radii_if_changed;
use crate::document::{Document, OBJECTS_TREE};
use crate::path_codec::{self, leaf_exists};
use crate::path_model::{NewAnchor, NodeId};
use crate::primitive_model::{
    EllipseFrame, InnerRatio, ObjectSnapshot, PointCount, PrimitiveSnapshot, RectBounds, Shape,
    StarFrame,
};
use crate::shape_arc_codec::write_arc_and_curve;
use crate::shape_codec::{self, SHAPE_ELLIPSE, SHAPE_POLYGON, SHAPE_RECT, SHAPE_STAR};
use crate::units::{Angle, Length};

/// Why a shape-editing [`Document`] method refused to apply.
///
/// Mirrors [`crate::path_model::PathEditError`]'s own framing: every
/// variant describes a caller error against a snapshot that is already
/// stale (ADR 0009 §2), not a defect in this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ShapeEditError {
    /// No object with this [`NodeId`] exists (deleted locally or by a
    /// collaborator, or already converted to a path, since the caller
    /// last read a snapshot).
    #[error("no such object")]
    NoSuchObject,
    /// The object exists but is a path, not a primitive — or (for
    /// `convert_to_paths`) is already a path.
    #[error("this object is not a primitive shape")]
    NotAPrimitive,
    /// The object exists and is a primitive, but not the shape this
    /// operation applies to (acceptance criterion 14: a polygon has no
    /// inner ratio to set).
    #[error("this operation does not apply to this primitive's shape")]
    WrongShape,
    /// A corner radius is not a finite number (NaN or infinite). Nothing is
    /// written. A negative radius is not an error: it is floored to 0.
    #[error("a corner radius must be a finite number")]
    InvalidRadius,
    /// A resize carried a stroke width that is not a finite number greater
    /// than zero. Nothing is written: a stored width of zero or less would
    /// make the file refuse to open.
    #[error("a stroke width must be a finite number greater than zero")]
    InvalidStrokeWidth,
    /// An arc on an ellipse whose curve is not 100 %, or a curve on an ellipse with an
    /// arc (`specs/0021-ellipse-arcs-and-shaping/` criterion 31): the two never coexist,
    /// so nothing is written.
    #[error("an ellipse cannot have both an arc and a curve")]
    ArcWithCurve,
}

impl Document {
    /// Creates a new primitive of `shape`'s kind, turned by `rotation` about its frame's own
    /// centre, in one commit with the kind's label (`create_rect`, `create_ellipse`,
    /// `create_polygon`, `create_star`): the keys the kind's create-drag writes, plus the
    /// `rotation` register when `rotation` is not zero, stored normalized
    /// (`specs/0052-click-to-create-shapes/adrs.md` decision 2). A rectangle keeps the corner
    /// radii `shape` carries. Becoming the selected object is the caller's job (ADR 0009 §2:
    /// selection is ephemeral `curvyo-ui-core` state).
    ///
    /// # Panics
    /// Does not panic in practice: see [`crate::paths::Document::
    /// create_path`]'s own doc comment — the same reasoning applies here.
    #[must_use]
    pub fn create_primitive(&self, shape: Shape, rotation: Angle) -> NodeId {
        let label = match shape {
            Shape::Rect { .. } => "create_rect",
            Shape::Ellipse { .. } => "create_ellipse",
            Shape::Polygon { .. } => "create_polygon",
            Shape::Star { .. } => "create_star",
        };
        let id = self.create_primitive_uncommitted(shape, rotation);
        self.commit_with_label(label);
        id
    }

    /// [`Document::create_primitive`] without the commit: the caller commits, so several new
    /// objects can be one step (`specs/0046-document-shapes` criterion 23).
    pub(crate) fn create_primitive_uncommitted(&self, shape: Shape, rotation: Angle) -> NodeId {
        let tag = match shape {
            Shape::Rect { .. } => SHAPE_RECT,
            Shape::Ellipse { .. } => SHAPE_ELLIPSE,
            Shape::Polygon { .. } => SHAPE_POLYGON,
            Shape::Star { .. } => SHAPE_STAR,
        };
        let id = self.create_primitive_node(tag);
        let tree = self.loro().get_tree(OBJECTS_TREE);
        // invariant: this node was just created by `create_primitive_node`.
        #[allow(clippy::unwrap_used)]
        let meta = tree.get_meta(tree_id_of(id)).unwrap();
        match shape {
            Shape::Rect {
                bounds,
                corner_radii,
            } => {
                shape_codec::write_rect_bounds(&meta, bounds);
                write_corner_radii_if_changed(&meta, corner_radii);
            }
            Shape::Ellipse { frame, .. } => shape_codec::write_ellipse_frame(&meta, frame),
            Shape::Polygon {
                frame, point_count, ..
            } => {
                shape_codec::write_star_frame(&meta, frame);
                shape_codec::write_point_count(&meta, point_count);
            }
            Shape::Star {
                frame,
                point_count,
                inner_ratio,
                ..
            } => {
                shape_codec::write_star_frame(&meta, frame);
                shape_codec::write_point_count(&meta, point_count);
                shape_codec::write_inner_ratio(&meta, inner_ratio);
            }
        }
        write_arc_and_curve(&meta, &shape);
        // As `convert_to_paths`: an absent register reads as 0, so a zero turn writes nothing.
        if rotation.normalized() != Angle::from_radians(0.0) {
            path_codec::write_rotation(&meta, rotation);
        }
        id
    }

    /// Creates a new rectangle primitive with four zero corner radii
    /// (acceptance criterion 1): [`Document::create_primitive`] with no turn.
    #[must_use]
    pub fn create_rect(&self, bounds: RectBounds) -> NodeId {
        let corner_radii = CornerRadii::uniform(Length::from_mm(0.0));
        self.create_primitive(
            Shape::Rect {
                bounds,
                corner_radii,
            },
            Angle::from_radians(0.0),
        )
    }

    /// Creates a new ellipse/circle primitive (acceptance criteria 7, 8):
    /// [`Document::create_primitive`] with no turn.
    #[must_use]
    pub fn create_ellipse(&self, frame: EllipseFrame) -> NodeId {
        self.create_primitive(
            Shape::Ellipse {
                frame,
                arc: None,
                curve: crate::Curve::ELLIPSE_DEFAULT,
            },
            Angle::from_radians(0.0),
        )
    }

    /// Creates a new regular polygon primitive (acceptance criterion 11):
    /// [`Document::create_primitive`] with no turn.
    #[must_use]
    pub fn create_polygon(&self, frame: StarFrame, point_count: PointCount) -> NodeId {
        self.create_primitive(
            Shape::Polygon {
                frame,
                point_count,
                curve: crate::Curve::POLYGON_DEFAULT,
            },
            Angle::from_radians(0.0),
        )
    }

    /// Creates a new star primitive (acceptance criterion 12):
    /// [`Document::create_primitive`] with no turn.
    #[must_use]
    pub fn create_star(
        &self,
        frame: StarFrame,
        point_count: PointCount,
        inner_ratio: InnerRatio,
    ) -> NodeId {
        let shape = Shape::Star {
            frame,
            point_count,
            inner_ratio,
            curve: crate::Curve::POLYGON_DEFAULT,
        };
        self.create_primitive(shape, Angle::from_radians(0.0))
    }

    /// Creates a root tree node tagged `shape` and carrying this slice's
    /// shared placeholder style (acceptance criterion 16), common to
    /// [`Document::create_primitive`]. Leaves the commit to its caller,
    /// which still has shape-specific fields left to write.
    ///
    /// # Panics
    /// Does not panic in practice: only creates a root-level node and
    /// inserts known-valid keys into its freshly created meta map,
    /// neither of which Loro's API can reject.
    fn create_primitive_node(&self, shape: &str) -> NodeId {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        // invariant: creating a node under the root or a live group on a
        // freshly obtained tree handle cannot fail.
        #[allow(clippy::unwrap_used)]
        let tree_id = tree.create(self.drawing_parent(&tree)).unwrap();
        // invariant: reading the meta map of a node this call just
        // created cannot fail.
        #[allow(clippy::unwrap_used)]
        let meta = tree.get_meta(tree_id).unwrap();
        shape_codec::write_shape_tag(&meta, shape);
        crate::style_codec::write_creation_style(&meta);
        NodeId::from_parts(tree_id.peer, tree_id.counter)
    }

    /// Reads one object's full current data generically, or `None` if it
    /// no longer exists — the dispatch point `specs/0003-primitive-shapes/
    /// adrs.md` names ("`Document::path(id)` returns `None` for a
    /// primitive node... reading goes through an object-level snapshot").
    #[must_use]
    pub fn object(&self, id: NodeId) -> Option<ObjectSnapshot> {
        if let Some(sketch) = self.sketch(id) {
            return Some(ObjectSnapshot::Sketch(Box::new(sketch)));
        }
        if self.clone_link_of(id).is_some() {
            return self.read_cached_clone(id).map(ObjectSnapshot::Clone);
        }
        object_in(&self.loro().get_tree(OBJECTS_TREE), id)
    }

    /// Reads one primitive's full current data, or `None` if it no
    /// longer exists or is a path instead.
    #[must_use]
    pub fn primitive(&self, id: NodeId) -> Option<PrimitiveSnapshot> {
        match self.object(id)? {
            ObjectSnapshot::Primitive(snapshot) => Some(snapshot),
            ObjectSnapshot::Sketch(_)
            | ObjectSnapshot::Path(_)
            | ObjectSnapshot::Clone(_)
            | ObjectSnapshot::Text(_)
            | ObjectSnapshot::Image(_)
            | ObjectSnapshot::Dimension(_) => None,
        }
    }

    /// Sets a rectangle's bounding box as one commit — a resize drag
    /// (acceptance criterion 3). Leaves `corner_radius` untouched; its
    /// *effective* value is reclamped on read, never rewritten here
    /// (`adrs.md`'s "clamped where it is evaluated" decision).
    ///
    /// # Errors
    /// [`ShapeEditError::NoSuchObject`] if `id` no longer exists;
    /// [`ShapeEditError::NotAPrimitive`] / [`ShapeEditError::WrongShape`]
    /// if it exists but is not a rectangle.
    pub fn set_rect_bounds(&self, id: NodeId, bounds: RectBounds) -> Result<(), ShapeEditError> {
        let meta = self.require_shape(id, SHAPE_RECT)?;
        shape_codec::write_rect_bounds(&meta, bounds);
        self.commit_with_label("set_rect_bounds");
        Ok(())
    }

    /// Sets an ellipse/circle's frame as one commit (acceptance
    /// criterion 9).
    ///
    /// # Errors
    /// [`ShapeEditError::NoSuchObject`] / [`ShapeEditError::NotAPrimitive`]
    /// / [`ShapeEditError::WrongShape`] if `id` is not an ellipse.
    pub fn set_ellipse_frame(&self, id: NodeId, frame: EllipseFrame) -> Result<(), ShapeEditError> {
        let meta = self.require_shape(id, SHAPE_ELLIPSE)?;
        shape_codec::write_ellipse_frame(&meta, frame);
        self.commit_with_label("set_ellipse_frame");
        Ok(())
    }

    /// Sets a polygon or star's frame (center, outer radius,
    /// orientation) as one commit — a uniform-scale resize (acceptance
    /// criterion 13).
    ///
    /// # Errors
    /// [`ShapeEditError::NoSuchObject`] / [`ShapeEditError::NotAPrimitive`]
    /// if `id` is neither a polygon nor a star.
    pub fn set_star_frame(&self, id: NodeId, frame: StarFrame) -> Result<(), ShapeEditError> {
        let meta = self.require_polygon_or_star(id)?;
        shape_codec::write_star_frame(&meta, frame);
        self.commit_with_label("set_star_frame");
        Ok(())
    }

    /// Resizes an ellipse's frame and stroke width together as one
    /// commit (acceptance criterion 8).
    ///
    /// # Errors
    /// [`ShapeEditError::NoSuchObject`] / [`ShapeEditError::NotAPrimitive`]
    /// / [`ShapeEditError::WrongShape`] if `id` is not an ellipse;
    /// [`ShapeEditError::InvalidStrokeWidth`] for a width that is not above
    /// zero.
    pub fn resize_ellipse(
        &self,
        id: NodeId,
        frame: EllipseFrame,
        stroke_width: Option<Length>,
    ) -> Result<(), ShapeEditError> {
        check_stroke_width(stroke_width)?;
        let meta = self.require_shape(id, SHAPE_ELLIPSE)?;
        shape_codec::write_ellipse_frame(&meta, frame);
        write_stroke_width_if_changed(&meta, stroke_width);
        self.commit_with_label("resize_ellipse");
        Ok(())
    }

    /// Resizes a polygon/star's frame (its uniform outer-radius scale,
    /// acceptance criterion 11) and stroke width together as one commit.
    /// `point_count`/`inner_ratio` are untouched.
    ///
    /// # Errors
    /// [`ShapeEditError::NoSuchObject`] / [`ShapeEditError::NotAPrimitive`]
    /// if `id` is neither a polygon nor a star;
    /// [`ShapeEditError::InvalidStrokeWidth`] for a width that is not above
    /// zero.
    pub fn resize_star_frame(
        &self,
        id: NodeId,
        frame: StarFrame,
        stroke_width: Option<Length>,
    ) -> Result<(), ShapeEditError> {
        check_stroke_width(stroke_width)?;
        let meta = self.require_polygon_or_star(id)?;
        shape_codec::write_star_frame(&meta, frame);
        write_stroke_width_if_changed(&meta, stroke_width);
        self.commit_with_label("resize_star_frame");
        Ok(())
    }

    /// Sets a point count as one commit for the **whole batch**
    /// (architect review, same reasoning as [`Document::set_corner_
    /// radius`]: the tool-options bar's point-count stepper can apply
    /// to an entire multi-selection, acceptance criteria 10, 15),
    /// independent of `star_frame`/`inner_ratio` (`adrs.md` decision 2:
    /// separate registers). Every id is resolved and confirmed to be a
    /// polygon or a star before any of them is written.
    ///
    /// # Errors
    /// [`ShapeEditError::NoSuchObject`] if any named id no longer
    /// exists; [`ShapeEditError::NotAPrimitive`] if any named id is
    /// neither a polygon nor a star.
    pub fn set_point_count(
        &self,
        ids: &[NodeId],
        point_count: PointCount,
    ) -> Result<(), ShapeEditError> {
        let metas: Vec<_> = ids
            .iter()
            .map(|&id| self.require_polygon_or_star(id))
            .collect::<Result<_, _>>()?;
        for meta in metas {
            shape_codec::write_point_count(&meta, point_count);
        }
        self.commit_with_label("set_point_count");
        Ok(())
    }

    /// Sets a star's inner/outer ratio as one commit for the **whole
    /// batch** (architect review, same reasoning as
    /// [`Document::set_corner_radius`], acceptance criteria 12, 14),
    /// independent of `star_frame` — the outer radius stays exactly as
    /// stored. Every id is resolved and confirmed to be a star before
    /// any of them is written.
    ///
    /// # Errors
    /// [`ShapeEditError::NoSuchObject`] if any named id no longer
    /// exists; [`ShapeEditError::WrongShape`] if any named id is a
    /// plain polygon (acceptance criterion 14: "a polygon has no inner
    /// radius... distinct from its outer one") or anything else that
    /// is not a star.
    pub fn set_inner_ratio(&self, ids: &[NodeId], ratio: InnerRatio) -> Result<(), ShapeEditError> {
        let metas: Vec<_> = ids
            .iter()
            .map(|&id| self.require_shape(id, SHAPE_STAR))
            .collect::<Result<_, _>>()?;
        for meta in metas {
            shape_codec::write_inner_ratio(&meta, ratio);
        }
        self.commit_with_label("set_inner_ratio");
        Ok(())
    }

    /// Converts every named primitive to a path, in one commit for the
    /// whole call (acceptance criteria 17, 22) — a rewrite of each node
    /// in place, keeping its [`NodeId`] (`adrs.md`'s "'object to path'
    /// keeps the `NodeId`" decision): `shape` and every primitive
    /// parameter key are deleted, `closed = true` and `anchors` are
    /// written, and the style keys are left exactly as they are.
    ///
    /// Every id is resolved — and confirmed to currently name a
    /// primitive — before any of them is written, so one unknown or
    /// already-a-path id anywhere in `conversions` refuses the whole
    /// call rather than converting a prefix of it. `curvyo-ui-core` is
    /// expected to have already filtered a mixed selection down to its
    /// primitives before calling this (`adrs.md`).
    ///
    /// # Errors
    /// [`ShapeEditError::NoSuchObject`] if any named id no longer exists;
    /// [`ShapeEditError::NotAPrimitive`] if any named id is already a
    /// path.
    pub fn convert_to_paths(
        &self,
        conversions: &[(NodeId, Vec<NewAnchor>)],
    ) -> Result<(), ShapeEditError> {
        self.convert_to_paths_uncommitted(conversions)?;
        self.commit_with_label("convert_to_paths");
        Ok(())
    }

    /// [`Document::convert_to_paths`] without the commit, for a command that converts as one part
    /// of its own commit (Linked clone, `specs/0049-linked-clone` criterion 2). Writes nothing
    /// when it refuses.
    pub(crate) fn convert_to_paths_uncommitted(
        &self,
        conversions: &[(NodeId, Vec<NewAnchor>)],
    ) -> Result<(), ShapeEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let metas: Vec<_> = conversions
            .iter()
            .map(|(id, _)| {
                let tree_id = tree_id_of(*id);
                if !leaf_exists(&tree, tree_id) {
                    return Err(ShapeEditError::NoSuchObject);
                }
                let meta = tree
                    .get_meta(tree_id)
                    .map_err(|_| ShapeEditError::NoSuchObject)?;
                if shape_codec::read_shape_tag(&meta).as_deref() == Some("sketch")
                    || shape_codec::read_shape_tag(&meta).is_none()
                    || crate::clone_codec::is_clone_meta(&meta)
                    || crate::text_codec::is_text_meta(&meta)
                {
                    return Err(ShapeEditError::NotAPrimitive);
                }
                Ok(meta)
            })
            .collect::<Result<_, _>>()?;

        // The shown angle (a polygon's or star's frame angle plus its register)
        // is what the converted path keeps as its `rotation`, read before the
        // primitive keys go, so that its readout and box direction do not change.
        let read = self.conversion_reads(conversions)?;
        for ((meta, (_, anchors)), (orientation, closed)) in
            metas.into_iter().zip(conversions.iter()).zip(read)
        {
            shape_codec::strip_primitive_keys(&meta);
            shape_codec::write_converted_path_fields(&meta, anchors, closed);
            if path_codec::read_rotation(&meta) != orientation.normalized() {
                path_codec::write_rotation(&meta, orientation);
            }
        }
        Ok(())
    }

    /// Looks up a primitive's meta map, refusing unless it both exists
    /// and currently has `shape == expected`.
    pub(crate) fn require_shape(
        &self,
        id: NodeId,
        expected: &str,
    ) -> Result<loro::LoroMap, ShapeEditError> {
        let meta = self.require_primitive(id)?;
        match shape_codec::read_shape_tag(&meta) {
            Some(shape) if shape == expected => Ok(meta),
            Some(_) => Err(ShapeEditError::WrongShape),
            None => Err(ShapeEditError::NotAPrimitive),
        }
    }

    /// Same as [`Document::require_shape`], but accepts either
    /// `"polygon"` or `"star"` — the operations whose parameter
    /// (`star_frame`, `point_count`) both shapes share.
    fn require_polygon_or_star(&self, id: NodeId) -> Result<loro::LoroMap, ShapeEditError> {
        let meta = self.require_primitive(id)?;
        match shape_codec::read_shape_tag(&meta) {
            Some(shape) if shape == SHAPE_POLYGON || shape == SHAPE_STAR => Ok(meta),
            Some(_) => Err(ShapeEditError::WrongShape),
            None => Err(ShapeEditError::NotAPrimitive),
        }
    }

    pub(crate) fn require_primitive(&self, id: NodeId) -> Result<loro::LoroMap, ShapeEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = tree_id_of(id);
        if !leaf_exists(&tree, tree_id) {
            return Err(ShapeEditError::NoSuchObject);
        }
        tree.get_meta(tree_id)
            .map_err(|_| ShapeEditError::NoSuchObject)
    }
}

/// Refuses a resize width that is not a finite number above zero.
pub(crate) fn check_stroke_width(stroke_width: Option<Length>) -> Result<(), ShapeEditError> {
    if crate::style_codec::stroke_width_is_writable(stroke_width) {
        Ok(())
    } else {
        Err(ShapeEditError::InvalidStrokeWidth)
    }
}

/// Writes `stroke_width` only if it is `Some` and differs from the stored
/// value. `None` leaves the key untouched (a resize with "Scale stroke
/// width" off: `specs/0005-object-transform/adrs.md`, 2026-10-06 — a
/// peer's concurrent stroke edit must survive); an equal `Some` is not
/// rewritten either (an LWW rewrite of an unchanged value is a new
/// operation).
pub(crate) fn write_stroke_width_if_changed(meta: &loro::LoroMap, stroke_width: Option<Length>) {
    if let Some(width) = stroke_width
        && crate::style_codec::read_width(meta) != width
    {
        crate::style_codec::write_stroke_width(meta, width);
    }
}

fn tree_id_of(id: NodeId) -> loro::TreeID {
    loro::TreeID::new(id.peer, id.counter)
}

/// One object of `tree` as [`Document::object`] reads it; also for a checked-out version
/// (`history/checkout.rs`).
pub(crate) fn object_in(tree: &loro::LoroTree, id: NodeId) -> Option<ObjectSnapshot> {
    let tree_id = tree_id_of(id);
    if !leaf_exists(tree, tree_id) {
        return None;
    }
    let meta = tree.get_meta(tree_id).ok()?;
    if crate::clone_codec::is_clone_meta(&meta) {
        return crate::object_read::read_one_clone(tree, id).map(ObjectSnapshot::Clone);
    }
    if crate::text_codec::is_text_meta(&meta) {
        return crate::text_clone_read::read_text_node(tree, id, &meta).map(|mut text| {
            crate::text_carrier::resolve_one(tree, &mut text);
            ObjectSnapshot::Text(Box::new(text))
        });
    }
    if crate::image_codec::is_image_meta(&meta) {
        return crate::image_codec::read_image_snapshot(id, &meta).map(ObjectSnapshot::Image);
    }
    if crate::dimension_codec::is_dimension_meta(&meta) {
        return crate::dimension_codec::read_dimension_snapshot(id, &meta).map(|mut dimension| {
            crate::dimension_read::resolve_into(&mut dimension, |_| None);
            ObjectSnapshot::Dimension(Box::new(dimension))
        });
    }
    match shape_codec::read_shape_tag(&meta) {
        Some(shape) if shape == "sketch" => crate::sketch_read::read(id, &meta)
            .ok()
            .map(|sketch| ObjectSnapshot::Sketch(Box::new(sketch))),
        Some(shape) => Some(ObjectSnapshot::Primitive(
            shape_codec::read_primitive_snapshot(id, &meta, &shape),
        )),
        None => Some(ObjectSnapshot::Path(crate::path_codec::read_path_snapshot(
            id, &meta,
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_model::{AnchorId, AnchorKind};
    use crate::primitive_model::Shape;
    use crate::units::{Angle, Point};

    fn rect_bounds(x: f64, y: f64, w: f64, h: f64) -> RectBounds {
        RectBounds {
            origin: Point::new(x, y),
            width: Length::from_mm(w),
            height: Length::from_mm(h),
        }
    }

    #[test]
    fn create_rect_round_trips_zero_radius_bounds() {
        let document = Document::new(1);
        let bounds = rect_bounds(0.0, 0.0, 10.0, 20.0);
        let id = document.create_rect(bounds);
        let snapshot = document.primitive(id).expect("exists");
        assert_eq!(
            snapshot.shape,
            Shape::Rect {
                bounds,
                corner_radii: CornerRadii::uniform(Length::from_mm(0.0))
            }
        );
        assert_eq!(snapshot.style, crate::style_model::Style::default());
    }

    #[test]
    fn path_returns_none_for_a_primitive_and_object_returns_the_primitive_variant() {
        let document = Document::new(1);
        let id = document.create_rect(rect_bounds(0.0, 0.0, 10.0, 10.0));
        assert_eq!(document.path(id), None);
        assert!(matches!(
            document.object(id),
            Some(ObjectSnapshot::Primitive(_))
        ));
    }

    #[test]
    fn set_rect_bounds_changes_bounds_but_not_corner_radius() {
        let document = Document::new(1);
        let id = document.create_rect(rect_bounds(0.0, 0.0, 10.0, 10.0));
        document
            .set_corner_radius(&[id], Length::from_mm(3.0))
            .expect("set radius");
        document
            .set_rect_bounds(id, rect_bounds(5.0, 5.0, 20.0, 20.0))
            .expect("resize");
        let snapshot = document.primitive(id).expect("exists");
        let Shape::Rect {
            bounds,
            corner_radii,
        } = snapshot.shape
        else {
            panic!("expected a rect");
        };
        assert_eq!(bounds, rect_bounds(5.0, 5.0, 20.0, 20.0));
        assert!((corner_radii.tl.as_mm() - 3.0).abs() < f64::EPSILON);
    }

    /// Acceptance criteria 8, 9: `resize_rect` writes bounds, corner
    /// radius and stroke width all together, in one commit.
    #[test]
    fn resize_rect_writes_bounds_radius_and_stroke_width_in_one_commit() {
        let document = Document::new(1);
        let id = document.create_rect(rect_bounds(0.0, 0.0, 10.0, 10.0));
        let before = document.loro().len_changes();
        document
            .resize_rect(
                id,
                rect_bounds(0.0, 0.0, 20.0, 20.0),
                CornerRadii::uniform(Length::from_mm(2.0)),
                Some(Length::from_mm(0.5)),
            )
            .expect("resize");
        let after = document.loro().len_changes();
        assert_eq!(after - before, 1, "one commit");
        let snapshot = document.primitive(id).expect("exists");
        let Shape::Rect {
            bounds,
            corner_radii,
        } = snapshot.shape
        else {
            panic!("expected rect");
        };
        assert!((bounds.width.as_mm() - 20.0).abs() < 1e-9);
        assert!((corner_radii.tl.as_mm() - 2.0).abs() < 1e-9);
        assert!((snapshot.style.stroke.width.as_mm() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn resize_ellipse_writes_frame_and_stroke_width() {
        let document = Document::new(1);
        let id = document.create_ellipse(EllipseFrame {
            center: Point::new(0.0, 0.0),
            rx: Length::from_mm(5.0),
            ry: Length::from_mm(5.0),
        });
        document
            .resize_ellipse(
                id,
                EllipseFrame {
                    center: Point::new(0.0, 0.0),
                    rx: Length::from_mm(10.0),
                    ry: Length::from_mm(8.0),
                },
                Some(Length::from_mm(0.6)),
            )
            .expect("resize");
        let snapshot = document.primitive(id).expect("exists");
        let Shape::Ellipse { frame, .. } = snapshot.shape else {
            panic!("expected ellipse");
        };
        assert!((frame.rx.as_mm() - 10.0).abs() < 1e-9);
        assert!((snapshot.style.stroke.width.as_mm() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn resize_star_frame_writes_frame_and_stroke_width_keeps_ratio() {
        let document = Document::new(1);
        let frame = StarFrame {
            center: Point::new(0.0, 0.0),
            radius: Length::from_mm(10.0),
            angle: Angle::from_radians(0.0),
        };
        let id = document.create_star(
            frame,
            PointCount::new(5).unwrap(),
            InnerRatio::new(0.5).unwrap(),
        );
        let new_frame = StarFrame {
            radius: Length::from_mm(15.0),
            ..frame
        };
        document
            .resize_star_frame(id, new_frame, Some(Length::from_mm(0.4)))
            .expect("resize");
        let snapshot = document.primitive(id).expect("exists");
        let Shape::Star {
            frame: resized_frame,
            inner_ratio,
            ..
        } = snapshot.shape
        else {
            panic!("expected star");
        };
        assert!((resized_frame.radius.as_mm() - 15.0).abs() < 1e-9);
        assert!((inner_ratio.get() - 0.5).abs() < 1e-9, "ratio untouched");
        assert!((snapshot.style.stroke.width.as_mm() - 0.4).abs() < 1e-9);
    }

    /// Two peers open the same one-rectangle document; returns them and
    /// the rectangle's id.
    fn two_peers() -> (Document, Document, NodeId) {
        let a = Document::new(1);
        let id = a.create_rect(rect_bounds(0.0, 0.0, 10.0, 10.0));
        let b = Document::from_loro_snapshot(2, &a.export_loro_snapshot().expect("snapshot"))
            .expect("peer B opens it");
        (a, b, id)
    }

    fn merge(a: &Document, b: &Document) {
        let from_b = b.loro().export(loro::ExportMode::all_updates()).expect("B");
        let from_a = a.loro().export(loro::ExportMode::all_updates()).expect("A");
        a.loro().import(&from_b).expect("A merges B");
        b.loro().import(&from_a).expect("B merges A");
    }

    fn set_stroke_directly(doc: &Document, id: NodeId, mm: f64) {
        let tree = doc.loro().get_tree(OBJECTS_TREE);
        let meta = tree.get_meta(tree_id_of(id)).expect("meta");
        meta.insert(crate::style_codec::KEY_STROKE_WIDTH, mm)
            .expect("insert");
        doc.commit_with_label("test: peer stroke edit");
    }

    /// AC 8/29 + adrs.md: a resize with the stroke width `None` does not
    /// write the key at all, so a peer's concurrent stroke edit survives
    /// — also when the peer's value differs from the drag-start snapshot.
    #[test]
    fn resize_with_no_stroke_width_leaves_a_peers_concurrent_stroke_edit_alone() {
        let (a, b, id) = two_peers();
        a.resize_rect(
            id,
            rect_bounds(0.0, 0.0, 30.0, 10.0),
            CornerRadii::uniform(Length::from_mm(0.0)),
            None,
        )
        .expect("A resizes, switch off");
        set_stroke_directly(&b, id, 2.0);
        merge(&a, &b);
        for peer in [&a, &b] {
            let p = peer.primitive(id).expect("exists");
            assert!(
                (p.style.stroke.width.as_mm() - 2.0).abs() < 1e-12,
                "peer edit kept"
            );
            let Shape::Rect { bounds, .. } = p.shape else {
                panic!("rect");
            };
            assert!((bounds.width.as_mm() - 30.0).abs() < 1e-12, "resize kept");
        }
    }

    /// With the switch on, a `Some` equal to the stored width (a stroke the
    /// resize did not change) and a radius equal to the stored one are not
    /// rewritten either — so peers' concurrent edits to them survive.
    #[test]
    fn resize_does_not_rewrite_an_unchanged_stroke_width_or_corner_radius() {
        let (a, b, id) = two_peers();
        a.resize_rect(
            id,
            rect_bounds(0.0, 0.0, 30.0, 10.0),
            CornerRadii::uniform(Length::from_mm(0.0)),
            Some(Length::from_mm(crate::style_model::DEFAULT_STROKE_WIDTH_MM)),
        )
        .expect("A resizes");
        set_stroke_directly(&b, id, 2.0);
        b.set_corner_radius(&[id], Length::from_mm(3.0))
            .expect("B radius");
        merge(&a, &b);
        for peer in [&a, &b] {
            let p = peer.primitive(id).expect("exists");
            assert!((p.style.stroke.width.as_mm() - 2.0).abs() < 1e-12);
            let Shape::Rect { corner_radii, .. } = p.shape else {
                panic!("rect");
            };
            assert!((corner_radii.tl.as_mm() - 3.0).abs() < 1e-12);
        }
    }

    /// A changed width is written (switch on).
    #[test]
    fn resize_with_a_new_stroke_width_writes_it() {
        let document = Document::new(1);
        let id = document.create_rect(rect_bounds(0.0, 0.0, 10.0, 10.0));
        document
            .resize_rect(
                id,
                rect_bounds(0.0, 0.0, 20.0, 20.0),
                CornerRadii::uniform(Length::from_mm(0.0)),
                Some(Length::from_mm(0.5)),
            )
            .expect("resize");
        assert!((document.primitive(id).unwrap().style.stroke.width.as_mm() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn resize_rect_on_an_ellipse_is_refused() {
        let document = Document::new(1);
        let id = document.create_ellipse(EllipseFrame {
            center: Point::new(0.0, 0.0),
            rx: Length::from_mm(5.0),
            ry: Length::from_mm(5.0),
        });
        let result = document.resize_rect(
            id,
            rect_bounds(0.0, 0.0, 1.0, 1.0),
            CornerRadii::uniform(Length::from_mm(0.0)),
            Some(Length::from_mm(0.25)),
        );
        assert_eq!(result, Err(ShapeEditError::WrongShape));
    }

    #[test]
    fn set_corner_radius_on_an_ellipse_is_refused() {
        let document = Document::new(1);
        let id = document.create_ellipse(EllipseFrame {
            center: Point::new(0.0, 0.0),
            rx: Length::from_mm(5.0),
            ry: Length::from_mm(5.0),
        });
        let result = document.set_corner_radius(&[id], Length::from_mm(1.0));
        assert_eq!(result, Err(ShapeEditError::WrongShape));
    }

    #[test]
    fn set_inner_ratio_on_a_polygon_is_refused() {
        let document = Document::new(1);
        let frame = StarFrame {
            center: Point::new(0.0, 0.0),
            radius: Length::from_mm(10.0),
            angle: Angle::from_radians(0.0),
        };
        let id = document.create_polygon(frame, PointCount::new(5).unwrap());
        let result = document.set_inner_ratio(&[id], InnerRatio::new(0.5).unwrap());
        assert_eq!(result, Err(ShapeEditError::WrongShape));
    }

    #[test]
    fn set_point_count_keeps_frame_and_ratio_on_a_star() {
        let document = Document::new(1);
        let frame = StarFrame {
            center: Point::new(1.0, 2.0),
            radius: Length::from_mm(10.0),
            angle: Angle::from_radians(0.0),
        };
        let id = document.create_star(
            frame,
            PointCount::new(5).unwrap(),
            InnerRatio::new(0.5).unwrap(),
        );
        document
            .set_point_count(&[id], PointCount::new(7).unwrap())
            .expect("set count");
        let snapshot = document.primitive(id).expect("exists");
        let Shape::Star {
            frame: new_frame,
            point_count,
            inner_ratio,
            ..
        } = snapshot.shape
        else {
            panic!("expected a star");
        };
        assert_eq!(new_frame, frame, "AC15: size/orientation unchanged");
        assert_eq!(point_count.get(), 7);
        assert!(
            (inner_ratio.get() - 0.5).abs() < f64::EPSILON,
            "AC15: ratio unchanged"
        );
    }

    #[test]
    fn operations_on_an_unknown_id_are_refused() {
        let document = Document::new(1);
        let id = document.create_rect(rect_bounds(0.0, 0.0, 1.0, 1.0));
        document
            .convert_to_paths(&[(
                id,
                vec![
                    NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                    NewAnchor::corner(AnchorId::new(1, 2), Point::new(1.0, 0.0)),
                ],
            )])
            .expect("convert");
        let result = document.set_rect_bounds(id, rect_bounds(0.0, 0.0, 2.0, 2.0));
        assert_eq!(result, Err(ShapeEditError::NotAPrimitive));
    }

    /// AC17: converting replaces the primitive with a path keeping the
    /// same id; the shape's own parameters no longer exist anywhere.
    #[test]
    fn convert_to_paths_keeps_the_node_id_and_removes_shape_params() {
        let document = Document::new(1);
        let id = document.create_rect(rect_bounds(0.0, 0.0, 10.0, 10.0));
        let anchors = vec![
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 3), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(1, 4), Point::new(0.0, 10.0)),
        ];
        document
            .convert_to_paths(&[(id, anchors.clone())])
            .expect("convert");

        assert_eq!(document.primitive(id), None, "no shape params remain");
        let path = document.path(id).expect("now a path, same id");
        assert!(path.closed);
        assert_eq!(path.anchors.len(), 4);
        assert!(path.anchors.iter().all(|a| a.kind == AnchorKind::Corner));
    }

    /// AC22: two primitives selected together convert independently in
    /// one call, each ending up a path with its own anchors.
    #[test]
    fn convert_to_paths_converts_a_multi_selection_independently() {
        let document = Document::new(1);
        let rect_id = document.create_rect(rect_bounds(0.0, 0.0, 10.0, 10.0));
        let ellipse_id = document.create_ellipse(EllipseFrame {
            center: Point::new(0.0, 0.0),
            rx: Length::from_mm(5.0),
            ry: Length::from_mm(5.0),
        });
        let rect_anchors = vec![
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 0.0)),
        ];
        let ellipse_anchors = vec![
            NewAnchor::corner(AnchorId::new(1, 3), Point::new(5.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 4), Point::new(-5.0, 0.0)),
        ];
        document
            .convert_to_paths(&[(rect_id, rect_anchors), (ellipse_id, ellipse_anchors)])
            .expect("convert both");
        assert_eq!(document.path(rect_id).expect("exists").anchors.len(), 2);
        assert_eq!(document.path(ellipse_id).expect("exists").anchors.len(), 2);
    }

    /// `convert_to_paths` refuses the whole batch, writing nothing, when
    /// one id in it is stale.
    #[test]
    fn convert_to_paths_refuses_the_whole_batch_on_one_stale_id() {
        let document = Document::new(1);
        let id = document.create_rect(rect_bounds(0.0, 0.0, 10.0, 10.0));
        let unknown = {
            let other = document.create_rect(rect_bounds(0.0, 0.0, 1.0, 1.0));
            document
                .convert_to_paths(&[(
                    other,
                    vec![
                        NewAnchor::corner(AnchorId::new(1, 9), Point::new(0.0, 0.0)),
                        NewAnchor::corner(AnchorId::new(1, 10), Point::new(1.0, 1.0)),
                    ],
                )])
                .expect("convert the throwaway");
            other
        };
        let result = document.convert_to_paths(&[
            (
                id,
                vec![
                    NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
                    NewAnchor::corner(AnchorId::new(1, 2), Point::new(1.0, 1.0)),
                ],
            ),
            (unknown, vec![]),
        ]);
        assert_eq!(result, Err(ShapeEditError::NotAPrimitive));
        assert!(
            document.primitive(id).is_some(),
            "untouched by the refused batch"
        );
    }

    /// Re-verification (tester, item 3): a batch corner-radius edit over
    /// several selected rectangles must cost exactly one Loro commit for
    /// the whole call, not one per object — `Document::loro().
    /// len_changes()` counts committed changes directly, which is a
    /// stronger check than comparing exported-snapshot byte counts.
    #[test]
    fn set_corner_radius_batch_is_one_commit_not_one_per_object() {
        let document = Document::new(1);
        let ids: Vec<_> = (0..5)
            .map(|i| document.create_rect(rect_bounds(f64::from(i) * 20.0, 0.0, 10.0, 10.0)))
            .collect();
        let before = document.loro().len_changes();
        document
            .set_corner_radius(&ids, Length::from_mm(2.0))
            .expect("batch radius");
        let after = document.loro().len_changes();
        assert_eq!(
            after - before,
            1,
            "5-object batch must add exactly 1 change, not 5"
        );
        for id in ids {
            let Shape::Rect { corner_radii, .. } = document.primitive(id).unwrap().shape else {
                panic!("expected rect");
            };
            assert!((corner_radii.tl.as_mm() - 2.0).abs() < 1e-9);
        }
    }

    /// Same check for the point-count stepper applied to a multi-
    /// selection of stars (acceptance criteria 10, 15).
    #[test]
    fn set_point_count_batch_is_one_commit_not_one_per_object() {
        let document = Document::new(1);
        let frame = StarFrame {
            center: Point::new(0.0, 0.0),
            radius: Length::from_mm(10.0),
            angle: Angle::from_radians(0.0),
        };
        let ids: Vec<_> = (0..4)
            .map(|_| {
                document.create_star(
                    frame,
                    PointCount::new(5).unwrap(),
                    InnerRatio::new(0.5).unwrap(),
                )
            })
            .collect();
        let before = document.loro().len_changes();
        document
            .set_point_count(&ids, PointCount::new(9).unwrap())
            .expect("batch point count");
        let after = document.loro().len_changes();
        assert_eq!(
            after - before,
            1,
            "4-object batch must add exactly 1 change, not 4"
        );
    }

    /// Same check for `convert_to_paths` (AC22): converting several
    /// selected primitives together is one commit, not one per object.
    #[test]
    fn convert_to_paths_batch_is_one_commit_not_one_per_object() {
        let document = Document::new(1);
        let ids: Vec<_> = (0..3)
            .map(|i| document.create_rect(rect_bounds(f64::from(i) * 20.0, 0.0, 10.0, 10.0)))
            .collect();
        let conversions: Vec<_> = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                let base = u64::try_from(i).unwrap() * 10;
                (
                    id,
                    vec![
                        NewAnchor::corner(AnchorId::new(1, base + 1), Point::new(0.0, 0.0)),
                        NewAnchor::corner(AnchorId::new(1, base + 2), Point::new(10.0, 0.0)),
                    ],
                )
            })
            .collect();
        let before = document.loro().len_changes();
        document.convert_to_paths(&conversions).expect("convert");
        let after = document.loro().len_changes();
        assert_eq!(
            after - before,
            1,
            "3-object batch conversion must add exactly 1 change, not 3"
        );
        for id in ids {
            assert!(document.path(id).is_some());
        }
    }

    /// The stored `rotation` key of `id`, if present (`specs/0052-click-to-create-shapes/adrs.md`
    /// decision 2: written only when not zero).
    fn stored_rotation(document: &Document, id: NodeId) -> Option<loro::LoroValue> {
        let tree = document.loro().get_tree(OBJECTS_TREE);
        let meta = tree.get_meta(tree_id_of(id)).expect("meta");
        meta.get(crate::path_codec::KEY_ROTATION)
            .map(|v| v.get_deep_value())
    }

    fn star_frame() -> StarFrame {
        StarFrame {
            center: Point::new(100.0, 100.0),
            radius: Length::from_mm(25.0),
            angle: Angle::from_radians(-std::f64::consts::FRAC_PI_2),
        }
    }

    /// One shape of each kind, as `create_primitive` takes it.
    fn one_of_each_kind() -> [(Shape, &'static str); 4] {
        let point_count = PointCount::new(6).expect("valid");
        [
            (
                Shape::Rect {
                    bounds: rect_bounds(100.0, 30.0, 40.0, 20.0),
                    corner_radii: CornerRadii::uniform(Length::from_mm(0.0)),
                },
                "create_rect",
            ),
            (
                Shape::Ellipse {
                    frame: EllipseFrame {
                        center: Point::new(60.0, 60.0),
                        rx: Length::from_mm(10.0),
                        ry: Length::from_mm(10.0),
                    },
                    arc: None,
                    curve: crate::Curve::ELLIPSE_DEFAULT,
                },
                "create_ellipse",
            ),
            (
                Shape::Polygon {
                    frame: star_frame(),
                    point_count,
                    curve: crate::Curve::POLYGON_DEFAULT,
                },
                "create_polygon",
            ),
            (
                Shape::Star {
                    frame: star_frame(),
                    point_count,
                    inner_ratio: InnerRatio::new(0.5).expect("valid"),
                    curve: crate::Curve::POLYGON_DEFAULT,
                },
                "create_star",
            ),
        ]
    }

    /// 0052 criteria 6 and 36: with no rotation, `create_primitive` writes exactly what the
    /// kind's `create_*` writes (the same object read back, no `rotation` key) as one step with
    /// the kind's existing label.
    #[test]
    fn create_primitive_without_rotation_writes_what_create_star_and_its_siblings_write() {
        for (shape, label) in one_of_each_kind() {
            let placed = Document::new(1);
            let id = placed.create_primitive(shape, Angle::from_radians(0.0));
            let dragged = Document::new(1);
            let reference = match shape {
                Shape::Rect { bounds, .. } => dragged.create_rect(bounds),
                Shape::Ellipse { frame, .. } => dragged.create_ellipse(frame),
                Shape::Polygon {
                    frame, point_count, ..
                } => dragged.create_polygon(frame, point_count),
                Shape::Star {
                    frame,
                    point_count,
                    inner_ratio,
                    ..
                } => dragged.create_star(frame, point_count, inner_ratio),
            };
            assert_eq!(placed.object(id), dragged.object(reference), "{label}");
            assert_eq!(stored_rotation(&placed, id), None, "{label}");
            let steps = placed.own_steps();
            assert_eq!(steps.len(), 1, "{label}: one commit, one step");
            assert_eq!(steps[0].label, label);
        }
    }

    /// 0052 criterion 16 (b): a rectangle placed turned by 90° stores its unturned box and the
    /// `rotation` register, in the same single step.
    #[test]
    fn create_primitive_stores_a_non_zero_rotation_in_the_same_step() {
        let document = Document::new(1);
        let bounds = rect_bounds(90.0, 60.0, 40.0, 20.0);
        let shape = Shape::Rect {
            bounds,
            corner_radii: CornerRadii::uniform(Length::from_mm(0.0)),
        };
        let id = document.create_primitive(shape, Angle::from_radians(std::f64::consts::FRAC_PI_2));
        let snapshot = document.primitive(id).expect("exists");
        assert_eq!(snapshot.shape, shape);
        assert!((snapshot.rotation.as_radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!(stored_rotation(&document, id).is_some());
        let steps = document.own_steps();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].label, "create_rect");
    }

    /// The register is written normalized to (-π, π], as every other writer does; a full turn
    /// reads as no turn.
    #[test]
    fn create_primitive_normalizes_the_rotation() {
        let document = Document::new(1);
        let frame = EllipseFrame {
            center: Point::new(40.0, 60.0),
            rx: Length::from_mm(10.0),
            ry: Length::from_mm(5.0),
        };
        let id = document.create_primitive(
            Shape::Ellipse {
                frame,
                arc: None,
                curve: crate::Curve::ELLIPSE_DEFAULT,
            },
            Angle::from_radians(450_f64.to_radians()),
        );
        let rotation = document.primitive(id).expect("exists").rotation;
        assert!((rotation.as_radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }

    /// A rectangle's own corner radii are written as the shape carries them.
    #[test]
    fn create_primitive_keeps_a_rectangles_corner_radii() {
        let document = Document::new(1);
        let shape = Shape::Rect {
            bounds: rect_bounds(0.0, 0.0, 40.0, 20.0),
            corner_radii: CornerRadii::uniform(Length::from_mm(3.0)),
        };
        let id = document.create_primitive(shape, Angle::from_radians(0.0));
        assert_eq!(document.primitive(id).expect("exists").shape, shape);
    }
}
