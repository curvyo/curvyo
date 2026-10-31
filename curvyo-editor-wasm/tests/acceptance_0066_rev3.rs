//! Independent acceptance tests for 0066 customer revision 3 (AC36-AC41), through public APIs.
//!
//! Written from `specification.md` "Customer revision 3 (2026-10-31)" before the implementation
//! diff was read; only the public `Session` surface and the existing revision-2 test helpers
//! were used to learn names. Coordinates are millimetres in the sketch frame (page centre).
//! Every behaviour is driven through pointer press, hover, release and key events (AC41); the
//! host's Tab and Enter on the on-canvas entry are `sketch_open_entry` and
//! `sketch_complete_fields`, exactly as the frontend calls them.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::type_complexity,
    clippy::items_after_statements,
    clippy::float_cmp,
    clippy::needless_pass_by_value,
    clippy::used_underscore_binding,
    clippy::manual_midpoint,
    clippy::cast_possible_truncation,
    clippy::collapsible_if,
    clippy::redundant_clone,
    clippy::assertions_on_result_states,
    clippy::unnecessary_to_owned,
    clippy::cloned_ref_to_slice_refs
)]
mod cpu_time;
use curvyo_document_core::{Document, Point, unpack};
use curvyo_editor_wasm::{KeyInput, Session, SketchTool, Workbench};
use curvyo_sketch_core as sk;
use serde_json::Value;

type Pt = (f64, f64);

// ---------------------------------------------------------------- helpers

fn session() -> Session {
    let mut s = Session::new(6600);
    s.set_workbench(Workbench::Sketch);
    s.resize_viewport(1000.0, 700.0);
    s
}
fn at(s: &Session, x: f64, y: f64) -> Point {
    let page = s.active_page();
    Point::new(
        page.origin.x + page.size.width.as_mm() / 2.0 + x,
        page.origin.y + page.size.height.as_mm() / 2.0 + y,
    )
}
fn document(s: &Session) -> Document {
    unpack(6601, &s.pack("rev3").unwrap()).unwrap()
}
fn graph(s: &Session) -> sk::Sketch {
    document(s)
        .sketch(s.active_sketch().unwrap())
        .unwrap()
        .solution
}
/// Number of entities besides the origin; 0 while no sketch exists yet.
fn geo(s: &Session) -> usize {
    s.active_sketch()
        .map_or(0, |_| graph(s).entities.len().saturating_sub(1))
}
fn view(s: &Session) -> Value {
    serde_json::from_str(&s.sketch_view_json()).unwrap()
}
fn click(s: &mut Session, x: f64, y: f64) {
    let p = at(s, x, y);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}
fn hover(s: &mut Session, x: f64, y: f64) {
    let p = at(s, x, y);
    s.pointer_hover(p, false, false);
}
fn alt(s: &mut Session) {
    s.key_down(KeyInput {
        key: "Alt",
        alt: true,
        ..KeyInput::default()
    });
}
fn mode(s: &Session) -> String {
    view(s)["mode"].as_str().unwrap_or("").to_owned()
}
fn pos(g: &sk::Sketch, id: &sk::EntityId) -> Pt {
    let p = g.point(id).unwrap();
    (p.x.0, p.y.0)
}
fn dist(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
fn points(g: &sk::Sketch) -> Vec<(sk::EntityId, Pt)> {
    g.entities
        .iter()
        .filter(|(id, e)| *id != &g.origin && matches!(e, sk::Entity::Point { .. }))
        .map(|(id, _)| (id.clone(), pos(g, id)))
        .collect()
}
fn lines(g: &sk::Sketch) -> Vec<(sk::EntityId, sk::EntityId, sk::EntityId)> {
    g.entities
        .iter()
        .filter_map(|(id, e)| match e {
            sk::Entity::Line { start, end } => Some((id.clone(), start.clone(), end.clone())),
            _ => None,
        })
        .collect()
}
fn arcs(g: &sk::Sketch) -> Vec<(sk::EntityId, sk::EntityId, sk::EntityId, sk::EntityId)> {
    g.entities
        .iter()
        .filter_map(|(id, e)| match e {
            sk::Entity::Arc {
                center, start, end, ..
            } => Some((id.clone(), center.clone(), start.clone(), end.clone())),
            _ => None,
        })
        .collect()
}
fn kinds(g: &sk::Sketch, pred: impl Fn(&sk::Entity) -> bool) -> Vec<sk::EntityId> {
    g.entities
        .iter()
        .filter(|(_, e)| pred(e))
        .map(|(id, _)| id.clone())
        .collect()
}
fn rel_names(g: &sk::Sketch) -> Vec<String> {
    let mut v: Vec<String> = g
        .constraints
        .values()
        .map(|c| format!("{:?}", c.relation))
        .collect();
    v.sort();
    v
}
fn dof(s: &Session) -> i64 {
    match &view(s)["status"] {
        Value::Object(o) => o.values().next().and_then(Value::as_i64).unwrap_or(0),
        Value::String(t) if t.contains("Fully") => 0,
        other => panic!("status {other}"),
    }
}
fn status_text(s: &Session) -> String {
    view(s)["status"].to_string()
}
fn capture_marker(s: &Session) -> bool {
    view(s)["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["id"] == "pending-capture" && a["text"] == "Endpoint")
}
fn hover_px(s: &mut Session, e: Pt, dx_px: f64, dy_px: f64) {
    let sc = s.view().scale();
    let p = at(s, e.0, e.1);
    s.pointer_hover(Point::new(p.x + dx_px / sc, p.y + dy_px / sc), false, false);
}
fn click_px(s: &mut Session, e: Pt, dx_px: f64, dy_px: f64) {
    let sc = s.view().scale();
    let p = at(s, e.0, e.1);
    let q = Point::new(p.x + dx_px / sc, p.y + dy_px / sc);
    s.pointer_hover(q, false, false);
    s.pointer_down(q, false);
    s.pointer_up(q, false, false);
}
fn draw_line(s: &mut Session, a: Pt, b: Pt) {
    s.set_sketch_tool(SketchTool::Polyline);
    click(s, a.0, a.1);
    click(s, b.0, b.1);
    s.escape();
}
fn place_point(s: &mut Session, p: Pt) {
    s.set_sketch_tool(SketchTool::Point);
    click(s, p.0, p.1);
}
fn select(s: &mut Session, ids: &[sk::EntityId]) {
    for (i, id) in ids.iter().enumerate() {
        assert!(s.sketch_select_entity(id, i > 0), "select {id:?}");
    }
}
fn selected(s: &Session) -> Vec<String> {
    let mut v: Vec<String> = view(s)["entities"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["selected"] == true)
        .map(|e| e["id"].as_str().unwrap().to_owned())
        .collect();
    v.sort();
    v
}
fn rail_state(s: &Session, name: &str) -> String {
    view(s)["rail"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap()["state"]
        .as_str()
        .unwrap()
        .to_owned()
}
fn texts(s: &Session) -> Vec<String> {
    view(s)["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["text"].as_str().unwrap_or("").to_owned())
        .collect()
}
const X_AXIS: Pt = (-220.0, 0.0);
const Y_AXIS: Pt = (0.0, -220.0);
/// Adds an axis to the selection by shift-clicking it in the Select tool.
fn select_axis(s: &mut Session, axis: Pt) {
    s.set_sketch_tool(SketchTool::Select);
    let p = at(s, axis.0, axis.1);
    s.pointer_hover(p, true, false);
    s.pointer_down(p, true);
    s.pointer_up(p, true, false);
}

// ---------------------------------------------------------------- AC37 coincident targets

/// A free line and a free point away from it.
fn line_and_point() -> (
    Session,
    sk::EntityId,
    sk::EntityId,
    sk::EntityId,
    sk::EntityId,
) {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 100.0));
    place_point(&mut s, (200.0, 140.0));
    s.set_sketch_tool(SketchTool::Select);
    let g = graph(&s);
    let (l, a, b) = lines(&g)[0].clone();
    let p = points(&g)
        .into_iter()
        .find(|(_, q)| near(*q, (200.0, 140.0)))
        .unwrap()
        .0;
    (s, l, a, b, p)
}
/// Distance of `p` from the infinite line through `a` and `b`.
fn off_line(p: Pt, a: Pt, b: Pt) -> f64 {
    let d = (b.0 - a.0, b.1 - a.1);
    ((p.0 - a.0) * d.1 - (p.1 - a.1) * d.0).abs() / d.0.hypot(d.1)
}
fn line_ends(g: &sk::Sketch, l: &sk::EntityId) -> (Pt, Pt) {
    let (_, a, b) = lines(g).into_iter().find(|x| &x.0 == l).unwrap();
    (pos(g, &a), pos(g, &b))
}
fn near(a: Pt, b: Pt) -> bool {
    dist(a, b) < 1e-6
}

#[test]
fn ac37_coincident_point_and_line_lies_on_the_infinite_line_dof_minus_one() {
    let (mut s, l, _a, _b, p) = line_and_point();
    let before = dof(&s);
    let steps = s.undo_len();
    select(&mut s, &[p.clone(), l.clone()]);
    assert_eq!(rail_state(&s, "coincident"), "apply");
    assert!(s.sketch_rail_press("coincident").unwrap());
    let g = graph(&s);
    // the point is moved onto y = 100 (or stays: the solver may keep x = 200, beyond the segment)
    let q = pos(&g, &p);
    let (la, lb) = line_ends(&g, &l);
    assert!(
        off_line(q, la, lb) < 1e-6,
        "on the line: {q:?} {la:?} {lb:?}"
    );
    assert_eq!(before - dof(&s), 1, "one DOF removed");
    assert_eq!(s.undo_len(), steps + 1, "one history step");
    assert_eq!(
        g.constraints.len(),
        1,
        "exactly one constraint: {:?}",
        rel_names(&g)
    );
    // kind named in Sketches and the symbol's accessible name
    let j = view(&s);
    let c = &j["constraints"].as_array().unwrap()[0];
    let kind = c["kind"].as_str().unwrap().to_lowercase();
    assert!(
        kind.contains("line") && kind.contains("point") || kind.contains("on"),
        "kind names point-on-line: {c}"
    );
    let sym = j["symbols"].as_array().unwrap();
    assert!(
        sym.iter().any(|y| {
            let n = y["name"].as_str().unwrap().to_lowercase();
            n.contains("point") && n.contains("line")
        }),
        "accessible name names the kind: {sym:?}"
    );
}

#[test]
fn ac37_point_on_line_extent_is_not_enforced_and_survives_drag_save_undo() {
    let (mut s, l, a, b, p) = line_and_point();
    select(&mut s, &[p.clone(), l.clone()]);
    s.sketch_rail_press("coincident").unwrap();
    let g = graph(&s);
    let q = pos(&g, &p);
    // outside the segment extent: infinite line reading (default R3-Q2 A)
    let (la, lb) = line_ends(&g, &l);
    let t = ((q.0 - la.0) * (lb.0 - la.0) + (q.1 - la.1) * (lb.1 - la.1)) / dist(la, lb).powi(2);
    assert!(
        t > 1.0 + 1e-3 || t < -1e-3,
        "not clamped to the segment: t={t}"
    );
    // drag the point along and off: it stays on the line, DOF stable
    s.set_sketch_tool(SketchTool::Select);
    let before = dof(&s);
    let from = at(&s, q.0, q.1);
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    for i in 1..=6 {
        let f = f64::from(i) / 6.0;
        s.pointer_hover(at(&s, q.0 + 40.0 * f, q.1 - 60.0 * f), false, false);
    }
    s.pointer_up(at(&s, q.0 + 40.0, q.1 - 60.0), false, false);
    let g2 = graph(&s);
    let (la2, lb2) = line_ends(&g2, &l);
    assert!(
        off_line(pos(&g2, &p), la2, lb2) < 1e-6,
        "still on the line after drag"
    );
    assert_eq!(dof(&s), before);
    let _ = (a, b);
    // save / reopen keeps the constraint
    let d = document(&s);
    let g3 = d
        .sketch(s.active_sketch().unwrap())
        .unwrap()
        .solution
        .clone();
    assert_eq!(g3.constraints.len(), 1);
    // undo removes it
    s.undo(); // the drag
    assert_eq!(graph(&s).constraints.len(), 1);
    s.undo(); // the constraint
    assert_eq!(
        graph(&s).constraints.len(),
        0,
        "undo reverts the constraint"
    );
}

#[test]
fn ac37_coincident_armed_picks_point_then_line_and_line_then_point() {
    for line_first in [false, true] {
        let (mut s, _l, _a, _b, _p) = line_and_point();
        let steps = s.undo_len();
        s.sketch_rail_press("coincident").unwrap();
        if line_first {
            click(&mut s, 130.0, 100.0);
            click(&mut s, 200.0, 140.0);
        } else {
            click(&mut s, 200.0, 140.0);
            click(&mut s, 130.0, 100.0);
        }
        let g = graph(&s);
        assert_eq!(
            g.constraints.len(),
            1,
            "line_first={line_first}: {:?}",
            rel_names(&g)
        );
        assert_eq!(s.undo_len(), steps + 1);
        let (l, _, _) = lines(&g)[0].clone();
        let (la, lb) = line_ends(&g, &l);
        let ids: Vec<_> = points(&g).into_iter().map(|p| p.0).collect();
        let free = ids
            .iter()
            .map(|i| pos(&g, i))
            .filter(|q| *q != la && *q != lb)
            .collect::<Vec<_>>();
        assert_eq!(free.len(), 1);
        assert!(off_line(free[0], la, lb) < 1e-6, "on the line");
    }
}

#[test]
fn ac37_coincident_point_and_axes() {
    for (axis, name) in [(X_AXIS, "x"), (Y_AXIS, "y")] {
        let mut s = session();
        place_point(&mut s, (130.0, 90.0));
        s.set_sketch_tool(SketchTool::Select);
        let p = points(&graph(&s))[0].0.clone();
        let before = dof(&s);
        let steps = s.undo_len();
        select(&mut s, &[p.clone()]);
        select_axis(&mut s, axis);
        assert_eq!(rail_state(&s, "coincident"), "apply", "axis {name}");
        assert!(s.sketch_rail_press("coincident").unwrap());
        let g = graph(&s);
        let q = pos(&g, &p);
        if name == "x" {
            assert!(q.1.abs() < 1e-6, "on X axis: {q:?}");
        } else {
            assert!(q.0.abs() < 1e-6, "on Y axis: {q:?}");
        }
        assert_eq!(before - dof(&s), 1, "axis {name}");
        assert_eq!(s.undo_len(), steps + 1);
        assert_eq!(g.constraints.len(), 1);
        let j = view(&s);
        let kind = j["constraints"][0]["kind"].as_str().unwrap().to_lowercase();
        assert!(kind.contains("axis") || kind.contains(name), "kind: {kind}");
    }
}

fn circle_scene(tool_alt: usize) -> (Session, sk::EntityId, sk::EntityId) {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Circle);
    for _ in 0..tool_alt {
        alt(&mut s);
    }
    click(&mut s, 200.0, 200.0);
    click(&mut s, 240.0, 200.0);
    place_point(&mut s, (300.0, 240.0));
    s.set_sketch_tool(SketchTool::Select);
    let g = graph(&s);
    let c = kinds(&g, |e| matches!(e, sk::Entity::Circle { .. }))
        .pop()
        .unwrap();
    let p = points(&g)
        .into_iter()
        .find(|(_, q)| near(*q, (300.0, 240.0)))
        .unwrap()
        .0;
    (s, c, p)
}

#[test]
fn ac37_coincident_point_and_circle_lies_on_the_curve() {
    let (mut s, c, p) = circle_scene(0);
    let before = dof(&s);
    let steps = s.undo_len();
    select(&mut s, &[p.clone(), c.clone()]);
    assert_eq!(rail_state(&s, "coincident"), "apply");
    assert!(s.sketch_rail_press("coincident").unwrap());
    let g = graph(&s);
    let sk::Entity::Circle { center, .. } = g.entities.get(&c).unwrap().clone() else {
        panic!()
    };
    let cen = pos(&g, &center);
    let r = match g.entities.get(&c).unwrap() {
        sk::Entity::Circle { radius, .. } => radius.0,
        _ => unreachable!(),
    };
    assert!(
        (dist(pos(&g, &p), cen) - r).abs() < 1e-6,
        "on the circle: d={} r={r}",
        dist(pos(&g, &p), cen)
    );
    assert_eq!(before - dof(&s), 1);
    assert_eq!(s.undo_len(), steps + 1);
    s.undo();
    assert_eq!(graph(&s).constraints.len(), 0);
}

#[test]
fn ac37_coincident_point_and_arc_lies_on_the_full_analytic_circle() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Arc);
    click(&mut s, 100.0, 200.0);
    click(&mut s, 160.0, 200.0);
    click(&mut s, 130.0, 170.0);
    place_point(&mut s, (130.0, 260.0)); // below the arc, on the same circle's far side area
    s.set_sketch_tool(SketchTool::Select);
    let g = graph(&s);
    let (arc, centre, _st, _en) = arcs(&g)[0].clone();
    let p = points(&g)
        .into_iter()
        .find(|(_, q)| near(*q, (130.0, 260.0)))
        .unwrap()
        .0;
    let before = dof(&s);
    select(&mut s, &[p.clone(), arc]);
    assert!(s.sketch_rail_press("coincident").unwrap());
    let g = graph(&s);
    let (_, _, st, _) = arcs(&g)[0].clone();
    let cen = pos(&g, &centre);
    let r = dist(pos(&g, &st), cen);
    assert!(
        (dist(pos(&g, &p), cen) - r).abs() < 1e-6,
        "on the full circle of the arc"
    );
    assert_eq!(before - dof(&s), 1);
}

#[test]
fn ac37_coincident_point_and_ellipse_lies_on_the_curve() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Ellipse);
    click(&mut s, 200.0, 200.0);
    click(&mut s, 250.0, 200.0);
    click(&mut s, 200.0, 170.0);
    place_point(&mut s, (300.0, 230.0));
    s.set_sketch_tool(SketchTool::Select);
    let g = graph(&s);
    let e = kinds(&g, |e| matches!(e, sk::Entity::Ellipse { .. }))
        .pop()
        .unwrap();
    let p = points(&g)
        .into_iter()
        .find(|(_, q)| near(*q, (300.0, 230.0)))
        .unwrap()
        .0;
    let before = dof(&s);
    let steps = s.undo_len();
    select(&mut s, &[p.clone(), e]);
    assert_eq!(rail_state(&s, "coincident"), "apply");
    assert!(s.sketch_rail_press("coincident").unwrap());
    assert_eq!(before - dof(&s), 1);
    assert_eq!(s.undo_len(), steps + 1);
    let g = graph(&s);
    let q = pos(&g, &p);
    let sk::Entity::Ellipse {
        center,
        focus,
        minor_radius,
        ..
    } = g
        .entities
        .values()
        .find(|e| matches!(e, sk::Entity::Ellipse { .. }))
        .unwrap()
        .clone()
    else {
        unreachable!()
    };
    let (c, f1) = (pos(&g, &center), pos(&g, &focus));
    let f2 = (2.0 * c.0 - f1.0, 2.0 * c.1 - f1.1);
    let a = (minor_radius.0.powi(2) + dist(c, f1).powi(2)).sqrt();
    assert!(
        (dist(q, f1) + dist(q, f2) - 2.0 * a).abs() < 1e-6,
        "on the ellipse: {q:?}"
    );
}

#[test]
fn ac37_coincident_point_and_point_still_works() {
    let mut s = session();
    place_point(&mut s, (100.0, 100.0));
    place_point(&mut s, (150.0, 130.0));
    s.set_sketch_tool(SketchTool::Select);
    let g = graph(&s);
    let ids: Vec<_> = points(&g).into_iter().map(|p| p.0).collect();
    let before = dof(&s);
    select(&mut s, &ids);
    assert!(s.sketch_rail_press("coincident").unwrap());
    let g = graph(&s);
    assert!(near(pos(&g, &ids[0]), pos(&g, &ids[1])));
    assert_eq!(before - dof(&s), 2);
}

// ---------------------------------------------------------------- AC37 symmetric targets

#[test]
fn ac37_symmetric_two_points_and_a_line() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 100.0));
    place_point(&mut s, (120.0, 70.0));
    place_point(&mut s, (140.0, 150.0));
    s.set_sketch_tool(SketchTool::Select);
    let g = graph(&s);
    let (l, _, _) = lines(&g)[0].clone();
    let pts: Vec<_> = points(&g)
        .into_iter()
        .filter(|(_, q)| q.1 != 100.0)
        .collect();
    assert_eq!(pts.len(), 2);
    let before = dof(&s);
    let steps = s.undo_len();
    select(&mut s, &[pts[0].0.clone(), pts[1].0.clone(), l]);
    assert_eq!(rail_state(&s, "symmetric"), "apply");
    assert!(s.sketch_rail_press("symmetric").unwrap());
    let g = graph(&s);
    let (a, b) = (pos(&g, &pts[0].0), pos(&g, &pts[1].0));
    let (la, lb) = {
        let (l, _, _) = lines(&g)[0].clone();
        line_ends(&g, &l)
    };
    let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    assert!(
        off_line(mid, la, lb) < 1e-6,
        "midpoint on the line: {a:?} {b:?}"
    );
    let d = (lb.0 - la.0, lb.1 - la.1);
    let dot = (b.0 - a.0) * d.0 + (b.1 - a.1) * d.1;
    assert!(
        dot.abs() < 1e-6 * d.0.hypot(d.1),
        "joined perpendicular to the mirror line"
    );
    assert_eq!(before - dof(&s), 2);
    assert_eq!(s.undo_len(), steps + 1);
    s.undo();
    assert_eq!(graph(&s).constraints.len(), 0);
}

#[test]
fn ac37_symmetric_two_points_and_an_axis() {
    for (axis, name) in [(X_AXIS, "x"), (Y_AXIS, "y")] {
        let mut s = session();
        place_point(&mut s, (120.0, 70.0));
        place_point(&mut s, (140.0, 150.0));
        s.set_sketch_tool(SketchTool::Select);
        let ids: Vec<_> = points(&graph(&s)).into_iter().map(|p| p.0).collect();
        let before = dof(&s);
        let steps = s.undo_len();
        select(&mut s, &ids);
        select_axis(&mut s, axis);
        assert_eq!(rail_state(&s, "symmetric"), "apply", "axis {name}");
        assert!(s.sketch_rail_press("symmetric").unwrap());
        let g = graph(&s);
        let (a, b) = (pos(&g, &ids[0]), pos(&g, &ids[1]));
        if name == "x" {
            assert!(
                (a.0 - b.0).abs() < 1e-6 && (a.1 + b.1).abs() < 1e-6,
                "{a:?} {b:?}"
            );
        } else {
            assert!(
                (a.1 - b.1).abs() < 1e-6 && (a.0 + b.0).abs() < 1e-6,
                "{a:?} {b:?}"
            );
        }
        assert_eq!(before - dof(&s), 2, "axis {name}");
        assert_eq!(s.undo_len(), steps + 1);
    }
}

#[test]
fn ac37_symmetric_two_points_and_a_point_as_today() {
    let mut s = session();
    place_point(&mut s, (100.0, 100.0));
    place_point(&mut s, (180.0, 140.0));
    place_point(&mut s, (150.0, 110.0));
    s.set_sketch_tool(SketchTool::Select);
    let g = graph(&s);
    let ids: Vec<_> = points(&g).into_iter().map(|p| p.0).collect();
    let before = dof(&s);
    select(&mut s, &ids);
    assert!(s.sketch_rail_press("symmetric").unwrap());
    let g = graph(&s);
    let (a, b) = (pos(&g, &ids[0]), pos(&g, &ids[1]));
    let c = pos(&g, &ids[2]);
    assert!(((a.0 + b.0) / 2.0 - c.0).abs() < 1e-6 && ((a.1 + b.1) / 2.0 - c.1).abs() < 1e-6);
    assert_eq!(before - dof(&s), 2);
}

// ---------------------------------------------------------------- AC40 selection first

struct Scene {
    s: Session,
    l: Vec<sk::EntityId>,
    c: Vec<sk::EntityId>,
    e: sk::EntityId,
    p: Vec<sk::EntityId>,
}
fn scene() -> Scene {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    draw_line(&mut s, (100.0, 160.0), (150.0, 200.0));
    draw_line(&mut s, (100.0, 260.0), (170.0, 300.0));
    s.set_sketch_tool(SketchTool::Circle);
    click(&mut s, 250.0, 250.0);
    click(&mut s, 275.0, 250.0);
    click(&mut s, 350.0, 250.0);
    click(&mut s, 365.0, 250.0);
    s.set_sketch_tool(SketchTool::Ellipse);
    click(&mut s, 450.0, 300.0);
    click(&mut s, 490.0, 300.0);
    click(&mut s, 450.0, 280.0);
    for q in [(300.0, 100.0), (340.0, 130.0), (400.0, 100.0)] {
        place_point(&mut s, q);
    }
    s.set_sketch_tool(SketchTool::Select);
    let g = graph(&s);
    let mut l: Vec<_> = lines(&g).into_iter().collect();
    l.sort_by(|a, b| pos(&g, &a.1).1.total_cmp(&pos(&g, &b.1).1));
    let mut c = kinds(&g, |e| matches!(e, sk::Entity::Circle { .. }));
    c.sort_by(|a, b| {
        let ca = match &g.entities[a] {
            sk::Entity::Circle { center, .. } => pos(&g, center).0,
            _ => 0.0,
        };
        let cb = match &g.entities[b] {
            sk::Entity::Circle { center, .. } => pos(&g, center).0,
            _ => 0.0,
        };
        ca.total_cmp(&cb)
    });
    let e = kinds(&g, |e| matches!(e, sk::Entity::Ellipse { .. }))
        .pop()
        .unwrap();
    let mut p: Vec<_> = points(&g)
        .into_iter()
        .filter(|(_, q)| q.0 >= 290.0 && q.1 < 150.0)
        .collect();
    p.sort_by(|a, b| a.1.0.total_cmp(&b.1.0));
    assert_eq!(p.len(), 3);
    Scene {
        s,
        l: l.into_iter().map(|x| x.0).collect(),
        c,
        e,
        p: p.into_iter().map(|x| x.0).collect(),
    }
}

fn pick(sc: &mut Scene, code: &str) {
    let mut ids = Vec::new();
    let mut axis = None;
    for part in code.split('+') {
        match part {
            "p1" => ids.push(sc.p[0].clone()),
            "p2" => ids.push(sc.p[1].clone()),
            "p3" => ids.push(sc.p[2].clone()),
            "l1" => ids.push(sc.l[0].clone()),
            "l2" => ids.push(sc.l[1].clone()),
            "l3" => ids.push(sc.l[2].clone()),
            "c1" => ids.push(sc.c[0].clone()),
            "c2" => ids.push(sc.c[1].clone()),
            "e" => ids.push(sc.e.clone()),
            "xa" => axis = Some(X_AXIS),
            "ya" => axis = Some(Y_AXIS),
            other => panic!("{other}"),
        }
    }
    select(&mut sc.s, &ids);
    if let Some(a) = axis {
        select_axis(&mut sc.s, a);
    }
}

#[test]
fn ac40_every_valid_preselection_applies_at_once_in_one_history_step() {
    let cases: &[(&str, &str)] = &[
        ("coincident", "p1+p2"),
        ("coincident", "p1+l1"),
        ("coincident", "p1+xa"),
        ("coincident", "p1+ya"),
        ("coincident", "p1+c1"),
        ("coincident", "p1+e"),
        ("horizontal", "l1"),
        ("horizontal", "p1+p2"),
        ("vertical", "l1"),
        ("vertical", "p1+p2"),
        ("parallel", "l1+l2"),
        ("parallel", "l1+l2+l3"),
        ("perpendicular", "l1+l2"),
        ("tangent", "l1+c1"),
        ("tangent", "c1+c2"),
        ("tangent", "l2+e"),
        ("equal", "l1+l2"),
        ("equal", "l1+l2+l3"),
        ("equal", "c1+c2"),
        ("fixed", "p1"),
        ("fixed", "p1+l1"),
        ("midpoint", "p1+l1"),
        ("symmetric", "p1+p2+p3"),
        ("symmetric", "p1+p2+l1"),
        ("symmetric", "p1+p2+xa"),
        ("symmetric", "p1+p2+ya"),
    ];
    ac40_run(cases);
}

/// AC40 lists Dimension among the tools that apply a valid preselection at once. Revision 2
/// armed the tool and waited for a label placement; the wording of AC40 asks for no further pick.
#[test]
fn ac40_dimension_preselection_applies_at_once_in_one_history_step() {
    ac40_run(&[
        ("dimension", "l1"),
        ("dimension", "p1+p2"),
        ("dimension", "c1"),
        ("dimension", "l1+l2"),
    ]);
}

fn ac40_run(cases: &[(&str, &str)]) {
    let mut failures = Vec::new();
    for (tool, code) in cases {
        let mut sc = scene();
        pick(&mut sc, code);
        let n0 = graph(&sc.s).constraints.len();
        let steps = sc.s.undo_len();
        let sel0 = selected(&sc.s);
        let v0 = view(&sc.s);
        let state = rail_state(&sc.s, tool);
        let r = sc.s.sketch_rail_press(tool);
        let n1 = graph(&sc.s).constraints.len();
        let v1 = view(&sc.s);
        let mut why = Vec::new();
        if state != "apply" {
            why.push(format!("rail state {state:?}"));
        }
        if r.is_err() {
            why.push(format!("press {r:?}"));
        }
        if n1 <= n0 {
            why.push(format!("no constraint added ({n0} -> {n1})"));
        }
        if sc.s.undo_len() != steps + 1 {
            why.push(format!("history {} -> {}", steps, sc.s.undo_len()));
        }
        if v1["tool"] != v0["tool"] || v1["constraint_tool"] != v0["constraint_tool"] {
            why.push(format!(
                "tool state changed {} {} -> {} {}",
                v0["tool"], v0["constraint_tool"], v1["tool"], v1["constraint_tool"]
            ));
        }
        let sel1 = selected(&sc.s);
        if sel1.iter().any(|x| !sel0.contains(x)) {
            why.push(format!("selected something new {sel0:?} -> {sel1:?}"));
        }
        if why.is_empty() {
            sc.s.undo();
            if graph(&sc.s).constraints.len() != n0 || sc.s.undo_len() != steps {
                why.push("undo did not revert exactly".to_owned());
            }
        }
        if !why.is_empty() {
            failures.push(format!("{tool} [{code}]: {}", why.join("; ")));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn ac40_invalid_preselection_adds_nothing_and_waits_with_a_hint() {
    let cases: &[(&str, &str)] = &[
        ("parallel", "l1"),
        ("perpendicular", "l1+p1"),
        ("coincident", "l1+l2"),
        ("coincident", "c1+c2"),
        ("midpoint", "p1+p2"),
        ("tangent", "p1+p2"),
        ("symmetric", "p1"),
        ("horizontal", "c1"),
        ("equal", "l1+c1"),
    ];
    let mut failures = Vec::new();
    for (tool, code) in cases {
        let mut sc = scene();
        pick(&mut sc, code);
        let n0 = graph(&sc.s).constraints.len();
        let steps = sc.s.undo_len();
        let _ = sc.s.sketch_rail_press(tool);
        let v = view(&sc.s);
        let mut why = Vec::new();
        if graph(&sc.s).constraints.len() != n0 {
            why.push("constraint added".to_owned());
        }
        if sc.s.undo_len() != steps {
            why.push("history step added".to_owned());
        }
        if v["constraint_tool"] != *tool {
            why.push(format!(
                "tool did not wait: constraint_tool = {}",
                v["constraint_tool"]
            ));
        }
        let hint = format!(
            "{} {:?} {}",
            v["canvas"]["mode_label"],
            texts(&sc.s),
            v["error"]
        );
        let hint_ok = v["canvas"]["mode_label"]["text"]
            .as_str()
            .is_some_and(|t| !t.is_empty())
            || texts(&sc.s).iter().any(|t| !t.is_empty());
        if !hint_ok {
            why.push(format!("no on-canvas hint ({hint})"));
        }
        if !why.is_empty() {
            failures.push(format!("{tool} [{code}]: {}", why.join("; ")));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

// ---------------------------------------------------------------- AC38 click to follow

const P0: Pt = (100.0, 100.0);
const Q0: Pt = (180.0, 150.0);
const W0: Pt = (140.0, 100.0);

/// (name, tool, Alt presses before the first pick, further picks after the second).
fn creation_cases() -> Vec<(&'static str, SketchTool, usize, Vec<Pt>)> {
    vec![
        ("line", SketchTool::Polyline, 0, vec![]),
        ("rectangle", SketchTool::Rectangle, 0, vec![]),
        ("slot", SketchTool::Slot, 0, vec![W0]),
        ("circle-centre", SketchTool::Circle, 0, vec![]),
        ("circle-3pt", SketchTool::Circle, 1, vec![W0]),
        ("ellipse-centre", SketchTool::Ellipse, 0, vec![W0]),
        ("ellipse-3pt", SketchTool::Ellipse, 1, vec![W0]),
        ("arc-start", SketchTool::Arc, 0, vec![W0]),
        ("arc-centre", SketchTool::Arc, 1, vec![W0]),
    ]
}
fn begin(tool: SketchTool, alts: usize) -> Session {
    let mut s = session();
    s.set_sketch_tool(tool);
    for _ in 0..alts {
        alt(&mut s);
    }
    s
}
/// Press, drag far (>= 4 px), release at `q`.
fn drag_first(s: &mut Session, p: Pt, q: Pt) {
    let pp = at(s, p.0, p.1);
    s.pointer_hover(pp, false, false);
    s.pointer_down(pp, false);
    for i in 1..=6 {
        let f = f64::from(i) / 6.0;
        hover(s, p.0 + (q.0 - p.0) * f, p.1 + (q.1 - p.1) * f);
    }
    let qq = at(s, q.0, q.1);
    s.pointer_up(qq, false, false);
}
/// Press and release in place, move, click to commit.
fn follow_first(s: &mut Session, p: Pt, q: Pt) {
    click(s, p.0, p.1);
    for i in 1..=6 {
        let f = f64::from(i) / 6.0;
        hover(s, p.0 + (q.0 - p.0) * f, p.1 + (q.1 - p.1) * f);
    }
    click(s, q.0, q.1);
}
fn fingerprint(g: &sk::Sketch) -> (Vec<(i64, i64)>, Vec<String>, usize) {
    let mut pts: Vec<(i64, i64)> = points(g)
        .into_iter()
        .map(|(_, q)| ((q.0 * 1e6).round() as i64, (q.1 * 1e6).round() as i64))
        .collect();
    pts.sort_unstable();
    let mut ks: Vec<String> = g
        .entities
        .values()
        .map(|e| format!("{:?}", std::mem::discriminant(e)))
        .collect();
    ks.sort();
    (pts, ks, g.constraints.len())
}

#[test]
fn ac38_follow_and_drag_paths_give_identical_geometry_for_every_tool() {
    let mut failures = Vec::new();
    for (name, tool, alts, rest) in creation_cases() {
        let mut a = begin(tool, alts);
        drag_first(&mut a, P0, Q0);
        for r in &rest {
            click(&mut a, r.0, r.1);
        }
        let mut b = begin(tool, alts);
        follow_first(&mut b, P0, Q0);
        for r in &rest {
            click(&mut b, r.0, r.1);
        }
        if geo(&a) == 0 || geo(&b) == 0 {
            failures.push(format!(
                "{name}: drag path made {} entities, follow path {}",
                geo(&a),
                geo(&b)
            ));
            continue;
        }
        let (ga, gb) = (graph(&a), graph(&b));
        if fingerprint(&ga) != fingerprint(&gb) {
            failures.push(format!(
                "{name}: drag {:?} vs follow {:?}",
                fingerprint(&ga),
                fingerprint(&gb)
            ));
        }
        if a.undo_len() != b.undo_len() {
            failures.push(format!(
                "{name}: history {} vs {}",
                a.undo_len(),
                b.undo_len()
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn ac38_release_under_4_px_follows_and_4_px_or_more_commits_at_every_zoom() {
    for zoom in [1.0, 4.0, 12.0] {
        for (name, tool, alts, _) in creation_cases() {
            if name == "line"
                || name.contains("3pt")
                || name.contains("arc")
                || name.contains("ellipse")
                || name == "slot"
            {
                // single-step commit is observable only for two-pick tools; the multi-pick tools
                // are checked through the pending-picks count below
            }
            let mut s = begin(tool, alts);
            set_scale(&mut s, zoom);
            let sc = s.view().scale();
            let p = at(&s, 300.0, 200.0);
            // 3.5 px: follow state, nothing committed, picks pending
            s.pointer_hover(p, false, false);
            s.pointer_down(p, false);
            let m = Point::new(p.x + 3.5 / sc, p.y);
            s.pointer_hover(m, false, false);
            s.pointer_up(m, false, false);
            assert_eq!(
                geo(&s),
                0,
                "{name} zoom {zoom}: 3.5 px release must not commit"
            );
            assert!(
                view(&s)["pending_picks"].as_i64().unwrap_or(0) >= 1,
                "{name} zoom {zoom}: the pick is kept and the tool follows: {}",
                view(&s)["pending_picks"]
            );
            // moving the pointer follows with a live preview (entry or annotation), no history
            let steps = s.undo_len();
            s.pointer_hover(Point::new(p.x + 60.0 / sc, p.y + 40.0 / sc), false, false);
            assert_eq!(s.undo_len(), steps);
            s.escape();
            assert_eq!(geo(&s), 0, "{name}: Escape cancels");
            assert_eq!(s.undo_len(), steps, "{name}: Escape adds no history");
            // 4.5 px release behaves as a drag: for the two-pick tools it commits
            if matches!(name, "line" | "rectangle" | "circle-centre") {
                let mut s = begin(tool, alts);
                set_scale(&mut s, zoom);
                let sc = s.view().scale();
                let p = at(&s, 300.0, 200.0);
                s.pointer_hover(p, false, false);
                s.pointer_down(p, false);
                let m = Point::new(p.x + 4.5 / sc * 0.8, p.y + 4.5 / sc * 0.6);
                s.pointer_hover(m, false, false);
                s.pointer_up(m, false, false);
                assert!(geo(&s) > 0, "{name} zoom {zoom}: a 4.5 px release commits");
            }
        }
    }
}

fn set_scale(s: &mut Session, target: f64) {
    for _ in 0..80 {
        let now = s.view().scale();
        if (now / target - 1.0).abs() < 0.05 {
            return;
        }
        s.wheel(
            0.0,
            if now < target { -40.0 } else { 40.0 },
            400.0,
            300.0,
            false,
            true,
        );
    }
}

#[test]
fn ac38_escape_in_the_follow_state_cancels_every_shape_without_history() {
    for (name, tool, alts, rest) in creation_cases() {
        let mut s = begin(tool, alts);
        let steps = s.undo_len();
        click(&mut s, P0.0, P0.1);
        hover(&mut s, Q0.0, Q0.1);
        if !rest.is_empty() {
            click(&mut s, Q0.0, Q0.1);
            hover(&mut s, W0.0, W0.1);
        }
        s.escape();
        assert_eq!(geo(&s), 0, "{name}: nothing created");
        assert_eq!(s.undo_len(), steps, "{name}: no history step");
        // the next click starts afresh, not a leftover shape
        click(&mut s, 300.0, 300.0);
        hover(&mut s, 340.0, 340.0);
        assert!(
            view(&s)["pending_picks"].as_i64().unwrap_or(0) <= 1,
            "{name}"
        );
    }
}

#[test]
fn ac38_alt_cycling_and_ctrl_lock_work_in_the_follow_state() {
    // Alt cycles the polyline mode in the follow state, as in the click-click state
    let mut s = begin(SketchTool::Polyline, 0);
    click(&mut s, 100.0, 100.0);
    click(&mut s, 150.0, 120.0);
    hover(&mut s, 200.0, 120.0);
    click(&mut s, 200.0, 120.0); // press-release: follow into segment 3
    let m0 = mode(&s);
    alt(&mut s);
    assert_ne!(mode(&s), m0, "Alt advances the mode in follow");
    s.escape();
    // Ctrl lock in follow: a nearly-horizontal end snaps to an exact horizontal line
    let mut s = begin(SketchTool::Polyline, 0);
    click(&mut s, 100.0, 100.0); // follow state
    s.modifiers_changed(false, true, false);
    let p = at(&s, 180.0, 103.0);
    s.pointer_hover(p, false, true);
    s.pointer_down(p, false);
    s.pointer_up(p, false, true);
    s.modifiers_changed(false, false, false);
    let g = graph(&s);
    let l = lines(&g)[0].clone();
    let (a, b) = (pos(&g, &l.1), pos(&g, &l.2));
    assert!(
        (a.1 - b.1).abs() < 1e-6,
        "Ctrl locked the direction: {a:?} {b:?}"
    );
}

// ---------------------------------------------------------------- AC39 typed sizes become dimensions

fn entry_fields(s: &Session) -> Vec<(String, String)> {
    view(s)["canvas"]["entry"]["fields"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|f| {
                    (
                        f["label"].as_str().unwrap().to_owned(),
                        f["value"].as_str().unwrap().to_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}
/// Dimension constraints of the sketch as (kind, value) read from the Sketches view.
fn dims(s: &Session) -> Vec<(String, f64)> {
    view(s)["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| !c["value"].is_null())
        .map(|c| {
            (
                c["kind"].as_str().unwrap().to_owned(),
                c["value"]["value"].as_f64().unwrap(),
            )
        })
        .collect()
}
/// Host Enter: typed text per field (None keeps the pointer's value and stays free).
fn enter(s: &mut Session, typed: &[Option<&str>]) -> bool {
    let fields = entry_fields(s);
    let texts: Vec<String> = fields
        .iter()
        .zip(typed)
        .map(|((_, v), t)| t.map_or(v.clone(), str::to_owned))
        .collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let flags: Vec<bool> = typed.iter().map(Option::is_some).collect();
    s.sketch_complete_fields(&refs, &flags).unwrap()
}

#[test]
fn ac39_rectangle_in_follow_state_typed_width_only_is_one_dimension_and_dof_3() {
    let mut s = begin(SketchTool::Rectangle, 0);
    click(&mut s, 100.0, 100.0); // press-release: follow
    hover(&mut s, 180.0, 150.0);
    let labels: Vec<_> = entry_fields(&s).into_iter().map(|f| f.0).collect();
    assert_eq!(labels, ["Width", "Height"]);
    let steps = s.undo_len();
    s.sketch_open_entry(); // Tab
    assert!(enter(&mut s, &[Some("40"), None]));
    let d = dims(&s);
    assert_eq!(d.len(), 1, "one dimension only: {d:?}");
    assert_eq!(d[0].0, "length");
    assert!((d[0].1 - 40.0).abs() < 1e-9);
    assert_eq!(
        dof(&s),
        3,
        "x, y and the free height remain: {}",
        status_text(&s)
    );
    assert_eq!(s.undo_len(), steps + 1);
    // the width really is 40 mm, the height is the pointer's 50
    let g = graph(&s);
    let xs: Vec<f64> = points(&g).iter().map(|p| p.1.0).collect();
    let ys: Vec<f64> = points(&g).iter().map(|p| p.1.1).collect();
    let span = |v: &[f64]| {
        v.iter().copied().fold(f64::MIN, f64::max) - v.iter().copied().fold(f64::MAX, f64::min)
    };
    assert!((span(&xs) - 40.0).abs() < 1e-6, "width {}", span(&xs));
    assert!((span(&ys) - 50.0).abs() < 1e-6, "height {}", span(&ys));
}

#[test]
fn ac39_rectangle_both_typed_two_dimensions_dof_2_and_press_drag_path_matches() {
    // follow path
    let mut a = begin(SketchTool::Rectangle, 0);
    click(&mut a, 100.0, 100.0);
    hover(&mut a, 180.0, 150.0);
    a.sketch_open_entry();
    assert!(enter(&mut a, &[Some("40"), Some("25")]));
    assert_eq!(dims(&a).len(), 2);
    assert_eq!(dof(&a), 2);
    // press-drag path: the entry opens while the button is down
    let mut b = begin(SketchTool::Rectangle, 0);
    let p = at(&b, 100.0, 100.0);
    b.pointer_hover(p, false, false);
    b.pointer_down(p, false);
    hover(&mut b, 140.0, 125.0);
    hover(&mut b, 180.0, 150.0);
    b.sketch_open_entry();
    let q = at(&b, 180.0, 150.0);
    b.pointer_up(q, false, false); // releasing now must not commit the untyped shape
    assert_eq!(geo(&b), 0, "release after Tab does not commit");
    assert!(enter(&mut b, &[Some("40"), Some("25")]));
    assert_eq!(dims(&b).len(), 2, "{:?}", dims(&b));
    assert_eq!(dof(&b), 2);
    assert_eq!(fingerprint(&graph(&a)), fingerprint(&graph(&b)));
}

#[test]
fn ac39_untyped_enter_creates_no_dimension() {
    let mut s = begin(SketchTool::Rectangle, 0);
    click(&mut s, 100.0, 100.0);
    hover(&mut s, 180.0, 150.0);
    s.sketch_open_entry();
    assert!(enter(&mut s, &[None, None]));
    assert_eq!(dims(&s).len(), 0);
    assert_eq!(dof(&s), 4);
}

#[test]
fn ac39_escape_in_the_entry_cancels_without_history() {
    let mut s = begin(SketchTool::Rectangle, 0);
    click(&mut s, 100.0, 100.0);
    hover(&mut s, 180.0, 150.0);
    s.sketch_open_entry();
    let steps = s.undo_len();
    s.escape();
    assert_eq!(geo(&s), 0);
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac39_circle_radius_line_length_slot_fields_and_typed_dimensions() {
    // circle: Radius field, typed radius is a driving radius dimension, centre stays free
    let mut s = begin(SketchTool::Circle, 0);
    click(&mut s, 200.0, 200.0);
    hover(&mut s, 230.0, 200.0);
    assert_eq!(entry_fields(&s)[0].0, "Radius");
    s.sketch_open_entry();
    assert!(enter(&mut s, &[Some("12")]));
    let d = dims(&s);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].0 == "radius" && (d[0].1 - 12.0).abs() < 1e-9, "{d:?}");
    assert_eq!(dof(&s), 2);
    // line: Length typed
    let mut s = begin(SketchTool::Polyline, 0);
    click(&mut s, 100.0, 100.0);
    hover(&mut s, 150.0, 120.0);
    let labels: Vec<_> = entry_fields(&s).into_iter().map(|f| f.0).collect();
    assert_eq!(labels, ["Length", "Angle"]);
    s.sketch_open_entry();
    assert!(enter(&mut s, &[Some("30"), None]));
    let d = dims(&s);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].0 == "length" && (d[0].1 - 30.0).abs() < 1e-9);
    assert_eq!(dof(&s), 3, "{}", status_text(&s));
    // slot: first pick in follow, fields are length, angle and width (AC39 wording)
    let mut s = begin(SketchTool::Slot, 0);
    click(&mut s, 100.0, 100.0);
    hover(&mut s, 180.0, 130.0);
    let labels: Vec<_> = entry_fields(&s).into_iter().map(|f| f.0).collect();
    assert!(
        labels.iter().any(|l| l == "Length")
            && labels.iter().any(|l| l == "Angle")
            && labels.iter().any(|l| l == "Width"),
        "slot fields {labels:?}"
    );
}

#[test]
fn ac39_ellipse_both_radii_fields() {
    let mut s = begin(SketchTool::Ellipse, 0);
    click(&mut s, 200.0, 200.0);
    hover(&mut s, 250.0, 200.0);
    s.sketch_open_entry();
    let labels: Vec<_> = entry_fields(&s).into_iter().map(|f| f.0).collect();
    assert_eq!(
        labels.len(),
        2,
        "ellipse: both radii in the entry, got {labels:?}"
    );
}

#[test]
fn ac39_known_open_typed_angle_and_ellipse_radii_become_dimensions() {
    let mut s = begin(SketchTool::Polyline, 0);
    click(&mut s, 100.0, 100.0);
    hover(&mut s, 150.0, 120.0);
    s.sketch_open_entry();
    assert!(enter(&mut s, &[Some("30"), Some("20")]));
    assert_eq!(dims(&s).len(), 2, "length and angle: {:?}", dims(&s));
    let mut s = begin(SketchTool::Ellipse, 0);
    click(&mut s, 200.0, 200.0);
    hover(&mut s, 250.0, 200.0);
    s.sketch_open_entry();
    let n = entry_fields(&s).len();
    let typed: Vec<Option<&str>> = vec![Some("40"); n];
    assert!(enter(&mut s, &typed));
    assert_eq!(dims(&s).len(), n, "{:?}", dims(&s));
}

// ---------------------------------------------------------------- AC36 capture on every pick

fn to_mode(s: &mut Session, name: &str) {
    if !s.set_sketch_mode(name) {
        for _ in 0..8 {
            if mode(s) == name {
                break;
            }
            alt(s);
        }
    }
    assert_eq!(mode(s), name, "mode {name} reachable");
}
fn key(s: &mut Session, k: &str) {
    s.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    });
}

/// Polyline arc, tangent line, tangent arc, tangent arc closed on the start.
/// Returns the session, the two gap endpoints and the line's id once segment 2 is deleted.
fn customer_chain() -> (Session, sk::EntityId, sk::EntityId, Pt, Pt) {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Arc);
    // three picks: start A, a point on the arc, end B
    click(&mut s, 100.0, 200.0);
    click(&mut s, 130.0, 170.0);
    click(&mut s, 160.0, 200.0);
    assert_eq!(arcs(&graph(&s)).len(), 1);
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 160.0, 200.0); // resume from the arc's open end B
    hover(&mut s, 160.0, 250.0);
    to_mode(&mut s, "tangent-line");
    click(&mut s, 160.0, 240.0);
    hover(&mut s, 200.0, 290.0);
    to_mode(&mut s, "tangent-arc");
    click(&mut s, 200.0, 280.0);
    hover(&mut s, 130.0, 260.0);
    to_mode(&mut s, "tangent-arc");
    click(&mut s, 100.0, 200.0); // closes on the chain start A
    s.escape();
    s.set_sketch_tool(SketchTool::Select);
    let g = graph(&s);
    assert_eq!(arcs(&g).len(), 3, "three arcs: {g:?}");
    assert_eq!(lines(&g).len(), 1, "one tangent line");
    // the closed chain shares its start: no duplicate point at A
    let a_pts = points(&g)
        .into_iter()
        .filter(|(_, q)| dist(*q, (100.0, 200.0)) < 0.01)
        .count();
    assert_eq!(a_pts, 1, "closing reuses the start point");
    let (l, s1, e1) = lines(&g)[0].clone();
    let (pa, pb) = (pos(&g, &s1), pos(&g, &e1));
    // delete segment 2 (the line)
    select(&mut s, &[l]);
    key(&mut s, "Delete");
    let g = graph(&s);
    assert!(lines(&g).is_empty(), "segment 2 deleted: {g:?}");
    (s, s1, e1, pa, pb)
}

#[test]
fn ac36_customer_scenario_centre_arc_refills_the_gap_on_the_two_free_endpoints() {
    let (mut s, _s1, _e1, pa, pb) = customer_chain();
    let g0 = graph(&s);
    let free: Vec<_> = points(&g0)
        .into_iter()
        .filter(|(_, q)| dist(*q, pa) < 0.01 || dist(*q, pb) < 0.01)
        .collect();
    assert_eq!(free.len(), 2, "the gap has two endpoints: {free:?}");
    let rel0 = rel_names(&g0);
    let npts0 = points(&g0).len();
    // centre-first variant of the Arc tool (AC35 Alt cycling before the first pick)
    s.set_sketch_tool(SketchTool::Arc);
    hover(&mut s, 300.0, 300.0);
    alt(&mut s);
    assert_eq!(
        view(&s)["tool"],
        "arc-center",
        "Alt switches to the centre-first variant"
    );
    // pick 1: centre, equidistant from both gap endpoints (on their perpendicular bisector)
    let mid = ((pa.0 + pb.0) / 2.0, (pa.1 + pb.1) / 2.0);
    let dir = (pb.0 - pa.0, pb.1 - pa.1);
    let n = dir.0.hypot(dir.1);
    let perp = (-dir.1 / n, dir.0 / n);
    let centre = (mid.0 + perp.0 * 60.0, mid.1 + perp.1 * 60.0);
    click(&mut s, centre.0, centre.1);
    // pick 2: start, 6 px from the endpoint; the capture marker shows before the click
    hover_px(&mut s, pa, 6.0, 0.0);
    assert!(capture_marker(&s), "capture marker before the start click");
    click_px(&mut s, pa, 6.0, 0.0);
    // pick 3: end, likewise
    hover_px(&mut s, pb, 6.0, 4.0);
    assert!(capture_marker(&s), "capture marker before the end click");
    click_px(&mut s, pb, 6.0, 4.0);
    let g = graph(&s);
    let new_arcs: Vec<_> = arcs(&g)
        .into_iter()
        .filter(|a| !arcs(&g0).iter().any(|o| o.0 == a.0))
        .collect();
    assert_eq!(new_arcs.len(), 1, "one new arc");
    let (_, c, st, en) = new_arcs[0].clone();
    let ids: Vec<_> = free.iter().map(|f| f.0.clone()).collect();
    assert!(
        ids.contains(&st) && ids.contains(&en) && st != en,
        "start/end reuse the identities: {st:?} {en:?} vs {ids:?}"
    );
    assert!(!ids.contains(&c));
    for f in [pa, pb] {
        let n = points(&g)
            .into_iter()
            .filter(|(_, q)| dist(*q, f) < 0.05)
            .count();
        assert_eq!(n, 1, "no duplicate point at {f:?}");
    }
    assert_eq!(
        points(&g).len(),
        npts0 + 1,
        "only the centre is a new point"
    );
    assert_eq!(rel_names(&g), rel0, "no unintended extra constraints");
    // the arc endpoints did not move
    for (id, f) in [(&ids[0], &free[0].1), (&ids[1], &free[1].1)] {
        assert!(dist(pos(&g, id), *f) < 1e-6, "endpoint kept its place");
    }
}

/// Seed: one free line whose end E is open. Returns E.
fn seeded() -> (Session, Pt) {
    let mut s = session();
    draw_line(&mut s, (60.0, 60.0), (120.0, 80.0));
    (s, (120.0, 80.0))
}
/// Each tool and variant with picks that finish on `end_index`; the last pick goes at E.
fn end_cases() -> Vec<(&'static str, SketchTool, usize, Vec<Pt>)> {
    vec![
        ("point", SketchTool::Point, 0, vec![]),
        ("line", SketchTool::Polyline, 0, vec![(200.0, 200.0)]),
        (
            "polyline-3rd",
            SketchTool::Polyline,
            0,
            vec![(200.0, 200.0), (220.0, 140.0)],
        ),
        (
            "arc-start-end",
            SketchTool::Arc,
            0,
            vec![(200.0, 200.0), (170.0, 150.0)],
        ),
        (
            "arc-centre-end",
            SketchTool::Arc,
            1,
            vec![(200.0, 200.0), (55.777_948_981_440_4, 200.0)],
        ),
        (
            "circle-3pt-end",
            SketchTool::Circle,
            1,
            vec![(200.0, 200.0), (230.0, 170.0)],
        ),
        (
            "ellipse-3pt-end",
            SketchTool::Ellipse,
            1,
            vec![(200.0, 200.0), (240.0, 150.0)],
        ),
        ("rectangle", SketchTool::Rectangle, 0, vec![(200.0, 200.0)]),
        ("slot-spine-end", SketchTool::Slot, 0, vec![(200.0, 200.0)]),
        ("circle-centre", SketchTool::Circle, 0, vec![]),
        ("ellipse-centre", SketchTool::Ellipse, 0, vec![]),
        ("arc-centre-first", SketchTool::Arc, 1, vec![]),
    ]
}

#[test]
fn ac36_every_tool_and_variant_captures_a_nearby_endpoint_on_its_final_picks() {
    let mut failures = Vec::new();
    for follow in [false, true] {
        for (name, tool, alts, earlier) in end_cases() {
            let (mut s, e) = seeded();
            s.set_sketch_tool(tool);
            hover(&mut s, 300.0, 300.0);
            for _ in 0..alts {
                alt(&mut s);
            }
            let want = match name {
                "arc-centre-end" | "arc-centre-first" => Some("arc-center"),
                "circle-3pt-end" => Some("circle-three-points"),
                "ellipse-3pt-end" => Some("ellipse-three-points"),
                _ => None,
            };
            if let Some(w) = want {
                assert_eq!(view(&s)["tool"], w, "{name}: variant reached");
            }
            let seed_geo = geo(&s);
            // creation tools that need a prior pick: place those first, then the pick at E
            let mut it = earlier.iter();
            // the first pick of tools with an empty list is the pick at E itself
            if let Some(first) = it.next() {
                click(&mut s, first.0, first.1);
                for mid in it {
                    click(&mut s, mid.0, mid.1);
                }
            }
            let npts = points(&graph(&s)).len();
            let _ = npts;
            hover_px(&mut s, e, 6.0, 3.0);
            if !capture_marker(&s) {
                failures.push(format!(
                    "{name} follow={follow}: no capture marker before the click"
                ));
                continue;
            }
            click_px(&mut s, e, 6.0, 3.0);
            if name == "slot-spine-end" {
                click(&mut s, 90.0, 140.0); // width pick
            }
            if !earlier.is_empty() {
                if geo(&s) <= seed_geo {
                    failures.push(format!("{name} follow={follow}: nothing was created"));
                }
            }
            s.escape();
            let g = graph(&s);
            let near_e = points(&g)
                .into_iter()
                .filter(|(_, q)| dist(*q, e) < 0.5)
                .count();
            if near_e != 1 {
                failures.push(format!(
                    "{name} follow={follow}: {near_e} points near the endpoint (duplicate or lost): {:?}",
                    points(&g).into_iter().filter(|(_, q)| dist(*q, e) < 0.5).collect::<Vec<_>>()
                ));
            }
            // the original endpoint kept its exact position (nothing was pulled the other way)
            if !points(&g).iter().any(|(_, q)| dist(*q, e) < 1e-6) {
                failures.push(format!(
                    "{name} follow={follow}: pick was not pulled onto the endpoint exactly"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn ac36_drag_release_and_follow_click_both_reuse_the_endpoint_for_the_end_pick() {
    for tool in [
        SketchTool::Rectangle,
        SketchTool::Circle,
        SketchTool::Polyline,
    ] {
        for follow in [false, true] {
            let (mut s, e) = seeded();
            s.set_sketch_tool(tool);
            let start = (200.0, 200.0);
            if follow {
                click(&mut s, start.0, start.1);
                hover_px(&mut s, e, 6.0, 2.0);
                assert!(capture_marker(&s), "{tool:?} follow: marker");
                click_px(&mut s, e, 6.0, 2.0);
            } else {
                let p = at(&s, start.0, start.1);
                s.pointer_hover(p, false, false);
                s.pointer_down(p, false);
                for i in 1..=5 {
                    let f = f64::from(i) / 5.0;
                    hover(
                        &mut s,
                        start.0 + (e.0 - start.0) * f,
                        start.1 + (e.1 - start.1) * f,
                    );
                }
                hover_px(&mut s, e, 6.0, 2.0);
                assert!(capture_marker(&s), "{tool:?} drag: marker before release");
                let sc = s.view().scale();
                let q = at(&s, e.0, e.1);
                s.pointer_up(Point::new(q.x + 6.0 / sc, q.y + 2.0 / sc), false, false);
            }
            s.escape();
            let g = graph(&s);
            let n = points(&g)
                .into_iter()
                .filter(|(_, q)| dist(*q, e) < 0.5)
                .count();
            assert_eq!(n, 1, "{tool:?} follow={follow}: duplicate near endpoint");
        }
    }
}

// ---------------------------------------------------------------- AC40 valid start kept as first picks

/// One selected point, Dimension pressed (arms), a click on a second point: a point-point
/// distance results (the click completes the pair; it does not replace the first point).
#[test]
fn ac40_one_selected_point_then_dimension_then_a_second_point_gives_a_point_distance() {
    let mut sc = scene();
    pick(&mut sc, "p1");
    let steps = sc.s.undo_len();
    sc.s.sketch_rail_press("dimension").unwrap();
    assert_eq!(sc.s.undo_len(), steps, "one point is incomplete: it arms");
    let p2 = graph(&sc.s).entities[&sc.p[1]].clone();
    let _ = p2;
    click(&mut sc.s, 340.0, 130.0); // the second free point
    click(&mut sc.s, 320.0, 60.0); // places the label
    let dists: Vec<_> = view(&sc.s)["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["kind"] == "distance")
        .map(|c| c["entities"].clone())
        .collect();
    assert_eq!(
        dists.len(),
        1,
        "a point-point distance: {}",
        view(&sc.s)["constraints"]
    );
    let ents: Vec<String> = dists[0]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_str().unwrap().to_owned())
        .collect();
    assert!(
        ents.contains(&sc.p[0].0) && ents.contains(&sc.p[1].0),
        "between p1 and p2, got {ents:?} (p1 {}, p2 {})",
        sc.p[0].0,
        sc.p[1].0
    );
}
