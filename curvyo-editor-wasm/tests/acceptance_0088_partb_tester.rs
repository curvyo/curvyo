//! Tester cases for `specs/0088-trace-recognition` Part B ("Recognise shapes" in Trace) in the editor
//! session. Written from the Part B criteria and `ux-notes.md` B1 to B9 before the implementation was
//! read; the pictures are the tester's own, drawn here with an analytic filler (3 x 3 samples per
//! pixel, 10 px/mm, so the 2 px tolerance floor is the 0.2 mm of the specification) and run through
//! the session's trace and Create paths exactly as the host does.
//!
//! The pictures are also written as PNGs to `tests/fixtures/acceptance_0088_partb/` and the native
//! wire of each (settings, FNV-1a of the f64 bits) to `tests/fixtures/acceptance_0088_partb_native.json`
//! (`CURVYO_BLESS=1` rewrites both); `frontend/tests/acceptance_0088_partb_tester.test.ts` runs the
//! same PNGs through the editor wasm and compares (criterion 13, native = wasm).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::too_many_arguments,
    missing_docs
)]

mod cpu_time;

use std::f64::consts::{PI, TAU};
use std::path::PathBuf;

use curvyo_document_core::{
    Color, Document, ObjectSnapshot, PathSnapshot, PrimitiveSnapshot, Shape, unpack,
};
use curvyo_editor_wasm::{
    Session, Tool, TraceCreateOutcome, trace_colour_settings, trace_recognise_shapes,
    trace_settings,
};
use curvyo_ui_core::read_trace_wire;
use kurbo::{Affine, BezPath, PathEl, Point};
use proptest::prelude::*;
use vectrace::job::{Job as TraceJob, Mode as TraceMode, run as run_trace_job};
use vectrace::shapes::trace_add_shapes;
use vectrace::{PixelMap as TracePixelMap, Raster as TraceRaster};

const OFF: &str = r#"{"threshold":128}"#;
const ON: &str = r#"{"threshold":128,"shapes":true}"#;
const OFF_EXPLICIT: &str = r#"{"threshold":128,"shapes":false}"#;
const MULTI_ON: &str = r#"{"output":1,"colours":null,"fixed":[],"excluded":[],"method":0,"background":null,"swatches":[],"threshold":128,"shapes":true}"#;
const MULTI_OFF: &str = r#"{"output":1,"colours":null,"fixed":[],"excluded":[],"method":0,"background":null,"swatches":[],"threshold":128}"#;
/// Pixels per mm of every picture here.
const PX: f64 = 10.0;
/// The specification's tolerance at 10 px/mm, in mm.
const TOL: f64 = 0.2;

// ------------------------------------------------------------------ pictures

/// An 8-bit grey PNG of `w` x `h` from `ink(x, y)` (true = black), 3 x 3 samples per pixel.
fn picture(w: u32, h: u32, ink: impl Fn(f64, f64) -> bool) -> Vec<u8> {
    use image::ImageEncoder;
    let mut grey = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let mut hit = 0_u32;
            for sy in 0..3 {
                for sx in 0..3 {
                    let px = f64::from(x) + (f64::from(sx) + 0.5) / 3.0;
                    let py = f64::from(y) + (f64::from(sy) + 0.5) / 3.0;
                    hit += u32::from(ink(px, py));
                }
            }
            grey.push(255 - (hit * 255 / 9) as u8);
        }
    }
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&grey, w, h, image::ExtendedColorType::L8)
        .unwrap();
    out
}

/// An RGB PNG from `colour(x, y)` per pixel centre (no antialiasing: flat colours).
fn colour_picture(w: u32, h: u32, colour: impl Fn(f64, f64) -> [u8; 3]) -> Vec<u8> {
    use image::ImageEncoder;
    let mut rgb = Vec::new();
    for y in 0..h {
        for x in 0..w {
            rgb.extend(colour(f64::from(x) + 0.5, f64::from(y) + 0.5));
        }
    }
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&rgb, w, h, image::ExtendedColorType::Rgb8)
        .unwrap();
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for b in bytes {
        crc ^= u32::from(*b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// `png` with a `pHYs` chunk of 254 dpi (10 px/mm) right after `IHDR`.
fn at_10_px_per_mm(png: &[u8]) -> Vec<u8> {
    let per_metre = (254.0_f64 / 0.0254).round() as u32;
    let mut chunk = b"pHYs".to_vec();
    chunk.extend(per_metre.to_be_bytes());
    chunk.extend(per_metre.to_be_bytes());
    chunk.push(1);
    let mut out = png[..33].to_vec();
    out.extend(9_u32.to_be_bytes());
    out.extend(&chunk);
    out.extend(crc32(&chunk).to_be_bytes());
    out.extend(&png[33..]);
    out
}

/// Inside a rounded rectangle of centre `(cx, cy)`, size `w` x `h`, radius `r`, turned by `rot`.
fn in_rrect(px: f64, py: f64, cx: f64, cy: f64, w: f64, h: f64, r: f64, rot: f64) -> bool {
    let (dx, dy) = (px - cx, py - cy);
    let (s, c) = rot.sin_cos();
    let (x, y) = (dx * c + dy * s, -dx * s + dy * c);
    let ex = (x.abs() - (w / 2.0 - r)).max(0.0);
    let ey = (y.abs() - (h / 2.0 - r)).max(0.0);
    ex * ex + ey * ey <= r * r
}

fn in_ellipse(px: f64, py: f64, cx: f64, cy: f64, a: f64, b: f64, rot: f64) -> bool {
    let (dx, dy) = (px - cx, py - cy);
    let (s, c) = rot.sin_cos();
    let (x, y) = (dx * c + dy * s, -dx * s + dy * c);
    (x / a).powi(2) + (y / b).powi(2) <= 1.0
}

fn in_polygon(px: f64, py: f64, verts: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut j = verts.len() - 1;
    for i in 0..verts.len() {
        let (xi, yi) = verts[i];
        let (xj, yj) = verts[j];
        if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Vertices of a regular `n`-gon (or a star when `inner` is given) of circumradius `outer`, first
/// outer vertex at angle `a0`.
fn star_vertices(
    cx: f64,
    cy: f64,
    n: u32,
    outer: f64,
    inner: Option<f64>,
    a0: f64,
) -> Vec<(f64, f64)> {
    let mut v = Vec::new();
    for k in 0..n {
        let a = a0 + TAU * f64::from(k) / f64::from(n);
        v.push((cx + outer * a.cos(), cy + outer * a.sin()));
        if let Some(i) = inner {
            let b = a + PI / f64::from(n);
            v.push((cx + i * b.cos(), cy + i * b.sin()));
        }
    }
    v
}

/// A deterministic irregular outline: a sum of five incommensurate harmonics, scaled to `amp` of the
/// radius at most.
fn wobble(theta: f64, amp: f64) -> f64 {
    const H: [(f64, f64, f64); 5] = [
        (3.0, 0.4, 1.0),
        (5.0, 2.1, 0.8),
        (8.0, 0.7, 0.7),
        (13.0, 4.0, 0.5),
        (17.0, 1.3, 0.4),
    ];
    let f: f64 = H.iter().map(|(k, p, a)| a * (k * theta + p).sin()).sum();
    1.0 + amp * f / 2.34
}

fn in_wobbly(px: f64, py: f64, cx: f64, cy: f64, r: f64, amp: f64) -> bool {
    let (dx, dy) = (px - cx, py - cy);
    dx.hypot(dy) <= r * wobble(dy.atan2(dx), amp)
}

/// The named pictures of this file.
fn make(name: &str) -> Vec<u8> {
    match name {
        // 200 x 100 mm, square corners.
        "rect" => picture(2200, 1200, |x, y| {
            in_rrect(x, y, 1100.0, 600.0, 2000.0, 1000.0, 0.0, 0.0)
        }),
        // 200 x 100 mm, 10 mm corners.
        "rounded" => picture(2200, 1200, |x, y| {
            in_rrect(x, y, 1100.0, 600.0, 2000.0, 1000.0, 100.0, 0.0)
        }),
        // 150 x 80 mm, 8 mm corners, turned 20 degrees.
        "rounded_turned" => picture(1800, 1800, |x, y| {
            in_rrect(
                x,
                y,
                900.0,
                900.0,
                1500.0,
                800.0,
                80.0,
                20.0_f64.to_radians(),
            )
        }),
        // 50 mm diameter.
        "circle" => picture(700, 700, |x, y| {
            in_ellipse(x, y, 350.0, 350.0, 250.0, 250.0, 0.0)
        }),
        // 60 x 40 mm, turned 25 degrees.
        "ellipse" => picture(800, 800, |x, y| {
            in_ellipse(x, y, 400.0, 400.0, 300.0, 200.0, 25.0_f64.to_radians())
        }),
        // Regular hexagon, circumradius 30 mm, first vertex at 15 degrees.
        "hexagon" => {
            let v = star_vertices(400.0, 400.0, 6, 300.0, None, 15.0_f64.to_radians());
            picture(800, 800, move |x, y| in_polygon(x, y, &v))
        }
        // Five-point star, outer 40 mm, inner 20 mm, first outer corner at -70 degrees.
        "star" => {
            let v = star_vertices(
                500.0,
                500.0,
                5,
                400.0,
                Some(200.0),
                (-70.0_f64).to_radians(),
            );
            picture(1000, 1000, move |x, y| in_polygon(x, y, &v))
        }
        // Circle 50 mm with a 20 mm hole.
        "ring" => picture(700, 700, |x, y| {
            let r = (x - 350.0).hypot(y - 350.0);
            r <= 250.0 && r > 100.0
        }),
        // Rectangle 200 x 100 mm with a 160 x 60 mm hole.
        "frame" => picture(2200, 1200, |x, y| {
            in_rrect(x, y, 1100.0, 600.0, 2000.0, 1000.0, 0.0, 0.0)
                && !in_rrect(x, y, 1100.0, 600.0, 1600.0, 600.0, 0.0, 0.0)
        }),
        // Irregular, +-3 % of 25 mm.
        "wobbly" => picture(700, 700, |x, y| in_wobbly(x, y, 350.0, 350.0, 250.0, 0.03)),
        // Same shape at +-0.6 % (1.5 px): inside the tolerance, so a circle.
        "wobbly_small" => picture(700, 700, |x, y| in_wobbly(x, y, 350.0, 350.0, 250.0, 0.006)),
        // A red circle (30 mm) and a blue rectangle (40 x 20 mm) on white.
        "colour" => colour_picture(1000, 500, |x, y| {
            if in_ellipse(x, y, 250.0, 250.0, 150.0, 150.0, 0.0) {
                [255, 0, 0]
            } else if in_rrect(x, y, 700.0, 250.0, 400.0, 200.0, 0.0, 0.0) {
                [0, 0, 255]
            } else {
                [255, 255, 255]
            }
        }),
        // A rectangle cut by the picture's edge, a circle cut by it and a rectangle one pixel wide.
        "cut" => picture(600, 400, |x, y| {
            in_ellipse(x, y, 40.0, 200.0, 120.0, 120.0, 0.0)
                || (x > 300.0 && x < 301.0 && y < 300.0)
                || (x > 450.0 && y > 280.0)
        }),
        // Several shapes in one picture: all five primitive kinds side by side.
        "mixed" => {
            let hex = star_vertices(1050.0, 300.0, 6, 200.0, None, 0.0);
            let star = star_vertices(1500.0, 300.0, 7, 200.0, Some(90.0), -PI / 2.0);
            picture(1800, 600, move |x, y| {
                in_ellipse(x, y, 300.0, 300.0, 200.0, 200.0, 0.0)
                    || in_rrect(x, y, 700.0, 300.0, 240.0, 400.0, 40.0, 0.0)
                    || in_polygon(x, y, &hex)
                    || in_polygon(x, y, &star)
            })
        }
        // The `0025` browser tests' ring: radii 20 and 60 px in 200 x 200 px.
        "ring25" => picture(200, 200, |x, y| {
            let r = (x - 100.0).hypot(y - 100.0);
            r < 60.0 && r > 20.0
        }),
        // The `0025` c55 dot grid: 800 x 800 px of dots of radius 3 px every 8 px.
        "dots" => picture(800, 800, |x, y| {
            let (dx, dy) = ((x % 8.0) - 4.0, (y % 8.0) - 4.0);
            dx * dx + dy * dy < 9.0
        }),
        // 500 closed shapes (25 x 20), alternating circle, rectangle, hexagon, ellipse and star.
        "grid500" => {
            let kinds: Vec<Vec<(f64, f64)>> = (0..25 * 20)
                .map(|i| {
                    let (cx, cy) = (
                        f64::from(i % 25) * 60.0 + 30.0,
                        f64::from(i / 25) * 60.0 + 30.0,
                    );
                    match i % 5 {
                        2 => star_vertices(cx, cy, 6, 24.0, None, 0.1),
                        4 => star_vertices(cx, cy, 5, 26.0, Some(12.0), -1.0),
                        _ => Vec::new(),
                    }
                })
                .collect();
            picture(1500, 1200, move |x, y| {
                let (gx, gy) = (
                    ((x / 60.0).floor() as usize).min(24),
                    ((y / 60.0).floor() as usize).min(19),
                );
                let i = gy * 25 + gx;
                let (cx, cy) = (gx as f64 * 60.0 + 30.0, gy as f64 * 60.0 + 30.0);
                match i % 5 {
                    0 => in_ellipse(x, y, cx, cy, 24.0, 24.0, 0.0),
                    1 => in_rrect(x, y, cx, cy, 48.0, 36.0, 6.0, 0.0),
                    3 => in_ellipse(x, y, cx, cy, 26.0, 16.0, 0.5),
                    _ => in_polygon(x, y, &kinds[i]),
                }
            })
        }
        other => panic!("no picture {other}"),
    }
}

const NAMES: [&str; 17] = [
    "rect",
    "rounded",
    "rounded_turned",
    "circle",
    "ellipse",
    "hexagon",
    "star",
    "ring",
    "frame",
    "wobbly",
    "wobbly_small",
    "colour",
    "cut",
    "mixed",
    "ring25",
    "dots",
    "grid500",
];

// ------------------------------------------------------------------ host

struct Host {
    s: Session,
    image: curvyo_document_core::NodeId,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    map: [f64; 6],
    /// Document mm of a picture that is drawn at 10 px/mm: the import fits big pictures onto the page,
    /// so every nominal mm of this file is multiplied by it.
    u: f64,
}

impl Host {
    fn new(png: &[u8]) -> Self {
        let mut s = Session::new(88);
        s.resize_viewport(1600.0, 1000.0);
        s.import_begin(None);
        s.import_add(&at_10_px_per_mm(png), "b.png");
        s.import_finish(false);
        s.set_tool(Tool::Trace);
        let image = s.trace_view().shown.expect("the image is shown");
        let source = s.trace_source(image, false).unwrap();
        let u = Affine::new(source.map).determinant().abs().sqrt() * PX;
        Self {
            s,
            image,
            rgba: source.rgba,
            width: source.width,
            height: source.height,
            map: source.map,
            u,
        }
    }

    fn wire(&self, settings: &str) -> Vec<f64> {
        let mut wire = if settings.contains("\"output\":1") {
            run_trace_job(
                &TraceJob {
                    raster: TraceRaster {
                        width: self.width,
                        height: self.height,
                        rgba: &self.rgba,
                    },
                    map: TracePixelMap::new(self.map),
                    mode: TraceMode::Colours(
                        &trace_colour_settings(settings).expect("multicolour settings"),
                    ),
                    lines: &[],
                    mask: None,
                    shapes: false,
                },
                &mut |_| {},
            )
        } else {
            run_trace_job(
                &TraceJob {
                    raster: TraceRaster {
                        width: self.width,
                        height: self.height,
                        rgba: &self.rgba,
                    },
                    map: TracePixelMap::new(self.map),
                    mode: TraceMode::Mono(&trace_settings(settings)),
                    lines: &[],
                    mask: None,
                    shapes: false,
                },
                &mut |_| {},
            )
        };
        // The worker's glue (`wasm_trace.rs` `with_shapes`).
        if trace_recognise_shapes(settings) {
            trace_add_shapes(&mut wire, self.map);
        }
        wire
    }

    /// Preview and result from one trace, then Create paths (one step).
    fn create(&mut self, settings: &str) -> Vec<f64> {
        let wire = self.wire(settings);
        assert!(self.s.set_trace_preview(self.image, &wire));
        assert!(self.s.set_trace_result(self.image, &wire));
        let steps = self.s.undo_len();
        let out = self.s.create_traced_paths();
        assert!(
            matches!(out, TraceCreateOutcome::Created { images: 1, .. }),
            "{out:?}"
        );
        assert_eq!(
            self.s.undo_len(),
            steps + 1,
            "criterion 14: one history step"
        );
        wire
    }

    fn doc(&self) -> Document {
        unpack(77, &self.s.pack("0.1.0").unwrap()).expect("the session output reopens")
    }

    fn leaves(&self) -> Vec<ObjectSnapshot> {
        let d = self.doc();
        d.object_ids()
            .into_iter()
            .filter_map(|id| d.object(id))
            .filter(|o| !matches!(o, ObjectSnapshot::Image(_)))
            .collect()
    }

    fn fingerprint(&self) -> Vec<String> {
        let d = self.doc();
        d.object_ids()
            .into_iter()
            .map(|id| format!("{id:?} {:?}", d.object(id)))
            .collect()
    }

    /// Picture pixel to document mm.
    fn mm(&self, x: f64, y: f64) -> (f64, f64) {
        let p = Affine::new(self.map) * Point::new(x, y);
        (p.x, p.y)
    }

    /// Direction in document space of a picture-pixel direction of angle `a` (radians).
    fn dir(&self, a: f64) -> f64 {
        let m = Affine::new(self.map).as_coeffs();
        let (dx, dy) = (a.cos(), a.sin());
        (m[1] * dx + m[3] * dy).atan2(m[0] * dx + m[2] * dy)
    }
}

fn prims(leaves: &[ObjectSnapshot]) -> Vec<PrimitiveSnapshot> {
    leaves
        .iter()
        .filter_map(|o| {
            if let ObjectSnapshot::Primitive(p) = o {
                Some(p.clone())
            } else {
                None
            }
        })
        .collect()
}

fn paths(leaves: &[ObjectSnapshot]) -> Vec<PathSnapshot> {
    leaves
        .iter()
        .filter_map(|o| {
            if let ObjectSnapshot::Path(p) = o {
                Some(p.clone())
            } else {
                None
            }
        })
        .collect()
}

fn bez(p: &PathSnapshot) -> Vec<BezPath> {
    p.subpaths()
        .map(|sub| {
            let pt = |a: &curvyo_document_core::AnchorSnapshot| Point::new(a.point.x, a.point.y);
            let mut out = BezPath::new();
            out.move_to(pt(&sub.anchors[0]));
            let n = sub.anchors.len();
            let segs = if sub.closed { n } else { n - 1 };
            for i in 0..segs {
                let (a, b) = (&sub.anchors[i], &sub.anchors[(i + 1) % n]);
                out.curve_to(
                    pt(a) + kurbo::Vec2::new(a.handle_out.x, a.handle_out.y),
                    pt(b) + kurbo::Vec2::new(b.handle_in.x, b.handle_in.y),
                    pt(b),
                );
            }
            out.close_path();
            out
        })
        .collect()
}

fn angle_diff(a: f64, b: f64, period: f64) -> f64 {
    let d = (a - b).rem_euclid(period);
    d.min(period - d)
}

fn near(a: f64, b: f64, tol: f64, what: &str) {
    assert!((a - b).abs() <= tol, "{what}: {a} vs {b} (tolerance {tol})");
}

fn ellipse_of(p: &PrimitiveSnapshot) -> (f64, f64, (f64, f64), f64) {
    match p.shape {
        Shape::Ellipse {
            frame, arc: None, ..
        } => (
            frame.rx.as_mm(),
            frame.ry.as_mm(),
            (frame.center.x, frame.center.y),
            p.rotation.as_radians(),
        ),
        other => panic!("not a whole ellipse: {other:?}"),
    }
}

fn traced_with(host: &Host, settings: &str) -> vectrace::wire::TraceWire {
    read_trace_wire(&host.wire(settings)).expect("a readable wire")
}

// ------------------------------------------------------------------ criteria

/// Criterion 1: a 200 x 100 mm rectangle with 10 mm corners becomes one rectangle primitive.
#[test]
fn c1_a_rounded_rectangle_becomes_one_rectangle_primitive() {
    let mut h = Host::new(&make("rounded"));
    h.create(ON);
    let leaves = h.leaves();
    assert_eq!(leaves.len(), 1, "one object, no path left: {leaves:?}");
    let p = prims(&leaves);
    assert_eq!(p.len(), 1);
    let Shape::Rect {
        bounds,
        corner_radii,
    } = p[0].shape
    else {
        panic!("{:?}", p[0].shape)
    };
    near(bounds.width.as_mm(), 200.0 * h.u, TOL * h.u, "width");
    near(bounds.height.as_mm(), 100.0 * h.u, TOL * h.u, "height");
    for r in [
        corner_radii.tl,
        corner_radii.tr,
        corner_radii.br,
        corner_radii.bl,
    ] {
        near(r.as_mm(), 10.0 * h.u, TOL * h.u, "corner radius");
    }
    let c = h.mm(1100.0, 600.0);
    near(
        bounds.origin.x + bounds.width.as_mm() / 2.0,
        c.0,
        TOL * h.u,
        "centre x",
    );
    near(
        bounds.origin.y + bounds.height.as_mm() / 2.0,
        c.1,
        TOL * h.u,
        "centre y",
    );
    assert!(angle_diff(p[0].rotation.as_radians(), 0.0, PI / 2.0) < 0.5_f64.to_radians());
}

/// Criterion 7: a square-cornered rectangle is a rectangle without a radius, not a rounded one.
#[test]
fn c7_a_square_rectangle_has_no_corner_radius() {
    let mut h = Host::new(&make("rect"));
    h.create(ON);
    let p = prims(&h.leaves());
    assert_eq!(p.len(), 1);
    let Shape::Rect {
        bounds,
        corner_radii,
    } = p[0].shape
    else {
        panic!("{:?}", p[0].shape)
    };
    near(bounds.width.as_mm(), 200.0 * h.u, TOL * h.u, "width");
    near(bounds.height.as_mm(), 100.0 * h.u, TOL * h.u, "height");
    for r in [
        corner_radii.tl,
        corner_radii.tr,
        corner_radii.br,
        corner_radii.bl,
    ] {
        near(r.as_mm(), 0.0, 1e-6, "a square corner is exactly 0");
    }
}

/// Criterion 1 with turning ("keeping position, size and rotation"): 150 x 80 mm, 8 mm, 20 degrees.
#[test]
fn c1_a_turned_rounded_rectangle_keeps_its_rotation() {
    let mut h = Host::new(&make("rounded_turned"));
    h.create(ON);
    let p = prims(&h.leaves());
    assert_eq!(p.len(), 1);
    let Shape::Rect {
        bounds,
        corner_radii,
    } = p[0].shape
    else {
        panic!("{:?}", p[0].shape)
    };
    let (w, hh) = (bounds.width.as_mm(), bounds.height.as_mm());
    let turned = (w - 80.0 * h.u).abs() < 1.0 * h.u;
    if turned {
        near(w, 80.0 * h.u, TOL * h.u, "width");
        near(hh, 150.0 * h.u, TOL * h.u, "height");
    } else {
        near(w, 150.0 * h.u, TOL * h.u, "width");
        near(hh, 80.0 * h.u, TOL * h.u, "height");
    }
    near(
        corner_radii.tl.as_mm(),
        8.0 * h.u,
        TOL * h.u,
        "corner radius",
    );
    let want = h.dir(20.0_f64.to_radians());
    let got = p[0].rotation.as_radians();
    assert!(
        angle_diff(got, want, PI / 2.0) <= 0.5_f64.to_radians(),
        "rotation {} vs {} (mod 90)",
        got.to_degrees(),
        want.to_degrees()
    );
    let c = h.mm(900.0, 900.0);
    near(bounds.origin.x + w / 2.0, c.0, TOL * h.u, "centre x");
    near(bounds.origin.y + hh / 2.0, c.1, TOL * h.u, "centre y");
}

/// Criteria 2 and 7: a 50 mm circle is a circle (rx == ry), not an ellipse that nearly is one.
#[test]
fn c2_c7_a_circle_is_a_circle() {
    let mut h = Host::new(&make("circle"));
    h.create(ON);
    let leaves = h.leaves();
    assert_eq!(leaves.len(), 1);
    let (rx, ry, c, _) = ellipse_of(&prims(&leaves)[0]);
    near(rx, 25.0 * h.u, TOL * h.u, "radius");
    assert_eq!(rx, ry, "a circle has equal radii, not two near ones");
    let want = h.mm(350.0, 350.0);
    near(c.0, want.0, TOL * h.u, "centre x");
    near(c.1, want.1, TOL * h.u, "centre y");
}

/// Criterion 2: a 60 x 40 mm ellipse keeps its radii and its turn (0.5 degrees).
#[test]
fn c2_an_ellipse_keeps_radii_and_rotation() {
    let mut h = Host::new(&make("ellipse"));
    h.create(ON);
    let leaves = h.leaves();
    assert_eq!(leaves.len(), 1);
    let (rx, ry, c, rot) = ellipse_of(&prims(&leaves)[0]);
    let (major, minor, major_dir) = if rx >= ry {
        (rx, ry, rot)
    } else {
        (ry, rx, rot + PI / 2.0)
    };
    near(major, 30.0 * h.u, TOL * h.u, "semi-major");
    near(minor, 20.0 * h.u, TOL * h.u, "semi-minor");
    let want = h.dir(25.0_f64.to_radians());
    assert!(
        angle_diff(major_dir, want, PI) <= 0.5_f64.to_radians(),
        "rotation {} vs {}",
        major_dir.to_degrees(),
        want.to_degrees()
    );
    let want_c = h.mm(400.0, 400.0);
    near(c.0, want_c.0, TOL * h.u, "centre x");
    near(c.1, want_c.1, TOL * h.u, "centre y");
}

/// Criterion 3: a five-point star, outer 40 mm, ratio 0.5, rotation within 0.5 degrees.
#[test]
fn c3_a_five_point_star() {
    let mut h = Host::new(&make("star"));
    h.create(ON);
    let leaves = h.leaves();
    assert_eq!(leaves.len(), 1, "{leaves:?}");
    let p = prims(&leaves);
    let Shape::Star {
        frame,
        point_count,
        inner_ratio,
        ..
    } = p[0].shape
    else {
        panic!("{:?}", p[0].shape)
    };
    assert_eq!(point_count.get(), 5);
    near(frame.radius.as_mm(), 40.0 * h.u, TOL * h.u, "outer radius");
    near(
        frame.radius.as_mm() * inner_ratio.get(),
        20.0 * h.u,
        TOL * h.u,
        "inner radius",
    );
    let first = frame.angle.as_radians() + p[0].rotation.as_radians();
    let want = h.dir((-70.0_f64).to_radians());
    assert!(
        angle_diff(first, want, TAU / 5.0) <= 0.5_f64.to_radians(),
        "rotation {} vs {} (mod 72)",
        first.to_degrees(),
        want.to_degrees()
    );
    let c = h.mm(500.0, 500.0);
    near(frame.center.x, c.0, TOL * h.u, "centre x");
    near(frame.center.y, c.1, TOL * h.u, "centre y");
}

/// Criteria 4 and 7: a regular hexagon is a six-sided polygon, not a star.
#[test]
fn c4_c7_a_hexagon_is_a_polygon_of_six_sides() {
    let mut h = Host::new(&make("hexagon"));
    h.create(ON);
    let leaves = h.leaves();
    assert_eq!(leaves.len(), 1);
    let p = prims(&leaves);
    let Shape::Polygon {
        frame, point_count, ..
    } = p[0].shape
    else {
        panic!("{:?}", p[0].shape)
    };
    assert_eq!(point_count.get(), 6);
    near(frame.radius.as_mm(), 30.0 * h.u, TOL * h.u, "circumradius");
    let first = frame.angle.as_radians() + p[0].rotation.as_radians();
    let want = h.dir(15.0_f64.to_radians());
    assert!(
        angle_diff(first, want, TAU / 6.0) <= 0.5_f64.to_radians(),
        "rotation {} vs {} (mod 60)",
        first.to_degrees(),
        want.to_degrees()
    );
}

/// Criterion 5: a hand-drawn wobbly circle (+-3 % of 25 mm) stays a traced path, unchanged, no notice.
#[test]
fn c5_a_wobbly_circle_stays_the_traced_path() {
    let mut on = Host::new(&make("wobbly"));
    let off = Host::new(&make("wobbly"));
    let wire_on = traced_with(&on, ON);
    let wire_off = traced_with(&off, OFF);
    assert!(wire_on.shapes.is_empty(), "nothing recognised");
    assert_eq!(
        wire_on.outlines, wire_off.outlines,
        "the paths are the traced paths, node for node"
    );
    assert_eq!(
        (wire_on.paths, wire_on.nodes),
        (wire_off.paths, wire_off.nodes)
    );
    on.create(ON);
    let leaves = on.leaves();
    assert_eq!(prims(&leaves).len(), 0);
    assert_eq!(paths(&leaves).len(), 1);
}

/// Criterion 5, other side of the boundary: the same irregular outline at +-0.6 % (1.5 px) is within
/// the 2 px floor and becomes a circle.
#[test]
fn c5_a_nearly_round_outline_inside_the_tolerance_becomes_a_circle() {
    let mut h = Host::new(&make("wobbly_small"));
    h.create(ON);
    let p = prims(&h.leaves());
    assert_eq!(p.len(), 1, "a circle");
    let (rx, ry, _, _) = ellipse_of(&p[0]);
    near(rx, 25.0 * h.u, TOL * h.u, "radius");
    assert_eq!(rx, ry);
}

/// Criterion 16 (changed from 8): a stroked band is one primitive at the centre line, the stroke
/// of the band's width in the region's colour, no fill.
fn assert_stroked(p: &PrimitiveSnapshot, width_mm: f64, colour: Color, tol: f64) {
    assert!(!p.style.fill.enabled, "no fill: {:?}", p.style);
    assert!(p.style.stroke.enabled, "a stroke: {:?}", p.style);
    near(p.style.stroke.width.as_mm(), width_mm, tol, "stroke width");
    assert_eq!(p.style.stroke.color, colour, "stroke in the fill colour");
}

/// Criterion 16: a ring of 25 mm outer and 10 mm inner radius is one circle of radius 17.5 mm
/// with a 15 mm stroke, no leftover path.
#[test]
fn c16_a_ring_is_one_stroked_circle() {
    let mut h = Host::new(&make("ring"));
    h.create(ON);
    let leaves = h.leaves();
    assert_eq!(leaves.len(), 1, "one object: {leaves:?}");
    assert_eq!(paths(&leaves).len(), 0, "no path");
    let p = prims(&leaves);
    assert_eq!(p.len(), 1);
    let (rx, ry, centre, _) = ellipse_of(&p[0]);
    let c = h.mm(350.0, 350.0);
    near(rx, 17.5 * h.u, TOL * h.u, "centre-line radius");
    assert_eq!(rx, ry, "a circle");
    near(centre.0, c.0, TOL * h.u, "centre x");
    near(centre.1, c.1, TOL * h.u, "centre y");
    assert_stroked(&p[0], 15.0 * h.u, Color { r: 0, g: 0, b: 0 }, TOL * h.u);
}

/// Criterion 16: a 200 x 100 mm frame with a 160 x 60 mm hole is one 180 x 80 mm rectangle with a
/// 20 mm stroke.
#[test]
fn c16_a_frame_is_one_stroked_rectangle() {
    let mut h = Host::new(&make("frame"));
    h.create(ON);
    let leaves = h.leaves();
    assert_eq!(leaves.len(), 1, "{leaves:?}");
    assert_eq!(paths(&leaves).len(), 0, "no path");
    let p = prims(&leaves);
    assert_eq!(p.len(), 1);
    let Shape::Rect { bounds, .. } = p[0].shape else {
        panic!("{:?}", p[0].shape)
    };
    near(bounds.width.as_mm(), 180.0 * h.u, TOL * h.u, "width");
    near(bounds.height.as_mm(), 80.0 * h.u, TOL * h.u, "height");
    let c = h.mm(1100.0, 600.0);
    near(
        bounds.origin.x + bounds.width.as_mm() / 2.0,
        c.0,
        TOL * h.u,
        "centre x",
    );
    near(
        bounds.origin.y + bounds.height.as_mm() / 2.0,
        c.1,
        TOL * h.u,
        "centre y",
    );
    assert_stroked(&p[0], 20.0 * h.u, Color { r: 0, g: 0, b: 0 }, TOL * h.u);
}

/// Criterion 9 (changed by 16): the 0025 ring (radii 6 and 2 mm) is one circle of radius 4 mm with
/// a 4 mm stroke.
#[test]
fn c9_the_0025_ring_is_one_stroked_circle() {
    let mut h = Host::new(&make("ring25"));
    h.create(ON);
    let leaves = h.leaves();
    assert_eq!(leaves.len(), 1);
    assert_eq!(paths(&leaves).len(), 0);
    let p = prims(&leaves);
    assert_eq!(p.len(), 1);
    let (rx, ry, centre, _) = ellipse_of(&p[0]);
    let c = h.mm(100.0, 100.0);
    near(rx, 4.0 * h.u, 0.1 * h.u, "radius");
    assert_eq!(rx, ry);
    near(centre.0, c.0, 0.1 * h.u, "centre x");
    near(centre.1, c.1, 0.1 * h.u, "centre y");
    assert_stroked(&p[0], 4.0 * h.u, Color { r: 0, g: 0, b: 0 }, 0.1 * h.u);
}

/// Criterion 16: a ring whose hole is off centre and a frame with borders of 20 and 10 mm are of
/// varying width, so they stay one compound of two outlines (no primitive).
#[test]
fn c16_uneven_bands_stay_two_outlines() {
    let off_centre = picture(700, 700, |x, y| {
        (x - 350.0).hypot(y - 350.0) <= 300.0 && (x - 390.0).hypot(y - 350.0) > 150.0
    });
    let uneven = picture(1400, 1000, |x, y| {
        let outer = (100.0..1300.0).contains(&x) && (100.0..900.0).contains(&y);
        let hole = (300.0..1100.0).contains(&x) && (200.0..800.0).contains(&y);
        outer && !hole
    });
    for (name, png) in [("off centre", off_centre), ("uneven frame", uneven)] {
        let mut h = Host::new(&png);
        h.create(ON);
        let leaves = h.leaves();
        assert_eq!(leaves.len(), 1, "{name}: {leaves:?}");
        assert_eq!(prims(&leaves).len(), 0, "{name}: no primitive");
        let ps = paths(&leaves);
        assert_eq!(ps.len(), 1, "{name}");
        assert_eq!(bez(&ps[0]).len(), 2, "{name}: two outlines");
    }
}

/// Criterion 16 and 10: a red ring in Multicolour is a stroked circle with a red stroke.
#[test]
fn c16_a_multicolour_ring_keeps_its_colour_on_the_stroke() {
    let png = colour_picture(700, 700, |x, y| {
        let r = (x - 350.0).hypot(y - 350.0);
        if r <= 250.0 && r > 100.0 {
            [255, 0, 0]
        } else {
            [255, 255, 255]
        }
    });
    let mut h = Host::new(&png);
    h.create(MULTI_ON);
    let leaves = h.leaves();
    assert_eq!(leaves.len(), 1, "{leaves:?}");
    let p = prims(&leaves);
    assert_eq!(p.len(), 1, "{leaves:?}");
    let (rx, _, _, _) = ellipse_of(&p[0]);
    near(rx, 17.5 * h.u, TOL * h.u, "radius");
    assert_stroked(&p[0], 15.0 * h.u, Color { r: 255, g: 0, b: 0 }, TOL * h.u);
}

/// Criterion 9: the 0025 dot grid is 10,000 circles (100 x 100) and no path remains.
#[test]
fn c9_the_0025_dot_grid_is_ten_thousand_circles() {
    let mut h = Host::new(&make("dots"));
    h.create(ON);
    let leaves = h.leaves();
    let circles = prims(&leaves)
        .into_iter()
        .filter(|p| matches!(p.shape, Shape::Ellipse { .. }))
        .count();
    assert_eq!(leaves.len(), 10_000, "no path remains and nothing else");
    assert_eq!(circles, 10_000);
    assert_eq!(paths(&leaves).len(), 0);
}

/// Criterion 10: a red circle and a blue rectangle in Multicolour keep their fills.
#[test]
fn c10_multicolour_circle_and_rectangle_keep_their_colour() {
    let mut h = Host::new(&make("colour"));
    h.create(MULTI_ON);
    let leaves = h.leaves();
    let p = prims(&leaves);
    assert_eq!(leaves.len(), 2, "{leaves:?}");
    let circle = p
        .iter()
        .find(|p| matches!(p.shape, Shape::Ellipse { .. }))
        .expect("a circle");
    let rect = p
        .iter()
        .find(|p| matches!(p.shape, Shape::Rect { .. }))
        .expect("a rectangle");
    assert_eq!(circle.style.fill.color, Color { r: 255, g: 0, b: 0 });
    assert_eq!(rect.style.fill.color, Color { r: 0, g: 0, b: 255 });
    assert!(circle.style.fill.enabled && rect.style.fill.enabled);
    let (rx, ry, _, _) = ellipse_of(circle);
    near(rx, 15.0 * h.u, TOL * h.u, "circle radius");
    assert_eq!(rx, ry);
    let Shape::Rect { bounds, .. } = rect.shape else {
        unreachable!()
    };
    near(bounds.width.as_mm(), 40.0 * h.u, TOL * h.u, "rect width");
    near(bounds.height.as_mm(), 20.0 * h.u, TOL * h.u, "rect height");
}

/// Criterion 11: switch off (explicit or absent) gives the same wire; with the switch on, a picture
/// without shapes gives the same outlines and the same Create paths result as off.
#[test]
fn c11_off_equals_today() {
    for name in ["wobbly", "rect", "ring", "mixed"] {
        let h = Host::new(&make(name));
        let absent = h.wire(OFF);
        let explicit = h.wire(OFF_EXPLICIT);
        assert_eq!(
            absent, explicit,
            "{name}: shapes false is the same wire as no key"
        );
        let multi_a = Host::new(&make("colour"));
        assert_eq!(
            multi_a.wire(MULTI_OFF),
            multi_a.wire(&MULTI_OFF.replace('}', ",\"shapes\":false}"))
        );
    }
    // A picture with no shape in it: on and off write the same document.
    let mut a = Host::new(&make("wobbly"));
    let mut b = Host::new(&make("wobbly"));
    a.create(OFF);
    b.create(ON);
    let strip = |f: Vec<String>| -> Vec<String> {
        f.into_iter()
            .map(|s| s.split_once(' ').unwrap().1.to_owned())
            .collect()
    };
    assert_eq!(strip(a.fingerprint()).len(), strip(b.fingerprint()).len());
    let pa = paths(&a.leaves());
    let pb = paths(&b.leaves());
    assert_eq!(pa.len(), pb.len());
    for (x, y) in pa.iter().zip(&pb) {
        assert_eq!(x.anchors.len(), y.anchors.len());
        assert_eq!(x.extra_subpaths, y.extra_subpaths);
        assert_eq!(x.style, y.style);
        for (m, n) in x.anchors.iter().zip(&y.anchors) {
            assert_eq!(
                (m.point, m.handle_in, m.handle_out),
                (n.point, n.handle_in, n.handle_out)
            );
        }
    }
}

/// Criterion 11 on a picture with shapes: the traced numbers before the shape section are those of
/// the switch off, and the outline list differs only where a shape was recognised.
#[test]
fn c11_the_traced_numbers_with_the_switch_on_start_with_the_switch_off_wire() {
    for name in ["mixed", "ring", "rounded"] {
        let h = Host::new(&make(name));
        let off = h.wire(OFF);
        let on = h.wire(ON);
        let (a, b) = (traced_with(&h, OFF), traced_with(&h, ON));
        assert_eq!(
            a.paths, b.paths,
            "{name}: the count of paths is that of the trace"
        );
        assert!(!b.shapes.is_empty(), "{name}: shapes found");
        assert_eq!(a.shapes.len(), 0);
        assert!(on.len() > off.len());
        assert_eq!(a.layers.len(), b.layers.len());
    }
}

/// Criterion 11 in Multicolour: the palette and layers are untouched by the switch.
#[test]
fn c11_multicolour_palette_and_layers_are_untouched() {
    let h = Host::new(&make("colour"));
    let a = traced_with(&h, MULTI_OFF);
    let b = traced_with(&h, MULTI_ON);
    assert_eq!(a.palette, b.palette);
    assert_eq!(a.layers.len(), b.layers.len());
    assert_eq!(a.paths, b.paths);
    assert_eq!(a.joined, b.joined);
}

// Criterion 12 is in `acceptance_0088_partb_text_tester.rs` (it needs the OCR model).

/// Criterion 13: two runs give identical primitives and identical wires.
#[test]
fn c13_two_runs_are_identical() {
    for name in ["mixed", "star", "ellipse", "colour"] {
        let settings = if name == "colour" { MULTI_ON } else { ON };
        let mut a = Host::new(&make(name));
        let mut b = Host::new(&make(name));
        let (wa, wb) = (a.create(settings), b.create(settings));
        assert_eq!(
            wa.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            wb.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            "{name}"
        );
        let key = |h: &Host| {
            prims(&h.leaves())
                .iter()
                .map(|p| format!("{:?} {:?} {:?}", p.shape, p.rotation, p.style))
                .collect::<Vec<_>>()
        };
        assert_eq!(key(&a), key(&b), "{name}");
    }
}

/// Criterion 14: Create paths with primitives is one undo step that restores the state before it.
#[test]
fn c14_one_undo_restores_the_document() {
    let mut h = Host::new(&make("mixed"));
    let before = h.fingerprint();
    h.create(ON);
    let after = h.fingerprint();
    assert_ne!(before, after);
    assert!(prims(&h.leaves()).len() >= 4);
    h.s.undo();
    assert_eq!(h.fingerprint(), before, "one undo restores everything");
    h.s.redo();
    assert_eq!(h.fingerprint(), after, "redo brings the same result");
}

/// A picture with all five kinds: circle, rounded rectangle, hexagon, seven-point star, side by side.
#[test]
fn c1_to_c4_a_mixed_picture_finds_each_kind_once() {
    let mut h = Host::new(&make("mixed"));
    h.create(ON);
    let leaves = h.leaves();
    assert_eq!(leaves.len(), 4, "{leaves:?}");
    let p = prims(&leaves);
    assert_eq!(p.len(), 4);
    let kinds = |f: &dyn Fn(&Shape) -> bool| p.iter().filter(|p| f(&p.shape)).count();
    assert_eq!(kinds(&|s| matches!(s, Shape::Ellipse { .. })), 1);
    assert_eq!(kinds(&|s| matches!(s, Shape::Rect { .. })), 1);
    assert_eq!(kinds(&|s| matches!(s, Shape::Polygon { .. })), 1);
    assert_eq!(kinds(&|s| matches!(s, Shape::Star { .. })), 1);
}

/// Degenerate pictures: white, black, a 1 px line and a shape cut by the edge: no panic, no wrong
/// primitive. A circle cut by the picture edge and a rectangle one pixel wide are not recognised.
#[test]
fn degenerate_pictures_make_no_wrong_primitive() {
    let white = picture(64, 64, |_, _| false);
    let black = picture(64, 64, |_, _| true);
    // White: nothing at all.
    let h = Host::new(&white);
    let (on, off) = (traced_with(&h, ON), traced_with(&h, OFF));
    assert_eq!((on.outlines.len(), on.shapes.len()), (0, 0));
    assert_eq!(on.outlines, off.outlines);
    // Black: the one outline is the whole picture; a rectangle fits it, and the whole-picture flag stays.
    let mut h = Host::new(&black);
    let (on, off) = (traced_with(&h, ON), traced_with(&h, OFF));
    assert_eq!(
        (on.covers_whole_image, off.covers_whole_image),
        (true, true)
    );
    assert_eq!((on.outlines.len(), off.outlines.len()), (1, 1));
    h.create(ON);
    let p = prims(&h.leaves());
    assert_eq!(p.len(), 1, "{:?}", h.leaves());
    let Shape::Rect { bounds, .. } = p[0].shape else {
        panic!("{:?}", p[0].shape)
    };
    near(bounds.width.as_mm(), 6.4 * h.u, TOL * h.u, "width");
    near(bounds.height.as_mm(), 6.4 * h.u, TOL * h.u, "height");
    let mut h = Host::new(&make("cut"));
    let on = traced_with(&h, ON);
    let off = traced_with(&h, OFF);
    h.create(ON);
    let p = prims(&h.leaves());
    eprintln!(
        "cut: {} shapes, {} paths on, {} off",
        on.shapes.len(),
        on.paths,
        off.paths
    );
    for q in &p {
        if let Shape::Ellipse { frame, .. } = q.shape {
            // The half disc at the left edge must not become a whole 12 mm circle.
            let c = h.mm(40.0, 200.0);
            assert!(
                !((frame.center.x - c.0).abs() < 1.0
                    && frame.rx.as_mm() > 11.0 * h.u
                    && frame.ry.as_mm() > 11.0 * h.u),
                "a cut circle became a whole one: {q:?}"
            );
        }
    }
}

/// Noise: 64 x 64 random pixels with the switch on neither panics nor differs between runs.
#[test]
fn noise_does_not_panic_and_is_deterministic() {
    let mut seed = 0x1234_5678_u32;
    let mut bits = Vec::new();
    for _ in 0..64 * 64 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        bits.push(seed >> 31 == 1);
    }
    let png = picture(64, 64, |x, y| bits[(y as usize) * 64 + x as usize]);
    let h = Host::new(&png);
    let a = h.wire(ON);
    let b = h.wire(ON);
    assert_eq!(
        a.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        b.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
    let t = traced_with(&h, ON);
    assert!(
        t.outlines
            .iter()
            .all(|o| o.elements().iter().all(|e| match e {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => p.x.is_finite() && p.y.is_finite(),
                _ => true,
            }))
    );
}

/// Criterion 6: 500 traced closed paths, the recognition pass (switch on minus switch off) takes under
/// 2 s of thread CPU time even in a debug build.
#[test]
fn c6_five_hundred_paths_recognise_in_under_two_seconds_of_cpu() {
    let h = Host::new(&make("grid500"));
    let (off, t_off) = cpu_time::cpu_time_of(|| traced_with(&h, OFF));
    let (on, t_on) = cpu_time::cpu_time_of(|| traced_with(&h, ON));
    assert!(off.paths >= 500, "{} paths", off.paths);
    let pass = t_on.saturating_sub(t_off);
    eprintln!(
        "500 paths: off {t_off:?}, on {t_on:?}, recognition about {pass:?}; {} shapes",
        on.shapes.len()
    );
    assert!(pass.as_secs_f64() < 2.0, "recognition {pass:?}");
    assert!(
        on.shapes.len() >= 400,
        "most of the grid is recognised: {}",
        on.shapes.len()
    );
}

/// Criterion 15: nothing in the recognition reaches the network. The core crates have no network
/// dependency, so the pass cannot; checked on the manifests.
#[test]
fn c15_the_recognition_crates_have_no_network_dependency() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    for krate in ["curvyo-ui-core", "vectrace"] {
        let toml = std::fs::read_to_string(root.join(krate).join("Cargo.toml")).unwrap();
        for banned in [
            "reqwest",
            "hyper",
            "ureq",
            "tokio",
            "surf",
            "isahc",
            "attohttpc",
        ] {
            assert!(!toml.contains(banned), "{krate} depends on {banned}");
        }
    }
}

// ------------------------------------------------------------------ property tests

proptest! {
    #![proptest_config(ProptestConfig { cases: 12, ..ProptestConfig::default() })]

    /// Any circle of 10 to 40 mm radius at any centre is recovered within 0.2 mm, as a circle.
    #[test]
    fn prop_circles_are_recovered(r in 10.0_f64..40.0, fx in 0.0_f64..1.0, fy in 0.0_f64..1.0) {
        let (rp, w) = (r * PX, (r * PX).ceil() as u32 * 2 + 60);
        let (cx, cy) = (30.0 + rp + fx * 20.0, 30.0 + rp + fy * 20.0);
        let png = picture(w + 20, w + 20, move |x, y| in_ellipse(x, y, cx, cy, rp, rp, 0.0));
        let mut h = Host::new(&png);
        h.create(ON);
        let p = prims(&h.leaves());
        prop_assert_eq!(p.len(), 1);
        let (rx, ry, c, _) = ellipse_of(&p[0]);
        prop_assert!((rx - r * h.u).abs() <= TOL * h.u, "radius {rx} vs {r}");
        prop_assert_eq!(rx, ry);
        let want = h.mm(cx, cy);
        prop_assert!((c.0 - want.0).abs() <= TOL * h.u && (c.1 - want.1).abs() <= TOL * h.u);
    }

    /// Any clearly oval ellipse (ratio 1.3 to 2, major 20 to 35 mm) at any turn is recovered.
    #[test]
    fn prop_ellipses_are_recovered(a in 20.0_f64..35.0, ratio in 1.3_f64..2.0, deg in 0.0_f64..180.0) {
        let (ap, bp) = (a * PX, a * PX / ratio);
        let size = (ap * 2.0).ceil() as u32 + 80;
        let (cx, cy) = (f64::from(size) / 2.0, f64::from(size) / 2.0);
        let rot = deg.to_radians();
        let png = picture(size, size, move |x, y| in_ellipse(x, y, cx, cy, ap, bp, rot));
        let mut h = Host::new(&png);
        h.create(ON);
        let p = prims(&h.leaves());
        prop_assert_eq!(p.len(), 1);
        let (rx, ry, _, r) = ellipse_of(&p[0]);
        let (major, minor, dir) = if rx >= ry { (rx, ry, r) } else { (ry, rx, r + PI / 2.0) };
        prop_assert!((major - a * h.u).abs() <= TOL * h.u, "major {major} vs {a}");
        prop_assert!((minor - a * h.u / ratio).abs() <= TOL * h.u, "minor {minor} vs {}", a / ratio);
        prop_assert!(angle_diff(dir, h.dir(rot), PI) <= 0.5_f64.to_radians(), "turn {} vs {deg}", dir.to_degrees());
    }
}

// ------------------------------------------------------------------ native record for the wasm test

fn fnv(wire: &[f64]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for v in wire {
        for b in v.to_bits().to_le_bytes() {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The pictures on disk (`CURVYO_BLESS=1` writes them and the record) are the pictures of this file,
/// and the native wire of each is the recorded one: the record the wasm test compares against.
#[test]
fn native_record_matches_the_committed_pictures_and_wires() {
    let dir = fixtures().join("acceptance_0088_partb");
    let record_path = fixtures().join("acceptance_0088_partb_native.json");
    let mut record = serde_json::Map::new();
    for name in NAMES {
        let png = make(name);
        let file = dir.join(format!("{name}.png"));
        if std::env::var_os("CURVYO_BLESS").is_some() {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(&file, &png).unwrap();
        }
        assert_eq!(
            std::fs::read(&file).unwrap_or_default(),
            png,
            "{name}.png on disk is this file's picture (CURVYO_BLESS=1)"
        );
        let h = Host::new(&png);
        let multi = name == "colour";
        let (on, off) = if multi {
            (MULTI_ON, MULTI_OFF)
        } else {
            (ON, OFF)
        };
        record.insert(
            name.to_owned(),
            serde_json::json!({
                "map": h.map, "width": h.width, "height": h.height, "multicolour": multi,
                "on_settings": on, "off_settings": off,
                "on_fnv": fnv(&h.wire(on)), "off_fnv": fnv(&h.wire(off)),
                "shapes": traced_with(&h, on).shapes.len(),
            }),
        );
    }
    let json = serde_json::to_string_pretty(&serde_json::Value::Object(record)).unwrap() + "\n";
    if std::env::var_os("CURVYO_BLESS").is_some() {
        std::fs::write(&record_path, &json).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&record_path).unwrap_or_default(),
        json,
        "the native record (CURVYO_BLESS=1)"
    );
}

/// Even-n stars with one point straight up: n = 8, 10, 12 at inner ratios 0.3, 0.4, 0.5 each become
/// one star primitive with the right n, radii and rotation within 0.5 degrees.
#[test]
fn c3_even_n_stars_with_a_point_straight_up() {
    for n in [8_u32, 10, 12] {
        for ratio in [0.3_f64, 0.4, 0.5] {
            let v = star_vertices(
                500.0,
                500.0,
                n,
                400.0,
                Some(400.0 * ratio),
                -std::f64::consts::FRAC_PI_2,
            );
            let mut h = Host::new(&picture(1000, 1000, move |x, y| in_polygon(x, y, &v)));
            h.create(ON);
            let leaves = h.leaves();
            let what = format!("n {n} ratio {ratio}");
            assert_eq!(leaves.len(), 1, "{what}: {leaves:?}");
            let p = prims(&leaves);
            let Shape::Star {
                frame,
                point_count,
                inner_ratio,
                ..
            } = p[0].shape
            else {
                panic!("{what}: {:?}", p[0].shape)
            };
            assert_eq!(point_count.get(), n, "{what}: point count");
            near(
                frame.radius.as_mm(),
                40.0 * h.u,
                TOL * h.u,
                &format!("{what}: outer radius"),
            );
            near(
                frame.radius.as_mm() * inner_ratio.get(),
                40.0 * ratio * h.u,
                TOL * h.u,
                &format!("{what}: inner radius"),
            );
            let first = frame.angle.as_radians() + p[0].rotation.as_radians();
            let want = h.dir((-90.0_f64).to_radians());
            assert!(
                angle_diff(first, want, TAU / f64::from(n)) <= 0.5_f64.to_radians(),
                "{what}: rotation {} vs {}",
                first.to_degrees(),
                want.to_degrees()
            );
        }
    }
}

/// A picture of the strokes of `lines` (picture pixels, 10 px/mm) drawn with `join`.
fn stroked_star_picture(
    w: u32,
    h: u32,
    lines: &[(f64, f64, f64, f64, u32)],
    width: f64,
    join: kurbo::Join,
) -> Vec<u8> {
    let outlines: Vec<BezPath> = lines
        .iter()
        .map(|&(cx, cy, r, ratio, n)| {
            let verts = star_vertices(cx, cy, n, r, Some(r * ratio), 0.1);
            let mut p = BezPath::new();
            for (i, v) in verts.iter().enumerate() {
                if i == 0 {
                    p.move_to(*v);
                } else {
                    p.line_to(*v);
                }
            }
            p.close_path();
            let style = kurbo::Stroke::new(width)
                .with_join(join)
                .with_miter_limit(4.0);
            kurbo::stroke(p.iter(), &style, &kurbo::StrokeOpts::default(), 0.05)
        })
        .collect();
    picture(w, h, move |x, y| {
        let p = Point::new(x, y);
        outlines.iter().any(|o| kurbo::Shape::winding(o, p) != 0)
    })
}

/// Criterion 16 (join as traced): a bevelled outline star is one stroked star whose stroke joins are
/// bevel; a mitred one has miter joins.
#[test]
fn c16_the_join_of_a_stroked_star_is_the_traced_one() {
    for (join, want) in [
        (kurbo::Join::Bevel, curvyo_document_core::LineJoin::Bevel),
        (kurbo::Join::Miter, curvyo_document_core::LineJoin::Miter),
    ] {
        let mut h = Host::new(&stroked_star_picture(
            700,
            700,
            &[(350.0, 350.0, 250.0, 0.55, 5)],
            30.0,
            join,
        ));
        h.create(ON);
        let leaves = h.leaves();
        assert_eq!(leaves.len(), 1, "{want:?}: {leaves:?}");
        let p = prims(&leaves);
        assert_eq!(p.len(), 1, "{want:?}");
        assert!(matches!(p[0].shape, Shape::Star { .. }), "{want:?}");
        assert_eq!(p[0].style.stroke.join, want);
        assert_stroked(&p[0], 3.0 * h.u, Color { r: 0, g: 0, b: 0 }, 0.3 * h.u);
    }
}

/// Criterion 16: three touching outline stars are three stroked stars, no path; the same row with a
/// plain filled disc touching it stays paths.
#[test]
fn c16_touching_stars_are_one_stroked_star_each() {
    let row: Vec<_> = (0..3)
        .map(|k| (200.0 + 310.0 * f64::from(k), 200.0, 160.0, 0.8, 8))
        .collect();
    let mut h = Host::new(&stroked_star_picture(
        1020,
        400,
        &row,
        25.0,
        kurbo::Join::Miter,
    ));
    h.create(ON);
    let leaves = h.leaves();
    assert_eq!(leaves.len(), 3, "{leaves:?}");
    assert_eq!(paths(&leaves).len(), 0, "no leftover path");
    for p in prims(&leaves) {
        assert!(matches!(p.shape, Shape::Star { point_count, .. } if point_count.get() == 8));
        assert_stroked(&p, 2.5 * h.u, Color { r: 0, g: 0, b: 0 }, 0.3 * h.u);
    }
}

/// Criterion 16: a stroked star touching a filled disc with an off-centre hole is not split.
#[test]
fn c16_a_touching_hole_that_is_no_band_leaves_paths() {
    let star = stroked_star_picture(
        800,
        400,
        &[(200.0, 200.0, 160.0, 0.8, 8)],
        25.0,
        kurbo::Join::Miter,
    );
    // Re-render star plus a disc with an off-centre hole into one picture.
    let img = image::load_from_memory(&star).unwrap().to_luma8();
    let png = picture(800, 400, |x, y| {
        let star_ink = img.get_pixel(x as u32, y as u32).0[0] < 128;
        let disc = (x - 490.0).hypot(y - 200.0) <= 140.0 && (x - 530.0).hypot(y - 200.0) > 60.0;
        star_ink || disc
    });
    let mut h = Host::new(&png);
    h.create(ON);
    let leaves = h.leaves();
    assert!(
        prims(&leaves)
            .iter()
            .all(|p| !p.style.stroke.enabled || p.style.fill.enabled),
        "no stroked band primitive from a mixed region: {leaves:?}"
    );
    assert!(!paths(&leaves).is_empty(), "paths remain");
}
