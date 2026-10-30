//! Independent snapping oracles from specification 0060's acceptance plan.
#![allow(clippy::unwrap_used, clippy::float_cmp)]
use curvyo_document_core::{Document, DocumentSize, DocumentSnapping, Point, Vec2};
use curvyo_ui_core::{
    MarkerLine, SnapCurve, SnapExclusions, SnapProbe, SnapSettings, SnapState, SnapTarget,
    SnapTargets, TargetAxis, TargetKind, resolve,
};
fn settings(mm_per_px: f64) -> SnapSettings {
    SnapSettings::from_document(
        DocumentSnapping {
            enabled: true,
            ..DocumentSnapping::DEFAULT
        },
        mm_per_px,
    )
}
fn point(x: f64, y: f64, kind: TargetKind) -> SnapTarget {
    let p = Point::new(x, y);
    SnapTarget::point(p, (p, p), kind)
}
fn line(x: f64, kind: TargetKind) -> SnapTarget {
    SnapTarget::vertical(x, (Point::new(x, -100.0), Point::new(x, 100.0)), kind)
}
fn corrected(targets: &SnapTargets, p: Point) -> (Point, curvyo_ui_core::SnapResult) {
    let result = resolve(
        &settings(1.0),
        targets,
        &SnapProbe::point(p),
        &mut SnapState::default(),
    );
    (p.translated(result.offset), result)
}

#[test]
fn ac3_ac4_capture_and_raw_release_are_screen_distances_at_every_zoom() {
    for mm in [0.125, 1.0, 8.0] {
        let targets = SnapTargets::new(vec![line(100.0, TargetKind::Box)]);
        let mut state = SnapState::default();
        for distance_px in [8.0, 9.0, 12.0, 14.0] {
            let raw = Point::new(100.0 + distance_px * mm, 27.0);
            let r = resolve(&settings(mm), &targets, &SnapProbe::point(raw), &mut state);
            assert!((raw.x + r.offset.x - 100.0).abs() < 1e-9);
            assert_eq!(raw.y + r.offset.y, 27.0);
        }
        let r = resolve(
            &settings(mm),
            &targets,
            &SnapProbe::point(Point::new(100.0 + 14.1 * mm, 27.0)),
            &mut state,
        );
        assert!(r.is_empty());
        assert!(r.marker.is_empty());
        assert!(state.is_empty());
        let raw = Point::new(100.0 + 9.0 * mm, 27.0);
        assert!(resolve(&settings(mm), &targets, &SnapProbe::point(raw), &mut state).is_empty());
    }
}

#[test]
fn ac6_nodes_are_atomic_and_distant_projections_cannot_make_a_ghost_point() {
    let ghosts = SnapTargets::new(vec![
        point(1.0, 100.0, TargetKind::Node),
        point(100.0, 1.0, TargetKind::Node),
    ]);
    assert!(corrected(&ghosts, Point::new(0.0, 0.0)).1.is_empty());
    for (line_x, expected) in [
        (4.0, Point::new(4.0, 0.0)),
        (5.0, Point::new(3.0, 4.0)),
        (6.0, Point::new(3.0, 4.0)),
    ] {
        let targets = SnapTargets::new(vec![
            line(line_x, TargetKind::Box),
            point(3.0, 4.0, TargetKind::Node),
        ]);
        let (p, r) = corrected(&targets, Point::new(0.0, 0.0));
        assert_eq!(p, expected);
        if line_x >= 5.0 {
            assert_eq!(r.held_x.unwrap().target, r.held_y.unwrap().target);
            assert_eq!(r.marker.cross, Some(expected));
        }
    }
}

#[test]
fn ac6_nearest_then_kind_ties_are_order_independent_and_real_lines_can_mix() {
    for reverse in [false, true] {
        let mut entries = vec![
            line(5.0, TargetKind::Page),
            line(5.0, TargetKind::Box),
            line(5.0, TargetKind::Guide),
        ];
        if reverse {
            entries.reverse();
        }
        let targets = SnapTargets::new(entries);
        let r = corrected(&targets, Point::new(0.0, 0.0)).1;
        assert_eq!(
            targets.entries()[r.held_x.unwrap().target].kind,
            TargetKind::Guide
        );
    }
    let targets = SnapTargets::new(vec![
        line(10.0, TargetKind::Box),
        SnapTarget::horizontal(
            20.0,
            (Point::new(0.0, 20.0), Point::new(100.0, 20.0)),
            TargetKind::Guide,
        ),
    ]);
    let (p, r) = corrected(&targets, Point::new(8.0, 17.0));
    assert_eq!(p, Point::new(10.0, 20.0));
    assert_eq!(r.marker.lines.len(), 2);
}

#[test]
fn ac5_ac8_true_bezier_guide_matches_curve_not_chord_or_control_polygon() {
    let curve = SnapCurve {
        start: Point::new(0.0, 0.0),
        handle_out: Vec2::new(0.0, 100.0),
        handle_in: Vec2::new(0.0, 100.0),
        end: Point::new(100.0, 0.0),
    };
    let mut target = point(0.0, 0.0, TargetKind::Guide);
    target.curve = Some(curve);
    target.extent = (Point::new(0.0, 0.0), Point::new(100.0, 100.0));
    let (p, r) = corrected(&SnapTargets::new(vec![target]), Point::new(50.0, 78.0));
    assert!(
        (p.x - 50.0).abs() < 1e-6 && (p.y - 75.0).abs() < 1e-6,
        "{p:?}"
    );
    assert_eq!(r.marker.lines, vec![MarkerLine::Curve(curve)]);
    let cross = r.marker.cross.unwrap();
    assert!((cross.x - p.x).abs() < 1e-9 && (cross.y - p.y).abs() < 1e-9);
}

#[test]
fn ac5_ac15_page_fractions_have_exact_thirds_global_origins_and_no_zero_line() {
    let d = Document::new(601);
    d.resize_page("first", DocumentSize::from_mm(300.0, 120.0), &[])
        .unwrap();
    d.move_page("first", Vec2::new(-40.0, 10.0), &[]).unwrap();
    let p = d.add_page();
    let targets =
        SnapTargets::gather_pages(&[], &d.pages(), &settings(1.0), SnapExclusions::default());
    assert_eq!(targets.len(), 24);
    for (axis, expected) in [
        (
            TargetAxis::Vertical,
            vec![35.0, 60.0, 110.0, 160.0, 185.0, 260.0],
        ),
        (
            TargetAxis::Horizontal,
            vec![40.0, 50.0, 70.0, 90.0, 100.0, 130.0],
        ),
    ] {
        let actual: Vec<_> = targets
            .entries()
            .iter()
            .take(12)
            .filter(|t| t.axis == axis)
            .map(|t| {
                if axis == TargetAxis::Vertical {
                    t.point.x
                } else {
                    t.point.y
                }
            })
            .collect();
        assert_eq!(actual, expected);
    }
    assert!(
        targets
            .entries()
            .iter()
            .any(|t| t.axis == TargetAxis::Vertical
                && (t.point.x - (p.origin.x + 100.0)).abs() < 1e-9)
    );
    d.move_page("first", Vec2::new(7.0, -3.0), &[]).unwrap();
    let updated =
        SnapTargets::gather_pages(&[], &d.pages(), &settings(1.0), SnapExclusions::default());
    assert_eq!(updated.entries()[0].point.x, 42.0);
}

#[test]
fn ac11_index_never_loses_the_nearby_target_late_in_document_order() {
    for reverse in [false, true] {
        let mut entries: Vec<_> = (0..10_000)
            .map(|n| point(1000.0 + f64::from(n), 2000.0, TargetKind::Node))
            .collect();
        entries.push(point(3.0, 4.0, TargetKind::Node));
        if reverse {
            entries.reverse();
        }
        let (p, r) = corrected(&SnapTargets::new(entries), Point::new(0.0, 0.0));
        assert_eq!(p, Point::new(3.0, 4.0));
        assert_eq!(r.marker.cross, Some(p));
    }
}

#[test]
fn ac2_ac5_disabled_kind_or_temporary_suppression_drops_held_markers() {
    for kind in [
        TargetKind::Node,
        TargetKind::Guide,
        TargetKind::Box,
        TargetKind::Page,
    ] {
        let targets = SnapTargets::new(vec![point(3.0, 4.0, kind)]);
        let mut state = SnapState::default();
        let probe = SnapProbe::point(Point::new(0.0, 0.0));
        assert!(!resolve(&settings(1.0), &targets, &probe, &mut state).is_empty());
        let mut disabled = settings(1.0);
        match kind {
            TargetKind::Node => disabled.nodes = false,
            TargetKind::Guide => disabled.guides = false,
            TargetKind::Box => disabled.objects = false,
            TargetKind::Page => disabled.page = false,
        }
        assert!(resolve(&disabled, &targets, &probe, &mut state).is_empty());
        assert!(state.is_empty());
        resolve(&settings(1.0), &targets, &probe, &mut state);
        assert!(
            resolve(&settings(1.0).suspended(), &targets, &probe, &mut state)
                .marker
                .is_empty()
        );
        assert!(state.is_empty());
    }
}

#[test]
fn ac6_finite_guide_and_box_edge_combine_on_both_axis_orientations_and_hold() {
    for vertical_guide in [false, true] {
        let curve = if vertical_guide {
            SnapCurve {
                start: Point::new(100.0, 0.0),
                end: Point::new(100.0, 200.0),
                handle_out: Vec2::ZERO,
                handle_in: Vec2::ZERO,
            }
        } else {
            SnapCurve {
                start: Point::new(0.0, 100.0),
                end: Point::new(200.0, 100.0),
                handle_out: Vec2::ZERO,
                handle_in: Vec2::ZERO,
            }
        };
        let mut guide = point(curve.start.x, curve.start.y, TargetKind::Guide);
        guide.extent = (curve.start, curve.end);
        guide.curve = Some(curve);
        let edge = if vertical_guide {
            SnapTarget::horizontal(
                100.0,
                (Point::new(0.0, 100.0), Point::new(200.0, 100.0)),
                TargetKind::Box,
            )
        } else {
            line(100.0, TargetKind::Box)
        };
        let targets = SnapTargets::new(vec![edge, guide]);
        let mut state = SnapState::default();
        for raw in [Point::new(99.0, 99.0), Point::new(102.0, 103.0)] {
            let result = resolve(&settings(1.0), &targets, &SnapProbe::point(raw), &mut state);
            assert_eq!(
                raw.translated(result.offset),
                Point::new(100.0, 100.0),
                "vertical_guide={vertical_guide}"
            );
            assert_eq!(result.marker.lines.len(), 2);
            assert_eq!(result.marker.cross, Some(Point::new(100.0, 100.0)));
        }
    }
}

#[test]
fn ac5_locked_group_is_a_reference_hidden_group_is_not_and_moving_descendants_are_excluded() {
    use curvyo_document_core::RectBounds;
    use std::collections::HashSet;
    let d = Document::new(621);
    let a = d.create_rect(RectBounds::from_corners(
        Point::new(10.0, 10.0),
        Point::new(20.0, 20.0),
    ));
    let b = d.create_rect(RectBounds::from_corners(
        Point::new(40.0, 10.0),
        Point::new(50.0, 20.0),
    ));
    let group = d.group_objects(&[a, b]).unwrap();
    let mut options = settings(1.0);
    options.nodes = false;
    options.guides = false;
    options.page = false;
    let visible = SnapTargets::gather(
        &d.read_nodes(),
        d.size(),
        &options,
        SnapExclusions::default(),
    );
    assert!(
        visible
            .entries()
            .iter()
            .any(|t| t.axis == TargetAxis::Vertical && (t.point.x - 30.0).abs() < 1e-9)
    );
    d.set_node_locked(&[group], true).unwrap();
    let locked = SnapTargets::gather(
        &d.read_nodes(),
        d.size(),
        &options,
        SnapExclusions::default(),
    );
    assert_eq!(locked.entries(), visible.entries());
    let exclude = HashSet::from([group]);
    assert!(
        SnapTargets::gather(
            &d.read_nodes(),
            d.size(),
            &options,
            SnapExclusions {
                boxes: Some(&exclude),
                nodes: Some(&exclude),
                ..SnapExclusions::default()
            }
        )
        .is_empty()
    );
    d.set_node_visible(&[group], false).unwrap();
    assert!(
        SnapTargets::gather(
            &d.read_nodes(),
            d.size(),
            &options,
            SnapExclusions::default()
        )
        .is_empty()
    );
}

#[test]
fn ac5_ac13_all_moving_anchors_and_following_clones_are_excluded_but_stationary_anchor_remains() {
    use curvyo_document_core::{AnchorId, NewAnchor};
    use std::collections::HashSet;
    let d = Document::new(622);
    let ids = [
        AnchorId::new(622, 1),
        AnchorId::new(622, 2),
        AnchorId::new(622, 3),
    ];
    let source = d.create_path(
        &[
            NewAnchor::corner(ids[0], Point::new(10.0, 10.0)),
            NewAnchor::corner(ids[1], Point::new(40.0, 10.0)),
            NewAnchor::corner(ids[2], Point::new(80.0, 80.0)),
        ],
        false,
    );
    d.linked_clone(&[source], &[]).unwrap();
    let moving = HashSet::from([source]);
    let anchors = HashSet::from([(source, ids[0]), (source, ids[1])]);
    let mut options = settings(1.0);
    options.objects = false;
    options.guides = false;
    options.page = false;
    let targets = SnapTargets::gather(
        &d.read_nodes(),
        d.size(),
        &options,
        SnapExclusions {
            boxes: Some(&moving),
            anchors: Some(&anchors),
            ..SnapExclusions::default()
        },
    );
    assert_eq!(targets.len(), 1);
    assert_eq!(targets.entries()[0].point, Point::new(80.0, 80.0));
    let all = SnapTargets::gather(
        &d.read_nodes(),
        d.size(),
        &options,
        SnapExclusions {
            boxes: Some(&moving),
            nodes: Some(&moving),
            ..SnapExclusions::default()
        },
    );
    assert!(all.is_empty());
}

#[test]
fn ac6_composition_never_extends_finite_guides_or_overrides_a_nearer_atomic_node() {
    let make_guide = |end_x: f64| {
        let curve = SnapCurve {
            start: Point::new(0.0, 100.0),
            end: Point::new(end_x, 100.0),
            handle_out: Vec2::ZERO,
            handle_in: Vec2::ZERO,
        };
        let mut guide = point(0.0, 100.0, TargetKind::Guide);
        guide.curve = Some(curve);
        guide.extent = (curve.start, curve.end);
        guide
    };
    let raw = Point::new(99.0, 99.0);
    for reverse in [false, true] {
        let mut entries = vec![line(100.0, TargetKind::Box), make_guide(99.0)];
        if reverse {
            entries.reverse();
        }
        let (p, r) = corrected(&SnapTargets::new(entries), raw);
        // Cubic endpoint projection is numerical, even for a straight guide.
        assert!((p.x - 99.0).abs() < 1e-8 && (p.y - 100.0).abs() < 1e-8);
        assert_eq!(r.marker.lines.len(), 1);
        let targets = SnapTargets::new(vec![
            line(100.0, TargetKind::Box),
            make_guide(200.0),
            point(99.0, 99.5, TargetKind::Node),
        ]);
        let (p, r) = corrected(&targets, raw);
        assert_eq!(p, Point::new(99.0, 99.5));
        assert_eq!(r.held_x.unwrap().target, r.held_y.unwrap().target);
        // UX Both axes explicitly requires two node-centred lines.
        assert_eq!(r.marker.lines.len(), 2);
        assert_eq!(r.marker.cross, Some(p));
    }
}

#[test]
fn ac6_held_edge_uses_a_compatible_guide_when_nearest_finite_guide_cannot_intersect() {
    let guide = |y: f64, end: f64| {
        let curve = SnapCurve {
            start: Point::new(90.0, y),
            end: Point::new(end, y),
            handle_out: Vec2::ZERO,
            handle_in: Vec2::ZERO,
        };
        let mut t = point(90.0, y, TargetKind::Guide);
        t.curve = Some(curve);
        t.extent = (curve.start, curve.end);
        t
    };
    let targets = SnapTargets::new(vec![
        line(100.0, TargetKind::Box),
        guide(99.5, 99.5),
        guide(100.0, 110.0),
    ]);
    let mut state = SnapState::default();
    let first = Point::new(99.0, 80.0);
    let r = resolve(
        &settings(1.0),
        &targets,
        &SnapProbe::point(first),
        &mut state,
    );
    assert_eq!(first.translated(r.offset), Point::new(100.0, 80.0));
    let next = Point::new(99.0, 99.0);
    let r = resolve(
        &settings(1.0),
        &targets,
        &SnapProbe::point(next),
        &mut state,
    );
    assert_eq!(next.translated(r.offset), Point::new(100.0, 100.0));
    assert_eq!(r.marker.lines.len(), 2);
}

#[test]
fn ac16_axis_masks_reject_unreachable_nodes_and_drop_incompatible_holds() {
    let raw = Point::new(99.0, 99.0);
    let lines = SnapTargets::new(vec![
        line(100.0, TargetKind::Box),
        SnapTarget::horizontal(100.0, (raw, Point::new(120.0, 100.0)), TargetKind::Page),
    ]);
    let mut state = SnapState::default();
    for (x, y) in [(true, true), (true, false), (false, true), (false, false)] {
        let settings = SnapSettings {
            x,
            y,
            ..settings(1.0)
        };
        let r = resolve(&settings, &lines, &SnapProbe::point(raw), &mut state);
        assert_eq!(r.offset, Vec2::new(f64::from(x), f64::from(y)));
        assert_eq!(r.marker.lines.len(), usize::from(x) + usize::from(y));
        for node in [
            Point::new(100.0, 100.0),
            Point::new(100.0, 99.0),
            Point::new(99.0, 100.0),
            Point::new(100.0, 999.0),
        ] {
            let targets = SnapTargets::new(vec![point(node.x, node.y, TargetKind::Node)]);
            let r = resolve(
                &settings,
                &targets,
                &SnapProbe::point(raw),
                &mut SnapState::default(),
            );
            let reachable =
                (x || node.x == raw.x) && (y || node.y == raw.y) && node.y < 200.0 && (x || y);
            assert_eq!(!r.is_empty(), reachable, "mask {x}/{y} node {node:?}");
            if reachable {
                assert_eq!(raw.translated(r.offset), node);
            }
            if !x {
                assert_eq!(r.offset.x, 0.0);
            }
            if !y {
                assert_eq!(r.offset.y, 0.0);
            }
        }
    }
}

#[test]
fn ac16_finite_curve_intersection_obeys_axis_and_constraint_scale() {
    use curvyo_ui_core::resolve_along;
    for swap in [false, true] {
        let raw = if swap {
            Point::new(100.0, 98.0)
        } else {
            Point::new(98.0, 100.0)
        };
        let config = SnapSettings {
            x: !swap,
            y: swap,
            ..settings(1.0)
        };
        for end in [95.0, 110.0] {
            let curve = SnapCurve {
                start: Point::new(90.0, 90.0),
                end: Point::new(end, end),
                handle_in: Vec2::ZERO,
                handle_out: Vec2::ZERO,
            };
            let mut t = point(90.0, 90.0, TargetKind::Guide);
            t.curve = Some(curve);
            t.extent = (curve.start, curve.end);
            let targets = SnapTargets::new(vec![t]);
            let r = resolve(
                &config,
                &targets,
                &SnapProbe::point(raw),
                &mut SnapState::default(),
            );
            if end < 100.0 {
                assert!(r.is_empty());
            } else {
                let p = raw.translated(r.offset);
                assert!((p.x - 100.0).abs() < 1e-9 && (p.y - 100.0).abs() < 1e-9);
            }
            // Architecture white-box finding after independent plan: direction scale
            // must never admit a nonzero correction on the disabled coordinate.
            for scale in [1e-8, 1.0, 1e8] {
                let direction = if swap {
                    Vec2::new(0.05 * scale, scale)
                } else {
                    Vec2::new(scale, 0.05 * scale)
                };
                let r = resolve_along(&config, &targets, raw, direction, &mut SnapState::default());
                assert!(r.is_empty(), "scale {scale}, axis {swap}");
            }
        }
    }
}
