//! Tester acceptance cases for `specs/0123-raster-engraving` through the public `Session`,
//! written from the specification before the implementation was read: what runs (criteria 1 to
//! 4), the dot grid (5 to 7), the adjustments and modes as burned in the file (9, 10), the Image
//! settings group, its ranges, keys, damage and undo (11 to 13), the rows in the file (14 to 19)
//! read back by the tester's own GRBL reader (`curvyo-laser-core/tests/tester_raster_0123`),
//! time and the play bar (21, 22), gradient fills (23 to 30), the checks (31 to 36), storage and
//! determinism (38, 39), the test grid's Fill and Dither with material record format 3 (44, 45),
//! the `0029` criterion 18 replacement, and edge cases (1 x 1, 16-bit PNG, CMYK JPEG, an image
//! larger than the bed, 1,000 images).
//!
//! Superseded by the Image kind rework (2026-11-19): raster items run in Image operations (`b.op(
//! "image", "Image <n>", ..)`, criterion 47), not in Engrave; Engrave and Engrave + Outline hatch an
//! image as its frame rectangle (criterion 1). The cases below are moved to the Image kind with the
//! same assertions unless a case says otherwise.
#![allow(
    clippy::single_element_loop,
    clippy::redundant_closure_for_method_calls,
    clippy::map_unwrap_or,
    clippy::type_complexity,
    clippy::needless_pass_by_value,
    clippy::nonminimal_bool,
    clippy::match_same_arms,
    clippy::bool_assert_comparison,
    clippy::manual_range_contains,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::needless_range_loop,
    missing_docs
)]

#[path = "../../curvyo-laser-core/tests/tester_raster_0123/mod.rs"]
mod reference;

use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, Color, Direction, Document, DocumentSize, EllipseFrame, FillPaint, GradientGeometry,
    GradientStop, GradientStops, Length, NewAnchor, NewImage, NewOperation, NewOutputPage, NodeId,
    Point, RectBounds, StopOffset, StyleEdit, Timestamp, pack, unpack,
};
use curvyo_document_core::{AnchorId, BoxPoint};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_image_core::{WorkingImage, encode_png};
use reference::{Gcode, burned_power, read};
use serde_json::{Value, json};

const LASER: &str = "curvyo.laser";
const BED_H: f64 = 300.0;

// ---- fixture ----------------------------------------------------------------------------------

fn png(w: u32, h: u32, mut px: impl FnMut(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            pixels.extend_from_slice(&px(x, y));
        }
    }
    encode_png(&WorkingImage {
        width: w,
        height: h,
        pixels,
        original_size: (w, h),
        truncated: false,
        animated: false,
    })
    .unwrap()
}

fn grey_png(w: u32, h: u32, mut g: impl FnMut(u32, u32) -> u8) -> Vec<u8> {
    png(w, h, |x, y| {
        let v = g(x, y);
        [v, v, v, 255]
    })
}

/// Left half black, right half white, 16 x 16.
fn half_black() -> Vec<u8> {
    grey_png(16, 16, |x, _| if x < 8 { 0 } else { 255 })
}

/// A PNG of any size (also above the 8,192 px working limit) with stored deflate blocks.
fn raw_png(w: u32, h: u32, mut g: impl FnMut(u32, u32) -> u8) -> Vec<u8> {
    fn crc(data: &[u8]) -> u32 {
        let mut c = 0xffff_ffffu32;
        for &b in data {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 == 1 {
                    0xedb8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc(&body).to_be_bytes());
    }
    let mut raw = Vec::new();
    for y in 0..h {
        raw.push(0);
        for x in 0..w {
            raw.push(g(x, y));
        }
    }
    let mut z = vec![0x78, 0x01];
    let chunks: Vec<&[u8]> = raw.chunks(65_535).collect();
    for (i, c) in chunks.iter().enumerate() {
        z.push(u8::from(i + 1 == chunks.len()));
        let n = c.len() as u16;
        z.extend_from_slice(&n.to_le_bytes());
        z.extend_from_slice(&(!n).to_le_bytes());
        z.extend_from_slice(c);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + u32::from(x)) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn picture_fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images")
            .join(name),
    )
    .unwrap()
}

struct Fx {
    s: Session,
    page: String,
}

/// Builder context: the document, the laser page's origin and its manufacturing node.
struct B<'a> {
    d: &'a Document,
    o: Point,
    m: NodeId,
}

impl B<'_> {
    fn image(&self, bytes: &[u8], placement: Affine, name: &str) -> NodeId {
        self.d
            .add_images(&[NewImage {
                bytes: Arc::from(bytes),
                placement,
                name: name.to_owned(),
            }])
            .unwrap()[0]
    }
    /// An axis-aligned image `w` x `h` mm at page (x, y).
    fn img(&self, bytes: &[u8], x: f64, y: f64, w: f64, h: f64) -> NodeId {
        self.image(
            bytes,
            Affine::new(w, 0.0, 0.0, h, self.o.x + x, self.o.y + y),
            "Photo 1",
        )
    }
    fn rect(&self, x: f64, y: f64, w: f64, h: f64) -> NodeId {
        self.d.create_rect(RectBounds::from_corners(
            Point::new(self.o.x + x, self.o.y + y),
            Point::new(self.o.x + x + w, self.o.y + y + h),
        ))
    }
    fn ellipse(&self, cx: f64, cy: f64, rx: f64, ry: f64) -> NodeId {
        self.d.create_ellipse(EllipseFrame {
            center: Point::new(self.o.x + cx, self.o.y + cy),
            rx: Length::from_mm(rx),
            ry: Length::from_mm(ry),
        })
    }
    fn path(&self, pts: &[(f64, f64)], closed: bool, first: u64) -> NodeId {
        let anchors: Vec<NewAnchor> = pts
            .iter()
            .enumerate()
            .map(|(k, &(x, y))| {
                NewAnchor::corner(
                    AnchorId::new(1, first + k as u64),
                    Point::new(self.o.x + x, self.o.y + y),
                )
            })
            .collect();
        self.d.create_path(&anchors, closed)
    }
    fn op(&self, kind: &str, name: &str, members: &[NodeId]) -> NodeId {
        let index = self.d.operations(self.m).unwrap().len();
        let keys: Vec<Vec<NodeId>> = members.iter().map(|&n| vec![n]).collect();
        self.d
            .add_operation(self.m, &NewOperation { kind, name }, index, &keys)
            .unwrap()
    }
    /// A black-to-white (left to right, Angle 0) gradient fill on `id`.
    fn shade(&self, id: NodeId) {
        shade_with(self.d, id, GradientGeometry::DEFAULT_LINEAR, bw());
    }
}

fn stop(v: u8, a: u8, offset: f64) -> GradientStop {
    GradientStop {
        colour: Color { r: v, g: v, b: v },
        alpha: a,
        offset: StopOffset::new(offset).unwrap(),
    }
}

fn bw() -> Vec<GradientStop> {
    vec![stop(0, 255, 0.0), stop(255, 255, 1.0)]
}

fn shade_with(d: &Document, id: NodeId, g: GradientGeometry, stops: Vec<GradientStop>) {
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::GradientGeometry(g))
        .unwrap();
    d.edit_style(
        &[id],
        &StyleEdit::GradientStops(GradientStops::new(stops).unwrap()),
    )
    .unwrap();
}

/// A 400 x 300 mm laser page on a saved profile "Diode 10 W"; `build` draws on it.
fn laser(build: impl FnOnce(&B) -> Vec<NodeId>) -> (Fx, Vec<NodeId>) {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let page = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let m = page.kind.manufacturing().unwrap();
    let ids = build(&B {
        d: &d,
        o: page.origin,
        m,
    });
    let mut s = Session::open(7, &pack(&d, "0123-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(&page.id));
    s.load_materials(&[], None);
    let _ = s.save_machine_profile([13; 16], 1_000);
    let _ = s.set_machine_name("Diode 10 W");
    s.wipe_history();
    (Fx { s, page: page.id }, ids)
}

/// Power, speed, passes and (for Engrave and Image) the Interval of `id`.
fn set(s: &mut Session, id: NodeId, power: f64, speed: f64, interval: Option<f64>) {
    assert!(s.edit_operation_setting(id, "main", "power", power).is_ok());
    assert!(s.edit_operation_setting(id, "main", "speed", speed).is_ok());
    if let Some(i) = interval {
        let _ = s.edit_operation_option(id, "interval", i);
    }
}

fn passes(s: &mut Session, id: NodeId, n: f64) {
    assert_eq!(s.edit_operation_setting(id, "main", "passes", n), Ok(true));
}

/// The operation's Dither `mode`. Superseded by the Dither on the picture (2026-11-20): the same
/// mode on each picture of the operation too (a picture burns its own dither, ADR 0072 amendment
/// A4), so the dots of these criteria stay; the selection is left empty.
fn dither(s: &mut Session, id: NodeId, mode: &str) {
    let _ = s.set_operation_dither(id, mode);
    let d = unpack(99, &s.pack("0.1.0").unwrap()).unwrap();
    let Some(mfg) = d.pages().into_iter().find_map(|p| p.kind.manufacturing()) else {
        return;
    };
    let pictures: Vec<NodeId> = d
        .operations(mfg)
        .unwrap()
        .into_iter()
        .filter(|op| op.id == id)
        .flat_map(|op| op.members.into_iter().flatten())
        .filter(|m| {
            matches!(
                d.object(*m),
                Some(curvyo_document_core::ObjectSnapshot::Image(_))
            )
        })
        .collect();
    for picture in pictures {
        s.select_output_items(&[picture]);
        let _ = s.set_picture_dither(mode);
    }
    s.select_output_items(&[]);
}

/// One Image operation of `image` (or any leaf) at 60 %, 3,000 mm/min, Interval 0.5, Threshold.
fn engrave_one(build: impl FnOnce(&B) -> NodeId) -> (Fx, NodeId, NodeId) {
    let (mut fx, ids) = laser(|b| {
        let leaf = build(b);
        vec![leaf, b.op("image", "Image 1", &[leaf])]
    });
    set(&mut fx.s, ids[1], 60.0, 3000.0, Some(0.5));
    dither(&mut fx.s, ids[1], "threshold");
    // The run-out of `0029` criterion 15 is off, so the rows are exactly the dots.
    let _ = fx.s.edit_operation_option(ids[1], "overscan", 0.0);
    fx.s.wipe_history();
    (fx, ids[0], ids[1])
}

fn out(s: &Session) -> Value {
    serde_json::from_str(&s.output_group_json()).unwrap()
}

fn ops_view(s: &Session) -> Value {
    serde_json::from_str(&s.operations_view_json()).unwrap()
}

fn checks(s: &Session) -> Vec<Value> {
    out(s)["checks"].as_array().cloned().unwrap_or_default()
}

fn texts(s: &Session, severity: &str) -> Vec<String> {
    checks(s)
        .iter()
        .filter(|c| c["severity"] == severity)
        .map(|c| c["text"].as_str().unwrap().to_owned())
        .collect()
}

fn export_bytes(s: &mut Session) -> Option<Vec<u8>> {
    s.export_job("Untitled.curvyo").map(|e| e.bytes)
}

fn gcode(s: &mut Session) -> Gcode {
    let bytes = export_bytes(s).unwrap_or_else(|| panic!("export blocked: {:?}", checks(s)));
    read(&bytes).unwrap_or_else(|e| panic!("{e}\n{}", String::from_utf8_lossy(&bytes)))
}

fn text_of(s: &mut Session) -> String {
    String::from_utf8(export_bytes(s).unwrap()).unwrap()
}

fn undo_name(s: &mut Session) -> Option<String> {
    s.history_target_name(Direction::Undo)
}

/// The 16 x 16 dot map of an image at page (20, 20), 8 x 8 mm, Interval 0.5 (bottom-left
/// origin: the first row's centre at machine Y 279.75).
fn map16(g: &Gcode) -> Vec<Vec<Option<u32>>> {
    g.dot_map(20.0, BED_H - 20.25, 0.5, 16, 16)
}

fn row_of(s: &Session, id: NodeId) -> Value {
    let full = serde_json::to_value(id).unwrap();
    ops_view(s)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == full)
        .cloned()
        .unwrap_or(Value::Null)
}

fn image_group(s: &mut Session, id: NodeId) -> Value {
    let row = row_of(s, id);
    if row["unfolded"] != true {
        assert!(s.unfold_operation_row(id));
    }
    row_of(s, id)["settings"]["options"]["image"].clone()
}

fn options_of(s: &Session, id: NodeId) -> serde_json::Map<String, Value> {
    let d = unpack(99, &s.pack("0.1.0").unwrap()).unwrap();
    let page = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap();
    d.operations(page.kind.manufacturing().unwrap())
        .unwrap()
        .into_iter()
        .find(|o| o.id == id)
        .and_then(|o| o.options)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

// ---- criterion 1: what runs; `0029` criterion 18 replaced ----------------------------------

#[test]
fn c1_an_image_in_engrave_is_raster_engraved_not_silently_ignored() {
    let (mut fx, _, _) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    let g = gcode(&mut fx.s);
    assert!(
        (g.burn_mm() - 16.0 * 8.0 * 0.5).abs() < 0.01,
        "16 rows of 8 burned dots of 0.5 mm: {} mm",
        g.burn_mm()
    );
    let m = map16(&g);
    for r in 0..16 {
        for c in 0..16 {
            assert_eq!(m[r][c].is_some(), c < 8, "dot {r},{c}");
        }
    }
    assert!(g.burns().iter().all(|b| b.s == 600), "60 % of S-max 1000");
}

#[test]
fn c1_an_image_in_cut_or_score_writes_its_frame_as_a_line_only() {
    // Superseded by the Image kind rework (2026-11-19): Score scores an image's frame rectangle
    // like Cut (criterion 1, Frage 271 A); it ran nothing under `0101` criterion 20 before.
    for kind in ["cut", "score"] {
        let (mut fx, ids) = laser(|b| {
            let i = b.img(&half_black(), 20.0, 20.0, 8.0, 8.0);
            vec![i, b.op(kind, "Op 1", &[i])]
        });
        set(&mut fx.s, ids[1], 80.0, 600.0, None);
        let g = gcode(&mut fx.s);
        assert!(
            (g.burn_mm() - 32.0).abs() < 0.01,
            "{kind}: the 8 x 8 frame, {}",
            g.burn_mm()
        );
        assert!(g.burns().len() <= 5, "{kind}: a line, not rows");
    }
}

// ---- criterion 2: Engrave + Outline, Cut-out ----------------------------------------------

// Superseded by the Image kind rework (2026-11-19): Engrave + Outline hatches an image's frame
// rectangle with the Engrave set, then traces the frame with the Score set; no dots (criterion 1).
#[test]
fn c2_engrave_outline_hatches_the_frame_then_traces_it_with_the_score_set() {
    let (mut fx, ids) = laser(|b| {
        let i = b.img(&half_black(), 20.0, 20.0, 8.0, 8.0);
        vec![i, b.op("engrave_outline", "Engrave 1", &[i])]
    });
    let e = ids[1];
    set(&mut fx.s, e, 60.0, 3000.0, Some(0.5));
    assert!(
        fx.s.edit_operation_setting(e, "outline", "power", 20.0)
            .is_ok()
    );
    assert!(
        fx.s.edit_operation_setting(e, "outline", "speed", 500.0)
            .is_ok()
    );
    let g = gcode(&mut fx.s);
    let burns = g.burns();
    let last_hatch = burns.iter().rposition(|m| m.s == 600).unwrap();
    let first_line = burns.iter().position(|m| m.s == 200).expect("the outline");
    assert!(last_hatch < first_line, "hatch first");
    let line: f64 = burns.iter().filter(|m| m.s == 200).map(|m| m.len()).sum();
    assert!((line - 32.0).abs() < 0.01, "frame {line}");
    let hatch: f64 = burns.iter().filter(|m| m.s == 600).map(|m| m.len()).sum();
    assert!(
        (hatch - 16.0 * 8.0).abs() < 0.01,
        "16 hatch lines of 8 mm: {hatch}"
    );
}
#[test]
fn c2_cut_out_runs_nothing_of_the_pixels() {
    let (mut fx, ids) = laser(|b| {
        let i = b.img(&half_black(), 20.0, 20.0, 8.0, 8.0);
        vec![i, b.op("cut_out", "Cut-out 1", &[i])]
    });
    set(&mut fx.s, ids[1], 80.0, 600.0, None);
    if let Some(bytes) = export_bytes(&mut fx.s) {
        let g = read(&bytes).unwrap();
        // At most the frame's outer contour, never rows.
        assert!(g.burn_mm() <= 32.0 + 3.0 + 0.01, "{}", g.burn_mm());
        assert!(g.rows().len() <= 3, "no rows of dots");
    }
}

// ---- criterion 3: gradient-filled shapes --------------------------------------------------

#[test]
fn c3_a_gradient_ellipse_engraves_dots_inside_and_no_outline() {
    let (mut fx, _, e) = engrave_one(|b| {
        let el = b.ellipse(50.0, 50.0, 10.0, 5.0);
        b.shade(el);
        el
    });
    let _ = fx.s.edit_operation_option(e, "interval", 0.2);
    let g = gcode(&mut fx.s);
    let (x0, y0, x1, y1) = g.burn_extent().expect("dots");
    assert!(x0 >= 40.0 - 1e-6 && x1 <= 60.0 + 1e-6, "{x0} {x1}");
    assert!(
        y0 >= BED_H - 55.0 - 1e-6 && y1 <= BED_H - 45.0 + 1e-6,
        "{y0} {y1}"
    );
    // Threshold 50 %: only the dark left half burns.
    assert!(x1 <= 50.0 + 0.3, "only the left half: {x1}");
    // Every burn is horizontal (a row), none follows the outline.
    assert!(g.burns().iter().all(|m| (m.from.1 - m.to.1).abs() < 1e-6));
    // The burned area is about half the ellipse: pi * 10 * 5 / 2 = 78.5 mm².
    let area = g.burn_mm() * 0.2;
    assert!((area - 78.5).abs() < 8.0, "{area}");
}

#[test]
fn c3_a_gradient_shape_in_cut_runs_only_its_outline_and_colour_fill_keeps_the_hatch() {
    let (mut fx, ids) = laser(|b| {
        let r = b.rect(10.0, 10.0, 20.0, 10.0);
        b.shade(r);
        vec![r, b.op("cut", "Cut 1", &[r])]
    });
    set(&mut fx.s, ids[1], 80.0, 600.0, None);
    let g = gcode(&mut fx.s);
    assert!((g.burn_mm() - 60.0).abs() < 0.01, "{}", g.burn_mm());

    // Superseded by the Image kind rework (2026-11-19): in Image an all-white gradient and a
    // colour fill both burn nothing (criteria 10, 48); in Engrave both are hatched, the fill
    // paint never counts there (criterion 3).
    for (kind, name) in [("image", "Image 1"), ("engrave", "Engrave 1")] {
        let (mut fx, ids) = laser(|b| {
            let a = b.rect(10.0, 10.0, 10.0, 10.0);
            shade_with(
                b.d,
                a,
                GradientGeometry::DEFAULT_LINEAR,
                vec![stop(255, 255, 0.0), stop(255, 255, 1.0)],
            );
            let c = b.rect(40.0, 10.0, 10.0, 10.0);
            b.d.edit_style(&[c], &StyleEdit::FillEnabled(true)).unwrap();
            vec![a, c, b.op(kind, name, &[a, c])]
        });
        set(&mut fx.s, ids[2], 60.0, 3000.0, Some(0.5));
        if kind == "image" {
            let burned = export_bytes(&mut fx.s).map_or(0.0, |b| read(&b).unwrap().burn_mm());
            assert!(burned < 1e-6, "nothing burns in Image: {burned}");
            continue;
        }
        let g = gcode(&mut fx.s);
        let (x0, _, x1, _) = g.burn_extent().expect("the hatch");
        assert!(
            x0 <= 10.0 + 0.5 && x1 >= 50.0 - 0.5,
            "both squares: {x0} {x1}"
        );
        assert!(g.burn_mm() > 300.0, "hatched at 0.5 mm: {}", g.burn_mm());
    }
}

#[test]
fn c3_an_open_path_with_a_gradient_is_closed_by_its_straight_line() {
    // A V: (10, 10) -> (20, 30) -> (30, 10); closed it is a triangle of 200 mm².
    let (mut fx, _, e) = engrave_one(|b| {
        let p = b.path(&[(10.0, 10.0), (20.0, 30.0), (30.0, 10.0)], false, 10);
        shade_with(
            b.d,
            p,
            GradientGeometry::DEFAULT_LINEAR,
            vec![stop(0, 255, 0.0), stop(0, 255, 1.0)],
        );
        p
    });
    let _ = fx.s.edit_operation_option(e, "interval", 0.2);
    let g = gcode(&mut fx.s);
    let area = g.burn_mm() * 0.2;
    assert!((area - 200.0).abs() < 10.0, "triangle area {area}");
    let (_, y0, _, y1) = g.burn_extent().unwrap();
    assert!(y1 <= BED_H - 10.0 + 1e-6 && y0 >= BED_H - 30.0 - 1e-6);
}

#[test]
fn c3_a_holed_shape_engraves_without_its_hole() {
    // A 20 mm square minus a centred 10 mm square (Difference), with a Gradient fill, in an
    // Engrave at Threshold 100 %: the ring burns (300 mm²), the hole never.
    let (mut fx, ids) = laser(|b| {
        let outer = b.rect(20.0, 20.0, 20.0, 20.0);
        let inner = b.rect(25.0, 25.0, 10.0, 10.0);
        vec![outer, inner, b.op("image", "Image 1", &[])]
    });
    let e = ids[2];
    assert!(fx.s.select_output_items(&[ids[0], ids[1]]));
    let _ = fx.s.apply_boolean(curvyo_ui_core::BooleanOp::Difference);
    assert!(fx.s.set_fill_kind("gradient"), "the ring gets a gradient");
    assert!(fx.s.set_selection_in_operation(e, true), "assigned");
    set(&mut fx.s, e, 60.0, 3000.0, Some(0.25));
    dither(&mut fx.s, e, "threshold");
    let _ = fx.s.edit_operation_option(e, "threshold", 100.0);
    let _ = fx.s.edit_operation_option(e, "overscan", 0.0);
    let g = gcode(&mut fx.s);
    let area = g.burn_mm() * 0.25;
    assert!(area > 250.0 && area <= 300.0 + 1e-6, "ring area {area}");
    for m in g.burns() {
        let y = BED_H - m.from.1;
        if (25.5..34.5).contains(&y) {
            let lo = m.from.0.min(m.to.0);
            let hi = m.from.0.max(m.to.0);
            assert!(
                hi <= 25.0 + 1e-6 || lo >= 35.0 - 1e-6,
                "a burn over the hole: {m:?}"
            );
        }
    }
}

// ---- criterion 4: transforms --------------------------------------------------------------

/// An image whose top-left quarter is black (8 x 8 px of 16), placed by `placement` relative to
/// page (20, 20) with a 8 x 8 mm frame.
fn quarter() -> Vec<u8> {
    grey_png(16, 16, |x, y| if x < 8 && y < 8 { 0 } else { 255 })
}

fn burned_box(fx: &mut Fx) -> (f64, f64, f64, f64) {
    gcode(&mut fx.s).burn_extent().unwrap()
}

#[test]
fn c4_a_mirrored_image_engraves_mirrored_on_the_bed() {
    let (mut plain, _, _) = engrave_one(|b| b.img(&quarter(), 20.0, 20.0, 8.0, 8.0));
    let (x0, _, x1, _) = burned_box(&mut plain);
    assert!(
        x0 >= 20.0 - 1e-6 && x1 <= 24.0 + 1e-6,
        "top-left quarter: {x0} {x1}"
    );
    let (mut fx, _, _) = engrave_one(|b| {
        b.image(
            &quarter(),
            Affine::new(-8.0, 0.0, 0.0, 8.0, b.o.x + 28.0, b.o.y + 20.0),
            "Photo 1",
        )
    });
    let (x0, y0, x1, y1) = burned_box(&mut fx);
    assert!(
        x0 >= 24.0 - 1e-6 && x1 <= 28.0 + 1e-6,
        "mirrored: {x0} {x1}"
    );
    assert!(
        y0 >= BED_H - 24.0 - 1e-6 && y1 <= BED_H - 20.0 + 1e-6,
        "{y0} {y1}"
    );
}

#[test]
fn c4_a_rotated_image_engraves_rotated() {
    // 90 degrees: u runs down, v runs left: the picture's top-left lands at the frame's top
    // right.
    let (mut fx, _, _) = engrave_one(|b| {
        b.image(
            &quarter(),
            Affine::new(0.0, 8.0, -8.0, 0.0, b.o.x + 28.0, b.o.y + 20.0),
            "Photo 1",
        )
    });
    let g = gcode(&mut fx.s);
    let (x0, y0, x1, y1) = g.burn_extent().unwrap();
    assert!(x0 >= 24.0 - 1e-6 && x1 <= 28.0 + 1e-6, "{x0} {x1}");
    assert!(
        y0 >= BED_H - 24.0 - 1e-6 && y1 <= BED_H - 20.0 + 1e-6,
        "{y0} {y1}"
    );
    assert!(
        (g.burn_mm() * 0.5 - 16.0).abs() < 2.0,
        "a 4 x 4 mm area: {}",
        g.burn_mm()
    );
}

#[test]
fn c4_c5_a_skewed_image_keeps_every_dot_inside_its_parallelogram() {
    // All black, frame skewed by 6 mm over its height.
    let black = grey_png(8, 8, |_, _| 0);
    let (mut fx, _, e) = engrave_one(|b| {
        b.image(
            &black,
            Affine::new(10.0, 0.0, 6.0, 10.0, b.o.x + 20.0, b.o.y + 20.0),
            "Photo 1",
        )
    });
    let _ = fx.s.edit_operation_option(e, "interval", 0.25);
    let g = gcode(&mut fx.s);
    // Parallelogram: x from 20 + 0.6 * (y - 20) to 30 + 0.6 * (y - 20), page y 20..30.
    for m in g.burns() {
        for p in [m.from, m.to] {
            let y = BED_H - p.1;
            let left = 20.0 + 0.6 * (y - 20.0);
            assert!(
                p.0 >= left - 0.25 - 1e-6 && p.0 <= left + 10.0 + 0.25 + 1e-6,
                "{p:?} outside the parallelogram (left {left})"
            );
            assert!((20.0..=30.0).contains(&y), "{y}");
        }
    }
    assert!(
        (g.burn_mm() * 0.25 - 100.0).abs() < 5.0,
        "area {}",
        g.burn_mm() * 0.25
    );
}

#[test]
fn c4_a_rotated_image_dots_never_leave_the_frame() {
    let black = grey_png(8, 8, |_, _| 0);
    let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    let (mut fx, _, e) = engrave_one(|b| {
        b.image(
            &black,
            Affine::new(
                10.0 * c,
                10.0 * s,
                -10.0 * s,
                10.0 * c,
                b.o.x + 50.0,
                b.o.y + 50.0,
            ),
            "Photo 1",
        )
    });
    let _ = fx.s.edit_operation_option(e, "interval", 0.2);
    let g = gcode(&mut fx.s);
    for m in g.burns() {
        for p in [m.from, m.to] {
            let (px, py) = (p.0 - 50.0, (BED_H - p.1) - 50.0);
            // Back into the unit frame: u = (c px + s py) / 10, v = (-s px + c py) / 10.
            let u = (c * px + s * py) / 10.0;
            let v = (-s * px + c * py) / 10.0;
            assert!(
                (-0.011..=1.011).contains(&u) && (-0.011..=1.011).contains(&v),
                "{p:?} -> ({u}, {v})"
            );
        }
    }
    assert!(
        (g.burn_mm() * 0.2 - 100.0).abs() < 6.0,
        "{}",
        g.burn_mm() * 0.2
    );
}

#[test]
fn c5_interval_is_0_01_to_2_and_zero_is_impossible() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    for bad in [0.0, -0.1, 2.01, 0.009, f64::NAN] {
        assert!(
            fx.s.edit_operation_option(e, "interval", bad).is_err(),
            "{bad} refused"
        );
    }
    assert_eq!(fx.s.edit_operation_option(e, "interval", 2.0), Ok(true));
    let g = gcode(&mut fx.s);
    // 8 mm at 2 mm: 4 rows, left 2 dots burn: 4 x 4 mm.
    assert!((g.burn_mm() - 16.0).abs() < 0.01, "{}", g.burn_mm());
    assert_eq!(fx.s.edit_operation_option(e, "interval", 0.01), Ok(true));
}

// ---- criterion 6: alpha, colour ------------------------------------------------------------

#[test]
fn c6_transparent_pixels_never_burn_and_half_alpha_reads_mid_grey() {
    let clear = png(
        16,
        16,
        |x, _| if x < 8 { [0, 0, 0, 0] } else { [0, 0, 0, 255] },
    );
    let (mut fx, _, e) = engrave_one(|b| b.img(&clear, 20.0, 20.0, 8.0, 8.0));
    let m = map16(&gcode(&mut fx.s));
    for r in 0..16 {
        for c in 0..16 {
            assert_eq!(m[r][c].is_some(), c >= 8, "{r},{c}");
        }
    }
    let half = png(16, 16, |_, _| [0, 0, 0, 128]);
    let (mut fx, _, e2) = engrave_one(|b| b.img(&half, 20.0, 20.0, 8.0, 8.0));
    let _ = e;
    dither(&mut fx.s, e2, "grey");
    let g = gcode(&mut fx.s);
    let ss: Vec<u32> = g.burns().iter().map(|m| m.s).collect();
    assert_ne!(ss.len(), 0);
    for s in ss {
        assert!((280..=320).contains(&s), "half of 600: {s}");
    }
}

// ---- criterion 7: working image reduced ------------------------------------------------------

#[test]
fn c7_c36_a_reduced_image_engraves_and_warns() {
    let big = raw_png(9000, 4, |x, _| if x < 4500 { 0 } else { 255 });
    let (mut fx, _, _) = engrave_one(|b| b.img(&big, 20.0, 20.0, 90.0, 4.0));
    assert!(
        texts(&fx.s, "warn").contains(&"1 image was reduced and loses detail.".to_owned()),
        "{:?}",
        texts(&fx.s, "warn")
    );
    let g = gcode(&mut fx.s);
    assert!((g.burn_mm() - 8.0 * 45.0).abs() < 1.0, "{}", g.burn_mm());
}

// ---- criteria 8, 39: determinism ------------------------------------------------------------

#[test]
fn c8_c39_two_exports_and_two_sessions_are_byte_identical() {
    let mut seed = 5u64;
    let noisy = grey_png(40, 30, |_, _| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (seed >> 40) as u8
    });
    let mk = || {
        let (mut fx, _, e) = engrave_one(|b| b.img(&noisy, 20.0, 20.0, 20.0, 15.0));
        dither(&mut fx.s, e, "stucki");
        let _ = fx.s.edit_operation_option(e, "interval", 0.1);
        fx
    };
    let mut a = mk();
    let one = export_bytes(&mut a.s).unwrap();
    let two = export_bytes(&mut a.s).unwrap();
    assert_eq!(one, two);
    let mut b = mk();
    assert_eq!(
        export_bytes(&mut b.s).unwrap(),
        one,
        "a second session writes the same"
    );
    for (bad, what) in [
        ("2026", "date"),
        ("/home", "path"),
        ("Untitled", "project path"),
    ] {
        assert!(!String::from_utf8_lossy(&one).contains(bad), "{what}");
    }
}

// ---- criterion 9: adjustments through the session ------------------------------------------

#[test]
fn c9_brightness_100_engraves_nothing_in_every_mode() {
    let ramp = grey_png(16, 16, |x, _| (x * 17) as u8);
    for mode in [
        "threshold",
        "ordered",
        "floyd_steinberg",
        "jarvis",
        "atkinson",
        "stucki",
        "grey",
    ] {
        let (mut fx, _, e) = engrave_one(|b| b.img(&ramp, 20.0, 20.0, 8.0, 8.0));
        dither(&mut fx.s, e, mode);
        assert_eq!(fx.s.edit_operation_option(e, "brightness", 100.0), Ok(true));
        let burned = export_bytes(&mut fx.s)
            .map(|b| read(&b).unwrap().burn_mm())
            .unwrap_or(0.0);
        assert_eq!(burned, 0.0, "{mode}");
    }
}

#[test]
fn c9_invert_on_black_engraves_nothing_and_on_white_burns_the_square() {
    let black = grey_png(16, 16, |_, _| 0);
    let white = grey_png(16, 16, |_, _| 255);
    let (mut fx, _, e) = engrave_one(|b| b.img(&black, 20.0, 20.0, 8.0, 8.0));
    assert_eq!(fx.s.edit_operation_option(e, "invert", 1.0), Ok(true));
    let burned = export_bytes(&mut fx.s)
        .map(|b| read(&b).unwrap().burn_mm())
        .unwrap_or(0.0);
    assert_eq!(burned, 0.0);
    let (mut fx, _, e) = engrave_one(|b| b.img(&white, 20.0, 20.0, 8.0, 8.0));
    assert_eq!(fx.s.edit_operation_option(e, "invert", 1.0), Ok(true));
    let g = gcode(&mut fx.s);
    assert!((g.burn_mm() - 128.0).abs() < 0.01, "{}", g.burn_mm());
}

// ---- criterion 10: the ramp in the file -----------------------------------------------------

#[test]
fn c10_a_16_step_ramp_burns_monotonically_in_six_modes() {
    // 16 steps of 8 px, 64 rows: 128 x 64 px over 64 x 32 mm at Interval 0.5.
    let ramp = grey_png(128, 64, |x, _| ((x / 8) * 17) as u8);
    for mode in [
        "threshold",
        "ordered",
        "floyd_steinberg",
        "jarvis",
        "atkinson",
        "stucki",
    ] {
        let (mut fx, _, e) = engrave_one(|b| b.img(&ramp, 20.0, 20.0, 64.0, 32.0));
        dither(&mut fx.s, e, mode);
        let g = gcode(&mut fx.s);
        let m = g.dot_map(20.0, BED_H - 20.25, 0.5, 128, 64);
        let counts: Vec<usize> = (0..16)
            .map(|s| {
                m.iter()
                    .map(|r| (s * 8..s * 8 + 8).filter(|&c| r[c].is_some()).count())
                    .sum()
            })
            .collect();
        for w in counts.windows(2) {
            assert!(w[0] >= w[1], "{mode}: {counts:?}");
        }
        assert_eq!(counts[0], 512, "{mode}: black burns fully");
        assert_eq!(counts[15], 0, "{mode}: white burns nothing");
        assert!(
            (g.burn_mm() - burned_power(&m) as f64 * 0.5).abs() < 0.01,
            "{mode}: burned length = dots x Interval"
        );
    }
}

// ---- criterion 11: the Image settings group --------------------------------------------------

#[test]
fn c11_the_group_shows_only_with_a_raster_item_in_the_documented_order() {
    // Superseded by the Image kind rework (2026-11-19): the group shows on every Image row and on
    // no Engrave row (criterion 11); the Image row has no Crosshatch.
    let (mut fx, ids) = laser(|b| {
        let r = b.rect(10.0, 10.0, 10.0, 10.0);
        let i = b.img(&half_black(), 40.0, 40.0, 8.0, 8.0);
        vec![
            r,
            i,
            b.op("engrave", "Engrave 1", &[r, i]),
            b.op("image", "Image 1", &[i]),
            b.op("cut", "Cut 1", &[]),
        ]
    });
    let (shapes_only, with_image) = (ids[2], ids[3]);
    assert!(
        image_group(&mut fx.s, shapes_only).is_null(),
        "an Engrave row shows what 0111 built"
    );
    // Superseded by the Dither on the picture (2026-11-20): the Dither strip and its Threshold
    // are the picture's (its Adjustments group); the Image row of a picture-only operation shows
    // no Dither, and Lowest power while an item burns Grey (ADR 0072 amendment A4, Frage 284 B).
    let g = image_group(&mut fx.s, with_image);
    assert!(g["dither"].is_null(), "no gradient fill, no Dither: {g}");
    assert!(g["lowest_power"].is_null(), "nothing burns Grey");
    let picture = ids[1];
    fx.s.select_output_items(&[picture]);
    let strip: Value = serde_json::from_str(&fx.s.adjustments_view_json()).unwrap();
    let strip = strip["dither"].clone();
    assert_eq!(
        strip["dither"], "floyd_steinberg",
        "default for a new picture"
    );
    let modes: Vec<&str> = strip["modes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["value"].as_str().unwrap())
        .collect();
    assert_eq!(
        modes,
        [
            "threshold",
            "ordered",
            "floyd_steinberg",
            "jarvis",
            "atkinson",
            "stucki",
            "grey",
            "halftone",
            "line_screen",
        ]
    );
    let labels: Vec<&str> = strip["modes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["label"].as_str().unwrap())
        .collect();
    assert_eq!(
        labels,
        [
            "Threshold",
            "Ordered",
            "Floyd-Steinberg",
            "Jarvis",
            "Atkinson",
            "Stucki",
            "Grey",
            "Halftone dots",
            "Line screen",
        ]
    );
    assert!(
        strip["mode_field"].is_null(),
        "no Threshold with Floyd-Steinberg"
    );
    fx.s.select_output_items(&[]);
    let g = image_group(&mut fx.s, with_image);
    let fields: Vec<&str> = g["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["label"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["Brightness", "Contrast", "Gamma"]);
    let units: Vec<&str> = g["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["unit"].as_str().unwrap())
        .collect();
    assert_eq!(units, ["%", "%", ""]);
    assert_eq!(g["invert"], false);
    let vals: Vec<f64> = g["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["value"].as_f64().unwrap())
        .collect();
    assert_eq!(vals, [0.0, 0.0, 1.0]);

    dither(&mut fx.s, with_image, "threshold");
    fx.s.select_output_items(&[picture]);
    let strip: Value = serde_json::from_str(&fx.s.adjustments_view_json()).unwrap();
    let field = &strip["dither"]["mode_field"];
    assert_eq!(field["label"], "Threshold");
    assert_eq!(field["value"], 50.0);
    assert_eq!(field["unit"], "%");
    fx.s.select_output_items(&[]);
    dither(&mut fx.s, with_image, "grey");
    let g = image_group(&mut fx.s, with_image);
    assert_eq!(g["lowest_power"]["label"], "Lowest power");
    assert_eq!(g["lowest_power"]["value"], 0.0);
    // The hatch options keep their place.
    let opts = row_of(&fx.s, with_image)["settings"]["options"].clone();
    for k in ["bidirectional", "scan_angle_deg", "overscan_pct"] {
        assert!(!opts["hatch"][k].is_null(), "{k}");
    }
    assert!(
        opts["hatch"]["crosshatch"].is_null(),
        "no Crosshatch on Image"
    );
    // No group on a Cut row.
    assert!(image_group(&mut fx.s, ids[4]).is_null());
}

#[test]
fn c11_settings_stay_stored_while_the_group_is_hidden() {
    let (mut fx, ids) = laser(|b| {
        let r = b.rect(10.0, 10.0, 10.0, 10.0);
        let i = b.img(&half_black(), 40.0, 40.0, 8.0, 8.0);
        vec![r, i, b.op("image", "Image 1", &[r, i])]
    });
    let e = ids[2];
    dither(&mut fx.s, e, "jarvis");
    assert_eq!(fx.s.edit_operation_option(e, "gamma", 1.5), Ok(true));
    assert!(fx.s.select_output_items(&[ids[1]]));
    fx.s.delete_selected();
    // Superseded by the Image kind rework (2026-11-19): the group stays on an Image row without a
    // raster item (criterion 11).
    assert!(
        !image_group(&mut fx.s, e).is_null(),
        "shown on every Image row"
    );
    let o = options_of(&fx.s, e);
    assert_eq!(o.get("dither"), Some(&json!("jarvis")), "{o:?}");
    assert_eq!(o.get("gamma").and_then(Value::as_f64), Some(1.5));
}

// ---- criterion 12: ranges, rounding, undo --------------------------------------------------

#[test]
fn c12_out_of_range_is_refused_with_the_range_and_writes_nothing() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    let before = fx.s.undo_len();
    for (field, bad, needle) in [
        ("brightness", 101.0, "-100 to 100"),
        ("brightness", -101.0, "-100 to 100"),
        ("contrast", 100.5, "-100 to 100"),
        ("contrast", -150.0, "-100 to 100"),
        ("gamma", 4.01, "0.10 to 4.00"),
        ("gamma", 0.09, "0.10 to 4.00"),
        ("gamma", 0.0, "0.10 to 4.00"),
        ("threshold", 101.0, "0 to 100"),
        ("threshold", -1.0, "0 to 100"),
        ("lowest_power", 100.1, "0 to 100"),
        ("lowest_power", -0.5, "0 to 100"),
    ] {
        let r = fx.s.edit_operation_option(e, field, bad);
        let msg = r.expect_err(&format!("{field} {bad} refused"));
        assert!(msg.contains(needle), "{field} {bad}: {msg}");
    }
    assert_eq!(fx.s.undo_len(), before, "nothing written");
}

#[test]
fn c12_extra_decimals_round_to_the_step() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    assert_eq!(fx.s.edit_operation_option(e, "gamma", 1.234), Ok(true));
    assert_eq!(options_of(&fx.s, e).get("gamma"), Some(&json!(1.23)));
    assert_eq!(fx.s.edit_operation_option(e, "brightness", 12.6), Ok(true));
    assert_eq!(
        options_of(&fx.s, e)
            .get("brightness_pct")
            .and_then(Value::as_f64),
        Some(13.0)
    );
    assert_eq!(fx.s.edit_operation_option(e, "threshold", 40.4), Ok(true));
    assert_eq!(
        options_of(&fx.s, e)
            .get("threshold_pct")
            .and_then(Value::as_f64),
        Some(40.0)
    );
}

#[test]
fn c12_every_image_edit_is_one_undo_step_edit_settings_and_undo_restores() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    let edits: Vec<(&str, Box<dyn Fn(&mut Session) -> bool>)> = vec![
        (
            "dither",
            Box::new(move |s| s.set_operation_dither(e, "atkinson")),
        ),
        (
            "threshold",
            Box::new(move |s| s.edit_operation_option(e, "threshold", 30.0) == Ok(true)),
        ),
        (
            "brightness",
            Box::new(move |s| s.edit_operation_option(e, "brightness", -20.0) == Ok(true)),
        ),
        (
            "contrast",
            Box::new(move |s| s.edit_operation_option(e, "contrast", 15.0) == Ok(true)),
        ),
        (
            "gamma",
            Box::new(move |s| s.edit_operation_option(e, "gamma", 2.0) == Ok(true)),
        ),
        (
            "invert",
            Box::new(move |s| s.edit_operation_option(e, "invert", 1.0) == Ok(true)),
        ),
        (
            "lowest_power",
            Box::new(move |s| s.edit_operation_option(e, "lowest_power", 10.0) == Ok(true)),
        ),
    ];
    for (name, edit) in edits {
        let before_opts = options_of(&fx.s, e);
        let before_file = export_bytes(&mut fx.s);
        let n = fx.s.undo_len();
        assert!(edit(&mut fx.s), "{name} written");
        assert_eq!(fx.s.undo_len(), n + 1, "{name}: one step");
        assert_eq!(
            undo_name(&mut fx.s).as_deref(),
            Some("Edit settings"),
            "{name}"
        );
        assert_ne!(options_of(&fx.s, e), before_opts, "{name} stored");
        let _ = fx.s.undo();
        assert_eq!(options_of(&fx.s, e), before_opts, "{name}: undo restores");
        assert_eq!(
            export_bytes(&mut fx.s),
            before_file,
            "{name}: the file follows"
        );
        let _ = fx.s.redo();
        assert_ne!(options_of(&fx.s, e), before_opts, "{name}: redo");
        let _ = fx.s.undo();
    }
    // The same value writes nothing.
    assert!(!fx.s.set_operation_dither(e, "threshold"));
    assert!(!fx.s.set_operation_dither(e, "halftone_x"));
}

#[test]
fn c12_an_edit_recomputes_the_checks_and_the_view_in_the_same_frame() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    let before = out(&fx.s)["total"]["lengths"].clone();
    assert_eq!(fx.s.edit_operation_option(e, "brightness", 100.0), Ok(true));
    let after = out(&fx.s)["total"]["lengths"].clone();
    assert_ne!(before, after, "the total follows at once");
}

// ---- criterion 13: stored keys and damage ---------------------------------------------------

#[test]
fn c13_the_settings_are_stored_as_the_documented_op_options_keys() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    dither(&mut fx.s, e, "grey");
    for (f, v) in [
        ("threshold", 30.0),
        ("brightness", -5.0),
        ("contrast", 7.0),
        ("gamma", 0.8),
        ("lowest_power", 12.5),
        ("invert", 1.0),
    ] {
        assert_eq!(fx.s.edit_operation_option(e, f, v), Ok(true), "{f}");
    }
    let o = options_of(&fx.s, e);
    assert_eq!(o.get("dither"), Some(&json!("grey")));
    for (k, v) in [
        ("threshold_pct", 30.0),
        ("brightness_pct", -5.0),
        ("contrast_pct", 7.0),
        ("gamma", 0.8),
        ("lowest_power_pct", 12.5),
    ] {
        assert_eq!(o.get(k).and_then(Value::as_f64), Some(v), "{k} in {o:?}");
    }
    assert_eq!(o.get("invert"), Some(&json!(true)));
    for k in o.keys() {
        assert!(
            [
                "dither",
                "threshold_pct",
                "brightness_pct",
                "contrast_pct",
                "gamma",
                "invert",
                "lowest_power_pct",
                "crosshatch",
                "bidirectional",
                "scan_angle_deg",
                "overscan_pct",
                "z_offset_mm",
                "interval_mm",
            ]
            .contains(&k.as_str()),
            "unexpected key {k}"
        );
    }
}

/// Opens a page with an Engrave and a Cut on which `options` are written raw.
fn damaged(engrave: Value, cut: Value) -> Fx {
    let (fx, _) = laser(|b| {
        let i = b.img(&half_black(), 20.0, 20.0, 8.0, 8.0);
        let r = b.rect(50.0, 50.0, 10.0, 10.0);
        let e = b.op("image", "Image 1", &[i]);
        let c = b.op("cut", "Cut 1", &[r]);
        b.d.write_operation_settings(
            curvyo_document_core::SettingsStep::EditSettings,
            &[
                curvyo_document_core::SettingsWrite {
                    operation: e,
                    value: Some(
                        json!({"main": {"power_pct": 60.0, "speed_mm_min": 3000, "passes": 1,
                            "interval_mm": 0.5}})
                        .as_object()
                        .unwrap()
                        .clone(),
                    ),
                },
                curvyo_document_core::SettingsWrite {
                    operation: c,
                    value: Some(
                        json!({"main": {"power_pct": 80.0, "speed_mm_min": 600, "passes": 1}})
                            .as_object()
                            .unwrap()
                            .clone(),
                    ),
                },
            ],
        )
        .unwrap();
        if let Some(o) = engrave.as_object() {
            b.d.write_operation_options(e, o).unwrap();
        }
        if let Some(o) = cut.as_object() {
            b.d.write_operation_options(c, o).unwrap();
        }
        vec![]
    });
    fx
}

#[test]
fn c13_c41_each_damage_reason_stops_the_operation_with_one_warn_line() {
    for (what, e, c) in [
        (
            "unknown dither",
            json!({"dither": "halftone_x"}),
            Value::Null,
        ),
        ("gamma 4.01", json!({"gamma": 4.01}), Value::Null),
        ("gamma three decimals", json!({"gamma": 1.234}), Value::Null),
        ("threshold 101", json!({"threshold_pct": 101}), Value::Null),
        (
            "brightness -101",
            json!({"brightness_pct": -101}),
            Value::Null,
        ),
        ("contrast 1.5", json!({"contrast_pct": 1.5}), Value::Null),
        (
            "lowest power 100.5",
            json!({"lowest_power_pct": 100.5}),
            Value::Null,
        ),
        ("invert not a bool", json!({"invert": 1}), Value::Null),
        ("dither on a Cut", Value::Null, json!({"dither": "ordered"})),
        ("gamma on a Cut", Value::Null, json!({"gamma": 1.0})),
    ] {
        let mut fx = damaged(e, c);
        let warns = texts(&fx.s, "warn");
        let line = warns
            .iter()
            .find(|t| t.contains("damaged settings"))
            .unwrap_or_else(|| panic!("{what}: {warns:?} / {:?}", texts(&fx.s, "block")));
        assert!(line.starts_with("1 operation"), "{what}: {line}");
        let g = export_bytes(&mut fx.s).map(|b| read(&b).unwrap());
        let burns = g.map(|g| g.burns()).unwrap_or_default();
        if what.contains("Cut") {
            assert!(
                burns.iter().all(|m| m.s != 800),
                "{what}: the Cut does not run"
            );
            assert!(burns.iter().any(|m| m.s == 600), "{what}: the Engrave runs");
        } else {
            assert!(
                burns.iter().all(|m| m.s != 600),
                "{what}: the Engrave does not run"
            );
        }
    }
    // A missing key reads its default: no damage.
    let fx = damaged(json!({"bidirectional": true}), Value::Null);
    assert!(!texts(&fx.s, "warn").iter().any(|t| t.contains("damaged")));
}

#[test]
fn c13_a_material_pick_never_changes_the_image_settings() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    dither(&mut fx.s, e, "stucki");
    assert_eq!(fx.s.edit_operation_option(e, "gamma", 2.5), Ok(true));
    let before = options_of(&fx.s, e);
    let material = fx.s.add_material("Birch 3 mm", 3.0, &[3; 16]);
    let id = material.unwrap();
    let _ = fx.s.use_material_on(&[e], &id);
    let _ = fx.s.reset_operation_to_material(e);
    let after = options_of(&fx.s, e);
    for k in ["dither", "gamma"] {
        assert_eq!(before.get(k), after.get(k), "{k}");
    }
}

// ---- criteria 14, 15: rows, order, gaps -------------------------------------------------------

// Superseded by the Image kind rework (2026-11-19): an Image block holds its raster items only,
// in stacking order, bottom first, and no outline; a Colour-filled rect there burns nothing
// (criteria 14, 48).
#[test]
fn c14_raster_items_run_bottom_first_and_a_plain_shape_runs_nothing_in_image() {
    let (mut fx, ids) = laser(|b| {
        // Stacking: image A (bottom), the rect, image B (top).
        let a = b.img(&grey_png(4, 4, |_, _| 0), 100.0, 20.0, 4.0, 4.0);
        let r = b.rect(20.0, 20.0, 4.0, 4.0);
        b.d.edit_style(&[r], &StyleEdit::FillEnabled(true)).unwrap();
        let bb = b.img(&grey_png(4, 4, |_, _| 0), 60.0, 20.0, 4.0, 4.0);
        vec![a, r, bb, b.op("image", "Image 1", &[bb, r, a])]
    });
    let e = ids[3];
    set(&mut fx.s, e, 60.0, 3000.0, Some(0.5));
    dither(&mut fx.s, e, "threshold");
    let g = gcode(&mut fx.s);
    let phases: Vec<char> = g
        .burns()
        .iter()
        .map(|m| {
            if m.from.0 < 30.0 {
                'h'
            } else if m.from.0 < 80.0 {
                'b'
            } else {
                'a'
            }
        })
        .collect();
    let mut order: Vec<char> = Vec::new();
    for p in phases {
        if order.last() != Some(&p) {
            order.push(p);
        }
    }
    assert_eq!(
        order,
        ['a', 'b'],
        "raster bottom first, the rect burns nothing"
    );
}
#[test]
fn c15_a_half_white_image_writes_no_move_over_the_white_half() {
    let (mut fx, _, _) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    let g = gcode(&mut fx.s);
    for m in g.moves.iter().filter(|m| (m.from.1 - m.to.1).abs() < 1e-9) {
        let lo = m.from.0.min(m.to.0);
        let hi = m.from.0.max(m.to.0);
        if (BED_H - 28.0..=BED_H - 20.0).contains(&m.from.1) {
            assert!(
                hi <= 24.0 + 1e-6 || lo >= 28.0 - 1e-6 || m.len() < 1e-9,
                "a move over white: {m:?}"
            );
        }
    }
}

#[test]
fn c15_gaps_over_5_mm_travel_at_max_feed_shorter_gaps_sweep_off() {
    // 80 px over 40 mm at 0.5: black 0..10 mm, white 10..16 mm (6 mm gap), black 16..20,
    // white 20..24 (4 mm gap), black 24..28, white to the end (trailing, not travelled).
    let bands = grey_png(80, 2, |x, _| {
        let mm = f64::from(x) * 0.5;
        if mm < 10.0 || (16.0..20.0).contains(&mm) || (24.0..28.0).contains(&mm) {
            0
        } else {
            255
        }
    });
    let (mut fx, _, _) = engrave_one(|b| b.img(&bands, 20.0, 20.0, 40.0, 1.0));
    let g = gcode(&mut fx.s);
    eprintln!("{}", text_of(&mut fx.s));
    assert!((g.burn_mm() - 2.0 * 18.0).abs() < 0.01, "{}", g.burn_mm());
    let row_y = BED_H - 20.25;
    let on_row: Vec<_> = g
        .moves
        .iter()
        .filter(|m| (m.from.1 - row_y).abs() < 1e-6 && (m.to.1 - row_y).abs() < 1e-6)
        .collect();
    // The 6 mm gap is a G0; the 4 mm gap is a laser-off G1 (S0).
    let g0_gap = on_row
        .iter()
        .any(|m| m.rapid && (m.len() - 6.0).abs() < 1e-6);
    let off_gap = on_row
        .iter()
        .any(|m| !m.rapid && !m.burn && (m.len() - 4.0).abs() < 1e-6);
    assert!(g0_gap, "6 mm gap travels: {on_row:#?}");
    assert!(off_gap, "4 mm gap sweeps with the laser off: {on_row:#?}");
    // Nothing beyond 48 mm (trailing white) is travelled.
    assert!(
        g.moves
            .iter()
            .filter(|m| (m.from.1 - row_y).abs() < 0.3)
            .all(|m| m.from.0.max(m.to.0) <= 48.0 + 1e-6),
        "trailing white not travelled"
    );
}

#[test]
fn c15_a_row_with_no_burned_dot_writes_nothing() {
    // Black rows 0..3 and 12..15 (of 16), white 4..11.
    let rows = grey_png(16, 16, |_, y| if (4..12).contains(&y) { 255 } else { 0 });
    let (mut fx, _, _) = engrave_one(|b| b.img(&rows, 20.0, 20.0, 8.0, 8.0));
    let g = gcode(&mut fx.s);
    let ys: Vec<f64> = g.rows().iter().map(|r| r.0).collect();
    assert_eq!(ys.len(), 8, "{ys:?}");
    for m in &g.moves {
        if (m.from.1 - m.to.1).abs() < 1e-9 {
            let page_y = BED_H - m.from.1;
            assert!(
                !(22.0..26.0).contains(&page_y),
                "no move along a white row: {m:?}"
            );
        }
    }
}

#[test]
fn c15_c19_a_solid_area_steps_one_interval_and_writes_no_line_per_dot() {
    let black = grey_png(40, 40, |_, _| 0);
    let (mut fx, _, e) = engrave_one(|b| b.img(&black, 20.0, 20.0, 20.0, 20.0));
    let _ = fx.s.edit_operation_option(e, "interval", 0.1);
    let g = gcode(&mut fx.s);
    assert_eq!(g.rows().len(), 200);
    // Rows + steps + header: never 40,000 dots.
    assert!(g.code_lines < 200 * 3 + 30, "{} lines", g.code_lines);
    let steps: Vec<_> = g
        .moves
        .iter()
        .filter(|m| !m.burn && (m.from.0 - m.to.0).abs() < 1e-9 && (m.from.1 - m.to.1).abs() > 1e-9)
        .collect();
    assert!(steps.len() >= 199);
    assert!(
        steps
            .iter()
            .filter(|m| (m.len() - 0.1).abs() < 1e-6)
            .count()
            >= 199,
        "one-Interval steps"
    );
}

// ---- criterion 16: bidirectional, overscan, crosshatch ----------------------------------------

fn directions(g: &Gcode) -> Vec<bool> {
    g.rows()
        .iter()
        .map(|(y, _)| {
            let m = g
                .burns()
                .into_iter()
                .find(|m| (m.from.1 - y).abs() < 1e-6)
                .unwrap();
            m.to.0 > m.from.0
        })
        .collect()
}

#[test]
fn c16_bidirectional_alternates_and_off_runs_one_way_with_travel_back() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    let on = gcode(&mut fx.s);
    let d = directions(&on);
    for w in d.windows(2) {
        assert_ne!(w[0], w[1], "{d:?}");
    }
    assert_eq!(
        fx.s.edit_operation_option(e, "bidirectional", 0.0),
        Ok(true)
    );
    let off = gcode(&mut fx.s);
    let d = directions(&off);
    assert!(d.iter().all(|&x| x == d[0]), "{d:?}");
    assert!((on.burn_mm() - off.burn_mm()).abs() < 0.01);
    assert!(
        off.rapid_mm() > on.rapid_mm() + 50.0,
        "travel back each row"
    );
}

#[test]
fn c16_overscan_extends_each_row_with_the_laser_off() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    let plain = gcode(&mut fx.s);
    assert_eq!(fx.s.edit_operation_option(e, "overscan", 50.0), Ok(true));
    let g = gcode(&mut fx.s);
    assert!((g.burn_mm() - plain.burn_mm()).abs() < 0.01);
    let offs: Vec<_> = g.moves.iter().filter(|m| !m.rapid && !m.burn).collect();
    assert!(offs.len() >= 32, "two run-outs per row: {}", offs.len());
    for m in offs {
        assert!((m.from.1 - m.to.1).abs() < 1e-9, "along the row");
        assert!(m.from.0.max(m.to.0) <= 20.0 + 1e-6 || m.from.0.min(m.to.0) >= 24.0 - 1e-6);
    }
}

#[test]
fn c16_crosshatch_is_ignored_for_a_raster_item() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    let a = export_bytes(&mut fx.s).unwrap();
    // Superseded by the Image kind rework (2026-11-19): an Image operation has no Crosshatch, so
    // the edit is refused and nothing changes (criteria 2, 16).
    assert_ne!(fx.s.edit_operation_option(e, "crosshatch", 1.0), Ok(true));
    let b = export_bytes(&mut fx.s).unwrap();
    assert_eq!(read(&a).unwrap().burn_mm(), read(&b).unwrap().burn_mm());
}

#[test]
fn c16_scan_angle_90_turns_the_rows() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    assert_eq!(fx.s.edit_operation_option(e, "scan_angle", 90.0), Ok(true));
    let g = gcode(&mut fx.s);
    assert!(
        g.burns().iter().all(|m| (m.from.0 - m.to.0).abs() < 1e-6),
        "vertical rows"
    );
    assert!((g.burn_mm() - 64.0).abs() < 0.01, "{}", g.burn_mm());
}

// ---- criterion 17: passes ---------------------------------------------------------------------

#[test]
fn c17_each_pass_burns_the_same_dots() {
    let mut seed = 9u64;
    let noisy = grey_png(16, 16, |_, _| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(3);
        (seed >> 40) as u8
    });
    let (mut fx, _, e) = engrave_one(|b| b.img(&noisy, 20.0, 20.0, 8.0, 8.0));
    dither(&mut fx.s, e, "floyd_steinberg");
    let one = gcode(&mut fx.s);
    passes(&mut fx.s, e, 3.0);
    let three = gcode(&mut fx.s);
    assert!((three.burn_mm() - 3.0 * one.burn_mm()).abs() < 0.01);
    let a: Vec<_> = one.burns().iter().map(|m| (m.from, m.to)).collect();
    let n = a.len();
    let b: Vec<_> = three.burns().iter().map(|m| (m.from, m.to)).collect();
    assert_eq!(b.len(), 3 * n);
    for p in 0..3 {
        let mut x: Vec<_> = b[p * n..(p + 1) * n].to_vec();
        let mut y = a.clone();
        let key = |v: &((f64, f64), (f64, f64))| {
            let (lo, hi) = if v.0.0 <= v.1.0 {
                (v.0, v.1)
            } else {
                (v.1, v.0)
            };
            (
                (lo.0 * 1000.0) as i64,
                (lo.1 * 1000.0) as i64,
                (hi.0 * 1000.0) as i64,
            )
        };
        x.sort_by_key(key);
        y.sort_by_key(key);
        let kx: Vec<_> = x.iter().map(key).collect();
        let ky: Vec<_> = y.iter().map(key).collect();
        assert_eq!(kx, ky, "pass {p}: the same dots");
    }
}

// ---- criterion 18: feed ------------------------------------------------------------------------

#[test]
fn c18_a_speed_above_max_feed_is_written_at_max_and_warns() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    assert!(
        fx.s.edit_operation_setting(e, "main", "speed", 6000.0)
            .is_ok()
    );
    let _ = fx.s.set_machine_max_feed(5000.0);
    let g = gcode(&mut fx.s);
    assert!(g.burns().iter().all(|m| m.f == 5000.0), "{:?}", g.feeds);
    assert!(
        !texts(&fx.s, "warn").is_empty(),
        "the gate warns about the feed"
    );
}

// ---- criterion 19: GRBL M4 and M3 ----------------------------------------------------------

#[test]
fn c19_m4_writes_s_per_run_s0_inside_and_f_only_where_it_changes() {
    // Two greys in one row (dark and darker) separated by a 2 mm white gap.
    let img = grey_png(16, 1, |x, _| match x {
        0..=3 => 0,
        4..=7 => 128,
        8..=11 => 255,
        _ => 0,
    });
    let (mut fx, _, e) = engrave_one(|b| b.img(&img, 20.0, 20.0, 8.0, 0.5));
    dither(&mut fx.s, e, "grey");
    let g = gcode(&mut fx.s);
    let burns = g.burns();
    assert_eq!(burns.len(), 3, "one G1 per run: {burns:#?}");
    assert_eq!(burns[0].s, 600);
    assert!((290..=310).contains(&burns[1].s), "{}", burns[1].s);
    assert_eq!(burns[2].s, 600);
    let off: Vec<_> = g.moves.iter().filter(|m| !m.rapid && !m.burn).collect();
    assert!(
        off.iter().any(|m| m.s == 0 && (m.len() - 2.0).abs() < 1e-6),
        "S0 over the 2 mm gap: {:#?}",
        g.moves
    );
    assert_eq!(g.feeds.len(), 1, "F once: {:?}", g.feeds);
    assert!(g.spindle_words.contains(&4) && !g.spindle_words.contains(&3));
}

#[test]
fn c19_m3_turns_on_and_off_per_run_with_g0_gaps() {
    let img = grey_png(16, 1, |x, _| if (4..8).contains(&x) { 255 } else { 0 });
    let (mut fx, _, _) = engrave_one(|b| b.img(&img, 20.0, 20.0, 8.0, 0.5));
    fx.s.set_machine_laser_mode(false);
    let g = gcode(&mut fx.s);
    let burns = g.burns();
    assert_eq!(burns.len(), 2);
    assert!(burns.iter().all(|m| m.spindle == 3));
    assert!(
        g.moves.iter().filter(|m| !m.burn).all(|m| m.rapid),
        "gaps are G0: {:#?}",
        g.moves
    );
    assert!(
        g.moves.iter().filter(|m| m.rapid).all(|m| m.spindle == 5),
        "M5 before G0"
    );
}

#[test]
fn c33_grey_on_m3_blocks() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    fx.s.set_machine_laser_mode(false);
    dither(&mut fx.s, e, "grey");
    let blocks = texts(&fx.s, "block");
    assert!(
        blocks.contains(&"Image 1 needs laser mode M4 for Grey.".to_owned()),
        "{blocks:?}"
    );
    let c = checks(&fx.s)
        .into_iter()
        .find(|c| c["text"] == "Image 1 needs laser mode M4 for Grey.")
        .unwrap();
    assert_eq!(c["target"]["kind"], "operations");
    assert_eq!(c["target"]["focus"], "dither");
    assert!(export_bytes(&mut fx.s).is_none());
}

// ---- criterion 21: time and lengths --------------------------------------------------------

#[test]
fn c21_the_spec_example_reads_about_1_min_40_s_and_burn_10_m_travel_10_mm() {
    let black = grey_png(1000, 100, |_, _| 0);
    let (mut fx, ids) = laser(|b| {
        let i = b.img(&black, 0.0, 0.0, 100.0, 10.0);
        vec![i, b.op("image", "Image 1", &[i])]
    });
    let e = ids[1];
    let _ = fx.s.set_machine_max_feed(60_000.0);
    // The image starts at the top-left of the bed; put the origin there so no travel to it.
    let _ = fx.s.set_machine_origin("top_left");
    set(&mut fx.s, e, 60.0, 6000.0, Some(0.1));
    dither(&mut fx.s, e, "threshold");
    let total = out(&fx.s)["total"].clone();
    let g = gcode(&mut fx.s);
    assert!((g.burn_mm() - 10_000.0).abs() < 0.01, "{}", g.burn_mm());
    assert_eq!(total["time"], "About 1 min 40 s", "{total}");
    let lengths = total["lengths"].as_str().unwrap();
    assert!(lengths.starts_with("Burn 10.00 m · travel "), "{lengths}");
    // 99 steps of 0.1 mm = 9.9 mm, plus the way to the first row and home.
    let steps: f64 = g
        .moves
        .iter()
        .filter(|m| !m.burn && (m.from.0 - m.to.0).abs() < 1e-9 && (m.len() - 0.1).abs() < 1e-9)
        .map(|m| m.len())
        .sum();
    assert!((steps - 9.9).abs() < 1e-6, "{steps}");
}

// ---- criterion 22: the play bar --------------------------------------------------------------

fn secs(t: &str) -> f64 {
    let mut s = 0.0;
    let mut n = String::new();
    for w in t.split_whitespace() {
        if let Ok(v) = w.replace(',', "").parse::<f64>() {
            n = v.to_string();
            continue;
        }
        let v: f64 = n.parse().unwrap_or(0.0);
        match w {
            "h" => s += v * 3600.0,
            "min" => s += v * 60.0,
            "s" => s += v,
            _ => {}
        }
        n.clear();
    }
    s
}

#[test]
fn c22_slider_at_half_reads_half_of_the_raster_job() {
    let black = grey_png(400, 100, |_, _| 0);
    let (mut fx, _, e) = engrave_one(|b| b.img(&black, 20.0, 20.0, 40.0, 10.0));
    let _ = fx.s.edit_operation_option(e, "interval", 0.1);
    let page = fx.page.clone();
    assert!(fx.s.toggle_toolpath_view(&page));
    fx.s.set_play_position(0.5);
    let r = fx.s.play_view().unwrap().readout;
    let (at, of) = r.split_once(" of ").unwrap();
    let (at, of) = (secs(at), secs(of));
    assert!(of > 60.0, "{r}");
    assert!((at - of / 2.0).abs() <= 0.5 + 1.0, "{r}");
}

// ---- criterion 20: Toolpath view = file -------------------------------------------------------

#[test]
fn c20_the_toolpath_totals_are_the_file() {
    let (mut fx, _, _) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    let g = gcode(&mut fx.s);
    let page = fx.page.clone();
    assert!(fx.s.toggle_toolpath_view(&page));
    let _ = fx.s.frame_draw_list();
    let lengths = out(&fx.s)["total"]["lengths"].as_str().unwrap().to_owned();
    assert_eq!(
        lengths,
        curvyo_laser_core::job::lengths_text(g.burn_mm(), g.rapid_mm() + g.off_mm()),
        "file burn {} travel {}",
        g.burn_mm(),
        g.rapid_mm() + g.off_mm()
    );
    let labels = fx.s.toolpath_ring_labels();
    assert!(!labels.is_empty(), "the operation keeps its ring");
}

#[test]
fn c20_the_operations_view_draws_the_dots_and_toggling_writes_nothing() {
    let (mut fx, _, _) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    let packed = fx.s.pack("0.1.0").unwrap();
    let page = fx.page.clone();
    let n = fx.s.undo_len();
    let _ = fx.s.toggle_operations_view(&page);
    let _ = fx.s.frame_draw_list();
    let _ = fx.s.toggle_operations_view(&page);
    let _ = fx.s.frame_draw_list();
    assert_eq!(fx.s.undo_len(), n);
    assert_eq!(fx.s.pack("0.1.0").unwrap(), packed);
}

// ---- criteria 24, 26, 27, 30: gradients ------------------------------------------------------

#[test]
fn c24_grey_mode_on_a_black_to_white_gradient_runs_full_half_none() {
    let (mut fx, _, e) = engrave_one(|b| {
        let r = b.rect(20.0, 20.0, 20.0, 1.0);
        b.shade(r);
        r
    });
    dither(&mut fx.s, e, "grey");
    let _ = fx.s.edit_operation_option(e, "interval", 0.5);
    let g = gcode(&mut fx.s);
    let m = g.dot_map(20.0, BED_H - 20.25, 0.5, 40, 2);
    let row = &m[0];
    // The first dot's centre is 1.25 % into the shade: 600 x 0.9875.
    assert!(row[0].is_some_and(|s| s >= 585), "leftmost full: {row:?}");
    let mid = row[20].unwrap_or(0);
    assert!((270..=330).contains(&mid), "middle about half: {row:?}");
    assert!(row[39].is_none_or(|s| s <= 20), "rightmost none: {row:?}");
}

#[test]
fn c24_angle_90_shades_over_the_height() {
    let (mut fx, _, e) = engrave_one(|b| {
        let r = b.rect(20.0, 20.0, 2.0, 20.0);
        shade_with(
            b.d,
            r,
            GradientGeometry::Linear {
                start: BoxPoint::new(0.5, 0.0).unwrap(),
                end: BoxPoint::new(0.5, 1.0).unwrap(),
            },
            bw(),
        );
        r
    });
    let _ = fx.s.edit_operation_option(e, "interval", 0.5);
    let g = gcode(&mut fx.s);
    let (_, y0, _, y1) = g.burn_extent().unwrap();
    // Threshold 50 %: the black top half burns.
    assert!(
        y0 >= BED_H - 30.0 - 0.6 && y1 <= BED_H - 20.0 + 1e-6,
        "{y0} {y1}"
    );
}

#[test]
fn c27_adjustments_apply_to_a_gradient_like_an_image() {
    let (mut fx, _, e) = engrave_one(|b| {
        let r = b.rect(20.0, 20.0, 20.0, 5.0);
        b.shade(r);
        r
    });
    let base = gcode(&mut fx.s).burn_mm();
    assert_eq!(fx.s.edit_operation_option(e, "invert", 1.0), Ok(true));
    let inv = gcode(&mut fx.s);
    let (x0, _, _, _) = inv.burn_extent().unwrap();
    assert!(x0 >= 29.5, "inverted: the right half burns: {x0}");
    assert_eq!(fx.s.edit_operation_option(e, "invert", 0.0), Ok(true));
    assert_eq!(fx.s.edit_operation_option(e, "brightness", 100.0), Ok(true));
    let none = export_bytes(&mut fx.s).map_or(0.0, |b| read(&b).unwrap().burn_mm());
    assert_eq!(none, 0.0);
    assert!(base > 0.0);
    for mode in ["ordered", "floyd_steinberg", "jarvis", "atkinson", "stucki"] {
        assert_eq!(
            fx.s.edit_operation_option(e, "brightness", 0.0).is_ok(),
            true
        );
        dither(&mut fx.s, e, mode);
        let g = gcode(&mut fx.s);
        let left: f64 = g
            .burns()
            .iter()
            .filter(|m| m.from.0.max(m.to.0) <= 30.0 + 1e-6)
            .map(|m| m.len())
            .sum();
        let right: f64 = g
            .burns()
            .iter()
            .filter(|m| m.from.0.min(m.to.0) >= 30.0 - 1e-6)
            .map(|m| m.len())
            .sum();
        assert!(left > right, "{mode}: the ramp burns more at the black end");
    }
}

#[test]
fn c30_a_gradient_switched_to_colour_engraves_as_a_hatch_again() {
    let (mut fx, r, _) = engrave_one(|b| {
        let r = b.rect(20.0, 20.0, 10.0, 10.0);
        b.shade(r);
        r
    });
    let raster = gcode(&mut fx.s).burn_mm();
    assert!(raster > 0.0);
    assert!(fx.s.select_output_items(&[r]));
    assert!(fx.s.set_fill_kind("solid"));
    // Superseded by the Image kind rework (2026-11-19): in an Image operation the shape then
    // burns nothing (criterion 30); an Engrave keeps hatching it.
    let burned = export_bytes(&mut fx.s).map_or(0.0, |b| read(&b).unwrap().burn_mm());
    assert!(
        burned < 1e-6,
        "a Colour fill burns nothing in Image: {burned}"
    );
}

#[test]
fn c28_a_gradient_in_no_operation_burns_nothing() {
    let (mut fx, _) = laser(|b| {
        let r = b.rect(20.0, 20.0, 10.0, 10.0);
        b.shade(r);
        let c = b.rect(50.0, 50.0, 10.0, 10.0);
        vec![r, c, b.op("cut", "Cut 1", &[c])]
    });
    let ops: Value = ops_view(&fx.s);
    let _ = ops;
    let ids: Vec<NodeId> = vec![];
    let _ = ids;
    let cut = unpack(1, &fx.s.pack("t").unwrap()).unwrap();
    let _ = cut;
    // The Cut has no settings: give it some through its row.
    let rows = ops_view(&fx.s)["rows"].as_array().unwrap().clone();
    let id: NodeId = serde_json::from_value(rows[0]["id"].clone()).unwrap();
    set(&mut fx.s, id, 80.0, 600.0, None);
    let g = gcode(&mut fx.s);
    assert!(
        (g.burn_mm() - 40.0).abs() < 0.01,
        "only the Cut square: {}",
        g.burn_mm()
    );
}

// ---- criteria 31 to 36: checks ------------------------------------------------------------

#[test]
fn c31_an_image_outside_the_bed_blocks_and_selects_it() {
    let (mut fx, i, _) = engrave_one(|b| b.img(&half_black(), 395.0, 20.0, 8.0, 8.0));
    let blocks = texts(&fx.s, "block");
    assert!(
        blocks.contains(&"1 item lies outside the 400 \u{d7} 300 mm bed.".to_owned()),
        "{blocks:?}"
    );
    let c = checks(&fx.s)
        .into_iter()
        .find(|c| c["severity"] == "block")
        .unwrap();
    let ids: Vec<NodeId> = serde_json::from_value(c["target"]["ids"].clone()).unwrap();
    assert_eq!(ids, [i]);
    assert!(export_bytes(&mut fx.s).is_none());
}

#[test]
fn c31_an_image_bigger_than_the_bed_blocks() {
    let (fx, _, _) = engrave_one(|b| b.img(&half_black(), 0.0, 0.0, 500.0, 400.0));
    let blocks = texts(&fx.s, "block");
    assert!(
        blocks
            .iter()
            .any(|t| t.contains("outside the 400 \u{d7} 300 mm bed")),
        "{blocks:?}"
    );
}

#[test]
fn c31_an_image_on_the_bed_edge_within_0_01_mm_does_not_block() {
    let (fx, _, _) = engrave_one(|b| b.img(&half_black(), 392.005, 20.0, 8.0, 8.0));
    assert!(
        !texts(&fx.s, "block").iter().any(|t| t.contains("outside")),
        "{:?}",
        texts(&fx.s, "block")
    );
}

#[test]
fn c32_lowest_power_above_power_caps_at_power_and_warns() {
    let ramp = grey_png(16, 1, |x, _| (x * 17) as u8);
    let (mut fx, _, e) = engrave_one(|b| b.img(&ramp, 20.0, 20.0, 8.0, 0.5));
    dither(&mut fx.s, e, "grey");
    assert_eq!(
        fx.s.edit_operation_option(e, "lowest_power", 80.0),
        Ok(true)
    );
    let warns = texts(&fx.s, "warn");
    assert!(
        warns.contains(&"Image 1: Lowest power is above Power.".to_owned()),
        "{warns:?}"
    );
    let g = gcode(&mut fx.s);
    assert!(
        g.burns().iter().all(|m| m.s == 600),
        "every dot at the set's power"
    );
    assert!(g.burns().iter().all(|m| m.s > 0));
    let c = checks(&fx.s)
        .into_iter()
        .find(|c| c["text"] == "Image 1: Lowest power is above Power.")
        .unwrap();
    assert_eq!(c["target"]["kind"], "operations");
    // Not above: no warn, and the ramp holds.
    assert_eq!(
        fx.s.edit_operation_option(e, "lowest_power", 20.0),
        Ok(true)
    );
    assert!(
        !texts(&fx.s, "warn")
            .iter()
            .any(|t| t.contains("Lowest power"))
    );
    let g = gcode(&mut fx.s);
    let ss: Vec<u32> = g.burns().iter().map(|m| m.s).collect();
    assert!(ss.iter().all(|&s| s >= 200 && s <= 600), "{ss:?}");
    // White still burns nothing with Lowest power above 0.
    assert!((g.burn_mm() - 15.0 * 0.5).abs() < 0.01, "{}", g.burn_mm());
}

#[test]
fn c34_more_than_50_million_dots_blocks_naming_the_item() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 0.0, 0.0, 400.0, 300.0));
    let _ = fx.s.edit_operation_option(e, "interval", 0.01);
    let blocks = texts(&fx.s, "block");
    assert!(
        blocks.contains(
            &"Photo 1 would need 1,200,000,000 dots; the limit is 50,000,000.".to_owned()
        ),
        "{blocks:?}"
    );
    assert!(export_bytes(&mut fx.s).is_none());
    // A gradient shape too, named by its name.
    let (fx, _, e) = engrave_one(|b| {
        let r = b.rect(0.0, 0.0, 100.0, 100.0);
        b.shade(r);
        r
    });
    let mut s = fx.s;
    let _ = s.edit_operation_option(e, "interval", 0.01);
    let blocks = texts(&s, "block");
    assert!(
        blocks
            .iter()
            .any(|t| t.ends_with("would need 100,000,000 dots; the limit is 50,000,000.")),
        "{blocks:?}"
    );
}

#[test]
fn c34_exactly_50_million_dots_does_not_block() {
    // 100 x 50 mm at 0.01 mm = 50,000,000 dots exactly.
    let (fx, _, e) = engrave_one(|b| b.img(&half_black(), 0.0, 0.0, 100.0, 50.0));
    let mut s = fx.s;
    let _ = s.edit_operation_option(e, "interval", 0.01);
    assert!(
        !texts(&s, "block")
            .iter()
            .any(|t| t.contains("dots; the limit")),
        "{:?}",
        texts(&s, "block")
    );
}

#[test]
fn c35_a_coarse_image_warns_and_a_fine_one_does_not() {
    // 16 px over 8 mm = 0.5 mm pitch; Interval 0.2 → coarser than 2 x 0.2.
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    let _ = fx.s.edit_operation_option(e, "interval", 0.2);
    assert!(
        texts(&fx.s, "warn").contains(&"1 image is coarser than its Interval.".to_owned()),
        "{:?}",
        texts(&fx.s, "warn")
    );
    // Interval 0.25: exactly twice is not "more than twice".
    let _ = fx.s.edit_operation_option(e, "interval", 0.25);
    assert!(!texts(&fx.s, "warn").iter().any(|t| t.contains("coarser")));
    let _ = fx.s.edit_operation_option(e, "interval", 0.5);
    assert!(!texts(&fx.s, "warn").iter().any(|t| t.contains("coarser")));
}

#[test]
fn c35_overlapping_raster_items_of_one_operation_warn_once() {
    let (mut fx, ids) = laser(|b| {
        let a = b.img(&half_black(), 20.0, 20.0, 8.0, 8.0);
        let c = b.img(&half_black(), 25.0, 25.0, 8.0, 8.0);
        let r = b.rect(30.0, 30.0, 10.0, 10.0);
        b.shade(r);
        let far = b.img(&half_black(), 200.0, 200.0, 8.0, 8.0);
        vec![a, c, r, far, b.op("image", "Image 1", &[a, c, r, far])]
    });
    set(&mut fx.s, ids[4], 60.0, 3000.0, Some(0.5));
    let warns = texts(&fx.s, "warn");
    let lines: Vec<_> = warns.iter().filter(|t| t.contains("overlap")).collect();
    assert_eq!(
        lines,
        [&"3 items overlap and burn twice.".to_owned()],
        "{warns:?}"
    );
}

#[test]
fn c35_touching_raster_items_do_not_overlap() {
    let (mut fx, ids) = laser(|b| {
        let a = b.img(&half_black(), 20.0, 20.0, 8.0, 8.0);
        let c = b.img(&half_black(), 28.0, 20.0, 8.0, 8.0);
        vec![a, c, b.op("image", "Image 1", &[a, c])]
    });
    set(&mut fx.s, ids[2], 60.0, 3000.0, Some(0.5));
    assert!(
        !texts(&fx.s, "warn").iter().any(|t| t.contains("overlap")),
        "{:?}",
        texts(&fx.s, "warn")
    );
}

#[test]
fn c36_an_unreadable_image_is_skipped_and_warns() {
    let broken = picture_fixture("png_truncated.png");
    let (mut fx, ids) = laser(|b| {
        let a = b.img(&broken, 20.0, 20.0, 8.0, 8.0);
        let ok = b.img(&half_black(), 50.0, 20.0, 8.0, 8.0);
        vec![a, ok, b.op("image", "Image 1", &[a, ok])]
    });
    set(&mut fx.s, ids[2], 60.0, 3000.0, Some(0.5));
    let warns = texts(&fx.s, "warn");
    assert!(
        warns.contains(&"1 image cannot be read and is skipped.".to_owned()),
        "{warns:?}"
    );
    let g = gcode(&mut fx.s);
    assert!(
        (g.burn_mm() - 64.0).abs() < 0.01,
        "only the readable one: {}",
        g.burn_mm()
    );
}

#[test]
fn c36_new_lines_come_in_the_documented_order() {
    // 33 block, 34 block, 35 coarse + overlap, 36 reduced, then 32 (b).
    let big = raw_png(9000, 2, |_, _| 0);
    let (mut fx, ids) = laser(|b| {
        let a = b.img(&half_black(), 20.0, 20.0, 8.0, 8.0);
        let c = b.img(&half_black(), 24.0, 24.0, 8.0, 8.0);
        let r = b.img(&big, 20.0, 100.0, 90.0, 0.5);
        vec![a, c, r, b.op("image", "Image 1", &[a, c, r])]
    });
    let e = ids[3];
    set(&mut fx.s, e, 60.0, 3000.0, Some(0.2));
    dither(&mut fx.s, e, "grey");
    let _ = fx.s.edit_operation_option(e, "lowest_power", 80.0);
    let warns = texts(&fx.s, "warn");
    let pos = |needle: &str| {
        warns
            .iter()
            .position(|t| t.contains(needle))
            .unwrap_or_else(|| panic!("{needle} in {warns:?}"))
    };
    let order = [
        pos("coarser"),
        pos("overlap"),
        pos("reduced"),
        pos("Lowest power is above Power"),
    ];
    let mut sorted = order;
    sorted.sort_unstable();
    assert_eq!(order, sorted, "{warns:?}");
}

// ---- criterion 38: storage --------------------------------------------------------------------

#[test]
fn c38_engraving_does_not_change_the_saved_project() {
    let (mut fx, _, e) = engrave_one(|b| b.img(&half_black(), 20.0, 20.0, 8.0, 8.0));
    let a = fx.s.pack("0.1.0").unwrap();
    let _ = gcode(&mut fx.s);
    let page = fx.page.clone();
    let _ = fx.s.toggle_toolpath_view(&page);
    let _ = fx.s.frame_draw_list();
    let _ = fx.s.toggle_toolpath_view(&page);
    let _ = fx.s.operations_view_json();
    let _ = e;
    let b = fx.s.pack("0.1.0").unwrap();
    assert_eq!(a, b, "the two files are byte-identical");
}

#[test]
fn c38_one_image_in_two_operations_is_stored_once() {
    let (fx, _) = laser(|b| {
        let a = b.img(&half_black(), 20.0, 20.0, 8.0, 8.0);
        let c = b.img(&half_black(), 60.0, 20.0, 8.0, 8.0);
        let e1 = b.op("image", "Image 1", &[a]);
        let e2 = b.op("image", "Image 2", &[c]);
        vec![e1, e2]
    });
    let bytes = fx.s.pack("0.1.0").unwrap();
    let z = zip_names(&bytes);
    assert_eq!(
        z.iter().filter(|n| n.starts_with("assets/")).count(),
        1,
        "{z:?}"
    );
}

fn zip_names(bytes: &[u8]) -> Vec<String> {
    // Central directory entries: signature 0x02014b50.
    let mut out = Vec::new();
    let mut i = 0;
    while i + 46 <= bytes.len() {
        if bytes[i..i + 4] == [0x50, 0x4b, 0x01, 0x02] {
            let n = u16::from_le_bytes([bytes[i + 28], bytes[i + 29]]) as usize;
            out.push(String::from_utf8_lossy(&bytes[i + 46..i + 46 + n]).to_string());
            i += 46 + n;
        } else {
            i += 1;
        }
    }
    out
}

// ---- criterion 41: format 50 goldens --------------------------------------------------------

fn golden_project(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn open_golden(name: &str) -> Session {
    let mut s = Session::open(3, &golden_project(name)).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.load_machine_profiles_json("[]");
    s.load_materials(&[], None);
    let laser = unpack(9, &golden_project(name))
        .unwrap()
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap()
        .id;
    assert!(s.activate_page(&laser));
    s
}

#[test]
fn c41_the_format_50_golden_opens_engraves_and_round_trips() {
    let bytes = golden_project("raster_engraving_v50.curvyo");
    let d = unpack(9, &bytes).unwrap();
    // Superseded by 0135 slice 1 (2026-11-19): format 51 (ADR 0072 §8).
    assert_eq!(curvyo_document_core::CURRENT_FORMAT_VERSION, 53);
    let page = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap();
    let ops = d.operations(page.kind.manufacturing().unwrap()).unwrap();
    // Superseded by the Image kind rework (2026-11-19): the settings are on the Image operation.
    let engrave = ops
        .iter()
        .find(|o| o.kind.as_deref() == Some("image"))
        .expect("an Image operation");
    let o = engrave
        .options
        .clone()
        .unwrap()
        .as_object()
        .cloned()
        .unwrap();
    for k in [
        "dither",
        "threshold_pct",
        "brightness_pct",
        "contrast_pct",
        "gamma",
        "invert",
        "lowest_power_pct",
    ] {
        assert!(o.contains_key(k), "every setting: {k} in {o:?}");
    }
    let mut s = open_golden("raster_engraving_v50.curvyo");
    assert!(
        !texts(&s, "warn").iter().any(|t| t.contains("damaged")),
        "{:?}",
        texts(&s, "warn")
    );
    let _ = export_bytes(&mut s);
    // Re-saving without a change keeps the document the same.
    let again = unpack(9, &s.pack("0.1.0").unwrap()).unwrap();
    let page2 = again
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap();
    let ops2 = again
        .operations(page2.kind.manufacturing().unwrap())
        .unwrap();
    assert_eq!(
        ops.iter().map(|o| o.options.clone()).collect::<Vec<_>>(),
        ops2.iter().map(|o| o.options.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn c41_the_damaged_golden_has_three_damaged_operations_that_do_not_run() {
    // Superseded by the Image kind rework (2026-11-19): the damaged golden adds `dither` on an
    // Engrave and `crosshatch` on an Image, five damaged operations.
    let s = open_golden("raster_engraving_damaged_v50.curvyo");
    let warns = texts(&s, "warn");
    assert!(
        warns.contains(&"5 operations have damaged settings and do not run.".to_owned()),
        "{warns:?} / {:?}",
        texts(&s, "block")
    );
}

// ---- criteria 44, 45: the test grid ----------------------------------------------------------

fn grid_session() -> (Session, String) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mut s = Session::open(5, &pack(&d, "0123-grid").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    let mut snap = s.snapping();
    snap.enabled = false;
    s.set_snapping(snap);
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(&laser.id));
    load_fonts(&mut s);
    s.load_materials(&[], None);
    s.wipe_history();
    (s, laser.id)
}

fn load_fonts(s: &mut Session) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts");
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("index.json")).unwrap()).unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect();
    s.set_font_list(curvyo_ui_core::FontBlock::Included, faces, Vec::new());
    s.set_tool(Tool::TestGrid);
    let _ = s.test_grid_press(None);
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
    s.set_tool(Tool::Select);
}

fn place_grid(s: &mut Session) -> NodeId {
    let o = s.active_page().origin;
    s.set_tool(Tool::TestGrid);
    let g = s
        .test_grid_press(Some(Point::new(o.x + 10.0, o.y + 10.0)))
        .unwrap();
    s.set_tool(Tool::Select);
    g
}

fn grid_view(s: &Session) -> Value {
    serde_json::from_str(&s.test_grid_view_json()).unwrap()
}

fn derived_cells(s: &Session) -> Vec<curvyo_document_core::OperationSnapshot> {
    let d = unpack(99, &s.pack("0.1.0").unwrap()).unwrap();
    let p = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap();
    d.operations(p.kind.manufacturing().unwrap())
        .unwrap()
        .into_iter()
        .filter(|o| o.grid_part.as_ref().is_some_and(|g| g["part"] == "cell"))
        .collect()
}

// Superseded by the Image kind rework (2026-11-19): the test grid gains a fourth kind Image with
// a Dither strip and no Fill; the Engrave grid loses Fill and Dither (criterion 44).
#[test]
fn c44_dither_shows_only_for_an_image_grid_and_there_is_no_fill() {
    let (mut s, _) = grid_session();
    place_grid(&mut s);
    let v = grid_view(&s);
    assert!(v["dither"].is_null(), "no Dither on a Cut grid: {v}");
    assert_eq!(s.set_test_grid_kind("engrave"), Ok(true));
    let v = grid_view(&s);
    assert!(v["dither"].is_null(), "no Dither on an Engrave grid: {v}");
    assert!(v.get("fill").is_none(), "no Fill: {v}");
    assert_eq!(s.set_test_grid_kind("image"), Ok(true));
    let v = grid_view(&s);
    let offered: Vec<&str> = v["dither"]["modes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["value"].as_str().unwrap())
        .collect();
    assert_eq!(
        offered,
        [
            "threshold",
            "ordered",
            "floyd_steinberg",
            "jarvis",
            "atkinson",
            "stucki"
        ],
        "the modes of criterion 10 without Grey"
    );
    assert_eq!(v["dither"]["value"], "floyd_steinberg", "{v}");
    let n = s.undo_len();
    assert_eq!(s.set_test_grid_dither("ordered"), Ok(true));
    assert_eq!(s.undo_len(), n + 1);
    assert_eq!(
        s.history_target_name(Direction::Undo).as_deref(),
        Some("Edit test grid")
    );
    assert!(
        s.set_test_grid_dither("grey").is_err() || s.set_test_grid_dither("grey") == Ok(false),
        "Grey is not offered"
    );
    assert_eq!(grid_view(&s)["dither"]["value"], "ordered");
    let _ = s.undo();
    assert_eq!(grid_view(&s)["dither"]["value"], "floyd_steinberg");
}

// Superseded by the Image kind rework (2026-11-19): the ramps are an Image grid's cells, Image
// operations carrying the Dither (criterion 44).
#[test]
fn c44_an_image_grid_makes_gradient_cells_with_the_chosen_dither_and_engraves_ramps() {
    let (mut s, _) = grid_session();
    place_grid(&mut s);
    assert_eq!(s.set_test_grid_kind("image"), Ok(true));
    let _ = s.set_test_grid_axis("columns", "interval");
    let _ = s.set_test_grid_axis("rows", "power");
    assert_eq!(s.set_test_grid_dither("ordered"), Ok(true));
    let cells = derived_cells(&s);
    assert_ne!(cells.len(), 0);
    for c in &cells {
        let o = c
            .options
            .clone()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        assert_eq!(o.get("dither"), Some(&json!("ordered")), "{o:?}");
        assert_eq!(c.kind.as_deref(), Some("image"));
    }
    // The cells are gradient-filled rectangles black to white at Angle 0.
    let d = unpack(99, &s.pack("0.1.0").unwrap()).unwrap();
    let gradients: Vec<_> = d
        .object_ids()
        .into_iter()
        .filter_map(|id| d.object(id))
        .filter_map(|o| o.style().fill.painted_gradient().cloned())
        .collect();
    assert!(
        gradients.len() >= cells.len(),
        "{} gradients, {} cells",
        gradients.len(),
        cells.len()
    );
    for g in &gradients {
        let stops = g.stops.as_slice();
        assert_eq!(stops[0].colour, Color { r: 0, g: 0, b: 0 });
        assert_eq!(
            stops[stops.len() - 1].colour,
            Color {
                r: 255,
                g: 255,
                b: 255
            }
        );
        assert_eq!(g.geometry, GradientGeometry::DEFAULT_LINEAR);
    }
    // Each cell engraves its ramp: the file has raster rows (horizontal burns of many lengths).
    let _ = s.save_machine_profile([13; 16], 1_000);
    if let Some(bytes) = export_bytes(&mut s) {
        let g = read(&bytes).unwrap();
        assert!(g.rows().len() > 50, "raster rows");
    } else {
        let v: Value = serde_json::from_str(&s.output_group_json()).unwrap();
        panic!("blocked: {}", v["checks"]);
    }
}

// Superseded by the Image kind rework (2026-11-19): the record keeps an Image test's Dither (no
// Fill), and Place on page rebuilds the Image grid (criterion 45).
#[test]
fn c45_an_engrave_grid_is_unchanged_and_the_record_keeps_the_image_grids_dither() {
    let (mut s, _) = grid_session();
    let b = s.add_material("Slate", 5.0, &[4; 16]).unwrap();
    place_grid(&mut s);
    assert_eq!(s.set_test_grid_kind("engrave"), Ok(true));
    let solid = derived_cells(&s);
    for c in &solid {
        let o = c
            .options
            .clone()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        assert!(
            !o.contains_key("dither"),
            "Engrave cells carry no dither: {o:?}"
        );
    }
    assert_eq!(s.set_test_grid_kind("image"), Ok(true));
    assert_eq!(s.set_test_grid_dither("atkinson"), Ok(true));
    assert_eq!(s.pick_test_grid_material(Some(&b)), Ok(true));
    let id = grid_view(&s)["id"].as_str().unwrap().to_owned();
    let writes: Value = serde_json::from_str(&s.take_material_writes_json()).unwrap();
    let text = writes
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|w| w["op"] == "write" && w["id"] == json!(b))
        .and_then(|w| w["text"].as_str().map(str::to_owned))
        .unwrap_or_else(|| panic!("{writes}"));
    let rec: Value = serde_json::from_str(&text).unwrap();
    // Superseded by 0135 slice 1 (2026-11-19): material record format 4 (ADR 0072 §6).
    assert_eq!(rec["format_version"], 4);
    let t = rec["tests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == json!(id))
        .unwrap();
    assert_eq!(t["recipe"]["kind"], "image");
    assert!(t["recipe"].get("fill").is_none());
    assert_eq!(t["recipe"]["dither"], "atkinson");
    // Place on page rebuilds the same grid.
    s.delete_selected();
    let _ = s.place_material_test(&b, &id).unwrap();
    let v = grid_view(&s);
    assert_eq!(v["kind"], "image");
    assert_eq!(v["dither"]["value"], "atkinson");
}

// ---- edge cases -----------------------------------------------------------------------------

#[test]
fn edge_a_1x1_image_engraves_its_frame_area() {
    let one = grey_png(1, 1, |_, _| 0);
    let (mut fx, _, _) = engrave_one(|b| b.img(&one, 20.0, 20.0, 4.0, 2.0));
    let g = gcode(&mut fx.s);
    assert!((g.burn_mm() * 0.5 - 8.0).abs() < 0.01, "{}", g.burn_mm());
}

#[test]
fn edge_a_16_bit_png_and_a_cmyk_jpeg_engrave() {
    for name in [
        "png_gray_16.png",
        "jpeg_cmyk_adobe.jpg",
        "png_indexed_trns.png",
        "jpeg_gray.jpg",
    ] {
        let bytes = picture_fixture(name);
        let (mut fx, _, e) = engrave_one(|b| b.img(&bytes, 20.0, 20.0, 20.0, 20.0));
        // Grey, so a light colour (the CMYK fixture is pure green) still burns.
        dither(&mut fx.s, e, "grey");
        let warns = texts(&fx.s, "warn");
        assert!(
            !warns.iter().any(|t| t.contains("cannot be read")),
            "{name}: {warns:?}"
        );
        let ok = export_bytes(&mut fx.s).map(|b| read(&b).unwrap());
        assert!(ok.is_some(), "{name}: export {:?}", checks(&fx.s));
    }
}

#[test]
fn edge_1000_images_engrave_each() {
    let black = grey_png(2, 2, |_, _| 0);
    let (mut fx, ids) = laser(|b| {
        let mut imgs = Vec::new();
        for k in 0..1000 {
            let x = f64::from(k % 40) * 9.5 + 5.0;
            let y = f64::from(k / 40) * 9.5 + 5.0;
            imgs.push(b.img(&black, x, y, 2.0, 2.0));
        }
        let e = b.op("image", "Image 1", &imgs);
        vec![e]
    });
    set(&mut fx.s, ids[0], 60.0, 3000.0, Some(0.5));
    dither(&mut fx.s, ids[0], "threshold");
    let t = std::time::Instant::now();
    let g = gcode(&mut fx.s);
    println!("1000 images: export {:?}", t.elapsed());
    assert!(
        (g.burn_mm() - 1000.0 * 4.0 * 2.0).abs() < 0.1,
        "{}",
        g.burn_mm()
    );
}

#[test]
fn edge_damaged_image_bytes_are_never_rewritten() {
    let broken = b"not a picture at all".to_vec();
    let (fx, _, _) = engrave_one(|b| b.img(&broken, 20.0, 20.0, 8.0, 8.0));
    let warns = texts(&fx.s, "warn");
    assert!(
        warns.contains(&"1 image cannot be read and is skipped.".to_owned()),
        "{warns:?}"
    );
    let _ = text_of;
}
