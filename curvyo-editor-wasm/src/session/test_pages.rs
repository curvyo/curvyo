//! Test support: reaching a workbench by its page, for the session tests of earlier specs.
#![allow(clippy::unwrap_used, dead_code)]

use curvyo_document_core::{Document, NodeId, Point};

use super::{Session, Workbench};

/// Lays page `id` with its corner at `origin` through a merged edit of another replica, as an
/// older file or a peer's concurrent move does: a drop on the canvas pushes the page it lands on
/// (`specs/0109` criterion 22), so a page laid over another for a test is not dropped there.
pub(crate) fn lay_page_at(document: &Document, id: &str, origin: Point) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static PEER: AtomicU64 = AtomicU64::new(0xF1A_0000);
    let page = document.pages().into_iter().find(|p| p.id == id).unwrap();
    let loro = loro::LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro.set_peer_id(PEER.fetch_add(1, Ordering::Relaxed))
        .unwrap();
    let known = loro.oplog_vv();
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
    let updates = loro.export(loro::ExportMode::updates(&known)).unwrap();
    let epoch = document.history_epoch().unwrap_or_default();
    let mut bytes = u32::try_from(epoch.len()).unwrap().to_le_bytes().to_vec();
    bytes.extend_from_slice(epoch.as_bytes());
    bytes.extend_from_slice(&updates);
    document.import_updates(&bytes).unwrap();
}

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
            lay_page_at(&self.document, &id, page.origin);
            // A session without steps of its own keeps none, so the step counts stay as written.
            if fresh {
                self.wipe_history();
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
