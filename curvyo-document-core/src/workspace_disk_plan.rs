//! Plans how the disk follows the shared list (`specs/0095-workspace` criteria 35 to 41, ADR 0037
//! decision 6): one pure function from the derived list, the scanned files keyed by document id
//! and the last known places to file operations, list changes and row states.
//!
//! A file whose place differs from the list's is moved when it still lies where this computer
//! last saw it (the list changed); when the list still names that last place, another program
//! moved the file and the list follows it; when both changed, the list wins **(Proposal)**.
//! Nothing replaces an entry: a place taken by a file outside the workspace blocks the operation.

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::BuildHasher;

use crate::workspace_list::ListNodeId;
use crate::workspace_list_view::ListEntry;
use crate::workspace_plan::{DOCUMENT_EXTENSION, EntryKind, WorkspacePath};

/// A `.curvyo` file the scan found, with the document id of its `share.json` version 2 when it
/// names this workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedDocument {
    /// Its path, the file name with extension.
    pub path: WorkspacePath,
    /// Its document id in this workspace.
    pub document: Option<[u8; 16]>,
}

/// The disk as scanned, and where this computer last saw each document (`index.json`).
#[derive(Debug, Clone, Copy)]
pub struct DiskState<'a> {
    /// Every folder.
    pub folders: &'a [WorkspacePath],
    /// Every document file.
    pub documents: &'a [ScannedDocument],
    /// Document id to the file path last seen.
    pub known: &'a HashMap<[u8; 16], WorkspacePath>,
}

/// One file operation the host executes in order. None replaces an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskOp {
    /// Create a folder.
    CreateFolder {
        /// Where.
        path: WorkspacePath,
    },
    /// Move or rename a file.
    Move {
        /// From.
        from: WorkspacePath,
        /// To.
        to: WorkspacePath,
    },
    /// Write the document `document` from its stream.
    Write {
        /// Where.
        path: WorkspacePath,
        /// Which document.
        document: [u8; 16],
    },
    /// Move a file or folder to the OS trash.
    Trash {
        /// What.
        path: WorkspacePath,
    },
}

/// A document another program moved: the list places it where its file is (names without
/// extension).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListPlace {
    /// The document's node.
    pub node: ListNodeId,
    /// Its new path in the list.
    pub path: Vec<String>,
}

/// What [`plan_disk`] answers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiskPlan {
    /// The file operations, in order.
    pub ops: Vec<DiskOp>,
    /// Moves another program made, for the list.
    pub list_changes: Vec<ListPlace>,
    /// Documents whose file was removed by another program: "Missing on this computer".
    pub missing: Vec<ListNodeId>,
    /// Files not in the shared workspace, sorted.
    pub outside: Vec<WorkspacePath>,
    /// Places the list names that a file outside the workspace holds, sorted.
    pub blocked: Vec<WorkspacePath>,
}

/// The plan that brings the disk to the list. `tombstoned` holds the ids of deleted documents,
/// those under a deleted folder included.
#[must_use]
pub fn plan_disk<S: BuildHasher>(
    list: &[ListEntry],
    tombstoned: &HashSet<[u8; 16], S>,
    disk: &DiskState<'_>,
) -> DiskPlan {
    let tombstoned: HashSet<[u8; 16]> = tombstoned.iter().copied().collect();
    let tombstoned = &tombstoned;
    let mut plan = DiskPlan::default();
    let wanted: Vec<(&ListEntry, WorkspacePath)> = list
        .iter()
        .filter(|entry| entry.kind == EntryKind::Document)
        .filter_map(|entry| Some((entry, file_path(&entry.path)?)))
        .collect();
    let keepers = choose_files(&wanted, tombstoned, disk, &mut plan.outside);
    let mut occupied: HashMap<String, WorkspacePath> = disk
        .documents
        .iter()
        .map(|file| (key(&file.path), file.path.clone()))
        .collect();
    let mut folders: HashSet<String> = disk.folders.iter().map(|f| key(f)).collect();
    for entry in list.iter().filter(|e| e.kind == EntryKind::Folder) {
        if occupied.contains_key(&key(&entry.path)) {
            plan.blocked.push(entry.path.clone());
        } else if folders.insert(key(&entry.path)) {
            plan.ops.push(DiskOp::CreateFolder {
                path: entry.path.clone(),
            });
        }
    }
    // A folder holds its place against a file of the same name.
    for folder in &folders {
        occupied.entry(folder.clone()).or_default();
    }
    let mut moves = Vec::new();
    let mut writes = Vec::new();
    for (entry, target) in &wanted {
        let Some(document) = entry.document else {
            continue;
        };
        let known = disk.known.get(&document);
        match keepers.get(&document) {
            None if known.is_some() => plan.missing.push(entry.node),
            None => writes.push((target.clone(), document)),
            Some(file) if file == target => {}
            Some(file) if known == Some(target) && known != Some(file) => {
                plan.list_changes.push(ListPlace {
                    node: entry.node,
                    path: list_path(file),
                });
            }
            Some(file) => moves.push((file.clone(), target.clone())),
        }
    }
    let targets: HashSet<String> = moves
        .iter()
        .map(|(_, to)| key(to))
        .chain(writes.iter().map(|(to, _)| key(to)))
        .collect();
    let freed = trash_deleted_on(&targets, disk, tombstoned, &mut occupied, &mut plan);
    order_moves(moves, &mut occupied, &mut plan);
    for (path, document) in writes {
        match occupied.entry(key(&path)) {
            Entry::Occupied(_) => plan.blocked.push(path),
            Entry::Vacant(place) => {
                place.insert(path.clone());
                plan.ops.push(DiskOp::Write { path, document });
            }
        }
    }
    trash(disk, list, tombstoned, &occupied, &freed, &mut plan);
    plan.outside.sort();
    plan.blocked.sort();
    plan
}

/// Per listed or deleted document id, the one file that keeps it: the file at the list's place,
/// else at the last known place, else the first by path. Every other file is outside.
fn choose_files(
    wanted: &[(&ListEntry, WorkspacePath)],
    tombstoned: &HashSet<[u8; 16]>,
    disk: &DiskState<'_>,
    outside: &mut Vec<WorkspacePath>,
) -> HashMap<[u8; 16], WorkspacePath> {
    let targets: HashMap<[u8; 16], &WorkspacePath> = wanted
        .iter()
        .filter_map(|(entry, path)| Some((entry.document?, path)))
        .collect();
    let mut by_id: BTreeMap<[u8; 16], Vec<&WorkspacePath>> = BTreeMap::new();
    for file in disk.documents {
        match file.document {
            Some(id) if targets.contains_key(&id) || tombstoned.contains(&id) => {
                by_id.entry(id).or_default().push(&file.path);
            }
            _ => outside.push(file.path.clone()),
        }
    }
    let mut keepers = HashMap::new();
    for (id, mut files) in by_id {
        files.sort();
        let at = |place: Option<&WorkspacePath>| {
            place.and_then(|place| files.iter().position(|f| *f == place))
        };
        let index = at(targets.get(&id).copied())
            .or_else(|| at(disk.known.get(&id)))
            .unwrap_or(0);
        for (other, path) in files.iter().enumerate() {
            if other != index {
                outside.push((*path).clone());
            }
        }
        keepers.insert(id, files[index].clone());
    }
    keepers
}

/// Trashes, before any move or write, the files of deleted documents that lie where a move or
/// write goes, so one plan follows a deletion and a rename onto its place. Answers their keys.
fn trash_deleted_on(
    targets: &HashSet<String>,
    disk: &DiskState<'_>,
    tombstoned: &HashSet<[u8; 16]>,
    occupied: &mut HashMap<String, WorkspacePath>,
    plan: &mut DiskPlan,
) -> HashSet<String> {
    let mut freed = HashSet::new();
    for file in disk.documents {
        let place = key(&file.path);
        let deleted = file.document.is_some_and(|id| tombstoned.contains(&id));
        if deleted && targets.contains(&place) && !plan.outside.contains(&file.path) {
            occupied.remove(&place);
            plan.ops.push(DiskOp::Trash {
                path: file.path.clone(),
            });
            freed.insert(place);
        }
    }
    freed
}

/// Emits the moves so that none lands on a place still taken: a move waits for the file on its
/// target to move away; a ring of moves goes through a temporary name.
fn order_moves(
    mut pending: Vec<(WorkspacePath, WorkspacePath)>,
    occupied: &mut HashMap<String, WorkspacePath>,
    plan: &mut DiskPlan,
) {
    pending.sort();
    while !pending.is_empty() {
        let movers: HashSet<String> = pending.iter().map(|(from, _)| key(from)).collect();
        let before = pending.len();
        let mut waiting = Vec::new();
        for (from, to) in pending {
            let target = key(&to);
            let free = !occupied.contains_key(&target) || target == key(&from);
            if free {
                occupied.remove(&key(&from));
                occupied.insert(target, to.clone());
                plan.ops.push(DiskOp::Move { from, to });
            } else if movers.contains(&target) {
                waiting.push((from, to));
            } else {
                plan.blocked.push(to);
            }
        }
        if !waiting.is_empty() && waiting.len() == before {
            let (from, to) = waiting.remove(0);
            let temporary = temporary_path(&from);
            occupied.remove(&key(&from));
            occupied.insert(key(&temporary), temporary.clone());
            plan.ops.push(DiskOp::Move {
                from,
                to: temporary.clone(),
            });
            waiting.insert(0, (temporary, to));
        }
        pending = waiting;
    }
}

/// Trashes the files of deleted documents and the folders the list no longer has once nothing
/// is left in them; a folder that goes takes its deleted files with it.
fn trash(
    disk: &DiskState<'_>,
    list: &[ListEntry],
    tombstoned: &HashSet<[u8; 16]>,
    occupied: &HashMap<String, WorkspacePath>,
    freed: &HashSet<String>,
    plan: &mut DiskPlan,
) {
    let listed: HashSet<String> = list
        .iter()
        .filter(|e| e.kind == EntryKind::Folder)
        .map(|e| key(&e.path))
        .collect();
    let deleted_files: Vec<&WorkspacePath> = disk
        .documents
        .iter()
        .filter(|f| f.document.is_some_and(|id| tombstoned.contains(&id)))
        .filter(|f| !plan.outside.contains(&f.path) && !freed.contains(&key(&f.path)))
        .map(|f| &f.path)
        .collect();
    let deleted_keys: HashSet<String> = deleted_files.iter().map(|f| key(f)).collect();
    let remaining: Vec<&WorkspacePath> = occupied
        .values()
        .filter(|path| !deleted_keys.contains(&key(path)))
        .collect();
    let candidates: Vec<&WorkspacePath> = disk
        .folders
        .iter()
        .filter(|folder| !listed.contains(&key(folder)))
        .filter(|folder| held_workspace_files(disk, plan, folder))
        .filter(|folder| !remaining.iter().any(|file| below(file, folder)))
        .collect();
    let mut gone: Vec<&WorkspacePath> = candidates
        .iter()
        .filter(|folder| {
            disk.folders
                .iter()
                .filter(|other| below(other, folder))
                .all(|other| candidates.contains(&other))
        })
        .copied()
        .collect();
    gone.sort();
    let outermost: Vec<&WorkspacePath> = gone
        .iter()
        .filter(|folder| !gone.iter().any(|other| below(folder, other)))
        .copied()
        .collect();
    for path in deleted_files {
        if !outermost.iter().any(|folder| below(path, folder)) {
            plan.ops.push(DiskOp::Trash { path: path.clone() });
        }
    }
    for folder in outermost {
        plan.ops.push(DiskOp::Trash {
            path: folder.clone(),
        });
    }
}

/// Whether `folder` held a file of the workspace before the plan: only such a folder is trashed
/// once emptied, never one another program made.
fn held_workspace_files(disk: &DiskState<'_>, plan: &DiskPlan, folder: &[String]) -> bool {
    disk.documents.iter().any(|file| {
        file.document.is_some() && !plan.outside.contains(&file.path) && below(&file.path, folder)
    })
}

/// Whether `path` lies strictly below `folder`.
fn below(path: &[String], folder: &[String]) -> bool {
    path.len() > folder.len() && key(&path[..folder.len()]) == key(folder)
}

/// The file path of a listed document: its shown path with `.curvyo`.
fn file_path(path: &[String]) -> Option<WorkspacePath> {
    let (name, folders) = path.split_last()?;
    let mut file = folders.to_vec();
    file.push(format!("{name}.{DOCUMENT_EXTENSION}"));
    Some(file)
}

/// The list path of a document file: without its extension.
fn list_path(file: &[String]) -> Vec<String> {
    let mut path = file.to_vec();
    if let Some(last) = path.last_mut() {
        *last = crate::workspace_plan::document_name(last).to_owned();
    }
    path
}

/// The temporary name a file passes through when moves form a ring: visible (the hosts refuse
/// names with a leading dot) and never a document file, so a scan does not take it for one.
fn temporary_path(path: &[String]) -> WorkspacePath {
    let mut temporary = path.to_vec();
    if let Some(last) = temporary.last_mut() {
        *last = format!("{last}.moving");
    }
    temporary
}

/// Paths compare like names: equal under `to_lowercase` (ADR 0037 decision 6).
fn key(path: &[String]) -> String {
    path.join("/").to_lowercase()
}
