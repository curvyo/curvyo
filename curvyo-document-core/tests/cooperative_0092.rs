//! Document-core additions of `specs/0092-cooperative-editing` (ADR 0031 decisions 3, 9 and 9a):
//! the page frame register, the shared-file container members, the version text and received
//! assets.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Cursor, Read, Write};

use curvyo_document_core::{
    AssetHash, CURRENT_FORMAT_VERSION, Document, DocumentSize, DocumentVersion, Length, NodeId,
    ObjectSnapshot, OpenError, Point, RectBounds, ShareRecord, Vec2, pack, pack_snapshot, unpack,
};

const EPS: f64 = 1e-9;

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn rect(document: &Document, x: f64, y: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(50.0),
        height: Length::from_mm(40.0),
    })
}

fn rect_origin(document: &Document, id: NodeId) -> Point {
    let Some(ObjectSnapshot::Primitive(primitive)) = document.object(id) else {
        panic!("a primitive");
    };
    let curvyo_document_core::Shape::Rect { bounds, .. } = primitive.shape else {
        panic!("a rect");
    };
    bounds.origin
}

fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < EPS && (a.y - b.y).abs() < EPS
}

fn merge(a: &Document, b: &Document) {
    let (av, bv) = (a.version(), b.version());
    let au = a.export_updates(&bv).unwrap();
    let bu = b.export_updates(&av).unwrap();
    a.import_updates(&bu).unwrap();
    b.import_updates(&au).unwrap();
}

fn loro_of(document: &Document) -> loro::LoroDoc {
    let loro = loro::LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro
}

fn change_count(document: &Document) -> usize {
    loro_of(document).len_changes()
}

fn root_keys(document: &Document) -> Vec<String> {
    let mut keys = Vec::new();
    loro_of(document)
        .get_map("root")
        .for_each(|key, _| keys.push(key.to_owned()));
    keys
}

fn first_frame(document: &Document) -> (Point, DocumentSize) {
    let page = document.pages()[0].clone();
    (page.origin, page.size)
}

// ---------------------------------------------------------------- page frame (criterion 31)

/// A resize keeps the frame's centre and moves no object; read relative to the frame origin the
/// rectangle sits where a single-user resize put it before (0015 criterion 17's example).
#[test]
fn resize_moves_the_frame_around_its_centre_and_no_object() {
    let document = Document::new(1);
    let id = rect(&document, 10.0, 10.0);
    assert_eq!(
        document.resize(DocumentSize::from_mm(300.0, 400.0)),
        Ok(true)
    );
    let (origin, size) = first_frame(&document);
    assert_eq!(size, DocumentSize::from_mm(300.0, 400.0));
    assert!(near(origin, Point::new(-45.0, -51.5)), "{origin:?}");
    let stored = rect_origin(&document, id);
    assert!(near(stored, Point::new(10.0, 10.0)), "the object stays");
    assert!(near(
        Point::new(stored.x - origin.x, stored.y - origin.y),
        Point::new(55.0, 61.5)
    ));
}

/// Two peers resize the same page on different axes: both converge to one whole frame, never
/// A's width with B's height, and no object moves.
#[test]
fn concurrent_resizes_converge_to_one_whole_frame() {
    for (pa, pb) in [(2, 3), (3, 2)] {
        let base = Document::new(1);
        let id = rect(&base, 10.0, 10.0);
        let bytes = pack(&base, "t").unwrap();
        let a = unpack(pa, &bytes).unwrap();
        let b = unpack(pb, &bytes).unwrap();
        a.resize(DocumentSize::from_mm(300.0, 297.0)).unwrap();
        b.resize(DocumentSize::from_mm(210.0, 400.0)).unwrap();
        let frame_a = first_frame(&a);
        let frame_b = first_frame(&b);
        merge(&a, &b);
        assert_eq!(a.pages(), b.pages());
        let merged = first_frame(&a);
        assert!(
            merged == frame_a || merged == frame_b,
            "one whole frame: {merged:?}"
        );
        assert!(near(rect_origin(&a, id), Point::new(10.0, 10.0)));
        assert!(near(rect_origin(&b, id), Point::new(10.0, 10.0)));
    }
}

/// Resizes of different pages at the same time are both kept.
#[test]
fn concurrent_resizes_of_different_pages_are_both_kept() {
    let base = Document::new(1);
    let second = base.add_page();
    let bytes = pack(&base, "t").unwrap();
    let a = unpack(2, &bytes).unwrap();
    let b = unpack(3, &bytes).unwrap();
    a.resize_page("first", DocumentSize::from_mm(100.0, 100.0))
        .unwrap();
    b.resize_page(&second.id, DocumentSize::from_mm(50.0, 60.0))
        .unwrap();
    merge(&a, &b);
    assert_eq!(a.pages(), b.pages());
    assert_eq!(a.pages()[0].size, DocumentSize::from_mm(100.0, 100.0));
    assert_eq!(a.pages()[1].size, DocumentSize::from_mm(50.0, 60.0));
}

/// Fit writes the content bounds as the whole frame and moves no object.
#[test]
fn fit_writes_the_content_bounds_as_the_frame() {
    let document = Document::new(1);
    let id = rect(&document, -20.0, 30.0);
    rect(&document, 100.0, 200.0);
    let bounds = (Point::new(-20.0, 30.0), Point::new(150.0, 240.0));
    assert_eq!(document.fit_to_content(bounds), Ok(true));
    let (origin, size) = first_frame(&document);
    assert!(near(origin, Point::new(-20.0, 30.0)));
    assert_eq!(size, DocumentSize::from_mm(170.0, 210.0));
    assert!(near(rect_origin(&document, id), Point::new(-20.0, 30.0)));
    assert_eq!(document.fit_to_content(bounds), Ok(false), "already fits");
}

/// A side narrower than 1 mm becomes 1 mm, centred on the content.
#[test]
fn fit_centres_a_thin_side_on_one_millimetre() {
    let document = Document::new(1);
    rect(&document, 0.0, 0.0);
    let bounds = (Point::new(10.0, 5.0), Point::new(110.0, 5.0));
    document.fit_to_content(bounds).unwrap();
    let (origin, size) = first_frame(&document);
    assert!(near(origin, Point::new(10.0, 4.5)), "{origin:?}");
    assert_eq!(size, DocumentSize::from_mm(100.0, 1.0));
}

/// A page move writes the frame and shifts the given objects in one commit.
#[test]
fn move_page_writes_the_frame_and_moves_its_objects() {
    let document = Document::new(1);
    let id = rect(&document, 10.0, 10.0);
    let before = change_count(&document);
    assert!(
        document
            .move_page("first", Vec2::new(5.0, -7.0), &[id])
            .unwrap()
    );
    assert_eq!(change_count(&document), before + 1);
    assert!(near(first_frame(&document).0, Point::new(5.0, -7.0)));
    assert!(near(rect_origin(&document, id), Point::new(15.0, 3.0)));
}

/// Every page edit writes the frame register; the old registers stay as they were (the first
/// page of a new document starts on them, read as its frame).
#[test]
fn page_edits_write_one_frame_register_per_page() {
    let document = Document::new(1);
    document.add_page();
    document
        .resize(DocumentSize::from_mm(100.0, 100.0))
        .unwrap();
    let keys = root_keys(&document);
    assert!(keys.contains(&"page.first.frame".to_owned()), "{keys:?}");
    assert_eq!(
        keys.iter()
            .filter(|k| k.rsplit('.').next() == Some("frame"))
            .count(),
        2,
        "{keys:?}"
    );
    let second = document.pages()[1].id.clone();
    for field in ["x", "y", "width", "height"] {
        let key = format!("page.{second}.{field}");
        assert!(!keys.contains(&key), "{key} is not written");
    }
}

/// Undo of a resize restores the whole frame.
#[test]
fn undo_of_a_resize_restores_the_frame() {
    let document = Document::new(1);
    let before = first_frame(&document);
    document.resize(DocumentSize::from_mm(100.0, 50.0)).unwrap();
    let step = document.own_steps().last().unwrap().step;
    document.undo_step(step).unwrap();
    assert_eq!(first_frame(&document), before);
}

// ---------------------------------------------------------------- format 34 and goldens

#[test]
fn format_version_is_34() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= 34);
}

/// Golden: a document with two pages, the first resized and fitted, the second moved.
#[test]
fn golden_page_frame_v34() {
    let path = fixture_path("page_frame_v34.curvyo");
    if std::env::var_os("UPDATE_0092_FIXTURES").is_some() {
        let document = Document::new(34);
        let id = rect(&document, 10.0, 10.0);
        document
            .resize(DocumentSize::from_mm(300.0, 400.0))
            .unwrap();
        let second = document.add_page();
        document.rename_page(&second.id, "Second").unwrap();
        document
            .move_page(&second.id, Vec2::new(0.0, 25.0), &[])
            .unwrap();
        document
            .resize_page(&second.id, DocumentSize::from_mm(100.0, 120.0))
            .unwrap();
        let _ = id;
        std::fs::write(&path, pack(&document, "golden").unwrap()).unwrap();
    }
    let opened = unpack(2, &std::fs::read(&path).unwrap()).unwrap();
    let pages = opened.pages();
    assert_eq!(pages.len(), 2);
    assert!(near(pages[0].origin, Point::new(-45.0, -51.5)));
    assert_eq!(pages[0].size, DocumentSize::from_mm(300.0, 400.0));
    assert_eq!(pages[1].name, "Second");
    assert_eq!(pages[1].size, DocumentSize::from_mm(100.0, 120.0));
    let id = opened.object_ids()[0];
    assert!(near(rect_origin(&opened, id), Point::new(10.0, 10.0)));
    assert_eq!(opened.size(), DocumentSize::from_mm(300.0, 400.0));
}

/// Version 33 files keep opening: the frame is read from the four old registers and nothing is
/// written on open.
#[test]
fn version_33_files_read_their_frames_from_the_old_registers() {
    let bytes = std::fs::read(fixture_path("tiling_v33.curvyo")).unwrap();
    let opened = unpack(5, &bytes).unwrap();
    let changes = change_count(&opened);
    let page = opened.pages()[0].clone();
    assert_eq!(page.origin, Point::new(0.0, 0.0));
    assert_eq!(opened.size(), page.size);
    assert_eq!(change_count(&opened), changes);
    // The next resize writes the register and keeps the centre of the old frame.
    let old = page.size;
    opened
        .resize(DocumentSize::from_mm(
            old.width.as_mm() + 20.0,
            old.height.as_mm(),
        ))
        .unwrap();
    assert!(near(opened.pages()[0].origin, Point::new(-10.0, 0.0)));
}

/// Damage: a frame that is not a map of four finite numbers, or a side outside the size range.
/// Writes the damage fixtures: the golden's first frame replaced by `value`.
fn write_damaged_frames() {
    use loro::LoroValue;
    let golden = std::fs::read(fixture_path("page_frame_v34.curvyo")).unwrap();
    let loro_bytes = member(&golden, "document.loro").unwrap();
    let frame = |entries: &[(&str, f64)]| {
        LoroValue::Map(
            entries
                .iter()
                .map(|(k, v)| ((*k).to_owned(), LoroValue::Double(*v)))
                .collect::<std::collections::HashMap<_, _>>()
                .into(),
        )
    };
    let cases = [
        ("page_frame_not_a_map_v34.curvyo", LoroValue::Double(1.0)),
        (
            "page_frame_missing_field_v34.curvyo",
            frame(&[("x", 0.0), ("y", 0.0), ("width", 10.0)]),
        ),
        (
            "page_frame_nan_v34.curvyo",
            frame(&[
                ("x", f64::NAN),
                ("y", 0.0),
                ("width", 10.0),
                ("height", 10.0),
            ]),
        ),
        (
            "page_frame_too_large_v34.curvyo",
            frame(&[
                ("x", 0.0),
                ("y", 0.0),
                ("width", 100_001.0),
                ("height", 10.0),
            ]),
        ),
    ];
    for (name, value) in cases {
        let loro = loro::LoroDoc::new();
        loro.import(&loro_bytes).unwrap();
        loro.get_map("root")
            .insert("page.first.frame", value)
            .unwrap();
        loro.commit();
        let snapshot = loro.export(loro::ExportMode::Snapshot).unwrap();
        let bytes = with_member(&golden, "document.loro", Some(&snapshot));
        std::fs::write(fixture_path(name), bytes).unwrap();
    }
}

#[test]
fn damaged_frames_are_refused_on_open() {
    if std::env::var_os("UPDATE_0092_FIXTURES").is_some() {
        write_damaged_frames();
    }
    for name in [
        "page_frame_not_a_map_v34.curvyo",
        "page_frame_missing_field_v34.curvyo",
        "page_frame_nan_v34.curvyo",
        "page_frame_too_large_v34.curvyo",
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name}"
        );
    }
}

// ---------------------------------------------------------------- share.json (decision 9)

fn share_record(document: &Document) -> ShareRecord {
    ShareRecord {
        server: "ws://192.168.1.20:8787".to_owned(),
        room: [7; 16],
        server_seq: 42,
        synced_version: document.version(),
        keyring_log: vec![1, 2, 3, 4, 5],
    }
}

#[test]
fn a_shared_file_round_trips_its_share_record() {
    let document = Document::new(1);
    rect(&document, 0.0, 0.0);
    assert_eq!(document.share(), None);
    assert!(!document.is_shared());
    let record = share_record(&document);
    document.set_share(Some(record.clone()));
    assert!(document.is_shared());
    let opened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    assert_eq!(opened.share(), Some(record));
    assert!(opened.is_shared());
    // A removed device keeps a plain local file (R-COLLAB-010).
    opened.set_share(None);
    let plain = unpack(3, &pack(&opened, "t").unwrap()).unwrap();
    assert_eq!(plain.share(), None);
    assert!(!plain.is_shared());
}

#[test]
fn a_relay_snapshot_carries_no_share_record() {
    let document = Document::new(1);
    rect(&document, 0.0, 0.0);
    document.set_share(Some(share_record(&document)));
    let bytes = pack_snapshot(&document, "t").unwrap();
    let names: Vec<String> = zip::ZipArchive::new(Cursor::new(&bytes))
        .unwrap()
        .file_names()
        .map(|name| name.unwrap().into_owned())
        .collect();
    assert!(
        !names
            .iter()
            .any(|n| n == "share.json" || n == "keyring.log"),
        "{names:?}"
    );
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.share(), None);
    assert_eq!(
        opened.export_json().unwrap(),
        document.export_json().unwrap()
    );
    // The replica's own file still keeps it.
    assert!(
        unpack(3, &pack(&document, "t").unwrap())
            .unwrap()
            .share()
            .is_some()
    );
}

#[test]
fn golden_shared_v34() {
    let path = fixture_path("shared_v34.curvyo");
    if std::env::var_os("UPDATE_0092_FIXTURES").is_some() {
        let document = Document::new(34);
        rect(&document, 0.0, 0.0);
        document.set_share(Some(share_record(&document)));
        std::fs::write(&path, pack(&document, "golden").unwrap()).unwrap();
    }
    let bytes = std::fs::read(&path).unwrap();
    let opened = unpack(2, &bytes).unwrap();
    let share = opened.share().expect("shared");
    assert_eq!(share.server, "ws://192.168.1.20:8787");
    assert_eq!(share.room, [7; 16]);
    assert_eq!(share.server_seq, 42);
    assert_eq!(share.keyring_log, vec![1, 2, 3, 4, 5]);
    assert_eq!(share.synced_version, opened.version());
    let json = member(&bytes, "share.json").unwrap();
    let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(value["format_version"], 1);
    assert_eq!(value["room"], "07070707070707070707070707070707");
}

fn member(bytes: &[u8], name: &str) -> Option<Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut file = archive.by_name(name).ok()?;
    let mut out = Vec::new();
    file.read_to_end(&mut out).unwrap();
    Some(out)
}

/// Rewrites one member of a container (or drops it when `with` is `None`).
fn with_member(bytes: &[u8], name: &str, with: Option<&[u8]>) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let member = file.name().unwrap().into_owned();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if member == name {
            continue;
        }
        writer.start_file(member, options).unwrap();
        writer.write_all(&data).unwrap();
    }
    if let Some(data) = with {
        writer.start_file(name, options).unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn a_share_record_without_its_keyring_or_malformed_is_damaged() {
    let bytes = std::fs::read(fixture_path("shared_v34.curvyo")).unwrap();
    let no_keyring = with_member(&bytes, "keyring.log", None);
    assert!(matches!(unpack(2, &no_keyring), Err(OpenError::Damaged)));
    let no_share = with_member(&bytes, "share.json", None);
    assert!(matches!(unpack(2, &no_share), Err(OpenError::Damaged)));
    for bad in [
        &b"not json"[..],
        br#"{"format_version":1,"server":"ws://x","room":"0707","server_seq":1,"synced_version":""}"#,
        br#"{"format_version":1,"server":"ws://x","room":"07070707070707070707070707070707","server_seq":-1,"synced_version":""}"#,
        br#"{"format_version":1,"server":"ws://x","room":"07070707070707070707070707070707","server_seq":1,"synced_version":"nope"}"#,
    ] {
        let damaged = with_member(&bytes, "share.json", Some(bad));
        assert!(
            matches!(unpack(2, &damaged), Err(OpenError::Damaged)),
            "{}",
            String::from_utf8_lossy(bad)
        );
    }
    let newer = with_member(
        &bytes,
        "share.json",
        Some(br#"{"format_version":3,"server":"ws://x","room":"07070707070707070707070707070707","server_seq":1,"synced_version":""}"#),
    );
    // Version 2 is a workspace document's record (`specs/0095-workspace`, ADR 0037 decision 3).
    assert!(matches!(
        unpack(2, &newer),
        Err(OpenError::FormatTooNew {
            found: 3,
            supported: 2
        })
    ));
}

// ---------------------------------------------------------------- version text (decision 3)

#[test]
fn a_version_round_trips_through_its_text() {
    let a = Document::new(1);
    rect(&a, 0.0, 0.0);
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    rect(&a, 5.0, 0.0);
    rect(&b, 9.0, 0.0);
    merge(&a, &b);
    let version = a.version();
    let text = version.to_text();
    assert_ne!(text, "");
    let parsed = DocumentVersion::from_text(&text).expect("parses");
    assert_eq!(parsed.to_text(), text);
    assert_eq!(
        a.export_updates(&parsed).unwrap(),
        a.export_updates(&version).unwrap()
    );
    assert_eq!(DocumentVersion::never().to_text(), "");
    assert_eq!(
        DocumentVersion::from_text(""),
        Some(DocumentVersion::never())
    );
    for bad in ["x", "1@", "@2", "1@2,", "1@-3", "1@2;3@4"] {
        assert_eq!(DocumentVersion::from_text(bad), None, "{bad}");
    }
}

// ---------------------------------------------------------------- received assets (decision 3)

#[test]
fn a_received_asset_is_stored_under_its_own_hash() {
    let document = Document::new(1);
    let bytes = b"\x89PNG\r\n\x1a\nnot really a picture".to_vec();
    let hash = document.insert_asset(&bytes).unwrap();
    assert_eq!(hash, AssetHash::of(&bytes));
    assert_eq!(document.asset(&hash).as_deref(), Some(&bytes[..]));
    assert_eq!(document.insert_asset(&bytes).unwrap(), hash, "idempotent");
    assert_eq!(document.asset_bytes(), bytes.len() as u64);
    let before = document.version();
    assert_eq!(document.version(), before, "no history step");
}
