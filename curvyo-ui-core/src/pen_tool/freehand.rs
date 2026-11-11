//! Simplifies a freehand sample chain within an explicit distance tolerance.

use curvyo_document_core::{Length, NewAnchor, Point};

fn distance(point: Point, start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length_sq = dx.mul_add(dx, dy * dy);
    let t = if length_sq > 0.0 {
        ((point.x - start.x).mul_add(dx, (point.y - start.y) * dy) / length_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (point.x - (start.x + t * dx)).hypot(point.y - (start.y + t * dy))
}

pub(super) fn simplify(nodes: &mut Vec<NewAnchor>, tolerance: Length) {
    if nodes.len() < 3 || tolerance.as_mm() <= 0.0 {
        return;
    }
    let mut keep = vec![false; nodes.len()];
    let last = nodes.len() - 1;
    keep[0] = true;
    keep[last] = true;
    let mut spans = vec![(0, last)];
    while let Some((first, last)) = spans.pop() {
        let mut greatest = tolerance.as_mm();
        let mut farthest = None;
        for i in first + 1..last {
            let d = distance(nodes[i].point, nodes[first].point, nodes[last].point);
            if d > greatest {
                greatest = d;
                farthest = Some(i);
            }
        }
        if let Some(i) = farthest {
            keep[i] = true;
            spans.push((first, i));
            spans.push((i, last));
        }
    }
    let mut index = 0;
    nodes.retain(|_| {
        let result = keep[index];
        index += 1;
        result
    });
}
