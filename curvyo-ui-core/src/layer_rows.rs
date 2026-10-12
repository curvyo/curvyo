//! The rows of the Layers list: the node read flattened depth first, front-most at the top, with
//! collapsed containers hiding their descendants, and the window of rows the page draws
//! (`specs/0050-layers-and-objects-panel` criteria 5 to 9, 12, 22 and 48 to 52, `adrs.md`
//! decision 7).
//!
//! The flattening is O(n) and is made once per document read, collapse change and selection; a
//! window is a slice of it.

use std::collections::HashSet;

use curvyo_document_core::{NodeId, NodeKind, NodeRead, ObjectSnapshot, Opacity, Shape};

/// What a row stands for, as its glyph and default name say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A layer.
    Layer,
    /// A group.
    Group,
    /// A path.
    Path,
    /// A compound path.
    CompoundPath,
    /// A rectangle.
    Rectangle,
    /// An ellipse.
    Ellipse,
    /// A polygon.
    Polygon,
    /// A star.
    Star,
}

impl RowKind {
    fn of(kind: &NodeKind) -> Self {
        match kind {
            NodeKind::Layer => Self::Layer,
            NodeKind::Group => Self::Group,
            NodeKind::Leaf(ObjectSnapshot::Path(path)) if path.is_compound() => Self::CompoundPath,
            NodeKind::Leaf(ObjectSnapshot::Path(_)) => Self::Path,
            NodeKind::Leaf(ObjectSnapshot::Primitive(primitive)) => match primitive.shape {
                Shape::Rect { .. } => Self::Rectangle,
                Shape::Ellipse { .. } => Self::Ellipse,
                Shape::Polygon { .. } => Self::Polygon,
                Shape::Star { .. } => Self::Star,
            },
        }
    }

    /// The word of the kind: "Rectangle", "Compound path", "Layer".
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Layer => "Layer",
            Self::Group => "Group",
            Self::Path => "Path",
            Self::CompoundPath => "Compound path",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Polygon => "Polygon",
            Self::Star => "Star",
        }
    }

    /// The code the host keys its glyph on: `"layer"`, `"compound-path"` and so on.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Layer => "layer",
            Self::Group => "group",
            Self::Path => "path",
            Self::CompoundPath => "compound-path",
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Polygon => "polygon",
            Self::Star => "star",
        }
    }
}

/// The ancestor that hides or locks a row, by its kind and name ("layer Cut").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cause {
    /// The ancestor.
    pub id: NodeId,
    /// "layer" or "group".
    pub kind: &'static str,
    /// Its name as the list shows it.
    pub name: String,
}

/// A node's visible and locked flags, its own or the effective ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
    /// Shown.
    pub visible: bool,
    /// Locked.
    pub locked: bool,
}

/// What a container row adds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Container {
    /// How many children it has.
    pub children: usize,
    /// Whether it shows its children.
    pub expanded: bool,
    /// Collapsed, with a selected descendant (criterion 8).
    pub contains_selection: bool,
}

/// The one mark a row can carry: selected (an object or group of the canvas selection,
/// criterion 12), the active layer (criterion 35) or the entered group (`0023`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowMark {
    /// None of them.
    None,
    /// A selected object of the canvas.
    Selected,
    /// The active layer, while no group is entered.
    ActiveLayer,
    /// The entered group.
    Entered,
}

/// One row of the Layers list.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerRow {
    /// The node.
    pub id: NodeId,
    /// The node that holds it; `None` at the top level.
    pub parent: Option<NodeId>,
    /// 0 at the top level (`aria-level` is one more).
    pub depth: usize,
    /// What it is.
    pub kind: RowKind,
    /// The name the row shows: its own, or `"<Kind> <id>"` (criterion 7).
    pub name: String,
    /// Whether the name is the node's own.
    pub named: bool,
    /// The node's own flags.
    pub own: Flags,
    /// The flags through its ancestors.
    pub effective: Flags,
    /// The nearest ancestor that hides it.
    pub hidden_by: Option<Cause>,
    /// The nearest ancestor that locks it.
    pub locked_by: Option<Cause>,
    /// The node's own opacity.
    pub opacity: Opacity,
    /// `Some` for a layer or a group.
    pub container: Option<Container>,
    /// Selected, active layer or entered group.
    pub mark: RowMark,
    /// Its place among its siblings, front-most first, from 1 (`aria-posinset`).
    pub position: usize,
    /// How many siblings it has, itself included (`aria-setsize`).
    pub siblings: usize,
}

impl LayerRow {
    /// Whether it is a container that shows its children.
    #[must_use]
    pub fn expanded(&self) -> bool {
        self.container.is_some_and(|container| container.expanded)
    }

    /// The opacity readout ("60%"), shown only when it is not 100 % (criterion 6).
    #[must_use]
    pub fn opacity_readout(&self) -> Option<String> {
        let percent = percent(self.opacity);
        (percent < 100).then(|| format!("{percent}%"))
    }
}

/// `opacity` as a whole percentage.
#[must_use]
pub fn percent(opacity: Opacity) -> u8 {
    // invariant: an opacity lies in 0..=1, so the rounded percentage fits a u8.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = (opacity.get() * 100.0).round() as u8;
    value
}

/// Which containers are expanded. View state: not saved, no step, dropped by New and Open
/// (criterion 8). Layers start expanded and groups collapsed; the set holds the nodes whose
/// state differs from that default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Expansion {
    flipped: HashSet<NodeId>,
}

impl Expansion {
    /// Whether the container `id` of kind `kind` shows its children.
    #[must_use]
    pub fn is_expanded(&self, id: NodeId, kind: RowKind) -> bool {
        (kind == RowKind::Layer) != self.flipped.contains(&id)
    }

    /// Sets whether the container `id` of kind `kind` shows its children; returns whether that
    /// changed anything.
    pub fn set(&mut self, id: NodeId, kind: RowKind, expanded: bool) -> bool {
        if self.is_expanded(id, kind) == expanded {
            return false;
        }
        if !self.flipped.remove(&id) {
            self.flipped.insert(id);
        }
        true
    }
}

/// The facts of the session the rows show besides the document.
#[derive(Debug, Clone, Copy)]
pub struct RowFacts<'a> {
    /// The selected objects of the context: leaves and groups.
    pub selected: &'a HashSet<NodeId>,
    /// The active layer.
    pub active_layer: Option<NodeId>,
    /// The entered group.
    pub entered: Option<NodeId>,
}

/// The mark of the row of `id`: entered beats selected beats active layer.
fn row_mark(facts: RowFacts<'_>, id: NodeId, kind: RowKind) -> RowMark {
    if facts.entered == Some(id) {
        RowMark::Entered
    } else if facts.selected.contains(&id) {
        RowMark::Selected
    } else if kind == RowKind::Layer && facts.entered.is_none() && facts.active_layer == Some(id) {
        RowMark::ActiveLayer
    } else {
        RowMark::None
    }
}

/// The name a node shows: its own, or "<Kind> <six-character id>" (criterion 7).
fn shown_name(read: &NodeRead, kind: RowKind) -> (String, bool) {
    match &read.registers.name {
        Some(name) => (name.clone(), true),
        None => (format!("{} {}", kind.word(), read.id.short_id()), false),
    }
}

/// The node read as a tree, made once per build: the children of each node, the top level, and
/// what each node shows.
struct Tree<'a> {
    nodes: &'a [NodeRead],
    index: std::collections::HashMap<NodeId, usize>,
    children: Vec<Vec<usize>>,
    top: Vec<usize>,
    /// A container holds a selected node somewhere below it.
    below_selected: Vec<bool>,
    kinds: Vec<RowKind>,
    names: Vec<(String, bool)>,
}

impl<'a> Tree<'a> {
    fn of(nodes: &'a [NodeRead], selected: &HashSet<NodeId>) -> Self {
        let index: std::collections::HashMap<NodeId, usize> = nodes
            .iter()
            .enumerate()
            .map(|(at, read)| (read.id, at))
            .collect();
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
        let mut top: Vec<usize> = Vec::new();
        for (at, read) in nodes.iter().enumerate() {
            match read.parent.and_then(|parent| index.get(&parent)) {
                Some(&parent) => children[parent].push(at),
                None => top.push(at),
            }
        }
        let mut below_selected = vec![false; nodes.len()];
        for at in (0..nodes.len()).rev() {
            let here = below_selected[at] || selected.contains(&nodes[at].id);
            if here && let Some(&parent) = nodes[at].parent.and_then(|p| index.get(&p)) {
                below_selected[parent] = true;
            }
        }
        let kinds: Vec<RowKind> = nodes.iter().map(|read| RowKind::of(&read.kind)).collect();
        let names = nodes
            .iter()
            .zip(&kinds)
            .map(|(read, &kind)| shown_name(read, kind))
            .collect();
        Self {
            nodes,
            index,
            children,
            top,
            below_selected,
            kinds,
            names,
        }
    }

    /// The ancestor `id` as a cause of an inherited state.
    fn cause(&self, id: Option<NodeId>) -> Option<Cause> {
        id.and_then(|id| self.index.get(&id)).map(|&at| Cause {
            id: self.nodes[at].id,
            kind: if self.kinds[at] == RowKind::Layer {
                "layer"
            } else {
                "group"
            },
            name: self.names[at].0.clone(),
        })
    }

    /// The row of the node at `at`, `position` of `siblings` among its siblings.
    fn row(
        &self,
        at: usize,
        position: usize,
        siblings: usize,
        expansion: &Expansion,
        facts: RowFacts<'_>,
    ) -> LayerRow {
        let read = &self.nodes[at];
        let kind = self.kinds[at];
        let expanded = read.is_container() && expansion.is_expanded(read.id, kind);
        let container = read.is_container().then_some(Container {
            children: self.children[at].len(),
            expanded,
            contains_selection: !expanded && self.below_selected[at],
        });
        LayerRow {
            id: read.id,
            parent: read.parent,
            depth: read.depth,
            kind,
            name: self.names[at].0.clone(),
            named: self.names[at].1,
            own: Flags {
                visible: read.registers.visible,
                locked: read.registers.locked,
            },
            effective: Flags {
                visible: read.visible,
                locked: read.locked,
            },
            hidden_by: self.cause(read.hidden_by),
            locked_by: self.cause(read.locked_by),
            opacity: read.registers.opacity,
            container,
            mark: row_mark(facts, read.id, kind),
            position,
            siblings,
        }
    }
}

/// The flattened list of every visible row and the counts the panel needs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayerRows {
    rows: Vec<LayerRow>,
    objects: usize,
}

impl LayerRows {
    /// The rows of `nodes` (a pre-order read, back to front) under `expansion` and `facts`.
    #[must_use]
    pub fn build(nodes: &[NodeRead], expansion: &Expansion, facts: RowFacts<'_>) -> Self {
        let tree = Tree::of(nodes, facts.selected);
        let mut rows = Vec::new();
        // Depth first, the front-most (last) child first.
        let mut stack: Vec<(usize, usize, usize)> = tree
            .top
            .iter()
            .enumerate()
            .map(|(place, &at)| (at, tree.top.len() - place, tree.top.len()))
            .collect();
        while let Some((at, position, siblings)) = stack.pop() {
            let row = tree.row(at, position, siblings, expansion, facts);
            if row.expanded() {
                let below = &tree.children[at];
                stack.extend(
                    below
                        .iter()
                        .enumerate()
                        .map(|(place, &child)| (child, below.len() - place, below.len())),
                );
            }
            rows.push(row);
        }
        let objects = nodes
            .iter()
            .filter(|read| read.kind != NodeKind::Layer)
            .count();
        Self { rows, objects }
    }

    /// How many rows the list has (`aria-rowcount`): the scrollbar is exact from it.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the list has no row (criterion 9).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// How many nodes are not layers: the subject line's count (criterion 4).
    #[must_use]
    pub const fn objects(&self) -> usize {
        self.objects
    }

    /// The rows from `first` on, at most `count`: what the page draws. O(count).
    #[must_use]
    pub fn window(&self, first: usize, count: usize) -> &[LayerRow] {
        let start = first.min(self.rows.len());
        let end = start.saturating_add(count).min(self.rows.len());
        &self.rows[start..end]
    }

    /// The row index of `id`, if its row is shown.
    #[must_use]
    pub fn index_of(&self, id: NodeId) -> Option<usize> {
        self.rows.iter().position(|row| row.id == id)
    }

    /// The row of `id`, if it is shown.
    #[must_use]
    pub fn row(&self, id: NodeId) -> Option<&LayerRow> {
        self.rows.iter().find(|row| row.id == id)
    }

    /// The rows from the row of `anchor` to the row of `clicked`, either way round, that share
    /// `clicked`'s parent, in stacking order (back to front): the range of a Shift-click
    /// (criterion 11). Only `clicked` when the anchor is not shown or has another parent.
    #[must_use]
    pub fn sibling_range(&self, anchor: NodeId, clicked: NodeId) -> Vec<NodeId> {
        let (Some(from), Some(to)) = (self.index_of(anchor), self.index_of(clicked)) else {
            return vec![clicked];
        };
        let parent = self.rows[to].parent;
        if self.rows[from].parent != parent {
            return vec![clicked];
        }
        let (low, high) = (from.min(to), from.max(to));
        self.rows[low..=high]
            .iter()
            .rev()
            .filter(|row| row.parent == parent)
            .map(|row| row.id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Document, MovePlan, MoveTarget, Point, RectBounds};

    use super::*;

    fn rect(document: &Document, x: f64) -> NodeId {
        document.create_rect(RectBounds::from_corners(
            Point::new(x, 0.0),
            Point::new(x + 10.0, 10.0),
        ))
    }

    fn into(document: &Document, nodes: &[NodeId], layer: NodeId) {
        document
            .move_nodes(&MovePlan {
                nodes: nodes.to_vec(),
                target: MoveTarget::FrontOf(Some(layer)),
            })
            .unwrap();
    }

    fn rows(document: &Document, expansion: &Expansion, selected: &HashSet<NodeId>) -> LayerRows {
        LayerRows::build(
            &document.read_nodes(),
            expansion,
            RowFacts {
                selected,
                active_layer: None,
                entered: None,
            },
        )
    }

    fn ids(rows: &LayerRows) -> Vec<NodeId> {
        rows.window(0, usize::MAX)
            .iter()
            .map(|row| row.id)
            .collect()
    }

    /// Criterion 5's test: A, B, C gives C, B, A; a layer between B and C gives C, the layer
    /// (with its children), B, A.
    #[test]
    #[allow(clippy::many_single_char_names)]
    fn the_front_most_node_is_the_first_row() {
        let document = Document::new(1);
        let (a, b, c) = (
            rect(&document, 0.0),
            rect(&document, 20.0),
            rect(&document, 40.0),
        );
        let none = HashSet::new();
        assert_eq!(
            ids(&rows(&document, &Expansion::default(), &none)),
            vec![c, b, a]
        );
        let layer = document.new_layer(None);
        let (x, y) = (rect(&document, 60.0), rect(&document, 80.0));
        into(&document, &[x, y], layer);
        document
            .move_nodes(&MovePlan {
                nodes: vec![layer],
                target: MoveTarget::Below(c),
            })
            .unwrap();
        let list = rows(&document, &Expansion::default(), &none);
        assert_eq!(ids(&list), vec![c, layer, y, x, b, a]);
        let layer_row = list.row(layer).unwrap();
        assert_eq!((layer_row.position, layer_row.siblings), (2, 4));
        let y_row = list.row(y).unwrap();
        assert_eq!((y_row.depth, y_row.position, y_row.siblings), (1, 1, 2));
        assert_eq!(list.objects(), 5);
    }

    #[test]
    fn groups_start_collapsed_and_layers_expanded_and_collapsing_hides_descendants() {
        let document = Document::new(1);
        let (a, b) = (rect(&document, 0.0), rect(&document, 20.0));
        let group = document.group_objects(&[a, b]).unwrap();
        let layer = document.new_layer(None);
        into(&document, &[group], layer);
        let none = HashSet::new();
        let mut expansion = Expansion::default();
        assert_eq!(ids(&rows(&document, &expansion, &none)), vec![layer, group]);
        assert!(expansion.set(group, RowKind::Group, true));
        assert_eq!(
            ids(&rows(&document, &expansion, &none)),
            vec![layer, group, b, a]
        );
        assert!(expansion.set(layer, RowKind::Layer, false));
        assert_eq!(ids(&rows(&document, &expansion, &none)), vec![layer]);
        assert!(
            !expansion.set(layer, RowKind::Layer, false),
            "already collapsed"
        );
    }

    #[test]
    fn a_collapsed_container_with_a_selected_descendant_carries_the_mark() {
        let document = Document::new(1);
        let (a, b) = (rect(&document, 0.0), rect(&document, 20.0));
        let group = document.group_objects(&[a, b]).unwrap();
        let selected: HashSet<NodeId> = [a].into_iter().collect();
        let list = rows(&document, &Expansion::default(), &selected);
        assert!(
            list.row(group)
                .unwrap()
                .container
                .unwrap()
                .contains_selection
        );
        let mut open = Expansion::default();
        open.set(group, RowKind::Group, true);
        let list = rows(&document, &open, &selected);
        assert!(
            !list
                .row(group)
                .unwrap()
                .container
                .unwrap()
                .contains_selection
        );
        assert_eq!(list.row(a).unwrap().mark, RowMark::Selected);
    }

    #[test]
    fn default_names_and_the_opacity_readout() {
        let document = Document::new(1);
        let a = rect(&document, 0.0);
        let layer = document.new_layer(None);
        into(&document, &[a], layer);
        document
            .set_node_opacity(&[a], Opacity::new(0.6).unwrap())
            .unwrap();
        let list = rows(&document, &Expansion::default(), &HashSet::new());
        let row = list.row(a).unwrap();
        assert_eq!(row.name, format!("Rectangle {}", a.short_id()));
        assert!(!row.named);
        assert_eq!(row.opacity_readout().as_deref(), Some("60%"));
        let layer_row = list.row(layer).unwrap();
        assert_eq!(layer_row.name, "Layer 1");
        assert!(layer_row.named);
        assert_eq!(layer_row.opacity_readout(), None);
    }

    #[test]
    fn an_inherited_state_names_the_nearest_ancestor() {
        let document = Document::new(1);
        let a = rect(&document, 0.0);
        let layer = document.new_layer(None);
        document.rename_node(layer, "Cut").unwrap();
        into(&document, &[a], layer);
        document.set_node_visible(&[layer], false).unwrap();
        let list = rows(&document, &Expansion::default(), &HashSet::new());
        let row = list.row(a).unwrap();
        assert!(row.own.visible && !row.effective.visible);
        let cause = row.hidden_by.clone().unwrap();
        assert_eq!((cause.kind, cause.name.as_str()), ("layer", "Cut"));
    }

    #[test]
    #[allow(clippy::many_single_char_names)]
    fn a_shift_range_holds_only_siblings_of_the_clicked_row() {
        let document = Document::new(1);
        let ids_: Vec<NodeId> = (0..4).map(|i| rect(&document, f64::from(i))).collect();
        let layer = document.new_layer(None);
        let inner = rect(&document, 9.0);
        into(&document, &[inner], layer);
        let list = rows(&document, &Expansion::default(), &HashSet::new());
        assert_eq!(
            list.sibling_range(ids_[3], ids_[1]),
            vec![ids_[1], ids_[2], ids_[3]]
        );
        assert_eq!(list.sibling_range(inner, ids_[1]), vec![ids_[1]]);
    }

    #[test]
    fn a_window_is_a_slice_and_an_empty_document_has_no_rows() {
        let document = Document::new(1);
        let list = rows(&document, &Expansion::default(), &HashSet::new());
        assert!(list.is_empty());
        for i in 0..100 {
            rect(&document, f64::from(i));
        }
        let list = rows(&document, &Expansion::default(), &HashSet::new());
        assert_eq!(list.len(), 100);
        assert_eq!(list.window(90, 60).len(), 10);
        assert_eq!(list.window(500, 60).len(), 0);
    }
}
