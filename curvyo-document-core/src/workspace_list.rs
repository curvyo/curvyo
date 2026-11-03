//! The replica of a shared workspace's document list (`specs/0095-workspace` criteria 35 to 40,
//! ADR 0037 decision 6): a Loro movable tree of folders and documents, with no Loro type in its
//! API.
//!
//! A node's meta holds `kind` (`folder` | `document`), `name` (a register, without extension),
//! `document` (a document's 16-byte id), `created` (`[unix seconds, 16-byte id]`, written once)
//! and `deleted` (absent, or `{by, at}`). The shown names and the paths are derived on read
//! (`crate::workspace_list_view`); nothing derived is written back.

use std::cell::RefCell;
use std::collections::HashMap;

use loro::{
    ExportMode, LoroDoc, LoroMap, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer,
};

use crate::document::DocumentVersion;
use crate::workspace_names::{NameRefusal, check_entry_name};
use crate::workspace_plan::{DOCUMENT_EXTENSION, EntryKind};

/// The list format this build writes and the newest it reads (the root register
/// `format_version`).
pub const WORKSPACE_LIST_FORMAT_VERSION: i64 = 1;

const TREE: &str = "list";
const ROOT: &str = "workspace";
const KEY_FORMAT_VERSION: &str = "format_version";
pub(crate) const KEY_KIND: &str = "kind";
pub(crate) const KEY_NAME: &str = "name";
pub(crate) const KEY_DOCUMENT: &str = "document";
pub(crate) const KEY_CREATED: &str = "created";
pub(crate) const KEY_DELETED: &str = "deleted";
pub(crate) const KEY_BY: &str = "by";
pub(crate) const KEY_AT: &str = "at";
pub(crate) const KIND_FOLDER: &str = "folder";
pub(crate) const KIND_DOCUMENT: &str = "document";

/// The identity of one node of the list; stable across renames, moves and deletions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ListNodeId {
    pub(crate) peer: u64,
    pub(crate) counter: i32,
}

impl ListNodeId {
    /// The id as text, `counter@peer`, for the page.
    #[must_use]
    pub fn to_text(self) -> String {
        format!("{}@{}", self.counter, self.peer)
    }

    /// The inverse of [`ListNodeId::to_text`]; `None` for a malformed text.
    #[must_use]
    pub fn from_text(text: &str) -> Option<Self> {
        let (counter, peer) = text.split_once('@')?;
        Some(Self {
            peer: peer.parse().ok()?,
            counter: counter.parse().ok()?,
        })
    }

    pub(crate) fn tree_id(self) -> TreeID {
        TreeID::new(self.peer, self.counter)
    }

    pub(crate) fn of(id: TreeID) -> Self {
        Self {
            peer: id.peer,
            counter: id.counter,
        }
    }
}

/// When and by which random id a node was created: orders siblings of one name (criterion 38).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Created {
    /// Unix seconds.
    pub at: i64,
    /// 16 random bytes the caller drew.
    pub id: [u8; 16],
}

/// Why a list operation or import did nothing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ListError {
    /// The name breaks a naming rule.
    #[error("{0}")]
    Name(#[from] NameRefusal),
    /// No visible node has that id.
    #[error("no such entry")]
    NotFound,
    /// A folder would move into itself or below itself.
    #[error("a folder cannot move into itself")]
    Loop,
    /// The target of a create or a move is not a folder.
    #[error("not a folder")]
    NotAFolder,
    /// The list could not be encoded.
    #[error("the list could not be encoded")]
    Encode,
    /// The bytes are not a list this build reads.
    #[error("the bytes are not a workspace list")]
    Malformed,
    /// The list was written by a newer build.
    #[error("the list was written by a newer version")]
    FormatTooNew,
}

/// What an import changed that the maker is told about.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListImportReport {
    /// Folders this replica moved whose move was not applied because, with a concurrent move,
    /// it would have closed a loop (criterion 39).
    pub dropped_moves: Vec<ListNodeId>,
}

/// One replica of the list.
pub struct WorkspaceList {
    pub(crate) doc: LoroDoc,
    /// This replica's moves not yet seen overturned, node to the folder it was moved into.
    moves: RefCell<HashMap<TreeID, TreeParentId>>,
}

impl WorkspaceList {
    /// An empty list bound to the Loro peer `peer` (drawn by the caller).
    ///
    /// # Panics
    /// Does not panic in practice: a fresh root map takes a known-valid key.
    #[must_use]
    pub fn new(peer: u64) -> Self {
        let list = Self::bound(peer);
        // invariant: a fresh, attached root map takes a known-valid key.
        #[allow(clippy::unwrap_used)]
        list.doc
            .get_map(ROOT)
            .insert(KEY_FORMAT_VERSION, WORKSPACE_LIST_FORMAT_VERSION)
            .unwrap();
        list.doc.commit();
        list
    }

    /// The list a snapshot holds, bound to the Loro peer `peer`.
    ///
    /// # Errors
    /// [`ListError::Malformed`], [`ListError::FormatTooNew`].
    pub fn from_snapshot(peer: u64, bytes: &[u8]) -> Result<Self, ListError> {
        let list = Self::bound(peer);
        list.doc.import(bytes).map_err(|_| ListError::Malformed)?;
        list.check_format()?;
        Ok(list)
    }

    fn bound(peer: u64) -> Self {
        let doc = LoroDoc::new();
        // invariant: a fresh doc with no operations takes any peer id.
        #[allow(clippy::unwrap_used)]
        doc.set_peer_id(peer).unwrap();
        Self {
            doc,
            moves: RefCell::default(),
        }
    }

    fn check_format(&self) -> Result<(), ListError> {
        match self.doc.get_map(ROOT).get(KEY_FORMAT_VERSION) {
            Some(ValueOrContainer::Value(LoroValue::I64(version))) => {
                if version > WORKSPACE_LIST_FORMAT_VERSION {
                    Err(ListError::FormatTooNew)
                } else {
                    Ok(())
                }
            }
            _ => Err(ListError::Malformed),
        }
    }

    pub(crate) fn tree(&self) -> LoroTree {
        self.doc.get_tree(TREE)
    }

    /// The list's version, for [`WorkspaceList::export_updates`].
    #[must_use]
    pub fn version(&self) -> DocumentVersion {
        DocumentVersion(self.doc.oplog_frontiers())
    }

    /// Everything this replica holds since `since`.
    ///
    /// # Errors
    /// [`ListError::Encode`].
    pub fn export_updates(&self, since: &DocumentVersion) -> Result<Vec<u8>, ListError> {
        self.doc.commit();
        let known = self.doc.frontiers_to_vv(&since.0).unwrap_or_default();
        self.doc
            .export(ExportMode::updates(&known))
            .map_err(|_| ListError::Encode)
    }

    /// The whole list, for `list.loro` and the workspace snapshot.
    ///
    /// # Errors
    /// [`ListError::Encode`].
    pub fn snapshot(&self) -> Result<Vec<u8>, ListError> {
        self.doc.commit();
        self.doc
            .export(ExportMode::Snapshot)
            .map_err(|_| ListError::Encode)
    }

    /// Merges updates of another replica and says which of this replica's folder moves were not
    /// applied because they would have closed a loop.
    ///
    /// # Errors
    /// [`ListError::Malformed`], [`ListError::FormatTooNew`].
    pub fn import_updates(&self, bytes: &[u8]) -> Result<ListImportReport, ListError> {
        self.doc.import(bytes).map_err(|_| ListError::Malformed)?;
        self.check_format()?;
        Ok(self.overturned_moves())
    }

    /// Drops the recorded moves whose node no longer sits where this replica moved it, and
    /// reports those whose target now lies below the node: the loop rule dropped them.
    fn overturned_moves(&self) -> ListImportReport {
        let tree = self.tree();
        let mut report = ListImportReport::default();
        self.moves.borrow_mut().retain(|node, target| {
            if tree.parent(*node).as_ref() == Some(target) {
                return true;
            }
            if let TreeParentId::Node(folder) = target
                && self.is_at_or_below(*folder, *node)
            {
                report.dropped_moves.push(ListNodeId::of(*node));
            }
            false
        });
        report.dropped_moves.sort();
        report
    }

    /// Whether `node` is `ancestor` or lies below it.
    pub(crate) fn is_at_or_below(&self, node: TreeID, ancestor: TreeID) -> bool {
        let tree = self.tree();
        let mut current = Some(node);
        while let Some(at) = current {
            if at == ancestor {
                return true;
            }
            current = match tree.parent(at) {
                Some(TreeParentId::Node(parent)) => Some(parent),
                _ => None,
            };
        }
        false
    }

    /// A new folder in `parent` (the top when `None`).
    ///
    /// # Errors
    /// [`ListError::Name`], [`ListError::NotFound`], [`ListError::NotAFolder`].
    pub fn create_folder(
        &self,
        parent: Option<ListNodeId>,
        name: &str,
        created: Created,
    ) -> Result<ListNodeId, ListError> {
        self.create(parent, name, created, None)
    }

    /// A new document entry with the document id `document` in `parent`.
    ///
    /// # Errors
    /// [`ListError::Name`], [`ListError::NotFound`], [`ListError::NotAFolder`].
    pub fn create_document(
        &self,
        parent: Option<ListNodeId>,
        name: &str,
        document: [u8; 16],
        created: Created,
    ) -> Result<ListNodeId, ListError> {
        self.create(parent, name, created, Some(document))
    }

    fn create(
        &self,
        parent: Option<ListNodeId>,
        name: &str,
        created: Created,
        document: Option<[u8; 16]>,
    ) -> Result<ListNodeId, ListError> {
        check_list_name(name, document.is_none())?;
        let parent = self.folder_parent(parent)?;
        let tree = self.tree();
        let node = tree.create(parent).map_err(|_| ListError::NotFound)?;
        let meta = tree.get_meta(node).map_err(|_| ListError::NotFound)?;
        let kind = if document.is_some() {
            KIND_DOCUMENT
        } else {
            KIND_FOLDER
        };
        write(&meta, KEY_KIND, kind)?;
        write(&meta, KEY_NAME, name)?;
        if let Some(document) = document {
            write(&meta, KEY_DOCUMENT, document.to_vec())?;
        }
        let stamp = LoroValue::List(vec![created.at.into(), created.id.to_vec().into()].into());
        write(&meta, KEY_CREATED, stamp)?;
        self.doc.commit();
        Ok(ListNodeId::of(node))
    }

    /// The tree parent of a create or move into `folder`: a visible folder, or the top.
    fn folder_parent(&self, folder: Option<ListNodeId>) -> Result<TreeParentId, ListError> {
        let Some(folder) = folder else {
            return Ok(TreeParentId::Root);
        };
        match self.visible_kind(folder) {
            Some(EntryKind::Folder) => Ok(TreeParentId::Node(folder.tree_id())),
            Some(EntryKind::Document) => Err(ListError::NotAFolder),
            None => Err(ListError::NotFound),
        }
    }

    /// Renames `node` (the register; the shown name is derived).
    ///
    /// # Errors
    /// [`ListError::Name`], [`ListError::NotFound`].
    pub fn rename(&self, node: ListNodeId, name: &str) -> Result<(), ListError> {
        check_list_name(name, self.visible_kind(node) == Some(EntryKind::Folder))?;
        let meta = self.visible_meta(node)?;
        write(&meta, KEY_NAME, name)?;
        self.doc.commit();
        Ok(())
    }

    /// Moves `node` into `folder` (the top when `None`).
    ///
    /// # Errors
    /// [`ListError::Loop`], [`ListError::NotFound`], [`ListError::NotAFolder`].
    pub fn move_to(&self, node: ListNodeId, folder: Option<ListNodeId>) -> Result<(), ListError> {
        self.visible_meta(node)?;
        let target = self.folder_parent(folder)?;
        if let TreeParentId::Node(parent) = target
            && self.is_at_or_below(parent, node.tree_id())
        {
            return Err(ListError::Loop);
        }
        self.tree()
            .mov(node.tree_id(), target)
            .map_err(|_| ListError::Loop)?;
        self.doc.commit();
        self.moves.borrow_mut().insert(node.tree_id(), target);
        Ok(())
    }

    /// Places `node` at `path` (shown names, its own last), making the folders on the way that
    /// the list lacks with the stamps `stamp` draws: a document another program moved
    /// (criterion 41).
    ///
    /// # Errors
    /// [`ListError::Name`], [`ListError::NotFound`], [`ListError::Loop`].
    pub fn place(
        &self,
        node: ListNodeId,
        path: &[String],
        mut stamp: impl FnMut() -> Created,
    ) -> Result<(), ListError> {
        let (name, folders) = path.split_last().ok_or(ListError::NotFound)?;
        let mut parent = None;
        for folder in folders {
            let found = self.entries().into_iter().find(|entry| {
                entry.parent == parent
                    && entry.kind == EntryKind::Folder
                    && entry
                        .path
                        .last()
                        .is_some_and(|last| crate::same_entry_name(last, folder))
            });
            parent = Some(match found {
                Some(entry) => entry.node,
                None => self.create_folder(parent, folder, stamp())?,
            });
        }
        let entry = self
            .entries()
            .into_iter()
            .find(|entry| entry.node == node)
            .ok_or(ListError::NotFound)?;
        if entry.parent != parent {
            self.move_to(node, parent)?;
        }
        if entry.path.last() != Some(name) {
            self.rename(node, name)?;
        }
        Ok(())
    }

    /// Marks `node` deleted by the member `by` at `at` (unix seconds): it and everything below
    /// it are hidden; its state stays (criterion 40).
    ///
    /// # Errors
    /// [`ListError::NotFound`].
    pub fn delete(&self, node: ListNodeId, by: &str, at: i64) -> Result<(), ListError> {
        let meta = self.visible_meta(node)?;
        let mut mark = std::collections::HashMap::new();
        mark.insert(KEY_BY.to_owned(), LoroValue::from(by));
        mark.insert(KEY_AT.to_owned(), LoroValue::from(at));
        write(&meta, KEY_DELETED, LoroValue::from(mark))?;
        self.doc.commit();
        Ok(())
    }

    /// Clears the deletion of `node`; under a folder that is still deleted it moves to the top.
    ///
    /// # Errors
    /// [`ListError::NotFound`] when `node` is not a deleted entry.
    pub fn restore(&self, node: ListNodeId) -> Result<(), ListError> {
        let tree = self.tree();
        let id = node.tree_id();
        let meta = tree.get_meta(id).map_err(|_| ListError::NotFound)?;
        if meta.get(KEY_DELETED).is_none() {
            return Err(ListError::NotFound);
        }
        meta.delete(KEY_DELETED).map_err(|_| ListError::NotFound)?;
        if self.hidden_by_ancestor(id) {
            tree.mov(id, TreeParentId::Root)
                .map_err(|_| ListError::NotFound)?;
        }
        self.doc.commit();
        Ok(())
    }

    /// Whether a folder above `node` is deleted.
    fn hidden_by_ancestor(&self, node: TreeID) -> bool {
        let tree = self.tree();
        let mut current = tree.parent(node);
        while let Some(TreeParentId::Node(parent)) = current {
            if tree
                .get_meta(parent)
                .is_ok_and(|meta| meta.get(KEY_DELETED).is_some())
            {
                return true;
            }
            current = tree.parent(parent);
        }
        false
    }

    /// The meta of a node that is shown.
    fn visible_meta(&self, node: ListNodeId) -> Result<LoroMap, ListError> {
        if self.visible_kind(node).is_none() {
            return Err(ListError::NotFound);
        }
        self.tree()
            .get_meta(node.tree_id())
            .map_err(|_| ListError::NotFound)
    }
}

fn write(meta: &LoroMap, key: &str, value: impl Into<LoroValue>) -> Result<(), ListError> {
    meta.insert(key, value).map_err(|_| ListError::Encode)
}

/// [`check_entry_name`], and for a folder no name ending in `.curvyo`: on disk it would be the
/// same entry as the file of a document beside it with the name before the extension.
fn check_list_name(name: &str, folder: bool) -> Result<(), NameRefusal> {
    check_entry_name(name)?;
    let suffix = format!(".{DOCUMENT_EXTENSION}");
    if folder && name.to_lowercase().ends_with(&suffix) {
        return Err(NameRefusal::NotAllowed);
    }
    Ok(())
}
