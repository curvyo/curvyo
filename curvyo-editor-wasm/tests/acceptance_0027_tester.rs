//! Independent acceptance tests for `specs/0027-manufacturing-roles` through the public session:
//! the Layers block rows (criteria 1, 11, 14, 16), role presses with undo (2, 4), storage through
//! the session (5, 11), the role looks on the output page and the Role view switch (6, 7, 8),
//! the layer that decides (9), guides and dimensions (12), locked and opacity (13), hidden (14),
//! the notices (15), deleted layers and unknown values (17, 18), the golden through the session
//! and the extension switch (19), delete of the manufacturing (20), the read-only page (22) and
//! the untouched Vector workbench (23). The output page of the first page at the origin has the
//! drawing's coordinates (`0093` criterion 5), so leaves are found by place and role colour.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::cast_possible_truncation
)]

use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, AnchorId, Color, DimensionKind, DimensionPlace, DimensionSlots, Document, Length,
    MovePlan, MoveTarget, NewAnchor, NewImage, NodeId, Opacity, Point, PointSlot, RectBounds,
    StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, SvgExportResult, Tool, Workbench};
use curvyo_laser_core::Role;
use curvyo_render_core::{DrawList, ROLE_CUT, ROLE_ENGRAVE, ROLE_IGNORE, RgbaColor};
use curvyo_ui_core::SvgExportOptions;
use serde_json::Value;

const LASER: &str = "curvyo.laser";

// ---- helpers ------------------------------------------------------------------------------------

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// A rectangle with a red fill and no stroke at (x, y), w x h.
fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    let id = d.create_rect(RectBounds::from_corners(pt(x, y), pt(x + w, y + h)));
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[id], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::StrokeEnabled(false))
        .unwrap();
    id
}

fn new_layer(d: &Document, name: &str) -> NodeId {
    let l = d.new_layer(None);
    d.rename_node(l, name).unwrap();
    l
}

fn into(d: &Document, nodes: &[NodeId], layer: NodeId) {
    d.move_nodes(&MovePlan {
        nodes: nodes.to_vec(),
        target: MoveTarget::FrontOf(Some(layer)),
    })
    .unwrap();
}

/// A layer named `name` with one 20 x 20 red square at (x, 20).
fn layer_with_square(d: &Document, name: &str, x: f64) -> (NodeId, NodeId) {
    let l = new_layer(d, name);
    let r = rect(d, x, 20.0, 20.0, 20.0);
    into(d, &[r], l);
    (l, r)
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2700, &pack(d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}

fn add_laser(s: &mut Session) -> NodeId {
    s.add_manufacturing(LASER).expect("Laser can be added")
}

fn opened(s: &Session) -> Document {
    unpack(2799, &s.pack("0.1.0").unwrap()).unwrap()
}

fn kt(id: NodeId) -> String {
    id.to_register_text()
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.manufacturing_view_json()).unwrap()
}

fn block(s: &Session) -> Value {
    view(s)["layers"].clone()
}

fn rows(s: &Session) -> Vec<Value> {
    block(s)["rows"].as_array().cloned().unwrap_or_default()
}

fn row(s: &Session, key: &str) -> Value {
    rows(s)
        .into_iter()
        .find(|r| r["key"] == key)
        .unwrap_or_else(|| panic!("no row {key}: {}", block(s)))
}

fn notices(s: &Session) -> Vec<String> {
    block(s)["notices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap().to_owned())
        .collect()
}

fn stored_roles(s: &Session) -> Vec<(String, Value)> {
    let d = opened(s);
    let mut out: Vec<(String, Value)> = d
        .manufacturings()
        .into_iter()
        .flat_map(|m| m.process_values.into_iter())
        .filter(|(k, _)| k.starts_with("x.curvyo.laser.role."))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn same(a: RgbaColor, b: RgbaColor) -> bool {
    a.r == b.r && a.g == b.g && a.b == b.b && a.a > 0
}

fn tris(list: &DrawList, c: RgbaColor) -> Vec<[Point; 3]> {
    list.triangles
        .chunks(3)
        .filter(|t| same(t[0].color, c))
        .map(|t| [t[0].position, t[1].position, t[2].position])
        .collect()
}

fn inside(t: &[Point; 3], p: Point) -> bool {
    let [a, b, c] = *t;
    let d1 = (p.x - b.x) * (a.y - b.y) - (a.x - b.x) * (p.y - b.y);
    let d2 = (p.x - c.x) * (b.y - c.y) - (b.x - c.x) * (p.y - c.y);
    let d3 = (p.x - a.x) * (c.y - a.y) - (c.x - a.x) * (p.y - a.y);
    let neg = d1 < -1e-12 || d2 < -1e-12 || d3 < -1e-12;
    let pos = d1 > 1e-12 || d2 > 1e-12 || d3 > 1e-12;
    !(neg && pos)
}

fn covers(list: &DrawList, c: RgbaColor, p: Point) -> bool {
    tris(list, c).iter().any(|t| inside(t, p))
}

/// Vertices of colour `c` within the box (x0, y0)-(x1, y1).
fn in_box(list: &DrawList, c: RgbaColor, b: (f64, f64, f64, f64)) -> usize {
    list.triangles
        .iter()
        .filter(|v| {
            same(v.color, c)
                && v.position.x >= b.0
                && v.position.x <= b.2
                && v.position.y >= b.1
                && v.position.y <= b.3
        })
        .count()
}

fn any_role_colour_near(list: &DrawList, b: (f64, f64, f64, f64)) -> bool {
    [ROLE_CUT, ROLE_ENGRAVE, ROLE_IGNORE]
        .into_iter()
        .any(|c| in_box(list, c, b) > 0)
}

/// What a square at (x, 20) 20 x 20 looks like: "cut", "engrave", "ignore", "none" (nothing in
/// any role colour) or "mixed".
fn look_of_square(list: &DrawList, x: f64) -> &'static str {
    let centre = pt(x + 10.0, 30.0);
    let edge = pt(x, 30.0);
    let near = (x - 1.0, 19.0, x + 21.0, 41.0);
    let cut = in_box(list, ROLE_CUT, near) > 0;
    let eng = in_box(list, ROLE_ENGRAVE, near) > 0;
    let ign = in_box(list, ROLE_IGNORE, near) > 0;
    match (cut, eng, ign) {
        (false, false, false) => "none",
        (true, false, false) => {
            assert!(covers(list, ROLE_CUT, edge), "Cut traces the outline");
            assert!(!covers(list, ROLE_CUT, centre), "Cut has no fill");
            "cut"
        }
        (false, true, false) => {
            assert!(covers(list, ROLE_ENGRAVE, centre), "Engrave fills");
            "engrave"
        }
        (false, false, true) => {
            assert!(!covers(list, ROLE_IGNORE, centre), "Ignore has no fill");
            "ignore"
        }
        _ => "mixed",
    }
}

fn mm_per_px(s: &Session) -> f64 {
    let a = s.screen_to_document(100.0, 100.0);
    let b = s.screen_to_document(200.0, 100.0);
    (b.x - a.x).abs() / 100.0
}

/// The thickness in mm of the Cut line along the left edge x = x0 of a square spanning y 20..40.
fn cut_thickness_left(list: &DrawList, x0: f64) -> f64 {
    let t = tris(list, ROLE_CUT);
    let steps = 4000;
    let step = 4.0 / f64::from(steps);
    let covered = (0..steps)
        .filter(|i| {
            let p = pt(x0 - 2.0 + step * f64::from(*i), 30.0);
            t.iter().any(|tr| inside(tr, p))
        })
        .count();
    assert!(covered > 0, "a Cut line at x = {x0}");
    covered as f64 * step
}

fn key_with(s: &mut Session, k: &str, ctrl: bool, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        ..KeyInput::default()
    })
}

fn svg(s: &Session) -> String {
    match s.export_svg(SvgExportOptions::default()) {
        SvgExportResult::Written { text, .. } => text,
        SvgExportResult::NothingSelected => panic!("nothing selected"),
    }
}

fn picture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images")
            .join(name),
    )
    .unwrap()
}

/// Layers A, B, C (front-most is the last made) each with one square at x = 20, 60, 100.
fn abc() -> (Document, [NodeId; 3]) {
    let d = Document::new(1);
    let (a, _) = layer_with_square(&d, "A", 20.0);
    let (b, _) = layer_with_square(&d, "B", 60.0);
    let (c, _) = layer_with_square(&d, "C", 100.0);
    (d, [a, b, c])
}

// ---- criterion 1: rows ---------------------------------------------------------------------

#[test]
fn ac1_one_row_per_layer_front_most_first_with_name_and_state() {
    let (d, [a, b, c]) = abc();
    d.set_node_visible(&[b], false).unwrap();
    d.set_node_locked(&[c], true).unwrap();
    let mut s = open(&d);
    add_laser(&mut s);
    let r = rows(&s);
    let keys: Vec<&str> = r.iter().map(|r| r["key"].as_str().unwrap()).collect();
    let names: Vec<&str> = r.iter().map(|r| r["name"].as_str().unwrap()).collect();
    // The Layers tab's order: front-most first.
    let tab_order: Vec<String> = {
        let v: Value = serde_json::from_str(&s.layers_view_json(0, 500)).unwrap();
        v["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["depth"] == 0 || r["depth"].is_null())
            .filter_map(|r| r["name"].as_str().map(str::to_owned))
            .filter(|n| ["A", "B", "C"].contains(&n.as_str()))
            .collect()
    };
    assert_eq!(names, tab_order, "the order of the Layers tab");
    assert_eq!(names.len(), 3, "{r:?}");
    assert_eq!(
        keys.iter()
            .copied()
            .collect::<std::collections::HashSet<_>>(),
        [kt(a), kt(b), kt(c)]
            .iter()
            .map(String::as_str)
            .collect::<std::collections::HashSet<_>>()
    );
    assert_eq!(row(&s, &kt(b))["hidden"], true);
    assert_eq!(row(&s, &kt(a))["hidden"], false);
    assert_eq!(row(&s, &kt(c))["locked"], true);
    for r in &r {
        assert!(r["role"].is_null(), "every layer starts with no role: {r}");
        assert_eq!(r["outside"], false);
    }
}

#[test]
fn ac1_ac11_loose_objects_add_one_outside_row_at_the_end() {
    let (d, _) = abc();
    let _loose = rect(&d, 200.0, 20.0, 20.0, 20.0);
    let mut s = open(&d);
    add_laser(&mut s);
    let r = rows(&s);
    assert_eq!(r.len(), 4, "{r:?}");
    assert_eq!(r[3]["key"], "outside");
    assert_eq!(r[3]["outside"], true);
    assert_eq!(r[3]["name"], "Objects outside layers");
    assert!(r[3]["role"].is_null());
    for x in &r[..3] {
        assert_eq!(x["outside"], false);
    }
}

#[test]
fn ac1_a_drawing_without_layers_and_loose_objects_has_no_rows() {
    let d = Document::new(1);
    let mut s = open(&d);
    add_laser(&mut s);
    assert!(rows(&s).is_empty(), "{}", block(&s));
    assert!(notices(&s).is_empty());
}

#[test]
fn ac1_only_loose_objects_show_only_the_outside_row() {
    let d = Document::new(1);
    let _ = rect(&d, 20.0, 20.0, 20.0, 20.0);
    let mut s = open(&d);
    add_laser(&mut s);
    let r = rows(&s);
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!(r[0]["key"], "outside");
}

#[test]
fn ac1_a_drawing_workbench_has_no_layers_block() {
    let (d, _) = abc();
    let s = open(&d);
    assert_eq!(s.manufacturing_view_json(), "null");
}

// ---- criteria 2 and 4: presses and undo ------------------------------------------------------

#[test]
fn ac4_a_press_is_one_set_role_step_m2_unaffected_undo_redo() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    let m1 = add_laser(&mut s);
    let m2 = add_laser(&mut s);
    s.set_workbench(s.workbench_named(&m1.full_id()).unwrap());
    let drawing = format!("{:?} {:?}", opened(&s).read_nodes(), opened(&s).pages());
    let steps = s.undo_len();
    assert!(s.set_layer_role(&kt(a), Some(Role::Cut)));
    assert_eq!(s.undo_len(), steps + 1, "one undo step");
    assert_eq!(curvyo_ui_core::operation_name("set_role"), "Set role");
    assert_eq!(row(&s, &kt(a))["role"], "cut");
    assert!(!s.set_layer_role(&kt(a), Some(Role::Cut)), "same state");
    assert_eq!(s.undo_len(), steps + 1, "no second step");

    s.set_workbench(s.workbench_named(&m2.full_id()).unwrap());
    assert!(row(&s, &kt(a))["role"].is_null(), "M2 still No role");

    s.set_workbench(s.workbench_named(&m1.full_id()).unwrap());
    let _ = s.undo();
    assert!(row(&s, &kt(a))["role"].is_null(), "undo clears it");
    let _ = s.redo();
    assert_eq!(row(&s, &kt(a))["role"], "cut", "redo restores it");
    assert_eq!(
        format!("{:?} {:?}", opened(&s).read_nodes(), opened(&s).pages()),
        drawing,
        "the drawing never changes"
    );
}

#[test]
fn ac2_clearing_is_one_step_and_clearing_no_role_is_none() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    add_laser(&mut s);
    let steps = s.undo_len();
    assert!(!s.set_layer_role(&kt(a), None), "nothing to clear");
    assert_eq!(s.undo_len(), steps);
    s.set_layer_role(&kt(a), Some(Role::Engrave));
    assert!(s.set_layer_role(&kt(a), None));
    assert_eq!(s.undo_len(), steps + 2);
    assert!(row(&s, &kt(a))["role"].is_null());
    assert!(stored_roles(&s).is_empty(), "clearing deletes the register");
}

#[test]
fn ac2_hidden_and_locked_layers_take_roles() {
    let (d, [a, b, _]) = abc();
    d.set_node_visible(&[a], false).unwrap();
    d.set_node_locked(&[b], true).unwrap();
    let mut s = open(&d);
    add_laser(&mut s);
    assert!(s.set_layer_role(&kt(a), Some(Role::Cut)));
    assert!(s.set_layer_role(&kt(b), Some(Role::Engrave)));
    assert_eq!(row(&s, &kt(a))["role"], "cut");
    assert_eq!(row(&s, &kt(b))["role"], "engrave");
}

#[test]
fn ac4_a_press_for_a_key_with_no_row_writes_nothing() {
    let (d, _) = abc();
    let mut s = open(&d);
    add_laser(&mut s);
    let steps = s.undo_len();
    for k in [
        "",
        "junk",
        "1:999999",
        "outside ",
        "x.curvyo.laser.role.1:1",
    ] {
        assert!(!s.set_layer_role(k, Some(Role::Cut)), "{k:?}");
    }
    assert_eq!(s.undo_len(), steps);
    assert!(stored_roles(&s).is_empty(), "{:?}", stored_roles(&s));
}

#[test]
fn ac4_a_press_in_the_vector_workbench_writes_nothing() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_workbench(Workbench::Vector);
    let steps = s.undo_len();
    assert!(!s.set_layer_role(&kt(a), Some(Role::Cut)));
    assert_eq!(s.undo_len(), steps);
}

// ---- criterion 3: duplicate ------------------------------------------------------------------

#[test]
fn ac3_a_duplicated_layer_starts_with_no_role() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    s.set_workbench(Workbench::Vector);
    let before: Vec<NodeId> = opened(&s).children(None);
    let _ = s.press_layer_row(
        &serde_json::to_string(&a).unwrap(),
        curvyo_editor_wasm::RowPress::Plain,
    );
    let _ = s.duplicate_layer_target();
    let after: Vec<NodeId> = opened(&s).children(None);
    let new: Vec<NodeId> = after.into_iter().filter(|n| !before.contains(n)).collect();
    assert_eq!(new.len(), 1, "a layer copy was made");
    let m = opened(&s).manufacturings()[0].id;
    s.set_workbench(s.workbench_named(&m.full_id()).unwrap());
    assert!(row(&s, &kt(new[0]))["role"].is_null(), "copy has no role");
    assert_eq!(row(&s, &kt(a))["role"], "cut", "source keeps its role");
}

// ---- criterion 5 and 11: storage through the session -----------------------------------------

#[test]
fn ac5_ac11_the_session_writes_the_layer_key_and_the_reserved_outside_key() {
    let (d, [a, _, _]) = abc();
    let _ = rect(&d, 200.0, 20.0, 20.0, 20.0);
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    s.set_layer_role("outside", Some(Role::Ignore));
    let r = stored_roles(&s);
    assert_eq!(
        r,
        vec![
            (format!("x.curvyo.laser.role.{}", kt(a)), Value::from("cut")),
            (
                "x.curvyo.laser.role.outside".to_owned(),
                Value::from("ignore")
            ),
        ]
    );
}

#[test]
fn ac11_the_outside_row_goes_with_its_objects_and_its_role_is_kept() {
    let (d, [a, _, _]) = abc();
    let loose = rect(&d, 200.0, 20.0, 20.0, 20.0);
    let mut s = open(&d);
    let m = add_laser(&mut s);
    s.set_layer_role("outside", Some(Role::Engrave));
    // Move the loose object into layer A in the drawing (a peer's edit, through a reopen).
    let d2 = opened(&s);
    into(&d2, &[loose], a);
    let mut s = open(&d2);
    s.set_workbench(s.workbench_named(&m.full_id()).unwrap());
    assert!(
        rows(&s).iter().all(|r| r["key"] != "outside"),
        "no outside row without loose objects"
    );
    assert!(
        stored_roles(&s)
            .iter()
            .any(|(k, v)| k.ends_with(".outside") && v == "engrave"),
        "the stored role is kept"
    );
}

// ---- criterion 6: looks and the switch ---------------------------------------------------------

#[test]
fn ac6_the_three_looks_differ_by_shape() {
    let (d, [a, b, c]) = abc();
    let (u, _) = layer_with_square(&d, "U", 140.0);
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    s.set_layer_role(&kt(b), Some(Role::Engrave));
    s.set_layer_role(&kt(c), Some(Role::Ignore));
    let _ = u;
    let list = s.frame_draw_list();
    assert_eq!(look_of_square(&list, 20.0), "cut");
    assert_eq!(look_of_square(&list, 60.0), "engrave");
    assert_eq!(look_of_square(&list, 100.0), "ignore");
    assert_eq!(look_of_square(&list, 140.0), "ignore", "unassigned");
    assert!(
        !covers(&list, ROLE_ENGRAVE, pt(60.0 - 0.2, 30.0)),
        "Engrave has no outline beyond the area"
    );
}

#[test]
fn ac6_ignore_is_dashed_along_its_outline() {
    let (d, [_, _, c]) = abc();
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_layer_role(&kt(c), Some(Role::Ignore));
    let list = s.frame_draw_list();
    let samples: Vec<bool> = (0..400)
        .map(|i| {
            covers(
                &list,
                ROLE_IGNORE,
                pt(100.5 + 19.0 * f64::from(i) / 400.0, 20.0),
            )
        })
        .collect();
    let on = samples.iter().filter(|x| **x).count();
    assert!(on > 0, "a dashed line is drawn");
    assert!(on < samples.len(), "with gaps: {on} of {}", samples.len());
}

#[test]
fn ac6_cut_is_one_screen_pixel_at_any_zoom() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    for _ in 0..3 {
        let px = mm_per_px(&s);
        let w = cut_thickness_left(&s.frame_draw_list(), 20.0);
        assert!(
            w > 0.5 * px && w < 1.6 * px,
            "{w} mm wide at {px} mm per px (zoom {})",
            s.zoom_percent()
        );
        s.wheel(0.0, -300.0, 400.0, 300.0, false, true);
    }
}

#[test]
fn ac6_role_view_is_on_by_default_off_shows_the_drawing_not_saved_no_undo() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    assert_eq!(block(&s)["role_view"], true, "on by default");
    let on = s.frame_draw_list();
    assert!(!tris(&on, ROLE_CUT).is_empty());
    let steps = s.undo_len();
    let saved = s.pack("0.1.0").unwrap();
    s.set_role_view(false);
    assert_eq!(s.undo_len(), steps, "not an undo step");
    assert_eq!(block(&s)["role_view"], false);
    let off = s.frame_draw_list();
    for c in [ROLE_CUT, ROLE_ENGRAVE, ROLE_IGNORE] {
        assert!(tris(&off, c).is_empty(), "no role look with the switch off");
    }
    let red = RgbaColor::opaque(255, 0, 0);
    assert!(covers(&off, red, pt(30.0, 30.0)), "the drawing as drawn");
    let saved_off = s.pack("0.1.0").unwrap();
    assert_eq!(
        unpack(1, &saved).unwrap().export_json().unwrap(),
        unpack(1, &saved_off).unwrap().export_json().unwrap(),
        "not saved in the file"
    );
    let mut r = Session::open(2701, &saved_off).unwrap();
    let m = opened(&r).manufacturings()[0].id;
    r.set_workbench(r.workbench_named(&m.full_id()).unwrap());
    assert_eq!(block(&r)["role_view"], true, "on again at start");
}

// ---- criterion 7: own style ignored ------------------------------------------------------------

fn styled_square(style: impl FnOnce(&Document, NodeId), role: Role) -> DrawList {
    let d = Document::new(1);
    let l = new_layer(&d, "L");
    let r = d.create_rect(RectBounds::from_corners(pt(20.0, 20.0), pt(40.0, 40.0)));
    style(&d, r);
    into(&d, &[r], l);
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_layer_role(&kt(l), Some(role));
    s.frame_draw_list()
}

fn colour_signature(list: &DrawList, c: RgbaColor) -> (usize, (i64, i64, i64, i64)) {
    let t = tris(list, c);
    let pts: Vec<Point> = t.iter().flatten().copied().collect();
    let q = |v: f64| (v * 1000.0).round() as i64;
    let b = pts.iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |a, p| (a.0.min(p.x), a.1.min(p.y), a.2.max(p.x), a.3.max(p.y)),
    );
    (t.len(), (q(b.0), q(b.1), q(b.2), q(b.3)))
}

#[test]
fn ac7_a_wide_red_stroke_and_a_blue_fill_draw_the_same_under_cut_and_engrave() {
    let wide = |d: &Document, r: NodeId| {
        d.edit_style(&[r], &StyleEdit::FillEnabled(false)).unwrap();
        d.edit_style(&[r], &StyleEdit::StrokeEnabled(true)).unwrap();
        d.edit_style(&[r], &StyleEdit::StrokeColor(Color { r: 255, g: 0, b: 0 }))
            .unwrap();
        d.edit_style(&[r], &StyleEdit::StrokeWidth(Length::from_mm(10.0)))
            .unwrap();
    };
    let blue = |d: &Document, r: NodeId| {
        d.edit_style(&[r], &StyleEdit::StrokeEnabled(false))
            .unwrap();
        d.edit_style(&[r], &StyleEdit::FillEnabled(true)).unwrap();
        d.edit_style(&[r], &StyleEdit::FillColor(Color { r: 0, g: 0, b: 255 }))
            .unwrap();
    };
    for (role, c) in [(Role::Cut, ROLE_CUT), (Role::Engrave, ROLE_ENGRAVE)] {
        let a = styled_square(wide, role);
        let b = styled_square(blue, role);
        assert_eq!(
            colour_signature(&a, c),
            colour_signature(&b, c),
            "{role:?}: own style changes nothing"
        );
        assert!(colour_signature(&a, c).0 > 0);
        // Nothing of the own colours is drawn.
        for own in [RgbaColor::opaque(255, 0, 0), RgbaColor::opaque(0, 0, 255)] {
            assert!(
                tris(&a, own).is_empty() && tris(&b, own).is_empty(),
                "{role:?}"
            );
        }
    }
    // The Cut hairline is along the centre line: the 10 mm stroke would reach x = 15.
    let a = styled_square(wide, Role::Cut);
    let (_, (x0, _, x1, _)) = colour_signature(&a, ROLE_CUT);
    assert!(x0 > 19_000 && x1 < 41_000, "{x0} {x1}");
}

// ---- criterion 8: kinds -----------------------------------------------------------------------

fn open_path(d: &Document, peer: u64, from: Point, to: Point) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(peer, 1), from),
            NewAnchor::corner(AnchorId::new(peer, 2), pt(from.x, to.y)),
            NewAnchor::corner(AnchorId::new(peer, 3), to),
        ],
        false,
    )
}

#[test]
fn ac8_an_open_path_is_traced_under_cut_and_a_line_under_engrave() {
    for (role, c) in [(Role::Cut, ROLE_CUT), (Role::Engrave, ROLE_ENGRAVE)] {
        let d = Document::new(1);
        let l = new_layer(&d, "L");
        let p = open_path(&d, 1, pt(20.0, 20.0), pt(40.0, 40.0));
        into(&d, &[p], l);
        let mut s = open(&d);
        add_laser(&mut s);
        s.set_layer_role(&kt(l), Some(role));
        let list = s.frame_draw_list();
        assert!(covers(&list, c, pt(20.0, 30.0)), "{role:?} draws the line");
        assert!(
            !covers(&list, c, pt(30.0, 30.0)),
            "{role:?}: an all-open leaf is not filled"
        );
        assert!(
            !covers(&list, c, pt(30.0, 20.0)),
            "{role:?}: the open side is not closed"
        );
    }
}

#[test]
fn ac8_groups_at_depth_and_clones_take_the_role_of_their_own_layer() {
    let d = Document::new(1);
    let cut = new_layer(&d, "Cut");
    let eng = new_layer(&d, "Engrave");
    let r1 = rect(&d, 20.0, 20.0, 20.0, 20.0);
    let r2 = rect(&d, 60.0, 20.0, 20.0, 20.0);
    let g1 = d.group_objects(&[r1, r2]).unwrap();
    let r3 = rect(&d, 220.0, 20.0, 20.0, 20.0);
    let g2 = d.group_objects(&[g1, r3]).unwrap();
    into(&d, &[g2], cut);
    let original = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 501), pt(100.0, 20.0)),
            NewAnchor::corner(AnchorId::new(1, 502), pt(120.0, 20.0)),
            NewAnchor::corner(AnchorId::new(1, 503), pt(120.0, 40.0)),
            NewAnchor::corner(AnchorId::new(1, 504), pt(100.0, 40.0)),
        ],
        true,
    );
    into(&d, &[original], cut);
    let clone = d.linked_clone(&[original], &[]).unwrap()[0];
    d.translate_objects(&[clone], curvyo_document_core::Vec2::new(40.0, 0.0))
        .unwrap();
    into(&d, &[clone], eng);
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_layer_role(&kt(cut), Some(Role::Cut));
    s.set_layer_role(&kt(eng), Some(Role::Engrave));
    let list = s.frame_draw_list();
    assert_eq!(look_of_square(&list, 20.0), "cut", "group member depth 2");
    assert_eq!(look_of_square(&list, 60.0), "cut");
    assert_eq!(look_of_square(&list, 100.0), "cut", "the original");
    assert_eq!(
        look_of_square(&list, 140.0),
        "engrave",
        "the clone takes its own layer's role"
    );
}

fn image_session(role: Option<Role>) -> (Session, DrawList) {
    let d = Document::new(1);
    let l = new_layer(&d, "Pictures");
    let id = d
        .add_images(&[NewImage {
            bytes: Arc::from(picture("png_rgba_8.png").as_slice()),
            placement: Affine::new(20.0, 0.0, 0.0, 20.0, 20.0, 20.0),
            name: "a.png".to_owned(),
        }])
        .unwrap()[0];
    into(&d, &[id], l);
    let mut s = open(&d);
    add_laser(&mut s);
    if let Some(r) = role {
        s.set_layer_role(&kt(l), Some(r));
    }
    let list = s.frame_draw_list();
    (s, list)
}

#[test]
fn ac8_an_image_under_engrave_is_a_grey_preview() {
    let (_, list) = image_session(Some(Role::Engrave));
    assert_eq!(list.images.len(), 1, "the image is drawn");
    assert!(list.images[0].grey, "in grey");
}

#[test]
fn ac8_an_image_under_cut_ignore_or_no_role_is_its_frame_only() {
    for (role, c) in [
        (Some(Role::Cut), ROLE_CUT),
        (Some(Role::Ignore), ROLE_IGNORE),
        (None, ROLE_IGNORE),
    ] {
        let (_, list) = image_session(role);
        assert!(
            list.images.is_empty(),
            "{role:?}: the pixels are not shown: {:?}",
            list.images.len()
        );
        assert!(
            in_box(&list, c, (19.0, 19.0, 41.0, 41.0)) > 0,
            "{role:?}: the frame in that role's look"
        );
        assert!(
            !covers(&list, c, pt(30.0, 30.0)),
            "{role:?}: only the frame"
        );
    }
}

#[test]
fn ac6_off_shows_the_image_in_colour() {
    let (mut s, _) = image_session(Some(Role::Engrave));
    s.set_role_view(false);
    let list = s.frame_draw_list();
    assert_eq!(list.images.len(), 1);
    assert!(!list.images[0].grey);
}

// ---- criterion 9: the layer that holds it --------------------------------------------------

#[test]
fn ac9_a_leaf_moved_to_another_layer_takes_its_role_at_once() {
    let (d, [a, b, _]) = abc();
    let mut s = open(&d);
    let m = add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    s.set_layer_role(&kt(b), Some(Role::Engrave));
    assert_eq!(look_of_square(&s.frame_draw_list(), 20.0), "cut");
    // Move A's square into B in Vector: select it and drop onto B's row.
    s.set_workbench(Workbench::Vector);
    let square = opened(&s).children(Some(a))[0];
    let r = s.drop_layer_rows(
        &serde_json::to_string(&square).unwrap(),
        &serde_json::to_string(&b).unwrap(),
        "into",
    );
    let _ = r;
    if opened(&s).parent_node(square) != Some(b) {
        // The drop may need the row pressed first.
        let _ = s.press_layer_row(
            &serde_json::to_string(&square).unwrap(),
            curvyo_editor_wasm::RowPress::Plain,
        );
        let _ = s.drop_layer_rows(
            &serde_json::to_string(&square).unwrap(),
            &serde_json::to_string(&b).unwrap(),
            "into",
        );
    }
    assert_eq!(opened(&s).parent_node(square), Some(b), "moved");
    s.set_workbench(s.workbench_named(&m.full_id()).unwrap());
    assert_eq!(look_of_square(&s.frame_draw_list(), 20.0), "engrave");
}

// ---- criterion 12: guides and dimensions --------------------------------------------------------

#[test]
fn ac12_guides_and_dimensions_are_never_drawn_or_counted() {
    let d = Document::new(1);
    let l = new_layer(&d, "L");
    let g = rect(&d, 20.0, 20.0, 20.0, 20.0);
    d.set_guide(&[g], true).unwrap();
    let p = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 900), pt(60.0, 30.0)),
            NewAnchor::corner(AnchorId::new(1, 901), pt(80.0, 30.0)),
        ],
        false,
    );
    into(&d, &[g, p], l);
    let dim = d
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![
                PointSlot::Anchor {
                    node: p,
                    anchor: AnchorId::new(1, 900),
                },
                PointSlot::Anchor {
                    node: p,
                    anchor: AnchorId::new(1, 901),
                },
            ]),
            DimensionPlace::at_offset(Length::from_mm(8.0)),
        )
        .unwrap();
    if d.parent_node(dim) != Some(l) {
        into(&d, &[dim], l);
    }
    // Remove the measured path's own output from the picture: make it a guide too? No: delete
    // nothing; the path stays and is drawn; the dimension's text and arrows must not be.
    let mut s = open(&d);
    add_laser(&mut s);
    for role in [
        None,
        Some(Role::Cut),
        Some(Role::Engrave),
        Some(Role::Ignore),
    ] {
        if let Some(r) = role {
            s.set_layer_role(&kt(l), Some(r));
        }
        let list = s.frame_draw_list();
        assert!(
            !any_role_colour_near(&list, (19.0, 19.0, 41.0, 41.0)),
            "{role:?}: a guide is never drawn in a role look"
        );
        // The dimension sits 8 mm off the 60..80 line.
        assert!(
            !any_role_colour_near(&list, (58.0, 34.0, 82.0, 45.0))
                && !any_role_colour_near(&list, (58.0, 15.0, 82.0, 26.0)),
            "{role:?}: a dimension is never drawn in a role look"
        );
    }
    // Only guides and a path: the counted notice sees one layer (the path) when no role.
    s.set_layer_role(&kt(l), None);
    assert_eq!(notices(&s).len(), 1, "{:?}", notices(&s));
}

#[test]
fn ac12_ac15_a_layer_with_only_guides_is_not_counted() {
    let d = Document::new(1);
    let l = new_layer(&d, "Guides");
    let g = rect(&d, 20.0, 20.0, 20.0, 20.0);
    d.set_guide(&[g], true).unwrap();
    into(&d, &[g], l);
    let mut s = open(&d);
    add_laser(&mut s);
    assert!(notices(&s).is_empty(), "{:?}", notices(&s));
}

#[test]
fn ac12_guides_are_not_drawn_with_role_view_off_either() {
    let d = Document::new(1);
    let l = new_layer(&d, "L");
    let g = rect(&d, 20.0, 20.0, 20.0, 20.0);
    d.set_guide(&[g], true).unwrap();
    into(&d, &[g], l);
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_role_view(false);
    let red = RgbaColor::opaque(255, 0, 0);
    assert!(!covers(&s.frame_draw_list(), red, pt(30.0, 30.0)));
}

// ---- criterion 13: locked and opacity -----------------------------------------------------------

#[test]
fn ac13_locked_is_drawn_and_zero_opacity_is_drawn_at_full_strength() {
    let (d, [a, b, _]) = abc();
    d.set_node_locked(&[a], true).unwrap();
    let sq = d.children(Some(b))[0];
    d.set_node_opacity(&[sq], Opacity::new(0.0).unwrap())
        .unwrap();
    d.set_node_opacity(&[b], Opacity::new(0.0).unwrap())
        .unwrap();
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    s.set_layer_role(&kt(b), Some(Role::Engrave));
    let list = s.frame_draw_list();
    assert_eq!(look_of_square(&list, 20.0), "cut", "locked is output");
    assert_eq!(look_of_square(&list, 60.0), "engrave", "0 % is drawn");
    let alpha = list
        .triangles
        .iter()
        .filter(|v| same(v.color, ROLE_ENGRAVE))
        .map(|v| v.color.a)
        .min()
        .unwrap();
    assert_eq!(alpha, 255, "at full strength");
}

// ---- criterion 14: hidden ---------------------------------------------------------------------

#[test]
fn ac14_a_hidden_cut_layer_is_not_drawn_and_comes_back_when_shown() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    let m = add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    s.set_workbench(Workbench::Vector);
    let _ = s.toggle_layer_row_flag(
        &serde_json::to_string(&a).unwrap(),
        curvyo_editor_wasm::RowFlag::Visible,
    );
    s.set_workbench(s.workbench_named(&m.full_id()).unwrap());
    assert_eq!(row(&s, &kt(a))["hidden"], true, "the block marks it");
    assert_eq!(row(&s, &kt(a))["role"], "cut", "its role stays");
    assert_eq!(look_of_square(&s.frame_draw_list(), 20.0), "none");
    s.set_workbench(Workbench::Vector);
    let _ = s.toggle_layer_row_flag(
        &serde_json::to_string(&a).unwrap(),
        curvyo_editor_wasm::RowFlag::Visible,
    );
    s.set_workbench(s.workbench_named(&m.full_id()).unwrap());
    assert_eq!(row(&s, &kt(a))["hidden"], false);
    assert_eq!(look_of_square(&s.frame_draw_list(), 20.0), "cut");
}

#[test]
fn ac14_hidden_objects_and_groups_inside_a_visible_layer_are_not_drawn_in_any_role() {
    let d = Document::new(1);
    let l = new_layer(&d, "L");
    let r1 = rect(&d, 20.0, 20.0, 20.0, 20.0);
    let r2 = rect(&d, 60.0, 20.0, 20.0, 20.0);
    let r3 = rect(&d, 100.0, 20.0, 20.0, 20.0);
    let r4 = rect(&d, 60.0, 50.0, 20.0, 5.0);
    let g = d.group_objects(&[r2, r4]).unwrap();
    into(&d, &[r1, g, r3], l);
    d.set_node_visible(&[r1, g], false).unwrap();
    let mut s = open(&d);
    add_laser(&mut s);
    for role in [
        None,
        Some(Role::Cut),
        Some(Role::Engrave),
        Some(Role::Ignore),
    ] {
        s.set_layer_role(&kt(l), role);
        let list = s.frame_draw_list();
        assert_eq!(
            look_of_square(&list, 20.0),
            "none",
            "{role:?} hidden object"
        );
        assert_eq!(look_of_square(&list, 60.0), "none", "{role:?} hidden group");
        assert_ne!(look_of_square(&list, 100.0), "none", "{role:?} visible one");
    }
}

// ---- criterion 15: notices ---------------------------------------------------------------------

#[test]
fn ac15_layers_without_role_are_counted_only_with_output_capable_content() {
    let d = Document::new(1);
    let (_, _) = layer_with_square(&d, "A", 20.0);
    let (_, _) = layer_with_square(&d, "B", 60.0);
    let _empty = new_layer(&d, "Empty");
    let (h, _) = layer_with_square(&d, "HiddenOnly", 100.0);
    let only_hidden_obj = d.children(Some(h))[0];
    d.set_node_visible(&[only_hidden_obj], false).unwrap();
    let mut s = open(&d);
    add_laser(&mut s);
    assert_eq!(
        notices(&s),
        vec!["2 layers have no role and are not output.".to_owned()]
    );
}

#[test]
fn ac15_a_hidden_layer_without_role_is_not_counted() {
    let (d, [a, b, c]) = abc();
    d.set_node_visible(&[a], false).unwrap();
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_layer_role(&kt(b), Some(Role::Cut));
    s.set_layer_role(&kt(c), Some(Role::Ignore));
    assert!(notices(&s).is_empty(), "{:?}", notices(&s));
}

#[test]
fn ac15_unknown_role_and_the_outside_row_count_as_no_role() {
    let (d, [a, b, c]) = abc();
    let _loose = rect(&d, 200.0, 20.0, 20.0, 20.0);
    let mut s = open(&d);
    let m = add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    s.set_layer_role(&kt(b), Some(Role::Cut));
    s.set_layer_role(&kt(c), Some(Role::Cut));
    // Write "score" for C through the document command, as a newer build would.
    let d2 = opened(&s);
    d2.set_process_register(
        m,
        &format!("x.curvyo.laser.role.{}", kt(c)),
        Some(&Value::from("score")),
        "set_role",
    )
    .unwrap();
    let mut s = open(&d2);
    s.set_workbench(s.workbench_named(&m.full_id()).unwrap());
    assert_eq!(
        notices(&s),
        vec!["2 layers have no role and are not output.".to_owned()],
        "C (unknown) and the outside row"
    );
}

#[test]
fn ac15_an_image_in_a_cut_layer_is_counted_and_an_image_alone_counts_as_content() {
    let (s, _) = image_session(Some(Role::Cut));
    assert_eq!(
        notices(&s),
        vec!["1 image in a Cut layer is not output.".to_owned()]
    );
    let (s, _) = image_session(None);
    assert_eq!(
        notices(&s),
        vec!["1 layer has no role and is not output.".to_owned()],
        "an image is output-capable"
    );
    let (s, _) = image_session(Some(Role::Engrave));
    assert!(notices(&s).is_empty());
}

// ---- criterion 16 and 17: live follow, deleted layers -------------------------------------------

#[test]
fn ac16_the_block_follows_new_renamed_reordered_and_deleted_layers() {
    let (d, [a, b, _]) = abc();
    let mut s = open(&d);
    let m = add_laser(&mut s);
    let back = |s: &mut Session| s.set_workbench(s.workbench_named(&m.full_id()).unwrap());
    s.set_layer_role(&kt(a), Some(Role::Cut));
    s.set_workbench(Workbench::Vector);
    let _ = s.new_layer_row();
    let fresh = s.active_layer().unwrap();
    let _ = s.rename_layer_row(&serde_json::to_string(&b).unwrap(), "Bee");
    back(&mut s);
    assert!(row(&s, &kt(fresh))["role"].is_null(), "new layer: No role");
    assert_eq!(row(&s, &kt(b))["name"], "Bee", "renamed");
    let before: Vec<String> = rows(&s)
        .iter()
        .map(|r| r["key"].as_str().unwrap().to_owned())
        .collect();
    s.set_workbench(Workbench::Vector);
    let _ = s.reorder_layer_rows(
        &serde_json::to_string(&a).unwrap(),
        curvyo_ui_core::Reorder::Front,
    );
    back(&mut s);
    let after: Vec<String> = rows(&s)
        .iter()
        .map(|r| r["key"].as_str().unwrap().to_owned())
        .collect();
    assert_ne!(before, after, "a reordered layer moves");
    assert_eq!(after[0], kt(a), "A is front-most now");
    s.set_workbench(Workbench::Vector);
    let _ = s.delete_layer_row(&serde_json::to_string(&a).unwrap());
    back(&mut s);
    assert!(
        rows(&s).iter().all(|r| r["key"] != kt(a).as_str()),
        "the deleted layer's row disappears"
    );
    assert!(
        stored_roles(&s)
            .iter()
            .any(|(k, v)| k.ends_with(&kt(a)) && v == "cut"),
        "its role entry stays in the file"
    );
    let _ = s.undo();
    assert_eq!(
        row(&s, &kt(a))["role"],
        "cut",
        "undo brings it back with role"
    );
}

#[test]
fn ac17_a_stale_entry_is_shown_nowhere_and_saved_back_unchanged() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    let m = add_laser(&mut s);
    let d2 = opened(&s);
    d2.set_process_register(
        m,
        "x.curvyo.laser.role.77:4242",
        Some(&Value::from("cut")),
        "set_role",
    )
    .unwrap();
    let mut s = open(&d2);
    s.set_workbench(s.workbench_named(&m.full_id()).unwrap());
    assert!(rows(&s).iter().all(|r| r["key"] != "77:4242"));
    assert_eq!(rows(&s).len(), 3);
    assert!(notices(&s).len() <= 1);
    s.set_layer_role(&kt(a), Some(Role::Engrave));
    assert!(
        stored_roles(&s)
            .iter()
            .any(|(k, v)| k.ends_with("77:4242") && v == "cut"),
        "kept after another write"
    );
}

// ---- criterion 18: unknown values -------------------------------------------------------------

#[test]
fn ac18_an_unknown_value_reads_as_no_role_is_flagged_and_a_press_replaces_it() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    let m = add_laser(&mut s);
    let d2 = opened(&s);
    let k = format!("x.curvyo.laser.role.{}", kt(a));
    d2.set_process_register(m, &k, Some(&Value::from(3)), "set_role")
        .unwrap();
    let mut s = open(&d2);
    s.set_workbench(s.workbench_named(&m.full_id()).unwrap());
    let r = row(&s, &kt(a));
    assert!(r["role"].is_null(), "{r}");
    assert_eq!(r["unknown"], true);
    assert_eq!(
        look_of_square(&s.frame_draw_list(), 20.0),
        "ignore",
        "drawn as not output"
    );
    assert!(
        stored_roles(&s).iter().any(|(kk, v)| *kk == k && v == 3),
        "never rewritten on read"
    );
    let steps = s.undo_len();
    assert!(s.set_layer_role(&kt(a), Some(Role::Cut)));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(row(&s, &kt(a))["unknown"], false);
    assert_eq!(row(&s, &kt(a))["role"], "cut");
}

#[test]
fn ac18_clearing_an_unknown_value_is_a_step() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    let m = add_laser(&mut s);
    let d2 = opened(&s);
    let k = format!("x.curvyo.laser.role.{}", kt(a));
    d2.set_process_register(m, &k, Some(&Value::from("score")), "set_role")
        .unwrap();
    let mut s = open(&d2);
    s.set_workbench(s.workbench_named(&m.full_id()).unwrap());
    let steps = s.undo_len();
    assert!(
        s.set_layer_role(&kt(a), None),
        "a clear of an unknown value removes it"
    );
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(row(&s, &kt(a))["unknown"], false);
}

// ---- criterion 19: golden through the session, extension off -------------------------------------

#[test]
fn ac19_the_golden_opens_with_its_roles_and_looks_the_same_after_save() {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures/laser_roles_v36.curvyo"),
    )
    .unwrap();
    let mut s = Session::open(2702, &bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    let m = opened(&s).manufacturings()[0].id;
    s.set_workbench(s.workbench_named(&m.full_id()).unwrap());
    let by_name: std::collections::HashMap<String, Value> = rows(&s)
        .into_iter()
        .map(|r| (r["name"].as_str().unwrap().to_owned(), r))
        .collect();
    assert_eq!(by_name["A"]["role"], "cut");
    assert_eq!(by_name["B"]["role"], "engrave");
    assert_eq!(by_name["C"]["role"], "ignore");
    assert!(by_name["D"]["role"].is_null());
    assert_eq!(by_name["D"]["unknown"], false);
    let unknown: Vec<_> = by_name.values().filter(|r| r["unknown"] == true).collect();
    assert_eq!(unknown.len(), 1, "one unknown role");
    let frame = s.frame_draw_list();
    let mut r = Session::open(2703, &s.pack("0.1.0").unwrap()).unwrap();
    r.resize_viewport(1600.0, 1000.0);
    r.set_workbench(r.workbench_named(&m.full_id()).unwrap());
    assert_eq!(rows(&r), rows(&s));
    assert_eq!(stored_roles(&r), stored_roles(&s));
    let again = r.frame_draw_list();
    for c in [ROLE_CUT, ROLE_ENGRAVE, ROLE_IGNORE] {
        assert_eq!(tris(&frame, c), tris(&again, c), "the page looks the same");
    }
}

#[test]
fn ac19_with_the_laser_extension_off_roles_are_kept_and_none_written() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    let m = add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    let before = stored_roles(&s);
    let _ = s.set_extension_enabled(LASER, false);
    let steps = s.undo_len();
    assert!(!s.set_layer_role(&kt(a), Some(Role::Engrave)));
    assert!(!s.set_layer_role(&kt(a), None));
    assert_eq!(s.undo_len(), steps);
    let mut r = Session::open(2704, &s.pack("0.1.0").unwrap()).unwrap();
    r.load_extension_settings("disabled = [\"curvyo.laser\"]\n");
    assert_eq!(stored_roles(&r), before, "kept in the file");
    let _ = m;
}

// ---- criterion 20: delete the manufacturing ----------------------------------------------------

#[test]
fn ac20_deleting_the_manufacturing_takes_its_roles_and_undo_restores_them() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    let m = add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    let before = stored_roles(&s);
    assert!(s.delete_manufacturing(m));
    assert!(stored_roles(&s).is_empty());
    let _ = s.undo();
    assert_eq!(stored_roles(&s), before);
}

// ---- criterion 22 and 23: read-only page, Vector unchanged ---------------------------------------

#[test]
fn ac22_clicking_a_placed_object_selects_nothing_and_changes_nothing() {
    let (d, [a, _, _]) = abc();
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    let steps = s.undo_len();
    let roles = stored_roles(&s);
    let p = pt(30.0, 30.0);
    s.pointer_down(p, false);
    s.pointer_hover(pt(60.0, 60.0), false, false);
    s.pointer_up(pt(60.0, 60.0), false, false);
    assert!(s.style_scope().ids.is_empty(), "nothing selected");
    assert_eq!(s.undo_len(), steps);
    assert_eq!(stored_roles(&s), roles);
}

#[test]
fn ac23_the_vector_workbench_draws_and_exports_the_same_with_or_without_roles() {
    let (d, [a, b, c]) = abc();
    let plain = open(&d);
    let plain_list = plain.frame_draw_list();
    let plain_svg = svg(&plain);
    let mut s = open(&d);
    add_laser(&mut s);
    s.set_layer_role(&kt(a), Some(Role::Cut));
    s.set_layer_role(&kt(b), Some(Role::Engrave));
    s.set_layer_role(&kt(c), Some(Role::Ignore));
    s.set_workbench(Workbench::Vector);
    let list = s.frame_draw_list();
    for col in [ROLE_CUT, ROLE_ENGRAVE, ROLE_IGNORE] {
        assert!(tris(&list, col).is_empty(), "no role look in Vector");
    }
    let red = RgbaColor::opaque(255, 0, 0);
    assert_eq!(tris(&list, red), tris(&plain_list, red));
    assert_eq!(svg(&s), plain_svg, "Export is the same");
    let v: Value = serde_json::from_str(&s.layers_view_json(0, 500)).unwrap();
    assert!(
        !v.to_string().to_lowercase().contains("engrave"),
        "the Layers tab shows no role"
    );
    // Undo keys still work from here and never touch the drawing's own steps.
    let _ = key_with(&mut s, "z", true, false);
}

// ---- criterion 10: one layer over several pages --------------------------------------------------

#[test]
fn ac10_the_role_of_a_layer_applies_to_its_objects_on_every_page() {
    let d = Document::new(1);
    let second = d.add_page();
    let o = second.origin;
    let l = new_layer(&d, "Shared");
    let r1 = rect(&d, 20.0, 20.0, 20.0, 20.0);
    let r2 = rect(&d, o.x + 60.0, o.y + 20.0, 20.0, 20.0);
    into(&d, &[r1, r2], l);
    let mut s = open(&d);
    let m1 = add_laser(&mut s);
    s.set_layer_role(&kt(l), Some(Role::Engrave));
    assert_eq!(
        look_of_square(&s.frame_draw_list(), 20.0),
        "engrave",
        "page 1"
    );
    s.set_workbench(Workbench::Vector);
    assert!(s.activate_page(&second.id));
    let m2 = add_laser(&mut s);
    assert_ne!(m1, m2);
    s.set_layer_role(&kt(l), Some(Role::Engrave));
    let list = s.frame_draw_list();
    assert_eq!(look_of_square(&list, 60.0), "engrave", "page 2, page-local");
    assert_eq!(
        look_of_square(&list, 20.0),
        "none",
        "page 1 is not placed here"
    );
}
