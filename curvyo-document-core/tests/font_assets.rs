//! Fonts as assets (`specs/0107-document-fonts` criteria 1 to 3, 5; ADR 0047): the faces the
//! texts use, the bytes in the container under the hash the text carries, and the wipe that keeps
//! exactly the used ones.

#![allow(clippy::unwrap_used)]

use curvyo_document_core::{
    Affine, AssetHash, Document, FontKind, FontRef, Length, TextLayoutValues, pack, unpack,
};

fn font_of(bytes: &[u8], index: u32) -> FontRef {
    FontRef {
        family: "Sample Serif".to_owned(),
        style: "Regular".to_owned(),
        face_hash: AssetHash::of(bytes).as_str().to_owned(),
        face_index: index,
        kind: FontKind::Outline,
    }
}

fn add_text(document: &Document, string: &str, font: &FontRef) {
    document
        .create_text(
            string,
            font,
            Length::from_mm(10.0),
            Affine::new(1.0, 0.0, 0.0, 1.0, 5.0, 5.0),
            &TextLayoutValues::default(),
        )
        .unwrap();
}

#[test]
fn ac1_ac2_used_fonts_are_listed_once_per_face() {
    let file = b"font file one".as_slice();
    let d = Document::new(1);
    add_text(&d, "A", &font_of(file, 0));
    add_text(&d, "B", &font_of(file, 0));
    add_text(&d, "C", &font_of(file, 1));
    let used = d.used_fonts();
    assert_eq!(used.len(), 2, "two faces of one file");
    assert_eq!(d.used_font_hashes().len(), 1, "one file");
}

#[test]
fn ac1_the_font_bytes_survive_save_and_open_under_their_hash() {
    let file = b"font file one".as_slice();
    let d = Document::new(1);
    add_text(&d, "A", &font_of(file, 0));
    let hash = d.insert_asset(file).unwrap();
    let opened = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    assert_eq!(opened.asset(&hash).unwrap().as_ref(), file);
    assert_eq!(opened.used_fonts().len(), 1);
}

#[test]
fn ac3_a_wipe_keeps_the_fonts_a_text_uses_and_drops_the_rest() {
    let used = b"used font".as_slice();
    let unused = b"unused font".as_slice();
    let mut d = Document::new(1);
    add_text(&d, "A", &font_of(used, 0));
    let kept = d.insert_asset(used).unwrap();
    let gone = d.insert_asset(unused).unwrap();
    d.wipe_history().unwrap();
    assert!(d.asset(&kept).is_some());
    assert!(d.asset(&gone).is_none());
}
