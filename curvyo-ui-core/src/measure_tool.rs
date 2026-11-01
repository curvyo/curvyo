//! The pure rules of the Measure tool (`specs/0070-measure-and-dimension` criteria 1, 3, 5 and
//! 6): the kind from the picks and the pointer, the angle of two segment picks, the angle sector,
//! the Radius/Diameter switch, the readout lines and which selection can be dimensioned at once.

use curvyo_document_core::{
    Angle, CIRCLE_TOLERANCE, CurveSlot, DimensionKind, DimensionValue, DisplayUnit, Length,
    ObjectSnapshot, Point, PointSlot, PrimitiveSnapshot, Shape, Vec2,
};

use crate::measure_pick::MeasurePick;

use crate::dimension_text::{VECTOR_DECIMALS, dimension_text, trimmed};

/// The kind of a two-point dimension by where the pointer `p` is relative to the box around `a`
/// and `b` (criterion 3, SolidWorks-style): above or below the box (x within it) gives
/// Horizontal, left or right (y within it) Vertical, inside or in a corner quadrant Aligned.
#[must_use]
pub fn two_point_kind(a: Point, b: Point, p: Point) -> DimensionKind {
    let (min_x, max_x) = (a.x.min(b.x), a.x.max(b.x));
    let (min_y, max_y) = (a.y.min(b.y), a.y.max(b.y));
    let within_x = (min_x..=max_x).contains(&p.x);
    let within_y = (min_y..=max_y).contains(&p.y);
    match (within_x, within_y) {
        (true, false) => DimensionKind::Horizontal,
        (false, true) => DimensionKind::Vertical,
        _ => DimensionKind::Aligned,
    }
}

/// The sector (bit 0 turns the first ray round, bit 1 the second) whose angle between the rays
/// `first` and `second` from `vertex` holds the pointer `p` (criterion 3: the sector under the
/// pointer chooses which of the four angles).
#[must_use]
pub fn angle_sector(vertex: Point, first: Vec2, second: Vec2, p: Point) -> u8 {
    let to = vertex.vector_to(p);
    let cross = |u: Vec2, v: Vec2| u.x * v.y - u.y * v.x;
    (0..4)
        .find(|&sector| {
            let r1 = if sector & 1 == 0 {
                first
            } else {
                first.negated()
            };
            let r2 = if sector & 2 == 0 {
                second
            } else {
                second.negated()
            };
            let turn = cross(r1, r2);
            // Inside the smaller angle from r1 to r2: on r2's side of r1 and r1's side of r2.
            cross(r1, to) * turn >= 0.0 && cross(to, r2) * turn >= 0.0
        })
        .unwrap_or(0)
}

/// The Radius/Diameter switch while a curve dimension is placed (criterion 6, UX notes §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurveKind {
    /// Radius or Diameter.
    pub kind: DimensionKind,
    /// Alt was held when the curve was picked: it does not switch until it is pressed again.
    alt_at_pick: bool,
}

impl CurveKind {
    /// Radius, the default; `alt_held` is whether Alt was down at the pick.
    #[must_use]
    pub const fn picked(alt_held: bool) -> Self {
        Self {
            kind: DimensionKind::Radius,
            alt_at_pick: alt_held,
        }
    }

    /// An Alt keydown: switches between Radius and Diameter unless it repeats, comes with Ctrl,
    /// Cmd or Shift, or Alt was already held at the pick (then only a fresh press switches).
    /// Returns whether it switched.
    pub fn alt_down(&mut self, repeat: bool, other_modifier: bool) -> bool {
        if repeat || other_modifier {
            return false;
        }
        if self.alt_at_pick {
            self.alt_at_pick = false;
            return false;
        }
        self.kind = if self.kind == DimensionKind::Radius {
            DimensionKind::Diameter
        } else {
            DimensionKind::Radius
        };
        true
    }

    /// An Alt key-up: the next press switches.
    pub const fn alt_up(&mut self) {
        self.alt_at_pick = false;
    }
}

/// U+2212 for a minus sign, as the move readout writes it.
fn signed(value: f64, decimals: usize) -> String {
    trimmed(value, decimals).replace('-', "\u{2212}")
}

/// The two readout lines of a measurement from `a` to `b` (criterion 1, UX notes §3): length and
/// angle (clockwise on screen from straight right, −180° to 180°), then the x and y distances.
#[must_use]
pub fn readout_lines(a: Point, b: Point, unit: DisplayUnit) -> [String; 2] {
    let d = a.vector_to(b);
    let length = dimension_text(
        DimensionKind::Aligned,
        DimensionValue::Length(Length::from_mm(d.length())),
        unit,
        VECTOR_DECIMALS,
    );
    let angle = signed(d.y.atan2(d.x).to_degrees(), VECTOR_DECIMALS);
    let per = unit.mm_per_unit();
    [
        format!("{length}, {angle}°"),
        format!(
            "\u{394} {}, {} {}",
            signed(d.x / per, VECTOR_DECIMALS),
            signed(d.y / per, VECTOR_DECIMALS),
            unit.symbol()
        ),
    ]
}

/// The readout of a radius pick: "R 25 mm" and "⌀ 50 mm".
#[must_use]
pub fn radius_readout(radius: Length, unit: DisplayUnit) -> [String; 2] {
    [
        dimension_text(
            DimensionKind::Radius,
            DimensionValue::Length(radius),
            unit,
            VECTOR_DECIMALS,
        ),
        dimension_text(
            DimensionKind::Diameter,
            DimensionValue::Length(Length::from_mm(2.0 * radius.as_mm())),
            unit,
            VECTOR_DECIMALS,
        ),
    ]
}

/// What a lone selected object can be dimensioned as at once (criterion 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionDimension {
    /// A circle or arc primitive: Radius.
    Circle,
    /// A straight two-node open path: its Aligned length.
    Segment,
}

/// Whether `object` is eligible for selection first: a circle or arc (an ellipse with equal
/// radii), or an open path of two nodes with a straight segment.
#[must_use]
pub fn selection_dimension(object: &ObjectSnapshot) -> Option<SelectionDimension> {
    match object {
        ObjectSnapshot::Primitive(PrimitiveSnapshot {
            shape: Shape::Ellipse { frame, .. },
            ..
        }) if (frame.rx.as_mm() - frame.ry.as_mm()).abs() <= CIRCLE_TOLERANCE.as_mm() => {
            Some(SelectionDimension::Circle)
        }
        ObjectSnapshot::Path(path)
            if !path.closed
                && !path.is_compound()
                && path.anchors.len() == 2
                && path.anchors[0].handle_out == Vec2::ZERO
                && path.anchors[1].handle_in == Vec2::ZERO =>
        {
            Some(SelectionDimension::Segment)
        }
        _ => None,
    }
}

/// The slot and position of a point pick; `None` for a curve or a segment.
#[must_use]
pub fn pick_point(pick: &MeasurePick) -> Option<(PointSlot, Point)> {
    match *pick {
        MeasurePick::Point { slot, at } => Some((slot, at)),
        _ => None,
    }
}

/// A free point pick at `at`, linked to nothing.
#[must_use]
pub const fn free_pick(at: Point) -> MeasurePick {
    MeasurePick::Point {
        slot: PointSlot::Free { point: at },
        at,
    }
}

/// An angle's three points with their slots (vertex, first, second), and its two rays.
pub type AnglePicks = ([(PointSlot, Point); 3], (Vec2, Vec2));

/// The angle between two segment picks (criterion 3): the shared end as vertex when there is
/// one, else their lines' crossing as a free point **(Proposal)**; the far ends as the rays'
/// points. `None` unless both are segments and not parallel.
#[must_use]
pub fn angle_picks(first: &MeasurePick, second: &MeasurePick) -> Option<AnglePicks> {
    let (MeasurePick::Segment { start: s0, end: s1 }, MeasurePick::Segment { start: t0, end: t1 }) =
        (first, second)
    else {
        return None;
    };
    let d1 = s0.1.vector_to(s1.1);
    let d2 = t0.1.vector_to(t1.1);
    let cross = d1.x * d2.y - d1.y * d2.x;
    if cross.abs() <= 1e-9 * d1.length() * d2.length() {
        return None;
    }
    let shared = [
        (s0, s1, t0, t1),
        (s0, s1, t1, t0),
        (s1, s0, t0, t1),
        (s1, s0, t1, t0),
    ]
    .into_iter()
    .find(|(a, _, b, _)| a.0 == b.0 || a.1.vector_to(b.1).length() <= 1e-9);
    let (vertex, far1, far2) = if let Some((a, a_far, _, b_far)) = shared {
        (*a, *a_far, *b_far)
    } else {
        let t = (s0.1.vector_to(t0.1).x * d2.y - s0.1.vector_to(t0.1).y * d2.x) / cross;
        let at = s0.1.translated(d1.scaled(t));
        let farther = |p: &(PointSlot, Point), q: &(PointSlot, Point)| {
            if p.1.vector_to(at).length() >= q.1.vector_to(at).length() {
                *p
            } else {
                *q
            }
        };
        (
            (PointSlot::Free { point: at }, at),
            farther(s0, s1),
            farther(t0, t1),
        )
    };
    Some((
        [vertex, far1, far2],
        (vertex.1.vector_to(far1.1), vertex.1.vector_to(far2.1)),
    ))
}

/// The pick of a selection-first dimension of `object` (criterion 5): its segment, or its circle
/// from its bounds; a free point at the origin for an object [`selection_dimension`] refuses.
#[must_use]
pub fn selection_pick(object: &ObjectSnapshot, eligible: SelectionDimension) -> MeasurePick {
    match (eligible, object) {
        (SelectionDimension::Segment, ObjectSnapshot::Path(path)) if path.anchors.len() >= 2 => {
            let end = |i: usize| {
                (
                    PointSlot::Anchor {
                        node: path.id,
                        anchor: path.anchors[i].id,
                    },
                    path.anchors[i].point,
                )
            };
            MeasurePick::Segment {
                start: end(0),
                end: end(1),
            }
        }
        (_, ObjectSnapshot::Primitive(primitive)) => {
            let (min, max) = crate::object_bounds(object);
            MeasurePick::Curve {
                slot: CurveSlot::Circle { node: primitive.id },
                center: Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y)),
                radius: Length::from_mm((max.x - min.x) / 2.0),
                arc: (
                    Angle::from_radians(0.0),
                    Angle::from_radians(std::f64::consts::TAU),
                ),
            }
        }
        _ => free_pick(Point::new(0.0, 0.0)),
    }
}

/// The mode label (UX notes §4): the kind alone while measuring, "Dimension, " and the kind in
/// lower case while a dimension is left; "Measure" or "Dimension" before a kind is known.
#[must_use]
pub fn mode_label(kind: Option<DimensionKind>, dimension: bool) -> String {
    match (kind, dimension) {
        (None, false) => "Measure".to_owned(),
        (None, true) => "Dimension".to_owned(),
        (Some(kind), false) => kind.name().to_owned(),
        (Some(kind), true) => format!("Dimension, {}", kind.name().to_lowercase()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_eight_regions_around_two_points() {
        let (a, b) = (Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let at = |x, y| two_point_kind(a, b, Point::new(x, y));
        assert_eq!(at(5.0, -5.0), DimensionKind::Horizontal, "above");
        assert_eq!(at(5.0, 15.0), DimensionKind::Horizontal, "below");
        assert_eq!(at(-5.0, 5.0), DimensionKind::Vertical, "left");
        assert_eq!(at(15.0, 5.0), DimensionKind::Vertical, "right");
        assert_eq!(at(-5.0, -5.0), DimensionKind::Aligned, "top-left");
        assert_eq!(at(15.0, -5.0), DimensionKind::Aligned, "top-right");
        assert_eq!(at(-5.0, 15.0), DimensionKind::Aligned, "bottom-left");
        assert_eq!(at(15.0, 15.0), DimensionKind::Aligned, "bottom-right");
        assert_eq!(at(5.0, 5.0), DimensionKind::Aligned, "inside");
    }

    #[test]
    fn the_sector_follows_the_pointer() {
        let v = Point::new(0.0, 0.0);
        let (x, y) = (Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0));
        assert_eq!(angle_sector(v, x, y, Point::new(1.0, 1.0)), 0);
        assert_eq!(angle_sector(v, x, y, Point::new(-1.0, 1.0)), 1);
        assert_eq!(angle_sector(v, x, y, Point::new(1.0, -1.0)), 2);
        assert_eq!(angle_sector(v, x, y, Point::new(-1.0, -1.0)), 3);
    }

    #[test]
    fn alt_switches_radius_and_diameter_by_its_phase_rules() {
        let mut curve = CurveKind::picked(false);
        assert_eq!(curve.kind, DimensionKind::Radius);
        assert!(curve.alt_down(false, false));
        assert_eq!(curve.kind, DimensionKind::Diameter);
        assert!(!curve.alt_down(true, false), "a repeat does not switch");
        assert!(!curve.alt_down(false, true), "not with Ctrl, Cmd or Shift");
        let mut held = CurveKind::picked(true);
        assert!(!held.alt_down(false, false), "Alt held at the pick");
        assert!(held.alt_down(false, false), "the next press switches");
    }

    #[test]
    fn the_readout_shows_length_angle_and_distances() {
        let [first, second] = readout_lines(
            Point::new(0.0, 0.0),
            Point::new(8.839, -8.839),
            DisplayUnit::Mm,
        );
        assert_eq!(first, "12.5 mm, \u{2212}45°");
        assert_eq!(second, "\u{394} 8.839, \u{2212}8.839 mm");
        assert_eq!(
            radius_readout(Length::from_mm(25.0), DisplayUnit::Mm),
            ["R 25 mm".to_owned(), "⌀ 50 mm".to_owned()]
        );
    }

    fn segment(node: u64, a: (f64, f64), b: (f64, f64)) -> MeasurePick {
        // Ids come from the document in real use; the tests take one from a fresh path.
        let id = curvyo_document_core::Document::new(node).create_path(
            &[
                curvyo_document_core::NewAnchor::corner(
                    curvyo_document_core::AnchorId::new(1, 1),
                    Point::new(0.0, 0.0),
                ),
                curvyo_document_core::NewAnchor::corner(
                    curvyo_document_core::AnchorId::new(1, 2),
                    Point::new(1.0, 0.0),
                ),
            ],
            false,
        );
        let end = |n: u64, p: (f64, f64)| {
            (
                PointSlot::Anchor {
                    node: id,
                    anchor: curvyo_document_core::AnchorId::new(node, n),
                },
                Point::new(p.0, p.1),
            )
        };
        MeasurePick::Segment {
            start: end(1, a),
            end: end(2, b),
        }
    }

    #[test]
    fn two_segments_meet_at_their_shared_end_or_their_crossing() {
        let first = segment(1, (60.0, 0.0), (0.0, 0.0));
        let mut second = segment(1, (0.0, 0.0), (60.0, -60.0));
        if let MeasurePick::Segment { start, .. } = &mut second {
            *start = match first {
                MeasurePick::Segment { end, .. } => end,
                _ => unreachable!(),
            };
        }
        let ([vertex, far1, far2], _) = angle_picks(&first, &second).unwrap();
        assert_eq!(vertex.1, Point::new(0.0, 0.0));
        assert_eq!(
            (far1.1, far2.1),
            (Point::new(60.0, 0.0), Point::new(60.0, -60.0))
        );
        let apart = segment(2, (10.0, 10.0), (10.0, 50.0));
        let ([crossing, ..], _) = angle_picks(&first, &apart).unwrap();
        assert_eq!(
            crossing.0,
            PointSlot::Free {
                point: Point::new(10.0, 0.0)
            }
        );
        let parallel = segment(3, (0.0, 5.0), (60.0, 5.0));
        assert!(angle_picks(&first, &parallel).is_none());
        assert!(angle_picks(&first, &free_pick(Point::new(0.0, 0.0))).is_none());
        assert_eq!(pick_point(&first), None);
    }

    #[test]
    fn mode_labels() {
        assert_eq!(mode_label(None, false), "Measure");
        assert_eq!(mode_label(None, true), "Dimension");
        assert_eq!(mode_label(Some(DimensionKind::Radius), false), "Radius");
        assert_eq!(
            mode_label(Some(DimensionKind::Horizontal), true),
            "Dimension, horizontal"
        );
    }
}
