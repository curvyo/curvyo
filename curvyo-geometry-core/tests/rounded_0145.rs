//! `specs/0145-shape-spiro-handles` criterion 10: a looped polygon on a laser page is flattened
//! at the job tolerance of `0029` (0.0095 mm) and stays within it of the Bezier outline.
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{
    Angle, Curve, Length, Point, PointCount, Rounded, Shape, StarFrame, Tolerance, outline_of,
};
use curvyo_geometry_core::flatten_outline;
use kurbo::{CubicBez, ParamCurveNearest};

const JOB_TOLERANCE_MM: f64 = 0.0095;

#[test]
fn a_1024_gon_flattened_for_a_job_stays_within_the_tolerance_of_its_outline() {
    for q in [-10.0, -1.0, 2.5, 10.0] {
        let shape = Shape::Polygon {
            randomized: curvyo_document_core::Randomized::ZERO,
            frame: StarFrame {
                center: Point::new(100.0, 100.0),
                radius: Length::from_mm(60.0),
                angle: Angle::from_radians(0.0),
            },
            point_count: PointCount::new(1024).unwrap(),
            curve: Curve::POLYGON_DEFAULT,
            rounded: Rounded::new(q).unwrap(),
        };
        let outline = outline_of(&shape);
        let triples: Vec<_> = outline
            .anchors
            .iter()
            .map(|a| (a.point, a.handle_in, a.handle_out))
            .collect();
        let polyline = flatten_outline(&triples, true, Tolerance::from_mm(JOB_TOLERANCE_MM));
        assert!(polyline.len() >= triples.len(), "q {q}");
        let n = triples.len();
        let cubics: Vec<CubicBez> = (0..n)
            .map(|i| {
                let (a, b) = (outline.anchors[i], outline.anchors[(i + 1) % n]);
                let at = |p: Point| kurbo::Point::new(p.x, p.y);
                CubicBez::new(
                    at(a.point),
                    at(a.point.translated(a.handle_out)),
                    at(b.point.translated(b.handle_in)),
                    at(b.point),
                )
            })
            .collect();
        let step = (polyline.len() / 60).max(1);
        for i in (0..polyline.len()).step_by(step) {
            let (a, b) = (polyline[i], polyline[(i + 1) % polyline.len()]);
            let mid = kurbo::Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y));
            let nearest = cubics
                .iter()
                .map(|c| c.nearest(mid, 1e-6).distance_sq.sqrt())
                .fold(f64::INFINITY, f64::min);
            assert!(
                nearest <= JOB_TOLERANCE_MM + 1e-6,
                "q {q}: the chord {i} is {nearest} mm from the outline"
            );
        }
    }
}
