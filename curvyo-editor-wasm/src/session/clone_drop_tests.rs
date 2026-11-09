//! The cloning drop of a Select move onto a laser page (`specs/0100-bring-content-in` criteria
//! 1 to 6).
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{FIRST_PAGE_ID, NodeId, ObjectSnapshot, Point};

use crate::session::align_test_support::draw_rect;
use crate::session::{DropBadge, Session, Tool};

/// A session with two rectangles on the drawing page, both selected, and a laser page; the
/// laser page's centre.
fn setup(peer: u64) -> (Session, Vec<NodeId>, String, Point) {
    let mut s = Session::new(peer);
    let a = draw_rect(&mut s, (10.0, 10.0), (30.0, 30.0));
    let b = draw_rect(&mut s, (40.0, 10.0), (60.0, 30.0));
    let laser = s.add_page_of_kind("curvyo.laser", None).unwrap();
    s.activate_page(FIRST_PAGE_ID);
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
    s.set_tool(Tool::Select);
    s.selection.set(&[a, b]);
    (s, vec![a, b], laser, centre)
}

/// Presses on the first rectangle and moves to `to` with `alt` and `ctrl` held.
fn drag_to(s: &mut Session, to: Point, alt: bool, ctrl: bool) -> Option<DropBadge> {
    // On the first rectangle's right edge, inside the selection's box: a move.
    let from = Point::new(30.0, 20.0);
    s.pointer_down(from, false);
    s.modifiers_changed(false, ctrl, alt);
    s.pointer_hover(to, false, ctrl);
    s.clone_drop_answer().map(|a| a.badge)
}

#[test]
fn criterion_1_a_drop_on_a_laser_page_places_linked_clones_and_writes_no_original() {
    let (mut s, originals, laser, centre) = setup(100_001);
    let before: Vec<ObjectSnapshot> = originals
        .iter()
        .map(|&id| s.document.object(id).unwrap())
        .collect();
    let steps = s.undo_len();
    let version = s.document.version();
    assert_eq!(
        drag_to(&mut s, centre, false, false),
        Some(DropBadge::Clone)
    );
    assert_eq!(
        s.document.version(),
        version,
        "nothing is written before the release"
    );
    s.pointer_up(centre, false, false);
    assert_eq!(s.undo_len(), steps + 1, "one step");
    let after: Vec<ObjectSnapshot> = originals
        .iter()
        .map(|&id| s.document.object(id).unwrap())
        .collect();
    assert_eq!(before, after, "the originals are unchanged");
    let clones = s.selection.ids().to_vec();
    assert_eq!(clones.len(), 2);
    for (clone, original) in clones.iter().zip(&originals) {
        let Some(ObjectSnapshot::Clone(c)) = s.document.object(*clone) else {
            panic!("a clone")
        };
        assert_eq!(c.chain.origin, *original);
        assert!(c.chain.own_placement);
    }
    assert_eq!(s.active_page().id, laser);
    // Undo removes the clones and keeps the originals.
    s.undo();
    assert_eq!(s.objects().len(), 2);
}

#[test]
fn criterion_2_and_3_alt_moves_the_objects_themselves() {
    let (mut s, originals, laser, centre) = setup(100_002);
    assert_eq!(drag_to(&mut s, centre, true, false), Some(DropBadge::Move));
    s.pointer_up(centre, false, false);
    assert_eq!(s.objects().len(), 2, "no clone, no copy");
    assert!(matches!(
        s.document.object(originals[0]),
        Some(ObjectSnapshot::Primitive(_))
    ));
    assert_eq!(s.active_page().id, laser);
}

#[test]
fn criterion_4_ctrl_and_ctrl_alt_copy() {
    for alt in [false, true] {
        let (mut s, _, _, centre) = setup(100_003);
        assert_eq!(drag_to(&mut s, centre, alt, true), Some(DropBadge::Copy));
        s.pointer_up(centre, false, true);
        let objects = s.objects();
        assert_eq!(objects.len(), 4, "two copies");
        assert!(
            !objects
                .iter()
                .any(|o| matches!(o, ObjectSnapshot::Clone(_)))
        );
    }
}

#[test]
fn criterion_5_a_drop_elsewhere_is_a_plain_move_with_no_badge_and_escape_writes_nothing() {
    let (mut s, _, _, _) = setup(100_004);
    let to = Point::new(25.0, 60.0);
    assert_eq!(drag_to(&mut s, to, false, false), Some(DropBadge::None));
    s.pointer_up(to, false, false);
    assert_eq!(s.objects().len(), 2);
    let (mut s, _, _, centre) = setup(100_005);
    let steps = s.undo_len();
    drag_to(&mut s, centre, false, false);
    s.escape();
    s.pointer_up(centre, false, false);
    assert_eq!(s.undo_len(), steps);
}
