//! What the shell is told about the curved box: the size handles on the plates and the marks of
//! the canvas overlay (ux-notes of `0147` §10.7 to §10.9).

use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, SQRT_2};

use curvyo_document_core::{Length, Point, Vec2};
use curvyo_extension_core::{Handle, Mark, Marks};

use super::Sizes;
use crate::outline::{Builder, polar};
use crate::rounded_box::layout::Placed;

fn offset_of(placed: &[Placed], role: &str) -> (f64, f64) {
    placed
        .iter()
        .find(|p| p.role == role)
        .map_or((0.0, 0.0), |p| p.offset)
}

/// The size handles: width and depth on the base's right and back edges, height on the front's
/// top edge beside the hollow, the Corner radius on the left end plate's arc.
pub(super) fn handles(placed: &[Placed], s: &Sizes, hollow_width: Option<f64>) -> Vec<Handle> {
    let (base, front, left) = (
        offset_of(placed, "base"),
        offset_of(placed, "front"),
        offset_of(placed, "left"),
    );
    let (w, d) = (s.outer_width(), s.depth + s.front + s.wall);
    // Midway between the hollow's right side and the edge's end, else the edge's middle.
    let top = hollow_width.map_or(w / 2.0, |hw| (w / 2.0 + hw / 2.0 + w) / 2.0);
    // The arc's middle point moves 0.414 mm per mm of radius along the inward diagonal.
    let rate = 1.0 / (1.0 - FRAC_1_SQRT_2) / SQRT_2;
    let (px, r) = (s.profile_width(), s.radius);
    vec![
        Handle {
            key: "inner_width",
            anchor: Point::new(base.0 + w - s.end, base.1 + d / 2.0),
            direction: Vec2::new(1.0, 0.0),
            rate: 1.0,
        },
        Handle {
            key: "inner_depth",
            anchor: Point::new(base.0 + w / 2.0, base.1 + d - s.wall),
            direction: Vec2::new(0.0, 1.0),
            rate: 1.0,
        },
        Handle {
            key: "inner_height",
            anchor: Point::new(front.0 + top, front.1),
            direction: Vec2::new(0.0, -1.0),
            rate: 1.0,
        },
        Handle {
            key: "corner_radius",
            anchor: Point::new(
                left.0 + px - (1.0 - FRAC_1_SQRT_2) * r,
                left.1 + (1.0 - FRAC_1_SQRT_2) * r,
            ),
            direction: Vec2::new(-FRAC_1_SQRT_2, FRAC_1_SQRT_2),
            rate,
        },
    ]
}

/// The middle of the bounding box of a placed part's outline.
fn middle(placed: &Placed) -> Option<Point> {
    let path = placed.part.paths.first()?;
    let points: Vec<Point> = std::iter::once(path.start)
        .chain(path.segments.iter().map(|c| c.to))
        .collect();
    let range = |f: fn(&Point) -> f64| {
        points
            .iter()
            .map(f)
            .fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(v), b.max(v)))
    };
    let (x, y) = (range(|p| p.x), range(|p| p.y));
    Some(Point::new(f64::midpoint(x.0, x.1), f64::midpoint(y.0, y.1)))
}

/// The bend band on each end plate's arc and the part numbers (ux-notes §10.8, §10.9). All marks
/// are in the object's frame, so the footprint offset is zero.
pub(super) fn marks(placed: &[Placed], s: &Sizes) -> Marks {
    let r = s.radius;
    let mut on_footprint = Vec::new();
    for plate in placed.iter().filter(|p| matches!(p.role, "left" | "right")) {
        let centre = Point::new(plate.offset.0 + s.profile_width() - r, plate.offset.1 + r);
        let start = polar(centre, r, -FRAC_PI_2);
        let mut builder = Builder::new(start);
        builder.arc(centre, r, -FRAC_PI_2, 0.0);
        on_footprint.push(Mark::Bend {
            start,
            segments: builder.segments(),
            tight: false,
        });
    }
    let on_parts = placed
        .iter()
        .filter_map(|plate| {
            // The strip's badge sits in the middle of the lid run, which has no slits.
            let at = if plate.role == "strip" {
                let lid = s.depth - r + s.front;
                Point::new(
                    plate.offset.0 + lid / 2.0,
                    plate.offset.1 + s.outer_width() / 2.0,
                )
            } else {
                middle(plate)?
            };
            Some(Mark::Piece {
                number: plate.number,
                at,
                inward: Vec2::new(0.0, 0.0),
                length: Length::from_mm(0.0),
            })
        })
        .collect();
    Marks {
        footprint: Vec2::new(0.0, 0.0),
        on_footprint,
        on_parts,
    }
}
