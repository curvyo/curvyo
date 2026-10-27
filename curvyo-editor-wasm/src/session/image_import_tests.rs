//! The Import button's session side (`specs/0058-bitmap-import` criteria 2, 5, 15, 26, 30, 37,
//! 45).

use curvyo_document_core::{AssetHash, ObjectSnapshot};

use super::{Session, Tool};

const PNG: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");
const JPEG: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/jpeg_progressive.jpg");
const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="40mm" height="40mm" viewBox="0 0 40 40"><rect x="5" y="5" width="10" height="10"/></svg>"#;

fn import(session: &mut Session, files: &[(&[u8], &str)]) -> super::ImportOutcome {
    session.import_begin(None);
    for (bytes, name) in files {
        session.import_add(bytes, name);
    }
    session.import_finish(false)
}

fn images(session: &Session) -> Vec<curvyo_document_core::ImageSnapshot> {
    session
        .objects()
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn ac2_ac15_one_image_is_one_step_selected_with_the_select_tool() {
    let mut session = Session::new(1);
    session.set_tool(Tool::Rectangle);
    let outcome = import(&mut session, &[(PNG, "/home/maker/photo.png")]);
    assert_eq!((outcome.imported, outcome.alert), (1, false));
    assert_eq!(outcome.notice, "");
    assert_eq!(session.document.own_steps().len(), 1);
    assert_eq!(session.document.own_steps()[0].label, "import_image");
    assert_eq!(session.tool, Tool::Select);
    let image = &images(&session)[0];
    assert_eq!(session.selection.ids(), &[image.id]);
    assert_eq!(image.asset, AssetHash::of(PNG));
}

#[test]
fn ac26_a_refused_file_changes_nothing_and_names_itself() {
    let mut session = Session::new(1);
    let outcome = import(&mut session, &[(b"%PDF-1.7", "scan.pdf")]);
    assert_eq!((outcome.imported, outcome.alert), (0, true));
    assert_eq!(
        outcome.notice,
        "scan.pdf\nThis file type isn't supported. Use SVG, PNG, JPEG or WebP."
    );
    assert_eq!(session.document.own_steps().len(), 0);
    assert!(session.objects().is_empty());
}

#[test]
fn ac5_a_mixed_batch_is_one_step_named_import_with_the_failures_counted() {
    let mut session = Session::new(1);
    let outcome = import(
        &mut session,
        &[
            (PNG, "a.png"),
            (SVG, "b.svg"),
            (b"nope", "c.txt"),
            (JPEG, "d.jpg"),
        ],
    );
    assert_eq!(outcome.imported, 3);
    assert_eq!(
        outcome.notice,
        "1 file could not be imported.\nThis file type isn't supported. Use SVG, PNG, JPEG or WebP."
    );
    let steps = session.document.own_steps();
    assert_eq!((steps.len(), steps[0].label.as_str()), (1, "import"));
    let placed = images(&session);
    assert_eq!(placed.len(), 2);
    let step = curvyo_ui_core::STAGGER_MM * 2.0;
    let centre =
        |i: &curvyo_document_core::ImageSnapshot| f64::midpoint(i.corners()[0].x, i.corners()[2].x);
    assert!((centre(&placed[1]) - centre(&placed[0]) - step).abs() < 1e-6);
}

#[test]
fn ac45_undo_removes_the_import_and_redo_needs_no_decode_from_disk() {
    let mut session = Session::new(1);
    import(&mut session, &[(PNG, "a.png")]);
    assert_eq!(session.undo().notice, "Undid: Import image (1 image).");
    assert_eq!(images(&session).len(), 0);
    assert_eq!(session.redo().notice, "Redid: Import image (1 image).");
    assert_eq!(images(&session).len(), 1);
    assert!(session.document.asset(&AssetHash::of(PNG)).is_some());
}

#[test]
fn ac37_outline_commands_refuse_a_selection_with_an_image() {
    let mut session = Session::new(1);
    import(&mut session, &[(PNG, "a.png")]);
    assert!(session.refuse_images());
    assert!(!session.linked_clone_shown(), "criterion 38");
    assert!(!session.select_bar_state().object_to_path);
    session.selection.set(&[]);
    assert!(!session.refuse_images());
}

#[test]
fn ac30_an_opened_project_decodes_on_first_draw_and_a_bad_asset_is_told_once() {
    let mut session = Session::new(1);
    import(&mut session, &[(PNG, "a.png")]);
    let bytes = session.pack("t").unwrap();
    let mut opened = Session::open(2, &bytes).unwrap();
    let mut list = opened.frame_draw_list();
    let pictures = opened.frame_images(&mut list);
    assert_eq!(pictures.len(), 1);
    assert_eq!((pictures[0].1.width, pictures[0].1.height), (3, 1));
    assert_eq!(opened.take_bad_asset_notice(), "");

    let mut broken = Session::new(3);
    import(&mut broken, &[(b"\x89PNG\r\n\x1a\nbroken", "b.png")]);
    assert!(images(&broken).is_empty(), "a damaged file is not imported");
    let ids = broken
        .document
        .add_images(&[curvyo_document_core::NewImage {
            bytes: std::sync::Arc::from(&b"\x89PNG\r\n\x1a\nbroken"[..]),
            placement: curvyo_document_core::Affine::new(10.0, 0.0, 0.0, 10.0, 0.0, 0.0),
            name: String::new(),
        }])
        .unwrap();
    assert_eq!(ids.len(), 1);
    let mut list = broken.frame_draw_list();
    let before = list.triangles.len();
    assert_eq!(broken.frame_images(&mut list).len(), 0);
    assert!(
        list.triangles.len() > before,
        "the frame and its cross are drawn"
    );
    assert_eq!(
        broken.take_bad_asset_notice(),
        "An image in this project could not be read."
    );
    let mut list = broken.frame_draw_list();
    let _ = broken.frame_images(&mut list);
    assert_eq!(broken.take_bad_asset_notice(), "", "once per open");
}

#[test]
fn ac3_ctrl_i_imports_unless_a_key_repeats_or_the_page_blocks_it() {
    let session = Session::new(1);
    let press = |repeat: bool, dom_blocked: bool| super::keys::KeyInput {
        key: "i",
        ctrl: true,
        repeat,
        dom_blocked,
        ..super::keys::KeyInput::default()
    };
    assert!(session.import_chord(&press(false, false)));
    assert!(!session.import_chord(&press(true, false)));
    assert!(!session.import_chord(&press(false, true)));
}
