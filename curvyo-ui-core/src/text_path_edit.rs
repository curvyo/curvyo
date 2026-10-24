//! The commands and handle math of a text on path (`specs/0080-text-on-path` criteria 5, 15, 16,
//! 17, 20, 21, 23; `adrs.md` decision 5): the default position, the straight placement on
//! Release, the position handle's drag with the side switch and the Ctrl snap, the arrow nudge
//! and the degree readout on an ellipse carrier.

use curvyo_document_core::{
    Affine, EllipseCarrier, Length, OnPathValues, PathSide, Point, TextSnapshot, Vec2,
};
use curvyo_geometry_core::PathMeasure;
use curvyo_text_core::{Face, TextLayout};

use crate::text_on_path::PathPlacement;

/// The Position (the arc length of the text's middle) at which `laid` sits by default on
/// `measure` (criterion 5, customer 2026-10-24): on an open carrier its start at the carrier's
/// start, its middle half its width along; on a closed one its middle at the topmost point.
#[must_use]
pub fn default_position(laid: &TextLayout, measure: &PathMeasure) -> Length {
    if measure.is_closed() {
        measure.topmost()
    } else {
        Length::from_mm(crate::text_on_path::text_half_width(laid))
    }
}

/// The straight placement Release from path writes (criterion 23): the first letter stays where
/// it is drawn, turned by the carrier's tangent there. For a vertical text (criterion 48) the
/// first character of its stored vertical layout lands where the first letter on the path was.
/// `on_path` is `text` laid out on its carrier (`laid` horizontally); `vertical` the layout in the
/// stored writing mode when that is not horizontal.
#[must_use]
pub fn release_placement(
    text: &TextSnapshot,
    laid: &TextLayout,
    placement: &PathPlacement,
    vertical: Option<&TextLayout>,
) -> Affine {
    let Some((line, glyph)) = laid
        .lines
        .iter()
        .enumerate()
        .find_map(|(index, line)| line.glyphs.first().map(|g| (index, *g)))
    else {
        return placement.frame(0, 0.0).unwrap_or(text.placement);
    };
    let Some(frame) = placement.frame(line, glyph.centre_x.0) else {
        return text.placement;
    };
    let target = frame.apply_point(Point::new(glyph.origin.x, glyph.origin.y));
    let source = vertical
        .and_then(|v| {
            v.lines
                .iter()
                .find_map(|line| line.glyphs.first())
                .map(|g| v.frame() * g.origin)
        })
        .map_or(Point::new(glyph.origin.x, glyph.origin.y), |p| {
            Point::new(p.x, p.y)
        });
    let linear = Affine::new(frame.a, frame.b, frame.c, frame.d, 0.0, 0.0);
    let moved = linear.apply_point(source);
    Affine::new(
        frame.a,
        frame.b,
        frame.c,
        frame.d,
        target.x - moved.x,
        target.y - moved.y,
    )
}

/// Where the position handle sits: the text's middle on the carrier (Distance not applied), and
/// on the line on path (with Distance), for the handle guide.
#[must_use]
pub fn handle_points(placement: &PathPlacement) -> (Point, Point) {
    let values = placement.values();
    let on_carrier = placement.measure().at(values.position).point;
    let normal = side_normal(placement, values.position);
    let on_line = on_carrier.translated(normal.scaled(values.distance.as_mm()));
    (on_carrier, on_line)
}

/// The unit normal towards the letters' side at arc length `s`.
fn side_normal(placement: &PathPlacement, s: Length) -> Vec2 {
    let tau = placement.measure().at(s).tangent;
    let n = Vec2::new(tau.y, -tau.x);
    match placement.values().side {
        PathSide::Above => n,
        PathSide::Below => n.scaled(-1.0),
    }
}

/// The values of a position-handle drag at `pointer` (criteria 15, 16): the text's middle at
/// the carrier point nearest the pointer; Side and Line switched against `start` when the pointer
/// lies more than `cross` (mm, 8 screen px) beyond the carrier on the other side. With `snap`
/// (Ctrl held) the arc length snaps to the stops of [`snap_stops`].
#[must_use]
pub fn dragged_values(
    measure: &PathMeasure,
    start: OnPathValues,
    pointer: Point,
    cross: f64,
    snap: Option<&[Length]>,
) -> OnPathValues {
    slid_values(measure, start, Length::from_mm(0.0), pointer, cross, snap)
}

/// [`dragged_values`] for a drag grabbed `grab` along the carrier from the text's middle (a
/// body drag, criterion 21; [`shift_along`] of the press): the text's middle keeps that
/// offset behind the pointer's nearest carrier point, so the text does not jump at the grab. On
/// an open carrier it stays between the ends.
#[must_use]
pub fn slid_values(
    measure: &PathMeasure,
    start: OnPathValues,
    grab: Length,
    pointer: Point,
    cross: f64,
    snap: Option<&[Length]>,
) -> OnPathValues {
    let length = measure.length().as_mm();
    let raw = measure.nearest(pointer).as_mm() - grab.as_mm();
    let mut s = if measure.is_closed() {
        measure.wrapped(Length::from_mm(raw))
    } else {
        Length::from_mm(raw.clamp(0.0, length.max(0.0)))
    };
    if let Some(stops) = snap {
        s = nearest_stop(measure, s, stops);
    }
    let sample = measure.at(Length::from_mm(s.as_mm() + grab.as_mm()));
    let n = Vec2::new(sample.tangent.y, -sample.tangent.x);
    let towards = match start.side {
        PathSide::Above => n,
        PathSide::Below => n.scaled(-1.0),
    };
    let offset = sample.point.vector_to(pointer);
    let beyond = offset.x.mul_add(towards.x, offset.y * towards.y) < -cross;
    let values = OnPathValues {
        position: s,
        ..start
    };
    if beyond { values.mirrored() } else { values }
}

/// The values of a Ctrl-copy move of a text on path (criterion 31): Position shifted from
/// `start` by the drag from `press` to `pointer` projected on the carrier's tangent at the point
/// nearest the press; on an open carrier held between the ends. Side and Line are kept.
#[must_use]
pub fn projected_values(
    measure: &PathMeasure,
    start: OnPathValues,
    press: Point,
    pointer: Point,
) -> OnPathValues {
    let tangent = measure.at(measure.nearest(press)).tangent;
    let drag = press.vector_to(pointer);
    let s = start.position.as_mm() + drag.x.mul_add(tangent.x, drag.y * tangent.y);
    let position = if measure.is_closed() {
        measure.wrapped(Length::from_mm(s))
    } else {
        Length::from_mm(s.clamp(0.0, measure.length().as_mm().max(0.0)))
    };
    OnPathValues { position, ..start }
}

/// The travel along the carrier from arc length `from` to `to`: their difference, on a closed
/// carrier the short way round (criteria 21, 31).
#[must_use]
pub fn shift_along(measure: &PathMeasure, from: Length, to: Length) -> Length {
    let d = to.as_mm() - from.as_mm();
    let length = measure.length().as_mm();
    if !measure.is_closed() || length <= 0.0 {
        return Length::from_mm(d);
    }
    let d = d.rem_euclid(length);
    Length::from_mm(if d > length / 2.0 { d - length } else { d })
}

/// The stop of `stops` nearest to `s` along the carrier (wrapping on a closed one).
fn nearest_stop(measure: &PathMeasure, s: Length, stops: &[Length]) -> Length {
    let length = measure.length().as_mm();
    let gap = |stop: f64| {
        let d = (stop - s.as_mm()).abs();
        if measure.is_closed() && length > 0.0 {
            d.min(length - d)
        } else {
            d
        }
    };
    stops
        .iter()
        .copied()
        .min_by(|a, b| gap(a.as_mm()).total_cmp(&gap(b.as_mm())))
        .unwrap_or(s)
}

/// The Ctrl snap stops of the position handle (UX notes §3.3): on an open carrier its start,
/// middle and end; on a closed one its start, the quarter lengths and `default`; on an ellipse
/// the 15° and 22.5° stops of the arc handles.
#[must_use]
pub fn snap_stops(
    measure: &PathMeasure,
    ellipse: Option<&EllipseCarrier>,
    default: Length,
) -> Vec<Length> {
    let length = measure.length().as_mm();
    if let Some(ellipse) = ellipse {
        let mut stops = Vec::new();
        for step in [15.0_f64, 22.5] {
            let mut degrees = 0.0_f64;
            while degrees < 360.0 - 1e-9 {
                stops.push(measure.nearest(ellipse_point(ellipse, degrees.to_radians())));
                degrees += step;
            }
        }
        return stops;
    }
    if measure.is_closed() {
        vec![
            Length::from_mm(0.0),
            Length::from_mm(length / 4.0),
            Length::from_mm(length / 2.0),
            Length::from_mm(3.0 * length / 4.0),
            default,
        ]
    } else {
        vec![
            Length::from_mm(0.0),
            Length::from_mm(length / 2.0),
            Length::from_mm(length),
        ]
    }
}

/// The point of `ellipse` at parametric angle `angle` (radians, clockwise on screen from its own
/// x-axis, the convention of `0072`).
#[must_use]
pub fn ellipse_point(ellipse: &EllipseCarrier, angle: f64) -> Point {
    let local = Vec2::new(ellipse.rx * angle.cos(), ellipse.ry * angle.sin());
    let (sin, cos) = ellipse.rotation.sin_cos();
    ellipse.centre.translated(Vec2::new(
        local.x.mul_add(cos, -local.y * sin),
        local.x.mul_add(sin, local.y * cos),
    ))
}

/// The parametric angle of `point` on `ellipse`, degrees in `[0, 360)` (criterion 17's readout).
#[must_use]
pub fn ellipse_degrees(ellipse: &EllipseCarrier, point: Point) -> f64 {
    let v = ellipse.centre.vector_to(point);
    let (sin, cos) = ellipse.rotation.sin_cos();
    let local = Vec2::new(v.x.mul_add(cos, v.y * sin), v.y.mul_add(cos, -v.x * sin));
    let (rx, ry) = (ellipse.rx.max(1e-12), ellipse.ry.max(1e-12));
    (local.y / ry)
        .atan2(local.x / rx)
        .to_degrees()
        .rem_euclid(360.0)
}

/// The arc length of the point at `degrees` on `ellipse` (a typed Position in degrees, the
/// rotation of criterion 20).
#[must_use]
pub fn position_at_degrees(
    measure: &PathMeasure,
    ellipse: &EllipseCarrier,
    degrees: f64,
) -> Length {
    measure.nearest(ellipse_point(ellipse, degrees.to_radians()))
}

/// The values after an arrow nudge (criterion 21, UX notes §0.7): Left and Right move Position by
/// `along` in reading direction (positive: Right); Up and Down change Distance by `up` towards the
/// top of the letters (positive: Up).
#[must_use]
pub fn nudged_values(values: OnPathValues, along: f64, up: f64) -> OnPathValues {
    let sigma: f64 = if values.reads_with_carrier() {
        1.0
    } else {
        -1.0
    };
    // Text up is σ·n; the Distance grows along n Above and along -n Below.
    let side: f64 = match values.side {
        PathSide::Above => 1.0,
        PathSide::Below => -1.0,
    };
    OnPathValues {
        position: Length::from_mm(sigma.mul_add(along, values.position.as_mm())),
        distance: Length::from_mm((sigma * side).mul_add(up, values.distance.as_mm())),
        ..values
    }
}

/// The values of `text` on its carrier after a rotation by `degrees` about an ellipse carrier's
/// centre (criterion 20): the text's middle moves by that parametric angle.
#[must_use]
pub fn rotated_position(
    measure: &PathMeasure,
    ellipse: &EllipseCarrier,
    values: &OnPathValues,
    degrees: f64,
) -> Length {
    let here = measure.at(values.position).point;
    position_at_degrees(measure, ellipse, ellipse_degrees(ellipse, here) + degrees)
}

/// The outline of the compound path `path` nearest to `point`, in storage order: the one a click
/// in pick mode puts the text on (UX notes §2).
#[must_use]
pub fn nearest_outline(path: &curvyo_document_core::PathSnapshot, point: Point) -> u32 {
    let mut best = (f64::INFINITY, 0_u32);
    for (index, subpath) in path.subpaths().enumerate() {
        let anchors: Vec<_> = subpath
            .anchors
            .iter()
            .map(|a| (a.point, a.handle_in, a.handle_out))
            .collect();
        let measure = PathMeasure::new(&anchors, subpath.closed);
        let distance = measure
            .at(measure.nearest(point))
            .point
            .vector_to(point)
            .length();
        if distance < best.0 {
            best = (distance, u32::try_from(index).unwrap_or(u32::MAX));
        }
    }
    best.1
}

/// The vertical layout of `text` in its stored writing mode, when that is not horizontal: what
/// Release measures its first character on (criterion 48).
#[must_use]
pub fn stored_vertical_layout(text: &TextSnapshot, face: &Face) -> Option<TextLayout> {
    if text.layout.writing_mode.is_horizontal() {
        return None;
    }
    let mut straight = text.clone();
    straight.on_path = None;
    crate::text_layout::text_layout(&straight, face)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nudge_follows_the_reading_direction_and_the_letters_top() {
        let base = OnPathValues::at(Length::from_mm(10.0));
        let right = nudged_values(base, 1.0, 1.0);
        assert_eq!(right.position, Length::from_mm(11.0));
        assert_eq!(right.distance, Length::from_mm(1.0));
        // Above + Top edge reads against the carrier: Right lowers the arc length; its letters'
        // top points to the carrier, so Up lowers the Distance.
        let against = OnPathValues {
            line: curvyo_document_core::LineOnPath::TopEdge,
            ..base
        };
        let nudged = nudged_values(against, 1.0, 1.0);
        assert_eq!(nudged.position, Length::from_mm(9.0));
        assert_eq!(nudged.distance, Length::from_mm(-1.0));
    }

    #[test]
    fn degrees_round_trip_on_a_turned_ellipse() {
        let ellipse = EllipseCarrier {
            centre: Point::new(10.0, 20.0),
            rx: 30.0,
            ry: 10.0,
            rotation: 0.4,
        };
        for degrees in [0.0, 45.0, 90.0, 200.0] {
            let p = ellipse_point(&ellipse, f64::to_radians(degrees));
            assert!((ellipse_degrees(&ellipse, p) - degrees).abs() < 1e-9);
        }
    }
}
