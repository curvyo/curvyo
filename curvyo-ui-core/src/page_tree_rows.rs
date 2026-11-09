//! The rows of a laser page's own Layers tree (`specs/0106-move-versus-edit` criteria 17 to 20,
//! Frage 166 A, `adrs.md` decision 7): a second builder of [`LayerRows`].
//!
//! The tree is rooted at the page; below it the own items, clones and placed pages in the order
//! the canvas draws them, topmost first (placed pages lie under the drawing), then the Fixtures
//! row. No layer row appears; a layer's eye and lock still reach its objects as inherited state.
//! Inner rows exist only for expanded placed pages, so the flattening stays O(visible + own
//! items) and the window of `0050` is reused unchanged.

use std::collections::{HashMap, HashSet};

use curvyo_document_core::{NodeId, NodeKind, NodeRead, Opacity};

use crate::layer_rows::{
    Cause, Container, Expansion, Flags, LayerRow, LayerRows, RowFacts, RowKind, RowMark,
};
use crate::share_text::cut_label;

/// How long a placed page's source name may be on its row (criterion 18).
const SOURCE_NAME_CHARS: usize = 40;

/// A placed page as its row reads it.
#[derive(Debug, Clone)]
pub struct PlacedPageRow<'a> {
    /// The placement.
    pub id: NodeId,
    /// The source page's name.
    pub source_name: &'a str,
    /// The source page's members: top-level objects (children of a layer or of the root).
    pub members: &'a HashSet<NodeId>,
}

/// What a laser page's tree shows besides the document's nodes.
#[derive(Debug, Clone)]
pub struct PageTree<'a> {
    /// The id the root and Fixtures rows carry: the page's manufacturing node.
    pub page: NodeId,
    /// The page's name.
    pub page_name: &'a str,
    /// The page's own items and clones: top-level objects.
    pub own: &'a HashSet<NodeId>,
    /// Its placed pages, topmost first.
    pub placed: &'a [PlacedPageRow<'a>],
    /// Whether the fixtures are shown.
    pub fixtures_shown: bool,
}

/// The subtrees of `tops` out of `nodes` (pre-order), without layers, each top lifted to the top
/// level: the input of [`LayerRows::build`].
fn lifted(nodes: &[NodeRead], tops: &HashSet<NodeId>) -> Vec<NodeRead> {
    let mut depth_of: HashMap<NodeId, usize> = HashMap::new();
    let mut kept = Vec::new();
    for node in nodes {
        if node.kind == NodeKind::Layer {
            continue;
        }
        let depth = if tops.contains(&node.id) {
            0
        } else if let Some(&parent) = node.parent.as_ref().and_then(|p| depth_of.get(p)) {
            parent + 1
        } else {
            continue;
        };
        depth_of.insert(node.id, depth);
        let mut lifted = node.clone();
        lifted.depth = depth;
        if depth == 0 {
            lifted.parent = None;
        }
        kept.push(lifted);
    }
    kept
}

/// A row that is no document node: the page root and the Fixtures row.
fn plain_row(id: NodeId, kind: RowKind, name: &str, depth: usize, shown: bool) -> LayerRow {
    LayerRow {
        id,
        parent: None,
        depth,
        kind,
        name: name.to_owned(),
        named: true,
        text: name.to_owned(),
        own: Flags {
            visible: shown,
            locked: false,
        },
        effective: Flags {
            visible: shown,
            locked: false,
        },
        hidden_by: None,
        locked_by: None,
        opacity: Opacity::OPAQUE,
        container: None,
        mark: RowMark::None,
        position: 1,
        siblings: 1,
        chain_line: String::new(),
        overrides_line: String::new(),
        via: None,
        read_only: false,
        page_tree: true,
    }
}

/// The layer that hides or locks a node, as the row names it ("layer Cut").
fn layer_cause(layers: &HashMap<NodeId, &NodeRead>, id: Option<NodeId>) -> Option<Cause> {
    let layer = layers.get(&id?)?;
    Some(Cause {
        id: layer.id,
        kind: "layer",
        name: layer
            .registers
            .name
            .clone()
            .unwrap_or_else(|| format!("Layer {}", layer.id.short_id())),
    })
}

/// Marks `rows` as a laser page's tree and numbers the rows at the page's level as siblings.
fn mark_page_tree(rows: &mut [LayerRow]) {
    for row in rows.iter_mut() {
        row.page_tree = true;
    }
    // Every row at the page's level is a sibling of every other (`aria-posinset`).
    let level: Vec<usize> = (0..rows.len())
        .filter(|&i| rows[i].depth == 1 && rows[i].kind != RowKind::Note)
        .collect();
    for (place, &at) in level.iter().enumerate() {
        rows[at].position = place + 1;
        rows[at].siblings = level.len();
    }
}

impl LayerRows {
    /// The rows of a laser page's tree (criteria 17 to 19) over the document's `nodes`.
    #[must_use]
    pub fn for_laser_page(
        nodes: &[NodeRead],
        tree: &PageTree<'_>,
        expansion: &Expansion,
        facts: RowFacts<'_>,
    ) -> Self {
        let layers: HashMap<NodeId, &NodeRead> = nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Layer)
            .map(|n| (n.id, n))
            .collect();
        let read: HashMap<NodeId, &NodeRead> = nodes.iter().map(|n| (n.id, n)).collect();
        // The own items keep the document's layer state as inherited state (Proposal).
        let with_layer_state = |mut row: LayerRow| {
            if let Some(node) = read.get(&row.id) {
                if row.hidden_by.is_none() {
                    row.hidden_by = layer_cause(&layers, node.hidden_by);
                }
                if row.locked_by.is_none() {
                    row.locked_by = layer_cause(&layers, node.locked_by);
                }
            }
            row
        };
        let own_nodes = lifted(nodes, tree.own);
        let own = LayerRows::build(&own_nodes, expansion, facts);
        let objects = own_nodes.len() + tree.placed.len();
        let mut rows = vec![plain_row(
            tree.page,
            RowKind::PageRoot,
            tree.page_name,
            0,
            true,
        )];
        rows.extend(own.into_rows().into_iter().map(|mut row| {
            row.depth += 1;
            with_layer_state(row)
        }));
        for placed in tree.placed {
            let expanded = expansion.is_expanded(placed.id, RowKind::PlacedPage);
            let name = format!(
                "Placed page of {}",
                cut_label(placed.source_name, SOURCE_NAME_CHARS)
            );
            let mut row = plain_row(placed.id, RowKind::PlacedPage, &name, 1, true);
            row.container = Some(Container {
                children: placed.members.len(),
                expanded,
                contains_selection: false,
            });
            if facts.selected.contains(&placed.id) {
                row.mark = RowMark::Selected;
            }
            rows.push(row);
            if !expanded {
                continue;
            }
            if placed.members.is_empty() {
                let mut empty = plain_row(placed.id, RowKind::Note, "Empty", 2, true);
                empty.via = Some(placed.id);
                rows.push(empty);
                continue;
            }
            let inner = lifted(nodes, placed.members);
            let none = HashSet::new();
            let inner = LayerRows::build(
                &inner,
                expansion,
                RowFacts {
                    selected: &none,
                    active_layer: None,
                    entered: None,
                },
            );
            rows.extend(inner.into_rows().into_iter().map(|mut row| {
                row.depth += 2;
                row.via = Some(placed.id);
                row.read_only = true;
                with_layer_state(row)
            }));
        }
        if own_nodes.is_empty() && tree.placed.is_empty() {
            rows.push(plain_row(
                tree.page,
                RowKind::Note,
                "Nothing on this page yet.",
                1,
                true,
            ));
        }
        rows.push(plain_row(
            tree.page,
            RowKind::Fixtures,
            "Fixtures",
            1,
            tree.fixtures_shown,
        ));
        mark_page_tree(&mut rows);
        LayerRows::from_rows(rows, objects)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use curvyo_document_core::{Document, Point, RectBounds};

    use super::*;

    fn rect(document: &Document, x: f64) -> NodeId {
        document.create_rect(RectBounds::from_corners(
            Point::new(x, 0.0),
            Point::new(x + 10.0, 10.0),
        ))
    }

    fn page_id() -> NodeId {
        serde_json::from_value(serde_json::json!({"peer": "9", "counter": 9})).unwrap()
    }

    fn placement() -> NodeId {
        serde_json::from_value(serde_json::json!({"peer": "9", "counter": 10})).unwrap()
    }

    fn build(
        document: &Document,
        own: &HashSet<NodeId>,
        members: &HashSet<NodeId>,
        expansion: &Expansion,
        selected: &HashSet<NodeId>,
    ) -> LayerRows {
        let placed = [PlacedPageRow {
            id: placement(),
            source_name: "Coaster",
            members,
        }];
        LayerRows::for_laser_page(
            &document.read_nodes(),
            &PageTree {
                page: page_id(),
                page_name: "Laser 1",
                own,
                placed: &placed,
                fixtures_shown: true,
            },
            expansion,
            RowFacts {
                selected,
                active_layer: None,
                entered: None,
            },
        )
    }

    fn kinds(rows: &LayerRows) -> Vec<(RowKind, usize)> {
        rows.window(0, usize::MAX)
            .iter()
            .map(|row| (row.kind, row.depth))
            .collect()
    }

    #[test]
    fn criterion_17_the_page_root_own_items_topmost_first_placed_pages_then_fixtures() {
        let document = Document::new(1);
        let source = rect(&document, 0.0);
        let a = rect(&document, 500.0);
        let b = rect(&document, 520.0);
        let own: HashSet<NodeId> = [a, b].into_iter().collect();
        let members: HashSet<NodeId> = [source].into_iter().collect();
        let rows = build(
            &document,
            &own,
            &members,
            &Expansion::default(),
            &HashSet::new(),
        );
        assert_eq!(
            kinds(&rows),
            [
                (RowKind::PageRoot, 0),
                (RowKind::Rectangle, 1),
                (RowKind::Rectangle, 1),
                (RowKind::PlacedPage, 1),
                (RowKind::Fixtures, 1),
            ]
        );
        let window = rows.window(0, usize::MAX);
        assert_eq!(window[1].id, b, "topmost first");
        assert_eq!(window[3].name, "Placed page of Coaster");
        assert_eq!((window[4].position, window[4].siblings), (4, 4));
        assert_eq!(
            rows.index_of(page_id()),
            None,
            "root and Fixtures are no object rows"
        );
        assert_eq!(rows.index_of(placement()), Some(3));
        assert!(
            rows.window(0, usize::MAX)
                .iter()
                .all(|r| r.kind != RowKind::Layer)
        );
    }

    #[test]
    fn the_rows_are_named_and_described_as_the_ux_notes_say() {
        use crate::layer_text::{row_accessible_name, row_state_line};
        let document = Document::new(4);
        let source = rect(&document, 0.0);
        let own_item = rect(&document, 500.0);
        let own: HashSet<NodeId> = [own_item].into_iter().collect();
        let members: HashSet<NodeId> = [source].into_iter().collect();
        let mut expansion = Expansion::default();
        expansion.set(placement(), RowKind::PlacedPage, true);
        let rows = build(&document, &own, &members, &expansion, &HashSet::new());
        let names: Vec<String> = rows
            .window(0, usize::MAX)
            .iter()
            .map(row_accessible_name)
            .collect();
        let short = |id: NodeId| id.short_id();
        assert_eq!(
            names,
            [
                "Laser 1, laser page".to_owned(),
                format!("Rectangle {}, own item", short(own_item)),
                "Placed page of Coaster, 1 object, read-only".to_owned(),
                format!("Rectangle {}, in placed page, read-only", short(source)),
                "Fixtures, shown".to_owned(),
            ]
        );
        assert_eq!(
            row_state_line(&rows.window(0, usize::MAX)[2]),
            "Live, read-only"
        );
    }

    #[test]
    fn an_empty_page_and_an_empty_placed_page_say_so_in_a_muted_row() {
        let document = Document::new(3);
        let none = HashSet::new();
        let placed: [PlacedPageRow<'_>; 0] = [];
        let rows = LayerRows::for_laser_page(
            &document.read_nodes(),
            &PageTree {
                page: page_id(),
                page_name: "Laser 1",
                own: &none,
                placed: &placed,
                fixtures_shown: true,
            },
            &Expansion::default(),
            RowFacts {
                selected: &none,
                active_layer: None,
                entered: None,
            },
        );
        let window = rows.window(0, usize::MAX);
        assert_eq!(window[1].kind, RowKind::Note);
        assert_eq!(window[1].name, "Nothing on this page yet.");
        assert_eq!(window[2].kind, RowKind::Fixtures);
        assert_eq!(
            (window[2].position, window[2].siblings),
            (1, 1),
            "a note is no sibling"
        );
        // A placed page of an empty source expands to one muted "Empty" row.
        let mut expansion = Expansion::default();
        expansion.set(placement(), RowKind::PlacedPage, true);
        let rows = build(&document, &none, &none, &expansion, &none);
        let window = rows.window(0, usize::MAX);
        assert_eq!(
            (window[2].kind, window[2].name.as_str()),
            (RowKind::Note, "Empty")
        );
        assert_eq!(window[2].via, Some(placement()));
        assert_eq!(window[2].depth, 2);
    }

    #[test]
    fn criterion_18_an_expanded_placed_page_shows_its_source_read_only_through_it() {
        let document = Document::new(2);
        let source = rect(&document, 0.0);
        let own = HashSet::new();
        let members: HashSet<NodeId> = [source].into_iter().collect();
        let mut expansion = Expansion::default();
        expansion.set(placement(), RowKind::PlacedPage, true);
        let selected: HashSet<NodeId> = [placement()].into_iter().collect();
        let rows = build(&document, &own, &members, &expansion, &selected);
        let window = rows.window(0, usize::MAX);
        assert_eq!(window[1].mark, RowMark::Selected);
        let inner = &window[2];
        assert_eq!((inner.id, inner.depth), (source, 2));
        assert_eq!(inner.via, Some(placement()));
        assert!(inner.read_only);
        assert_eq!(inner.mark, RowMark::None, "no selected ground inside");
        assert_eq!(
            rows.index_of(source),
            None,
            "the source's own row is elsewhere"
        );
    }
}
