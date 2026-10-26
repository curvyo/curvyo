//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 2 (the
//! taper) on the pure rules in `curvyo-ui-core`: the Taper field's view, range, steps, drag and
//! typed values (criterion 16), the circle values of criterion 19 on all four Side and Line cases
//! of a round badge (`0080` criterion 10), a uniform resize scaling the tapered text as one (34)
//! and the drawn outline being the engine's tapered layout (35).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CarrierRef, Document, EllipseFrame, FontKind, FontRef, Length, LineOnPath, NewAnchor,
    OnPathValues, PathSide, PathSnapshot, Point, Style, TextAlign, TextChars, TextLayoutValues,
    TextOutline, TextSnapshot, UNCREATED_TEXT_ID, Vec2,
};
use curvyo_text_core::{Face, FaceStore, Mm};
use curvyo_ui_core::{
    Grid, LayoutField, LayoutSubject, default_position, field_view, lay_out, measure_of,
    parse_field_text, set_field, stepped, text_layout, value_at,
};

const TOL: f64 = 0.01;
const D: TextLayoutValues = TextLayoutValues::DEFAULT;

fn sans() -> std::sync::Arc<Face> {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf"),
    )
    .unwrap();
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    store.get(&key).unwrap().clone()
}

fn font() -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

fn tapered(t: f64) -> TextLayoutValues {
    TextLayoutValues { taper: t, ..D }
}

fn subject(size: f64, values: TextLayoutValues) -> LayoutSubject {
    LayoutSubject {
        size_mm: size,
        values,
        natural_line_mm: size * 1.4,
        kern: curvyo_document_core::CharValues::DEFAULT,
    }
}

fn snapshot(string: &str, size: f64, layout: TextLayoutValues, placement: Affine) -> TextSnapshot {
    TextSnapshot {
        id: UNCREATED_TEXT_ID,
        string: string.to_owned(),
        font: font(),
        size: Length::from_mm(size),
        layout,
        chars: TextChars::default(),
        placement,
        style: Style::default(),
        outline: None,
        on_path: None,
        source: None,
    }
}

fn anchors(path: &PathSnapshot) -> Vec<Point> {
    let mut all: Vec<Point> = path.anchors.iter().map(|a| a.point).collect();
    for s in &path.extra_subpaths {
        all.extend(s.anchors.iter().map(|a: &NewAnchor| a.point));
    }
    all
}

fn points(o: &TextOutline) -> Vec<Point> {
    anchors(&o.glyphs)
}

// ---------------------------------------------------------------------------------------------
// Criterion 16: the field
// ---------------------------------------------------------------------------------------------

/// Criterion 16: default 0 % with no reset; a set taper shows in percent with the reset; the
/// track is the whole range -50 % to 50 % with a zero mark; differing texts are Mixed; the
/// value does not depend on Size (it is not relative).
#[test]
fn c16_field_view() {
    let v = field_view(LayoutField::Taper, &[subject(20.0, D)]);
    assert_eq!(
        (v.text.as_str(), v.value, v.mixed, v.resettable),
        ("0", 0.0, false, false)
    );
    assert!(
        (v.track_min + 50.0).abs() < 1e-9 && (v.track_max - 50.0).abs() < 1e-9,
        "{v:?}"
    );
    assert!(v.zero.is_some(), "a signed bar with a zero mark: {v:?}");
    let set = field_view(LayoutField::Taper, &[subject(20.0, tapered(0.2))]);
    assert_eq!(set.text, "20");
    assert!((set.value - 20.0).abs() < 1e-9 && set.resettable && !set.mixed);
    let neg = field_view(LayoutField::Taper, &[subject(7.0, tapered(-0.125))]);
    assert_eq!(neg.text, "\u{2212}12.5", "one decimal, U+2212 minus");
    // Same taper on different Sizes: one value, track unchanged.
    let two = field_view(
        LayoutField::Taper,
        &[subject(5.0, tapered(0.3)), subject(50.0, tapered(0.3))],
    );
    assert!(!two.mixed && two.text == "30");
    assert!((two.track_min + 50.0).abs() < 1e-9 && (two.track_max - 50.0).abs() < 1e-9);
    let mixed = field_view(
        LayoutField::Taper,
        &[subject(10.0, tapered(0.3)), subject(10.0, D)],
    );
    assert!(mixed.mixed && mixed.text.is_empty() && mixed.resettable);
}

/// Criterion 16: typed values are percent and held at -50 % and 50 % with the limit reported
/// (the notice's value); inside the range nothing is limited; other values are untouched; Size
/// does not change the range.
#[test]
fn c16_typed_values_and_limits() {
    let keep = TextLayoutValues {
        letter_spacing: 0.1,
        align: TextAlign::Centre,
        ..D
    };
    let one = [subject(10.0, keep)];
    let c = set_field(LayoutField::Taper, &one, Some(20.0));
    assert!((c.values[0].taper - 0.2).abs() < 1e-12);
    assert_eq!(c.limited, 0);
    assert!(c.limit.is_none());
    assert_eq!(c.values[0].letter_spacing, 0.1);
    assert_eq!(c.values[0].align, TextAlign::Centre);
    let c = set_field(LayoutField::Taper, &one, Some(80.0));
    assert!((c.values[0].taper - 0.5).abs() < 1e-12);
    let (top, v) = c.limit.unwrap();
    assert!(top && (v - 50.0).abs() < 1e-9, "{:?}", c.limit);
    let c = set_field(LayoutField::Taper, &one, Some(-50.01));
    assert!((c.values[0].taper + 0.5).abs() < 1e-12);
    let (top, v) = c.limit.unwrap();
    assert!(!top && (v + 50.0).abs() < 1e-9, "{:?}", c.limit);
    let c = set_field(LayoutField::Taper, &one, Some(50.0));
    assert!(c.limit.is_none(), "the limit itself is inside the range");
    let two = [subject(3.0, D), subject(300.0, D)];
    let c = set_field(LayoutField::Taper, &two, Some(-33.3));
    assert_eq!(c.limited, 0);
    assert!(c.values.iter().all(|v| (v.taper + 0.333).abs() < 1e-12));
    let c = set_field(LayoutField::Taper, &one, None);
    assert_eq!(c.values[0].taper, 0.0, "reset to 0 %");
}

/// Criterion 16: Up and Down 1 %, Shift 10 %, Ctrl 0.1 %; typed "20" and "20 %".
#[test]
fn c16_steps_and_parse() {
    let f = LayoutField::Taper;
    assert!((stepped(f, 10.0, 1, Grid::Normal) - 11.0).abs() < 1e-9);
    assert!((stepped(f, 10.0, -1, Grid::Normal) - 9.0).abs() < 1e-9);
    assert!((stepped(f, 20.0, 1, Grid::Coarse) - 30.0).abs() < 1e-9);
    assert!((stepped(f, 20.0, -1, Grid::Coarse) - 10.0).abs() < 1e-9);
    assert!((stepped(f, 20.0, 1, Grid::Fine) - 20.1).abs() < 1e-9);
    assert!(
        stepped(f, 50.0, 1, Grid::Coarse) <= 50.0,
        "steps stop at the range end"
    );
    assert!(stepped(f, -50.0, -1, Grid::Normal) >= -50.0);
    for (typed, want) in [
        ("20", 20.0),
        ("20 %", 20.0),
        ("20%", 20.0),
        ("-12.5", -12.5),
        ("\u{2212}12.5 %", -12.5),
    ] {
        assert_eq!(parse_field_text(f, typed), Some(want), "{typed}");
    }
    assert_eq!(parse_field_text(f, "much"), None);
    assert_eq!(parse_field_text(f, ""), None);
}

/// Criterion 16: the track is the whole range, so a drag from end to end covers -50 % to 50 %,
/// the middle is 0 %, and a drag past an end stops at the range end.
#[test]
fn c16_drag_covers_the_range() {
    let s = [subject(10.0, D)];
    let f = LayoutField::Taper;
    assert!((value_at(f, &s, 0.0, Grid::Normal) + 50.0).abs() < 1e-9);
    assert!((value_at(f, &s, 1.0, Grid::Normal) - 50.0).abs() < 1e-9);
    assert!(value_at(f, &s, 0.5, Grid::Normal).abs() < 1e-9);
    assert!((value_at(f, &s, 7.0, Grid::Normal) - 50.0).abs() < 1e-9);
    assert!((value_at(f, &s, -7.0, Grid::Normal) + 50.0).abs() < 1e-9);
}

// ---------------------------------------------------------------------------------------------
// Criterion 19: the circle values on the four cases of a round badge
// ---------------------------------------------------------------------------------------------

/// The outer stem corners of an upright H in `pts`: (left, right) x at height `y` (screen y).
fn stem_xs(pts: &[Point], y: f64) -> (f64, f64) {
    let row: Vec<f64> = pts
        .iter()
        .filter(|p| (p.y - y).abs() < 1e-4)
        .map(|p| p.x)
        .collect();
    assert!(row.len() >= 2, "corners at y = {y}: {row:?}");
    (
        row.iter().copied().fold(f64::INFINITY, f64::min),
        row.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    )
}

/// How far the circle's centre (100, 100) lies from the line through `a` and `b`.
fn miss(a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (cx, cy) = (100.0 - a.x, 100.0 - a.y);
    (cx * dy - cy * dx).abs() / dx.hypot(dy)
}

/// Criterion 19 with `0080` criterion 10: an "H" of Size 10 mm on the circle of radius 50 about
/// (100, 100), in each of the four Side and Line cases. With the spec's value for its baseline
/// radius `R` (outside the ring, letters pointing away: t = h / (2(R + h/2)); letters pointing at
/// the centre: t = -h / (2(R - h/2))), both outer stem edges point exactly at the centre, the
/// H is (1 - t) times as wide at its baseline and (1 + t) times at its top, and its top and
/// baseline stay where the untapered H has them.
#[test]
fn c19_circle_values_on_all_four_cases() {
    use LineOnPath::{Baseline, TopEdge};
    use PathSide::{Above, Below};
    let face = sans();
    let h = 10.0;
    let d = Document::new(1);
    let carrier = d.create_ellipse(EllipseFrame {
        center: Point::new(100.0, 100.0),
        rx: Length::from_mm(50.0),
        ry: Length::from_mm(50.0),
    });
    let make = |layout: TextLayoutValues, values: &OnPathValues| {
        let t = d
            .create_text_with_chars(
                "H",
                &font(),
                Length::from_mm(h),
                Affine::IDENTITY,
                &layout,
                &TextChars::default(),
            )
            .unwrap();
        d.put_text_on_path(
            t,
            CarrierRef {
                node: carrier,
                outline: 0,
            },
            values,
            true,
        )
        .unwrap();
        d.text(t).unwrap()
    };
    let probe = make(D, &OnPathValues::at(Length::from_mm(0.0)));
    let m = measure_of(probe.on_path.as_ref().unwrap().curve.as_ref().unwrap());
    let top_pos = default_position(&text_layout(&probe, &face).unwrap(), &m);
    let bottom_pos = Length::from_mm(m.length().as_mm() / 4.0);

    // (side, line, position, screen y of the baseline, of the top, baseline radius, outward)
    for (side, line, pos, base_y, top_y, r, outward, what) in [
        (
            Above,
            Baseline,
            top_pos,
            50.0,
            40.0,
            50.0,
            true,
            "Above + Baseline",
        ),
        (
            Below,
            TopEdge,
            top_pos,
            60.0,
            50.0,
            40.0,
            true,
            "Below + Top edge",
        ),
        (
            Above,
            TopEdge,
            bottom_pos,
            160.0,
            150.0,
            60.0,
            false,
            "Above + Top edge",
        ),
        (
            Below,
            Baseline,
            bottom_pos,
            150.0,
            140.0,
            50.0,
            false,
            "Below + Baseline",
        ),
    ] {
        let t = if outward {
            h / (2.0 * (r + h / 2.0))
        } else {
            -h / (2.0 * (r - h / 2.0))
        };
        let values = OnPathValues {
            side,
            line,
            position: pos,
            ..OnPathValues::at(pos)
        };
        let plain = points(&lay_out(&make(D, &values), &face, false).unwrap());
        let taper = points(&lay_out(&make(tapered(t), &values), &face, false).unwrap());

        // The untapered H: vertical stems, so the stem edges are parallel and miss the centre.
        let (l0b, r0b) = stem_xs(&plain, base_y);
        let (l0t, r0t) = stem_xs(&plain, top_y);
        assert!(
            (l0b - l0t).abs() < TOL && (r0b - r0t).abs() < TOL,
            "{what}: upright H"
        );
        let w = r0b - l0b;
        assert!(
            (f64::midpoint(l0b, r0b) - 100.0).abs() < 0.5,
            "{what}: the H sits at x = 100"
        );

        let (lb, rb) = stem_xs(&taper, base_y);
        let (lt, rt) = stem_xs(&taper, top_y);
        for (a, b, edge) in [
            (Point::new(lb, base_y), Point::new(lt, top_y), "left"),
            (Point::new(rb, base_y), Point::new(rt, top_y), "right"),
        ] {
            let off = miss(a, b);
            // The edge is 10 mm long and ~50 mm from the centre; 0.01 mm at the ends allows
            // about 0.1 mm of miss at the centre.
            assert!(
                off < 0.1,
                "{what}, {edge} stem edge misses the centre by {off} mm (t {t})"
            );
        }
        assert!(
            ((rb - lb) - (1.0 - t) * w).abs() < TOL,
            "{what}: baseline width {} vs {} (t {t})",
            rb - lb,
            (1.0 - t) * w
        );
        assert!(
            ((rt - lt) - (1.0 + t) * w).abs() < TOL,
            "{what}: top width {} vs {} (t {t})",
            rt - lt,
            (1.0 + t) * w
        );
        // The ring's own ratio: the radii of the top and of the baseline.
        let ring = (top_y - 100.0).abs() / (base_y - 100.0).abs();
        assert!(
            ((rt - lt) / (rb - lb) - ring).abs() < 1e-3,
            "{what}: top/baseline width ratio is the ring's"
        );
        // Taper keeps the height.
        let ys = |p: &[Point]| {
            (
                p.iter().map(|q| q.y).fold(f64::INFINITY, f64::min),
                p.iter().map(|q| q.y).fold(f64::NEG_INFINITY, f64::max),
            )
        };
        let (a0, b0) = ys(&plain);
        let (a1, b1) = ys(&taper);
        assert!(
            (a0 - a1).abs() < TOL && (b0 - b1).abs() < TOL,
            "{what}: height kept"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Criteria 34 and 35
// ---------------------------------------------------------------------------------------------

/// Criterion 34: a tapered text at twice the Size draws as the text at Size scaled by two about
/// its anchor: the taper is a fraction, it does not change with a uniform resize.
#[test]
fn c34_uniform_resize_scales_the_tapered_text_as_one() {
    let face = sans();
    let v = TextLayoutValues {
        rotation: curvyo_document_core::Angle::from_radians(0.3),
        letter_spacing: 0.05,
        taper: -0.3,
        ..D
    };
    let small = lay_out(&snapshot("Ring S", 10.0, v, Affine::IDENTITY), &face, false).unwrap();
    let big = lay_out(&snapshot("Ring S", 20.0, v, Affine::IDENTITY), &face, false).unwrap();
    assert_eq!(small.glyph_outlines, big.glyph_outlines);
    let a = points(&small);
    let b = points(&big);
    // The refit may place nodes differently at another scale; compare both ways by distance to
    // the other's anchors only where the anchor counts agree, else by the boxes.
    if a.len() == b.len() {
        for (p, q) in a.iter().zip(&b) {
            assert!(
                (p.x * 2.0 - q.x).abs() < 2.0 * TOL && (p.y * 2.0 - q.y).abs() < 2.0 * TOL,
                "{p:?} x 2 vs {q:?}"
            );
        }
    }
    let bbox = |p: &[Point]| {
        p.iter().fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(x0, y0, x1, y1), q| (x0.min(q.x), y0.min(q.y), x1.max(q.x), y1.max(q.y)),
        )
    };
    let (x0, y0, x1, y1) = bbox(&a);
    let (u0, v0, u1, v1) = bbox(&b);
    for (s, l) in [(x0, u0), (y0, v0), (x1, u1), (y1, v1)] {
        assert!((2.0 * s - l).abs() < 2.0 * TOL, "box {s} x 2 vs {l}");
    }
}

/// Criterion 35 at the mapping level: the drawn outline of a tapered, turned, placed text is the
/// engine's layout with the taper as a fraction, mapped by the placement.
#[test]
fn c35_drawn_outline_is_the_tapered_engine_layout() {
    let face = sans();
    let v = TextLayoutValues {
        rotation: curvyo_document_core::Angle::from_radians(-0.4),
        taper: 0.35,
        ..D
    };
    let placement = Affine::translation(Vec2::new(30.0, 40.0));
    let out = lay_out(&snapshot("Tap er", 12.0, v, placement), &face, false).unwrap();
    let engine = curvyo_text_core::layout(
        &face,
        "Tap er",
        Mm(12.0),
        &curvyo_text_core::LayoutOptions {
            rotation_rad: -0.4,
            taper: 0.35,
            ..curvyo_text_core::LayoutOptions::default()
        },
    )
    .unwrap();
    let paths = engine.glyph_paths(&face);
    assert_eq!(
        out.glyph_outlines.len(),
        paths.len(),
        "one outline per glyph"
    );
    let want: Vec<Point> = paths
        .iter()
        .flat_map(|p| {
            p.elements()
                .iter()
                .filter_map(kurbo::PathEl::end_point)
                .map(|q| Point::new(q.x + 30.0, q.y + 40.0))
                .collect::<Vec<_>>()
        })
        .collect();
    for g in points(&out) {
        assert!(
            want.iter()
                .any(|w| (w.x - g.x).abs() < TOL && (w.y - g.y).abs() < TOL),
            "anchor {g:?} is not on the engine's tapered outline"
        );
    }
    // And it is not the untapered outline.
    let flat = lay_out(&snapshot("Tap er", 12.0, D, placement), &face, false).unwrap();
    assert_ne!(points(&flat), points(&out));
}

// ---------------------------------------------------------------------------------------------
// White-box: the accuracy under a stretched placement, the box over tapered glyphs
// ---------------------------------------------------------------------------------------------

/// Every subpath of a path snapshot as cubic segments (document space).
fn subpath_segments(path: &PathSnapshot) -> Vec<kurbo::CubicBez> {
    let mut out = Vec::new();
    let mut one = |anchors: &[NewAnchor], closed: bool| {
        let k = |p: Point| kurbo::Point::new(p.x, p.y);
        let n = anchors.len();
        let count = if closed { n } else { n.saturating_sub(1) };
        for i in 0..count {
            let a = &anchors[i];
            let b = &anchors[(i + 1) % n];
            out.push(kurbo::CubicBez::new(
                k(a.point),
                k(a.point.translated(a.handle_out)),
                k(b.point.translated(b.handle_in)),
                k(b.point),
            ));
        }
    };
    one(&path.anchors, path.closed);
    for s in &path.extra_subpaths {
        one(&s.anchors, s.closed);
    }
    out
}

/// Criterion 18 with `adrs.md` decision 9: under a non-uniform stretch of 8 (which keeps Size
/// and goes into the placement, criterion 34) the drawn tapered curves still lie within 0.01 mm
/// of the exact map in the document, both ways.
#[test]
fn edge_stretched_placement_keeps_the_document_accuracy() {
    use kurbo::{ParamCurve, ParamCurveNearest};
    let face = sans();
    let (size, t) = (30.0, 0.5);
    let placement = Affine::new(8.0, 0.0, 0.0, 1.0, 20.0, 30.0);
    let plain_t = snapshot("S", size, D, placement);
    let laid = text_layout(&plain_t, &face).unwrap();
    let g = &laid.lines[0].glyphs[0];
    let (cx, base) = (g.centre_x.0, g.baseline.0);
    let plain = subpath_segments(&lay_out(&plain_t, &face, false).unwrap().glyphs);
    let tapered = subpath_segments(
        &lay_out(&snapshot("S", size, tapered(t), placement), &face, false)
            .unwrap()
            .glyphs,
    );
    let to_doc = |p: kurbo::Point| kurbo::Point::new(8.0 * p.x + 20.0, p.y + 30.0);
    let to_text = |q: kurbo::Point| kurbo::Point::new((q.x - 20.0) / 8.0, q.y - 30.0);
    let exact = |q: kurbo::Point| {
        let p = to_text(q);
        let s = (1.0 + t * (2.0 * (base - p.y) / size - 1.0)).max(0.05);
        to_doc(kurbo::Point::new(cx + (p.x - cx) * s, p.y))
    };
    let mut polys: Vec<Vec<kurbo::Point>> = Vec::new();
    let mut worst: f64 = 0.0;
    for seg in &plain {
        let poly: Vec<kurbo::Point> = (0..=800)
            .map(|i| exact(seg.eval(f64::from(i) / 800.0)))
            .collect();
        for p in poly.iter().step_by(5) {
            let d = tapered
                .iter()
                .map(|c| c.nearest(*p, 1e-9).distance_sq.sqrt())
                .fold(f64::INFINITY, f64::min);
            worst = worst.max(d);
        }
        polys.push(poly);
    }
    for c in &tapered {
        for i in 0..=30 {
            let p = c.eval(f64::from(i) / 30.0);
            let d = polys
                .iter()
                .flat_map(|poly| poly.windows(2))
                .map(|w| {
                    kurbo::Line::new(w[0], w[1])
                        .nearest(p, 1e-12)
                        .distance_sq
                        .sqrt()
                })
                .fold(f64::INFINITY, f64::min);
            worst = worst.max(d);
        }
    }
    assert!(
        worst < TOL,
        "stretched x 8: worst {worst} mm in the document"
    );
}

/// Criterion 15 with the taper: the selection box covers the tapered glyphs, which at +50 % stand
/// out of their untapered box at the top.
#[test]
fn edge_box_covers_tapered_glyphs() {
    let face = sans();
    let out = lay_out(
        &snapshot("MW", 20.0, tapered(0.5), Affine::IDENTITY),
        &face,
        false,
    )
    .unwrap();
    let plain = lay_out(&snapshot("MW", 20.0, D, Affine::IDENTITY), &face, false).unwrap();
    let span = |b: &[Point; 4]| {
        (
            b.iter().map(|p| p.x).fold(f64::INFINITY, f64::min),
            b.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max),
            b.iter().map(|p| p.y).fold(f64::INFINITY, f64::min),
            b.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let (x0, x1, y0, y1) = span(&out.line_box);
    for p in subpath_segments(&out.glyphs)
        .iter()
        .flat_map(|c| [c.p0, c.p3])
    {
        assert!(
            p.x >= x0 - TOL && p.x <= x1 + TOL && p.y >= y0 - TOL && p.y <= y1 + TOL,
            "{p:?} outside the box {:?}",
            out.line_box
        );
    }
    let (px0, px1, _, _) = span(&plain.line_box);
    assert!(x1 - x0 > px1 - px0, "the tapered top widens the box");
}
