//! Shared by the session tests of earlier specs: reaching a workbench by its page.
#![allow(dead_code, clippy::unwrap_used)]

use curvyo_document_core::{Document, NodeId, PageKind};
use curvyo_editor_wasm::{Session, Workbench};

/// Reaching a workbench by its page (`specs/0098-page-kinds` criterion 8), for tests written when
/// a switcher chose the workbench: a sketch page of the active page's size laid over it, so every
/// coordinate stays as written, the first drawing page again, or a laser page.
pub trait PageKinds {
    /// Activates a sketch page lying over the active page (adding it once).
    fn to_sketch_page(&mut self);
    /// Activates the first drawing page.
    fn to_drawing_page(&mut self);
    /// Activates the laser page of the manufacturing `id`.
    fn to_output_page(&mut self, id: NodeId);
}

fn rows(session: &Session) -> Vec<serde_json::Value> {
    serde_json::from_str(&session.pages_json()).unwrap()
}

impl PageKinds for Session {
    fn to_sketch_page(&mut self) {
        if self.workbench() == Workbench::Sketch {
            return;
        }
        let page = self.active_page();
        let over = rows(self).into_iter().find(|row| {
            row["kind"] == "sketch"
                && (row["x"].as_f64().unwrap() - page.origin.x).abs() < 1e-9
                && (row["y"].as_f64().unwrap() - page.origin.y).abs() < 1e-9
        });
        let id = if let Some(row) = over {
            row["id"].as_str().unwrap().to_owned()
        } else {
            let fresh = self.undo_len() == 0;
            let id = self.add_page_of_kind("sketch", None).unwrap();
            if fresh {
                // A drop pushes the page it lands on (`specs/0109` criterion 22): the sketch page
                // is laid over the active page in the file, as an older file holds it, and the
                // session reopened from it keeps no steps, so the step counts stay as written.
                let bytes = lay_page_at(&self.pack("test").unwrap(), &id, page.origin);
                let peer = id.split('-').next().unwrap().parse().unwrap();
                *self = Session::open(peer, &bytes).unwrap();
            } else {
                self.move_page_to(&id, page.origin);
            }
            id
        };
        self.activate_page(&id);
    }

    fn to_drawing_page(&mut self) {
        if let Some(row) = rows(self).into_iter().find(|row| row["kind"] == "drawing") {
            let id = row["id"].as_str().unwrap().to_owned();
            self.activate_page(&id);
        }
    }

    fn to_output_page(&mut self, id: NodeId) {
        if let Some(row) = rows(self)
            .into_iter()
            .find(|row| row["manufacturing"] == id.full_id())
        {
            let page = row["id"].as_str().unwrap().to_owned();
            self.activate_page(&page);
        }
    }
}

/// A document whose only page is a sketch page named "Page 1" at (0, 0), where the first page of
/// a new document lies (`specs/0098` criterion 3: only a sketch page owns a sketch).
pub fn sketch_document(peer: u64) -> Document {
    let d = Document::new(peer);
    let page = d.add_page_of(&PageKind::Sketch).unwrap();
    d.delete_page(
        curvyo_document_core::FIRST_PAGE_ID,
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    // Deleting page 1 closes the hole (`specs/0109` criterion 38): the sketch page now lies at
    // (0, 0).
    assert_eq!(
        d.pages()[0].origin,
        curvyo_document_core::Point::new(0.0, 0.0)
    );
    d.rename_page(&page.id, "Page 1").unwrap();
    d
}

/// The file `bytes` with page `id`'s corner at `origin`, its size kept.
pub fn lay_page_at(bytes: &[u8], id: &str, origin: curvyo_document_core::Point) -> Vec<u8> {
    let document = curvyo_document_core::unpack(1, bytes).unwrap();
    let page = document.pages().into_iter().find(|p| p.id == id).unwrap();
    let loro = loro::LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    let frame: std::collections::HashMap<String, loro::LoroValue> = [
        ("x", origin.x),
        ("y", origin.y),
        ("width", page.size.width.as_mm()),
        ("height", page.size.height.as_mm()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), loro::LoroValue::Double(v)))
    .collect();
    loro.get_map("root")
        .insert(&format!("page.{id}.frame"), loro::LoroValue::from(frame))
        .unwrap();
    loro.commit();
    container(&loro)
}

/// `document` saved with a placement of `page` at identity under its first manufacturing, as a
/// file of an older build holds it (`specs/0098` criterion 12: placements come only from older
/// files).
pub fn with_placement(document: &Document, page: &str) -> Vec<u8> {
    let loro = loro::LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    let tree = loro.get_tree("paths");
    let node = tree
        .children(loro::TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_some())
        .unwrap();
    let child = tree.create(loro::TreeParentId::Node(node)).unwrap();
    let meta = tree.get_meta(child).unwrap();
    meta.insert("shape", "placement").unwrap();
    let identity: Vec<loro::LoroValue> = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        .into_iter()
        .map(loro::LoroValue::Double)
        .collect();
    let link: std::collections::HashMap<String, loro::LoroValue> = [
        ("page".to_owned(), loro::LoroValue::from(page)),
        ("matrix".to_owned(), loro::LoroValue::from(identity)),
    ]
    .into();
    meta.insert("placement_link", loro::LoroValue::from(link))
        .unwrap();
    loro.commit();
    container(&loro)
}

/// The `.curvyo` file of the Loro document `loro` at the current format.
fn container(loro: &loro::LoroDoc) -> Vec<u8> {
    use std::io::Write;
    let manifest = serde_json::json!({
        "format_version": curvyo_document_core::CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": curvyo_document_core::CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "0.1.0",
    });
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("manifest.json", options).unwrap();
    writer.write_all(manifest.to_string().as_bytes()).unwrap();
    writer.start_file("document.loro", options).unwrap();
    writer
        .write_all(&loro.export(loro::ExportMode::Snapshot).unwrap())
        .unwrap();
    writer.start_file("document.json", options).unwrap();
    writer.write_all(b"{}").unwrap();
    writer.finish().unwrap().into_inner()
}
