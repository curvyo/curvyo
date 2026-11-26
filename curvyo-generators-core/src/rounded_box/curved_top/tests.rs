//! The curved top's joints at the part level: every pair of mating edges at four distinct
//! thicknesses and at all four equal (ADR 0082 §9, §11).

use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2};

use super::plates::{Counts, base, end_plate, front};
use super::strip::{back_start, end_steps, footprint, strip, tab_centres};
use super::{Sizes, plane};
use crate::fingers::is_notched;
use crate::footprint::Shape;
use crate::plate::{Corner, Edge, Fingers, Plate};
use crate::simple_check::assert_simple;
use crate::slit_bend::{SlitKind, SlitPattern};
use curvyo_document_core::Length;

/// `(wall, end, front, base)` thicknesses: four distinct sets and all equal.
const SETS: [(f64, f64, f64, f64); 5] = [
    (3.0, 4.0, 5.0, 2.0),
    (2.5, 3.0, 4.0, 6.0),
    (4.0, 2.0, 3.0, 5.0),
    (3.0, 6.0, 2.0, 4.0),
    (3.0, 3.0, 3.0, 3.0),
];

fn sizes((wall, end, front, base): (f64, f64, f64, f64)) -> Sizes {
    Sizes {
        width: 120.0,
        depth: 80.0,
        height: 60.0,
        radius: 15.0,
        wall,
        end,
        front,
        base,
        kerf: 0.0,
        limits: Vec::new(),
        joints: crate::joint_layout::AUTO,
        used_count: 0,
    }
}

fn pattern(s: &Sizes) -> SlitPattern {
    SlitPattern {
        kind: SlitKind::Straight,
        width: Length::from_mm(0.3),
        spacing: Length::from_mm(0.22 * s.radius),
        bridge: Length::from_mm(s.wall),
        rows: 1,
    }
}

fn fingers(plate: &Plate, edge: usize) -> Fingers {
    match &plate.edges[edge] {
        Edge::Fingered(f) => f.clone(),
        other => panic!("edge {edge} is not fingered: {other:?}"),
    }
}

/// The pieces of a joint belong to exactly one of the two plates; the owner keeps the end pieces.
fn assert_one_owner(a: &Fingers, b: &Fingers, what: &str) {
    assert_eq!(a.segments, b.segments, "{what}: the same pieces");
    assert!(a.segments % 2 == 1, "{what}: an odd number of pieces");
    assert!(
        (a.from - b.from).abs() < 1e-9 && (a.to - b.to).abs() < 1e-9,
        "{what}: the same span"
    );
    for index in 0..a.segments {
        assert_ne!(
            is_notched(index, a.even_solid),
            is_notched(index, b.even_solid),
            "{what}: piece {index} is owned by both or neither"
        );
    }
    assert!(
        !is_notched(0, a.even_solid) || !is_notched(0, b.even_solid),
        "{what}: the owner keeps the first piece"
    );
}

fn assert_on_one_line(a: &[f64], b: &[f64], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}: the same steps");
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() < 1e-9, "{what}: {x} against {y}");
    }
}

#[test]
fn every_pair_of_mating_edges_agrees_at_distinct_thicknesses() {
    for set in SETS {
        let s = sizes(set);
        let counts = Counts::of(&s);
        let (end, front, base) = (
            end_plate(&s, &counts),
            front(&s, &counts, None),
            base(&s, &counts),
        );
        let what = |joint: &str| format!("{joint} at {set:?}");
        // Front and end plate: the vertical joint, the front owns the end pieces.
        for (front_edge, plate_edge) in [(1, 3), (3, 3)] {
            let (a, b) = (fingers(&front, front_edge), fingers(&end, plate_edge));
            assert_one_owner(&a, &b, &what("front and end plate"));
            assert!(a.even_solid, "the front keeps the even pieces");
            assert_on_one_line(
                &front.steps(front_edge),
                &end.steps(plate_edge),
                &what("front and end plate"),
            );
        }
        // Front and base across the width.
        assert_one_owner(
            &fingers(&front, 2),
            &fingers(&base, 0),
            &what("front and base"),
        );
        assert_on_one_line(&front.steps(2), &base.steps(0), &what("front and base"));
        // Base and end plates along the depth, both sides.
        for edge in [1, 3] {
            assert_one_owner(
                &fingers(&base, edge),
                &fingers(&end, 2),
                &what("base and end plate"),
            );
            assert_on_one_line(
                &base.steps(edge),
                &end.steps(2),
                &what("base and end plate"),
            );
        }
        // Base and the strip's end across the width: the strip keeps the odd pieces.
        let steps = end_steps(&s, 100.0);
        let cuts: Vec<f64> = steps.windows(2).map(|pair| pair[0].y1).collect();
        assert_on_one_line(&base.steps(2), &cuts, &what("base and strip"));
        assert!(
            steps[0].y0 == 0.0 && steps[0].y1 >= s.end,
            "the zone over the end plate is cut back"
        );
        assert!(
            (steps[0].x - (100.0 - s.base)).abs() < 1e-9,
            "by the base's thickness"
        );
        let last = steps[steps.len() - 1];
        assert!(last.y0 <= s.width + s.end && (last.x - (100.0 - s.base)).abs() < 1e-9);
        let strip_end = fingers(&base, 2);
        assert!(
            strip_end.even_solid,
            "the base keeps the even pieces of its back edge"
        );
    }
}

#[test]
fn the_base_owns_every_bottom_corner_cube_once() {
    for set in SETS {
        let s = sizes(set);
        let counts = Counts::of(&s);
        let (end, front, base) = (
            end_plate(&s, &counts),
            front(&s, &counts, None),
            base(&s, &counts),
        );
        assert!(base.corners.iter().all(|c| *c == Corner::Owned));
        // The front's bottom corners and the end plate's front-bottom corner are cut out.
        assert_eq!(front.corners[2], Corner::Notched);
        assert_eq!(front.corners[3], Corner::Notched);
        assert_eq!(end.corners[3], Corner::Notched);
        // At the back the cube lies behind the end plate's back edge: nothing to cut.
        assert_eq!(end.corners[2], Corner::Open);
        // The three faces of the front cube are t_e x t_f x t_b, of the back cube t_e x t_w x t_b.
        assert_eq!((base.depths[3], base.depths[0]), (s.end, s.front));
        assert_eq!((front.depths[1], front.depths[2]), (s.end, s.base));
        assert_eq!((end.depths[3], end.depths[2]), (s.front, s.base));
        assert_eq!((base.depths[1], base.depths[2]), (s.end, s.wall));
    }
}

#[test]
fn every_outline_is_a_simple_polygon_at_kerf_zero_and_two_tenths() {
    for set in SETS {
        let s = sizes(set);
        let counts = Counts::of(&s);
        let hollow = Some(crate::wall_plates::Hollow {
            width: 16.0,
            depth: 8.0,
        });
        for kerf in [0.0, 0.2] {
            for (name, plate) in [
                ("end plate", end_plate(&s, &counts)),
                ("front", front(&s, &counts, hollow)),
                ("base", base(&s, &counts)),
            ] {
                assert_simple(
                    &plate.outline(kerf),
                    &format!("{name} at {set:?}, kerf {kerf}"),
                );
            }
            let contours = strip(&s, pattern(&s), false);
            for (index, contour) in contours.iter().enumerate() {
                assert_simple(
                    &contour.offset(kerf / 2.0).to_path(),
                    &format!("strip contour {index} at {set:?}, kerf {kerf}"),
                );
            }
        }
    }
}

#[test]
fn the_tabs_keep_two_wall_thicknesses_from_the_arc_and_from_the_base() {
    for set in SETS {
        let s = Sizes {
            height: 150.0,
            ..sizes(set)
        };
        let centres = tab_centres(&s);
        assert!(centres.len() >= 2);
        let length = footprint(&s).total();
        for y in centres {
            assert!(
                y - s.end >= s.radius + 2.0 * s.wall - 1e-9,
                "{set:?}: 2 t_w below the arc"
            );
            let along = back_start(&s) + y - s.radius;
            assert!(
                length - (along + s.end) >= s.base + 2.0 * s.wall - 1e-9,
                "{set:?}: t_b + 2 t_w above the strip's end"
            );
        }
    }
}

#[test]
fn the_strip_spans_the_profile_from_the_fronts_outer_face_to_the_bases_underside() {
    for set in SETS {
        let s = sizes(set);
        let spans = footprint(&s).spans;
        let (
            Shape::Straight { from, to },
            Shape::Bend {
                centre,
                radius,
                sweep,
                ..
            },
            Shape::Straight { to: last, .. },
        ) = (spans[0].shape, spans[1].shape, spans[2].shape)
        else {
            panic!("a run, an arc and a run");
        };
        assert_eq!(
            from,
            plane(-s.front, s.height),
            "the front's outer face, at the seat"
        );
        assert_eq!(to.y, from.y, "the lid run is level");
        assert_eq!(
            last,
            plane(s.depth, -s.base),
            "the base's underside, on the end plates' back edge"
        );
        assert!((radius - s.radius).abs() < 1e-12 && (sweep - FRAC_PI_2).abs() < 1e-12);
        assert!(
            (centre.x - (s.depth - s.radius)).abs() < 1e-9,
            "the arc is tangent to the back run"
        );
    }
}

#[test]
fn the_end_plates_arc_has_the_radius_of_the_strips_inner_face() {
    let s = sizes(SETS[4]);
    let counts = Counts::of(&s);
    let plate = end_plate(&s, &counts);
    let path = plate.contour().to_path();
    let middle = (
        s.profile_width() - s.radius + s.radius * FRAC_1_SQRT_2,
        s.radius - s.radius * FRAC_1_SQRT_2,
    );
    let on_arc = std::iter::once(path.start)
        .chain(path.segments.iter().map(|c| c.to))
        .any(|p| (p.x - middle.0).abs() < 1e-6 && (p.y - middle.1).abs() < 1e-6);
    assert!(
        on_arc,
        "the arc passes the 45 degree point of a circle of radius r"
    );
    // The back edge is the strip's inner face: the plate ends where the back run begins.
    assert!((plate.width - (s.depth + s.front)).abs() < 1e-12);
}
