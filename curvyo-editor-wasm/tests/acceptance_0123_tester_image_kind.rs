//! Tester cases for the Image kind rework of `specs/0123-raster-engraving` (2026-11-19), written
//! from the specification through the public `Session`, read back by the tester's own GRBL
//! reader (`curvyo-laser-core/tests/tester_raster_0123`): every image reached through a clone or
//! a placed page burns its dots in an Image operation (criterion 46), the Image kind button
//! (47), an image in the other five kinds is, move for move, the rectangle of its frame
//! (criterion 1, Frage 271 A), a plain vector leaf in an Image operation burns nothing and warns
//! (48), the Image keys and Crosshatch are damage on the wrong kind (2, 13), and the material's
//! Image set (Frage 270 B; criteria 2, 13, 45).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::needless_pass_by_value,
    missing_docs
)]

#[path = "../../curvyo-laser-core/tests/tester_raster_0123/mod.rs"]
mod reference;

use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, Color, Document, DocumentSize, FIRST_PAGE_ID, FillPaint, GradientGeometry,
    GradientStop, GradientStops, NewImage, NewOperation, NewOutputPage, NodeId, Point, RectBounds,
    StopOffset, StyleEdit, Timestamp, Vec2, pack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_image_core::{WorkingImage, encode_png};
use reference::{Gcode, Mv, read};
use serde_json::{Value, json};

const LASER: &str = "curvyo.laser";
const BED_H: f64 = 300.0;

// ---- fixture ----------------------------------------------------------------------------------

fn grey_png(w: u32, h: u32, mut g: impl FnMut(u32, u32) -> u8) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let v = g(x, y);
            pixels.extend_from_slice(&[v, v, v, 255]);
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

/// Left half black, right half white, 16 x 16: at 8 x 8 mm and Interval 0.5, 16 rows of 8
/// burned dots, 128 dots, 64 mm burned.
fn half_black() -> Vec<u8> {
    grey_png(16, 16, |x, _| if x < 8 { 0 } else { 255 })
}

const HALF_DOTS: usize = 128;

struct B<'a> {
    d: &'a Document,
    o: Point,
    m: NodeId,
}

impl B<'_> {
    fn img_at(&self, o: Point, x: f64, y: f64, w: f64, h: f64) -> NodeId {
        self.d
            .add_images(&[NewImage {
                bytes: Arc::from(half_black().as_slice()),
                placement: Affine::new(w, 0.0, 0.0, h, o.x + x, o.y + y),
                name: "Photo 1".to_owned(),
            }])
            .unwrap()[0]
    }
    fn img(&self, x: f64, y: f64, w: f64, h: f64) -> NodeId {
        self.img_at(self.o, x, y, w, h)
    }
    fn rect(&self, x: f64, y: f64, w: f64, h: f64) -> NodeId {
        self.d.create_rect(RectBounds::from_corners(
            Point::new(self.o.x + x, self.o.y + y),
            Point::new(self.o.x + x + w, self.o.y + y + h),
        ))
    }
    fn clone_of(&self, of: NodeId, dx: f64, dy: f64) -> NodeId {
        let c = self.d.linked_clone(&[of], &[]).unwrap()[0];
        self.d.translate_objects(&[c], Vec2::new(dx, dy)).unwrap();
        c
    }
    fn op(&self, kind: &str, name: &str, members: &[NodeId]) -> NodeId {
        let index = self.d.operations(self.m).unwrap().len();
        let keys: Vec<Vec<NodeId>> = members.iter().map(|&n| vec![n]).collect();
        self.d
            .add_operation(
                self.m,
                &NewOperation {
                    kind,
                    name,
                    settings: None,
                },
                index,
                &keys,
            )
            .unwrap()
    }
    /// The drawing page's origin.
    fn drawing_origin(&self) -> Point {
        self.d
            .pages()
            .into_iter()
            .find(|p| p.id == FIRST_PAGE_ID)
            .unwrap()
            .origin
    }
    /// Places the drawing page with its origin at laser page `(x, y)` (page-local matrix).
    fn place(&self, laser: &str, x: f64, y: f64) -> NodeId {
        self.d
            .place_page(laser, FIRST_PAGE_ID, Affine::translation(Vec2::new(x, y)))
            .unwrap()
    }
}

fn shade(d: &Document, id: NodeId) {
    let stop = |v: u8, offset: f64| GradientStop {
        colour: Color { r: v, g: v, b: v },
        alpha: 255,
        offset: StopOffset::new(offset).unwrap(),
    };
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    d.edit_style(
        &[id],
        &StyleEdit::GradientGeometry(GradientGeometry::DEFAULT_LINEAR),
    )
    .unwrap();
    d.edit_style(
        &[id],
        &StyleEdit::GradientStops(GradientStops::new(vec![stop(0, 0.0), stop(255, 1.0)]).unwrap()),
    )
    .unwrap();
}

struct Fx {
    s: Session,
    page: String,
    profile: Option<String>,
}

/// A 400 x 300 mm laser page on a saved profile; `build` draws on it (given the laser page id).
fn laser(build: impl FnOnce(&B, &str) -> Vec<NodeId>) -> (Fx, Vec<NodeId>) {
    laser_on(true, build)
}

/// [`laser`]; `saved` false keeps the page on the Generic laser (`generic-laser`).
fn laser_on(saved: bool, build: impl FnOnce(&B, &str) -> Vec<NodeId>) -> (Fx, Vec<NodeId>) {
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
    let ids = build(
        &B {
            d: &d,
            o: page.origin,
            m,
        },
        &page.id,
    );
    let mut s = Session::open(7, &pack(&d, "0123-image-kind").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(&page.id));
    s.load_materials(&[], None);
    let profile = if saved {
        let p = s.save_machine_profile([13; 16], 1_000);
        let _ = s.set_machine_name("Diode 10 W");
        p
    } else {
        Some("generic-laser".to_owned())
    };
    s.wipe_history();
    (
        Fx {
            s,
            page: page.id,
            profile,
        },
        ids,
    )
}

/// Power, speed (and the outline set of Engrave + Outline), Interval 0.5, Threshold, no
/// overscan, so the rows are exactly the dots.
fn set_up(s: &mut Session, op: NodeId, kind: &str) {
    assert!(s.edit_operation_setting(op, "main", "power", 60.0).is_ok());
    assert!(
        s.edit_operation_setting(op, "main", "speed", 3000.0)
            .is_ok()
    );
    if kind == "engrave_outline" {
        assert!(
            s.edit_operation_setting(op, "outline", "power", 20.0)
                .is_ok()
        );
        assert!(
            s.edit_operation_setting(op, "outline", "speed", 500.0)
                .is_ok()
        );
    }
    let _ = s.edit_operation_option(op, "interval", 0.5);
    let _ = s.edit_operation_option(op, "overscan", 0.0);
    if kind == "image" {
        let _ = s.set_operation_dither(op, "threshold");
    }
}

fn out(s: &Session) -> Value {
    serde_json::from_str(&s.output_group_json()).unwrap()
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

fn ops_view(s: &Session) -> Value {
    serde_json::from_str(&s.operations_view_json()).unwrap()
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

fn export(s: &mut Session) -> Option<Vec<u8>> {
    s.export_job("Untitled.curvyo").map(|e| e.bytes)
}

fn gcode(s: &mut Session) -> Gcode {
    let bytes = export(s).unwrap_or_else(|| panic!("export blocked: {:?}", checks(s)));
    read(&bytes).unwrap_or_else(|e| panic!("{e}\n{}", String::from_utf8_lossy(&bytes)))
}

/// The burned dots of horizontal rows at Interval `i`: every burn segment must be a whole
/// number of dots; returns that number summed.
fn dots_of(g: &Gcode, i: f64) -> usize {
    let mut n = 0;
    for (_, segs) in g.rows() {
        for (a, b, _) in segs {
            let k = ((b - a) / i).round();
            assert!(
                ((b - a) - k * i).abs() < 0.01,
                "a run of whole dots: {a}..{b}"
            );
            n += k as usize;
        }
    }
    n
}

/// Burned length equals burned dots times the Interval (criterion 40), and the dot count.
fn assert_dots(g: &Gcode, i: f64, dots: usize, what: &str) {
    let n = dots_of(g, i);
    assert_eq!(n, dots, "{what}: burned dots");
    assert!(
        (g.burn_mm() - n as f64 * i).abs() < 0.01,
        "{what}: burned {} mm = {n} dots x {i}",
        g.burn_mm()
    );
}

/// The burned rows in machine coordinates, grouped by Y (top first), each row's runs.
fn rows_sorted(g: &Gcode) -> Vec<(f64, Vec<(f64, f64)>)> {
    let mut rows: Vec<(f64, Vec<(f64, f64)>)> = g
        .rows()
        .into_iter()
        .map(|(y, segs)| {
            let mut s: Vec<(f64, f64)> = segs.into_iter().map(|(a, b, _)| (a, b)).collect();
            s.sort_by(|p, q| p.0.total_cmp(&q.0));
            (y, s)
        })
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    rows
}

/// The rows of `g` whose runs lie in the X range `x0..x1`.
fn rows_in(g: &Gcode, x0: f64, x1: f64) -> Vec<(f64, Vec<(f64, f64)>)> {
    rows_sorted(g)
        .into_iter()
        .map(|(y, segs)| {
            (
                y,
                segs.into_iter()
                    .filter(|(a, b)| *a >= x0 - 1e-6 && *b <= x1 + 1e-6)
                    .collect::<Vec<_>>(),
            )
        })
        .filter(|(_, s)| !s.is_empty())
        .collect()
}

fn same_moves(a: &[Mv], b: &[Mv]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(p, q)| {
            p.rapid == q.rapid
                && p.burn == q.burn
                && p.s == q.s
                && (p.from.0 - q.from.0).abs() < 1e-6
                && (p.from.1 - q.from.1).abs() < 1e-6
                && (p.to.0 - q.to.0).abs() < 1e-6
                && (p.to.1 - q.to.1).abs() < 1e-6
        })
}

// ---- criterion 46: every image reached through a clone burns in Image ---------------------

/// The customer's acceptance case: an image and its linked clone beside it, both selected, the
/// Image kind button pressed once.
#[test]
fn c46_c47_an_image_and_its_linked_clone_pressed_into_image_burn_two_equal_blocks() {
    let (mut fx, ids) = laser(|b, _| {
        let i = b.img(20.0, 20.0, 8.0, 8.0);
        let c = b.clone_of(i, 30.0, 0.0);
        vec![i, c]
    });
    assert!(fx.s.select_output_items(&ids));
    let op = fx.s.press_operation_kind("image", false).expect("Image 1");
    let rows = ops_view(&fx.s)["rows"].as_array().unwrap().clone();
    assert_eq!(rows.len(), 1, "one row: {rows:?}");
    assert_eq!(rows[0]["name"], "Image 1");
    assert_eq!(rows[0]["kind"], "image");
    assert_eq!(rows[0]["items"], 2, "two items");
    set_up(&mut fx.s, op, "image");
    let g = gcode(&mut fx.s);
    assert_dots(&g, 0.5, 2 * HALF_DOTS, "original and clone");
    let orig = rows_in(&g, 20.0, 28.0);
    let clone = rows_in(&g, 50.0, 58.0);
    assert_eq!(orig.len(), 16, "the original's 16 rows");
    assert_eq!(clone.len(), orig.len(), "the clone's rows");
    for ((ya, ra), (yb, rb)) in orig.iter().zip(&clone) {
        assert!((ya - yb).abs() < 1e-6, "same row heights");
        assert_eq!(ra.len(), rb.len());
        for ((a0, a1), (b0, b1)) in ra.iter().zip(rb) {
            assert!(
                (b0 - a0 - 30.0).abs() < 1e-6 && (b1 - a1 - 30.0).abs() < 1e-6,
                "the clone's run is the original's moved by 30 mm"
            );
        }
    }
    // Two raster blocks: the original's 16 rows burn before (or after) all of the clone's.
    let order: Vec<bool> = g
        .burns()
        .iter()
        .map(|m| m.from.0.min(m.to.0) >= 40.0)
        .collect();
    let switches = order.windows(2).filter(|w| w[0] != w[1]).count();
    assert_eq!(switches, 1, "two blocks, not interleaved rows");
    assert!(
        !texts(&fx.s, "warn")
            .iter()
            .any(|t| t.contains("not a picture") || t.contains("not pictures")),
        "{:?}",
        texts(&fx.s, "warn")
    );
}

#[test]
fn c46_a_clone_alone_and_a_clone_of_a_clone_burn_at_their_place() {
    for depth in [1, 2] {
        let (mut fx, ids) = laser(|b, _| {
            let i = b.img(20.0, 20.0, 8.0, 8.0);
            let c = b.clone_of(i, 30.0, 0.0);
            let leaf = if depth == 2 {
                b.clone_of(c, 0.0, 40.0)
            } else {
                c
            };
            vec![leaf, b.op("image", "Image 1", &[leaf])]
        });
        set_up(&mut fx.s, ids[1], "image");
        let g = gcode(&mut fx.s);
        assert_dots(&g, 0.5, HALF_DOTS, "the clone");
        let (x0, y0, x1, y1) = g.burn_extent().unwrap();
        let dy = if depth == 2 { 40.0 } else { 0.0 };
        assert!(
            (x0 - 50.0).abs() < 1e-6 && (x1 - 54.0).abs() < 1e-6,
            "depth {depth}: the left half at x 50..54: {x0} {x1}"
        );
        // Machine Y is bottom-left: page y 20..28 is machine 272..280.
        assert!(
            y0 >= BED_H - 28.0 - dy - 1e-6 && y1 <= BED_H - 20.0 - dy + 1e-6,
            "depth {depth}: {y0} {y1}"
        );
    }
}

#[test]
fn c46_a_clone_of_a_group_of_two_images_burns_both() {
    let (mut fx, ids) = laser(|b, _| {
        let i = b.img(20.0, 20.0, 8.0, 8.0);
        let j = b.img(40.0, 20.0, 8.0, 8.0);
        let g = b.d.group_objects(&[i, j]).unwrap();
        let c = b.clone_of(g, 0.0, 50.0);
        vec![c, b.op("image", "Image 1", &[c])]
    });
    set_up(&mut fx.s, ids[1], "image");
    let g = gcode(&mut fx.s);
    assert_dots(&g, 0.5, 2 * HALF_DOTS, "both members of the group's clone");
    let (_, y0, _, y1) = g.burn_extent().unwrap();
    assert!(
        y0 >= BED_H - 78.0 - 1e-6 && y1 <= BED_H - 70.0 + 1e-6,
        "{y0} {y1}"
    );
    assert_eq!(row_of(&fx.s, ids[1])["items"], 1, "the clone is one item");
}

#[test]
fn c46_an_image_member_of_a_placed_page_burns_through_the_placement() {
    let (mut fx, ids) = laser(|b, laser| {
        let o = b.drawing_origin();
        let _ = b.img_at(o, 5.0, 5.0, 8.0, 8.0);
        let p = b.place(laser, 100.0, 100.0);
        vec![p, b.op("image", "Image 1", &[p])]
    });
    set_up(&mut fx.s, ids[1], "image");
    let g = gcode(&mut fx.s);
    assert_dots(&g, 0.5, HALF_DOTS, "the placed page's image");
    // The image itself never causes the criterion 48 line; the placed page's own frame (a
    // vector leaf of `0106`) is counted there, which is reported to the lead, not asserted.
    let warns = texts(&fx.s, "warn");
    assert!(
        !warns
            .iter()
            .any(|t| t.contains("items in Image 1 are not pictures")),
        "at most the page frame: {warns:?}"
    );
    let (x0, y0, x1, y1) = g.burn_extent().unwrap();
    assert!(
        (x0 - 105.0).abs() < 1e-6 && (x1 - 109.0).abs() < 1e-6,
        "the placement's offset: {x0} {x1}"
    );
    assert!(
        y0 >= BED_H - 113.0 - 1e-6 && y1 <= BED_H - 105.0 + 1e-6,
        "{y0} {y1}"
    );
}

/// Every clone is an item of the row and counts in the checks: a clone partly off the bed blocks
/// (criterion 46 b).
#[test]
fn c46_clones_count_in_the_checks_and_the_row() {
    // Control: a plain image wholly off the bed blocks.
    let (mut fx, ids) = laser(|b, _| {
        let i = b.img(396.0, 20.0, 8.0, 8.0);
        vec![b.op("image", "Image 1", &[i])]
    });
    set_up(&mut fx.s, ids[0], "image");
    assert!(
        texts(&fx.s, "block")
            .iter()
            .any(|t| t.starts_with("1 item lies outside")),
        "control: {:?}",
        texts(&fx.s, "block")
    );
    let (mut fx, ids) = laser(|b, _| {
        let i = b.img(20.0, 20.0, 8.0, 8.0);
        let c = b.clone_of(i, 30.0, 0.0);
        // Partly off the bed: the clone counts in the bed check.
        let far = b.clone_of(i, 376.0, 0.0);
        vec![i, c, far, b.op("image", "Image 1", &[i, c, far])]
    });
    set_up(&mut fx.s, ids[3], "image");
    let blocks = texts(&fx.s, "block");
    assert!(
        blocks.iter().any(|t| t.starts_with("1 item lies outside")),
        "the far clone blocks: {blocks:?}"
    );
    assert_eq!(row_of(&fx.s, ids[3])["items"], 3);
}

// ---- clone of a text with a Gradient fill -------------------------------------------------

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
}

fn add_wanted_fonts(s: &mut Session) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts");
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
}

/// A gradient-filled text "Hi" on the laser page; its clone pressed alone into Image burns the
/// same number of dots as the text pressed alone (criteria 3, 46).
#[test]
fn c46_a_clone_of_a_gradient_text_burns_dots_in_image() {
    let burn = |clone: bool| -> (usize, (f64, f64, f64, f64)) {
        let (mut fx, _) = laser(|_, _| vec![]);
        load_fonts(&mut fx.s);
        add_wanted_fonts(&mut fx.s);
        let o = fx.s.active_page().origin;
        let at = Point::new(o.x + 20.0, o.y + 40.0);
        fx.s.set_tool(Tool::Text);
        fx.s.pointer_down(at, false);
        fx.s.pointer_up(at, false, false);
        add_wanted_fonts(&mut fx.s);
        for c in "HH".chars() {
            fx.s.text_input(curvyo_ui_core::TextInput::Insert(c.to_string()));
        }
        let typed = fx.s.selected_object_count();
        fx.s.finish_text_edit();
        add_wanted_fonts(&mut fx.s);
        let finished = fx.s.selected_object_count();
        fx.s.set_tool(Tool::Select);
        assert_eq!(
            fx.s.selected_object_count(),
            1,
            "the text is selected (typing {typed}, finished {finished})"
        );
        assert!(fx.s.set_fill_kind("gradient"));
        if clone {
            let _ = fx.s.linked_clone();
            assert_eq!(fx.s.selected_object_count(), 1, "the clone is selected");
        }
        let op = fx.s.press_operation_kind("image", false).expect("Image 1");
        set_up(&mut fx.s, op, "image");
        assert!(
            !texts(&fx.s, "warn")
                .iter()
                .any(|t| t.contains("not a picture")),
            "a gradient text is a raster item: {:?}",
            texts(&fx.s, "warn")
        );
        let g = gcode(&mut fx.s);
        (dots_of(&g, 0.5), g.burn_extent().expect("dots"))
    };
    let (text, te) = burn(false);
    let (clone, ce) = burn(true);
    assert!(text > 0, "the text burns dots");
    // A clone made by Clone may sit offset; the same dots wherever it stands, within the dot
    // grid's phase (one row and one dot either way at most).
    let rows = |e: (f64, f64, f64, f64)| ((e.3 - e.1) / 0.5).round();
    assert!(
        (rows(te) - rows(ce)).abs() <= 1.0,
        "the same height in rows: {te:?} {ce:?}"
    );
    let diff = text.abs_diff(clone) as f64;
    assert!(
        diff <= 0.1 * text as f64,
        "the clone burns the text's dots: {text} vs {clone}"
    );
}

// ---- criterion 1: an image in the other kinds is its frame rectangle, move for move -------

/// The file of one leaf (built by `leaf`) in an operation of `kind`.
fn file_of(kind: &str, leaf: impl FnOnce(&B, &str) -> NodeId) -> (Gcode, Vec<String>) {
    let (mut fx, ids) = laser(|b, laser| {
        let l = leaf(b, laser);
        vec![l, b.op(kind, "Op 1", &[l])]
    });
    set_up(&mut fx.s, ids[1], kind);
    let warns = texts(&fx.s, "warn");
    (gcode(&mut fx.s), warns)
}

const VECTOR_KINDS: [&str; 5] = ["cut", "score", "cut_out", "engrave", "engrave_outline"];

fn assert_frame_is_a_rectangle(
    what: &str,
    leaf: impl Fn(&B, &str) -> NodeId,
    (x, y, w, h): (f64, f64, f64, f64),
) {
    assert_frames_are_rectangles(what, leaf, &[(x, y, w, h)]);
}

/// `leaf` (one item) runs in every vector kind move for move as the rectangles `frames` drawn
/// directly, each its own item.
fn assert_frames_are_rectangles(
    what: &str,
    leaf: impl Fn(&B, &str) -> NodeId,
    frames: &[(f64, f64, f64, f64)],
) {
    let (w, h) = (frames[0].2, frames[0].3);
    for kind in VECTOR_KINDS {
        let (image, warns) = file_of(kind, &leaf);
        let rect = {
            let (mut fx, ids) = laser(|b, _| {
                let rects: Vec<NodeId> = frames
                    .iter()
                    .map(|&(x, y, w, h)| b.rect(x, y, w, h))
                    .collect();
                vec![b.op(kind, "Op 1", &rects)]
            });
            set_up(&mut fx.s, ids[0], kind);
            gcode(&mut fx.s)
        };
        assert!(
            same_moves(&image.moves, &rect.moves),
            "{what} in {kind}: the frame runs move for move as a {w} x {h} rectangle\n\
             image {:?}\nrect {:?}",
            image.moves,
            rect.moves
        );
        assert!(image.burn_mm() > 0.0, "{what} in {kind}: it burns");
        assert!(
            !warns
                .iter()
                .any(|t| t.contains("image") || t.contains("picture")),
            "{what} in {kind}: no warn line for an image: {warns:?}"
        );
    }
}

#[test]
fn c1_an_image_in_each_vector_kind_runs_as_its_frame_rectangle() {
    assert_frame_is_a_rectangle(
        "an image",
        |b, _| b.img(20.0, 20.0, 40.0, 30.0),
        (20.0, 20.0, 40.0, 30.0),
    );
}

#[test]
fn c1_c46_a_linked_clone_of_an_image_runs_as_its_frame_rectangle() {
    assert_frame_is_a_rectangle(
        "a linked clone",
        |b, _| {
            let i = b.img(20.0, 20.0, 40.0, 30.0);
            b.clone_of(i, 100.0, 50.0)
        },
        (120.0, 70.0, 40.0, 30.0),
    );
    assert_frame_is_a_rectangle(
        "a clone of a clone",
        |b, _| {
            let i = b.img(20.0, 20.0, 40.0, 30.0);
            let c = b.clone_of(i, 100.0, 0.0);
            b.clone_of(c, 0.0, 50.0)
        },
        (120.0, 70.0, 40.0, 30.0),
    );
}

#[test]
fn c1_c46_a_clone_of_a_group_and_a_placed_page_member_run_as_the_frame_rectangle() {
    assert_frames_are_rectangles(
        "a clone of a group of two images",
        |b, _| {
            let i = b.img(20.0, 20.0, 40.0, 30.0);
            let j = b.img(80.0, 20.0, 40.0, 30.0);
            let g = b.d.group_objects(&[i, j]).unwrap();
            b.clone_of(g, 100.0, 50.0)
        },
        &[(120.0, 70.0, 40.0, 30.0), (180.0, 70.0, 40.0, 30.0)],
    );
    // A placed page holding the image runs as the same placed page holding a rectangle where
    // the image is (its page frame is part of both, `0106`).
    for kind in VECTOR_KINDS {
        let (image, warns) = file_of(kind, |b, laser| {
            let o = b.drawing_origin();
            let _ = b.img_at(o, 5.0, 5.0, 40.0, 30.0);
            b.place(laser, 10.0, 0.0)
        });
        let (rect, _) = file_of(kind, |b, laser| {
            let o = b.drawing_origin();
            let _ = b.d.create_rect(RectBounds::from_corners(
                Point::new(o.x + 5.0, o.y + 5.0),
                Point::new(o.x + 45.0, o.y + 35.0),
            ));
            b.place(laser, 10.0, 0.0)
        });
        assert!(
            same_moves(&image.moves, &rect.moves),
            "a placed page's image in {kind}: as a rectangle\nimage {:?}\nrect {:?}",
            image.moves,
            rect.moves
        );
        assert!(
            !warns
                .iter()
                .any(|t| t.contains("image") || t.contains("picture")),
            "{kind}: {warns:?}"
        );
    }
}

/// The spec's test of criterion 1: Cut, Score and Cut-out write one closed 40 x 30 line;
/// Engrave hatch lines that fill 40 x 30 and no outline; Engrave + Outline both; Image dots
/// inside and no line.
#[test]
fn c1_the_40_x_30_test_in_every_kind() {
    let img = |b: &B, _: &str| b.img(20.0, 20.0, 40.0, 30.0);
    for kind in ["cut", "score", "cut_out"] {
        let (g, _) = file_of(kind, img);
        assert!(
            (g.burn_mm() - 140.0).abs() < 0.01,
            "{kind}: {}",
            g.burn_mm()
        );
        let b = g.burns();
        assert!(b.len() <= 5, "{kind}: one closed line");
        let (s, e) = (b[0].from, b[b.len() - 1].to);
        assert!(
            (s.0 - e.0).abs() < 1e-6 && (s.1 - e.1).abs() < 1e-6,
            "{kind}: closed"
        );
    }
    let (hatch, _) = file_of("engrave", img);
    let (x0, y0, x1, y1) = hatch.burn_extent().unwrap();
    assert!(x0 >= 20.0 - 1e-6 && x1 <= 60.0 + 1e-6, "{x0} {x1}");
    assert!(y0 >= BED_H - 50.0 - 1e-6 && y1 <= BED_H - 20.0 + 1e-6);
    assert!(
        (hatch.burn_mm() - 60.0 * 40.0).abs() < 0.01,
        "60 hatch lines of 40 mm: {}",
        hatch.burn_mm()
    );
    assert!(
        hatch
            .burns()
            .iter()
            .all(|m| (m.from.1 - m.to.1).abs() < 1e-6),
        "only hatch lines, no outline"
    );
    let (both, _) = file_of("engrave_outline", img);
    let line: f64 = both
        .burns()
        .iter()
        .filter(|m| m.s == 200)
        .map(Mv::len)
        .sum();
    let fill: f64 = both
        .burns()
        .iter()
        .filter(|m| m.s == 600)
        .map(Mv::len)
        .sum();
    assert!((line - 140.0).abs() < 0.01, "the traced frame: {line}");
    assert!((fill - 2400.0).abs() < 0.01, "the hatch: {fill}");
    let (dots, _) = file_of("image", img);
    let (x0, _, x1, _) = dots.burn_extent().unwrap();
    assert!(x0 >= 20.0 - 1e-6 && x1 <= 60.0 + 1e-6);
    assert_dots(&dots, 0.5, 60 * 40, "the left half of 60 rows of 80 dots");
}

/// An image in an Image and a Cut operation: the dots in Marking, then the frame in Cutting.
#[test]
fn c1_an_image_in_image_and_cut_burns_dots_then_its_frame() {
    let (mut fx, ids) = laser(|b, _| {
        let i = b.img(20.0, 20.0, 8.0, 8.0);
        vec![
            i,
            b.op("cut", "Cut 1", &[i]),
            b.op("image", "Image 1", &[i]),
        ]
    });
    set_up(&mut fx.s, ids[2], "image");
    assert!(
        fx.s.edit_operation_setting(ids[1], "main", "power", 80.0)
            .is_ok()
    );
    assert!(
        fx.s.edit_operation_setting(ids[1], "main", "speed", 600.0)
            .is_ok()
    );
    let g = gcode(&mut fx.s);
    let b = g.burns();
    let first_cut = b.iter().position(|m| m.s == 800).expect("the frame");
    assert!(b[..first_cut].iter().all(|m| m.s == 600), "dots first");
    assert!(b[first_cut..].iter().all(|m| m.s == 800), "the frame last");
    let frame: f64 = b[first_cut..].iter().map(Mv::len).sum();
    assert!((frame - 32.0).abs() < 0.01, "{frame}");
    let dots: f64 = b[..first_cut].iter().map(Mv::len).sum();
    assert!((dots - HALF_DOTS as f64 * 0.5).abs() < 0.01, "{dots}");
}

// ---- criterion 48: a plain vector leaf in Image -------------------------------------------

#[test]
fn c48_a_colour_rect_in_image_burns_nothing_and_warns_once_by_leaf() {
    let (mut fx, ids) = laser(|b, _| {
        let r = b.rect(20.0, 20.0, 10.0, 10.0);
        b.d.edit_style(&[r], &StyleEdit::FillEnabled(true)).unwrap();
        vec![r, b.op("image", "Image 1", &[r])]
    });
    set_up(&mut fx.s, ids[1], "image");
    let warns = texts(&fx.s, "warn");
    assert!(
        warns.contains(&"1 item in Image 1 is not a picture and does not burn.".to_owned()),
        "{warns:?}"
    );
    let c = checks(&fx.s)
        .into_iter()
        .find(|c| c["text"] == "1 item in Image 1 is not a picture and does not burn.")
        .unwrap();
    assert_eq!(c["severity"], "warn");
    let target = c["target"].to_string();
    assert!(
        target.contains(&serde_json::to_value(ids[0]).unwrap().to_string()),
        "the line selects the rect: {target}"
    );
    let burned = export(&mut fx.s).map_or(0.0, |b| read(&b).unwrap().burn_mm());
    assert!(burned < 1e-6, "no move for it: {burned}");
    // The same rect with a Gradient fill burns and does not warn.
    let (mut fx, ids) = laser(|b, _| {
        let r = b.rect(20.0, 20.0, 10.0, 10.0);
        shade(b.d, r);
        vec![r, b.op("image", "Image 1", &[r])]
    });
    set_up(&mut fx.s, ids[1], "image");
    assert!(
        !texts(&fx.s, "warn")
            .iter()
            .any(|t| t.contains("not a picture")),
        "{:?}",
        texts(&fx.s, "warn")
    );
    assert!(gcode(&mut fx.s).burn_mm() > 0.0);
}

#[test]
fn c48_counts_leaves_of_a_group_and_names_the_operation_in_the_plural() {
    let (mut fx, ids) = laser(|b, _| {
        let r = b.rect(20.0, 20.0, 10.0, 10.0);
        let p = b.rect(40.0, 20.0, 10.0, 10.0);
        let i = b.img(60.0, 20.0, 8.0, 8.0);
        let g = b.d.group_objects(&[r, p, i]).unwrap();
        vec![
            g,
            b.op("cut", "Cut 1", &[]),
            b.op("image", "Image 1", &[]),
            b.op("image", "Image 2", &[g]),
        ]
    });
    set_up(&mut fx.s, ids[3], "image");
    let warns = texts(&fx.s, "warn");
    assert!(
        warns.contains(&"2 items in Image 2 are not pictures and do not burn.".to_owned()),
        "{warns:?}"
    );
    let n = warns.iter().filter(|t| t.contains("not picture")).count()
        + warns.iter().filter(|t| t.contains("not a picture")).count();
    assert_eq!(n, 1, "once: {warns:?}");
    let g = gcode(&mut fx.s);
    assert_dots(&g, 0.5, HALF_DOTS, "only the image burns");
}

#[test]
fn c48_an_image_never_causes_the_line_in_any_kind() {
    for kind in [
        "cut",
        "score",
        "cut_out",
        "engrave",
        "engrave_outline",
        "image",
    ] {
        let (fx, _) = laser(|b, _| {
            let i = b.img(20.0, 20.0, 8.0, 8.0);
            vec![i, b.op(kind, "Op 1", &[i])]
        });
        let warns = texts(&fx.s, "warn");
        assert!(
            !warns
                .iter()
                .any(|t| t.contains("not a picture") || t.contains("does not burn")),
            "{kind}: {warns:?}"
        );
    }
}

// ---- criteria 2, 13: damage on the wrong kind ----------------------------------------------

fn damaged_on(kind: &str, options: Value) -> Fx {
    let (mut fx, ids) = laser(|b, _| {
        let i = b.img(20.0, 20.0, 8.0, 8.0);
        let op = b.op(kind, "Op 1", &[i]);
        b.d.write_operation_options(op, options.as_object().unwrap())
            .unwrap();
        vec![op]
    });
    set_up(&mut fx.s, ids[0], kind);
    fx
}

#[test]
fn c13_an_image_key_on_an_engrave_is_damage_and_it_does_not_run() {
    for options in [
        json!({"dither": "ordered"}),
        json!({"gamma": 1.0}),
        json!({"invert": false}),
        json!({"threshold_pct": 50}),
        json!({"lowest_power_pct": 0}),
    ] {
        let mut fx = damaged_on("engrave", options.clone());
        let warns = texts(&fx.s, "warn");
        assert!(
            warns.contains(&"1 operation has damaged settings and does not run.".to_owned())
                || warns
                    .iter()
                    .any(|t| t.starts_with("1 operation") && t.contains("damaged settings")),
            "{options}: {warns:?}"
        );
        let burned = export(&mut fx.s).map_or(0.0, |b| read(&b).unwrap().burn_mm());
        assert!(burned < 1e-6, "{options}: the Engrave does not run");
    }
}

#[test]
fn c2_c13_crosshatch_on_an_image_operation_is_damage_and_refused_as_an_edit() {
    let mut fx = damaged_on("image", json!({"crosshatch": true}));
    let warns = texts(&fx.s, "warn");
    assert!(
        warns
            .iter()
            .any(|t| t.starts_with("1 operation") && t.contains("damaged settings")),
        "{warns:?}"
    );
    let burned = export(&mut fx.s).map_or(0.0, |b| read(&b).unwrap().burn_mm());
    assert!(burned < 1e-6, "the Image does not run");
    // A healthy Image operation refuses the edit and has no Crosshatch field.
    let (mut fx, ids) = laser(|b, _| {
        let i = b.img(20.0, 20.0, 8.0, 8.0);
        vec![b.op("image", "Image 1", &[i])]
    });
    set_up(&mut fx.s, ids[0], "image");
    assert_ne!(
        fx.s.edit_operation_option(ids[0], "crosshatch", 1.0),
        Ok(true)
    );
    assert!(fx.s.unfold_operation_row(ids[0]) || row_of(&fx.s, ids[0])["unfolded"] == true);
    let hatch = &row_of(&fx.s, ids[0])["settings"]["options"]["hatch"];
    assert!(hatch["crosshatch"].is_null(), "{hatch}");
    assert!(!texts(&fx.s, "warn").iter().any(|t| t.contains("damaged")));
}

// ---- criteria 2, 13, 45: the Image set of a material (Frage 270 B) -------------------------

fn materials(s: &Session) -> Value {
    serde_json::from_str(&s.materials_view_json()).unwrap()
}

fn stored_main(s: &Session, op: NodeId) -> Value {
    let d = curvyo_document_core::unpack(99, &s.pack("0.1.0").unwrap()).unwrap();
    let page = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap();
    let o = d
        .operations(page.kind.manufacturing().unwrap())
        .unwrap()
        .into_iter()
        .find(|o| o.id == op)
        .unwrap();
    o.settings.map_or(Value::Null, |s| s["main"].clone())
}

#[test]
fn c2_an_image_operation_reads_the_image_set_never_the_engrave_set() {
    let (mut fx, ids) = laser_on(false, |b, _| {
        let i = b.img(20.0, 20.0, 8.0, 8.0);
        vec![
            b.op("image", "Image 1", &[i]),
            b.op("engrave", "Engrave 1", &[i]),
        ]
    });
    let (image, engrave) = (ids[0], ids[1]);
    let laser_id = fx
        .profile
        .clone()
        .unwrap_or_else(|| "generic-laser".to_owned());
    let m = fx.s.add_material("Slate", 5.0, &[1; 16]).unwrap();
    fx.s.add_material_set(&m, &laser_id, "engrave", &[2; 16])
        .unwrap();
    assert_eq!(
        fx.s.edit_material_set(&m, &laser_id, "engrave", "power", 77.0, &[3; 16]),
        Ok(true)
    );
    for op in [image, engrave] {
        assert!(
            fx.s.edit_operation_setting(op, "main", "power", 33.0)
                .is_ok()
        );
        assert!(
            fx.s.edit_operation_setting(op, "main", "speed", 1200.0)
                .is_ok()
        );
    }
    // No Image set: the Image operation keeps its stored numbers and names the missing set.
    assert!(fx.s.use_material_on(&[image, engrave], &m));
    assert_eq!(
        stored_main(&fx.s, engrave)["power_pct"],
        77.0,
        "Engrave set"
    );
    assert_eq!(
        stored_main(&fx.s, image)["power_pct"],
        33.0,
        "no fallback to the Engrave set"
    );
    let _ = fx.s.unfold_operation_row(image);
    let row = row_of(&fx.s, image);
    assert_eq!(
        row["settings"]["disabled"][0]["reason"], "No Image set",
        "the pick names the missing Image set: {row}"
    );
    // With an Image set: the Image operation reads it.
    fx.s.add_material_set(&m, &laser_id, "image", &[4; 16])
        .unwrap();
    assert_eq!(
        fx.s.edit_material_set(&m, &laser_id, "image", "power", 44.0, &[5; 16]),
        Ok(true)
    );
    assert_eq!(
        fx.s.edit_material_set(&m, &laser_id, "image", "interval", 0.25, &[6; 16]),
        Ok(true)
    );
    assert!(fx.s.use_material_on(&[image], &m));
    assert_eq!(
        stored_main(&fx.s, image)["power_pct"],
        44.0,
        "the Image set"
    );
    assert_eq!(stored_main(&fx.s, image)["interval_mm"], 0.25);
    assert_eq!(
        stored_main(&fx.s, engrave)["power_pct"],
        77.0,
        "the Engrave keeps the Engrave set"
    );
    // The Materials tab shows the four kinds' sets with Image among them.
    fx.s.unfold_material(Some(&m));
    let v = materials(&fx.s);
    let kinds: Vec<String> = v["unfolded"]["lasers"][0]["sets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["kind"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(kinds, ["engrave", "image"], "{v}");
    // The linked material loses its Image set: the row says so and keeps the numbers.
    fx.s.delete_material_set(&m, &laser_id, "image", &[7; 16])
        .unwrap();
    let row = row_of(&fx.s, image).to_string();
    assert!(
        row.contains("Material has no Image set for this laser"),
        "{row}"
    );
    assert_eq!(
        stored_main(&fx.s, image)["power_pct"],
        44.0,
        "the stored numbers stay"
    );
    fx.s.add_material_set(&m, &laser_id, "image", &[8; 16])
        .unwrap();
    // A material pick never writes the Image settings (criterion 13).
    assert!(fx.s.set_operation_dither(image, "jarvis"));
    assert!(fx.s.use_material_on(&[image, engrave], &m));
    let o = curvyo_document_core::unpack(99, &fx.s.pack("0.1.0").unwrap()).unwrap();
    let page = o
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap();
    let op = o
        .operations(page.kind.manufacturing().unwrap())
        .unwrap()
        .into_iter()
        .find(|x| x.id == image)
        .unwrap();
    assert_eq!(op.options.unwrap()["dither"], "jarvis");
}

/// A worked cell of an Image grid sets the material's Image set, never the Engrave set; the
/// Materials tab shows it with the source "Test <id>".
#[test]
fn c45_judging_an_image_test_writes_the_image_set() {
    let (mut fx, _) = laser_on(false, |_, _| vec![]);
    load_fonts(&mut fx.s);
    fx.s.set_tool(Tool::TestGrid);
    let _ = fx.s.test_grid_press(None);
    add_wanted_fonts(&mut fx.s);
    let o = fx.s.active_page().origin;
    let _ =
        fx.s.test_grid_press(Some(Point::new(o.x + 10.0, o.y + 10.0)))
            .unwrap();
    fx.s.set_tool(Tool::Select);
    assert_eq!(fx.s.set_test_grid_kind("image"), Ok(true));
    let m = fx.s.add_material("Slate", 5.0, &[1; 16]).unwrap();
    assert_eq!(fx.s.pick_test_grid_material(Some(&m)), Ok(true));
    let grid: Value = serde_json::from_str(&fx.s.test_grid_view_json()).unwrap();
    let id = grid["id"].as_str().unwrap().to_owned();
    fx.s.mark_pick(&m, &id, "best", 1, 1, &[8; 16]).unwrap();
    fx.s.unfold_material(Some(&m));
    let v = materials(&fx.s);
    let sets = v["unfolded"]["lasers"][0]["sets"]
        .as_array()
        .unwrap()
        .clone();
    let kinds: Vec<&str> = sets.iter().map(|s| s["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["image"], "only the Image set: {v}");
    assert_eq!(sets[0]["source"], format!("Test {id}"));
    assert!(!sets[0]["interval_mm"].is_null(), "{}", sets[0]);
    let _ = fx.page;
}

// ---- criterion 41: the format 50 goldens ---------------------------------------------------

fn open_golden(name: &str) -> Session {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures")
            .join(name),
    )
    .unwrap();
    let d = curvyo_document_core::unpack(3, &bytes).unwrap();
    let laser = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap()
        .id;
    let mut s = Session::open(3, &bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.load_machine_profiles_json("[]");
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(&laser));
    s.load_materials(&[], None);
    s
}

/// The golden holds an Image operation with an image, its linked clone and a gradient ellipse,
/// and an Engrave that holds the image and hatches its frame (criterion 41).
#[test]
fn c41_the_golden_has_an_image_operation_of_three_items_and_an_engrave_of_the_image() {
    let mut s = open_golden("raster_engraving_v50.curvyo");
    let rows = ops_view(&s)["rows"].as_array().unwrap().clone();
    let image = rows
        .iter()
        .find(|r| r["kind"] == "image" && r["source"].is_null())
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(image["items"], 3, "image, linked clone, ellipse: {image}");
    let engrave = rows
        .iter()
        .find(|r| r["kind"] == "engrave")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert!(engrave["items"].as_u64().unwrap() >= 1, "{engrave}");
    assert!(!engrave["run"].is_null(), "the Engrave runs: {engrave}");
    assert!(!image["run"].is_null(), "the Image runs: {image}");
    let warns = texts(&s, "warn");
    assert!(
        !warns
            .iter()
            .any(|t| t.contains("damaged") || t.contains("not a picture")),
        "{warns:?}"
    );
    let bytes = export(&mut s).unwrap_or_else(|| panic!("{:?}", checks(&s)));
    let g = read(&bytes).unwrap();
    assert!(g.burn_mm() > 0.0);
    assert!(!g.rows().is_empty(), "rows: hatch and dots");
}

#[test]
fn c41_the_damaged_golden_runs_none_of_its_damaged_operations() {
    let mut s = open_golden("raster_engraving_damaged_v50.curvyo");
    let warns = texts(&s, "warn");
    let line = warns
        .iter()
        .find(|t| t.contains("damaged settings"))
        .unwrap_or_else(|| panic!("{warns:?}"));
    assert!(
        line.ends_with("operations have damaged settings and do not run."),
        "{line}"
    );
    let n: usize = line.split(' ').next().unwrap().parse().unwrap();
    let ops = ops_view(&s)["rows"].as_array().unwrap().len();
    assert!(n >= 3 && n <= ops, "{n} of {ops}");
    if n == ops {
        let burned = export(&mut s).map_or(0.0, |b| read(&b).unwrap().burn_mm());
        assert!(burned < 1e-6, "nothing runs: {burned}");
    }
}
