//! Session tests of `specs/0029-laser-job-preview-and-output` Part C, one file per criterion group;
//! this module holds their shared laser page builders.

mod export_tests;
mod group_tests;
mod play_tests;
mod send_tests;
mod toolpath_tests;

use curvyo_document_core::{Direction, NodeId};

use super::super::Session;
use crate::session::align_test_support::draw_rect;

/// A laser page with one 20 mm square on it; the session and the square.
pub(super) fn laser_with_square() -> (Session, NodeId) {
    let mut s = Session::new(290_001);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let origin = s.active_manufacturing().unwrap().page.unwrap().origin;
    let at = |x: f64, y: f64| (origin.x + x, origin.y + y);
    let square = draw_rect(&mut s, at(10.0, 10.0), at(30.0, 30.0));
    s.selection.set(&[]);
    (s, square)
}

/// A Cut operation holding `items`, with own numbers 60 %, 600 mm/min.
pub(super) fn cut(s: &mut Session, items: &[NodeId]) -> NodeId {
    s.selection.set(items);
    let id = s.press_operation_kind("cut", true).unwrap();
    s.selection.set(&[]);
    s.edit_operation_setting(id, "main", "power", 60.0).unwrap();
    s.edit_operation_setting(id, "main", "speed", 600.0)
        .unwrap();
    id
}

pub(super) fn view(s: &Session) -> serde_json::Value {
    serde_json::from_str(&s.output_group_json()).unwrap()
}

/// The Operations tab's view as JSON.
pub(super) fn rows(s: &Session) -> serde_json::Value {
    serde_json::from_str(&s.operations_view_json()).unwrap()
}

pub(super) fn undo_label(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}
