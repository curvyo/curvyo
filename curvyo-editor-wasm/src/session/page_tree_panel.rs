//! `Session`'s side of a laser page's own Layers tree (`specs/0106-move-versus-edit` criteria 17
//! to 19, `adrs.md` decision 7): which objects are the page's own items and which are its placed
//! pages' members, read by the one membership rule, and the press on a placed page row.

use std::collections::{HashMap, HashSet};

use curvyo_document_core::{NodeId, NodeKind, NodeRead};
use curvyo_ui_core::{
    Expansion, Home, LayerRows, PageTree, PlacedPageRow, Reach, RowFacts, object_bounds,
};

use super::Session;

/// The top-level object (a child of a layer or of the root) each node lies in.
fn top_level_units(nodes: &[NodeRead]) -> HashMap<NodeId, NodeId> {
    let layers: HashSet<NodeId> = nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Layer)
        .map(|n| n.id)
        .collect();
    let mut tops: HashMap<NodeId, NodeId> = HashMap::new();
    for node in nodes {
        if layers.contains(&node.id) {
            continue;
        }
        let top = match node.parent {
            Some(parent) if !layers.contains(&parent) => {
                tops.get(&parent).copied().unwrap_or(node.id)
            }
            _ => node.id,
        };
        tops.insert(node.id, top);
    }
    tops
}

impl Session {
    /// The rows of the active laser page's tree over the node read `nodes`.
    pub(super) fn laser_page_rows(
        &self,
        nodes: &[NodeRead],
        expansion: &Expansion,
        facts: RowFacts<'_>,
    ) -> LayerRows {
        let page = self.active_page();
        let (pages, membership) = self.context_read();
        let tops = top_level_units(nodes);
        let read = self.document_read();
        let home = Home::Laser(page.id.clone());
        let mut own: HashSet<NodeId> = HashSet::new();
        for leaf in read.leaves.iter() {
            if pages.home_of(&membership, leaf.id(), object_bounds(leaf)) == home
                && let Some(&top) = tops.get(&leaf.id())
            {
                own.insert(top);
            }
        }
        // Later placements lie above earlier ones: topmost first.
        let placed: Vec<_> = self
            .document
            .placed_pages()
            .into_iter()
            .filter(|p| p.output.id == page.id)
            .rev()
            .collect();
        let members: Vec<HashSet<NodeId>> = placed
            .iter()
            .map(|p| {
                p.source.as_ref().map_or_else(HashSet::new, |source| {
                    read.leaves
                        .iter()
                        .filter(|leaf| {
                            membership.is_member(
                                source,
                                leaf.id(),
                                object_bounds(leaf),
                                Reach::Touches,
                            )
                        })
                        .filter_map(|leaf| tops.get(&leaf.id()).copied())
                        .collect()
                })
            })
            .collect();
        let rows: Vec<PlacedPageRow<'_>> = placed
            .iter()
            .zip(&members)
            .map(|(p, members)| PlacedPageRow {
                id: p.id,
                source_name: p.source.as_ref().map_or("", |s| s.name.as_str()),
                members,
            })
            .collect();
        let fixtures_shown = self
            .active_manufacturing()
            .is_none_or(|m| m.fixtures.iter().all(|f| f.visible));
        let Some(root) = page.kind.manufacturing() else {
            return LayerRows::build(nodes, expansion, facts);
        };
        LayerRows::for_laser_page(
            nodes,
            &PageTree {
                page: root,
                page_name: &page.name,
                own: &own,
                placed: &rows,
                fixtures_shown,
            },
            expansion,
            facts,
        )
    }

    /// A press on a placed page row or on a row inside it (the host sends the placed page's id):
    /// the placed page becomes the selection (criterion 18). `false` for any other id.
    pub(super) fn press_placed_page_row(&mut self, id: NodeId) -> bool {
        if !self.document.is_placement(id) {
            return false;
        }
        if self.context_group().is_some() {
            self.leave_to_level(0);
        }
        self.selection.set(&[id]);
        self.panel.from_list = true;
        self.panel.target_layer = false;
        true
    }

    /// The Fixtures row's eye (criterion 17): shows or hides every fixture of the active laser
    /// page, as the Document tab's fixture eyes do. `false` when nothing changed.
    pub fn set_layer_fixtures_shown(&mut self, shown: bool) -> bool {
        let Some(manufacturing) = self.active_manufacturing() else {
            return false;
        };
        let mut changed = false;
        for fixture in manufacturing.fixtures.iter().filter(|f| f.visible != shown) {
            changed |= self.set_fixture_visible(fixture.id, shown);
        }
        changed
    }
}
