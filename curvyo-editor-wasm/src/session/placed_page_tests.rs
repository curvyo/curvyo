//! A placed page in the one selection (`specs/0100-bring-content-in` criteria 17 to 19): the
//! typed values, Flip, align, the arrow keys, the off-page refusal, the drag look and the frame
//! mark, and the box of a Linked clone drag staying with the originals.
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{Affine, FIRST_PAGE_ID, NodeId, Point};
use curvyo_ui_core::{AlignCommand, FlipAxis};

use crate::session::keys::KeyInput;
use crate::session::{Session, Tool};

/// A laser page with two placed pages of the drawing page, the first selected.
fn setup(peer: u64) -> (Session, NodeId, NodeId) {
    let mut s = Session::new(peer);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(s.place_page_on_active(FIRST_PAGE_ID));
    let second = s.selected_placement().unwrap();
    s.translate_second(second);
    assert!(s.place_page_on_active(FIRST_PAGE_ID));
    let first = s.selected_placement().unwrap();
    s.set_tool(Tool::Select);
    s.selection.set(&[first]);
    (s, first, second)
}

impl Session {
    fn translate_second(&mut self, id: NodeId) {
        self.document
            .translate_objects(&[id], curvyo_document_core::Vec2::new(30.0, 20.0))
            .unwrap();
    }
}

fn map(s: &Session, id: NodeId) -> Affine {
    s.document.placed_page(id).unwrap().global_map().unwrap()
}

fn key(s: &mut Session, k: &str) {
    s.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    });
}

#[test]
fn criterion_17_a_typed_angle_turns_the_placed_page_in_one_step() {
    let (mut s, id, _) = setup(100_171);
    let steps = s.undo_len();
    key(&mut s, "r");
    s.commit_transform_entry("90", "", 0);
    assert_eq!(s.undo_len(), steps + 1);
    let m = map(&s, id);
    assert!(m.a.abs() < 1e-9 && (m.b.abs() - 1.0).abs() < 1e-9, "{m:?}");
}

#[test]
fn criterion_17_flip_align_and_the_arrow_keys_write_the_placed_page() {
    let (mut s, first, second) = setup(100_172);
    s.flip_selection(FlipAxis::Horizontal);
    assert!(map(&s, first).determinant() < 0.0, "mirrored");
    let before = map(&s, first);
    key(&mut s, "ArrowRight");
    assert!((map(&s, first).e - before.e - 1.0).abs() < 1e-9);
    s.selection.set(&[first, second]);
    s.align_selection(AlignCommand::Left);
    let left = |id| {
        s.placed_frame(id)
            .unwrap()
            .iter()
            .map(|p| p.x)
            .fold(f64::INFINITY, f64::min)
    };
    assert!((left(first) - left(second)).abs() < 1e-6);
}

#[test]
fn criterion_18_a_typed_move_off_the_page_is_refused_with_the_notice() {
    let (mut s, id, _) = setup(100_173);
    let before = map(&s, id);
    let steps = s.undo_len();
    key(&mut s, "m");
    s.commit_move_entry(
        "9000",
        "0",
        curvyo_ui_core::MoveEntryMode {
            absolute: false,
            copy: false,
        },
    );
    assert_eq!(s.undo_len(), steps);
    assert_eq!(map(&s, id), before);
    assert_eq!(
        s.take_transform_notice(),
        "A placed page stays on its laser page."
    );
}

#[test]
fn a_drag_off_the_laser_page_draws_it_faint_and_every_placed_page_has_its_frame_mark() {
    let (mut s, id, _) = setup(100_174);
    assert_ne!(s.placed_frame_marks().triangles.len(), 0, "a frame mark");
    let c = s.placed_frame(id).unwrap();
    let inside = Point::new(f64::midpoint(c[0].x, c[2].x), f64::midpoint(c[0].y, c[2].y));
    s.pointer_hover(inside, false, false);
    s.pointer_down(inside, false);
    s.pointer_hover(Point::new(inside.x + 5000.0, inside.y), false, false);
    let live = s.placed_live();
    assert!(
        live.iter().any(|(lid, (_, _, off))| *lid == id && *off),
        "{live:?}"
    );
    s.escape();
}

#[test]
fn the_box_of_a_linked_clone_drag_stays_with_the_originals() {
    let mut s = Session::new(100_175);
    let a = crate::session::align_test_support::draw_rect(&mut s, (10.0, 10.0), (30.0, 30.0));
    let laser = s.add_page_of_kind("curvyo.laser", None).unwrap();
    s.activate_page(FIRST_PAGE_ID);
    s.set_tool(Tool::Select);
    s.selection.set(&[a]);
    let page = s
        .document
        .pages()
        .into_iter()
        .find(|p| p.id == laser)
        .unwrap();
    let to = Point::new(page.origin.x + 100.0, page.origin.y + 100.0);
    // On the top edge away from the handles: a move.
    let from = Point::new(15.0, 10.0);
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    let objects = s.objects();
    let live = s.select_live_edit_in(&objects).unwrap();
    assert!(live.copy, "the originals and their box stay");
    s.escape();
}

#[test]
fn criterion_17_a_typed_size_and_a_typed_skew_write_the_placed_page() {
    let (mut s, id, _) = setup(100_176);
    key(&mut s, "s");
    s.commit_transform_entry("105", "148.5", 0);
    let c = s.placed_frame(id).unwrap();
    assert!((c[1].x - c[0].x - 105.0).abs() < 1e-6, "{c:?}");
    let before = map(&s, id);
    key(&mut s, "k");
    s.commit_transform_entry("10", "", 0);
    assert_ne!(map(&s, id), before, "skewed");
}

#[test]
fn ux_a_selected_placed_page_keeps_the_style_tab_reachable_and_hides_stroke_to_path() {
    let (s, _, _) = setup(100_181);
    let view = s.panel_view();
    let style = view.tabs.iter().find(|t| t.name == "style").unwrap();
    assert!(style.enabled, "the Style tab is not dimmed");
    assert!(!s.stroke_to_path_shown(), "Stroke to path is hidden");
}

#[test]
fn ux_the_cursor_is_not_allowed_while_a_dragged_placed_page_is_off_its_laser_page() {
    let (mut s, id, _) = setup(100_182);
    let c = s.placed_frame(id).unwrap();
    let inside = Point::new(f64::midpoint(c[0].x, c[2].x), f64::midpoint(c[0].y, c[2].y));
    s.pointer_hover(inside, false, false);
    s.pointer_down(inside, false);
    s.pointer_hover(Point::new(inside.x + 5000.0, inside.y), false, false);
    assert_eq!(s.cursor_hint(), "not-allowed");
    s.pointer_hover(Point::new(inside.x + 1.0, inside.y), false, false);
    assert_ne!(s.cursor_hint(), "not-allowed");
    s.escape();
}

/// Two rectangles on the drawing page, both selected, and a laser page: (session, its centre).
fn drawing_scene(peer: u64) -> (Session, String, Point) {
    let mut s = Session::new(peer);
    let a = crate::session::align_test_support::draw_rect(&mut s, (10.0, 10.0), (30.0, 30.0));
    let laser = s.add_page_of_kind("curvyo.laser", None).unwrap();
    s.activate_page(FIRST_PAGE_ID);
    s.set_tool(Tool::Select);
    s.selection.set(&[a]);
    let page = s
        .document
        .pages()
        .into_iter()
        .find(|p| p.id == laser)
        .unwrap();
    let centre = Point::new(
        page.origin.x + page.size.width.as_mm() / 2.0,
        page.origin.y + page.size.height.as_mm() / 2.0,
    );
    (s, laser, centre)
}

#[test]
fn ux_a_drag_cancelled_by_escape_changes_nothing_on_release_not_even_the_active_page() {
    let (mut s, _, centre) = drawing_scene(100_183);
    let steps = s.undo_len();
    let from = Point::new(15.0, 10.0);
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(centre, false, false);
    s.escape();
    s.pointer_up(centre, false, false);
    assert_eq!(s.undo_len(), steps);
    assert_eq!(s.active_page().id, FIRST_PAGE_ID, "the active page stays");
}

#[test]
fn ux_a_ctrl_drag_over_a_laser_page_says_copy_once_in_the_badge_not_in_the_readout() {
    let (mut s, _, centre) = drawing_scene(100_184);
    let from = Point::new(15.0, 10.0);
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.modifiers_changed(false, true, false);
    s.pointer_hover(centre, false, true);
    assert_eq!(
        s.clone_drop_answer().map(|a| a.badge),
        Some(crate::session::DropBadge::Copy)
    );
    let readout = s.live_readout().unwrap();
    assert!(!readout.text.contains("Copy"), "{}", readout.text);
    s.escape();
}
