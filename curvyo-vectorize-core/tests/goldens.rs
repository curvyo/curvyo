//! The tracer's golden fixtures (`specs/0025-raster-trace` criteria 11, 18 to 24): the pictures in
//! `tests/fixtures/trace/` and `goldens.json` (counts, boxes and the first points of the first
//! outlines, exact on native). `CURVYO_UPDATE_GOLDENS=1` rewrites the goldens; the `#[ignore]`
//! writer remakes the pictures. The frontend's `npm test` traces the same pictures in the wasm
//! build and compares them with these goldens within the tolerance of criterion 18.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::needless_pass_by_value
)]

mod common;

use std::ops::ControlFlow;

use common::{MM_PER_PX_96, Picture, area, at_96_dpi, params, read_picture, render, run};
use curvyo_document_core::Affine;
use curvyo_vectorize_core::{MAX_HOLES_PER_REGION, TraceError, TraceParams, trace};
use kurbo::{BezPath, PathEl, Shape};
use serde_json::{Value, json};

/// One golden fixture: its file and the settings it is traced with (at 96 dpi).
struct Fixture {
    name: &'static str,
    file: &'static str,
    params: TraceParams,
}

fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture {
            name: "rectangle",
            file: "rectangle.pgm",
            params: params(128, 80, 0.5),
        },
        Fixture {
            name: "disc",
            file: "disc.pgm",
            params: params(128, 80, 0.5),
        },
        Fixture {
            name: "ring-with-dot",
            file: "ring-with-dot.pgm",
            params: params(128, 80, 0.5),
        },
        Fixture {
            name: "specks",
            file: "specks.pgm",
            params: params(128, 80, 0.5),
        },
        Fixture {
            name: "logo-alpha",
            file: "logo-alpha.pam",
            params: params(128, 80, 0.5),
        },
        Fixture {
            name: "disc-smooth-20",
            file: "disc.pgm",
            params: params(128, 20, 0.5),
        },
        Fixture {
            name: "ring-specks-0",
            file: "ring-with-dot.pgm",
            params: params(128, 80, 0.0),
        },
    ]
}

fn star(cx: f64, cy: f64, outer: f64, inner: f64) -> Vec<(f64, f64)> {
    (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { outer } else { inner };
            let a = std::f64::consts::PI * f64::from(i) / 5.0 - std::f64::consts::FRAC_PI_2;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

fn in_polygon(p: (f64, f64), points: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[j]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// The pictures of the fixtures, as the writer makes them.
fn make_pictures() -> Vec<(&'static str, Picture, bool)> {
    let rectangle = render(200, 100, 1, |x, y| {
        (50.0..150.0).contains(&x) && (25.0..75.0).contains(&y)
    });
    let disc = render(500, 500, 16, |x, y| {
        (x - 250.0).powi(2) + (y - 250.0).powi(2) <= 100.0 * 100.0
    });
    let ring = render(240, 240, 4, |x, y| {
        let d = ((x - 120.0).powi(2) + (y - 120.0).powi(2)).sqrt();
        (60.0..=100.0).contains(&d) || d <= 25.0
    });
    let specks = render(120, 80, 1, |x, y| {
        let r = |x0: f64, y0: f64, x1: f64, y1: f64| (x0..x1).contains(&x) && (y0..y1).contains(&y);
        let speck = [
            (5.0, 70.0),
            (50.0, 5.0),
            (50.0, 60.0),
            (100.0, 35.0),
            (115.0, 5.0),
        ]
        .iter()
        .any(|&(sx, sy)| r(sx, sy, sx + 1.0, sy + 1.0));
        (r(10.0, 10.0, 40.0, 40.0) && !r(20.0, 20.0, 28.0, 28.0))
            || r(60.0, 10.0, 90.0, 30.0)
            || r(60.0, 45.0, 110.0, 70.0)
            || speck
    });
    let logo = {
        let shape = star(50.0, 52.0, 45.0, 20.0);
        let cover = render(100, 100, 8, |x, y| in_polygon((x, y), &shape));
        // Transparent black around an opaque black star: the alpha carries the edge.
        let rgba = cover
            .levels()
            .iter()
            .flat_map(|&v| [0, 0, 0, 255 - v])
            .collect();
        Picture {
            width: 100,
            height: 100,
            rgba,
        }
    };
    vec![
        ("rectangle.pgm", rectangle, false),
        ("disc.pgm", disc, false),
        ("ring-with-dot.pgm", ring, false),
        ("specks.pgm", specks, false),
        ("logo-alpha.pam", logo, true),
    ]
}

/// Remakes the fixture pictures (`cargo test -p curvyo-vectorize-core --test goldens -- --ignored
/// write_fixtures`).
#[test]
#[ignore = "writes the fixture pictures"]
fn write_fixtures() {
    for (file, picture, alpha) in make_pictures() {
        common::write_picture(file, &picture, alpha);
    }
}

#[test]
fn the_fixture_files_are_what_the_writer_makes() {
    for (file, picture, _) in make_pictures() {
        assert_eq!(read_picture(file), picture, "{file}");
    }
}

fn points_of(path: &BezPath) -> Vec<Value> {
    path.elements()
        .iter()
        .filter_map(|el| match *el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) => {
                Some(json!([p.x, p.y]))
            }
            _ => None,
        })
        .take(4)
        .collect()
}

fn golden_of(fixture: &Fixture) -> Value {
    let picture = read_picture(fixture.file);
    let result = run(&picture, at_96_dpi(), &fixture.params);
    let boxes: Vec<Value> = result
        .outlines
        .iter()
        .map(|p| {
            let b = p.bounding_box();
            json!([b.x0, b.y0, b.x1, b.y1])
        })
        .collect();
    json!({
        "file": fixture.file,
        "map": at_96_dpi().to_array(),
        "threshold": fixture.params.threshold,
        "smooth": fixture.params.smooth,
        "specks_mm": fixture.params.specks.as_mm(),
        "paths": result.counts.paths,
        "nodes": result.counts.nodes,
        "specks_removed": result.counts.specks_removed,
        "boxes": boxes,
        "points": result.outlines.iter().take(4).map(points_of).collect::<Vec<_>>(),
    })
}

fn assert_close(actual: &Value, expected: &Value, at: &str) {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => {
            let (a, e) = (a.as_f64().unwrap(), e.as_f64().unwrap());
            assert!((a - e).abs() <= 1e-9, "{at}: {a} != {e}");
        }
        (Value::Array(a), Value::Array(e)) => {
            assert_eq!(a.len(), e.len(), "{at}: length");
            for (i, (a, e)) in a.iter().zip(e).enumerate() {
                assert_close(a, e, &format!("{at}[{i}]"));
            }
        }
        (Value::Object(a), Value::Object(e)) => {
            assert_eq!(a.len(), e.len(), "{at}: keys");
            for (key, e) in e {
                assert_close(&a[key], e, &format!("{at}.{key}"));
            }
        }
        _ => assert_eq!(actual, expected, "{at}"),
    }
}

/// Criterion 18: the result equals the golden file on native.
#[test]
fn the_traces_equal_the_goldens() {
    let path = common::fixture_dir().join("goldens.json");
    let actual: serde_json::Map<String, Value> = fixtures()
        .iter()
        .map(|f| (f.name.to_string(), golden_of(f)))
        .collect();
    let actual = Value::Object(actual);
    if std::env::var_os("CURVYO_UPDATE_GOLDENS").is_some() {
        std::fs::write(&path, serde_json::to_string_pretty(&actual).unwrap() + "\n").unwrap();
        return;
    }
    let expected: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_close(&actual, &expected, "goldens");
}

/// Criterion 18: two runs of the same build are bit-identical.
#[test]
fn a_trace_is_identical_on_every_run() {
    let picture = read_picture("ring-with-dot.pgm");
    let first = run(&picture, at_96_dpi(), &params(128, 80, 0.5));
    let second = run(&picture, at_96_dpi(), &params(128, 80, 0.5));
    assert_eq!(first, second);
}

/// Criterion 20: one closed outline of at most 8 nodes with the corners on the rectangle.
#[test]
fn the_rectangle_is_four_sharp_corners() {
    let result = run(
        &read_picture("rectangle.pgm"),
        at_96_dpi(),
        &params(128, 80, 0.5),
    );
    assert_eq!(result.counts.paths, 1);
    assert!(result.counts.nodes <= 8, "{}", result.counts.nodes);
    let outline = &result.outlines[0];
    assert!(matches!(outline.elements().last(), Some(PathEl::ClosePath)));
    let anchors: Vec<kurbo::Point> = outline
        .segments()
        .map(|s| kurbo::ParamCurve::end(&s))
        .collect();
    for (x, y) in [(50.0, 25.0), (150.0, 25.0), (150.0, 75.0), (50.0, 75.0)] {
        let corner = kurbo::Point::new(x * MM_PER_PX_96, y * MM_PER_PX_96);
        let nearest = anchors
            .iter()
            .map(|a| a.distance(corner))
            .fold(f64::INFINITY, f64::min);
        assert!(
            nearest <= 0.25 * MM_PER_PX_96,
            "corner {x},{y}: {nearest} mm"
        );
    }
}

/// Criterion 21: at most 16 nodes, every point within 0.75 px of the true circle, mean 0.3 px.
#[test]
fn the_disc_is_round_and_light() {
    let result = run(
        &read_picture("disc.pgm"),
        Affine::IDENTITY,
        &params(128, 80, 0.5),
    );
    assert_eq!(result.counts.paths, 1);
    assert!(result.counts.nodes <= 16, "{}", result.counts.nodes);
    let (mut max, mut sum, mut n) = (0.0_f64, 0.0, 0);
    for seg in result.outlines[0].segments() {
        for i in 0..100 {
            let p = kurbo::ParamCurve::eval(&seg, f64::from(i) / 100.0);
            let d = (p.distance(kurbo::Point::new(250.0, 250.0)) - 100.0).abs();
            max = max.max(d);
            sum += d;
            n += 1;
        }
    }
    let mean = sum / f64::from(n);
    eprintln!(
        "disc: {} nodes, max {max:.3} px, mean {mean:.3} px",
        result.counts.nodes
    );
    assert!(max <= 0.75, "max {max}");
    assert!(mean <= 0.3, "mean {mean}");
}

/// Nonzero winding of `outlines` at `p`.
fn winding(outlines: &[BezPath], p: kurbo::Point) -> i32 {
    outlines.iter().map(|o| o.winding(p)).sum()
}

/// Criterion 22: three outlines, the hole against the ring, the dot with it, the area within 1 %.
#[test]
fn the_ring_with_a_dot_has_three_outlines() {
    let picture = read_picture("ring-with-dot.pgm");
    let result = run(&picture, Affine::IDENTITY, &params(128, 80, 0.5));
    assert_eq!(result.counts.paths, 3);
    let areas: Vec<f64> = result.outlines.iter().map(area).collect();
    // Order: the ring, its hole, then the dot (first scan positions).
    assert!(areas[0].signum() != areas[1].signum(), "{areas:?}");
    assert_eq!(areas[0].signum(), areas[2].signum(), "{areas:?}");
    let filled: f64 = areas.iter().sum::<f64>().abs();
    let foreground = picture.levels().iter().filter(|&&v| v < 128).count() as f64;
    assert!(
        (filled - foreground).abs() / foreground < 0.01,
        "{filled} vs {foreground}"
    );
    assert_eq!(
        winding(&result.outlines, kurbo::Point::new(120.0, 120.0)).abs(),
        1,
        "the dot"
    );
    assert_eq!(
        winding(&result.outlines, kurbo::Point::new(120.0, 160.0)),
        0,
        "the hole"
    );
    assert_eq!(
        winding(&result.outlines, kurbo::Point::new(120.0, 200.0)).abs(),
        1,
        "the ring"
    );
}

/// Criterion 11: 3 regions, one hole and 5 one-pixel specks at Specks 0.5 mm.
#[test]
fn the_specks_fixture_counts() {
    let result = run(
        &read_picture("specks.pgm"),
        at_96_dpi(),
        &params(128, 80, 0.5),
    );
    assert_eq!(result.counts.paths, 4);
    assert_eq!(result.counts.specks_removed, 5);
    assert_eq!(result.counts.nodes, 16);
    let kept = run(
        &read_picture("specks.pgm"),
        at_96_dpi(),
        &params(128, 80, 0.0),
    );
    assert_eq!((kept.counts.paths, kept.counts.specks_removed), (9, 0));
}

/// Criterion 19: hole specks are filled, and nothing below the Specks size remains.
#[test]
fn hole_specks_are_filled_and_no_outline_is_below_the_specks_size() {
    // A square with a one-pixel hole and a 3 x 3 hole: 0.5 mm at 96 dpi is about 3.6 px².
    let picture = render(30, 30, 1, |x, y| {
        let square = (5.0..25.0).contains(&x) && (5.0..25.0).contains(&y);
        let pin = (10.0..11.0).contains(&x) && (10.0..11.0).contains(&y);
        let hole = (15.0..18.0).contains(&x) && (15.0..18.0).contains(&y);
        square && !pin && !hole
    });
    let result = run(&picture, at_96_dpi(), &params(128, 80, 0.5));
    assert_eq!((result.counts.paths, result.counts.specks_removed), (2, 1));
    for outline in &result.outlines {
        assert!(area(outline).abs() >= 0.25, "{}", area(outline));
    }
}

/// Criterion 23: higher Smooth never more nodes; higher Threshold never less area.
#[test]
fn smooth_and_threshold_are_monotone_on_the_fixtures() {
    for fixture in fixtures() {
        let picture = read_picture(fixture.file);
        let nodes = |smooth| {
            run(&picture, at_96_dpi(), &params(128, smooth, 0.5))
                .counts
                .nodes
        };
        let (n20, n60, n80) = (nodes(20), nodes(60), nodes(80));
        assert!(
            n80 <= n60 && n60 <= n20,
            "{}: {n20} {n60} {n80}",
            fixture.name
        );
        let filled = |t| {
            let r = run(&picture, at_96_dpi(), &params(t, 80, 0.5));
            r.outlines.iter().map(area).sum::<f64>().abs()
        };
        for t in [60, 100, 128, 160] {
            let (low, high) = (filled(t), filled(t + 10));
            assert!(
                high >= low * 0.995,
                "{} at {t}: {low} -> {high}",
                fixture.name
            );
        }
    }
}

/// Criterion 24: transparent pixels are white; the result is the logo's outline.
#[test]
fn transparent_pixels_are_background() {
    let result = run(
        &read_picture("logo-alpha.pam"),
        at_96_dpi(),
        &params(128, 80, 0.5),
    );
    assert_eq!(result.counts.paths, 1);
    let b = result.outlines[0].bounding_box();
    let mm = |px: f64| px * MM_PER_PX_96;
    assert!(
        b.x0 > mm(4.0) && b.x1 < mm(96.0) && b.y0 > mm(6.0) && b.y1 < mm(98.0),
        "{b:?}"
    );
}

/// Criterion 14: the outline points are the pixel corners mapped by the hand-computed affine.
#[test]
fn outlines_follow_a_rotated_flipped_pixel_map() {
    // 4 x 2 px, the left 2 x 2 black; framed 40 x 20 mm, rotated 90 degrees, flipped
    // horizontally: pixel (x, y) to mm (10 y, 10 x) + (5, 7), so the frame is 20 x 40 mm.
    let picture = Picture::grey(4, 2, &[0, 0, 255, 255, 0, 0, 255, 255]);
    let map = Affine::new(0.0, 10.0, 10.0, 0.0, 5.0, 7.0);
    let result = run(&picture, map, &params(128, 80, 0.0));
    assert_eq!(result.counts.paths, 1);
    let mut corners: Vec<(f64, f64)> = result.outlines[0]
        .elements()
        .iter()
        .filter_map(|el| match *el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => Some((p.x, p.y)),
            _ => None,
        })
        .collect();
    corners.sort_by(|a, b| a.partial_cmp(b).unwrap());
    corners.dedup();
    let mut expected: Vec<(f64, f64)> = [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)]
        .iter()
        .map(|&(x, y)| (10.0 * y + 5.0, 10.0 * x + 7.0))
        .collect();
    expected.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(corners.len(), 4, "{corners:?}");
    for (c, e) in corners.iter().zip(&expected) {
        assert!(
            (c.0 - e.0).abs() < 1e-6 && (c.1 - e.1).abs() < 1e-6,
            "{c:?} vs {e:?}"
        );
    }
}

/// Criterion 15 and the Terms: a full black page is one outline covering the whole picture; a
/// white one has none.
#[test]
fn whole_and_empty_pictures() {
    let black = Picture::grey(10, 10, &[0; 100]);
    let result = run(&black, at_96_dpi(), &params(128, 80, 0.5));
    assert_eq!(result.counts.paths, 1);
    assert!(result.covers_whole_image);
    let white = Picture::grey(10, 10, &[255; 100]);
    let result = run(&white, at_96_dpi(), &params(128, 80, 0.5));
    assert_eq!((result.counts.paths, result.foreground_pixels), (0, 0));
    assert!(!result.covers_whole_image);
}

/// Criterion 16: a region with 65 000 hole pixels returns "too much detail", never a crash.
#[test]
fn a_region_with_65000_holes_is_too_detailed() {
    // A black square with a one-pixel hole at every other pixel of every other row.
    let side = 2 * 256 + 1;
    let levels: Vec<u8> = (0..side * side)
        .map(|i| {
            let (x, y) = (i % side, i / side);
            if x % 2 == 1 && y % 2 == 1 { 255 } else { 0 }
        })
        .collect();
    let picture = Picture::grey(side as u32, side as u32, &levels);
    let holes = 256 * 256;
    assert!(holes >= MAX_HOLES_PER_REGION);
    let mut go = |_| ControlFlow::Continue(());
    let result = trace(
        picture.raster(),
        at_96_dpi(),
        &params(128, 80, 0.0),
        &mut go,
    );
    assert_eq!(result, Err(TraceError::TooDetailed));
    // With Specks raised the holes are filled and it traces.
    let result = trace(
        picture.raster(),
        at_96_dpi(),
        &params(128, 80, 0.5),
        &mut go,
    )
    .unwrap();
    assert_eq!(result.counts.paths, 1);
}

/// Criterion 17: a stop request returns `Cancelled`; invalid input is refused, not a panic.
#[test]
fn a_stop_cancels_and_bad_input_is_invalid() {
    let picture = read_picture("ring-with-dot.pgm");
    let mut calls = 0;
    let mut stop = |_| {
        calls += 1;
        if calls == 3 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };
    let result = trace(
        picture.raster(),
        at_96_dpi(),
        &params(128, 80, 0.5),
        &mut stop,
    );
    assert_eq!(result, Err(TraceError::Cancelled));
    let mut go = |_| ControlFlow::Continue(());
    let short = curvyo_vectorize_core::Raster {
        width: 3,
        height: 3,
        rgba: &[0; 8],
    };
    assert!(matches!(
        trace(short, at_96_dpi(), &params(128, 80, 0.5), &mut go),
        Err(TraceError::Invalid(_))
    ));
    let flat = Affine::new(1.0, 0.0, 1.0, 0.0, 0.0, 0.0);
    assert!(matches!(
        trace(picture.raster(), flat, &params(128, 80, 0.5), &mut go),
        Err(TraceError::Invalid(_))
    ));
}
