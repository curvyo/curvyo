//! Plans the file operations of a local workspace on its scanned folder, without touching the
//! disk (`specs/0094-workspace` criteria 13, 14, 16 to 19; ADR 0036 decision 6).
//!
//! The host scans the folder, this module answers with one [`FileOp`] or a [`PlanRefusal`], the
//! host executes the operation. Paths are relative to the workspace folder, one component per
//! entry, and name files on disk: a document's last component ends in `.curvyo`.

use std::collections::BTreeMap;

use crate::workspace_names::{
    NameRefusal, check_entry_name, copy_entry_name, free_entry_name, same_entry_name,
};

/// The file extension of a workspace document, without the dot.
pub const DOCUMENT_EXTENSION: &str = "curvyo";

/// A path inside the workspace folder, one component per entry; empty for the folder itself.
pub type WorkspacePath = Vec<String>;

/// What a scanned entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// A folder.
    Folder,
    /// A `.curvyo` file.
    Document,
}

/// One file operation the host executes. None of them replaces an existing entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOp {
    /// Write a new document file; the caller supplies its bytes.
    CreateDocument {
        /// Where the file goes.
        path: WorkspacePath,
    },
    /// Create an empty folder.
    CreateFolder {
        /// Where the folder goes.
        path: WorkspacePath,
    },
    /// Rename or move a document or folder.
    Move {
        /// The entry's path now.
        from: WorkspacePath,
        /// The entry's path afterwards.
        to: WorkspacePath,
    },
    /// Move a document or folder to the OS trash.
    Trash {
        /// The entry to trash.
        path: WorkspacePath,
    },
}

impl FileOp {
    /// The operation that undoes this one: only a move has one (Undo move, criterion 16).
    #[must_use]
    pub fn inverse(&self) -> Option<FileOp> {
        match self {
            FileOp::Move { from, to } => Some(FileOp::Move {
                from: to.clone(),
                to: from.clone(),
            }),
            _ => None,
        }
    }
}

/// A planned creation and the name it ends with, which the maker is told when it differs from
/// the name asked for (criterion 13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    /// The operation.
    pub op: FileOp,
    /// The final name, without extension.
    pub name: String,
}

/// Why an operation is not planned; nothing is changed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanRefusal {
    /// The new name breaks a naming rule.
    #[error("{0}")]
    Name(#[from] NameRefusal),
    /// Another entry of the target folder has that name.
    #[error("an entry with this name exists")]
    Exists,
    /// A folder would move into itself or below itself.
    #[error("a folder cannot move into itself")]
    IntoItself,
    /// The entry is already in the target folder.
    #[error("the entry is already there")]
    AlreadyThere,
    /// The path names no scanned entry.
    #[error("no such entry")]
    NotFound,
    /// The target of a creation or a move is not a folder.
    #[error("not a folder")]
    NotAFolder,
}

/// The scanned entries of one workspace folder.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceTree {
    entries: BTreeMap<WorkspacePath, EntryKind>,
}

impl WorkspaceTree {
    /// The tree of the scanned `entries`; a folder implied by a deeper entry counts as scanned.
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (WorkspacePath, EntryKind)>) -> Self {
        let mut tree = BTreeMap::new();
        for (path, kind) in entries {
            for depth in 1..path.len() {
                tree.insert(path[..depth].to_vec(), EntryKind::Folder);
            }
            if !path.is_empty() {
                tree.insert(path, kind);
            }
        }
        Self { entries: tree }
    }

    /// What `path` is; the empty path is the workspace folder.
    #[must_use]
    pub fn kind(&self, path: &[String]) -> Option<EntryKind> {
        if path.is_empty() {
            return Some(EntryKind::Folder);
        }
        self.entries.get(path).copied()
    }

    /// The documents in `folder` and below it (the delete confirm of a folder, criterion 17).
    #[must_use]
    pub fn document_count(&self, folder: &[String]) -> usize {
        self.entries
            .iter()
            .filter(|(path, kind)| {
                **kind == EntryKind::Document
                    && path.len() > folder.len()
                    && path.starts_with(folder)
            })
            .count()
    }

    /// Plans a new document named `name` (without extension) in `folder` (criterion 13).
    ///
    /// # Errors
    /// [`PlanRefusal::Name`] for a refused name, [`PlanRefusal::NotAFolder`] for a target that is
    /// not a folder.
    pub fn plan_new_document(&self, folder: &[String], name: &str) -> Result<Planned, PlanRefusal> {
        self.plan_creation(folder, name, EntryKind::Document)
    }

    /// Plans a new empty folder named `name` in `folder` (criterion 18).
    ///
    /// # Errors
    /// As [`WorkspaceTree::plan_new_document`].
    pub fn plan_new_folder(&self, folder: &[String], name: &str) -> Result<Planned, PlanRefusal> {
        self.plan_creation(folder, name, EntryKind::Folder)
    }

    /// Plans a copy of the document at `path` beside it, "`name` copy" (criterion 19). The caller
    /// writes the original's bytes without its share record.
    ///
    /// # Errors
    /// [`PlanRefusal::NotFound`] when `path` is not a document.
    pub fn plan_duplicate(&self, path: &[String]) -> Result<Planned, PlanRefusal> {
        if self.kind(path) != Some(EntryKind::Document) {
            return Err(PlanRefusal::NotFound);
        }
        let (folder, name) = split(path, EntryKind::Document);
        let name = free_entry_name(
            &copy_entry_name(&name),
            self.sibling_names(folder, EntryKind::Document),
        );
        Ok(Planned {
            op: FileOp::CreateDocument {
                path: child(folder, &name, EntryKind::Document),
            },
            name,
        })
    }

    /// Plans renaming the entry at `path` to `name` (without extension) in its folder
    /// (criterion 14). A change of case alone is allowed.
    ///
    /// # Errors
    /// [`PlanRefusal::NotFound`], [`PlanRefusal::Name`], or [`PlanRefusal::Exists`] when a
    /// sibling of the same kind has that name.
    pub fn plan_rename(&self, path: &[String], name: &str) -> Result<FileOp, PlanRefusal> {
        let kind = self.existing(path)?;
        check_entry_name(name)?;
        let (folder, old) = split(path, kind);
        let taken = self
            .sibling_names(folder, kind)
            .any(|sibling| sibling != old && same_entry_name(sibling, name));
        if taken {
            return Err(PlanRefusal::Exists);
        }
        Ok(FileOp::Move {
            from: path.to_vec(),
            to: child(folder, name, kind),
        })
    }

    /// Plans moving the entry at `path` into `folder`, keeping its name (criterion 16).
    ///
    /// # Errors
    /// [`PlanRefusal::NotFound`], [`PlanRefusal::NotAFolder`], [`PlanRefusal::IntoItself`] for a
    /// folder moved into itself or below itself, [`PlanRefusal::AlreadyThere`], or
    /// [`PlanRefusal::Exists`] when the target holds that name.
    pub fn plan_move(&self, path: &[String], folder: &[String]) -> Result<FileOp, PlanRefusal> {
        let kind = self.existing(path)?;
        if self.kind(folder) != Some(EntryKind::Folder) {
            return Err(PlanRefusal::NotAFolder);
        }
        if folder.starts_with(path) {
            return Err(PlanRefusal::IntoItself);
        }
        let (parent, name) = split(path, kind);
        if parent == folder {
            return Err(PlanRefusal::AlreadyThere);
        }
        if self
            .sibling_names(folder, kind)
            .any(|s| same_entry_name(s, &name))
        {
            return Err(PlanRefusal::Exists);
        }
        Ok(FileOp::Move {
            from: path.to_vec(),
            to: child(folder, &name, kind),
        })
    }

    /// Plans moving the entry at `path` to the OS trash (criterion 17).
    ///
    /// # Errors
    /// [`PlanRefusal::NotFound`].
    pub fn plan_delete(&self, path: &[String]) -> Result<FileOp, PlanRefusal> {
        self.existing(path)?;
        Ok(FileOp::Trash {
            path: path.to_vec(),
        })
    }

    fn plan_creation(
        &self,
        folder: &[String],
        name: &str,
        kind: EntryKind,
    ) -> Result<Planned, PlanRefusal> {
        check_entry_name(name)?;
        if self.kind(folder) != Some(EntryKind::Folder) {
            return Err(PlanRefusal::NotAFolder);
        }
        let name = free_entry_name(name, self.sibling_names(folder, kind));
        let path = child(folder, &name, kind);
        let op = match kind {
            EntryKind::Document => FileOp::CreateDocument { path },
            EntryKind::Folder => FileOp::CreateFolder { path },
        };
        Ok(Planned { op, name })
    }

    fn existing(&self, path: &[String]) -> Result<EntryKind, PlanRefusal> {
        match self.entries.get(path) {
            Some(kind) => Ok(*kind),
            None => Err(PlanRefusal::NotFound),
        }
    }

    /// The names (documents without extension) of the entries of `kind` directly in `folder`.
    fn sibling_names<'a>(
        &'a self,
        folder: &'a [String],
        kind: EntryKind,
    ) -> impl Iterator<Item = &'a str> + 'a {
        self.entries
            .iter()
            .filter(move |(path, k)| {
                **k == kind && path.len() == folder.len() + 1 && path.starts_with(folder)
            })
            .filter_map(move |(path, _)| {
                let last = path.last()?;
                Some(match kind {
                    EntryKind::Folder => last.as_str(),
                    EntryKind::Document => document_name(last),
                })
            })
    }
}

/// The name of a document file without its `.curvyo` extension (any case).
#[must_use]
pub fn document_name(file_name: &str) -> &str {
    let suffix_len = DOCUMENT_EXTENSION.len() + 1;
    match file_name.len().checked_sub(suffix_len) {
        Some(at)
            if file_name.is_char_boundary(at)
                && file_name[at..].eq_ignore_ascii_case(".curvyo") =>
        {
            &file_name[..at]
        }
        _ => file_name,
    }
}

fn split(path: &[String], kind: EntryKind) -> (&[String], String) {
    match path.split_last() {
        Some((last, folder)) => {
            let name = match kind {
                EntryKind::Folder => last.as_str(),
                EntryKind::Document => document_name(last),
            };
            (folder, name.to_string())
        }
        None => (path, String::new()),
    }
}

fn child(folder: &[String], name: &str, kind: EntryKind) -> WorkspacePath {
    let mut path = folder.to_vec();
    path.push(match kind {
        EntryKind::Folder => name.to_string(),
        EntryKind::Document => format!("{name}.{DOCUMENT_EXTENSION}"),
    });
    path
}

#[cfg(test)]
#[path = "workspace_plan_tests.rs"]
mod tests;
