//! The rows of the File tab's workspace tree (`specs/0095-workspace` criteria 5, 6, 11;
//! `ux-notes.md` section 3): the scanned entries flattened depth first by the expanded folders,
//! folders before documents, each level in natural case-insensitive order.
//!
//! The flattening is O(n · depth) and runs once per scan or expand; the page windows the result.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

use curvyo_document_core::{EntryKind, document_name};

/// One scanned entry: its path inside the workspace folder (a document's last component ends in
/// `.curvyo`), its kind and its last change in seconds since the Unix epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEntry {
    /// The path, one component per entry.
    pub path: Vec<String>,
    /// Folder or document.
    pub kind: EntryKind,
    /// The last change, seconds since the Unix epoch.
    pub modified: u64,
}

/// One row of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRow {
    /// The entry's path.
    pub path: Vec<String>,
    /// Folder or document.
    pub kind: EntryKind,
    /// The name shown: a folder's name, a document's name without extension.
    pub name: String,
    /// `aria-level`, 1 at the top.
    pub level: usize,
    /// `aria-posinset`, 1-based among its siblings.
    pub posinset: usize,
    /// `aria-setsize`, the number of its siblings including itself.
    pub setsize: usize,
    /// A folder's children are shown.
    pub expanded: bool,
    /// A folder's documents, at any depth below it; 0 for a document.
    pub count: usize,
    /// The last change, seconds since the Unix epoch.
    pub modified: u64,
}

/// `a` against `b` in natural order, ignoring case: runs of digits compare by value, so "Lid 2"
/// sorts before "Lid 10".
#[must_use]
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let a: Vec<char> = a.to_lowercase().chars().collect();
    let b: Vec<char> = b.to_lowercase().chars().collect();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let start_a = i;
            let start_b = j;
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let run_a: String = a[start_a..i].iter().collect();
            let run_b: String = b[start_b..j].iter().collect();
            let trimmed_a = run_a.trim_start_matches('0');
            let trimmed_b = run_b.trim_start_matches('0');
            let by_value = trimmed_a
                .len()
                .cmp(&trimmed_b.len())
                .then_with(|| trimmed_a.cmp(trimmed_b));
            if by_value != Ordering::Equal {
                return by_value;
            }
        } else {
            match a[i].cmp(&b[j]) {
                Ordering::Equal => {
                    i += 1;
                    j += 1;
                }
                other => return other,
            }
        }
    }
    (a.len() - i).cmp(&(b.len() - j))
}

/// The name a row shows for `entry`.
fn shown_name(entry: &WorkspaceEntry) -> &str {
    let last = entry.path.last().map_or("", String::as_str);
    match entry.kind {
        EntryKind::Folder => last,
        EntryKind::Document => document_name(last),
    }
}

/// The rows of `entries` with the folders in `expanded` open: folders first, then documents,
/// each in natural case-insensitive order, a folder's children right below it.
#[must_use]
pub fn workspace_rows<S: BuildHasher>(
    entries: &[WorkspaceEntry],
    expanded: &HashSet<Vec<String>, S>,
) -> Vec<WorkspaceRow> {
    let mut children: HashMap<&[String], Vec<usize>> = HashMap::new();
    let mut counts: HashMap<&[String], usize> = HashMap::new();
    for (index, entry) in entries.iter().enumerate() {
        if entry.path.is_empty() {
            continue;
        }
        let parent = &entry.path[..entry.path.len() - 1];
        children.entry(parent).or_default().push(index);
        if entry.kind == EntryKind::Document {
            for depth in 1..entry.path.len() {
                *counts.entry(&entry.path[..depth]).or_default() += 1;
            }
        }
    }
    for list in children.values_mut() {
        list.sort_by(|&x, &y| {
            let (a, b) = (&entries[x], &entries[y]);
            let folder_first =
                (a.kind == EntryKind::Document).cmp(&(b.kind == EntryKind::Document));
            folder_first
                .then_with(|| natural_cmp(shown_name(a), shown_name(b)))
                .then_with(|| a.path.cmp(&b.path))
        });
    }
    let mut rows = Vec::with_capacity(entries.len());
    push_children(&[], 1, entries, &children, &counts, expanded, &mut rows);
    rows
}

fn push_children<S: BuildHasher>(
    folder: &[String],
    level: usize,
    entries: &[WorkspaceEntry],
    children: &HashMap<&[String], Vec<usize>>,
    counts: &HashMap<&[String], usize>,
    expanded: &HashSet<Vec<String>, S>,
    rows: &mut Vec<WorkspaceRow>,
) {
    let Some(list) = children.get(folder) else {
        return;
    };
    for (position, &index) in list.iter().enumerate() {
        let entry = &entries[index];
        let is_open = entry.kind == EntryKind::Folder && expanded.contains(&entry.path);
        rows.push(WorkspaceRow {
            path: entry.path.clone(),
            kind: entry.kind,
            name: shown_name(entry).to_owned(),
            level,
            posinset: position + 1,
            setsize: list.len(),
            expanded: is_open,
            count: counts.get(entry.path.as_slice()).copied().unwrap_or(0),
            modified: entry.modified,
        });
        if is_open {
            push_children(
                &entry.path,
                level + 1,
                entries,
                children,
                counts,
                expanded,
                rows,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, kind: EntryKind) -> WorkspaceEntry {
        WorkspaceEntry {
            path: path.split('/').map(str::to_owned).collect(),
            kind,
            modified: 0,
        }
    }

    fn names(rows: &[WorkspaceRow]) -> Vec<&str> {
        rows.iter().map(|row| row.name.as_str()).collect()
    }

    #[test]
    fn natural_order_compares_digit_runs_by_value_and_ignores_case() {
        assert_eq!(natural_cmp("Lid 2", "Lid 10"), Ordering::Less);
        assert_eq!(natural_cmp("lid", "Lid"), Ordering::Equal);
        assert_eq!(natural_cmp("a", "B"), Ordering::Less);
        assert_eq!(natural_cmp("Part 007", "Part 7b"), Ordering::Less);
        assert_eq!(natural_cmp("x", "x1"), Ordering::Less);
    }

    /// Criterion 5 and `ux-notes.md` section 3: folders first, natural order, collapsed by default.
    #[test]
    fn folders_come_first_and_collapsed_folders_hide_their_children() {
        let entries = vec![
            entry("Lid 10.curvyo", EntryKind::Document),
            entry("lid 2.curvyo", EntryKind::Document),
            entry("Sides", EntryKind::Folder),
            entry("Sides/Left.curvyo", EntryKind::Document),
            entry("Sides/Deep", EntryKind::Folder),
            entry("Sides/Deep/Right.curvyo", EntryKind::Document),
            entry("Base", EntryKind::Folder),
        ];
        let rows = workspace_rows(&entries, &HashSet::new());
        assert_eq!(names(&rows), ["Base", "Sides", "lid 2", "Lid 10"]);
        assert_eq!(rows[1].count, 2, "documents at any depth");
        assert_eq!(rows[0].count, 0);
        assert_eq!(
            (rows[2].posinset, rows[2].setsize, rows[2].level),
            (3, 4, 1)
        );

        let open = HashSet::from([vec!["Sides".to_owned()]]);
        let rows = workspace_rows(&entries, &open);
        assert_eq!(
            names(&rows),
            ["Base", "Sides", "Deep", "Left", "lid 2", "Lid 10"]
        );
        assert!(rows[1].expanded);
        assert_eq!(
            (rows[2].level, rows[2].posinset, rows[2].setsize),
            (2, 1, 2)
        );
        assert_eq!(rows[2].count, 1);
    }
}
