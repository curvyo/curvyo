//! Session tests of the Materials tab (`specs/0112` criteria 16 to 27): materials and sets per
//! laser as steps, the test record that follows a grid, judging, note, photo bytes, deleting with
//! retired ids, the journal's undo and redo with photo bytes, the per-laser tie.

use super::view_types::SetKindView;
use curvyo_laser_core::grids::TestId;
use curvyo_laser_core::library::{MAX_RECORD_TEXT, MaterialRead};

use super::super::Session;
use super::super::test_grid::tests::{laser_with_fonts, place_grid, recipe_of, undo_label};

pub(in crate::session) const RANDOM: [u8; 16] = [7; 16];

fn random(n: u8) -> [u8; 16] {
    let mut bytes = RANDOM;
    bytes[0] = n;
    bytes
}

/// A PNG header of `width` x `height` (no pixels: core reads the header only).
pub(in crate::session) fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    bytes
}

fn writes(s: &mut Session) -> Vec<serde_json::Value> {
    serde_json::from_str(&s.take_material_writes_json()).unwrap()
}

/// A laser page with "Birch 3 mm" and a grid that uses it.
pub(in crate::session) fn birch_grid() -> (Session, String, curvyo_document_core::NodeId) {
    let mut s = laser_with_fonts();
    let birch = s.add_material("Birch 3 mm", 3.0, &random(1)).unwrap();
    let group = place_grid(&mut s);
    assert_eq!(s.pick_test_grid_material(Some(&birch)), Ok(true));
    (s, birch, group)
}

#[test]
fn criterion_17_add_material_is_one_step_and_undo_takes_the_record_back() {
    let mut s = laser_with_fonts();
    let steps = s.undo_len();
    let id = s.add_material("Birch 3 mm", 3.0, &random(1)).unwrap();
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Add material");
    let w = writes(&mut s);
    assert_eq!(w[0]["op"], "write");
    let view = s.materials_view().unwrap();
    assert_eq!(view.unfolded.as_ref().unwrap().id, id);
    assert_eq!(
        s.add_material(" ", 3.0, &random(2)),
        Err("Enter a name.".to_owned())
    );
    assert_eq!(
        s.add_material("x", 0.0, &random(2)),
        Err("Thickness is 0.1 to 100 mm.".to_owned())
    );
    let _ = s.undo();
    assert!(s.materials().find(&id).is_none());
    assert_eq!(writes(&mut s)[0]["op"], "remove");
}

#[test]
fn criteria_18_to_21_sets_per_laser_a_set_adds_its_laser() {
    let mut s = laser_with_fonts();
    let id = s.add_material("Birch", 3.0, &random(1)).unwrap();
    let laser = s.active_profile_id().unwrap();
    s.add_material_set(&id, &laser, "cut", &random(2)).unwrap();
    assert_eq!(undo_label(&mut s), "Add set");
    assert_eq!(
        s.edit_material_set(&id, &laser, "cut", "power", 60.0, &random(3)),
        Ok(true)
    );
    assert_eq!(undo_label(&mut s), "Edit material");
    assert_eq!(
        s.edit_material_set(&id, &laser, "cut", "interval", 0.1, &random(4)),
        Err("Enter a number.".to_owned()),
        "no interval on Cut"
    );
    s.add_material_set(&id, &laser, "engrave", &random(5))
        .unwrap();
    assert_eq!(
        s.edit_material_set(&id, &laser, "engrave", "interval", 0.08, &random(6)),
        Ok(true)
    );
    let m = s.materials().find(&id).unwrap();
    assert_eq!(m.lasers.len(), 1);
    assert_eq!(m.set_count(), 2);
    s.unfold_material(Some(&id));
    let view = s.materials_view().unwrap();
    let body = view.unfolded.unwrap();
    assert!(body.lasers[0].this_page);
    assert_eq!(body.lasers[0].sets[0].source, "Typed");
    assert_eq!(
        body.lasers[0].missing.len(),
        2,
        "Score and Image are missing (`specs/0123` Frage 270 B)"
    );
    s.delete_material_set(&id, &laser, "cut", &random(7))
        .unwrap();
    assert_eq!(undo_label(&mut s), "Delete set");
    let line = match &s.materials_view().unwrap().rows[0] {
        super::view_types::MaterialRowView::Loaded { line, .. } => line.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(line, "1 laser · 1 set");
}

#[test]
fn criterion_21_an_operation_uses_the_set_of_its_page_laser() {
    let mut s = laser_with_fonts();
    let id = s.add_material("Birch", 3.0, &random(1)).unwrap();
    // A set for another laser only.
    let other = curvyo_laser_core::settings::MachineTie {
        id: "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f".to_owned(),
        name: "Diode 20 W".to_owned(),
    };
    s.materials
        .tab
        .picked_lasers
        .insert(id.clone(), vec![other.clone()]);
    s.add_material_set(&id, &other.id, "cut", &random(2))
        .unwrap();
    s.add_material_set(&id, &s.active_profile_id().unwrap(), "engrave", &random(3))
        .unwrap();
    let cut = s.add_test_operation("cut").unwrap();
    assert!(
        !s.use_material_on(&[cut], &id),
        "no Cut set for this laser: skipped"
    );
    let engrave = s.add_test_operation("engrave").unwrap();
    assert!(s.use_material_on(&[engrave], &id));
}

#[test]
fn criterion_22_a_pick_makes_the_test_a_commit_rewrites_it_another_pick_moves_it() {
    let (mut s, birch, group) = birch_grid();
    let id = recipe_of(&s, group).id;
    assert_eq!(undo_label(&mut s), "Use material");
    let test = s
        .materials()
        .find(&birch)
        .unwrap()
        .test(id)
        .unwrap()
        .clone();
    assert!(test.lasered.is_none());
    assert_eq!(s.set_test_grid_field("columns_count", "4"), Ok(true));
    let rewritten = s
        .materials()
        .find(&birch)
        .unwrap()
        .test(id)
        .unwrap()
        .clone();
    assert_eq!(
        rewritten
            .recipe
            .axis(curvyo_laser_core::grids::Side::Columns)
            .count(),
        4
    );
    let walnut = s.add_material("Walnut", 4.0, &random(9)).unwrap();
    s.select_grid(group);
    assert_eq!(s.pick_test_grid_material(Some(&walnut)), Ok(true));
    assert!(s.materials().find(&birch).unwrap().test(id).is_none());
    assert!(s.materials().find(&walnut).unwrap().test(id).is_some());
    assert!(!s.materials.id_taken(TestId::parse("ZZZZZ").unwrap()));
    // Undo takes both records back in one step.
    let _ = s.undo();
    assert!(s.materials().find(&birch).unwrap().test(id).is_some());
    assert!(s.materials().find(&walnut).unwrap().test(id).is_none());
}

#[test]
fn criterion_24_one_tap_makes_the_best_cell_and_the_set() {
    let (mut s, birch, group) = birch_grid();
    let id = recipe_of(&s, group).id.to_string();
    s.unfold_material(Some(&birch));
    s.open_material_test(Some(&id));
    s.mark_pick(&birch, &id, "best", 2, 1, &random(3)).unwrap();
    assert_eq!(undo_label(&mut s), "Mark pick");
    let laser = s.active_profile_id().unwrap();
    let m = s.materials().find(&birch).unwrap();
    let set = m
        .set_for(&laser, curvyo_laser_core::settings::SetKind::Cut)
        .unwrap();
    assert_eq!(set.numbers.settings.power.percent(), 60.0);
    let detail = s.materials_view().unwrap().unfolded.unwrap().test.unwrap();
    assert_eq!(detail.picks[0].cell.column, 2);
    assert!(
        detail.picks[0]
            .line
            .as_deref()
            .unwrap()
            .starts_with("Best: Cut set of ")
    );
    assert!(detail.place.on_page);
    s.mark_pick(&birch, &id, "best", 2, 1, &random(4)).unwrap();
    assert_eq!(
        undo_label(&mut s),
        "Unmark pick",
        "a tap on the best cell unmarks it"
    );
}

#[test]
fn criteria_25_to_27_note_photo_and_delete_with_its_retired_id() {
    let (mut s, birch, group) = birch_grid();
    let id = recipe_of(&s, group).id.to_string();
    let _ = writes(&mut s);
    assert_eq!(s.set_test_note(&birch, &id, "Clean edge."), Ok(true));
    assert_eq!(undo_label(&mut s), "Edit note");
    assert_eq!(s.set_test_note(&birch, &id, "Clean edge."), Ok(false));
    assert!(s.set_test_note(&birch, &id, &"x".repeat(501)).is_err());
    assert_eq!(
        s.set_test_photo(&birch, &id, b"GIF89a", false),
        Err("Use a JPEG or PNG image.".to_owned())
    );
    s.set_test_photo(&birch, &id, &png(600, 400), false)
        .unwrap();
    assert_eq!(undo_label(&mut s), "Add photo");
    let w = writes(&mut s);
    let photo = w.iter().find(|w| w["op"] == "write_photo").unwrap();
    let name = photo["name"].as_str().unwrap().to_owned();
    assert_eq!(name, format!("{birch}.{id}.png"));
    let handle = u32::try_from(photo["handle"].as_u64().unwrap()).unwrap();
    assert_eq!(s.take_material_photo_bytes(handle), Some(png(600, 400)));
    assert_eq!(s.take_material_photo_bytes(handle), None, "once");
    s.set_test_photo(&birch, &id, &png(500, 400), true).unwrap();
    assert_eq!(undo_label(&mut s), "Crop photo");
    // Undo of the crop writes the earlier bytes back.
    let _ = writes(&mut s);
    let _ = s.undo();
    let w = writes(&mut s);
    let back = w.iter().find(|w| w["op"] == "write_photo").unwrap();
    let handle = u32::try_from(back["handle"].as_u64().unwrap()).unwrap();
    assert_eq!(s.take_material_photo_bytes(handle), Some(png(600, 400)));
    s.remove_test_photo(&birch, &id).unwrap();
    assert!(writes(&mut s).iter().any(|w| w["op"] == "remove_photo"));
    s.set_test_photo(&birch, &id, &png(600, 400), false)
        .unwrap();
    let _ = writes(&mut s);
    s.delete_test(&birch, &id).unwrap();
    assert_eq!(undo_label(&mut s), "Delete test");
    let w = writes(&mut s);
    assert!(w.iter().any(|w| w["op"] == "remove_photo"));
    let retired = w.iter().find(|w| w["op"] == "retired").unwrap();
    assert!(retired["text"].as_str().unwrap().contains(&id));
    assert!(s.materials.id_taken(TestId::parse(&id).unwrap()));
}

#[test]
fn criterion_17_deleting_a_material_retires_its_tests_and_removes_photos() {
    let (mut s, birch, group) = birch_grid();
    let id = recipe_of(&s, group).id.to_string();
    s.set_test_photo(&birch, &id, &png(10, 10), false).unwrap();
    let _ = writes(&mut s);
    s.delete_material(&birch).unwrap();
    assert_eq!(undo_label(&mut s), "Delete material");
    let w = writes(&mut s);
    let ops: Vec<&str> = w.iter().map(|w| w["op"].as_str().unwrap()).collect();
    assert!(ops.contains(&"remove") && ops.contains(&"remove_photo") && ops.contains(&"retired"));
    let _ = s.undo();
    assert!(s.materials().find(&birch).is_some());
    assert!(
        s.materials.id_taken(TestId::parse(&id).unwrap()),
        "undo leaves the id retired"
    );
}

#[test]
fn deleting_a_material_in_the_same_second_as_its_last_edit_is_a_step() {
    let mut s = laser_with_fonts();
    s.set_clock(|| curvyo_document_core::Timestamp::from_secs(1_700_000_000));
    let birch = s.add_material("Birch 3 mm", 3.0, &random(1)).unwrap();
    let steps = s.undo_len();
    assert_eq!(s.delete_material(&birch), Ok(()));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Delete material");
}

#[test]
fn the_journal_counts_photo_bytes_and_drops_the_oldest_past_its_limit() {
    let (mut s, birch, group) = birch_grid();
    let id = recipe_of(&s, group).id.to_string();
    s.library_journal().set_limit(4_000);
    s.set_test_photo(&birch, &id, &png(600, 400), false)
        .unwrap();
    s.set_test_note(&birch, &id, "a").unwrap();
    assert!(s.library_journal().bytes() <= 4_000);
}

#[test]
fn a_material_record_over_640_kib_is_refused_with_its_notice() {
    let mut s = laser_with_fonts();
    let id = s.add_material("Birch", 3.0, &random(1)).unwrap();
    let mut change = super::LibraryChange::default();
    let refused = s.change_record(&mut change, &id, |m| {
        m.name = "Birch".to_owned();
        for i in 0..8_000 {
            m.history.push(curvyo_laser_core::library::HistoryEntry {
                id: format!("{i:0>36}"),
                date: 1,
                machine: "generic-laser".to_owned(),
                kind: curvyo_laser_core::settings::SetKind::Cut,
                event: curvyo_laser_core::library::HistoryEvent::Removed,
            });
        }
        Ok(())
    });
    assert_eq!(
        refused,
        Err("Could not save Birch: it is over 640 KiB.".to_owned())
    );
    assert!(s.materials().text(&id).unwrap().len() < MAX_RECORD_TEXT);
    assert!(matches!(s.materials().reads()[0], MaterialRead::Loaded(_)));
}

/// `specs/0123` Frage 270 B (ADR 0069 §14): an Image operation reads the Image set, follows its
/// edit in the same step, and names a missing Image set; the tab shows four sets, Image last.
#[test]
fn raster_an_image_operation_follows_the_image_set_and_names_a_missing_one() {
    let mut s = laser_with_fonts();
    let id = s.add_material("Slate", 5.0, &random(1)).unwrap();
    let laser = s.active_profile_id().unwrap();
    s.add_material_set(&id, &laser, "engrave", &random(2))
        .unwrap();
    s.add_material_set(&id, &laser, "image", &random(3))
        .unwrap();
    assert_eq!(undo_label(&mut s), "Add set");
    assert_eq!(
        s.edit_material_set(&id, &laser, "image", "power", 35.0, &random(4)),
        Ok(true)
    );
    assert_eq!(
        s.edit_material_set(&id, &laser, "image", "interval", 0.08, &random(5)),
        Ok(true)
    );
    let op = s.add_test_operation("image").unwrap();
    assert!(s.use_material_on(&[op], &id));
    let stored = |s: &Session| {
        let row = s
            .document
            .operations(s.active_manufacturing().unwrap().id)
            .unwrap()
            .into_iter()
            .find(|o| o.id == op)
            .unwrap();
        row.settings.unwrap()
    };
    assert_eq!(stored(&s)["main"]["power_pct"], 35.0, "the Image set");
    assert_eq!(stored(&s)["main"]["interval_mm"], 0.08);
    let steps = s.undo_len();
    assert_eq!(
        s.edit_material_set(&id, &laser, "image", "power", 40.0, &random(6)),
        Ok(true)
    );
    assert_eq!(
        s.undo_len(),
        steps + 1,
        "the material edit and the refresh: one step"
    );
    assert_eq!(
        stored(&s)["main"]["power_pct"],
        40.0,
        "the linked operation follows"
    );
    s.unfold_material(Some(&id));
    let view = serde_json::to_value(s.materials_view().unwrap()).unwrap();
    let kinds: Vec<&str> = view["unfolded"]["lasers"][0]["sets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|set| set["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["engrave", "image"], "Image last");
    s.delete_material_set(&id, &laser, "image", &random(7))
        .unwrap();
    let rows: serde_json::Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    let text = rows.to_string();
    assert!(
        text.contains("Material has no Image set for this laser, using stored numbers"),
        "{text}"
    );
    assert_eq!(
        stored(&s)["main"]["power_pct"],
        40.0,
        "the stored numbers stay"
    );
}

/// `specs/0146` criteria 2, 3, 7, 9: a Fastest mark writes its pick set without a history entry,
/// shows its line and the folded line under the set, and a second tap unmarks it ("Unmark pick");
/// the pick set goes with the mark.
#[test]
fn criteria_2_3_7_9_marking_fastest_and_unmarking_it() {
    let (mut s, birch, group) = birch_grid();
    let id = recipe_of(&s, group).id.to_string();
    s.unfold_material(Some(&birch));
    s.open_material_test(Some(&id));
    s.mark_pick(&birch, &id, "best", 2, 1, &random(3)).unwrap();
    s.mark_pick(&birch, &id, "fastest", 3, 3, &random(4))
        .unwrap();
    assert_eq!(undo_label(&mut s), "Mark pick");
    let laser = s.active_profile_id().unwrap();
    let m = s.materials().find(&birch).unwrap();
    assert_eq!(m.history.len(), 1, "a pick other than Best writes no entry");
    let kind = curvyo_laser_core::settings::SetKind::Cut;
    let fastest = m
        .pick_set(&laser, kind, curvyo_laser_core::settings::PickKind::Fastest)
        .unwrap();
    assert_eq!(fastest.numbers.settings.power.percent(), 80.0);
    let body = s.materials_view().unwrap().unfolded.unwrap();
    let picks = &body.test.as_ref().unwrap().picks;
    assert_eq!(
        picks.iter().map(|p| p.pick).collect::<Vec<_>>(),
        ["best", "fastest"]
    );
    assert!(
        picks[1]
            .line
            .as_deref()
            .unwrap()
            .starts_with("Fastest: 80 %, ")
    );
    let other = &body.lasers[0].sets[0].other_picks;
    assert_eq!(
        (other[0].pick, other[0].source.as_str()),
        ("fastest", format!("Test {id}").as_str())
    );
    s.mark_pick(&birch, &id, "fastest", 3, 3, &random(5))
        .unwrap();
    assert_eq!(undo_label(&mut s), "Unmark pick");
    let m = s.materials().find(&birch).unwrap();
    assert!(
        m.pick_set(&laser, kind, curvyo_laser_core::settings::PickKind::Fastest)
            .is_none()
    );
    let body = s.materials_view().unwrap().unfolded.unwrap();
    assert_eq!(body.lasers[0].sets[0].other_picks, []);
    assert_eq!(body.test.unwrap().picks.len(), 1);
    assert!(
        s.mark_pick(&birch, &id, "cleanest", 0, 0, &random(6))
            .is_err()
    );
}

fn stored_of(s: &Session, op: curvyo_document_core::NodeId) -> serde_json::Value {
    s.document
        .operations(s.active_manufacturing().unwrap().id)
        .unwrap()
        .into_iter()
        .find(|o| o.id == op)
        .unwrap()
        .settings
        .unwrap()
}

/// `specs/0146` criteria 13 to 15: Use pick links an operation to a pick in one step "Use pick",
/// the link follows a re-judged pick, a typed number keeps pick and link, a press on the pressed
/// pick clears the typed number, an operation stored without a pick follows Best, and a pick the
/// material lacks is skipped with a notice of at most 70 characters.
#[test]
fn criteria_13_to_15_an_operation_follows_the_pick_it_was_given() {
    let (mut s, birch, group) = birch_grid();
    let test = recipe_of(&s, group).id.to_string();
    s.mark_pick(&birch, &test, "best", 2, 1, &random(3))
        .unwrap();
    s.mark_pick(&birch, &test, "fastest", 3, 3, &random(4))
        .unwrap();
    let cut = s.add_test_operation("cut").unwrap();
    assert!(s.use_material_on(&[cut], &birch));
    assert!(
        stored_of(&s, cut)["material"].get("pick").is_none(),
        "Best is never written"
    );
    assert_eq!(stored_of(&s, cut)["main"]["power_pct"], 60.0);
    let steps = s.undo_len();
    assert!(s.use_pick_on(&[cut], "fastest"));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Use pick");
    let _ = s.redo();
    assert_eq!(stored_of(&s, cut)["material"]["pick"], "fastest");
    assert_eq!(stored_of(&s, cut)["main"]["power_pct"], 80.0);
    // Criterion 14: re-judging the pick moves the linked operation in the same step.
    s.mark_pick(&birch, &test, "fastest", 0, 0, &random(5))
        .unwrap();
    assert_eq!(stored_of(&s, cut)["main"]["power_pct"], 20.0);
    // A typed number keeps the link and the pick; the same pick pressed again clears it.
    assert_eq!(
        s.edit_operation_setting(cut, "main", "power", 70.0),
        Ok(true)
    );
    assert_eq!(stored_of(&s, cut)["material"]["pick"], "fastest");
    assert_eq!(stored_of(&s, cut)["overrides"]["main"]["power_pct"], 70.0);
    assert!(s.use_pick_on(&[cut], "fastest"));
    assert!(stored_of(&s, cut).get("overrides").is_none());
    // Criterion 15: a pick the material lacks changes nothing and says so.
    assert!(!s.use_pick_on(&[cut], "deepest"));
    assert_eq!(
        s.operations_notice().as_deref(),
        Some("Cut 1 skipped: Birch 3 mm has no Deepest set.")
    );
    assert_eq!(stored_of(&s, cut)["material"]["pick"], "fastest");
    // Several rows: the one that can follow does, the other is named, the notice stays short.
    let other = s.add_test_operation("cut").unwrap();
    assert!(s.use_material_on(&[other], &birch));
    s.mark_pick(&birch, &test, "fastest", 0, 0, &random(6))
        .unwrap();
    assert!(s.use_pick_on(&[other, cut], "best"));
    assert!(stored_of(&s, cut)["material"].get("pick").is_none());
    assert!(
        s.operations_notice()
            .is_none_or(|n| n.chars().count() <= 70)
    );
}

/// Criterion 16: a derived cell operation of a test grid has no pick pick.
#[test]
fn criterion_16_a_derived_cell_row_takes_no_pick() {
    let (mut s, birch, group) = birch_grid();
    let derived = s.grid_object(group).unwrap().derived[0];
    assert!(!s.use_pick_on(&[derived], "best"));
    let _ = birch;
}

/// Criterion 8: the selected grid reports its marked cells for the canvas, a deselected one none.
#[test]
fn criterion_8_the_selected_grid_reports_its_picks_for_the_canvas() {
    let (mut s, birch, group) = birch_grid();
    let test = recipe_of(&s, group).id.to_string();
    assert!(s.pick_marks().is_empty(), "no pick, nothing to draw");
    s.mark_pick(&birch, &test, "best", 2, 1, &random(3))
        .unwrap();
    s.mark_pick(&birch, &test, "fastest", 2, 1, &random(4))
        .unwrap();
    s.mark_pick(&birch, &test, "deepest", 0, 0, &random(5))
        .unwrap();
    let marks = s.pick_marks();
    assert_eq!(marks.len(), 2);
    let kinds: Vec<Vec<&str>> = marks.iter().map(|m| m.kinds.clone()).collect();
    assert!(kinds.contains(&vec!["best", "fastest"]) && kinds.contains(&vec!["deepest"]));
    assert!(
        marks
            .iter()
            .all(|m| m.width > 0.0 && (m.pitch - m.width - m.gap).abs() < 1e-9)
    );
    s.selection.clear();
    assert!(
        s.pick_marks().is_empty(),
        "it disappears when the grid is not selected"
    );
}

/// `specs/0146` criterion 9: a kind with picks but no main set (its Best set deleted) lists its
/// pick lines under that kind's "+ Cut set" button.
#[test]
fn criterion_9_pick_lines_of_a_kind_without_a_main_set_sit_under_its_add_button() {
    let (mut s, birch, group) = birch_grid();
    let id = recipe_of(&s, group).id.to_string();
    s.unfold_material(Some(&birch));
    s.open_material_test(Some(&id));
    s.mark_pick(&birch, &id, "best", 2, 1, &random(3)).unwrap();
    s.mark_pick(&birch, &id, "fastest", 3, 3, &random(4))
        .unwrap();
    let laser = s.active_profile_id().unwrap();
    let body = s.materials_view().unwrap().unfolded.unwrap();
    assert!(
        body.lasers[0].missing_picks.is_empty(),
        "a main set holds them"
    );
    s.delete_material_set(&birch, &laser, "cut", &random(5))
        .unwrap();
    let body = s.materials_view().unwrap().unfolded.unwrap();
    let block = &body.lasers[0];
    assert!(block.sets.is_empty() && block.missing.contains(&SetKindView::Cut));
    let lines = &block.missing_picks;
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].kind, SetKindView::Cut);
    assert_eq!(lines[0].other_picks[0].pick, "fastest");
    assert_eq!(lines[0].other_picks[0].source, format!("Test {id}"));
}
