//! Centre lines of strokes drawn from known centre lines (`specs/0117-centre-lines` criteria 5
//! to 13): position, width, ends, junctions, branch starts, blobs and inner areas.

use curvyo_document_core::{Length, Point as DocPoint, Tolerance};
use kurbo::Point;

use super::{CentreLines, CentreOptions, CentrePath, CentreRole, CentreSkeleton};
use crate::medial_axis::tests::{Stroke, Truth, arc, union};
use crate::{PathMeasure, signed_area_mm2};

fn tolerance(width: f64) -> f64 {
    0.05_f64.max(0.03 * width)
}

fn lines(strokes: &[Stroke], options: &CentreOptions) -> CentreLines {
    CentreSkeleton::from_regions(union(strokes)).finish(options, &[])
}

/// The point and width at 20 positions along a path.
fn samples(path: &CentrePath) -> Vec<(Point, f64)> {
    let measure = PathMeasure::new(&path.anchors, path.closed);
    let total = measure.length().as_mm();
    (0..20)
        .map(|i| {
            let at = (f64::from(i) + 0.5) / 20.0;
            let p = measure.at(Length::from_mm(at * total)).point;
            (
                Point::new(p.x, p.y),
                path.width.as_mm() * path.profile.sample(at).unwrap(),
            )
        })
        .collect()
}

fn nearest(truth: &[Truth], point: Point) -> Truth {
    *truth
        .iter()
        .min_by(|a, b| {
            (a.point - point)
                .hypot2()
                .total_cmp(&(b.point - point).hypot2())
        })
        .unwrap()
}

/// Every one of 20 samples lies within `T(w)` of the truth and has its width within `T(w)`.
fn assert_follows(path: &CentrePath, truth: &[Truth], name: &str) {
    for (point, width) in samples(path) {
        let t = nearest(truth, point);
        let budget = tolerance(t.width);
        assert!(
            (t.point - point).hypot() <= budget,
            "{name}: off axis at {point:?}"
        );
        assert!(
            (t.width - width).abs() <= budget,
            "{name}: width {width} vs {}",
            t.width
        );
    }
}

/// The painted area of a path, as `0085` paints it.
fn painted_area(path: &CentrePath) -> f64 {
    let style = crate::StrokeShape {
        width: path.width,
        join: curvyo_document_core::LineJoin::Round,
        cap: curvyo_document_core::LineCap::Round,
        dash: curvyo_document_core::DashPattern::default(),
        markers: curvyo_document_core::Markers::default(),
    };
    let outline = crate::variable_stroke_outline(
        crate::Outline::new(&path.anchors, path.closed),
        &style,
        &path.profile,
        Tolerance::from_mm(0.01),
    )
    .unwrap();
    outline
        .outlines()
        .iter()
        .map(|ring| {
            let points: Vec<DocPoint> = ring.iter().map(|a| a.point).collect();
            signed_area_mm2(&points)
        })
        .sum()
}

fn subject_area(stroke: &Stroke) -> f64 {
    stroke
        .paths()
        .iter()
        .map(|ring| {
            let points: Vec<DocPoint> = ring
                .iter()
                .map(crate::boolean_grid::to_millimetres)
                .collect();
            signed_area_mm2(&points)
        })
        .sum::<f64>()
        .abs()
}

fn single(stroke: &Stroke, name: &str) {
    let result = lines(std::slice::from_ref(stroke), &CentreOptions::default());
    assert_eq!(result.paths.len(), 1, "{name}");
    let path = &result.paths[0];
    assert_eq!(path.role, CentreRole::Main);
    assert_follows(path, &stroke.truth(), name);
    let (painted, subject) = (painted_area(path).abs(), subject_area(stroke));
    eprintln!(
        "{name}: area {painted:.3} vs {subject:.3}, {} anchors",
        path.anchors.len()
    );
    assert!((painted - subject).abs() <= 0.02 * subject, "{name}: area");
}

#[test]
fn criterion_5_constant_strokes_round_trip() {
    single(
        &Stroke::line(Point::new(0.0, 0.0), Point::new(40.0, 0.0), 2.0),
        "straight",
    );
    single(
        &Stroke {
            anchors: arc(Point::new(0.0, 0.0), 15.0, 0.0, 2.0),
            closed: false,
            width: 2.0,
            profile: vec![(0.0, 1.0), (1.0, 1.0)],
        },
        "arc",
    );
}

/// A linear taper from 3 mm to 0.5 mm over 30 mm: two discs and their outer tangents, so the
/// true width falls linearly along the axis.
fn linear_taper() -> (Vec<crate::medial_axis::Region>, Vec<Truth>) {
    use crate::boolean_grid::snap_polygon;
    let (r1, r2, length) = (1.5_f64, 0.25_f64, 30.0_f64);
    let s = (r1 - r2) / length;
    let c = (1.0 - s * s).sqrt();
    // The discs are flattened as the production outlines are.
    let tolerance = crate::medial_axis::region::FLATTEN_MM - crate::boolean_grid::GRID_MM;
    let disc = |x: f64, r: f64| {
        let sides = (std::f64::consts::PI / (1.0 - tolerance / r).acos()).ceil();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let points: Vec<DocPoint> = (0..sides as u32)
            .map(|i| {
                let a = f64::from(i) * 2.0 * std::f64::consts::PI / sides;
                DocPoint::new(x + r * a.cos(), r * a.sin())
            })
            .collect();
        snap_polygon(&points)
    };
    let band = snap_polygon(&[
        DocPoint::new(r1 * s, -r1 * c),
        DocPoint::new(length + r2 * s, -r2 * c),
        DocPoint::new(length + r2 * s, r2 * c),
        DocPoint::new(r1 * s, r1 * c),
    ]);
    let regions =
        crate::medial_axis::region::regions_of_paths(&[disc(0.0, r1), disc(length, r2), band]);
    let truth = (0..=3000)
        .map(|i| {
            let x = f64::from(i) / 100.0;
            Truth {
                point: Point::new(x, 0.0),
                width: 2.0 * (r1 - s * x),
            }
        })
        .collect();
    (regions, truth)
}

#[test]
fn criterion_6_a_taper_keeps_its_profile_with_few_points() {
    let (regions, truth) = linear_taper();
    let result = CentreSkeleton::from_regions(regions).finish(&CentreOptions::default(), &[]);
    assert_eq!(result.paths.len(), 1);
    let path = &result.paths[0];
    assert_follows(path, &truth, "taper");
    let points = path.profile.positions().count();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let limit = (0.0962 * 2.5 / (tolerance(0.5) / 2.0)).ceil() as usize + 2;
    eprintln!("taper: {points} profile points, limit {limit}");
    assert!(points <= limit);
}

#[test]
fn criterion_7_a_ring_is_one_closed_path_without_seam_step() {
    let mut anchors = arc(Point::new(0.0, 0.0), 5.0, 0.0, 2.0 * std::f64::consts::PI);
    anchors.pop();
    let stroke = Stroke {
        anchors,
        closed: true,
        width: 2.0,
        profile: vec![(0.0, 1.0)],
    };
    let result = lines(std::slice::from_ref(&stroke), &CentreOptions::default());
    assert_eq!(result.paths.len(), 1);
    let path = &result.paths[0];
    assert!(path.closed && path.profile.is_closed());
    assert_follows(path, &stroke.truth(), "ring");
    let step = (path.profile.sample(0.0).unwrap() - path.profile.sample(0.999_999).unwrap()).abs()
        * path.width.as_mm();
    assert!(step <= tolerance(2.0));
}

#[test]
fn criterion_8_a_rounded_main_end_stops_half_a_width_before_the_tip() {
    let result = lines(
        &[Stroke::line(
            Point::new(0.0, 0.0),
            Point::new(40.0, 0.0),
            2.0,
        )],
        &CentreOptions::default(),
    );
    let anchors = &result.paths[0].anchors;
    let (a, b) = (anchors[0].0, anchors[anchors.len() - 1].0);
    let (lo, hi) = if a.x < b.x { (a, b) } else { (b, a) };
    assert!(lo.x.abs() <= tolerance(2.0) && lo.y.abs() <= tolerance(2.0));
    assert!((hi.x - 40.0).abs() <= tolerance(2.0) && hi.y.abs() <= tolerance(2.0));
}

fn t_shape() -> Vec<Stroke> {
    vec![
        Stroke::line(Point::new(-20.0, 0.0), Point::new(20.0, 0.0), 2.0),
        Stroke::line(Point::new(0.0, 0.0), Point::new(0.0, 20.0), 2.0),
    ]
}

#[test]
fn criterion_9_the_bar_of_a_t_is_the_main_path() {
    let result = lines(&t_shape(), &CentreOptions::default());
    assert_eq!(result.paths.len(), 2);
    let main = &result.paths[0];
    assert_eq!(main.role, CentreRole::Main);
    let bar = Stroke::line(Point::new(-20.0, 0.0), Point::new(20.0, 0.0), 2.0).truth();
    assert_follows(main, &bar, "bar");
    assert_eq!(result.paths[1].role, CentreRole::Branch);
}

#[test]
fn criterion_9_a_y_continues_straight_into_the_wider_arm() {
    let strokes = vec![
        Stroke::line(Point::new(0.0, 0.0), Point::new(20.0, 0.0), 2.0),
        Stroke::line(Point::new(20.0, 0.0), Point::new(40.0, 0.0), 2.0),
        Stroke::line(
            Point::new(20.0, 0.0),
            Point::new(20.0 + 15.0 * 0.5, 15.0 * 0.866_025_403_784_438_6),
            1.0,
        ),
    ];
    let result = lines(&strokes, &CentreOptions::default());
    let main = &result.paths[0];
    let straight = Stroke::line(Point::new(0.0, 0.0), Point::new(40.0, 0.0), 2.0).truth();
    assert_follows(main, &straight, "y main");
}

#[test]
fn criterion_10_a_branch_starts_at_the_overlap() {
    for (overlap, expected) in [(0.2, 0.6), (0.0, 1.0), (-0.2, 1.4)] {
        let options = CentreOptions {
            overlap,
            ..CentreOptions::default()
        };
        let result = lines(&t_shape(), &options);
        let branch = result
            .paths
            .iter()
            .find(|p| p.role == CentreRole::Branch)
            .unwrap();
        let (a, b) = (
            branch.anchors[0].0,
            branch.anchors[branch.anchors.len() - 1].0,
        );
        let start = if a.y < b.y { a } else { b };
        let at = if a.y < b.y { 0.0 } else { 1.0 };
        let width = branch.width.as_mm() * branch.profile.sample(at).unwrap();
        eprintln!("p {overlap}: start {start:?}, width {width}");
        assert!((start.y - expected).abs() <= 0.01, "p {overlap}: {start:?}");
        assert!(
            (width - 2.0).abs() <= tolerance(2.0),
            "p {overlap}: width {width}"
        );
        let tip = if a.y < b.y { b } else { a };
        let tip_width = branch.width.as_mm() * branch.profile.sample(1.0 - at).unwrap();
        assert!((tip.y - 21.0).abs() <= tolerance(2.0) && tip_width <= tolerance(2.0));
    }
}

#[test]
fn criterion_12_a_disc_stays_filled_even_at_the_maximum() {
    let disc = Stroke::line(Point::new(0.0, 0.0), Point::new(0.001, 0.0), 6.0);
    let options = CentreOptions {
        thick_parts: None,
        ..CentreOptions::default()
    };
    let result = lines(&[disc], &options);
    assert!(result.all_filled());
    assert_eq!(result.filled.len(), 1);
}

/// The customer's whale eye: an outline ring with a filled iris that touches it, the iris with a
/// highlight hole. The region as a whole is no disc (the ring spans it), but at Thick parts 3 mm
/// the iris (4.4 mm with its highlight filled) stays filled while the ring gets its centre line
/// (CENTRE-BLOBS A, coordinator 2026-11-16; Thick parts in mm, customer 2026-11-18).
#[test]
fn criterion_12_an_iris_in_an_eye_ring_stays_filled() {
    use crate::boolean_grid::snap_polygon;
    let circle = |cx: f64, cy: f64, r: f64, reverse: bool| {
        let mut points: Vec<DocPoint> = (0..96)
            .map(|i| {
                let a = f64::from(i) * std::f64::consts::PI / 48.0;
                DocPoint::new(cx + r * a.cos(), cy + r * a.sin())
            })
            .collect();
        if reverse {
            points.reverse();
        }
        snap_polygon(&points)
    };
    let mut anchors = arc(Point::new(50.0, 30.0), 5.0, 0.0, 2.0 * std::f64::consts::PI);
    anchors.pop();
    let ring = Stroke {
        anchors,
        closed: true,
        width: 1.3,
        profile: vec![(0.0, 1.0)],
    };
    let mut paths = ring.paths();
    paths.push(circle(50.0, 27.6, 2.2, false));
    paths.push(circle(50.7, 28.3, 0.7, true));
    let regions = crate::medial_axis::region::regions_of_paths(&paths);
    assert_eq!(regions.len(), 1);
    let options = CentreOptions {
        thick_parts: Some(Length::from_mm(3.0)),
        ..CentreOptions::default()
    };
    let result = CentreSkeleton::from_regions(regions).finish(&options, &[]);
    assert_eq!(result.filled.len(), 1, "the iris stays filled");
    let iris: Vec<DocPoint> = result.filled[0][0].iter().map(|a| a.0).collect();
    let area = signed_area_mm2(&iris).abs();
    assert!(
        area > 0.8 * std::f64::consts::PI * 2.2 * 2.2,
        "filled {area} mm2"
    );
    assert!(!result.paths.is_empty(), "the ring has a centre line");
    for path in &result.paths {
        assert!(path.width.as_mm() < 2.0, "{} mm wide", path.width.as_mm());
    }
}

#[test]
fn criterion_13_inner_areas_move_the_hole_inward() {
    let mut anchors = arc(Point::new(0.0, 0.0), 5.0, 0.0, 2.0 * std::f64::consts::PI);
    anchors.pop();
    let ring = Stroke {
        anchors,
        closed: true,
        width: 2.0,
        profile: vec![(0.0, 1.0)],
    };
    let options = CentreOptions {
        inner_offset: Some(Length::from_mm(0.3)),
        ..CentreOptions::default()
    };
    let result = lines(&[ring], &options);
    assert_eq!(result.inner_areas.len(), 1);
    for p in &result.inner_areas[0] {
        let r = p.x.hypot(p.y);
        assert!((r - 3.7).abs() <= 0.012, "{r}");
    }
    let off = lines(
        &[Stroke::line(
            Point::new(0.0, 0.0),
            Point::new(40.0, 0.0),
            2.0,
        )],
        &CentreOptions::default(),
    );
    assert_eq!(off.inner_areas.len(), 0);
}

#[test]
fn the_same_input_gives_the_same_result() {
    let a = lines(&t_shape(), &CentreOptions::default());
    let b = lines(&t_shape(), &CentreOptions::default());
    assert_eq!(format!("{:?}", a.paths), format!("{:?}", b.paths));
}

#[test]
fn criterion_5_a_noisy_bar_gives_one_path_near_its_centre() {
    let skeleton = CentreSkeleton::from_regions(vec![crate::medial_axis::tests::noisy_bar()]);
    let result = skeleton.finish(&CentreOptions::default(), &[]);
    assert_eq!(result.paths.len(), 1);
    let truth = Stroke::line(Point::new(0.0, 0.0), Point::new(40.0, 0.0), 2.0).truth();
    // The centre within T(w); the width is the inscribed one, below the jaggies' mean by up to
    // their depth (0.05 mm a side), so it is checked against that band.
    for (point, width) in samples(&result.paths[0]) {
        let t = nearest(&truth, point);
        assert!(
            (t.point - point).hypot() <= tolerance(2.0),
            "off axis at {point:?}"
        );
        assert!(
            (1.9 - tolerance(2.0)..=2.0 + tolerance(2.0)).contains(&width),
            "width {width}"
        );
    }
}

/// The traced whale's tail has two holes whose junctions merge, so both loops around the holes
/// leave the merged junction and come back to it; they were dropped as inner junction edges and
/// the tail stayed filled (criterion 11). Only the pupil is a disc.
#[test]
fn criterion_11_loops_back_to_one_merged_junction_are_kept() {
    let text = include_str!("../../tests/fixtures/centre_lines/whale_traced.txt");
    let mut outlines: Vec<(Vec<crate::OutlineTriple>, bool)> = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first() {
            Some(&"subject") | None => {}
            Some(&"outline") => outlines.push((Vec::new(), words[1] == "true")),
            Some(_) => {
                let v: Vec<f64> = words.iter().map(|w| w.parse().unwrap()).collect();
                outlines.last_mut().unwrap().0.push((
                    DocPoint::new(v[0], v[1]),
                    curvyo_document_core::Vec2::new(v[2], v[3]),
                    curvyo_document_core::Vec2::new(v[4], v[5]),
                ));
            }
        }
    }
    let outlines: Vec<crate::Outline<'_>> = outlines
        .iter()
        .map(|(a, c)| crate::Outline::new(a, *c))
        .collect();
    // Without a Thick parts limit: at 6 mm the 7.9 mm spout stays filled too.
    let result = CentreSkeleton::build(&outlines).unwrap().finish(
        &CentreOptions {
            thick_parts: None,
            ..CentreOptions::default()
        },
        &[],
    );
    assert_eq!(result.filled.len(), 1, "only the pupil stays filled");
    let tail = result
        .paths
        .iter()
        .filter(|p| p.anchors.iter().all(|a| a.0.x > 100.0 && a.0.y < 130.0))
        .count();
    assert!(tail >= 1, "no centre path in the tail");
}

/// `strokes` rotated by `degrees`, scaled by `scale` and moved off the origin.
fn transformed(strokes: Vec<Stroke>, degrees: f64, scale: f64) -> Vec<Stroke> {
    let (s, c) = degrees.to_radians().sin_cos();
    let map = |x: f64, y: f64| ((c * x - s * y) * scale, (s * x + c * y) * scale);
    strokes
        .into_iter()
        .map(|stroke| Stroke {
            anchors: stroke
                .anchors
                .iter()
                .map(|(p, i, o)| {
                    let ((px, py), (ix, iy), (ox, oy)) =
                        (map(p.x, p.y), map(i.x, i.y), map(o.x, o.y));
                    (
                        DocPoint::new(px + 3.0, py - 7.0),
                        curvyo_document_core::Vec2::new(ix, iy),
                        curvyo_document_core::Vec2::new(ox, oy),
                    )
                })
                .collect(),
            closed: stroke.closed,
            width: stroke.width * scale,
            profile: stroke.profile,
        })
        .collect()
}

fn l_shape() -> Vec<Stroke> {
    vec![
        Stroke::line(Point::new(0.0, 0.0), Point::new(20.0, 0.0), 2.0),
        Stroke::line(Point::new(0.0, 0.0), Point::new(0.0, 15.0), 2.0),
    ]
}

const ANGLES: [f64; 5] = [0.0, 17.0, 45.0, 90.0, 133.0];

/// The options without a Thick parts limit: the scaled strokes reach 8 mm, and these tests are
/// about rotation and scale, not about what stays filled.
fn no_thick_limit() -> CentreOptions {
    CentreOptions {
        thick_parts: None,
        ..CentreOptions::default()
    }
}

/// ADR 0063 §12: a rotated and scaled T keeps its main path and branch, and the main path
/// follows the transformed bar within `T(w)` in position and width.
#[test]
fn rotation_and_scale_keep_the_t() {
    for degrees in ANGLES {
        for scale in [0.5, 1.0, 2.5, 4.0] {
            let strokes = transformed(t_shape(), degrees, scale);
            let result = lines(&strokes, &no_thick_limit());
            let name = format!("T at {degrees} degrees, x{scale}");
            let roles: Vec<CentreRole> = result.paths.iter().map(|p| p.role).collect();
            assert_eq!(roles, [CentreRole::Main, CentreRole::Branch], "{name}");
            assert_follows(&result.paths[0], &strokes[0].truth(), &name);
        }
    }
}

/// An L up to 2 mm wide is one path at any angle. Wider Ls must not fail (a dropped short
/// chain once left a stale parent index behind and the finish panicked).
#[test]
fn rotation_and_scale_keep_a_narrow_l() {
    for degrees in ANGLES {
        for scale in [0.5, 1.0, 2.5, 4.0] {
            let result = lines(&transformed(l_shape(), degrees, scale), &no_thick_limit());
            let name = format!("L at {degrees} degrees, x{scale}");
            if scale <= 1.0 {
                assert_eq!(result.paths.len(), 1, "{name}");
            } else {
                assert!(!result.paths.is_empty(), "{name}");
            }
        }
    }
}

/// The round outer corner of a thick L triangulates into a tree of junction triangles near its
/// arc; it is pruned as a whole, so a 5 and an 8 mm L stay one path (coordinator 2026-11-16:
/// thick strokes at corners are the customer's use case).
#[test]
fn rotation_and_scale_keep_a_wide_l() {
    for degrees in ANGLES {
        for scale in [2.5, 4.0] {
            let result = lines(&transformed(l_shape(), degrees, scale), &no_thick_limit());
            assert_eq!(result.paths.len(), 1, "L at {degrees} degrees, x{scale}");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(24))]

    /// ADR 0063 §12: a T rotated by any angle, scaled from 0.5 to 4 and moved keeps its main
    /// path and branch, and the main path follows the transformed bar within `T(w)`.
    #[test]
    fn a_t_under_rotation_scale_and_translation(
        degrees in 0.0_f64..360.0,
        scale in 0.5_f64..4.0,
        dx in -500.0_f64..500.0,
        dy in -500.0_f64..500.0,
    ) {
        let strokes: Vec<Stroke> = transformed(t_shape(), degrees, scale)
            .into_iter()
            .map(|mut stroke| {
                for anchor in &mut stroke.anchors {
                    anchor.0 = DocPoint::new(anchor.0.x + dx, anchor.0.y + dy);
                }
                stroke
            })
            .collect();
        let result = lines(&strokes, &no_thick_limit());
        let roles: Vec<CentreRole> = result.paths.iter().map(|p| p.role).collect();
        proptest::prop_assert_eq!(roles, vec![CentreRole::Main, CentreRole::Branch]);
        assert_follows(&result.paths[0], &strokes[0].truth(), "T");
    }
}

/// ADR 0063 §9: permuting a compound subject's subpaths and their start anchors gives a
/// bit-identical result.
#[test]
fn permuted_subpaths_give_the_same_result() {
    let ring = |cx: f64, r: f64, reverse: bool, start: usize| {
        let mut anchors = arc(Point::new(cx, 0.0), r, 0.0, 2.0 * std::f64::consts::PI);
        anchors.pop();
        if reverse {
            anchors.reverse();
            for a in &mut anchors {
                std::mem::swap(&mut a.1, &mut a.2);
            }
        }
        let count = anchors.len();
        anchors.rotate_left(start % count);
        anchors
    };
    let run = |order: &[(f64, f64, bool, usize)]| {
        let rings: Vec<Vec<crate::OutlineTriple>> = order
            .iter()
            .map(|&(cx, r, reverse, start)| ring(cx, r, reverse, start))
            .collect();
        let outlines: Vec<crate::Outline<'_>> =
            rings.iter().map(|r| crate::Outline::new(r, true)).collect();
        format!(
            "{:?}",
            CentreSkeleton::build(&outlines)
                .unwrap()
                .finish(&CentreOptions::default(), &[])
                .paths
        )
    };
    let a = run(&[
        (0.0, 6.0, false, 0),
        (0.0, 4.5, true, 0),
        (30.0, 6.0, false, 0),
        (30.0, 4.5, true, 0),
    ]);
    let b = run(&[
        (30.0, 4.5, true, 3),
        (0.0, 4.5, true, 5),
        (30.0, 6.0, false, 2),
        (0.0, 6.0, false, 7),
    ]);
    assert_eq!(a, b);
}

/// Optimise (amendments 2026-11-16 and 2026-11-18, D): nodes plus width points never grow as
/// the setting grows, in 5 % steps, on the traced whale, the customer's dolphin and the
/// synthetic whale.
#[test]
fn optimise_never_adds_nodes() {
    let traced = |text: &str| -> Vec<CentreSkeleton> {
        super::measure_tests::traced_subjects(text)
            .iter()
            .map(|subject| {
                let outlines: Vec<crate::Outline<'_>> = subject
                    .iter()
                    .map(|(a, c)| crate::Outline::new(a, *c))
                    .collect();
                CentreSkeleton::build(&outlines).unwrap()
            })
            .collect()
    };
    let fixtures = [
        (
            "whale",
            traced(include_str!(
                "../../tests/fixtures/centre_lines/whale_traced.txt"
            )),
        ),
        (
            "dolphin",
            traced(include_str!(
                "../../tests/fixtures/centre_lines/dolphin_traced.txt"
            )),
        ),
        (
            "synthetic whale",
            vec![CentreSkeleton::from_regions(union(
                &crate::medial_axis::tests::synthetic_whale(),
            ))],
        ),
    ];
    for (name, skeletons) in &fixtures {
        let mut last = usize::MAX;
        let mut totals = Vec::new();
        for step in 0..=20 {
            let optimise = f64::from(step) * 0.05;
            let options = CentreOptions {
                optimise,
                ..CentreOptions::default()
            };
            let (mut anchors, mut points) = (0, 0);
            for skeleton in skeletons {
                let result = skeleton.finish(&options, &[]);
                anchors += result.paths.iter().map(|p| p.anchors.len()).sum::<usize>();
                points += result
                    .paths
                    .iter()
                    .map(|p| p.profile.positions().count())
                    .sum::<usize>();
            }
            assert!(
                anchors + points <= last,
                "{name} at {optimise:.2}: {anchors} + {points} > {last}"
            );
            last = anchors + points;
            totals.push(anchors + points);
        }
        // Below the default the fit follows the shape closer, step by step (the slider is
        // never dead between 0 and 50 %).
        assert!(
            totals[..=10].windows(2).filter(|w| w[0] > w[1]).count() >= 5,
            "{name}: Optimise 0 to 50 % changes too little: {totals:?}"
        );
    }
}

/// Optimise keeps the result within its budget at every setting: on a 2 mm arc the centre and
/// the width stay within `(share + 1/2) T(w)` (the fit's share plus the measurement's half).
#[test]
fn optimise_bounds_the_error_at_every_setting() {
    let stroke = Stroke {
        anchors: arc(Point::new(0.0, 0.0), 15.0, 0.0, 2.0),
        closed: false,
        width: 2.0,
        profile: vec![(0.0, 1.0), (1.0, 1.0)],
    };
    let truth = stroke.truth();
    for optimise in [0.0, 0.5, 0.75, 1.0] {
        let share = super::fit::Budget::of(optimise).share;
        let result = lines(
            std::slice::from_ref(&stroke),
            &CentreOptions {
                optimise,
                ..CentreOptions::default()
            },
        );
        assert_eq!(result.paths.len(), 1);
        let budget = (share + 0.5) * tolerance(2.0);
        for (point, width) in samples(&result.paths[0]) {
            let t = nearest(&truth, point);
            assert!(
                (t.point - point).hypot() <= budget,
                "optimise {optimise}: off axis"
            );
            assert!(
                (t.width - width).abs() <= budget,
                "optimise {optimise}: width {width}"
            );
        }
    }
}

/// CENTRE-DEFAULT A (customer 2026-11-17): corners keep their detail at the default and at
/// 100 %: an L and a bent polyline keep an anchor within `T(w)` of each corner, and a T's bar
/// stays straight with its stem a branch.
#[test]
fn corners_keep_their_anchors_at_every_setting() {
    use crate::medial_axis::tests::corner;
    let bent = || Stroke {
        anchors: vec![corner(0.0, 0.0), corner(20.0, 0.0), corner(30.0, 15.0)],
        closed: false,
        width: 2.0,
        profile: vec![(0.0, 1.0), (1.0, 1.0)],
    };
    for optimise in [DEFAULT_OPTIMISE_FOR_TESTS, 1.0] {
        let options = CentreOptions {
            optimise,
            ..CentreOptions::default()
        };
        for (name, strokes, corner_at) in [
            ("L", l_shape(), Point::new(0.0, 0.0)),
            ("bent", vec![bent()], Point::new(20.0, 0.0)),
        ] {
            let result = lines(&strokes, &options);
            assert_eq!(result.paths.len(), 1, "{name} at {optimise}");
            let nearest_anchor = result.paths[0]
                .anchors
                .iter()
                .map(|a| (Point::new(a.0.x, a.0.y) - corner_at).hypot())
                .fold(f64::INFINITY, f64::min);
            assert!(
                nearest_anchor <= tolerance(2.0),
                "{name} at {optimise}: the nearest anchor is {nearest_anchor:.3} from the corner"
            );
        }
        let tee = lines(&t_shape(), &options);
        let roles: Vec<CentreRole> = tee.paths.iter().map(|p| p.role).collect();
        assert_eq!(
            roles,
            [CentreRole::Main, CentreRole::Branch],
            "T at {optimise}"
        );
        for (point, _) in samples(&tee.paths[0]) {
            assert!(
                point.y.abs() <= tolerance(2.0),
                "T at {optimise}: bar off at {point:?}"
            );
        }
    }
}

const DEFAULT_OPTIMISE_FOR_TESTS: f64 = crate::DEFAULT_OPTIMISE;

/// One closed outline of corner anchors.
fn polygon(points: &[(f64, f64)]) -> Vec<crate::OutlineTriple> {
    points
        .iter()
        .map(|&(x, y)| {
            (
                DocPoint::new(x, y),
                curvyo_document_core::Vec2::ZERO,
                curvyo_document_core::Vec2::ZERO,
            )
        })
        .collect()
}

/// A 30 x 10 mm bar with a 20 x 1.5 mm stroke leaving its right side, as one outline.
fn bar_with_stroke() -> Vec<crate::OutlineTriple> {
    polygon(&[
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 4.25),
        (50.0, 4.25),
        (50.0, 5.75),
        (30.0, 5.75),
        (30.0, 10.0),
        (0.0, 10.0),
    ])
}

fn finish_outline(anchors: &[crate::OutlineTriple], options: &CentreOptions) -> CentreLines {
    CentreSkeleton::build(&[crate::Outline::new(anchors, true)])
        .unwrap()
        .finish(options, &[])
}

/// Criterion 12 (Thick parts in mm, customer 2026-11-18): at the default 6 mm the 10 mm bar
/// stays filled with its own nodes, split only where the cut meets the outline, and the stroke
/// gets its centre line from the cut on.
#[test]
fn criterion_12_a_thick_part_stays_filled_with_its_own_nodes() {
    let outline = bar_with_stroke();
    let result = finish_outline(&outline, &CentreOptions::default());
    assert_eq!(result.filled.len(), 1, "the bar stays filled");
    assert_eq!(result.filled[0].len(), 1, "one ring");
    let ring = &result.filled[0][0];
    for &(x, y) in &[(0.0, 0.0), (30.0, 0.0), (30.0, 10.0), (0.0, 10.0)] {
        assert!(
            ring.iter()
                .any(|a| (a.0.x - x).abs() < 1e-6 && (a.0.y - y).abs() < 1e-6),
            "the bar's node ({x}, {y}) is kept: {ring:?}"
        );
    }
    assert!(
        ring.len() <= 8,
        "the bar's own nodes and the two ends of the cut: {ring:?}"
    );
    assert!(
        ring.iter().all(|a| a.0.x <= 31.0),
        "the stroke is cut off: {ring:?}"
    );
    assert_eq!(result.paths.len(), 1, "the stroke: {:?}", result.paths);
    let stroke = &result.paths[0];
    assert!(
        (stroke.width.as_mm() - 1.5).abs() <= tolerance(1.5),
        "{}",
        stroke.width.as_mm()
    );
    let xs: Vec<f64> = stroke.anchors.iter().map(|a| a.0.x).collect();
    let start = xs.iter().copied().fold(f64::INFINITY, f64::min);
    assert!(
        (30.0..34.0).contains(&start),
        "the stroke starts at the cut: {xs:?}"
    );
    // The kept bar and the painted stroke cover the subject within criterion 5's 4 %.
    let kept: Vec<Vec<crate::OutlineTriple>> = result.filled[0].clone();
    let style = crate::StrokeShape {
        width: stroke.width,
        join: curvyo_document_core::LineJoin::Round,
        cap: curvyo_document_core::LineCap::Round,
        dash: curvyo_document_core::DashPattern::default(),
        markers: curvyo_document_core::Markers::default(),
    };
    let painted = crate::variable_stroke_outline(
        crate::Outline::new(&stroke.anchors, stroke.closed),
        &style,
        &stroke.profile,
        Tolerance::from_mm(0.01),
    )
    .unwrap();
    let painted: Vec<Vec<crate::OutlineTriple>> = painted
        .outlines()
        .iter()
        .map(|r| {
            r.iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect()
        })
        .collect();
    let all: Vec<crate::Outline<'_>> = kept
        .iter()
        .chain(&painted)
        .map(|r| crate::Outline::new(r, true))
        .collect();
    let union = crate::boolean(crate::BooleanOp::Union, &[&all], Tolerance::from_mm(0.01)).unwrap();
    let area: f64 = union.outlines().iter().map(|r| signed_area_mm2(r)).sum();
    let subject = 30.0 * 10.0 + 20.0 * 1.5;
    assert!(
        (area - subject).abs() <= 0.04 * subject,
        "covered {area:.2} of {subject} mm2"
    );
}

/// At the maximum, the bar is centre-lined too: only a disc stays filled.
#[test]
fn criterion_12_at_the_maximum_a_bar_gets_centre_lines() {
    let result = finish_outline(
        &bar_with_stroke(),
        &CentreOptions {
            thick_parts: None,
            ..CentreOptions::default()
        },
    );
    assert!(result.filled.is_empty(), "nothing kept filled");
    assert!(!result.paths.is_empty(), "centre lines");
}

/// A stroke drawn exactly as wide as Thick parts gets one centre line, not pieces.
#[test]
fn criterion_12_a_stroke_as_wide_as_the_limit_is_one_path() {
    let bar = Stroke::line(Point::new(0.0, 0.0), Point::new(40.0, 0.0), 6.0);
    let result = lines(&[bar], &CentreOptions::default());
    assert!(result.filled.is_empty(), "nothing kept filled");
    assert_eq!(result.paths.len(), 1);
}

/// An eye blob with a highlight: no ring around the highlight, the blob stays filled at every
/// setting (the disc test counts the small hole as filled).
#[test]
fn criterion_12_an_eye_with_a_highlight_stays_filled() {
    let circle = |cx: f64, cy: f64, r: f64, reverse: bool| {
        let mut points: Vec<(f64, f64)> = (0..64)
            .map(|i| {
                let a = f64::from(i) * std::f64::consts::PI / 32.0;
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect();
        if reverse {
            points.reverse();
        }
        polygon(&points)
    };
    let (eye, highlight) = (circle(10.0, 10.0, 2.0, false), circle(10.6, 9.4, 0.5, true));
    for thick_parts in [Some(Length::from_mm(6.0)), None] {
        let result = CentreSkeleton::build(&[
            crate::Outline::new(&eye, true),
            crate::Outline::new(&highlight, true),
        ])
        .unwrap()
        .finish(
            &CentreOptions {
                thick_parts,
                ..CentreOptions::default()
            },
            &[],
        );
        assert!(result.paths.is_empty(), "{thick_parts:?}: no centre line");
        assert_eq!(result.filled.len(), 1);
        assert_eq!(result.filled[0].len(), 2, "the highlight stays a hole");
        assert_eq!(result.filled[0][0].len(), 64, "the eye keeps its own nodes");
    }
}
