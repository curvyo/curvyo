//! The snap targets of the Active fixtures (`specs/0102-laser-fixtures` criteria 24 and 27): grid
//! lines, slot and board points, stop points; clamps and unknown kinds give none. Built once at
//! a drag's start, and only while the document's Fixtures switch is on.

use curvyo_document_core::{Page, Point};
use curvyo_laser_core::{Placement, board_points, layout, stop_points};
use curvyo_ui_core::{SnapTarget, TargetKind};

use super::super::Session;
use super::placed::{PageFixtures, PlacedFixture, Shape};

/// The point targets of one fixture, in document space; the eye plays no part (criterion 6).
fn fixture_points(fixture: &PlacedFixture) -> Vec<Point> {
    let m = fixture.global.to_array();
    let Some(placement) = Placement::new(m) else {
        return Vec::new();
    };
    match &fixture.shape {
        Shape::Board { width, height } => {
            let mut points = board_points(*width, *height, placement);
            for slot in &fixture.slots {
                points.extend(
                    slot.slot
                        .local_points()
                        .into_iter()
                        .map(|p| placement.apply(p)),
                );
            }
            points
        }
        Shape::Stop { leg_a, leg_b, .. } => stop_points(*leg_a, *leg_b, placement),
        // A screw is no snap target, not even its centre (criterion 40).
        Shape::Grid(_) | Shape::Clamp { .. } | Shape::Screw { .. } | Shape::Unknown => Vec::new(),
    }
}

/// The grid's lines as line targets: snapping a box point onto both a vertical and a horizontal
/// line is the nearest grid intersection (criterion 27 a).
fn grid_lines(fixture: &PlacedFixture, page: &Page, out: &mut Vec<SnapTarget>) {
    let Shape::Grid(spec) = &fixture.shape else {
        return;
    };
    let grid = layout(*spec, page.size);
    let (x0, y0) = (page.origin.x, page.origin.y);
    let (x1, y1) = (x0 + page.size.width.as_mm(), y0 + page.size.height.as_mm());
    for line in grid.minor.iter().chain(&grid.major) {
        if (line.start.x - line.end.x).abs() < 1e-9 {
            let x = x0 + line.start.x;
            out.push(SnapTarget::vertical(
                x,
                (Point::new(x, y0), Point::new(x, y1)),
                TargetKind::Fixture,
            ));
        } else {
            let y = y0 + line.start.y;
            out.push(SnapTarget::horizontal(
                y,
                (Point::new(x0, y), Point::new(x1, y)),
                TargetKind::Fixture,
            ));
        }
    }
}

/// The targets of every Active fixture of `fixtures`.
pub(super) fn targets_of(fixtures: &PageFixtures) -> Vec<SnapTarget> {
    let mut out = Vec::new();
    for fixture in fixtures.fixtures.iter().filter(|f| f.active) {
        grid_lines(fixture, &fixtures.page, &mut out);
        out.extend(
            fixture_points(fixture)
                .into_iter()
                .map(|p| SnapTarget::point(p, (p, p), TargetKind::Fixture)),
        );
    }
    out
}

impl Session {
    /// The fixture targets for a drag starting now: those of the active laser page, or with
    /// `every_page` those of every laser page (a page row dragged over any of them, criterion
    /// 26). Empty while the Fixtures switch is off.
    pub(in crate::session) fn fixture_snap_targets(&self, every_page: bool) -> Vec<SnapTarget> {
        if !self.document.snapping().fixtures {
            return Vec::new();
        }
        if !every_page {
            return self
                .active_fixtures()
                .map(|f| targets_of(&f))
                .unwrap_or_default();
        }
        self.available_page_fixtures()
            .iter()
            .flat_map(targets_of)
            .collect()
    }

    /// The fixtures of every laser page whose extension is on.
    pub(super) fn available_page_fixtures(&self) -> Vec<PageFixtures> {
        self.document
            .manufacturings()
            .iter()
            .filter(|m| self.manufacturing_available(m.id))
            .filter_map(PageFixtures::read)
            .collect()
    }
}
