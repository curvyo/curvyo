//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 4 on
//! `curvyo-document-core` (ADR 0021): a character a peer types directly next to a kerned one gets
//! no kern and a deleted and retyped character has none (criterion 30, `ExpandType::None`), a copy
//! of a text object keeps its kerns (ADR 0021 §4), two peers kerning different characters both
//! survive (31), format 21 with the golden file and older files opening without marks (37), the
//! lenient read of merge residue (ADR 0021 §1) and "only differences are written" (§6).
//!
//! Written from the specification, ADR 0021 and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty,
    clippy::similar_names,
    missing_docs
)]

use std::io::{Cursor, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, CharValues, CopySource,
    Document, FontKind, FontRef, Length, NodeId, TextChars, TextCharsLabel, TextLayoutValues, Vec2,
    pack, unpack,
};
use loro::{Container, LoroDoc, LoroText, LoroValue, TreeID, ValueOrContainer};

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

fn new_text(d: &Document, s: &str) -> NodeId {
    d.create_text(
        s,
        &sans(),
        Length::from_mm(10.0),
        Affine::translation(Vec2::new(10.0, 10.0)),
        &TextLayoutValues::DEFAULT,
    )
    .unwrap()
}

fn kx(x: f64) -> CharValues {
    CharValues {
        kern_x: x,
        ..CharValues::DEFAULT
    }
}

fn chars(d: &Document, id: NodeId) -> Vec<(usize, CharValues)> {
    d.text(id).unwrap().chars.entries().to_vec()
}

fn kern(d: &Document, id: NodeId, entries: Vec<(usize, CharValues)>) {
    d.set_text_chars(&[(id, TextChars::new(entries))], TextCharsLabel::Kerning)
        .unwrap();
}

/// A raw Loro peer (id 7) holding `d`'s state, and the text container of `id` in it.
fn raw_peer(d: &Document, id: NodeId) -> (LoroDoc, LoroText) {
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro.set_peer_id(7).unwrap();
    let mut styles = loro::StyleConfigMap::new();
    for key in ["kern_x", "kern_y"] {
        styles.insert(
            key.into(),
            loro::StyleConfig {
                expand: loro::ExpandType::None,
            },
        );
    }
    loro.config_text_style(styles);
    let meta = loro.get_tree("paths").get_meta(tree_id(id)).unwrap();
    let Some(ValueOrContainer::Container(Container::Text(text))) = meta.get("text_string") else {
        panic!("text_string is a LoroText");
    };
    (loro, text)
}

/// Opens the raw peer's state as a file of the current format, as a reopened or merged document.
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

/// Criterion 30 with ADR 0021 §1 (`ExpandType::None`): a peer typing directly before and after a
/// kerned character gives the new characters no kern and the kern moves along; a peer deleting
/// the kerned character and typing it again leaves no kern.
#[test]
fn c30_remote_typing_next_to_a_kern_inherits_nothing_and_retyping_resurrects_nothing() {
    let d = Document::new(1);
    let id = new_text(&d, "ABC");
    kern(&d, id, vec![(1, kx(0.2))]);
    let (peer, text) = raw_peer(&d, id);
    text.insert(2, "y").unwrap(); // after B
    text.insert(1, "x").unwrap(); // before B
    peer.commit();
    let d = reopen(&peer);
    let t = d.text(id).unwrap();
    assert_eq!(t.string, "AxByC");
    assert_eq!(
        chars(&d, id),
        vec![(2, kx(0.2))],
        "only B, at its new index"
    );

    let (peer, text) = raw_peer(&d, id);
    text.delete(2, 1).unwrap();
    text.insert(2, "B").unwrap();
    peer.commit();
    let d = reopen(&peer);
    assert_eq!(d.text(id).unwrap().string, "AxByC");
    assert!(chars(&d, id).is_empty(), "a retyped B has no kern");
}

/// ADR 0021 §4: a copy of a text object (duplicate, paste of objects) keeps every kern of the
/// source; a later change on the copy does not touch the source.
#[test]
fn c30_copy_of_a_text_object_keeps_its_kerns() {
    let d = Document::new(1);
    let id = new_text(&d, "Kern me");
    let k = vec![
        (0, kx(-0.1)),
        (
            3,
            CharValues {
                kern_x: 0.05,
                kern_y: 0.3,
                rotation: 0.0,
            },
        ),
    ];
    kern(&d, id, k.clone());
    let copies = d
        .duplicate_objects(
            &[CopySource {
                id,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(5.0, 5.0),
        )
        .unwrap();
    assert_eq!(copies.len(), 1);
    assert_eq!(chars(&d, copies[0]), k);
    kern(&d, copies[0], vec![(5, kx(0.5))]);
    assert_eq!(chars(&d, id), k, "the source is untouched");
}

/// Criterion 31 (ADR 0009 §3, ADR 0021 §3): two peers kern different characters; both kerns
/// survive on both peers; the same character kerned by both converges on one value.
#[test]
fn c31_two_peers_kern_different_characters_and_both_survive() {
    let a = Document::new(1);
    let id = new_text(&a, "WAVE");
    let b = Document::new(2);
    b.import_updates(&a.export_updates(&b.version()).unwrap())
        .unwrap();
    kern(&a, id, vec![(1, kx(-0.1)), (3, kx(0.2))]);
    kern(
        &b,
        id,
        vec![
            (
                2,
                CharValues {
                    kern_x: 0.0,
                    kern_y: 0.4,
                    rotation: 0.0,
                },
            ),
            (3, kx(-0.3)),
        ],
    );
    for (from, to) in [(&a, &b), (&b, &a)] {
        to.import_updates(&from.export_updates(&to.version()).unwrap())
            .unwrap();
    }
    let (ca, cb) = (chars(&a, id), chars(&b, id));
    assert_eq!(ca, cb, "converged");
    assert_eq!(ca[0], (1, kx(-0.1)));
    assert_eq!(ca[1].0, 2);
    assert_eq!(ca[1].1.kern_y, 0.4);
    assert_eq!(ca[2].0, 3);
    assert!([0.2, -0.3].contains(&ca[2].1.kern_x), "{ca:?}");
}

/// Criterion 37: format 21, the golden file opens with its kerns and saves them back; a file of
/// format 20 opens, draws without kerns and saves with no mark; a text without kerns stores no
/// mark.
#[test]
fn c37_format_21_golden_file_and_older_files() {
    assert_eq!(CURRENT_FORMAT_VERSION, 21);
    let golden = unpack(3, &fixture("text_layout_v21.curvyo")).unwrap();
    let kerned: Vec<_> = golden
        .object_ids()
        .into_iter()
        .filter_map(|id| golden.text(id))
        .collect();
    assert!(
        kerned.len() >= 4,
        "one text per value, both, and one without"
    );
    assert!(kerned.iter().any(|t| t.chars.is_empty()));
    assert!(
        kerned
            .iter()
            .any(|t| t.chars.entries().iter().any(|(_, v)| v.kern_x != 0.0))
    );
    assert!(
        kerned
            .iter()
            .any(|t| t.chars.entries().iter().any(|(_, v)| v.kern_y != 0.0))
    );
    assert!(kerned.iter().any(|t| t.chars.entries().len() > 1));
    let again = unpack(4, &pack(&golden, "tester").unwrap()).unwrap();
    for id in golden.object_ids() {
        assert_eq!(
            golden.text(id).map(|t| t.chars),
            again.text(id).map(|t| t.chars)
        );
    }

    for old in [
        "text_layout_v20.curvyo",
        "text_layout_v19.curvyo",
        "text_v17.curvyo",
    ] {
        let d = unpack(5, &fixture(old)).unwrap();
        let ids = d.object_ids();
        assert!(ids.iter().filter_map(|id| d.text(*id)).count() > 0, "{old}");
        let saved = unpack(6, &pack(&d, "tester").unwrap()).unwrap();
        for id in ids {
            if let Some(t) = saved.text(id) {
                assert!(t.chars.is_empty(), "{old}: no mark");
                let (_, text) = raw_peer(&saved, id);
                let delta = text.to_delta();
                assert!(
                    delta.iter().all(|seg| !format!("{seg:?}").contains("kern")),
                    "{old}: {delta:?}"
                );
            }
        }
    }
}

/// White-box (ADR 0021 §1, §6): merge residue reads leniently (out of range clamped, non-number
/// as 0, a mark on `\n` kept but harmless); `set_text_chars` writes only differences and commits
/// nothing when nothing changed; a value set back to 0 leaves no entry.
#[test]
fn edge_lenient_read_and_only_differences_written() {
    let d = Document::new(1);
    let id = new_text(&d, "AB\nC");
    let (peer, text) = raw_peer(&d, id);
    text.mark(0..1, "kern_x", LoroValue::Double(7.0)).unwrap();
    text.mark(1..2, "kern_y", LoroValue::String("x".into()))
        .unwrap();
    peer.commit();
    let r = reopen(&peer);
    assert_eq!(
        chars(&r, id),
        vec![(0, kx(1.0))],
        "clamped; a string reads as 0"
    );

    let before = d.version();
    kern(&d, id, vec![]);
    assert_eq!(d.version(), before, "nothing changed: no commit");
    kern(&d, id, vec![(1, kx(0.1))]);
    kern(&d, id, vec![(1, kx(0.0))]);
    assert!(chars(&d, id).is_empty());
}
