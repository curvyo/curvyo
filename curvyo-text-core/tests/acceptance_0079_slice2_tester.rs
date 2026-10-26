//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 2 (the
//! taper) on `curvyo-text-core`: the map of criterion 17 checked against every sampled outline
//! point (including the 5 % floor of `s`), straight segments and the 0.01 mm refit at a large Size
//! (18), unchanged advances (18), taper before rotation (19) and degenerate input.
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    clippy::similar_names,
    missing_docs
)]
#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;

use curvyo_text_core::{Face, FaceStore, LayoutOptions, Mm, TextLayout, layout};
use kurbo::{BezPath, ParamCurve, ParamCurveNearest, PathEl, PathSeg, Point, Shape};

const TOL: f64 = 0.01;

fn with_face<R>(file: &str, f: impl FnOnce(&Face) -> R) -> R {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts")
            .join(file),
    )
    .unwrap();
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    f(store.get(&key).unwrap())
}

fn sans<R>(f: impl FnOnce(&Face) -> R) -> R {
    with_face("source-sans-3/SourceSans3-Regular.otf", f)
}

fn opts(taper: f64, rotation_deg: f64) -> LayoutOptions {
    LayoutOptions {
        taper,
        rotation_rad: rotation_deg.to_radians(),
        ..LayoutOptions::default()
    }
}

fn lay(face: &Face, text: &str, size: f64, o: &LayoutOptions) -> TextLayout {
    layout(face, text, Mm(size), o).unwrap()
}

/// The spec's map of criterion 17 for one glyph, written here from the text: `y` up from the
/// baseline, `s(y) = max(0.05, 1 + t(2y/h - 1))`. Line space is y down, so the height above the
/// baseline is `baseline - y`.
fn spec_map(p: Point, centre_x: f64, baseline: f64, size: f64, t: f64, floor: bool) -> Point {
    let y_up = baseline - p.y;
    let raw = 1.0 + t * (2.0 * y_up / size - 1.0);
    let s = if floor { raw.max(0.05) } else { raw };
    Point::new(centre_x + (p.x - centre_x) * s, p.y)
}

fn segs(path: &BezPath) -> Vec<PathSeg> {
    path.segments().collect()
}

/// Dense samples of a path, `n` per segment, ends included.
fn samples(path: &BezPath, n: usize) -> Vec<Point> {
    let mut out = Vec::new();
    for s in segs(path) {
        for i in 0..=n {
            out.push(s.eval(i as f64 / n as f64));
        }
    }
    out
}

fn dist_to_path(path: &BezPath, p: Point) -> f64 {
    segs(path)
        .iter()
        .map(|s| s.nearest(p, 1e-9).distance_sq.sqrt())
        .fold(f64::INFINITY, f64::min)
}

fn dist_to_polyline(poly: &[Point], p: Point) -> f64 {
    poly.windows(2)
        .map(|w| {
            kurbo::Line::new(w[0], w[1])
                .nearest(p, 1e-12)
                .distance_sq
                .sqrt()
        })
        .fold(f64::INFINITY, f64::min)
}

/// The worst distance, both ways, between the tapered path and the exact map of the untapered
/// path (the exact map sampled densely per segment, so it is a polyline per segment).
fn worst_both_ways(untapered: &BezPath, tapered: &BezPath, map: impl Fn(Point) -> Point) -> f64 {
    let mut worst: f64 = 0.0;
    let mut exact_polys: Vec<Vec<Point>> = Vec::new();
    for s in segs(untapered) {
        let poly: Vec<Point> = (0..=600)
            .map(|i| map(s.eval(f64::from(i) / 600.0)))
            .collect();
        for p in poly.iter().step_by(7) {
            worst = worst.max(dist_to_path(tapered, *p));
        }
        exact_polys.push(poly);
    }
    for p in samples(tapered, 40) {
        let d = exact_polys
            .iter()
            .map(|poly| dist_to_polyline(poly, p))
            .fold(f64::INFINITY, f64::min);
        worst = worst.max(d);
    }
    worst
}

/// Criterion 17: for several glyphs, Sizes and tapers (both signs, the range ends), the tapered
/// outline is the exact map of the untapered outline within 0.01 mm both ways; the glyph keeps
/// its height (the vertical extent of the box is unchanged).
#[test]
fn c17_every_outline_point_follows_the_map() {
    sans(|face| {
        for (ch, size, t) in [
            ("H", 20.0, 0.2),
            ("H", 7.0, -0.35),
            ("O", 15.0, 0.5),
            ("A", 12.0, -0.5),
            ("g", 25.0, 0.3),
        ] {
            let plain = lay(face, ch, size, &opts(0.0, 0.0));
            let tapered = lay(face, ch, size, &opts(t, 0.0));
            let g = &plain.lines[0].glyphs[0];
            let (cx, base) = (g.centre_x.0, g.baseline.0);
            let p0 = &plain.glyph_paths(face)[0];
            let p1 = &tapered.glyph_paths(face)[0];
            let worst = worst_both_ways(p0, p1, |p| spec_map(p, cx, base, size, t, true));
            assert!(worst < TOL, "{ch} at {size} mm, t {t}: worst {worst} mm");
            let (b0, b1) = (p0.bounding_box(), p1.bounding_box());
            assert!(
                (b0.y0 - b1.y0).abs() < TOL && (b0.y1 - b1.y1).abs() < TOL,
                "{ch}: the height is kept: {b0:?} vs {b1:?}"
            );
        }
    });
}

/// Criterion 17, the spec's own test, read from the outline corners: "H" at 20 mm, 20 %: the
/// outer stem edges are 0.8 and 1.2 times the untapered width apart at y = 0 and y = 20 mm and
/// unchanged at y = 10 mm; the top stays 20.00 mm above the baseline.
#[test]
fn c17_h_trapezoid_at_baseline_middle_and_top() {
    sans(|face| {
        let size = 20.0;
        let plain = lay(face, "H", size, &opts(0.0, 0.0));
        let tapered = lay(face, "H", size, &opts(0.2, 0.0));
        let base = plain.lines[0].glyphs[0].baseline.0;
        let width_at = |path: &BezPath, y_up: f64| {
            // The outline crossed by the horizontal line at height y_up: outermost crossings.
            let y = base - y_up;
            let line = kurbo::Line::new((-1000.0, y), (1000.0, y));
            let mut xs: Vec<f64> = Vec::new();
            for s in segs(path) {
                for hit in s.intersect_line(line) {
                    xs.push(line.eval(hit.line_t).x);
                }
            }
            // At the exact baseline and cap line the crossing is a horizontal edge: use corners.
            for p in samples(path, 1) {
                if (p.y - y).abs() < 1e-6 {
                    xs.push(p.x);
                }
            }
            let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
            let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            hi - lo
        };
        let p0 = &plain.glyph_paths(face)[0];
        let p1 = &tapered.glyph_paths(face)[0];
        let top0 = base - p0.bounding_box().y0;
        assert!((top0 - size).abs() < TOL, "the H is Size high: {top0}");
        let top1 = base - p1.bounding_box().y0;
        assert!((top1 - size).abs() < TOL, "tapered H top: {top1}");
        for (y_up, factor) in [(0.0, 0.8), (10.0, 1.0), (20.0, 1.2), (5.0, 0.9)] {
            let w0 = width_at(p0, y_up);
            let w1 = width_at(p1, y_up);
            assert!(
                (w1 - factor * w0).abs() < TOL,
                "y {y_up}: {w1} vs {factor} x {w0}"
            );
        }
    });
}

/// The first character from U+0021 to U+024F in `face` whose untapered outline reaches below
/// `-depth x Size` (or above `(1 + depth) x Size` when `up`), with the layout at `size`.
fn reaching(face: &Face, size: f64, depth: f64, up: bool) -> Option<String> {
    (0x21_u32..0x250)
        .filter_map(char::from_u32)
        .map(String::from)
        .find(|c| {
            let l = lay(face, c, size, &opts(0.0, 0.0));
            let Some(g) = l.lines[0].glyphs.first() else {
                return false;
            };
            let Some(p) = l.glyph_paths(face).into_iter().next() else {
                return false;
            };
            if p.elements().is_empty() {
                return false;
            }
            let b = p.bounding_box();
            if up {
                g.baseline.0 - b.y0 > (1.0 + depth) * size
            } else {
                b.y1 - g.baseline.0 > depth * size
            }
        })
}

/// Criterion 17, the 5 % floor: where `1 + t(2y/h - 1)` falls below 0.05 (below y = -0.45 h at
/// +50 %: the deep descender of a script face; above y = 1.45 h at -50 %: a stacked accent of the
/// serif face), the glyph is 0.05 times as wide there, never mirrored. The tapered outline
/// matches the floored map within 0.01 mm and does not match the unfloored one.
#[test]
fn c17_five_percent_floor() {
    let size = 20.0;
    for (file, t, up, depth) in [
        ("great-vibes/GreatVibes-Regular.ttf", 0.5, false, 0.55),
        ("source-serif-4/SourceSerif4-Regular.ttf", -0.5, true, 0.47),
    ] {
        with_face(file, |face| {
            let ch = reaching(face, size, depth, up)
                .unwrap_or_else(|| panic!("{file}: a glyph reaches the floor"));
            let plain = lay(face, &ch, size, &opts(0.0, 0.0));
            let tapered = lay(face, &ch, size, &opts(t, 0.0));
            let g = &plain.lines[0].glyphs[0];
            let (cx, base) = (g.centre_x.0, g.baseline.0);
            let p0 = &plain.glyph_paths(face)[0];
            let p1 = &tapered.glyph_paths(face)[0];
            let floored = worst_both_ways(p0, p1, |p| spec_map(p, cx, base, size, t, true));
            assert!(floored < TOL, "{ch} t {t}: floored map worst {floored} mm");
            let unfloored = worst_both_ways(p0, p1, |p| spec_map(p, cx, base, size, t, false));
            assert!(
                unfloored > 2.0 * TOL,
                "{ch} t {t}: the floor must matter here (unfloored worst {unfloored} mm)"
            );
        });
    }
}

/// Criterion 18: horizontal and vertical straight segments of the outline stay single straight
/// segments: the tapered "H", "E", "T", "L", "I" are lines only, as many as untapered, and every
/// horizontal line stays horizontal.
#[test]
fn c18_straight_segments_stay_straight() {
    sans(|face| {
        for ch in ["H", "E", "T", "L", "I"] {
            for t in [0.5, -0.5, 0.17] {
                let p0 = &lay(face, ch, 30.0, &opts(0.0, 0.0)).glyph_paths(face)[0];
                let p1 = &lay(face, ch, 30.0, &opts(t, 0.0)).glyph_paths(face)[0];
                let lines0 = segs(p0)
                    .iter()
                    .filter(|s| matches!(s, PathSeg::Line(_)))
                    .count();
                assert!(lines0 > 0);
                for s in segs(p1) {
                    let PathSeg::Line(l) = s else {
                        panic!("{ch} t {t}: a straight edge became {s:?}");
                    };
                    let _ = l;
                }
                assert_eq!(segs(p1).len(), segs(p0).len(), "{ch} t {t}: no nodes added");
                let horizontal = |p: &BezPath| {
                    segs(p)
                        .iter()
                        .filter(|s| {
                            let (a, b) = (s.start(), s.end());
                            (a.y - b.y).abs() < 1e-9 && (a.x - b.x).abs() > 1e-6
                        })
                        .count()
                };
                assert_eq!(horizontal(p1), horizontal(p0), "{ch} t {t}");
            }
        }
    });
}

/// Criterion 18 at a large Size: the refit holds 0.01 mm in absolute terms on a 300 mm "S", "@"
/// and "&" at the range end, with the default accuracy of the draw path.
#[test]
fn c18_refit_holds_at_a_large_size() {
    sans(|face| {
        for ch in ["S", "@", "&", "e"] {
            let size = 300.0;
            let t = 0.5;
            let plain = lay(face, ch, size, &opts(0.0, 0.0));
            let tapered = lay(face, ch, size, &opts(t, 0.0));
            let g = &plain.lines[0].glyphs[0];
            let (cx, base) = (g.centre_x.0, g.baseline.0);
            let p0 = &plain.glyph_paths(face)[0];
            let p1 = &tapered.glyph_paths(face)[0];
            let worst = worst_both_ways(p0, p1, |p| spec_map(p, cx, base, size, t, true));
            assert!(worst < TOL, "{ch} at {size} mm: worst {worst} mm");
            assert!(
                segs(p1).len() < 40 * segs(p0).len(),
                "{ch}: the refit stays a modest number of segments ({} from {})",
                segs(p1).len(),
                segs(p0).len()
            );
        }
    });
}

/// Criterion 18: advances and line layout do not change with the taper: same extents, origins,
/// centres, baselines and caret points.
#[test]
fn c18_advances_and_lines_unchanged() {
    sans(|face| {
        let text = "Ring of\nletters AV";
        let a = lay(face, text, 12.0, &opts(0.0, 0.0));
        let b = lay(face, text, 12.0, &opts(-0.42, 0.0));
        assert_eq!(a.lines.len(), b.lines.len());
        for (la, lb) in a.lines.iter().zip(&b.lines) {
            assert_eq!(la.extent, lb.extent);
            assert_eq!(la.baseline, lb.baseline);
            assert_eq!(la.width, lb.width);
            assert_eq!(la.glyphs.len(), lb.glyphs.len());
            for (ga, gb) in la.glyphs.iter().zip(&lb.glyphs) {
                assert_eq!(ga.origin, gb.origin);
                assert_eq!(ga.centre_x, gb.centre_x);
                assert_eq!(ga.baseline, gb.baseline);
            }
        }
        for byte in [0, 3, 8, text.len()] {
            assert_eq!(a.caret_point(byte), b.caret_point(byte));
        }
        assert_eq!(a.line_box(), b.line_box());
    });
}

/// Criterion 19: taper first in the glyph's frame, then the rotation about the cluster centre
/// (the middle of the advance, Size/2 above the baseline). Rotating the tapered outline by hand
/// gives the tapered and turned outline; tapering the turned outline by hand does not.
#[test]
fn c19_taper_then_rotation_not_the_other_way() {
    sans(|face| {
        let (size, t, deg) = (20.0, 0.3, 90.0);
        let tapered = lay(face, "F", size, &opts(t, 0.0));
        let both = lay(face, "F", size, &opts(t, deg));
        let g = &tapered.lines[0].glyphs[0];
        let pivot = Point::new(g.centre_x.0, g.baseline.0 - size / 2.0);
        let turn = |sign: f64| {
            move |p: Point| {
                // sign 1: clockwise on screen (y down), the Select tool's sign.
                let (s, c) = (sign * deg.to_radians()).sin_cos();
                let v = p - pivot;
                pivot + kurbo::Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
            }
        };
        let p_t = &tapered.glyph_paths(face)[0];
        let p_both = &both.glyph_paths(face)[0];
        let cw = worst_both_ways(p_t, p_both, turn(1.0));
        let ccw = worst_both_ways(p_t, p_both, turn(-1.0));
        let worst = cw.min(ccw);
        assert!(worst < TOL, "taper then rotation: worst {worst} mm");

        // The other order: rotate the plain glyph, then taper in the line's frame.
        let plain = lay(face, "F", size, &opts(0.0, 0.0));
        let p_plain = &plain.glyph_paths(face)[0];
        let rot = turn(if cw < ccw { 1.0 } else { -1.0 });
        let wrong = worst_both_ways(p_plain, p_both, |p| {
            spec_map(rot(p), g.centre_x.0, g.baseline.0, size, t, true)
        });
        assert!(wrong > 0.5, "rotation then taper must differ: {wrong} mm");
    });
}

/// Degenerate input: empty text, only spaces, only line breaks, a single-line face (open paths
/// stay open), and a zero taper equals no taper exactly.
#[test]
fn edge_degenerate_input() {
    sans(|face| {
        for text in ["", "   ", "\n\n", " \n "] {
            let l = lay(face, text, 10.0, &opts(0.5, 33.0));
            let _ = l.glyph_paths(face);
        }
        let a = lay(face, "Zero", 10.0, &opts(0.0, 0.0)).glyph_paths(face);
        let b = lay(face, "Zero", 10.0, &LayoutOptions::default()).glyph_paths(face);
        assert_eq!(a, b);
    });
    with_face("hershey-sans-1/HersheySans1.svg", |face| {
        let plain = lay(face, "HO", 10.0, &opts(0.0, 0.0)).glyph_paths(face);
        let tapered = lay(face, "HO", 10.0, &opts(0.25, 0.0)).glyph_paths(face);
        assert_eq!(plain.len(), tapered.len());
        let closes = |p: &BezPath| p.elements().iter().any(|e| matches!(e, PathEl::ClosePath));
        for (p0, p1) in plain.iter().zip(&tapered) {
            assert_eq!(closes(p0), closes(p1), "open strokes stay open");
            assert_ne!(p0, p1, "the single-line glyph is tapered too");
        }
    });
}

/// White-box (glyph frame): a vertically and horizontally kerned letter is tapered about its own
/// moved baseline and middle, so it is the unkerned tapered letter moved by its kern.
#[test]
fn edge_kerned_letter_tapers_in_its_own_frame() {
    sans(|face| {
        let base = opts(0.4, 0.0);
        let kerned = LayoutOptions {
            char_offsets: vec![curvyo_text_core::CharOffset {
                index: 1,
                along: Mm(1.5),
                across: Mm(4.0),
                rotation_rad: 0.0,
            }],
            ..base.clone()
        };
        let a = lay(face, "HH", 20.0, &base);
        let b = lay(face, "HH", 20.0, &kerned);
        let (ga, gb) = (&a.lines[0].glyphs[1], &b.lines[0].glyphs[1]);
        let shift = gb.origin - ga.origin;
        assert!(shift.hypot() > 1.0, "the kern moves the letter: {shift:?}");
        let pa = &a.glyph_paths(face)[1];
        let pb = &b.glyph_paths(face)[1];
        let moved = kurbo::Affine::translate(shift) * pa;
        let worst = worst_both_ways(&moved, pb, |p| p);
        assert!(
            worst < TOL,
            "kerned tapered = tapered moved: worst {worst} mm"
        );
    });
}

/// White-box (criterion 22 with 17): in a vertical column set Sideways, each letter is tapered in
/// its own frame and then laid on its side: its box is the horizontal tapered letter's box with
/// width and height swapped.
#[test]
fn edge_sideways_column_tapers_in_the_glyph_frame() {
    sans(|face| {
        let horizontal = lay(face, "A", 20.0, &opts(0.3, 0.0));
        let column = lay(
            face,
            "A",
            20.0,
            &LayoutOptions {
                writing_mode: curvyo_text_core::WritingMode::VerticalRl,
                orientation: curvyo_text_core::GlyphOrientation::Sideways,
                ..opts(0.3, 0.0)
            },
        );
        let h = horizontal.glyph_paths(face)[0].bounding_box();
        let v = column.glyph_paths(face)[0].bounding_box();
        assert!(
            (h.width() - v.height()).abs() < TOL && (h.height() - v.width()).abs() < TOL,
            "horizontal {h:?} vs sideways {v:?}"
        );
    });
}

/// White-box: degenerate Sizes never give non-finite outlines.
#[test]
fn edge_tiny_size_stays_finite() {
    sans(|face| {
        for size in [0.0, 1e-9, 1e-3] {
            if let Ok(l) = layout(face, "Hg", Mm(size), &opts(0.5, 10.0)) {
                for p in l.glyph_paths(face) {
                    for el in p.elements() {
                        if let Some(q) = el.end_point() {
                            assert!(q.x.is_finite() && q.y.is_finite(), "size {size}: {q:?}");
                        }
                    }
                }
            }
        }
    });
}
