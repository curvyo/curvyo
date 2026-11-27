use super::*;

fn square() -> Measured {
    // 40 x 40 mm drawn from its bottom-left corner (Y down).
    Measured::new(
        vec![
            PointUm::new(0, 40_000),
            PointUm::new(40_000, 40_000),
            PointUm::new(40_000, 0),
            PointUm::new(0, 0),
        ],
        true,
    )
}

fn line() -> Measured {
    Measured::new(vec![PointUm::new(0, 0), PointUm::new(100_000, 0)], false)
}

fn count(n: u64) -> TabCount {
    TabCount::new(n).unwrap()
}

fn width(h: i64) -> CutLength {
    CutLength::tab(h).unwrap()
}

#[test]
fn four_tabs_on_a_square_sit_on_the_side_midpoints() {
    let square = square();
    assert!((square.length() - 160_000.0).abs() < 1e-9);
    let spans = automatic(square.length(), count(4), width(200), true);
    let mids = [(20_000, 40_000), (40_000, 20_000), (20_000, 0), (0, 20_000)];
    for (span, mid) in spans.iter().zip(mids) {
        assert_eq!(square.point_um(span.centre), PointUm::new(mid.0, mid.1));
        assert!((span.width - 2_000.0).abs() < 1e-9);
    }
    assert_eq!(spans.len(), 4);
}

#[test]
fn the_count_is_limited_by_the_length() {
    let line = line();
    let one = automatic(line.length(), count(1), width(50), false);
    assert_eq!(one.len(), 1);
    assert!((one[0].centre - 50_000.0).abs() < 1e-9);
    // A circle of 10 mm diameter: 31.4 mm, Width 2.5 mm, Count 20 places 3.
    let circumference = std::f64::consts::PI * 10_000.0;
    assert_eq!(
        automatic_count(circumference, count(20), width(250), true),
        3
    );
    assert_eq!(automatic_count(3_000.0, count(4), width(50), true), 1);
    assert_eq!(automatic_count(1_000.0, count(4), width(50), true), 0);
    assert_eq!(automatic_count(100_000.0, count(0), width(50), true), 0);
    // An open 10 mm line at 0.1 mm: floor(10 / 2.1) = 4, so both end pieces keep 1 mm.
    assert_eq!(automatic_count(10_000.0, count(20), width(10), false), 4);
    assert_eq!(automatic_count(10_000.0, count(20), width(10), true), 9);
}

#[test]
fn a_drag_stops_at_one_millimetre_from_its_neighbour_and_snaps_to_nodes() {
    let line = line();
    let spans = vec![
        TabSpan {
            centre: 20_000.0,
            width: 2_000.0,
        },
        TabSpan {
            centre: 50_000.0,
            width: 2_000.0,
        },
    ];
    // Towards the first tab: its end is at 21 mm, so the second may start at 22 mm, centre 23.
    let towards = slide(&line, &spans, 1, 21_400.0 + 1_000.0, 0.0);
    assert!(towards.limited);
    assert!((towards.centre - 23_000.0).abs() < 1e-9);
    let free = slide(&line, &spans, 1, 60_000.0, 0.0);
    assert!(!free.limited && (free.centre - 60_000.0).abs() < 1e-9);
    let end = slide(&line, &spans, 1, 99_900.0, 0.0);
    assert!(end.limited && (end.centre - 98_000.0).abs() < 1e-9);
    let square = square();
    let one = vec![TabSpan {
        centre: 20_000.0,
        width: 2_000.0,
    }];
    let snap = slide(&square, &one, 0, 39_000.0, 2_000.0);
    assert!(snap.snapped && (snap.centre - 40_000.0).abs() < 1e-9);
    let wrap = slide(&square, &one, 0, 159_000.0, 0.0);
    assert!(
        (wrap.centre - 159_000.0).abs() < 1e-9,
        "a closed path wraps"
    );
}

#[test]
fn an_add_without_room_and_a_width_without_room_change_nothing() {
    let line = line();
    let spans = vec![TabSpan {
        centre: 50_000.0,
        width: 2_000.0,
    }];
    assert!(add(&line, &spans, 52_500.0, 2_000.0).is_none());
    let (next, at) = add(&line, &spans, 80_000.0, 2_000.0).unwrap();
    assert_eq!((next.len(), at), (2, 1));
    assert_eq!(with_width(&line, &next, 0, 58_000.0), Err(NO_ROOM_TEXT));
    assert!(with_width(&line, &next, 0, 10_000.0).is_ok());
    assert!(overlap(
        &line,
        &[
            TabSpan {
                centre: 10_000.0,
                width: 4_000.0
            },
            TabSpan {
                centre: 12_000.0,
                width: 4_000.0
            },
        ]
    ));
    assert!(!overlap(
        &square(),
        &automatic(160_000.0, count(4), width(200), true)
    ));
}

#[test]
fn the_placed_line_names_what_short_paths_get() {
    assert_eq!(placed_text(count(4), &[4, 4]), None);
    assert_eq!(
        placed_text(count(4), &[3, 3]).as_deref(),
        Some("Placed: 3 per path")
    );
    assert_eq!(
        placed_text(count(4), &[2, 3, 4]).as_deref(),
        Some("Placed: 2 to 4 per path")
    );
    assert_eq!(
        placed_text(count(4), &[0, 0]).as_deref(),
        Some("Placed: none")
    );
    assert_eq!(placed_text(count(0), &[0]), None);
}

#[test]
fn the_nearest_point_and_the_tangent_follow_the_outline() {
    let square = square();
    let (arc, d2) = square.nearest((20_000.0, 41_000.0));
    assert!((arc - 20_000.0).abs() < 1e-9 && (d2 - 1_000_000.0).abs() < 1e-6);
    assert_eq!(square.tangent_at(20_000.0), (1.0, 0.0));
    assert_eq!(square.tangent_at(60_000.0), (0.0, -1.0));
    assert_eq!(square.fraction_of(40_000.0).ppb(), 250_000_000);
}
