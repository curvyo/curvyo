//! The shared rig of the session-level tests of
//! `specs/0061-align-and-distribute/`: rectangles drawn by a drag, selected
//! by clicks, and the reads the tests compare.

use curvyo_document_core::{NodeId, ObjectSnapshot, Point};
use curvyo_ui_core::{AlignBox, object_outline_bounds};

use super::{Session, Tool};

/// The geometric tolerance of the criteria.
pub(super) const EPS: f64 = 1e-6;

/// The left edge of the `at`-th object of the context.
pub(super) fn left_x(session: &Session, at: usize) -> f64 {
    let object = session.objects()[at].clone();
    object_outline_bounds(&object).0.x
}

/// The true box of the object `id`.
pub(super) fn box_of(session: &Session, id: NodeId) -> AlignBox {
    let objects = session.objects();
    let object = objects
        .iter()
        .find(|object| object.id() == id)
        .expect("the object is in the context");
    let (min, max) = object_outline_bounds(object);
    AlignBox::new(min, max)
}

/// One rectangle by its drag with the Rectangle tool; its id.
pub(super) fn draw_rect(session: &mut Session, from: (f64, f64), to: (f64, f64)) -> NodeId {
    session.set_tool(Tool::Rectangle);
    session.pointer_down(Point::new(from.0, from.1), false);
    session.pointer_up(Point::new(to.0, to.1), false, false);
    session
        .objects()
        .last()
        .expect("a rectangle was drawn")
        .id()
}

/// A click with the Select tool, Shift adding to the selection.
pub(super) fn click(session: &mut Session, at: (f64, f64), shift: bool) {
    session.set_tool(Tool::Select);
    let point = Point::new(at.0, at.1);
    session.pointer_hover(point, shift, false);
    session.pointer_down(point, shift);
    session.pointer_up(point, shift, false);
}

/// Nothing selected, the Select tool active.
pub(super) fn select_nothing(session: &mut Session) {
    session.set_tool(Tool::Select);
    session.escape();
    session.escape();
    assert_eq!(
        session.selection.ids(),
        &[] as &[NodeId],
        "nothing is selected"
    );
}

/// Select every object of the context.
pub(super) fn select_all(session: &mut Session) {
    session.set_tool(Tool::Select);
    let ids: Vec<_> = session.objects().iter().map(ObjectSnapshot::id).collect();
    session.selection.clear();
    for id in ids {
        session.selection.add(id);
    }
}

/// The label of the newest own history step.
pub(super) fn last_label(session: &Session) -> String {
    session
        .document
        .own_steps()
        .last()
        .map(|step| step.label.clone())
        .unwrap_or_default()
}

/// The number of own history steps.
pub(super) fn steps(session: &Session) -> usize {
    session.document.own_steps().len()
}

/// A second page, moved to (250, 30) off the document origin and active;
/// its id and box. (A press on the canvas activates the page under it, a
/// row of the Pages list activates its page.)
pub(super) fn second_page(session: &mut Session) -> (String, AlignBox) {
    let id = session.add_page();
    assert!(session.move_page_to(&id, Point::new(250.0, 30.0)));
    assert_eq!(session.active_page().id, id, "the new page is active");
    (id, session.active_page_box())
}

pub(super) fn near(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < EPS,
        "{what}: {actual} != {expected}"
    );
}

/// Two rectangles, both selected, Select tool active.
pub(super) fn two_rects(session: &mut Session) {
    draw_rect(session, (0.0, 0.0), (30.0, 10.0));
    draw_rect(session, (50.0, 0.0), (70.0, 10.0));
    select_all(session);
}
