//! The traces of linked-clone marks (`specs/0049-linked-clone` criteria 57 to 62): a selected
//! clone dashed, its original solid, a root thin, a faint clone thin dashed, each in
//! `--clone-link` over a white casing at a constant screen width, or an 8 px ring for an object
//! under 6 px on screen. Decoration only: stored nowhere, hit by nothing.

use curvyo_document_core::{NodeId, ObjectSnapshot, PathSnapshot, ViewTransform};
use lyon::algorithms::measure::{PathMeasurements, SampleType};
use lyon::path::Path;

use crate::color::RgbaColor;
use crate::glyphs::{self, DrawList};
use crate::stroke::{self, OutlineRef};
use crate::theme;

/// How one marked object is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneTrace {
    /// A selected clone: 2 px, 6 on and 3 off.
    Clone,
    /// A direct original: 2 px solid.
    Original,
    /// A root original: 1 px solid.
    Root,
    /// A clone of a selected original: 1 px, 6 on and 3 off, at 65 %.
    Faint,
    /// A clone under 6 px on screen: an 8 px dashed ring.
    CloneRing,
    /// An original under 6 px on screen: an 8 px solid ring.
    OriginalRing,
}

const DASH_ON_PX: f64 = 6.0;
const DASH_OFF_PX: f64 = 3.0;
const RING_DIAMETER_PX: f64 = 8.0;
/// The most dashes one trace draws; a longer outline draws solid.
const MAX_TRACE_DASHES: usize = 4000;

/// The traces of `marks` (object id and how), drawn from the outlines in `objects`. Every
/// casing first, then every line.
#[must_use]
pub fn build_clone_marks(
    objects: &[ObjectSnapshot],
    marks: &[(NodeId, CloneTrace)],
    view: ViewTransform,
) -> DrawList {
    let px = 1.0 / view.scale();
    let tolerance = theme::DISPLAY_TOLERANCE_PX * px;
    let mut list = DrawList::default();
    // The marked objects found once, not one scan of the list per mark (review S8).
    let wanted: std::collections::HashSet<NodeId> = marks.iter().map(|(id, _)| *id).collect();
    let found: std::collections::HashMap<NodeId, &ObjectSnapshot> = objects
        .iter()
        .filter(|object| wanted.contains(&object.id()))
        .map(|object| (object.id(), object))
        .collect();
    for casing in [true, false] {
        for &(id, trace) in marks {
            let Some(object) = found.get(&id).copied() else {
                continue;
            };
            let (width_px, dashed, alpha) = match trace {
                CloneTrace::Clone | CloneTrace::CloneRing => (2.0, true, 255),
                CloneTrace::Original | CloneTrace::OriginalRing => (2.0, false, 255),
                CloneTrace::Root => (1.0, false, 255),
                CloneTrace::Faint => (1.0, true, 166),
            };
            let (width, color) = if casing {
                (
                    (width_px + 2.0) * px,
                    theme::SELECTION_CASING.with_alpha(alpha),
                )
            } else {
                (width_px * px, theme::CLONE_LINK.with_alpha(alpha))
            };
            if matches!(trace, CloneTrace::CloneRing | CloneTrace::OriginalRing) {
                let centre = centre_of(object);
                if trace == CloneTrace::CloneRing {
                    // Dashed for a clone, solid for an original (criterion 61).
                    let ring = (ring_anchors(centre, RING_DIAMETER_PX * px / 2.0), true);
                    list.extend(trace_outline(&ring, (width, color), true, px, tolerance));
                } else {
                    list.extend(glyphs::ring(centre, RING_DIAMETER_PX * px, width, color));
                }
                continue;
            }
            for outline in outlines_of(object) {
                list.extend(trace_outline(
                    &outline,
                    (width, color),
                    dashed,
                    px,
                    tolerance,
                ));
            }
        }
    }
    list
}

/// A closed ring of 32 straight segments of `radius` around `centre`, for a dashed ring.
fn ring_anchors(
    centre: curvyo_document_core::Point,
    radius: f64,
) -> Vec<curvyo_document_core::AnchorSnapshot> {
    (0..32_u32)
        .map(|k| {
            let angle = f64::from(k) / 32.0 * std::f64::consts::TAU;
            curvyo_document_core::NewAnchor::corner(
                curvyo_document_core::AnchorId::new(0, u64::from(k)),
                curvyo_document_core::Point::new(
                    radius.mul_add(angle.cos(), centre.x),
                    radius.mul_add(angle.sin(), centre.y),
                ),
            )
        })
        .collect()
}

/// The centre of the box of an object's outline points.
fn centre_of(object: &ObjectSnapshot) -> curvyo_document_core::Point {
    let mut min = (f64::INFINITY, f64::INFINITY);
    let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for (anchors, _) in outlines_of(object) {
        for anchor in anchors {
            min = (min.0.min(anchor.point.x), min.1.min(anchor.point.y));
            max = (max.0.max(anchor.point.x), max.1.max(anchor.point.y));
        }
    }
    curvyo_document_core::Point::new(f64::midpoint(min.0, max.0), f64::midpoint(min.1, max.1))
}

/// The outlines an object is traced along: a path's or clone's own, a primitive's rotated
/// outline.
fn outlines_of(object: &ObjectSnapshot) -> Vec<(Vec<curvyo_document_core::AnchorSnapshot>, bool)> {
    let path: Option<&PathSnapshot> = match object {
        ObjectSnapshot::Path(path)
        | ObjectSnapshot::Clone(curvyo_document_core::CloneSnapshot { resolved: path, .. }) => {
            Some(path)
        }
        ObjectSnapshot::Primitive(_) => None,
    };
    match (path, object) {
        (Some(path), _) => path
            .subpaths()
            .map(|subpath| (subpath.anchors.to_vec(), subpath.closed))
            .collect(),
        (None, ObjectSnapshot::Primitive(primitive)) => vec![(
            crate::shape_preview::outline_to_anchors(&curvyo_document_core::outline_of_rotated(
                &primitive.shape,
                primitive.rotation,
            )),
            true,
        )],
        (None, _) => Vec::new(),
    }
}

fn trace_outline(
    (anchors, closed): &(Vec<curvyo_document_core::AnchorSnapshot>, bool),
    (width, color): (f64, RgbaColor),
    dashed: bool,
    px: f64,
    tolerance: f64,
) -> DrawList {
    if !dashed {
        return stroke::path_stroke(anchors, *closed, width, color, tolerance);
    }
    let Some(path) = stroke::build_outlines(
        &[OutlineRef {
            anchors,
            closed: *closed,
        }],
        false,
    ) else {
        return DrawList::default();
    };
    match dashes(&path, DASH_ON_PX * px, DASH_OFF_PX * px, tolerance) {
        Some(dashed) => stroke::path_stroke_of(&dashed, width, color, tolerance),
        None => stroke::path_stroke(anchors, *closed, width, color, tolerance),
    }
}

/// The on-intervals of `path` for a dash of `on` and `off` (mm), or `None` when there would be
/// more than [`MAX_TRACE_DASHES`].
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)] // small screen lengths and counts
fn dashes(path: &Path, on: f64, off: f64, tolerance: f64) -> Option<Path> {
    let measurements = PathMeasurements::from_path(path, tolerance as f32);
    let length = measurements.length();
    let period = (on + off) as f32;
    if !length.is_finite() || period <= 0.0 || length / period > MAX_TRACE_DASHES as f32 {
        return None;
    }
    let mut sampler = measurements.create_sampler(path, SampleType::Distance);
    let mut builder = Path::builder();
    let mut distance = 0.0_f32;
    while distance < length {
        let end = (distance + on as f32).min(length);
        sampler.split_range(distance..end, &mut builder);
        distance += period;
    }
    Some(builder.build())
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, Point};

    use super::*;

    #[test]
    fn a_dashed_trace_draws_less_than_a_solid_one_and_a_casing_comes_first() {
        let d = Document::new(1);
        let c = |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
        let o = d.create_path(&[c(0, 0.0, 0.0), c(1, 50.0, 0.0), c(2, 50.0, 50.0)], true);
        let clone = d.linked_clone(&[o], &[]).unwrap()[0];
        let objects = d.read_objects().objects;
        let view = ViewTransform::new(4.0, Point::new(0.0, 0.0));
        let dashed = build_clone_marks(&objects, &[(clone, CloneTrace::Clone)], view);
        let solid = build_clone_marks(&objects, &[(o, CloneTrace::Original)], view);
        assert!(!dashed.triangles.is_empty() && !solid.triangles.is_empty());
        assert_eq!(dashed.triangles[0].color, theme::SELECTION_CASING);
        assert!(solid.triangles.iter().any(|v| v.color == theme::CLONE_LINK));
        let ring = build_clone_marks(&objects, &[(o, CloneTrace::OriginalRing)], view);
        let dashed_ring = build_clone_marks(&objects, &[(clone, CloneTrace::CloneRing)], view);
        assert!(
            dashed_ring.triangles.len() < ring.triangles.len() * 2
                && dashed_ring.triangles.len() > 2
        );
        assert!(ring.triangles.len() > 2);
        assert_eq!(build_clone_marks(&objects, &[], view).triangles.len(), 0);
    }
}
