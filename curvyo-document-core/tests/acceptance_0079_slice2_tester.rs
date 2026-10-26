//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 2 (the
//! taper) on `curvyo-document-core`: every older golden file opens untapered and saves without a
//! taper (criterion 37), the golden file of format 24 holds the slice's texts (37), a Size change
//! keeps the taper (34), and merge residue in the `text_taper` register is read leniently
//! (`adrs.md` decision 7).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, FontKind, FontRef,
    Length, NodeId, TextLayoutLabel, TextLayoutValues, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn sans() -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

fn new_text(d: &Document, s: &str, layout: &TextLayoutValues) -> NodeId {
    d.create_text(
        s,
        &sans(),
        Length::from_mm(10.0),
        Affine::translation(Vec2::new(10.0, 10.0)),
        layout,
    )
    .unwrap()
}

fn tree_id(id: NodeId) -> TreeID {
    let json = serde_json::to_value(id).unwrap();
    TreeID::new(
        json["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(json["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn manifest_version(bytes: &[u8]) -> u64 {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    let m: serde_json::Value = serde_json::from_slice(&out).unwrap();
    m["format_version"].as_u64().unwrap()
}

/// Opens a raw Loro state as a file of the current format, as a merged document would be.
fn reopen(peer: &LoroDoc) -> Document {
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "tester",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        (
            "document.loro",
            peer.export(loro::ExportMode::Snapshot).unwrap(),
        ),
        ("document.json", b"{}".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    unpack(3, &writer.finish().unwrap().into_inner()).expect("opens, never refused")
}

/// Criterion 37: every golden file from before this slice (formats 17 to 23) opens with every
/// text untapered, keeps its other layout values, and saves with no taper anywhere in the JSON
/// mirror.
#[test]
fn c37_every_older_file_opens_untapered() {
    for old in [
        "text_v17.curvyo",
        "text_single_line_v18.curvyo",
        "text_layout_v19.curvyo",
        "text_layout_v20.curvyo",
        "text_layout_v21.curvyo",
        "text_on_path_v22.curvyo",
        "text_clones_v23.curvyo",
    ] {
        let bytes = fixture(old);
        assert!(
            manifest_version(&bytes) < 24,
            "{old} is older than format 24"
        );
        let d = unpack(5, &bytes).unwrap();
        let texts: Vec<_> = d
            .object_ids()
            .into_iter()
            .filter_map(|id| d.text(id))
            .collect();
        assert!(!texts.is_empty(), "{old} has texts");
        assert!(texts.iter().all(|t| t.layout.taper == 0.0), "{old}");
        let saved = pack(&d, "tester").unwrap();
        assert_eq!(manifest_version(&saved), u64::from(CURRENT_FORMAT_VERSION));
        let again = unpack(6, &saved).unwrap();
        let json = String::from_utf8(again.export_json().unwrap()).unwrap();
        assert!(!json.contains("taper"), "{old}: no taper stored");
        for id in d.object_ids() {
            assert_eq!(
                d.text(id).map(|t| t.layout),
                again.text(id).map(|t| t.layout),
                "{old}: layout values kept"
            );
        }
    }
}

/// Criterion 37: the golden file is format 24 and holds one text with only a taper (per value of
/// the slice), one with the taper and every other value, and texts of both taper signs.
#[test]
fn c37_golden_v24_holds_the_slice_values() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= 24);
    let bytes = fixture("text_layout_v24.curvyo");
    assert_eq!(manifest_version(&bytes), 24);
    let d = unpack(7, &bytes).unwrap();
    let texts: Vec<_> = d
        .object_ids()
        .into_iter()
        .filter_map(|id| d.text(id))
        .collect();
    let only_taper =
        |l: &TextLayoutValues| l.taper != 0.0 && TextLayoutValues { taper: 0.0, ..*l }.is_default();
    assert!(
        texts.iter().any(|t| only_taper(&t.layout)),
        "one text with only a taper"
    );
    let d0 = TextLayoutValues::DEFAULT;
    assert!(
        texts.iter().any(|t| {
            let l = t.layout;
            l.taper != 0.0
                && l.align != d0.align
                && l.letter_spacing != 0.0
                && l.word_spacing != 0.0
                && l.line_spacing.is_some()
                && l.rotation.as_radians() != 0.0
                && l.writing_mode != d0.writing_mode
                && l.orientation != d0.orientation
        }),
        "one text with all of them"
    );
    assert!(texts.iter().any(|t| t.layout.taper > 0.0));
    assert!(texts.iter().any(|t| t.layout.taper < 0.0));
    assert!(texts.iter().all(|t| (-0.5..=0.5).contains(&t.layout.taper)));
}

/// Criterion 34 at the model level: a Size change keeps the taper (it is a fraction, not
/// relative), and the taper survives save and open next to the new Size.
#[test]
fn c34_size_change_keeps_the_taper() {
    let d = Document::new(1);
    let id = new_text(&d, "Ring", &TextLayoutValues::DEFAULT);
    d.set_text_layout(
        &[(
            id,
            TextLayoutValues {
                taper: -0.25,
                ..TextLayoutValues::DEFAULT
            },
            Affine::IDENTITY,
        )],
        TextLayoutLabel::Taper,
    )
    .unwrap();
    d.set_text_size(&[id], Length::from_mm(37.0)).unwrap();
    let t = d.text(id).unwrap();
    assert!((t.size.as_mm() - 37.0).abs() < 1e-9);
    assert_eq!(t.layout.taper, -0.25);
    let again = unpack(8, &pack(&d, "tester").unwrap()).unwrap();
    assert_eq!(again.text(id).unwrap().layout.taper, -0.25);
}

/// `adrs.md` decision 7 (white-box): merge residue in the register opens and never crashes; an
/// out-of-range taper is held in -0.5..0.5, a non-finite or non-number value reads as 0.
#[test]
fn edge_lenient_read_of_the_register() {
    for (value, want) in [
        (LoroValue::Double(0.9), 0.5),
        (LoroValue::Double(-7.0), -0.5),
        (LoroValue::Double(f64::NAN), 0.0),
        (LoroValue::Double(f64::INFINITY), 0.0),
        (LoroValue::String("wide".into()), 0.0),
        (LoroValue::Double(0.3), 0.3),
    ] {
        let d = Document::new(1);
        let id = new_text(&d, "Residue", &TextLayoutValues::DEFAULT);
        let peer = LoroDoc::new();
        peer.import(&d.export_loro_snapshot().unwrap()).unwrap();
        peer.set_peer_id(7).unwrap();
        let meta = peer.get_tree("paths").get_meta(tree_id(id)).unwrap();
        meta.insert("text_taper", value.clone()).unwrap();
        peer.commit();
        let r = reopen(&peer);
        let got = r.text(id).expect("the text opens").layout.taper;
        assert!(got.is_finite(), "{value:?} -> {got}");
        assert!(
            (got - want).abs() < 1e-12,
            "{value:?} -> {got}, want {want}"
        );
    }
}
