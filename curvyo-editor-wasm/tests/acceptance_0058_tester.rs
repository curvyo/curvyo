//! Tester acceptance cases of `specs/0058-bitmap-import` at the session level (criteria 2 to 5,
//! 31, 32, 36, 37, 45): routing by content, drop placement in a zoomed and panned view, one
//! undo step per batch, the Ctrl+I gate, hit, lock, Node tool, flip and nudge, and the outline
//! command refusal. Everything goes through `Session`'s public API.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::sync::Arc;

use curvyo_document_core::{
    Affine, Document, FontKind, ImageSnapshot, NewImage, ObjectSnapshot, Point, RectBounds, pack,
    unpack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};
use curvyo_ui_core::{FlipAxis, FontBlock};

const TOL: f64 = 1e-6;

fn picture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images")
            .join(name),
    )
    .unwrap()
}

fn images(session: &Session) -> Vec<ImageSnapshot> {
    let document = unpack(99, &session.pack("t").unwrap()).unwrap();
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| match document.object(id) {
            Some(ObjectSnapshot::Image(image)) => Some(image),
            _ => None,
        })
        .collect()
}

fn object_count(session: &Session) -> usize {
    unpack(99, &session.pack("t").unwrap())
        .unwrap()
        .object_ids()
        .len()
}

fn centre(image: &ImageSnapshot) -> Point {
    image.placement.apply_point(Point::new(0.5, 0.5))
}

fn import(
    session: &mut Session,
    drop: Option<(f64, f64)>,
    files: &[(&[u8], &str)],
) -> curvyo_editor_wasm::ImportOutcome {
    session.import_begin(drop);
    for (bytes, name) in files {
        session.import_add(bytes, name);
    }
    session.import_finish(false)
}

fn fresh() -> Session {
    let mut session = Session::new(1);
    session.resize_viewport(1600.0, 1000.0);
    session
}

/// A session over a document with one 40 x 20 mm PNG (alpha 0 in its left third) at (50, 50),
/// plus whatever `extra` adds.
fn session_with_image(extra: impl FnOnce(&Document, curvyo_document_core::NodeId)) -> Session {
    let document = Document::new(1);
    let id = document
        .add_images(&[NewImage {
            bytes: Arc::from(picture("png_rgba_8.png").as_slice()),
            placement: Affine::new(40.0, 0.0, 0.0, 20.0, 50.0, 50.0),
            name: "a.png".to_owned(),
        }])
        .unwrap()[0];
    extra(&document, id);
    let mut session = Session::open(2, &pack(&document, "t").unwrap()).unwrap();
    session.resize_viewport(1600.0, 1000.0);
    session.set_tool(Tool::Select);
    session
}

fn click(session: &mut Session, at: Point) {
    session.pointer_hover(at, false, false);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
}

fn key(session: &mut Session, key: &str, ctrl: bool) {
    session.key_down(KeyInput {
        key,
        ctrl,
        ..KeyInput::default()
    });
}

fn rect(document: &Document, x: f64, y: f64) -> curvyo_document_core::NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + 10.0, y + 10.0),
    ))
}

/// Loads the included fonts as the app does, so the Text tool can open its editor.
fn load_fonts(session: &mut Session) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts");
    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("index.json")).unwrap()).unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let kind = if f["kind"] == "single_line" {
                FontKind::SingleLine
            } else {
                FontKind::Outline
            };
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
                kind,
            )
        })
        .collect();
    session.set_font_list_with_kinds(FontBlock::Included, faces, Vec::new());
    for file in session.wanted_font_files() {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        assert!(session.add_font_file(&file, &bytes), "{file} loads");
    }
}

// --- criteria 2 and 4: routing and drop ---------------------------------------------------------

#[test]
fn ac2_an_svg_named_png_and_a_png_named_svg_go_by_their_content() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="20mm" height="20mm" viewBox="0 0 20 20"><rect x="1" y="1" width="5" height="5"/></svg>"#;
    let mut session = fresh();
    let outcome = import(
        &mut session,
        None,
        &[
            (svg.as_slice(), "photo.png"),
            (&picture("png_rgb_8.png"), "drawing.svg"),
        ],
    );
    assert_eq!(outcome.imported, 2, "{}", outcome.notice);
    assert!(!outcome.alert, "{}", outcome.notice);
    assert_eq!(images(&session).len(), 1, "the PNG is an image");
    assert!(object_count(&session) >= 2, "the SVG became vector objects");
}

#[test]
fn ac4_a_drop_in_a_zoomed_and_panned_view_centres_the_image_under_the_pointer() {
    let mut session = fresh();
    session.wheel(0.0, -480.0, 300.0, 200.0, false, true);
    session.wheel(0.0, -240.0, 900.0, 700.0, false, true);
    session.pan_to(-350.0, 120.0);
    let (sx, sy) = (1234.5, 321.25);
    let under = session.screen_to_document(sx, sy);
    let outcome = import(
        &mut session,
        Some((sx, sy)),
        &[(&picture("png_rgb_8.png"), "a.png")],
    );
    assert_eq!(outcome.imported, 1);
    let got = centre(&images(&session)[0]);
    assert!(
        (got.x - under.x).abs() < TOL && (got.y - under.y).abs() < TOL,
        "centre {got:?}, pointer {under:?}"
    );
}

// --- criterion 5: several files ------------------------------------------------------------------

#[test]
fn ac5_a_batch_with_one_damaged_file_is_one_step_and_one_undo_removes_all() {
    let mut session = fresh();
    let outcome = import(
        &mut session,
        Some((800.0, 500.0)),
        &[
            (&picture("png_rgb_8.png"), "a.png"),
            (&picture("png_truncated.png"), "broken.png"),
            (&picture("jpeg_gray.jpg"), "b.jpg"),
            (&picture("webp_lossy.webp"), "c.webp"),
        ],
    );
    assert_eq!(outcome.imported, 3);
    assert!(
        outcome.notice.contains("could not be imported"),
        "{}",
        outcome.notice
    );
    assert_eq!(session.selected_object_count(), 3);
    let placed = images(&session);
    assert_eq!(placed.len(), 3);
    for pair in placed.windows(2) {
        let (a, b) = (centre(&pair[0]), centre(&pair[1]));
        assert!(
            (b.x - a.x - 10.0).abs() < TOL && (b.y - a.y - 10.0).abs() < TOL,
            "{a:?} {b:?}"
        );
    }
    session.undo();
    assert_eq!(object_count(&session), 0, "one undo step for the batch");
    session.redo();
    assert_eq!(images(&session).len(), 3);
}

#[test]
fn ac5_more_than_20_files_import_the_first_20_with_the_count_notice() {
    let png = picture("png_rgb_8.png");
    let files: Vec<(&[u8], &str)> = (0..23).map(|_| (png.as_slice(), "a.png")).collect();
    let mut session = fresh();
    let outcome = import(&mut session, None, &files);
    assert_eq!(outcome.imported, 20);
    assert!(
        outcome
            .notice
            .contains("Imported 20 of 23 images. The rest were skipped."),
        "{}",
        outcome.notice
    );
}

// --- criterion 3: Ctrl+I ---------------------------------------------------------------------------

#[test]
fn ac3_ctrl_i_is_import_outside_text_edit_and_not_inside_it() {
    let mut session = fresh();
    let chord = |shift, alt| KeyInput {
        key: "i",
        ctrl: true,
        shift,
        alt,
        ..KeyInput::default()
    };
    assert!(session.import_chord(&chord(false, false)));
    assert!(
        !session.import_chord(&chord(true, false)),
        "Shift+Ctrl+I is not the chord"
    );
    assert!(
        !session.import_chord(&chord(false, true)),
        "Ctrl+Alt+I is not the chord"
    );
    assert!(!session.import_chord(&KeyInput {
        key: "i",
        ..KeyInput::default()
    }));
    load_fonts(&mut session);
    session.set_tool(Tool::Text);
    session.pointer_hover(Point::new(40.0, 40.0), false, false);
    session.pointer_down(Point::new(40.0, 40.0), false);
    session.pointer_up(Point::new(40.0, 40.0), false, false);
    if session.text_editing() {
        assert!(
            !session.import_chord(&chord(false, false)),
            "Ctrl+I stays with the text"
        );
    } else {
        panic!("a click with the Text tool did not open the on-canvas editor");
    }
}

// --- criteria 31, 32, 36: hit, transforms, Node tool --------------------------------------------------

#[test]
fn ac31_a_click_on_a_transparent_pixel_inside_the_frame_selects_the_image() {
    let mut session = session_with_image(|_, _| {});
    click(&mut session, Point::new(96.0, 60.0)); // 6 mm outside the frame
    assert_eq!(session.selected_object_count(), 0);
    click(&mut session, Point::new(53.0, 60.0)); // the alpha-0 third
    assert_eq!(session.selected_object_count(), 1);
}

#[test]
fn ac31_a_locked_image_is_never_selected_by_a_click() {
    let mut session = session_with_image(|document, id| {
        document.set_node_locked(&[id], true).unwrap();
    });
    click(&mut session, Point::new(70.0, 60.0));
    assert_eq!(session.selected_object_count(), 0);
}

#[test]
fn ac36_the_node_tool_selects_nothing_on_an_image() {
    let mut session = session_with_image(|_, _| {});
    session.set_tool(Tool::Node);
    click(&mut session, Point::new(70.0, 60.0));
    assert_eq!(session.selected_object_count(), 0);
}

#[test]
fn ac32_ac45_flip_and_nudge_map_the_frame_and_undo_restores_it_exactly() {
    let mut session = session_with_image(|_, _| {});
    click(&mut session, Point::new(70.0, 60.0));
    let before = images(&session)[0].clone();
    session.flip_selection(FlipAxis::Horizontal);
    let flipped = images(&session)[0].clone();
    let mirror = Affine::new(-1.0, 0.0, 0.0, 1.0, 140.0, 0.0); // about x = 70, the frame centre
    for (old, new) in before.corners().iter().zip(flipped.corners()) {
        let want = mirror.apply_point(*old);
        assert!(
            (want.x - new.x).abs() < TOL && (want.y - new.y).abs() < TOL,
            "{want:?} {new:?}"
        );
    }
    key(&mut session, "ArrowRight", false);
    let nudged = images(&session)[0].clone();
    let dx = nudged.corners()[0].x - flipped.corners()[0].x;
    assert!(dx > 0.0 && (nudged.corners()[0].y - flipped.corners()[0].y).abs() < TOL);
    session.undo();
    session.undo();
    assert_eq!(
        images(&session)[0].placement,
        before.placement,
        "exact previous state"
    );
}

// --- criterion 37: outline commands -----------------------------------------------------------

#[test]
fn ac37_a_selection_with_an_image_refuses_the_outline_commands() {
    let mut session = session_with_image(|document, _| {
        rect(document, 0.0, 0.0);
    });
    key(&mut session, "a", true);
    assert_eq!(session.selected_object_count(), 2);
    assert!(session.refuse_images());
    let mut plain = fresh();
    let document = Document::new(1);
    rect(&document, 0.0, 0.0);
    rect(&document, 20.0, 0.0);
    plain = {
        drop(plain);
        let mut s = Session::open(2, &pack(&document, "t").unwrap()).unwrap();
        s.resize_viewport(1600.0, 1000.0);
        s
    };
    key(&mut plain, "a", true);
    assert!(!plain.refuse_images(), "no image, no refusal");
}

#[test]
fn ac37_a_selected_group_holding_an_image_is_refused_for_the_outline_commands() {
    // "A group may contain images; a group holding one is not refused for Group or transform,
    // only for the outline commands above."
    let mut session = session_with_image(|document, id| {
        let inside = rect(document, 0.0, 0.0);
        document.group_objects(&[id, inside]).unwrap();
        rect(document, 100.0, 100.0);
    });
    key(&mut session, "a", true);
    assert!(
        session.selected_object_count() >= 2,
        "the group and the rectangle"
    );
    assert!(session.refuse_images());
}

// --- criteria 17 and 26: the refusal names the numbers ---------------------------------------------

#[test]
fn ac17_ac26_a_60000_px_square_bomb_is_refused_with_its_numbers_and_nothing_changes() {
    let mut session = fresh();
    let outcome = import(
        &mut session,
        None,
        &[(&picture("png_bomb_60000x60000.png"), "bomb.png")],
    );
    assert_eq!((outcome.imported, outcome.alert), (0, true));
    assert_eq!(
        outcome.notice,
        "bomb.png\nImage too large: 60,000 x 60,000 px. The limit is 50 megapixels."
    );
    assert_eq!(object_count(&session), 0);
}
