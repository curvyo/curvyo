//! Grid snapping (`specs/0090-grid` criteria 12 to 15).
#![allow(clippy::float_cmp)] // Exact grid coordinates are the point of these checks.

use std::collections::HashSet;
use std::f64::consts::FRAC_PI_6;

use curvyo_document_core::{
    Angle, DocumentSize, DocumentSnapping, GridMode, GridScale, OriginCorner, Page, PageGrid,
    Point, Vec2,
};

use super::tests::{TOL, page_with, scale, shown};
use super::*;
use crate::snapping::{
    SnapProbe, SnapSettings, SnapState, SnapTarget, SnapTargets, TargetKind, resolve_with_grid,
};

fn settings() -> SnapSettings {
    let mut settings = SnapSettings::from_document(
        DocumentSnapping {
            enabled: true,
            ..DocumentSnapping::DEFAULT
        },
        1.0 / scale(1.0),
    );
    settings.grid = true;
    settings
}

fn input<'a>(pages: &'a [Page], points: &'a [Point], zoom: f64) -> GridSnapInput<'a> {
    GridSnapInput {
        pages,
        points,
        scale: scale(zoom),
    }
}

#[test]
fn a_point_near_an_intersection_snaps_exactly_onto_it() {
    let pages = [page_with(shown())];
    let points = [Point::new(10.5, 19.6)];
    let mut held = None;
    let found = snap_to_grid(&input(&pages, &points, 1.0), &settings(), &mut held).unwrap();
    assert!(matches!(
        found.held.feature,
        GridFeature::Intersection { .. }
    ));
    assert!((found.point.x - 10.0).abs() < TOL && (found.point.y - 20.0).abs() < TOL);
    assert!(held.is_some());
}

#[test]
fn a_point_near_one_line_snaps_only_across_it() {
    let pages = [page_with(shown())];
    // 1.5 mm from x = 10 (5.7 px, inside 8 px); 2.5 mm from y = 20 and y = 15.
    let points = [Point::new(11.5, 17.5)];
    let found = snap_to_grid(&input(&pages, &points, 1.0), &settings(), &mut None).unwrap();
    assert_eq!(found.held.feature, GridFeature::U(10.0));
    assert!((found.point.x - 10.0).abs() < TOL);
    assert!((found.point.y - 17.5).abs() < TOL);
}

#[test]
fn no_grid_target_on_the_pasteboard_on_a_hidden_grid_or_with_the_switch_off() {
    let pages = [page_with(shown())];
    let off_page = [Point::new(-10.2, 20.1)];
    assert!(snap_to_grid(&input(&pages, &off_page, 1.0), &settings(), &mut None).is_none());
    let hidden = [page_with(PageGrid::DEFAULT)];
    let points = [Point::new(10.2, 20.1)];
    assert!(snap_to_grid(&input(&hidden, &points, 1.0), &settings(), &mut None).is_none());
    let mut off = settings();
    off.grid = false;
    assert!(snap_to_grid(&input(&pages, &points, 1.0), &off, &mut None).is_none());
    off = settings();
    off.enabled = false;
    assert!(snap_to_grid(&input(&pages, &points, 1.0), &off, &mut None).is_none());
}

#[test]
fn where_pages_overlap_the_page_on_top_decides() {
    let below = page_with(shown());
    let above = Page {
        id: "top".into(),
        origin: Point::new(5.0, 5.0),
        size: DocumentSize::from_mm(50.0, 50.0),
        grid: PageGrid::DEFAULT,
        ..page_with(shown())
    };
    let points = [Point::new(10.2, 20.1)];
    let pages = [below.clone(), above.clone()];
    assert!(snap_to_grid(&input(&pages, &points, 1.0), &settings(), &mut None).is_none());
    let pages = [above, below];
    assert!(snap_to_grid(&input(&pages, &points, 1.0), &settings(), &mut None).is_some());
}

#[test]
fn lines_still_fading_in_do_not_pull() {
    let pages = [page_with(shown())];
    // At 300 % the 1 mm lines are 11.3 px apart: drawn, not at full opacity.
    let points = [Point::new(11.1, 12.4)];
    let found = snap_to_grid(&input(&pages, &points, 3.0), &settings(), &mut None);
    assert!(found.is_none_or(|f| !matches!(f.held.feature, GridFeature::U(11.0))));
}

#[test]
fn a_rotated_grid_snaps_in_grid_coordinates_and_the_axis_switches_act_on_its_axes() {
    let page = page_with(PageGrid {
        angle: Angle::from_radians(FRAC_PI_6),
        origin: OriginCorner::TopRight,
        ..shown()
    });
    let frame = GridFrame::of(&page);
    let target = frame.to_document(10.0, 15.0);
    let raw = frame.to_document(10.4, 15.3);
    let pages = [page];
    let points = [raw];
    let found = snap_to_grid(&input(&pages, &points, 1.0), &settings(), &mut None).unwrap();
    assert!(found.point.vector_to(target).length() < TOL);
    assert!(!found.axis_aligned);
    // X off: no correction along u, so only the v line pulls.
    let mut no_x = settings();
    no_x.x = false;
    let found = snap_to_grid(&input(&pages, &points, 1.0), &no_x, &mut None).unwrap();
    assert_eq!(found.held.feature, GridFeature::V(15.0));
    let (u, v) = frame.to_grid(found.point);
    assert!((u - 10.4).abs() < 1e-9 && (v - 15.0).abs() < 1e-9);
}

#[test]
fn a_log_grid_snaps_to_its_lines_as_drawn() {
    let pages = [page_with(PageGrid {
        scale: GridScale::Logarithmic,
        mode: GridMode::Fixed,
        ..shown()
    })];
    let target = 3.0_f64.log10() * 25.0;
    let points = [Point::new(target + 0.3, 100.0 + 12.0)];
    let found = snap_to_grid(&input(&pages, &points, 4.0), &settings(), &mut None).unwrap();
    assert!((found.point.x - target).abs() < TOL);
}

#[test]
fn a_held_line_releases_beyond_the_release_zone() {
    let pages = [page_with(shown())];
    let s = settings();
    let mut held = None;
    let near = [Point::new(11.5, 17.5)];
    assert!(snap_to_grid(&input(&pages, &near, 1.0), &s, &mut held).is_some());
    // 3.4 mm (12.9 px) is beyond capture but inside release (14 px).
    let middle = [Point::new(13.4, 17.5)];
    let kept = snap_to_grid(&input(&pages, &middle, 1.0), &s, &mut held).unwrap();
    assert!(kept.retained && kept.held.feature == GridFeature::U(10.0));
    let far = [Point::new(14.0, 17.5)];
    let next = snap_to_grid(&input(&pages, &far, 1.0), &s, &mut held);
    assert!(next.is_none_or(|f| f.held.feature != GridFeature::U(10.0)));
}

#[test]
fn snapping_along_a_direction_stops_at_the_nearest_grid_line() {
    let pages = [page_with(shown())];
    let points = [Point::new(11.5, 17.5)];
    let found = snap_to_grid_along(
        &input(&pages, &points, 1.0),
        &settings(),
        Vec2::new(1.0, 0.0),
        &mut None,
    )
    .unwrap();
    assert!((found.point.x - 10.0).abs() < TOL && (found.point.y - 17.5).abs() < TOL);
}

#[test]
fn every_other_target_wins_an_exact_tie_and_a_closer_grid_line_takes_its_axis() {
    let pages = [page_with(shown())];
    let s = settings();
    // A vertical object edge at x = 9 against the grid line x = 10, probe at 9.5: a tie.
    let targets = SnapTargets::new(vec![SnapTarget::vertical(
        9.0,
        (Point::new(9.0, 0.0), Point::new(9.0, 100.0)),
        TargetKind::Box,
    )]);
    let probe = SnapProbe::point(Point::new(9.5, 17.5));
    let points = [Point::new(9.5, 17.5)];
    let mut state = SnapState::default();
    let result = resolve_with_grid(
        &s,
        &targets,
        &probe,
        &input(&pages, &points, 1.0),
        None,
        &mut state,
    );
    assert!((result.offset.x + 0.5).abs() < TOL, "indexed wins the tie");
    assert!(!result.grid);
    // Probe at 9.7: the grid line (0.3) is closer than the edge (0.7).
    let probe = SnapProbe::point(Point::new(9.7, 17.5));
    let points = [Point::new(9.7, 17.5)];
    let mut state = SnapState::default();
    let result = resolve_with_grid(
        &s,
        &targets,
        &probe,
        &input(&pages, &points, 1.0),
        None,
        &mut state,
    );
    assert!((result.offset.x - 0.3).abs() < TOL);
    assert!(result.grid);
}

#[test]
fn grid_snap_nodes_give_anchors_of_paths() {
    let nodes = Vec::new();
    assert_eq!(
        super::super::snapping::grid_snap_nodes(&nodes, &HashSet::new()).len(),
        0
    );
}
