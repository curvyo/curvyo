//! A clone-member key of a file of format 39 to 43 (`member.<clone>/<member>`, Frage 167 B),
//! planted raw into a saved document: no command writes one since `specs/0111` criterion 14 (ADR
//! 0056 §2), so the tests of Frage 212 A make one the way an old file holds it.

#![allow(dead_code, clippy::unwrap_used)]

use std::io::{Cursor, Read, Write};

use curvyo_document_core::{Document, NodeId, pack, unpack};

/// The stored text of `id` (`<peer>:<counter>`, ADR 0046 §2).
fn id_text(id: NodeId) -> String {
    let value = serde_json::to_value(id).unwrap();
    format!("{}:{}", value["peer"].as_str().unwrap(), value["counter"])
}

/// `d` saved with the legacy key of `path` planted on its operation named `operation`, opened
/// again.
pub fn with_legacy_key(d: &Document, operation: &str, path: &[NodeId]) -> Document {
    let bytes = pack(d, "legacy key").unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let key = format!(
        "member.{}",
        path.iter()
            .map(|&id| id_text(id))
            .collect::<Vec<_>>()
            .join("/")
    );
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let mut content = Vec::new();
        file.read_to_end(&mut content).unwrap();
        let name = file.name().unwrap().to_string();
        if name == "document.loro" {
            let loro = loro::LoroDoc::new();
            loro.import(&content).unwrap();
            let tree = loro.get_tree("paths");
            let mut stack = tree.children(loro::TreeParentId::Root).unwrap();
            let mut planted = false;
            while let Some(node) = stack.pop() {
                let meta = tree.get_meta(node).unwrap();
                let is = |k: &str, v: &str| {
                    meta.get(k).and_then(|x| x.into_value().ok()) == Some(loro::LoroValue::from(v))
                };
                if !planted && is("shape", "operation") && is("name", operation) {
                    meta.insert(&key, true).unwrap();
                    planted = true;
                }
                stack.extend(
                    tree.children(loro::TreeParentId::Node(node))
                        .unwrap_or_default(),
                );
            }
            assert!(planted, "an operation named {operation} holds the key");
            loro.commit();
            content = loro.export(loro::ExportMode::Snapshot).unwrap();
        }
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&content).unwrap();
    }
    unpack(1, &writer.finish().unwrap().into_inner()).unwrap()
}
