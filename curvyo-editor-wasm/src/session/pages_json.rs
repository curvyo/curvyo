//! The host JSON of the page rows (`pages_json`).

use curvyo_document_core::{NodeId, Page};

use super::Session;

impl Session {
    /// The sketch facts of a sketch page's row (`specs/0099` criteria 10, 14, 16): its state, how
    /// many clones follow it and whether it has something to draw; `null` for other kinds.
    fn sketch_row_json(&self, page: &Page) -> serde_json::Value {
        let Some(state) = self.sketch_row_state(page) else {
            return serde_json::Value::Null;
        };
        let owner = self.document.page_sketch(&page.id);
        let usage = owner.map(|id| self.sketch_usage(id)).unwrap_or_default();
        serde_json::json!({
            "state": state,
            "clones": usage.clones,
            "pages": usage.pages,
            "drawable": owner.is_some_and(|id| self.sketch_has_geometry(id)),
        })
    }
    /// Page rows and screen rectangles, formatted for the host.
    #[must_use]
    pub fn pages_json(&self) -> String {
        let active = self.active_page().id;
        let drop_target = self.row_drag_target();
        // The pages a move-only selection belongs to: semibold labels (`specs/0106` UX §2).
        let homes = self.home_page_ids();
        let pages = self
            .document_preview()
            .map_or_else(|| self.shown_pages(), |p| p.pages.clone());
        let rows:Vec<_>=pages.iter().map(|p| {
            let (x,y)=self.view().document_to_screen(p.origin);
            let chip = self.kind_chip(&p.kind);
            let sketch = self.sketch_row_json(p);
            // The Operations view toggle of a laser page's label (`specs/0111` criterion 15).
            let operations_view = p
                .kind
                .manufacturing()
                .filter(|&m| self.manufacturing_available(m))
                .map(|m| self.operations_view_pressed(m) && !self.toolpath_view_pressed(m));
            // The Toolpath view toggle of the same label (`specs/0029` criterion 8), exclusive with
            // the Operations view (UX P2).
            let toolpath_view = p
                .kind
                .manufacturing()
                .filter(|&m| self.manufacturing_available(m))
                .map(|m| self.toolpath_view_pressed(m));
            serde_json::json!({"operationsView":operations_view,"toolpathView":toolpath_view,"id":p.id,"name":p.name,"active":p.id==active,"frameOnly":self.frame_moves_alone(p),"kind":chip.kind,"kindWord":chip.word,"kindIcon":chip.icon,"kindOff":chip.off,"fixedSize":p.kind.fixed_size(),"manufacturing":p.kind.manufacturing().map(NodeId::full_id),"machine":self.machine_name_of(&p.kind),"size":curvyo_ui_core::format_size(p.size,self.display_unit()),"x":p.origin.x,"y":p.origin.y,"screenX":x,"screenY":y,"screenWidth":p.size.width.as_mm()*self.view().scale(),"screenHeight":p.size.height.as_mm()*self.view().scale(),"sketch":sketch,"dropTarget":drop_target==Some(p.id.as_str()),"home":homes.contains(&p.id)})
        }).collect();
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_owned())
    }
}
