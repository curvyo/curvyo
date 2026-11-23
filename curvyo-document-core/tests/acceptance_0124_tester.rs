//! Independent tester tests for `specs/0124-gradient-fill` in the document model (criteria 5,
//! 22, 24 e, 25 and the merge of ADR 0066 §2): an own reference evaluation of linear, radial and
//! elliptical gradients with many stops compared with the production colour, the stop list
//! invariants, unlimited stops, the format round trip, every damage reason refused on open
//! (the goldens and damage written into the Loro registers by hand), and a drag on one peer
//! merged with a stop edit on another. Written from the specification and the ADRs; only
//! public names were looked up.

#![allow(clippy::many_single_char_names)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, missing_docs, clippy::doc_markdown)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
#![allow(clippy::cast_precision_loss)]

use std::io::{Cursor, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    AxisRatio, BoxPoint, CURRENT_FORMAT_VERSION, Color, Document, FillPaint, GradientBox,
    GradientGeometry, GradientStop, GradientStops, Length, NodeId, ObjectSnapshot, PlacedGradient,
    Point, PremultipliedRgba8, RectBounds, StopOffset, StyleEdit, over_white_grey, pack, unpack,
};

// ---------- the tester's own reference ----------

/// Premultiplied channels of a stop in `f64`, 0..255.
fn premul(s: &GradientStop) -> [f64; 4] {
    let a = f64::from(s.alpha);
    [
        f64::from(s.colour.r) * a / 255.0,
        f64::from(s.colour.g) * a / 255.0,
        f64::from(s.colour.b) * a / 255.0,
        a,
    ]
}

/// The colour at `t` as the spec states it: pad outside, premultiplied linear blend between
/// the neighbouring stops, at a shared offset the later stop wins.
fn reference_colour(stops: &[GradientStop], t: f64) -> [f64; 4] {
    let t = t.clamp(0.0, 1.0);
    // The last stop whose offset is <= t (later wins on a hard edge).
    let mut i = 0;
    for (k, s) in stops.iter().enumerate() {
        if s.offset.get() <= t {
            i = k;
        }
    }
    if i + 1 >= stops.len() {
        return premul(&stops[stops.len() - 1]);
    }
    let (a, b) = (&stops[i], &stops[i + 1]);
    let span = b.offset.get() - a.offset.get();
    let f = if span <= 0.0 {
        1.0
    } else {
        (t - a.offset.get()) / span
    };
    let (pa, pb) = (premul(a), premul(b));
    [0, 1, 2, 3].map(|c| pa[c] + (pb[c] - pa[c]) * f)
}

/// Box frame (axis aligned) of `min .. max`.
fn frame(min: (f64, f64), max: (f64, f64)) -> GradientBox {
    GradientBox::axis_aligned(Point::new(min.0, min.1), Point::new(max.0, max.1))
}

fn mm(b: &GradientBox, p: BoxPoint) -> (f64, f64) {
    (
        b.min.x + p.x.get() * (b.max.x - b.min.x),
        b.min.y + p.y.get() * (b.max.y - b.min.y),
    )
}

/// The tester's own gradient parameter `t` at a document point of an axis-aligned box.
fn reference_t(g: &GradientGeometry, b: &GradientBox, p: (f64, f64)) -> f64 {
    match *g {
        GradientGeometry::Linear { start, end } => {
            let (s, e) = (mm(b, start), mm(b, end));
            let (dx, dy) = (e.0 - s.0, e.1 - s.1);
            ((p.0 - s.0) * dx + (p.1 - s.1) * dy) / (dx * dx + dy * dy)
        }
        GradientGeometry::Radial {
            centre,
            radius,
            ratio,
        } => {
            let (c, r) = (mm(b, centre), mm(b, radius));
            let (ax, ay) = (r.0 - c.0, r.1 - c.1);
            let len2 = ax * ax + ay * ay;
            let (qx, qy) = (p.0 - c.0, p.1 - c.1);
            let u = (qx * ax + qy * ay) / len2;
            // Second axis: a quarter turn of the first, scaled by the ratio.
            let v = (qx * -ay + qy * ax) / len2 / ratio.get();
            (u * u + v * v).sqrt()
        }
    }
}

fn stop(hex: u32, offset: f64) -> GradientStop {
    GradientStop {
        colour: Color {
            r: (hex >> 24) as u8,
            g: (hex >> 16) as u8,
            b: (hex >> 8) as u8,
        },
        alpha: hex as u8,
        offset: StopOffset::new(offset).unwrap(),
    }
}

fn close(got: PremultipliedRgba8, want: [f64; 4], tol: f64, ctx: &str) {
    let g = [got.r, got.g, got.b, got.a].map(f64::from);
    for c in 0..4 {
        assert!(
            (g[c] - want[c]).abs() <= tol,
            "{ctx}: channel {c} got {g:?} want {want:?}"
        );
    }
}

/// A deterministic pseudo random stream (no dependency).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

fn many_stops(rng: &mut Lcg, n: usize) -> Vec<GradientStop> {
    let mut offs: Vec<f64> = (0..n - 2).map(|_| rng.next()).collect();
    // One hard edge among them.
    if n > 4 {
        offs[1] = offs[0];
    }
    offs.push(0.0);
    offs.push(1.0);
    offs.sort_by(f64::total_cmp);
    offs.iter()
        .map(|&o| stop((rng.next() * f64::from(u32::MAX)) as u32, o))
        .collect()
}

// ---------- criterion 5: the shade ----------

#[test]
fn c5_black_to_white_reads_half_grey_in_the_middle() {
    let fill = curvyo_document_core::GradientFill {
        geometry: GradientGeometry::DEFAULT_LINEAR,
        stops: GradientStops::new(vec![stop(0x0000_00FF, 0.0), stop(0xFFFF_FFFF, 1.0)]).unwrap(),
    };
    let placed = PlacedGradient::new(&fill, &frame((0.0, 0.0), (100.0, 40.0)));
    let c = placed.colour_at(Point::new(50.0, 20.0));
    for ch in [c.r, c.g, c.b] {
        assert!((i32::from(ch) - 128).abs() <= 1, "{c:?}");
    }
    assert_eq!(c.a, 255);
    // Pad: beyond both ends the end colours continue.
    assert_eq!(placed.colour_at(Point::new(-500.0, 20.0)).r, 0);
    assert_eq!(placed.colour_at(Point::new(900.0, 20.0)).r, 255);
}

#[test]
fn c5_three_stops_read_pure_red_in_the_middle() {
    let fill = curvyo_document_core::GradientFill {
        geometry: GradientGeometry::DEFAULT_LINEAR,
        stops: GradientStops::new(vec![
            stop(0x0000_00FF, 0.0),
            stop(0xFF00_00FF, 0.5),
            stop(0xFFFF_FFFF, 1.0),
        ])
        .unwrap(),
    };
    let placed = PlacedGradient::new(&fill, &frame((0.0, 0.0), (100.0, 40.0)));
    let c = placed.colour_at(Point::new(50.0, 7.0));
    assert_eq!((c.r, c.g, c.b, c.a), (255, 0, 0, 255));
}

#[test]
fn c5_a_radial_reads_one_colour_on_a_circle_in_millimetres() {
    // A non-square box: the circle is in the box frame in mm (criterion 4).
    let fill = curvyo_document_core::GradientFill {
        geometry: GradientGeometry::DEFAULT_RADIAL,
        stops: GradientStops::new(vec![stop(0x1020_30FF, 0.0), stop(0xF0A0_10FF, 1.0)]).unwrap(),
    };
    let placed = PlacedGradient::new(&fill, &frame((0.0, 0.0), (100.0, 40.0)));
    let r = 30.0;
    let first = placed.colour_at(Point::new(50.0 + r, 20.0));
    for k in 0..36 {
        let a = f64::from(k) * std::f64::consts::TAU / 36.0;
        let c = placed.colour_at(Point::new(50.0 + r * a.cos(), 20.0 + r * a.sin()));
        for (x, y) in [
            (c.r, first.r),
            (c.g, first.g),
            (c.b, first.b),
            (c.a, first.a),
        ] {
            assert!(
                (i32::from(x) - i32::from(y)).abs() <= 1,
                "angle {k}: {c:?} vs {first:?}"
            );
        }
    }
}

#[test]
fn c5_fade_to_transparent_has_no_grey_fringe() {
    // Red opaque to red transparent: premultiplied, so the colour stays red where alpha > 0.
    let stops = GradientStops::new(vec![stop(0xFF00_00FF, 0.0), stop(0x0000_0000, 1.0)]).unwrap();
    let c = stops.colour_at(0.5);
    assert!(c.g == 0 && c.b == 0, "{c:?}");
    assert!((i32::from(c.a) - 128).abs() <= 1);
    assert!(
        (i32::from(c.r) - i32::from(c.a)).abs() <= 1,
        "red premultiplied by alpha: {c:?}"
    );
}

#[test]
fn c5_independent_reference_matches_linear_radial_and_elliptical_with_many_stops() {
    let mut rng = Lcg(0x0124);
    let boxes = [
        frame((0.0, 0.0), (100.0, 40.0)),
        frame((-30.0, 12.5), (7.0, 90.0)),
        frame((1.0e5, -2.0e5), (1.0e5 + 300.0, -2.0e5 + 5.0)),
    ];
    let mut checked = 0;
    for case in 0..30 {
        let n = [2, 3, 7, 33, 256][case % 5];
        let stops = many_stops(&mut rng, n);
        let geometry = match case % 3 {
            0 => GradientGeometry::Linear {
                start: BoxPoint::new(rng.next() * 1.4 - 0.2, rng.next()).unwrap(),
                end: BoxPoint::new(rng.next() * 1.4 - 0.2, rng.next() + 0.3).unwrap(),
            },
            1 => GradientGeometry::Radial {
                centre: BoxPoint::new(rng.next(), rng.next()).unwrap(),
                radius: BoxPoint::new(rng.next() + 1.1, rng.next()).unwrap(),
                ratio: AxisRatio::CIRCLE,
            },
            _ => GradientGeometry::Radial {
                centre: BoxPoint::new(0.5, 0.5).unwrap(),
                radius: BoxPoint::new(rng.next() + 0.8, rng.next() * 0.4).unwrap(),
                ratio: AxisRatio::new(0.2 + rng.next() * 3.0).unwrap(),
            },
        };
        let fill = curvyo_document_core::GradientFill {
            geometry,
            stops: GradientStops::new(stops.clone()).unwrap(),
        };
        for b in &boxes {
            let placed = PlacedGradient::new(&fill, b);
            let (w, h) = (b.max.x - b.min.x, b.max.y - b.min.y);
            for _ in 0..60 {
                let p = (
                    b.min.x + (rng.next() * 1.6 - 0.3) * w,
                    b.min.y + (rng.next() * 1.6 - 0.3) * h,
                );
                let t = reference_t(&geometry, b, p);
                // Skip points within quantisation of a stop boundary (a hard edge flips there).
                if stops.iter().any(|s| (s.offset.get() - t).abs() < 1.0e-4) {
                    continue;
                }
                let tp = placed.t_at(Point::new(p.0, p.1));
                assert!(
                    (tp - t).abs() < 1.0e-6 * (1.0 + t.abs()),
                    "case {case} t at {p:?}: production {tp}, reference {t}"
                );
                // ADR 0066 §4 quantises `t` to 1/65,536: accept the reference over that step,
                // plus one level of rounding.
                let q = 1.0 / 65_536.0;
                let lo = reference_colour(&stops, t - q);
                let hi = reference_colour(&stops, t + q);
                let mid = reference_colour(&stops, t);
                let got = placed.colour_at(Point::new(p.0, p.1));
                let g = [got.r, got.g, got.b, got.a].map(f64::from);
                for c in 0..4 {
                    let (a, b) = (lo[c].min(hi[c]).min(mid[c]), lo[c].max(hi[c]).max(mid[c]));
                    assert!(
                        g[c] >= a - 1.0 && g[c] <= b + 1.0,
                        "case {case} n {n} at {p:?} t {t}: channel {c} got {g:?} want {mid:?}"
                    );
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 4000, "{checked}");
}

#[test]
fn c5_hard_edge_the_later_stop_wins() {
    let stops = GradientStops::new(vec![
        stop(0x0000_00FF, 0.0),
        stop(0x0000_00FF, 0.5),
        stop(0xFFFF_FFFF, 0.5),
        stop(0xFFFF_FFFF, 1.0),
    ])
    .unwrap();
    assert_eq!(
        stops.colour_at(0.5).r,
        255,
        "at the shared offset the later stop wins"
    );
    assert_eq!(stops.colour_at(0.4999).r, 0);
    assert_eq!(stops.colour_at(0.5001).r, 255);
}

#[test]
fn c10_over_white_grey_matches_the_luminance_weights() {
    let mut rng = Lcg(77);
    for _ in 0..5000 {
        let a = (rng.next() * 255.99) as u8;
        let f = f64::from(a) / 255.0;
        let (r, g, b) = (
            (rng.next() * 255.99 * f) as u8,
            (rng.next() * 255.99 * f) as u8,
            (rng.next() * 255.99 * f) as u8,
        );
        let c = PremultipliedRgba8 { r, g, b, a };
        let w = 255.0 - f64::from(a);
        let want =
            0.2126 * (f64::from(r) + w) + 0.7152 * (f64::from(g) + w) + 0.0722 * (f64::from(b) + w);
        let got = f64::from(over_white_grey(c));
        assert!((got - want).abs() <= 1.0, "{c:?}: {got} vs {want}");
    }
    assert_eq!(
        over_white_grey(PremultipliedRgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0
        }),
        255
    );
    assert_eq!(
        over_white_grey(PremultipliedRgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 255
        }),
        0
    );
}

// ---------- criterion 22 / Words: the stop list invariants ----------

#[test]
fn c22_zero_and_one_stop_and_bad_orders_are_refused_equal_offsets_are_allowed() {
    assert!(GradientStops::new(vec![]).is_err(), "0 stops");
    assert!(GradientStops::new(vec![stop(0xFF, 0.0)]).is_err(), "1 stop");
    assert!(
        GradientStops::new(vec![stop(0xFF, 1.0), stop(0xFF, 0.0)]).is_err(),
        "unordered"
    );
    assert!(
        GradientStops::new(vec![
            stop(0xFF, 0.0),
            stop(0xFF, 0.7),
            stop(0xFF, 0.3),
            stop(0xFF, 1.0)
        ])
        .is_err(),
        "inner offsets out of order"
    );
    assert!(
        GradientStops::new(vec![stop(0xFF, 0.1), stop(0xFF, 1.0)]).is_err(),
        "first not 0"
    );
    assert!(
        GradientStops::new(vec![stop(0xFF, 0.0), stop(0xFF, 0.9)]).is_err(),
        "last not 1"
    );
    assert!(StopOffset::new(-0.01).is_err());
    assert!(StopOffset::new(1.01).is_err());
    assert!(StopOffset::new(f64::NAN).is_err());
    assert!(
        GradientStops::new(vec![
            stop(0xFF, 0.0),
            stop(0xFF, 0.4),
            stop(0xFF, 0.4),
            stop(0xFF, 1.0)
        ])
        .is_ok(),
        "equal offsets are a hard edge"
    );
    assert!(GradientStops::new(vec![stop(0xFF, 0.0), stop(0xFF, 0.0), stop(0xFF, 1.0)]).is_ok());
    assert!(BoxPoint::new(f64::INFINITY, 0.0).is_err());
    assert!(BoxPoint::new(-1.0e9, 1.0e9).is_ok(), "free beyond the box");
    assert!(AxisRatio::new(0.0).is_err());
    assert!(AxisRatio::new(-1.0).is_err());
}

// ---------- the model on a document ----------

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

fn shade(d: &Document, id: NodeId, geometry: GradientGeometry, stops: Vec<GradientStop>) {
    d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::GradientGeometry(geometry))
        .unwrap();
    d.edit_style(
        &[id],
        &StyleEdit::GradientStops(GradientStops::new(stops).unwrap()),
    )
    .unwrap();
}

fn obj(d: &Document, id: NodeId) -> ObjectSnapshot {
    d.read_objects()
        .objects
        .iter()
        .find(|o| o.id() == id)
        .cloned()
        .unwrap()
}

#[test]
fn c2_c22_first_gradient_defaults_and_kept_when_switched_away() {
    let d = Document::new(1);
    let id = rect(&d, 0.0, 0.0, 100.0, 40.0);
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(
        &[id],
        &StyleEdit::FillColor(Color {
            r: 10,
            g: 200,
            b: 30,
        }),
    )
    .unwrap();
    let colour_before = obj(&d, id).style().fill.color;
    d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    let g = obj(&d, id)
        .style()
        .fill
        .painted_gradient()
        .cloned()
        .expect("painted");
    assert_eq!(g.geometry, GradientGeometry::DEFAULT_LINEAR);
    let s = g.stops.as_slice();
    assert_eq!(s.len(), 2);
    assert_eq!(
        s[0].colour,
        Color {
            r: 10,
            g: 200,
            b: 30
        }
    );
    assert_eq!(
        (s[1].colour, s[1].alpha),
        (
            Color {
                r: 255,
                g: 255,
                b: 255
            },
            255
        )
    );
    // Change it, switch away and back: unchanged; the Colour stayed untouched.
    d.edit_style(&[id], &StyleEdit::AddGradientStop(None))
        .unwrap();
    let edited = obj(&d, id)
        .style()
        .fill
        .painted_gradient()
        .cloned()
        .unwrap();
    assert_eq!(edited.stops.len(), 3);
    assert_eq!(obj(&d, id).style().fill.color, colour_before);
    for away in [FillPaint::Colour, FillPaint::Pattern] {
        d.edit_style(&[id], &StyleEdit::FillPaint(away)).unwrap();
        assert!(obj(&d, id).style().fill.painted_gradient().is_none());
        d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Gradient))
            .unwrap();
        assert_eq!(
            obj(&d, id).style().fill.painted_gradient().unwrap(),
            &edited
        );
    }
    d.edit_style(&[id], &StyleEdit::FillEnabled(false)).unwrap();
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    assert_eq!(
        obj(&d, id).style().fill.painted_gradient().unwrap(),
        &edited,
        "None keeps it"
    );
}

#[test]
fn c24e_ten_thousand_stops_store_open_and_evaluate() {
    let d = Document::new(1);
    let id = rect(&d, 0.0, 0.0, 100.0, 40.0);
    let n = 10_000;
    let stops: Vec<GradientStop> = (0..n)
        .map(|k| {
            let o = if k == n - 1 {
                1.0
            } else {
                f64::from(k) / f64::from(n - 1)
            };
            stop(((k as u32) << 8) | 0xFF, o)
        })
        .collect();
    shade(&d, id, GradientGeometry::DEFAULT_LINEAR, stops.clone());
    let back = unpack(5, &pack(&d, "t").unwrap()).unwrap();
    let g = obj(&back, id)
        .style()
        .fill
        .painted_gradient()
        .cloned()
        .unwrap();
    assert_eq!(
        g.stops.as_slice(),
        stops.as_slice(),
        "every stop round-trips exactly"
    );
    let placed = PlacedGradient::new(&g, &obj(&back, id).gradient_box().unwrap());
    for k in [0, 1, 777, 5000, 9998] {
        let t = (f64::from(k) + 0.5) / f64::from(n - 1);
        let want = reference_colour(&stops, t);
        close(
            placed.colour_at(Point::new(t * 100.0, 20.0)),
            want,
            1.0,
            &format!("{k}"),
        );
    }
}

#[test]
fn c22_exact_round_trip_of_awkward_numbers() {
    let d = Document::new(1);
    let id = rect(&d, 3.0, 4.0, 10.0, 10.0);
    let geometry = GradientGeometry::Radial {
        centre: BoxPoint::new(0.1 + 0.2, -1.0e-300).unwrap(),
        radius: BoxPoint::new(1.0e300, 1.0 / 3.0).unwrap(),
        ratio: AxisRatio::new(f64::MIN_POSITIVE).unwrap(),
    };
    shade(
        &d,
        id,
        geometry,
        vec![
            stop(0xABCD_EF01, 0.0),
            stop(0x1234_5678, 0.1 + 0.2),
            stop(0xFFFF_FF00, 1.0),
        ],
    );
    let back = unpack(5, &pack(&d, "t").unwrap()).unwrap();
    let g = obj(&back, id)
        .style()
        .fill
        .painted_gradient()
        .cloned()
        .unwrap();
    assert_eq!(g.geometry, geometry, "fractions round-trip bit for bit");
    assert_eq!(g.stops.as_slice()[1].offset.get(), 0.1 + 0.2);
    assert_eq!(g.stops.as_slice()[0].alpha, 0x01);
}

// ---------- criterion 22: the goldens and every damage reason ----------

fn fixture(name: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&p).unwrap_or_else(|_| panic!("{} missing", p.display()))
}

#[test]
fn c22_format_bump_and_the_good_golden_opens_with_every_kind() {
    // Superseded by 0135 slice 1 (2026-11-19).
    assert_eq!(
        CURRENT_FORMAT_VERSION, 54,
        "format 49, then 50 by `0123`, 51 by `0135`"
    );
    let name = "gradient_fill_v49.curvyo".to_owned();
    let d = unpack(3, &fixture(&name)).expect("the golden opens");
    let read = d.read_objects();
    let shaded: Vec<_> = read
        .objects
        .iter()
        .filter_map(|o| o.style().fill.painted_gradient().map(|g| (o, g)))
        .collect();
    assert!(shaded.len() >= 5, "{}", shaded.len());
    assert!(
        shaded.iter().any(|(_, g)| g.stops.len() == 9),
        "a 9-stop gradient"
    );
    assert!(
        shaded
            .iter()
            .any(|(_, g)| matches!(g.geometry, GradientGeometry::Radial { .. }))
    );
    assert!(
        shaded
            .iter()
            .any(|(_, g)| matches!(g.geometry, GradientGeometry::Linear { .. }))
    );
    // An older format number is refused for this file (an old build must refuse it): the
    // manifest says the number of `0124` (49; 50 is `0123`).
    let bytes = fixture(&name);
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut manifest = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("manifest.json").unwrap(), &mut manifest)
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(v["format_version"], 49);
}

#[test]
fn c22_each_damaged_golden_is_refused() {
    for reason in [
        "colour",
        "fraction",
        "one_stop",
        "unordered",
        "paint_without_gradient",
    ] {
        let name = format!("gradient_fill_damaged_{reason}_v49.curvyo");
        assert!(
            unpack(3, &fixture(&name)).is_err(),
            "{name} must be refused"
        );
    }
}

/// Packs a raw Loro snapshot as a project.
fn container(loro_bytes: &[u8]) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": 1,
        "app_version": "tester",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    writer.start_file("manifest.json", options).unwrap();
    writer
        .write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    writer.start_file("document.loro", options).unwrap();
    writer.write_all(loro_bytes).unwrap();
    writer.start_file("document.json", options).unwrap();
    writer.write_all(b"{}").unwrap();
    writer.finish().unwrap().into_inner()
}

/// A gradient square, then `key` overwritten (or removed with `None`) in the raw registers.
fn damaged(key: &str, value: Option<&str>) -> Vec<u8> {
    let d = Document::new(1);
    let id = rect(&d, 0.0, 0.0, 10.0, 10.0);
    shade(
        &d,
        id,
        GradientGeometry::DEFAULT_LINEAR,
        vec![stop(0x0000_00FF, 0.0), stop(0xFFFF_FFFF, 1.0)],
    );
    let loro = loro::LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = loro.get_tree("paths");
    let mut hit = 0;
    for node in tree.nodes() {
        let Ok(meta) = tree.get_meta(node) else {
            continue;
        };
        if meta.get("fill_gradient").is_some() {
            match value {
                Some(v) => meta.insert(key, v).unwrap(),
                None => meta.delete(key).unwrap(),
            }
            hit += 1;
        }
    }
    assert_eq!(hit, 1, "one shaded node");
    loro.commit();
    container(&loro.export(loro::ExportMode::Snapshot).unwrap())
}

#[test]
fn c22_the_undamaged_raw_container_opens() {
    // Control: the hand-made container opens when the value is a valid one.
    let good = damaged("fill_gradient", Some("linear 0 0.5 1 0.5"));
    let d = unpack(3, &good).expect("valid registers open");
    assert!(
        d.read_objects()
            .objects
            .iter()
            .any(|o| o.style().fill.painted_gradient().is_some())
    );
    let good = damaged(
        "fill_gradient_stops",
        Some("#000000 0;#ffffff80 0.5;#FFFFFF 1"),
    );
    unpack(3, &good).expect("6 and 8 digits, either case");
    let good = damaged("fill_gradient", Some("radial 0.5 0.5 1 0.5 2.5"));
    unpack(3, &good).expect("radial ratio");
    let good = damaged(
        "fill_gradient_stops",
        Some("#000000 0;#111111 0.5;#222222 0.5;#FFFFFF 1"),
    );
    unpack(3, &good).expect("equal offsets");
}

#[test]
fn c22_every_damage_reason_written_by_hand_refuses_the_file() {
    let cases: &[(&str, Option<&str>, &str)] = &[
        ("fill_paint", Some("sparkle"), "unknown paint"),
        ("fill_gradient", Some("conic 0 0 1 1"), "unknown kind"),
        (
            "fill_gradient",
            Some("linear abc 0.5 1 0.5"),
            "fraction abc",
        ),
        (
            "fill_gradient",
            Some("linear NaN 0.5 1 0.5"),
            "fraction NaN",
        ),
        (
            "fill_gradient",
            Some("linear inf 0.5 1 0.5"),
            "fraction inf",
        ),
        (
            "fill_gradient",
            Some("linear 1e999 0.5 1 0.5"),
            "fraction overflows",
        ),
        ("fill_gradient", Some("linear 0 0.5 1"), "too few fields"),
        ("fill_gradient", Some("linear 0  0.5 1 0.5"), "two spaces"),
        (
            "fill_gradient",
            Some(" linear 0 0.5 1 0.5"),
            "leading space",
        ),
        (
            "fill_gradient",
            Some("linear 0 0.5 1 0.5 2"),
            "ratio on linear",
        ),
        ("fill_gradient", Some("radial 0.5 0.5 1 0.5 0"), "ratio 0"),
        (
            "fill_gradient",
            Some("radial 0.5 0.5 1 0.5 -1"),
            "ratio negative",
        ),
        (
            "fill_gradient_stops",
            Some("#00000 0;#FFFFFF 1"),
            "5 hex digits",
        ),
        (
            "fill_gradient_stops",
            Some("#0000000 0;#FFFFFF 1"),
            "7 hex digits",
        ),
        (
            "fill_gradient_stops",
            Some("#GG0000 0;#FFFFFF 1"),
            "not hex",
        ),
        ("fill_gradient_stops", Some("000000 0;#FFFFFF 1"), "no #"),
        ("fill_gradient_stops", Some("#000000 0"), "one stop"),
        ("fill_gradient_stops", Some(""), "no stop"),
        (
            "fill_gradient_stops",
            Some("#000000 0;#111111 0.7;#222222 0.3;#FFFFFF 1"),
            "unordered",
        ),
        (
            "fill_gradient_stops",
            Some("#000000 0.1;#FFFFFF 1"),
            "first not 0",
        ),
        (
            "fill_gradient_stops",
            Some("#000000 0;#FFFFFF 0.9"),
            "last not 1",
        ),
        (
            "fill_gradient_stops",
            Some("#000000 -0;#FFFFFF 1.5"),
            "out of range",
        ),
        (
            "fill_gradient_stops",
            Some("#000000 0;#FFFFFF 1;"),
            "trailing separator",
        ),
        (
            "fill_gradient_stops",
            Some("#000000 x;#FFFFFF 1"),
            "offset not a number",
        ),
        (
            "fill_gradient",
            None,
            "gradient paint without fill_gradient",
        ),
        ("fill_gradient_stops", None, "gradient paint without stops"),
        // The number grammar of ADR 0066 §1: no leading zero, no bare dot, no plus sign.
        ("fill_gradient", Some("linear 01 0.5 1 0.5"), "leading zero"),
        ("fill_gradient", Some("linear .5 0.5 1 0.5"), "bare dot"),
        ("fill_gradient", Some("linear 1. 0.5 1 0.5"), "trailing dot"),
        ("fill_gradient", Some("linear +1 0.5 1 0.5"), "plus sign"),
        ("fill_gradient", Some("linear 0x1 0.5 1 0.5"), "hex number"),
        (
            "fill_gradient",
            Some("Linear 0 0.5 1 0.5"),
            "kind in capitals",
        ),
        (
            "fill_gradient",
            Some("linear\t0 0.5 1 0.5"),
            "tab separator",
        ),
        (
            "fill_gradient_stops",
            Some("#000000 0; #FFFFFF 1"),
            "space after ;",
        ),
        (
            "fill_gradient_stops",
            Some("#000000  0;#FFFFFF 1"),
            "two spaces in a stop",
        ),
        (
            "fill_gradient_stops",
            Some("#000000 0;;#FFFFFF 1"),
            "empty stop",
        ),
    ];
    let mut opened = Vec::new();
    for (key, value, why) in cases {
        if unpack(3, &damaged(key, *value)).is_ok() {
            opened.push(*why);
        }
    }
    assert!(opened.is_empty(), "these damaged files opened: {opened:?}");
}

// ---------- ADR 0066 §2: two registers merge a drag and a stop edit ----------

fn merged(a: &Document, b: &Document) -> Document {
    let loro = loro::LoroDoc::new();
    loro.import(&a.export_loro_snapshot().unwrap()).unwrap();
    loro.import(&b.export_loro_snapshot().unwrap()).unwrap();
    loro.commit();
    unpack(
        9,
        &container(&loro.export(loro::ExportMode::Snapshot).unwrap()),
    )
    .expect("merged opens")
}

#[test]
fn merge_a_drag_on_one_peer_and_a_stop_edit_on_another_both_survive() {
    let d = Document::new(1);
    let id = rect(&d, 0.0, 0.0, 100.0, 40.0);
    shade(
        &d,
        id,
        GradientGeometry::DEFAULT_LINEAR,
        vec![
            stop(0x0000_00FF, 0.0),
            stop(0x8080_80FF, 0.5),
            stop(0xFFFF_FFFF, 1.0),
        ],
    );
    let bytes = pack(&d, "t").unwrap();
    let a = unpack(2, &bytes).unwrap();
    let b = unpack(3, &bytes).unwrap();
    let dragged = GradientGeometry::Linear {
        start: BoxPoint::new(0.2, 0.0).unwrap(),
        end: BoxPoint::new(0.8, 1.0).unwrap(),
    };
    a.edit_style(&[id], &StyleEdit::GradientGeometry(dragged))
        .unwrap();
    b.edit_style(
        &[id],
        &StyleEdit::GradientStopColour(1, Color { r: 255, g: 0, b: 0 }, 128),
    )
    .unwrap();
    b.edit_style(&[id], &StyleEdit::AddGradientStop(None))
        .unwrap();
    for m in [merged(&a, &b), merged(&b, &a)] {
        let g = obj(&m, id)
            .style()
            .fill
            .painted_gradient()
            .cloned()
            .unwrap();
        assert_eq!(g.geometry, dragged, "the drag survives");
        assert_eq!(g.stops.len(), 4, "the added stop survives");
        assert!(
            g.stops
                .as_slice()
                .iter()
                .any(|s| s.colour == Color { r: 255, g: 0, b: 0 } && s.alpha == 128),
            "the colour edit survives"
        );
    }
    // Two peers editing stops: one wins as a whole and the list stays valid.
    let a = unpack(2, &bytes).unwrap();
    let b = unpack(3, &bytes).unwrap();
    a.edit_style(&[id], &StyleEdit::GradientStopOffset(1, 0.1))
        .unwrap();
    b.edit_style(&[id], &StyleEdit::RemoveGradientStop(1))
        .unwrap();
    let m = merged(&a, &b);
    let g = obj(&m, id)
        .style()
        .fill
        .painted_gradient()
        .cloned()
        .unwrap();
    assert!(g.stops.len() == 2 || g.stops.len() == 3);
    let pack_again = pack(&m, "t").unwrap();
    unpack(4, &pack_again).expect("the merge result is a valid file");
}

#[test]
fn c4_kind_switch_keeps_stops_and_sets_the_new_default_geometry() {
    let d = Document::new(1);
    let id = rect(&d, 0.0, 0.0, 100.0, 40.0);
    let stops = vec![
        stop(0x0000_00FF, 0.0),
        stop(0x00FF_00FF, 0.3),
        stop(0xFFFF_FFFF, 1.0),
    ];
    shade(&d, id, GradientGeometry::DEFAULT_LINEAR, stops.clone());
    d.edit_style(
        &[id],
        &StyleEdit::GradientKind(curvyo_document_core::GradientKind::Radial),
    )
    .unwrap();
    let g = obj(&d, id)
        .style()
        .fill
        .painted_gradient()
        .cloned()
        .unwrap();
    assert_eq!(g.geometry, GradientGeometry::DEFAULT_RADIAL);
    assert_eq!(g.stops.as_slice(), stops.as_slice());
    d.edit_style(&[id], &StyleEdit::ReverseGradient).unwrap();
    let r = obj(&d, id)
        .style()
        .fill
        .painted_gradient()
        .cloned()
        .unwrap();
    let offs: Vec<f64> = r.stops.as_slice().iter().map(|s| s.offset.get()).collect();
    assert!((offs[1] - 0.7).abs() < 1e-12, "{offs:?}");
    assert_eq!(
        r.stops.as_slice()[0].colour,
        Color {
            r: 255,
            g: 255,
            b: 255
        }
    );
    // Position clamped between neighbours.
    d.edit_style(&[id], &StyleEdit::GradientStopOffset(1, 5.0))
        .unwrap();
    let c = obj(&d, id)
        .style()
        .fill
        .painted_gradient()
        .cloned()
        .unwrap();
    assert_eq!(c.stops.as_slice()[1].offset.get(), 1.0);
    // The ends cannot be removed; an inner one can, down to two.
    let _ = d.edit_style(&[id], &StyleEdit::RemoveGradientStop(0));
    let _ = d.edit_style(&[id], &StyleEdit::RemoveGradientStop(2));
    assert_eq!(
        obj(&d, id)
            .style()
            .fill
            .painted_gradient()
            .unwrap()
            .stops
            .len(),
        3
    );
    d.edit_style(&[id], &StyleEdit::RemoveGradientStop(1))
        .unwrap();
    let _ = d.edit_style(&[id], &StyleEdit::RemoveGradientStop(1));
    assert_eq!(
        obj(&d, id)
            .style()
            .fill
            .painted_gradient()
            .unwrap()
            .stops
            .len(),
        2
    );
}

#[test]
fn c3_add_stop_lands_in_the_widest_gap_in_the_shade_colour() {
    let d = Document::new(1);
    let id = rect(&d, 0.0, 0.0, 100.0, 40.0);
    let stops = vec![
        stop(0x0000_00FF, 0.0),
        stop(0x0000_00FF, 0.2),
        stop(0xFFFF_FFFF, 1.0),
    ];
    shade(&d, id, GradientGeometry::DEFAULT_LINEAR, stops.clone());
    d.edit_style(&[id], &StyleEdit::AddGradientStop(None))
        .unwrap();
    let g = obj(&d, id)
        .style()
        .fill
        .painted_gradient()
        .cloned()
        .unwrap();
    let s = g.stops.as_slice();
    assert_eq!(s.len(), 4);
    assert!(
        (s[2].offset.get() - 0.6).abs() < 1e-9,
        "middle of 0.2..1: {:?}",
        s[2].offset
    );
    let want = reference_colour(&stops, 0.6);
    for (got, w) in [(s[2].colour.r, want[0]), (s[2].alpha, want[3])] {
        assert!((f64::from(got) - w).abs() <= 1.0, "{got} {w}");
    }
}

#[test]
fn degenerate_boxes_and_lines_do_not_panic() {
    let stops = GradientStops::new(vec![stop(0x0000_00FF, 0.0), stop(0xFFFF_FFFF, 1.0)]).unwrap();
    for geometry in [
        GradientGeometry::DEFAULT_LINEAR,
        GradientGeometry::DEFAULT_RADIAL,
        GradientGeometry::Linear {
            start: BoxPoint::new(0.5, 0.5).unwrap(),
            end: BoxPoint::new(0.5, 0.5).unwrap(),
        },
        GradientGeometry::Radial {
            centre: BoxPoint::new(0.5, 0.5).unwrap(),
            radius: BoxPoint::new(0.5, 0.5).unwrap(),
            ratio: AxisRatio::CIRCLE,
        },
    ] {
        let fill = curvyo_document_core::GradientFill {
            geometry,
            stops: stops.clone(),
        };
        for b in [
            frame((5.0, 5.0), (5.0, 5.0)),
            frame((0.0, 0.0), (100.0, 0.0)),
            frame((1.0e12, 1.0e12), (1.0e12 + 1.0, 1.0e12 + 1.0)),
        ] {
            let placed = PlacedGradient::new(&fill, &b);
            for p in [b.min, b.max, Point::new(0.0, 0.0)] {
                let c = placed.colour_at(p);
                assert!(c.r >= c.a.min(c.r), "{c:?}");
                assert!(
                    placed.t_at(p).is_finite() || placed.t_at(p).is_nan(),
                    "no panic"
                );
            }
        }
    }
}
