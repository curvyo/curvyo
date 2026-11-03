//! A shared workspace's live tree between two members through one relay (`specs/0095-workspace`
//! criteria 35 to 41): A's creations, renames, moves and deletions reach B as file operations
//! and notices; a deleted document is restored from its kept copy; files outside the list and
//! missing files show as such; Download again reads the room anew.

use std::collections::HashMap;

use curvyo_document_core::{Created, Document, ScannedDocument, pack, unpack};

use super::Session;
use super::align_test_support::draw_rect;
use super::workspace_sync::{BackgroundKind, DocumentPlace};
use super::workspace_sync_tests::{LEFT, LID, WORKSPACE, joined, objects, p};
use super::workspace_tree::{DiskScan, DiskStep, ListEdit};
use crate::{BackgroundWriter, Waiting};

const TOP: [u8; 16] = [0x33; 16];

fn stamp(n: u8) -> Created {
    Created {
        at: 1_800_000_000,
        id: [n; 16],
    }
}

/// B's folder as written by the join: Lid and Sides/Left at their places, known there.
fn joined_disk() -> DiskScan {
    let mut known = HashMap::new();
    known.insert(LID, p("Lid.curvyo"));
    known.insert(LEFT, p("Sides/Left.curvyo"));
    DiskScan {
        folders: vec![p("Sides")],
        documents: vec![
            ScannedDocument {
                path: p("Lid.curvyo"),
                document: Some(LID),
            },
            ScannedDocument {
                path: p("Sides/Left.curvyo"),
                document: Some(LEFT),
            },
        ],
        known,
    }
}

fn notices(session: &mut Session) -> Vec<String> {
    std::iter::from_fn(|| session.take_sync_notice().map(|(text, _)| text)).collect()
}

/// Criterion 35: A creates a document; B's list names it, B's plan writes it, and B's
/// background writer builds its file from the records of its stream.
#[test]
fn a_document_another_member_creates_is_written_from_its_records() {
    let (mut net, a, b, _) = joined();
    let mut top = Session::new(0xa7);
    draw_rect(&mut top, (1.0, 1.0), (9.0, 9.0));
    let bytes = pack(&top.document, "test").unwrap();
    net.s(a)
        .workspace_list_edit(
            ListEdit::CreateDocument {
                path: p("Top.curvyo"),
                document: TOP,
            },
            stamp(1),
        )
        .unwrap();
    let adopted = net.s(a).workspace_adopt_document(TOP, &bytes).unwrap();
    assert_eq!(
        unpack(1, &adopted)
            .unwrap()
            .workspace_share()
            .unwrap()
            .document,
        TOP
    );
    net.run(500);
    assert!(notices(net.s(b)).is_empty(), "a creation shows no notice");
    let answer = net
        .s(b)
        .workspace_disk_plan(&joined_disk(), || stamp(9))
        .unwrap();
    assert_eq!(
        answer.steps,
        vec![DiskStep::Write {
            path: p("Top.curvyo"),
            document: TOP
        }]
    );
    assert_eq!(
        net.s(b).workspace_document_place(TOP),
        Some(DocumentPlace::File(p("Top.curvyo")))
    );
    let mut writer = BackgroundWriter::new(0xbb);
    for item in net.s(b).take_background() {
        assert_eq!(item.document, TOP);
        writer.add(
            item.document,
            Waiting {
                seq: item.seq,
                kind: item.kind,
                bytes: item.bytes,
            },
        );
    }
    let (file, through) = writer.create(TOP, WORKSPACE, 0).unwrap();
    net.s(b).background_written(TOP, through);
    let opened = Session::open(0xbc, &file).unwrap();
    assert_eq!(objects(&opened), 1);
    assert_eq!(
        opened.document.workspace_share().unwrap().workspace,
        WORKSPACE
    );
}

/// Criteria 36 and 37: A renames Lid and moves Left to the top; B's plan moves the files and
/// B is told who did it (`ux-notes.md` section 7: A's presence names its list replica).
#[test]
fn renames_and_moves_reach_the_other_disk_with_a_notice() {
    let (mut net, a, b, _) = joined();
    // One presence heartbeat (2 s) tells B the list replica of A.
    net.run(2_500);
    net.s(a)
        .workspace_list_edit(
            ListEdit::Move {
                from: p("Lid.curvyo"),
                to: p("Cover.curvyo"),
            },
            stamp(1),
        )
        .unwrap();
    net.s(a)
        .workspace_list_edit(
            ListEdit::Move {
                from: p("Sides/Left.curvyo"),
                to: p("Left.curvyo"),
            },
            stamp(2),
        )
        .unwrap();
    net.run(500);
    let shown = notices(net.s(b));
    assert!(
        shown.contains(&"Anna renamed Lid to Cover.".to_owned()),
        "{shown:?}"
    );
    assert!(
        shown.contains(&"Anna moved Left to Box job.".to_owned()),
        "{shown:?}"
    );
    let answer = net
        .s(b)
        .workspace_disk_plan(&joined_disk(), || stamp(9))
        .unwrap();
    assert!(answer.steps.contains(&DiskStep::Move {
        from: p("Lid.curvyo"),
        to: p("Cover.curvyo")
    }));
    assert!(answer.steps.contains(&DiskStep::Move {
        from: p("Sides/Left.curvyo"),
        to: p("Left.curvyo")
    }));
    assert!(answer.settled.is_empty(), "no file is at its new place yet");
}

/// Criterion 40: A deletes Lid; B is told who deleted it, B's plan keeps a copy and trashes
/// the file, the Deleted list names it, a snapshot still carries it; B restores it and A's
/// plan writes it again from its kept copy.
#[test]
fn a_deletion_reaches_everyone_and_any_member_restores_it() {
    let (mut net, a, b, _) = joined();
    net.s(a)
        .workspace_list_edit(
            ListEdit::Delete {
                path: p("Lid.curvyo"),
                at: 1_800_000_100,
            },
            stamp(1),
        )
        .unwrap();
    net.run(500);
    assert_eq!(notices(net.s(b)), vec!["Anna deleted Lid.".to_owned()]);
    let answer = net
        .s(b)
        .workspace_disk_plan(&joined_disk(), || stamp(9))
        .unwrap();
    assert_eq!(
        answer.steps,
        vec![DiskStep::TrashDocument {
            path: p("Lid.curvyo"),
            document: LID,
            by: "Anna".to_owned()
        }]
    );
    let deleted = net.s(b).workspace_deleted();
    assert_eq!(deleted.len(), 1);
    assert_eq!(
        (deleted[0].name.as_str(), deleted[0].by.as_str()),
        ("Lid", "Anna")
    );
    assert_eq!(
        net.s(b).workspace_document_place(LID),
        Some(DocumentPlace::Deleted)
    );
    assert!(
        net.s(b)
            .workspace_snapshot_documents()
            .contains(&(LID, DocumentPlace::Deleted)),
        "a deleted document stays in every snapshot"
    );

    net.s(b)
        .workspace_list_edit(
            ListEdit::Restore {
                node: deleted[0].node,
            },
            stamp(2),
        )
        .unwrap();
    net.run(500);
    let mut a_disk = joined_disk();
    a_disk.documents.remove(0);
    // A trashed Lid's file: its index names it no longer.
    a_disk.known.remove(&LID);
    let answer = net.s(a).workspace_disk_plan(&a_disk, || stamp(8)).unwrap();
    assert_eq!(
        answer.steps,
        vec![DiskStep::Write {
            path: p("Lid.curvyo"),
            document: LID
        }]
    );
    assert_eq!(net.s(a).workspace_deleted(), Vec::new());
}

/// Criterion 41: a file another program put into the folder is not in the shared workspace;
/// a document whose file another program removed is missing, not deleted; a file another
/// program moved moves in the list.
#[test]
fn outside_missing_and_externally_moved_files_are_told_apart() {
    let (mut net, a, b, _) = joined();
    let mut disk = joined_disk();
    disk.documents[1].path = p("Left.curvyo");
    disk.documents.remove(0);
    disk.documents.push(ScannedDocument {
        path: p("Stray.curvyo"),
        document: None,
    });
    let answer = net.s(b).workspace_disk_plan(&disk, || stamp(9)).unwrap();
    assert_eq!(answer.outside, vec![p("Stray.curvyo")]);
    assert_eq!(answer.missing, vec![p("Lid.curvyo")]);
    assert!(answer.steps.is_empty(), "{:?}", answer.steps);
    net.run(500);
    assert_eq!(
        net.s(a).workspace_document_place(LEFT),
        Some(DocumentPlace::File(p("Left.curvyo"))),
        "B's external move went to the list"
    );
}

/// Criterion 41, Download again: the room is read again from its snapshot, so every document's
/// state arrives once more for the background writer.
#[test]
fn download_again_reads_the_room_from_its_snapshot() {
    let (mut net, _a, b, _) = joined();
    assert_eq!(net.s(b).take_background(), Vec::new());
    net.s(b).workspace_download_again();
    net.run(1_000);
    let items = net.s(b).take_background();
    assert!(
        items
            .iter()
            .any(|i| i.document == LID && i.kind == BackgroundKind::Container),
        "{items:?}"
    );
    let empty = pack(&Document::new(3), "t").unwrap();
    assert!(Session::open(4, &empty).is_ok());
}
