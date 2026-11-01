//! Grid snap targets for moving points (`specs/0090-grid` criteria 12 to 15, `adrs.md`
//! decision 5): computed from the page grids and the view, never gathered or indexed.
//!
//! The targets are the lines a layout draws at full opacity and their intersections, of the shown
//! grid of the page drawn on top under the moving point. Each probe point costs O(1).

use curvyo_document_core::{Page, Point, Vec2};

use super::{ANGLE_EPS, FULL_ALPHA_EPS, GridFrame, axis_lines, layout::clip};
use crate::snapping::{MarkerLine, SnapCurve, SnapSettings};
use crate::viewport::ScreenPx;

/// Snapped coordinates equal the grid coordinate within this many millimetres.
const EPS_MM: f64 = 1e-9;
/// Half the length of a grid marker line on screen, each side of the cross (UX notes §4).
const MARKER_HALF: ScreenPx = ScreenPx(24.0);

/// What grid snapping reads for one frame: the page list the drag already holds, the moving
/// points chosen by the tool (`snapping_grid_point`, or the dragged point), and the view scale.
#[derive(Debug, Clone, Copy)]
pub struct GridSnapInput<'a> {
    /// The pages in list order; a later page is drawn on top.
    pub pages: &'a [Page],
    /// The moving points that may snap to a grid.
    pub points: &'a [Point],
    /// Screen pixels per document millimetre.
    pub scale: f64,
}

/// Which grid feature a probe point snapped to, in grid coordinates (mm from the grid origin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridFeature {
    /// The crossing of the `u` line and the `v` line: both grid axes snap.
    Intersection {
        /// The `u` line.
        u: f64,
        /// The `v` line.
        v: f64,
    },
    /// A line of constant `u`: only the coordinate across it snaps.
    U(f64),
    /// A line of constant `v`: only the coordinate across it snaps.
    V(f64),
}

/// The held grid feature, carried from one frame to the next. Ephemeral, never stored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeldGrid {
    /// The index into [`GridSnapInput::points`].
    pub probe: usize,
    /// The index into [`GridSnapInput::pages`].
    pub page: usize,
    /// The feature.
    pub feature: GridFeature,
}

/// One resolved grid snap.
#[derive(Debug, Clone, PartialEq)]
pub struct GridCandidate {
    /// The held feature.
    pub held: HeldGrid,
    /// The correction to add to the raw position.
    pub offset: Vec2,
    /// The snapped probe point, where the marker cross sits.
    pub point: Point,
    /// Whether the feature was held from the previous frame.
    pub retained: bool,
    /// Whether the grid is axis aligned (angle 0), so a line competes per axis.
    pub axis_aligned: bool,
    /// The marker lines along the matched grid lines, clipped to the page.
    pub lines: Vec<MarkerLine>,
}

/// The index of the page drawn on top that contains `point`.
fn top_page(pages: &[Page], point: Point) -> Option<usize> {
    pages.iter().rposition(|page| page.contains(point))
}

/// The nearest full-opacity line of `page`'s grid to `c` within `radius`, mm.
fn nearest_full_line(page: &Page, scale: f64, c: f64, radius: f64) -> Option<f64> {
    axis_lines(&page.grid, scale, c - radius - EPS_MM, c + radius + EPS_MM)
        .into_iter()
        .filter(|line| line.alpha >= 1.0 - FULL_ALPHA_EPS)
        .map(|line| line.pos)
        .min_by(|a, b| (a - c).abs().total_cmp(&(b - c).abs()))
        .filter(|pos| (pos - c).abs() <= radius + EPS_MM)
}

/// Whether `pos` is still a full-opacity line of `page`'s grid at `scale`.
fn is_full_line(page: &Page, scale: f64, pos: f64) -> bool {
    let tolerance = EPS_MM * pos.abs().max(1.0);
    nearest_full_line(page, scale, pos, tolerance).is_some()
}

struct Probe<'a> {
    index: usize,
    page_index: usize,
    page: &'a Page,
    frame: GridFrame,
    point: Point,
    u: f64,
    v: f64,
}

fn probe_at<'a>(input: &GridSnapInput<'a>, index: usize, page: usize) -> Option<Probe<'a>> {
    let point = *input.points.get(index)?;
    let page_index = page;
    let page = input.pages.get(page)?;
    if !page.grid.shown || !page.contains(point) {
        return None;
    }
    let frame = GridFrame::of(page);
    let (u, v) = frame.to_grid(point);
    Some(Probe {
        index,
        page_index,
        page,
        frame,
        point,
        u,
        v,
    })
}

/// The correction that moves `probe` onto `feature`.
fn correction(probe: &Probe<'_>, feature: GridFeature) -> Vec2 {
    match feature {
        GridFeature::Intersection { u, v } => probe.point.vector_to(probe.frame.to_document(u, v)),
        GridFeature::U(u) => probe.frame.u_axis().scaled(u - probe.u),
        GridFeature::V(v) => probe.frame.v_axis().scaled(v - probe.v),
    }
}

fn candidate(
    input: &GridSnapInput<'_>,
    probe: &Probe<'_>,
    held: HeldGrid,
    offset: Vec2,
    retained: bool,
) -> Option<GridCandidate> {
    let point = probe.point.translated(offset);
    if !probe.page.contains(point) {
        return None;
    }
    Some(GridCandidate {
        held,
        offset,
        point,
        retained,
        axis_aligned: probe.page.grid.angle.as_radians() <= ANGLE_EPS,
        lines: marker_lines(probe, held.feature, point, input.scale),
    })
}

/// Resolves grid snapping for the probe points (criteria 12 to 14): a feature held from the
/// previous frame stays while its probe is within the release zone and the line is still drawn
/// at full opacity; otherwise, per probe point on a shown grid, an intersection within the capture
/// zone beats a line, then the smaller correction, then the probe order. The `0060` X and Y
/// switches act on the grid's own axes. `held` is updated in place.
pub fn snap_to_grid(
    input: &GridSnapInput<'_>,
    settings: &SnapSettings,
    held: &mut Option<HeldGrid>,
) -> Option<GridCandidate> {
    if !settings.enabled || !settings.grid || (!settings.x && !settings.y) {
        *held = None;
        return None;
    }
    if let Some(previous) = *held
        && let Some(found) = retain(input, settings, previous)
    {
        return Some(found);
    }
    let mut best: Option<((u8, f64, usize), GridCandidate)> = None;
    for index in 0..input.points.len() {
        let Some(page) = top_page(input.pages, input.points[index]) else {
            continue;
        };
        let Some(probe) = probe_at(input, index, page) else {
            continue;
        };
        let u = settings
            .x
            .then(|| nearest_full_line(probe.page, input.scale, probe.u, settings.capture))
            .flatten();
        let v = settings
            .y
            .then(|| nearest_full_line(probe.page, input.scale, probe.v, settings.capture))
            .flatten();
        let mut features = Vec::with_capacity(3);
        if let (Some(u), Some(v)) = (u, v)
            && (u - probe.u).hypot(v - probe.v) <= settings.capture + EPS_MM
        {
            features.push((0, GridFeature::Intersection { u, v }));
        }
        features.extend(u.map(|u| (1, GridFeature::U(u))));
        features.extend(v.map(|v| (1, GridFeature::V(v))));
        for (rank, feature) in features {
            let offset = correction(&probe, feature);
            let held = HeldGrid {
                probe: index,
                page,
                feature,
            };
            let Some(found) = candidate(input, &probe, held, offset, false) else {
                continue;
            };
            let key = (rank, offset.length(), index);
            if best.as_ref().is_none_or(|(previous, _)| {
                (key.0, key.1, key.2)
                    .partial_cmp(previous)
                    .is_some_and(std::cmp::Ordering::is_lt)
            }) {
                best = Some((key, found));
            }
        }
    }
    *held = best.as_ref().map(|(_, found)| found.held);
    best.map(|(_, found)| found)
}

fn retain(
    input: &GridSnapInput<'_>,
    settings: &SnapSettings,
    previous: HeldGrid,
) -> Option<GridCandidate> {
    let probe = probe_at(input, previous.probe, previous.page)?;
    if top_page(input.pages, probe.point) != Some(previous.page) {
        return None;
    }
    let scale = input.scale;
    let (still_drawn, allowed) = match previous.feature {
        GridFeature::Intersection { u, v } => (
            is_full_line(probe.page, scale, u) && is_full_line(probe.page, scale, v),
            settings.x && settings.y,
        ),
        GridFeature::U(u) => (is_full_line(probe.page, scale, u), settings.x),
        GridFeature::V(v) => (is_full_line(probe.page, scale, v), settings.y),
    };
    let offset = correction(&probe, previous.feature);
    if !still_drawn || !allowed || offset.length() > settings.release + EPS_MM {
        return None;
    }
    candidate(input, &probe, previous, offset, true)
}

/// Resolves grid snapping for movement along `direction` (a constrained move, an angle-locked
/// Pen segment): the nearest crossing of a full-opacity grid line along the direction, within
/// the capture zone (the release zone for the held line). `held` is updated in place.
pub(crate) fn snap_to_grid_along(
    input: &GridSnapInput<'_>,
    settings: &SnapSettings,
    direction: Vec2,
    held: &mut Option<HeldGrid>,
) -> Option<GridCandidate> {
    let length = direction.length();
    let blocked = (!settings.x && direction.x.abs() > EPS_MM * length)
        || (!settings.y && direction.y.abs() > EPS_MM * length);
    if !settings.enabled || !settings.grid || length <= EPS_MM || blocked {
        *held = None;
        return None;
    }
    let unit = direction.scaled(1.0 / length);
    let mut best: Option<((bool, f64, usize), GridCandidate)> = None;
    for index in 0..input.points.len() {
        let Some(page) = top_page(input.pages, input.points[index]) else {
            continue;
        };
        let Some(probe) = probe_at(input, index, page) else {
            continue;
        };
        let was = held.filter(|h| h.probe == index && h.page == page);
        for axis in [0, 1] {
            let Some(found) = along_axis(input, settings, &probe, unit, axis, was) else {
                continue;
            };
            let key = (!found.retained, found.offset.length(), index);
            if best.as_ref().is_none_or(|(previous, _)| {
                key.partial_cmp(previous)
                    .is_some_and(std::cmp::Ordering::is_lt)
            }) {
                best = Some((key, found));
            }
        }
    }
    *held = best.as_ref().map(|(_, found)| found.held);
    best.map(|(_, found)| found)
}

/// The nearest full-opacity line of grid axis `axis` (0 = u, 1 = v) crossed along `unit`, or the
/// held one while it is within the release zone.
fn along_axis(
    input: &GridSnapInput<'_>,
    settings: &SnapSettings,
    probe: &Probe<'_>,
    unit: Vec2,
    axis: u8,
    was: Option<HeldGrid>,
) -> Option<GridCandidate> {
    let (along, at) = if axis == 0 {
        (dot(unit, probe.frame.u_axis()), probe.u)
    } else {
        (dot(unit, probe.frame.v_axis()), probe.v)
    };
    if along.abs() <= EPS_MM {
        return None;
    }
    let held_pos = was.and_then(|h| match (axis, h.feature) {
        (0, GridFeature::U(pos)) | (1, GridFeature::V(pos)) => Some(pos),
        _ => None,
    });
    let retained = held_pos.is_some_and(|pos| {
        is_full_line(probe.page, input.scale, pos)
            && ((pos - at) / along).abs() <= settings.release + EPS_MM
    });
    let pos = if retained {
        held_pos
    } else {
        nearest_full_line(probe.page, input.scale, at, settings.capture * along.abs())
    }?;
    let feature = if axis == 0 {
        GridFeature::U(pos)
    } else {
        GridFeature::V(pos)
    };
    let held = HeldGrid {
        probe: probe.index,
        page: probe.page_index,
        feature,
    };
    candidate(
        input,
        probe,
        held,
        unit.scaled((pos - at) / along),
        retained,
    )
}

const fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

/// The marker lines along the matched grid lines: 24 screen px each side of the cross, clipped
/// to the page.
fn marker_lines(
    probe: &Probe<'_>,
    feature: GridFeature,
    cross: Point,
    scale: f64,
) -> Vec<MarkerLine> {
    let half = MARKER_HALF.to_length(scale).as_mm();
    let page = probe.page;
    let rect = (
        page.origin,
        Point::new(
            page.origin.x + page.size.width.as_mm(),
            page.origin.y + page.size.height.as_mm(),
        ),
    );
    // Each direction runs along a matched line; `true` marks the `v` axis, which is the
    // document's vertical when the grid is axis aligned.
    let mut directions = Vec::with_capacity(2);
    match feature {
        GridFeature::Intersection { .. } => {
            directions.push((probe.frame.v_axis(), true));
            directions.push((probe.frame.u_axis(), false));
        }
        GridFeature::U(_) => directions.push((probe.frame.v_axis(), true)),
        GridFeature::V(_) => directions.push((probe.frame.u_axis(), false)),
    }
    let axis_aligned = page.grid.angle.as_radians() <= ANGLE_EPS;
    directions
        .into_iter()
        .filter_map(|(along, vertical)| {
            let (a, b) = clip(
                cross.translated(along.scaled(-half)),
                cross.translated(along.scaled(half)),
                rect,
            )?;
            Some(if axis_aligned && vertical {
                MarkerLine::Vertical {
                    x: cross.x,
                    y0: a.y,
                    y1: b.y,
                }
            } else if axis_aligned {
                MarkerLine::Horizontal {
                    y: cross.y,
                    x0: a.x,
                    x1: b.x,
                }
            } else {
                MarkerLine::Curve(SnapCurve {
                    start: a,
                    handle_out: Vec2::ZERO,
                    handle_in: Vec2::ZERO,
                    end: b,
                })
            })
        })
        .collect()
}
