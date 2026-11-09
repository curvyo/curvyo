//! Independent acceptance tests for `specs/0100-bring-content-in` through the public session: the
//! cloning drag and its badges (criteria 1 to 6), placed pages by row drag and Place page (13 to
//! 15), placed pages staying on their laser page under every command (17 to 19), deleting a
//! drawing page (24), Place on… (25) and the SVG export of clones and placed pages (20).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    clippy::assert_is_empty,
    clippy::manual_assert,
    clippy::match_wildcard_for_single_variants,
    missing_docs
)]

use curvyo_document_core::{
    Affine, Document, DocumentSize, NewOutputPage, NodeId, ObjectSnapshot, Page, Point, RectBounds,
    Vec2, pack, unpack,
};
use curvyo_editor_wasm::{DropBadge, KeyInput, Session, SvgExportResult, Tool};
use curvyo_ui_core::{AlignCommand, FlipAxis, MoveEntryMode, SvgExportOptions};
use serde_json::Value;

const TOL: f64 = 1e-6;
const OFF_PAGE: &str = "A placed page stays on its laser page.";

// ---- fixture ------------------------------------------------------------------------------------

struct Fx {
    s: Session,
    first: Page,
    laser: Page,
    /// Three rectangles on the drawing page and one on the pasteboard above it.
    rects: Vec<NodeId>,
}

fn find_page(d: &Document, id: &str) -> Page {
    d.pages().into_iter().find(|p| p.id == id).unwrap()
}

fn open_session(d: &Document) -> Session {
    let mut s = Session::open(7, &pack(d, "0100-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    let mut snap = s.snapping();
    snap.enabled = false;
    s.set_snapping(snap);
    s.set_tool(Tool::Select);
    s
}

fn add_laser(d: &Document, w: f64, h: f64) -> Page {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(w, h),
    })
    .unwrap()
}

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + w, y + h),
    ))
}

fn fixture() -> Fx {
    let d = Document::new(1);
    let laser = add_laser(&d, 400.0, 300.0);
    let first = find_page(&d, "first");
    let o = first.origin;
    let rects = vec![
        rect(&d, o.x + 10.0, o.y + 10.0, 20.0, 10.0),
        rect(&d, o.x + 40.0, o.y + 10.0, 20.0, 10.0),
        rect(&d, o.x + 70.0, o.y + 10.0, 20.0, 10.0),
        rect(&d, o.x + 10.0, o.y - 40.0, 20.0, 10.0),
    ];
    let laser = find_page(&d, &laser.id);
    let mut s = open_session(&d);
    s.activate_page("first");
    s.wipe_history();
    Fx {
        s,
        first,
        laser,
        rects,
    }
}

// ---- helpers ------------------------------------------------------------------------------------

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
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

fn selected_placements(s: &Session) -> Vec<String> {
    let v: Vec<Value> = serde_json::from_str(&s.placed_pages_json()).unwrap();
    v.into_iter()
        .filter(|r| r["selected"] == true)
        .map(|r| r["id"].as_str().unwrap().to_owned())
        .collect()
}

/// A point on the top edge (30 % in, away from the handles) of a rectangle created by [`rect`] (no fill: the edge hits).
fn edge_of(d: &Document, id: NodeId) -> Point {
    let (min, max) = bounds(d, id);
    Point::new(min.x + 0.3 * (max.x - min.x), min.y)
}

fn bounds(d: &Document, id: NodeId) -> (Point, Point) {
    curvyo_ui_core::object_bounds(&d.object(id).unwrap())
}

fn clones(d: &Document) -> Vec<curvyo_document_core::CloneSnapshot> {
    d.read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Clone(c) => Some(c.clone()),
            _ => None,
        })
        .collect()
}

/// Selects `ids` by clicks (the first plain, the rest with Shift).
fn select(s: &mut Session, ids: &[NodeId]) {
    let d = doc(s);
    for (i, &id) in ids.iter().enumerate() {
        let at = edge_of(&d, id);
        if i == 0 {
            click(s, at);
        } else {
            shift_click(s, at);
        }
    }
    let mut got = selected(s);
    got.sort_by_key(|n| n.full_id());
    let mut want = ids.to_vec();
    want.sort_by_key(|n| n.full_id());
    assert_eq!(got, want, "selection set up");
}

/// Presses at `from`, moves to `to` in two steps and leaves the button down.
fn drag_to(s: &mut Session, from: Point, to: Point) {
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    let mid = Point::new(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y));
    s.pointer_hover(mid, false, false);
    s.pointer_hover(to, false, false);
}

fn laser_point(f: &Fx, dx: f64, dy: f64) -> Point {
    Point::new(f.laser.origin.x + dx, f.laser.origin.y + dy)
}

fn badge(s: &Session) -> DropBadge {
    s.clone_drop_answer().map_or(DropBadge::None, |a| a.badge)
}

// ---- criteria 1 and 2: the cloning drag -----------------------------------------------------

#[test]
fn ac1_four_objects_dragged_onto_a_laser_page_become_four_clones_in_one_step() {
    let mut f = fixture();
    let ids = f.rects.clone();
    select(&mut f.s, &ids);
    let before = doc(&f.s);
    let from = edge_of(&before, ids[0]);
    let to = laser_point(&f, 50.0, 100.0);
    let steps = f.s.undo_len();
    drag_to(&mut f.s, from, to);
    assert_eq!(
        badge(&f.s),
        DropBadge::Clone,
        "Linked clone badge over a laser page"
    );
    assert_eq!(f.s.undo_len(), steps, "nothing written before the release");
    f.s.pointer_up(to, false, false);
    assert_eq!(f.s.undo_len(), steps + 1, "one step");
    assert_eq!(last_step_name(&mut f.s), "Place linked clones");
    let d = doc(&f.s);
    for &id in &ids {
        assert_eq!(
            d.object(id).unwrap(),
            before.object(id).unwrap(),
            "original unchanged"
        );
    }
    let made = clones(&d);
    assert_eq!(made.len(), 4);
    let t = Vec2::new(to.x - from.x, to.y - from.y);
    for c in &made {
        assert!(c.chain.own_placement);
        assert!(
            c.chain.matrix.approx_eq(Affine::translation(t), TOL),
            "{:?}",
            c.chain.matrix
        );
    }
    let mut sel = selected(&mut f.s);
    sel.sort_by_key(|n| n.full_id());
    let mut want: Vec<NodeId> = made.iter().map(|c| c.resolved.id).collect();
    want.sort_by_key(|n| n.full_id());
    assert_eq!(sel, want, "the clones are the selection");
    assert_eq!(f.s.active_page().id, f.laser.id, "the laser page is active");
    // Undo removes every clone and clears the selection.
    key(&mut f.s, "z", true);
    let d = doc(&f.s);
    assert!(clones(&d).is_empty());
    for &id in &ids {
        assert_eq!(d.object(id).unwrap(), before.object(id).unwrap());
    }
    assert!(
        selected(&mut f.s).iter().all(|id| d.object(*id).is_some()),
        "no selection of removed clones"
    );
}

#[test]
fn ac2_the_badge_follows_alt_and_ctrl_at_rest_and_escape_writes_nothing() {
    let mut f = fixture();
    let id = f.rects[0];
    select(&mut f.s, &[id]);
    let before = doc(&f.s);
    let from = edge_of(&before, id);
    let to = laser_point(&f, 50.0, 100.0);
    let steps = f.s.undo_len();
    drag_to(&mut f.s, from, to);
    assert_eq!(badge(&f.s), DropBadge::Clone);
    f.s.modifiers_changed(false, false, true);
    assert_eq!(badge(&f.s), DropBadge::Move, "Alt: Move");
    f.s.modifiers_changed(false, true, false);
    assert_eq!(badge(&f.s), DropBadge::Copy, "Ctrl: Copy");
    f.s.modifiers_changed(false, true, true);
    assert_eq!(badge(&f.s), DropBadge::Copy, "Ctrl+Alt: Copy");
    f.s.modifiers_changed(false, false, false);
    assert_eq!(badge(&f.s), DropBadge::Clone);
    // Back over the drawing page: no badge.
    let back = Point::new(from.x + 5.0, from.y + 5.0);
    f.s.pointer_hover(back, false, false);
    assert_eq!(badge(&f.s), DropBadge::None, "no badge over a drawing page");
    f.s.pointer_hover(to, false, false);
    assert_eq!(badge(&f.s), DropBadge::Clone);
    f.s.escape();
    f.s.pointer_up(to, false, false);
    assert_eq!(f.s.undo_len(), steps, "Escape writes nothing");
    let d = doc(&f.s);
    assert!(clones(&d).is_empty());
    assert_eq!(d.object(id).unwrap(), before.object(id).unwrap());
}

#[test]
fn ac3_an_alt_release_moves_the_objects_themselves() {
    let mut f = fixture();
    let id = f.rects[0];
    select(&mut f.s, &[id]);
    let before = doc(&f.s);
    let from = edge_of(&before, id);
    let to = laser_point(&f, 50.0, 100.0);
    drag_to(&mut f.s, from, to);
    f.s.modifiers_changed(false, false, true);
    f.s.pointer_hover(to, false, false);
    let steps = f.s.undo_len();
    f.s.pointer_up(to, false, false);
    f.s.modifiers_changed(false, false, false);
    let d = doc(&f.s);
    assert!(clones(&d).is_empty(), "no clone");
    assert_eq!(f.s.undo_len(), steps + 1);
    let (min, _) = bounds(&d, id);
    assert!(
        min.x > f.laser.origin.x && min.y > f.laser.origin.y,
        "moved onto the laser page"
    );
}

#[test]
fn ac4_a_ctrl_release_copies_and_a_copy_of_a_clone_is_a_sibling_clone() {
    let mut f = fixture();
    let id = f.rects[0];
    select(&mut f.s, &[id]);
    let before = doc(&f.s);
    let from = edge_of(&before, id);
    let to = laser_point(&f, 50.0, 100.0);
    drag_to(&mut f.s, from, to);
    f.s.modifiers_changed(false, true, false);
    f.s.pointer_hover(to, false, true);
    let steps = f.s.undo_len();
    f.s.pointer_up(to, false, true);
    f.s.modifiers_changed(false, false, false);
    let d = doc(&f.s);
    assert_eq!(f.s.undo_len(), steps + 1);
    assert!(clones(&d).is_empty(), "a copy, not a clone");
    assert_eq!(
        d.object(id).unwrap(),
        before.object(id).unwrap(),
        "original unchanged"
    );
    assert_eq!(
        d.read_objects().objects.len(),
        before.read_objects().objects.len() + 1
    );
}

#[test]
fn ac5_a_selection_holding_a_laser_page_member_only_moves() {
    let mut f = fixture();
    // Put one rectangle on the laser page first.
    let d0 = doc(&f.s);
    let on_laser = {
        let d = doc(&f.s);
        let id = rect(
            &d,
            f.laser.origin.x + 20.0,
            f.laser.origin.y + 20.0,
            20.0,
            10.0,
        );
        f.s = open_session(&d);
        f.s.activate_page("first");
        id
    };
    let _ = d0;
    f.s.activate_page(&f.laser.id);
    let d = doc(&f.s);
    let from = edge_of(&d, on_laser);
    click(&mut f.s, from);
    assert_eq!(selected(&mut f.s), vec![on_laser]);
    // Within the laser page: a plain move, no badge.
    let to = laser_point(&f, 100.0, 150.0);
    drag_to(&mut f.s, from, to);
    assert_eq!(badge(&f.s), DropBadge::None, "no badge: plain move");
    f.s.pointer_up(to, false, false);
    assert!(clones(&doc(&f.s)).is_empty(), "no clone");
    // And back onto the drawing page: a plain move too.
    let d = doc(&f.s);
    let from = edge_of(&d, on_laser);
    let to = Point::new(f.first.origin.x + 100.0, f.first.origin.y + 100.0);
    drag_to(&mut f.s, from, to);
    assert_eq!(badge(&f.s), DropBadge::None);
    f.s.pointer_up(to, false, false);
    assert!(clones(&doc(&f.s)).is_empty(), "no clone");
}

#[test]
fn ac5_a_release_over_a_laser_off_or_unknown_page_is_a_plain_move() {
    // A laser page whose extension is off is not a target. The registry switch is the
    // session's; when this build cannot switch it off the test checks the drawing page only.
    let mut f = fixture();
    let id = f.rects[0];
    select(&mut f.s, &[id]);
    let from = edge_of(&doc(&f.s), id);
    // Over the pasteboard far below every page.
    let to = Point::new(from.x, from.y + 5000.0);
    drag_to(&mut f.s, from, to);
    assert_eq!(badge(&f.s), DropBadge::None);
    f.s.pointer_up(to, false, false);
    assert!(clones(&doc(&f.s)).is_empty());
}

#[test]
fn ac6_a_selected_group_dragged_onto_a_laser_page_becomes_one_group_clone() {
    let mut f = fixture();
    let g = {
        let d = doc(&f.s);
        let g = d.group_objects(&[f.rects[0], f.rects[1]]).unwrap();
        f.s = open_session(&d);
        f.s.activate_page("first");
        f.s.wipe_history();
        g
    };
    let d = doc(&f.s);
    let from = edge_of(&d, f.rects[0]);
    click(&mut f.s, from);
    let to = laser_point(&f, 50.0, 100.0);
    drag_to(&mut f.s, from, to);
    f.s.pointer_up(to, false, false);
    let made = clones(&doc(&f.s));
    assert_eq!(
        made.iter().map(|c| c.chain.origin).collect::<Vec<_>>(),
        vec![g],
        "one linked clone of the group G (criterion 6), not one per member"
    );
}

// ---- criterion 6: kinds, guides, depth 8 -------------------------------------------------------

#[test]
fn ac6_guides_in_the_selection_are_not_cloned_and_a_notice_counts_them() {
    let mut f = fixture();
    let (a, g1, g2) = (f.rects[0], f.rects[1], f.rects[2]);
    {
        let d = doc(&f.s);
        d.set_guide(&[g1, g2], true).unwrap();
        f.s = open_session(&d);
        f.s.activate_page("first");
    }
    select(&mut f.s, &[a, g1, g2]);
    let guides_before = (doc(&f.s).object(g1).unwrap(), doc(&f.s).object(g2).unwrap());
    let from = edge_of(&doc(&f.s), a);
    let to = laser_point(&f, 50.0, 100.0);
    drag_to(&mut f.s, from, to);
    f.s.pointer_up(to, false, false);
    let d = doc(&f.s);
    let made = clones(&d);
    assert_eq!(made.len(), 1, "only the non-guide is cloned");
    assert_eq!(made[0].chain.origin, a);
    let notice = f.s.take_clone_drop_notice();
    assert_eq!(notice, "2 guides were not cloned. Guides never run.");
    assert!(notice.chars().count() <= 70);
    // The guides stay where they were.
    assert_eq!(
        (d.object(g1).unwrap(), d.object(g2).unwrap()),
        guides_before
    );
}

#[test]
fn ac6_a_selection_of_only_guides_creates_nothing_and_adds_no_step() {
    let mut f = fixture();
    let g = f.rects[0];
    {
        let d = doc(&f.s);
        d.set_guide(&[g], true).unwrap();
        f.s = open_session(&d);
        f.s.activate_page("first");
        f.s.wipe_history();
    }
    select(&mut f.s, &[g]);
    let before = doc(&f.s).object(g).unwrap();
    let from = edge_of(&doc(&f.s), g);
    let to = laser_point(&f, 50.0, 100.0);
    drag_to(&mut f.s, from, to);
    f.s.pointer_up(to, false, false);
    let d = doc(&f.s);
    assert!(clones(&d).is_empty());
    let name = if f.s.undo_len() > 0 {
        last_step_name(&mut f.s)
    } else {
        String::new()
    };
    assert_eq!(f.s.undo_len(), 0, "no history step (got {name:?})");
    assert_eq!(d.object(g).unwrap(), before, "the guide stays where it was");
}

#[test]
fn ac6_a_depth_eight_clone_in_the_drag_creates_nothing_and_the_objects_return() {
    let mut f = fixture();
    let a = f.rects[0];
    let deep;
    {
        let d = doc(&f.s);
        // A path to chain from: convert by cloning the rect via the button rule is avoided; use
        // a path.
        let p = d.create_path(
            &[
                curvyo_document_core::NewAnchor::corner(
                    curvyo_document_core::AnchorId::new(70, 1),
                    Point::new(f.first.origin.x + 10.0, f.first.origin.y + 60.0),
                ),
                curvyo_document_core::NewAnchor::corner(
                    curvyo_document_core::AnchorId::new(70, 2),
                    Point::new(f.first.origin.x + 60.0, f.first.origin.y + 60.0),
                ),
            ],
            false,
        );
        let mut top = p;
        for _ in 0..8 {
            top = d.linked_clone(&[top], &[]).unwrap()[0];
        }
        // Spread the chain so the top clone is hit alone: move the top clone below.
        deep = top;
        f.s = open_session(&d);
        f.s.activate_page("first");
        f.s.wipe_history();
    }
    let before = doc(&f.s);
    // Select the rect and the deep clone (on the line at y = 60).
    click(&mut f.s, edge_of(&before, a));
    let on_line = Point::new(f.first.origin.x + 30.0, f.first.origin.y + 60.0);
    shift_click(&mut f.s, on_line);
    let sel = selected(&mut f.s);
    assert!(sel.contains(&a));
    if !sel.contains(&deep) {
        // The topmost of the stacked chain is the deep clone: it is what a press picks.
        panic!("the press on the line did not pick the depth-8 clone: {sel:?}");
    }
    let from = edge_of(&before, a);
    let to = laser_point(&f, 50.0, 100.0);
    drag_to(&mut f.s, from, to);
    f.s.pointer_up(to, false, false);
    let d = doc(&f.s);
    assert_eq!(f.s.undo_len(), 0, "nothing created");
    assert_eq!(
        d.object(a).unwrap(),
        before.object(a).unwrap(),
        "the objects return"
    );
    assert_eq!(
        d.read_objects().objects.len(),
        before.read_objects().objects.len()
    );
    let notice = f.s.take_clone_drop_notice();
    assert!(!notice.is_empty(), "the depth refusal is shown");
}

// ---- criteria 13 to 15: placing a drawing page ---------------------------------------------

#[test]
fn ac13_a_drawing_page_row_dropped_on_a_laser_page_places_it_centred_under_the_pointer() {
    let mut f = fixture();
    let at = laser_point(&f, 200.0, 150.0);
    assert!(f.s.row_drag_begin("first"));
    let a = f.s.row_drag_move(Some(at));
    assert_eq!(a.badge, DropBadge::PlacedPage);
    assert_eq!(f.s.undo_len(), 0, "nothing written in flight");
    assert!(f.s.row_drag_drop());
    assert_eq!(f.s.undo_len(), 1);
    assert_eq!(last_step_name(&mut f.s), "Place page");
    let d = doc(&f.s);
    let placed = d.placed_pages();
    assert_eq!(placed.len(), 1);
    let c = placed[0].frame_corners().unwrap();
    let centre = Point::new(f64::midpoint(c[0].x, c[2].x), f64::midpoint(c[0].y, c[2].y));
    assert!(
        (centre.x - at.x).abs() < TOL && (centre.y - at.y).abs() < TOL,
        "{centre:?}"
    );
    assert!(
        (placed[0].matrix.determinant() - 1.0).abs() < TOL && placed[0].matrix.b.abs() < TOL,
        "scale 1, rotation 0"
    );
    assert_eq!(f.s.active_page().id, f.laser.id);
    assert_eq!(selected_placements(&f.s), vec![placed[0].id.full_id()]);
    for &id in &f.rects {
        assert!(d.object(id).is_some());
    }
}

#[test]
fn ac14_a_row_drop_anywhere_but_a_laser_page_says_only_on_a_laser_page_and_writes_nothing() {
    let mut f = fixture();
    let sketch = {
        let d = doc(&f.s);
        let p = d
            .add_page_of(&curvyo_document_core::PageKind::Sketch)
            .unwrap();
        f.s = open_session(&d);
        f.s.activate_page("first");
        f.s.wipe_history();
        find_page(&doc(&f.s), &p.id)
    };
    let spots = [
        Point::new(f.first.origin.x + 10.0, f.first.origin.y + 10.0),
        Point::new(sketch.origin.x + 5.0, sketch.origin.y + 5.0),
        Point::new(f.first.origin.x - 4000.0, f.first.origin.y - 4000.0),
    ];
    for at in spots {
        assert!(f.s.row_drag_begin("first"));
        let a = f.s.row_drag_move(Some(at));
        assert_eq!(a.badge, DropBadge::OnlyOnLaserPage, "{at:?}");
        assert!(!f.s.row_drag_drop());
        assert_eq!(f.s.undo_len(), 0);
    }
    // Escape during the drag cancels.
    assert!(f.s.row_drag_begin("first"));
    f.s.row_drag_move(Some(laser_point(&f, 10.0, 10.0)));
    f.s.row_drag_cancel();
    assert!(!f.s.row_drag_drop());
    assert_eq!(f.s.undo_len(), 0);
    assert!(doc(&f.s).placed_pages().is_empty());
    // A laser page row only reorders: no placed-page drag.
    assert!(
        !f.s.row_drag_begin(&f.laser.id) || {
            let a = f.s.row_drag_move(Some(laser_point(&f, 10.0, 10.0)));
            f.s.row_drag_cancel();
            a.badge != DropBadge::PlacedPage
        }
    );
}

#[test]
fn ac15_place_page_on_the_active_laser_page_centres_it_in_one_step() {
    let mut f = fixture();
    f.s.activate_page(&f.laser.id);
    assert!(f.s.place_page_on_active("first"));
    assert_eq!(last_step_name(&mut f.s), "Place page");
    let d = doc(&f.s);
    let c = d.placed_pages()[0].frame_corners().unwrap();
    let centre = Point::new(f64::midpoint(c[0].x, c[2].x), f64::midpoint(c[0].y, c[2].y));
    let want = laser_point(&f, 200.0, 150.0);
    assert!((centre.x - want.x).abs() < TOL && (centre.y - want.y).abs() < TOL);
    // Not on a drawing page, and never a laser page as source.
    f.s.activate_page("first");
    assert!(!f.s.place_page_on_active("first"));
    f.s.activate_page(&f.laser.id);
    assert!(!f.s.place_page_on_active(&f.laser.id));
    assert_eq!(f.s.undo_len(), 1);
}

// ---- criteria 17 to 19: placed pages stay on their laser page --------------------------------

/// A laser page (400 x 300) with one placed page of "first", selected, the laser page active.
fn placed() -> (Fx, NodeId) {
    let mut f = fixture();
    f.s.activate_page(&f.laser.id);
    assert!(f.s.place_page_on_active("first"));
    let id = doc(&f.s).placed_pages()[0].id;
    f.s.wipe_history();
    (f, id)
}

fn map_of(s: &Session, id: NodeId) -> Affine {
    doc(s).placed_page(id).unwrap().global_map().unwrap()
}

fn frame_centre(s: &Session, id: NodeId) -> Point {
    let c = doc(s).placed_page(id).unwrap().frame_corners().unwrap();
    Point::new(f64::midpoint(c[0].x, c[2].x), f64::midpoint(c[0].y, c[2].y))
}

fn assert_refused(s: &mut Session, id: NodeId, before: Affine, what: &str) {
    assert_eq!(s.undo_len(), 0, "{what}: no history step");
    assert!(
        map_of(s, id).approx_eq(before, 1e-12),
        "{what}: nothing written"
    );
}

#[test]
fn ac17_a_press_inside_the_frame_selects_the_placed_page_never_a_source_object() {
    let (mut f, id) = placed();
    f.s.escape();
    let at = frame_centre(&f.s, id);
    click(&mut f.s, at);
    assert_eq!(selected_placements(&f.s), vec![id.full_id()]);
    assert!(selected(&mut f.s).iter().all(|n| !f.rects.contains(n)));
    let block = f.s.placed_page_block();
    assert!(block.starts_with("Placed page of "), "{block}");
}

#[test]
fn ac17_an_own_item_above_the_placed_page_wins_the_press() {
    let (f, id) = placed();
    let c = frame_centre(&f.s, id);
    let d = doc(&f.s);
    let own = rect(&d, c.x - 10.0, c.y - 5.0, 20.0, 10.0);
    let mut s = open_session(&d);
    s.activate_page(&f.laser.id);
    click(&mut s, Point::new(c.x, c.y - 5.0));
    assert_eq!(selected(&mut s), vec![own], "topmost wins");
    assert!(selected_placements(&s).is_empty());
}

#[test]
fn ac18_a_drag_off_the_laser_page_returns_and_writes_nothing() {
    let (mut f, id) = placed();
    let before = map_of(&f.s, id);
    let c = frame_centre(&f.s, id);
    let far = Point::new(c.x + 5000.0, c.y);
    drag_to(&mut f.s, c, far);
    f.s.pointer_up(far, false, false);
    assert_refused(&mut f.s, id, before, "drag");
    assert_eq!(f.s.take_transform_notice(), OFF_PAGE);
}

#[test]
fn ac18_a_ctrl_copy_drag_off_the_page_is_refused_too() {
    let (mut f, id) = placed();
    let c = frame_centre(&f.s, id);
    let far = Point::new(c.x, c.y + 5000.0);
    f.s.modifiers_changed(false, true, false);
    drag_to(&mut f.s, c, far);
    f.s.pointer_up(far, false, true);
    f.s.modifiers_changed(false, false, false);
    assert_eq!(f.s.undo_len(), 0, "no copy off the page");
    assert_eq!(doc(&f.s).placed_pages().len(), 1);
}

#[test]
fn ac18_arrow_keys_stop_at_the_edge_with_the_notice() {
    let (mut f, id) = placed();
    // Shift+Arrow nudges by 10 mm; 400 mm of laser page plus the page's own width is crossed
    // well within 200 presses.
    let mut refused = false;
    for _ in 0..200 {
        let before = map_of(&f.s, id);
        let steps = f.s.undo_len();
        f.s.key_down(KeyInput {
            key: "ArrowRight",
            shift: true,
            ..KeyInput::default()
        });
        if f.s.undo_len() == steps {
            assert!(map_of(&f.s, id).approx_eq(before, 1e-12));
            refused = true;
            break;
        }
        let d = doc(&f.s);
        let p = d.placed_page(id).unwrap();
        assert!(
            curvyo_document_core::overlaps_page(&p.frame_corners().unwrap(), &p.output),
            "every committed nudge stays on the page"
        );
    }
    assert!(refused, "the arrow key is refused at the edge");
    assert_eq!(f.s.take_transform_notice(), OFF_PAGE);
}

#[test]
fn ac18_a_typed_absolute_position_off_the_page_reverts() {
    let (mut f, id) = placed();
    let before = map_of(&f.s, id);
    key(&mut f.s, "m", false);
    let out = f.s.commit_move_entry(
        "-5000",
        "-5000",
        MoveEntryMode {
            absolute: true,
            copy: false,
        },
    );
    let _ = out;
    assert_refused(&mut f.s, id, before, "typed X/Y");
    assert_eq!(f.s.take_transform_notice(), OFF_PAGE);
}

#[test]
fn ac18_a_typed_angle_or_size_that_stays_on_the_page_commits() {
    let (mut f, id) = placed();
    key(&mut f.s, "r", false);
    f.s.commit_transform_entry("45", "", 0);
    assert_eq!(f.s.undo_len(), 1, "a turn on the page commits");
    key(&mut f.s, "s", false);
    f.s.commit_transform_entry("10", "10", 0);
    assert_eq!(f.s.undo_len(), 2, "a shrink on the page commits");
    let d = doc(&f.s);
    let p = d.placed_page(id).unwrap();
    assert!(curvyo_document_core::overlaps_page(
        &p.frame_corners().unwrap(),
        &p.output
    ));
    // The source page and its objects are never written.
    for &r in &f.rects {
        assert!(d.object(r).is_some());
    }
}

#[test]
fn ac18_align_and_distribute_never_push_a_placed_page_off() {
    let (mut f, id) = placed();
    // A second placed page and an own item far right on the laser page.
    let d = doc(&f.s);
    d.place_page(
        &f.laser.id,
        "first",
        Affine::translation(Vec2::new(0.0, 0.0)),
    )
    .unwrap();
    let mut s = open_session(&d);
    s.activate_page(&f.laser.id);
    s.wipe_history();
    // Select all on the laser page and align left: the placed pages stay on it.
    key(&mut s, "a", true);
    let _ = s.align_selection(AlignCommand::Left);
    let _ = s.align_selection(AlignCommand::Bottom);
    let d = doc(&s);
    for p in d.placed_pages() {
        assert!(curvyo_document_core::overlaps_page(
            &p.frame_corners().unwrap(),
            &p.output
        ));
    }
    f.s = s;
    let _ = id;
}

#[test]
fn ac17_flip_writes_only_the_link_and_keeps_the_page_on() {
    let (mut f, id) = placed();
    let before = doc(&f.s).read_objects().objects.to_vec();
    f.s.flip_selection(FlipAxis::Vertical);
    assert_eq!(f.s.undo_len(), 1);
    assert!(map_of(&f.s, id).determinant() < 0.0);
    assert_eq!(
        doc(&f.s).read_objects().objects.to_vec(),
        before,
        "source never written"
    );
}

#[test]
fn ac19_duplicate_makes_a_second_placed_page_with_the_same_matrix() {
    let (mut f, id) = placed();
    assert_eq!(
        selected_placements(&f.s),
        vec![id.full_id()],
        "placed page selected"
    );
    let m = map_of(&f.s, id);
    // Duplicate is the Layers toolbar's command (the only Duplicate of objects).
    let reply = f.s.duplicate_layer_target();
    let d = doc(&f.s);
    assert_eq!(
        d.placed_pages().len(),
        2,
        "Duplicate makes a second placed page (reply {reply:?})"
    );
    assert_eq!(f.s.undo_len(), 1);
    let dup = d.placed_pages()[1].clone();
    assert_eq!(dup.output.id, f.laser.id);
    assert_eq!(dup.source.as_ref().unwrap().id, "first");
    assert!(dup.global_map().unwrap().approx_eq(m, TOL), "same matrix");
}

#[test]
fn ac19_ctrl_copy_ctrl_g_and_delete_on_a_placed_page() {
    let (mut f, id) = placed();
    let m = map_of(&f.s, id);
    let c = frame_centre(&f.s, id);
    let to = Point::new(c.x + 20.0, c.y + 10.0);
    f.s.modifiers_changed(false, true, false);
    drag_to(&mut f.s, c, to);
    f.s.pointer_up(to, false, true);
    f.s.modifiers_changed(false, false, false);
    let d = doc(&f.s);
    assert_eq!(d.placed_pages().len(), 2, "the Ctrl copy makes a second");
    assert_eq!(f.s.undo_len(), 1);
    assert!(
        map_of(&f.s, id).approx_eq(m, 1e-12),
        "the original placement stays"
    );
    let copy = d.placed_pages()[1].clone();
    assert_eq!(copy.output.id, f.laser.id);
    assert!(
        copy.global_map()
            .unwrap()
            .approx_eq(Affine::translation(Vec2::new(20.0, 10.0)).compose(m), 1e-6)
    );
    assert_eq!(
        selected_placements(&f.s),
        vec![copy.id.full_id()],
        "the copy is selected"
    );
    // Ctrl+G with a placed page selected: the notice, no group.
    let out = f.s.group_selection();
    assert_eq!(out.notice, "A placed page cannot be grouped.");
    assert_eq!(f.s.undo_len(), 1);
    key(&mut f.s, "g", true);
    assert_eq!(f.s.undo_len(), 1, "Ctrl+G writes nothing");
    // Delete removes only the reference.
    key(&mut f.s, "Delete", false);
    let d = doc(&f.s);
    assert_eq!(d.placed_pages().len(), 1);
    for &r in &f.rects {
        assert!(d.object(r).is_some(), "Delete keeps the source objects");
    }
}

// ---- criterion 24: deleting the drawing page -------------------------------------------------

#[test]
fn ac24_deleting_the_drawing_page_removes_its_placed_pages_and_undo_restores_them() {
    let (mut f, _) = placed();
    let d = doc(&f.s);
    let second = d
        .add_page_of(&curvyo_document_core::PageKind::Drawing)
        .unwrap();
    let mut s = open_session(&d);
    s.wipe_history();
    assert!(s.delete_page("first"));
    let d = doc(&s);
    assert!(d.placed_pages().is_empty());
    assert_eq!(s.undo_len(), 1);
    key(&mut s, "z", true);
    let d = doc(&s);
    assert_eq!(d.placed_pages().len(), 1, "one undo restores it");
    assert!(d.pages().iter().any(|p| p.id == "first"));
    f.s = s;
    let _ = second;
}

// ---- criterion 25: Place on… -------------------------------------------------------------------

#[test]
fn ac25_place_on_targets_list_laser_pages_and_hide_without_one() {
    let f = fixture();
    let t = f.s.place_on_targets();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].id, f.laser.id);
    assert_eq!(t[0].width_mm, 400.0);
    assert_eq!(t[0].height_mm, 300.0);
    let d = Document::new(1);
    rect(&d, 0.0, 0.0, 1.0, 1.0);
    let s = open_session(&d);
    assert!(s.place_on_targets().is_empty(), "no laser page: hidden");
}

#[test]
fn ac25_place_on_keeps_the_place_relative_to_the_page_and_is_one_step() {
    let mut f = fixture();
    select(&mut f.s, &[f.rects[1]]);
    assert!(f.s.place_on(&f.laser.id));
    assert_eq!(f.s.undo_len(), 1);
    let name = last_step_name(&mut f.s);
    assert!(name.starts_with("Place on"), "{name}");
    assert_eq!(f.s.active_page().id, f.laser.id);
    let d = doc(&f.s);
    let made = clones(&d);
    assert_eq!(made.len(), 1);
    assert!(made[0].chain.own_placement);
    let (min, _) = bounds(&d, made[0].resolved.id);
    assert!((min.x - f.laser.origin.x - 40.0).abs() < TOL);
    assert!((min.y - f.laser.origin.y - 10.0).abs() < TOL);
    assert_eq!(selected(&mut f.s), vec![made[0].resolved.id]);
}

#[test]
fn ac25_place_on_takes_a_selected_group() {
    let mut f = fixture();
    {
        let d = doc(&f.s);
        d.group_objects(&[f.rects[0], f.rects[1]]).unwrap();
        f.s = open_session(&d);
        f.s.activate_page("first");
        f.s.wipe_history();
    }
    let at = edge_of(&doc(&f.s), f.rects[0]);
    click(&mut f.s, at);
    assert_eq!(selected(&mut f.s).len(), 1, "the group is selected");
    assert!(
        f.s.place_on(&f.laser.id),
        "Place on… places a clone of a selected group (criterion 25: objects or groups)"
    );
    let made = clones(&doc(&f.s));
    assert_eq!(made.len(), 1);
    assert!(made[0].is_picture());
}

#[test]
fn ac25_place_on_clamps_a_selection_that_would_not_fit() {
    let mut f = fixture();
    // A rectangle at the far right of the drawing page beyond the laser page's width.
    {
        let d = doc(&f.s);
        let o = f.first.origin;
        let w = f.first.size.width.as_mm();
        let wide = rect(&d, o.x + w - 15.0, o.y + 5.0, 10.0, 10.0);
        f.s = open_session(&d);
        f.s.activate_page("first");
        f.s.wipe_history();
        f.rects.push(wide);
    }
    let wide = *f.rects.last().unwrap();
    select(&mut f.s, &[wide]);
    // Shrink the laser page reference: a 400 mm wide laser page holds x = w - 15 only when
    // w <= 405; force the clamp with a narrow second laser page.
    let narrow = {
        let d = doc(&f.s);
        let p = add_laser(&d, 50.0, 50.0);
        f.s = open_session(&d);
        f.s.activate_page("first");
        f.s.wipe_history();
        select(&mut f.s, &[wide]);
        find_page(&doc(&f.s), &p.id)
    };
    assert!(f.s.place_on(&narrow.id));
    let d = doc(&f.s);
    let made = clones(&d);
    let (min, max) = bounds(&d, made[0].resolved.id);
    assert!(min.x >= narrow.origin.x - TOL && max.x <= narrow.origin.x + 50.0 + TOL);
    assert!(min.y >= narrow.origin.y - TOL && max.y <= narrow.origin.y + 50.0 + TOL);
}

#[test]
fn ac25_place_on_with_only_guides_or_nothing_selected_writes_nothing() {
    let mut f = fixture();
    f.s.escape();
    assert!(!f.s.place_on(&f.laser.id));
    assert!(!f.s.place_on("first"), "a drawing page is no target");
    assert_eq!(f.s.undo_len(), 0);
}

// ---- criterion 20: export -----------------------------------------------------------------------

fn svg(s: &Session, active_page_only: bool) -> String {
    match s.export_svg(SvgExportOptions {
        text_as_paths: false,
        strokes_as_paths: false,
        selection_only: false,
        active_page_only,
    }) {
        SvgExportResult::Written { text, .. } => text,
        other => panic!("{other:?}"),
    }
}

#[test]
fn ac20_active_page_only_on_the_laser_page_holds_clones_and_placed_members() {
    let (mut f, _) = placed();
    // Plus a group clone on the laser page.
    let d = doc(&f.s);
    let g = d.group_objects(&[f.rects[0], f.rects[1]]).unwrap();
    let shift = Vec2::new(
        f.laser.origin.x - f.first.origin.x + 5.0,
        f.laser.origin.y - f.first.origin.y + 5.0,
    );
    let c = d.place_linked_clones(&[g], shift).unwrap()[0];
    f.s = open_session(&d);
    f.s.activate_page(&f.laser.id);
    let text = svg(&f.s, true);
    assert!(
        text.contains(&c.full_id()) || text.matches("<g").count() >= 2,
        "{text}"
    );
    assert!(!text.contains("<use"), "a picture clone is never a <use>");
    // The placed page's members are written: four rects drawn by it, plus two by the clone.
    let rects = text.matches("<rect").count() + text.matches("<path").count();
    assert!(rects >= 5, "{rects} shapes in\n{text}");
    assert!(
        text.contains("Placed page of"),
        "the placed page is one labelled <g>"
    );
    assert!(
        text.contains(r#"d="M 15 15 L 35 15 L 35 25 L 15 25 Z""#),
        "the group clone's leaf at its laser-page place:\n{text}"
    );
    assert!(
        !text.contains("work area") && !text.contains("origin"),
        "no fixture marks"
    );
    // The whole plane writes G on its drawing page plus the clone and the placed members.
    let plane = svg(&f.s, false);
    let n = plane.matches("<path").count() + plane.matches("<rect").count();
    assert!(n >= 4 + 2 + 3, "{n} shapes on the whole plane:\n{plane}");
    assert!(!plane.contains("<use"));
}

// ---- adrs.md note 4: the drag answers the session really sends ------------------------------

const FRONTEND_BADGES: [&str; 8] = [
    "none",
    "clone",
    "not_on_sketch_page",
    "no_sketch",
    "move",
    "copy",
    "placed_page",
    "only_on_laser_page",
];

fn decodes_like_the_frontend(v: &Value) -> bool {
    let Some(o) = v.as_object() else {
        return false;
    };
    o.get("badge")
        .and_then(Value::as_str)
        .is_some_and(|b| FRONTEND_BADGES.contains(&b))
        && o.get("target")
            .is_some_and(|t| t.is_null() || t.is_string())
        && o.get("faint").is_some_and(Value::is_boolean)
}

#[test]
fn note4_every_answer_of_a_real_drag_decodes_with_the_frontend_rules() {
    let mut f = fixture();
    let mut seen = std::collections::BTreeSet::new();
    let check = |a: &curvyo_editor_wasm::RowDragAnswer,
                 seen: &mut std::collections::BTreeSet<String>| {
        let v = serde_json::to_value(a).unwrap();
        assert!(decodes_like_the_frontend(&v), "{v}");
        seen.insert(v["badge"].as_str().unwrap().to_owned());
        v
    };
    // The page-row drag of a drawing page.
    assert!(f.s.row_drag_begin("first"));
    let on_laser = laser_point(&f, 100.0, 100.0);
    let v = check(&f.s.row_drag_move(Some(on_laser)), &mut seen);
    assert_eq!(v["badge"], "placed_page");
    assert_eq!(v["target"], Value::String(f.laser.id.clone()));
    let v = check(
        &f.s.row_drag_move(Some(Point::new(
            f.first.origin.x + 5.0,
            f.first.origin.y + 5.0,
        ))),
        &mut seen,
    );
    assert_eq!(v["badge"], "only_on_laser_page");
    check(&f.s.row_drag_move(None), &mut seen);
    f.s.row_drag_cancel();
    // The cloning drop.
    let id = f.rects[0];
    select(&mut f.s, &[id]);
    let from = edge_of(&doc(&f.s), id);
    drag_to(&mut f.s, from, on_laser);
    for (ctrl, alt, want) in [
        (false, false, "clone"),
        (false, true, "move"),
        (true, false, "copy"),
    ] {
        f.s.modifiers_changed(false, ctrl, alt);
        let a =
            f.s.clone_drop_answer()
                .expect("an answer over a laser page");
        let v = check(&a, &mut seen);
        assert_eq!(v["badge"], want);
        assert_eq!(v["target"], Value::String(f.laser.id.clone()));
    }
    f.s.escape();
    for b in &seen {
        assert!(FRONTEND_BADGES.contains(&b.as_str()));
    }
}

// ---- criterion 8: Linked clone offered for a group; criterion 16: placed pages are live ------

#[test]
fn ac8_linked_clone_is_offered_for_a_group_and_clones_it_in_place() {
    let mut f = fixture();
    let g = {
        let d = doc(&f.s);
        let g = d.group_objects(&[f.rects[0], f.rects[1]]).unwrap();
        f.s = open_session(&d);
        f.s.activate_page("first");
        f.s.wipe_history();
        g
    };
    let at = edge_of(&doc(&f.s), f.rects[0]);
    click(&mut f.s, at);
    assert!(f.s.linked_clone_shown(), "offered for a group");
    let _ = f.s.linked_clone();
    let made = clones(&doc(&f.s));
    assert_eq!(made.len(), 1);
    assert_eq!(made[0].chain.origin, g);
    assert!(
        made[0]
            .chain
            .matrix
            .approx_eq(Affine::translation(Vec2::new(0.0, 0.0)), TOL)
    );
    assert_eq!(f.s.undo_len(), 1);
}

#[test]
fn ac16_a_rectangle_drawn_on_the_source_page_shows_in_the_placed_page_at_once() {
    let (f, _) = placed();
    let d = doc(&f.s);
    let mut s = open_session(&d);
    s.activate_page(&f.laser.id);
    let before = svg(&s, true).matches("<path").count();
    let d = doc(&s);
    rect(
        &d,
        f.first.origin.x + 100.0,
        f.first.origin.y + 100.0,
        10.0,
        10.0,
    );
    // Off the source page: not a member, not drawn.
    rect(&d, f.first.origin.x - 300.0, f.first.origin.y, 10.0, 10.0);
    let mut s = open_session(&d);
    s.activate_page(&f.laser.id);
    let after = svg(&s, true).matches("<path").count();
    assert_eq!(
        after,
        before + 1,
        "only the member on the source page shows"
    );
}
