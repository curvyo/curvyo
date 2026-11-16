//! The centres the Centres target offers to a job fixture being placed or dragged
//! (`specs/0113-fixture-sets-and-pins` criterion 30, amended 2026-11-16).
//!
//! A circle, an ellipse and a polygon offer the centre of their frame; every other object offers
//! the centre of its box; an object a laser page shows through a placed page offers its centre
//! where the placement shows it. The list goes through the same index and [`super::resolve`] as
//! every other target, so capture, release, hysteresis and the marker are those of `0060`.

use curvyo_document_core::{
    Affine, NodeKind, NodeRead, ObjectSnapshot, Point, Shape as PrimitiveShape,
};

use super::{SnapExclusions, SnapTarget, SnapTargets, TargetKind};

/// The centre an object offers: its frame's centre for an ellipse or a polygon, else the centre
/// of its outline box.
fn centre_of(object: &ObjectSnapshot) -> Option<(Point, (Point, Point))> {
    let (min, max) = crate::object_bounds::object_outline_bounds_with_artwork(
        object,
        &crate::variable_artwork::synchronous_artwork,
    )?;
    let box_centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
    let centre = match object {
        ObjectSnapshot::Primitive(primitive) => match &primitive.shape {
            PrimitiveShape::Ellipse { frame, .. } => frame.center,
            PrimitiveShape::Polygon { frame, .. } => frame.center,
            _ => box_centre,
        },
        _ => box_centre,
    };
    Some((centre, (min, max)))
}

impl SnapTargets {
    /// The centres of every effectively visible object that is not a guide, as point targets
    /// (criterion 30); empty while the Centres target is off. `exclusions.boxes` names nodes that
    /// offer none.
    #[must_use]
    pub fn gather_centres(
        nodes: &[NodeRead],
        centres: bool,
        exclusions: SnapExclusions<'_>,
    ) -> Self {
        if !centres {
            return Self::empty();
        }
        let moving = super::collection::moving(nodes, exclusions.boxes);
        let entries = nodes
            .iter()
            .filter(|read| read.visible && !read.guide && !moving.contains(&read.id))
            .filter(|read| !matches!(read.kind, NodeKind::Group))
            .filter_map(|read| centre_of(read.object()?))
            .map(|(centre, extent)| SnapTarget::point(centre, extent, TargetKind::Box))
            .collect();
        Self::new(entries)
    }

    /// Adds the centres of `objects` as a placed page shows them, through `map` (the drawing
    /// page's plane to the laser page's plane): a laser page's drawing comes from its placed
    /// pages (ADR 0042 §4), so their objects offer their centres where they are seen.
    pub fn add_placed_centres<'a>(
        &mut self,
        objects: impl IntoIterator<Item = &'a ObjectSnapshot>,
        map: Affine,
    ) {
        let before = self.entries.len();
        for (centre, (min, max)) in objects.into_iter().filter_map(centre_of) {
            let corners = [min, Point::new(max.x, min.y), max, Point::new(min.x, max.y)]
                .map(|corner| map.apply_point(corner));
            let (lo, hi) = corners
                .iter()
                .fold((corners[0], corners[0]), |(lo, hi), c| {
                    (
                        Point::new(lo.x.min(c.x), lo.y.min(c.y)),
                        Point::new(hi.x.max(c.x), hi.y.max(c.y)),
                    )
                });
            let centre = map.apply_point(centre);
            self.entries
                .push(SnapTarget::point(centre, (lo, hi), TargetKind::Box));
        }
        if self.entries.len() > before {
            self.index = super::index::Index::build(&self.entries);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SnapProbe, SnapSettings, SnapState, resolve};
    use curvyo_document_core::{
        Angle, Document, EllipseFrame, Length, PointCount, RectBounds, StarFrame, Vec2,
    };

    fn settings() -> SnapSettings {
        SnapSettings {
            enabled: true,
            objects: true,
            guides: false,
            nodes: false,
            page: false,
            grid: false,
            x: true,
            y: true,
            capture: 2.0,
            release: 3.0,
        }
    }

    #[test]
    fn a_circle_an_ellipse_a_polygon_and_a_rectangle_offer_their_centres_only() {
        let document = Document::new(1);
        let _ = document.create_ellipse(EllipseFrame {
            center: Point::new(10.0, 10.0),
            rx: Length::from_mm(3.0),
            ry: Length::from_mm(3.0),
        });
        let _ = document.create_ellipse(EllipseFrame {
            center: Point::new(40.0, 10.0),
            rx: Length::from_mm(8.0),
            ry: Length::from_mm(2.0),
        });
        let _ = document.create_polygon(
            StarFrame {
                center: Point::new(70.0, 10.0),
                radius: Length::from_mm(5.0),
                angle: Angle::from_radians(-std::f64::consts::FRAC_PI_2),
            },
            PointCount::new(3).unwrap(),
        );
        let _ = document.create_rect(RectBounds::from_corners(
            Point::new(0.0, 50.0),
            Point::new(20.0, 60.0),
        ));
        let targets =
            SnapTargets::gather_centres(&document.read_nodes(), true, SnapExclusions::default());
        let points: Vec<Point> = targets.entries().iter().map(|t| t.point).collect();
        assert_eq!(
            points.len(),
            4,
            "one centre per object, no edges or corners"
        );
        let near = |a: Point, b: Point| a.vector_to(b).length() < 1e-9;
        assert!(near(points[0], Point::new(10.0, 10.0)));
        assert!(near(points[1], Point::new(40.0, 10.0)));
        assert!(
            near(points[2], Point::new(70.0, 10.0)),
            "a triangle offers its frame's centre, not its box's"
        );
        assert!(near(points[3], Point::new(10.0, 55.0)));
        assert!(
            SnapTargets::gather_centres(&document.read_nodes(), false, SnapExclusions::default())
                .is_empty(),
            "nothing while the Centres target is off"
        );
    }

    #[test]
    fn a_placed_circle_and_rectangle_offer_their_centres_where_the_placement_shows_them() {
        let document = Document::new(1);
        let _ = document.create_ellipse(EllipseFrame {
            center: Point::new(10.0, 10.0),
            rx: Length::from_mm(3.0),
            ry: Length::from_mm(3.0),
        });
        let _ = document.create_rect(RectBounds::from_corners(
            Point::new(0.0, 50.0),
            Point::new(20.0, 60.0),
        ));
        let nodes = document.read_nodes();
        let objects: Vec<&ObjectSnapshot> = nodes.iter().filter_map(NodeRead::object).collect();
        let mut targets = SnapTargets::empty();
        let map = Affine::translation(Vec2::new(500.0, 200.0));
        targets.add_placed_centres(objects, map);
        let points: Vec<Point> = targets.entries().iter().map(|t| t.point).collect();
        let near = |a: Point, b: Point| a.vector_to(b).length() < 1e-9;
        assert_eq!(points.len(), 2);
        assert!(near(points[0], Point::new(510.0, 210.0)), "{points:?}");
        assert!(near(points[1], Point::new(510.0, 255.0)), "{points:?}");
        let mut state = SnapState::default();
        let probe = [Point::new(510.8, 209.5)];
        let result = resolve(
            &settings(),
            &targets,
            &SnapProbe::points(&probe),
            &mut state,
        );
        assert!(
            near(probe[0].translated(result.offset), Point::new(510.0, 210.0)),
            "the placed circle's centre captures"
        );
    }

    /// The nearest centre within the capture zone by a full scan, as the index must find it.
    fn scan(centres: &[Point], probe: &[Point], capture: f64) -> Option<(usize, usize)> {
        let mut best: Option<(f64, usize, usize)> = None;
        for (t, c) in centres.iter().enumerate() {
            for (p, q) in probe.iter().enumerate() {
                let d = c.vector_to(*q).length();
                if d <= capture && best.is_none_or(|(b, bt, _)| (d, t) < (b, bt)) {
                    best = Some((d, t, p));
                }
            }
        }
        best.map(|(_, t, p)| (t, p))
    }

    #[test]
    fn on_a_thousand_shapes_the_index_finds_the_nearest_centre_of_any_grid_centre() {
        let document = Document::new(1);
        let mut centres = Vec::new();
        for i in 0..1000_u32 {
            let (col, row) = (f64::from(i % 40), f64::from(i / 40));
            let centre = Point::new(col * 9.7 + row * 0.31, row * 11.3 + col * 0.17);
            centres.push(centre);
            let _ = document.create_ellipse(EllipseFrame {
                center: centre,
                rx: Length::from_mm(2.0),
                ry: Length::from_mm(2.0),
            });
        }
        let targets =
            SnapTargets::gather_centres(&document.read_nodes(), true, SnapExclusions::default());
        assert_eq!(targets.len(), 1000);
        for k in 0..60_u32 {
            let origin = Point::new(f64::from(k) * 6.1 + 0.4, f64::from(k) * 4.9 + 0.7);
            let grid: Vec<Point> = (0..4)
                .map(|n| origin.translated(Vec2::new(f64::from(n) * 20.0, 0.0)))
                .collect();
            let mut state = SnapState::default();
            let result = resolve(&settings(), &targets, &SnapProbe::points(&grid), &mut state);
            match scan(&centres, &grid, settings().capture) {
                None => assert!(result.is_empty(), "frame {k}: nothing within capture"),
                Some((target, probe)) => {
                    let moved = grid[probe].translated(result.offset);
                    assert!(
                        moved.vector_to(centres[target]).length() < 1e-9,
                        "frame {k}: grid centre {probe} lands on centre {target}"
                    );
                    assert!(result.marker.cross.is_some(), "the 0060 marker shows");
                }
            }
        }
    }

    #[test]
    fn a_held_centre_keeps_its_grip_until_the_release_zone() {
        let document = Document::new(1);
        let _ = document.create_ellipse(EllipseFrame {
            center: Point::new(50.0, 50.0),
            rx: Length::from_mm(2.0),
            ry: Length::from_mm(2.0),
        });
        let targets =
            SnapTargets::gather_centres(&document.read_nodes(), true, SnapExclusions::default());
        let mut state = SnapState::default();
        let at = |x: f64| SnapProbe::points(&[Point::new(x, 50.0)]);
        assert!(!resolve(&settings(), &targets, &at(51.5), &mut state).is_empty());
        assert!(
            !resolve(&settings(), &targets, &at(52.5), &mut state).is_empty(),
            "within release (3 mm) it holds"
        );
        assert!(resolve(&settings(), &targets, &at(53.5), &mut state).is_empty());
        assert!(
            resolve(
                &settings().suspended(),
                &targets,
                &at(50.5),
                &mut SnapState::default()
            )
            .is_empty(),
            "Alt suspends it"
        );
    }
}
