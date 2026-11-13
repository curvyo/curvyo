//! Session tests of the Materials tab (`specs/0112` criteria 16 to 27): materials and sets per
//! laser as steps, the test record that follows a grid, judging, note, photo bytes, deleting with
//! retired ids, the journal's undo and redo with photo bytes, the per-laser tie.

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
    assert_eq!(body.lasers[0].missing.len(), 1, "Score is missing");
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
    s.judge_test(&birch, &id, 2, 1, &random(3)).unwrap();
    assert_eq!(undo_label(&mut s), "Choose best cell");
    let laser = s.active_profile_id().unwrap();
    let m = s.materials().find(&birch).unwrap();
    let set = m
        .set_for(&laser, curvyo_laser_core::settings::SetKind::Cut)
        .unwrap();
    assert_eq!(set.numbers.settings.power.percent(), 60.0);
    let detail = s.materials_view().unwrap().unfolded.unwrap().test.unwrap();
    assert_eq!(detail.best.unwrap().column, 2);
    assert!(detail.set_line.unwrap().starts_with("Cut set of "));
    assert!(detail.place.on_page);
    assert_eq!(
        s.judge_test(&birch, &id, 2, 1, &random(4)),
        Err("Nothing changed.".to_owned()),
        "a tap on the best cell (G14)"
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
fn the_journal_counts_photo_bytes_and_drops_the_oldest_past_its_limit() {
    let (mut s, birch, group) = birch_grid();
    let id = recipe_of(&s, group).id.to_string();
    s.material_journal().set_limit(4_000);
    s.set_test_photo(&birch, &id, &png(600, 400), false)
        .unwrap();
    s.set_test_note(&birch, &id, "a").unwrap();
    assert!(s.material_journal().bytes() <= 4_000);
}

#[test]
fn a_material_record_over_512_kib_is_refused_with_its_notice() {
    let mut s = laser_with_fonts();
    let id = s.add_material("Birch", 3.0, &random(1)).unwrap();
    let mut change = super::LibraryChange::default();
    let refused = s.change_record(&mut change, &id, |m| {
        m.name = "Birch".to_owned();
        for i in 0..5_000 {
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
        Err("Could not save Birch: it is over 512 KiB.".to_owned())
    );
    assert!(s.materials().text(&id).unwrap().len() < MAX_RECORD_TEXT);
    assert!(matches!(s.materials().reads()[0], MaterialRead::Loaded(_)));
}
