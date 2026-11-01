//! Ctrl+V and Ctrl+C for images, session side (`specs/0087-image-clipboard-and-drop` criteria 1,
//! 2, 3, 5, 7, 9, 12, 13 and 16).

use curvyo_document_core::{ImageSnapshot, ObjectSnapshot, Point};
use curvyo_image_core::{WorkingImage, encode_png};
use curvyo_ui_core::FlipAxis;

use super::image_import::PASTED_IMAGE_NAME;
use super::{ImportOrigin, Session, Tool};

const PNG: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");
const PNG_300_DPI: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/png_phys_300dpi.png");
const DAMAGED: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/png_bad_crc.png");

fn session() -> Session {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session
}

fn paste(
    session: &mut Session,
    at: Option<(f64, f64)>,
    bytes: &[u8],
    name: &str,
) -> super::ImportOutcome {
    session.import_begin_from(ImportOrigin::Paste, at);
    session.import_add(bytes, name);
    session.import_finish(false)
}

fn images(session: &Session) -> Vec<ImageSnapshot> {
    session
        .objects()
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.clone()),
            _ => None,
        })
        .collect()
}

fn name_of(session: &Session, id: curvyo_document_core::NodeId) -> Option<String> {
    session.document.node_registers(id).and_then(|r| r.name)
}

fn centre(image: &ImageSnapshot) -> Point {
    image.placement.apply_point(Point::new(0.5, 0.5))
}

/// 40 x 20 px with a red 5 x 5 marker top-left, as a PNG.
fn marker_png() -> Vec<u8> {
    let pixels = (0..20u32)
        .flat_map(|y| (0..40u32).map(move |x| (x, y)))
        .flat_map(|(x, y)| {
            if x < 5 && y < 5 {
                [255, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            }
        })
        .collect();
    encode_png(&WorkingImage {
        width: 40,
        height: 20,
        pixels,
        original_size: (40, 20),
        truncated: false,
        animated: false,
    })
    .unwrap()
}

#[test]
fn ac1_a_paste_is_one_step_paste_image_named_pasted_image_and_selected() {
    let mut s = session();
    s.set_tool(Tool::Rectangle);
    let outcome = paste(&mut s, None, PNG, "");
    assert_eq!((outcome.imported, outcome.alert), (1, false));
    let steps = s.document.own_steps();
    assert_eq!((steps.len(), steps[0].label.as_str()), (1, "paste_image"));
    assert_eq!(curvyo_ui_core::operation_name("paste_image"), "Paste image");
    assert_eq!(s.tool, Tool::Select);
    let image = &images(&s)[0];
    assert_eq!(s.selection.ids(), &[image.id]);
    let name = name_of(&s, image.id);
    assert_eq!(name.as_deref(), Some(PASTED_IMAGE_NAME));
    // A second paste is "Pasted image" again.
    paste(&mut s, None, PNG, "  ");
    let second = &images(&s)[1];
    assert_eq!(name_of(&s, second.id).as_deref(), Some("Pasted image"));
    // Ctrl+Z removes it in one step.
    assert!(s.undo().changed);
    assert_eq!(images(&s).len(), 1);
}

#[test]
fn ac1_a_named_clipboard_file_keeps_its_name_and_the_size_follows_its_density() {
    let mut s = session();
    paste(&mut s, None, PNG_300_DPI, "/home/maker/scan.png");
    let image = &images(&s)[0];
    assert_eq!(name_of(&s, image.id).as_deref(), Some("scan.png"));
    // 30 x 20 px at 300 dpi: 2.54 x 1.69 mm.
    let corner = image.placement.apply_point(Point::new(1.0, 1.0));
    let origin = image.placement.apply_point(Point::new(0.0, 0.0));
    assert!((corner.x - origin.x - 30.0 / 300.0 * 25.4).abs() < 1e-3);
    assert!((corner.y - origin.y - 20.0 / 300.0 * 25.4).abs() < 1e-3);
}

#[test]
fn ac2_the_centre_is_under_the_pointer_else_the_view_centre() {
    let mut s = session();
    paste(&mut s, Some((500.0, 300.0)), PNG, "");
    let want = s.screen_to_document(500.0, 300.0);
    let got = centre(&images(&s)[0]);
    assert!(
        (got.x - want.x).abs() < 1e-6 && (got.y - want.y).abs() < 1e-6,
        "{got:?} {want:?}"
    );
    paste(&mut s, None, PNG, "");
    let view = s.viewport_centre();
    let got = centre(&images(&s)[1]);
    assert!(
        (got.x - view.x).abs() < 1e-6 && (got.y - view.y).abs() < 1e-6,
        "{got:?} {view:?}"
    );
}

#[test]
fn ac5_ac7_refused_clipboard_data_changes_nothing() {
    let mut s = session();
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="10mm"><rect width="5" height="5"/></svg>"#;
    for bytes in [&svg[..], DAMAGED, b"GIF89a\x01\x00\x01\x00"] {
        let outcome = paste(&mut s, None, bytes, "");
        assert_eq!(
            (outcome.imported, outcome.alert),
            (0, true),
            "{}",
            outcome.notice
        );
        assert!(
            outcome.notice.starts_with("Pasted image\n"),
            "{}",
            outcome.notice
        );
    }
    assert_eq!(s.document.own_steps().len(), 0);
    assert_eq!(s.objects().len(), 0);
}

#[test]
fn ac9_a_paste_in_the_trace_tool_keeps_the_tool_with_the_new_image() {
    let mut s = session();
    s.set_tool(Tool::Trace);
    paste(&mut s, None, PNG, "");
    assert_eq!(s.tool, Tool::Trace);
    let image = images(&s)[0].id;
    assert_eq!(s.trace_view().images, vec![image]);
}

#[test]
fn ac12_the_copy_is_the_stored_pixels_without_the_objects_transform() {
    let mut s = session();
    paste(&mut s, None, &marker_png(), "");
    s.flip_selection(FlipAxis::Horizontal);
    let copy = s.image_copy().unwrap();
    assert_eq!((copy.width, copy.height), (40, 20));
    assert_eq!(&copy.rgba[..4], &[255, 0, 0, 255]);
    assert_eq!(&copy.rgba[(39 * 4)..(40 * 4)], &[255, 255, 255, 255]);
}

#[test]
fn ac13_the_topmost_selected_image_is_copied() {
    let mut s = session();
    paste(&mut s, None, PNG, "");
    paste(&mut s, None, &marker_png(), "");
    paste(&mut s, None, PNG, "");
    let ids: Vec<_> = images(&s).iter().map(|i| i.id).collect();
    s.selection.set(&ids[..2]);
    let copy = s.image_copy().unwrap();
    assert_eq!((copy.width, copy.height), (40, 20));
    s.selection.set(&ids);
    assert!(s.image_copy().is_some(), "three images selected");
    s.selection.set(&[]);
    assert_eq!(s.image_copy(), None);
}

#[test]
fn ac3_ac16_the_chords_wait_for_a_drag_and_stay_with_the_text_editor() {
    let mut s = super::text_tests::session();
    assert!(s.clipboard_allowed(false));
    assert!(!s.clipboard_allowed(true));
    s.set_tool(Tool::Text);
    let at = Point::new(10.0, 20.0);
    s.pointer_down(at, false);
    assert!(!s.clipboard_allowed(false), "a pointer drag runs");
    s.pointer_up(at, false, false);
    assert!(s.text_editing());
    assert!(
        !s.clipboard_allowed(false),
        "the text editor keeps its clipboard"
    );
}

/// `specs/0087-image-clipboard-and-drop` criteria 20 and 24: dropped image data without a name is
/// "Dropped image" in one step "Import image"; the Trace tool stays.
#[test]
fn ac20_ac24_a_nameless_drop_is_dropped_image_and_the_trace_tool_stays() {
    let mut s = session();
    s.set_tool(Tool::Trace);
    s.import_begin_from(ImportOrigin::Drop, Some((400.0, 300.0)));
    s.import_add(PNG, "");
    assert_eq!(s.import_finish(false).imported, 1);
    let image = &images(&s)[0];
    assert_eq!(name_of(&s, image.id).as_deref(), Some("Dropped image"));
    assert_eq!(s.document.own_steps()[0].label, "import_image");
    assert_eq!(s.tool, Tool::Trace);
}

/// Lead decision 2026-10-29 (criterion 6): files a paste named as local paths, read by the host,
/// are one step under the pointer; one image is "Paste image" with its file's name, an SVG comes
/// in as Import SVG brings it, several are one step.
#[test]
fn ac6_pasted_local_files_keep_their_names_and_svg_comes_in() {
    let mut s = session();
    s.import_begin_from(ImportOrigin::PastedFiles, Some((500.0, 300.0)));
    s.import_add(PNG, "photo.png");
    assert_eq!(s.import_finish(false).imported, 1);
    assert_eq!(s.document.own_steps()[0].label, "paste_image");
    let image = &images(&s)[0];
    assert_eq!(name_of(&s, image.id).as_deref(), Some("photo.png"));
    let want = s.screen_to_document(500.0, 300.0);
    assert!((centre(image).x - want.x).abs() < 1e-6);

    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="10mm"><rect width="5" height="5"/></svg>"#;
    s.import_begin_from(ImportOrigin::PastedFiles, None);
    s.import_add(svg, "drawing.svg");
    s.import_add(PNG, "second.png");
    let outcome = s.import_finish(true);
    assert_eq!(
        (outcome.imported, outcome.alert),
        (2, false),
        "{}",
        outcome.notice
    );
    assert_eq!(s.document.own_steps().len(), 2, "one step for both");
    assert_eq!(s.document.own_steps()[1].label, "import");
}

/// Lead decision 2026-10-29: a local file over the import's file limit is refused with the
/// import's own notice, from its length alone; nothing changes.
#[test]
fn ac8_a_local_file_over_the_limit_is_refused_by_its_length() {
    let mut s = session();
    s.import_begin_from(ImportOrigin::Drop, Some((400.0, 300.0)));
    s.import_add_too_large("huge.png", 140 * 1024 * 1024);
    let outcome = s.import_finish(false);
    assert_eq!((outcome.imported, outcome.alert), (0, true));
    assert_eq!(
        outcome.notice,
        "huge.png\nFile too large: 140 MB. The limit is 100 MB."
    );
    assert_eq!(s.document.own_steps().len(), 0);
}
