use super::targets::{push_box, push_guide};
use super::*;
use curvyo_document_core::{Document, Length, RectBounds};

fn settings(capture: f64, release: f64) -> SnapSettings {
    SnapSettings {
        x: true,
        y: true,
        enabled: true,
        objects: true,
        guides: true,
        nodes: true,
        page: true,
        grid: false,
        capture,
        release: release.max(capture),
    }
}

fn one_vertical(x: f64, kind: TargetKind) -> SnapTargets {
    SnapTargets::new(vec![SnapTarget::vertical(
        x,
        (Point::new(0.0, 0.0), Point::new(0.0, 100.0)),
        kind,
    )])
}

fn x_probe(x: f64) -> SnapProbe {
    SnapProbe {
        points: Vec::new(),
        x: vec![AxisProbe {
            value: x,
            cross_other: 50.0,
        }],
        y: Vec::new(),
    }
}

#[test]
fn a_distant_node_never_attracts_one_coordinate() {
    let targets = SnapTargets::new(vec![SnapTarget::point(
        Point::new(10.0, 1000.0),
        (Point::new(10.0, 1000.0), Point::new(10.0, 1000.0)),
        TargetKind::Node,
    )]);
    let result = resolve(
        &settings(8.0, 14.0),
        &targets,
        &SnapProbe::point(Point::new(9.0, 0.0)),
        &mut SnapState::default(),
    );
    assert!(result.is_empty());
}

#[test]
fn point_capture_uses_euclidean_distance() {
    let targets = SnapTargets::new(vec![SnapTarget::point(
        Point::new(10.0, 10.0),
        (Point::new(10.0, 10.0), Point::new(10.0, 10.0)),
        TargetKind::Node,
    )]);
    let result = resolve(
        &settings(8.0, 14.0),
        &targets,
        &SnapProbe::point(Point::new(3.0, 3.0)),
        &mut SnapState::default(),
    );
    assert!(result.is_empty());
}

#[test]
fn a_new_document_resolves_to_the_default_zones_at_one_to_one() {
    let value = SnapSettings::from_document(DocumentSnapping::DEFAULT, 1.0);
    assert!(!value.enabled);
    assert!(value.objects && value.guides && value.nodes && value.page);
    assert_eq!(value.capture, 8.0);
    assert_eq!(value.release, 14.0);
}

#[test]
fn the_zones_convert_from_pixels_by_the_zoom() {
    let value = SnapSettings::from_document(DocumentSnapping::DEFAULT, 0.5);
    assert_eq!(value.capture, 4.0);
    assert_eq!(value.release, 7.0);
}

#[test]
fn a_release_below_the_capture_is_raised_to_it_on_read() {
    let stored = DocumentSnapping {
        capture_px: 10.0,
        release_px: 3.0,
        ..DocumentSnapping::DEFAULT
    };
    let value = SnapSettings::from_document(stored, 1.0);
    assert_eq!(value.capture, 10.0);
    assert_eq!(value.release, 10.0);
}

#[test]
fn capture_is_exactly_at_the_zone_edge() {
    let targets = one_vertical(100.0, TargetKind::Box);
    let mut state = SnapState::default();
    let mut probe = x_probe(108.0);
    let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
    assert_eq!(result.held_x.unwrap().coordinate, 100.0);
    assert_eq!(result.offset.x, -8.0);
    probe.x[0].value = 108.000_1;
    let mut fresh = SnapState::default();
    let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut fresh);
    assert!(result.held_x.is_none(), "beyond capture");
}

#[test]
fn a_held_target_survives_to_the_release_edge_and_lets_go_beyond_it() {
    let targets = one_vertical(100.0, TargetKind::Box);
    let mut state = SnapState::default();
    let mut probe = x_probe(108.0);
    assert!(
        resolve(&settings(8.0, 14.0), &targets, &probe, &mut state)
            .held_x
            .is_some()
    );
    probe.x[0].value = 113.0;
    let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
    assert_eq!(result.held_x.unwrap().coordinate, 100.0, "13 is inside 14");
    probe.x[0].value = 114.0;
    let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
    assert!(result.held_x.is_some(), "exactly 14 still holds");
    probe.x[0].value = 114.001;
    let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
    assert!(result.held_x.is_none(), "beyond release and beyond capture");
}

/// A sweep across a line must capture once and release once, never flicker, because every
/// distance is measured from the raw position (criterion 4).
#[test]
fn a_slow_sweep_captures_once_and_releases_once() {
    let targets = one_vertical(100.0, TargetKind::Box);
    let mut state = SnapState::default();
    let mut captures = 0;
    let mut releases = 0;
    let mut held = false;
    let mut probe = x_probe(80.0);
    for step in 0..=400 {
        probe.x[0].value = 80.0 + f64::from(step) * 0.1;
        let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
        let now = result.held_x.is_some();
        if now && !held {
            captures += 1;
        }
        if !now && held {
            releases += 1;
        }
        held = now;
    }
    assert_eq!(captures, 1);
    assert_eq!(releases, 1);
}

#[test]
fn the_nearest_target_wins_even_against_a_higher_kind() {
    let targets = SnapTargets::new(vec![
        SnapTarget::vertical(
            100.0,
            (Point::new(0.0, 0.0), Point::new(0.0, 10.0)),
            TargetKind::Node,
        ),
        SnapTarget::vertical(
            104.0,
            (Point::new(0.0, 0.0), Point::new(0.0, 10.0)),
            TargetKind::Page,
        ),
    ]);
    let mut state = SnapState::default();
    let result = resolve(&settings(8.0, 14.0), &targets, &x_probe(105.0), &mut state);
    assert_eq!(result.held_x.unwrap().coordinate, 104.0);
    assert_eq!(result.held_x.unwrap().target, 1);
}

#[test]
fn an_exact_tie_goes_nodes_then_guides_then_boxes_then_page() {
    let kinds = [
        TargetKind::Page,
        TargetKind::Box,
        TargetKind::Guide,
        TargetKind::Node,
    ];
    let targets = SnapTargets::new(
        kinds
            .iter()
            .map(|kind| {
                SnapTarget::vertical(100.0, (Point::new(0.0, 0.0), Point::new(0.0, 10.0)), *kind)
            })
            .collect(),
    );
    let mut state = SnapState::default();
    let result = resolve(&settings(8.0, 14.0), &targets, &x_probe(103.0), &mut state);
    assert_eq!(result.held_x.unwrap().target, 3, "node entry is last");
}

#[test]
fn a_point_target_can_win_both_axes_at_once() {
    let targets = SnapTargets::new(vec![SnapTarget::point(
        Point::new(100.0, 100.0),
        (Point::new(100.0, 100.0), Point::new(100.0, 100.0)),
        TargetKind::Node,
    )]);
    let mut state = SnapState::default();
    let probe = SnapProbe::point(Point::new(104.0, 96.0));
    let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
    assert!(result.held_x.is_some() && result.held_y.is_some());
    assert_eq!(result.offset, Vec2::new(-4.0, 4.0));
    assert_eq!(result.marker.cross, Some(Point::new(100.0, 100.0)));
    assert_eq!(result.marker.lines.len(), 2);
}

#[test]
fn an_empty_probe_on_one_axis_leaves_that_axis_free() {
    let targets = SnapTargets::new(vec![
        SnapTarget::vertical(
            100.0,
            (Point::new(0.0, 0.0), Point::new(0.0, 10.0)),
            TargetKind::Box,
        ),
        SnapTarget::horizontal(
            100.0,
            (Point::new(0.0, 0.0), Point::new(10.0, 0.0)),
            TargetKind::Box,
        ),
    ]);
    let mut state = SnapState::default();
    let probe = SnapProbe {
        points: Vec::new(),
        x: vec![AxisProbe {
            value: 103.0,
            cross_other: 20.0,
        }],
        y: Vec::new(),
    };
    let result = resolve(&settings(8.0, 14.0), &targets, &probe, &mut state);
    assert!(result.held_x.is_some());
    assert!(result.held_y.is_none());
    assert_eq!(result.offset, Vec2::new(-3.0, 0.0));
    assert_eq!(result.marker.cross, Some(Point::new(100.0, 20.0)));
    assert_eq!(result.marker.lines.len(), 1);
}

#[test]
fn switching_snapping_off_clears_the_held_target_and_returns_nothing() {
    let targets = one_vertical(100.0, TargetKind::Box);
    let mut state = SnapState::default();
    assert!(
        resolve(&settings(8.0, 14.0), &targets, &x_probe(103.0), &mut state)
            .held_x
            .is_some()
    );
    let off = settings(8.0, 14.0).suspended();
    let result = resolve(&off, &targets, &x_probe(103.0), &mut state);
    assert!(result.is_empty());
    assert!(state.is_empty());
}

#[test]
fn a_disabled_kind_contributes_nothing() {
    let mut value = settings(8.0, 14.0);
    value.objects = false;
    let targets = one_vertical(100.0, TargetKind::Box);
    let mut state = SnapState::default();
    assert!(resolve(&value, &targets, &x_probe(103.0), &mut state).is_empty());
}

#[test]
fn page_fractions_are_exact_thirds_on_both_axes() {
    let settings = SnapSettings {
        x: true,
        y: true,
        enabled: true,
        objects: false,
        guides: false,
        nodes: false,
        page: true,
        capture: 1.0,
        release: 1.0,
        grid: false,
    };
    let targets = SnapTargets::gather(
        &[],
        DocumentSize::from_mm(210.0, 297.0),
        &settings,
        SnapExclusions::default(),
    );
    let vertical: Vec<f64> = targets
        .entries()
        .iter()
        .filter(|t| t.axis == TargetAxis::Vertical)
        .map(|t| t.point.x)
        .collect();
    let horizontal: Vec<f64> = targets
        .entries()
        .iter()
        .filter(|t| t.axis == TargetAxis::Horizontal)
        .map(|t| t.point.y)
        .collect();
    assert_eq!(vertical, vec![52.5, 70.0, 105.0, 140.0, 157.5, 210.0]);
    assert_eq!(horizontal, vec![74.25, 99.0, 148.5, 198.0, 222.75, 297.0]);
    assert!(!vertical.contains(&0.0));
    assert!(!horizontal.contains(&0.0));
}

#[test]
fn page_fractions_follow_a_resize_with_nothing_stored() {
    let settings = SnapSettings {
        x: true,
        y: true,
        enabled: true,
        objects: false,
        guides: false,
        nodes: false,
        page: true,
        capture: 1.0,
        release: 1.0,
        grid: false,
    };
    let targets = SnapTargets::gather(
        &[],
        DocumentSize::from_mm(300.0, 400.0),
        &settings,
        SnapExclusions::default(),
    );
    let vertical: Vec<f64> = targets
        .entries()
        .iter()
        .filter(|t| t.axis == TargetAxis::Vertical)
        .map(|t| t.point.x)
        .collect();
    assert_eq!(vertical, vec![75.0, 100.0, 150.0, 200.0, 225.0, 300.0]);
}

#[test]
fn object_boxes_contribute_three_lines_each_way_and_nine_points() {
    let mut entries = Vec::new();
    push_box(
        &mut entries,
        Point::new(0.0, 0.0),
        Point::new(10.0, 20.0),
        TargetKind::Box,
    );
    assert_eq!(
        entries
            .iter()
            .filter(|t| t.axis == TargetAxis::Vertical)
            .count(),
        3
    );
    assert_eq!(
        entries
            .iter()
            .filter(|t| t.axis == TargetAxis::Horizontal)
            .count(),
        3
    );
    assert_eq!(
        entries
            .iter()
            .filter(|t| t.axis == TargetAxis::Both)
            .count(),
        9
    );
}

#[test]
fn a_straight_guide_is_one_line_not_a_box() {
    let mut entries = Vec::new();
    push_guide(&mut entries, Point::new(0.0, 50.0), Point::new(100.0, 50.0));
    assert_eq!(entries.len(), 1);
    assert!(entries[0].curve.is_some());
    assert_eq!(entries[0].point.y, 50.0);
}

#[test]
fn gather_excludes_hidden_objects_and_keeps_locked_ones() {
    let document = Document::new(1);
    let hidden = document.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(10.0, 10.0),
    ));
    let locked = document.create_rect(RectBounds::from_corners(
        Point::new(30.0, 30.0),
        Point::new(40.0, 40.0),
    ));
    document.set_node_visible(&[hidden], false).unwrap();
    document.set_node_locked(&[locked], true).unwrap();
    let reads = document.read_nodes();
    let settings = SnapSettings {
        x: true,
        y: true,
        enabled: true,
        objects: true,
        guides: false,
        nodes: false,
        page: false,
        capture: 8.0,
        release: 14.0,
        grid: false,
    };
    let targets = SnapTargets::gather(
        &reads,
        document.size(),
        &settings,
        SnapExclusions::default(),
    );
    let xs: Vec<f64> = targets
        .entries()
        .iter()
        .filter(|t| t.axis == TargetAxis::Vertical)
        .map(|t| t.point.x)
        .collect();
    assert!(
        !xs.contains(&0.0) && !xs.contains(&10.0),
        "hidden box is no target"
    );
    assert!(
        xs.contains(&30.0) && xs.contains(&40.0),
        "locked box is a target"
    );
}

#[test]
fn gather_skips_the_dragged_object() {
    let document = Document::new(1);
    let dragged = document.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(10.0, 10.0),
    ));
    let other = document.create_rect(RectBounds::from_corners(
        Point::new(30.0, 30.0),
        Point::new(40.0, 40.0),
    ));
    let mut excluded = HashSet::new();
    excluded.insert(dragged);
    let reads = document.read_nodes();
    let settings = SnapSettings {
        x: true,
        y: true,
        enabled: true,
        objects: true,
        guides: false,
        nodes: false,
        page: false,
        capture: 8.0,
        release: 14.0,
        grid: false,
    };
    let targets = SnapTargets::gather(
        &reads,
        document.size(),
        &settings,
        SnapExclusions {
            boxes: Some(&excluded),
            nodes: Some(&excluded),
            anchor: None,
            anchors: None,
        },
    );
    let xs: Vec<f64> = targets
        .entries()
        .iter()
        .filter(|t| t.axis == TargetAxis::Vertical)
        .map(|t| t.point.x)
        .collect();
    assert!(!xs.contains(&0.0) && !xs.contains(&10.0));
    assert!(xs.contains(&30.0) && xs.contains(&40.0));
    assert!(document.object(other).is_some());
}

#[test]
fn a_guide_clone_and_its_anchors_are_targets() {
    let document = Document::new(1);
    let guide = document.create_path(
        &[
            curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 70.0)),
            curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 2), Point::new(100.0, 70.0)),
        ],
        false,
    );
    document.set_guide(&[guide], true).unwrap();
    let reads = document.read_nodes();
    let settings = SnapSettings {
        x: true,
        y: true,
        enabled: true,
        objects: false,
        guides: true,
        nodes: false,
        page: false,
        capture: 8.0,
        release: 14.0,
        grid: false,
    };
    let targets = SnapTargets::gather(
        &reads,
        document.size(),
        &settings,
        SnapExclusions::default(),
    );
    assert!(targets.entries().iter().any(|t| {
        t.curve
            .is_some_and(|c| (c.start.y - 70.0).abs() < 1e-9 && (c.end.y - 70.0).abs() < 1e-9)
    }));
    assert_eq!(
        targets
            .entries()
            .iter()
            .filter(|t| t.curve.is_none()
                && t.axis == TargetAxis::Both
                && t.kind == TargetKind::Guide)
            .count(),
        2
    );
}

#[test]
fn the_node_tool_can_add_its_own_path_anchors_but_not_the_dragged_one() {
    let document = Document::new(1);
    let id = document.create_path(
        &[
            curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 2), Point::new(50.0, 50.0)),
        ],
        false,
    );
    let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
        panic!("path")
    };
    let settings = SnapSettings {
        x: true,
        y: true,
        enabled: true,
        objects: false,
        guides: false,
        nodes: true,
        page: false,
        capture: 8.0,
        release: 14.0,
        grid: false,
    };
    let reads = document.read_nodes();
    let mut excluded = HashSet::new();
    excluded.insert(id);
    let mut targets = SnapTargets::gather(
        &reads,
        document.size(),
        &settings,
        SnapExclusions {
            boxes: Some(&excluded),
            nodes: Some(&excluded),
            anchor: None,
            anchors: None,
        },
    );
    assert!(targets.is_empty());
    targets.add_path_nodes(&path, Some(AnchorId::new(1, 1)));
    assert_eq!(targets.len(), 1);
    assert_eq!(targets.entries()[0].point, Point::new(50.0, 50.0));
}

#[test]
fn gather_excludes_one_dragged_anchor_from_an_otherwise_included_path() {
    let document = Document::new(1);
    let id = document.create_path(
        &[
            curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            curvyo_document_core::NewAnchor::corner(AnchorId::new(1, 2), Point::new(50.0, 50.0)),
        ],
        false,
    );
    let reads = document.read_nodes();
    let settings = SnapSettings {
        x: true,
        y: true,
        enabled: true,
        objects: false,
        guides: false,
        nodes: true,
        page: false,
        capture: 8.0,
        release: 14.0,
        grid: false,
    };
    let targets = SnapTargets::gather(
        &reads,
        document.size(),
        &settings,
        SnapExclusions {
            boxes: None,
            nodes: None,
            anchor: Some((id, AnchorId::new(1, 1))),
            anchors: None,
        },
    );
    assert_eq!(targets.len(), 1);
    assert_eq!(targets.entries()[0].point, Point::new(50.0, 50.0));
}

#[test]
fn a_box_move_snaps_by_its_edges_and_centre_lines() {
    let document = Document::new(1);
    let _ = document.create_rect(RectBounds::from_corners(
        Point::new(100.0, 100.0),
        Point::new(110.0, 120.0),
    ));
    let reads = document.read_nodes();
    let settings = SnapSettings {
        x: true,
        y: true,
        enabled: true,
        objects: true,
        guides: false,
        nodes: false,
        page: false,
        capture: 8.0,
        release: 14.0,
        grid: false,
    };
    let targets = SnapTargets::gather(
        &reads,
        document.size(),
        &settings,
        SnapExclusions::default(),
    );
    let mut state = SnapState::default();
    let probe = SnapProbe::box_edges(Point::new(92.0, 92.0), Point::new(102.0, 112.0));
    let result = resolve(&settings, &targets, &probe, &mut state);
    assert_eq!(result.offset, Vec2::new(-2.0, -2.0));
    assert_eq!(result.marker.cross, Some(Point::new(100.0, 100.0)));
}

#[test]
fn a_length_of_two_thirds_is_an_exact_third_not_a_decimal() {
    assert!((210.0 * PAGE_FRACTIONS[3] - 140.0).abs() < 1e-9);
    assert!((297.0 * PAGE_FRACTIONS[1] - 99.0).abs() < 1e-9);
    assert!((Length::from_mm(297.0).as_mm() * PAGE_FRACTIONS[4] - 222.75).abs() < 1e-9);
}

#[test]
fn a_diagonal_guide_projects_onto_its_segment_not_its_box() {
    let mut entries = Vec::new();
    push_guide(&mut entries, Point::new(0.0, 0.0), Point::new(100.0, 100.0));
    let targets = SnapTargets::new(entries);
    let result = resolve(
        &settings(8.0, 14.0),
        &targets,
        &SnapProbe::point(Point::new(49.0, 51.0)),
        &mut SnapState::default(),
    );
    assert!((result.offset.x - 1.0).abs() < 1e-8);
    assert!((result.offset.y + 1.0).abs() < 1e-8);
    let distant = resolve(
        &settings(8.0, 14.0),
        &targets,
        &SnapProbe::point(Point::new(1.0, 50.0)),
        &mut SnapState::default(),
    );
    assert!(distant.is_empty());
}

#[test]
fn moving_a_group_excludes_its_descendants() {
    let d = Document::new(5);
    let a = d.create_rect(RectBounds::from_corners(
        Point::new(10.0, 10.0),
        Point::new(20.0, 20.0),
    ));
    let b = d.create_rect(RectBounds::from_corners(
        Point::new(40.0, 10.0),
        Point::new(50.0, 20.0),
    ));
    let group = d.group_objects(&[a, b]).unwrap();
    let excluded = HashSet::from([group]);
    let settings = SnapSettings {
        page: false,
        ..settings(8.0, 14.0)
    };
    let targets = SnapTargets::gather(
        &d.read_nodes(),
        d.size(),
        &settings,
        SnapExclusions {
            boxes: Some(&excluded),
            nodes: Some(&excluded),
            anchor: None,
            anchors: None,
        },
    );
    assert_eq!(targets.len(), 0);
    let targets = SnapTargets::gather(
        &d.read_nodes(),
        d.size(),
        &settings,
        SnapExclusions::default(),
    );
    assert!(
        targets
            .entries()
            .iter()
            .any(|t| t.axis == TargetAxis::Vertical && (t.point.x - 30.0).abs() < 1e-9)
    );
}

#[test]
fn moving_a_child_excludes_its_aggregate_group_but_keeps_stationary_sibling() {
    let d = Document::new(5);
    let a = d.create_rect(RectBounds::from_corners(
        Point::new(10.0, 10.0),
        Point::new(20.0, 20.0),
    ));
    let b = d.create_rect(RectBounds::from_corners(
        Point::new(40.0, 10.0),
        Point::new(50.0, 20.0),
    ));
    d.group_objects(&[a, b]).unwrap();
    let excluded = HashSet::from([a]);
    let settings = SnapSettings {
        page: false,
        ..settings(8.0, 14.0)
    };
    let targets = SnapTargets::gather(
        &d.read_nodes(),
        d.size(),
        &settings,
        SnapExclusions {
            boxes: Some(&excluded),
            nodes: Some(&excluded),
            anchor: None,
            anchors: None,
        },
    );
    assert!(
        !targets
            .entries()
            .iter()
            .any(|t| t.axis == TargetAxis::Vertical && (t.point.x - 30.0).abs() < 1e-9)
    );
    assert!(
        targets
            .entries()
            .iter()
            .any(|t| t.axis == TargetAxis::Vertical && (t.point.x - 45.0).abs() < 1e-9)
    );
}

#[test]
fn constrained_motion_hits_diagonal_guide_without_changing_locked_coordinate() {
    let mut entries = Vec::new();
    push_guide(&mut entries, Point::new(0.0, 0.0), Point::new(100.0, 100.0));
    let target = SnapTargets::new(entries);
    let r = resolve_along(
        &settings(8.0, 14.0),
        &target,
        Point::new(46.0, 50.0),
        Vec2::new(1.0, 0.0),
        &mut SnapState::default(),
    );
    assert!((r.offset.x - 4.0).abs() < 1e-8);
    assert!(r.offset.y.abs() < 1e-9);
    let node = SnapTargets::new(vec![SnapTarget::point(
        Point::new(50.0, 51.0),
        (Point::new(50.0, 51.0), Point::new(50.0, 51.0)),
        TargetKind::Node,
    )]);
    assert!(
        resolve_along(
            &settings(8.0, 14.0),
            &node,
            Point::new(46.0, 50.0),
            Vec2::new(1.0, 0.0),
            &mut SnapState::default()
        )
        .is_empty()
    );
}

#[test]
fn spatial_pages_contribute_exact_translated_fractions_without_zero_edges() {
    let d = Document::new(9);
    let second = d.add_page();
    d.move_page(&second.id, Vec2::new(123.0, -450.0), &[])
        .unwrap();
    let pages = d.pages();
    let targets =
        SnapTargets::gather_pages(&[], &pages, &settings(8.0, 14.0), SnapExclusions::default());
    assert_eq!(targets.len(), pages.len() * 12);
    for page in pages {
        for fraction in PAGE_FRACTIONS {
            assert!(
                targets
                    .entries()
                    .iter()
                    .any(|t| t.axis == TargetAxis::Vertical
                        && (t.point.x - page.origin.x - page.size.width.as_mm() * fraction).abs()
                            < 1e-9)
            );
            assert!(
                targets
                    .entries()
                    .iter()
                    .any(|t| t.axis == TargetAxis::Horizontal
                        && (t.point.y - page.origin.y - page.size.height.as_mm() * fraction).abs()
                            < 1e-9)
            );
        }
        assert!(
            !targets
                .entries()
                .iter()
                .any(|t| t.axis == TargetAxis::Horizontal
                    && (t.point.y - page.origin.y).abs() < 1e-9)
        );
    }
}

#[test]
fn rotated_object_targets_use_drawn_axis_aligned_bounds() {
    let d = Document::new(8);
    let id = d.create_rect(RectBounds::from_corners(
        Point::new(50.0, 50.0),
        Point::new(70.0, 70.0),
    ));
    let object = d.object(id).unwrap().rotated(
        Point::new(60.0, 60.0),
        curvyo_document_core::Angle::from_radians(std::f64::consts::FRAC_PI_4),
    );
    d.rotate_object(&object).unwrap();
    let targets = SnapTargets::gather(
        &d.read_nodes(),
        d.size(),
        &SnapSettings {
            page: false,
            ..settings(8.0, 14.0)
        },
        SnapExclusions::default(),
    );
    let edge = 60.0 - 10.0 * 2.0_f64.sqrt();
    assert!(
        targets
            .entries()
            .iter()
            .any(|t| t.axis == TargetAxis::Vertical && (t.point.x - edge).abs() < 1e-9)
    );
    assert!(
        !targets
            .entries()
            .iter()
            .any(|t| t.axis == TargetAxis::Vertical && (t.point.x - 50.0).abs() < 1e-9)
    );
}

#[test]
fn composition_searches_beyond_the_nearest_finite_guide() {
    let mut entries = vec![SnapTarget::vertical(
        100.0,
        (Point::new(100.0, 90.0), Point::new(100.0, 110.0)),
        TargetKind::Box,
    )];
    push_guide(&mut entries, Point::new(90.0, 99.5), Point::new(99.5, 99.5));
    push_guide(
        &mut entries,
        Point::new(90.0, 100.0),
        Point::new(110.0, 100.0),
    );
    let targets = SnapTargets::new(entries);
    let r = resolve(
        &settings(8.0, 14.0),
        &targets,
        &SnapProbe::point(Point::new(99.0, 99.0)),
        &mut SnapState::default(),
    );
    assert_eq!(r.offset, Vec2::new(1.0, 1.0));
}

#[test]
fn axis_mask_requires_reachable_points_and_exact_curve_intersection() {
    let mut config = settings(8.0, 14.0);
    config.y = false;
    let point = |p| SnapTargets::new(vec![SnapTarget::point(p, (p, p), TargetKind::Node)]);
    let raw = SnapProbe::point(Point::new(99.0, 99.0));
    assert!(
        resolve(
            &config,
            &point(Point::new(100.0, 100.0)),
            &raw,
            &mut SnapState::default()
        )
        .is_empty()
    );
    let r = resolve(
        &config,
        &point(Point::new(100.0, 99.0)),
        &raw,
        &mut SnapState::default(),
    );
    assert_eq!(r.offset, Vec2::new(1.0, 0.0));
    let mut guides = Vec::new();
    push_guide(
        &mut guides,
        Point::new(90.0, 90.0),
        Point::new(110.0, 110.0),
    );
    let r = resolve(
        &config,
        &SnapTargets::new(guides),
        &SnapProbe::point(Point::new(98.0, 100.0)),
        &mut SnapState::default(),
    );
    assert_eq!(r.offset, Vec2::new(2.0, 0.0));
    config.x = false;
    assert!(
        resolve(
            &config,
            &point(Point::new(100.0, 99.0)),
            &raw,
            &mut SnapState::default()
        )
        .is_empty()
    );
}

#[test]
fn axis_constraints_are_independent_of_direction_vector_scale() {
    let config = SnapSettings {
        y: false,
        ..settings(8.0, 14.0)
    };
    let targets = one_vertical(100.0, TargetKind::Box);
    for scale in [1.0, 1e-8] {
        let r = resolve_along(
            &config,
            &targets,
            Point::new(99.0, 99.0),
            Vec2::new(scale, 0.05 * scale),
            &mut SnapState::default(),
        );
        assert!(r.is_empty());
    }
}
