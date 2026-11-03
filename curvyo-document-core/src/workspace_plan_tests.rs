//! Tests of the local workspace plans (`specs/0094-workspace` criteria 13, 14, 16 to 19).

use super::*;

fn p(text: &str) -> WorkspacePath {
    text.split('/')
        .filter(|c| !c.is_empty())
        .map(str::to_string)
        .collect()
}

/// Box job: `Lid`, `Base`, folder `Sides` with `Left`, `Right`, empty folder `Old`.
fn tree() -> WorkspaceTree {
    WorkspaceTree::new([
        (p("Lid.curvyo"), EntryKind::Document),
        (p("Base.CURVYO"), EntryKind::Document),
        (p("Sides/Left.curvyo"), EntryKind::Document),
        (p("Sides/Right.curvyo"), EntryKind::Document),
        (p("Old"), EntryKind::Folder),
    ])
}

#[test]
fn deeper_entries_imply_their_folders() {
    let tree = tree();
    assert_eq!(tree.kind(&p("Sides")), Some(EntryKind::Folder));
    assert_eq!(tree.kind(&[]), Some(EntryKind::Folder));
    assert_eq!(tree.kind(&p("Missing")), None);
}

#[test]
fn folders_count_their_documents_at_every_depth() {
    let mut entries = vec![(p("Sides/Inner/Deep.curvyo"), EntryKind::Document)];
    entries.push((p("Sides/Left.curvyo"), EntryKind::Document));
    let tree = WorkspaceTree::new(entries);
    assert_eq!(tree.document_count(&p("Sides")), 2);
    assert_eq!(tree.document_count(&p("Sides/Inner")), 1);
    assert_eq!(tree.document_count(&[]), 2);
}

#[test]
fn a_new_document_gets_a_free_name_in_its_folder() {
    let tree = tree();
    let planned = tree.plan_new_document(&[], "Lid").unwrap();
    assert_eq!(planned.name, "Lid 2");
    assert_eq!(
        planned.op,
        FileOp::CreateDocument {
            path: p("Lid 2.curvyo")
        }
    );
    let planned = tree.plan_new_document(&p("Sides"), "Lid").unwrap();
    assert_eq!(
        planned.op,
        FileOp::CreateDocument {
            path: p("Sides/Lid.curvyo")
        }
    );
    assert_eq!(
        tree.plan_new_document(&[], "base").unwrap().name,
        "base 2",
        "case-insensitive, any extension case"
    );
}

#[test]
fn a_folder_and_a_document_may_share_a_name() {
    let planned = tree().plan_new_folder(&[], "Lid").unwrap();
    assert_eq!(planned.op, FileOp::CreateFolder { path: p("Lid") });
    assert_eq!(tree().plan_new_folder(&[], "old").unwrap().name, "old 2");
}

#[test]
fn creation_refuses_bad_names_and_non_folders() {
    let tree = tree();
    assert_eq!(
        tree.plan_new_document(&[], "a/b"),
        Err(PlanRefusal::Name(NameRefusal::Separator))
    );
    assert_eq!(
        tree.plan_new_document(&p("Lid.curvyo"), "x"),
        Err(PlanRefusal::NotAFolder)
    );
    assert_eq!(
        tree.plan_new_folder(&p("Nowhere"), "x"),
        Err(PlanRefusal::NotAFolder)
    );
}

#[test]
fn a_duplicate_is_named_copy_beside_its_original() {
    let tree = tree();
    let planned = tree.plan_duplicate(&p("Sides/Left.curvyo")).unwrap();
    assert_eq!(planned.name, "Left copy");
    assert_eq!(
        planned.op,
        FileOp::CreateDocument {
            path: p("Sides/Left copy.curvyo")
        }
    );
    assert_eq!(tree.plan_duplicate(&p("Sides")), Err(PlanRefusal::NotFound));
}

#[test]
fn rename_keeps_the_folder_and_the_extension() {
    let tree = tree();
    assert_eq!(
        tree.plan_rename(&p("Sides/Left.curvyo"), "Front"),
        Ok(FileOp::Move {
            from: p("Sides/Left.curvyo"),
            to: p("Sides/Front.curvyo")
        })
    );
    assert_eq!(
        tree.plan_rename(&p("Sides"), "Walls"),
        Ok(FileOp::Move {
            from: p("Sides"),
            to: p("Walls")
        })
    );
}

#[test]
fn rename_refuses_a_taken_name_but_allows_a_change_of_case() {
    let tree = tree();
    assert_eq!(
        tree.plan_rename(&p("Lid.curvyo"), "BASE"),
        Err(PlanRefusal::Exists)
    );
    assert_eq!(
        tree.plan_rename(&p("Lid.curvyo"), "LID"),
        Ok(FileOp::Move {
            from: p("Lid.curvyo"),
            to: p("LID.curvyo")
        })
    );
    assert_eq!(
        tree.plan_rename(&p("Lid.curvyo"), "Old"),
        Ok(FileOp::Move {
            from: p("Lid.curvyo"),
            to: p("Old.curvyo")
        }),
        "a folder's name does not block a document"
    );
    assert_eq!(
        tree.plan_rename(&p("Lid.curvyo"), "Lid."),
        Err(PlanRefusal::Name(NameRefusal::NotAllowed))
    );
    assert_eq!(
        tree.plan_rename(&p("Gone.curvyo"), "X"),
        Err(PlanRefusal::NotFound)
    );
}

#[test]
fn move_keeps_the_name_and_refuses_loops_and_taken_names() {
    let tree = tree();
    assert_eq!(
        tree.plan_move(&p("Lid.curvyo"), &p("Sides")),
        Ok(FileOp::Move {
            from: p("Lid.curvyo"),
            to: p("Sides/Lid.curvyo")
        })
    );
    assert_eq!(
        tree.plan_move(&p("Sides"), &p("Old")),
        Ok(FileOp::Move {
            from: p("Sides"),
            to: p("Old/Sides")
        })
    );
    assert_eq!(
        tree.plan_move(&p("Sides"), &p("Sides")),
        Err(PlanRefusal::IntoItself)
    );
    let nested = WorkspaceTree::new([(p("A/B/x.curvyo"), EntryKind::Document)]);
    assert_eq!(
        nested.plan_move(&p("A"), &p("A/B")),
        Err(PlanRefusal::IntoItself)
    );
    assert_eq!(
        tree.plan_move(&p("Sides/Left.curvyo"), &p("Sides")),
        Err(PlanRefusal::AlreadyThere)
    );
    assert_eq!(
        tree.plan_move(&p("Lid.curvyo"), &p("Base.CURVYO")),
        Err(PlanRefusal::NotAFolder)
    );
    let clash = WorkspaceTree::new([
        (p("Lid.curvyo"), EntryKind::Document),
        (p("Sides/lid.curvyo"), EntryKind::Document),
    ]);
    assert_eq!(
        clash.plan_move(&p("Lid.curvyo"), &p("Sides")),
        Err(PlanRefusal::Exists)
    );
}

#[test]
fn a_move_is_undone_by_its_inverse() {
    let op = tree().plan_move(&p("Lid.curvyo"), &p("Sides")).unwrap();
    assert_eq!(
        op.inverse(),
        Some(FileOp::Move {
            from: p("Sides/Lid.curvyo"),
            to: p("Lid.curvyo")
        })
    );
    assert_eq!(
        tree().plan_delete(&p("Lid.curvyo")).unwrap().inverse(),
        None
    );
}

#[test]
fn delete_trashes_an_existing_entry() {
    assert_eq!(
        tree().plan_delete(&p("Sides")),
        Ok(FileOp::Trash { path: p("Sides") })
    );
    assert_eq!(tree().plan_delete(&p("Nope")), Err(PlanRefusal::NotFound));
}

#[test]
fn document_names_drop_the_extension_in_any_case() {
    assert_eq!(document_name("Lid.curvyo"), "Lid");
    assert_eq!(document_name("Lid.CurVyo"), "Lid");
    assert_eq!(document_name("Lid.svg"), "Lid.svg");
    assert_eq!(document_name("é.curvyo"), "é");
}
