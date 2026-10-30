//! Raw-geometry hysteresis resolution for indexed snap targets.
use super::{
    AxisProbe, HeldSnap, MarkerLine, Point, SnapMarker, SnapProbe, SnapResult, SnapSettings,
    SnapState, SnapTargets, TargetAxis, Vec2,
};
const EPS_MM: f64 = 1e-9;
/// Resolves one frame (`adrs.md` decision 4): with a held target per axis that is still within
/// the release zone of the same probe, keep it; otherwise the nearest target within the capture
/// zone wins, ties broken by kind and then document order.
///
/// `state` is updated in place with the result's held targets.
pub fn resolve(
    settings: &SnapSettings,
    targets: &SnapTargets,
    probe: &SnapProbe,
    state: &mut SnapState,
) -> SnapResult {
    if !settings.enabled || !settings.any_target() || probe.is_empty() || targets.is_empty() {
        state.clear();
        return SnapResult::default();
    }
    if !settings.x || !settings.y {
        let direction = Vec2::new(f64::from(settings.x), f64::from(settings.y));
        return super::constrained::resolve_probe_along(settings, targets, probe, direction, state);
    }
    let point = resolve_point(settings, targets, probe, state, false);
    let guide = resolve_point(settings, targets, probe, state, true);
    let held_line = [state.x, state.y].into_iter().flatten().any(|h| {
        let t = &targets.entries[h.target];
        let values = if t.axis == TargetAxis::Vertical {
            &probe.x
        } else {
            &probe.y
        };
        t.axis != TargetAxis::Both
            && t.kind.is_enabled(settings)
            && values
                .get(h.probe)
                .is_some_and(|p| (p.value - h.coordinate).abs() <= settings.release + EPS_MM)
    });
    let mut held_x = resolve_axis(
        settings,
        targets,
        &probe.x,
        TargetAxis::Vertical,
        &mut state.x,
    );
    let mut held_y = resolve_axis(
        settings,
        targets,
        &probe.y,
        TargetAxis::Horizontal,
        &mut state.y,
    );
    state.curve = None;
    if let Some((x, y, held)) = guide {
        let composed =
            super::compose::compose(settings, targets, probe, (x, y, held), held_x, held_y);
        if let Some((cx, cy, curve)) = composed {
            held_x = Some(cx);
            held_y = Some(cy);
            state.curve = Some(curve);
        } else if point_wins(targets, probe, (x, y, held), held_x, held_y, held_line) {
            held_x = Some(x);
            held_y = Some(y);
            state.curve = Some((x, y));
        }
    }
    if let Some(candidate @ (x, y, _)) = point
        && point_wins(
            targets,
            probe,
            candidate,
            held_x,
            held_y,
            held_line || guide.is_some_and(|(_, _, held)| held),
        )
    {
        held_x = Some(x);
        held_y = Some(y);
        state.curve = None;
    }
    state.x = held_x;
    state.y = held_y;
    if held_x.is_none() && held_y.is_none() {
        return SnapResult::default();
    }
    let offset = Vec2::new(
        held_x.map_or(0.0, |held| held.coordinate - probe.x[held.probe].value),
        held_y.map_or(0.0, |held| held.coordinate - probe.y[held.probe].value),
    );
    let marker = marker_of(targets, probe, held_x, held_y);
    SnapResult {
        offset,
        held_x,
        held_y,
        marker,
    }
}

fn point_wins(
    targets: &SnapTargets,
    probe: &SnapProbe,
    point: (HeldSnap, HeldSnap, bool),
    x: Option<HeldSnap>,
    y: Option<HeldSnap>,
    held_line: bool,
) -> bool {
    let (px, py, held) = point;
    let distance =
        (px.coordinate - probe.x[px.probe].value).hypot(py.coordinate - probe.y[py.probe].value);
    let other = x
        .map_or(0.0, |h| (h.coordinate - probe.x[h.probe].value).abs())
        .hypot(y.map_or(0.0, |h| (h.coordinate - probe.y[h.probe].value).abs()));
    let kind = [x, y]
        .into_iter()
        .flatten()
        .map(|h| targets.entries[h.target].kind)
        .min();
    held || (!held_line
        && (kind.is_none()
            || distance < other
            || (distance.total_cmp(&other).is_eq()
                && Some(targets.entries[px.target].kind) <= kind)))
}

/// Resolves a held axis line or queries the coordinate index near each feature.
fn resolve_axis(
    settings: &SnapSettings,
    targets: &SnapTargets,
    probes: &[AxisProbe],
    axis: TargetAxis,
    held: &mut Option<HeldSnap>,
) -> Option<HeldSnap> {
    if let Some(current) = *held
        && targets
            .entries
            .get(current.target)
            .is_some_and(|target| target.axis == axis && target.kind.is_enabled(settings))
        && probes.get(current.probe).is_some_and(|probe| {
            (probe.value - current.coordinate).abs() <= settings.release + EPS_MM
        })
    {
        return Some(current);
    }
    let mut best = None;
    for (probe, feature) in probes.iter().enumerate() {
        for &(coordinate, target) in
            targets
                .index
                .lines(axis, feature.value, settings.capture + EPS_MM)
        {
            let kind = targets.entries[target].kind;
            if !kind.is_enabled(settings) {
                continue;
            }
            let candidate = ((feature.value - coordinate).abs(), kind, target, probe);
            if best.is_none_or(|previous| candidate < previous) {
                best = Some(candidate);
            }
        }
    }
    *held = best.map(|(_, _, target, probe)| HeldSnap {
        target,
        probe,
        coordinate: if axis == TargetAxis::Vertical {
            targets.entries[target].point.x
        } else {
            targets.entries[target].point.y
        },
    });
    *held
}

/// A point target holds both coordinates as one Euclidean capture.
fn resolve_point(
    settings: &SnapSettings,
    targets: &SnapTargets,
    probe: &SnapProbe,
    state: &SnapState,
    curves: bool,
) -> Option<(HeldSnap, HeldSnap, bool)> {
    let previous = if curves {
        state
            .curve
            .map_or((state.x, state.y), |(x, y)| (Some(x), Some(y)))
    } else {
        (state.x, state.y)
    };
    if let (Some(x), Some(y)) = previous
        && x.target == y.target
        && targets.entries.get(x.target).is_some_and(|t| {
            t.axis == TargetAxis::Both && t.curve.is_some() == curves && t.kind.is_enabled(settings)
        })
        && let (Some(px), Some(py)) = (probe.x.get(x.probe), probe.y.get(y.probe))
    {
        let raw = Point::new(px.value, py.value);
        let target = &targets.entries[x.target];
        let point = target
            .curve
            .map_or(target.point, |curve| curve.project(raw));
        let distance = (raw.x - point.x).hypot(raw.y - point.y);
        if distance <= settings.release + EPS_MM {
            return Some((
                HeldSnap {
                    coordinate: point.x,
                    ..x
                },
                HeldSnap {
                    coordinate: point.y,
                    ..y
                },
                true,
            ));
        }
    }
    let mut best = None;
    for &(xi, yi) in &probe.points {
        if let (Some(x), Some(y)) = (probe.x.get(xi), probe.y.get(yi)) {
            let raw = Point::new(x.value, y.value);
            for target in targets.index.points(raw, settings.capture + EPS_MM) {
                let t = &targets.entries[target];
                if t.curve.is_some() != curves || !t.kind.is_enabled(settings) {
                    continue;
                }
                let point = t.curve.map_or(t.point, |curve| curve.project(raw));
                let distance = (raw.x - point.x).hypot(raw.y - point.y);
                if distance > settings.capture + EPS_MM {
                    continue;
                }
                let candidate = (distance, t.kind, target, xi, yi);
                if best.is_none_or(|previous| candidate < previous) {
                    best = Some(candidate);
                }
            }
        }
    }
    best.map(|(_, _, target, xi, yi)| {
        let t = &targets.entries[target];
        let raw = Point::new(probe.x[xi].value, probe.y[yi].value);
        let p = t.curve.map_or(t.point, |curve| curve.project(raw));
        (
            HeldSnap {
                target,
                probe: xi,
                coordinate: p.x,
            },
            HeldSnap {
                target,
                probe: yi,
                coordinate: p.y,
            },
            false,
        )
    })
}

/// The marker for the held targets: one line per axis along the target's extent and a cross at
/// the matched moving point. A point target that wins both axes draws both lines through itself.
fn marker_of(
    targets: &SnapTargets,
    probe: &SnapProbe,
    held_x: Option<HeldSnap>,
    held_y: Option<HeldSnap>,
) -> SnapMarker {
    if let (Some(x), Some(y)) = (held_x, held_y)
        && x.target == y.target
        && let Some(curve) = targets.entries[x.target].curve
    {
        return SnapMarker {
            lines: vec![MarkerLine::Curve(curve)],
            cross: Some(Point::new(x.coordinate, y.coordinate)),
        };
    }
    let mut lines = Vec::with_capacity(2);
    if let Some(held) = held_x
        && let Some(target) = targets.entries.get(held.target)
    {
        lines.push(target.curve.map_or(
            MarkerLine::Vertical {
                x: held.coordinate,
                y0: target.extent.0.y,
                y1: target.extent.1.y,
            },
            MarkerLine::Curve,
        ));
    }
    if let Some(held) = held_y
        && let Some(target) = targets.entries.get(held.target)
    {
        lines.push(target.curve.map_or(
            MarkerLine::Horizontal {
                y: held.coordinate,
                x0: target.extent.0.x,
                x1: target.extent.1.x,
            },
            MarkerLine::Curve,
        ));
    }
    let cross = match (held_x, held_y) {
        (Some(x), Some(y)) => Some(Point::new(x.coordinate, y.coordinate)),
        (Some(x), None) => probe
            .x
            .get(x.probe)
            .map(|probe| Point::new(x.coordinate, probe.cross_other)),
        (None, Some(y)) => probe
            .y
            .get(y.probe)
            .map(|probe| Point::new(probe.cross_other, y.coordinate)),
        (None, None) => None,
    };
    SnapMarker { lines, cross }
}
