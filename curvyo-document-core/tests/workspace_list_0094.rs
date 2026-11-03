//! The shared document list of a workspace (`specs/0094-workspace` criteria 35 to 40, ADR 0036
//! decision 6): a replicated tree of folders and documents whose replicas converge under
//! concurrent creates, renames, moves, loops and deletions.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    Created, EntryKind, ListEntry, ListError, ListNodeId, NameRefusal,
    WORKSPACE_LIST_FORMAT_VERSION, WorkspaceList,
};

fn created(at: i64, tag: u8) -> Created {
    Created { at, id: [tag; 16] }
}

/// Each replica's updates imported into the other: the two have seen everything.
fn sync(a: &WorkspaceList, b: &WorkspaceList) -> (Vec<ListNodeId>, Vec<ListNodeId>) {
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    let dropped_b = b.import_updates(&to_b).unwrap().dropped_moves;
    let dropped_a = a.import_updates(&to_a).unwrap().dropped_moves;
    (dropped_a, dropped_b)
}

/// `(path, kind, document)` of every visible entry, sorted: what both replicas must agree on.
fn shape(list: &WorkspaceList) -> Vec<(String, EntryKind, Option<[u8; 16]>)> {
    let mut rows: Vec<_> = list
        .entries()
        .into_iter()
        .map(|entry: ListEntry| (entry.path.join("/"), entry.kind, entry.document))
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

fn find(list: &WorkspaceList, path: &str) -> ListNodeId {
    list.entries()
        .into_iter()
        .find(|entry| entry.path.join("/") == path)
        .unwrap_or_else(|| panic!("{path} in {:?}", shape(list)))
        .node
}

/// A replica B that starts from A's snapshot.
fn joined(a: &WorkspaceList, peer: u64) -> WorkspaceList {
    WorkspaceList::from_snapshot(peer, &a.snapshot().unwrap()).unwrap()
}

#[test]
fn a_list_holds_folders_and_documents_by_their_ids() {
    let list = WorkspaceList::new(1);
    let sides = list.create_folder(None, "Sides", created(1, 1)).unwrap();
    list.create_document(Some(sides), "Lid", [0xd1; 16], created(2, 2))
        .unwrap();
    list.create_document(None, "Base", [0xd2; 16], created(3, 3))
        .unwrap();
    assert_eq!(
        shape(&list),
        vec![
            ("Base".to_owned(), EntryKind::Document, Some([0xd2; 16])),
            ("Sides".to_owned(), EntryKind::Folder, None),
            (
                "Sides/Lid".to_owned(),
                EntryKind::Document,
                Some([0xd1; 16])
            ),
        ]
    );
    assert_eq!(
        list.find_document([0xd1; 16]),
        Some(find(&list, "Sides/Lid"))
    );
    assert_eq!(list.find_document([0xee; 16]), None);
}

/// Names are checked at input; a folder cannot move into itself.
#[test]
fn bad_names_and_local_loops_are_refused() {
    let list = WorkspaceList::new(1);
    assert_eq!(
        list.create_folder(None, "a/b", created(1, 1)),
        Err(ListError::Name(NameRefusal::Separator))
    );
    let outer = list.create_folder(None, "Outer", created(1, 1)).unwrap();
    let inner = list
        .create_folder(Some(outer), "Inner", created(2, 2))
        .unwrap();
    assert_eq!(list.move_to(outer, Some(inner)), Err(ListError::Loop));
    assert_eq!(list.move_to(outer, Some(outer)), Err(ListError::Loop));
    assert_eq!(
        list.rename(inner, ""),
        Err(ListError::Name(NameRefusal::Empty))
    );
    assert_eq!(shape(&list).len(), 2);
}

/// Criteria 35 and 36: a create, a rename and a move reach the other replica.
#[test]
fn creates_renames_and_moves_reach_the_other_replica() {
    let a = WorkspaceList::new(1);
    let b = joined(&a, 2);
    let sides = a.create_folder(None, "Sides", created(1, 1)).unwrap();
    let lid = a
        .create_document(None, "Lid", [0xd1; 16], created(2, 2))
        .unwrap();
    sync(&a, &b);
    assert_eq!(shape(&a), shape(&b));
    a.rename(lid, "Cover").unwrap();
    a.move_to(lid, Some(sides)).unwrap();
    sync(&a, &b);
    assert_eq!(shape(&b), shape(&a));
    assert_eq!(find(&b, "Sides/Cover"), lid);
}

/// Criterion 37: two moves of one document into different folders converge on one folder.
#[test]
fn concurrent_moves_of_one_document_converge() {
    let a = WorkspaceList::new(1);
    let left = a.create_folder(None, "Left", created(1, 1)).unwrap();
    let right = a.create_folder(None, "Right", created(2, 2)).unwrap();
    let lid = a
        .create_document(None, "Lid", [0xd1; 16], created(3, 3))
        .unwrap();
    let b = joined(&a, 2);
    a.move_to(lid, Some(left)).unwrap();
    b.move_to(lid, Some(right)).unwrap();
    let (dropped_a, dropped_b) = sync(&a, &b);
    assert_eq!(shape(&a), shape(&b));
    let lids: Vec<_> = shape(&a)
        .into_iter()
        .filter(|row| row.2 == Some([0xd1; 16]))
        .collect();
    assert_eq!(lids.len(), 1, "{lids:?}");
    // One wins without a message (no loop).
    assert!(dropped_a.is_empty() && dropped_b.is_empty());
}

/// Criterion 38: two documents of one name in one folder both stay; the later one, by the same
/// rule everywhere, is shown with " 2". Two renames of one document converge.
#[test]
fn concurrent_creates_of_one_name_both_stay_and_renames_converge() {
    let a = WorkspaceList::new(1);
    let b = joined(&a, 2);
    a.create_document(None, "Lid", [0xa1; 16], created(10, 1))
        .unwrap();
    b.create_document(None, "lid", [0xb1; 16], created(11, 2))
        .unwrap();
    sync(&a, &b);
    assert_eq!(shape(&a), shape(&b));
    assert_eq!(
        shape(&a),
        vec![
            ("Lid".to_owned(), EntryKind::Document, Some([0xa1; 16])),
            ("lid 2".to_owned(), EntryKind::Document, Some([0xb1; 16])),
        ]
    );
    let first = find(&a, "Lid");
    a.rename(first, "Cover").unwrap();
    b.rename(first, "Top").unwrap();
    sync(&a, &b);
    assert_eq!(shape(&a), shape(&b));
}

/// A folder and a document may share a name; the suffix counts siblings of one kind.
#[test]
fn a_folder_and_a_document_of_one_name_stand_side_by_side() {
    let list = WorkspaceList::new(1);
    list.create_folder(None, "Lid", created(1, 1)).unwrap();
    list.create_document(None, "Lid", [0xd1; 16], created(2, 2))
        .unwrap();
    assert_eq!(
        shape(&list)
            .into_iter()
            .map(|row| row.0)
            .collect::<Vec<_>>(),
        vec!["Lid", "Lid"]
    );
}

/// Criterion 39: two folder moves that together would close a loop converge on a loop-free tree;
/// every document stays, and the replica whose move was not applied says so.
#[test]
fn a_concurrent_loop_is_broken_the_same_way_on_both_sides() {
    let a = WorkspaceList::new(1);
    let first = a.create_folder(None, "First", created(1, 1)).unwrap();
    let second = a.create_folder(None, "Second", created(2, 2)).unwrap();
    a.create_document(Some(first), "One", [0xd1; 16], created(3, 3))
        .unwrap();
    a.create_document(Some(second), "Two", [0xd2; 16], created(4, 4))
        .unwrap();
    let b = joined(&a, 2);
    a.move_to(first, Some(second)).unwrap();
    b.move_to(second, Some(first)).unwrap();
    let (dropped_a, dropped_b) = sync(&a, &b);
    assert_eq!(shape(&a), shape(&b));
    let documents = shape(&a)
        .into_iter()
        .filter(|row| row.1 == EntryKind::Document)
        .count();
    assert_eq!(documents, 2);
    let dropped: Vec<_> = dropped_a.iter().chain(&dropped_b).collect();
    assert_eq!(dropped.len(), 1, "a: {dropped_a:?} b: {dropped_b:?}");
    assert!(*dropped[0] == first || *dropped[0] == second);
}

/// Criterion 40: a deletion hides the entry and everything below it on both sides, lists it as
/// deleted with who and when, and a restore brings it back with its id.
#[test]
fn a_deletion_is_a_tombstone_that_any_member_restores() {
    let a = WorkspaceList::new(1);
    let sides = a.create_folder(None, "Sides", created(1, 1)).unwrap();
    a.create_document(Some(sides), "Lid", [0xd1; 16], created(2, 2))
        .unwrap();
    let base = a
        .create_document(None, "Base", [0xd2; 16], created(3, 3))
        .unwrap();
    let b = joined(&a, 2);
    a.delete(sides, "ben", 100).unwrap();
    sync(&a, &b);
    assert_eq!(
        shape(&b),
        vec![("Base".to_owned(), EntryKind::Document, Some([0xd2; 16]))]
    );
    let deleted = b.deleted_entries();
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].node, sides);
    assert_eq!(deleted[0].name, "Sides");
    assert_eq!(deleted[0].by, "ben");
    assert_eq!(deleted[0].at, 100);
    assert_eq!(deleted[0].kind, EntryKind::Folder);
    // The tombstoned document keeps its id: its file can be written again from its stream.
    assert_eq!(b.find_document([0xd1; 16]), None);
    // A new "Sides" took the name meanwhile: the restored one, created first, keeps it.
    b.create_folder(None, "Sides", created(50, 9)).unwrap();
    b.restore(sides).unwrap();
    sync(&a, &b);
    assert_eq!(shape(&a), shape(&b));
    assert_eq!(find(&a, "Sides/Lid"), find(&b, "Sides/Lid"));
    assert!(shape(&a).iter().any(|row| row.0 == "Sides 2"));
    assert_eq!(a.deleted_entries(), Vec::new());
    a.delete(base, "ana", 200).unwrap();
    assert_eq!(a.deleted_entries()[0].document, Some([0xd2; 16]));
}

/// A restored entry under a folder that is still deleted comes back at the top.
#[test]
fn a_restore_under_a_deleted_folder_lands_at_the_top() {
    let list = WorkspaceList::new(1);
    let sides = list.create_folder(None, "Sides", created(1, 1)).unwrap();
    let lid = list
        .create_document(Some(sides), "Lid", [0xd1; 16], created(2, 2))
        .unwrap();
    list.delete(lid, "ana", 10).unwrap();
    list.delete(sides, "ana", 11).unwrap();
    list.restore(lid).unwrap();
    assert_eq!(
        shape(&list),
        vec![("Lid".to_owned(), EntryKind::Document, Some([0xd1; 16]))]
    );
}

/// Node ids round-trip through their text, which the page uses.
#[test]
fn node_ids_round_trip_through_text() {
    let list = WorkspaceList::new(7);
    let node = list.create_folder(None, "A", created(1, 1)).unwrap();
    assert_eq!(ListNodeId::from_text(&node.to_text()), Some(node));
    assert_eq!(ListNodeId::from_text("nope"), None);
}

/// Bytes that are no list, and a list of a newer format, are refused.
#[test]
fn foreign_or_newer_lists_are_refused() {
    assert_eq!(
        WorkspaceList::from_snapshot(1, b"nope").err(),
        Some(ListError::Malformed)
    );
    let list = WorkspaceList::new(1);
    assert_eq!(
        list.import_updates(b"nope").err(),
        Some(ListError::Malformed)
    );
    assert_eq!(WORKSPACE_LIST_FORMAT_VERSION, 1);
}

/// The golden `list.loro` (ADR 0036 decision 12); regenerate with `UPDATE_0094_FIXTURES=1`.
#[test]
fn golden_list_v1() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/list_v1.loro");
    if std::env::var_os("UPDATE_0094_FIXTURES").is_some() {
        let list = WorkspaceList::new(1);
        let sides = list.create_folder(None, "Sides", created(1, 1)).unwrap();
        list.create_document(Some(sides), "Lid", [0xd1; 16], created(2, 2))
            .unwrap();
        let gone = list
            .create_document(None, "Old", [0xd3; 16], created(3, 3))
            .unwrap();
        list.delete(gone, "ana", 9).unwrap();
        std::fs::write(&path, list.snapshot().unwrap()).unwrap();
    }
    let list = WorkspaceList::from_snapshot(5, &std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        shape(&list),
        vec![
            ("Sides".to_owned(), EntryKind::Folder, None),
            (
                "Sides/Lid".to_owned(),
                EntryKind::Document,
                Some([0xd1; 16])
            ),
        ]
    );
    let deleted = list.deleted_entries();
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].name, "Old");
    assert_eq!(deleted[0].created, created(3, 3));
}

/// Criterion 41: a document another program moved is placed in the list where its file is,
/// with the folders on the way made as needed.
#[test]
fn a_document_is_placed_where_another_program_moved_it() {
    let list = WorkspaceList::new(1);
    let lid = list
        .create_document(None, "Lid", [0xd1; 16], created(1, 1))
        .unwrap();
    let mut tag = 10;
    list.place(lid, &["Walls".to_owned(), "Top".to_owned()], || {
        tag += 1;
        created(i64::from(tag), tag)
    })
    .unwrap();
    assert_eq!(
        shape(&list),
        vec![
            ("Walls".to_owned(), EntryKind::Folder, None),
            (
                "Walls/Top".to_owned(),
                EntryKind::Document,
                Some([0xd1; 16])
            ),
        ]
    );
    // An existing folder is reused, matched like names.
    list.place(lid, &["walls".to_owned(), "Top".to_owned()], || {
        created(99, 99)
    })
    .unwrap();
    assert_eq!(shape(&list).len(), 2);
}
