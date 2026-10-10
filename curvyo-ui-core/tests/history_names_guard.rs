//! The guard of `specs/0020-undo-redo` criterion 41: every commit label in the source has an entry
//! in the table of operation names, so a new command cannot ship without a name.
//!
//! The test reads the source of the workspace's crates and collects the labels from the places a
//! label is written: `commit_with_label("..")`, `batch("..")`, a `*_COMMIT_LABEL` constant, the
//! label argument of `apply_size`, and the `boolean_*` labels of the Boolean operations. Test code
//! (everything from the first `#[cfg(test)]` of a file on) is left out.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use curvyo_ui_core::{RESERVED_LABELS, known_operation_name};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The string literals that follow `marker`, up to the closing parenthesis or semicolon.
fn literals_after(source: &str, marker: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (start, _) in source.match_indices(marker) {
        let rest = &source[start + marker.len()..];
        let end = rest.find([';', ')']).map_or(rest.len(), |at| at + 1);
        let mut parts = rest[..end].split('"');
        parts.next();
        while let (Some(text), Some(_)) = (parts.next(), parts.next()) {
            found.push(text.to_owned());
        }
    }
    found
}

/// Every `"boolean_xxx"` literal.
fn boolean_labels(source: &str) -> Vec<String> {
    source
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|text| {
            text.starts_with("boolean_") && text.chars().all(|c| c.is_ascii_lowercase() || c == '_')
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_commit_label_in_the_source_has_an_operation_name() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for krate in [
        "curvyo-document-core",
        "curvyo-ui-core",
        "curvyo-editor-wasm",
        "curvyo-app",
    ] {
        rust_files(&root.join(krate).join("src"), &mut files);
    }
    assert!(files.len() > 50, "the sources were found");
    let mut labels = BTreeSet::new();
    for file in files {
        let full = fs::read_to_string(&file).unwrap();
        let source = full.split("#[cfg(test)]").next().unwrap_or_default();
        for marker in [
            "commit_with_label(",
            ".batch(",
            "_COMMIT_LABEL: &str =",
            "apply_size(",
        ] {
            labels.extend(literals_after(source, marker));
        }
        labels.extend(boolean_labels(source));
    }
    assert!(labels.len() > 30, "the labels were found: {labels:?}");
    let unnamed: Vec<_> = labels
        .iter()
        .filter(|label| {
            known_operation_name(label).is_none() && !RESERVED_LABELS.contains(&label.as_str())
        })
        .collect();
    assert!(
        unnamed.is_empty(),
        "commit labels without an entry in curvyo-ui-core/src/history_names.rs: {unnamed:?}"
    );
}
