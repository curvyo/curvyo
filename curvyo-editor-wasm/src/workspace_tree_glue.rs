//! The JSON the page and the session exchange about a shared workspace's live tree
//! (`specs/0095-workspace` criteria 35 to 41): list edits, the host's scan with the documents'
//! `share.json`, the disk plan's steps and the Deleted list. Plain Rust, so the host tests it;
//! `wasm_workspace_tree` only binds it (ADR 0001 §5: strings and scalars).

use std::collections::HashMap;

use curvyo_document_core::{
    Created, DeletedEntry, EntryKind, ListNodeId, ScannedDocument, WorkspaceShareRecord,
};
use curvyo_ui_core::workspace_text::change_time_text;
use curvyo_ui_core::workspace_tree_text::deleted_row_text;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::session::{DiskAnswer, DiskScan, DiskStep, ListEdit, hex, unhex};

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum EditJson {
    CreateDocument { path: Vec<String>, document: String },
    CreateFolder { path: Vec<String> },
    Move { from: Vec<String>, to: Vec<String> },
    Delete { path: Vec<String> },
    Restore { node: String },
}

/// The edit the page asks for (JSON tagged by `kind`: `create-document`, `create-folder`,
/// `move`, `delete`, `restore`); `now_seconds` dates a deletion. `None` for unreadable JSON.
#[must_use]
pub fn list_edit(json: &str, now_seconds: i64) -> Option<ListEdit> {
    Some(match serde_json::from_str::<EditJson>(json).ok()? {
        EditJson::CreateDocument { path, document } => ListEdit::CreateDocument {
            path,
            document: unhex::<16>(&document)?,
        },
        EditJson::CreateFolder { path } => ListEdit::CreateFolder { path },
        EditJson::Move { from, to } => ListEdit::Move { from, to },
        EditJson::Delete { path } => ListEdit::Delete {
            path,
            at: now_seconds,
        },
        EditJson::Restore { node } => ListEdit::Restore {
            node: ListNodeId::from_text(&node)?,
        },
    })
}

#[derive(Deserialize)]
struct ScanJson {
    #[serde(default)]
    folders: Vec<Vec<String>>,
    #[serde(default)]
    documents: Vec<ScannedJson>,
    #[serde(default)]
    known: Vec<KnownJson>,
}

#[derive(Deserialize)]
struct ScannedJson {
    path: Vec<String>,
    /// The file's `share.json` text, when it has one.
    #[serde(default)]
    share: Option<String>,
}

#[derive(Deserialize)]
struct KnownJson {
    document: String,
    path: Vec<String>,
}

/// The host's scan of a shared workspace (`scan_shared_workspace`): a file is the workspace's
/// document when its `share.json` is version 2 and names `workspace`.
#[must_use]
pub fn disk_scan(json: &str, workspace: [u8; 16]) -> Option<DiskScan> {
    let scan: ScanJson = serde_json::from_str(json).ok()?;
    let documents = scan
        .documents
        .into_iter()
        .map(|file| ScannedDocument {
            document: file
                .share
                .as_deref()
                .and_then(|text| WorkspaceShareRecord::from_share_json(text.as_bytes()))
                .filter(|record| record.workspace == workspace)
                .map(|record| record.document),
            path: file.path,
        })
        .collect();
    let known: HashMap<[u8; 16], Vec<String>> = scan
        .known
        .into_iter()
        .filter_map(|entry| Some((unhex::<16>(&entry.document)?, entry.path)))
        .collect();
    Some(DiskScan {
        folders: scan.folders,
        documents,
        known,
    })
}

/// The plan's answer as the page reads it: `{steps, missing, outside, blocked, settled}`.
#[must_use]
pub fn disk_answer_json(answer: &DiskAnswer) -> String {
    let steps: Vec<Value> = answer.steps.iter().map(step_json).collect();
    let settled: Vec<Value> = answer
        .settled
        .iter()
        .map(|(id, path)| json!({ "document": hex(id), "path": path }))
        .collect();
    json!({
        "steps": steps,
        "missing": answer.missing,
        "outside": answer.outside,
        "blocked": answer.blocked,
        "settled": settled,
    })
    .to_string()
}

fn step_json(step: &DiskStep) -> Value {
    match step {
        DiskStep::CreateFolder { path } => json!({ "kind": "create-folder", "path": path }),
        DiskStep::Move { from, to } => json!({ "kind": "move", "from": from, "to": to }),
        DiskStep::Write { path, document } => {
            json!({ "kind": "write", "path": path, "document": hex(document) })
        }
        DiskStep::Keep { path, document } => {
            json!({ "kind": "keep", "path": path, "document": hex(document) })
        }
        DiskStep::TrashDocument { path, document, by } => json!({
            "kind": "trash-document",
            "path": path,
            "document": hex(document),
            "by": by,
        }),
        DiskStep::Trash { path } => json!({ "kind": "trash", "path": path }),
    }
}

/// The Deleted list's rows (`ux-notes.md` section 5): `[{node, name, kind, document, line}]`,
/// `me` being this device's label, times local `offset_minutes` east of UTC against `now`.
#[must_use]
pub fn deleted_json(entries: &[DeletedEntry], me: &str, now: u64, offset_minutes: i32) -> String {
    let rows: Vec<Value> = entries
        .iter()
        .map(|entry| {
            let at = u64::try_from(entry.at).unwrap_or(0);
            let time = change_time_text(at, now, offset_minutes);
            json!({
                "node": entry.node.to_text(),
                "name": entry.name,
                "kind": match entry.kind {
                    EntryKind::Folder => "folder",
                    EntryKind::Document => "document",
                },
                "document": entry.document.map(|id| hex(&id)).unwrap_or_default(),
                "line": deleted_row_text(&entry.by, entry.by == me, &time),
            })
        })
        .collect();
    Value::Array(rows).to_string()
}

/// Creation stamps at `now_seconds`, each with its own id drawn from `seed` and a counter.
pub fn stamps(seed: [u8; 16], now_seconds: i64) -> impl FnMut() -> Created {
    let mut counter: u32 = 0;
    move || {
        counter = counter.wrapping_add(1);
        let mut id = seed;
        for (byte, add) in id[12..].iter_mut().zip(counter.to_le_bytes()) {
            *byte ^= add;
        }
        Created {
            at: now_seconds,
            id,
        }
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Document, DocumentVersion};

    use super::*;

    #[test]
    fn edits_read_from_the_page() {
        assert_eq!(
            list_edit(
                r#"{"kind":"create-document","path":["Lid.curvyo"],"document":"11111111111111111111111111111111"}"#,
                0
            ),
            Some(ListEdit::CreateDocument {
                path: vec!["Lid.curvyo".to_owned()],
                document: [0x11; 16]
            })
        );
        assert_eq!(
            list_edit(r#"{"kind":"delete","path":["Lid.curvyo"]}"#, 42),
            Some(ListEdit::Delete {
                path: vec!["Lid.curvyo".to_owned()],
                at: 42
            })
        );
        assert_eq!(list_edit(r#"{"kind":"restore","node":"x"}"#, 0), None);
        assert_eq!(list_edit("not json", 0), None);
    }

    /// Criterion 41: only a version 2 record of this workspace makes a file the workspace's.
    #[test]
    fn a_scan_keeps_only_this_workspaces_ids() {
        let record = |workspace: [u8; 16]| {
            let document = Document::new(1);
            document.set_workspace_share(Some(WorkspaceShareRecord {
                workspace,
                document: [0x22; 16],
                synced_version: DocumentVersion::never(),
            }));
            let bytes = curvyo_document_core::pack(&document, "t").unwrap();
            let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
            let mut text = String::new();
            std::io::Read::read_to_string(&mut archive.by_name("share.json").unwrap(), &mut text)
                .unwrap();
            text
        };
        let json = json!({
            "folders": [["Sides"]],
            "documents": [
                {"path": ["Lid.curvyo"], "share": record([0x57; 16])},
                {"path": ["Other.curvyo"], "share": record([0x58; 16])},
                {"path": ["Plain.curvyo"]},
            ],
            "known": [{"document": hex(&[0x22; 16]), "path": ["Lid.curvyo"]}],
        })
        .to_string();
        let scan = disk_scan(&json, [0x57; 16]).unwrap();
        let ids: Vec<Option<[u8; 16]>> = scan.documents.iter().map(|d| d.document).collect();
        assert_eq!(ids, vec![Some([0x22; 16]), None, None]);
        assert_eq!(scan.known.len(), 1);
        assert_eq!(scan.folders, vec![vec!["Sides".to_owned()]]);
    }

    #[test]
    fn steps_and_stamps_are_written_for_the_page() {
        let answer = DiskAnswer {
            steps: vec![DiskStep::TrashDocument {
                path: vec!["Lid.curvyo".to_owned()],
                document: [1; 16],
                by: "Ben".to_owned(),
            }],
            ..DiskAnswer::default()
        };
        let value: Value = serde_json::from_str(&disk_answer_json(&answer)).unwrap();
        assert_eq!(value["steps"][0]["kind"], "trash-document");
        assert_eq!(value["steps"][0]["by"], "Ben");
        let mut next = stamps([0; 16], 7);
        assert_ne!(next().id, next().id);
    }
}
