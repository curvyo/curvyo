//! Text on path in the session (`specs/0080-text-on-path` slice 1): pick mode, the Select-tool
//! route, the position handle with its side switch, the options and their steps, Release, edit
//! mode along the carrier, a carrier drag followed in the same frame, Flip and the nudge.

use curvyo_document_core::{
    Angle, Curve, EllipseFrame, Length, LineOnPath, NodeId, ObjectSnapshot, PathSide, Point, Shape,
    TextSnapshot,
};
use curvyo_ui_core::{FlipAxis, TextInput};

use super::keys::{EscapeStep, KeyInput};
use super::text_on_path_pick::TextPathPress;
use super::text_tests::{labels, load_wanted, session};
use super::{Session, Tool};

fn circle(session: &Session) -> NodeId {
    session.document.create_primitive(
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(100.0, 100.0),
                rx: Length::from_mm(50.0),
                ry: Length::from_mm(50.0),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        },
        Angle::from_radians(0.0),
    )
}

/// A session with a text "Badge" at (20, 20) selected and a circle.
fn badge() -> (Session, NodeId, NodeId) {
    let mut session = session();
    session.set_tool(Tool::Text);
    session.pointer_down(Point::new(20.0, 20.0), false);
    session.pointer_up(Point::new(20.0, 20.0), false, false);
    for c in "Badge".chars() {
        session.text_input(TextInput::Insert(c.to_string()));
    }
    session.finish_text_edit();
    load_wanted(&mut session);
    let text = session.selection.ids()[0];
    let c = circle(&session);
    session.set_tool(Tool::Select);
    session.selection.select_single(text);
    (session, text, c)
}

fn text(session: &Session, id: NodeId) -> TextSnapshot {
    session
        .objects()
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Text(t) if t.id == id => Some((**t).clone()),
            _ => None,
        })
        .unwrap()
}

fn is_guide(session: &Session, id: NodeId) -> bool {
    session.document_read().guides.contains(&id)
}

/// Criteria 2, 4 to 6: pick mode, a click on the circle puts the text on it in one step, the
/// circle is a guide, the text is selected; the middle of the text at 12 o'clock.
#[test]
fn c02_pick_mode_puts_the_text_on_the_clicked_carrier() {
    let (mut session, t, c) = badge();
    assert_eq!(session.text_path_press(), TextPathPress::Picking);
    assert!(session.text_path_picking());
    // Hover over nothing: no carrier; a click there changes nothing and stays in pick mode.
    session.pointer_hover(Point::new(100.0, 100.0), false, false);
    let steps = labels(&session).len();
    session.pointer_down(Point::new(300.0, 300.0), false);
    session.pointer_up(Point::new(300.0, 300.0), false, false);
    assert!(session.text_path_picking());
    assert_eq!(labels(&session).len(), steps);
    session.pointer_hover(Point::new(150.0, 100.0), false, false);
    assert_ne!(session.draw_list().triangles.len(), 0);
    session.pointer_down(Point::new(150.0, 100.0), false);
    session.pointer_up(Point::new(150.0, 100.0), false, false);
    assert!(!session.text_path_picking());
    assert_eq!(labels(&session).last().unwrap(), "put_text_on_path");
    assert_eq!(labels(&session).len(), steps + 1);
    assert!(is_guide(&session, c));
    assert_eq!(session.selection.ids(), &[t]);
    let on = text(&session, t);
    let outline = on.outline.unwrap();
    let (min, max) = (outline.line_box[0], outline.line_box[2]);
    assert!(
        (f64::midpoint(min.x, max.x) - 100.0).abs() < 0.5,
        "{min:?} {max:?}"
    );
    assert!(min.y < 45.0 && max.y < 65.0, "at the top: {min:?} {max:?}");
    assert_eq!(session.take_text_path_announcement(), Some("Put on path"));
}

/// Criterion 2: Escape ends pick mode and changes nothing; a second press too.
#[test]
fn c02_escape_and_a_second_press_end_pick_mode() {
    let (mut session, _, _) = badge();
    session.text_path_press();
    assert_eq!(session.escape(), EscapeStep::ClearedState);
    assert!(!session.text_path_picking());
    assert_eq!(session.selection.ids().len(), 1, "the selection stays");
    session.text_path_press();
    assert_eq!(session.text_path_press(), TextPathPress::Ended);
    assert!(!session.text_path_picking());
}

/// Criterion 3: the Select tool with the text and a carrier selected puts at once.
#[test]
fn c03_text_and_carrier_selected_put_at_once() {
    let (mut session, t, c) = badge();
    session.selection.set(&[t, c]);
    assert_eq!(session.text_path_press(), TextPathPress::Put);
    assert!(text(&session, t).on_path.is_some());
    assert!(is_guide(&session, c));
}

/// Criteria 15, 16: the handle drags the text along; past the circle it switches to the other
/// side; Escape cancels; one step on release.
#[test]
fn c15_c16_the_position_handle() {
    let (mut session, t, c) = badge();
    session.selection.set(&[t, c]);
    session.text_path_press();
    session.selection.select_single(t);
    let before = text(&session, t).on_path.unwrap().values;
    let handle = session
        .text_path_handles()
        .unwrap()
        .handles
        .first()
        .unwrap()
        .position;
    assert!((handle.vector_to(Point::new(100.0, 100.0)).length() - 50.0).abs() < 0.02);
    session.pointer_down(handle, false);
    session.pointer_hover(Point::new(160.0, 100.0), false, false);
    assert!(session.live_readout().unwrap().text.ends_with("°"));
    session.pointer_up(Point::new(160.0, 100.0), false, false);
    assert_eq!(labels(&session).last().unwrap(), "move_text_on_path");
    let moved = text(&session, t).on_path.unwrap().values;
    assert_ne!(moved.position, before.position);
    assert_eq!(moved.side, PathSide::Above);
    // Into the circle by far more than 8 px: across.
    let handle = session.text_path_handles().unwrap().handles[0].position;
    session.pointer_down(handle, false);
    session.pointer_hover(Point::new(120.0, 100.0), false, false);
    session.pointer_up(Point::new(120.0, 100.0), false, false);
    assert_eq!(labels(&session).last().unwrap(), "mirror_text_on_path");
    let crossed = text(&session, t).on_path.unwrap().values;
    assert_eq!(
        (crossed.side, crossed.line),
        (PathSide::Below, LineOnPath::TopEdge)
    );
    // Escape cancels a drag.
    let steps = labels(&session).len();
    let handle = session.text_path_handles().unwrap().handles[0].position;
    session.pointer_down(handle, false);
    session.pointer_hover(Point::new(100.0, 160.0), false, false);
    assert_eq!(session.escape(), EscapeStep::CancelledDrag);
    session.pointer_up(Point::new(100.0, 160.0), false, false);
    assert_eq!(labels(&session).len(), steps);
}

/// The position handle of the one selected text on path.
fn handle(session: &Session) -> Point {
    session.text_path_handles().unwrap().handles[0].position
}

/// Criteria 17, 18 (customer 2026-10-24): a double-click on the position handle opens the entry
/// box with Position, Distance, Side, Line and Mirror; its buttons preview, Escape writes
/// nothing, Enter writes one step.
#[test]
fn c17_the_handle_entry_box() {
    let (mut session, t, c) = badge();
    session.selection.set(&[t, c]);
    session.text_path_press();
    session.selection.select_single(t);
    let at = handle(&session);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
    session.double_click(at, false, false);
    let entry = session.text_path_entry().expect("the entry box opens");
    assert!(entry.degrees, "an ellipse carrier reads degrees");
    assert!((entry.position - 90.0).abs() < 0.1 || (entry.position - 270.0).abs() < 0.1);
    assert_eq!(
        (entry.side, entry.line),
        (PathSide::Above, LineOnPath::Baseline)
    );
    assert!(entry.handle.vector_to(at).length() < 1e-9);
    // The buttons preview; Escape closes without a change.
    let steps = labels(&session).len();
    session.text_path_entry_side(PathSide::Below);
    assert!(session.text_path_live().is_some(), "previewed");
    session.cancel_text_path_entry();
    assert!(session.text_path_entry().is_none());
    assert!(session.text_path_live().is_none());
    assert_eq!(labels(&session).len(), steps);
    // One field: its own step.
    session.double_click(at, false, false);
    let entry = session.text_path_entry().unwrap();
    let shown = format!("{:.1}", entry.position);
    assert_eq!(session.commit_text_path_entry(&shown, "2.5"), "committed");
    assert_eq!(labels(&session).last().unwrap(), "change_distance_on_path");
    assert!(session.text_path_entry().is_none(), "Enter closes it");
    // Mirror alone: "Mirror text on path"; several values: "Change text on path".
    session.double_click(at, false, false);
    session.text_path_entry_mirror();
    assert_eq!(session.commit_text_path_entry(&shown, "2.5"), "committed");
    assert_eq!(labels(&session).last().unwrap(), "mirror_text_on_path");
    let values = text(&session, t).on_path.unwrap().values;
    assert_eq!(
        (values.side, values.line),
        (PathSide::Below, LineOnPath::TopEdge)
    );
    session.double_click(at, false, false);
    session.text_path_entry_line(LineOnPath::Baseline);
    assert_eq!(session.commit_text_path_entry("0", "x"), "invalid:1");
    assert!(
        session.text_path_entry().is_some(),
        "an invalid field keeps it open"
    );
    assert_eq!(session.commit_text_path_entry("0", "0"), "committed");
    assert_eq!(labels(&session).last().unwrap(), "change_text_on_path");
    let values = text(&session, t).on_path.unwrap().values;
    assert_eq!(
        (values.side, values.line),
        (PathSide::Below, LineOnPath::Baseline)
    );
    assert!(values.distance.as_mm().abs() < 1e-9);
    // Nothing changed: no step. Position 0° is at 3 o'clock now.
    let at = handle(&session);
    assert!(
        (at.x - 150.0).abs() < 0.01 && (at.y - 100.0).abs() < 0.01,
        "{at:?}"
    );
    session.double_click(at, false, false);
    let steps = labels(&session).len();
    let entry = session.text_path_entry().unwrap();
    let shown = format!("{:.1}", entry.position);
    assert_eq!(session.commit_text_path_entry(&shown, "0"), "unchanged");
    assert_eq!(labels(&session).len(), steps);
}

/// Criteria 19 and 23: Flip toggles Mirror letters, Release straightens and keeps the carrier a
/// guide.
#[test]
fn c19_c23_flip_and_release() {
    let (mut session, t, c) = badge();
    session.selection.set(&[t, c]);
    session.text_path_press();
    session.selection.select_single(t);
    let view = session.text_path_view();
    assert_eq!((view.texts, view.on_path), (1, 1));
    session.flip_selection(FlipAxis::Horizontal);
    assert_eq!(labels(&session).last().unwrap(), "flip_letters");
    assert!(text(&session, t).on_path.unwrap().values.mirror_letters);
    session.release_text_path();
    assert_eq!(labels(&session).last().unwrap(), "release_from_path");
    assert!(text(&session, t).on_path.is_none());
    assert!(is_guide(&session, c));
}

/// Criterion 21: the arrows move along the reading direction and change Distance.
#[test]
fn c21_the_nudge() {
    let (mut session, t, c) = badge();
    session.selection.set(&[t, c]);
    session.text_path_press();
    session.selection.select_single(t);
    let before = text(&session, t).on_path.unwrap().values;
    session.key_down(KeyInput {
        key: "ArrowRight",
        ..KeyInput::default()
    });
    let after = text(&session, t).on_path.unwrap().values;
    assert!((after.position.as_mm() - before.position.as_mm() - 1.0).abs() < 1e-9);
    session.key_down(KeyInput {
        key: "ArrowUp",
        ..KeyInput::default()
    });
    assert!((text(&session, t).on_path.unwrap().values.distance.as_mm() - 1.0).abs() < 1e-9);
}

/// Criterion 24: a carrier move drag previews the text on the moved carrier in the same frame;
/// the commit moves the text without writing it.
#[test]
fn c24_the_text_follows_a_carrier_drag() {
    let (mut session, t, c) = badge();
    session.selection.set(&[t, c]);
    session.text_path_press();
    session.selection.select_single(c);
    let before = text(&session, t).outline.unwrap().line_box[0];
    let plain = session.draw_list().triangles.len();
    // On the outline at 135°, away from the box handles: a move drag.
    let on_outline = Point::new(100.0 - 35.355_339, 100.0 + 35.355_339);
    let moved = Point::new(on_outline.x + 20.0, on_outline.y);
    session.pointer_down(on_outline, false);
    session.pointer_hover(moved, false, false);
    let during = session.draw_list().triangles.len();
    assert!(during > plain, "the following text is drawn as a preview");
    session.pointer_up(moved, false, false);
    assert_eq!(labels(&session).last().unwrap(), "translate_objects");
    let after = text(&session, t).outline.unwrap().line_box[0];
    assert!(
        (after.x - before.x - 20.0).abs() < 0.01,
        "{before:?} {after:?}"
    );
}

/// Criterion 27: a click on a letter of a text on path places the caret there; typing lays out
/// along the carrier.
#[test]
fn c27_edit_mode_along_the_carrier() {
    let (mut session, t, c) = badge();
    session.selection.set(&[t, c]);
    session.text_path_press();
    let box_ = text(&session, t).outline.unwrap().letter_boxes[0];
    let first = Point::new(
        f64::midpoint(box_[0].x, box_[1].x) - 0.01,
        f64::midpoint(box_[0].y, box_[3].y),
    );
    session.set_tool(Tool::Text);
    session.pointer_down(first, false);
    session.pointer_up(first, false, false);
    assert!(session.text_editing());
    let caret = session.text.edit.as_ref().unwrap().caret();
    assert!(caret <= 1, "{caret}");
    session.text_input(TextInput::Insert("X".to_owned()));
    session.finish_text_edit();
    let edited = text(&session, t);
    assert!(edited.string.contains('X'));
    assert!(edited.on_path.is_some());
    assert_eq!(edited.outline.unwrap().letter_boxes.len(), 6);
}

/// Criterion 25: Delete of the carrier keeps the text's curve and says so.
#[test]
fn c25_deleting_the_carrier_keeps_the_text_and_says_so() {
    let (mut session, t, c) = badge();
    session.selection.set(&[t, c]);
    session.text_path_press();
    let before = text(&session, t).outline.unwrap().line_box;
    session.selection.select_single(c);
    session.key_down(KeyInput {
        key: "Delete",
        ..KeyInput::default()
    });
    assert_eq!(session.take_clone_repair_notice(), "1 text kept its path.");
    let kept = text(&session, t);
    assert!(kept.on_path.unwrap().kept);
    assert_eq!(kept.outline.unwrap().line_box, before);
}

/// Criterion 20: the typed rotate entry `R` on a text on a circle moves the text's middle by
/// the typed angle about the centre, as the rotate handles do, in one step "Move text on path".
#[test]
fn c20_typed_rotate_moves_the_start_along_the_circle() {
    let (mut session, t, c) = badge();
    session.selection.set(&[t, c]);
    session.text_path_press();
    session.selection.select_single(t);
    let angle_of = |session: &Session| {
        let p = session.text_path_handles().unwrap().handles[0].position;
        (p.y - 100.0).atan2(p.x - 100.0).to_degrees()
    };
    let before = angle_of(&session);
    let _ = session.key_down(KeyInput {
        key: "r",
        ..KeyInput::default()
    });
    let entry = session.transform_entry().expect("R opens the angle entry");
    assert_eq!(entry.kind, "angle");
    let start: f64 = entry.fields[0].prefill.parse().unwrap();
    let outcome = session.commit_transform_entry(&format!("{}", start + 90.0), "", 0);
    assert_eq!(outcome, curvyo_ui_core::EntryOutcome::Committed);
    assert!(session.transform_entry().is_none(), "the entry closes");
    assert_eq!(labels(&session).last().unwrap(), "move_text_on_path");
    let turned = (angle_of(&session) - before).rem_euclid(360.0);
    assert!((turned - 90.0).abs() < 0.05, "turned by {turned}°");
    assert!(text(&session, t).on_path.is_some(), "still on the circle");
}

/// Customer bug 2026-10-24: a double-click into edit mode, Escape twice and V leave no drag or
/// press behind: moving the pointer afterwards does not slide the text along its carrier.
#[test]
fn bug_no_slide_after_edit_mode_escape_and_v() {
    let (mut session, t, c) = badge();
    session.selection.set(&[t, c]);
    session.text_path_press();
    session.selection.select_single(t);
    let before = text(&session, t).on_path.unwrap().values;
    let box_ = text(&session, t).outline.unwrap().letter_boxes[2];
    let letter = Point::new(
        f64::midpoint(box_[0].x, box_[2].x),
        f64::midpoint(box_[0].y, box_[2].y),
    );
    // The host's double-click on the unselected text: the first press and release, the second
    // press withheld.
    session.selection.clear();
    session.pointer_hover(letter, false, false);
    session.pointer_down(letter, false);
    session.pointer_up(letter, false, false);
    session.double_click(letter, false, false);
    session.pointer_hover(letter, false, false);
    assert!(session.text_editing());
    session.escape();
    session.escape();
    let _ = session.key_down(KeyInput {
        key: "v",
        ..KeyInput::default()
    });
    for x in [110.0, 130.0, 160.0] {
        session.pointer_hover(Point::new(x, 140.0), false, false);
    }
    // V flips the letters (criterion 19); nothing slides.
    let after = text(&session, t).on_path.unwrap().values;
    assert_eq!(after.position, before.position);
    assert!(session.text_path_live().is_none(), "no drag preview");
    assert!(session.live_readout().is_none(), "no drag readout");
}
