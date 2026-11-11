//! Independent width points survive concurrent edits and save/reopen.
#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::assert_is_empty
)]

use curvyo_document_core::{AnchorId, Document, NewAnchor, Point, pack, unpack};

fn path(doc: &Document) -> curvyo_document_core::NodeId {
    doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 0.0)),
        ],
        false,
    )
}

#[test]
fn concurrent_first_points_survive_in_both_merge_directions_and_reopen() {
    let a = Document::new(1);
    let path = path(&a);
    let b = unpack(2, &pack(&a, "test").unwrap()).unwrap();
    a.set_width_point(path, AnchorId::new(1, 9), 0.25, 2.0)
        .unwrap();
    b.set_width_point(path, AnchorId::new(2, 9), 0.75, 0.5)
        .unwrap();
    let av = a.version();
    let bv = b.version();
    let au = a.export_updates(&bv).unwrap();
    let bu = b.export_updates(&av).unwrap();
    a.import_updates(&bu).unwrap();
    b.import_updates(&au).unwrap();
    let points = a.width_points(path).unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points, b.width_points(path).unwrap());
    let reopened = unpack(3, &pack(&a, "test").unwrap()).unwrap();
    assert_eq!(points, reopened.width_points(path).unwrap());
}

#[test]
fn invalid_point_leaves_document_unchanged() {
    let doc = Document::new(1);
    let path = path(&doc);
    let before = doc.version();
    assert!(
        doc.set_width_point(path, AnchorId::new(1, 4), f64::NAN, 1.0)
            .is_err()
    );
    assert!(
        doc.set_width_point(path, AnchorId::new(1, 4), 0.5, -1.0)
            .is_err()
    );
    assert_eq!(before, doc.version());
}

#[test]
fn point_edit_and_delete_are_undoable() {
    let doc = Document::new(1);
    let path = path(&doc);
    let id = AnchorId::new(1, 4);
    doc.set_width_point(path, id, 0.5, 2.0).unwrap();
    let step = doc.own_steps().last().unwrap().step;
    doc.undo_step(step).unwrap();
    assert!(doc.width_points(path).unwrap().is_empty());
    doc.redo_step(step).unwrap();
    assert_eq!(doc.width_points(path).unwrap().len(), 1);
    doc.remove_width_point(path, id).unwrap();
    let step = doc.own_steps().last().unwrap().step;
    assert!(doc.width_points(path).unwrap().is_empty());
    doc.undo_step(step).unwrap();
    assert_eq!(doc.width_points(path).unwrap().len(), 1);
}

#[test]
fn duplicate_keeps_profile_and_edits_are_independent() {
    use curvyo_document_core::{CopySource, Vec2};
    let doc = Document::new(1);
    let original = path(&doc);
    let id = AnchorId::new(1, 9);
    doc.set_width_point(original, id, 0.4, 2.0).unwrap();
    let copies = doc
        .duplicate_objects(
            &[CopySource {
                id: original,
                anchor_ids: vec![AnchorId::new(2, 1), AnchorId::new(2, 2)],
            }],
            Vec2::ZERO,
        )
        .unwrap();
    let copy = copies[0];
    assert_eq!(
        doc.width_points(original).unwrap(),
        doc.width_points(copy).unwrap()
    );
    doc.set_width_point(copy, id, 0.4, 3.0).unwrap();
    assert_eq!(doc.width_points(original).unwrap()[0].multiplier, 2.0);
    assert_eq!(doc.width_points(copy).unwrap()[0].multiplier, 3.0);
}

#[test]
fn profile_presence_follows_add_and_remove() {
    let doc = Document::new(1);
    let path = path(&doc);
    let id = AnchorId::new(1, 9);
    assert!(!doc.has_extended_path_history());
    doc.set_width_point(path, id, 0.4, 2.0).unwrap();
    assert!(doc.has_extended_path_history());
    doc.remove_width_point(path, id).unwrap();
    assert!(
        doc.has_extended_path_history(),
        "historical profile data still exists"
    );
}

#[test]
fn profile_history_marker_survives_undo_delete_and_reopen() {
    let doc = Document::new(1);
    let path = path(&doc);
    doc.set_width_point(path, AnchorId::new(1, 9), 0.4, 2.0)
        .unwrap();
    let step = doc.own_steps().last().unwrap().step;
    doc.undo_step(step).unwrap();
    assert!(doc.has_extended_path_history());
    doc.delete_objects(&[path]).unwrap();
    assert!(doc.has_extended_path_history());
    let reopened = unpack(2, &pack(&doc, "test").unwrap()).unwrap();
    assert!(reopened.has_extended_path_history());
}

#[test]
fn shared_document_rejects_profile_edits_without_changing_data() {
    use curvyo_document_core::{DocumentVersion, WorkspaceShareRecord};
    let doc = Document::new(1);
    let path = path(&doc);
    doc.set_workspace_share(Some(WorkspaceShareRecord {
        workspace: [1; 16],
        document: [2; 16],
        synced_version: DocumentVersion::never(),
    }));
    let before = doc.version();
    assert!(
        doc.set_width_point(path, AnchorId::new(1, 9), 0.4, 2.0)
            .is_err()
    );
    assert_eq!(doc.version(), before);
}

#[test]
fn explicitly_marked_shared_document_refuses_new_profile() {
    let doc = Document::new(28);
    let id = path(&doc);
    doc.mark_shared();
    let before = doc.version();
    assert!(
        doc.set_width_point(id, AnchorId::new(28, 3), 0.5, 2.0)
            .is_err()
    );
    assert_eq!(doc.version(), before);
}

#[test]
fn shared_import_rejects_new_feature_history_without_mutation() {
    let source = Document::new(29);
    let id = path(&source);
    let peer = unpack(30, &pack(&source, "test").unwrap()).unwrap();
    let before = peer.version();
    source
        .set_width_point(id, AnchorId::new(29, 3), 0.5, 2.0)
        .unwrap();
    let update = source.export_updates(&before).unwrap();
    assert!(peer.import_shared_updates(&update).is_err());
    assert_eq!(peer.version(), before);
    assert!(peer.width_points(id).unwrap().is_empty());
    peer.import_updates(&update).unwrap();
    assert_eq!(peer.width_points(id).unwrap().len(), 1);
}

/// The copy remote updates are checked on is kept between imports: it follows the local edits
/// made in between, and a refused update neither reaches the document nor spoils the next check.
#[test]
fn kept_trial_copy_follows_local_edits_and_refusals() {
    let source = Document::new(33);
    let first = path(&source);
    let peer = unpack(34, &pack(&source, "test").unwrap()).unwrap();
    peer.set_live_share(true);
    let mut seen = peer.version();
    let mut send = |source: &Document, peer: &Document, shared: bool| {
        let update = source.export_updates(&seen).unwrap();
        let result = if shared {
            peer.import_shared_updates(&update)
        } else {
            peer.import_updates(&update)
        };
        if result.is_ok() {
            seen = source.version();
        }
        result
    };
    let second = path(&source);
    send(&source, &peer, true).unwrap();
    let local = path(&peer);
    let third = path(&source);
    send(&source, &peer, true).unwrap();
    source
        .set_width_point(first, AnchorId::new(33, 9), 0.5, 2.0)
        .unwrap();
    let before = peer.version();
    assert!(send(&source, &peer, true).is_err());
    assert_eq!(peer.version(), before);
    send(&source, &peer, false).unwrap();
    for id in [first, second, third, local] {
        assert!(peer.object(id).is_some());
    }
    assert_eq!(peer.width_points(first).unwrap().len(), 1);
    assert_eq!(peer.object_ids().len(), 4);
}

#[test]
fn invalid_incoming_marker_does_not_mutate_live_document() {
    use loro::{ExportMode, LoroDoc};
    let doc = Document::new(31);
    let _path = path(&doc);
    // Ask an unknown replica for the entire history, then construct an invalid update.
    let unknown = Document::new(44).version();
    let initial = doc.export_updates(&unknown).unwrap();
    assert!(initial.len() > 4);
    let raw = LoroDoc::new();
    raw.import(&initial[4..]).unwrap();
    let since = raw.oplog_vv();
    raw.set_peer_id(99).unwrap();
    raw.get_map("root")
        .insert("extended_path_history", "invalid")
        .unwrap();
    raw.commit();
    let update = raw.export(ExportMode::updates(&since)).unwrap();
    let mut framed = vec![0, 0, 0, 0];
    framed.extend(update);
    let before = doc.version();
    assert!(doc.import_updates(&framed).is_err());
    assert_eq!(doc.version(), before);
}

#[test]
fn incomplete_incoming_updates_do_not_mutate_live_document() {
    use loro::{ExportMode, LoroDoc};
    let source = LoroDoc::new();
    source.set_peer_id(98).unwrap();
    source.get_map("root").insert("first", 1).unwrap();
    source.commit();
    let after_first = source.oplog_vv();
    source.get_map("root").insert("second", 2).unwrap();
    source.commit();
    let update = source.export(ExportMode::updates(&after_first)).unwrap();
    let mut framed = vec![0, 0, 0, 0];
    framed.extend(update);
    let doc = Document::new(32);
    let before = doc.version();
    assert!(doc.import_updates(&framed).is_err());
    assert_eq!(doc.version(), before);
}

#[test]
fn reset_removes_all_observed_points_in_one_undo_step() {
    let doc = Document::new(50);
    let id = path(&doc);
    doc.set_width_point(id, AnchorId::new(50, 1), 0.2, 2.0)
        .unwrap();
    doc.set_width_point(id, AnchorId::new(50, 2), 0.8, 0.5)
        .unwrap();
    let before = doc.width_points(id).unwrap();
    doc.reset_width_profile(id).unwrap();
    assert!(doc.width_points(id).unwrap().is_empty());
    assert!(doc.has_extended_path_history());
    doc.undo_step(doc.own_steps().last().unwrap().step).unwrap();
    assert_eq!(doc.width_points(id).unwrap(), before);
}

#[test]
fn reset_preserves_a_concurrently_added_point_in_both_merge_directions() {
    let a = Document::new(60);
    let id = path(&a);
    a.set_width_point(id, AnchorId::new(60, 1), 0.2, 2.0)
        .unwrap();
    let b = unpack(61, &pack(&a, "test").unwrap()).unwrap();
    a.reset_width_profile(id).unwrap();
    b.set_width_point(id, AnchorId::new(61, 1), 0.8, 0.5)
        .unwrap();
    let av = a.version();
    let bv = b.version();
    let au = a.export_updates(&bv).unwrap();
    let bu = b.export_updates(&av).unwrap();
    a.import_updates(&bu).unwrap();
    b.import_updates(&au).unwrap();
    let points = a.width_points(id).unwrap();
    assert_eq!(points, b.width_points(id).unwrap());
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].id, AnchorId::new(61, 1));
}

#[test]
fn incoming_newer_format_write_is_refused_even_if_overwritten() {
    use loro::{ExportMode, LoroDoc};
    let doc = Document::new(72);
    let _ = path(&doc);
    let unknown = Document::new(73).version();
    let initial = doc.export_updates(&unknown).unwrap();
    let raw = LoroDoc::new();
    raw.import(&initial[4..]).unwrap();
    let since = raw.oplog_vv();
    raw.set_peer_id(74).unwrap();
    let root = raw.get_map("root");
    root.insert(
        "format_version",
        i64::from(curvyo_document_core::CURRENT_FORMAT_VERSION) + 1,
    )
    .unwrap();
    raw.commit();
    root.insert(
        "format_version",
        i64::from(curvyo_document_core::CURRENT_FORMAT_VERSION),
    )
    .unwrap();
    raw.commit();
    let update = raw.export(ExportMode::updates(&since)).unwrap();
    let mut framed = vec![0, 0, 0, 0];
    framed.extend(update);
    let before = doc.version();
    assert!(doc.import_updates(&framed).is_err());
    assert_eq!(doc.version(), before);
}

#[test]
fn initial_update_stream_with_overwritten_newer_format_is_refused() {
    use loro::{ExportMode, LoroDoc};
    let doc = Document::new(75);
    let _ = path(&doc);
    let unknown = Document::new(76).version();
    let initial = doc.export_updates(&unknown).unwrap();
    let raw = LoroDoc::new();
    raw.import(&initial[4..]).unwrap();
    raw.set_peer_id(77).unwrap();
    let root = raw.get_map("root");
    root.insert(
        "format_version",
        i64::from(curvyo_document_core::CURRENT_FORMAT_VERSION) + 1,
    )
    .unwrap();
    raw.commit();
    root.insert(
        "format_version",
        i64::from(curvyo_document_core::CURRENT_FORMAT_VERSION),
    )
    .unwrap();
    raw.commit();
    let update = raw.export(ExportMode::all_updates()).unwrap();
    let mut framed = vec![0, 0, 0, 0];
    framed.extend(update);
    assert!(Document::from_updates(78, &framed).is_err());
}

#[test]
fn profile_marker_survives_history_wipe_and_reopen() {
    let mut doc = Document::new(79);
    let id = path(&doc);
    doc.set_width_point(id, AnchorId::new(79, 3), 0.5, 2.0)
        .unwrap();
    doc.wipe_history().unwrap();
    let reopened = unpack(80, &pack(&doc, "test").unwrap()).unwrap();
    assert!(reopened.has_extended_path_history());
    assert_eq!(reopened.width_points(id).unwrap().len(), 1);
}

#[test]
fn feature_undo_delete_wipe_and_reopen_keeps_history_marker() {
    let mut doc = Document::new(81);
    let id = path(&doc);
    doc.set_width_point(id, AnchorId::new(81, 3), 0.5, 2.0)
        .unwrap();
    doc.undo_step(doc.own_steps().last().unwrap().step).unwrap();
    doc.delete_objects(&[id]).unwrap();
    doc.wipe_history().unwrap();
    let reopened = unpack(82, &pack(&doc, "test").unwrap()).unwrap();
    assert!(reopened.has_extended_path_history());
}
