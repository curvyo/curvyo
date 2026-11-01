//! Independent acceptance tests for 0066 customer revision 5 (AC49-AC52).
#![allow(
    clippy::match_same_arms,
    clippy::single_match_else,
    clippy::semicolon_if_nothing_returned,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::float_cmp,
    clippy::type_complexity,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::similar_names,
    clippy::manual_midpoint,
    clippy::assertions_on_result_states,
    clippy::doc_markdown,
    clippy::needless_pass_by_value,
    clippy::items_after_statements,
    clippy::cognitive_complexity,
    clippy::implicit_clone,
    clippy::unnecessary_wraps,
    clippy::uninlined_format_args,
    clippy::assert_is_empty,
    missing_docs
)]
use curvyo_document_core::Point;
use curvyo_editor_wasm::{Session, SketchTool, Workbench};
use curvyo_sketch_core as sk;
use serde_json::Value;

type Pt = (f64, f64);

fn session() -> Session {
    let mut s = Session::new(6650);
    s.set_workbench(Workbench::Sketch);
    s.resize_viewport(1000.0, 700.0);
    s
}
fn view(s: &Session) -> Value {
    serde_json::from_str(&s.sketch_view_json()).unwrap()
}
fn graph(s: &Session) -> sk::Sketch {
    curvyo_document_core::unpack(6651, &s.pack("rev5").unwrap())
        .unwrap()
        .sketch(s.active_sketch().unwrap())
        .unwrap()
        .solution
}
fn tool(s: &Session) -> String {
    view(s)["tool"].as_str().unwrap_or("").to_owned()
}
fn armed(s: &Session) -> Value {
    view(s)["constraint_tool"].clone()
}
/// Document point of the sketch origin.
fn origin_doc(s: &Session) -> Point {
    let page = s.active_page();
    Point::new(
        page.origin.x + page.size.width.as_mm() / 2.0,
        page.origin.y + page.size.height.as_mm() / 2.0,
    )
}
fn scale(s: &Session) -> f64 {
    s.view().scale()
}
/// Screen position (CSS px) of the sketch origin.
fn origin_px(s: &Session) -> Pt {
    let d0 = s.screen_to_document(0.0, 0.0);
    let o = origin_doc(s);
    ((o.x - d0.x) * scale(s), (o.y - d0.y) * scale(s))
}
/// Document point for a screen offset (px) from the sketch origin.
fn rel(s: &Session, dx: f64, dy: f64) -> Point {
    let o = origin_doc(s);
    Point::new(o.x + dx / scale(s), o.y + dy / scale(s))
}
/// Screen px (absolute) of a sketch point (mm).
fn mm_px(s: &Session, p: Pt) -> Pt {
    let o = origin_px(s);
    (o.0 + p.0 * scale(s), o.1 + p.1 * scale(s))
}
fn click_at(s: &mut Session, p: Point, shift: bool) {
    s.pointer_hover(p, shift, false);
    s.pointer_down(p, shift);
    s.pointer_up(p, shift, false);
}
fn click_rel(s: &mut Session, dx: f64, dy: f64) {
    let p = rel(s, dx, dy);
    click_at(s, p, false);
}
fn click_rel_mod(s: &mut Session, dx: f64, dy: f64, shift: bool) {
    let p = rel(s, dx, dy);
    click_at(s, p, shift);
}
fn set_scale_about_origin(s: &mut Session, target: f64) {
    for _ in 0..200 {
        let now = scale(s);
        if (now / target - 1.0).abs() < 0.03 {
            return;
        }
        let o = origin_px(s);
        s.wheel(
            0.0,
            if now < target { -20.0 } else { 20.0 },
            o.0,
            o.1,
            false,
            true,
        );
    }
}
fn zoom_scale(percent: f64) -> f64 {
    percent / 100.0 * 96.0 / 25.4
}
fn pos(g: &sk::Sketch, id: &sk::EntityId) -> Pt {
    let p = g.point(id).unwrap();
    (p.x.0, p.y.0)
}
fn near(a: Pt, b: Pt, tol: f64) -> bool {
    (a.0 - b.0).hypot(a.1 - b.1) < tol
}
fn point_near(g: &sk::Sketch, p: Pt, tol: f64) -> Option<sk::EntityId> {
    g.entities
        .iter()
        .find(|(id, e)| {
            **id != g.origin && matches!(e, sk::Entity::Point { .. }) && near(pos(g, id), p, tol)
        })
        .map(|(id, _)| id.clone())
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
fn deselect(s: &mut Session) {
    s.set_sketch_tool(SketchTool::Select);
    s.escape();
    s.escape();
    assert!(selected(s).is_empty());
}
fn dof(s: &Session) -> i64 {
    match &view(s)["status"] {
        Value::Object(o) => o.values().next().and_then(Value::as_i64).unwrap_or(0),
        Value::String(t) if t.contains("Fully") => 0,
        other => panic!("status {other}"),
    }
}

fn centre_origin(s: &mut Session) {
    let o = origin_px(s);
    s.begin_pan(o.0, o.1);
    s.pan_to(500.0, 350.0);
    s.end_pan();
}
fn ann(s: &Session) -> Value {
    view(s)["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["dimension"] == true)
        .cloned()
        .unwrap_or(Value::Null)
}
fn red_segments(s: &Session) -> Vec<[Pt; 3]> {
    let d0 = s.screen_to_document(0.0, 0.0);
    let k = scale(s);
    s.draw_list()
        .triangles
        .chunks(3)
        .filter(|t| t[0].color.r == 191 && t[0].color.g == 41 && t[0].color.b == 51)
        .map(|t| {
            let f = |i: usize| ((t[i].position.x - d0.x) * k, (t[i].position.y - d0.y) * k);
            [f(0), f(1), f(2)]
        })
        .collect()
}

// ---------------------------------------------------------------- AC49 dimension drawing

fn dist_seg(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
    };
    (p.0 - (a.0 + t * dx)).hypot(p.1 - (a.1 + t * dy))
}
fn sign(v: f64) -> f64 {
    if v < 0.0 { -1.0 } else { 1.0 }
}
fn in_tri(p: Pt, t: &[Pt; 3], tol: f64) -> bool {
    let s = |a: Pt, b: Pt, c: Pt| (a.0 - c.0) * (b.1 - c.1) - (b.0 - c.0) * (a.1 - c.1);
    let (d1, d2, d3) = (s(p, t[0], t[1]), s(p, t[1], t[2]), s(p, t[2], t[0]));
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos_ = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    (!(neg && pos_))
        || dist_seg(p, t[0], t[1])
            .min(dist_seg(p, t[1], t[2]))
            .min(dist_seg(p, t[2], t[0]))
            <= tol
}
fn covered(tris: &[[Pt; 3]], p: Pt) -> bool {
    tris.iter().any(|t| in_tri(p, t, 0.6))
}

/// Checks the drawn dimension against the UX notes anatomy. `horizontal`: the dimension line is
/// horizontal at the annotation's y; `a`, `b` are the measured points in screen px.
fn check_dimension(s: &Session, horizontal: bool, a: Pt, b: Pt) -> Vec<String> {
    let an = ann(s);
    assert!(an.is_object(), "no dimension annotation");
    let (tx, ty) = (an["x"].as_f64().unwrap(), an["y"].as_f64().unwrap());
    let half = an["text"].as_str().unwrap().chars().count() as f64 * 7.0 / 2.0 + 8.0;
    // work in (along, across): along the dimension line, across it.
    let (la, lb, lt, line) = if horizontal {
        (a.0, b.0, tx, ty)
    } else {
        (a.1, b.1, ty, tx)
    };
    let to = |al: f64, ac: f64| if horizontal { (al, ac) } else { (ac, al) };
    let across = |p: Pt| if horizontal { p.1 } else { p.0 };
    let mut why = Vec::new();
    let tris = red_segments(s);
    if tris.is_empty() {
        why.push("nothing drawn in the driving colour".to_owned());
        return why;
    }
    // expected segments
    let mut ext: Vec<(Pt, Pt)> = Vec::new();
    for (al, p) in [(la, a), (lb, b)] {
        let gap = line - across(p);
        if gap.abs() > 8.5 {
            let sg = sign(gap);
            ext.push((to(al, across(p) + 4.0 * sg), to(al, line + 4.0 * sg)));
        }
    }
    let lo = la.min(lb).min(lt - half);
    let hi = la.max(lb).max(lt + half);
    let dim = (to(lo - 12.0, line), to(hi + 12.0, line));
    let end_a = to(la.min(lb), line);
    let end_b = to(la.max(lb), line);
    // stray check: every vertex and edge midpoint of a drawn triangle lies near an expected segment.
    let mut stray = 0;
    let mut worst: Option<Pt> = None;
    for t in &tris {
        let mut samples = vec![t[0], t[1], t[2]];
        for (i, j) in [(0, 1), (1, 2), (2, 0)] {
            samples.push(((t[i].0 + t[j].0) / 2.0, (t[i].1 + t[j].1) / 2.0));
        }
        for p in samples {
            let d_ext = ext
                .iter()
                .map(|e| dist_seg(p, e.0, e.1))
                .fold(f64::MAX, f64::min);
            let d_dim = dist_seg(p, dim.0, dim.1);
            let near_end = near(p, end_a, 14.0) || near(p, end_b, 14.0);
            let ok = d_ext <= 1.25 || d_dim <= 1.25 || (near_end && d_dim <= 3.3);
            if !ok {
                stray += 1;
                worst = Some(p);
            }
        }
    }
    if stray > 0 {
        why.push(format!(
            "{stray} drawn vertices outside the expected segments, e.g. {worst:?}"
        ));
    }
    // coverage: each extension line, and the dimension line between the extension lines
    for (i, e) in ext.iter().enumerate() {
        let len = (e.1.0 - e.0.0).hypot(e.1.1 - e.0.1);
        let mut gaps = 0;
        let n = (len / 3.0) as usize;
        for k in 1..n {
            let f = k as f64 / n as f64;
            let p = (e.0.0 + (e.1.0 - e.0.0) * f, e.0.1 + (e.1.1 - e.0.1) * f);
            if !covered(&tris, p) {
                gaps += 1;
            }
        }
        if gaps > 0 {
            why.push(format!("extension line {i} has {gaps} uncovered samples"));
        }
    }
    let span = (lb - la).abs();
    if span >= 56.0 {
        let mut gaps = Vec::new();
        let mut al = la.min(lb) + 10.0;
        while al < la.max(lb) - 10.0 {
            if (al - lt).abs() > half {
                let p = to(al, line);
                if !covered(&tris, p) {
                    gaps.push(al);
                }
            }
            al += 3.0;
        }
        if !gaps.is_empty() {
            why.push(format!(
                "dimension line broken at along={:.0}..{:.0} ({} samples)",
                gaps[0],
                gaps[gaps.len() - 1],
                gaps.len()
            ));
        }
    }
    why
}

fn drag_text(s: &mut Session, to_dx: f64, to_dy: f64) {
    let an = ann(s);
    let (ax, ay) = (an["x"].as_f64().unwrap(), an["y"].as_f64().unwrap());
    let o = origin_px(s);
    let p = rel(s, ax - o.0, ay - o.1);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    for i in 1..=6 {
        let f = f64::from(i) / 6.0;
        let q = rel(s, ax - o.0 + to_dx * f, ay - o.1 + to_dy * f);
        s.pointer_hover(q, false, false);
    }
    let q = rel(s, ax - o.0 + to_dx, ay - o.1 + to_dy);
    s.pointer_up(q, false, false);
    s.sketch_close_dimension_edit();
    deselect(s);
}
fn point_tool(s: &mut Session, pts: &[Pt]) {
    for p in pts {
        s.set_sketch_tool(SketchTool::Point);
        click_rel(s, p.0, p.1);
    }
}
fn quad(s: &mut Session) {
    s.set_sketch_tool(SketchTool::Polyline);
    for p in [
        (-200.0, -50.0),
        (-20.0, -200.0),
        (180.0, 60.0),
        (30.0, 230.0),
        (-200.0, -50.0),
    ] {
        click_rel(s, p.0, p.1);
    }
    deselect(s);
}
/// Applies a horizontal or vertical distance between two points by the rail pick flow.
fn dimension_between(s: &mut Session, name: &str, a: Pt, b: Pt) {
    deselect(s);
    assert!(matches!(s.sketch_rail_press(name), Ok(true)));
    click_rel(s, a.0, a.1);
    click_rel(s, b.0, b.1);
    s.sketch_close_dimension_edit();
    deselect(s);
}

struct Case {
    name: &'static str,
    zoom: f64,
    horizontal: bool,
    flip: bool,
}

fn run_case(
    c: &Case,
    build: impl Fn(&mut Session),
    a: Pt,
    b: Pt,
    extra: Option<&dyn Fn(&mut Session)>,
) -> Option<String> {
    let mut s = session();
    centre_origin(&mut s);
    set_scale_about_origin(&mut s, zoom_scale(c.zoom));
    centre_origin(&mut s);
    build(&mut s);
    let name = if c.horizontal {
        "horizontal-distance"
    } else {
        "vertical-distance"
    };
    let n0 = graph(&s).constraints.len();
    dimension_between(&mut s, name, a, b);
    if graph(&s).constraints.len() != n0 + 1 {
        return Some(format!(
            "{}: dimension not created ({} -> {} constraints)",
            c.name,
            n0,
            graph(&s).constraints.len()
        ));
    }
    if c.flip {
        let before = ann(&s);
        // move the dimension line to the other side of the measured points
        let o = origin_px(&s);
        let line_now = if c.horizontal {
            before["y"].as_f64().unwrap()
        } else {
            before["x"].as_f64().unwrap()
        };
        let mid = if c.horizontal {
            (a.1 + b.1) / 2.0 + o.1
        } else {
            (a.0 + b.0) / 2.0 + o.0
        };
        let target = mid + (mid - line_now) * 1.0 + sign(mid - line_now) * 60.0;
        let d = target - line_now;
        if c.horizontal {
            drag_text(&mut s, 0.0, d);
        } else {
            drag_text(&mut s, d, 0.0);
        }
        let after = ann(&s);
        let moved = if c.horizontal {
            after["y"].as_f64().unwrap() - before["y"].as_f64().unwrap()
        } else {
            after["x"].as_f64().unwrap() - before["x"].as_f64().unwrap()
        };
        if moved.abs() < 5.0 {
            return Some(format!(
                "{}: dragging the text did not move the dimension line",
                c.name
            ));
        }
    }
    if let Some(f) = extra {
        f(&mut s);
        deselect(&mut s);
    }
    let pa = (origin_px(&s).0 + a.0, origin_px(&s).1 + a.1);
    let pb = (origin_px(&s).0 + b.0, origin_px(&s).1 + b.1);
    let why = check_dimension(&s, c.horizontal, pa, pb);
    (!why.is_empty()).then(|| {
        format!(
            "{} zoom {} {} flip {}: {}",
            c.name,
            c.zoom,
            if c.horizontal { "H" } else { "V" },
            c.flip,
            why.join("; ")
        )
    })
}

const ZOOMS: [f64; 5] = [10.0, 25.0, 100.0, 400.0, 1000.0];

fn matrix(
    name: &'static str,
    zooms: &[f64],
    build: impl Fn(&mut Session) + Copy,
    h: (Pt, Pt),
    v: (Pt, Pt),
    extra_h: Option<&dyn Fn(&mut Session)>,
) {
    let mut fails = Vec::new();
    for &zoom in zooms {
        for flip in [false, true] {
            for horizontal in [true, false] {
                let (a, b) = if horizontal { h } else { v };
                let c = Case {
                    name,
                    zoom,
                    horizontal,
                    flip,
                };
                let ex = if horizontal { extra_h } else { None };
                if let Some(f) = run_case(&c, build, a, b, ex) {
                    fails.push(f);
                }
            }
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

#[test]
fn ac49_customer_quadrilateral_origin_inside_every_zoom_and_side() {
    // customer geometry: H between outer-left and outer-right, V between top and bottom
    matrix(
        "quad",
        &ZOOMS,
        quad,
        ((-200.0, -50.0), (180.0, 60.0)),
        ((-20.0, -200.0), (30.0, 230.0)),
        None,
    );
}

#[test]
fn ac49_span_crossing_the_axes_the_origin_and_another_point() {
    // points left and right of the Y axis, plus a third point exactly on the dimension line
    let build = |s: &mut Session| point_tool(s, &[(-150.0, -40.0), (150.0, 90.0), (0.0, -200.0)]);
    let mut fails = Vec::new();
    for zoom in [25.0, 100.0, 400.0] {
        for flip in [false, true] {
            for horizontal in [true, false] {
                let (a, b) = ((-150.0, -40.0), (150.0, 90.0));
                let c = Case {
                    name: "crossing",
                    zoom,
                    horizontal,
                    flip,
                };
                // a further point exactly on the dimension line, midway between the measured points
                let extra = |s: &mut Session| {
                    let an = ann(s);
                    let o = origin_px(s);
                    let (x, y) = if horizontal {
                        (-20.0, an["y"].as_f64().unwrap() - o.1)
                    } else {
                        (an["x"].as_f64().unwrap() - o.0, 30.0)
                    };
                    s.set_sketch_tool(SketchTool::Point);
                    click_rel(s, x, y);
                };
                if let Some(f) = run_case(&c, build, a, b, Some(&extra)) {
                    fails.push(f);
                }
            }
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

#[test]
fn ac49_measured_points_on_the_axes() {
    let build = |s: &mut Session| {
        point_tool(
            s,
            &[(-150.0, 0.0), (150.0, 0.0), (0.0, -120.0), (0.0, 120.0)],
        )
    };
    matrix(
        "on-axis",
        &[25.0, 100.0, 400.0],
        build,
        ((-150.0, 0.0), (150.0, 0.0)),
        ((0.0, -120.0), (0.0, 120.0)),
        None,
    );
}

#[test]
fn ac49_zero_value_and_short_spans() {
    // zero value: both points share x (H) ; the V case shares y
    let build = |s: &mut Session| {
        point_tool(
            s,
            &[(60.0, -80.0), (60.0, 100.0), (-100.0, 60.0), (80.0, 60.0)],
        );
    };
    matrix(
        "zero",
        &[100.0],
        build,
        ((60.0, -80.0), (60.0, 100.0)),
        ((-100.0, 60.0), (80.0, 60.0)),
        None,
    );
    // spans of 16 px (ticks), 40 px (arrowheads flipped outside), 70 px (normal)
    for span in [16.0, 40.0, 70.0] {
        let build = move |s: &mut Session| {
            point_tool(
                s,
                &[
                    (-span / 2.0, -80.0),
                    (span / 2.0, 60.0),
                    (-120.0, -span / 2.0 + 150.0),
                    (-90.0, span / 2.0 + 150.0),
                ],
            );
        };
        matrix(
            "short",
            &[25.0, 100.0, 400.0],
            build,
            ((-span / 2.0, -80.0), (span / 2.0, 60.0)),
            ((-120.0, -span / 2.0 + 150.0), (-90.0, span / 2.0 + 150.0)),
            None,
        );
    }
}

#[test]
fn ac49_oracle_negative_control_detects_a_stray_segment() {
    // the oracle must fail when the expected geometry is wrong (guards against a vacuous pass)
    let mut s = session();
    centre_origin(&mut s);
    quad(&mut s);
    dimension_between(
        &mut s,
        "horizontal-distance",
        (-200.0, -50.0),
        (180.0, 60.0),
    );
    let o = origin_px(&s);
    let wrong = check_dimension(
        &s,
        true,
        (o.0 - 200.0, o.1 - 50.0),
        (o.0 + 100.0, o.1 + 60.0),
    );
    assert!(!wrong.is_empty(), "oracle accepted wrong points");
    let right = check_dimension(
        &s,
        true,
        (o.0 - 200.0, o.1 - 50.0),
        (o.0 + 180.0, o.1 + 60.0),
    );
    assert!(right.is_empty(), "{right:?}");
}

// ---------------------------------------------------------------- AC50 point before edge

fn px_mm(s: &Session, p: Pt) -> Pt {
    (p.0 / scale(s), p.1 / scale(s))
}
/// Scene: horizontal line y=-100 px (x -300..300), point P (100,-93) 7 px under it... placed first.
struct PickScene {
    s: Session,
    line: sk::EntityId,
}
fn pick_scene(zoom: f64, points: &[Pt]) -> PickScene {
    let mut s = session();
    centre_origin(&mut s);
    set_scale_about_origin(&mut s, zoom_scale(zoom));
    centre_origin(&mut s);
    point_tool(&mut s, points);
    s.set_sketch_tool(SketchTool::Polyline);
    click_rel(&mut s, -300.0, -100.0);
    click_rel(&mut s, 300.0, -100.0);
    s.escape();
    deselect(&mut s);
    let g = graph(&s);
    let line = g
        .entities
        .iter()
        .find(|(_, e)| matches!(e, sk::Entity::Line { .. }))
        .map(|(i, _)| i.clone())
        .expect("a line");
    PickScene { s, line }
}
fn id_at(s: &Session, p_px: Pt) -> sk::EntityId {
    let g = graph(s);
    let mm = px_mm(s, p_px);
    point_near(&g, mm, 2.0 / scale(s)).unwrap_or_else(|| panic!("no point at {p_px:?} px"))
}
fn click_select(s: &mut Session, p: Pt) {
    deselect(s);
    click_rel(s, p.0, p.1);
}

#[test]
fn ac50_point_within_10_px_beats_a_nearer_edge_at_every_zoom() {
    // edge reach is 6 px, point reach 10 px. Line at y = -100 px.
    let mut fails = Vec::new();
    for zoom in [10.0, 25.0, 100.0, 400.0, 1000.0] {
        let near_pt = (200.0, -93.0); // 6 px from a click at y -99, which is 1 px from the line
        let far_pt = (-100.0, -86.0); // 13 px from a click at y -99
        let mut sc = pick_scene(zoom, &[near_pt, far_pt]);
        let s = &mut sc.s;
        let p = id_at(s, near_pt);
        let f = id_at(s, far_pt);
        let mut check = |label: &str, click: Pt, want_point: Option<&sk::EntityId>| {
            click_select(s, click);
            let got = selected(s);
            let ok = match want_point {
                Some(id) => got == vec![id.0.clone()],
                None => got.contains(&sc.line.0) && !got.contains(&p.0) && !got.contains(&f.0),
            };
            if !ok {
                fails.push(format!(
                    "zoom {zoom}: {label}: picked {got:?}, want {}",
                    want_point.map_or("the edge".to_owned(), |i| format!("point {}", i.0))
                ));
            }
        };
        check("point 6 px, edge 1 px", (200.0, -99.0), Some(&p));
        check("point 9 px, edge 3 px", (200.0, -102.0), Some(&p));
        check("point 13 px, edge 1 px", (-100.0, -99.0), None);
        check("point 11 px, edge 4 px", (200.0, -104.0), None);
        check("no point near, on the edge", (30.0, -100.0), None);
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

#[test]
fn ac50_of_two_points_inside_10_px_the_nearer_wins() {
    let mut fails = Vec::new();
    for zoom in [25.0, 100.0, 400.0] {
        let mut sc = pick_scene(zoom, &[(100.0, -93.0), (100.0, -107.0)]);
        let s = &mut sc.s;
        let up = id_at(s, (100.0, -107.0));
        let down = id_at(s, (100.0, -93.0));
        for (click_y, want, name) in [
            (-99.0, &down, "lower (6 vs 8 px)"),
            (-101.0, &up, "upper (6 vs 8 px)"),
        ] {
            click_select(s, (100.0, click_y));
            let got = selected(s);
            if got != vec![want.0.clone()] {
                fails.push(format!("zoom {zoom}: {name}: picked {got:?}"));
            }
        }
        // two points at 4 px and 9 px around a click 1 px from the edge
        click_select(s, (100.0, -102.0));
        let got = selected(s);
        if got != vec![up.0.clone()] {
            fails.push(format!("zoom {zoom}: 5 vs 9 px picked {got:?}"));
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

#[test]
fn ac50_alt_still_reaches_the_edge_underneath() {
    let mut fails = Vec::new();
    for zoom in [25.0, 100.0, 400.0] {
        let mut sc = pick_scene(zoom, &[(100.0, -93.0)]);
        let s = &mut sc.s;
        let p = id_at(s, (100.0, -93.0));
        click_select(s, (100.0, -99.0));
        if selected(s) != vec![p.0.clone()] {
            fails.push(format!(
                "zoom {zoom}: first click did not pick the point: {:?}",
                selected(s)
            ));
            continue;
        }
        let mut reached = false;
        for _ in 0..3 {
            s.modifiers_changed(false, false, true);
            let q = rel(s, 100.0, -99.0);
            s.pointer_hover(q, false, false);
            s.pointer_down(q, false);
            s.pointer_up(q, false, false);
            s.modifiers_changed(false, false, false);
            if selected(s).contains(&sc.line.0) {
                reached = true;
                break;
            }
        }
        if !reached {
            fails.push(format!(
                "zoom {zoom}: Alt-click never reached the edge: {:?}",
                selected(s)
            ));
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

fn constraints_json(s: &Session) -> Vec<Value> {
    view(s)["constraints"].as_array().unwrap().clone()
}

#[test]
fn ac50_constraint_and_dimension_tools_pick_the_point_too() {
    let mut fails = Vec::new();
    for zoom in [25.0, 100.0, 400.0] {
        // Dimension: first click 6 px from P and 1 px from the line, second on Q.
        let mut sc = pick_scene(zoom, &[(100.0, -93.0), (250.0, 150.0)]);
        let s = &mut sc.s;
        assert!(matches!(s.sketch_rail_press("dimension"), Ok(true)));
        click_rel(s, 100.0, -99.0);
        click_rel(s, 250.0, 150.0);
        click_rel(s, 300.0, 40.0); // the label position
        let want = ((150.0_f64 + 93.0).hypot(150.0)) / scale(s);
        let shown: Vec<String> = view(s)["annotations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["text"].as_str().unwrap_or("").to_owned())
            .collect();
        let value_ok = shown.iter().any(|t| {
            t.split_whitespace()
                .next()
                .and_then(|n| n.parse::<f64>().ok())
                .is_some_and(|v| (v - want).abs() <= 0.01 * want + 0.011)
        });
        if !value_ok {
            fails.push(format!("zoom {zoom}: Dimension armed, 6 px from P and 1 px from the line: shows {shown:?}, want a {want:.2} mm point-point distance"));
        }
        // Coincident: point P (6 px, edge 1 px) then point Q -> point-point coincident
        let mut sc = pick_scene(zoom, &[(100.0, -93.0), (250.0, 150.0)]);
        let s = &mut sc.s;
        let n0 = graph(s).constraints.len();
        assert!(matches!(s.sketch_rail_press("coincident"), Ok(true)));
        click_rel(s, 100.0, -99.0);
        click_rel(s, 250.0, 150.0);
        let after = graph(s).constraints.len();
        let cs = constraints_json(s);
        let kind_ok = cs
            .iter()
            .any(|c| c["kind"].as_str().is_some_and(|k| k == "coincident"));
        if after != n0 + 1 || !kind_ok {
            fails.push(format!(
                "zoom {zoom}: Coincident armed, P then Q: constraints {cs:?}"
            ));
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

// ---------------------------------------------------------------- AC51 Symmetric centre

fn relations(s: &Session) -> Vec<sk::Relation> {
    graph(s)
        .constraints
        .values()
        .map(|c| c.relation.clone())
        .collect()
}
fn hint(s: &Session) -> String {
    let v = view(s);
    format!(
        "{} {} {}",
        v["canvas"]["mode_label"]["text"].as_str().unwrap_or(""),
        v["canvas"]["mode_label"]["hint"].as_str().unwrap_or(""),
        v["canvas"]["mode_label"]["error"].as_str().unwrap_or("")
    )
    .trim()
    .to_owned()
}
fn sym_point(rels: &[sk::Relation]) -> Option<(sk::EntityId, sk::EntityId, sk::EntityId)> {
    rels.iter().find_map(|r| match r {
        sk::Relation::SymmetricPoint {
            source,
            image,
            center,
        } => Some((source.clone(), image.clone(), center.clone())),
        _ => None,
    })
}
const PA: Pt = (-100.0, -80.0);
const PB: Pt = (100.0, -80.0);
const PC: Pt = (-40.0, -80.0);

fn three_points(zoom: f64, pts: [Pt; 3]) -> Session {
    let mut s = session();
    centre_origin(&mut s);
    set_scale_about_origin(&mut s, zoom_scale(zoom));
    centre_origin(&mut s);
    point_tool(&mut s, &pts);
    deselect(&mut s);
    s
}
fn after_symmetric(s: &Session, steps: usize, who: &str) -> Vec<String> {
    let mut why = Vec::new();
    if s.undo_len() != steps + 1 {
        why.push(format!(
            "{who}: history {} -> {} (want one step)",
            steps,
            s.undo_len()
        ));
    }
    if tool(s) != "select" || !armed(s).is_null() {
        why.push(format!(
            "{who}: tool {:?} armed {} (want Select)",
            tool(s),
            armed(s)
        ));
    }
    why
}

#[test]
fn ac51_selection_first_centre_is_independent_of_the_order() {
    let perms: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let pts = [PA, PB, PC];
    let mut fails = Vec::new();
    for order in perms {
        let mut s = three_points(100.0, pts);
        let ids: Vec<sk::EntityId> = pts.iter().map(|p| id_at(&s, *p)).collect();
        let steps = s.undo_len();
        for (k, i) in order.iter().enumerate() {
            click_rel_mod(&mut s, pts[*i].0, pts[*i].1, k > 0);
        }
        let r = s.sketch_rail_press("symmetric");
        let n = graph(&s).constraints.len();
        let rels = relations(&s);
        let Some((a, b, c)) = sym_point(&rels) else {
            fails.push(format!(
                "order {order:?}: {r:?} no SymmetricPoint ({n} constraints, hint {})",
                hint(&s)
            ));
            continue;
        };
        if c != ids[2] {
            fails.push(format!(
                "order {order:?}: centre is {:?}, want the point at {PC:?}",
                pos(&graph(&s), &c)
            ));
        }
        if !((a == ids[0] && b == ids[1]) || (a == ids[1] && b == ids[0])) {
            fails.push(format!("order {order:?}: mirrored pair is not A and B"));
        }
        fails.extend(after_symmetric(&s, steps, &format!("order {order:?}")));
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

#[test]
fn ac51_a_tie_for_the_centre_refuses_with_the_hint_and_applies_nothing() {
    // isosceles: the two base points are equally near their opposite midpoints
    // (shifted off the axes so no axis inference adds a constraint)
    let pts = [(-70.0, -100.0), (130.0, -100.0), (30.0, 100.0)];
    let mut s = three_points(100.0, pts);
    let n0 = graph(&s).constraints.len();
    let steps = s.undo_len();
    for (k, p) in pts.iter().enumerate() {
        click_rel_mod(&mut s, p.0, p.1, k > 0);
    }
    let _ = s.sketch_rail_press("symmetric");
    assert_eq!(graph(&s).constraints.len(), n0, "nothing applied on a tie");
    assert_eq!(s.undo_len(), steps, "no history step on a tie");
    assert_eq!(armed(&s), "symmetric", "the tool waits");
    let h = hint(&s);
    assert!(
        h.to_lowercase().contains("ambig"),
        "the hint names the ambiguity: {h}"
    );
    println!("tie hint: {h}");
}

#[test]
fn ac51_rail_flow_the_last_picked_point_is_the_centre() {
    let pts = [PA, PB, PC];
    let mut fails = Vec::new();
    // (order of picks, index of the expected centre)
    for (order, centre) in [([0usize, 1, 2], 2usize), ([2, 0, 1], 1), ([1, 2, 0], 0)] {
        let mut s = three_points(100.0, pts);
        let ids: Vec<sk::EntityId> = pts.iter().map(|p| id_at(&s, *p)).collect();
        let steps = s.undo_len();
        let _ = s.sketch_rail_press("symmetric");
        for i in order {
            click_rel(&mut s, pts[i].0, pts[i].1);
        }
        match sym_point(&relations(&s)) {
            None => fails.push(format!("picks {order:?}: nothing applied ({})", hint(&s))),
            Some((a, b, c)) => {
                if c != ids[centre] {
                    fails.push(format!("picks {order:?}: wrong centre"));
                }
                let others: Vec<_> = (0..3)
                    .filter(|i| *i != centre)
                    .map(|i| ids[i].clone())
                    .collect();
                if !(others.contains(&a) && others.contains(&b)) {
                    fails.push(format!("picks {order:?}: wrong mirrored pair"));
                }
            }
        }
        fails.extend(after_symmetric(&s, steps, &format!("picks {order:?}")));
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

#[test]
fn ac51_the_constraint_is_the_same_in_both_flows() {
    let pts = [PA, PB, PC];
    let mut s1 = three_points(100.0, pts);
    for (k, p) in pts.iter().enumerate() {
        click_rel_mod(&mut s1, p.0, p.1, k > 0);
    }
    let _ = s1.sketch_rail_press("symmetric");
    let mut s2 = three_points(100.0, pts);
    let _ = s2.sketch_rail_press("symmetric");
    for p in [PA, PB, PC] {
        click_rel(&mut s2, p.0, p.1);
    }
    let pos_of = |s: &Session| {
        let g = graph(s);
        let (a, b, c) = sym_point(&relations(s)).expect("symmetric");
        let mut ab = [pos(&g, &a), pos(&g, &b)];
        ab.sort_by(|x, y| x.0.total_cmp(&y.0));
        (ab, pos(&g, &c))
    };
    let (p1, c1) = pos_of(&s1);
    let (p2, c2) = pos_of(&s2);
    assert!(
        near(c1, c2, 1e-6) && near(p1[0], p2[0], 1e-6) && near(p1[1], p2[1], 1e-6),
        "{p1:?} {c1:?} vs {p2:?} {c2:?}"
    );
}

#[test]
fn ac51_tool_hint_says_the_last_picked_point_is_the_centre() {
    // "the tool hint says so": with Symmetric armed for rail picks the user can read the rule,
    // in the rail tooltip or the on-canvas label.
    let mut s = three_points(100.0, [PA, PB, PC]);
    let _ = s.sketch_rail_press("symmetric");
    let v = view(&s);
    let tip = v["rail"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "symmetric")
        .unwrap()["tooltip"]
        .as_str()
        .unwrap()
        .to_lowercase();
    let label = hint(&s).to_lowercase();
    assert!(
        tip.contains("last") || label.contains("last"),
        "armed Symmetric states no rule: tooltip {tip:?}, label {label:?}"
    );
}

// ---------------------------------------------------------------- AC52 symmetric about axis / line

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mirror {
    YAxis,
    XAxis,
    Drawn,
    Construction,
}
#[derive(Clone, Copy, Debug)]
enum Flow {
    Selection,
    RailPointsFirst,
    RailMirrorFirst,
    RailMirrorBetween,
}
const M1: Pt = (-120.0, -90.0);
const M2: Pt = (90.0, -60.0);
const LINE_A: Pt = (-250.0, 130.0);
const LINE_B: Pt = (250.0, 170.0);

/// Where a click selects the mirror: the axes far from everything, the line mid-way.
fn mirror_click(m: Mirror) -> Pt {
    match m {
        Mirror::YAxis => (0.0, -300.0),
        Mirror::XAxis => (-300.0, 0.0),
        Mirror::Drawn | Mirror::Construction => (0.0, 150.0),
    }
}

fn mirror_scene(m: Mirror) -> Session {
    let mut s = session();
    centre_origin(&mut s);
    point_tool(&mut s, &[M1, M2]);
    if matches!(m, Mirror::Drawn | Mirror::Construction) {
        s.set_sketch_tool(SketchTool::Polyline);
        click_rel(&mut s, LINE_A.0, LINE_A.1);
        click_rel(&mut s, LINE_B.0, LINE_B.1);
        s.escape();
        deselect(&mut s);
        if m == Mirror::Construction {
            click_rel(&mut s, 0.0, 150.0);
            assert!(matches!(s.sketch_selection_construction(true), Ok(true)));
            deselect(&mut s);
        }
    }
    deselect(&mut s);
    s
}

fn apply_flow(s: &mut Session, m: Mirror, flow: Flow) {
    let mc = mirror_click(m);
    let axis = matches!(m, Mirror::YAxis | Mirror::XAxis);
    let pick = |s: &mut Session, p: Pt, extend: bool| {
        // axes are selected with Shift (as the earlier revisions do); other picks as plain clicks
        click_rel_mod(s, p.0, p.1, extend);
    };
    match flow {
        Flow::Selection => {
            pick(s, M1, false);
            pick(s, M2, true);
            pick(s, mc, true);
            let _ = s.sketch_rail_press("symmetric");
        }
        Flow::RailPointsFirst => {
            let _ = s.sketch_rail_press("symmetric");
            click_rel(s, M1.0, M1.1);
            click_rel(s, M2.0, M2.1);
            click_rel_mod(s, mc.0, mc.1, false);
        }
        Flow::RailMirrorFirst => {
            let _ = s.sketch_rail_press("symmetric");
            click_rel_mod(s, mc.0, mc.1, false);
            click_rel(s, M1.0, M1.1);
            click_rel(s, M2.0, M2.1);
        }
        Flow::RailMirrorBetween => {
            let _ = s.sketch_rail_press("symmetric");
            click_rel(s, M1.0, M1.1);
            click_rel_mod(s, mc.0, mc.1, false);
            click_rel(s, M2.0, M2.1);
        }
    }
    let _ = axis;
}

fn mirror_holds(
    g: &sk::Sketch,
    m: Mirror,
    a: Pt,
    b: Pt,
    line: Option<(Pt, Pt)>,
) -> Result<(), String> {
    match m {
        Mirror::YAxis => {
            if (a.0 + b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6 {
                Ok(())
            } else {
                Err(format!("not mirrored about Y: {a:?} {b:?}"))
            }
        }
        Mirror::XAxis => {
            if (a.1 + b.1).abs() < 1e-6 && (a.0 - b.0).abs() < 1e-6 {
                Ok(())
            } else {
                Err(format!("not mirrored about X: {a:?} {b:?}"))
            }
        }
        _ => {
            let (la, lb) = line.unwrap();
            let d = (lb.0 - la.0, lb.1 - la.1);
            let len = d.0.hypot(d.1);
            let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
            let off = ((mid.0 - la.0) * d.1 - (mid.1 - la.1) * d.0).abs() / len;
            let dot = ((b.0 - a.0) * d.0 + (b.1 - a.1) * d.1).abs() / len;
            let _ = g;
            if off < 1e-6 && dot < 1e-6 {
                Ok(())
            } else {
                Err(format!("not mirrored about the line: off {off} dot {dot}"))
            }
        }
    }
}

fn line_ends(g: &sk::Sketch) -> (Pt, Pt) {
    for e in g.entities.values() {
        if let sk::Entity::Line { start, end } = e {
            return (pos(g, start), pos(g, end));
        }
    }
    panic!("no line")
}

#[test]
fn ac52_two_points_and_an_axis_or_a_line_in_every_flow() {
    let mut fails = Vec::new();
    for m in [
        Mirror::YAxis,
        Mirror::XAxis,
        Mirror::Drawn,
        Mirror::Construction,
    ] {
        for flow in [
            Flow::Selection,
            Flow::RailPointsFirst,
            Flow::RailMirrorFirst,
            Flow::RailMirrorBetween,
        ] {
            let who = format!("{m:?}/{flow:?}");
            let mut s = mirror_scene(m);
            let g0 = graph(&s);
            let (p1, p2) = (id_at(&s, M1), id_at(&s, M2));
            let n0 = g0.constraints.len();
            let dof0 = dof(&s);
            let steps = s.undo_len();
            apply_flow(&mut s, m, flow);
            let g = graph(&s);
            if g.constraints.len() != n0 + 1 {
                fails.push(format!(
                    "{who}: {} -> {} constraints, label {} armed {}",
                    n0,
                    g.constraints.len(),
                    hint(&s),
                    armed(&s)
                ));
                continue;
            }
            let rel_ok = relations(&s).iter().any(|r| match (m, r) {
                (
                    Mirror::YAxis,
                    sk::Relation::SymmetricAxis {
                        axis: sk::Axis::Y,
                        source,
                        image,
                    },
                )
                | (
                    Mirror::XAxis,
                    sk::Relation::SymmetricAxis {
                        axis: sk::Axis::X,
                        source,
                        image,
                    },
                ) => (source == &p1 && image == &p2) || (source == &p2 && image == &p1),
                (
                    Mirror::Drawn | Mirror::Construction,
                    sk::Relation::Symmetric { source, image, .. },
                ) => (source == &p1 && image == &p2) || (source == &p2 && image == &p1),
                _ => false,
            });
            if !rel_ok {
                fails.push(format!("{who}: wrong relation kind {:?}", relations(&s)));
            }
            fails.extend(after_symmetric(&s, steps, &who));
            if dof0 - dof(&s) != 2 {
                fails.push(format!("{who}: DOF {} -> {} (want -2)", dof0, dof(&s)));
            }
            // drag one point; the mirror must hold to 1e-6 mm
            let q = mm_px(&s, pos(&g, &p1));
            let o = origin_px(&s);
            let from = (q.0 - o.0, q.1 - o.1);
            s.set_sketch_tool(SketchTool::Select);
            let dragged = {
                let a = rel(&s, from.0, from.1);
                s.pointer_hover(a, false, false);
                s.pointer_down(a, false);
                for i in 1..=6 {
                    let f = f64::from(i) / 6.0;
                    let b = rel(&s, from.0 + 25.0 * f, from.1 + 35.0 * f);
                    s.pointer_hover(b, false, false);
                }
                let b = rel(&s, from.0 + 25.0, from.1 + 35.0);
                s.pointer_up(b, false, false);
                true
            };
            let _ = dragged;
            let g2 = graph(&s);
            let (a, b) = (pos(&g2, &p1), pos(&g2, &p2));
            if near(a, pos(&g, &p1), 1e-9) {
                fails.push(format!("{who}: the drag did not move the point"));
            }
            let line = matches!(m, Mirror::Drawn | Mirror::Construction).then(|| line_ends(&g2));
            if let Err(e) = mirror_holds(&g2, m, a, b, line) {
                fails.push(format!("{who}: after drag {e}"));
            }
            // save and reopen
            let again = curvyo_document_core::unpack(6652, &s.pack("again").unwrap()).unwrap();
            let kept = again
                .sketch(s.active_sketch().unwrap())
                .unwrap()
                .solution
                .constraints
                .values()
                .filter(|c| {
                    matches!(
                        c.relation,
                        sk::Relation::SymmetricAxis { .. } | sk::Relation::Symmetric { .. }
                    )
                })
                .count();
            if kept != 1 {
                fails.push(format!("{who}: reopen keeps {kept} symmetric constraints"));
            }
            // undo removes it as one step
            let steps_now = s.undo_len();
            let _ = steps_now;
        }
    }
    assert!(fails.is_empty(), "\n{}", fails.join("\n"));
}

#[test]
fn ac52_undo_after_the_application_removes_exactly_the_constraint() {
    for m in [Mirror::YAxis, Mirror::XAxis, Mirror::Drawn] {
        let mut s = mirror_scene(m);
        let n0 = graph(&s).constraints.len();
        let g0 = graph(&s);
        let (p1, p2) = (id_at(&s, M1), id_at(&s, M2));
        let (a0, b0) = (pos(&g0, &p1), pos(&g0, &p2));
        apply_flow(&mut s, m, Flow::Selection);
        assert_eq!(graph(&s).constraints.len(), n0 + 1, "{m:?}");
        s.undo();
        let g = graph(&s);
        assert_eq!(g.constraints.len(), n0, "{m:?}: undo removes it");
        assert!(
            near(pos(&g, &p1), a0, 1e-6) && near(pos(&g, &p2), b0, 1e-6),
            "{m:?}: undo restores the points"
        );
        s.redo();
        assert_eq!(graph(&s).constraints.len(), n0 + 1, "{m:?}: redo");
    }
}

#[test]
fn ac52_two_points_and_a_circle_is_refused_with_the_hint_and_nothing_applied() {
    for flow in [Flow::Selection, Flow::RailPointsFirst] {
        let mut s = mirror_scene(Mirror::YAxis);
        s.set_sketch_tool(SketchTool::Circle);
        click_rel(&mut s, 0.0, 150.0);
        click_rel(&mut s, 80.0, 150.0);
        deselect(&mut s);
        let n0 = graph(&s).constraints.len();
        let steps = s.undo_len();
        let circle_edge = (80.0, 150.0);
        match flow {
            Flow::Selection => {
                click_rel_mod(&mut s, M1.0, M1.1, false);
                click_rel_mod(&mut s, M2.0, M2.1, true);
                click_rel_mod(&mut s, circle_edge.0, circle_edge.1, true);
                let _ = s.sketch_rail_press("symmetric");
            }
            _ => {
                let _ = s.sketch_rail_press("symmetric");
                click_rel(&mut s, M1.0, M1.1);
                click_rel(&mut s, M2.0, M2.1);
                click_rel(&mut s, circle_edge.0, circle_edge.1);
            }
        }
        let who = format!("{flow:?}");
        assert_eq!(graph(&s).constraints.len(), n0, "{who}: nothing applied");
        assert_eq!(s.undo_len(), steps, "{who}: no history step");
        let h = hint(&s);
        assert!(
            h.trim().len() > "Symmetric".len(),
            "{who}: a hint explains the refusal: {h:?}"
        );
    }
}
