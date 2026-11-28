//! The Pen strip's layer pick (`specs/0158-pen-display` slice 2b criterion 3, design-system row
//! "Pen layer pick"): the active page's layers front-most first, and choosing which one this
//! window draws into.

use curvyo_document_core::{NodeId, NodeKind};

use super::Session;
use super::layers::ActiveLayer;
use super::layers_panel::parse_id;

/// The name a layer without a name shows.
const UNNAMED_LAYER: &str = "Layer";

/// One row of the layer pick.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PenLayer {
    /// The layer's id.
    pub id: NodeId,
    /// Its name.
    pub name: String,
    /// Whether it is hidden (chooseable; drawing there shows the Pen's notice).
    pub hidden: bool,
    /// Whether it is locked (chooseable; drawing there shows the Pen's notice).
    pub locked: bool,
    /// Whether new strokes of this window go into it.
    pub active: bool,
}

impl Session {
    /// The active page's layers, front-most first; empty for a page without layers.
    #[must_use]
    pub fn pen_layers(&self) -> Vec<PenLayer> {
        let active = self.active_layer();
        self.document_read()
            .nodes
            .iter()
            .rev()
            .filter(|read| read.kind == NodeKind::Layer)
            .map(|read| PenLayer {
                id: read.id,
                name: read
                    .registers
                    .name
                    .clone()
                    .unwrap_or_else(|| UNNAMED_LAYER.to_owned()),
                hidden: !read.visible,
                locked: read.locked,
                active: active == Some(read.id),
            })
            .collect()
    }

    /// Makes the layer `id` (its JSON id) this window's active layer: session state, no step and
    /// no document change; the selection stays. `false` for an id that is not a live layer.
    pub fn set_pen_layer(&mut self, id: &str) -> bool {
        let Some(layer) = parse_id(id).filter(|&id| self.document.is_layer(id)) else {
            return false;
        };
        self.layers.active = ActiveLayer::Chosen(Some(layer));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::align_test_support::{click, draw_rect, select_nothing};
    use super::*;

    fn with_layers() -> (Session, NodeId, NodeId, NodeId) {
        let session = Session::new(1);
        let back = session.document.new_layer(None);
        let middle = session.document.new_layer(Some(back));
        let front = session.document.new_layer(Some(middle));
        session.document.rename_node(front, "Cut").unwrap();
        session.document.set_node_visible(&[middle], false).unwrap();
        session.document.set_node_locked(&[back], true).unwrap();
        (session, back, middle, front)
    }

    /// Criterion 3: front-most first, hidden and locked layers listed with their state, the
    /// front-most one active before anything was chosen.
    #[test]
    fn the_layers_are_listed_front_most_first() {
        let (session, back, middle, front) = with_layers();
        let layers = session.pen_layers();
        let ids: Vec<NodeId> = layers.iter().map(|l| l.id).collect();
        assert_eq!(ids, vec![front, middle, back]);
        assert_eq!(layers[0].name, "Cut");
        assert!(layers[0].active && !layers[1].active && !layers[2].active);
        assert!(layers[1].hidden && !layers[1].locked);
        assert!(layers[2].locked && !layers[2].hidden);
    }

    /// Criterion 3: choosing a layer sets this window's active layer, writes nothing and keeps
    /// the history; a hidden layer is chooseable; another id is refused.
    #[test]
    fn choosing_a_layer_is_no_step() {
        let (mut session, _, middle, _) = with_layers();
        let json = session.document.export_json().unwrap();
        let version = session.document.version();
        let id = serde_json::to_string(&middle).unwrap();
        assert!(session.set_pen_layer(&id));
        assert_eq!(session.active_layer(), Some(middle));
        assert!(session.pen_layers()[1].active);
        assert_eq!(session.document.export_json().unwrap(), json);
        assert_eq!(session.document.version(), version);
        assert!(!session.set_pen_layer("nonsense"));
        assert_eq!(session.active_layer(), Some(middle));
    }

    /// Criterion 3: a selection made later moves the active layer; a page without layers lists
    /// none.
    #[test]
    fn a_selection_moves_it_and_no_layers_list_none() {
        let (mut session, _, middle, front) = with_layers();
        draw_rect(&mut session, (10.0, 10.0), (20.0, 20.0));
        select_nothing(&mut session);
        assert!(session.set_pen_layer(&serde_json::to_string(&middle).unwrap()));
        click(&mut session, (10.0, 15.0), false);
        assert_eq!(session.active_layer(), Some(front), "the selection's layer");
        assert_eq!(Session::new(2).pen_layers(), Vec::new());
    }
}
