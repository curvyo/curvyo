//! Every command that removes objects repairs the linked clones that follow them
//! (`specs/0049-linked-clone` criterion 31, `adrs.md` decision 5): in every source file, each
//! removal of leaves (marked by its call of `remove_emptied_groups`) comes with one call of
//! `repair_clones_of_removed`. A command added later that removes objects without the repair
//! fails this test.

#![allow(clippy::unwrap_used)]

use std::path::Path;

/// Files whose removals need no repair, and why.
const EXEMPT: &[(&str, &str)] = &[
    (
        "groups.rs",
        "Group and Ungroup remove group nodes whose children were moved out: no leaf goes",
    ),
    (
        "node_move.rs",
        "a move of nodes into a container removes the groups it emptied, never a leaf",
    ),
    (
        "clones.rs",
        "the orphan repair removes orphans only, and every clone behind an orphan is one",
    ),
];

fn count(text: &str, call: &str) -> usize {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .filter(|line| !line.contains("fn ") && line.contains(call))
        .count()
}

fn visit(dir: &Path, found: &mut Vec<(String, usize, usize)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            visit(&path, found);
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if path.extension().is_none_or(|e| e != "rs")
            || EXEMPT.iter().any(|(file, _)| *file == name)
        {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        // Only production code: stop at the unit-test module.
        let production = text.split("#[cfg(test)]").next().unwrap();
        let removals = count(production, "remove_emptied_groups(");
        let repairs = count(production, "repair_clones_of_removed(");
        if removals + repairs > 0 {
            found.push((name, removals, repairs));
        }
    }
}

#[test]
fn every_removal_of_objects_repairs_their_clones() {
    let mut found = Vec::new();
    visit(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut found,
    );
    found.sort();
    let wrong: Vec<_> = found
        .iter()
        .filter(|(name, removals, repairs)| repairs < removals && name != "clone_repair.rs")
        .collect();
    assert!(
        wrong.is_empty(),
        "removals without a clone repair: {wrong:?}"
    );
    let files: Vec<&str> = found.iter().map(|(name, ..)| name.as_str()).collect();
    for expected in [
        "objects.rs",
        "replace.rs",
        "path_topology.rs",
        "path_extend.rs",
        "paths.rs",
    ] {
        assert!(
            files.contains(&expected),
            "{expected} lost its removal: update this list"
        );
    }
}
