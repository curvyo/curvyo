//! The combination of grid snapping with the indexed resolver (`specs/0090-grid` criteria 12 and
//! 13, `adrs.md` decision 5).
//!
//! A held target keeps winning while it holds. Otherwise the smaller correction wins and an exact
//! tie goes to the indexed target. With an axis-aligned grid a grid line competes per axis with
//! the indexed result; an intersection, a rotated grid or a constrained direction competes as one
//! atomic point against the whole indexed correction.

use super::{
    MarkerLine, Point, SnapMarker, SnapProbe, SnapResult, SnapSettings, SnapState, SnapTargets,
    Vec2,
};
use crate::grid::{GridCandidate, GridFeature, GridSnapInput, snap_to_grid, snap_to_grid_along};

/// Resolves one frame against the indexed targets and the page grids: `resolve` (or
/// `resolve_probe_along` with a `direction`) for `probe`, grid snapping for `grid.points`, then
/// the combination rule above. `state` is updated in place.
pub fn resolve_with_grid(
    settings: &SnapSettings,
    targets: &SnapTargets,
    probe: &SnapProbe,
    grid: &GridSnapInput<'_>,
    direction: Option<Vec2>,
    state: &mut SnapState,
) -> SnapResult {
    let mut held_grid = state.grid.take();
    let previous = (state.x, state.y);
    let indexed = match direction {
        Some(direction) => super::resolve_probe_along(settings, targets, probe, direction, state),
        None => super::resolve(settings, targets, probe, state),
    };
    let candidate = match direction {
        Some(direction) => snap_to_grid_along(grid, settings, direction, &mut held_grid),
        None => snap_to_grid(grid, settings, &mut held_grid),
    };
    let Some(found) = candidate else {
        return indexed;
    };
    let same = |before: Option<super::HeldSnap>, now: Option<super::HeldSnap>| {
        before.is_some() && before.map(|h| h.target) == now.map(|h| h.target)
    };
    let retained_indexed = same(previous.0, indexed.held_x) || same(previous.1, indexed.held_y);
    let atomic_indexed = indexed
        .held_x
        .zip(indexed.held_y)
        .is_some_and(|(x, y)| x.target == y.target);
    let line_axis = match found.held.feature {
        GridFeature::U(_) if found.axis_aligned && direction.is_none() => Some(Axis::X),
        GridFeature::V(_) if found.axis_aligned && direction.is_none() => Some(Axis::Y),
        _ => None,
    };
    if let Some(axis) = line_axis
        && !atomic_indexed
    {
        let indexed_on_axis = match axis {
            Axis::X => indexed.held_x.is_some(),
            Axis::Y => indexed.held_y.is_some(),
        };
        let grid_takes_axis = !indexed_on_axis
            || found.retained
            || (!retained_indexed && axis.of(found.offset).abs() < axis.of(indexed.offset).abs());
        if !grid_takes_axis {
            return indexed;
        }
        state.grid = held_grid;
        return merge_axis(&indexed, &found, axis, state);
    }
    let grid_wins = found.retained
        || (!retained_indexed
            && (indexed.is_empty() || found.offset.length() < indexed.offset.length()));
    if !grid_wins {
        return indexed;
    }
    state.grid = held_grid;
    grid_only(found, state)
}

/// The grid candidate as the whole result: the indexed hold is dropped, so the indexed targets
/// capture afresh once the grid lets go.
fn grid_only(found: GridCandidate, state: &mut SnapState) -> SnapResult {
    state.x = None;
    state.y = None;
    state.curve = None;
    SnapResult {
        offset: found.offset,
        held_x: None,
        held_y: None,
        marker: SnapMarker {
            lines: found.lines,
            cross: Some(found.point),
        },
        grid: true,
    }
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

impl Axis {
    const fn of(self, v: Vec2) -> f64 {
        match self {
            Self::X => v.x,
            Self::Y => v.y,
        }
    }
}

/// The indexed result with `axis` taken by the grid line of `found`.
fn merge_axis(
    indexed: &SnapResult,
    found: &GridCandidate,
    axis: Axis,
    state: &mut SnapState,
) -> SnapResult {
    let mut lines: Vec<MarkerLine> = indexed
        .marker
        .lines
        .iter()
        .copied()
        .filter(|line| match axis {
            Axis::X => !matches!(line, MarkerLine::Vertical { .. }),
            Axis::Y => !matches!(line, MarkerLine::Horizontal { .. }),
        })
        .collect();
    lines.extend(found.lines.iter().copied());
    let (offset, held_x, held_y) = match axis {
        Axis::X => (
            Vec2::new(found.offset.x, indexed.offset.y),
            None,
            indexed.held_y,
        ),
        Axis::Y => (
            Vec2::new(indexed.offset.x, found.offset.y),
            indexed.held_x,
            None,
        ),
    };
    state.x = held_x;
    state.y = held_y;
    state.curve = None;
    let cross = match (axis, indexed.marker.cross) {
        (Axis::X, Some(cross)) if held_y.is_some() => Point::new(found.point.x, cross.y),
        (Axis::Y, Some(cross)) if held_x.is_some() => Point::new(cross.x, found.point.y),
        _ => found.point,
    };
    SnapResult {
        offset,
        held_x,
        held_y,
        marker: SnapMarker {
            lines,
            cross: Some(cross),
        },
        grid: true,
    }
}
