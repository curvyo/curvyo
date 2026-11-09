//! The words of a selected placed page (`specs/0100-bring-content-in` criteria 17 to 19): its
//! Style block and the refusals of a move off its laser page and of Group. Placed pages are
//! objects of the one selection; the object writers move, turn, scale and copy them.

use super::super::Session;

/// The notice of a refused move off the laser page (criterion 18).
pub const OFF_PAGE_NOTICE: &str = "A placed page stays on its laser page.";

/// The notice of Group with a placed page selected (criterion 19).
pub const NOT_GROUPED_NOTICE: &str = "A placed page cannot be grouped.";

impl Session {
    /// The Style tab block of the selected placed page (criterion 17): "Placed page of <name>."
    /// with the source page's name cut at 40 characters; empty when none is selected.
    #[must_use]
    pub fn placed_page_block(&self) -> String {
        let Some(id) = self.selected_placement() else {
            return String::new();
        };
        let Some(source) = self.document.placed_page(id).and_then(|p| p.source) else {
            return String::new();
        };
        let name: String = if source.name.chars().count() > 40 {
            let cut: String = source.name.chars().take(39).collect();
            format!("{cut}…")
        } else {
            source.name
        };
        format!("Placed page of {name}.")
    }
}

impl Session {
    /// The name tag of every placed page (UX notes §5), JSON `[{id, name, x, y, width,
    /// selected}]`: the canvas position of the top-left of its frame's screen bounds and their
    /// width in CSS px; the host draws the label above it, upright.
    #[must_use]
    pub fn placed_pages_json(&self) -> String {
        let view = self.view();
        let selected: Vec<curvyo_document_core::NodeId> = self.selection.ids().to_vec();
        let rows: Vec<serde_json::Value> = self
            .document
            .placed_pages()
            .iter()
            .filter_map(|placed| {
                let source = placed.source.as_ref()?;
                let corners = self
                    .placed_frame(placed.id)?
                    .map(|p| view.document_to_screen(p));
                let left = corners.iter().map(|c| c.0).fold(f64::INFINITY, f64::min);
                let right = corners
                    .iter()
                    .map(|c| c.0)
                    .fold(f64::NEG_INFINITY, f64::max);
                let top = corners.iter().map(|c| c.1).fold(f64::INFINITY, f64::min);
                Some(serde_json::json!({
                    "id": placed.id.full_id(),
                    "name": source.name,
                    "x": left,
                    "y": top,
                    "width": right - left,
                    "selected": selected.contains(&placed.id),
                }))
            })
            .collect();
        serde_json::Value::Array(rows).to_string()
    }
}

#[cfg(test)]
#[path = "../placed_page_tests.rs"]
mod tests;
