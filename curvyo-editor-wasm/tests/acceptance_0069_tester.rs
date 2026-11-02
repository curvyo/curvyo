//! Tester's acceptance tests for `specs/0069-tiling` in the session, written from the
//! specification before the implementation was read: the Tile, Edit tiling and Release flow
//! (criteria 1, 2, 26, 27, 32), refusals before the first write (4), the preview (34, 35), the
//! counts (36), history steps (37), source deletion (30) and export and save (33, 43).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use curvyo_document_core::{
    AnchorId, Document, Length, NewAnchor, NodeId, Point, SpacingMode, TilingLayout, TilingResult,
    pack,
};
use curvyo_editor_wasm::{KeyInput, Session, SvgExportResult, Tool};
use curvyo_ui_core::SvgExportOptions;
use curvyo_ui_core::tiling::{CountStatus, TileButton};

fn tri(d: &Document, x: f64, n: u64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, n), Point::new(x, 10.0)),
            NewAnchor::corner(AnchorId::new(1, n + 1), Point::new(x + 20.0, 10.0)),
            NewAnchor::corner(AnchorId::new(1, n + 2), Point::new(x + 20.0, 20.0)),
        ],
        true,
    )
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 900.0);
    select_all(&mut s);
    s
}

fn select_all(s: &mut Session) {
    let _ = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
}

fn one_path() -> Session {
    let d = Document::new(1);
    tri(&d, 10.0, 1);
    open(&d)
}

fn bytes(s: &Session) -> Vec<u8> {
    s.pack("t").unwrap()
}

fn svg(s: &Session) -> String {
    match s.export_svg(SvgExportOptions::default()) {
        SvgExportResult::Written { text, .. } => text,
        SvgExportResult::NothingSelected => panic!("nothing"),
    }
}

fn newest_row(s: &mut Session) -> String {
    s.history_rows_json(0, 1)
}

fn key(s: &mut Session, k: &str) {
    let _ = s.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    });
}

#[test]
fn c01_entering_shows_the_defaults_and_writes_nothing() {
    let mut s = one_path();
    let before = bytes(&s);
    let steps = s.history_len();
    assert!(matches!(s.tile_button(), TileButton::Tile(_)));
    assert!(s.tiling_enter(1234));
    let r = s.tiling_recipe().unwrap();
    assert_eq!(r.layout, TilingLayout::Grid);
    assert_eq!((r.grid.columns, r.grid.rows), (5, 4));
    assert_eq!(r.seed, 1234, "the platform's seed");
    assert_eq!(r.result, TilingResult::Clones);
    assert!(r.keep_source);
    assert_eq!(r.circle.count, 12);
    let st = s.tiling_status().unwrap();
    assert_eq!(st.count, CountStatus::Fine(20));
    assert!(!st.reopened);
    let _ = s.draw_list();
    assert_eq!(s.history_len(), steps);
    assert!(!s.is_dirty());
    assert_eq!(bytes(&s), before, "nothing written until Apply");
}

#[test]
fn c02_cancel_and_escape_leave_the_document_byte_identical() {
    let mut s = one_path();
    let before = bytes(&s);
    let steps = s.history_len();
    s.tiling_enter(5);
    let mut r = s.tiling_recipe().unwrap();
    r.grid.columns = 9;
    r.variance.rotation.random = curvyo_document_core::Angle::from_radians(0.3);
    assert!(s.tiling_set_recipe(r));
    let _ = s.draw_list();
    s.tiling_cancel();
    assert!(!s.tiling_open());
    assert_eq!(s.history_len(), steps);
    assert_eq!(bytes(&s), before);
    // Escape as a key while the bar is open writes nothing either.
    s.tiling_enter(5);
    key(&mut s, "Escape");
    s.tiling_cancel();
    assert_eq!(bytes(&s), before);
    assert_eq!(s.history_len(), steps);
}

#[test]
fn c02_c37_apply_is_one_step_tile_and_undo_redo_are_one_step_each() {
    let mut s = one_path();
    let steps = s.history_len();
    s.tiling_enter(5);
    let mut r = s.tiling_recipe().unwrap();
    // Many option changes before Apply.
    for c in 2..8 {
        r.grid.columns = c;
        assert!(s.tiling_set_recipe(r));
    }
    assert!(s.tiling_apply());
    assert!(!s.tiling_open());
    assert_eq!(s.history_len(), steps + 1);
    assert!(newest_row(&mut s).contains("\"Tile\"") || newest_row(&mut s).contains("Tile"));
    assert!(matches!(s.tile_button(), TileButton::Edit(_)));
    let tiled = svg(&s);
    assert!(s.undo().changed);
    select_all(&mut s);
    assert!(
        matches!(s.tile_button(), TileButton::Tile(_)),
        "undo removed the tiling"
    );
    assert!(s.redo().changed);
    assert_eq!(svg(&s), tiled);
}

#[test]
fn c03_tile_disabled_with_nothing_or_tiling_plus_others() {
    let d = Document::new(1);
    tri(&d, 10.0, 1);
    let mut s = Session::open(2, &pack(&d, "t").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    assert_eq!(s.tile_button(), TileButton::NothingSelected);
    assert!(!s.tiling_enter(1));
    assert!(!s.tiling_open());
    select_all(&mut s);
    s.tiling_enter(1);
    s.tiling_apply();
    // Add another object and select everything.
    let d2 = curvyo_document_core::unpack(3, &bytes(&s)).unwrap();
    tri(&d2, 300.0, 50);
    s = open(&d2);
    assert_eq!(s.tile_button(), TileButton::TilingWithOthers);
    assert!(!s.tiling_enter(1));
}

#[test]
fn c04_a_refusal_keeps_the_bar_open_and_writes_nothing() {
    let d = Document::new(1);
    let mut top = tri(&d, 10.0, 1);
    for _ in 0..curvyo_document_core::MAX_CLONE_DEPTH {
        top = d.linked_clone(&[top], &[]).unwrap()[0];
    }
    let mut s = open(&d);
    let before = bytes(&s);
    let steps = s.history_len();
    assert!(s.tiling_enter(3));
    assert!(!s.tiling_apply(), "refused");
    assert!(s.tiling_open(), "the bar stays open");
    let refusal = s.tiling_status().unwrap().refusal;
    assert!(
        refusal.as_deref().is_some_and(|t| t.contains("clone")),
        "notice names the reason: {refusal:?}"
    );
    assert_eq!(s.history_len(), steps);
    assert_eq!(bytes(&s), before);
}

#[test]
fn c26_c27_reopen_shows_stored_values_and_edit_is_one_step() {
    let mut s = one_path();
    s.tiling_enter(77);
    let mut r = s.tiling_recipe().unwrap();
    r.result = TilingResult::Copies;
    r.grid.columns = 3;
    r.grid.rows = 2;
    s.tiling_set_recipe(r);
    s.tiling_apply();
    let before = bytes(&s);
    let steps = s.history_len();
    assert!(s.tiling_enter(5), "Edit tiling on a copies tiling");
    let stored = s.tiling_recipe().unwrap();
    assert_eq!(stored, r, "stored values, stored seed");
    assert!(s.tiling_status().unwrap().reopened);
    let _ = s.draw_list();
    assert_eq!(bytes(&s), before, "opening writes nothing");
    // Unchanged Apply: nothing written.
    assert!(s.tiling_apply());
    assert_eq!(s.history_len(), steps);
    assert_eq!(bytes(&s), before);
    // Change and apply: one step "Edit tiling".
    s.tiling_enter(5);
    let mut wider = stored;
    wider.grid.columns = 4;
    wider.layout = TilingLayout::Circle;
    s.tiling_set_recipe(wider);
    s.tiling_apply();
    assert_eq!(s.history_len(), steps + 1);
    assert!(newest_row(&mut s).contains("Edit tiling"));
    s.tiling_enter(5);
    let back = s.tiling_recipe().unwrap();
    assert_eq!(back.seed, 77, "seed never changed by Apply or reopen");
    assert_eq!(
        back.grid.columns, 4,
        "grid values kept while circle is used"
    );
    s.tiling_cancel();
}

#[test]
fn c07_switching_spacing_mode_keeps_the_preview_still() {
    let mut s = one_path();
    s.tiling_enter(2);
    let gap = s.tiling_recipe().unwrap();
    let preview = format!("{:?}", s.draw_list().triangles.len());
    assert!(s.tiling_set_spacing_mode(SpacingMode::Pitch));
    let pitch = s.tiling_recipe().unwrap();
    assert_eq!(pitch.grid.spacing_mode, SpacingMode::Pitch);
    // Box 20 x 10, gap 5: pitch 25 x 15.
    assert!((pitch.grid.spacing_x.as_mm() - 25.0).abs() < 1e-9);
    assert!((pitch.grid.spacing_y.as_mm() - 15.0).abs() < 1e-9);
    assert_eq!(format!("{:?}", s.draw_list().triangles.len()), preview);
    assert!(s.tiling_set_spacing_mode(SpacingMode::Gap));
    assert_eq!(
        s.tiling_recipe().unwrap().grid.spacing_x,
        gap.grid.spacing_x
    );
}

#[test]
fn c30_a_deleted_source_offers_only_release() {
    // Tile with the source left out, then delete the source.
    let mut s = one_path();
    s.tiling_enter(4);
    let mut r = s.tiling_recipe().unwrap();
    r.keep_source = false;
    r.grid.columns = 2;
    r.grid.rows = 1;
    s.tiling_set_recipe(r);
    s.tiling_apply();
    let doc = curvyo_document_core::unpack(3, &bytes(&s)).unwrap();
    let source = doc
        .children(None)
        .into_iter()
        .flat_map(|l| {
            let mut v = vec![l];
            v.extend(doc.children(Some(l)));
            v
        })
        .find(|id| {
            !doc.is_group(*id) && doc.tiling(*id).is_none() && doc.read_objects().get(*id).is_some()
        })
        .expect("the source outside the tiling");
    doc.delete_objects(&[source]).unwrap();
    let mut s = open(&doc);
    assert!(matches!(s.tile_button(), TileButton::Edit(_)));
    assert!(!s.tiling_enter(1), "reopen refused");
    assert!(!s.tiling_open());
    assert!(s.tiling_release(), "Release still offered");
}

#[test]
fn c32_release_is_one_step_and_the_selection_stays_on_the_group() {
    let mut s = one_path();
    s.tiling_enter(3);
    s.tiling_apply();
    let selected = s.selected_object_count();
    let steps = s.history_len();
    let drawn = svg(&s);
    assert!(s.tiling_release());
    assert_eq!(s.history_len(), steps + 1);
    assert!(newest_row(&mut s).contains("Release tiling"));
    assert_eq!(s.selected_object_count(), selected, "selection stays");
    assert!(matches!(s.tile_button(), TileButton::Tile(_)));
    assert_eq!(svg(&s), drawn, "exactly as drawn");
    assert!(s.undo().changed);
    select_all(&mut s);
    assert!(
        matches!(s.tile_button(), TileButton::Edit(_)),
        "undo restores the tiling"
    );
}

#[test]
fn c33_svg_of_a_tiling_equals_svg_after_release_and_survives_save() {
    let mut s = one_path();
    s.tiling_enter(42);
    let mut r = s.tiling_recipe().unwrap();
    r.variance.rotation.random = curvyo_document_core::Angle::from_radians(0.4);
    r.variance.hue.per_column = curvyo_document_core::Angle::from_radians(0.5);
    r.variance.opacity.random = curvyo_document_core::Percent::new(30.0);
    r.grid.mirror_rows = true;
    r.grid.offset = curvyo_document_core::OffsetLines::Rows;
    s.tiling_set_recipe(r);
    s.tiling_apply();
    let tiled = svg(&s);
    assert!(!tiled.contains("tiling"));
    // Save and reopen reproduce the instances.
    let mut reopened = Session::open(7, &bytes(&s)).unwrap();
    assert_eq!(svg(&reopened), tiled);
    reopened.set_tool(Tool::Select);
    select_all(&mut reopened);
    assert!(reopened.tiling_enter(1));
    assert_eq!(reopened.tiling_recipe().unwrap(), r);
    reopened.tiling_cancel();
    s.tiling_release();
    assert_eq!(svg(&s), tiled);
}

#[test]
fn c34_an_invalid_entry_keeps_the_last_valid_recipe() {
    let mut s = one_path();
    s.tiling_enter(1);
    let good = s.tiling_recipe().unwrap();
    for bad in [
        {
            let mut r = good;
            r.grid.columns = 0;
            r
        },
        {
            let mut r = good;
            r.circle.radius = Length::from_mm(-2.0);
            r
        },
        {
            let mut r = good;
            r.grid.spacing_x = Length::from_mm(f64::NAN);
            r
        },
        {
            let mut r = good;
            r.variance.rotation.random = curvyo_document_core::Angle::from_radians(-0.1);
            r
        },
    ] {
        assert!(!s.tiling_set_recipe(bad));
        assert_eq!(s.tiling_recipe().unwrap(), good);
    }
    assert!(s.tiling_open());
}

#[test]
fn c34_the_preview_changes_with_options_and_disappears_on_cancel() {
    let mut s = one_path();
    let plain = s.draw_list().triangles.len();
    s.tiling_enter(1);
    // The baseline is taken with the bar open: it draws no transform handles (UX review §11
    // finding 2), so a count against the plain selection would not measure the preview.
    let mut r = s.tiling_recipe().unwrap();
    r.grid.columns = 1;
    r.grid.rows = 1;
    s.tiling_set_recipe(r);
    let one_by_one = s.draw_list().triangles.len();
    r.grid.columns = 5;
    r.grid.rows = 4;
    s.tiling_set_recipe(r);
    let five_by_four = s.draw_list().triangles.len();
    assert!(five_by_four > one_by_one, "preview drawn");
    r.grid.columns = 10;
    s.tiling_set_recipe(r);
    assert!(
        s.draw_list().triangles.len() > five_by_four,
        "more instances drawn"
    );
    s.tiling_cancel();
    assert_eq!(s.draw_list().triangles.len(), plain);
}

#[test]
fn c35_undo_is_locked_and_switching_tool_cancels() {
    let mut s = one_path();
    s.tiling_enter(1);
    let before = bytes(&s);
    assert!(!s.undo().changed);
    assert!(!s.redo().changed);
    key(&mut s, "z");
    assert!(s.tiling_open());
    assert_eq!(bytes(&s), before);
    s.set_tool(Tool::Pen);
    assert!(!s.tiling_open());
    assert_eq!(bytes(&s), before);
}

#[test]
fn c36_above_the_limit_apply_is_disabled_and_writes_nothing() {
    let mut s = one_path();
    s.tiling_enter(1);
    let mut r = s.tiling_recipe().unwrap();
    r.grid.columns = 100_001;
    r.grid.rows = 1;
    assert!(s.tiling_set_recipe(r), "the bar shows it");
    assert_eq!(
        s.tiling_status().unwrap().count,
        CountStatus::Refused(100_001)
    );
    let before = bytes(&s);
    let steps = s.history_len();
    assert!(!s.tiling_apply());
    assert_eq!(s.history_len(), steps);
    assert_eq!(bytes(&s), before);
    r.grid.columns = 20_000;
    assert!(s.tiling_set_recipe(r));
    assert_eq!(
        s.tiling_status().unwrap().count,
        CountStatus::Simplified(20_000)
    );
    let _ = s.draw_list();
    assert_eq!(bytes(&s), before, "a large preview writes nothing");
}

#[test]
fn c38_moving_the_tiling_keeps_the_stored_layout_numbers() {
    let mut s = one_path();
    s.tiling_enter(9);
    s.tiling_apply();
    let _ = s.key_down(KeyInput {
        key: "ArrowRight",
        ..KeyInput::default()
    });
    s.tiling_enter(1);
    let r = s.tiling_recipe().unwrap();
    assert_eq!((r.grid.columns, r.grid.rows), (5, 4));
    assert_eq!(r.seed, 9);
    let before = bytes(&s);
    assert!(s.tiling_apply());
    assert_eq!(
        bytes(&s),
        before,
        "after a move, an unchanged Apply writes nothing"
    );
}
