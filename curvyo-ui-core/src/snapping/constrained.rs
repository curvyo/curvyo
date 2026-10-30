//! Snapping along one permitted movement direction.
use super::{
    HeldSnap, MarkerLine, Point, SnapCurve, SnapMarker, SnapProbe, SnapResult, SnapSettings,
    SnapState, SnapTarget, SnapTargets, TargetAxis, TargetKind, Vec2,
};
const EPS: f64 = 1e-9;

/// Resolves a moving point along `direction`, retaining its geometric constraint.
/// The direction need not be normalized; a zero direction cannot move.
pub fn resolve_along(
    settings: &SnapSettings,
    targets: &SnapTargets,
    point: Point,
    direction: Vec2,
    state: &mut SnapState,
) -> SnapResult {
    let Some(unit) = permitted_direction(settings, direction) else {
        state.clear();
        return SnapResult::default();
    };
    state.curve = None;
    let held = state.x.or(state.y).and_then(|h| {
        targets.entries.get(h.target).and_then(|t| {
            if !t.kind.is_enabled(settings) {
                return None;
            }
            correction(t, point, unit)
                .filter(|d| d.length() <= settings.release + EPS)
                .map(|d| (h.target, d))
        })
    });
    let choice = held.or_else(|| {
        let mut ids = targets.index.points(point, settings.capture + EPS);
        for (axis, value) in [
            (TargetAxis::Vertical, point.x),
            (TargetAxis::Horizontal, point.y),
        ] {
            ids.extend(
                targets
                    .index
                    .lines(axis, value, settings.capture + EPS)
                    .iter()
                    .map(|&(_, id)| id),
            );
        }
        let mut best: Option<(f64, TargetKind, usize, Vec2)> = None;
        for id in ids {
            let target = &targets.entries[id];
            if !target.kind.is_enabled(settings) {
                continue;
            }
            let Some(offset) = correction(target, point, unit) else {
                continue;
            };
            let distance = offset.length();
            if distance > settings.capture + EPS {
                continue;
            }
            let key = (distance, target.kind, id);
            if best.is_none_or(|(distance, kind, id, _)| key < (distance, kind, id)) {
                best = Some((distance, target.kind, id, offset));
            }
        }
        best.map(|(_, _, id, offset)| (id, offset))
    });
    let Some((target, offset)) = choice else {
        state.clear();
        return SnapResult::default();
    };
    let end = point.translated(offset);
    let x = HeldSnap {
        target,
        probe: 0,
        coordinate: end.x,
    };
    let y = HeldSnap {
        target,
        probe: 0,
        coordinate: end.y,
    };
    state.x = Some(x);
    state.y = Some(y);
    let t = &targets.entries[target];
    let lines = if let Some(curve) = t.curve {
        vec![MarkerLine::Curve(curve)]
    } else {
        let mut lines = Vec::new();
        if t.axis != TargetAxis::Horizontal {
            lines.push(MarkerLine::Vertical {
                x: end.x,
                y0: t.extent.0.y,
                y1: t.extent.1.y,
            });
        }
        if t.axis != TargetAxis::Vertical {
            lines.push(MarkerLine::Horizontal {
                y: end.y,
                x0: t.extent.0.x,
                x1: t.extent.1.x,
            });
        }
        lines
    };
    SnapResult {
        offset,
        held_x: Some(x),
        held_y: Some(y),
        marker: SnapMarker {
            lines,
            cross: Some(end),
        },
    }
}
fn permitted_direction(settings: &SnapSettings, direction: Vec2) -> Option<Vec2> {
    if !settings.enabled || direction.length() <= EPS {
        return None;
    }
    let mut unit = direction.scaled(1.0 / direction.length());
    if (!settings.x && unit.x.abs() > EPS) || (!settings.y && unit.y.abs() > EPS) {
        return None;
    }
    if !settings.x {
        unit.x = 0.0;
    }
    if !settings.y {
        unit.y = 0.0;
    }
    Some(unit.scaled(1.0 / unit.length()))
}

fn correction(target: &SnapTarget, p: Point, d: Vec2) -> Option<Vec2> {
    if let Some(curve) = target.curve {
        return curve_intersections(curve, p, d)
            .into_iter()
            .min_by(|a, b| a.length().total_cmp(&b.length()));
    }
    match target.axis {
        TargetAxis::Vertical if d.x.abs() > EPS => Some(d.scaled((target.point.x - p.x) / d.x)),
        TargetAxis::Horizontal if d.y.abs() > EPS => Some(d.scaled((target.point.y - p.y) / d.y)),
        TargetAxis::Both => {
            let delta = p.vector_to(target.point);
            ((delta.x * d.y - delta.y * d.x).abs() <= EPS)
                .then_some(d.scaled(delta.x * d.x + delta.y * d.y))
        }
        _ => None,
    }
}
/// Cubic/line intersections, partitioned at derivative roots so tangencies survive.
#[allow(clippy::many_single_char_names)] // Cubic polynomial coefficients and parameter t.
pub(super) fn curve_intersections(curve: SnapCurve, p: Point, d: Vec2) -> Vec<Vec2> {
    let controls = [
        curve.start,
        curve.start.translated(curve.handle_out),
        curve.end.translated(curve.handle_in),
        curve.end,
    ];
    let v = controls.map(|q| (q.x - p.x) * d.y - (q.y - p.y) * d.x);
    if v.iter().all(|x| x.abs() <= EPS) {
        let q = curve.project(p);
        return vec![d.scaled((q.x - p.x) * d.x + (q.y - p.y) * d.y)];
    }
    let a = -v[0] + 3.0 * v[1] - 3.0 * v[2] + v[3];
    let b = 3.0 * v[0] - 6.0 * v[1] + 3.0 * v[2];
    let c = -3.0 * v[0] + 3.0 * v[1];
    let eval = |t: f64| ((a * t + b) * t + c) * t + v[0];
    let mut cuts = vec![0.0, 1.0];
    if a.abs() <= EPS {
        if b.abs() > EPS {
            cuts.push(-c / (2.0 * b));
        }
    } else {
        let discriminant = 4.0 * b * b - 12.0 * a * c;
        if discriminant >= 0.0 {
            let root = discriminant.sqrt();
            cuts.extend([(-2.0 * b - root) / (6.0 * a), (-2.0 * b + root) / (6.0 * a)]);
        }
    }
    cuts.retain(|&t| (0.0..=1.0).contains(&t));
    cuts.sort_by(f64::total_cmp);
    let mut roots: Vec<_> = cuts
        .iter()
        .copied()
        .filter(|&t| eval(t).abs() <= EPS)
        .collect();
    for pair in cuts.windows(2) {
        let (mut lo, mut hi) = (pair[0], pair[1]);
        if eval(lo) * eval(hi) >= 0.0 {
            continue;
        }
        for _ in 0..52 {
            let mid = f64::midpoint(lo, hi);
            if eval(lo) * eval(mid) > 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        roots.push(f64::midpoint(lo, hi));
    }
    roots
        .into_iter()
        .map(|t| {
            // De Casteljau preserves constant coordinates exactly (e.g. y=100).
            let lerp = |a: Point, b: Point| a.translated(a.vector_to(b).scaled(t));
            let ab = lerp(controls[0], controls[1]);
            let bc = lerp(controls[1], controls[2]);
            let cd = lerp(controls[2], controls[3]);
            let q = lerp(lerp(ab, bc), lerp(bc, cd));
            d.scaled((q.x - p.x) * d.x + (q.y - p.y) * d.y)
        })
        .collect()
}

/// Resolves the eligible features of a movement unit along a common direction.
pub fn resolve_probe_along(
    settings: &SnapSettings,
    targets: &SnapTargets,
    probe: &SnapProbe,
    direction: Vec2,
    state: &mut SnapState,
) -> SnapResult {
    let previous = state.clone();
    let mut best: Option<SnapResult> = None;
    for &(xi, yi) in &probe.points {
        let (Some(x), Some(y)) = (probe.x.get(xi), probe.y.get(yi)) else {
            continue;
        };
        let mut held = if previous.x.is_some_and(|h| h.probe == xi)
            && previous.y.is_some_and(|h| h.probe == yi)
        {
            previous.clone()
        } else {
            SnapState::default()
        };
        let was_held = !held.is_empty();
        let old_target = held.x.or(held.y).map(|h| h.target);
        let mut result = resolve_along(
            settings,
            targets,
            Point::new(x.value, y.value),
            direction,
            &mut held,
        );
        if result.is_empty() {
            continue;
        }
        if let Some(h) = &mut result.held_x {
            h.probe = xi;
        }
        if let Some(h) = &mut result.held_y {
            h.probe = yi;
        }
        let retained = was_held && result.held_x.map(|h| h.target) == old_target;
        let key = |r: &SnapResult| {
            let id = r.held_x.or(r.held_y).map_or(usize::MAX, |h| h.target);
            (r.offset.length(), targets.entries[id].kind, id)
        };
        if retained || best.as_ref().is_none_or(|r| key(&result) < key(r)) {
            best = Some(result);
        }
        if retained {
            break;
        }
    }
    let result = best.unwrap_or_default();
    state.x = result.held_x;
    state.y = result.held_y;
    state.curve = None;
    result
}
