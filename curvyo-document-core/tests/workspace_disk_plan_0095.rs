//! The disk following the shared list (`specs/0095-workspace` criteria 36, 37, 40 and 41, ADR
//! 0037 decision 6): one pure plan from the list, the scanned files keyed by document id and the
//! last known places, to file operations, list changes and row states.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::needless_pass_by_value
)]

use std::collections::HashMap;

use curvyo_document_core::{
    Created, DiskOp, DiskPlan, DiskState, ListPlace, ScannedDocument, WorkspaceList, plan_disk,
};

const LID: [u8; 16] = [0xd1; 16];
const BASE: [u8; 16] = [0xd2; 16];

fn p(text: &str) -> Vec<String> {
    text.split('/')
        .filter(|c| !c.is_empty())
        .map(str::to_owned)
        .collect()
}

fn created(tag: u8) -> Created {
    Created {
        at: i64::from(tag),
        id: [tag; 16],
    }
}

fn file(path: &str, document: Option<[u8; 16]>) -> ScannedDocument {
    ScannedDocument {
        path: p(path),
        document,
    }
}

/// Plans `list` against folders, files and the last known places.
fn plan(
    list: &WorkspaceList,
    folders: &[&str],
    files: Vec<ScannedDocument>,
    known: &[([u8; 16], &str)],
) -> DiskPlan {
    let folders: Vec<Vec<String>> = folders.iter().map(|f| p(f)).collect();
    let known: HashMap<[u8; 16], Vec<String>> =
        known.iter().map(|(id, path)| (*id, p(path))).collect();
    plan_disk(
        &list.entries(),
        &list.tombstoned_documents(),
        &DiskState {
            folders: &folders,
            documents: &files,
            known: &known,
        },
    )
}

/// The list of the box job: `Sides/Lid` and `Base`.
fn box_job() -> WorkspaceList {
    let list = WorkspaceList::new(1);
    let sides = list.create_folder(None, "Sides", created(1)).unwrap();
    list.create_document(Some(sides), "Lid", LID, created(2))
        .unwrap();
    list.create_document(None, "Base", BASE, created(3))
        .unwrap();
    list
}

fn on_disk() -> Vec<ScannedDocument> {
    vec![
        file("Sides/Lid.curvyo", Some(LID)),
        file("Base.curvyo", Some(BASE)),
    ]
}

const KNOWN: [([u8; 16], &str); 2] = [(LID, "Sides/Lid.curvyo"), (BASE, "Base.curvyo")];

#[test]
fn a_disk_that_matches_the_list_needs_nothing() {
    let plan = plan(&box_job(), &["Sides"], on_disk(), &KNOWN);
    assert_eq!(plan, DiskPlan::default());
}

/// Criterion 35: a document the list has and this computer never had is written from its
/// stream, its folders first.
#[test]
fn a_new_document_of_the_list_is_written() {
    let plan = plan(&box_job(), &[], Vec::new(), &[]);
    assert_eq!(
        plan.ops,
        vec![
            DiskOp::CreateFolder { path: p("Sides") },
            DiskOp::Write {
                path: p("Sides/Lid.curvyo"),
                document: LID,
            },
            DiskOp::Write {
                path: p("Base.curvyo"),
                document: BASE,
            },
        ]
    );
}

/// Criteria 36 and 37: a rename and a move in the list move the file.
#[test]
fn renames_and_moves_of_the_list_move_the_files() {
    let list = box_job();
    let lid = list.find_document(LID).unwrap();
    let walls = list.create_folder(None, "Walls", created(4)).unwrap();
    list.rename(lid, "Cover").unwrap();
    list.move_to(lid, Some(walls)).unwrap();
    let plan = plan(&list, &["Sides"], on_disk(), &KNOWN);
    assert_eq!(
        plan.ops,
        vec![
            DiskOp::CreateFolder { path: p("Walls") },
            DiskOp::Move {
                from: p("Sides/Lid.curvyo"),
                to: p("Walls/Cover.curvyo"),
            },
        ]
    );
}

/// A renamed folder: its documents move to the new folder, and the old folder, emptied, goes to
/// the trash (never erased).
#[test]
fn a_renamed_folder_moves_its_documents_and_trashes_the_empty_one() {
    let list = box_job();
    list.rename(list.entries()[0].node, "Walls").unwrap();
    let plan = plan(&list, &["Sides"], on_disk(), &KNOWN);
    assert_eq!(
        plan.ops,
        vec![
            DiskOp::CreateFolder { path: p("Walls") },
            DiskOp::Move {
                from: p("Sides/Lid.curvyo"),
                to: p("Walls/Lid.curvyo"),
            },
            DiskOp::Trash { path: p("Sides") },
        ]
    );
}

/// Criterion 40: a deleted document goes to the trash; a deleted folder goes as one item.
#[test]
fn deletions_of_the_list_trash_the_files() {
    let list = box_job();
    list.delete(list.find_document(BASE).unwrap(), "ben", 5)
        .unwrap();
    let sides = list.entries()[0].node;
    list.delete(sides, "ben", 6).unwrap();
    let plan = plan(&list, &["Sides"], on_disk(), &KNOWN);
    assert_eq!(
        plan.ops,
        vec![
            DiskOp::Trash {
                path: p("Base.curvyo")
            },
            DiskOp::Trash { path: p("Sides") },
        ]
    );
}

/// A deleted folder that also holds a file outside the workspace stays; only the workspace's
/// files leave.
#[test]
fn a_deleted_folder_with_an_outside_file_keeps_the_outside_file() {
    let list = box_job();
    list.delete(list.entries()[0].node, "ben", 6).unwrap();
    let mut files = on_disk();
    files.push(file("Sides/Mine.curvyo", None));
    let plan = plan(&list, &["Sides"], files, &KNOWN);
    assert_eq!(
        plan.ops,
        vec![DiskOp::Trash {
            path: p("Sides/Lid.curvyo")
        }]
    );
    assert_eq!(plan.outside, vec![p("Sides/Mine.curvyo")]);
}

/// Criterion 41: a file another program renamed or moved stays where it is and the list follows
/// it; a file another program removed is missing, not deleted.
#[test]
fn changes_by_another_program_go_to_the_list_or_are_missing() {
    let list = box_job();
    let files = vec![file("Sides/Top.curvyo", Some(LID))];
    let plan = plan(&list, &["Sides"], files, &KNOWN);
    assert_eq!(plan.ops, Vec::new());
    assert_eq!(
        plan.list_changes,
        vec![ListPlace {
            node: list.find_document(LID).unwrap(),
            path: p("Sides/Top"),
        }]
    );
    assert_eq!(plan.missing, vec![list.find_document(BASE).unwrap()]);
}

/// The list wins when both the list and another program moved a document.
#[test]
fn the_list_wins_when_both_moved_a_document() {
    let list = box_job();
    let lid = list.find_document(LID).unwrap();
    list.rename(lid, "Cover").unwrap();
    let files = vec![
        file("Sides/Top.curvyo", Some(LID)),
        file("Base.curvyo", Some(BASE)),
    ];
    let plan = plan(&list, &["Sides"], files, &KNOWN);
    assert_eq!(
        plan.ops,
        vec![DiskOp::Move {
            from: p("Sides/Top.curvyo"),
            to: p("Sides/Cover.curvyo"),
        }]
    );
    assert_eq!(plan.list_changes, Vec::new());
}

/// Criterion 41 and ADR 0037 decision 3: a file without a workspace id, with an unknown id, or a
/// second file with a taken id is outside the shared workspace.
#[test]
fn files_not_in_the_list_are_outside() {
    let list = box_job();
    let mut files = on_disk();
    files.push(file("New.curvyo", None));
    files.push(file("Copy of Base.curvyo", Some(BASE)));
    files.push(file("Foreign.curvyo", Some([0xee; 16])));
    let plan = plan(&list, &["Sides"], files, &KNOWN);
    assert_eq!(plan.ops, Vec::new());
    assert_eq!(
        plan.outside,
        vec![
            p("Copy of Base.curvyo"),
            p("Foreign.curvyo"),
            p("New.curvyo")
        ]
    );
}

/// Two documents that swap names are moved through a hidden temporary name.
#[test]
fn a_swap_of_two_names_goes_through_a_temporary_name() {
    let list = box_job();
    let lid = list.find_document(LID).unwrap();
    let base = list.find_document(BASE).unwrap();
    list.move_to(lid, None).unwrap();
    list.rename(lid, "Spare").unwrap();
    list.rename(base, "Lid").unwrap();
    list.rename(lid, "Base").unwrap();
    let files = vec![
        file("Lid.curvyo", Some(LID)),
        file("Base.curvyo", Some(BASE)),
    ];
    let known = [(LID, "Lid.curvyo"), (BASE, "Base.curvyo")];
    let plan = plan(&list, &["Sides"], files, &known);
    assert_eq!(
        plan.ops,
        vec![
            DiskOp::Move {
                from: p("Base.curvyo"),
                to: p("Base.curvyo.moving"),
            },
            DiskOp::Move {
                from: p("Lid.curvyo"),
                to: p("Base.curvyo"),
            },
            DiskOp::Move {
                from: p("Base.curvyo.moving"),
                to: p("Lid.curvyo"),
            },
        ]
    );
}

/// A place taken by a file outside the workspace blocks the move or write; nothing is replaced.
#[test]
fn a_place_taken_by_an_outside_file_blocks_the_operation() {
    let list = box_job();
    let lid = list.find_document(LID).unwrap();
    list.move_to(lid, None).unwrap();
    let files = vec![
        file("Sides/Lid.curvyo", Some(LID)),
        file("Lid.curvyo", None),
        file("base.curvyo", None),
    ];
    let plan = plan(&list, &["Sides"], files, &[(LID, "Sides/Lid.curvyo")]);
    assert_eq!(plan.ops, Vec::new());
    assert_eq!(plan.missing, Vec::new());
    assert_eq!(plan.blocked, vec![p("Base.curvyo"), p("Lid.curvyo")]);
}

/// A folder another program made, empty or holding only its own files, is never trashed.
#[test]
fn a_folder_another_program_made_stays() {
    let plan = plan(
        &box_job(),
        &["Sides", "Mine", "Mine/Inner"],
        on_disk(),
        &KNOWN,
    );
    assert_eq!(plan, DiskPlan::default());
}
