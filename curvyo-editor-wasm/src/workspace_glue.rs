//! The open workspace as the File tab's Workspace section sees it (`specs/0094-workspace`
//! criteria 5, 6, 11, 13 to 19): the host's scan, the expanded folders, the rows and the planned
//! file operations with their words, as JSON. Plain Rust, so the host tests it;
//! `wasm_workspace` only binds it (ADR 0001 §5: strings and scalars).

use std::collections::HashSet;

use curvyo_document_core::{EntryKind, FileOp, PlanRefusal, Planned, WorkspaceTree};
use curvyo_ui_core::workspace_rows::{WorkspaceEntry, workspace_rows};
use curvyo_ui_core::workspace_text::{
    change_time_text, created_text, delete_confirm_text, deleted_text, document_count_text,
    move_refusal_text, moved_text, name_refusal_text, no_trash_text, row_label,
};
use curvyo_ui_core::workspace_tree_text::{
    MISSING_TEXT, NOT_IN_WORKSPACE_TEXT, RECEIVING_TEXT, shared_delete_confirm_text,
};
use serde::Deserialize;
use serde_json::{Value, json};

/// One entry of the host's scan (`curvyo-app`'s `ScannedPayload`).
#[derive(Deserialize)]
struct Scanned {
    path: Vec<String>,
    kind: String,
    #[serde(default)]
    modified: u64,
}

/// What the page asks to plan.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum PlanRequest {
    NewDocument {
        folder: Vec<String>,
        name: String,
    },
    NewFolder {
        folder: Vec<String>,
        name: String,
    },
    Rename {
        path: Vec<String>,
        name: String,
    },
    Move {
        path: Vec<String>,
        folder: Vec<String>,
    },
    Duplicate {
        path: Vec<String>,
    },
    Delete {
        path: Vec<String>,
    },
}

/// The open workspace: its name, the last scan, the folders the maker expanded and, once it is
/// shared, the rows' states from the disk plan.
#[derive(Debug, Default)]
pub struct WorkspaceGlue {
    name: String,
    entries: Vec<WorkspaceEntry>,
    tree: WorkspaceTree,
    expanded: HashSet<Vec<String>>,
    shared: SharedStates,
}

/// The rows' states of a shared workspace (criteria 35, 40, 41).
#[derive(Debug, Default, Deserialize, PartialEq, Eq)]
struct SharedStates {
    #[serde(default)]
    shared: bool,
    /// Files that are not part of the shared workspace.
    #[serde(default)]
    outside: Vec<Vec<String>>,
    /// Documents the list names whose file is gone; shown as rows of their own.
    #[serde(default)]
    missing: Vec<Vec<String>>,
    /// Documents the list names whose file has not arrived yet; shown as rows of their own.
    #[serde(default)]
    receiving: Vec<Vec<String>>,
}

impl WorkspaceGlue {
    /// An empty workspace named `name` (the folder's name), every folder collapsed.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            ..Self::default()
        }
    }

    /// Takes the host's scan; whether it differs from the last one. Unreadable JSON changes
    /// nothing.
    pub fn set_scan(&mut self, json: &str) -> bool {
        let Ok(scanned) = serde_json::from_str::<Vec<Scanned>>(json) else {
            return false;
        };
        let entries: Vec<WorkspaceEntry> = scanned
            .into_iter()
            .filter(|entry| !entry.path.is_empty())
            .map(|entry| WorkspaceEntry {
                path: entry.path,
                kind: if entry.kind == "folder" {
                    EntryKind::Folder
                } else {
                    EntryKind::Document
                },
                modified: entry.modified,
            })
            .collect();
        if entries == self.entries {
            return false;
        }
        self.tree =
            WorkspaceTree::new(entries.iter().map(|entry| (entry.path.clone(), entry.kind)));
        self.entries = entries;
        true
    }

    /// Takes the rows' states of a shared workspace (JSON `{shared, outside, missing,
    /// receiving}` with file paths); whether they changed. Unreadable JSON changes nothing.
    pub fn set_shared_states(&mut self, json: &str) -> bool {
        let Ok(states) = serde_json::from_str::<SharedStates>(json) else {
            return false;
        };
        if states == self.shared {
            return false;
        }
        self.shared = states;
        true
    }

    /// The scanned entries and the documents a shared workspace lists but has no file of.
    fn shown_entries(&self) -> Vec<WorkspaceEntry> {
        let mut entries = self.entries.clone();
        for path in self.shared.missing.iter().chain(&self.shared.receiving) {
            if self.tree.kind(path).is_none() && !path.is_empty() {
                entries.push(WorkspaceEntry {
                    path: path.clone(),
                    kind: EntryKind::Document,
                    modified: 0,
                });
            }
        }
        entries
    }

    /// A shared workspace's state of the row at `path`: its code and its words.
    fn row_state(&self, path: &[String]) -> (&'static str, &'static str) {
        let has = |list: &[Vec<String>]| list.iter().any(|p| p == path);
        if has(&self.shared.outside) {
            ("outside", NOT_IN_WORKSPACE_TEXT)
        } else if has(&self.shared.missing) {
            ("missing", MISSING_TEXT)
        } else if has(&self.shared.receiving) {
            ("receiving", RECEIVING_TEXT)
        } else {
            ("", "")
        }
    }

    /// The header line and every shown row, as JSON: `{documents, header, rows: [{path, kind,
    /// name, level, posinset, setsize, expanded, count, time, label, state}]}`. Times are local,
    /// `offset_minutes` east of UTC, against `now` in seconds since the Unix epoch; `state` is a
    /// shared workspace's row state (`outside`, `missing`, `receiving`), its words replace the
    /// time.
    #[must_use]
    pub fn rows_json(&self, now: u64, offset_minutes: i32) -> String {
        let documents = self.tree.document_count(&[]);
        let rows: Vec<Value> = workspace_rows(&self.shown_entries(), &self.expanded)
            .into_iter()
            .map(|row| {
                let (state, state_text) = self.row_state(&row.path);
                let time = match row.kind {
                    EntryKind::Document if !state_text.is_empty() => state_text.to_owned(),
                    EntryKind::Document => change_time_text(row.modified, now, offset_minutes),
                    EntryKind::Folder => String::new(),
                };
                let label = row_label(&row.name, row.kind, row.count, row.expanded, &time);
                json!({
                    "path": row.path,
                    "kind": kind_code(row.kind),
                    "name": row.name,
                    "level": row.level,
                    "posinset": row.posinset,
                    "setsize": row.setsize,
                    "expanded": row.expanded,
                    "count": row.count,
                    "time": time,
                    "label": label,
                    "state": state,
                })
            })
            .collect();
        json!({
            "documents": documents,
            "header": document_count_text(documents),
            "rows": rows,
        })
        .to_string()
    }

    /// Expands or collapses the folder at `path` (JSON array); whether it is a folder.
    pub fn set_expanded(&mut self, path: &str, expanded: bool) -> bool {
        let Some(path) = parse_path(path) else {
            return false;
        };
        if path.is_empty() || self.tree.kind(&path) != Some(EntryKind::Folder) {
            return false;
        }
        if expanded {
            self.expanded.insert(path);
        } else {
            self.expanded.remove(&path);
        }
        true
    }

    /// Whether the last scan holds the entry at `path` (JSON array): the open document's file is
    /// gone when not (criterion 7).
    #[must_use]
    pub fn contains(&self, path: &str) -> bool {
        parse_path(path).is_some_and(|path| !path.is_empty() && self.tree.kind(&path).is_some())
    }

    /// Keeps the expanded folders of a move or rename from `from` to `to` (JSON arrays) open at
    /// their new place.
    pub fn moved(&mut self, from: &str, to: &str) {
        let (Some(from), Some(to)) = (parse_path(from), parse_path(to)) else {
            return;
        };
        let moved: Vec<Vec<String>> = self
            .expanded
            .iter()
            .filter(|path| path.starts_with(&from))
            .cloned()
            .collect();
        for path in moved {
            self.expanded.remove(&path);
            let mut renamed = to.clone();
            renamed.extend_from_slice(&path[from.len()..]);
            self.expanded.insert(renamed);
        }
    }

    /// Plans what the page asks (JSON tagged by `kind`: `new-document`, `new-folder`, `rename`,
    /// `move`, `duplicate`, `delete`): `{op, name, status, confirm,
    /// noTrash}` with the operation for the host, the final name and the words of its outcome,
    /// or `{refusal}` with the reason; `{}` for unreadable JSON.
    #[must_use]
    pub fn plan(&self, request: &str) -> String {
        let Ok(request) = serde_json::from_str::<PlanRequest>(request) else {
            return "{}".to_owned();
        };
        match request {
            PlanRequest::NewDocument { folder, name } => created(
                self.tree.plan_new_document(&folder, &name),
                EntryKind::Document,
            ),
            PlanRequest::NewFolder { folder, name } => {
                created(self.tree.plan_new_folder(&folder, &name), EntryKind::Folder)
            }
            PlanRequest::Duplicate { path } => {
                created(self.tree.plan_duplicate(&path), EntryKind::Document)
            }
            PlanRequest::Rename { path, name } => {
                let kind = self.tree.kind(&path).unwrap_or(EntryKind::Document);
                match self.tree.plan_rename(&path, &name) {
                    Ok(op) => json!({ "op": op_json(&op), "name": name, "status": "" }),
                    Err(refusal) => json!({ "refusal": name_refusal_text(&refusal, kind) }),
                }
                .to_string()
            }
            PlanRequest::Move { path, folder } => {
                let name = self.entry_name(&path);
                let folder_name = self.folder_name(&folder);
                match self.tree.plan_move(&path, &folder) {
                    Ok(op) => json!({
                        "op": op_json(&op),
                        "name": name,
                        "status": moved_text(&name, &folder_name),
                    }),
                    Err(refusal) => {
                        json!({ "refusal": move_refusal_text(&refusal, &name, &folder_name) })
                    }
                }
                .to_string()
            }
            PlanRequest::Delete { path } => {
                let name = self.entry_name(&path);
                match self.tree.plan_delete(&path) {
                    Ok(op) => {
                        let kind = self.tree.kind(&path).unwrap_or(EntryKind::Document);
                        let documents = self.tree.document_count(&path);
                        let confirm = if self.shared.shared {
                            shared_delete_confirm_text(&name, kind == EntryKind::Folder, documents)
                        } else {
                            delete_confirm_text(&name, kind, documents)
                        };
                        json!({
                            "op": op_json(&op),
                            "name": name,
                            "confirm": confirm,
                            "noTrash": no_trash_text(&name),
                            "status": deleted_text(&name),
                        })
                    }
                    Err(refusal) => json!({
                        "refusal": name_refusal_text(&refusal, EntryKind::Document),
                    }),
                }
                .to_string()
            }
        }
    }

    /// The name a row shows for the entry at `path`.
    fn entry_name(&self, path: &[String]) -> String {
        let last = path.last().map_or("", String::as_str);
        match self.tree.kind(path) {
            Some(EntryKind::Document) => curvyo_document_core::document_name(last).to_owned(),
            _ => last.to_owned(),
        }
    }

    /// The name of the folder at `path`; the workspace's name for its root.
    fn folder_name(&self, path: &[String]) -> String {
        path.last().cloned().unwrap_or_else(|| self.name.clone())
    }
}

/// The answer for a planned creation: the operation, the final name and the status line.
fn created(planned: Result<Planned, PlanRefusal>, kind: EntryKind) -> String {
    match planned {
        Ok(planned) => json!({
            "op": op_json(&planned.op),
            "name": planned.name,
            "status": created_text(&planned.name),
        }),
        Err(refusal) => json!({ "refusal": name_refusal_text(&refusal, kind) }),
    }
    .to_string()
}

fn parse_path(json: &str) -> Option<Vec<String>> {
    serde_json::from_str(json).ok()
}

const fn kind_code(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Folder => "folder",
        EntryKind::Document => "document",
    }
}

/// An operation as the host's `apply_workspace_op` reads it.
fn op_json(op: &FileOp) -> Value {
    match op {
        FileOp::CreateDocument { path } => json!({ "kind": "create-document", "path": path }),
        FileOp::CreateFolder { path } => json!({ "kind": "create-folder", "path": path }),
        FileOp::Move { from, to } => json!({ "kind": "move", "from": from, "to": to }),
        FileOp::Trash { path } => json!({ "kind": "trash", "path": path }),
    }
}

/// The bytes of a copy of the document `bytes` (Duplicate, criterion 19): the same drawing and
/// history without the share record, so the copy is no member of the original's room.
///
/// # Errors
/// The open error's sentence when `bytes` is not a document this build reads.
pub fn workspace_copy_bytes(bytes: &[u8], app_version: &str) -> Result<Vec<u8>, String> {
    let document = curvyo_document_core::unpack(1, bytes).map_err(|error| error.to_string())?;
    curvyo_document_core::pack_snapshot(&document, app_version).map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "workspace_glue_tests.rs"]
mod tests;
