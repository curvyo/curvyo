//! Independent acceptance tests for `specs/0099-sketch-pages` through the public session: objects
//! over a sketch page (criteria 1 to 3), the fixed sketch (4 to 6), the page-row drag (8 to 10),
//! Place clone on (11), the usage count and Select clones (12), Select original and double-click
//! (13), the row state (14), Convert to paths (15), Delete (16, 17), SVG export (18) and the
//! history names of the three new steps.
#![allow(
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AnchorSnapshot, Document, DocumentSize, NewOutputPage, NodeId, ObjectSnapshot, Page,
    PageKind, Point, RectBounds, Style, Vec2, pack, unpack,
};
use curvyo_editor_wasm::{
    DropBadge, KeyInput, RowPress, Session, SketchTool, SvgExportResult, Tool, Workbench,
};
use curvyo_sketch_core as sk;
use curvyo_ui_core::SvgExportOptions;
use serde_json::Value;

const TOL: f64 = 1e-6;
const CURVE_TOL: f64 = 0.01;

// ---- fixture ------------------------------------------------------------------------------------

struct Fx {
    s: Session,
    first: Page,
    second: Page,
    laser: Page,
    sketch_page: Page,
    sketch: NodeId,
    /// A drawing rectangle wholly over the sketch page.
    over: NodeId,
}

fn closed_box() -> sk::Sketch {
    sk::create_polyline(
        &sk::Sketch::new("origin".into()),
        "edge",
        &[
            sk::Position::mm(-20.0, -15.0),
            sk::Position::mm(20.0, -15.0),
            sk::Position::mm(20.0, 15.0),
            sk::Position::mm(-20.0, 15.0),
        ],
        true,
    )
    .unwrap()
}

fn find_page(d: &Document, id: &str) -> Page {
    d.pages().into_iter().find(|p| p.id == id).unwrap()
}

fn centre(p: &Page) -> Point {
    Point::new(
        p.origin.x + p.size.width.as_mm() / 2.0,
        p.origin.y + p.size.height.as_mm() / 2.0,
    )
}

fn open_session(d: &Document) -> Session {
    let mut s = Session::open(7, &pack(d, "0099-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    let mut snap = s.snapping();
    snap.enabled = false;
    s.set_snapping(snap);
    s.set_tool(Tool::Select);
    s
}

/// first (drawing), second (drawing), a laser page, then "Bracket" (sketch) with a closed box
/// and a rectangle lying over the sketch page; "first" is active.
fn fixture_with(sketch: Option<&sk::Sketch>) -> Fx {
    let d = Document::new(1);
    let second = d.add_page_of(&PageKind::Drawing).unwrap();
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    d.rename_page(&sp.id, "Bracket").unwrap();
    let sketch = sketch.map(|g| d.create_page_sketch(&sp.id, g, &Style::default()).unwrap());
    let sp = find_page(&d, &sp.id);
    let over = d.create_rect(RectBounds::from_corners(
        Point::new(sp.origin.x + 5.0, sp.origin.y + 5.0),
        Point::new(sp.origin.x + 25.0, sp.origin.y + 15.0),
    ));
    // Without a sketch the field names the rectangle; no test of an empty page reads it.
    let sketch = sketch.unwrap_or(over);
    let first = find_page(&d, "first");
    let second = find_page(&d, &second.id);
    let laser = find_page(&d, &laser.id);
    let mut s = open_session(&d);
    s.activate_page("first");
    s.wipe_history();
    Fx {
        s,
        first,
        second,
        laser,
        sketch_page: sp,
        sketch,
        over,
    }
}

fn fixture() -> Fx {
    fixture_with(Some(&closed_box()))
}

// ---- helpers ------------------------------------------------------------------------------------

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn rows(s: &Session) -> Vec<Value> {
    serde_json::from_str(&s.pages_json()).unwrap()
}

fn row(s: &Session, id: &str) -> Value {
    rows(s).into_iter().find(|r| r["id"] == id).unwrap()
}

fn last_step_name(s: &mut Session) -> String {
    let rows: Vec<Value> = serde_json::from_str(&s.history_rows_json(0, 1)).unwrap();
    rows[0]["name"].as_str().unwrap().to_owned()
}

fn key(s: &mut Session, k: &str, ctrl: bool) {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    });
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn shift_click(s: &mut Session, at: Point) {
    s.modifiers_changed(true, false, false);
    s.pointer_hover(at, true, false);
    s.pointer_down(at, true);
    s.pointer_up(at, true, false);
    s.modifiers_changed(false, false, false);
}

fn drag(s: &mut Session, from: Point, to: Point) {
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    let mid = Point::new(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y));
    s.pointer_hover(mid, false, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

fn layer_rows(s: &mut Session) -> Vec<Value> {
    let v: Value = serde_json::from_str(&s.layers_view_json(0, 500)).unwrap();
    v["rows"].as_array().unwrap().clone()
}

fn selected(s: &mut Session) -> Vec<NodeId> {
    layer_rows(s)
        .into_iter()
        .filter(|r| r["mark"] == "selected")
        .filter_map(|r| serde_json::from_value(r["id"].clone()).ok())
        .collect()
}

fn outline(o: &ObjectSnapshot) -> Vec<AnchorSnapshot> {
    match o {
        ObjectSnapshot::Clone(c) => c.resolved.all_anchors().copied().collect(),
        ObjectSnapshot::Sketch(s) => s.resolved.all_anchors().copied().collect(),
        ObjectSnapshot::Path(p) => p.all_anchors().copied().collect(),
        other => panic!("no outline for {other:?}"),
    }
}

fn outline_of(d: &Document, id: NodeId) -> Vec<AnchorSnapshot> {
    outline(&d.object(id).unwrap())
}

fn same_outline(a: &[AnchorSnapshot], b: &[AnchorSnapshot], tol: f64) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(p, q)| {
            (p.point.x - q.point.x).abs() < tol && (p.point.y - q.point.y).abs() < tol
        })
}

fn box_centre(anchors: &[AnchorSnapshot]) -> Point {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for a in anchors {
        x0 = x0.min(a.point.x);
        y0 = y0.min(a.point.y);
        x1 = x1.max(a.point.x);
        y1 = y1.max(a.point.y);
    }
    Point::new(f64::midpoint(x0, x1), f64::midpoint(y0, y1))
}

fn clone_ids(d: &Document, root: NodeId) -> Vec<NodeId> {
    d.read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Clone(c) if c.chain.root == root => Some(c.resolved.id),
            _ => None,
        })
        .collect()
}

fn ids(d: &Document) -> Vec<NodeId> {
    d.read_objects()
        .objects
        .iter()
        .map(ObjectSnapshot::id)
        .collect()
}

fn sketch_selection(s: &Session) -> Vec<String> {
    let v: Value = serde_json::from_str(&s.sketch_view_json()).unwrap();
    v["entities"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|e| e["selected"] == true)
                .map(|e| e["id"].as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn place_by_drag(s: &mut Session, page: &str, at: Point) -> bool {
    assert!(s.row_drag_begin(page));
    s.row_drag_move(None);
    s.row_drag_move(Some(at));
    s.row_drag_drop()
}

// ---- criterion 1 / 2: objects over a sketch page --------------------------------------------

#[test]
fn ac2_a_press_on_an_object_over_a_sketch_page_selects_it_and_keeps_the_drawing_page() {
    let mut f = fixture();
    // On the rectangle's top edge (it has no fill).
    let at = Point::new(f.sketch_page.origin.x + 15.0, f.sketch_page.origin.y + 5.0);
    click(&mut f.s, at);
    assert_eq!(
        f.s.active_page().id,
        "first",
        "the drawing page stays active"
    );
    assert_eq!(f.s.workbench(), Workbench::Vector);
    assert_eq!(f.s.selected_object_count(), 1);
    assert_eq!(selected(&mut f.s), vec![f.over]);
    // It moves as a pasteboard object.
    let before = outline_points(&doc(&f.s), f.over);
    key(&mut f.s, "ArrowRight", false);
    let after = outline_points(&doc(&f.s), f.over);
    assert!(after[0].x > before[0].x, "the arrow key moves it");
    assert_eq!(f.s.active_page().id, "first");
}

fn outline_points(d: &Document, id: NodeId) -> Vec<Point> {
    match d.object(id).unwrap() {
        ObjectSnapshot::Primitive(p) => curvyo_document_core::outline_of(&p.shape)
            .anchors
            .iter()
            .map(|a| a.point)
            .collect(),
        o => outline(&o).iter().map(|a| a.point).collect(),
    }
}

#[test]
fn ac2_shift_press_and_a_pasteboard_marquee_select_objects_over_a_sketch_page() {
    let mut f = fixture();
    let sp = f.sketch_page.clone();
    let mut d = doc(&f.s);
    let other = d.create_rect(RectBounds::from_corners(
        Point::new(sp.origin.x + 40.0, sp.origin.y + 40.0),
        Point::new(sp.origin.x + 50.0, sp.origin.y + 50.0),
    ));
    f.s = open_session(&d);
    f.s.activate_page("first");
    let r = d.object(f.over).unwrap();
    let _ = r;
    click(&mut f.s, Point::new(sp.origin.x + 15.0, sp.origin.y + 5.0));
    shift_click(&mut f.s, Point::new(sp.origin.x + 45.0, sp.origin.y + 40.0));
    assert_eq!(f.s.active_page().id, "first");
    let mut sel = selected(&mut f.s);
    sel.sort_by_key(|n| n.full_id());
    let mut want = vec![f.over, other];
    want.sort_by_key(|n| n.full_id());
    assert_eq!(sel, want, "shift+press adds");
    // A marquee from the pasteboard above the sketch page over both objects (and the sketch).
    key(&mut f.s, "Escape", false);
    f.s.escape();
    f.s.activate_page("first");
    let from = Point::new(sp.origin.x - 5.0, sp.origin.y - 30.0);
    let to = Point::new(
        sp.origin.x + sp.size.width.as_mm() + 5.0,
        sp.origin.y + sp.size.height.as_mm() + 5.0,
    );
    drag(&mut f.s, from, to);
    let mut sel = selected(&mut f.s);
    sel.sort_by_key(|n| n.full_id());
    assert_eq!(sel, want, "the marquee takes the objects, never the sketch");
    assert!(!sel.contains(&f.sketch));
    d = doc(&f.s);
    assert!(d.sketch(f.sketch).is_some());
}

#[test]
fn ac2_a_press_inside_the_sketch_frame_on_nothing_activates_the_sketch_page() {
    let mut f = fixture();
    let sp = f.sketch_page.clone();
    let steps = f.s.undo_len();
    click(
        &mut f.s,
        Point::new(
            sp.origin.x + 5.0,
            sp.origin.y + sp.size.height.as_mm() - 5.0,
        ),
    );
    assert_eq!(f.s.active_page().id, sp.id);
    assert_eq!(f.s.workbench(), Workbench::Sketch);
    assert_eq!(f.s.undo_len(), steps, "activation is no step");
}

// ---- criterion 3: the sketch tools never touch drawing objects -----------------------------

#[test]
fn ac3_sketch_select_all_delete_and_presses_act_on_sketch_geometry_only() {
    let mut f = fixture();
    f.s.activate_page(&f.sketch_page.id);
    assert_eq!(f.s.workbench(), Workbench::Sketch);
    f.s.set_sketch_tool(SketchTool::Select);
    let over_before = doc(&f.s).object(f.over).unwrap();
    // Amended by `specs/0106` criterion 3: a press on the drawing object off sketch geometry
    // selects it move-only, and a drag moves it; the sketch page stays active.
    let on_rect = Point::new(f.sketch_page.origin.x + 5.0, f.sketch_page.origin.y + 10.0);
    click(&mut f.s, on_rect);
    assert!(
        selected(&mut f.s).contains(&f.over),
        "the drawing object is selected move-only"
    );
    assert!(sketch_selection(&f.s).is_empty());
    drag(
        &mut f.s,
        on_rect,
        Point::new(on_rect.x + 30.0, on_rect.y + 30.0),
    );
    assert_ne!(
        doc(&f.s).object(f.over).unwrap(),
        over_before,
        "a drag moves it"
    );
    let over_before = doc(&f.s).object(f.over).unwrap();
    // Ctrl+A and Delete act on the sketch only.
    key(&mut f.s, "a", true);
    assert!(
        !selected(&mut f.s).contains(&f.over),
        "no drawing object selected"
    );
    assert!(
        !sketch_selection(&f.s).is_empty(),
        "sketch geometry selected"
    );
    key(&mut f.s, "Delete", false);
    let d = doc(&f.s);
    assert_eq!(d.object(f.over).unwrap(), over_before, "Delete keeps it");
    assert_eq!(f.s.active_page().id, f.sketch_page.id);
}

// ---- criterion 4: the sketch is never a whole object ----------------------------------------

#[test]
fn ac4_select_all_and_marquee_on_drawing_and_laser_pages_never_include_the_sketch() {
    let mut f = fixture();
    for page in ["first".to_owned(), f.laser.id.clone()] {
        f.s.activate_page(&page);
        key(&mut f.s, "a", true);
        assert!(!selected(&mut f.s).contains(&f.sketch), "{page}: Ctrl+A");
        f.s.escape();
    }
    f.s.activate_page("first");
    let sp = f.sketch_page.clone();
    drag(
        &mut f.s,
        Point::new(sp.origin.x - 10.0, sp.origin.y - 40.0),
        Point::new(
            sp.origin.x + sp.size.width.as_mm() + 10.0,
            sp.origin.y + sp.size.height.as_mm() + 10.0,
        ),
    );
    assert!(!selected(&mut f.s).contains(&f.sketch));
}

#[test]
fn ac4_a_press_on_the_sketch_outline_outside_its_frame_never_selects_or_moves_it() {
    // An edge running from the frame out onto the pasteboard above it.
    let reach = sk::create_polyline(
        &sk::Sketch::new("origin".into()),
        "out",
        &[sk::Position::mm(0.0, 0.0), sk::Position::mm(0.0, -400.0)],
        false,
    )
    .unwrap();
    let mut f = fixture_with(Some(&reach));
    let d = doc(&f.s);
    let a = outline_of(&d, f.sketch);
    let top = if a[0].point.y < a[1].point.y {
        a[0].point
    } else {
        a[1].point
    };
    let at = Point::new(top.x, top.y + 20.0);
    assert!(at.y < f.sketch_page.origin.y, "fixture: on the pasteboard");
    let placement = d.sketch(f.sketch).unwrap().placement;
    f.s.activate_page(&f.second.id);
    click(&mut f.s, at);
    assert_eq!(
        f.s.active_page().id,
        f.second.id,
        "a pasteboard press keeps the page"
    );
    assert!(selected(&mut f.s).is_empty(), "the sketch is not picked");
    drag(&mut f.s, at, Point::new(at.x + 25.0, at.y + 10.0));
    assert_eq!(doc(&f.s).sketch(f.sketch).unwrap().placement, placement);
    assert!(selected(&mut f.s).is_empty());
}

#[test]
fn ac4_arrow_keys_never_move_the_sketch() {
    let mut f = fixture();
    let placement = doc(&f.s).sketch(f.sketch).unwrap().placement;
    for page in ["first".to_owned(), f.sketch_page.id.clone()] {
        f.s.activate_page(&page);
        key(&mut f.s, "a", true);
        key(&mut f.s, "ArrowRight", false);
        key(&mut f.s, "ArrowDown", false);
        f.s.escape();
    }
    let after = doc(&f.s).sketch(f.sketch).unwrap().placement;
    assert!(
        after.approx_eq(placement, TOL),
        "{placement:?} -> {after:?}"
    );
}

#[test]
fn ac4_the_sketch_layers_row_activates_the_sketch_page_and_selects_nothing() {
    let mut f = fixture();
    let want = serde_json::to_value(f.sketch).unwrap();
    let Some(r) = layer_rows(&mut f.s).into_iter().find(|r| r["id"] == want) else {
        // No Layers row for the sketch on a drawing page: it cannot be selected from there.
        return;
    };
    let id = r["id"].to_string();
    let steps = f.s.undo_len();
    f.s.press_layer_row(&id, RowPress::Plain);
    assert_eq!(
        f.s.active_page().id,
        f.sketch_page.id,
        "activates its sketch page"
    );
    assert_eq!(f.s.workbench(), Workbench::Sketch);
    // Like Select original (criterion 13): no object is selected, no Layers row is marked.
    assert!(selected(&mut f.s).is_empty(), "selects nothing");
    assert_eq!(f.s.selected_object_count(), 0);
    assert!(sketch_selection(&f.s).is_empty());
    assert_eq!(f.s.undo_len(), steps);
}

// ---- criterion 5: the frame move ------------------------------------------------------------

#[test]
fn ac5_the_frame_edge_drag_moves_the_sketch_holds_clones_and_leaves_foreign_objects() {
    let mut f = fixture();
    // A clone without own placement on the first page, held back nevertheless.
    let d = doc(&f.s);
    let plain = d.linked_clone(&[f.sketch], &[]).unwrap()[0];
    d.translate_objects(
        &[plain],
        Vec2::new(f.first.origin.x - f.sketch_page.origin.x, 0.0),
    )
    .unwrap();
    f.s = open_session(&d);
    f.s.activate_page("first");
    let before = doc(&f.s);
    let sketch_before = outline_of(&before, f.sketch);
    let plain_before = outline_of(&before, plain);
    let over_before = before.object(f.over).unwrap();
    let sp = f.sketch_page.clone();
    let start = Point::new(sp.origin.x - 0.5, sp.origin.y + 100.0);
    let end = Point::new(start.x + 30.0, start.y + 20.0);
    let steps = f.s.undo_len();
    drag(&mut f.s, start, end);
    assert_eq!(f.s.undo_len(), steps + 1, "one step");
    assert_eq!(last_step_name(&mut f.s), "Move page");
    let after = doc(&f.s);
    let p = find_page(&after, &sp.id);
    let delta = Vec2::new(p.origin.x - sp.origin.x, p.origin.y - sp.origin.y);
    assert!(delta.x.abs() > 1.0, "the frame moved: {delta:?}");
    let moved: Vec<_> = sketch_before
        .iter()
        .copied()
        .map(|mut a| {
            a.point = Point::new(a.point.x + delta.x, a.point.y + delta.y);
            a
        })
        .collect();
    assert!(
        same_outline(&outline_of(&after, f.sketch), &moved, TOL),
        "sketch moved"
    );
    assert!(
        same_outline(&outline_of(&after, plain), &plain_before, TOL),
        "clone held"
    );
    assert_eq!(
        after.object(f.over).unwrap(),
        over_before,
        "object over the page stays"
    );
    f.s.undo();
    assert!(same_outline(
        &outline_of(&doc(&f.s), f.sketch),
        &sketch_before,
        TOL
    ));
}

#[test]
fn ac5_the_frame_alone_switch_never_applies_to_a_sketch_page() {
    let mut f = fixture();
    let sp = f.sketch_page.clone();
    assert_ne!(row(&f.s, &sp.id)["frameOnly"], true);
    f.s.set_page_frame_only(&sp.id, true);
    assert_ne!(
        row(&f.s, &sp.id)["frameOnly"],
        true,
        "not shown on a sketch row"
    );
    let before = outline_of(&doc(&f.s), f.sketch);
    f.s.move_page_to(&sp.id, Point::new(sp.origin.x + 50.0, sp.origin.y));
    let after = outline_of(&doc(&f.s), f.sketch);
    assert!(
        (after[0].point.x - before[0].point.x - 50.0).abs() < TOL,
        "the sketch moves"
    );
    // Moving the frame again moves frame and sketch together.
    f.s.activate_page(&sp.id);
    let x = sp.origin.x + 120.0;
    assert!(f.s.move_page_to(&sp.id, Point::new(x, sp.origin.y)));
    let again = outline_of(&doc(&f.s), f.sketch);
    let p = find_page(&doc(&f.s), &sp.id);
    assert!((again[0].point.x - before[0].point.x - (p.origin.x - sp.origin.x)).abs() < TOL);
}

// ---- criterion 6: resize --------------------------------------------------------------------

#[test]
fn ac6_resize_keeps_the_sketch_in_place_and_is_one_step() {
    let mut f = fixture();
    f.s.activate_page(&f.sketch_page.id);
    let placement = doc(&f.s).sketch(f.sketch).unwrap().placement;
    let steps = f.s.undo_len();
    f.s.set_document_side(curvyo_editor_wasm::DocumentSide::Width, "333");
    assert_eq!(f.s.undo_len(), steps + 1);
    let d = doc(&f.s);
    assert!((find_page(&d, &f.sketch_page.id).size.width.as_mm() - 333.0).abs() < TOL);
    assert_eq!(d.sketch(f.sketch).unwrap().placement, placement);
}

// ---- criteria 8 to 10: the row drag ---------------------------------------------------------

#[test]
fn ac8_a_row_drag_onto_a_drawing_page_places_a_selected_clone_with_its_centre_under_the_pointer() {
    let mut f = fixture();
    let before = doc(&f.s);
    let objects_before = ids(&before);
    let sketch_before = before.sketch(f.sketch).unwrap();
    let pages_before = before.pages();
    let version = f.s.pack("x").unwrap();
    let steps = f.s.undo_len();
    let at = Point::new(f.second.origin.x + 70.0, f.second.origin.y + 90.0);
    assert!(f.s.row_drag_begin(&f.sketch_page.id));
    let a = f.s.row_drag_move(None);
    assert_eq!(a.badge, DropBadge::None, "over the list: no badge");
    let a = f.s.row_drag_move(Some(at));
    assert_eq!(a.badge, DropBadge::Clone);
    assert_eq!(a.target.as_deref(), Some(f.second.id.as_str()));
    assert!(!a.faint);
    assert_eq!(f.s.undo_len(), steps, "nothing written before the drop");
    assert_eq!(f.s.pack("x").unwrap(), version);
    assert!(f.s.row_drag_drop());
    assert_eq!(f.s.undo_len(), steps + 1, "one step");
    assert_eq!(last_step_name(&mut f.s), "Place sketch clone");
    let d = doc(&f.s);
    let new: Vec<_> = ids(&d)
        .into_iter()
        .filter(|i| !objects_before.contains(i))
        .collect();
    assert_eq!(new.len(), 1);
    let c = new[0];
    let ObjectSnapshot::Clone(snap) = d.object(c).unwrap() else {
        panic!("a linked clone");
    };
    assert_eq!(snap.chain.root, f.sketch);
    assert!(snap.chain.own_placement);
    let got = box_centre(&outline_of(&d, c));
    assert!(
        (got.x - at.x).abs() < TOL && (got.y - at.y).abs() < TOL,
        "{got:?} vs {at:?}"
    );
    assert_eq!(*ids(&d).last().unwrap(), c, "on top");
    assert_eq!(
        f.s.active_page().id,
        f.second.id,
        "the page under the pointer is active"
    );
    assert_eq!(f.s.workbench(), Workbench::Vector);
    assert_eq!(selected(&mut f.s), vec![c], "the clone is selected");
    let after = d.sketch(f.sketch).unwrap();
    assert_eq!(after.graph, sketch_before.graph);
    assert_eq!(after.placement, sketch_before.placement);
    assert_eq!(d.pages(), pages_before, "pages unchanged");
    // Undo removes the clone and clears the selection.
    f.s.undo();
    assert!(doc(&f.s).object(c).is_none());
    assert_eq!(f.s.selected_object_count(), 0);
}

#[test]
fn ac8_a_row_drag_onto_a_laser_page_places_the_clone_there_and_activates_it() {
    let mut f = fixture();
    let at = centre(&f.laser);
    let objects_before = ids(&doc(&f.s));
    assert!(place_by_drag(&mut f.s, &f.sketch_page.id.clone(), at));
    assert_eq!(f.s.active_page().id, f.laser.id);
    let d = doc(&f.s);
    let c = *ids(&d)
        .iter()
        .find(|i| !objects_before.contains(i))
        .unwrap();
    assert!(matches!(d.object(c), Some(ObjectSnapshot::Clone(_))));
    let got = box_centre(&outline_of(&d, c));
    assert!((got.x - at.x).abs() < TOL && (got.y - at.y).abs() < TOL);
}

#[test]
fn ac8_a_row_drag_onto_the_pasteboard_places_the_clone_and_keeps_the_active_page() {
    let mut f = fixture();
    f.s.activate_page(&f.second.id);
    let at = Point::new(-500.0, -400.0);
    let objects_before = ids(&doc(&f.s));
    assert!(f.s.row_drag_begin(&f.sketch_page.id));
    let a = f.s.row_drag_move(Some(at));
    assert_eq!(a.badge, DropBadge::Clone);
    assert_eq!(a.target, None);
    assert!(f.s.row_drag_drop());
    assert_eq!(f.s.active_page().id, f.second.id, "the active page stays");
    let d = doc(&f.s);
    let c = *ids(&d)
        .iter()
        .find(|i| !objects_before.contains(i))
        .unwrap();
    let got = box_centre(&outline_of(&d, c));
    assert!((got.x - at.x).abs() < TOL && (got.y - at.y).abs() < TOL);
    assert_eq!(selected(&mut f.s), vec![c]);
}

#[test]
fn ac8_a_drop_on_the_pasteboard_while_the_sketch_page_is_active_keeps_it_active() {
    let mut f = fixture();
    f.s.activate_page(&f.sketch_page.id);
    let objects_before = ids(&doc(&f.s));
    assert!(place_by_drag(
        &mut f.s,
        &f.sketch_page.id.clone(),
        Point::new(-600.0, -600.0)
    ));
    assert_eq!(f.s.active_page().id, f.sketch_page.id);
    assert_eq!(f.s.workbench(), Workbench::Sketch);
    let d = doc(&f.s);
    assert_eq!(ids(&d).len(), objects_before.len() + 1);
    assert_eq!(
        f.s.selected_object_count(),
        1,
        "selected (box without handles)"
    );
}

#[test]
fn ac9_a_drop_on_a_sketch_page_or_escape_writes_nothing() {
    let mut f = fixture();
    // A second sketch page with a sketch.
    let d = doc(&f.s);
    let other = d.add_page_of(&PageKind::Sketch).unwrap();
    d.create_page_sketch(&other.id, &closed_box(), &Style::default())
        .unwrap();
    f.s = open_session(&d);
    f.s.activate_page("first");
    f.s.wipe_history();
    let bytes = f.s.pack("x").unwrap();
    for target in [centre(&f.sketch_page), centre(&find_page(&d, &other.id))] {
        assert!(f.s.row_drag_begin(&f.sketch_page.id));
        f.s.row_drag_move(Some(centre(&f.second)));
        let a = f.s.row_drag_move(Some(target));
        assert_eq!(a.badge, DropBadge::NotOnSketchPage);
        assert!(a.faint);
        assert!(!f.s.row_drag_drop(), "nothing created");
        assert_eq!(f.s.undo_len(), 0);
        assert_eq!(f.s.pack("x").unwrap(), bytes);
    }
    // Escape (cancel) after a move over a drawing page.
    assert!(f.s.row_drag_begin(&f.sketch_page.id));
    let a = f.s.row_drag_move(Some(centre(&f.second)));
    assert_eq!(a.badge, DropBadge::Clone);
    f.s.row_drag_cancel();
    assert_eq!(f.s.undo_len(), 0);
    assert_eq!(f.s.pack("x").unwrap(), bytes);
    assert!(!f.s.row_drag_drop(), "a drop after cancel does nothing");
    assert_eq!(f.s.active_page().id, "first");
}

#[test]
fn ac8_a_release_over_the_list_places_nothing() {
    let mut f = fixture();
    assert!(f.s.row_drag_begin(&f.sketch_page.id));
    f.s.row_drag_move(Some(centre(&f.second)));
    f.s.row_drag_move(None);
    assert!(!f.s.row_drag_drop());
    assert_eq!(f.s.undo_len(), 0);
}

fn only_parameters() -> sk::Sketch {
    let mut s = sk::Sketch::new("origin".into());
    s.parameters.insert(
        "width".into(),
        sk::Parameter::literal("width", sk::Quantity::length(80.0).unwrap()),
    );
    s
}

fn only_construction() -> sk::Sketch {
    let mut s = closed_box();
    let all: Vec<_> = s.entities.keys().cloned().collect();
    s.construction.extend(all);
    s
}

#[test]
fn ac10_an_empty_or_undrawable_sketch_places_nothing_and_reads_no_sketch_yet() {
    for (name, g) in [
        ("lazy", None),
        ("parameters only", Some(only_parameters())),
        ("construction only", Some(only_construction())),
    ] {
        let mut f = fixture_with(g.as_ref());
        let bytes = f.s.pack("x").unwrap();
        if f.s.row_drag_begin(&f.sketch_page.id) {
            let a = f.s.row_drag_move(Some(centre(&f.second)));
            assert_eq!(a.badge, DropBadge::NoSketch, "{name}");
            assert!(!f.s.row_drag_drop(), "{name}: nothing created");
        }
        assert_eq!(f.s.undo_len(), 0, "{name}");
        assert_eq!(f.s.pack("x").unwrap(), bytes, "{name}");
        assert_eq!(
            row(&f.s, &f.sketch_page.id)["sketch"]["drawable"],
            false,
            "{name}"
        );
        // Place clone on is hidden: the command places nothing either.
        f.s.activate_page(&f.sketch_page.id);
        assert!(!f.s.place_sketch_clone_on(&f.second.id), "{name}");
        assert_eq!(f.s.undo_len(), 0, "{name}");
    }
}

#[test]
fn ac10_the_row_state_empty_follows_0098_no_geometry_and_no_parameter() {
    let f = fixture_with(None);
    assert_eq!(
        row(&f.s, &f.sketch_page.id)["sketch"]["state"]["state"],
        "empty"
    );
    let f = fixture_with(Some(&only_parameters()));
    assert_ne!(
        row(&f.s, &f.sketch_page.id)["sketch"]["state"]["state"],
        "empty",
        "a parameter makes it not Empty"
    );
    let f = fixture_with(Some(&only_construction()));
    assert_ne!(
        row(&f.s, &f.sketch_page.id)["sketch"]["state"]["state"],
        "empty"
    );
}

#[test]
// Amended by `specs/0100-bring-content-in` criterion 13: a drawing page row places the page;
// a laser row still only reorders.
fn ac8_a_laser_page_row_places_nothing() {
    let mut f = fixture();
    assert!(f.s.row_drag_begin(&f.second.id));
    f.s.row_drag_cancel();
    assert!(!f.s.row_drag_begin(&f.laser.id));
    assert!(!f.s.row_drag_drop());
    assert_eq!(f.s.undo_len(), 0);
}

// ---- criterion 11: Place clone on -----------------------------------------------------------

#[test]
fn ac11_place_clone_on_puts_the_clone_at_the_page_centre_and_activates_that_page() {
    let mut f = fixture();
    f.s.activate_page(&f.sketch_page.id);
    for target in [f.second.clone(), f.laser.clone()] {
        f.s.activate_page(&f.sketch_page.id);
        let objects_before = ids(&doc(&f.s));
        let steps = f.s.undo_len();
        assert!(f.s.place_sketch_clone_on(&target.id));
        assert_eq!(f.s.undo_len(), steps + 1);
        assert_eq!(last_step_name(&mut f.s), "Place sketch clone");
        assert_eq!(f.s.active_page().id, target.id);
        let d = doc(&f.s);
        let c = *ids(&d)
            .iter()
            .find(|i| !objects_before.contains(i))
            .unwrap();
        let got = box_centre(&outline_of(&d, c));
        let want = centre(&target);
        assert!((got.x - want.x).abs() < TOL && (got.y - want.y).abs() < TOL);
        assert_eq!(selected(&mut f.s), vec![c]);
    }
    // Never onto a sketch page.
    f.s.activate_page(&f.sketch_page.id);
    let steps = f.s.undo_len();
    assert!(!f.s.place_sketch_clone_on(&f.sketch_page.id));
    assert!(!f.s.place_sketch_clone_on("no-such-page"));
    assert_eq!(f.s.undo_len(), steps);
}

// ---- criterion 12: usage and Select clones --------------------------------------------------

#[test]
fn ac12_the_usage_counts_direct_chained_and_pasteboard_clones() {
    let mut f = fixture();
    assert_eq!(f.s.sketch_usage(f.sketch).clones, 0);
    assert_eq!(row(&f.s, &f.sketch_page.id)["sketch"]["clones"], 0);
    let sp = f.sketch_page.id.clone();
    place_by_drag(&mut f.s, &sp, centre(&f.first));
    place_by_drag(
        &mut f.s,
        &sp,
        Point::new(f.second.origin.x + 40.0, f.second.origin.y + 40.0),
    );
    place_by_drag(
        &mut f.s,
        &sp,
        Point::new(f.second.origin.x + 120.0, f.second.origin.y + 150.0),
    );
    let u = f.s.sketch_usage(f.sketch);
    assert_eq!((u.clones, u.pages), (3, 2));
    place_by_drag(&mut f.s, &sp, Point::new(-700.0, -700.0));
    let u = f.s.sketch_usage(f.sketch);
    assert_eq!(
        (u.clones, u.pages),
        (4, 2),
        "pasteboard counts in the first number only"
    );
    // A chained clone (clone of a clone) counts too.
    let d = doc(&f.s);
    let first_clone = clone_ids(&d, f.sketch)[0];
    let chained = d.linked_clone(&[first_clone], &[]).unwrap()[0];
    d.translate_objects(&[chained], Vec2::new(0.0, 30.0))
        .unwrap();
    let s2 = open_session(&d);
    let u = s2.sketch_usage(f.sketch);
    assert_eq!(u.clones, 5);
    assert_eq!(row(&s2, &sp)["sketch"]["clones"], 5);
    assert_eq!(row(&s2, &sp)["sketch"]["pages"], 2);
}

#[test]
fn ac12_select_clones_activates_the_first_page_with_clones_and_selects_them_there_only() {
    let mut f = fixture();
    let sp = f.sketch_page.id.clone();
    // Two on the second page, one on the first, one on the laser page.
    place_by_drag(
        &mut f.s,
        &sp,
        Point::new(f.second.origin.x + 40.0, f.second.origin.y + 40.0),
    );
    place_by_drag(
        &mut f.s,
        &sp,
        Point::new(f.second.origin.x + 120.0, f.second.origin.y + 150.0),
    );
    place_by_drag(&mut f.s, &sp, centre(&f.first));
    place_by_drag(&mut f.s, &sp, centre(&f.laser));
    let d = doc(&f.s);
    let all = clone_ids(&d, f.sketch);
    f.s.activate_page(&sp);
    f.s.select_sketch_clones();
    assert_eq!(f.s.active_page().id, "first", "first page in list order");
    let sel = selected(&mut f.s);
    assert_eq!(sel.len(), 1, "only the clones on that page");
    assert!(all.contains(&sel[0]));
}

#[test]
fn ac12_select_clones_with_every_clone_on_the_pasteboard_keeps_the_last_drawing_page() {
    let mut f = fixture();
    let sp = f.sketch_page.id.clone();
    f.s.activate_page(&f.second.id);
    place_by_drag(&mut f.s, &sp, Point::new(-700.0, -700.0));
    place_by_drag(&mut f.s, &sp, Point::new(-900.0, -700.0));
    f.s.activate_page(&sp);
    f.s.select_sketch_clones();
    assert_eq!(
        f.s.active_page().id,
        f.second.id,
        "the last active drawing page"
    );
    assert_eq!(selected(&mut f.s).len(), 2);
}

// ---- criterion 13: Select original, double-click, Unlink -----------------------------------

#[test]
fn ac13_double_click_on_a_clone_opens_its_sketch_page_with_nothing_selected() {
    let mut f = fixture();
    let sp = f.sketch_page.id.clone();
    let at = centre(&f.second);
    place_by_drag(&mut f.s, &sp, at);
    f.s.escape();
    let on = {
        let d = doc(&f.s);
        let c = clone_ids(&d, f.sketch)[0];
        let a = outline_of(&d, c);
        Point::new(
            f64::midpoint(a[0].point.x, a[1].point.x),
            f64::midpoint(a[0].point.y, a[1].point.y),
        )
    };
    let steps = f.s.undo_len();
    f.s.pointer_down(on, false);
    f.s.pointer_up(on, false, false);
    f.s.double_click(on, false, false);
    assert_eq!(f.s.active_page().id, sp);
    assert_eq!(f.s.workbench(), Workbench::Sketch);
    assert_eq!(
        f.s.selected_object_count(),
        0,
        "the clone's selection is cleared"
    );
    assert!(
        sketch_selection(&f.s).is_empty(),
        "no sketch geometry selected"
    );
    assert_eq!(f.s.undo_len(), steps);
}

#[test]
fn ac13_select_original_on_a_sketch_clone_activates_the_sketch_page() {
    let mut f = fixture();
    let sp = f.sketch_page.id.clone();
    place_by_drag(&mut f.s, &sp, centre(&f.second));
    assert_eq!(f.s.selected_object_count(), 1);
    f.s.select_original();
    assert_eq!(f.s.active_page().id, sp);
    assert_eq!(f.s.workbench(), Workbench::Sketch);
    assert_eq!(f.s.selected_object_count(), 0);
    assert!(sketch_selection(&f.s).is_empty());
}

#[test]
fn ac13_select_original_of_clones_of_two_sketches_activates_the_first_sketch_page_in_order() {
    let f = fixture();
    let d = doc(&f.s);
    let other = d.add_page_of(&PageKind::Sketch).unwrap();
    let other_sketch = d
        .create_page_sketch(&other.id, &closed_box(), &Style::default())
        .unwrap();
    // The new sketch page goes first in the list.
    d.reorder_page(&other.id, 0).unwrap();
    // Both on the pasteboard, where Select all on a drawing page takes them (`specs/0106`
    // criterion 3 keeps Select all to the objects in context).
    let a = d
        .place_linked_clone(f.sketch, Vec2::new(0.0, -2000.0))
        .unwrap();
    let b = d
        .place_linked_clone(other_sketch, Vec2::new(0.0, -2300.0))
        .unwrap();
    let mut s = open_session(&d);
    s.activate_page("first");
    key(&mut s, "a", true);
    let sel = selected(&mut s);
    assert!(sel.contains(&a) && sel.contains(&b), "{sel:?}");
    s.select_original();
    assert_eq!(
        s.active_page().id,
        other.id,
        "first sketch page in list order"
    );
}

#[test]
fn ac13_unlink_turns_one_clone_into_paths_and_leaves_the_sketch_and_other_clones() {
    let mut f = fixture();
    let sp = f.sketch_page.id.clone();
    place_by_drag(&mut f.s, &sp, centre(&f.first));
    place_by_drag(&mut f.s, &sp, centre(&f.second));
    let d = doc(&f.s);
    let clones = clone_ids(&d, f.sketch);
    let unlinked = *selected(&mut f.s).first().unwrap();
    let kept = *clones.iter().find(|c| **c != unlinked).unwrap();
    let before = outline_of(&d, unlinked);
    let kept_before = outline_of(&d, kept);
    let graph = d.sketch(f.sketch).unwrap().graph;
    f.s.unlink_clones();
    let d = doc(&f.s);
    let o = d.object(unlinked).unwrap();
    let ObjectSnapshot::Path(p) = &o else {
        panic!("paths: {o:?}")
    };
    assert!(p.closed, "closed where the sketch's loop closes");
    assert!(same_outline(&outline(&o), &before, CURVE_TOL));
    assert_eq!(d.sketch(f.sketch).unwrap().graph, graph);
    assert!(same_outline(&outline_of(&d, kept), &kept_before, TOL));
    assert!(matches!(d.object(kept), Some(ObjectSnapshot::Clone(_))));
}

// ---- criterion 14: the row state ------------------------------------------------------------

#[test]
fn ac14_the_row_state_reads_full_free_and_conflict() {
    // An unconstrained closed box: some degrees of freedom.
    let f = fixture();
    let st = row(&f.s, &f.sketch_page.id)["sketch"]["state"].clone();
    assert_eq!(st["state"], "free", "{st}");
    assert!(st["dof"].as_u64().unwrap() > 0);
    // A fully fixed one: a lone fixed point on a fixed line is awkward; fix every point.
    let mut g = closed_box();
    for e in g.entities.values_mut() {
        if let sk::Entity::Point { fixed, .. } = e {
            *fixed = true;
        }
    }
    let f = fixture_with(Some(&g));
    let st = row(&f.s, &f.sketch_page.id)["sketch"]["state"].clone();
    assert_eq!(st["state"], "full", "{st}");
}

#[test]
fn ac14_the_row_state_follows_a_conflict_in_the_same_frame() {
    let mut f = fixture_with(Some(&sk::Sketch::new("origin".into())));
    f.s.activate_page(&f.sketch_page.id);
    f.s.set_sketch_tool(SketchTool::Polyline);
    let base = doc(&f.s).sketch(f.sketch).unwrap().placement;
    for (x, y) in [(10.0, 10.0), (40.0, 10.0)] {
        let p = base.apply_point(Point::new(x, y));
        f.s.pointer_down(p, false);
        f.s.pointer_up(p, false, false);
    }
    f.s.escape();
    let graph = doc(&f.s).sketch(f.sketch).unwrap().graph;
    let line = graph
        .entities
        .iter()
        .find_map(|(id, e)| matches!(e, sk::Entity::Line { .. }).then_some(id.clone()))
        .unwrap();
    assert!(f.s.sketch_select_entity(&line, false));
    f.s.sketch_apply_action("horizontal", "", false).unwrap();
    f.s.sketch_apply_action("length", "20 mm", false).unwrap();
    f.s.sketch_apply_action("length", "30 mm", false).unwrap();
    let st = row(&f.s, &f.sketch_page.id)["sketch"]["state"].clone();
    assert_eq!(st["state"], "conflict", "{st}");
    // Convert to paths from the last valid solve (criterion 15).
    let last_valid = doc(&f.s).sketch(f.sketch).unwrap().resolved;
    f.s.sketch_convert_to_path().unwrap();
    let d = doc(&f.s);
    let o = d.object(f.sketch).unwrap();
    assert!(matches!(o, ObjectSnapshot::Path(_)));
    let got: Vec<_> = outline(&o);
    let want: Vec<_> = last_valid.all_anchors().copied().collect();
    assert!(same_outline(&got, &want, CURVE_TOL));
    assert_eq!(find_page(&d, &f.sketch_page.id).kind, PageKind::Drawing);
}

// ---- criterion 15: Convert to paths ---------------------------------------------------------

#[test]
fn ac15_convert_turns_the_page_into_a_drawing_page_in_one_step_and_undo_restores_it() {
    let mut f = fixture();
    let sp = f.sketch_page.id.clone();
    place_by_drag(&mut f.s, &sp, centre(&f.first));
    let d = doc(&f.s);
    let clone = clone_ids(&d, f.sketch)[0];
    let clone_before = outline_of(&d, clone);
    let graph = d.sketch(f.sketch).unwrap().graph;
    f.s.activate_page(&sp);
    let steps = f.s.undo_len();
    f.s.sketch_convert_to_path().unwrap();
    assert_eq!(f.s.undo_len(), steps + 1);
    assert_eq!(last_step_name(&mut f.s), "Convert sketch to paths");
    assert_eq!(
        f.s.workbench(),
        Workbench::Vector,
        "drawing tools in the same frame"
    );
    assert_eq!(
        f.s.active_page().id,
        sp,
        "the same page, now a drawing page"
    );
    let r = row(&f.s, &sp);
    assert_eq!(r["kind"], "drawing");
    assert_eq!(r["kindWord"], "Drawing");
    assert_eq!(r["name"], "Bracket");
    assert_eq!(
        selected(&mut f.s),
        vec![f.sketch],
        "the new paths are selected"
    );
    let d = doc(&f.s);
    assert!(matches!(d.object(f.sketch), Some(ObjectSnapshot::Path(_))));
    assert!(same_outline(
        &outline_of(&d, clone),
        &clone_before,
        CURVE_TOL
    ));
    // The rectangle over it now belongs to it: Select all on it takes it.
    key(&mut f.s, "a", true);
    assert!(selected(&mut f.s).contains(&f.over));
    f.s.escape();
    f.s.undo();
    assert_eq!(row(&f.s, &sp)["kind"], "sketch");
    assert_eq!(f.s.workbench(), Workbench::Sketch);
    assert!(
        sketch_selection(&f.s).is_empty(),
        "no sketch geometry selected after undo"
    );
    assert_eq!(doc(&f.s).sketch(f.sketch).unwrap().graph, graph);
    f.s.redo();
    assert_eq!(row(&f.s, &sp)["kind"], "drawing");
}

#[test]
fn ac15_undo_of_convert_leaves_an_empty_selection() {
    let mut f = fixture();
    f.s.activate_page(&f.sketch_page.id);
    f.s.sketch_convert_to_path().unwrap();
    f.s.undo();
    assert_eq!(f.s.workbench(), Workbench::Sketch);
    assert!(selected(&mut f.s).is_empty(), "{:?}", selected(&mut f.s));
    assert_eq!(f.s.selected_object_count(), 0);
}

#[test]
fn ac15_the_open_tab_after_convert_is_the_last_used_drawing_tab_else_document() {
    let mut f = fixture();
    f.s.press_panel_tab("layers");
    f.s.activate_page(&f.sketch_page.id);
    f.s.sketch_convert_to_path().unwrap();
    let open =
        f.s.panel_view()
            .tabs
            .iter()
            .find(|t| t.selected)
            .map(|t| t.name)
            .unwrap_or_default();
    assert_eq!(open, "layers");
}

// ---- criteria 16, 17: Delete ----------------------------------------------------------------

#[test]
fn ac17_delete_a_sketch_page_with_clones_on_several_pages_is_one_step_and_undo_restores() {
    let mut f = fixture();
    let sp = f.sketch_page.id.clone();
    place_by_drag(&mut f.s, &sp, centre(&f.first));
    place_by_drag(
        &mut f.s,
        &sp,
        Point::new(f.second.origin.x + 40.0, f.second.origin.y + 40.0),
    );
    place_by_drag(
        &mut f.s,
        &sp,
        Point::new(f.second.origin.x + 120.0, f.second.origin.y + 150.0),
    );
    place_by_drag(&mut f.s, &sp, Point::new(-700.0, -700.0));
    let d = doc(&f.s);
    let clones = clone_ids(&d, f.sketch);
    assert_eq!(clones.len(), 4);
    let before: Vec<_> = clones.iter().map(|c| outline_of(&d, *c)).collect();
    let over_before = d.object(f.over).unwrap();
    assert_eq!(
        row(&f.s, &sp)["sketch"]["clones"],
        4,
        "live count with pasteboard"
    );
    let steps = f.s.undo_len();
    assert!(f.s.delete_page(&sp));
    assert_eq!(f.s.undo_len(), steps + 1);
    assert_eq!(last_step_name(&mut f.s), "Delete sketch page");
    let d = doc(&f.s);
    assert!(d.pages().iter().all(|p| p.id != sp));
    assert!(d.sketch(f.sketch).is_none());
    for (c, b) in clones.iter().zip(&before) {
        let o = d.object(*c).unwrap();
        assert!(matches!(o, ObjectSnapshot::Path(_)), "{c:?} became a path");
        assert!(same_outline(&outline(&o), b, CURVE_TOL));
    }
    assert_eq!(d.object(f.over).unwrap(), over_before);
    f.s.undo();
    let d = doc(&f.s);
    assert!(d.pages().iter().any(|p| p.id == sp));
    assert_eq!(
        clone_ids(&d, f.sketch).len(),
        4,
        "clones are linked clones again"
    );
    assert_eq!(f.s.sketch_usage(f.sketch).clones, 4);
}

// ---- criterion 18: SVG export ---------------------------------------------------------------

fn export_fixture() -> (Session, Page, Page, NodeId, NodeId) {
    let d = Document::new(18);
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    // A closed outline with two holes, a construction line, three dimensions.
    let mut g = sk::create_polyline(
        &sk::Sketch::new("origin".into()),
        "outer",
        &[
            sk::Position::mm(-40.0, -25.0),
            sk::Position::mm(40.0, -25.0),
            sk::Position::mm(40.0, 25.0),
            sk::Position::mm(-40.0, 25.0),
        ],
        true,
    )
    .unwrap();
    g = sk::create_circle(&g, "hole_a", sk::Position::mm(-20.0, 0.0), sk::Length(5.0)).unwrap();
    g = sk::create_circle(&g, "hole_b", sk::Position::mm(20.0, 0.0), sk::Length(5.0)).unwrap();
    let before: Vec<_> = g.entities.keys().cloned().collect();
    g = sk::create_polyline(
        &g,
        "axis",
        &[sk::Position::mm(-45.0, 0.0), sk::Position::mm(45.0, 0.0)],
        false,
    )
    .unwrap();
    let construction: Vec<_> = g
        .entities
        .iter()
        .filter(|(id, e)| !before.contains(id) && matches!(e, sk::Entity::Line { .. }))
        .map(|(id, _)| id.clone())
        .collect();
    g.construction.extend(construction);
    let lines: Vec<_> = g
        .entities
        .iter()
        .filter(|(id, e)| matches!(e, sk::Entity::Line { .. }) && !g.construction.contains(*id))
        .map(|(id, _)| id.clone())
        .collect();
    for (n, l) in lines.iter().take(3).enumerate() {
        g.constraints.insert(
            format!("dim{n}").as_str().into(),
            sk::Constraint::driving(sk::Relation::Length {
                curve: l.clone(),
                value: sk::Expression::Literal(
                    sk::Quantity::length(if n % 2 == 0 { 80.0 } else { 50.0 }).unwrap(),
                ),
            }),
        );
    }
    let sketch = d.create_page_sketch(&sp.id, &g, &Style::default()).unwrap();
    let sp = find_page(&d, &sp.id);
    let circle = d.create_ellipse(curvyo_document_core::EllipseFrame::from_corners(
        Point::new(sp.origin.x + 10.0, sp.origin.y + 10.0),
        Point::new(sp.origin.x + 30.0, sp.origin.y + 30.0),
    ));
    let first = find_page(&d, "first");
    let zero = d
        .sketch(sketch)
        .unwrap()
        .placement
        .apply_point(Point::new(0.0, 0.0));
    let c = centre(&first);
    d.place_linked_clone(sketch, Vec2::new(c.x - zero.x, c.y - zero.y))
        .unwrap();
    let s = open_session(&d);
    (s, first, sp, sketch, circle)
}

fn svg(s: &Session, active_page_only: bool) -> String {
    match s.export_svg(SvgExportOptions {
        active_page_only,
        ..SvgExportOptions::default()
    }) {
        SvgExportResult::Written { text, .. } => text,
        other @ SvgExportResult::NothingSelected => panic!("{other:?}"),
        SvgExportResult::UnsupportedExtendedPath => panic!("unexpected unsupported extended path"),
    }
}

fn golden(name: &str, text: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/export/sketch_pages_tester")
        .join(name);
    if std::env::var_os("CURVYO_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("golden {name}"));
    assert_eq!(text, want, "golden {name}");
}

fn count(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

#[test]
fn ac18a_c_active_page_only_on_the_sketch_page_holds_exactly_the_sketch() {
    let (mut s, _first, sp, _sketch, _circle) = export_fixture();
    s.activate_page(&sp.id);
    let text = svg(&s, true);
    golden("active_sketch_page.svg", &text);
    // Four edges and two circles, one element per entity; no construction line, no circle
    // object, no clone.
    let elements = count(&text, "<path")
        + count(&text, "<circle")
        + count(&text, "<line")
        + count(&text, "<ellipse")
        + count(&text, "<use");
    assert_eq!(elements, 6, "{text}");
    assert_eq!(count(&text, "<use"), 0, "{text}");
    assert!(!text.contains("dim"), "no dimensions: {text}");
}

#[test]
fn ac18c_a_whole_plane_export_writes_the_circle_over_the_sketch_page() {
    let (s, _first, _sp, _sketch, _circle) = export_fixture();
    let text = svg(&s, false);
    golden("whole_plane.svg", &text);
    let without_circle = {
        let (s2, _f, _p, _sk, circle) = export_fixture();
        let d = doc(&s2);
        d.delete_objects(&[circle]).unwrap();
        svg(&open_session(&d), false)
    };
    assert_ne!(
        text, without_circle,
        "the circle is in the whole-plane file"
    );
}

#[test]
fn ac18d_active_page_only_on_the_drawing_page_writes_the_clone_as_a_path() {
    let (mut s, first, _sp, _sketch, _circle) = export_fixture();
    s.activate_page(&first.id);
    let text = svg(&s, true);
    golden("active_drawing_page.svg", &text);
    assert_eq!(count(&text, "<use"), 0, "a path, not a use: {text}");
    assert!(count(&text, "<path") >= 1, "{text}");
}

// ---- criterion 21: no I/O ---------------------------------------------------------------

#[test]
fn ac21_the_new_session_modules_reach_no_network_or_device() {
    for file in ["src/session/row_drag.rs", "src/session/sketch_use.rs"] {
        let text =
            std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file)).unwrap();
        for bad in [
            "std::net",
            "std::fs",
            "reqwest",
            "fetch(",
            "serialport",
            "std::time",
        ] {
            assert!(!text.contains(bad), "{file} uses {bad}");
        }
    }
}

#[test]
fn history_names_of_the_three_new_steps() {
    let mut f = fixture();
    let sp = f.sketch_page.id.clone();
    place_by_drag(&mut f.s, &sp, centre(&f.first));
    assert_eq!(last_step_name(&mut f.s), "Place sketch clone");
    f.s.activate_page(&sp);
    f.s.sketch_convert_to_path().unwrap();
    assert_eq!(last_step_name(&mut f.s), "Convert sketch to paths");
    f.s.undo();
    assert!(f.s.delete_page(&sp));
    assert_eq!(last_step_name(&mut f.s), "Delete sketch page");
    let _ = Affine::IDENTITY;
}

// ---- white-box edges -----------------------------------------------------------------------

#[test]
fn edge_escape_during_a_sketch_frame_drag_writes_nothing() {
    let mut f = fixture();
    let bytes = f.s.pack("x").unwrap();
    let sp = f.sketch_page.clone();
    let start = Point::new(sp.origin.x - 0.5, sp.origin.y + 100.0);
    f.s.pointer_hover(start, false, false);
    f.s.pointer_down(start, false);
    f.s.pointer_hover(Point::new(start.x + 40.0, start.y + 10.0), false, false);
    f.s.escape();
    assert_eq!(f.s.pack("x").unwrap(), bytes);
    assert_eq!(f.s.undo_len(), 0);
}

#[test]
fn edge_a_marquee_started_on_a_drawing_page_takes_objects_over_a_sketch_page_but_not_the_sketch() {
    let mut f = fixture();
    // Lay the sketch page next to the second drawing page's right edge.
    let d = doc(&f.s);
    let right = f.second.origin.x + f.second.size.width.as_mm();
    let owner = f.sketch;
    d.move_page(
        &f.sketch_page.id,
        Vec2::new(
            right + 5.0 - f.sketch_page.origin.x,
            f.second.origin.y - f.sketch_page.origin.y,
        ),
        &curvyo_document_core::PageContents::from([(f.sketch_page.id.clone(), vec![owner])]),
    )
    .unwrap();
    let sp = find_page(&d, &f.sketch_page.id);
    let over = d.create_rect(RectBounds::from_corners(
        Point::new(sp.origin.x + 2.0, sp.origin.y + 2.0),
        Point::new(sp.origin.x + 12.0, sp.origin.y + 12.0),
    ));
    f.s = open_session(&d);
    f.s.activate_page(&f.second.id);
    drag(
        &mut f.s,
        Point::new(right - 10.0, sp.origin.y + 1.0),
        Point::new(
            sp.origin.x + sp.size.width.as_mm() + 5.0,
            sp.origin.y + sp.size.height.as_mm() + 5.0,
        ),
    );
    let sel = selected(&mut f.s);
    assert!(sel.contains(&over), "{sel:?}");
    assert!(!sel.contains(&owner));
    assert_eq!(f.s.active_page().id, f.second.id);
}
