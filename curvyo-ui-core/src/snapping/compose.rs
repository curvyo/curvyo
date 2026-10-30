//! Composition of an eligible finite guide with one independent axis constraint.
use super::{HeldSnap, Point, SnapProbe, SnapSettings, SnapTargets, Vec2};

type Composition = (HeldSnap, HeldSnap, (HeldSnap, HeldSnap));

pub(super) fn compose(
    settings: &SnapSettings,
    targets: &SnapTargets,
    probe: &SnapProbe,
    guide: (HeldSnap, HeldSnap, bool),
    x: Option<HeldSnap>,
    y: Option<HeldSnap>,
) -> Option<Composition> {
    if x.is_none() && y.is_none() {
        return None;
    }
    if guide.2 {
        return compose_one(settings, targets, probe, guide, x, y)
            .map(|(x, y)| (x, y, (guide.0, guide.1)));
    }
    let mut best: Option<(f64, usize, Composition)> = None;
    for &(xi, yi) in &probe.points {
        let raw = Point::new(probe.x[xi].value, probe.y[yi].value);
        for target in targets.index.points(raw, settings.capture + 1e-9) {
            let entry = &targets.entries[target];
            let Some(curve) = entry.curve else { continue };
            if !entry.kind.is_enabled(settings) {
                continue;
            }
            let point = curve.project(raw);
            if raw.vector_to(point).length() > settings.capture + 1e-9 {
                continue;
            }
            let gx = HeldSnap {
                target,
                probe: xi,
                coordinate: point.x,
            };
            let gy = HeldSnap {
                target,
                probe: yi,
                coordinate: point.y,
            };
            let Some((cx, cy)) = compose_one(settings, targets, probe, (gx, gy, false), x, y)
            else {
                continue;
            };
            let distance = (cx.coordinate - probe.x[cx.probe].value)
                .hypot(cy.coordinate - probe.y[cy.probe].value);
            if best
                .as_ref()
                .is_none_or(|(d, id, _)| (distance, target) < (*d, *id))
            {
                best = Some((distance, target, (cx, cy, (gx, gy))));
            }
        }
    }
    best.map(|(_, _, result)| result)
}

fn compose_one(
    settings: &SnapSettings,
    targets: &SnapTargets,
    probe: &SnapProbe,
    guide: (HeldSnap, HeldSnap, bool),
    x: Option<HeldSnap>,
    y: Option<HeldSnap>,
) -> Option<(HeldSnap, HeldSnap)> {
    let (gx, gy, retained) = guide;
    let curve = targets.entries[gx.target].curve?;
    let raw = Point::new(probe.x[gx.probe].value, probe.y[gy.probe].value);
    let radius = if retained {
        settings.release
    } else {
        settings.capture
    };
    let mut best: Option<(f64, usize, HeldSnap, HeldSnap)> = None;
    for (axis, line) in [x, y].into_iter().enumerate() {
        let Some(line) = line else { continue };
        let (origin, direction) = if axis == 0 {
            (
                Point::new(raw.x + line.coordinate - probe.x[line.probe].value, raw.y),
                Vec2::new(0.0, 1.0),
            )
        } else {
            (
                Point::new(raw.x, raw.y + line.coordinate - probe.y[line.probe].value),
                Vec2::new(1.0, 0.0),
            )
        };
        for offset in super::constrained::curve_intersections(curve, origin, direction) {
            let point = origin.translated(offset);
            let distance = raw.vector_to(point).length();
            if distance > radius + 1e-9 {
                continue;
            }
            let pair = if axis == 0 {
                (
                    line,
                    HeldSnap {
                        coordinate: point.y,
                        ..gy
                    },
                )
            } else {
                (
                    HeldSnap {
                        coordinate: point.x,
                        ..gx
                    },
                    line,
                )
            };
            if best
                .as_ref()
                .is_none_or(|&(d, id, _, _)| (distance, line.target) < (d, id))
            {
                best = Some((distance, line.target, pair.0, pair.1));
            }
        }
    }
    best.map(|(_, _, x, y)| (x, y))
}
