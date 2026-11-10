//! The selection's announcement, the canvas tag and tape edges, the Layers mark of a selected fixture and
//! the hit of a fixture of another laser page (`specs/0102-laser-fixtures` criteria 8, 11, 12, 14).
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use curvyo_document_core::Point;
use serde_json::Value;

use super::super::Session;
use super::tests::{laser, view};

fn layers(s: &mut Session) -> Vec<Value> {
    let v: Value = serde_json::from_str(&s.layers_view_json(0, 60)).unwrap();
    v["rows"].as_array().unwrap().clone()
}

#[test]
fn ux_11_the_selected_fixture_is_announced_with_its_sizes() {
    let mut s = laser();
    s.add_fixture("holder").unwrap();
    assert_eq!(
        view(&s).selection_announcement.as_deref(),
        Some("Holder 1 selected, 100.0 by 60.0 mm.")
    );
    s.add_fixture("stop").unwrap();
    assert_eq!(
        view(&s).selection_announcement.as_deref(),
        Some("Stop 1 selected, legs 60.0 and 60.0 mm, thickness 5.0 mm.")
    );
    s.add_fixture("screw").unwrap();
    assert_eq!(
        view(&s).selection_announcement.as_deref(),
        Some("Screw 1 selected, thread 6.0 mm, head 10.0 mm.")
    );
    s.clear_fixture_selection();
    assert!(view(&s).selection_announcement.is_none());
}

#[test]
fn criterion_8_the_selected_fixture_has_one_tag_at_its_corner_and_no_name_label() {
    let mut s = laser();
    s.add_fixture("holder").unwrap();
    let labels = s.fixture_labels();
    let tags: Vec<_> = labels.iter().filter(|l| l.kind == "tag").collect();
    assert_eq!(tags.len(), 1, "one tag for the selected fixture");
    assert_eq!(tags[0].text, "Holder 1");
    assert!(
        labels
            .iter()
            .all(|l| !(l.kind == "fixture" && l.text == "Holder 1")),
        "the tag replaces the name label"
    );
}

#[test]
fn criterion_8_tape_numbers_name_the_edge_they_sit_on() {
    let mut s = laser();
    s.add_fixture("grid").unwrap();
    let labels = s.fixture_labels();
    let tapes: Vec<_> = labels.iter().filter(|l| l.kind == "tape").collect();
    assert!(!tapes.is_empty(), "the grid has tape numbers");
    assert!(
        tapes.iter().all(|l| l.edge.is_some()),
        "every tape number names its edge"
    );
    let edges: Vec<&str> = tapes.iter().filter_map(|l| l.edge).collect();
    for edge in ["bottom", "left", "top", "right"] {
        assert!(edges.contains(&edge), "a number on the {edge} tape");
    }
}

#[test]
fn criterion_12_the_layers_row_of_the_selected_fixture_is_marked_selected() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    let clamp = s.add_fixture("clamp").unwrap();
    assert!(s.select_fixture(holder));
    let rows = layers(&mut s);
    let mark = |id: &str| {
        rows.iter()
            .find(|r| r["kind"] == "fixture" && r["fixture"]["id"] == id)
            .map(|r| r["mark"].clone())
            .unwrap()
    };
    assert_eq!(mark(&holder.full_id()), "selected");
    assert_eq!(mark(&clamp.full_id()), "");
}

#[test]
fn criterion_11_a_press_on_a_fixture_of_another_laser_page_activates_that_page_and_selects_nothing()
{
    let mut s = laser();
    let page_a = s.active_page();
    let holder = s.add_fixture("holder").unwrap();
    assert!(super::tests::place(&mut s, holder, 20.0, 30.0));
    // A second laser page becomes the active one.
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert_ne!(s.active_page().id, page_a.id);
    // The left outline of the holder on page A, halfway up its 60 mm height.
    let on_outline = Point::new(page_a.origin.x + 20.0, page_a.origin.y + 60.0);
    s.pointer_down(on_outline, false);
    assert_eq!(s.active_page().id, page_a.id, "the press activates page A");
    assert!(view(&s).selected.is_none(), "and selects no fixture");
}
