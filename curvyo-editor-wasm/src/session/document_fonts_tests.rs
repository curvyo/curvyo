//! Document fonts in the session (`specs/0107-document-fonts` criteria 1, 6, 7, 10 to 12, 16):
//! the file a text uses is embedded on save, found again on a device that has not installed it,
//! listed as the Document Fonts block, and carried to a replica through a room snapshot.

use curvyo_document_core::ObjectSnapshot;

use super::Session;
use super::text_tests::{texts, with_text};

/// A session reopened from `bytes` with no font listed at all: a device that installed nothing.
fn bare(bytes: &[u8]) -> Session {
    Session::open(2, bytes).unwrap()
}

fn document_faces(session: &Session) -> Vec<serde_json::Value> {
    let json: serde_json::Value = serde_json::from_str(&session.fonts_tab_json()).unwrap();
    json["faces"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|face| face["block"] == "document")
        .cloned()
        .collect()
}

#[test]
fn ac1_a_save_embeds_the_used_font_once() {
    let session = with_text("Name");
    let used = session.document.used_fonts();
    assert_eq!(used.len(), 1);
    let bytes = session.pack("t").unwrap();
    let reopened = bare(&bytes);
    let hash = curvyo_document_core::AssetHash::parse(&used[0].face_hash).unwrap();
    assert!(reopened.document.asset(&hash).is_some());
}

#[test]
fn ac6_a_device_without_the_font_draws_the_text_in_it() {
    let session = with_text("Name");
    let reopened = bare(&session.pack("t").unwrap());
    let font = texts(&reopened).remove(0).font;
    assert!(
        !reopened.fonts.is_missing(&font),
        "the embedded face is found"
    );
    let drawn = reopened.objects().iter().any(|object| match object {
        ObjectSnapshot::Text(text) => text.outline.as_ref().is_some_and(|o| !o.substitute),
        _ => false,
    });
    assert!(drawn, "drawn in its own face, not the substitute");
}

#[test]
fn ac10_ac12_the_document_block_marks_a_font_that_is_not_installed() {
    let session = with_text("Name");
    let reopened = bare(&session.pack("t").unwrap());
    let faces = document_faces(&reopened);
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0]["installed"], false);
    assert_eq!(faces[0]["loaded"], true);
}

#[test]
fn ac7_ac11_an_installed_font_marks_its_row_installed() {
    let mut session = with_text("Name");
    session.sync_document_fonts();
    let faces = document_faces(&session);
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0]["installed"], true);
}

#[test]
fn ac14_the_embedded_file_is_handed_out_for_install() {
    let session = with_text("Name");
    let reopened = bare(&session.pack("t").unwrap());
    let faces = document_faces(&reopened);
    let id = faces[0]["id"].as_str().unwrap();
    let bytes = reopened.document_font_file(id).unwrap();
    let hash = curvyo_document_core::AssetHash::of(&bytes);
    assert_eq!(id, format!("doc:{}", hash.as_str()));
}

#[test]
fn ac16_a_replica_gets_the_font_with_the_room_snapshot() {
    let mut a = with_text("Name");
    a.sync_document_fonts();
    let snapshot = curvyo_document_core::pack_snapshot(&a.document, super::APP_VERSION).unwrap();
    let mut b = Session::new(3);
    assert!(b.merge_snapshot(&snapshot));
    b.refresh_document_fonts();
    let font = texts(&b).remove(0).font;
    assert!(!b.fonts.is_missing(&font));
    assert_eq!(document_faces(&b).len(), 1);
}

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/document_font_foreign.curvyo")
}

/// Writes the golden file: one text in the included sans' bytes under a family name no device
/// lists ("Zzz Embedded"), so a device opening it has the font only through the file. Run with
/// `cargo test -p curvyo-editor-wasm write_document_font_fixture -- --ignored`.
#[test]
#[ignore = "writes the golden file; run by hand"]
fn write_document_font_fixture() {
    let mut session = with_text("Embedded");
    let id = session.document.object_ids()[0];
    let mut font = session.document.text(id).unwrap().font;
    font.family = "Zzz Embedded".to_owned();
    session.document.set_text_fonts(&[(id, font)]).unwrap();
    session.sync_document_fonts();
    std::fs::create_dir_all(fixture_path().parent().unwrap()).unwrap();
    std::fs::write(fixture_path(), session.pack("t").unwrap()).unwrap();
}

#[test]
fn golden_a_file_with_an_embedded_font_opens_with_it_on_a_bare_device() {
    let session = bare(&std::fs::read(fixture_path()).unwrap());
    let text = texts(&session).remove(0);
    assert_eq!(text.font.family, "Zzz Embedded");
    assert!(!session.fonts.is_missing(&text.font));
    let faces = document_faces(&session);
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0]["installed"], false);
}
