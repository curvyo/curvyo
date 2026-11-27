use super::*;

fn rect(x: i64, y: i64, w: i64, h: i64) -> Vec<PointUm> {
    vec![
        PointUm::new(x, y),
        PointUm::new(x + w, y),
        PointUm::new(x + w, y + h),
        PointUm::new(x, y + h),
    ]
}

fn closed(points: Vec<PointUm>, stacking: u32) -> Unit {
    Unit {
        shape: UnitShape::Path {
            points,
            closed: true,
        },
        stacking,
        outline: 0,
        cut: None,
    }
}

fn firsts(paths: &[OrderedPath]) -> Vec<usize> {
    paths.iter().map(|p| p.unit).collect()
}

#[test]
fn inner_shapes_run_before_their_outlines_to_any_depth() {
    // A rectangle, a hole inside it, a text-like square inside the hole (criterion 22 test).
    let units = vec![
        closed(rect(0, 0, 100_000, 100_000), 0),
        closed(rect(20_000, 20_000, 60_000, 60_000), 0),
        closed(rect(40_000, 40_000, 20_000, 20_000), 1),
    ];
    let (paths, _) = order(&units, PointUm::new(0, 0), true);
    assert_eq!(firsts(&paths), [2, 1, 0]);
}

#[test]
fn the_nearest_start_goes_next_and_a_closed_path_starts_at_its_nearest_node() {
    let units = vec![
        closed(rect(50_000, 0, 10_000, 10_000), 0),
        closed(rect(10_000, 0, 10_000, 10_000), 1),
    ];
    let (paths, exit) = order(&units, PointUm::new(0, 20_000), true);
    assert_eq!(firsts(&paths), [1, 0]);
    // The first square starts at its bottom-left node (nearest to the head), drawn direction kept.
    assert_eq!(paths[0].points[0], PointUm::new(10_000, 10_000));
    assert_eq!(paths[0].points[1], PointUm::new(10_000, 0));
    assert_eq!(
        paths[0].points.last(),
        paths[0].points.first(),
        "closed returns to its start"
    );
    assert_eq!(exit, paths[1].points[0]);
}

#[test]
fn an_open_path_runs_from_its_nearer_end() {
    let units = vec![Unit {
        shape: UnitShape::Path {
            points: vec![PointUm::new(0, 0), PointUm::new(10_000, 0)],
            closed: false,
        },
        stacking: 0,
        outline: 0,
        cut: None,
    }];
    let (paths, exit) = order(&units, PointUm::new(12_000, 0), true);
    assert_eq!(
        paths[0].points,
        [PointUm::new(10_000, 0), PointUm::new(0, 0)]
    );
    assert_eq!(exit, PointUm::new(0, 0));
}

#[test]
fn an_open_path_inside_a_closed_one_runs_first() {
    let units = vec![
        closed(rect(0, 0, 100_000, 100_000), 0),
        Unit {
            shape: UnitShape::Path {
                points: vec![PointUm::new(40_000, 50_000), PointUm::new(60_000, 50_000)],
                closed: false,
            },
            stacking: 1,
            outline: 0,
            cut: None,
        },
    ];
    let (paths, _) = order(&units, PointUm::new(0, 0), true);
    assert_eq!(firsts(&paths), [1, 0]);
}

#[test]
fn ties_go_to_the_lower_stacking_order() {
    let units = vec![
        closed(rect(10_000, 0, 10_000, 10_000), 5),
        closed(rect(-20_000, 0, 10_000, 10_000), 2),
    ];
    // Both nearest nodes lie 10 mm from the head.
    let (paths, _) = order(&units, PointUm::new(0, 0), true);
    assert_eq!(firsts(&paths), [1, 0]);
}

#[test]
fn a_hatch_is_entered_at_its_nearest_corner_and_alternates() {
    let lines: HatchLines = (0..3)
        .map(|k| vec![(PointUm::new(0, k * 1_000), PointUm::new(10_000, k * 1_000))])
        .collect();
    let units = vec![Unit {
        shape: UnitShape::Hatch(vec![lines]),
        stacking: 0,
        outline: 0,
        cut: None,
    }];
    let (paths, exit) = order(&units, PointUm::new(11_000, 3_000), true);
    let spans: Vec<(PointUm, PointUm)> = paths.iter().map(|p| (p.points[0], p.points[1])).collect();
    assert_eq!(
        spans,
        [
            (PointUm::new(10_000, 2_000), PointUm::new(0, 2_000)),
            (PointUm::new(0, 1_000), PointUm::new(10_000, 1_000)),
            (PointUm::new(10_000, 0), PointUm::new(0, 0)),
        ]
    );
    assert_eq!(exit, PointUm::new(0, 0));
    let (one_way, _) = order(&units, PointUm::new(0, 0), false);
    assert!(
        one_way.iter().all(|p| p.points[0].x == 0),
        "every line the same way"
    );
}

#[test]
fn every_closed_path_runs_after_all_it_contains_on_random_fixtures() {
    // A small deterministic generator: nested and side-by-side squares.
    let mut seed: u64 = 7;
    let mut next = |m: i64| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        i64::try_from((seed >> 33) % u64::try_from(m).unwrap()).unwrap()
    };
    for _ in 0..20 {
        let mut units = Vec::new();
        for i in 0..12 {
            let (x, y) = (next(200) * 1_000, next(200) * 1_000);
            let side = (1 + next(60)) * 1_000;
            units.push(closed(rect(x, y, side, side), i));
        }
        let (paths, _) = order(&units, PointUm::new(0, 0), true);
        assert_eq!(paths.len(), units.len());
        let parents = parents(&units);
        let at: Vec<usize> = (0..units.len())
            .map(|u| paths.iter().position(|p| p.unit == u).unwrap())
            .collect();
        for (child, parent) in parents.iter().enumerate() {
            if let Some(parent) = parent {
                assert!(
                    at[child] < at[*parent],
                    "child {child} before parent {parent}"
                );
            }
        }
        let (again, _) = order(&units, PointUm::new(0, 0), true);
        assert_eq!(paths, again, "deterministic");
    }
}
