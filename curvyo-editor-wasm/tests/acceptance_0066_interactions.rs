//! Independent sketch interactions: public pointer/command routes and analytical oracles.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Document, Point, unpack};
use curvyo_editor_wasm::{KeyInput, Session, SketchTool, Workbench};
use curvyo_sketch_core as sk;
fn session() -> Session {
    let mut s = Session::new(800);
    s.set_workbench(Workbench::Sketch);
    let mut snap = s.snapping();
    snap.enabled = false;
    s.set_snapping(snap);
    s
}
fn owned_session() -> Session {
    let d = Document::new(802);
    d.create_page_sketch(
        &d.pages()[0].id,
        &sk::Sketch::new("origin".into()),
        &curvyo_document_core::Style::default(),
    )
    .unwrap();
    let mut s = Session::open(
        803,
        &curvyo_document_core::pack(&d, "owned interaction fixture").unwrap(),
    )
    .unwrap();
    s.set_workbench(Workbench::Sketch);
    s
}
fn document(s: &Session) -> Document {
    unpack(801, &s.pack("acceptance").unwrap()).unwrap()
}
fn graph(s: &Session) -> sk::Sketch {
    document(s)
        .sketch(s.active_sketch().unwrap())
        .unwrap()
        .solution
}
fn at(s: &Session, x: f64, y: f64) -> Point {
    let page = s.active_page();
    Point::new(
        page.origin.x + page.size.width.as_mm() / 2.0 + x,
        page.origin.y + page.size.height.as_mm() / 2.0 + y,
    )
}
fn click(s: &mut Session, x: f64, y: f64) {
    let p = at(s, x, y);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}
#[test]
fn s03_s04_actual_gestures_create_analytic_entities_and_keep_correctable_picks() {
    for (tool, picks, kind) in [
        (SketchTool::Point, vec![(30.0, 30.0)], "point"),
        (
            SketchTool::Circle,
            vec![(30.0, 30.0), (40.0, 30.0)],
            "circle",
        ),
        (
            SketchTool::CircleThreePoints,
            vec![(20.0, 30.0), (30.0, 40.0), (40.0, 30.0)],
            "circle",
        ),
        (
            SketchTool::Ellipse,
            vec![(30.0, 30.0), (40.0, 30.0), (30.0, 35.0)],
            "ellipse",
        ),
        (
            SketchTool::EllipseThreePoints,
            vec![(20.0, 30.0), (40.0, 30.0), (36.0, 34.0)],
            "ellipse",
        ),
        (
            SketchTool::Arc,
            vec![(20.0, 30.0), (30.0, 40.0), (40.0, 30.0)],
            "arc",
        ),
        (
            SketchTool::Rectangle,
            vec![(20.0, 20.0), (40.0, 30.0)],
            "line",
        ),
        (
            SketchTool::Slot,
            vec![(20.0, 20.0), (40.0, 20.0), (30.0, 23.0)],
            "arc",
        ),
    ] {
        let mut s = session();
        s.set_sketch_tool(tool);
        let steps = s.undo_len();
        for &(x, y) in &picks {
            click(&mut s, x, y);
        }
        assert_eq!(
            s.undo_len(),
            steps + 1,
            "{tool:?}: {}",
            s.sketch_view_json()
        );
        let g = graph(&s);
        assert!(
            g.entities.iter().any(|(id, e)| id != &g.origin
                && matches!(
                    (kind, e),
                    ("point", sk::Entity::Point { .. })
                        | ("circle", sk::Entity::Circle { .. })
                        | ("ellipse", sk::Entity::Ellipse { .. })
                        | ("arc", sk::Entity::Arc { .. })
                        | ("line", sk::Entity::Line { .. })
                )),
            "{tool:?}"
        );
    }
    let mut s = session();
    s.set_sketch_tool(SketchTool::CircleThreePoints);
    let steps = s.undo_len();
    click(&mut s, 20.0, 20.0);
    click(&mut s, 20.0, 20.0);
    click(&mut s, 30.0, 30.0);
    assert_eq!(s.undo_len(), steps);
    click(&mut s, 40.0, 20.0);
    assert_eq!(
        s.undo_len(),
        steps + 1,
        "duplicate early pick must be correctable: {}",
        s.sketch_view_json()
    );
}
#[test]
fn s04_polyline_escape_keeps_segments_and_shared_identity() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    let count = s.undo_len();
    click(&mut s, 20.0, 20.0);
    click(&mut s, 30.0, 20.0);
    click(&mut s, 30.0, 30.0);
    assert_eq!(s.undo_len(), count + 2);
    let before = graph(&s);
    s.pointer_hover(at(&s, 40.0, 40.0), false, false);
    s.escape();
    assert_eq!(graph(&s), before);
    assert_eq!(s.undo_len(), count + 2);
    let lines: Vec<_> = before
        .entities
        .values()
        .filter_map(|e| {
            if let sk::Entity::Line { start, end } = e {
                Some((start, end))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].1 == lines[1].0 || lines[1].1 == lines[0].0);
    s.key_down(KeyInput {
        key: "Enter",
        ..KeyInput::default()
    });
    assert_eq!(s.undo_len(), count + 2);
}
#[test]
fn s07_formula_rename_and_invalid_cycle_keep_valid_values_and_history() {
    let mut s = session();
    let width = s
        .sketch_add_parameter("width", "80 mm", sk::Dimension::Length)
        .unwrap();
    s.sketch_add_parameter("margin", "10 mm", sk::Dimension::Length)
        .unwrap();
    let spacing = s
        .sketch_add_parameter("spacing", "width - 2 * margin", sk::Dimension::Length)
        .unwrap();
    assert!((graph(&s).parameters.evaluate().unwrap()[&spacing].value() - 60.0).abs() < 1e-6);
    s.sketch_parameter_text(
        width.clone(),
        "plate_width",
        "100 mm",
        sk::Dimension::Length,
    )
    .unwrap();
    assert!((graph(&s).parameters.evaluate().unwrap()[&spacing].value() - 80.0).abs() < 1e-6);
    let before = graph(&s);
    let steps = s.undo_len();
    assert!(
        s.sketch_parameter_text(width, "plate_width", "spacing", sk::Dimension::Length)
            .is_err()
    );
    assert_eq!(s.undo_len(), steps);
    assert_eq!(graph(&s), before);
}
#[test]
fn s01_navigation_is_inert_and_first_page_owner_edit_has_exactly_one_step() {
    let mut s = Session::new(810);
    let initial = s.undo_len();
    s.set_workbench(Workbench::Sketch);
    assert_eq!(s.undo_len(), initial);
    s.set_workbench(Workbench::Vector);
    s.new_layer_row();
    let layer = s.active_layer().unwrap();
    let count = s.undo_len();
    s.set_workbench(Workbench::Sketch);
    assert_eq!(s.undo_len(), count);
    assert!(s.active_sketch().is_none());
    s.set_sketch_tool(SketchTool::Point);
    click(&mut s, 30.0, 30.0);
    let id = s.active_sketch().unwrap();
    assert_eq!(s.undo_len(), count + 1);
    assert_eq!(document(&s).page_sketch(&s.active_page().id), Some(id));
    assert!(document(&s).contains_node(layer));
    let before = document(&s).version();
    s.set_workbench(Workbench::Vector);
    s.set_workbench(Workbench::Sketch);
    assert_eq!(s.undo_len(), count + 1);
    assert_eq!(document(&s).version(), before);
}

#[test]
fn s11_rectangle_ignores_vector_snapping_and_preserves_anchor_history_in_both_phases() {
    for stored in [false, true] {
        for (x, y) in [(true, true), (true, false), (false, true), (false, false)] {
            for held in [false, true] {
                for alt in [true, false] {
                    let mut s = owned_session();
                    s.set_snapping(curvyo_document_core::DocumentSnapping {
                        enabled: stored,
                        objects: false,
                        guides: false,
                        nodes: false,
                        page: true,
                        x,
                        y,
                        ..curvyo_document_core::DocumentSnapping::DEFAULT
                    });
                    s.set_sketch_tool(SketchTool::Rectangle);
                    let steps = s.undo_len();
                    if held {
                        s.pointer_down(at(&s, 20.0, 20.0), false);
                    } else {
                        click(&mut s, 20.0, 20.0);
                    }
                    s.pointer_hover(at(&s, 104.0, 149.0), false, false);
                    s.modifiers_changed(false, false, true);
                    if !alt {
                        s.modifiers_changed(false, false, false);
                    }
                    assert_eq!(s.undo_len(), steps);
                    assert_eq!(s.snapping().enabled, stored);
                    assert!(s.sketch_commit_preview().unwrap());
                    let preview = graph(&s);
                    let points: Vec<_> = preview
                        .entities
                        .iter()
                        .filter_map(|(id, e)| {
                            if id == &preview.origin {
                                return None;
                            }
                            if let sk::Entity::Point { position, .. } = e {
                                Some(*position)
                            } else {
                                None
                            }
                        })
                        .collect();
                    let max_x = points
                        .iter()
                        .map(|p| p.x.0)
                        .fold(f64::NEG_INFINITY, f64::max);
                    let max_y = points
                        .iter()
                        .map(|p| p.y.0)
                        .fold(f64::NEG_INFINITY, f64::max);
                    // Revised AC: Sketch never uses Vector target snapping, even with Alt.
                    assert!((max_x - 104.0).abs() < 1e-6);
                    assert!((max_y - 149.0).abs() < 1e-6);
                    assert!(
                        points
                            .iter()
                            .any(|p| (p.x.0 - 20.0).abs() < 1e-6 && (p.y.0 - 20.0).abs() < 1e-6)
                    );
                    assert_eq!(s.undo_len(), steps + 1);
                }
            }
        }
    }
}
