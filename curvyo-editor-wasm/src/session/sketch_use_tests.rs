//! Sketch pages in the session (`specs/0099-sketch-pages` criteria 1 to 7, 11 to 17).
#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::assert_is_empty,
    clippy::many_single_char_names
)]

use curvyo_document_core::{
    Affine, Angle, FIRST_PAGE_ID, NodeId, ObjectSnapshot, Page, PageKind, Point, Vec2,
};
use curvyo_sketch_core::{Construction, Position};
use curvyo_ui_core::object_bounds;

use crate::session::{Session, Tool, Workbench};

/// A session with the first drawing page and a sketch page right of it holding a 40 x 20 mm
/// rectangle around its centre; the drawing page is active.
pub(in crate::session) fn setup(seed: u64) -> (Session, String, NodeId) {
    let mut s = Session::new(seed);
    s.resize_viewport(1600.0, 900.0);
    let page = s.add_page_of_kind("sketch", None).unwrap();
    s.sketch_construct(&Construction::Rectangle(
        Position::mm(-20.0, -10.0),
        Position::mm(20.0, 10.0),
    ))
    .unwrap();
    let sketch = s.active_sketch().unwrap();
    s.activate_page(FIRST_PAGE_ID);
    (s, page, sketch)
}

pub(in crate::session) fn page(s: &Session, id: &str) -> Page {
    s.document.pages().into_iter().find(|p| p.id == id).unwrap()
}

pub(in crate::session) fn centre_of(s: &Session, id: NodeId) -> Point {
    let object = s.object_read().get(id).cloned().unwrap();
    let (min, max) = object_bounds(&object);
    Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y))
}

fn page_centre(p: &Page) -> Point {
    Point::new(
        p.origin.x + p.size.width.as_mm() / 2.0,
        p.origin.y + p.size.height.as_mm() / 2.0,
    )
}

fn near(a: Point, b: Point) -> bool {
    a.vector_to(b).length() <= 0.01
}

/// A rectangle written directly, so it can lie over a sketch page (a press inside its frame
/// would activate it).
fn rect_at(s: &mut Session, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    s.document
        .create_rect(curvyo_document_core::RectBounds::from_corners(
            Point::new(x, y),
            Point::new(x + w, y + h),
        ))
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

/// Lays the sketch page over the drawing page's right half.
fn overlapping(seed: u64) -> (Session, String, NodeId) {
    let (mut s, sketch_page, sketch) = setup(seed);
    s.move_page_to(&sketch_page, Point::new(150.0, 0.0));
    s.activate_page(FIRST_PAGE_ID);
    (s, sketch_page, sketch)
}

// ---- criteria 2, 3, 4: the sketch is not pickable; objects over it are --------------------

#[test]
fn select_all_and_presses_on_a_drawing_page_never_take_the_sketch() {
    let (mut s, sketch_page, sketch) = overlapping(99_001);
    let rect = rect_at(&mut s, 20.0, 20.0, 40.0, 40.0);
    s.set_tool(Tool::Select);
    s.select_all();
    assert_eq!(s.selection.ids(), &[rect]);
    assert!(s.pickable_objects().iter().all(|o| o.id() != sketch));
    // A press on the sketch's outline with nothing over it activates the sketch page and
    // selects nothing (criterion 2, `0098` criterion 8).
    let edge = Point::new(centre_of(&s, sketch).x - 20.0, centre_of(&s, sketch).y);
    s.escape();
    click(&mut s, edge);
    assert_eq!(s.active_page().id, sketch_page);
    assert_eq!(s.workbench(), Workbench::Sketch);
    s.activate_page(FIRST_PAGE_ID);
    assert!(
        !s.selection.ids().contains(&sketch),
        "the sketch never follows to a drawing page"
    );
}

#[test]
fn a_drawing_object_over_a_sketch_page_is_pressed_and_the_drawing_page_stays() {
    let (mut s, sketch_page, _) = setup(99_002);
    let frame = page(&s, &sketch_page);
    let (x, y) = (frame.origin.x + 10.0, frame.origin.y + 10.0);
    s.activate_page(FIRST_PAGE_ID);
    let rect = rect_at(&mut s, x, y, 30.0, 20.0);
    s.set_tool(Tool::Select);
    s.escape();
    click(&mut s, Point::new(x, y + 10.0));
    assert_eq!(s.selection.ids(), &[rect]);
    assert_eq!(s.active_page().id, FIRST_PAGE_ID);
    assert_eq!(s.workbench(), Workbench::Vector);
    // It belongs to no page: Fit page and the membership read pass it over (criterion 1).
    let membership = s.page_membership();
    let pages = s.document.pages();
    let object = s.object_read().get(rect).cloned().unwrap();
    assert_eq!(
        membership
            .pages_of(
                &pages,
                rect,
                object_bounds(&object),
                curvyo_ui_core::Reach::Touches
            )
            .count(),
        0
    );
}

#[test]
fn the_sketch_select_never_takes_a_drawing_object_over_its_page() {
    let (mut s, sketch_page, sketch) = setup(99_003);
    let frame = page(&s, &sketch_page);
    let (x, y) = (frame.origin.x + 10.0, frame.origin.y + 10.0);
    let rect = rect_at(&mut s, x, y, 30.0, 20.0);
    s.activate_page(&sketch_page);
    click(&mut s, Point::new(x, y + 10.0));
    // Amended by `specs/0106` criterion 3: off sketch geometry the press selects the drawing
    // object move-only, never together with the sketch, and the sketch page stays active.
    assert_eq!(s.selection.ids(), [rect]);
    assert!(!s.selection.ids().contains(&sketch));
    assert_eq!(s.active_page().id, sketch_page);
    assert_eq!(s.edit_scope(), curvyo_ui_core::Scope::MoveOnly);
}

#[test]
fn a_layers_row_press_on_the_sketch_opens_its_page_and_selects_nothing() {
    let (mut s, sketch_page, sketch) = setup(99_004);
    s.press_layer_row(
        &serde_json::to_string(&sketch).unwrap(),
        crate::session::RowPress::Plain,
    );
    assert_eq!(s.active_page().id, sketch_page);
    s.activate_page(FIRST_PAGE_ID);
    assert!(s.selection.ids().is_empty());
}

// ---- criteria 5, 6: the frame move carries the sketch and holds clones ---------------------

#[test]
fn a_sketch_frame_move_carries_only_the_sketch_and_holds_its_clones() {
    let (mut s, sketch_page, sketch) = setup(99_005);
    let frame = page(&s, &sketch_page);
    let (x, y) = (frame.origin.x + 5.0, frame.origin.y + 5.0);
    let rect = rect_at(&mut s, x, y, 10.0, 10.0);
    assert!(
        s.place_sketch_clone_on(FIRST_PAGE_ID) || {
            s.activate_page(&sketch_page);
            s.place_sketch_clone_on(FIRST_PAGE_ID)
        }
    );
    let clone = s.selection.ids()[0];
    s.set_page_frame_only(&sketch_page, true);
    let rows: Vec<serde_json::Value> = serde_json::from_str(&s.pages_json()).unwrap();
    assert!(
        rows.iter()
            .all(|r| r["id"] != sketch_page.as_str() || r["frameOnly"] == false)
    );
    let before = (
        centre_of(&s, sketch),
        centre_of(&s, clone),
        centre_of(&s, rect),
    );
    let steps = s.undo_len();
    let to = Point::new(frame.origin.x + 40.0, frame.origin.y - 30.0);
    assert!(s.move_page_to(&sketch_page, to));
    assert_eq!(s.undo_len(), steps + 1);
    assert!(near(
        centre_of(&s, sketch),
        before.0.translated(Vec2::new(40.0, -30.0))
    ));
    assert!(near(centre_of(&s, clone), before.1), "the clone is held");
    assert!(near(centre_of(&s, rect), before.2), "objects over it stay");
    // A size change keeps the sketch where it is (criterion 6).
    s.activate_page(&sketch_page);
    let kept = centre_of(&s, sketch);
    s.set_document_side(crate::session::DocumentSide::Width, "300");
    assert!(near(centre_of(&s, sketch), kept));
}

// ---- criterion 7: clones follow a changed dimension ----------------------------------------

#[test]
fn clones_follow_a_changed_dimension_by_their_scale_along_their_turned_axis() {
    let mut s = Session::new(99_007);
    s.resize_viewport(1600.0, 900.0);
    let second = s.add_page();
    let sketch_page = s.add_page_of_kind("sketch", None).unwrap();
    s.sketch_construct(&Construction::Polyline(vec![
        Position::mm(0.0, 0.0),
        Position::mm(80.0, 0.0),
    ]))
    .unwrap();
    let graph = s.document.sketch(s.active_sketch().unwrap()).unwrap().graph;
    let line = graph
        .entities
        .iter()
        .find_map(|(id, e)| {
            matches!(e, curvyo_sketch_core::Entity::Line { .. }).then(|| id.clone())
        })
        .unwrap();
    assert!(s.sketch_select_entity(&line, false));
    assert!(matches!(
        s.sketch_rail_press("horizontal-distance"),
        Ok(true)
    ));
    let view: serde_json::Value = serde_json::from_str(&s.sketch_view_json()).unwrap();
    let dimension = view["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["expression"].is_string())
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(s.place_sketch_clone_on(FIRST_PAGE_ID));
    let plain = s.selection.ids()[0];
    s.activate_page(&sketch_page);
    assert!(s.place_sketch_clone_on(&second));
    let turned = s.selection.ids()[0];
    let pivot = centre_of(&s, turned);
    let map = Affine::rotation_about(pivot, Angle::from_radians(12.0_f64.to_radians()))
        .compose(Affine::translation(Vec2::new(pivot.x, pivot.y)))
        .compose(Affine::scale(0.5, 0.5))
        .compose(Affine::translation(Vec2::new(-pivot.x, -pivot.y)));
    let Some(ObjectSnapshot::Clone(clone)) = s.document.object(turned) else {
        panic!("not a clone");
    };
    s.document
        .transform_objects(
            &[ObjectSnapshot::Clone(
                clone.mapped_by(Affine::scale(1.0, 1.0).compose(map)),
            )],
            false,
        )
        .unwrap();
    let span = |s: &Session, id: NodeId| {
        let Some(ObjectSnapshot::Clone(c)) = s.document.object(id) else {
            panic!("not a clone");
        };
        let points: Vec<Point> = c.resolved.all_anchors().map(|a| a.point).collect();
        points[0].vector_to(points[points.len() - 1])
    };
    let (a0, b0) = (span(&s, plain), span(&s, turned));
    s.activate_page(&sketch_page);
    assert!(
        s.sketch_dimension_text(&dimension.clone().into(), "100 mm", false)
            .is_ok()
    );
    let (a1, b1) = (span(&s, plain), span(&s, turned));
    assert!((a1.length() - a0.length() - 20.0).abs() <= 0.01);
    assert!((b1.length() - b0.length() - 10.0).abs() <= 0.01);
    let angle = b1.y.atan2(b1.x).abs().to_degrees();
    assert!(
        (angle - 12.0).abs() <= 0.01 || (angle - 168.0).abs() <= 0.01,
        "{angle}"
    );
}

// ---- criteria 11, 12, 13: Place clone on, usage, Select clones, Select original -----------

#[test]
fn place_clone_on_centres_the_clone_activates_the_page_and_is_one_undo_step() {
    let (mut s, sketch_page, sketch) = setup(99_011);
    let second = s.add_page();
    s.activate_page(&sketch_page);
    let steps = s.undo_len();
    assert!(
        !s.place_sketch_clone_on(&sketch_page),
        "never onto a sketch page"
    );
    assert!(s.place_sketch_clone_on(&second));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(s.active_page().id, second);
    assert_eq!(s.workbench(), Workbench::Vector);
    let clone = s.selection.ids()[0];
    assert!(near(centre_of(&s, clone), page_centre(&page(&s, &second))));
    let ObjectSnapshot::Clone(snapshot) = s.document.object(clone).unwrap() else {
        panic!("not a clone");
    };
    assert!(snapshot.chain.own_placement);
    assert_eq!(snapshot.chain.root, sketch);
    s.undo();
    assert!(s.document.object(clone).is_none());
    assert!(
        s.selection
            .ids()
            .iter()
            .all(|&id| s.document.object(id).is_none())
    );
}

#[test]
fn the_usage_counts_clones_and_their_pages_and_select_clones_reaches_the_first_page() {
    let (mut s, sketch_page, sketch) = setup(99_012);
    let second = s.add_page();
    s.activate_page(&sketch_page);
    assert_eq!(s.sketch_usage(sketch).clones, 0);
    for target in [FIRST_PAGE_ID, second.as_str(), second.as_str()] {
        s.activate_page(&sketch_page);
        assert!(s.place_sketch_clone_on(target));
    }
    let on_first = s.selection.ids()[0];
    let usage = s.sketch_usage(sketch);
    assert_eq!((usage.clones, usage.pages), (3, 2));
    s.activate_page(&sketch_page);
    let result = s.select_sketch_clones();
    assert_eq!(result.code(), "done");
    assert_eq!(s.active_page().id, FIRST_PAGE_ID);
    assert_eq!(s.selection.ids().len(), 1);
    assert_ne!(s.selection.ids()[0], on_first);
}

#[test]
fn select_original_and_a_double_click_on_a_clone_open_the_sketch_page() {
    let (mut s, sketch_page, _) = setup(99_013);
    s.activate_page(&sketch_page);
    assert!(s.place_sketch_clone_on(FIRST_PAGE_ID));
    let clone = s.selection.ids()[0];
    assert_eq!(s.select_original().code(), "done");
    assert_eq!(s.active_page().id, sketch_page);
    s.activate_page(FIRST_PAGE_ID);
    assert!(s.selection.ids().is_empty());
    s.set_tool(Tool::Select);
    let at = centre_of(&s, clone);
    let edge = Point::new(at.x - 20.0, at.y);
    click(&mut s, edge);
    assert_eq!(s.selection.ids(), &[clone]);
    s.double_click(edge, false, false);
    assert_eq!(s.active_page().id, sketch_page);
    assert_eq!(s.workbench(), Workbench::Sketch);
}

// ---- criterion 14: the row state -----------------------------------------------------------

#[test]
fn the_page_row_carries_the_sketch_state_and_its_clone_count() {
    let (mut s, sketch_page, _) = setup(99_014);
    let empty = s.add_page_of_kind("sketch", None).unwrap();
    s.activate_page(&sketch_page);
    assert!(s.place_sketch_clone_on(FIRST_PAGE_ID));
    let rows: Vec<serde_json::Value> = serde_json::from_str(&s.pages_json()).unwrap();
    let row = |id: &str| rows.iter().find(|r| r["id"] == id).unwrap().clone();
    assert_eq!(row(&empty)["sketch"]["state"]["state"], "empty");
    assert_eq!(row(&empty)["sketch"]["drawable"], false);
    assert_eq!(row(&sketch_page)["sketch"]["state"]["state"], "free");
    assert_eq!(row(&sketch_page)["sketch"]["clones"], 1);
    assert!(row(FIRST_PAGE_ID)["sketch"].is_null());
}

// ---- criterion 15: Convert to paths --------------------------------------------------------

#[test]
fn convert_to_paths_makes_a_drawing_page_with_the_paths_selected_and_undo_returns() {
    let (mut s, sketch_page, sketch) = setup(99_015);
    s.activate_page(&sketch_page);
    assert!(s.place_sketch_clone_on(FIRST_PAGE_ID));
    let clone = s.selection.ids()[0];
    let outline = centre_of(&s, clone);
    s.activate_page(&sketch_page);
    let steps = s.undo_len();
    s.sketch_convert_to_path().unwrap();
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(s.document.page_kind(&sketch_page), Some(PageKind::Drawing));
    assert_eq!(s.active_page().id, sketch_page);
    assert_eq!(s.workbench(), Workbench::Vector);
    assert_eq!(s.selection.ids(), &[sketch]);
    assert!(matches!(
        s.document.object(sketch),
        Some(ObjectSnapshot::Path(_))
    ));
    assert!(near(centre_of(&s, clone), outline));
    s.undo();
    assert_eq!(s.document.page_kind(&sketch_page), Some(PageKind::Sketch));
    assert_eq!(s.workbench(), Workbench::Sketch);
    assert_eq!(s.document.page_sketch(&sketch_page), Some(sketch));
}

// ---- criterion 17: Delete a sketch page ----------------------------------------------------

#[test]
fn deleting_a_sketch_page_leaves_paths_where_its_clones_were_and_undo_relinks() {
    let (mut s, sketch_page, sketch) = setup(99_017);
    s.activate_page(&sketch_page);
    assert!(s.place_sketch_clone_on(FIRST_PAGE_ID));
    let clone = s.selection.ids()[0];
    let at = centre_of(&s, clone);
    let steps = s.undo_len();
    assert!(s.delete_page(&sketch_page));
    assert_eq!(s.undo_len(), steps + 1);
    assert!(s.document.object(sketch).is_none());
    assert!(matches!(
        s.document.object(clone),
        Some(ObjectSnapshot::Path(_))
    ));
    assert!(near(centre_of(&s, clone), at));
    s.undo();
    assert!(matches!(
        s.document.object(clone),
        Some(ObjectSnapshot::Clone(_))
    ));
    assert_eq!(s.document.page_sketch(&sketch_page), Some(sketch));
}
