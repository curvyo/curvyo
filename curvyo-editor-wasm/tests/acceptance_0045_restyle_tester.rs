//! Independent tester cases for the 2026-11-10 follow-up of
//! `0045-document-formats-library` at the `Session` level: the view flags of the
//! restyled list (criteria 35 to 38), formats of one size and the one current
//! format (criterion 39), no orientation in the form (criterion 42), inline
//! rename of formats and groups (criterion 43), no success notices
//! (criteria 20, 27 amended). Written from `specification.md` and `ux-notes.md`
//! before the implementation diff was read.

#![allow(
    unused_must_use,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    missing_docs
)]

use curvyo_editor_wasm::{FormTexts, Session, SizeOutcome};
use curvyo_ui_core::{FormField, FormatGroupView, FormatListView, FormatRowView};

fn form(name: &str, w: &str, h: &str, group_id: &str) -> FormTexts {
    FormTexts {
        name: name.to_string(),
        width: w.to_string(),
        height: h.to_string(),
        unit: "mm".to_string(),
        group_id: group_id.to_string(),
        new_group_name: String::new(),
        favourite: true,
    }
}

fn form_new_group(name: &str, w: &str, h: &str, group: &str) -> FormTexts {
    FormTexts {
        new_group_name: group.to_string(),
        ..form(name, w, h, "")
    }
}

fn row<'a>(v: &'a FormatListView, id: &str) -> &'a FormatRowView {
    v.groups
        .iter()
        .flat_map(|g| g.rows.iter())
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id}"))
}

fn group<'a>(v: &'a FormatListView, id: &str) -> &'a FormatGroupView {
    v.groups
        .iter()
        .find(|g| g.id == id)
        .unwrap_or_else(|| panic!("no group {id}"))
}

fn selected_ids(v: &FormatListView) -> Vec<String> {
    v.groups
        .iter()
        .flat_map(|g| g.rows.iter())
        .filter(|r| r.selected)
        .map(|r| r.id.clone())
        .collect()
}

fn pressed_names(s: &Session) -> Vec<String> {
    s.document_presets_view()
        .groups
        .iter()
        .flat_map(|g| g.entries.iter())
        .filter(|e| e.pressed)
        .map(|e| e.name.clone())
        .collect()
}

const LASER: &str = r#"format = 1
favourites = ["a0", "a1", "a2", "a3", "a4", "a5", "a6", "u-key-ring", "u-coaster"]

[[group]]
id = "u-laser"
name = "Laser"
default_orientation = "portrait"

  [[group.preset]]
  id = "u-key-ring"
  name = "Key ring"
  short_side = 90
  long_side = 90
  unit = "mm"

  [[group.preset]]
  id = "u-coaster"
  name = "Coaster"
  short_side = 90
  long_side = 90
  unit = "mm"
"#;

fn laser() -> Session {
    let mut s = Session::new(1);
    s.load_formats(LASER).unwrap();
    s
}

// ---- criteria 35 to 38: the view flags ---------------------------------------

#[test]
fn c35_built_in_rows_and_headers_have_no_edit_trash_or_rename() {
    let s = laser();
    let v = s.format_list();
    let paper = group(&v, "paper");
    assert!(!paper.can_edit);
    assert!(paper.rows.iter().all(|r| !r.can_edit));
    let laser = group(&v, "u-laser");
    assert!(laser.can_edit);
    assert!(laser.rows.iter().all(|r| r.can_edit));
}

#[test]
fn c36_c37_armed_and_idle_names_follow_the_ux_notes() {
    let s = laser();
    let v = s.format_list();
    let r = row(&v, "u-key-ring");
    assert_eq!(r.delete_name, "Delete Key ring");
    assert_eq!(r.armed_name, "Press again to delete Key ring.");
    assert_eq!(r.rename_name, "Name of Key ring");
    assert_eq!(r.edit_name, "Edit Key ring");
    let g = group(&v, "u-laser");
    assert_eq!(g.delete_name, "Delete group Laser");
    assert_eq!(
        g.armed_name,
        "Press again to delete group Laser. Its 2 formats go too."
    );
    assert_eq!(g.rename_name, "Name of group Laser");
}

#[test]
fn c37_armed_group_name_singular_and_empty() {
    let mut s = laser();
    assert!(s.delete_format("u-coaster").ok);
    assert_eq!(
        group(&s.format_list(), "u-laser").armed_name,
        "Press again to delete group Laser. Its 1 format goes too."
    );
    assert!(s.delete_format("u-key-ring").ok);
    let v = s.format_list();
    assert_eq!(
        group(&v, "u-laser").armed_name,
        "Press again to delete group Laser."
    );
}

#[test]
fn c36_c37_a_delete_has_no_notice_and_removes_the_favourite() {
    let mut s = laser();
    let e = s.delete_format("u-key-ring");
    assert!(e.ok && e.notice.is_empty(), "{e:?}");
    assert!(!e.file_text.contains("u-key-ring"), "{}", e.file_text);
    let e = s.delete_format_group("u-laser");
    assert!(e.ok && e.notice.is_empty(), "{e:?}");
    assert!(s.format_list().groups.iter().all(|g| g.id != "u-laser"));
}

#[test]
fn c38_export_exists_only_with_a_user_format_and_nothing_with_a_broken_file() {
    let mut s = Session::new(1);
    let v = s.format_list();
    assert!(v.can_add && !v.can_export);
    let e = s.add_format(&form_new_group("Key ring", "30", "50", "Laser"));
    assert!(e.ok, "{e:?}");
    assert!(s.format_list().can_export);
    s.mark_formats_broken("bad");
    let v = s.format_list();
    assert!(!v.can_add && !v.can_export);
    assert!(v.groups.iter().all(|g| !g.can_edit && !g.can_toggle));
}

// ---- criterion 20 / 27 amended: no success notice ---------------------------

#[test]
fn c20_c27_successes_have_no_notice_failures_have_one() {
    let mut s = Session::new(1);
    let e = s.add_format(&form_new_group("Key ring", "30", "50", "Laser"));
    assert!(e.ok && e.notice.is_empty(), "{e:?}");
    let e = s.import_formats(LASER, "laser.toml");
    assert!(e.ok && e.notice.is_empty(), "{e:?}");
    let e = s.import_formats("format = 1\n[[group]]\nid=", "bad.toml");
    assert!(!e.ok);
    assert!(
        e.notice.starts_with("Could not import bad.toml:")
            && e.notice.ends_with("Nothing was changed."),
        "{}",
        e.notice
    );
}

// ---- criterion 39: one size twice -------------------------------------------

#[test]
fn c39_add_two_of_one_size_both_survive_save_and_reload() {
    let mut s = Session::new(1);
    let a = s.add_format(&form_new_group("Key ring", "90", "90", "Laser"));
    assert!(a.ok, "{a:?}");
    let gid = s.format_list().groups.last().unwrap().id.clone();
    let b = s.add_format(&form("Coaster", "90", "90", &gid));
    assert!(b.ok && b.errors.is_empty(), "{b:?}");
    let mut t = Session::new(2);
    t.load_formats(&b.file_text).unwrap();
    let names: Vec<_> = group(&t.format_list(), &gid)
        .rows
        .iter()
        .map(|r| r.name.clone())
        .collect();
    assert_eq!(names, ["Key ring", "Coaster"]);
}

#[test]
fn c39_add_of_a_built_in_size_and_edit_to_another_size_are_accepted() {
    let mut s = laser();
    let e = s.add_format(&form("My A4", "297", "210", "u-laser"));
    assert!(e.ok, "{e:?}");
    let e = s.save_format("u-coaster", &form("Coaster", "210", "297", "u-laser"));
    assert!(e.ok, "{e:?}");
    // The name rule stays.
    let e = s.add_format(&form("coaster", "10", "10", "u-laser"));
    assert!(!e.ok);
    assert_eq!(e.errors[0].field, FormField::Name);
    assert_eq!(e.errors[0].message, "This group already has Coaster");
    assert!(
        e.errors.iter().all(|x| !x.message.starts_with("Same size")),
        "{e:?}"
    );
}

#[test]
fn c39_only_the_first_match_is_selected_named_and_pressed() {
    let mut s = laser();
    // Apply the second one; the first in list order is the current format.
    assert_eq!(s.apply_document_preset("u-coaster"), SizeOutcome::Committed);
    let v = s.format_list();
    assert_eq!(selected_ids(&v), ["u-key-ring"]);
    assert!(
        s.document_presets_view().subject.starts_with("Key ring"),
        "{}",
        s.document_presets_view().subject
    );
    assert_eq!(pressed_names(&s), ["Key ring"]);
}

#[test]
fn c39_c11_first_match_not_a_favourite_means_no_pressed_cell() {
    let mut s = laser();
    s.set_format_favourite("u-key-ring", false);
    s.apply_document_preset("u-coaster");
    assert_eq!(selected_ids(&s.format_list()), ["u-key-ring"]);
    assert!(pressed_names(&s).is_empty(), "{:?}", pressed_names(&s));
    assert!(s.document_presets_view().subject.starts_with("Key ring"));
}

#[test]
fn c39_a_built_in_first_wins_over_a_user_twin() {
    let mut s = laser();
    s.add_format(&form("My A4", "210", "297", "u-laser"));
    assert_eq!(s.apply_document_preset("a3"), SizeOutcome::Committed);
    assert_eq!(s.apply_document_preset("a4"), SizeOutcome::Committed);
    assert_eq!(selected_ids(&s.format_list()), ["a4"]);
    assert_eq!(pressed_names(&s), ["A4"]);
    // Paper off: the user twin becomes current.
    s.set_format_group_enabled("paper", false);
    let v = s.format_list();
    let sel = selected_ids(&v);
    assert_eq!(sel.len(), 1, "{sel:?}");
    assert_eq!(row(&v, &sel[0]).name, "My A4");
}

#[test]
fn c39_importing_the_same_file_twice_adds_nothing() {
    let mut s = Session::new(1);
    assert!(s.import_formats(LASER, "l.toml").ok);
    let v = s.format_list();
    assert_eq!(group(&v, "u-laser").rows.len(), 2);
    let e = s.import_formats(LASER, "l.toml");
    assert!(e.ok && e.notice.is_empty());
    assert_eq!(s.format_list(), v);
}

// ---- criterion 42: no orientation in the form -------------------------------

#[test]
fn c42_a_group_made_from_the_form_is_portrait_in_the_file() {
    let mut s = Session::new(1);
    let e = s.add_format(&form_new_group("Board", "300", "200", "Kitchen"));
    assert!(e.ok, "{e:?}");
    assert!(
        e.file_text.contains("default_orientation = \"portrait\""),
        "{}",
        e.file_text
    );
}

#[test]
fn c42_existing_groups_keep_their_orientation_through_edits() {
    let land = LASER.replace("\"portrait\"", "\"landscape\"");
    let mut s = Session::new(1);
    s.load_formats(&land).unwrap();
    let e = s.add_format(&form("Tag", "20", "60", "u-laser"));
    assert!(e.file_text.contains("default_orientation = \"landscape\""));
    let e = s.rename_format_group("u-laser", "Laser cut");
    assert!(e.ok, "{e:?}");
    assert!(
        e.file_text.contains("default_orientation = \"landscape\""),
        "{}",
        e.file_text
    );
}

// ---- criterion 43: inline rename ---------------------------------------------

#[test]
fn c43_rename_with_enter_changes_list_quick_selection_and_file_keeps_id() {
    let mut s = laser();
    let e = s.rename_format("u-key-ring", "Keyring");
    assert!(e.ok && e.notice.is_empty(), "{e:?}");
    assert!(e.file_text.contains("id = \"u-key-ring\""));
    assert!(e.file_text.contains("name = \"Keyring\""));
    assert_eq!(row(&s.format_list(), "u-key-ring").name, "Keyring");
    let cells: Vec<String> = s
        .document_presets_view()
        .groups
        .iter()
        .flat_map(|g| g.entries.iter().map(|x| x.name.clone()))
        .collect();
    assert!(cells.contains(&"Keyring".to_string()), "{cells:?}");
    assert!(!cells.contains(&"Key ring".to_string()), "{cells:?}");
    assert!(row(&s.format_list(), "u-key-ring").favourite);
}

#[test]
fn c43_rename_is_refused_for_a_name_the_group_has_and_for_bad_lengths() {
    let mut s = laser();
    let e = s.rename_format("u-key-ring", "COASTER");
    assert!(!e.ok && e.file_text.is_empty());
    assert_eq!(e.errors[0].message, "This group already has Coaster");
    for bad in ["", "   ", "1234567890123456789012345"] {
        let e = s.rename_format("u-key-ring", bad);
        assert!(!e.ok, "{bad:?}");
        assert_eq!(e.errors[0].message, "Enter a name of 1 to 24 characters");
    }
    // 24 characters is fine, and a name with surrounding spaces is trimmed.
    assert!(s.rename_format("u-key-ring", "123456789012345678901234").ok);
    assert!(s.rename_format("u-key-ring", "  Ring  ").ok);
    assert_eq!(row(&s.format_list(), "u-key-ring").name, "Ring");
    assert_eq!(row(&s.format_list(), "u-coaster").name, "Coaster");
}

#[test]
fn c43_renaming_to_its_own_name_or_its_case_is_not_a_clash() {
    let mut s = laser();
    let e = s.rename_format("u-key-ring", "Key ring");
    assert!(e.errors.is_empty(), "{e:?}");
    let e = s.rename_format("u-key-ring", "key ring");
    assert!(e.errors.is_empty(), "{e:?}");
}

#[test]
fn c43_group_rename_validates_unique_ignoring_case_and_built_ins_refuse() {
    let mut s = laser();
    s.add_format(&form_new_group("Hoop", "100", "100", "Embroidery"));
    let e = s.rename_format_group("u-laser", "embroidery");
    assert!(!e.ok && e.file_text.is_empty(), "{e:?}");
    assert!(!e.errors.is_empty());
    let e = s.rename_format_group("u-laser", "PAPER");
    assert!(!e.ok, "a built-in group name is taken too");
    let e = s.rename_format_group("u-laser", "");
    assert!(!e.ok);
    assert!(!s.rename_format_group("paper", "Papers").ok);
    assert!(!s.rename_format("a4", "A 4").ok);
    let e = s.rename_format_group("u-laser", "Laser cut");
    assert!(e.ok && e.notice.is_empty());
    assert_eq!(group(&s.format_list(), "u-laser").name, "Laser cut");
    assert_eq!(
        group(&s.format_list(), "u-laser").rename_name,
        "Name of group Laser cut"
    );
}

#[test]
fn c43_rename_never_changes_the_document() {
    let mut s = laser();
    s.apply_document_preset("u-key-ring");
    let before = s.pack("0.1.0").unwrap();
    assert!(s.rename_format("u-key-ring", "Ring").ok);
    assert!(s.rename_format_group("u-laser", "Shop").ok);
    assert_eq!(s.pack("0.1.0").unwrap(), before);
}

#[test]
fn c43_c6_rename_is_refused_while_the_file_is_broken() {
    let mut s = laser();
    s.mark_formats_broken("bad");
    assert!(!s.rename_format("u-key-ring", "Ring").ok);
    assert!(!s.rename_format_group("u-laser", "Shop").ok);
}

#[test]
fn c39_c21_after_an_import_with_a_name_the_group_has_both_formats_stay_editable() {
    // Same group and name, other size: the import skips both (criteria 27, 39),
    // so no two formats of one name exist and Save on each still works.
    let mut s = laser();
    let other = LASER
        .replace("u-key-ring", "u-key-ring-big")
        .replace("u-coaster", "u-coaster-big")
        .replace(
            "short_side = 90\n  long_side = 90",
            "short_side = 95\n  long_side = 95",
        );
    assert!(s.import_formats(&other, "big.toml").ok);
    let ids: Vec<String> = group(&s.format_list(), "u-laser")
        .rows
        .iter()
        .map(|r| r.id.clone())
        .collect();
    assert_eq!(ids.len(), 2, "the import adds nothing: {ids:?}");
    for id in &ids {
        let r = row(&s.format_list(), id).clone();
        let (w, h) = if r.size_text.starts_with("95") {
            ("95", "95")
        } else {
            ("90", "90")
        };
        let e = s.save_format(id, &form(&r.name, w, h, "u-laser"));
        assert!(
            e.ok,
            "Save of {} ({id}) unchanged was refused: {:?}",
            r.name, e.errors
        );
    }
}
