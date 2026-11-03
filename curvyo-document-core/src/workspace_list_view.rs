//! What a workspace list replica shows, derived on read and never written back (ADR 0037
//! decision 6): the visible entries with their shown names and paths, and the deleted ones.
//!
//! Among visible siblings of one kind whose names are equal under `to_lowercase`, ordered by
//! `created`, the first keeps its name and the others get " 2", " 3" (criterion 38). A name a
//! peer wrote that this build refuses is shown with `_` in place of each refused character.

use std::collections::{HashMap, HashSet};

use loro::{LoroValue, TreeID, TreeParentId};

use crate::workspace_list::{
    Created, KEY_AT, KEY_BY, KEY_CREATED, KEY_DELETED, KEY_DOCUMENT, KEY_KIND, KEY_NAME,
    KIND_DOCUMENT, KIND_FOLDER, ListNodeId, WorkspaceList,
};
use crate::workspace_names::{MAX_ENTRY_NAME_CHARS, check_entry_name, free_entry_name};
use crate::workspace_plan::EntryKind;

/// One visible entry of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEntry {
    /// The node.
    pub node: ListNodeId,
    /// The folder it is in; `None` at the top.
    pub parent: Option<ListNodeId>,
    /// Folder or document.
    pub kind: EntryKind,
    /// The shown names from the top down to this entry's own, without extension.
    pub path: Vec<String>,
    /// A document's id.
    pub document: Option<[u8; 16]>,
}

/// One entry deleted by a member, as the Deleted list shows it (criterion 40). Entries hidden
/// only because a folder above them is deleted are not listed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedEntry {
    /// The node.
    pub node: ListNodeId,
    /// Folder or document.
    pub kind: EntryKind,
    /// Its name register, with refused characters replaced.
    pub name: String,
    /// A document's id.
    pub document: Option<[u8; 16]>,
    /// The member who deleted it.
    pub by: String,
    /// When, in unix seconds.
    pub at: i64,
    /// Its creation stamp.
    pub created: Created,
}

/// A node's meta as read.
struct NodeRead {
    kind: EntryKind,
    name: String,
    document: Option<[u8; 16]>,
    created: Created,
    deleted: Option<(String, i64)>,
}

impl WorkspaceList {
    /// Every visible entry, folders and documents, with its shown path; parents before
    /// children.
    #[must_use]
    pub fn entries(&self) -> Vec<ListEntry> {
        let reads = self.reads();
        let mut out = Vec::new();
        self.collect(TreeParentId::Root, &[], &reads, &mut out);
        out
    }

    /// The node of the visible document with the id `document`.
    #[must_use]
    pub fn find_document(&self, document: [u8; 16]) -> Option<ListNodeId> {
        self.entries()
            .into_iter()
            .find(|entry| entry.document == Some(document))
            .map(|entry| entry.node)
    }

    /// The ids of every document that is not shown: deleted, or under a deleted folder. Their
    /// files go to the trash; their state stays in the room (criterion 40).
    #[must_use]
    pub fn tombstoned_documents(&self) -> HashSet<[u8; 16]> {
        let shown: HashSet<[u8; 16]> = self
            .entries()
            .into_iter()
            .filter_map(|entry| entry.document)
            .collect();
        self.reads()
            .into_values()
            .filter_map(|read| read.document)
            .filter(|document| !shown.contains(document))
            .collect()
    }

    /// Every entry a member deleted, most recent first.
    #[must_use]
    pub fn deleted_entries(&self) -> Vec<DeletedEntry> {
        let mut out: Vec<DeletedEntry> = self
            .reads()
            .into_iter()
            .filter_map(|(id, read)| {
                let (by, at) = read.deleted?;
                Some(DeletedEntry {
                    node: ListNodeId::of(id),
                    kind: read.kind,
                    name: shown_name(&read.name),
                    document: read.document,
                    by,
                    at,
                    created: read.created,
                })
            })
            .collect();
        out.sort_by(|a, b| b.at.cmp(&a.at).then(a.node.cmp(&b.node)));
        out
    }

    /// The kind of `node` when it is shown: it and every folder above it are not deleted.
    pub(crate) fn visible_kind(&self, node: ListNodeId) -> Option<EntryKind> {
        let tree = self.tree();
        let id = node.tree_id();
        if !tree.contains(id) || tree.is_node_deleted(&id).unwrap_or(true) {
            return None;
        }
        let kind = read_node(self, id)?.kind;
        let mut current = Some(id);
        while let Some(at) = current {
            let read = read_node(self, at)?;
            if read.deleted.is_some() {
                return None;
            }
            current = match tree.parent(at) {
                Some(TreeParentId::Node(parent)) => Some(parent),
                _ => None,
            };
        }
        Some(kind)
    }

    /// Every node of the tree with a readable meta.
    fn reads(&self) -> HashMap<TreeID, NodeRead> {
        self.tree()
            .nodes()
            .into_iter()
            .filter_map(|id| Some((id, read_node(self, id)?)))
            .collect()
    }

    fn collect(
        &self,
        parent: TreeParentId,
        path: &[String],
        reads: &HashMap<TreeID, NodeRead>,
        out: &mut Vec<ListEntry>,
    ) {
        let children = self.tree().children(parent).unwrap_or_default();
        let mut visible: Vec<(TreeID, &NodeRead)> = children
            .into_iter()
            .filter_map(|id| Some((id, reads.get(&id)?)))
            .filter(|(_, read)| read.deleted.is_none())
            .collect();
        visible.sort_by(|a, b| a.1.created.cmp(&b.1.created).then(a.0.cmp(&b.0)));
        let mut taken: HashMap<EntryKind, Vec<String>> = HashMap::new();
        for (id, read) in visible {
            let siblings = taken.entry(read.kind).or_default();
            let name =
                free_entry_name(&shown_name(&read.name), siblings.iter().map(String::as_str));
            siblings.push(name.clone());
            let mut entry_path = path.to_vec();
            entry_path.push(name);
            out.push(ListEntry {
                node: ListNodeId::of(id),
                parent: match parent {
                    TreeParentId::Node(folder) => Some(ListNodeId::of(folder)),
                    _ => None,
                },
                kind: read.kind,
                path: entry_path.clone(),
                document: read.document,
            });
            if read.kind == EntryKind::Folder {
                self.collect(TreeParentId::Node(id), &entry_path, reads, out);
            }
        }
    }
}

fn read_node(list: &WorkspaceList, id: TreeID) -> Option<NodeRead> {
    let LoroValue::Map(meta) = list.tree().get_meta(id).ok()?.get_value() else {
        return None;
    };
    let kind = match meta.get(KEY_KIND)? {
        LoroValue::String(kind) if kind.as_str() == KIND_FOLDER => EntryKind::Folder,
        LoroValue::String(kind) if kind.as_str() == KIND_DOCUMENT => EntryKind::Document,
        _ => return None,
    };
    let LoroValue::String(name) = meta.get(KEY_NAME)? else {
        return None;
    };
    let document = match (kind, meta.get(KEY_DOCUMENT)) {
        (EntryKind::Document, Some(LoroValue::Binary(bytes))) => {
            Some(<[u8; 16]>::try_from(bytes.as_slice()).ok()?)
        }
        (EntryKind::Document, _) => return None,
        (EntryKind::Folder, _) => None,
    };
    let created = read_created(meta.get(KEY_CREATED)?)?;
    let deleted = match meta.get(KEY_DELETED) {
        Some(LoroValue::Map(mark)) => Some((
            match mark.get(KEY_BY) {
                Some(LoroValue::String(by)) => by.to_string(),
                _ => String::new(),
            },
            match mark.get(KEY_AT) {
                Some(LoroValue::I64(at)) => *at,
                _ => 0,
            },
        )),
        Some(_) => Some((String::new(), 0)),
        None => None,
    };
    Some(NodeRead {
        kind,
        name: name.to_string(),
        document,
        created,
        deleted,
    })
}

fn read_created(value: &LoroValue) -> Option<Created> {
    let LoroValue::List(parts) = value else {
        return None;
    };
    let (Some(LoroValue::I64(at)), Some(LoroValue::Binary(id))) = (parts.first(), parts.get(1))
    else {
        return None;
    };
    Some(Created {
        at: *at,
        id: <[u8; 16]>::try_from(id.as_slice()).ok()?,
    })
}

/// `name` as shown and written to disk: a name this build refuses has `_` in place of each
/// refused character (ADR 0037 decision 6).
pub(crate) fn shown_name(name: &str) -> String {
    if check_entry_name(name).is_ok() {
        return name.to_owned();
    }
    let refused: HashSet<char> = [':', '*', '?', '"', '<', '>', '|', '/', '\\']
        .into_iter()
        .collect();
    let mut shown: String = name
        .chars()
        .take(MAX_ENTRY_NAME_CHARS)
        .map(|c| {
            if c.is_control() || refused.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    if shown.starts_with('.') {
        shown.replace_range(..1, "_");
    }
    while shown.ends_with(['.', ' ']) {
        shown.pop();
        shown.push('_');
        if check_entry_name(&shown).is_ok() {
            break;
        }
    }
    if check_entry_name(&shown).is_err() {
        // An empty name or a reserved device name.
        shown.push('_');
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::shown_name;

    #[test]
    fn refused_names_from_a_peer_are_shown_with_underscores() {
        assert_eq!(shown_name("Lid"), "Lid");
        assert_eq!(shown_name("a/b:c"), "a_b_c");
        assert_eq!(shown_name(".hidden"), "_hidden");
        assert_eq!(shown_name("end. "), "end._");
        assert_eq!(shown_name("CON"), "CON_");
        assert_eq!(shown_name(""), "_");
        assert_eq!(shown_name("tab\there"), "tab_here");
    }
}
