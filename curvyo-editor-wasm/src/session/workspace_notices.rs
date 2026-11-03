//! The notices of other members' changes to a shared workspace's list (`specs/0094-workspace`
//! criteria 36 to 40, `ux-notes.md` section 7), from one comparison of the list before and after
//! an import.
//!
//! A rename or move names the member whose list record carried it, by the list peer that
//! member's presence announced; "Someone" when that member was not seen online in this session
//! **(Proposal)**. Creations show no notice.

use std::collections::{BTreeMap, HashMap, HashSet};

use curvyo_document_core::{DeletedEntry, ListEntry, ListNodeId};
use curvyo_ui_core::workspace_tree_text::{
    LOOP_NOTICE, deleted_notice, moved_notice, renamed_notice,
};

/// The names a notice uses: the label of the member whose record was imported (empty when
/// unknown), this device's label (its own deletions arrive back on a resync and show nothing)
/// and the workspace's name for the top.
pub(super) struct NoticeNames<'a> {
    pub(super) author: &'a str,
    pub(super) me: &'a str,
    pub(super) workspace: &'a str,
}

/// What changed between two reads of the list, as notices.
pub(super) fn list_notices(
    before: &[ListEntry],
    after: &[ListEntry],
    deleted_before: &HashSet<ListNodeId>,
    deleted_after: &[DeletedEntry],
    loop_dropped: bool,
    names: &NoticeNames<'_>,
) -> Vec<String> {
    let NoticeNames {
        author,
        me,
        workspace,
    } = *names;
    let mut notices = Vec::new();
    let earlier: HashMap<ListNodeId, &ListEntry> = before.iter().map(|e| (e.node, e)).collect();
    let folder_name = |entry: &ListEntry| -> String {
        let parent = &entry.path[..entry.path.len().saturating_sub(1)];
        parent
            .last()
            .map_or_else(|| workspace.to_owned(), Clone::clone)
    };
    for entry in after {
        let Some(old) = earlier.get(&entry.node) else {
            continue;
        };
        let (Some(old_name), Some(new_name)) = (old.path.last(), entry.path.last()) else {
            continue;
        };
        if old.parent != entry.parent {
            notices.push(moved_notice(author, new_name, &folder_name(entry)));
        } else if old_name != new_name && old.path.len() == entry.path.len() {
            // Only the entry's own name changed, not a folder above it.
            if old.path[..old.path.len() - 1] == entry.path[..entry.path.len() - 1] {
                notices.push(renamed_notice(author, old_name, new_name));
            }
        }
    }
    let mut deleted: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for entry in deleted_after {
        if deleted_before.contains(&entry.node) || entry.by == me {
            continue;
        }
        deleted
            .entry(&entry.by)
            .or_default()
            .push(entry.name.clone());
    }
    for (by, names) in deleted {
        notices.push(deleted_notice(by, &names));
    }
    if loop_dropped {
        notices.push(LOOP_NOTICE.to_owned());
    }
    notices
}
