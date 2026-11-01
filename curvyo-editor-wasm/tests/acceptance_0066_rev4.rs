//! Independent acceptance tests for 0066 customer revision 4 (AC42-AC48), through public APIs.
//!
//! Written from `specification.md` "Customer revision 4 (2026-10-31)" before the implementation
//! diff was read; only the public `Session` surface, the sketch view JSON and the earlier
//! revision test helpers were used to learn names. Coordinates are millimetres in the sketch
//! frame (page centre). Behaviour is driven through pointer press, hover, release and key events
//! (AC41); the host's Enter on the on-canvas entry is `sketch_complete_fields` and a dimension
//! value change is `sketch_dimension_text`, exactly as the frontend calls them.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::float_cmp,
    clippy::type_complexity,
    clippy::cast_precision_loss,
    clippy::cloned_ref_to_slice_refs
)]
use std::collections::BTreeMap;

use curvyo_document_core::{Document, Point, unpack};
use curvyo_editor_wasm::{KeyInput, Session, SketchTool, Workbench};
use curvyo_sketch_core as sk;
use serde_json::Value;

type Pt = (f64, f64);

// ---------------------------------------------------------------- helpers

fn session() -> Session {
    let mut s = Session::new(6640);
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
    unpack(6641, &s.pack("rev4").unwrap()).unwrap()
}
fn graph(s: &Session) -> sk::Sketch {
    document(s)
        .sketch(s.active_sketch().unwrap())
        .unwrap()
        .solution
}
fn view(s: &Session) -> Value {
    serde_json::from_str(&s.sketch_view_json()).unwrap()
}
fn tool(s: &Session) -> String {
    view(s)["tool"].as_str().unwrap_or("").to_owned()
}
fn armed(s: &Session) -> Value {
    view(s)["constraint_tool"].clone()
}
fn click(s: &mut Session, x: f64, y: f64) {
    click_mod(s, x, y, false);
}
fn click_mod(s: &mut Session, x: f64, y: f64, shift: bool) {
    let p = at(s, x, y);
    s.pointer_hover(p, shift, false);
    s.pointer_down(p, shift);
    s.pointer_up(p, shift, false);
}
fn hover(s: &mut Session, x: f64, y: f64) {
    let p = at(s, x, y);
    s.pointer_hover(p, false, false);
}
fn key(s: &mut Session, k: &str) {
    s.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    });
}
fn alt(s: &mut Session) {
    s.key_down(KeyInput {
        key: "Alt",
        alt: true,
        ..KeyInput::default()
    });
}
fn pos(g: &sk::Sketch, id: &sk::EntityId) -> Pt {
    let p = g.point(id).unwrap();
    (p.x.0, p.y.0)
}
fn near(a: Pt, b: Pt) -> bool {
    (a.0 - b.0).hypot(a.1 - b.1) < 1e-3
}
fn ids(g: &sk::Sketch) -> Vec<String> {
    g.entities
        .keys()
        .filter(|id| **id != g.origin)
        .map(|id| id.0.clone())
        .collect()
}
fn point_at(g: &sk::Sketch, p: Pt) -> sk::EntityId {
    g.entities
        .iter()
        .find(|(id, e)| {
            **id != g.origin && matches!(e, sk::Entity::Point { .. }) && near(pos(g, id), p)
        })
        .unwrap_or_else(|| panic!("no point at {p:?}: {g:?}"))
        .0
        .clone()
}
fn line_ids(g: &sk::Sketch) -> Vec<(sk::EntityId, sk::EntityId, sk::EntityId)> {
    g.entities
        .iter()
        .filter_map(|(id, e)| match e {
            sk::Entity::Line { start, end } => Some((id.clone(), start.clone(), end.clone())),
            _ => None,
        })
        .collect()
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
fn sorted(mut v: Vec<String>) -> Vec<String> {
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
/// Sketches-tab rows that carry a value: (id, kind, value).
fn dims(s: &Session) -> Vec<(String, String, f64)> {
    view(s)["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| !c["value"].is_null())
        .map(|c| {
            (
                c["id"].as_str().unwrap().to_owned(),
                c["kind"].as_str().unwrap().to_owned(),
                c["value"]["value"].as_f64().unwrap(),
            )
        })
        .collect()
}
fn annotation(s: &Session, id: &str) -> Value {
    view(s)["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id)
        .cloned()
        .unwrap_or(Value::Null)
}
fn new_dim(
    before: &[(String, String, f64)],
    after: &[(String, String, f64)],
) -> (String, String, f64) {
    let fresh: Vec<_> = after
        .iter()
        .filter(|d| !before.iter().any(|b| b.0 == d.0))
        .cloned()
        .collect();
    assert_eq!(fresh.len(), 1, "exactly one new dimension: {fresh:?}");
    fresh[0].clone()
}
const X_AXIS: Pt = (-220.0, 0.0);
const Y_AXIS: Pt = (0.0, -220.0);
fn shift_click_axis(s: &mut Session, axis: Pt) {
    click_mod(s, axis.0, axis.1, true);
}
fn place_point(s: &mut Session, p: Pt) {
    s.set_sketch_tool(SketchTool::Point);
    click(s, p.0, p.1);
}
fn draw_line(s: &mut Session, a: Pt, b: Pt) {
    s.set_sketch_tool(SketchTool::Polyline);
    click(s, a.0, a.1);
    click(s, b.0, b.1);
    s.escape();
}
/// The selectable geometry of a commit: its curves, or its free points when it has no curve.
/// Interpretation of AC42 "exactly the new geometry": a curve's defining points are handles of
/// that curve (AC22 keeps points and edges distinct), so they are not expected separately.
fn shape_of(g: &sk::Sketch, created: &[String]) -> Vec<String> {
    let curves: Vec<String> = created
        .iter()
        .filter(|id| {
            !matches!(
                g.entities[&sk::EntityId((*id).clone())],
                sk::Entity::Point { .. }
            )
        })
        .cloned()
        .collect();
    sorted(if curves.is_empty() {
        created.to_vec()
    } else {
        curves
    })
}
/// Empty selection in Select.
fn deselect(s: &mut Session) {
    s.set_sketch_tool(SketchTool::Select);
    s.escape();
    s.escape();
    assert!(selected(s).is_empty(), "deselect: {:?}", selected(s));
}
/// Press on `from` in Select and drag to `to` with intermediate moves.
fn drag(s: &mut Session, from: Pt, to: Pt) {
    let p = at(s, from.0, from.1);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    for i in 1..=6 {
        let f = f64::from(i) / 6.0;
        hover(
            s,
            from.0 + (to.0 - from.0) * f,
            from.1 + (to.1 - from.1) * f,
        );
    }
    let q = at(s, to.0, to.1);
    s.pointer_up(q, false, false);
}

// ---------------------------------------------------------------- AC42 return to Select

const P0: Pt = (100.0, 100.0);
const Q0: Pt = (180.0, 150.0);
const W0: Pt = (140.0, 100.0);

/// (name, tool, Alt presses, picks). Every first pick is `P0`, a point the shape owns.
fn shapes() -> Vec<(&'static str, SketchTool, usize, Vec<Pt>)> {
    vec![
        ("point", SketchTool::Point, 0, vec![P0]),
        ("rectangle", SketchTool::Rectangle, 0, vec![P0, Q0]),
        ("slot", SketchTool::Slot, 0, vec![P0, Q0, W0]),
        ("circle-centre", SketchTool::Circle, 0, vec![P0, Q0]),
        ("circle-3pt", SketchTool::Circle, 1, vec![P0, Q0, W0]),
        ("ellipse-centre", SketchTool::Ellipse, 0, vec![P0, Q0, W0]),
        ("ellipse-3pt", SketchTool::Ellipse, 1, vec![P0, Q0, W0]),
        ("arc-3pt", SketchTool::Arc, 0, vec![P0, Q0, W0]),
        ("arc-centre", SketchTool::Arc, 1, vec![P0, Q0, W0]),
    ]
}

/// A pre-existing selected point far away, so "exactly the new geometry" is observable.
fn with_old_selection() -> (Session, String) {
    let mut s = session();
    place_point(&mut s, (-150.0, -150.0));
    deselect(&mut s);
    let g = graph(&s);
    let old = point_at(&g, (-150.0, -150.0));
    assert!(s.sketch_select_entity(&old, false));
    (s, old.0)
}

#[test]
fn ac42_every_drawing_tool_returns_to_select_with_exactly_the_new_geometry() {
    let mut failures = Vec::new();
    for (name, t, alts, picks) in shapes() {
        let (mut s, _old) = with_old_selection();
        let before = ids(&graph(&s));
        let steps = s.undo_len();
        s.set_sketch_tool(t);
        for _ in 0..alts {
            alt(&mut s);
        }
        for (i, p) in picks.iter().enumerate() {
            if i > 0 {
                hover(&mut s, p.0, p.1);
            }
            click(&mut s, p.0, p.1);
        }
        let after = ids(&graph(&s));
        let created: Vec<String> = after
            .iter()
            .filter(|i| !before.contains(i))
            .cloned()
            .collect();
        let mut why = Vec::new();
        if created.is_empty() {
            failures.push(format!("{name}: nothing created"));
            continue;
        }
        if tool(&s) != "select" {
            why.push(format!("tool {:?}", tool(&s)));
        }
        let want = shape_of(&graph(&s), &created);
        if selected(&s) != want {
            why.push(format!("selection {:?} != created {want:?}", selected(&s)));
        }
        if s.undo_len() != steps + 1 {
            why.push(format!("history {} -> {}", steps, s.undo_len()));
        }
        // AC29: the new shape is draggable at once; the drag is its own single step.
        let g0 = graph(&s);
        let steps1 = s.undo_len();
        drag(&mut s, P0, (P0.0 - 30.0, P0.1 - 20.0));
        let g1 = graph(&s);
        let moved = created.iter().any(|id| {
            let id = sk::EntityId(id.clone());
            g0.point(&id).is_ok() && !near(pos(&g0, &id), pos(&g1, &id))
        });
        if !moved {
            why.push("drag of the new shape moved nothing".to_owned());
        }
        if s.undo_len() != steps1 + 1 {
            why.push(format!("drag history {} -> {}", steps1, s.undo_len()));
        }
        // the commit is one history step: two undos (drag, commit) restore the old entities
        s.undo();
        s.undo();
        if ids(&graph(&s)) != before {
            why.push("undo of the commit did not remove the shape in one step".to_owned());
        }
        if !why.is_empty() {
            failures.push(format!("{name}: {}", why.join("; ")));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn ac42_fillet_returns_to_select_with_the_new_arc_and_trimmed_members_selected() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 160.0);
    click(&mut s, 100.0, 100.0);
    click(&mut s, 160.0, 100.0);
    s.escape();
    deselect(&mut s);
    let g = graph(&s);
    let before = ids(&g);
    let ls: Vec<_> = line_ids(&g).into_iter().map(|l| l.0).collect();
    assert_eq!(ls.len(), 2);
    s.set_sketch_tool(SketchTool::Fillet);
    assert!(s.sketch_select_entity(&ls[0], false));
    assert!(s.sketch_select_entity(&ls[1], true));
    let steps = s.undo_len();
    // host Enter on the on-canvas Fillet entry
    assert!(s.sketch_complete_fields(&["10 mm"], &[true]).unwrap());
    assert_eq!(tool(&s), "select", "fillet returns to Select");
    assert_eq!(s.undo_len(), steps + 1, "one history step");
    let g1 = graph(&s);
    let created: Vec<String> = ids(&g1)
        .into_iter()
        .filter(|i| !before.contains(i))
        .collect();
    assert!(
        g1.entities
            .iter()
            .any(|(id, e)| created.contains(&id.0) && matches!(e, sk::Entity::Arc { .. })),
        "fillet created an arc: {created:?}"
    );
    assert_eq!(
        selected(&s),
        shape_of(&g1, &created),
        "exactly the new geometry is selected"
    );
}

#[test]
fn ac42_polyline_stays_while_the_chain_is_open_and_selects_the_whole_chain_on_escape() {
    let (mut s, _old) = with_old_selection();
    let before = ids(&graph(&s));
    let steps = s.undo_len();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 100.0);
    click(&mut s, 160.0, 100.0);
    assert_eq!(
        tool(&s),
        "polyline",
        "a committed segment keeps the chain open"
    );
    click(&mut s, 160.0, 160.0);
    assert_eq!(tool(&s), "polyline");
    let mid = s.undo_len();
    s.escape();
    assert_eq!(
        tool(&s),
        "select",
        "Escape after segments ends the chain in Select"
    );
    let created: Vec<String> = ids(&graph(&s))
        .into_iter()
        .filter(|i| !before.contains(i))
        .collect();
    assert_eq!(created.len(), 5, "two lines and three points: {created:?}");
    assert_eq!(
        selected(&s),
        shape_of(&graph(&s), &created),
        "the whole chain is selected"
    );
    assert_eq!(
        s.undo_len(),
        mid,
        "ending the chain and selecting add no history step"
    );
    assert!(s.undo_len() > steps);
}

#[test]
fn ac42_polyline_enter_ends_the_chain_in_select() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 100.0);
    click(&mut s, 160.0, 100.0);
    hover(&mut s, 160.0, 160.0);
    let steps = s.undo_len();
    key(&mut s, "Enter");
    assert_eq!(tool(&s), "select", "Enter ends the chain");
    let g = graph(&s);
    assert_eq!(line_ids(&g).len(), 1, "Enter commits nothing new");
    assert_eq!(
        selected(&s),
        shape_of(&g, &ids(&g)),
        "the chain is selected"
    );
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac42_polyline_closing_on_the_start_ends_the_chain_in_select() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    for p in [(100.0, 100.0), (160.0, 100.0), (160.0, 160.0)] {
        hover(&mut s, p.0, p.1);
        click(&mut s, p.0, p.1);
    }
    hover(&mut s, 100.0, 100.0);
    click(&mut s, 100.0, 100.0);
    assert_eq!(tool(&s), "select", "closing on the start ends the chain");
    let g = graph(&s);
    assert_eq!(line_ids(&g).len(), 3);
    assert_eq!(
        selected(&s),
        shape_of(&g, &ids(&g)),
        "the closed chain is selected"
    );
}

#[test]
fn ac42_escape_without_a_segment_keeps_the_previous_selection() {
    let (mut s, old) = with_old_selection();
    let steps = s.undo_len();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 100.0); // first pick only, nothing committed
    s.escape();
    s.escape();
    assert_eq!(tool(&s), "select");
    assert_eq!(selected(&s), vec![old], "previous selection is kept");
    assert_eq!(s.undo_len(), steps);
}

// ---------------------------------------------------------------- AC43 constraints return

struct Scene {
    s: Session,
    l1: sk::EntityId,
    l2: sk::EntityId,
    p1: sk::EntityId,
    p2: sk::EntityId,
    p3: sk::EntityId,
}
fn scene() -> Scene {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    draw_line(&mut s, (100.0, 180.0), (150.0, 220.0));
    for q in [(-100.0, 100.0), (-60.0, 130.0), (-20.0, 160.0)] {
        place_point(&mut s, q);
    }
    deselect(&mut s);
    let g = graph(&s);
    let mut ls = line_ids(&g);
    ls.sort_by(|a, b| pos(&g, &a.1).1.total_cmp(&pos(&g, &b.1).1));
    Scene {
        l1: ls[0].0.clone(),
        l2: ls[1].0.clone(),
        p1: point_at(&g, (-100.0, 100.0)),
        p2: point_at(&g, (-60.0, 130.0)),
        p3: point_at(&g, (-20.0, 160.0)),
        s,
    }
}
fn refs(sc: &Scene, code: &str) -> Vec<sk::EntityId> {
    code.split('+')
        .map(|p| match p {
            "l1" => sc.l1.clone(),
            "l2" => sc.l2.clone(),
            "p1" => sc.p1.clone(),
            "p2" => sc.p2.clone(),
            "p3" => sc.p3.clone(),
            other => panic!("{other}"),
        })
        .collect()
}
/// Endpoint ids of every line in `sel`, plus `sel`.
fn with_endpoints(g: &sk::Sketch, sel: &[sk::EntityId]) -> Vec<String> {
    let mut v: Vec<String> = sel.iter().map(|i| i.0.clone()).collect();
    for (l, a, b) in line_ids(g) {
        if sel.contains(&l) {
            v.push(a.0);
            v.push(b.0);
        }
    }
    v
}

const AC43_CASES: &[(&str, &str)] = &[
    ("coincident", "p1+p2"),
    ("horizontal", "l1"),
    ("vertical", "l1"),
    ("parallel", "l1+l2"),
    ("perpendicular", "l1+l2"),
    ("equal", "l1+l2"),
    ("fixed", "p1"),
    ("midpoint", "p1+l1"),
    ("symmetric", "p1+p2+l2"),
    ("dimension", "l1"),
    ("dimension", "p1+p2"),
    ("horizontal-distance", "l1"),
    ("horizontal-distance", "p1+p2"),
    ("vertical-distance", "l1"),
    ("vertical-distance", "p1+p2"),
];

#[test]
fn ac43_selection_first_apply_returns_to_select_with_the_referred_geometry() {
    let mut failures = Vec::new();
    for (t, code) in AC43_CASES {
        let mut sc = scene();
        sc.s.set_sketch_tool(SketchTool::Polyline); // a non-Select tool before the apply
        sc.s.set_sketch_tool(SketchTool::Select);
        let sel = refs(&sc, code);
        for (i, id) in sel.iter().enumerate() {
            assert!(sc.s.sketch_select_entity(id, i > 0));
        }
        let n0 = graph(&sc.s).constraints.len();
        let steps = sc.s.undo_len();
        let r = sc.s.sketch_rail_press(t);
        let mut why = Vec::new();
        if !matches!(r, Ok(true)) {
            why.push(format!("press {r:?}"));
        }
        if graph(&sc.s).constraints.len() <= n0 {
            why.push("nothing applied".to_owned());
        }
        if sc.s.undo_len() != steps + 1 {
            why.push(format!("history {} -> {}", steps, sc.s.undo_len()));
        }
        if tool(&sc.s) != "select" || !armed(&sc.s).is_null() {
            why.push(format!("tool {:?} armed {}", tool(&sc.s), armed(&sc.s)));
        }
        let allowed = with_endpoints(&graph(&sc.s), &sel);
        let now = selected(&sc.s);
        if now.is_empty() || now.iter().any(|x| !allowed.contains(x)) {
            why.push(format!("selection {now:?} not within {allowed:?}"));
        }
        let v = view(&sc.s);
        let entry_open = !v["canvas"]["entry"].is_null();
        if !v["edit_constraint"].is_null() && !entry_open {
            why.push("left in an edit state without an open entry".to_owned());
        }
        if !t.contains("dimension") && !t.contains("distance") && entry_open {
            why.push("a constraint opened a value entry".to_owned());
        }
        if !why.is_empty() {
            failures.push(format!("{t} [{code}]: {}", why.join("; ")));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn ac43_pick_completion_returns_to_select() {
    // Armed with nothing selected, then picks on the canvas complete the relationship.
    let cases: &[(&str, &[Pt])] = &[
        ("horizontal", &[(130.0, 110.0)]),
        ("fixed", &[(-100.0, 100.0)]),
        ("coincident", &[(-100.0, 100.0), (-60.0, 130.0)]),
        ("horizontal-distance", &[(-100.0, 100.0), (-60.0, 130.0)]),
        ("vertical-distance", &[(-100.0, 100.0), (-60.0, 130.0)]),
    ];
    let mut failures = Vec::new();
    for (t, picks) in cases {
        let mut sc = scene();
        let n0 = graph(&sc.s).constraints.len();
        let steps = sc.s.undo_len();
        let _ = sc.s.sketch_rail_press(t);
        if armed(&sc.s) != **t {
            failures.push(format!("{t}: not armed: {}", armed(&sc.s)));
            continue;
        }
        for p in *picks {
            click(&mut sc.s, p.0, p.1);
        }
        // a dimension may place its label with a further click
        if graph(&sc.s).constraints.len() == n0 && t.contains("distance") {
            click(&mut sc.s, -40.0, 60.0);
        }
        let mut why = Vec::new();
        if graph(&sc.s).constraints.len() != n0 + 1 {
            why.push("not applied".to_owned());
        }
        if sc.s.undo_len() != steps + 1 {
            why.push(format!("history {} -> {}", steps, sc.s.undo_len()));
        }
        if tool(&sc.s) != "select" || !armed(&sc.s).is_null() {
            why.push(format!("tool {:?} armed {}", tool(&sc.s), armed(&sc.s)));
        }
        if !why.is_empty() {
            failures.push(format!("{t}: {}", why.join("; ")));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn ac43_refused_use_leaves_the_tool_active() {
    let cases: &[(&str, &str)] = &[
        ("parallel", "l1"),
        ("coincident", "l1+l2"),
        ("midpoint", "p1+p2"),
        ("dimension", "p1+p2+p3"),
        ("horizontal-distance", "p1+p2+p3"),
        ("vertical-distance", "p1+p2+p3"),
        ("horizontal-distance", "l1+l2"),
    ];
    let mut failures = Vec::new();
    for (t, code) in cases {
        let mut sc = scene();
        let sel = refs(&sc, code);
        for (i, id) in sel.iter().enumerate() {
            assert!(sc.s.sketch_select_entity(id, i > 0));
        }
        let n0 = graph(&sc.s).constraints.len();
        let steps = sc.s.undo_len();
        let _ = sc.s.sketch_rail_press(t);
        let v = view(&sc.s);
        let mut why = Vec::new();
        if graph(&sc.s).constraints.len() != n0 || sc.s.undo_len() != steps {
            why.push("something applied".to_owned());
        }
        if v["constraint_tool"] != *t {
            why.push(format!("tool not active: {}", v["constraint_tool"]));
        }
        let hint = v["canvas"]["mode_label"]["text"]
            .as_str()
            .is_some_and(|x| !x.is_empty())
            || !v["error"].is_null();
        if !hint {
            why.push("no hint".to_owned());
        }
        if !why.is_empty() {
            failures.push(format!("{t} [{code}]: {}", why.join("; ")));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

// ---------------------------------------------------------------- AC44 tools and kinds

#[test]
fn ac44_rail_groups_the_two_axis_tools_after_dimension_with_names() {
    let s = scene().s;
    let v = view(&s);
    let rail = v["rail"].as_array().unwrap();
    let names: Vec<&str> = rail.iter().map(|r| r["name"].as_str().unwrap()).collect();
    let d = names.iter().position(|n| *n == "dimension").unwrap();
    assert_eq!(
        &names[d..d + 3],
        &["dimension", "horizontal-distance", "vertical-distance"]
    );
    for (n, label) in [
        ("horizontal-distance", "Horizontal distance"),
        ("vertical-distance", "Vertical distance"),
    ] {
        let r = rail.iter().find(|r| r["name"] == n).unwrap();
        assert_eq!(r["label"], label);
        assert!(r["tooltip"].as_str().unwrap().starts_with(label));
    }
}

/// Selection-first apply; returns the new dimension (kind, value).
fn apply(s: &mut Session, sel: &[sk::EntityId], axes: &[Pt], t: &str) -> (String, String, f64) {
    s.set_sketch_tool(SketchTool::Select);
    for (i, id) in sel.iter().enumerate() {
        assert!(s.sketch_select_entity(id, i > 0));
    }
    for a in axes {
        shift_click_axis(s, *a);
    }
    let before = dims(s);
    let steps = s.undo_len();
    let r = s.sketch_rail_press(t);
    assert!(matches!(r, Ok(true)), "{t}: {r:?} {}", view(s)["canvas"]);
    assert_eq!(s.undo_len(), steps + 1, "{t}: one history step");
    new_dim(&before, &dims(s))
}

#[test]
fn ac44_axis_distances_for_points_line_origin_and_axes() {
    // p = (-60, 130), q = (-100, 100); line (100,100)-(160,120)
    let cases: &[(&str, &str, &str, f64)] = &[
        ("pp-h", "horizontal-distance", "horizontal-distance", 40.0),
        ("pp-v", "vertical-distance", "vertical-distance", 30.0),
        ("line-h", "horizontal-distance", "horizontal-distance", 60.0),
        ("line-v", "vertical-distance", "vertical-distance", 20.0),
        ("line-aligned", "dimension", "", 60.0_f64.hypot(20.0)),
        (
            "origin-h",
            "horizontal-distance",
            "horizontal-distance",
            60.0,
        ),
        ("origin-v", "vertical-distance", "vertical-distance", 130.0),
        (
            "yaxis-h",
            "horizontal-distance",
            "horizontal-distance",
            60.0,
        ),
        ("xaxis-v", "vertical-distance", "vertical-distance", 130.0),
    ];
    let mut failures = Vec::new();
    for (case, t, kind, want) in cases {
        let mut sc = scene();
        let g = graph(&sc.s);
        let (sel, axes): (Vec<sk::EntityId>, Vec<Pt>) = match *case {
            "pp-h" | "pp-v" => (vec![sc.p1.clone(), sc.p2.clone()], vec![]),
            "line-h" | "line-v" | "line-aligned" => (vec![sc.l1.clone()], vec![]),
            "origin-h" | "origin-v" => (vec![sc.p2.clone(), g.origin.clone()], vec![]),
            "yaxis-h" => (vec![sc.p2.clone()], vec![Y_AXIS]),
            "xaxis-v" => (vec![sc.p2.clone()], vec![X_AXIS]),
            _ => unreachable!(),
        };
        sc.s.set_sketch_tool(SketchTool::Select);
        for (i, id) in sel.iter().enumerate() {
            if !sc.s.sketch_select_entity(id, i > 0) {
                failures.push(format!("{case}: cannot select {id:?}"));
            }
        }
        for a in &axes {
            shift_click_axis(&mut sc.s, *a);
        }
        let before = dims(&sc.s);
        let steps = sc.s.undo_len();
        let r = sc.s.sketch_rail_press(t);
        let after = dims(&sc.s);
        let fresh: Vec<_> = after.iter().filter(|d| !before.contains(d)).collect();
        let mut why = Vec::new();
        if !matches!(r, Ok(true)) || fresh.len() != 1 {
            why.push(format!("press {r:?}, new {fresh:?}"));
        } else {
            let d = fresh[0];
            if !kind.is_empty() && d.1 != *kind {
                why.push(format!("kind {}", d.1));
            }
            if kind.is_empty() && d.1.contains("horizontal") | d.1.contains("vertical") {
                why.push(format!("aligned Dimension became {}", d.1));
            }
            if (d.2 - want).abs() > 1e-6 {
                why.push(format!("value {} want {want}", d.2));
            }
            if sc.s.undo_len() != steps + 1 {
                why.push("history".to_owned());
            }
        }
        if !why.is_empty() {
            failures.push(format!("{case}: {}", why.join("; ")));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn ac44_alt_does_not_turn_dimension_into_an_axis_distance() {
    let mut sc = scene();
    let _ = sc.s.sketch_rail_press("dimension");
    alt(&mut sc.s);
    alt(&mut sc.s);
    assert_eq!(armed(&sc.s), "dimension", "Alt keeps the Dimension tool");
    let l1 = sc.l1.clone();
    let d = apply(&mut sc.s, &[l1], &[], "dimension");
    assert!(
        !d.1.contains("horizontal") && !d.1.contains("vertical"),
        "Alt must not choose horizontal/vertical: {d:?}"
    );
}

// ---------------------------------------------------------------- AC45 annotation and driving

fn diagonal() -> (Session, sk::EntityId, sk::EntityId, sk::EntityId) {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 140.0));
    deselect(&mut s);
    let g = graph(&s);
    let (l, a, b) = line_ids(&g)[0].clone();
    (s, l, a, b)
}

#[test]
fn ac45_horizontal_value_drives_projection_only_and_removes_one_dof() {
    let (mut s, l, a, b) = diagonal();
    let dof0 = dof(&s);
    let (id, kind, v) = apply(&mut s, &[l], &[], "horizontal-distance");
    assert_eq!(kind, "horizontal-distance");
    assert!((v - 60.0).abs() < 1e-6);
    assert_eq!(dof(&s), dof0 - 1, "DOF reduced by exactly one");
    let g0 = graph(&s);
    let dy0 = pos(&g0, &b).1 - pos(&g0, &a).1;
    let steps = s.undo_len();
    assert!(
        s.sketch_dimension_text(&id.clone().into(), "30 mm", false)
            .unwrap()
    );
    assert_eq!(s.undo_len(), steps + 1);
    let g1 = graph(&s);
    let (pa, pb) = (pos(&g1, &a), pos(&g1, &b));
    assert!((pb.0 - pa.0 - 30.0).abs() < 1e-6, "dx {}", pb.0 - pa.0);
    assert!(
        (pb.1 - pa.1 - dy0).abs() < 1e-6,
        "dy changed {} -> {}",
        dy0,
        pb.1 - pa.1
    );
    assert_eq!(dof(&s), dof0 - 1);
    // undo restores the old value and geometry
    s.undo();
    let g2 = graph(&s);
    assert!((pos(&g2, &b).0 - pos(&g2, &a).0 - 60.0).abs() < 1e-6);
}

#[test]
fn ac45_vertical_value_drives_projection_only() {
    let (mut s, l, a, b) = diagonal();
    let (id, kind, v) = apply(&mut s, &[l], &[], "vertical-distance");
    assert_eq!(kind, "vertical-distance");
    assert!((v - 40.0).abs() < 1e-6);
    let g0 = graph(&s);
    let dx0 = pos(&g0, &b).0 - pos(&g0, &a).0;
    assert!(s.sketch_dimension_text(&id.into(), "15 mm", false).unwrap());
    let g1 = graph(&s);
    let (pa, pb) = (pos(&g1, &a), pos(&g1, &b));
    assert!((pb.1 - pa.1 - 15.0).abs() < 1e-6);
    assert!((pb.0 - pa.0 - dx0).abs() < 1e-6);
}

#[test]
fn ac45_annotation_names_the_kind_and_reference_is_parenthesised() {
    let (mut s, l, _, _) = diagonal();
    let (h, _, _) = apply(&mut s, &[l.clone()], &[], "horizontal-distance");
    let (v, _, _) = apply(&mut s, &[l], &[], "vertical-distance");
    let ah = annotation(&s, &h);
    let av = annotation(&s, &v);
    let th = ah["text"].as_str().unwrap();
    let tv = av["text"].as_str().unwrap();
    assert!(
        th.contains('↔') && th.contains("60") && th.contains("mm"),
        "{th}"
    );
    assert!(
        tv.contains('↕') && tv.contains("40") && tv.contains("mm"),
        "{tv}"
    );
    assert_eq!(ah["dimension"], true);
    for (a, word) in [(&ah, "Horizontal"), (&av, "Vertical")] {
        assert!(
            a["name"].as_str().unwrap().contains(word),
            "accessible name {a}"
        );
    }
    // Sketches rows carry the kind
    let kinds: Vec<String> = dims(&s).into_iter().map(|d| d.1).collect();
    assert!(kinds.contains(&"horizontal-distance".to_owned()));
    assert!(kinds.contains(&"vertical-distance".to_owned()));
    // reference variant
    assert!(
        s.sketch_dimension_text(&h.clone().into(), "60 mm", true)
            .unwrap()
    );
    let r = annotation(&s, &h)["text"].as_str().unwrap().to_owned();
    assert!(
        r.starts_with('(') && r.ends_with(')') && r.contains('↔'),
        "{r}"
    );
}

#[test]
fn ac45_double_click_opens_the_edit_and_expressions_are_accepted() {
    let (mut s, l, a, b) = diagonal();
    let (h, _, _) = apply(&mut s, &[l], &[], "horizontal-distance");
    s.sketch_close_dimension_edit();
    deselect(&mut s);
    assert!(!annotation(&s, &h).is_null());
    // the host's double-click on the annotation text (SketchAnnotations.tsx)
    assert!(s.sketch_edit_dimension(&h.clone().into()));
    assert_eq!(
        view(&s)["canvas"]["entry"]["constraint"],
        h,
        "double-click opens the entry at the dimension"
    );
    assert!(
        s.sketch_dimension_text(&h.into(), "10 mm + 2 * 5 mm", false)
            .unwrap()
    );
    let g = graph(&s);
    assert!((pos(&g, &b).0 - pos(&g, &a).0 - 20.0).abs() < 1e-6);
}

#[test]
fn ac45_save_reopen_keeps_the_axis_dimensions_and_convert_omits_annotations() {
    let (mut s, l, a, b) = diagonal();
    let (h, _, _) = apply(&mut s, &[l.clone()], &[], "horizontal-distance");
    let (v, _, _) = apply(&mut s, &[l], &[], "vertical-distance");
    assert!(
        s.sketch_dimension_text(&h.clone().into(), "30 mm", false)
            .unwrap()
    );
    assert!(
        s.sketch_dimension_text(&v.clone().into(), "25 mm", false)
            .unwrap()
    );
    let bytes = s.pack("rev4 reopen").unwrap();
    let g0 = graph(&s);
    let mut r = Session::open(6642, &bytes).unwrap();
    r.set_workbench(Workbench::Sketch);
    let d: BTreeMap<String, (String, f64)> =
        dims(&r).into_iter().map(|x| (x.0, (x.1, x.2))).collect();
    assert_eq!(d[&h].0, "horizontal-distance");
    assert!((d[&h].1 - 30.0).abs() < 1e-6);
    assert_eq!(d[&v].0, "vertical-distance");
    assert!((d[&v].1 - 25.0).abs() < 1e-6);
    let g1 = graph(&r);
    for p in [&a, &b] {
        assert!(near(pos(&g0, p), pos(&g1, p)));
    }
    let id = r.active_sketch().unwrap();
    r.sketch_convert_to_path().unwrap();
    let doc = document(&r);
    match doc.object(id) {
        Some(curvyo_document_core::ObjectSnapshot::Path(p)) => {
            let text = format!("{p:?}");
            assert!(!text.contains('↔') && !text.contains('↕'));
        }
        other => panic!("converted to {other:?}"),
    }
}

// ---------------------------------------------------------------- AC46 side and value checks

#[test]
fn ac46_side_is_kept_zero_aligns_negative_and_invalid_refuse() {
    for (t, axis) in [
        ("horizontal-distance", 0usize),
        ("vertical-distance", 1usize),
    ] {
        // q is left of / above p (negative signed difference from p to q)
        let mut s = session();
        place_point(&mut s, (150.0, 170.0));
        place_point(&mut s, (90.0, 110.0));
        deselect(&mut s);
        let g = graph(&s);
        let p = point_at(&g, (150.0, 170.0));
        let q = point_at(&g, (90.0, 110.0));
        let (id, _, v) = apply(&mut s, &[p.clone(), q.clone()], &[], t);
        assert!((v - 60.0).abs() < 1e-6, "{t}: absolute value, got {v}");
        let c = |s: &Session| {
            let g = graph(s);
            let (a, b) = (pos(&g, &p), pos(&g, &q));
            if axis == 0 { a.0 - b.0 } else { a.1 - b.1 }
        };
        assert!(
            s.sketch_dimension_text(&id.clone().into(), "25 mm", false)
                .unwrap()
        );
        assert!((c(&s) - 25.0).abs() < 1e-6, "{t}: side kept, got {}", c(&s));
        assert!(
            s.sketch_dimension_text(&id.clone().into(), "0 mm", false)
                .unwrap()
        );
        assert!(c(&s).abs() < 1e-6, "{t}: zero aligns");
        assert!(
            s.sketch_dimension_text(&id.clone().into(), "40 mm", false)
                .unwrap()
        );
        assert!(
            (c(&s) - 40.0).abs() < 1e-6,
            "{t}: side kept after zero, got {}",
            c(&s)
        );
        let steps = s.undo_len();
        assert!(s.sketch_edit_dimension(&id.clone().into()));
        assert!(
            s.sketch_dimension_text(&id.clone().into(), "-5 mm", false)
                .is_err(),
            "{t}"
        );
        assert!(
            s.sketch_dimension_text(&id.clone().into(), "nonsense_name", false)
                .is_err()
        );
        assert_eq!(s.undo_len(), steps, "{t}: refusals add no step");
        assert!((c(&s) - 40.0).abs() < 1e-6);
        assert_eq!(
            view(&s)["canvas"]["entry"]["constraint"],
            id,
            "{t}: entry kept after refusal"
        );
    }
}

// ---------------------------------------------------------------- AC47 two-point selection

#[test]
fn ac47_shift_click_selects_two_endpoints_without_the_line() {
    let (mut s, _l, a, b) = diagonal();
    s.set_sketch_tool(SketchTool::Select);
    click(&mut s, 100.0, 100.0);
    click_mod(&mut s, 160.0, 140.0, true);
    assert_eq!(selected(&s), sorted(vec![a.0.clone(), b.0.clone()]));
    // Ctrl removes (Ctrl marquee, the existing Curvyo convention of AC22)
    s.modifiers_changed(false, true, false);
    let (p, q) = (at(&s, 90.0, 90.0), at(&s, 110.0, 110.0));
    s.pointer_down(p, false);
    s.pointer_hover(q, false, true);
    s.pointer_up(q, false, true);
    s.modifiers_changed(false, false, false);
    assert_eq!(selected(&s), vec![b.0.clone()], "Ctrl marquee removes");
}

#[test]
fn ac47_marquee_selects_exactly_two_points() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 140.0));
    place_point(&mut s, (-100.0, 100.0));
    place_point(&mut s, (-60.0, 130.0));
    deselect(&mut s);
    let g = graph(&s);
    let p = point_at(&g, (-100.0, 100.0));
    let q = point_at(&g, (-60.0, 130.0));
    let steps = s.undo_len();
    drag(&mut s, (-120.0, 80.0), (-40.0, 150.0));
    assert_eq!(selected(&s), sorted(vec![p.0, q.0]), "marquee pair");
    assert_eq!(s.undo_len(), steps, "a marquee is not an edit");
    assert_eq!(graph(&s), g, "a marquee moves nothing");
}

// ---------------------------------------------------------------- AC48 same two points

#[test]
fn ac48_three_tools_on_the_same_two_points() {
    let mut sc = scene();
    let (p1, p2) = (sc.p1.clone(), sc.p2.clone());
    let want = [
        ("dimension", 40.0_f64.hypot(30.0)),
        ("horizontal-distance", 40.0),
        ("vertical-distance", 30.0),
    ];
    let mut kinds = Vec::new();
    for (t, v) in want {
        let d = apply(&mut sc.s, &[p1.clone(), p2.clone()], &[], t);
        assert!((d.2 - v).abs() < 1e-6, "{t}: {d:?}");
        assert_eq!(tool(&sc.s), "select", "{t}: returns to Select");
        assert!(armed(&sc.s).is_null(), "{t}");
        kinds.push(d.1);
        sc.s.sketch_close_dimension_edit();
    }
    assert_eq!(kinds[1], "horizontal-distance");
    assert_eq!(kinds[2], "vertical-distance");
    assert!(
        !kinds[0].contains("horizontal") && !kinds[0].contains("vertical"),
        "{kinds:?}"
    );
    assert_ne!(kinds[0], kinds[1]);
}

#[test]
fn ac48_selection_built_by_shift_click_then_applied_at_once() {
    let mut sc = scene();
    sc.s.set_sketch_tool(SketchTool::Select);
    click(&mut sc.s, -100.0, 100.0);
    click_mod(&mut sc.s, -60.0, 130.0, true);
    assert_eq!(
        selected(&sc.s),
        sorted(vec![sc.p1.0.clone(), sc.p2.0.clone()])
    );
    let before = dims(&sc.s);
    let steps = sc.s.undo_len();
    assert!(sc.s.sketch_rail_press("horizontal-distance").unwrap());
    assert_eq!(sc.s.undo_len(), steps + 1);
    let d = new_dim(&before, &dims(&sc.s));
    assert_eq!(d.1, "horizontal-distance");
    assert!((d.2 - 40.0).abs() < 1e-6);
    assert_eq!(tool(&sc.s), "select");
}

// ---------------------------------------------------------------- white-box edges

#[test]
fn edge_point_left_of_origin_keeps_its_side_and_reports_an_absolute_value() {
    let mut s = session();
    place_point(&mut s, (-70.0, -90.0));
    deselect(&mut s);
    let g = graph(&s);
    let p = point_at(&g, (-70.0, -90.0));
    let o = g.origin.clone();
    let (h, _, hv) = apply(&mut s, &[p.clone(), o.clone()], &[], "horizontal-distance");
    let (v, _, vv) = apply(&mut s, &[o, p.clone()], &[], "vertical-distance");
    assert!(
        (hv - 70.0).abs() < 1e-6 && (vv - 90.0).abs() < 1e-6,
        "{hv} {vv}"
    );
    assert!(s.sketch_dimension_text(&h.into(), "20 mm", false).unwrap());
    assert!(s.sketch_dimension_text(&v.into(), "35 mm", false).unwrap());
    let q = pos(&graph(&s), &p);
    assert!(
        (q.0 + 20.0).abs() < 1e-6 && (q.1 + 35.0).abs() < 1e-6,
        "{q:?}"
    );
}

#[test]
fn edge_axis_parallel_to_the_measured_direction_is_refused() {
    // Horizontal distance from a point to the X axis is not defined (the axis is horizontal).
    let mut failures = Vec::new();
    for (t, axis) in [
        ("horizontal-distance", X_AXIS),
        ("vertical-distance", Y_AXIS),
    ] {
        let mut sc = scene();
        let p2 = sc.p2.clone();
        sc.s.set_sketch_tool(SketchTool::Select);
        assert!(sc.s.sketch_select_entity(&p2, false));
        shift_click_axis(&mut sc.s, axis);
        let n0 = graph(&sc.s).constraints.len();
        let steps = sc.s.undo_len();
        let _ = sc.s.sketch_rail_press(t);
        if graph(&sc.s).constraints.len() != n0 || sc.s.undo_len() != steps {
            failures.push(format!("{t}: applied to a parallel axis {:?}", dims(&sc.s)));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn edge_coincident_points_start_at_zero_and_huge_values_do_not_panic() {
    let mut s = session();
    place_point(&mut s, (50.0, 60.0));
    place_point(&mut s, (50.0, 120.0));
    deselect(&mut s);
    let g = graph(&s);
    let p = point_at(&g, (50.0, 60.0));
    let q = point_at(&g, (50.0, 120.0));
    let (h, _, v) = apply(&mut s, &[p.clone(), q.clone()], &[], "horizontal-distance");
    assert!(v.abs() < 1e-9, "aligned points measure zero: {v}");
    assert!(
        s.sketch_dimension_text(&h.clone().into(), "15 mm", false)
            .unwrap()
    );
    let g1 = graph(&s);
    assert!(((pos(&g1, &q).0 - pos(&g1, &p).0).abs() - 15.0).abs() < 1e-6);
    let steps = s.undo_len();
    let r = s.sketch_dimension_text(&h.clone().into(), "1e12 mm", false);
    if r.is_ok() {
        let g2 = graph(&s);
        assert!(pos(&g2, &q).0.is_finite());
    } else {
        assert_eq!(s.undo_len(), steps);
    }
    s.undo();
    s.undo();
    s.undo();
    assert!(
        graph(&s).constraints.is_empty(),
        "undo removes the dimension"
    );
}
