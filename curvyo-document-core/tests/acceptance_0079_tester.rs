//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 1 on
//! `curvyo-document-core`: the stored registers (criterion 37, `adrs.md` decision 1: absent =
//! default, a default is deleted), lenient reading of merge residue, per-value merge of two peers
//! (ADR 0009 §3), the relative values across a resize (34) and the error paths of
//! `set_text_layout` (nothing written on a refusal).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::assert_is_empty,
    missing_docs
)]

use std::io::{Cursor, Write};

use curvyo_document_core::{
    Affine, Angle, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, FontKind,
    FontRef, Length, NodeId, Point, RectBounds, TextAlign, TextEditError, TextLayoutLabel,
    TextLayoutValues, Vec2, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID};

const D: TextLayoutValues = TextLayoutValues::DEFAULT;
const KEYS: [&str; 5] = [
    "text_align",
    "text_letter_space",
    "text_word_space",
    "text_line_space",
    "text_rotation",
];

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

fn deg(d: f64) -> Angle {
    Angle::from_radians(d.to_radians())
}

fn new_text(d: &Document, layout: &TextLayoutValues) -> NodeId {
    d.create_text(
        "Two\nlines",
        &sans(),
        Length::from_mm(10.0),
        Affine::translation(Vec2::new(10.0, 10.0)),
        layout,
    )
    .unwrap()
}

/// The stored value of each layout key of text `id` (`None`: absent).
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

/// Criterion 37: a default value is not stored; a set value is stored under its key (degrees for
/// the rotation, the align word); back at the default the key is deleted.
#[test]
fn ac37_defaults_are_never_stored() {
    let d = Document::new(1);
    let id = new_text(&d, &D);
    assert_eq!(
        raw(&d, id),
        vec![None; 5],
        "a default text stores no layout key"
    );
    let all = TextLayoutValues {
        align: TextAlign::Justify,
        letter_spacing: 0.25,
        word_spacing: -0.5,
        line_spacing: Some(0.0),
        rotation: deg(-12.5),
    };
    let p = d.text(id).unwrap().placement;
    d.set_text_layout(&[(id, all, p)], TextLayoutLabel::LetterSpacing)
        .unwrap();
    let r = raw(&d, id);
    assert_eq!(r[0], Some(LoroValue::from("justify")));
    assert_eq!(r[1], Some(LoroValue::Double(0.25)));
    assert_eq!(r[2], Some(LoroValue::Double(-0.5)));
    assert_eq!(
        r[3],
        Some(LoroValue::Double(0.0)),
        "line spacing 0 is a value, not Auto"
    );
    match &r[4] {
        Some(LoroValue::Double(v)) => assert!((v + 12.5).abs() < 1e-9, "degrees: {v}"),
        other => panic!("rotation stored as {other:?}"),
    }
    d.set_text_layout(&[(id, D, p)], TextLayoutLabel::LetterSpacing)
        .unwrap();
    assert_eq!(
        raw(&d, id),
        vec![None; 5],
        "back at default: every key deleted"
    );
    // A text created with values stores only the non-default ones.
    let id2 = new_text(
        &d,
        &TextLayoutValues {
            align: TextAlign::Right,
            ..D
        },
    );
    let r = raw(&d, id2);
    assert_eq!(r[0], Some(LoroValue::from("right")));
    assert!(r[1..].iter().all(Option::is_none), "{r:?}");
}

/// Criterion 37 (`adrs.md` decision 1): residue a merge or an older peer could leave is read
/// leniently: an unknown align reads as left, out-of-range factors are clamped, a rotation is
/// wrapped; the file opens.
#[test]
fn ac37_lenient_read_of_residue() {
    let d = Document::new(1);
    let id = new_text(&d, &D);
    let opened = reopen_with(
        &d,
        id,
        &[
            ("text_align", LoroValue::from("diagonal")),
            ("text_letter_space", LoroValue::Double(99.0)),
            ("text_word_space", LoroValue::Double(-50.0)),
            ("text_line_space", LoroValue::Double(-1.0)),
            ("text_rotation", LoroValue::Double(270.0)),
        ],
    );
    let t = opened.text(id).unwrap().layout;
    assert_eq!(t.align, TextAlign::Left);
    assert_eq!(t.letter_spacing, 2.0);
    assert_eq!(t.word_spacing, -1.0);
    assert_eq!(t.line_spacing, Some(0.0));
    assert!((t.rotation.as_radians().to_degrees() + 90.0).abs() < 1e-9);

    let opened = reopen_with(
        &d,
        id,
        &[
            ("text_letter_space", LoroValue::Double(f64::NAN)),
            ("text_line_space", LoroValue::Double(f64::INFINITY)),
            ("text_rotation", LoroValue::Double(f64::NEG_INFINITY)),
            ("text_word_space", LoroValue::from("wide")),
            ("text_align", LoroValue::I64(3)),
        ],
    );
    let t = opened.text(id).unwrap().layout;
    assert!(
        t.is_default(),
        "non-finite and mistyped values read as default: {t:?}"
    );
    // A huge rotation wraps to a finite angle in (-180, 180].
    let opened = reopen_with(&d, id, &[("text_rotation", LoroValue::Double(1e300))]);
    let r = opened
        .text(id)
        .unwrap()
        .layout
        .rotation
        .as_radians()
        .to_degrees();
    assert!(r.is_finite() && r > -180.0 && r <= 180.0, "{r}");
}

/// Criterion 37 / ADR 0009 §3: two peers changing different layout values of one text both
/// survive the merge.
#[test]
fn ac37_two_peers_merge_per_value() {
    let a = Document::new(1);
    let id = new_text(&a, &D);
    let b = Document::new(2);
    b.import_updates(&a.export_updates(&b.version()).unwrap())
        .unwrap();
    let p = a.text(id).unwrap().placement;
    a.set_text_layout(
        &[(
            id,
            TextLayoutValues {
                letter_spacing: 0.3,
                ..D
            },
            p,
        )],
        TextLayoutLabel::LetterSpacing,
    )
    .unwrap();
    b.set_text_layout(
        &[(
            id,
            TextLayoutValues {
                line_spacing: Some(2.0),
                ..D
            },
            p,
        )],
        TextLayoutLabel::LineSpacing,
    )
    .unwrap();
    for (from, to) in [(&a, &b), (&b, &a)] {
        to.import_updates(&from.export_updates(&to.version()).unwrap())
            .unwrap();
    }
    for d in [&a, &b] {
        let l = d.text(id).unwrap().layout;
        assert_eq!(
            (l.letter_spacing, l.line_spacing),
            (0.3, Some(2.0)),
            "{l:?}"
        );
    }
}

/// Criterion 34: a uniform resize changes Size only; every factor and the rotation stay.
#[test]
fn ac34_resize_keeps_the_factors() {
    let d = Document::new(1);
    let v = TextLayoutValues {
        letter_spacing: 0.15,
        word_spacing: 0.4,
        line_spacing: Some(1.5),
        rotation: deg(33.0),
        align: TextAlign::Centre,
    };
    let id = new_text(&d, &v);
    d.set_text_size(&[id], Length::from_mm(20.0)).unwrap();
    let t = d.text(id).unwrap();
    assert_eq!(t.size.as_mm(), 20.0);
    assert_eq!(t.layout, v);
}

/// Error paths of `set_text_layout`: an unknown id or a non-text node, a singular placement: the
/// call is refused and nothing is written, not even for the valid texts of the same call.
#[test]
fn edge_set_text_layout_refusals_write_nothing() {
    let d = Document::new(1);
    let id = new_text(&d, &D);
    let rect = d.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    });
    let p = d.text(id).unwrap().placement;
    let steps = d.own_steps().len();
    let spaced = TextLayoutValues {
        letter_spacing: 0.2,
        ..D
    };
    assert_eq!(
        d.set_text_layout(
            &[(id, spaced, p), (rect, spaced, p)],
            TextLayoutLabel::LetterSpacing
        ),
        Err(TextEditError::NoSuchText)
    );
    let singular = Affine::scale(0.0, 1.0);
    assert_eq!(
        d.set_text_layout(&[(id, spaced, singular)], TextLayoutLabel::LetterSpacing),
        Err(TextEditError::InvalidPlacement)
    );
    let nan = Affine::new(f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0);
    assert_eq!(
        d.set_text_layout(&[(id, spaced, nan)], TextLayoutLabel::LetterSpacing),
        Err(TextEditError::InvalidPlacement)
    );
    assert_eq!(d.own_steps().len(), steps, "no step written");
    assert_eq!(d.text(id).unwrap().layout, D);
    // An empty call writes no step.
    d.set_text_layout(&[], TextLayoutLabel::Alignment).unwrap();
    assert_eq!(d.own_steps().len(), steps);
    // A non-finite factor handed in is not stored as such.
    d.set_text_layout(
        &[(
            id,
            TextLayoutValues {
                word_spacing: f64::NAN,
                letter_spacing: 1e9,
                ..D
            },
            p,
        )],
        TextLayoutLabel::WordSpacing,
    )
    .unwrap();
    let l = d.text(id).unwrap().layout;
    assert!(
        l.word_spacing.is_finite() && l.letter_spacing <= 2.0,
        "{l:?}"
    );
}
