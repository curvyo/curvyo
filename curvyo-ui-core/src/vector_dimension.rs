//! Turns a Vector dimension's stored place into the label point the shared layout takes, and a
//! dragged label point back into a place (`specs/0070-measure-and-dimension` criteria 7 and 8).
//!
//! The place is relative to what is measured (ADR 0027 §2): an offset across the line and a
//! fraction along it, the leader direction of a radius, the arc radius and the fraction of an
//! angle, so a linked dimension follows its geometry.

use curvyo_document_core::{
    Angle, DimensionKind, DimensionPlace, DimensionSnapshot, Length, Point, Vec2, angle_rays,
};

use crate::{DimensionGeometry, DimensionItem};

/// Below this length (mm) a direction is undefined.
const DEGENERATE_MM: f64 = 1e-9;

fn unit(v: Vec2, fallback: Vec2) -> Vec2 {
    if v.length() <= DEGENERATE_MM {
        fallback
    } else {
        v.normalized_to(1.0)
    }
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

/// The frame a distance is placed in: its origin, the unit direction along the dimension line,
/// the unit normal across it, and the span along it.
fn distance_frame(kind: DimensionKind, a: Point, b: Point) -> (Point, Vec2, Vec2, f64) {
    let along = match kind {
        DimensionKind::Horizontal => Vec2::new(1.0, 0.0),
        DimensionKind::Vertical => Vec2::new(0.0, 1.0),
        _ => unit(a.vector_to(b), Vec2::new(1.0, 0.0)),
    };
    let normal = Vec2::new(along.y, -along.x);
    (a, along, normal, dot(a.vector_to(b), along))
}

/// The measured geometry of `kind` over `points` (as `Measured::points`) with the angle `sector`.
#[must_use]
pub fn dimension_geometry(
    kind: DimensionKind,
    points: &[Point],
    sector: u8,
) -> Option<DimensionGeometry> {
    match (kind, points) {
        (DimensionKind::Aligned, [from, to, ..]) => Some(DimensionGeometry::Linear {
            from: *from,
            to: *to,
        }),
        (DimensionKind::Horizontal | DimensionKind::Vertical, [from, to, ..]) => {
            Some(DimensionGeometry::Projected {
                from: *from,
                to: *to,
                along: distance_frame(kind, *from, *to).1,
            })
        }
        (DimensionKind::Radius, [center, rim, ..]) => Some(DimensionGeometry::Radius {
            center: *center,
            radius: Length::from_mm(center.vector_to(*rim).length()),
            toward: Vec2::new(1.0, -1.0),
        }),
        (DimensionKind::Diameter, [center, rim, ..]) => Some(DimensionGeometry::Diameter {
            center: *center,
            radius: Length::from_mm(center.vector_to(*rim).length()),
            toward: Vec2::new(1.0, -1.0),
        }),
        (DimensionKind::Angle, [vertex, first, second, ..]) => {
            let (r1, r2) = angle_rays(*vertex, *first, *second, sector)?;
            Some(DimensionGeometry::Angle {
                vertex: *vertex,
                first: r1,
                second: r2,
            })
        }
        _ => None,
    }
}

/// The label point of `place` for a dimension of `kind` over `points`.
#[must_use]
pub fn label_point(kind: DimensionKind, points: &[Point], place: DimensionPlace) -> Option<Point> {
    let offset = place.offset.as_mm();
    match (kind, points) {
        (DimensionKind::Radius | DimensionKind::Diameter, [center, rim, ..]) => {
            let radius = center.vector_to(*rim).length();
            let leader = place.leader.as_radians();
            Some(center.translated(Vec2::new(leader.cos(), leader.sin()).scaled(radius + offset)))
        }
        (DimensionKind::Angle, [vertex, first, second, ..]) => {
            let (r1, r2) = angle_rays(*vertex, *first, *second, place.sector)?;
            let start = r1.y.atan2(r1.x);
            let sweep = (r1.x * r2.y - r1.y * r2.x).atan2(dot(r1, r2));
            let at = start + sweep * place.along;
            Some(vertex.translated(Vec2::new(at.cos(), at.sin()).scaled(offset.abs())))
        }
        (_, [a, b, ..]) => {
            let (origin, along, normal, span) = distance_frame(kind, *a, *b);
            Some(
                origin
                    .translated(along.scaled(span * place.along))
                    .translated(normal.scaled(offset)),
            )
        }
        _ => None,
    }
}

/// The place that puts the label of a dimension of `kind` over `points` at `label`, keeping the
/// parts of `place` the kind does not use.
#[must_use]
pub fn place_at(
    kind: DimensionKind,
    points: &[Point],
    place: DimensionPlace,
    label: Point,
) -> DimensionPlace {
    let mut moved = place;
    match (kind, points) {
        (DimensionKind::Radius | DimensionKind::Diameter, [center, rim, ..]) => {
            let to = center.vector_to(label);
            moved.leader = Angle::from_radians(to.y.atan2(to.x));
            moved.offset = Length::from_mm(to.length() - center.vector_to(*rim).length());
        }
        (DimensionKind::Angle, [vertex, first, second, ..]) => {
            if let Some((r1, r2)) = angle_rays(*vertex, *first, *second, place.sector) {
                let to = vertex.vector_to(label);
                let start = r1.y.atan2(r1.x);
                let sweep = (r1.x * r2.y - r1.y * r2.x).atan2(dot(r1, r2));
                let mut turn = to.y.atan2(to.x) - start;
                while turn - sweep / 2.0 > std::f64::consts::PI {
                    turn -= std::f64::consts::TAU;
                }
                while turn - sweep / 2.0 < -std::f64::consts::PI {
                    turn += std::f64::consts::TAU;
                }
                if sweep.abs() > DEGENERATE_MM {
                    moved.along = turn / sweep;
                }
                moved.offset = Length::from_mm(to.length());
            }
        }
        (_, [a, b, ..]) => {
            let (origin, along, normal, span) = distance_frame(kind, *a, *b);
            let to = origin.vector_to(label);
            moved.offset = Length::from_mm(dot(to, normal));
            if span.abs() > DEGENERATE_MM {
                moved.along = dot(to, along) / span;
            }
        }
        _ => {}
    }
    moved
}

/// The layout input of a Vector dimension as shown: its geometry, its label from the place, the
/// length of `text`.
#[must_use]
pub fn dimension_item(dimension: &DimensionSnapshot, text: &str) -> Option<DimensionItem> {
    let points = &dimension.shown.points;
    Some(DimensionItem {
        geometry: dimension_geometry(dimension.kind, points, dimension.place.sector)?,
        label: label_point(dimension.kind, points, dimension.place),
        text_chars: text.chars().count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, b: Point) -> bool {
        a.vector_to(b).length() < 1e-9
    }

    #[test]
    fn a_label_point_and_its_place_round_trip_for_every_kind() {
        let two = [Point::new(10.0, 10.0), Point::new(40.0, 30.0)];
        let three = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(0.0, 10.0),
        ];
        let label = Point::new(22.0, -3.0);
        for kind in DimensionKind::ALL {
            let points: &[Point] = if kind == DimensionKind::Angle {
                &three
            } else {
                &two
            };
            let start = DimensionPlace::at_offset(Length::from_mm(5.0));
            let placed = place_at(kind, points, start, label);
            let back = label_point(kind, points, placed).unwrap();
            if kind == DimensionKind::Angle {
                // The angle text stays on its arc; its distance from the vertex is kept.
                assert!(
                    (back.vector_to(three[0]).length() - label.vector_to(three[0]).length()).abs()
                        < 1e-9
                );
            } else {
                assert!(close(back, label), "{kind:?}: {back:?}");
            }
        }
    }

    #[test]
    fn a_linked_label_follows_its_points() {
        let place = place_at(
            DimensionKind::Aligned,
            &[Point::new(0.0, 0.0), Point::new(10.0, 0.0)],
            DimensionPlace::at_offset(Length::from_mm(0.0)),
            Point::new(5.0, -4.0),
        );
        let moved = label_point(
            DimensionKind::Aligned,
            &[Point::new(100.0, 0.0), Point::new(110.0, 0.0)],
            place,
        )
        .unwrap();
        assert!(close(moved, Point::new(105.0, -4.0)));
    }
}
