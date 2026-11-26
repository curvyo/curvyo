//! Unit tests of the box plates.

use super::*;
use crate::fingers::finger_count;
use crate::simple_check::assert_simple;

fn stored(entries: &[(&str, ParamValue)]) -> StoredParams {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.to_stored()))
        .collect()
}

fn mm(value: f64) -> ParamValue {
    ParamValue::Length(Length::from_mm(value))
}

fn assert_on_one_line(a: &[f64], b: &[f64], tolerance: f64) {
    assert_eq!(a.len(), b.len(), "the mated edges have the same fingers");
    assert_ne!(a.len(), 0);
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() <= tolerance + 1e-9, "{x} against {y}");
    }
}

#[test]
fn mated_edges_lie_on_one_line_within_the_clearance() {
    for clearance in [0.0, 0.2] {
        for lid in ["none", "fingers"] {
            let plates = plates_for_tests(&stored(&[
                ("clearance", mm(clearance)),
                ("finger_width", mm(7.0)),
                ("lid", ParamValue::Choice(lid.into())),
            ]));
            let (bottom, front, left) = (&plates[0], &plates[1], &plates[3]);
            // Front's bottom edge against the bottom's top edge, along x.
            assert_on_one_line(&front.steps(2), &bottom.steps(0), clearance / 2.0);
            // Left's bottom edge against the bottom's left edge, along y.
            assert_on_one_line(&left.steps(2), &bottom.steps(3), clearance / 2.0);
            // Front's vertical edges against the left plate's, along the height.
            assert_on_one_line(&front.steps(3), &left.steps(1), clearance / 2.0);
            assert_on_one_line(&front.steps(1), &left.steps(3), clearance / 2.0);
        }
    }
}

#[test]
fn every_plate_is_a_simple_polygon_at_kerf_zero_and_two_tenths() {
    for kerf in [0.0, 0.2] {
        for lid in ["none", "flat", "fingers"] {
            for pieces in 1..=9 {
                let size = f64::from(pieces) * 6.0;
                let plates = plates_for_tests(&stored(&[
                    ("inner_width", mm(size.max(10.0))),
                    ("inner_depth", mm(size.max(10.0))),
                    ("inner_height", mm(size.max(10.0))),
                    ("finger_width", mm(6.0)),
                    ("lid", ParamValue::Choice(lid.into())),
                ]));
                for (index, plate) in plates.iter().enumerate() {
                    assert_simple(
                        &plate.outline(kerf),
                        &format!("plate {index}, lid {lid}, {pieces} pieces, kerf {kerf}"),
                    );
                }
            }
        }
    }
}

#[test]
fn every_piece_of_a_joint_edge_belongs_to_exactly_one_plate() {
    let plates = plates_for_tests(&stored(&[("lid", ParamValue::Choice("fingers".into()))]));
    let pieces = |plate: &Plate, edge: usize| match &plate.edges[edge] {
        Edge::Fingered(f) => (0..f.segments)
            .map(|i| !crate::fingers::is_notched(i, f.even_solid))
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    // The 12 joint edges: the bottom's top and bottom edges against the front's and back's bottom edges, the
    // bottom's left and right edges against the side walls'; the lid does the same on top.
    let bottom = &plates[0];
    let lid = &plates[5];
    for (plate, edge, mate, mate_edge) in [
        (bottom, 0, &plates[1], 2),
        (bottom, 2, &plates[2], 2),
        (bottom, 3, &plates[3], 2),
        (bottom, 1, &plates[4], 2),
        (lid, 0, &plates[1], 0),
        (lid, 2, &plates[2], 0),
        (lid, 3, &plates[3], 0),
        (lid, 1, &plates[4], 0),
        // The four vertical edges: front and back own the end pieces.
        (&plates[1], 3, &plates[3], 1),
        (&plates[1], 1, &plates[4], 3),
        (&plates[2], 1, &plates[3], 3),
        (&plates[2], 3, &plates[4], 1),
    ] {
        let (a, b) = (pieces(plate, edge), pieces(mate, mate_edge));
        assert_eq!(a.len(), b.len());
        assert_ne!(a.len(), 0);
        for (index, (x, y)) in a.iter().zip(&b).enumerate() {
            assert_ne!(
                x, y,
                "piece {index} of edge {edge} is owned by both or neither"
            );
        }
        assert!(
            a[0] && a[a.len() - 1],
            "the owner keeps the pieces at the corners"
        );
    }
    // The corner cubes belong to the bottom and the lid, and are cut from every wall.
    for plate in [bottom, lid] {
        assert!(plate.corners.iter().all(|c| *c == Corner::Owned));
    }
    for wall in &plates[1..5] {
        assert_eq!(wall.corners[2], Corner::Notched);
        assert_eq!(wall.corners[3], Corner::Notched);
    }
}

#[test]
fn the_kerf_moves_every_outline_point_by_at_most_half_the_kerf_diagonal() {
    let plates = plates_for_tests(&stored(&[]));
    for plate in &plates {
        let nominal = plate.outline(0.0);
        let kerfed = plate.outline(0.2);
        assert_eq!(nominal.segments.len(), kerfed.segments.len());
        for (a, b) in nominal.segments.iter().zip(&kerfed.segments) {
            let d = (a.to.x - b.to.x).hypot(a.to.y - b.to.y);
            assert!((d - 0.1 * 2.0_f64.sqrt()).abs() < 1e-9, "{d}");
        }
    }
}

#[test]
fn a_hundred_fingers_per_edge_still_build() {
    let plates = plates_for_tests(&stored(&[
        ("inner_width", mm(400.0)),
        ("inner_depth", mm(400.0)),
        ("inner_height", mm(400.0)),
        ("finger_width", mm(4.0)),
    ]));
    assert_eq!(finger_count(400.0, 4.0), 101);
    assert!(plates[0].outline(0.1).segments.len() > 300);
}
