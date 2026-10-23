//! The Style group in the session (`specs/0078-font-style-buttons` criteria 3 to 18, 22 to 26),
//! with the included fonts listed as `index.json` lists them and loaded on request.

use curvyo_document_core::{NodeId, ObjectSnapshot, Point};
use curvyo_text_core::WeightBand;
use curvyo_ui_core::{FontBlock, Greyed, Pressed, TextInput};

use super::text_path::ObjectToPathOutcome;
use super::text_style::StyleOutcome;
use super::text_tests::{labels, load_wanted, session, texts, with_text};
use super::{Session, Tool};

fn style(session: &Session) -> String {
    let text = &texts(session)[0];
    format!("{} {}", text.font.family, text.font.style)
}

fn selected(string: &str) -> (Session, NodeId) {
    let mut session = with_text(string);
    let id = texts(&session)[0].id;
    session.set_tool(Tool::Select);
    session.selection.set(&[id]);
    (session, id)
}

/// The listed file of a face of the included block.
fn file_of(session: &Session, family: &str, style: &str) -> (String, u32) {
    let face = session
        .fonts
        .faces()
        .iter()
        .find(|f| f.block == FontBlock::Included && f.family == family && f.style == style)
        .unwrap();
    (face.host_id.clone(), face.index)
}

/// Criteria 3, 5 and 8: Source Sans 3 Regular with the shipped faces: Regular pressed, Extra
/// Bold greyed "Not in Source Sans 3", the rest available.
#[test]
fn ac03_ac08_source_sans_states() {
    let (session, _) = selected("Name");
    let buttons = session.text_bar().style;
    let pressed: Vec<Pressed> = buttons.weights.iter().map(|b| b.pressed).collect();
    assert_eq!(
        pressed,
        [
            Pressed::Off,
            Pressed::Off,
            Pressed::On,
            Pressed::Off,
            Pressed::Off
        ]
    );
    let greyed: Vec<Option<Greyed>> = buttons.weights.iter().map(|b| b.greyed.clone()).collect();
    assert_eq!(
        greyed,
        [
            None,
            None,
            None,
            None,
            Some(Greyed::NotIn("Source Sans 3".into()))
        ]
    );
    assert_eq!(buttons.italic.pressed, Pressed::Off);
    assert_eq!(buttons.italic.greyed, None);
}

/// Criteria 10, 13, 14, 17: Bold, then Italic, then Light, each one step "Change font" once
/// its face has arrived; the Size kept with the H 20.00 mm high; Undo is one step back.
#[test]
fn ac10_ac13_ac14_ac17_bold_italic_light() {
    let (mut session, _) = selected("H");
    session.set_text_size(20.0);
    let steps = labels(&session).len();
    let outcome = session.press_weight(WeightBand::Bold);
    assert_eq!(
        outcome,
        Some(StyleOutcome {
            set: 1,
            left: 0,
            upright: false
        })
    );
    assert_eq!(labels(&session).len(), steps, "waits for the Bold face");
    load_wanted(&mut session);
    assert_eq!(style(&session), "Source Sans 3 Bold");
    assert_eq!(labels(&session).len(), steps + 1);
    assert_eq!(labels(&session).last().unwrap(), "change_font");
    session.press_italic();
    load_wanted(&mut session);
    assert_eq!(style(&session), "Source Sans 3 Bold Italic");
    session.press_weight(WeightBand::Light);
    load_wanted(&mut session);
    assert_eq!(style(&session), "Source Sans 3 Light Italic");
    assert_eq!(labels(&session).len(), steps + 3);
    assert_eq!(session.text_bar().style.italic.pressed, Pressed::On);
    assert_eq!(session.text_bar().style.weights[1].pressed, Pressed::On);
    assert_eq!(session.text_bar().font, "Source Sans 3 Light Italic");
    let text = texts(&session)[0].clone();
    assert_eq!(text.size.as_mm(), 20.0);
    session.undo();
    assert_eq!(style(&session), "Source Sans 3 Bold Italic");
    session.redo();
    let baseline = text.placement.f;
    assert_eq!(session.object_to_path(), ObjectToPathOutcome::Converted);
    let top = session
        .objects()
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Path(path) => Some(
                path.all_anchors()
                    .map(|a| a.point.y)
                    .fold(f64::INFINITY, f64::min),
            ),
            _ => None,
        })
        .fold(f64::INFINITY, f64::min);
    assert!((baseline - top - 20.0).abs() < 0.01, "{}", baseline - top);
}

/// Criteria 6 and 11: several texts, one in a family without Bold: one step, the counts for the
/// notice; a greyed or pressed button does nothing.
#[test]
fn ac06_ac11_several_texts() {
    let (mut session, first) = selected("One");
    session.set_tool(Tool::Text);
    session.selection.clear();
    session.pointer_down(Point::new(10.0, 60.0), false);
    session.pointer_up(Point::new(10.0, 60.0), false, false);
    session.text_input(TextInput::Insert("Two".into()));
    session.escape();
    let second = texts(&session).iter().find(|t| t.id != first).unwrap().id;
    session.set_tool(Tool::Select);
    session.selection.set(&[second]);
    let (host, index) = file_of(&session, "Great Vibes", "Regular");
    session.pick_font_face(&host, index);
    load_wanted(&mut session);
    session.selection.set(&[first, second]);
    let steps = labels(&session).len();
    assert_eq!(session.press_weight(WeightBand::ExtraBold), None);
    assert_eq!(
        session
            .press_weight(WeightBand::Bold)
            .map(|o| (o.set, o.left)),
        Some((1, 1))
    );
    load_wanted(&mut session);
    assert_eq!(labels(&session).len(), steps + 1);
    let fonts: Vec<String> = texts(&session)
        .iter()
        .map(|t| t.font.style.clone())
        .collect();
    assert!(fonts.contains(&"Bold".to_owned()) && fonts.contains(&"Regular".to_owned()));
    session.selection.set(&[second]);
    assert_eq!(session.press_italic(), None, "Great Vibes has no italic");
    assert_eq!(labels(&session).len(), steps + 1);
}

/// Criterion 15: with no text, a press sets the next text's face and writes no step.
#[test]
fn ac15_next_text_without_a_step() {
    let mut session = session();
    session.set_tool(Tool::Text);
    session.press_weight(WeightBand::Light);
    load_wanted(&mut session);
    assert_eq!(session.text_bar().font, "Source Sans 3 Light");
    assert_eq!(session.text_bar().style.weights[1].pressed, Pressed::On);
    assert_eq!(labels(&session), Vec::<String>::new());
}

/// Criterion 16: in edit mode a press changes the edited text and edit mode stays.
#[test]
fn ac16_edit_mode_stays() {
    let mut session = with_text("Name");
    session.pointer_down(Point::new(10.2, 18.0), false);
    session.pointer_up(Point::new(10.2, 18.0), false, false);
    assert!(session.text_editing());
    session.press_weight(WeightBand::Bold);
    load_wanted(&mut session);
    assert!(session.text_editing());
    assert_eq!(style(&session), "Source Sans 3 Bold");
    assert_eq!(labels(&session).last().unwrap(), "change_font");
}

/// Criteria 22 and 24: a family pick keeps Bold Italic where the family has it, and the tile
/// shows that face; Great Vibes gives its only face.
#[test]
fn ac22_ac24_family_pick_keeps_the_style() {
    let (mut session, _) = selected("Name");
    session.press_weight(WeightBand::Bold);
    load_wanted(&mut session);
    session.press_italic();
    load_wanted(&mut session);
    let (serif, index) = file_of(&session, "Source Serif 4", "Regular");
    assert_eq!(
        session.family_tile_face(&serif, index),
        Some(file_of(&session, "Source Serif 4", "Bold Italic"))
    );
    session.pick_font_family(&serif, index);
    load_wanted(&mut session);
    assert_eq!(style(&session), "Source Serif 4 Bold Italic");
    let (vibes, index) = file_of(&session, "Great Vibes", "Regular");
    assert_eq!(
        session.family_tile_face(&vibes, index),
        Some((vibes.clone(), index))
    );
    session.pick_font_family(&vibes, index);
    load_wanted(&mut session);
    assert_eq!(style(&session), "Great Vibes Regular");
}

/// Criterion 26: a face set by the buttons is the same after save and open.
#[test]
fn ac26_reopen_keeps_the_face() {
    let (mut session, _) = selected("Name");
    session.press_weight(WeightBand::ExtraLight);
    load_wanted(&mut session);
    let font = texts(&session)[0].font.clone();
    assert_eq!(font.style, "Extra Light");
    let bytes = session.pack_for_save().unwrap();
    let opened = Session::open(2, &bytes).unwrap();
    assert_eq!(texts(&opened)[0].font, font);
}
