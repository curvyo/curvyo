//! The markers of one outline as closed loops (`specs/0085-stroke-to-path` criterion 10,
//! `adrs.md` decision 4): where the canvas puts them (`curvyo-render-core`'s `marker_place.rs`:
//! ends and nodes from the control polygon, spaced markers by arc length on the undashed outline)
//! and their exact shapes (`markers.rs`: an arrow 4 widths long and 3 wide, a dot 3 widths across,
//! centred on its point).

use curvyo_document_core::{MarkerPlace, MarkerShape, Markers};
use kurbo::{PathSeg, Point, Vec2};

use crate::stroke_dash::Measured;
use crate::stroke_loops::{circle_loop, polygon_loop};

/// The most markers one slot of one outline draws, as on the canvas.
const MAX_MARKERS_PER_SLOT: usize = 500;

/// A path shorter than this has no markers, millimetres.
const MIN_LENGTH_MM: f64 = 0.001;

/// A handle shorter than this has no direction, millimetres.
const MIN_HANDLE_MM: f64 = 1e-9;

/// A spaced marker this close to a node sits on it, millimetres.
const NODE_SNAP_MM: f64 = 1e-3;

/// The cosine of a turn within a degree of a reversal: `-cos(1°)`.
const REVERSAL_COS: f64 = -0.999_847_695_156_391_2;

/// An anchor: point, handle in, handle out (handles relative).
type Anchor = (Point, Vec2, Vec2);

fn unit(v: Vec2) -> Option<Vec2> {
    let length = v.hypot();
    (length > MIN_HANDLE_MM && length.is_finite()).then(|| v / length)
}

/// The control points met walking away from anchor `i` along the control polygon.
fn polygon_points(anchors: &[Anchor], closed: bool, i: usize, forward: bool) -> Vec<Point> {
    let n = anchors.len();
    let (point, handle_in, handle_out) = anchors[i];
    let mut points = vec![point + if forward { handle_out } else { handle_in }];
    for step in 1..n {
        let j = if forward {
            i + step
        } else if step <= i || closed {
            i + n - step
        } else {
            break;
        };
        if forward && !closed && j >= n {
            break;
        }
        let (p, hin, hout) = anchors[j % n];
        let (near, far) = if forward { (hin, hout) } else { (hout, hin) };
        points.extend([p + near, p, p + far]);
    }
    points
}

fn outgoing(anchors: &[Anchor], closed: bool, i: usize) -> Option<Vec2> {
    let origin = anchors[i].0;
    polygon_points(anchors, closed, i, true)
        .into_iter()
        .find_map(|target| unit(target - origin))
}

fn incoming(anchors: &[Anchor], closed: bool, i: usize) -> Option<Vec2> {
    let target = anchors[i].0;
    polygon_points(anchors, closed, i, false)
        .into_iter()
        .find_map(|source| unit(target - source))
}

fn at_node(anchors: &[Anchor], closed: bool, i: usize) -> Option<Vec2> {
    match (incoming(anchors, closed, i), outgoing(anchors, closed, i)) {
        (Some(inc), Some(out)) => {
            if inc.dot(out) < REVERSAL_COS {
                return Some(out);
            }
            unit(inc + out).or(Some(out))
        }
        (Some(only), None) | (None, Some(only)) => Some(only),
        (None, None) => None,
    }
}

/// The loops of every marker of the outline through `anchors` (segments `segments`), sized by
/// `width` millimetres.
pub(crate) fn marker_loops(
    anchors: &[Anchor],
    closed: bool,
    segments: &[PathSeg],
    markers: Markers,
    width: f64,
) -> Vec<Vec<PathSeg>> {
    let used = [markers.start, markers.mid, markers.end]
        .iter()
        .any(|shape| *shape != MarkerShape::None);
    if anchors.len() < 2 || segments.is_empty() || !used {
        return Vec::new();
    }
    let measured = Measured::new(segments);
    let length = measured.total();
    if !(length.is_finite() && length >= MIN_LENGTH_MM) {
        return Vec::new();
    }
    let mut placed: Vec<(MarkerShape, Point, Vec2)> = Vec::new();
    let last = anchors.len() - 1;
    if !closed {
        if let (true, Some(direction)) = (
            markers.start != MarkerShape::None,
            outgoing(anchors, false, 0),
        ) {
            placed.push((markers.start, anchors[0].0, -direction));
        }
        if let (true, Some(direction)) = (
            markers.end != MarkerShape::None,
            incoming(anchors, false, last),
        ) {
            placed.push((markers.end, anchors[last].0, direction));
        }
    }
    if markers.mid != MarkerShape::None {
        match markers.mid_place {
            MarkerPlace::AtNodes => {
                let nodes = if closed {
                    0..=last
                } else {
                    1..=last.saturating_sub(1)
                };
                for i in nodes.take(MAX_MARKERS_PER_SLOT) {
                    if let Some(direction) = at_node(anchors, closed, i) {
                        placed.push((markers.mid, anchors[i].0, direction));
                    }
                }
            }
            MarkerPlace::Spaced => {
                spaced(anchors, closed, &measured, markers, &mut placed);
            }
        }
    }
    placed
        .into_iter()
        .filter_map(|(shape, at, direction)| marker_loop(shape, at, direction, width))
        .collect()
}

/// The evenly spaced Middle markers: open `k / (N + 1)`, closed `k / N` of the length.
fn spaced(
    anchors: &[Anchor],
    closed: bool,
    measured: &Measured<'_>,
    markers: Markers,
    placed: &mut Vec<(MarkerShape, Point, Vec2)>,
) {
    let length = measured.total();
    let count = (markers.mid_count.get() as usize).min(MAX_MARKERS_PER_SLOT);
    let step = (length * 0.001).clamp(1e-4, 0.05);
    let position_at = |distance: f64| {
        let distance = if closed {
            distance.rem_euclid(length)
        } else {
            distance.clamp(0.0, length)
        };
        measured.sample(distance).0
    };
    for k in 0..count {
        #[allow(clippy::cast_precision_loss)] // at most 500
        let fraction = if closed {
            k as f64 / count as f64
        } else {
            (k + 1) as f64 / (count + 1) as f64
        };
        let at = length * fraction;
        let point = position_at(at);
        let on_node = anchors
            .iter()
            .position(|anchor| (anchor.0 - point).hypot() <= NODE_SNAP_MM);
        let direction = on_node
            .and_then(|i| at_node(anchors, closed, i))
            .or_else(|| unit(position_at(at + step) - position_at(at - step)))
            .unwrap_or(Vec2::new(1.0, 0.0));
        placed.push((markers.mid, point, direction));
    }
}

/// One marker's loop: an arrow (tip ahead) or a dot.
fn marker_loop(shape: MarkerShape, at: Point, along: Vec2, width: f64) -> Option<Vec<PathSeg>> {
    match shape {
        MarkerShape::None => None,
        MarkerShape::Arrow => {
            let across = Vec2::new(-along.y, along.x);
            let (half_length, half_width) = (2.0 * width, 1.5 * width);
            let base = at - along * half_length;
            Some(polygon_loop(&[
                at + along * half_length,
                base + across * half_width,
                base - across * half_width,
            ]))
        }
        MarkerShape::Dot => Some(circle_loop(at, 1.5 * width)),
    }
}
