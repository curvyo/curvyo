//! Test support: reaching a workbench by its page, for the session tests of earlier specs.
#![allow(clippy::unwrap_used, dead_code)]

use curvyo_document_core::NodeId;

use super::{Session, Workbench};

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
            self.move_page_to(&id, page.origin);
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
