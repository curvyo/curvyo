//! Session-level tests of the recovery hand-off (`specs/0136-autosave-and-restore` criteria 1
//! and 5): the replica the worker holds packs to the session's document, a reseed sends
//! everything again, a wiped history asks for one, and assets are handed once.

use curvyo_document_core::{
    AssetHash, Document, Length, Point, RectBounds, assemble_recovery_copy, unpack,
};

use super::Session;
use crate::recovery_writer::{RecoveryWriter, RecoveryWriterError};

fn rect(session: &Session, x: f64) {
    let _ = session.document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    });
}

/// The page's hand-over: the session's next delta into the worker's replica.
fn take(writer: &mut RecoveryWriter, session: &mut Session) {
    let delta = session.recovery_export().unwrap();
    writer.apply(7, delta.full, &delta.bytes).unwrap();
}

fn objects(document: &Document) -> usize {
    document.read_objects().objects.len()
}

#[test]
fn deltas_build_a_replica_that_packs_to_the_sessions_document() {
    let mut session = Session::new(1);
    let mut writer = RecoveryWriter::new();
    assert_eq!(writer.pack().unwrap(), None);
    assert!(!session.recovery_changed(), "the first call only remembers");
    rect(&session, 0.0);
    assert!(session.recovery_changed());
    assert!(!session.recovery_changed(), "a step is reported once");
    take(&mut writer, &mut session);
    rect(&session, 10.0);
    rect(&session, 20.0);
    take(&mut writer, &mut session);
    let packed = writer.pack().unwrap().unwrap();
    assert_eq!(packed.pages, 1);
    let restored = unpack(9, &assemble_recovery_copy(&packed.bytes, &[]).unwrap()).unwrap();
    assert_eq!(objects(&restored), 3);
    assert_eq!(objects(&restored), objects(&session.document));
}

#[test]
fn a_restarted_worker_is_reseeded_with_everything() {
    let mut session = Session::new(1);
    rect(&session, 0.0);
    let mut writer = RecoveryWriter::new();
    take(&mut writer, &mut session);
    rect(&session, 10.0);
    let mut restarted = RecoveryWriter::new();
    session.recovery_reseed();
    assert!(!session.recovery_is_seeded());
    take(&mut restarted, &mut session);
    assert!(session.recovery_is_seeded());
    let packed = restarted.pack().unwrap().unwrap();
    let restored = unpack(9, &assemble_recovery_copy(&packed.bytes, &[]).unwrap()).unwrap();
    assert_eq!(objects(&restored), 2);
}

#[test]
fn a_wiped_history_asks_for_a_full_export() {
    let mut session = Session::new(1);
    rect(&session, 0.0);
    let mut writer = RecoveryWriter::new();
    take(&mut writer, &mut session);
    session.document.wipe_history().unwrap();
    let delta = session.recovery_export().unwrap();
    assert!(!delta.full);
    assert_eq!(
        writer.apply(7, delta.full, &delta.bytes),
        Err(RecoveryWriterError::Reseed)
    );
    assert!(!writer.is_seeded());
    session.recovery_reseed();
    take(&mut writer, &mut session);
    assert!(writer.is_seeded());
}

#[test]
fn an_asset_is_handed_to_the_host_once() {
    let mut session = Session::new(1);
    assert!(session.recovery_new_assets().is_empty());
    let mut picture = b"\x89PNG\r\n\x1a\n".to_vec();
    picture.extend_from_slice(b"pixels");
    session.document.insert_asset(&picture).unwrap();
    let hash = AssetHash::of(&picture);
    let listed = session.recovery_new_assets();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, format!("{}.png", hash.as_str()));
    session.recovery_assets_done(listed.iter().map(|a| a.name.clone()));
    assert!(session.recovery_new_assets().is_empty());
    session.recovery_start_over();
    assert_eq!(
        session.recovery_new_assets().len(),
        1,
        "a new key has none yet"
    );
}
