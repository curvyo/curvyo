//! Tester's acceptance tests for `specs/0095-workspace` in `curvyo-document-core`, written from
//! the specification and the public API before reading the implementation: naming and local
//! plans (criteria 13 to 19), the shared list under concurrent edits (criteria 35 to 40), the
//! disk following the list without overwriting or losing a file (criteria 36 to 41), the
//! document identity in `share.json` version 2 (criterion 15) and the unchanged `.curvyo`
//! format (ADR 0037 decision 12).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::similar_names
)]

use std::collections::{BTreeMap, HashMap, HashSet};

use curvyo_document_core::{
    Created, DiskOp, DiskState, DocumentVersion, EntryKind, FileOp, ListEntry, ListError,
    ListNodeId, NameRefusal, PlanRefusal, ScannedDocument, WorkspaceList, WorkspacePath,
    WorkspaceShareRecord, WorkspaceTree, check_entry_name, free_entry_name, pack, plan_disk,
    same_entry_name, unpack,
};

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

fn p(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split('/').map(str::to_owned).collect()
}

fn stamp(at: i64, id: u8) -> Created {
    Created { at, id: [id; 16] }
}

fn doc(n: u8) -> [u8; 16] {
    let mut id = [0xd0; 16];
    id[15] = n;
    id
}

/// A replica of `of`, as a joiner gets it from the snapshot.
fn replica(of: &WorkspaceList, peer: u64) -> WorkspaceList {
    WorkspaceList::from_snapshot(peer, &of.snapshot().unwrap()).unwrap()
}

/// Everything `from` holds into `to`; the import report of `to`.
fn sync(from: &WorkspaceList, to: &WorkspaceList) -> curvyo_document_core::ListImportReport {
    let bytes = from.export_updates(&DocumentVersion::never()).unwrap();
    to.import_updates(&bytes).unwrap()
}

fn sync_all(lists: &[&WorkspaceList]) {
    for _ in 0..2 {
        for a in lists {
            for b in lists {
                if !std::ptr::eq(*a, *b) {
                    sync(a, b);
                }
            }
        }
    }
}

fn shown(list: &WorkspaceList) -> Vec<(Vec<String>, EntryKind, Option<[u8; 16]>)> {
    let mut out: Vec<_> = list
        .entries()
        .into_iter()
        .map(|e| (e.path, e.kind, e.document))
        .collect();
    out.sort_by(|a, b| {
        (&a.0, a.1 == EntryKind::Folder, a.2).cmp(&(&b.0, b.1 == EntryKind::Folder, b.2))
    });
    out
}

/// Checks the structural invariants of one replica's view.
fn check_view(list: &WorkspaceList) {
    let entries = list.entries();
    let by_node: HashMap<ListNodeId, &ListEntry> = entries.iter().map(|e| (e.node, e)).collect();
    let mut docs = HashSet::new();
    let mut siblings: HashMap<(Option<ListNodeId>, bool), HashSet<String>> = HashMap::new();
    for e in &entries {
        if let Some(parent) = e.parent {
            let parent = by_node
                .get(&parent)
                .unwrap_or_else(|| panic!("parent of {:?} is not shown", e.path));
            assert_eq!(parent.kind, EntryKind::Folder);
            assert_eq!(e.path[..e.path.len() - 1], parent.path[..]);
        } else {
            assert_eq!(e.path.len(), 1);
        }
        if let Some(d) = e.document {
            assert_eq!(e.kind, EntryKind::Document);
            assert!(docs.insert(d), "document {d:?} shown twice");
            assert!(!list.tombstoned_documents().contains(&d));
        }
        let name = e.path.last().unwrap().to_lowercase();
        assert!(
            check_entry_name(e.path.last().unwrap()).is_ok(),
            "{:?}",
            e.path
        );
        let set = siblings
            .entry((e.parent, e.kind == EntryKind::Folder))
            .or_default();
        assert!(
            set.insert(name),
            "two siblings shown as {:?}: {:?}",
            e.path,
            shown(list)
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Names (criteria 13, 14, 18)
// ---------------------------------------------------------------------------------------------

/// Criterion 14: names with separators or characters some OS refuses are refused; ordinary
/// names, including non-ASCII, are taken.
#[test]
fn c14_names_some_os_refuses_are_refused() {
    for bad in ["", "   ", "a/b", "a\\b", "/", "\\"] {
        assert!(check_entry_name(bad).is_err(), "{bad:?}");
    }
    assert_eq!(check_entry_name("a/b"), Err(NameRefusal::Separator));
    assert_eq!(check_entry_name(""), Err(NameRefusal::Empty));
    for bad in [
        "a:b", "a*b", "a?b", "a\"b", "a<b", "a>b", "a|b", "a\u{0}b", "a\u{7}b", "a\nb", "lid.",
        "lid ", "CON", "con", "Con.txt", "NUL", "COM1", "lpt9", "AUX", "PRN",
    ] {
        assert!(check_entry_name(bad).is_err(), "{bad:?} must be refused");
    }
    for good in [
        "Lid",
        "Box lid 2",
        "Deckel ÄÖÜ ß",
        "標識",
        "lid.v2",
        "CONSOLE",
        "a-b_c(1)",
    ] {
        assert!(check_entry_name(good).is_ok(), "{good:?} must be taken");
    }
    let long: String = "x".repeat(curvyo_document_core::MAX_ENTRY_NAME_CHARS);
    assert!(check_entry_name(&long).is_ok());
    let too_long: String = "x".repeat(curvyo_document_core::MAX_ENTRY_NAME_CHARS + 1);
    assert_eq!(check_entry_name(&too_long), Err(NameRefusal::TooLong));
    // Characters, not bytes.
    let wide: String = "ä".repeat(curvyo_document_core::MAX_ENTRY_NAME_CHARS);
    assert!(check_entry_name(&wide).is_ok());
}

/// Criterion 13: a taken name gets " 2", " 3"; names compare without case.
#[test]
fn c13_free_names_ignore_case_and_count_up() {
    assert_eq!(free_entry_name("Lid", ["Sides"]), "Lid");
    assert_eq!(free_entry_name("Lid", ["lid"]), "Lid 2");
    assert_eq!(free_entry_name("Lid", ["LID", "lid 2"]), "Lid 3");
    assert_eq!(free_entry_name("Lid", ["Lid", "Lid 3"]), "Lid 2");
    assert!(same_entry_name("Deckel", "DECKEL"));
    assert!(!same_entry_name("Lid", "Lid 2"));
}

// ---------------------------------------------------------------------------------------------
// Local plans (criteria 13 to 19)
// ---------------------------------------------------------------------------------------------

fn tree() -> WorkspaceTree {
    WorkspaceTree::new([
        (p("Lid.curvyo"), EntryKind::Document),
        (p("Sides"), EntryKind::Folder),
        (p("Sides/Left.curvyo"), EntryKind::Document),
        (p("Sides/Inner/Right.curvyo"), EntryKind::Document),
        (p("Empty"), EntryKind::Folder),
    ])
}

/// Criterion 13: New document writes `<name>.curvyo` into the folder, with a suffix for a
/// taken name (any case) and the final name returned.
#[test]
fn c13_new_document_gets_a_free_name_in_its_folder() {
    let t = tree();
    let planned = t.plan_new_document(&[], "Lid").unwrap();
    assert_eq!(
        planned.op,
        FileOp::CreateDocument {
            path: p("Lid 2.curvyo")
        }
    );
    assert_eq!(planned.name, "Lid 2");
    let planned = t.plan_new_document(&[], "LID").unwrap();
    assert_eq!(planned.name, "LID 2");
    let planned = t.plan_new_document(&p("Sides"), "Lid").unwrap();
    assert_eq!(
        planned.op,
        FileOp::CreateDocument {
            path: p("Sides/Lid.curvyo")
        }
    );
    // A folder named like the document does not take its name: they are different files.
    let planned = t.plan_new_document(&[], "Sides").unwrap();
    assert_eq!(planned.name, "Sides");
    assert!(matches!(
        t.plan_new_document(&[], "a/b"),
        Err(PlanRefusal::Name(_))
    ));
    assert!(matches!(
        t.plan_new_document(&[], ".."),
        Err(PlanRefusal::Name(_))
    ));
    assert_eq!(
        t.plan_new_document(&p("Lid.curvyo"), "X"),
        Err(PlanRefusal::NotAFolder)
    );
    assert!(t.plan_new_document(&p("Nowhere"), "X").is_err());
}

/// Criterion 18: New folder in a folder, with a free name.
#[test]
fn c18_new_folder_gets_a_free_name() {
    let t = tree();
    let planned = t.plan_new_folder(&[], "sides").unwrap();
    assert_eq!(planned.name, "sides 2");
    assert_eq!(planned.op, FileOp::CreateFolder { path: p("sides 2") });
    let planned = t.plan_new_folder(&p("Sides/Inner"), "Deep").unwrap();
    assert_eq!(
        planned.op,
        FileOp::CreateFolder {
            path: p("Sides/Inner/Deep")
        }
    );
}

/// Criterion 14: a rename keeps the folder, refuses a bad or taken name and allows a change of
/// case alone.
#[test]
fn c14_rename_refuses_bad_and_taken_names() {
    let t = WorkspaceTree::new([
        (p("Lid.curvyo"), EntryKind::Document),
        (p("Base.curvyo"), EntryKind::Document),
        (p("Sides"), EntryKind::Folder),
        (p("Sides/Left.curvyo"), EntryKind::Document),
    ]);
    assert_eq!(
        t.plan_rename(&p("Lid.curvyo"), "Top").unwrap(),
        FileOp::Move {
            from: p("Lid.curvyo"),
            to: p("Top.curvyo")
        }
    );
    assert_eq!(
        t.plan_rename(&p("Lid.curvyo"), "base"),
        Err(PlanRefusal::Exists)
    );
    assert!(matches!(
        t.plan_rename(&p("Lid.curvyo"), "a:b"),
        Err(PlanRefusal::Name(_))
    ));
    assert_eq!(
        t.plan_rename(&p("Lid.curvyo"), "LID").unwrap(),
        FileOp::Move {
            from: p("Lid.curvyo"),
            to: p("LID.curvyo")
        }
    );
    assert_eq!(
        t.plan_rename(&p("Sides"), "Walls").unwrap(),
        FileOp::Move {
            from: p("Sides"),
            to: p("Walls")
        }
    );
    assert_eq!(
        t.plan_rename(&p("Gone.curvyo"), "X"),
        Err(PlanRefusal::NotFound)
    );
    // A document may share its name with a folder: `Sides.curvyo` beside `Sides/`.
    assert!(t.plan_rename(&p("Lid.curvyo"), "Sides").is_ok());
}

/// Criterion 16: a move into itself, below itself or onto a taken name is refused; the undo of
/// a move is the move back.
#[test]
fn c16_moves_refuse_loops_and_taken_names() {
    let t = WorkspaceTree::new([
        (p("Lid.curvyo"), EntryKind::Document),
        (p("Sides"), EntryKind::Folder),
        (p("Sides/Lid.curvyo"), EntryKind::Document),
        (p("Sides/Inner"), EntryKind::Folder),
        (p("Other"), EntryKind::Folder),
    ]);
    assert_eq!(
        t.plan_move(&p("Sides"), &p("Sides")),
        Err(PlanRefusal::IntoItself)
    );
    assert_eq!(
        t.plan_move(&p("Sides"), &p("Sides/Inner")),
        Err(PlanRefusal::IntoItself)
    );
    assert_eq!(
        t.plan_move(&p("Lid.curvyo"), &p("Sides")),
        Err(PlanRefusal::Exists)
    );
    assert_eq!(
        t.plan_move(&p("Sides/Lid.curvyo"), &p("Sides")),
        Err(PlanRefusal::AlreadyThere)
    );
    assert_eq!(
        t.plan_move(&p("Sides"), &p("Lid.curvyo")),
        Err(PlanRefusal::NotAFolder)
    );
    let op = t.plan_move(&p("Sides"), &p("Other")).unwrap();
    assert_eq!(
        op,
        FileOp::Move {
            from: p("Sides"),
            to: p("Other/Sides")
        }
    );
    assert_eq!(
        op.inverse(),
        Some(FileOp::Move {
            from: p("Other/Sides"),
            to: p("Sides")
        })
    );
    // Moving to the top.
    assert_eq!(
        t.plan_move(&p("Sides/Inner"), &[]).unwrap(),
        FileOp::Move {
            from: p("Sides/Inner"),
            to: p("Inner")
        }
    );
    assert_eq!(
        t.plan_move(&p("Sides/Lid.curvyo"), &[]),
        Err(PlanRefusal::Exists)
    );
    assert_eq!(FileOp::Trash { path: p("x") }.inverse(), None);
}

/// Criterion 17: Delete trashes the item; the confirm counts the documents of a folder.
#[test]
fn c17_delete_trashes_and_counts_documents() {
    let t = tree();
    assert_eq!(
        t.plan_delete(&p("Sides")).unwrap(),
        FileOp::Trash { path: p("Sides") }
    );
    assert_eq!(t.document_count(&p("Sides")), 2);
    assert_eq!(t.document_count(&p("Empty")), 0);
    assert_eq!(t.document_count(&[]), 3);
    assert_eq!(t.plan_delete(&p("Nope")), Err(PlanRefusal::NotFound));
}

/// Criterion 19: Duplicate writes `<name> copy.curvyo` beside the original; a second one counts
/// up.
#[test]
fn c19_duplicate_is_name_copy_beside_it() {
    let t = tree();
    let planned = t.plan_duplicate(&p("Sides/Left.curvyo")).unwrap();
    assert_eq!(
        planned.op,
        FileOp::CreateDocument {
            path: p("Sides/Left copy.curvyo")
        }
    );
    let t = WorkspaceTree::new([
        (p("Lid.curvyo"), EntryKind::Document),
        (p("Lid copy.curvyo"), EntryKind::Document),
    ]);
    assert_eq!(
        t.plan_duplicate(&p("Lid.curvyo")).unwrap().name,
        "Lid copy 2"
    );
    assert_eq!(
        t.plan_duplicate(&p("Nope.curvyo")),
        Err(PlanRefusal::NotFound)
    );
    let t = tree();
    assert!(t.plan_duplicate(&p("Sides")).is_err(), "not a document");
}

/// Criterion 19 edge: a name at the length limit still gets a copy whose name is valid.
#[test]
fn c19_duplicate_of_a_longest_name_stays_a_valid_name() {
    let long: String = "x".repeat(curvyo_document_core::MAX_ENTRY_NAME_CHARS);
    let t = WorkspaceTree::new([(vec![format!("{long}.curvyo")], EntryKind::Document)]);
    let planned = t.plan_duplicate(&[format!("{long}.curvyo")]).unwrap();
    assert!(
        check_entry_name(&planned.name).is_ok(),
        "the copy's name {:?} chars breaks the naming rule",
        planned.name.chars().count()
    );
}

// ---------------------------------------------------------------------------------------------
// The shared list (criteria 35 to 40)
// ---------------------------------------------------------------------------------------------

/// Criteria 35 and 36: a creation and a rename by A reach B; the node keeps its identity.
#[test]
fn c35_c36_create_and_rename_reach_the_other_replica() {
    let a = WorkspaceList::new(1);
    let b = replica(&a, 2);
    let lid = a
        .create_document(None, "Lid", doc(1), stamp(100, 1))
        .unwrap();
    sync(&a, &b);
    assert_eq!(b.find_document(doc(1)), Some(lid));
    a.rename(lid, "Top").unwrap();
    sync(&a, &b);
    assert_eq!(
        shown(&b),
        vec![(p("Top"), EntryKind::Document, Some(doc(1)))]
    );
    assert_eq!(b.find_document(doc(1)), Some(lid), "same node after rename");
    assert!(matches!(a.rename(lid, "x/y"), Err(ListError::Name(_))));
    assert_eq!(
        b.entries()[0].node.to_text(),
        lid.to_text(),
        "same node text"
    );
    assert_eq!(ListNodeId::from_text(&lid.to_text()), Some(lid));
    assert_eq!(ListNodeId::from_text("garbage"), None);
    assert_eq!(ListNodeId::from_text(""), None);
}

/// Criterion 37: concurrent moves of one document into different folders converge to one
/// folder, the document exists once.
#[test]
fn c37_concurrent_moves_converge_to_one_folder() {
    let a = WorkspaceList::new(1);
    let x = a.create_folder(None, "X", stamp(1, 1)).unwrap();
    let y = a.create_folder(None, "Y", stamp(1, 2)).unwrap();
    let d = a.create_document(None, "Lid", doc(1), stamp(1, 3)).unwrap();
    let b = replica(&a, 2);
    a.move_to(d, Some(x)).unwrap();
    b.move_to(d, Some(y)).unwrap();
    let ra = sync(&b, &a);
    let rb = sync(&a, &b);
    assert_eq!(shown(&a), shown(&b));
    check_view(&a);
    let lids: Vec<_> = a
        .entries()
        .into_iter()
        .filter(|e| e.document == Some(doc(1)))
        .collect();
    assert_eq!(lids.len(), 1);
    assert!(lids[0].parent == Some(x) || lids[0].parent == Some(y));
    assert!(ra.dropped_moves.is_empty() && rb.dropped_moves.is_empty());
}

/// Criterion 38: two documents of one name created at once both exist; the later one by the
/// same rule on both sides gets " 2", case ignored.
#[test]
fn c38_same_name_creations_both_survive_with_a_suffix() {
    let a = WorkspaceList::new(1);
    let b = replica(&a, 2);
    // B's is created earlier, though by the higher peer.
    a.create_document(None, "Lid", doc(1), stamp(200, 1))
        .unwrap();
    b.create_document(None, "lid", doc(2), stamp(100, 2))
        .unwrap();
    sync_all(&[&a, &b]);
    assert_eq!(shown(&a), shown(&b));
    check_view(&a);
    let mut names: Vec<(String, [u8; 16])> = a
        .entries()
        .into_iter()
        .map(|e| (e.path.last().unwrap().clone(), e.document.unwrap()))
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![("Lid 2".to_owned(), doc(1)), ("lid".to_owned(), doc(2))]
    );
    // Folders of one name are suffixed too; a folder and a document of one name are not.
    a.create_folder(None, "Lid", stamp(50, 3)).unwrap();
    b.create_folder(None, "LID", stamp(60, 4)).unwrap();
    sync_all(&[&a, &b]);
    assert_eq!(shown(&a), shown(&b));
    let folders: Vec<String> = a
        .entries()
        .into_iter()
        .filter(|e| e.kind == EntryKind::Folder)
        .map(|e| e.path[0].clone())
        .collect();
    assert!(folders.contains(&"Lid".to_owned()) && folders.contains(&"LID 2".to_owned()));
}

/// Criterion 38 edge: a derived " 2" never meets a real "Lid 2" in the same folder; every shown
/// name is unique, so the disk can hold every document.
#[test]
fn c38_a_derived_suffix_does_not_collide_with_a_real_name() {
    let a = WorkspaceList::new(1);
    let b = replica(&a, 2);
    a.create_document(None, "Lid 2", doc(1), stamp(10, 1))
        .unwrap();
    a.create_document(None, "Lid", doc(2), stamp(20, 2))
        .unwrap();
    b.create_document(None, "Lid", doc(3), stamp(30, 3))
        .unwrap();
    sync_all(&[&a, &b]);
    assert_eq!(shown(&a), shown(&b));
    check_view(&a);
    assert_eq!(a.entries().len(), 3);
}

/// Criterion 38: two renames of one document at once converge to one name.
#[test]
fn c38_concurrent_renames_converge() {
    let a = WorkspaceList::new(1);
    let d = a.create_document(None, "Lid", doc(1), stamp(1, 1)).unwrap();
    let b = replica(&a, 2);
    let c = replica(&a, 3);
    a.rename(d, "Top").unwrap();
    b.rename(d, "Cover").unwrap();
    c.rename(d, "Deckel").unwrap();
    sync_all(&[&a, &b, &c]);
    assert_eq!(shown(&a), shown(&b));
    assert_eq!(shown(&a), shown(&c));
    assert_eq!(a.entries().len(), 1);
}

/// Criterion 39: A moves X into Y while B moves Y into X: both converge to the same loop-free
/// tree, no document is lost, and only the side whose move was dropped is told.
#[test]
fn c39_a_concurrent_loop_is_broken_the_same_way_everywhere() {
    let a = WorkspaceList::new(1);
    let x = a.create_folder(None, "X", stamp(1, 1)).unwrap();
    let y = a.create_folder(None, "Y", stamp(1, 2)).unwrap();
    a.create_document(Some(x), "InX", doc(1), stamp(1, 3))
        .unwrap();
    a.create_document(Some(y), "InY", doc(2), stamp(1, 4))
        .unwrap();
    let b = replica(&a, 2);
    a.move_to(x, Some(y)).unwrap();
    b.move_to(y, Some(x)).unwrap();
    let ra = sync(&b, &a);
    let rb = sync(&a, &b);
    assert_eq!(shown(&a), shown(&b));
    check_view(&a);
    check_view(&b);
    let docs: HashSet<_> = a.entries().into_iter().filter_map(|e| e.document).collect();
    assert_eq!(docs, HashSet::from([doc(1), doc(2)]), "no document lost");
    let x_parent = a
        .entries()
        .into_iter()
        .find(|e| e.node == x)
        .unwrap()
        .parent;
    let y_parent = a
        .entries()
        .into_iter()
        .find(|e| e.node == y)
        .unwrap()
        .parent;
    match (x_parent, y_parent) {
        (Some(px), None) => {
            assert_eq!(px, y);
            assert_eq!(rb.dropped_moves, vec![y], "B's move was dropped");
            assert!(ra.dropped_moves.is_empty());
        }
        (None, Some(py)) => {
            assert_eq!(py, x);
            assert_eq!(ra.dropped_moves, vec![x], "A's move was dropped");
            assert!(rb.dropped_moves.is_empty());
        }
        other => panic!("not one of the two moves applied: {other:?}"),
    }
}

/// Criterion 39 with three folders: A moves X into Y, B moves Y into Z, C moves Z into X.
#[test]
fn c39_a_three_way_loop_converges_loop_free() {
    let a = WorkspaceList::new(1);
    let x = a.create_folder(None, "X", stamp(1, 1)).unwrap();
    let y = a.create_folder(None, "Y", stamp(1, 2)).unwrap();
    let z = a.create_folder(None, "Z", stamp(1, 3)).unwrap();
    for (i, f) in [x, y, z].into_iter().enumerate() {
        a.create_document(Some(f), "D", doc(u8::try_from(i).unwrap()), stamp(2, 10))
            .unwrap();
    }
    let b = replica(&a, 2);
    let c = replica(&a, 3);
    a.move_to(x, Some(y)).unwrap();
    b.move_to(y, Some(z)).unwrap();
    c.move_to(z, Some(x)).unwrap();
    sync_all(&[&a, &b, &c]);
    assert_eq!(shown(&a), shown(&b));
    assert_eq!(shown(&a), shown(&c));
    check_view(&a);
    assert_eq!(
        a.entries().iter().filter(|e| e.document.is_some()).count(),
        3
    );
}

/// Criterion 16 in the list: a move into itself or below itself is refused locally.
#[test]
fn c16_list_refuses_a_local_loop() {
    let a = WorkspaceList::new(1);
    let x = a.create_folder(None, "X", stamp(1, 1)).unwrap();
    let y = a.create_folder(Some(x), "Y", stamp(1, 2)).unwrap();
    let d = a.create_document(None, "D", doc(1), stamp(1, 3)).unwrap();
    assert_eq!(a.move_to(x, Some(y)), Err(ListError::Loop));
    assert_eq!(a.move_to(x, Some(x)), Err(ListError::Loop));
    assert_eq!(a.move_to(x, Some(d)), Err(ListError::NotAFolder));
    assert_eq!(
        a.create_document(Some(d), "E", doc(2), stamp(1, 4)),
        Err(ListError::NotAFolder)
    );
}

/// Criterion 40: a deleted folder hides what lies in it; the Deleted list names only what a
/// member deleted, with who and when; Restore brings it back on every replica.
#[test]
fn c40_delete_hides_and_restore_brings_back() {
    let a = WorkspaceList::new(1);
    let f = a.create_folder(None, "Sides", stamp(1, 1)).unwrap();
    a.create_document(Some(f), "Left", doc(1), stamp(1, 2))
        .unwrap();
    let inner = a.create_folder(Some(f), "Inner", stamp(1, 3)).unwrap();
    a.create_document(Some(inner), "Right", doc(2), stamp(1, 4))
        .unwrap();
    a.create_document(None, "Lid", doc(3), stamp(1, 5)).unwrap();
    let b = replica(&a, 2);
    a.delete(f, "Anna", 1_000).unwrap();
    sync(&a, &b);
    assert_eq!(
        shown(&b),
        vec![(p("Lid"), EntryKind::Document, Some(doc(3)))]
    );
    assert_eq!(
        b.tombstoned_documents(),
        HashSet::from([doc(1), doc(2)]),
        "both nested documents are deleted"
    );
    let deleted = b.deleted_entries();
    assert_eq!(deleted.len(), 1, "only the folder: {deleted:?}");
    assert_eq!(deleted[0].node, f);
    assert_eq!(deleted[0].by, "Anna");
    assert_eq!(deleted[0].at, 1_000);
    assert_eq!(deleted[0].name, "Sides");
    // Any member restores.
    b.restore(f).unwrap();
    sync(&b, &a);
    assert_eq!(shown(&a), shown(&b));
    assert_eq!(a.entries().len(), 5);
    assert!(a.tombstoned_documents().is_empty());
    assert!(a.deleted_entries().is_empty());
    assert_eq!(a.restore(f), Err(ListError::NotFound), "not deleted now");
}

/// Criterion 40: Deleted lists most recent first; a document restored while its folder is
/// still deleted comes back at the top.
#[test]
fn c40_restore_under_a_deleted_folder_comes_back_at_the_top() {
    let a = WorkspaceList::new(1);
    let f = a.create_folder(None, "Sides", stamp(1, 1)).unwrap();
    let d = a
        .create_document(Some(f), "Left", doc(1), stamp(1, 2))
        .unwrap();
    a.delete(d, "Anna", 10).unwrap();
    a.delete(f, "Ben", 20).unwrap();
    let deleted = a.deleted_entries();
    assert_eq!(
        deleted.iter().map(|e| e.node).collect::<Vec<_>>(),
        vec![f, d]
    );
    a.restore(d).unwrap();
    assert_eq!(
        shown(&a),
        vec![(p("Left"), EntryKind::Document, Some(doc(1)))]
    );
    check_view(&a);
}

/// Criterion 40 against concurrency: A deletes a folder while B creates a document in it and
/// moves another one out of it. Both converge; nothing is lost (shown or restorable).
#[test]
fn c40_delete_concurrent_with_create_and_move_loses_nothing() {
    let a = WorkspaceList::new(1);
    let f = a.create_folder(None, "Sides", stamp(1, 1)).unwrap();
    let left = a
        .create_document(Some(f), "Left", doc(1), stamp(1, 2))
        .unwrap();
    let b = replica(&a, 2);
    a.delete(f, "Anna", 100).unwrap();
    b.create_document(Some(f), "New", doc(2), stamp(2, 3))
        .unwrap();
    b.move_to(left, None).unwrap();
    sync_all(&[&a, &b]);
    assert_eq!(shown(&a), shown(&b));
    assert_eq!(a.tombstoned_documents(), b.tombstoned_documents());
    check_view(&a);
    for d in [doc(1), doc(2)] {
        assert!(
            a.find_document(d).is_some() || a.tombstoned_documents().contains(&d),
            "document {d:?} neither shown nor restorable"
        );
    }
    // Restoring the folder brings everything back.
    a.restore(f).unwrap();
    sync_all(&[&a, &b]);
    assert!(a.find_document(doc(1)).is_some());
    assert!(a.find_document(doc(2)).is_some());
    assert_eq!(shown(&a), shown(&b));
}

/// Criterion 40: A deletes a document while B renames it; both converge.
#[test]
fn c40_delete_concurrent_with_rename_converges() {
    let a = WorkspaceList::new(1);
    let d = a.create_document(None, "Lid", doc(1), stamp(1, 1)).unwrap();
    let b = replica(&a, 2);
    a.delete(d, "Anna", 5).unwrap();
    b.rename(d, "Top").unwrap();
    sync_all(&[&a, &b]);
    assert_eq!(shown(&a), shown(&b));
    assert_eq!(a.deleted_entries(), b.deleted_entries());
    assert_eq!(a.tombstoned_documents(), b.tombstoned_documents());
}

/// Criterion 33: a list snapshot reads back to the same list, garbage is refused.
#[test]
fn c33_list_snapshot_round_trips_and_garbage_is_refused() {
    let a = WorkspaceList::new(1);
    let f = a.create_folder(None, "F", stamp(1, 1)).unwrap();
    a.create_document(Some(f), "D", doc(1), stamp(1, 2))
        .unwrap();
    let d2 = a.create_document(None, "E", doc(2), stamp(1, 3)).unwrap();
    a.delete(d2, "Anna", 3).unwrap();
    let b = replica(&a, 9);
    assert_eq!(shown(&a), shown(&b));
    assert_eq!(a.deleted_entries(), b.deleted_entries());
    assert!(WorkspaceList::from_snapshot(1, b"not a list").is_err());
    assert!(WorkspaceList::from_snapshot(1, &[]).is_err());
    assert!(a.import_updates(b"garbage").is_err());
    // An update from a later point only.
    let v = a.version();
    a.rename(f, "G").unwrap();
    let tail = a.export_updates(&v).unwrap();
    b.import_updates(&tail).unwrap();
    assert_eq!(shown(&a), shown(&b));
}

/// Tiny deterministic generator for the fuzz tests.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % (n as u64)).unwrap()
    }
}

const NAMES: [&str; 6] = ["Lid", "lid", "Lid 2", "Sides", "Box.curvyo", "Base"];

/// One random list operation on `list`; documents get ids from `next_doc`.
fn random_op(list: &WorkspaceList, rng: &mut Rng, next_doc: &mut u8, created: &mut Vec<[u8; 16]>) {
    let entries = list.entries();
    let folders: Vec<ListNodeId> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Folder)
        .map(|e| e.node)
        .collect();
    let pick_folder = |rng: &mut Rng| -> Option<ListNodeId> {
        if folders.is_empty() || rng.below(3) == 0 {
            None
        } else {
            Some(folders[rng.below(folders.len())])
        }
    };
    let name = NAMES[rng.below(NAMES.len())];
    let stamp = Created {
        at: i64::try_from(rng.below(5)).unwrap(),
        id: [u8::try_from(rng.below(256)).unwrap(); 16],
    };
    match rng.below(8) {
        0 | 1 => {
            let parent = pick_folder(rng);
            let _ = list.create_folder(parent, name, stamp);
        }
        2 | 3 => {
            let parent = pick_folder(rng);
            *next_doc = next_doc.wrapping_add(1);
            let mut id = [0x44; 16];
            id[0] = *next_doc;
            id[1] = 0x99;
            if list.create_document(parent, name, id, stamp).is_ok() {
                created.push(id);
            }
        }
        4 if !entries.is_empty() => {
            let e = &entries[rng.below(entries.len())];
            let _ = list.rename(e.node, name);
        }
        5 if !entries.is_empty() => {
            let e = &entries[rng.below(entries.len())];
            let target = pick_folder(rng);
            let _ = list.move_to(e.node, target);
        }
        6 if !entries.is_empty() => {
            let e = &entries[rng.below(entries.len())];
            let _ = list.delete(e.node, "M", 1);
        }
        7 => {
            let deleted = list.deleted_entries();
            if !deleted.is_empty() {
                let _ = list.restore(deleted[rng.below(deleted.len())].node);
            }
        }
        _ => {}
    }
}

/// Criteria 35 to 40 as a property: three replicas editing at random with random partial syncs
/// converge to the same view; every view is loop-free with unique shown names among siblings,
/// and every document ever created is shown or restorable.
#[test]
fn c35_c40_random_concurrent_edits_converge_and_lose_nothing() {
    for seed in 1..=40_u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
        let a = WorkspaceList::new(1);
        let b = replica(&a, 2);
        let c = replica(&a, 3);
        let all = [&a, &b, &c];
        let mut next_doc = 0_u8;
        let mut created = Vec::new();
        for _ in 0..60 {
            let i = rng.below(3);
            random_op(all[i], &mut rng, &mut next_doc, &mut created);
            if rng.below(4) == 0 {
                let from = rng.below(3);
                let to = rng.below(3);
                if from != to {
                    sync(all[from], all[to]);
                }
            }
        }
        sync_all(&all);
        for l in &all {
            check_view(l);
        }
        assert_eq!(shown(&a), shown(&b), "seed {seed}");
        assert_eq!(shown(&a), shown(&c), "seed {seed}");
        assert_eq!(a.tombstoned_documents(), b.tombstoned_documents());
        assert_eq!(a.deleted_entries(), c.deleted_entries());
        for d in &created {
            assert!(
                a.find_document(*d).is_some() || a.tombstoned_documents().contains(d),
                "seed {seed}: document {d:?} lost"
            );
        }
        // Restoring everything that was deleted shows every document again.
        loop {
            let deleted = a.deleted_entries();
            if deleted.is_empty() {
                break;
            }
            a.restore(deleted[0].node).unwrap();
        }
        check_view(&a);
        for d in &created {
            assert!(
                a.find_document(*d).is_some(),
                "seed {seed}: {d:?} not restorable"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The disk follows the list (criteria 36 to 41)
// ---------------------------------------------------------------------------------------------

/// What a file on the simulated disk holds.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Item {
    Folder,
    /// A document file with its id, or an outside file with a tag.
    File(Option<[u8; 16]>, u32),
}

/// A case-insensitive simulated file system, as on Windows and macOS.
#[derive(Debug, Clone, Default)]
struct Disk {
    items: BTreeMap<Vec<String>, (Vec<String>, Item)>,
    trashed: Vec<(Vec<String>, Item)>,
}

fn key(path: &[String]) -> Vec<String> {
    path.iter().map(|c| c.to_lowercase()).collect()
}

impl Disk {
    fn has(&self, path: &[String]) -> bool {
        path.is_empty() || self.items.contains_key(&key(path))
    }

    fn put(&mut self, path: &[String], item: Item) {
        self.items.insert(key(path), (path.to_vec(), item));
    }

    fn file(&mut self, path: &str, id: Option<[u8; 16]>, tag: u32) {
        let path = p(path);
        for i in 1..path.len() {
            if !self.has(&path[..i]) {
                self.put(&path[..i], Item::Folder);
            }
        }
        self.put(&path, Item::File(id, tag));
    }

    /// Applies one op; `Err` when it would replace or miss an entry.
    fn apply(&mut self, op: &DiskOp) -> Result<(), String> {
        let parent_ok = |d: &Self, path: &[String]| d.has(&path[..path.len() - 1]);
        match op {
            DiskOp::CreateFolder { path } => {
                if self.has(path) || !parent_ok(self, path) {
                    return Err(format!("CreateFolder {path:?}"));
                }
                self.put(path, Item::Folder);
            }
            DiskOp::Write { path, document } => {
                if self.has(path) || !parent_ok(self, path) {
                    return Err(format!("Write over or without parent {path:?}"));
                }
                self.put(path, Item::File(Some(*document), 0));
            }
            DiskOp::Move { from, to } => {
                let same = key(from) == key(to);
                if !self.has(from) || (self.has(to) && !same) || !parent_ok(self, to) {
                    return Err(format!("Move {from:?} -> {to:?}"));
                }
                if key(to).starts_with(&key(from)) && !same {
                    return Err(format!("Move into itself {from:?} -> {to:?}"));
                }
                let kf = key(from);
                let moving: Vec<_> = self
                    .items
                    .iter()
                    .filter(|(k, _)| k.starts_with(&kf))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                for (k, _) in &moving {
                    self.items.remove(k);
                }
                for (_, (path, item)) in moving {
                    let mut new = to.clone();
                    new.extend_from_slice(&path[from.len()..]);
                    self.put(&new, item);
                }
            }
            DiskOp::Trash { path } => {
                if !self.has(path) || path.is_empty() {
                    return Err(format!("Trash {path:?}"));
                }
                let kp = key(path);
                let gone: Vec<_> = self
                    .items
                    .iter()
                    .filter(|(k, _)| k.starts_with(&kp))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                for (k, v) in gone {
                    self.items.remove(&k);
                    self.trashed.push(v);
                }
            }
        }
        Ok(())
    }

    fn scan(&self) -> (Vec<WorkspacePath>, Vec<ScannedDocument>) {
        let mut folders = Vec::new();
        let mut documents = Vec::new();
        for (path, item) in self.items.values() {
            match item {
                Item::Folder => folders.push(path.clone()),
                Item::File(id, _) => documents.push(ScannedDocument {
                    path: path.clone(),
                    document: *id,
                }),
            }
        }
        (folders, documents)
    }

    fn files(&self) -> Vec<(Vec<String>, Option<[u8; 16]>, u32)> {
        self.items
            .values()
            .filter_map(|(path, item)| match item {
                Item::File(id, tag) => Some((path.clone(), *id, *tag)),
                Item::Folder => None,
            })
            .collect()
    }
}

fn file_path(entry: &ListEntry) -> Vec<String> {
    let mut path = entry.path.clone();
    if entry.kind == EntryKind::Document {
        let last = path.pop().unwrap();
        path.push(format!("{last}.curvyo"));
    }
    path
}

/// Plans the disk against `list`, applies the plan, checks that no op replaces or misses an
/// entry, that no file of a shown document and no outside file is trashed, and that every shown
/// document then lies at its place unless blocked or missing. Returns the next `known` map.
fn follow(
    list: &WorkspaceList,
    disk: &mut Disk,
    known: &HashMap<[u8; 16], WorkspacePath>,
    context: &str,
) -> (
    curvyo_document_core::DiskPlan,
    HashMap<[u8; 16], WorkspacePath>,
) {
    let entries = list.entries();
    let tomb = list.tombstoned_documents();
    let (folders, documents) = disk.scan();
    let before = disk.files();
    let plan = plan_disk(
        &entries,
        &tomb,
        &DiskState {
            folders: &folders,
            documents: &documents,
            known,
        },
    );
    let trashed_before = disk.trashed.len();
    for op in &plan.ops {
        disk.apply(op)
            .unwrap_or_else(|e| panic!("{context}: {e}\nplan: {:#?}\ndisk: {before:#?}", plan.ops));
    }
    let listed: HashSet<[u8; 16]> = entries.iter().filter_map(|e| e.document).collect();
    for (path, item) in &disk.trashed[trashed_before..] {
        if let Item::File(id, tag) = item {
            match id {
                Some(id) if listed.contains(id) => {
                    // Only a second file of the same id may go; the kept one must still exist.
                    assert!(
                        disk.files().iter().any(|(_, d, _)| *d == Some(*id)),
                        "{context}: the only file of shown document {id:?} was trashed ({path:?})"
                    );
                }
                Some(id) if tomb.contains(id) => {}
                _ => panic!(
                    "{context}: an outside file {path:?} (tag {tag}) was trashed\nplan: {:#?}",
                    plan.ops
                ),
            }
        }
    }
    // Outside files are never moved or lost.
    for (path, id, tag) in &before {
        let outside = id.is_none_or(|d| !listed.contains(&d) && !tomb.contains(&d));
        if outside {
            assert!(
                disk.files()
                    .iter()
                    .any(|(q, i, t)| key(q) == key(path) && i == id && t == tag),
                "{context}: outside file {path:?} moved or lost\nplan: {:#?}",
                plan.ops
            );
        }
    }
    let placed: HashMap<ListNodeId, Vec<String>> = plan
        .list_changes
        .iter()
        .map(|c| (c.node, c.path.clone()))
        .collect();
    let mut next = HashMap::new();
    for e in &entries {
        let Some(id) = e.document else {
            continue;
        };
        if plan.missing.contains(&e.node) {
            continue;
        }
        let want = if let Some(path) = placed.get(&e.node) {
            let mut path = path.clone();
            let last = path.pop().unwrap();
            path.push(format!("{last}.curvyo"));
            path
        } else {
            file_path(e)
        };
        if plan.blocked.iter().any(|b| key(b) == key(&want)) {
            continue;
        }
        let found = disk.items.get(&key(&want));
        assert!(
            matches!(found, Some((_, Item::File(Some(d), _))) if *d == id),
            "{context}: {want:?} should hold {id:?}, holds {found:?}\nplan: {:#?}",
            plan.ops
        );
        next.insert(id, want);
    }
    for e in entries.iter().filter(|e| e.kind == EntryKind::Folder) {
        if !plan
            .blocked
            .iter()
            .any(|b| key(&e.path).starts_with(&key(b)))
        {
            assert!(
                matches!(disk.items.get(&key(&e.path)), Some((_, Item::Folder))),
                "{context}: folder {:?} not on disk",
                e.path
            );
        }
    }
    for (_, id, _) in disk.files() {
        if let Some(id) = id {
            assert!(
                !tomb.contains(&id) || listed.contains(&id),
                "{context}: deleted document {id:?} still on disk"
            );
        }
    }
    (plan, next)
}

/// Criterion 28: an empty folder receives every listed document at its place.
#[test]
fn c28_an_empty_disk_gets_every_document_written() {
    let a = WorkspaceList::new(1);
    let f = a.create_folder(None, "Sides", stamp(1, 1)).unwrap();
    a.create_document(Some(f), "Left", doc(1), stamp(1, 2))
        .unwrap();
    a.create_document(None, "Lid", doc(2), stamp(1, 3)).unwrap();
    a.create_folder(None, "Empty", stamp(1, 4)).unwrap();
    let mut disk = Disk::default();
    let (plan, _) = follow(&a, &mut disk, &HashMap::new(), "join");
    assert!(plan.outside.is_empty() && plan.missing.is_empty() && plan.blocked.is_empty());
}

/// Criteria 36, 37 and 40: renames, moves (also into a new folder), a swap of two names and a
/// deletion are followed without replacing any file.
#[test]
fn c36_c37_c40_disk_follows_renames_moves_swaps_and_deletes() {
    let a = WorkspaceList::new(1);
    let f = a.create_folder(None, "Sides", stamp(1, 1)).unwrap();
    let left = a
        .create_document(Some(f), "Left", doc(1), stamp(1, 2))
        .unwrap();
    let right = a
        .create_document(Some(f), "Right", doc(2), stamp(1, 3))
        .unwrap();
    let lid = a.create_document(None, "Lid", doc(3), stamp(1, 4)).unwrap();
    let mut disk = Disk::default();
    let (_, known) = follow(&a, &mut disk, &HashMap::new(), "join");

    // Swap Left and Right: a ring of renames.
    a.rename(left, "Tmp").unwrap();
    a.rename(right, "Left").unwrap();
    a.rename(left, "Right").unwrap();
    let (_, known) = follow(&a, &mut disk, &known, "swap");
    // Case-only rename.
    a.rename(lid, "LID").unwrap();
    let (_, known) = follow(&a, &mut disk, &known, "case");
    assert!(
        disk.files()
            .iter()
            .any(|(path, _, _)| path == &p("LID.curvyo"))
    );
    // Move into a new folder and rename that folder.
    let g = a.create_folder(None, "Walls", stamp(2, 5)).unwrap();
    a.move_to(f, Some(g)).unwrap();
    a.rename(g, "Outer").unwrap();
    let (_, known) = follow(&a, &mut disk, &known, "nested move");
    // Delete the folder; then restore it.
    a.delete(g, "Anna", 9).unwrap();
    let (_, known) = follow(&a, &mut disk, &known, "delete");
    assert_eq!(disk.files().len(), 1, "only LID stays: {:?}", disk.files());
    a.restore(g).unwrap();
    // Restoring needs the files written again from the room.
    let (plan, _) = follow(&a, &mut disk, &known, "restore");
    assert!(
        plan.ops
            .iter()
            .any(|op| matches!(op, DiskOp::Write { document, .. } if *document == doc(1)))
    );
}

/// Criterion 36 and 40: A deletes `Lid` and renames `Base` to `Lid` before B's next poll; one
/// plan trashes the old file and moves `Base` onto the freed place, and reports no place as
/// blocked (no file outside the workspace holds it).
#[test]
fn c36_one_plan_follows_a_delete_and_a_rename_onto_its_place() {
    let a = WorkspaceList::new(1);
    let lid = a.create_document(None, "Lid", doc(1), stamp(1, 1)).unwrap();
    let base = a
        .create_document(None, "Base", doc(2), stamp(1, 2))
        .unwrap();
    let mut disk = Disk::default();
    let (_, known) = follow(&a, &mut disk, &HashMap::new(), "join");
    a.delete(lid, "Anna", 3).unwrap();
    a.rename(base, "Lid").unwrap();
    let (plan, known) = follow(&a, &mut disk, &known, "delete and rename");
    assert!(
        plan.blocked.is_empty(),
        "blocked by a workspace file: {plan:?}"
    );
    let (again, _) = follow(&a, &mut disk, &known, "again");
    assert!(
        again.ops.is_empty(),
        "a second plan was needed: {:?}",
        again.ops
    );
}

/// Criterion 41 and ADR 0037 decision 6: a file another program moved is followed by the list;
/// one it removed is "Missing", not a deletion; a new file is outside; a file holding a listed
/// place blocks the operation instead of being replaced.
#[test]
fn c41_files_changed_by_another_program() {
    let a = WorkspaceList::new(1);
    let lid = a.create_document(None, "Lid", doc(1), stamp(1, 1)).unwrap();
    let base = a
        .create_document(None, "Base", doc(2), stamp(1, 2))
        .unwrap();
    let mut disk = Disk::default();
    let (_, known) = follow(&a, &mut disk, &HashMap::new(), "join");

    // Another program moves Lid into a new folder and removes Base; a stranger file appears.
    disk.items.remove(&key(&p("Lid.curvyo")));
    disk.file("Other/Lid moved.curvyo", Some(doc(1)), 7);
    disk.items.remove(&key(&p("Base.curvyo")));
    disk.file("Stranger.curvyo", None, 8);
    let (plan, _) = follow(&a, &mut disk, &known, "outside changes");
    assert_eq!(plan.list_changes.len(), 1);
    assert_eq!(plan.list_changes[0].node, lid);
    assert_eq!(plan.list_changes[0].path, p("Other/Lid moved"));
    assert_eq!(plan.missing, vec![base]);
    assert_eq!(plan.outside, vec![p("Stranger.curvyo")]);
    assert!(
        !plan
            .ops
            .iter()
            .any(|op| matches!(op, DiskOp::Trash { .. } | DiskOp::Write { .. })),
        "a removed file is not deleted for everyone nor rewritten: {:?}",
        plan.ops
    );

    // A stranger file sits where a rename wants to go: blocked, never replaced.
    let b = WorkspaceList::new(1);
    let d = b.create_document(None, "Lid", doc(1), stamp(1, 1)).unwrap();
    let mut disk = Disk::default();
    let (_, known) = follow(&b, &mut disk, &HashMap::new(), "join 2");
    disk.file("Top.curvyo", None, 9);
    b.rename(d, "Top").unwrap();
    let (plan, _) = follow(&b, &mut disk, &known, "blocked rename");
    assert_eq!(plan.blocked, vec![p("Top.curvyo")]);
}

/// ADR 0037 decision 3: two files with one document id (a copy by another program): the one at
/// the list's place keeps it, the other is outside; neither is trashed.
#[test]
fn c41_a_copied_file_with_the_same_id_is_outside_and_kept() {
    let a = WorkspaceList::new(1);
    a.create_document(None, "Lid", doc(1), stamp(1, 1)).unwrap();
    let mut disk = Disk::default();
    let (_, known) = follow(&a, &mut disk, &HashMap::new(), "join");
    disk.file("Lid (copy).curvyo", Some(doc(1)), 5);
    let (plan, _) = follow(&a, &mut disk, &known, "copy");
    assert_eq!(plan.outside, vec![p("Lid (copy).curvyo")]);
    assert!(plan.ops.is_empty(), "{:?}", plan.ops);
}

/// Criterion 40 edge: a peer deletes a folder that also holds a file outside the workspace on
/// this computer; that file must not go to the trash with the folder.
#[test]
fn c40_deleting_a_folder_keeps_an_outside_file_in_it() {
    let a = WorkspaceList::new(1);
    let f = a.create_folder(None, "Sides", stamp(1, 1)).unwrap();
    a.create_document(Some(f), "Left", doc(1), stamp(1, 2))
        .unwrap();
    let mut disk = Disk::default();
    let (_, known) = follow(&a, &mut disk, &HashMap::new(), "join");
    disk.file("Sides/Mine.curvyo", None, 3);
    a.delete(f, "Anna", 5).unwrap();
    follow(&a, &mut disk, &known, "delete folder with an outside file");
}

/// Edge: a folder named `Box.curvyo` and a document named `Box` in one folder would be one disk
/// entry; the plan must not write over either.
#[test]
fn c38_folder_named_like_a_document_file_does_not_collide() {
    let a = WorkspaceList::new(1);
    if a.create_folder(None, "Box.curvyo", stamp(1, 1)).is_err() {
        return; // refused at input: no collision possible from this replica
    }
    a.create_document(None, "Box", doc(1), stamp(1, 2)).unwrap();
    let mut disk = Disk::default();
    let mut ops = Vec::new();
    let (folders, documents) = disk.scan();
    let plan = plan_disk(
        &a.entries(),
        &a.tombstoned_documents(),
        &DiskState {
            folders: &folders,
            documents: &documents,
            known: &HashMap::new(),
        },
    );
    for op in &plan.ops {
        ops.push(op.clone());
        disk.apply(op)
            .unwrap_or_else(|e| panic!("{e}: plan {:?}", plan.ops));
    }
}

/// Criteria 36 to 41 as a property: random list edits with occasional moves by another program
/// and stranger files; every plan applies without replacing an entry, trashes no shown or
/// outside file, places every shown document, and a second plan right after has nothing to do.
#[test]
fn c36_c41_random_disk_follow_never_overwrites_or_loses() {
    for seed in 1..=60_u64 {
        let mut rng = Rng(seed.wrapping_mul(0x2545_f491_4f6c_dd1d) | 1);
        let a = WorkspaceList::new(1);
        let b = replica(&a, 2);
        let mut disk = Disk::default();
        let mut known = HashMap::new();
        let mut next_doc = 0;
        let mut created = Vec::new();
        let mut tag = 100;
        for round in 0..25 {
            for _ in 0..3 {
                let which = if rng.below(2) == 0 { &a } else { &b };
                random_op(which, &mut rng, &mut next_doc, &mut created);
            }
            sync_all(&[&a, &b]);
            if rng.below(5) == 0 {
                tag += 1;
                let name = NAMES[rng.below(NAMES.len())];
                disk.file(&format!("{name}.curvyo"), None, tag);
            }
            let ctx = format!("seed {seed} round {round}");
            let (plan, next) = follow(&a, &mut disk, &known, &ctx);
            for change in &plan.list_changes {
                let _ = a.place(change.node, &change.path, || stamp(0, 0));
            }
            known = next;
            // Stable: at most one more plan finishes the work (the one-step case is
            // `c36_one_plan_follows_a_delete_and_a_rename_onto_its_place`), and a third has
            // nothing to do.
            let (again, next) = follow(&a, &mut disk, &known, &format!("{ctx} again"));
            known = next;
            let (third, _) = follow(&a, &mut disk, &known, &format!("{ctx} third"));
            assert!(
                third.ops.is_empty(),
                "{ctx}: third plan not empty: {:?}\nsecond: {:?}",
                third.ops,
                again.ops
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Identity in the file (criterion 15) and the format (ADR 0037 decision 12)
// ---------------------------------------------------------------------------------------------

/// Criterion 15: the document id is stored in the file (`share.json` v2) and survives a save and
/// open; it is not derived from a name.
#[test]
fn c15_the_document_id_is_kept_in_the_file() {
    let document = curvyo_document_core::Document::new(5);
    let record = WorkspaceShareRecord {
        workspace: [0x57; 16],
        document: doc(1),
        synced_version: document.version(),
    };
    document.set_workspace_share(Some(record.clone()));
    let bytes = pack(&document, "test").unwrap();
    let opened = unpack(6, &bytes).unwrap();
    assert_eq!(opened.workspace_share(), Some(record));
    assert!(!opened.legacy_share(), "no room record beside it");
    // Clearing it gives a plain local file.
    opened.set_workspace_share(None);
    let plain = unpack(7, &pack(&opened, "test").unwrap()).unwrap();
    assert!(plain.workspace_share().is_none() && !plain.is_shared());
}

/// ADR 0037 decision 12: the container's format number is not raised by this story: a `0092`
/// file of format 34 still opens (as a local file since ADR 0044 decision 3), and the workspace
/// fixture opens with its workspace record.
#[test]
fn c3_older_shared_files_and_workspace_files_open() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let v34 = std::fs::read(dir.join("shared_v34.curvyo")).unwrap();
    let opened = unpack(3, &v34).unwrap();
    assert!(opened.legacy_share(), "a 0092 file opens local");
    assert!(opened.workspace_share().is_none());
    let ws = std::fs::read(dir.join("workspace_shared_v35.curvyo")).unwrap();
    let opened = unpack(4, &ws).unwrap();
    let record = opened.workspace_share().expect("a workspace record");
    assert!(!opened.legacy_share());
    // A save keeps the record.
    let again = unpack(5, &pack(&opened, "test").unwrap()).unwrap();
    assert_eq!(again.workspace_share(), Some(record));
}

/// Criterion 3 / ADR 0037 decision 3: a `share.json` that names a workspace is read by a scan
/// without opening the document; anything else gives no record.
#[test]
fn c41_share_json_v2_is_read_without_opening_and_v1_is_not_a_workspace_record() {
    assert!(WorkspaceShareRecord::from_share_json(b"{}").is_none());
    assert!(WorkspaceShareRecord::from_share_json(b"not json").is_none());
    let v1 = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/workspace_marker_share_v1.json"),
    )
    .unwrap();
    assert!(WorkspaceShareRecord::from_share_json(&v1).is_none());
    // A v2 record with a newer format version is not read as this build's.
    let v3 = br#"{"format_version":3,"workspace":"57575757575757575757575757575757","document":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d001","synced_version":""}"#;
    assert!(WorkspaceShareRecord::from_share_json(v3).is_none());
}

/// ADR 0037 decision 12 (white-box: the root map `workspace`, register `format_version`): a
/// list written by a newer build is refused as too new, from a snapshot and from an update.
#[test]
fn c23_a_newer_list_is_refused_as_too_new() {
    let a = WorkspaceList::new(1);
    a.create_document(None, "Lid", doc(1), stamp(1, 1)).unwrap();
    let newer = loro::LoroDoc::new();
    newer.set_peer_id(9).unwrap();
    newer.import(&a.snapshot().unwrap()).unwrap();
    newer
        .get_map("workspace")
        .insert("format_version", 2_i64)
        .unwrap();
    newer.commit();
    let snapshot = newer.export(loro::ExportMode::Snapshot).unwrap();
    assert_eq!(
        WorkspaceList::from_snapshot(2, &snapshot).err(),
        Some(ListError::FormatTooNew)
    );
    let b = replica(&a, 3);
    let before = shown(&b);
    let update = newer.export(loro::ExportMode::all_updates()).unwrap();
    assert_eq!(b.import_updates(&update), Err(ListError::FormatTooNew));
    assert_eq!(shown(&b), before);
}
