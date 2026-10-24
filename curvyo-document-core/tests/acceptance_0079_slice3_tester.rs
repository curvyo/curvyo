//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 3 on
//! `curvyo-document-core`: the writing mode and glyph orientation keys (criterion 37, `adrs.md`
//! decision 13: absent = default, a default is deleted), format 20, files of format 19 and older
//! open horizontal, lenient reading, the per-value merge of two peers (ADR 0009 §3) and a resize
//! keeping both values (34).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::similar_names,
    missing_docs
)]

use std::io::{Cursor, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, FontKind, FontRef,
    Length, NodeId, TextAlign, TextGlyphOrientation, TextLayoutLabel, TextLayoutValues,
    TextWritingMode, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID};

const D: TextLayoutValues = TextLayoutValues::DEFAULT;
const KEYS: [&str; 2] = ["text_writing_mode", "text_glyph_orientation"];

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

fn tree_id(id: NodeId) -> TreeID {
    let json = serde_json::to_value(id).unwrap();
    TreeID::new(
        json["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(json["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn new_text(d: &Document, layout: &TextLayoutValues) -> NodeId {
    d.create_text(
        "日本\nAB",
        &sans(),
        Length::from_mm(10.0),
        Affine::translation(Vec2::new(10.0, 10.0)),
        layout,
    )
    .unwrap()
}

fn v(mode: TextWritingMode, orientation: TextGlyphOrientation) -> TextLayoutValues {
    TextLayoutValues {
        writing_mode: mode,
        orientation,
        ..D
    }
}

fn set(d: &Document, id: NodeId, values: TextLayoutValues, label: TextLayoutLabel) {
    let p = d.text(id).unwrap().placement;
    d.set_text_layout(&[(id, values, p)], label).unwrap();
}

fn raw(d: &Document, id: NodeId) -> Vec<Option<LoroValue>> {
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let meta = loro.get_tree("paths").get_meta(tree_id(id)).unwrap();
    KEYS.iter()
        .map(|k| meta.get(k).and_then(|v| v.into_value().ok()))
        .collect()
}

fn reopen_with(d: &Document, id: NodeId, writes: &[(&str, LoroValue)]) -> Document {
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro.set_peer_id(7).unwrap();
    let meta = loro.get_tree("paths").get_meta(tree_id(id)).unwrap();
    for (k, v) in writes {
        meta.insert(k, v.clone()).unwrap();
    }
    loro.commit();
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
            loro.export(loro::ExportMode::Snapshot).unwrap(),
        ),
        ("document.json", b"{}".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    unpack(3, &writer.finish().unwrap().into_inner()).expect("residue opens, never refused")
}

fn texts(d: &Document) -> Vec<TextLayoutValues> {
    d.object_ids()
        .into_iter()
        .filter_map(|id| d.text(id).map(|t| t.layout))
        .collect()
}

/// Criterion 37 / decision 13: the two values are stored under their keys with their words
/// only when not at the default; set back to the default the key is deleted.
#[test]
fn ac37_keys_words_and_defaults() {
    let d = Document::new(1);
    let id = new_text(&d, &D);
    assert_eq!(raw(&d, id), vec![None, None]);
    set(
        &d,
        id,
        v(TextWritingMode::VerticalRl, TextGlyphOrientation::Upright),
        TextLayoutLabel::WritingMode,
    );
    assert_eq!(
        raw(&d, id),
        vec![
            Some(LoroValue::from("vertical_rl")),
            Some(LoroValue::from("upright"))
        ]
    );
    set(
        &d,
        id,
        v(TextWritingMode::VerticalLr, TextGlyphOrientation::Sideways),
        TextLayoutLabel::GlyphOrientation,
    );
    assert_eq!(
        raw(&d, id),
        vec![
            Some(LoroValue::from("vertical_lr")),
            Some(LoroValue::from("sideways"))
        ]
    );
    set(&d, id, D, TextLayoutLabel::WritingMode);
    assert_eq!(raw(&d, id), vec![None, None], "defaults are deleted");
}

/// Criterion 37: save, close and open keeps every combination of the two values together with the
/// slice-1 values; the file is format 20.
#[test]
fn ac37_round_trip_every_combination() {
    assert_eq!(CURRENT_FORMAT_VERSION, 20);
    let d = Document::new(1);
    let mut want = Vec::new();
    for mode in [
        TextWritingMode::Horizontal,
        TextWritingMode::VerticalRl,
        TextWritingMode::VerticalLr,
    ] {
        for orientation in [
            TextGlyphOrientation::Auto,
            TextGlyphOrientation::Upright,
            TextGlyphOrientation::Sideways,
        ] {
            let values = TextLayoutValues {
                align: TextAlign::Centre,
                letter_spacing: 0.1,
                line_spacing: Some(1.5),
                ..v(mode, orientation)
            };
            want.push((new_text(&d, &values), values));
        }
    }
    let bytes = pack(&d, "tester").unwrap();
    let opened = unpack(2, &bytes).unwrap();
    for (id, values) in want {
        assert_eq!(opened.text(id).unwrap().layout, values);
    }
}

/// Criterion 37: files from before this slice (format 19, 17) open with every text horizontal
/// and Auto, so they draw as before.
#[test]
fn ac37_older_files_open_horizontal() {
    for name in ["text_layout_v19.curvyo", "text_v17.curvyo"] {
        let d = unpack(4, &fixture(name)).unwrap();
        let all = texts(&d);
        assert!(!all.is_empty(), "{name} has texts");
        for t in all {
            assert_eq!(t.writing_mode, TextWritingMode::Horizontal, "{name}");
            assert_eq!(t.orientation, TextGlyphOrientation::Auto, "{name}");
        }
    }
}

/// Criterion 37: the golden file of this slice opens with one text per value, one with both
/// values set and one with none.
#[test]
fn ac37_golden_v20_holds_every_value() {
    let d = unpack(4, &fixture("text_layout_v20.curvyo")).unwrap();
    let all = texts(&d);
    let has = |f: &dyn Fn(&TextLayoutValues) -> bool| all.iter().any(f);
    assert!(has(&|t| t.writing_mode == TextWritingMode::VerticalRl));
    assert!(has(&|t| t.writing_mode == TextWritingMode::VerticalLr));
    assert!(has(&|t| t.orientation == TextGlyphOrientation::Upright));
    assert!(has(&|t| t.orientation == TextGlyphOrientation::Sideways));
    assert!(has(&|t| t.writing_mode != TextWritingMode::Horizontal
        && t.orientation != TextGlyphOrientation::Auto));
    assert!(has(&|t| t.writing_mode == TextWritingMode::Horizontal
        && t.orientation == TextGlyphOrientation::Auto));
}

/// Criterion 37 (lenient read): an unknown word or a mistyped value reads as the default; the
/// file opens.
#[test]
fn ac37_lenient_read_of_residue() {
    let d = Document::new(1);
    let id = new_text(&d, &D);
    for (mode, orientation) in [
        (LoroValue::from("ttb"), LoroValue::from("Upright")),
        (LoroValue::I64(1), LoroValue::Bool(true)),
        (LoroValue::from(""), LoroValue::Double(f64::NAN)),
    ] {
        let opened = reopen_with(
            &d,
            id,
            &[
                ("text_writing_mode", mode),
                ("text_glyph_orientation", orientation),
            ],
        );
        let t = opened.text(id).unwrap().layout;
        assert_eq!(t.writing_mode, TextWritingMode::Horizontal);
        assert_eq!(t.orientation, TextGlyphOrientation::Auto);
    }
    // A valid orientation on a horizontal text is kept (criterion 21).
    let opened = reopen_with(
        &d,
        id,
        &[("text_glyph_orientation", LoroValue::from("sideways"))],
    );
    let t = opened.text(id).unwrap().layout;
    assert_eq!(
        (t.writing_mode, t.orientation),
        (TextWritingMode::Horizontal, TextGlyphOrientation::Sideways)
    );
}

/// ADR 0009 §3: one peer changes the writing mode, another the orientation: both survive.
#[test]
fn ac37_two_peers_merge_per_value() {
    let a = Document::new(1);
    let id = new_text(&a, &D);
    let b = Document::new(2);
    b.import_updates(&a.export_updates(&b.version()).unwrap())
        .unwrap();
    set(
        &a,
        id,
        v(TextWritingMode::VerticalLr, TextGlyphOrientation::Auto),
        TextLayoutLabel::WritingMode,
    );
    set(
        &b,
        id,
        v(TextWritingMode::Horizontal, TextGlyphOrientation::Upright),
        TextLayoutLabel::GlyphOrientation,
    );
    for (from, to) in [(&a, &b), (&b, &a)] {
        to.import_updates(&from.export_updates(&to.version()).unwrap())
            .unwrap();
    }
    for d in [&a, &b] {
        let l = d.text(id).unwrap().layout;
        assert_eq!(
            (l.writing_mode, l.orientation),
            (TextWritingMode::VerticalLr, TextGlyphOrientation::Upright)
        );
    }
}

/// Criterion 34: a uniform resize keeps the writing mode and the orientation.
#[test]
fn ac34_resize_keeps_writing_values() {
    let d = Document::new(1);
    let values = v(TextWritingMode::VerticalRl, TextGlyphOrientation::Sideways);
    let id = new_text(&d, &values);
    d.set_text_size(&[id], Length::from_mm(25.0)).unwrap();
    assert_eq!(d.text(id).unwrap().layout, values);
}
