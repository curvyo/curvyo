//! One text on several paths in the session (`specs/0080-text-on-path` slice 2, criteria 37 to 41;
//! UX notes §9): the pick set with Shift, Enter and the second press, the Select-tool route, edit
//! mode on a clone, Unlink clone, the Style row and the notice of a removed source.

use curvyo_document_core::{
    Angle, Curve, EllipseFrame, Length, NodeId, ObjectSnapshot, Point, Shape, TextSnapshot,
};
use curvyo_ui_core::TextInput;

use super::clones::CloneKind;
use super::keys::{KeyInput, KeyOutcome};
use super::text_on_path_pick::TextPathPress;
use super::text_tests::{labels, load_wanted, session};
use super::{Session, Tool};

fn circle(session: &Session, x: f64) -> NodeId {
    session.document.create_primitive(
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(x, 100.0),
                rx: Length::from_mm(50.0),
                ry: Length::from_mm(50.0),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        },
        Angle::from_radians(0.0),
    )
}

/// A session with a text "Badge" selected and three circles, centred at x = 100, 300, 500.
fn badge() -> (Session, NodeId, [NodeId; 3]) {
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
    let circles = [
        circle(&session, 100.0),
        circle(&session, 300.0),
        circle(&session, 500.0),
    ];
    session.set_tool(Tool::Select);
    session.selection.select_single(text);
    (session, text, circles)
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

fn clones_of(session: &Session, source: NodeId) -> Vec<NodeId> {
    session
        .object_read()
        .clones_of
        .get(&source)
        .cloned()
        .unwrap_or_default()
}

/// A click on the right edge of circle `i` (x = 150, 350, 550), with or without Shift.
fn pick(session: &mut Session, i: u32, shift: bool) {
    let at = Point::new(150.0 + f64::from(i) * 200.0, 100.0);
    session.pointer_hover(at, shift, false);
    session.pointer_down(at, shift);
    session.pointer_up(at, shift, false);
}

fn put_on_three(session: &mut Session, t: NodeId) -> Vec<NodeId> {
    session.text_path_press();
    for i in 0..3 {
        pick(session, i, true);
    }
    session.key_down(KeyInput {
        key: "Enter",
        ..KeyInput::default()
    });
    clones_of(session, t)
}

/// Criterion 37 and UX notes §9: Shift+click builds the pick set (a second Shift+click takes a
/// carrier out), the badge sits on the first; Enter puts the text on the set in one step.
#[test]
fn c37_shift_picks_a_set_and_enter_puts_the_text_on_it() {
    let (mut session, t, circles) = badge();
    session.text_path_press();
    pick(&mut session, 0, true);
    assert!(session.text_path_picking(), "Shift keeps pick mode");
    assert!(session.text_path_pick_badge().is_some());
    pick(&mut session, 1, true);
    pick(&mut session, 2, true);
    pick(&mut session, 2, true);
    let steps = labels(&session).len();
    let outcome = session.key_down(KeyInput {
        key: "Enter",
        ..KeyInput::default()
    });
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(!session.text_path_picking());
    assert_eq!(labels(&session).len(), steps + 1);
    assert_eq!(labels(&session).last().unwrap(), "put_text_on_paths");
    assert_eq!(
        text(&session, t).on_path.unwrap().carrier.node,
        circles[0],
        "the first picked carrier gets the text"
    );
    let clones = clones_of(&session, t);
    assert_eq!(clones.len(), 1);
    assert_eq!(
        text(&session, clones[0]).on_path.unwrap().carrier.node,
        circles[1]
    );
    assert_eq!(session.take_text_path_notice(), "Put text on 2 paths.");
    assert_eq!(session.selection.ids(), &[t]);
}

/// Criterion 2: a plain click with a pick set puts the text on the set and that carrier; a second
/// press of On path puts on the set too.
#[test]
fn c02_a_plain_click_or_a_second_press_ends_the_set() {
    let (mut session, t, _) = badge();
    session.text_path_press();
    pick(&mut session, 0, true);
    pick(&mut session, 1, false);
    assert!(!session.text_path_picking());
    assert_eq!(clones_of(&session, t).len(), 1);
    let (mut session, t, _) = badge();
    session.text_path_press();
    pick(&mut session, 0, true);
    pick(&mut session, 2, true);
    assert_eq!(session.text_path_press(), TextPathPress::Put);
    assert_eq!(clones_of(&session, t).len(), 1);
}

/// Criteria 3 and 37: the Select tool with the text and three circles selected puts at once; the
/// lowest circle in stacking order gets the text.
#[test]
fn c03_the_select_route_puts_on_every_selected_carrier() {
    let (mut session, t, circles) = badge();
    session
        .selection
        .set(&[t, circles[2], circles[0], circles[1]]);
    assert_eq!(session.text_path_press(), TextPathPress::Put);
    assert_eq!(text(&session, t).on_path.unwrap().carrier.node, circles[0]);
    assert_eq!(clones_of(&session, t).len(), 2);
    assert!(
        circles
            .iter()
            .all(|c| session.document_read().guides.contains(c))
    );
}

/// Criterion 38: edit mode on a clone shows each typed letter on the source and the other clone
/// in the same frame; the step writes the source.
#[test]
fn c38_typing_in_a_clone_shows_on_every_member() {
    let (mut session, t, _) = badge();
    let clones = put_on_three(&mut session, t);
    session.edit_text_at(clones[1], None);
    session.text_input(TextInput::Insert("!".to_owned()));
    let objects = session.objects();
    let drawn = session.with_text_edit(std::borrow::Cow::Borrowed(&objects[..]));
    for id in [t, clones[0], clones[1]] {
        let shown = drawn
            .iter()
            .find_map(|o| match o {
                ObjectSnapshot::Text(text) if text.id == id => Some((**text).clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(shown.string, "Badge!", "{id:?}");
        assert!(shown.outline.is_some());
    }
    session.finish_text_edit();
    assert_eq!(session.document.text(t).unwrap().string, "Badge!");
    assert_eq!(labels(&session).last().unwrap(), "edit_text");
}

/// Criteria 40, 41: the clone block without the Placement row, the Style row and Unlink clone on a
/// text clone.
#[test]
fn c40_c41_style_row_and_unlink_on_a_text_clone() {
    let (mut session, t, _) = badge();
    let clones = put_on_three(&mut session, t);
    session.selection.select_single(clones[0]);
    let panel = session.clone_panel();
    let block = panel.block.unwrap();
    assert!(!block.placement_row && block.texts);
    assert_eq!(session.set_clone_style(true).kind, CloneKind::Done);
    assert!(text(&session, clones[0]).source.unwrap().own_style);
    let unlinked = session.unlink_clones();
    assert_eq!(unlinked.notice, "Unlinked 1 clone.");
    assert_eq!(text(&session, clones[0]).source, None);
    assert_eq!(clones_of(&session, t), vec![clones[1]]);
}

/// Criterion 41: deleting the source turns its clones into texts; the notice says so.
#[test]
fn c41_deleting_the_source_says_the_clones_became_texts() {
    let (mut session, t, _) = badge();
    let clones = put_on_three(&mut session, t);
    session.selection.select_single(t);
    session.delete_selected();
    assert_eq!(
        session.take_delete_notice(),
        "Deleted 1 object. 2 text clones became texts."
    );
    assert!(clones.iter().all(|c| text(&session, *c).source.is_none()));
}

/// A linked clone of a fourth circle (x = 700) next to the text clones of `t`, selected with the
/// first text clone.
fn linked_and_text_clone(session: &mut Session, t: NodeId) -> (NodeId, NodeId) {
    let clones = put_on_three(session, t);
    let fourth = circle(session, 700.0);
    session.selection.select_single(fourth);
    assert_eq!(session.linked_clone().kind, CloneKind::Done);
    let linked = session.selection.ids()[0];
    session.selection.set(&[linked, clones[0]]);
    (linked, clones[0])
}

/// Lead decision 2: Unlink clone on linked and text clones selected together is one step.
#[test]
fn c41_unlink_of_linked_and_text_clones_is_one_step() {
    let (mut session, t, _) = badge();
    let (linked, clone) = linked_and_text_clone(&mut session, t);
    let before = labels(&session).len();
    assert_eq!(session.unlink_clones().notice, "Unlinked 2 clones.");
    assert_eq!(labels(&session)[before..], ["unlink_clone"]);
    assert!(matches!(
        session.object_read().get(linked),
        Some(ObjectSnapshot::Path(_))
    ));
    assert_eq!(text(&session, clone).source, None);
    session.undo();
    assert!(text(&session, clone).source.is_some());
    assert!(matches!(
        session.object_read().get(linked),
        Some(ObjectSnapshot::Clone(_))
    ));
}

/// Lead decision 2: the Style row on linked and text clones selected together is one step.
#[test]
fn c40_style_row_on_linked_and_text_clones_is_one_step() {
    let (mut session, t, _) = badge();
    let (_, clone) = linked_and_text_clone(&mut session, t);
    let before = labels(&session).len();
    assert_eq!(session.set_clone_style(true).kind, CloneKind::Done);
    assert_eq!(labels(&session)[before..], ["own_style"]);
    assert!(text(&session, clone).source.unwrap().own_style);
    assert_eq!(session.set_clone_style(false).kind, CloneKind::Done);
    assert_eq!(labels(&session)[before + 1..], ["original_style"]);
}

/// Deletes every character of the text in edit mode.
fn empty_edit(session: &mut Session) {
    for _ in 0.."Badge".len() {
        session.text_input(TextInput::DeleteBackward);
    }
}

/// The text `id` as drawn this frame (edit mode applied).
fn drawn(session: &Session, id: NodeId) -> Option<TextSnapshot> {
    let objects = session.objects();
    session
        .with_text_edit(std::borrow::Cow::Borrowed(&objects[..]))
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Text(text) if text.id == id => Some((**text).clone()),
            _ => None,
        })
}

/// Lead decision 1: an emptied source stays an empty text while edit mode is on (its clones draw
/// nothing, a Save writes no step; the file keeps the last written string); leaving edit mode
/// removes the source alone, and its clones become texts with the string they showed before, in the one step "Edit text" with the notice.
#[test]
fn emptied_source_removes_only_the_source_and_its_clones_become_texts() {
    let (mut session, t, _) = badge();
    let clones = put_on_three(&mut session, t);
    session.edit_text_at(t, None);
    empty_edit(&mut session);
    assert!(
        clones
            .iter()
            .all(|c| drawn(&session, *c).unwrap().string.is_empty())
    );
    let before = labels(&session).len();
    session.checkpoint_text_edit();
    assert!(session.text_editing());
    assert_eq!(labels(&session).len(), before);
    session.finish_text_edit();
    assert_eq!(labels(&session)[before..], ["edit_text"]);
    assert!(session.document.text(t).is_none());
    for clone in &clones {
        let read = text(&session, *clone);
        assert_eq!((read.source, read.string.as_str()), (None, "Badge"));
    }
    assert_eq!(
        session.take_text_path_notice(),
        "2 text clones became texts."
    );
    session.undo();
    assert_eq!(session.document.text(t).unwrap().string, "Badge");
    assert_eq!(clones_of(&session, t), clones);
}

/// Lead decision 1: an emptied text clone is removed alone; the source and the other clone keep
/// the string.
#[test]
fn emptied_clone_removes_only_that_clone() {
    let (mut session, t, _) = badge();
    let clones = put_on_three(&mut session, t);
    session.edit_text_at(clones[0], None);
    empty_edit(&mut session);
    session.finish_text_edit();
    assert!(session.document.text(clones[0]).is_none());
    assert_eq!(session.document.text(t).unwrap().string, "Badge");
    assert_eq!(text(&session, clones[1]).string, "Badge");
    assert_eq!(clones_of(&session, t), vec![clones[1]]);
    assert_eq!(session.take_text_path_notice(), "");
}

/// Lead decision 3: Spread colours works on text clones as on linked clones, together in one
/// step; each gets its own style in a hue of its own.
#[test]
fn c40_spread_colours_on_text_and_linked_clones_is_one_step() {
    let (mut session, t, _) = badge();
    let red = curvyo_document_core::Color { r: 200, g: 0, b: 0 };
    session
        .document
        .edit_style(&[t], &curvyo_document_core::StyleEdit::FillColor(red))
        .unwrap();
    let (linked, clone) = linked_and_text_clone(&mut session, t);
    let other = clones_of(&session, t)[1];
    session.selection.set(&[linked, clone, other]);
    assert!(session.spread_colours_ready());
    let before = labels(&session).len();
    assert_eq!(
        session.spread_colours().notice,
        "Spread hues over 3 clones."
    );
    assert_eq!(labels(&session)[before..], ["spread_colours"]);
    let (a, b) = (text(&session, clone), text(&session, other));
    assert!(a.source.unwrap().own_style && b.source.unwrap().own_style);
    assert_ne!(a.style.fill.color, b.style.fill.color);
    assert_eq!(text(&session, t).source, None);
}
