//! What a press of the Measure tool picks (`specs/0070-measure-and-dimension` criteria 3, 6 and 15,
//! UX notes §2): a point feature (node, segment midpoint, shape centre, corner, vertex or ellipse
//! axis end), else a curve outline (circle, circular arc, rounded corner), else a straight
//! segment or primitive edge, else a free point.

use curvyo_document_core::{
    AnchorId, Angle, CurveSlot, Length, NodeId, ObjectSnapshot, PathSnapshot, Point, PointSlot,
    PrimitiveSnapshot, Shape, ShapeFeature, arc_circle, cubic_at, effective_corner_radii,
    outline_of_rotated, shape_point,
};

/// One pick of the Measure tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeasurePick {
    /// A point: linked to a feature, or free.
    Point {
        /// What it refers to.
        slot: PointSlot,
        /// Where it is now, in document millimetres.
        at: Point,
    },
    /// A circle, arc or rounded corner.
    Curve {
        /// What it refers to.
        slot: CurveSlot,
        /// Its centre.
        center: Point,
        /// Its radius.
        radius: Length,
        /// The drawn part: the start angle and the signed sweep (a whole circle is `(0, TAU)`),
        /// for the hover band (UX notes §6).
        arc: (Angle, Angle),
    },
    /// A straight segment, by the point slots of its two ends.
    Segment {
        /// The start.
        start: (PointSlot, Point),
        /// The end.
        end: (PointSlot, Point),
    },
}

/// Below this length (mm) a segment has no direction.
const DEGENERATE_MM: f64 = 1e-9;

fn anchors_of(object: &ObjectSnapshot) -> Option<&PathSnapshot> {
    match object {
        ObjectSnapshot::Path(path) => Some(path),
        ObjectSnapshot::Clone(clone) => Some(&clone.resolved),
        _ => None,
    }
}

/// Every point feature of `object` with its slot.
fn point_features(object: &ObjectSnapshot) -> Vec<(PointSlot, Point)> {
    let node = object.id();
    if let Some(path) = anchors_of(object) {
        let mut out = Vec::new();
        for outline in path.subpaths() {
            let count = outline.anchors.len();
            for (i, anchor) in outline.anchors.iter().enumerate() {
                out.push((
                    PointSlot::Anchor {
                        node,
                        anchor: anchor.id,
                    },
                    anchor.point,
                ));
                let next = if i + 1 < count {
                    Some(i + 1)
                } else if outline.closed && count > 1 {
                    Some(0)
                } else {
                    None
                };
                if let Some(next) = next {
                    out.push((
                        PointSlot::SegmentMid {
                            node,
                            anchor: anchor.id,
                        },
                        cubic_at(*anchor, outline.anchors[next], 0.5),
                    ));
                }
            }
        }
        return out;
    }
    let ObjectSnapshot::Primitive(primitive) = object else {
        return Vec::new();
    };
    let count: u32 = match primitive.shape {
        Shape::Rect { .. } => 4,
        Shape::Polygon { point_count, .. } => point_count.get(),
        Shape::Star { point_count, .. } => 2 * point_count.get(),
        Shape::Ellipse { .. } => 0,
    };
    let mut features = vec![ShapeFeature::Center];
    for i in 0..count {
        features.push(if matches!(primitive.shape, Shape::Rect { .. }) {
            ShapeFeature::Corner(i)
        } else {
            ShapeFeature::Vertex(i)
        });
        features.push(ShapeFeature::EdgeMid(i));
    }
    // A whole ellipse that is no circle: its four axis ends (Proposal, customer feedback on
    // PR #243); a circle keeps its rim for the radius.
    if matches!(primitive.shape, Shape::Ellipse { arc: None, .. })
        && circle_of_ellipse(primitive).is_none()
    {
        features.extend((0..4).map(ShapeFeature::Vertex));
    }
    features
        .into_iter()
        .filter_map(|feature| {
            Some((
                PointSlot::ShapePoint { node, feature },
                shape_point(primitive, feature)?,
            ))
        })
        .collect()
}

/// The curves of `object` near `p` within `tolerance`, with their distance.
fn curve_near(object: &ObjectSnapshot, p: Point, tolerance: f64) -> Option<(f64, MeasurePick)> {
    let node = object.id();
    let near = |slot, center: Point, radius: Length, arc: (Angle, Angle)| {
        let off = (center.vector_to(p).length() - radius.as_mm()).abs();
        (off <= tolerance).then_some((
            off,
            MeasurePick::Curve {
                slot,
                center,
                radius,
                arc,
            },
        ))
    };
    if let Some(path) = anchors_of(object) {
        return path
            .subpaths()
            .flat_map(|outline| {
                let count = outline.anchors.len();
                (0..count).filter_map(move |i| {
                    let next = if i + 1 < count {
                        i + 1
                    } else if outline.closed && count > 1 {
                        0
                    } else {
                        return None;
                    };
                    let (a, b) = (outline.anchors[i], outline.anchors[next]);
                    let (center, radius) = arc_circle(a, b)?;
                    // Only near the drawn part: closer to the curve than to its chord's far side.
                    let mid = cubic_at(a, b, 0.5);
                    (mid.vector_to(p).length() <= a.point.vector_to(b.point).length())
                        .then_some(())?;
                    near(
                        CurveSlot::Arc { node, anchor: a.id },
                        center,
                        radius,
                        arc_through(center, a.point, mid, b.point),
                    )
                })
            })
            .min_by(|x, y| x.0.total_cmp(&y.0));
    }
    let ObjectSnapshot::Primitive(primitive) = object else {
        return None;
    };
    match primitive.shape {
        Shape::Ellipse { .. } => {
            let (center, radius) = circle_of_ellipse(primitive)?;
            near(
                CurveSlot::Circle { node },
                center,
                radius,
                (
                    Angle::from_radians(0.0),
                    Angle::from_radians(std::f64::consts::TAU),
                ),
            )
        }
        Shape::Rect { .. } => round_corner_near(primitive, p, tolerance),
        _ => None,
    }
}

/// The centre and radius of an ellipse primitive that is a circle within
/// [`curvyo_document_core::CIRCLE_TOLERANCE`]; `None` for any other shape.
fn circle_of_ellipse(primitive: &PrimitiveSnapshot) -> Option<(Point, Length)> {
    let Shape::Ellipse { frame, .. } = primitive.shape else {
        return None;
    };
    ((frame.rx.as_mm() - frame.ry.as_mm()).abs() <= curvyo_document_core::CIRCLE_TOLERANCE.as_mm())
        .then_some((frame.center, frame.rx))
}

fn round_corner_near(
    primitive: &PrimitiveSnapshot,
    p: Point,
    tolerance: f64,
) -> Option<(f64, MeasurePick)> {
    let Shape::Rect {
        bounds,
        corner_radii,
    } = primitive.shape
    else {
        return None;
    };
    let radii = effective_corner_radii(bounds, corner_radii);
    let center_of = shape_point(primitive, ShapeFeature::Center)?;
    [radii.tl, radii.tr, radii.br, radii.bl]
        .into_iter()
        .enumerate()
        .filter_map(|(i, radius)| {
            let corner = u32::try_from(i).ok()?;
            let r = radius.as_mm();
            if r <= DEGENERATE_MM {
                return None;
            }
            let tip = shape_point(primitive, ShapeFeature::Corner(corner))?;
            // The corner circle's centre lies `r` inside the corner along both sides.
            let to_center = tip.vector_to(center_of);
            let side_a = shape_point(primitive, ShapeFeature::Corner((corner + 1) % 4))?;
            let side_b = shape_point(primitive, ShapeFeature::Corner((corner + 3) % 4))?;
            let along = |q: Point| tip.vector_to(q).normalized_to(r);
            let center = tip.translated(along(side_a)).translated(along(side_b));
            let middle = center.translated(center.vector_to(tip).normalized_to(r));
            let arc = arc_through(
                center,
                tip.translated(along(side_a)),
                middle,
                tip.translated(along(side_b)),
            );
            let off = (center.vector_to(p).length() - r).abs();
            // Only on the corner's own quarter: on the far side of the centre from the middle.
            let outward = center.vector_to(p);
            let in_quarter = outward.x * to_center.x + outward.y * to_center.y <= 0.0;
            (off <= tolerance && in_quarter).then_some((
                off,
                MeasurePick::Curve {
                    slot: CurveSlot::RoundCorner {
                        node: primitive.id,
                        corner,
                    },
                    center,
                    radius,
                    arc,
                },
            ))
        })
        .min_by(|x, y| x.0.total_cmp(&y.0))
}

/// The start angle and signed sweep of the arc about `center` from `start` through `middle` to
/// `end`.
#[must_use]
pub fn arc_through(center: Point, start: Point, middle: Point, end: Point) -> (Angle, Angle) {
    let angle = |q: Point| {
        let d = center.vector_to(q);
        d.y.atan2(d.x)
    };
    let tau = std::f64::consts::TAU;
    let from = angle(start);
    let ahead = |q: Point| (angle(q) - from).rem_euclid(tau);
    let sweep = ahead(end);
    let sweep = if ahead(middle) <= sweep {
        sweep
    } else {
        sweep - tau
    };
    (Angle::from_radians(from), Angle::from_radians(sweep))
}

fn segment_distance(point: Point, start: Point, end: Point) -> f64 {
    let along = start.vector_to(end);
    let squared = along.x * along.x + along.y * along.y;
    if squared <= DEGENERATE_MM {
        return start.vector_to(point).length();
    }
    let to = start.vector_to(point);
    let fraction = ((to.x * along.x + to.y * along.y) / squared).clamp(0.0, 1.0);
    start
        .translated(along.scaled(fraction))
        .vector_to(point)
        .length()
}

/// The straight edge of a primitive nearest `p` within `tolerance`, between the corners of a
/// rectangle's box or two neighbouring vertices of a polygon or star (customer feedback on
/// PR #243).
fn edge_near(
    primitive: &PrimitiveSnapshot,
    p: Point,
    tolerance: f64,
) -> Option<(f64, MeasurePick)> {
    // Which edges are straight: every side of a rectangle's box; a polygon or star side with no
    // handles (a curved side has no length to measure).
    let (straight, corner): (Vec<bool>, fn(u32) -> ShapeFeature) = match primitive.shape {
        Shape::Rect { .. } => (vec![true; 4], ShapeFeature::Corner),
        Shape::Polygon { .. } | Shape::Star { .. } => {
            let anchors = outline_of_rotated(&primitive.shape, primitive.rotation).anchors;
            let straight = (0..anchors.len())
                .map(|i| {
                    anchors[i].handle_out.length() <= DEGENERATE_MM
                        && anchors[(i + 1) % anchors.len()].handle_in.length() <= DEGENERATE_MM
                })
                .collect();
            (straight, ShapeFeature::Vertex)
        }
        Shape::Ellipse { .. } => return None,
    };
    let count = u32::try_from(straight.len()).ok()?;
    let at = |feature| {
        let slot = PointSlot::ShapePoint {
            node: primitive.id,
            feature,
        };
        shape_point(primitive, feature).map(|point| (slot, point))
    };
    (0..count)
        .zip(straight)
        .filter(|(_, straight)| *straight)
        .filter_map(|(i, _)| {
            let (start, end) = (at(corner(i))?, at(corner((i + 1) % count))?);
            let off = segment_distance(p, start.1, end.1);
            (off <= tolerance && start.1.vector_to(end.1).length() > DEGENERATE_MM)
                .then_some((off, MeasurePick::Segment { start, end }))
        })
        .min_by(|x, y| x.0.total_cmp(&y.0))
}

/// The straight path segment or primitive edge of `object` nearest `p` within `tolerance`.
fn segment_near(object: &ObjectSnapshot, p: Point, tolerance: f64) -> Option<(f64, MeasurePick)> {
    let node = object.id();
    let end = |anchor: AnchorId, at: Point| (PointSlot::Anchor { node, anchor }, at);
    if let ObjectSnapshot::Primitive(primitive) = object {
        return edge_near(primitive, p, tolerance);
    }
    let path = anchors_of(object)?;
    path.subpaths()
        .flat_map(|outline| {
            let count = outline.anchors.len();
            (0..count).filter_map(move |i| {
                let next = if i + 1 < count {
                    i + 1
                } else if outline.closed && count > 1 {
                    0
                } else {
                    return None;
                };
                let (a, b) = (outline.anchors[i], outline.anchors[next]);
                let straight = a.handle_out.length() <= DEGENERATE_MM
                    && b.handle_in.length() <= DEGENERATE_MM
                    && a.point.vector_to(b.point).length() > DEGENERATE_MM;
                let off = segment_distance(p, a.point, b.point);
                (straight && off <= tolerance).then_some((
                    off,
                    MeasurePick::Segment {
                        start: end(a.id, a.point),
                        end: end(b.id, b.point),
                    },
                ))
            })
        })
        .min_by(|x, y| x.0.total_cmp(&y.0))
}

/// What a press at `p` picks among `objects` (the visible leaves; dimensions are never picked):
/// the nearest point feature within `snap` (the capture zone with snapping on, a press right on
/// the feature with it off), else the nearest curve within `hit`, else the nearest straight
/// segment within `hit`, else a free point.
#[must_use]
pub fn measure_pick(
    objects: &[ObjectSnapshot],
    p: Point,
    (snap, hit): (Length, Length),
) -> MeasurePick {
    let (snap, hit) = (snap.as_mm(), hit.as_mm());
    let candidates = || {
        objects
            .iter()
            .filter(|object| !matches!(object, ObjectSnapshot::Dimension(_)))
    };
    let nearest = candidates()
        .flat_map(point_features)
        .map(|(slot, at)| (at.vector_to(p).length(), slot, at))
        .filter(|(d, ..)| *d <= snap)
        .min_by(|x, y| x.0.total_cmp(&y.0));
    if let Some((_, slot, at)) = nearest {
        return MeasurePick::Point { slot, at };
    }
    let best = |found: Vec<(f64, MeasurePick)>| {
        found
            .into_iter()
            .min_by(|x, y| x.0.total_cmp(&y.0))
            .map(|(_, pick)| pick)
    };
    if let Some(pick) = best(candidates().filter_map(|o| curve_near(o, p, hit)).collect()) {
        return pick;
    }
    if let Some(pick) = best(
        candidates()
            .filter_map(|o| segment_near(o, p, hit))
            .collect(),
    ) {
        return pick;
    }
    MeasurePick::Point {
        slot: PointSlot::Free { point: p },
        at: p,
    }
}

/// The node a pick refers to, for the hover highlight; `None` for a free point.
#[must_use]
pub fn pick_node(pick: &MeasurePick) -> Option<NodeId> {
    match pick {
        MeasurePick::Point { slot, .. }
        | MeasurePick::Segment {
            start: (slot, _), ..
        } => slot.node(),
        MeasurePick::Curve { slot, .. } => slot.node(),
    }
}

#[cfg(test)]
#[path = "measure_pick_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "measure_pick_shape_tests.rs"]
mod shape_tests;
