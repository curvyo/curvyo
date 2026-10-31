//! Document commands that write an ellipse's arc and a shape's curve
//! (`specs/0021-ellipse-arcs-and-shaping/adrs.md` decision 5), split out of
//! [`crate::shapes`] so that module stays under the size limit. Each writes a register
//! only when its value changes and commits nothing when none does.

use crate::document::Document;
use crate::ellipse_arc::EllipseArc;
use crate::path_model::NodeId;
use crate::primitive_model::{EllipseFrame, Shape};
use crate::ring_curve::Curve;
use crate::shape_arc_codec::{write_arc_if_changed, write_curve_if_changed};
use crate::shape_codec::SHAPE_ELLIPSE;
use crate::shapes::ShapeEditError;
use crate::units::Length;

impl Document {
    /// The shown angle of each object `conversions` names, and whether the path it becomes
    /// is closed: an open Arc becomes an open path (criterion 13), every other primitive a
    /// closed one. The read "Object to path" makes before it strips the primitive keys.
    ///
    /// # Errors
    /// [`ShapeEditError::NoSuchObject`] if an id no longer exists.
    pub(crate) fn conversion_reads(
        &self,
        conversions: &[(NodeId, Vec<crate::path_model::NewAnchor>)],
    ) -> Result<Vec<(crate::units::Angle, bool)>, ShapeEditError> {
        conversions
            .iter()
            .map(|(id, _)| {
                self.object(*id).map(|o| {
                    let closed = match &o {
                        crate::ObjectSnapshot::Primitive(p) => crate::outline_of(&p.shape).closed,
                        crate::ObjectSnapshot::Sketch(_)
                        | crate::ObjectSnapshot::Path(_)
                        | crate::ObjectSnapshot::Clone(_)
                        | crate::ObjectSnapshot::Text(_)
                        | crate::ObjectSnapshot::Image(_) => true,
                    };
                    (o.orientation(), closed)
                })
            })
            .collect::<Option<_>>()
            .ok_or(ShapeEditError::NoSuchObject)
    }

    /// [`Document::resize_ellipse`] that also writes the arc the resize gives (a resize
    /// through zero mirrors it, `specs/0021-ellipse-arcs-and-shaping/` criterion 54), as one
    /// commit with the same label. The arc register is written only when it changes.
    ///
    /// # Errors
    /// As [`Document::resize_ellipse`]; [`ShapeEditError::ArcWithCurve`] for an arc on an
    /// ellipse whose curve is not 100 %.
    pub fn resize_ellipse_with_arc(
        &self,
        id: NodeId,
        frame: EllipseFrame,
        arc: Option<EllipseArc>,
        stroke_width: Option<Length>,
    ) -> Result<(), ShapeEditError> {
        crate::shapes::check_stroke_width(stroke_width)?;
        let meta = self.require_shape(id, SHAPE_ELLIPSE)?;
        if arc.is_some()
            && let Some(Shape::Ellipse { curve, .. }) = self.primitive(id).map(|p| p.shape)
            && curve != Curve::ELLIPSE_DEFAULT
        {
            return Err(ShapeEditError::ArcWithCurve);
        }
        crate::shape_codec::write_ellipse_frame(&meta, frame);
        write_arc_if_changed(&meta, arc);
        crate::shapes::write_stroke_width_if_changed(&meta, stroke_width);
        self.commit_with_label("resize_ellipse");
        Ok(())
    }

    /// Sets each named ellipse's arc (`None`: the whole ellipse) as **one commit** for the
    /// whole batch, labelled `set_ellipse_arc` ("Change arc", criterion 59). Every id is
    /// confirmed to be an ellipse whose curve is 100 % before any is written.
    ///
    /// # Errors
    /// [`ShapeEditError::NoSuchObject`], [`ShapeEditError::NotAPrimitive`] or
    /// [`ShapeEditError::WrongShape`] if an id is not an ellipse;
    /// [`ShapeEditError::ArcWithCurve`] for an arc on an ellipse whose curve is not 100 %.
    pub fn set_ellipse_arcs(
        &self,
        arcs: &[(NodeId, Option<EllipseArc>)],
    ) -> Result<(), ShapeEditError> {
        let metas: Vec<_> = arcs
            .iter()
            .map(|&(id, arc)| {
                let meta = self.require_shape(id, SHAPE_ELLIPSE)?;
                if arc.is_some()
                    && let Some(Shape::Ellipse { curve, .. }) = self.primitive(id).map(|p| p.shape)
                    && curve != Curve::ELLIPSE_DEFAULT
                {
                    return Err(ShapeEditError::ArcWithCurve);
                }
                Ok((meta, arc))
            })
            .collect::<Result<_, _>>()?;
        let mut changed = false;
        for (meta, arc) in metas {
            changed |= write_arc_if_changed(&meta, arc);
        }
        if changed {
            self.commit_with_label("set_ellipse_arc");
        }
        Ok(())
    }

    /// Sets each named ellipse's, polygon's or star's stored curve as **one commit** for
    /// the whole batch, labelled `set_curve` ("Change curve", criterion 59). The value is
    /// stored raw (a polygon or star below its limit keeps it, criterion 38); the shape's
    /// default deletes the register.
    ///
    /// # Errors
    /// [`ShapeEditError::NoSuchObject`] or [`ShapeEditError::NotAPrimitive`] if an id is
    /// not a primitive, [`ShapeEditError::WrongShape`] for a rectangle;
    /// [`ShapeEditError::ArcWithCurve`] for a curve other than 100 % on an ellipse with
    /// an arc.
    pub fn set_curves(&self, curves: &[(NodeId, Curve)]) -> Result<(), ShapeEditError> {
        let metas: Vec<_> = curves
            .iter()
            .map(|&(id, curve)| {
                let meta = self.require_primitive(id)?;
                let shape = self
                    .primitive(id)
                    .map(|p| p.shape)
                    .ok_or(ShapeEditError::NotAPrimitive)?;
                let default = match shape {
                    Shape::Ellipse { arc: Some(_), .. } if curve != Curve::ELLIPSE_DEFAULT => {
                        return Err(ShapeEditError::ArcWithCurve);
                    }
                    Shape::Ellipse { .. } => Curve::ELLIPSE_DEFAULT,
                    Shape::Polygon { .. } | Shape::Star { .. } => Curve::POLYGON_DEFAULT,
                    Shape::Rect { .. } => return Err(ShapeEditError::WrongShape),
                };
                Ok((meta, curve, default))
            })
            .collect::<Result<_, _>>()?;
        let mut changed = false;
        for (meta, curve, default) in metas {
            changed |= write_curve_if_changed(&meta, curve, default);
        }
        if changed {
            self.commit_with_label("set_curve");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Document;
    use crate::ellipse_arc::{ArcKind, EllipseArc};
    use crate::primitive_model::{EllipseFrame, PointCount, Shape, StarFrame};
    use crate::ring_curve::Curve;
    use crate::shapes::ShapeEditError;
    use crate::units::{Angle, Length, Point};

    fn frame() -> EllipseFrame {
        EllipseFrame {
            center: Point::new(10.0, 10.0),
            rx: Length::from_mm(20.0),
            ry: Length::from_mm(20.0),
        }
    }

    fn half() -> EllipseArc {
        EllipseArc::new(
            Angle::from_radians(0.0),
            Angle::from_radians(std::f64::consts::PI),
            ArcKind::Pie,
        )
        .unwrap()
    }

    /// Criteria 3, 14 and 59: one commit, nothing when unchanged, "whole" deletes,
    /// a save and open keeps the arc.
    #[test]
    fn set_ellipse_arcs_commits_once_and_round_trips() {
        let doc = Document::new(1);
        let id = doc.create_ellipse(frame());
        let before = doc.version();
        doc.set_ellipse_arcs(&[(id, Some(half()))]).unwrap();
        let after = doc.version();
        assert_ne!(before, after);
        doc.set_ellipse_arcs(&[(id, Some(half()))]).unwrap();
        assert_eq!(doc.version(), after, "unchanged writes nothing");
        let reopened = crate::unpack(2, &crate::pack(&doc, "test").unwrap()).unwrap();
        let Some(Shape::Ellipse { arc, .. }) = reopened.primitive(id).map(|p| p.shape) else {
            panic!("an ellipse")
        };
        assert_eq!(arc.unwrap().kind, ArcKind::Pie);
        doc.set_ellipse_arcs(&[(id, None)]).unwrap();
        let Some(Shape::Ellipse { arc, .. }) = doc.primitive(id).map(|p| p.shape) else {
            panic!("an ellipse")
        };
        assert!(arc.is_none());
    }

    /// Criteria 4 and 13: an open Arc's outline is open and converts to an open path of
    /// `ceil(S / 90°)` cubics; a Pie converts closed with the centre.
    #[test]
    fn an_open_arc_converts_to_an_open_path() {
        use crate::path_model::{AnchorId, NewAnchor};
        let doc = Document::new(1);
        let id = doc.create_ellipse(frame());
        let arc = EllipseArc::new(
            Angle::from_radians(0.0),
            Angle::from_radians(200_f64.to_radians()),
            ArcKind::Arc,
        );
        doc.set_ellipse_arcs(&[(id, arc)]).unwrap();
        let shape = doc.primitive(id).unwrap().shape;
        let outline = crate::outline_of(&shape);
        assert!(!outline.closed);
        assert_eq!(outline.len(), 4, "200° is three cubics");
        let anchors: Vec<NewAnchor> = outline
            .iter()
            .enumerate()
            .map(|(i, a)| NewAnchor {
                id: AnchorId::new(1, 1000 + u64::try_from(i).unwrap()),
                point: a.point,
                handle_in: a.handle_in,
                handle_out: a.handle_out,
                kind: a.kind,
            })
            .collect();
        doc.convert_to_paths(&[(id, anchors)]).unwrap();
        assert!(!doc.path(id).unwrap().closed);
    }

    /// Decision 1: a concurrent set and delete of the arc register resolve by
    /// last-writer-wins, and the object reads as a valid ellipse either way.
    #[test]
    fn a_concurrent_set_and_delete_resolve_by_last_writer_wins() {
        let a = Document::new(1);
        let id = a.create_ellipse(frame());
        a.set_ellipse_arcs(&[(id, Some(half()))]).unwrap();
        let b = Document::from_loro_snapshot(2, &a.export_loro_snapshot().unwrap()).unwrap();
        a.set_ellipse_arcs(&[(id, None)]).unwrap();
        let other = EllipseArc::new(
            Angle::from_radians(0.5),
            Angle::from_radians(2.0),
            ArcKind::Chord,
        );
        b.set_ellipse_arcs(&[(id, other)]).unwrap();
        let from_b = b.loro().export(loro::ExportMode::all_updates()).unwrap();
        let from_a = a.loro().export(loro::ExportMode::all_updates()).unwrap();
        a.loro().import(&from_b).unwrap();
        b.loro().import(&from_a).unwrap();
        let read = |d: &Document| match d.primitive(id).map(|p| p.shape) {
            Some(Shape::Ellipse { arc, .. }) => arc,
            other => panic!("an ellipse: {other:?}"),
        };
        assert_eq!(read(&a), read(&b), "both peers agree");
        let reopened = crate::unpack(3, &crate::pack(&a, "test").unwrap()).unwrap();
        assert_eq!(read(&reopened), read(&a), "the merged state opens");
    }

    /// Criterion 57: a copy has the same arc and is the same kind of primitive.
    #[test]
    fn a_copy_keeps_the_arc() {
        let doc = Document::new(1);
        let id = doc.create_ellipse(frame());
        doc.set_ellipse_arcs(&[(id, Some(half()))]).unwrap();
        let copies = doc
            .duplicate_objects(
                &[crate::CopySource {
                    id,
                    anchor_ids: Vec::new(),
                }],
                crate::units::Vec2::new(5.0, 0.0),
            )
            .unwrap();
        let Some(Shape::Ellipse { arc, .. }) = doc.primitive(copies[0]).map(|p| p.shape) else {
            panic!("an ellipse")
        };
        assert_eq!(arc, Some(half()));
    }

    /// Criterion 31: arc and curve never coexist.
    #[test]
    fn arc_and_curve_are_refused_together() {
        let doc = Document::new(1);
        let id = doc.create_ellipse(frame());
        doc.set_curves(&[(id, Curve::new(0.0).unwrap())]).unwrap();
        assert_eq!(
            doc.set_ellipse_arcs(&[(id, Some(half()))]),
            Err(ShapeEditError::ArcWithCurve)
        );
        doc.set_curves(&[(id, Curve::ELLIPSE_DEFAULT)]).unwrap();
        doc.set_ellipse_arcs(&[(id, Some(half()))]).unwrap();
        assert_eq!(
            doc.set_curves(&[(id, Curve::new(0.5).unwrap())]),
            Err(ShapeEditError::ArcWithCurve)
        );
    }

    /// Criteria 32, 46 and 50: a polygon's stored curve survives a save and open and a
    /// change of Points; 0 deletes it.
    #[test]
    fn a_polygon_curve_is_stored_raw() {
        let doc = Document::new(1);
        let id = doc.create_polygon(
            StarFrame {
                center: Point::new(0.0, 0.0),
                radius: Length::from_mm(20.0),
                angle: Angle::from_radians(0.0),
            },
            PointCount::new(6).unwrap(),
        );
        doc.set_curves(&[(id, Curve::new(-0.8).unwrap())]).unwrap();
        doc.set_point_count(&[id], PointCount::new(3).unwrap())
            .unwrap();
        let reopened = crate::unpack(2, &crate::pack(&doc, "test").unwrap()).unwrap();
        let Some(Shape::Polygon { curve, .. }) = reopened.primitive(id).map(|p| p.shape) else {
            panic!("a polygon")
        };
        assert_eq!(curve, Curve::new(-0.8).unwrap());
        doc.set_curves(&[(id, Curve::POLYGON_DEFAULT)]).unwrap();
    }
}
