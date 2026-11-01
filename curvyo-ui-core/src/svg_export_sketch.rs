//! A sketch's solved geometry as plain objects for the SVG export (`specs/0024-svg-import-export`
//! criterion 138): one object per drawn entity in sketch order, a circle or arc as the primitive
//! the import gives back, construction geometry left out.

use std::collections::{BTreeMap, BTreeSet};

use curvyo_document_core::{
    Angle, ArcKind, Curve, EllipseArc, EllipseFrame, Length, ObjectSnapshot, Point,
    PrimitiveSnapshot, Shape, SketchSnapshot, sketch_outline,
};
use curvyo_sketch_core::{Entity, EntityId, Sketch};
use kurbo::Affine;

/// How far the placement may be from a similarity (relative to its scale) for a circle or arc
/// to stay a circle or arc.
const SIMILAR_TOLERANCE: f64 = 1e-12;

/// The placement as a similarity: its scale, its turn and whether it mirrors.
#[derive(Debug, Clone, Copy)]
struct Similarity {
    scale: f64,
    turn: f64,
    mirrored: bool,
}

impl Similarity {
    #[allow(clippy::many_single_char_names)] // the matrix entries of SVG
    fn of(m: Affine) -> Option<Self> {
        let [a, b, c, d, _, _] = m.as_coeffs();
        let scale = a.hypot(b);
        let mirrored = m.determinant() < 0.0;
        let limit = SIMILAR_TOLERANCE * scale.max(1.0);
        let similar = if mirrored {
            (a + d).abs() <= limit && (b - c).abs() <= limit
        } else {
            (a - d).abs() <= limit && (b + c).abs() <= limit
        };
        (similar && scale > 0.0).then(|| Self {
            scale,
            turn: b.atan2(a),
            mirrored,
        })
    }

    /// A local direction angle in the document.
    fn angle(self, local: f64) -> f64 {
        if self.mirrored {
            self.turn - local
        } else {
            self.turn + local
        }
    }
}

/// The objects a sketch is exported as, in document millimetres (criterion 138).
pub(crate) fn sketch_objects(sketch: &SketchSnapshot) -> Vec<ObjectSnapshot> {
    let graph = &sketch.solution;
    let placement = Affine::new(sketch.placement.to_array());
    let similar = Similarity::of(placement);
    graph
        .entities
        .iter()
        .filter(|(id, _)| !graph.construction.contains(*id))
        .filter_map(|(id, entity)| {
            let primitive = similar.and_then(|s| round_shape(graph, entity, placement, s));
            match primitive {
                Some((shape, rotation)) => Some(ObjectSnapshot::Primitive(PrimitiveSnapshot {
                    id: sketch.id,
                    shape,
                    style: sketch.style.clone(),
                    rotation,
                })),
                None => entity_path(sketch, id, entity),
            }
        })
        .collect()
}

/// A circle, arc or ellipse entity as the primitive the import of its element gives.
fn round_shape(
    graph: &Sketch,
    entity: &Entity,
    placement: Affine,
    s: Similarity,
) -> Option<(Shape, Angle)> {
    let at = |id: &EntityId| {
        graph.point(id).ok().map(|p| {
            let q = placement * kurbo::Point::new(p.x.0, p.y.0);
            Point::new(q.x, q.y)
        })
    };
    let ellipse = |center: Point, rx: f64, ry: f64, arc: Option<EllipseArc>| Shape::Ellipse {
        frame: EllipseFrame {
            center,
            rx: Length::from_mm(rx),
            ry: Length::from_mm(ry),
        },
        arc,
        curve: Curve::ELLIPSE_DEFAULT,
    };
    let none = Angle::from_radians(0.0);
    match entity {
        Entity::Circle { center, radius, .. } => {
            let r = radius.0 * s.scale;
            Some((ellipse(at(center)?, r, r, None), none))
        }
        Entity::Arc {
            center,
            radius,
            start_angle,
            end_angle,
            ..
        } => {
            let sweep = (end_angle.0 - start_angle.0).rem_euclid(std::f64::consts::TAU);
            let (from, to) = (start_angle.0, start_angle.0 + sweep);
            let (start, end) = if s.mirrored {
                (s.angle(to), s.angle(from))
            } else {
                (s.angle(from), s.angle(to))
            };
            let arc = EllipseArc::new(
                Angle::from_radians(start),
                Angle::from_radians(end),
                ArcKind::Arc,
            );
            let r = radius.0 * s.scale;
            Some((ellipse(at(center)?, r, r, arc), none))
        }
        Entity::Ellipse {
            center,
            focus,
            minor_radius,
            ..
        } => {
            let (c, f) = (graph.point(center).ok()?, graph.point(focus).ok()?);
            let (dx, dy) = (f.x.0 - c.x.0, f.y.0 - c.y.0);
            let major = minor_radius.0.hypot(dx.hypot(dy));
            let turn = Angle::from_radians(s.angle(dy.atan2(dx))).normalized();
            Some((
                ellipse(at(center)?, major * s.scale, minor_radius.0 * s.scale, None),
                turn,
            ))
        }
        Entity::Point { .. } | Entity::Line { .. } => None,
    }
}

/// One entity alone as the path the canvas draws of it.
fn entity_path(sketch: &SketchSnapshot, id: &EntityId, entity: &Entity) -> Option<ObjectSnapshot> {
    let graph = &sketch.solution;
    let points: Vec<&EntityId> = match entity {
        Entity::Point { .. } => return None,
        Entity::Line { start, end } => vec![start, end],
        Entity::Circle { center, .. } => vec![center],
        Entity::Arc {
            center, start, end, ..
        } => vec![center, start, end],
        Entity::Ellipse { center, focus, .. } => vec![center, focus],
    };
    let mut entities = BTreeMap::new();
    for point in points {
        entities.insert(point.clone(), graph.entities.get(point)?.clone());
    }
    entities.insert(id.clone(), entity.clone());
    let origin = graph.origin.clone();
    if let Some(o) = graph.entities.get(&origin) {
        entities.entry(origin.clone()).or_insert_with(|| o.clone());
    }
    let alone = Sketch {
        origin,
        entities,
        construction: BTreeSet::new(),
        contact_seeds: BTreeMap::new(),
        constraints: BTreeMap::new(),
        parameters: curvyo_sketch_core::Parameters::default(),
    };
    sketch_outline(sketch.id, &alone, sketch.placement, &sketch.style)
        .ok()
        .filter(|path| !path.anchors.is_empty())
        .map(ObjectSnapshot::Path)
}
