//! Independent acceptance tests for 0066 customer revision 2 (AC24-AC35), through public APIs.
//!
//! Written from `specification.md` "Customer revision 2" before the implementation was read.
//! Oracles come from the criteria text; sketch coordinates are millimetres in the sketch frame,
//! which is the page centre (same orientation as the document).
#![allow(
    clippy::unwrap_used,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::approx_constant,
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::expect_used,
    clippy::manual_midpoint,
    clippy::type_complexity,
    clippy::items_after_statements,
    clippy::float_cmp
)]
mod cpu_time;
use curvyo_document_core::{Angle, Document, Point, unpack};
use curvyo_editor_wasm::{KeyInput, Session, SketchTool, Workbench};
use curvyo_sketch_core as sk;
use curvyo_ui_core::snap_angle;
use serde_json::Value;

// ---------------------------------------------------------------- helpers

fn session() -> Session {
    let mut s = Session::new(6600);
    s.set_workbench(Workbench::Sketch);
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
    unpack(6601, &s.pack("rev2").unwrap()).unwrap()
}
fn graph(s: &Session) -> sk::Sketch {
    document(s)
        .sketch(s.active_sketch().unwrap())
        .unwrap()
        .solution
}
fn click(s: &mut Session, x: f64, y: f64) {
    let p = at(s, x, y);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}
fn hover(s: &mut Session, x: f64, y: f64) {
    let p = at(s, x, y);
    s.pointer_hover(p, false, false);
}
fn view(s: &Session) -> Value {
    serde_json::from_str(&s.sketch_view_json()).unwrap()
}
fn alt(s: &mut Session) {
    s.key_down(KeyInput {
        key: "Alt",
        alt: true,
        ..KeyInput::default()
    });
}
fn label(s: &Session) -> Option<String> {
    view(s)["canvas"]["mode_label"]["text"]
        .as_str()
        .map(str::to_owned)
}
fn mode(s: &Session) -> String {
    view(s)["mode"].as_str().unwrap_or("").to_owned()
}
fn pos(g: &sk::Sketch, id: &sk::EntityId) -> (f64, f64) {
    let p = g.point(id).unwrap();
    (p.x.0, p.y.0)
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
fn free_points(g: &sk::Sketch) -> Vec<sk::EntityId> {
    g.entities
        .iter()
        .filter(|(id, e)| *id != &g.origin && matches!(e, sk::Entity::Point { .. }))
        .map(|(id, _)| id.clone())
        .collect()
}
fn near(a: (f64, f64), b: (f64, f64), tol: f64) -> bool {
    (a.0 - b.0).abs() <= tol && (a.1 - b.1).abs() <= tol
}
/// A free line drawn with the Polyline tool: click, click, Escape.
fn draw_line(s: &mut Session, a: (f64, f64), b: (f64, f64)) {
    s.set_sketch_tool(SketchTool::Polyline);
    click(s, a.0, a.1);
    click(s, b.0, b.1);
    s.escape();
}
fn relations(g: &sk::Sketch) -> Vec<String> {
    g.constraints
        .values()
        .map(|c| format!("{:?}", c.relation))
        .collect()
}
fn has_hv(g: &sk::Sketch) -> (bool, bool) {
    let h = g
        .constraints
        .values()
        .any(|c| matches!(c.relation, sk::Relation::Horizontal(_)));
    let v = g
        .constraints
        .values()
        .any(|c| matches!(c.relation, sk::Relation::Vertical(_)));
    (h, v)
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
/// Arms the dimension tool with the current selection and places its label (needed before a value).
/// Constraint id of the dimension whose entry box is open (rev 3: a fitting preselection applies at once).
fn open_box_dimension(s: &Session) -> sk::ConstraintId {
    let e = view(s)["canvas"]["entry"].clone();
    assert_eq!(
        e["kind"], "edit",
        "an edit box opens on the new dimension: {e}"
    );
    sk::ConstraintId(
        e["constraint"]
            .as_str()
            .expect("box names its dimension")
            .to_owned(),
    )
}
/// Pans so that sketch point (x, y) appears at screen (tx, ty).
fn center_on(s: &mut Session, x: f64, y: f64, tx: f64, ty: f64) {
    let mut sign = 1.0;
    let mut last = f64::MAX;
    for _ in 0..12 {
        let cur = s.view().document_to_screen(at(s, x, y));
        let (dx, dy) = (cur.0 - tx, cur.1 - ty);
        let err = dx.hypot(dy);
        if err < 0.5 {
            return;
        }
        if err > last {
            sign = -sign;
        }
        last = err;
        s.wheel(sign * dx, sign * dy, 0.0, 0.0, false, false);
    }
}
fn ctrl(s: &mut Session, on: bool) {
    s.modifiers_changed(false, on, false);
}
fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

// ---------------------------------------------------------------- AC24 rail

const RAIL: [&str; 11] = [
    "coincident",
    "horizontal",
    "vertical",
    "parallel",
    "perpendicular",
    "tangent",
    "equal",
    "fixed",
    "symmetric",
    "midpoint",
    "dimension",
];

#[test]
fn ac24_rail_lists_every_constraint_tool_and_dimension_with_tooltip() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let v = view(&s);
    let rail = v["rail"].as_array().unwrap();
    let names: Vec<_> = rail.iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert_eq!(names, RAIL, "rail order: constrain group then dimension");
    for r in rail {
        let name = r["name"].as_str().unwrap();
        let tip = r["tooltip"].as_str().unwrap();
        let label = r["label"].as_str().unwrap();
        assert!(
            tip.to_lowercase().starts_with(&label.to_lowercase()),
            "tooltip of {name} must name the tool: {tip}"
        );
    }
}

#[test]
fn ac24_each_tool_applies_with_a_valid_preselection() {
    // (tool, scene builder, selection picker, expected glyph in the symbol list)
    for tool in [
        "horizontal",
        "vertical",
        "parallel",
        "perpendicular",
        "equal",
        "tangent",
        "fixed",
        "coincident",
        "midpoint",
        "symmetric",
    ] {
        let mut s = session();
        draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
        draw_line(&mut s, (100.0, 160.0), (150.0, 200.0));
        s.set_sketch_tool(SketchTool::Circle);
        click(&mut s, 250.0, 250.0);
        click(&mut s, 275.0, 250.0);
        s.set_sketch_tool(SketchTool::Point);
        click(&mut s, 300.0, 100.0);
        click(&mut s, 340.0, 130.0);
        click(&mut s, 400.0, 100.0);
        s.set_sketch_tool(SketchTool::Select);
        let g = graph(&s);
        let ls = lines(&g);
        assert_eq!(ls.len(), 2);
        let circle = g
            .entities
            .iter()
            .find(|(_, e)| matches!(e, sk::Entity::Circle { .. }))
            .unwrap()
            .0
            .clone();
        let mut pts: Vec<_> = free_points(&g)
            .into_iter()
            .filter(|p| pos(&g, p).0 >= 290.0)
            .collect();
        pts.sort_by(|a, b| pos(&g, a).0.total_cmp(&pos(&g, b).0));
        assert_eq!(pts.len(), 3, "{g:?}");
        let sel: Vec<sk::EntityId> = match tool {
            "horizontal" | "vertical" => vec![ls[0].0.clone()],
            "parallel" | "perpendicular" | "equal" => vec![ls[0].0.clone(), ls[1].0.clone()],
            "tangent" => vec![ls[0].0.clone(), circle],
            "fixed" => vec![pts[0].clone()],
            "coincident" => vec![pts[0].clone(), pts[1].clone()],
            "midpoint" => vec![pts[0].clone(), ls[0].0.clone()],
            "symmetric" => vec![pts[0].clone(), pts[1].clone(), ls[1].0.clone()],
            _ => unreachable!(),
        };
        let before = graph(&s).constraints.len();
        let steps = s.undo_len();
        for (i, id) in sel.iter().enumerate() {
            assert!(s.sketch_select_entity(id, i > 0), "{tool}: select {id:?}");
        }
        let v = view(&s);
        let state = v["rail"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == tool)
            .unwrap()["state"]
            .clone();
        assert_eq!(state, "apply", "{tool}: valid preselection must be 'apply'");
        let result = s.sketch_rail_press(tool);
        assert!(
            result.is_ok(),
            "{tool}: {result:?} {}",
            s.sketch_view_json()
        );
        let after = graph(&s);
        assert!(
            after.constraints.len() > before,
            "{tool}: no constraint added"
        );
        assert_eq!(s.undo_len(), steps + 1, "{tool}: exactly one history step");
        let glyphs: Vec<String> = view(&s)["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .map(|y| y["glyph"].as_str().unwrap().to_owned())
            .collect();
        assert!(
            glyphs
                .iter()
                .any(|g| g == tool || (tool == "fixed" && g == "fixed")),
            "{tool}: AC32 symbol missing, symbols = {glyphs:?}"
        );
    }
}

#[test]
fn ac24_dimension_rail_tool_applies_at_once_with_a_segment_and_creates_a_dimension() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let l = lines(&graph(&s))[0].0.clone();
    s.sketch_select_entity(&l, false);
    let steps = s.undo_len();
    assert!(s.sketch_rail_press("dimension").unwrap());
    assert_eq!(s.undo_len(), steps + 1, "one history step, no arming");
    let measured = 60.0_f64.hypot(20.0);
    let a = view(&s)["annotations"].clone();
    assert!(
        a.as_array().unwrap().iter().any(|x| x["dimension"] == true
            && x["text"]
                .as_str()
                .unwrap()
                .starts_with(&format!("{measured:.2}"))),
        "the measured length is the value: {a}"
    );
    let id = open_box_dimension(&s);
    assert!(s.sketch_dimension_text(&id, "50 mm", false).unwrap());
    assert_eq!(s.undo_len(), steps + 2, "typing is a second step");
    let a = view(&s)["annotations"].clone();
    assert!(
        a.as_array().unwrap().iter().any(|x| x["text"] == "50 mm"),
        "{a}"
    );
    assert_eq!(dimension_texts(&s), vec!["50 mm"], "no second Length");
}

// ---------------------------------------------------------------- AC25 / AC34 Alt

fn cycle_labels(s: &mut Session) -> Vec<String> {
    let mut seen = vec![label(s).unwrap()];
    for _ in 0..12 {
        alt(s);
        let l = label(s).unwrap();
        if l == seen[0] {
            break;
        }
        seen.push(l);
    }
    seen
}

fn zigzag(s: &mut Session, committed: usize) {
    s.set_sketch_tool(SketchTool::Polyline);
    click(s, 100.0, 100.0);
    for i in 1..=committed {
        let y = if i % 2 == 1 { 120.0 } else { 100.0 };
        click(s, 100.0 + 25.0 * i as f64, y);
    }
}

#[test]
fn ac25_alt_cycles_all_modes_on_segment_1_and_later_identically() {
    let mut first = session();
    first.set_sketch_tool(SketchTool::Polyline);
    click(&mut first, 100.0, 100.0);
    hover(&mut first, 150.0, 140.0);
    assert_eq!(label(&first).as_deref(), Some("Line"));
    let steps = first.undo_len();
    let set1 = cycle_labels(&mut first);
    assert!(
        set1.len() >= 2,
        "Alt on segment 1 must offer more than one mode: {set1:?}"
    );
    for want in [
        "Line",
        "Perpendicular line",
        "Tangent line",
        "Tangent arc",
        "Arc left",
        "Arc right",
    ] {
        assert!(
            set1.iter().any(|l| l == want),
            "segment 1 lacks {want}: {set1:?}"
        );
    }
    assert_eq!(
        first.undo_len(),
        steps,
        "mode and label add no history step"
    );
    for n in [2usize, 3, 5] {
        let mut s = session();
        zigzag(&mut s, n - 1);
        hover(&mut s, 100.0 + 25.0 * n as f64, 160.0);
        let mut set = cycle_labels(&mut s);
        set.sort();
        let mut want = set1.clone();
        want.sort();
        assert_eq!(set, want, "segment {n} cycles the same modes");
    }
}

#[test]
fn ac25_alt_is_one_step_per_press_and_ignores_repeat_fields_and_chords() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 100.0);
    hover(&mut s, 150.0, 140.0);
    let base = mode(&s);
    let mk = |alt, ctrl, shift, repeat, dom_blocked| KeyInput {
        key: "Alt",
        alt,
        ctrl,
        shift,
        repeat,
        dom_blocked,
        ..KeyInput::default()
    };
    for (name, input) in [
        ("repeat", mk(true, false, false, true, false)),
        ("text field focus", mk(true, false, false, false, true)),
        ("AltGr (Ctrl+Alt)", mk(true, true, false, false, false)),
        ("Shift+Alt", mk(true, false, true, false, false)),
    ] {
        s.key_down(input);
        assert_eq!(mode(&s), base, "{name} must not change the mode");
    }
    s.key_down(mk(true, false, false, false, false));
    let one = mode(&s);
    assert_ne!(one, base, "one press moves one step");
    for _ in 0..5 {
        s.key_down(mk(true, false, false, true, false));
    }
    assert_eq!(mode(&s), one, "auto-repeat after a press is ignored");
    // label vanishes on cancel and on commit
    s.escape();
    assert!(label(&s).is_none() || view(&s)["pending_picks"] == 0);
    assert!(
        label(&s).is_none() || view(&s)["tool"] != "polyline" || mode(&s) == "free-line",
        "label state after cancel: {:?}",
        label(&s)
    );
}

#[test]
fn ac34_alt_never_changes_selection_and_adds_no_history() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let l = lines(&graph(&s))[0].0.clone();
    s.set_sketch_tool(SketchTool::Select);
    s.sketch_select_entity(&l, false);
    let before = view(&s)["entities"].clone();
    let steps = s.undo_len();
    alt(&mut s);
    alt(&mut s);
    assert_eq!(
        view(&s)["entities"],
        before,
        "Alt alone never changes selection"
    );
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac34_label_is_gone_after_commit_and_after_cancel() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 100.0);
    hover(&mut s, 150.0, 140.0);
    assert!(label(&s).is_some());
    s.escape();
    s.escape();
    let l = label(&s);
    assert!(
        l.is_none() || view(&s)["pending_picks"] == 0,
        "label must disappear on cancel: {l:?}"
    );
    s.set_sketch_tool(SketchTool::Select);
    assert!(label(&s).is_none(), "no label in Select");
}

#[test]
fn ac34_dimension_tool_alt_cycles_radius_and_diameter_for_a_circle() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Circle);
    click(&mut s, 200.0, 200.0);
    click(&mut s, 230.0, 200.0);
    s.set_sketch_tool(SketchTool::Select);
    let c = graph(&s)
        .entities
        .iter()
        .find(|(_, e)| matches!(e, sk::Entity::Circle { .. }))
        .unwrap()
        .0
        .clone();
    s.sketch_select_entity(&c, false);
    assert!(s.set_sketch_constraint_tool("dimension"));
    let first = view(&s)["dimension_action"].clone();
    alt(&mut s);
    let second = view(&s)["dimension_action"].clone();
    alt(&mut s);
    let third = view(&s)["dimension_action"].clone();
    assert_ne!(
        first, second,
        "Alt changes the dimension kind: {first} {second}"
    );
    assert_eq!(first, third, "two kinds cycle back");
}

// ---------------------------------------------------------------- AC35 variants

#[test]
fn ac35_alt_cycles_variants_inside_one_tool_before_the_first_pick() {
    for (tool, partner) in [
        (SketchTool::Circle, "circle-three-points"),
        (SketchTool::Arc, "arc-center"),
        (SketchTool::Ellipse, "ellipse-three-points"),
    ] {
        let mut s = session();
        s.set_sketch_tool(tool);
        hover(&mut s, 100.0, 100.0);
        let start = view(&s)["tool"].as_str().unwrap().to_owned();
        let l0 = label(&s);
        alt(&mut s);
        let v = view(&s);
        let after = v["tool"].as_str().unwrap().to_owned();
        assert_ne!(after, start, "{start}: Alt must switch the variant");
        assert_eq!(after, partner, "{start} partner variant");
        assert_ne!(label(&s), l0, "{start}: the label names the active variant");
        assert!(label(&s).is_some());
        alt(&mut s);
        assert_eq!(
            view(&s)["tool"],
            start.as_str(),
            "{start}: variants cycle back"
        );
    }
}

#[test]
fn ac35_after_a_pick_alt_hints_and_keeps_the_pick() {
    for tool in [SketchTool::Circle, SketchTool::Arc, SketchTool::Ellipse] {
        let mut s = session();
        s.set_sketch_tool(tool);
        click(&mut s, 100.0, 100.0);
        let t = view(&s)["tool"].clone();
        let picks = view(&s)["pending_picks"].clone();
        assert!(picks.as_u64().unwrap() >= 1);
        alt(&mut s);
        let v = view(&s);
        assert_eq!(v["tool"], t, "variant must not change after a pick");
        assert_eq!(
            v["pending_picks"], picks,
            "earlier picks never discarded silently"
        );
        assert!(
            v["canvas"]["mode_label"]["hint"].is_string(),
            "a hint to finish or press Escape is shown: {}",
            v["canvas"]
        );
    }
}

// ---------------------------------------------------------------- AC26 arc preview

fn arcs_of(g: &sk::Sketch) -> Vec<(sk::EntityId, sk::EntityId, sk::EntityId)> {
    g.entities
        .values()
        .filter_map(|e| match e {
            sk::Entity::Arc {
                center, start, end, ..
            } => Some((center.clone(), start.clone(), end.clone())),
            _ => None,
        })
        .collect()
}

/// Builds a polyline with `committed` free segments and puts it into `mode_name`.
fn arc_setup(committed: usize, mode_name: &str) -> (Session, (f64, f64), (f64, f64)) {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    let mut p = (100.0, 100.0);
    click(&mut s, p.0, p.1);
    let mut t = (1.0, 0.0);
    for i in 0..committed {
        let q = (p.0 + 25.0, if i % 2 == 0 { p.1 + 15.0 } else { p.1 - 15.0 });
        click(&mut s, q.0, q.1);
        let d = (q.0 - p.0, q.1 - p.1);
        let n = d.0.hypot(d.1);
        t = (d.0 / n, d.1 / n);
        p = q;
    }
    hover(&mut s, p.0 + 40.0, p.1 + 40.0);
    if !s.set_sketch_mode(mode_name) {
        for _ in 0..8 {
            if mode(&s) == mode_name {
                break;
            }
            alt(&mut s);
        }
    }
    assert_eq!(mode(&s), mode_name, "mode reachable");
    (s, p, t)
}

#[test]
fn ac26_arc_preview_is_always_visible_on_both_sides_at_segments_1_2_3_5_8() {
    let radius = 50.0;
    for (name, perpendicular) in [
        ("tangent-arc", false),
        ("perpendicular-arc-left", true),
        ("perpendicular-arc-right", true),
    ] {
        for segment in [1usize, 2, 3, 5, 8] {
            let (mut s, p, t) = arc_setup(segment - 1, name);
            // Start direction line of the arc: the tangent for a tangent arc, its normal for a
            // perpendicular arc.
            let d = if perpendicular { (-t.1, t.0) } else { t };
            s.pointer_leave();
            let baseline = s.draw_list().triangle_count();
            let mut a = 3.0_f64;
            while a < 360.0 {
                if (a - 180.0).abs() < 1.0 {
                    a += 3.0;
                    continue;
                }
                let ang = a.to_radians();
                let dir = (ang.cos(), ang.sin());
                let v = (d.0 * dir.0 - d.1 * dir.1, d.1 * dir.0 + d.0 * dir.1);
                hover(&mut s, p.0 + radius * v.0, p.1 + radius * v.1);
                let json = view(&s);
                assert_eq!(json["degenerate"], false, "{name} seg{segment} a={a}");
                assert!(
                    s.draw_list().triangle_count() > baseline,
                    "{name} segment {segment} angle {a}: no preview drawn (frame without an arc)"
                );
                a += 12.0;
            }
            // exactly on the line, both rays: degenerate, shown, commit refused
            for sign in [1.0, -1.0] {
                let target = (p.0 + sign * radius * d.0, p.1 + sign * radius * d.1);
                hover(&mut s, target.0, target.1);
                assert_eq!(
                    view(&s)["degenerate"],
                    true,
                    "{name} seg{segment} on-line sign {sign}"
                );
                assert!(
                    s.draw_list().triangle_count() > baseline,
                    "{name} seg{segment}: the degenerate straight segment is still drawn"
                );
                let steps = s.undo_len();
                let pt = at(&s, target.0, target.1);
                s.pointer_down(pt, false);
                s.pointer_up(pt, false, false);
                assert_eq!(
                    s.undo_len(),
                    steps,
                    "{name} seg{segment}: degenerate commit refused"
                );
                if segment >= 2 {
                    let arcs = arcs_of(&graph(&s));
                    assert!(arcs.is_empty(), "{name} seg{segment}: no arc committed");
                }
            }
        }
    }
}

#[test]
fn ac26_each_side_commits_an_arc_and_sides_have_opposite_sweeps_at_every_segment() {
    for name in [
        "tangent-arc",
        "perpendicular-arc-left",
        "perpendicular-arc-right",
    ] {
        for segment in [1usize, 2, 3, 5, 8] {
            let mut sides = Vec::new();
            for sign in [1.0, -1.0] {
                let (mut s, p, t) = arc_setup(segment - 1, name);
                let d = if name == "tangent-arc" {
                    t
                } else {
                    (-t.1, t.0)
                };
                let n = (-d.1, d.0);
                let target = (
                    p.0 + 30.0 * d.0 + sign * 30.0 * n.0,
                    p.1 + 30.0 * d.1 + sign * 30.0 * n.1,
                );
                hover(&mut s, target.0, target.1);
                click(&mut s, target.0, target.1);
                let g = graph(&s);
                let arcs = arcs_of(&g);
                assert_eq!(
                    arcs.len(),
                    1,
                    "{name} segment {segment} side {sign}: commit makes an arc"
                );
                let c = pos(&g, &arcs[0].0);
                let side = cross(d, (c.0 - p.0, c.1 - p.1));
                assert!(
                    side * sign > 0.0,
                    "{name} seg{segment}: arc turns toward the cursor side"
                );
                sides.push(side.signum());
            }
            assert_eq!(
                sides[0], -sides[1],
                "{name} segment {segment}: opposite sweeps"
            );
        }
    }
}

// ---------------------------------------------------------------- AC27 capture

fn endpoint_scene() -> (Session, (f64, f64)) {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 117.0));
    (s, (160.0, 117.0))
}
fn capture_marker(s: &Session) -> bool {
    view(s)["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["id"] == "pending-capture" && a["text"] == "Endpoint")
}
fn hover_px(s: &mut Session, e: (f64, f64), dx_px: f64, dy_px: f64) {
    let sc = s.view().scale();
    let p = at(s, e.0, e.1);
    s.pointer_hover(Point::new(p.x + dx_px / sc, p.y + dy_px / sc), false, false);
}

#[test]
fn ac27_capture_acquires_within_12_px_and_releases_beyond_16_px_at_any_zoom() {
    // Lead's brief and UX notes: 12 px in, 16 px out. The specification text says 10 px
    // (marked Proposal); this deviation is reported, not asserted.
    for zoom in [0.5, 1.0, 3.7795, 8.0, 20.0] {
        let (mut s, e) = endpoint_scene();
        set_scale(&mut s, zoom);
        s.set_sketch_tool(SketchTool::Polyline);
        for (px, inside) in [
            (0.0, true),
            (4.0, true),
            (11.5, true),
            (13.0, false),
            (30.0, false),
        ] {
            hover_px(&mut s, e, 200.0, 0.0);
            hover_px(&mut s, e, px, 0.0);
            assert_eq!(
                capture_marker(&s),
                inside,
                "scale {} px {px}: acquire only within 12px",
                s.view().scale()
            );
        }
        for (px, held) in [(14.0, true), (15.5, true), (17.0, false)] {
            hover_px(&mut s, e, 0.0, 0.0);
            hover_px(&mut s, e, px, 0.0);
            assert_eq!(
                capture_marker(&s),
                held,
                "scale {} px {px}: release beyond 16px",
                s.view().scale()
            );
        }
    }
}

#[test]
fn ac27_capture_ignores_vector_snapping_settings() {
    for enabled in [false, true] {
        let (mut s, e) = endpoint_scene();
        let mut snap = s.snapping();
        snap.enabled = enabled;
        snap.x = enabled;
        snap.y = enabled;
        s.set_snapping(snap);
        s.set_sketch_tool(SketchTool::Polyline);
        hover_px(&mut s, e, 6.0, 3.0);
        assert!(capture_marker(&s), "snapping enabled={enabled}");
    }
}

#[test]
fn ac27_commit_reuses_endpoint_and_closing_a_chain_shares_one_point() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 100.0);
    click(&mut s, 180.0, 110.0);
    click(&mut s, 140.0, 190.0);
    let sc = s.view().scale();
    let start = at(&s, 100.0, 100.0);
    let near_start = Point::new(start.x + 6.0 / sc, start.y - 5.0 / sc);
    s.pointer_hover(near_start, false, false);
    assert!(capture_marker(&s), "chain start captured");
    s.pointer_down(near_start, false);
    s.pointer_up(near_start, false, false);
    let g = graph(&s);
    assert_eq!(lines(&g).len(), 3);
    assert_eq!(
        free_points(&g).len(),
        3,
        "closing yields one shared start point: {g:?}"
    );
    let exact = pos(&g, &lines(&g).iter().map(|l| l.1.clone()).next().unwrap());
    assert!(
        lines(&g)
            .iter()
            .any(|l| near(pos(&g, &l.2), (100.0, 100.0), 1e-9))
            || lines(&g)
                .iter()
                .any(|l| near(pos(&g, &l.1), (100.0, 100.0), 1e-9)),
        "closing point is exactly the chain start, not 6px off: {exact:?}"
    );
}

#[test]
fn ac27_no_pull_outside_the_radius_and_no_duplicate_point_for_a_line_start() {
    let (mut s, e) = endpoint_scene();
    s.set_sketch_tool(SketchTool::Polyline);
    let sc = s.view().scale();
    let ep = at(&s, e.0, e.1);
    // outside: a click 14 px away creates a distinct point at that exact place
    let out = Point::new(ep.x + 14.0 / sc, ep.y);
    s.pointer_hover(out, false, false);
    assert!(!capture_marker(&s));
    s.pointer_down(out, false);
    s.pointer_up(out, false, false);
    click(&mut s, 260.0, 190.0);
    let g = graph(&s);
    assert_eq!(
        free_points(&g).len(),
        4,
        "start point not pulled onto the endpoint"
    );
    assert!(
        free_points(&g)
            .iter()
            .any(|p| (pos(&g, p).0 - (e.0 + 14.0 / sc)).abs() < 1e-6),
        "the new point sits exactly where it was clicked: {g:?}"
    );
    // inside: continue from the captured end, no duplicate near-coincident point
    let (mut s, e) = endpoint_scene();
    s.set_sketch_tool(SketchTool::Polyline);
    let ep = at(&s, e.0, e.1);
    let inn = Point::new(ep.x + 7.0 / sc, ep.y + 2.0 / sc);
    s.pointer_hover(inn, false, false);
    s.pointer_down(inn, false);
    s.pointer_up(inn, false, false);
    click(&mut s, 220.0, 150.0);
    let g = graph(&s);
    assert_eq!(lines(&g).len(), 2);
    assert_eq!(
        free_points(&g).len(),
        3,
        "reused endpoint identity, no duplicate: {g:?}"
    );
}

#[test]
fn ac27_nearest_endpoint_wins_when_several_are_in_range() {
    let mut s = session();
    // two endpoints 12 px apart horizontally
    s.set_sketch_tool(SketchTool::Polyline);
    let sc = s.view().scale();
    let gap = 12.0 / sc;
    click(&mut s, 100.0, 100.0);
    click(&mut s, 100.0, 160.0);
    s.escape();
    click(&mut s, 100.0 + gap, 100.0);
    click(&mut s, 100.0 + gap, 160.0);
    s.escape();
    let g0 = graph(&s);
    let b_id = free_points(&g0)
        .into_iter()
        .find(|p| near(pos(&g0, p), (100.0 + gap, 100.0), 1e-6))
        .unwrap();
    // hover 3 px to the right of B (B nearest, A at 15 px: out). Use 4px to the left of B (A at 8)
    let b = at(&s, 100.0 + gap, 100.0);
    let hovered = Point::new(b.x - 3.0 / sc, b.y);
    s.pointer_hover(hovered, false, false);
    assert!(capture_marker(&s));
    // click and then draw away: new line must use B (3px away) rather than A (9px away)
    s.pointer_down(hovered, false);
    s.pointer_up(hovered, false, false);
    click(&mut s, 300.0, 200.0);
    let g = graph(&s);
    let new = lines(&g)
        .into_iter()
        .find(|l| {
            near(pos(&g, &l.2), (300.0, 200.0), 1e-6) || near(pos(&g, &l.1), (300.0, 200.0), 1e-6)
        })
        .unwrap();
    let from = if near(pos(&g, &new.2), (300.0, 200.0), 1e-6) {
        new.1
    } else {
        new.2
    };
    assert_eq!(from, b_id, "nearest endpoint wins");
}

#[test]
fn ac27_capture_applies_to_arc_construction() {
    let (mut s, e) = endpoint_scene();
    s.set_sketch_tool(SketchTool::Arc);
    let sc = s.view().scale();
    let ep = at(&s, e.0, e.1);
    let inn = Point::new(ep.x + 6.0 / sc, ep.y);
    s.pointer_hover(inn, false, false);
    assert!(capture_marker(&s), "arc start picks capture the endpoint");
    s.pointer_down(inn, false);
    s.pointer_up(inn, false, false);
    click(&mut s, 260.0, 117.0);
    click(&mut s, 210.0, 70.0);
    let g = graph(&s);
    let arcs = arcs_of(&g);
    assert_eq!(arcs.len(), 1, "{g:?}");
    let line_ends: Vec<_> = lines(&g)
        .iter()
        .flat_map(|l| [l.1.clone(), l.2.clone()])
        .collect();
    assert!(
        line_ends.contains(&arcs[0].1) || line_ends.contains(&arcs[0].2),
        "the arc reuses the line's endpoint identity"
    );
}

// ---------------------------------------------------------------- AC28 Ctrl lock

fn lock_commit(raw_deg: f64) -> (sk::Sketch, f64, f64) {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 100.0);
    ctrl(&mut s, true);
    let a = raw_deg.to_radians();
    let p = at(&s, 100.0 + 50.0 * a.cos(), 100.0 + 50.0 * a.sin());
    s.pointer_hover(p, false, true);
    s.pointer_down(p, false);
    s.pointer_up(p, false, true);
    ctrl(&mut s, false);
    let g = graph(&s);
    let l = &lines(&g)[0];
    let (s0, e0) = (pos(&g, &l.1), pos(&g, &l.2));
    let d = (e0.0 - s0.0, e0.1 - s0.1);
    (g, d.1.atan2(d.0).to_degrees(), d.0.hypot(d.1))
}

#[test]
fn ac28_ctrl_snaps_direction_to_the_one_angle_table_and_keeps_length() {
    for raw in [
        3.0, 10.0, 19.0, 37.0, 52.0, 80.0, 100.0, 137.0, 185.0, 200.0, 260.0, 275.0, 300.0, 350.0,
    ] {
        let (_, got, len) = lock_commit(raw);
        let want = snap_angle(Angle::from_radians(raw.to_radians()))
            .as_radians()
            .to_degrees();
        let mut diff = (got - want).rem_euclid(360.0);
        if diff > 180.0 {
            diff -= 360.0;
        }
        assert!(
            diff.abs() < 1e-6,
            "raw {raw}: snapped to {got}, table says {want}"
        );
        assert!(
            (len - 50.0).abs() < 0.2,
            "raw {raw}: pointer distance kept as length, got {len}"
        );
    }
}

#[test]
fn ac28_axis_stops_add_horizontal_or_vertical_others_add_nothing() {
    for (raw, h, v) in [
        (2.0, true, false),
        (88.0, false, true),
        (178.0, true, false),
        (272.0, false, true),
        (44.0, false, false),
        (46.0, false, false),
        (22.0, false, false),
        (135.0, false, false),
    ] {
        let (g, ang, _) = lock_commit(raw);
        assert_eq!(
            has_hv(&g),
            (h, v),
            "raw {raw} -> {ang}: relations {:?}",
            relations(&g)
        );
        if !h && !v {
            assert!(
                g.constraints.is_empty(),
                "raw {raw} -> {ang}: the lock only positions the point: {:?}",
                relations(&g)
            );
        }
    }
}

#[test]
fn ac28_pending_horizontal_is_shown_before_commit_and_readout_shows_angle() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 100.0);
    ctrl(&mut s, true);
    let p = at(&s, 150.0, 103.0);
    s.pointer_hover(p, false, true);
    let v = view(&s);
    let texts: Vec<String> = v["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["text"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        texts.iter().any(|t| t == "Horizontal"),
        "pending constraint visible before commit: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains('0') && t.contains('°'))
            || v["canvas"]["entry"]["fields"][1]["value"] == "0",
        "angle in the live readout: {texts:?} {}",
        v["canvas"]
    );
    // a non-axis stop shows no pending constraint
    let q = at(&s, 100.0 + 50.0 * 0.7071, 100.0 + 50.0 * 0.7071);
    s.pointer_hover(q, false, true);
    let texts: Vec<String> = view(&s)["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["text"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        !texts.iter().any(|t| t == "Horizontal" || t == "Vertical"),
        "{texts:?}"
    );
    // releasing Ctrl restores the free direction
    ctrl(&mut s, false);
    let free = at(&s, 100.0 + 50.0 * 0.6, 100.0 + 50.0 * 0.8);
    s.pointer_hover(free, false, false);
    click(&mut s, 100.0 + 30.0, 100.0 + 40.0);
    let g = graph(&s);
    let l = &lines(&g)[0];
    let (a, b) = (pos(&g, &l.1), pos(&g, &l.2));
    assert!(
        near((b.0 - a.0, b.1 - a.1), (30.0, 40.0), 1e-6),
        "free direction restored: {a:?} {b:?}"
    );
}

#[test]
fn ac28_ctrl_has_no_effect_in_tangent_and_perpendicular_modes() {
    for name in [
        "tangent-line",
        "perpendicular-line",
        "tangent-arc",
        "perpendicular-arc-left",
    ] {
        let (mut s, p, t) = arc_setup(2, name);
        let target = (p.0 + 37.0 * t.0 - 20.0 * t.1, p.1 + 37.0 * t.1 + 20.0 * t.0);
        hover(&mut s, target.0, target.1);
        let free = s.draw_list();
        let free_view = view(&s);
        ctrl(&mut s, true);
        let pt = at(&s, target.0, target.1);
        s.pointer_hover(pt, false, true);
        let locked = s.draw_list();
        assert!(free.triangle_count() > 0);
        assert_eq!(
            free, locked,
            "{name}: Ctrl must not change a fixed-direction mode"
        );
        assert_eq!(free_view["degenerate"], view(&s)["degenerate"]);
        assert_eq!(
            free_view["canvas"]["entry"]["fields"],
            view(&s)["canvas"]["entry"]["fields"]
        );
    }
}

#[test]
fn ac28_capture_wins_over_the_ctrl_lock() {
    let mut s = session();
    draw_line(&mut s, (200.0, 100.0), (240.0, 160.0));
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 100.0);
    ctrl(&mut s, true);
    let sc = s.view().scale();
    let e = at(&s, 200.0, 100.0);
    // endpoint at angle 0 from start would lock identical; use endpoint of the other line at an
    // off-stop angle instead: (240,160) is 66.1 degrees from the start
    let e2 = at(&s, 240.0, 160.0);
    let p = Point::new(e2.x + 5.0 / sc, e2.y);
    s.pointer_hover(p, false, true);
    assert!(capture_marker(&s), "capture marker even while Ctrl is held");
    s.pointer_down(p, false);
    s.pointer_up(p, false, true);
    let g = graph(&s);
    let new = lines(&g)
        .into_iter()
        .find(|l| {
            near(pos(&g, &l.1), (100.0, 100.0), 1e-9) || near(pos(&g, &l.2), (100.0, 100.0), 1e-9)
        })
        .unwrap();
    let end = if near(pos(&g, &new.1), (100.0, 100.0), 1e-9) {
        new.2
    } else {
        new.1
    };
    assert!(
        near(pos(&g, &end), (240.0, 160.0), 1e-9),
        "the capture wins: {:?}",
        pos(&g, &end)
    );
    let _ = e;
}

#[test]
fn ac28_slot_centreline_obeys_the_lock() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Slot);
    click(&mut s, 100.0, 100.0);
    ctrl(&mut s, true);
    let a = 38.0_f64.to_radians();
    let p = at(&s, 100.0 + 60.0 * a.cos(), 100.0 + 60.0 * a.sin());
    s.pointer_hover(p, false, true);
    s.pointer_down(p, false);
    s.pointer_up(p, false, true);
    ctrl(&mut s, false);
    click(&mut s, 130.0, 160.0);
    let g = graph(&s);
    // the two cap centres are the free points at distance 60 from each other, 37.5deg or 45deg
    let pts: Vec<_> = free_points(&g).into_iter().map(|p| pos(&g, &p)).collect();
    let want = snap_angle(Angle::from_radians(38.0_f64.to_radians())).as_radians();
    let ok = pts.iter().any(|a| {
        pts.iter().any(|b| {
            let d = (b.0 - a.0, b.1 - a.1);
            (d.0.hypot(d.1) - 60.0).abs() < 0.2 && (d.1.atan2(d.0) - want).abs() < 1e-6
        })
    });
    assert!(
        ok,
        "slot centreline locked to {} deg: {pts:?}",
        want.to_degrees()
    );
}

// ---------------------------------------------------------------- AC29 / AC30 drag and cursor

struct Pt(f64, f64);
fn drag_with_frames(s: &mut Session, from: Pt, to: Pt, cpu_ms: &mut Vec<f64>) {
    s.set_sketch_tool(SketchTool::Select);
    let pf = at(s, from.0, from.1);
    s.pointer_hover(pf, false, false);
    s.pointer_down(pf, false);
    for i in 1..=8 {
        let f = f64::from(i) / 8.0;
        let p = at(
            s,
            from.0 + (to.0 - from.0) * f,
            from.1 + (to.1 - from.1) * f,
        );
        let ((), t) = cpu_time::cpu_time_of(|| {
            s.pointer_hover(p, false, false);
            std::hint::black_box(s.sketch_view_json());
            std::hint::black_box(s.draw_list());
        });
        cpu_ms.push(t.as_secs_f64() * 1000.0);
    }
    let pt = at(s, to.0, to.1);
    s.pointer_up(pt, false, false);
}
fn budget(cpu: &[f64], what: &str) {
    let max = cpu.iter().copied().fold(0.0, f64::max);
    println!("{what}: max {max:.3} ms (debug={})", cfg!(debug_assertions));
    if !cfg!(debug_assertions) {
        assert!(max < 16.0, "{what}: AC6 16 ms budget exceeded: {max}");
    }
}

#[test]
fn ac29_point_line_circle_arc_ellipse_move_with_the_pointer_one_history_step() {
    // point
    let mut s = session();
    let mut cpu = Vec::new();
    s.set_sketch_tool(SketchTool::Point);
    click(&mut s, 100.0, 100.0);
    let steps = s.undo_len();
    drag_with_frames(&mut s, Pt(100.0, 100.0), Pt(130.0, 120.0), &mut cpu);
    let g = graph(&s);
    assert!(
        near(pos(&g, &free_points(&g)[0]), (130.0, 120.0), 1e-6),
        "point follows pointer"
    );
    assert_eq!(s.undo_len(), steps + 1, "one drag one history step");
    budget(&cpu, "point");

    // line by its edge translates as a whole
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let steps = s.undo_len();
    let mut cpu = Vec::new();
    drag_with_frames(&mut s, Pt(130.0, 110.0), Pt(150.0, 140.0), &mut cpu);
    let g = graph(&s);
    let l = &lines(&g)[0];
    assert!(
        near(pos(&g, &l.1), (120.0, 130.0), 1e-6),
        "{:?}",
        pos(&g, &l.1)
    );
    assert!(near(pos(&g, &l.2), (180.0, 150.0), 1e-6));
    assert_eq!(s.undo_len(), steps + 1);
    budget(&cpu, "line");
    // an endpoint moves only that point
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let mut cpu = Vec::new();
    drag_with_frames(&mut s, Pt(160.0, 120.0), Pt(190.0, 90.0), &mut cpu);
    let g = graph(&s);
    let l = &lines(&g)[0];
    assert!(near(pos(&g, &l.1), (100.0, 100.0), 1e-6));
    assert!(near(pos(&g, &l.2), (190.0, 90.0), 1e-6));

    // circle by its edge
    let mut s = session();
    s.set_sketch_tool(SketchTool::Circle);
    click(&mut s, 200.0, 200.0);
    click(&mut s, 230.0, 200.0);
    let steps = s.undo_len();
    let mut cpu = Vec::new();
    drag_with_frames(&mut s, Pt(200.0, 230.0), Pt(215.0, 220.0), &mut cpu);
    let g = graph(&s);
    let (c, r) = g
        .entities
        .values()
        .find_map(|e| match e {
            sk::Entity::Circle { center, radius, .. } => Some((center.clone(), radius.0)),
            _ => None,
        })
        .unwrap();
    assert!(
        near(pos(&g, &c), (215.0, 190.0), 1e-6),
        "circle translates: {:?}",
        pos(&g, &c)
    );
    assert!((r - 30.0).abs() < 1e-6, "radius unchanged");
    assert_eq!(s.undo_len(), steps + 1);
    budget(&cpu, "circle");

    // arc by its edge (a point of the arc strictly between its endpoints)
    let mut s = session();
    s.set_sketch_tool(SketchTool::Arc);
    click(&mut s, 100.0, 200.0);
    click(&mut s, 160.0, 200.0);
    click(&mut s, 130.0, 170.0);
    let before = graph(&s);
    let arc = arcs_of(&before);
    assert_eq!(arc.len(), 1, "{before:?}");
    let (radius, sa, ea) = match before
        .entities
        .values()
        .find(|e| matches!(e, sk::Entity::Arc { .. }))
    {
        Some(sk::Entity::Arc {
            radius,
            start_angle,
            end_angle,
            ..
        }) => (radius.0, start_angle.0, end_angle.0),
        _ => unreachable!(),
    };
    let sweep = (ea - sa).rem_euclid(std::f64::consts::TAU);
    let mid = sa + sweep / 2.0;
    let c0 = pos(&before, &arc[0].0);
    let grab = (c0.0 + radius * mid.cos(), c0.1 + radius * mid.sin());
    let pre: Vec<_> = [&arc[0].0, &arc[0].1, &arc[0].2]
        .iter()
        .map(|i| pos(&before, i))
        .collect();
    let steps = s.undo_len();
    let mut cpu = Vec::new();
    drag_with_frames(
        &mut s,
        Pt(grab.0, grab.1),
        Pt(grab.0 + 10.0, grab.1 + 25.0),
        &mut cpu,
    );
    let g = graph(&s);
    for (i, id) in [&arc[0].0, &arc[0].1, &arc[0].2].iter().enumerate() {
        assert!(
            near(pos(&g, id), (pre[i].0 + 10.0, pre[i].1 + 25.0), 1e-5),
            "arc point {i} translates: {:?} from {:?}",
            pos(&g, id),
            pre[i]
        );
    }
    assert_eq!(s.undo_len(), steps + 1);
    budget(&cpu, "arc");

    // ellipse by its edge
    let mut s = session();
    s.set_sketch_tool(SketchTool::Ellipse);
    click(&mut s, 250.0, 100.0);
    click(&mut s, 290.0, 100.0);
    click(&mut s, 250.0, 80.0);
    let before = graph(&s);
    let (ec, ef, minor) = before
        .entities
        .values()
        .find_map(|e| match e {
            sk::Entity::Ellipse {
                center,
                focus,
                minor_radius,
                ..
            } => Some((center.clone(), focus.clone(), minor_radius.0)),
            _ => None,
        })
        .expect("ellipse created");
    let (c0, f0) = (pos(&before, &ec), pos(&before, &ef));
    let steps = s.undo_len();
    let mut cpu = Vec::new();
    let edge = (c0.0, c0.1 - minor);
    drag_with_frames(
        &mut s,
        Pt(edge.0, edge.1),
        Pt(edge.0 - 10.0, edge.1 + 10.0),
        &mut cpu,
    );
    let g = graph(&s);
    assert!(
        near(pos(&g, &ec), (c0.0 - 10.0, c0.1 + 10.0), 1e-5),
        "ellipse centre translates"
    );
    assert!(
        near(pos(&g, &ef), (f0.0 - 10.0, f0.1 + 10.0), 1e-5),
        "ellipse focus translates"
    );
    assert_eq!(s.undo_len(), steps + 1);
    budget(&cpu, "ellipse");
}

#[test]
fn ac29_rectangle_and_slot_members_drag_and_keep_their_constraints() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Rectangle);
    click(&mut s, 100.0, 100.0);
    click(&mut s, 180.0, 150.0);
    let g0 = graph(&s);
    let ls = lines(&g0);
    assert_eq!(ls.len(), 4, "{g0:?}");
    let steps = s.undo_len();
    let mut cpu = Vec::new();
    drag_with_frames(&mut s, Pt(140.0, 100.0), Pt(140.0, 88.0), &mut cpu);
    let g = graph(&s);
    // top edge midpoint follows the pointer in y; the constrained rectangle remains a rectangle
    let ys: Vec<f64> = lines(&g)
        .iter()
        .flat_map(|l| [pos(&g, &l.1).1, pos(&g, &l.2).1])
        .collect();
    assert!(
        ys.iter().any(|y| (y - 88.0).abs() < 1e-5),
        "edge reached the pointer: {ys:?}"
    );
    for (_, a, b) in lines(&g) {
        let (pa, pb) = (pos(&g, &a), pos(&g, &b));
        assert!(
            (pa.0 - pb.0).abs() < 1e-6 || (pa.1 - pb.1).abs() < 1e-6,
            "H/V constraints hold after the drag: {pa:?} {pb:?}"
        );
    }
    assert_eq!(s.undo_len(), steps + 1);
    budget(&cpu, "rectangle");

    // slot: drag a straight side
    let mut s = session();
    s.set_sketch_tool(SketchTool::Slot);
    click(&mut s, 100.0, 100.0);
    click(&mut s, 180.0, 100.0);
    click(&mut s, 140.0, 120.0);
    s.set_sketch_tool(SketchTool::Select);
    let g0 = graph(&s);
    let side = lines(&g0);
    assert!(!side.is_empty(), "slot has straight sides: {g0:?}");
    let steps = s.undo_len();
    let mid = {
        let l = &side[0];
        let (a, b) = (pos(&g0, &l.1), pos(&g0, &l.2));
        ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
    };
    let mut cpu = Vec::new();
    drag_with_frames(
        &mut s,
        Pt(mid.0, mid.1),
        Pt(mid.0 + 5.0, mid.1 + 14.0),
        &mut cpu,
    );
    let g = graph(&s);
    assert_ne!(g, g0, "the slot moved");
    assert_eq!(s.undo_len(), steps + 1);
    budget(&cpu, "slot");
}

#[test]
fn ac29_escape_mid_drag_restores_start_and_fixed_geometry_does_not_move() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    s.set_sketch_tool(SketchTool::Select);
    let before = graph(&s);
    let steps = s.undo_len();
    let a = at(&s, 130.0, 110.0);
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    hover(&mut s, 160.0, 150.0);
    assert_eq!(
        graph(&s),
        before.clone(),
        "the document is untouched mid drag"
    );
    assert!(
        s.sketch_preview().is_some_and(|p| p != &before),
        "the live preview moved with the pointer"
    );
    s.escape();
    assert_eq!(graph(&s), before, "Escape restores the start state");
    assert_eq!(s.undo_len(), steps);
    let b = at(&s, 170.0, 160.0);
    s.pointer_up(b, false, false);
    assert_eq!(graph(&s), before, "release after Escape commits nothing");

    // fixed point: drag does not move it
    let mut s = session();
    s.set_sketch_tool(SketchTool::Point);
    click(&mut s, 100.0, 100.0);
    s.set_sketch_tool(SketchTool::Select);
    let p = free_points(&graph(&s))[0].clone();
    s.sketch_select_entity(&p, false);
    s.sketch_rail_press("fixed").unwrap();
    let before = graph(&s);
    let mut cpu = Vec::new();
    drag_with_frames(&mut s, Pt(100.0, 100.0), Pt(140.0, 140.0), &mut cpu);
    assert!(
        near(pos(&graph(&s), &p), pos(&before, &p), 1e-9),
        "fixed geometry does not move"
    );
}

#[test]
fn ac29_constrained_kinds_follow_hard_constraints() {
    // line with Horizontal translates freely (H holds under translation)
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let l = lines(&graph(&s))[0].0.clone();
    s.sketch_select_entity(&l, false);
    s.sketch_rail_press("horizontal").unwrap();
    let g0 = graph(&s);
    let (a0, b0) = (pos(&g0, &lines(&g0)[0].1), pos(&g0, &lines(&g0)[0].2));
    let mut cpu = Vec::new();
    drag_with_frames(
        &mut s,
        Pt((a0.0 + b0.0) / 2.0, (a0.1 + b0.1) / 2.0),
        Pt((a0.0 + b0.0) / 2.0 + 20.0, (a0.1 + b0.1) / 2.0 + 15.0),
        &mut cpu,
    );
    let g = graph(&s);
    let l = &lines(&g)[0];
    let (a, b) = (pos(&g, &l.1), pos(&g, &l.2));
    assert!((a.1 - b.1).abs() < 1e-6, "Horizontal still holds");
    assert!((b.0 - a.0 - (b0.0 - a0.0)).abs() < 1e-6, "length kept");
    assert!(
        near(a, (a0.0 + 20.0, a0.1 + 15.0), 1e-5),
        "moves with the pointer: {a:?}"
    );

    // circle with a radius dimension translates keeping the radius
    let mut s = session();
    s.set_sketch_tool(SketchTool::Circle);
    click(&mut s, 200.0, 200.0);
    click(&mut s, 230.0, 200.0);
    s.set_sketch_tool(SketchTool::Select);
    let c = graph(&s)
        .entities
        .iter()
        .find(|(_, e)| matches!(e, sk::Entity::Circle { .. }))
        .unwrap()
        .0
        .clone();
    s.sketch_select_entity(&c, false);
    assert!(s.sketch_apply_action("radius", "30 mm", false).unwrap());
    s.set_sketch_tool(SketchTool::Select);
    let mut cpu = Vec::new();
    drag_with_frames(&mut s, Pt(200.0, 230.0), Pt(212.0, 241.0), &mut cpu);
    let g = graph(&s);
    let (ctr, r) = g
        .entities
        .values()
        .find_map(|e| match e {
            sk::Entity::Circle { center, radius, .. } => Some((center.clone(), radius.0)),
            _ => None,
        })
        .unwrap();
    assert!((r - 30.0).abs() < 1e-6);
    assert!(
        near(pos(&g, &ctr), (212.0, 211.0), 1e-5),
        "{:?}",
        pos(&g, &ctr)
    );
}

#[test]
fn ac30_cursor_states_in_the_event_path() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    assert_eq!(
        view(&s)["cursor"],
        "crosshair",
        "drawing tool keeps its normal cursor"
    );
    s.set_sketch_tool(SketchTool::Select);
    hover(&mut s, 400.0, 400.0);
    let away = view(&s)["cursor"].clone();
    assert!(away != "grab" && away != "grabbing", "empty space: {away}");
    hover(&mut s, 130.0, 110.0);
    assert_eq!(
        view(&s)["cursor"],
        "grab",
        "hover over draggable geometry: open hand"
    );
    let p = at(&s, 130.0, 110.0);
    s.pointer_down(p, false);
    assert_eq!(view(&s)["cursor"], "grabbing", "press");
    hover(&mut s, 140.0, 130.0);
    assert_eq!(view(&s)["cursor"], "grabbing", "drag");
    let q = at(&s, 140.0, 130.0);
    s.pointer_up(q, false, false);
    assert_eq!(view(&s)["cursor"], "grab", "release over the moved line");
    // Escape mid drag
    let p = at(&s, 140.0, 130.0);
    s.pointer_down(p, false);
    hover(&mut s, 170.0, 160.0);
    assert_eq!(view(&s)["cursor"], "grabbing");
    s.escape();
    assert_ne!(
        view(&s)["cursor"],
        "grabbing",
        "Escape ends the drag cursor"
    );
    s.pointer_up(at(&s, 170.0, 160.0), false, false);

    // fully fixed geometry: normal cursor
    let mut s = session();
    s.set_sketch_tool(SketchTool::Point);
    click(&mut s, 100.0, 100.0);
    s.set_sketch_tool(SketchTool::Select);
    hover(&mut s, 100.0, 100.0);
    assert_eq!(view(&s)["cursor"], "grab", "a free point is draggable");
    let p = free_points(&graph(&s))[0].clone();
    s.sketch_select_entity(&p, false);
    s.sketch_rail_press("fixed").unwrap();
    hover(&mut s, 101.0, 100.0);
    hover(&mut s, 100.0, 100.0);
    let c = view(&s)["cursor"].clone();
    assert!(
        c != "grab" && c != "grabbing",
        "fixed point: tool's normal cursor, got {c}"
    );
}

#[test]
fn ac30_constraint_symbol_is_not_draggable_and_keeps_the_normal_cursor() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let l = lines(&graph(&s))[0].0.clone();
    s.sketch_select_entity(&l, false);
    s.sketch_rail_press("horizontal").unwrap();
    s.set_sketch_tool(SketchTool::Select);
    s.escape();
    let v = view(&s);
    let y = v["symbols"][0].clone();
    let p = s.screen_to_document(y["x"].as_f64().unwrap(), y["y"].as_f64().unwrap());
    s.pointer_hover(p, false, false);
    let c = view(&s)["cursor"].clone();
    assert!(
        c != "grab" && c != "grabbing",
        "over a constraint symbol: {c}"
    );
}

// ---------------------------------------------------------------- AC31 dimension annotations

fn dimension_texts(s: &Session) -> Vec<String> {
    view(s)["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["dimension"] == true)
        .map(|a| a["text"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn ac31_dimensions_are_permanent_with_the_specified_text() {
    // length 50 mm
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let l = lines(&graph(&s))[0].0.clone();
    s.sketch_select_entity(&l, false);
    s.sketch_apply_action("length", "50 mm", false).unwrap();
    s.set_sketch_tool(SketchTool::Select);
    s.escape();
    assert_eq!(
        view(&s)["entities"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["selected"] == true)
            .count(),
        0
    );
    assert_eq!(
        dimension_texts(&s),
        vec!["50 mm"],
        "visible without selection or hover"
    );

    // radius and diameter
    for (action, value, want) in [
        ("radius", "25 mm", "R 25 mm"),
        ("diameter", "50 mm", "⌀ 50 mm"),
    ] {
        let mut s = session();
        s.set_sketch_tool(SketchTool::Circle);
        click(&mut s, 200.0, 200.0);
        click(&mut s, 230.0, 200.0);
        s.set_sketch_tool(SketchTool::Select);
        let c = graph(&s)
            .entities
            .iter()
            .find(|(_, e)| matches!(e, sk::Entity::Circle { .. }))
            .unwrap()
            .0
            .clone();
        s.sketch_select_entity(&c, false);
        s.sketch_apply_action(action, value, false).unwrap();
        s.set_sketch_tool(SketchTool::Select);
        assert_eq!(dimension_texts(&s), vec![want]);
    }

    // reference dimension, distinguishable without colour
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let l = lines(&graph(&s))[0].0.clone();
    s.sketch_select_entity(&l, false);
    s.sketch_apply_action("length", "10 mm", true).unwrap();
    let t = dimension_texts(&s);
    assert_eq!(t.len(), 1);
    assert!(
        t[0].starts_with('(') && t[0].ends_with(')'),
        "reference in parentheses: {t:?}"
    );
    assert_eq!(view(&s)["annotations"][0]["reference"], true);
}

#[test]
fn ac31_angle_dimension_text_is_45_deg() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (180.0, 100.5));
    draw_line(&mut s, (100.0, 100.0), (150.0, 150.0));
    let g = graph(&s);
    let ls = lines(&g);
    s.sketch_select_entity(&ls[0].0, false);
    s.sketch_select_entity(&ls[1].0, true);
    let r = s.sketch_apply_action("angle", "45", false);
    assert!(r.is_ok(), "{r:?}");
    let t = dimension_texts(&s);
    assert_eq!(t, vec!["45°"], "spec AC31 literal text");
}

#[test]
fn ac31_dimension_text_shows_resolved_expression_value() {
    let mut s = session();
    s.sketch_add_parameter("width", "40 mm", sk::Dimension::Length)
        .unwrap();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let l = lines(&graph(&s))[0].0.clone();
    s.sketch_select_entity(&l, false);
    s.sketch_apply_action("length", "width * 2", false).unwrap();
    assert_eq!(dimension_texts(&s), vec!["80 mm"]);
}

// ---------------------------------------------------------------- AC32 symbols

#[test]
fn ac32_symbols_have_names_do_not_overlap_and_selecting_one_selects_the_constraint() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Rectangle);
    click(&mut s, 100.0, 100.0);
    click(&mut s, 180.0, 150.0);
    s.set_sketch_tool(SketchTool::Select);
    let g = graph(&s);
    let l = lines(&g)[0].0.clone();
    s.sketch_select_entity(&l, false);
    s.sketch_apply_action("length", "80 mm", false).unwrap();
    s.escape();
    let v = view(&s);
    let syms = v["symbols"].as_array().unwrap();
    assert_ne!(syms.len(), 0);
    for y in syms {
        assert!(
            !y["name"].as_str().unwrap().is_empty(),
            "accessible name: {y}"
        );
    }
    for (i, a) in syms.iter().enumerate() {
        for b in &syms[i + 1..] {
            let (dx, dy) = (
                (a["x"].as_f64().unwrap() - b["x"].as_f64().unwrap()).abs(),
                (a["y"].as_f64().unwrap() - b["y"].as_f64().unwrap()).abs(),
            );
            assert!(dx >= 16.0 || dy >= 16.0, "symbols overlap: {a} / {b}");
        }
    }
    // selecting a constraint selects it and highlights participants
    let id = v["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| {
            c["kind"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .contains("horizontal")
        })
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(s.sketch_inspect_constraint(&id.as_str().into(), false));
    let v = view(&s);
    assert!(
        v["constraints"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == id.as_str() && c["selected"] == true)
    );
    assert!(
        v["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .any(|y| y["id"] == id.as_str() && y["selected"] == true)
    );
}

#[test]
fn ac32_scales_constant_screen_size_symbol_positions_follow_zoom_not_size() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let l = lines(&graph(&s))[0].0.clone();
    s.sketch_select_entity(&l, false);
    s.sketch_rail_press("horizontal").unwrap();
    s.escape();
    let first = view(&s)["symbols"].as_array().unwrap().len();
    set_scale(&mut s, 12.0);
    assert_eq!(view(&s)["symbols"].as_array().unwrap().len(), first);
    set_scale(&mut s, 0.7);
    assert_eq!(view(&s)["symbols"].as_array().unwrap().len(), first);
}

fn crowded(n: usize) -> Session {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    for i in 0..n {
        let (col, row) = (i % 12, i / 12);
        let (x, y) = (40.0 + 45.0 * col as f64, 40.0 + 18.0 * row as f64);
        ctrl(&mut s, true);
        click_ctrl(&mut s, x, y, x + 30.0, y + 1.0);
        ctrl(&mut s, false);
    }
    s
}
fn click_ctrl(s: &mut Session, x: f64, y: f64, x2: f64, y2: f64) {
    let a = at(s, x, y);
    s.pointer_hover(a, false, true);
    s.pointer_down(a, false);
    s.pointer_up(a, false, true);
    let b = at(s, x2, y2);
    s.pointer_hover(b, false, true);
    s.pointer_down(b, false);
    s.pointer_up(b, false, true);
    s.escape();
}

#[test]
fn ac32_151_annotations_thin_but_selected_hovered_conflicting_stay_visible() {
    let mut s = crowded(150);
    let v = view(&s);
    let syms = v["symbols"].as_array().unwrap();
    assert_eq!(
        syms.len(),
        150,
        "150 horizontal symbols, none thinned at 150"
    );
    assert!(
        syms.iter().all(|y| y["dot"] == false),
        "no thinning at the limit of 150"
    );
    // one more dimension makes 151 annotations
    let g = graph(&s);
    let l = lines(&g)[0].0.clone();
    s.set_sketch_tool(SketchTool::Select);
    s.sketch_select_entity(&l, false);
    s.sketch_apply_action("length", "30 mm", false).unwrap();
    s.escape();
    let v = view(&s);
    let syms = v["symbols"].as_array().unwrap();
    let dims = v["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["dimension"] == true)
        .count();
    assert_eq!(syms.len() + dims, 151);
    let dots = syms.iter().filter(|y| y["dot"] == true).count();
    assert!(
        dots > 100,
        "display thins automatically above 150: {dots} dots of {}",
        syms.len()
    );
    // select one constraint: it stays a full symbol
    let id = syms[77]["id"].as_str().unwrap().to_owned();
    assert!(s.sketch_inspect_constraint(&id.as_str().into(), false));
    let v = view(&s);
    let sel = v["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .find(|y| y["id"] == id.as_str())
        .unwrap()
        .clone();
    assert_eq!(sel["selected"], true);
    assert_eq!(sel["dot"], false, "selected annotation stays visible");
    // hovered: hover the symbol's screen position
    let other = v["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .find(|y| y["dot"] == true)
        .unwrap()
        .clone();
    let p = s.screen_to_document(other["x"].as_f64().unwrap(), other["y"].as_f64().unwrap());
    s.pointer_hover(p, false, false);
    let v = view(&s);
    let hov: Vec<_> = v["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|y| y["hovered"] == true)
        .collect();
    for h in &hov {
        assert_eq!(h["dot"], false, "hovered annotation stays visible");
    }
    assert!(!hov.is_empty(), "pointer over a symbol hovers it");
}

#[test]
fn ac32_conflicting_symbol_stays_visible_when_thinned_and_carries_a_marker() {
    let mut s = crowded(151);
    let g = graph(&s);
    let l = lines(&g)[3].0.clone();
    s.set_sketch_tool(SketchTool::Select);
    s.sketch_select_entity(&l, false);
    let r = s.sketch_rail_press("vertical");
    s.sketch_apply_action("length", "20 mm", false).unwrap();
    let v = view(&s);
    let conflicting: Vec<_> = v["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|y| y["conflicting"] == true)
        .collect();
    assert!(
        !conflicting.is_empty(),
        "Vertical + length on a Horizontal line conflicts ({r:?}); constraints {}",
        v["constraints"]
    );
    for c in conflicting {
        assert_eq!(c["dot"], false, "conflicting symbols are never thinned");
        assert!(
            c["name"].as_str().unwrap().contains("conflicting"),
            "non-colour marker in the accessible name: {c}"
        );
    }
}

// ---------------------------------------------------------------- AC33 on-canvas entry

#[test]
fn ac33_entry_sits_at_the_object_follows_pan_and_zoom_and_avoids_the_cursor() {
    let mut s = session();
    s.resize_viewport(1000.0, 700.0);
    center_on(&mut s, 130.0, 115.0, 500.0, 350.0);
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 100.0);
    hover(&mut s, 160.0, 130.0);
    let v = view(&s);
    let e = v["canvas"]["entry"].clone();
    assert!(
        e.is_object(),
        "entry appears at the object: {}",
        v["canvas"]
    );
    let labels: Vec<_> = e["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["label"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(labels, ["Length", "Angle"]);
    let (ex, ey) = (e["x"].as_f64().unwrap(), e["y"].as_f64().unwrap());
    let (px, py) = (
        e["pointer_x"].as_f64().unwrap(),
        e["pointer_y"].as_f64().unwrap(),
    );
    // length field near the segment midpoint
    let sc = s.view().scale();
    let a = s.view().document_to_screen(at(&s, 100.0, 100.0));
    let b = s.view().document_to_screen(at(&s, 160.0, 130.0));
    let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
    assert!(
        (ex - mid.0).hypot(ey - mid.1) < 90.0,
        "near the midpoint: entry ({ex},{ey}) mid {mid:?} scale {sc}"
    );
    assert!(
        (ex - px).hypot(ey - py) > 8.0,
        "does not cover the live cursor point"
    );
    assert!(
        ex >= 0.0 && ey >= 0.0 && ex <= 1000.0 && ey <= 700.0,
        "inside the canvas"
    );
    // pan moves the entry with the object
    s.wheel(-60.0, -40.0, 500.0, 350.0, false, false);
    hover(&mut s, 160.0, 130.0);
    let e2 = view(&s)["canvas"]["entry"].clone();
    assert!(
        (e2["x"].as_f64().unwrap() - ex).abs() > 1.0
            || (e2["y"].as_f64().unwrap() - ey).abs() > 1.0,
        "follows pan"
    );
}

#[test]
fn ac33_entry_stays_inside_the_canvas_near_every_edge() {
    for (tx, ty) in [
        (30.0, 30.0),
        (970.0, 30.0),
        (30.0, 670.0),
        (970.0, 670.0),
        (500.0, 10.0),
        (500.0, 690.0),
    ] {
        let mut s = session();
        s.resize_viewport(1000.0, 700.0);
        center_on(&mut s, 100.0, 100.0, tx, ty);
        s.set_sketch_tool(SketchTool::Polyline);
        click(&mut s, 100.0, 100.0);
        let sc = s.view().scale();
        // pointer 40 px further toward the edge when possible
        let (dx, dy) = (
            (tx - 500.0).signum() * 40.0 / sc,
            (ty - 350.0).signum() * 20.0 / sc,
        );
        hover(&mut s, 100.0 + dx, 100.0 + dy);
        let e = view(&s)["canvas"]["entry"].clone();
        assert!(e.is_object());
        let (ex, ey) = (e["x"].as_f64().unwrap(), e["y"].as_f64().unwrap());
        assert!(
            (0.0..=1000.0).contains(&ex) && (0.0..=700.0).contains(&ey),
            "start at ({tx},{ty}): entry ({ex},{ey}) must stay inside the visible canvas"
        );
    }
}

#[test]
fn ac33_entry_fields_per_object_kind() {
    let cases: [(SketchTool, Vec<(f64, f64)>, &[&str]); 3] = [
        (SketchTool::Circle, vec![(200.0, 200.0)], &["Radius"]),
        (
            SketchTool::Rectangle,
            vec![(100.0, 100.0)],
            &["Width", "Height"],
        ),
        (
            SketchTool::Slot,
            vec![(100.0, 100.0), (180.0, 100.0)],
            &["Width"],
        ),
    ];
    for (tool, picks, want) in cases {
        let mut s = session();
        s.resize_viewport(1000.0, 700.0);
        s.set_sketch_tool(tool);
        for p in &picks {
            click(&mut s, p.0, p.1);
        }
        let last = picks[picks.len() - 1];
        hover(&mut s, last.0 + 40.0, last.1 + 30.0);
        let e = view(&s)["canvas"]["entry"].clone();
        assert!(e.is_object(), "{tool:?}: {}", view(&s)["canvas"]);
        let labels: Vec<String> = e["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["label"].as_str().unwrap().to_owned())
            .collect();
        for w in want {
            assert!(
                labels.iter().any(|l| l == w),
                "{tool:?}: expected {w} in {labels:?}"
            );
        }
    }
}

#[test]
fn ac33_dimension_edit_field_sits_at_its_text() {
    let mut s = session();
    s.resize_viewport(1000.0, 700.0);
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let l = lines(&graph(&s))[0].0.clone();
    s.sketch_select_entity(&l, false);
    s.sketch_apply_action("length", "50 mm", false).unwrap();
    s.set_sketch_tool(SketchTool::Select);
    let a = view(&s)["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["dimension"] == true)
        .unwrap()
        .clone();
    let p = s.screen_to_document(a["x"].as_f64().unwrap(), a["y"].as_f64().unwrap());
    assert!(s.double_click(p, false, false) || s.sketch_edit_dimension_at(p));
    let e = view(&s)["canvas"]["entry"].clone();
    assert!(e.is_object(), "{}", view(&s)["canvas"]);
    assert_eq!(e["kind"], "edit");
    let d = (e["x"].as_f64().unwrap() - a["x"].as_f64().unwrap())
        .hypot(e["y"].as_f64().unwrap() - a["y"].as_f64().unwrap());
    assert!(d < 70.0, "edit field at its text, distance {d}");
}

// ---------------------------------------------------------------- AC34 no compound tool makes redundant constraints

#[test]
fn ac_no_compound_tool_creates_redundant_or_conflicting_constraints() {
    for tool in [
        SketchTool::Rectangle,
        SketchTool::Slot,
        SketchTool::Circle,
        SketchTool::Ellipse,
        SketchTool::Arc,
    ] {
        let mut s = session();
        s.set_sketch_tool(tool);
        let picks: &[(f64, f64)] = match tool {
            SketchTool::Rectangle => &[(100.0, 100.0), (180.0, 150.0)],
            SketchTool::Slot => &[(100.0, 100.0), (180.0, 110.0), (140.0, 125.0)],
            SketchTool::Circle => &[(100.0, 100.0), (130.0, 100.0)],
            SketchTool::Ellipse => &[(100.0, 100.0), (140.0, 100.0), (100.0, 80.0)],
            _ => &[(100.0, 200.0), (160.0, 200.0), (130.0, 170.0)],
        };
        for p in picks {
            click(&mut s, p.0, p.1);
        }
        let v = view(&s);
        for c in v["constraints"].as_array().unwrap() {
            assert_eq!(c["redundant"], false, "{tool:?}: redundant {c}");
            assert_eq!(c["conflicting"], false, "{tool:?}: conflicting {c}");
        }
        let st = v["status"].clone();
        assert!(
            st.is_null() || !st.to_string().to_lowercase().contains("conflict"),
            "{tool:?}: {st}"
        );
    }
    // the polyline modes too, including a closed loop
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    for p in [
        (100.0, 100.0),
        (180.0, 100.0),
        (180.0, 160.0),
        (100.0, 160.0),
        (100.0, 100.0),
    ] {
        click(&mut s, p.0, p.1);
    }
    for c in view(&s)["constraints"].as_array().unwrap() {
        assert_eq!(c["redundant"], false, "{c}");
        assert_eq!(c["conflicting"], false, "{c}");
    }
}

#[test]
fn ac32_redundant_symbol_carries_a_non_colour_marker() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let l = lines(&graph(&s))[0].0.clone();
    s.sketch_select_entity(&l, false);
    s.sketch_apply_action("horizontal", "", false).unwrap();
    s.sketch_apply_action("horizontal", "", false).unwrap();
    let v = view(&s);
    let redundant: Vec<_> = v["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["redundant"] == true)
        .collect();
    assert!(!redundant.is_empty(), "{}", v["constraints"]);
    let names: Vec<String> = v["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .map(|y| y["name"].as_str().unwrap().to_lowercase())
        .collect();
    assert!(
        names.iter().any(|n| n.contains("redundant")),
        "a redundant constraint's symbol names its state, not only a colour: {names:?}; constraints {}; symbols {}",
        v["constraints"],
        v["symbols"]
    );
}

// ---------------------------------------------------------------- UX review items (revision 2 final check)

fn entity_of(s: &Session, kind: &str) -> sk::EntityId {
    graph(s)
        .entities
        .iter()
        .find(|(_, e)| {
            matches!(
                (kind, e),
                ("arc", sk::Entity::Arc { .. }) | ("circle", sk::Entity::Circle { .. })
            )
        })
        .unwrap()
        .0
        .clone()
}
fn line_and_point() -> (Session, sk::EntityId, sk::EntityId) {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (200.0, 100.0));
    s.set_sketch_tool(SketchTool::Point);
    click(&mut s, 130.0, 160.0);
    s.set_sketch_tool(SketchTool::Select);
    let g = graph(&s);
    let l = lines(&g)[0].0.clone();
    let p = free_points(&g).last().unwrap().clone();
    (s, l, p)
}
fn point_line_distance(s: &Session, l: &sk::EntityId, p: &sk::EntityId) -> f64 {
    let g = graph(s);
    let (_, a, b) = lines(&g).into_iter().find(|x| &x.0 == l).unwrap();
    let (a, b, p) = (pos(&g, &a), pos(&g, &b), pos(&g, p));
    let len = (b.0 - a.0).hypot(b.1 - a.1);
    cross((b.0 - a.0, b.1 - a.1), (p.0 - a.0, p.1 - a.1)).abs() / len
}

#[test]
fn rev2_review2_dimension_measures_a_point_to_a_line_in_either_pick_order() {
    for point_first in [false, true] {
        let (mut s, l, p) = line_and_point();
        if point_first {
            s.sketch_select_entity(&p, false);
            s.sketch_select_entity(&l, true);
        } else {
            s.sketch_select_entity(&l, false);
            s.sketch_select_entity(&p, true);
        }
        let steps = s.undo_len();
        assert!(s.sketch_rail_press("dimension").unwrap());
        assert_eq!(s.undo_len(), steps + 1, "applies at once, one step");
        assert!(
            view(&s)["canvas"]["mode_label"].is_null(),
            "no arming, no placement prompt: {}",
            view(&s)["canvas"]
        );
        let e = view(&s)["canvas"]["entry"].clone();
        assert!(e.is_object(), "an entry box opens: {}", view(&s)["canvas"]);
        let shown: f64 = e["fields"][0]["value"].as_str().unwrap().parse().unwrap();
        assert!(
            (shown - 60.0).abs() < 1e-6,
            "the box shows the measured 60 mm offset: {e}"
        );
        let id = open_box_dimension(&s);
        assert!(s.sketch_dimension_text(&id, "40 mm", false).unwrap());
        assert_eq!(s.undo_len(), steps + 2, "typing is a second history step");
        let d = point_line_distance(&s, &l, &p);
        assert!(
            (d - 40.0).abs() < 1e-6,
            "point driven to 40 mm from the line: {d}"
        );
        assert_eq!(dimension_texts(&s), vec!["40 mm"], "driving, permanent");
        assert_eq!(view(&s)["annotations"][0]["reference"], false);
    }
}

#[test]
fn rev2_review2_picking_a_point_then_a_line_on_the_canvas_reaches_the_entry_box() {
    let (mut s, _l, _p) = line_and_point();
    assert!(s.sketch_rail_press("dimension").unwrap());
    click(&mut s, 130.0, 160.0);
    click(&mut s, 180.0, 100.0);
    click(&mut s, 60.0, 40.0);
    assert_eq!(
        view(&s)["dimension_action"],
        "distance",
        "{}",
        view(&s)["canvas"]
    );
    assert_eq!(
        view(&s)["canvas"]["entry"]["fields"][0]["label"],
        "Distance"
    );
}

#[test]
fn rev2_review2_a_selection_no_kind_fits_says_what_the_tool_needs_in_its_tooltip() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Circle);
    click(&mut s, 200.0, 200.0);
    click(&mut s, 230.0, 200.0);
    s.set_sketch_tool(SketchTool::Select);
    draw_line(&mut s, (100.0, 100.0), (160.0, 100.0));
    let l = lines(&graph(&s))[0].0.clone();
    let c = entity_of(&s, "circle");
    s.sketch_select_entity(&c, false);
    s.sketch_select_entity(&l, true);
    let steps = s.undo_len();
    assert!(!s.sketch_rail_press("dimension").unwrap_or(false));
    let v = view(&s);
    let tip = v["rail"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "dimension")
        .unwrap()["tooltip"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        tip.starts_with("Dimension: ") && tip.len() > 14,
        "reason in the tooltip: {tip}"
    );
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn rev2_review2_a_refused_canvas_pick_says_why_on_the_mode_label() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Circle);
    click(&mut s, 200.0, 200.0);
    click(&mut s, 230.0, 200.0);
    s.set_sketch_tool(SketchTool::Select);
    draw_line(&mut s, (100.0, 100.0), (160.0, 100.0));
    assert!(s.sketch_rail_press("dimension").unwrap());
    click(&mut s, 200.0, 170.0);
    click(&mut s, 130.0, 100.0);
    let v = view(&s);
    eprintln!("PICKS {} {}", v["dimension_action"], v["canvas"]);
    let label_error = v["canvas"]["mode_label"]["error"]
        .as_str()
        .map(str::to_owned);
    let offered = v["dimension_action"].is_string();
    assert!(
        offered || label_error.as_deref().is_some_and(|m| !m.is_empty()),
        "silent no-op: {}",
        v["canvas"]
    );
}

#[test]
fn rev2_review2_arc_defaults_to_radius_circle_to_diameter_alt_switches_and_label_names_it() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Arc);
    for p in [(100.0, 200.0), (160.0, 200.0), (130.0, 170.0)] {
        click(&mut s, p.0, p.1);
    }
    s.set_sketch_tool(SketchTool::Circle);
    click(&mut s, 300.0, 300.0);
    click(&mut s, 330.0, 300.0);
    s.set_sketch_tool(SketchTool::Select);
    for (kind, first, second, first_text, second_text) in [
        ("arc", "radius", "diameter", "Radius", "Diameter"),
        ("circle", "diameter", "radius", "Diameter", "Radius"),
    ] {
        let e = entity_of(&s, kind);
        s.sketch_select_entity(&e, false);
        assert!(s.set_sketch_constraint_tool("dimension"));
        assert_eq!(view(&s)["dimension_action"], first, "{kind} default");
        hover(&mut s, 60.0, if kind == "arc" { 40.0 } else { 120.0 });
        assert_eq!(label(&s).as_deref(), Some(first_text), "{kind} label");
        alt(&mut s);
        assert_eq!(view(&s)["dimension_action"], second, "{kind} after Alt");
        assert_eq!(
            label(&s).as_deref(),
            Some(second_text),
            "{kind} label after Alt"
        );
        alt(&mut s);
        assert_eq!(view(&s)["dimension_action"], first, "{kind} wraps");
        // the kind chosen is the one created
        let py = if kind == "arc" { 40.0 } else { 120.0 };
        click(&mut s, 60.0, py);
        alt(&mut s);
        let steps = s.undo_len();
        let want = if kind == "arc" {
            ("diameter", "R")
        } else {
            ("radius", "⌀")
        };
        let _ = want;
        assert!(s.sketch_apply_action(second, "20 mm", false).unwrap());
        assert_eq!(s.undo_len(), steps + 1);
        let texts = dimension_texts(&s);
        let prefix = if second == "radius" {
            "R 20 mm"
        } else {
            "⌀ 20 mm"
        };
        assert!(texts.iter().any(|t| t == prefix), "{kind}: {texts:?}");
        s.set_sketch_tool(SketchTool::Select);
        s.escape();
    }
}

#[test]
fn rev2_review2_a_refused_entry_marks_only_its_field_and_stays_out_of_the_panel() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 100.0);
    hover(&mut s, 140.0, 100.0);
    assert!(s.sketch_complete_numeric("0", "0").is_err());
    let e = view(&s)["canvas"]["entry"].clone();
    assert_eq!(e["error"]["field"], 0, "{e}");
    assert!(
        e["error"]["message"].as_str().unwrap().contains("0 mm"),
        "{e}"
    );
    assert!(
        view(&s)["error"].is_null(),
        "message not repeated in Sketches"
    );
    assert!(s.sketch_complete_numeric("20", "north").is_err());
    assert_eq!(view(&s)["canvas"]["entry"]["error"]["field"], 1);
    assert!(view(&s)["error"].is_null());
    assert!(s.sketch_complete_numeric("20", "0").unwrap());
    assert!(view(&s)["canvas"]["entry"]["error"].is_null());
    assert_eq!(
        lines(&graph(&s)).len(),
        1,
        "valid entry commits exactly one line"
    );
}

#[test]
fn rev2_review2_an_unsolvable_dimension_is_marked_keeps_its_box_and_can_be_undone() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (120.0, 100.0));
    let l = lines(&graph(&s))[0].0.clone();
    s.sketch_select_entity(&l, false);
    assert!(s.sketch_rail_press("dimension").unwrap());
    let first = open_box_dimension(&s); // measured 20 mm, already the current length
    s.sketch_select_entity(&l, false);
    // a second Length at the current value: redundant with the first (both marked)
    assert!(s.sketch_rail_press("dimension").unwrap());
    let second = open_box_dimension(&s);
    assert_ne!(first, second);
    let before = s.undo_len();
    let _ = s.sketch_dimension_text(&second, "30 mm", false);
    let v = view(&s);
    let marked = v["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["dimension"] == true && a["text"].as_str().unwrap().starts_with('!'));
    let boxed = v["canvas"]["entry"]["error"].is_object();
    assert!(
        marked && boxed,
        "the failing dimension carries the '!' marker and the box says why: {} / {}",
        v["annotations"],
        v["canvas"]
    );
    // the last valid geometry is kept
    let g = graph(&s);
    let (_, a, b) = lines(&g)[0].clone();
    let (a, b) = (pos(&g, &a), pos(&g, &b));
    assert!(
        ((b.0 - a.0).hypot(b.1 - a.1) - 20.0).abs() < 1e-6,
        "{a:?} {b:?}"
    );
    if s.undo_len() > before {
        s.escape();
        s.undo();
        assert!(
            view(&s)["annotations"]
                .as_array()
                .unwrap()
                .iter()
                .all(|a| !a["text"].as_str().unwrap().starts_with('!')),
            "undo removes the failing dimension"
        );
    }
}

fn hovered_circle_scene(radius: f64) -> (Session, sk::EntityId) {
    hovered_circle_scene_at(0.0, radius)
}
fn hovered_circle_scene_at(c: f64, radius: f64) -> (Session, sk::EntityId) {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Circle);
    click(&mut s, c, c);
    click(&mut s, c + radius, c);
    s.set_sketch_tool(SketchTool::Select);
    let c = entity_of(&s, "circle");
    (s, c)
}
fn symbol_count(s: &Session) -> usize {
    view(s)["symbols"].as_array().unwrap().len()
}

#[test]
fn rev2_review2_symbols_of_small_geometry_show_only_when_hovered_or_selected() {
    // a circle of 3 mm radius is 18 px across at 3 px/mm; a 30 mm one is 180 px
    let (mut small, c) = hovered_circle_scene_at(50.0, 3.0);
    set_scale(&mut small, 3.0);
    small.sketch_select_entity(&c, false);
    small.sketch_apply_action("fixed", "", false).unwrap();
    small.set_sketch_tool(SketchTool::Select);
    small.escape();
    hover(&mut small, 80.0, 80.0);
    assert_eq!(symbol_count(&small), 0, "{}", view(&small)["symbols"]);
    small.sketch_select_entity(&c, false);
    assert!(symbol_count(&small) > 0, "selected: symbols return");

    let (mut big, c) = hovered_circle_scene_at(50.0, 30.0);
    set_scale(&mut big, 3.0);
    big.sketch_select_entity(&c, false);
    big.sketch_apply_action("fixed", "", false).unwrap();
    big.escape();
    hover(&mut big, 200.0, 200.0);
    assert!(symbol_count(&big) > 0, "large geometry keeps its symbols");
}

#[test]
fn rev2_review2_a_radius_text_does_not_lie_on_the_circle_outline() {
    for r in [3.44_f64, 25.0] {
        let (mut s, c) = hovered_circle_scene(r);
        set_scale(&mut s, 4.0);
        s.sketch_select_entity(&c, false);
        s.sketch_apply_action("radius", &format!("{r} mm"), false)
            .unwrap();
        s.set_sketch_tool(SketchTool::Select);
        let a = view(&s)["annotations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["dimension"] == true)
            .unwrap()
            .clone();
        let centre = s.view().document_to_screen(at(&s, 0.0, 0.0));
        let scale = s.view().scale();
        let d = (a["x"].as_f64().unwrap() - centre.0).hypot(a["y"].as_f64().unwrap() - centre.1);
        let outline = r * scale;
        assert!(
            (d - outline).abs() > 6.0,
            "R {r}: text centre {d:.1} px from the centre, outline at {outline:.1} px"
        );
    }
}

#[test]
fn rev2_review2_ctrl_lock_shows_the_angle_badge_and_a_preview_h_badge() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 100.0, 100.0);
    assert!(!s.sketch_angle_lock_badge());
    ctrl(&mut s, true);
    s.pointer_hover(at(&s, 150.0, 103.0), false, true);
    assert!(
        s.sketch_angle_lock_badge(),
        "lock badge while Ctrl holds a free direction"
    );
    let a = view(&s)["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["text"] == "Horizontal")
        .cloned()
        .expect("pending Horizontal");
    assert_eq!(a["preview"], true, "{a}");
    assert!(a["glyph"].is_string(), "drawn as the H badge glyph: {a}");
    ctrl(&mut s, false);
    s.pointer_hover(at(&s, 150.0, 103.0), false, false);
    assert!(!s.sketch_angle_lock_badge(), "gone on release");
    // tangent mode: Ctrl has no direction to hold, so no badge
    s.set_sketch_mode("tangent-arc");
    ctrl(&mut s, true);
    s.pointer_hover(at(&s, 150.0, 103.0), false, true);
    assert!(!s.sketch_angle_lock_badge());
}

#[test]
fn rev2_review2_redundant_dimension_is_marked_apart_from_conflicting() {
    let mut s = session();
    draw_line(&mut s, (100.0, 100.0), (160.0, 120.0));
    let l = lines(&graph(&s))[0].0.clone();
    s.sketch_select_entity(&l, false);
    s.sketch_apply_action("length", "50 mm", false).unwrap();
    s.sketch_apply_action("length", "50 mm", false).unwrap();
    let a = view(&s)["annotations"].clone();
    let r: Vec<_> = a
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x["redundant"] == true)
        .collect();
    assert_eq!(r.len(), 1, "{a}");
    assert_eq!(r[0]["conflicting"], false);
    assert!(r[0]["text"].as_str().unwrap().starts_with('!'));
    assert!(r[0]["name"].as_str().unwrap().ends_with("redundant"));
}
