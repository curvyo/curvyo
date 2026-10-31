//! Analytic sketch output maps to ordinary cubic path snapshots for readers.
use crate::{
    Affine, AnchorId, Angle, NewAnchor, NodeId, PathSnapshot, Point, Style, SubpathSnapshot, Vec2,
};
use curvyo_sketch_core::Sketch;
use kurbo::PathEl;

/// Derives an output path for a validated analytic pose without creating editable path records.
/// The caller owns solution validation; this function only approximates its geometry.
/// # Errors
/// Rejects malformed analytic geometry or unsupported path elements.
pub fn outline(
    id: NodeId,
    graph: &Sketch,
    placement: Affine,
    style: &Style,
) -> Result<PathSnapshot, String> {
    outline_scaled(id, graph, placement, style, 1.0)
}

pub(crate) fn outline_scaled(
    id: NodeId,
    graph: &Sketch,
    placement: Affine,
    style: &Style,
    output_stretch: f64,
) -> Result<PathSnapshot, String> {
    // Bound local approximation by the largest possible linear stretch.
    let stretch = (placement.a * placement.a
        + placement.b * placement.b
        + placement.c * placement.c
        + placement.d * placement.d)
        .sqrt()
        .max(1.0);
    let path = curvyo_sketch_core::outline(
        graph,
        curvyo_sketch_core::Length(0.01 / (stretch * output_stretch.max(1.0))),
    )
    .map_err(|e| e.to_string())?;
    let mut subpaths = Vec::new();
    let mut current = SubpathSnapshot {
        closed: false,
        anchors: Vec::new(),
    };
    let mut serial = 0;
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) => {
                if !current.anchors.is_empty() {
                    subpaths.push(current);
                }
                current = SubpathSnapshot {
                    closed: false,
                    anchors: vec![anchor(id, &mut serial, p)],
                };
            }
            PathEl::LineTo(p) => current.anchors.push(anchor(id, &mut serial, p)),
            PathEl::CurveTo(a, b, end) => {
                if let Some(previous) = current.anchors.last_mut() {
                    previous.handle_out = Vec2::new(a.x - previous.point.x, a.y - previous.point.y);
                }
                let mut next = anchor(id, &mut serial, end);
                next.handle_in = Vec2::new(b.x - end.x, b.y - end.y);
                current.anchors.push(next);
            }
            PathEl::QuadTo(_, _) => return Err("unexpected quadratic sketch output".into()),
            PathEl::ClosePath => {
                current.closed = true;
                if current.anchors.len() > 1
                    && current
                        .anchors
                        .first()
                        .zip(current.anchors.last())
                        .is_some_and(|(a, b)| {
                            (a.point.x - b.point.x).hypot(a.point.y - b.point.y) <= 1e-9
                        })
                    && let Some(last) = current.anchors.pop()
                {
                    current.anchors[0].handle_in = last.handle_in;
                }
            }
        }
    }
    if !current.anchors.is_empty() {
        subpaths.push(current);
    }
    let first = if subpaths.is_empty() {
        SubpathSnapshot {
            closed: false,
            anchors: vec![],
        }
    } else {
        subpaths.remove(0)
    };
    Ok(PathSnapshot {
        id,
        closed: first.closed,
        style: style.clone(),
        anchors: first.anchors,
        extra_subpaths: subpaths,
        rotation: Angle::from_radians(placement.b.atan2(placement.a)),
    }
    .mapped(placement))
}
fn anchor(id: NodeId, serial: &mut u64, point: kurbo::Point) -> NewAnchor {
    *serial += 1;
    NewAnchor::corner(
        AnchorId::new(
            id.peer,
            (u64::from(id.counter.unsigned_abs()) << 32) | *serial,
        ),
        Point::new(point.x, point.y),
    )
}

/// Derives transient geometry before an object identity exists.
/// Anchor identities in this result are render-only and must never be persisted.
/// # Errors
/// Rejects malformed analytic geometry.
pub fn preview_outline(graph: &Sketch, placement: Affine) -> Result<Vec<SubpathSnapshot>, String> {
    let path = outline(
        NodeId::from_parts(0, 0),
        graph,
        placement,
        &Style::default(),
    )?;
    let mut paths = vec![SubpathSnapshot {
        closed: path.closed,
        anchors: path.anchors,
    }];
    paths.extend(path.extra_subpaths);
    Ok(paths)
}
