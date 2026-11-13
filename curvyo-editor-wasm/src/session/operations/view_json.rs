//! The Operations tab's view as JSON (`specs/0101-laser-operations` criteria 1, 5, 8, 17; ADR 0046
//! §5; `adrs.md` decision 6), with the kind buttons, the entered group, the unfolded row and the
//! Operations view toggle of `specs/0111-laser-operation-tools` (architecture finding 3). The only
//! place that builds this JSON; the shared fixtures in
//! `curvyo-editor-wasm/tests/fixtures/operations_view/` pin its shape.

use std::collections::HashSet;

use curvyo_document_core::NodeId;
use curvyo_laser_core::operations::{
    GroupTick, KindRead, Leaf, OperationKind, OperationList, Section, Summary, summarize,
};
use serde::Serialize;

use super::super::Session;
use super::assign::{Selection, target_tick};
use super::index::IndexedLeaf;
use super::items::{ItemLists, item_lists};
use super::kind_press::KindButtonView;
use super::settings_view::{
    PickView, SettingsView, SourceView, pick_views, settings_view, source_view,
};

/// The whole Operations tab of the active laser page.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OperationsView {
    /// The Cuts last switch.
    pub cuts_last: bool,
    /// The rows in run order (the list order, sections as the switch shows them).
    pub rows: Vec<RowView>,
    /// The summary counts of criterion 17. Nothing is counted as missing (Frage 169 A).
    pub summary: SummaryView,
    /// The items behind each count, for the count buttons (criterion 17).
    pub items: ItemLists,
    /// Whether the page's Operations view toggle is pressed (`specs/0111` criterion 15).
    #[serde(rename = "operations_view")]
    pub view_pressed: bool,
    /// The kind buttons of the Select bar in bar order; `None` while they are hidden
    /// (`specs/0111` criterion 1).
    pub select_bar_kinds: Option<Vec<KindButtonView>>,
    /// The read-only entered group's crumb (`specs/0111` criterion 11); `None` at the root.
    pub entered_group: Option<EnteredGroupView>,
    /// The materials of this machine the Material pick lists (`specs/0028` criterion 25), in the
    /// order of the Materials section; damaged and newer records are not listed.
    pub pick: Vec<PickView>,
}

/// One operation's row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RowView {
    /// The operation's node id, as JSON text.
    pub id: NodeId,
    /// Its name.
    pub name: String,
    /// Its kind; `unknown` for a kind this build does not know (criterion 26).
    pub kind: KindView,
    /// Its section with Cuts last on; `None` for an unknown operation.
    pub section: Option<SectionView>,
    /// The Show switch.
    pub show: bool,
    /// The Output switch.
    pub output: bool,
    /// The Cut-out margin in mm.
    pub margin_mm: f64,
    /// How many items it holds; a legacy clone-member key is not counted (`specs/0111`
    /// criterion 14, Frage 212 A).
    pub items: usize,
    /// How the current selection stands in it: none, some or all of the selection (ticks); a
    /// group by its leaves (`specs/0111` criterion 10).
    pub tick: TickView,
    /// "Group 3" or "2 groups" while selected groups run in it in part (`specs/0111` criterion 10,
    /// UX notes §4.1 and T3); `None` otherwise.
    pub group_in_part: Option<String>,
    /// Whether the row is the unfolded one (`specs/0111` criterion 17).
    pub unfolded: bool,
    /// Its Settings block (`specs/0028` criteria 24 to 26); `None` for a grid row and an
    /// unknown operation.
    pub settings: Option<SettingsView>,
    /// The source line of a test grid's derived row (`specs/0112` criterion 12); `None` for any
    /// other row.
    pub source: Option<SourceView>,
}

/// The kinds of the rows; the five known ones and the unknown one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KindView {
    /// Cut.
    Cut,
    /// Score.
    Score,
    /// Engrave.
    Engrave,
    /// Engrave + Outline.
    EngraveOutline,
    /// Cut-out.
    CutOut,
    /// A kind this build does not know, or an unreadable record.
    Unknown,
}

/// The two sections of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionView {
    /// The marking section.
    Marking,
    /// The cutting section.
    Cutting,
}

/// A row's tick over the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TickView {
    /// No selected item is in it, or nothing is selected.
    None,
    /// Some of the selected items are in it (the mixed tick).
    Some,
    /// All of the selected items are in it.
    All,
}

/// The summary counts (ADR 0046 §5): the first three are disjoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SummaryView {
    /// Operations that run at least one item.
    pub operations: usize,
    /// Items that hold an assignment and run, hidden or not.
    pub running: usize,
    /// Items with no assignment at any level.
    pub not_assigned: usize,
    /// Items that run while hidden (a part of `running`).
    pub runs_while_hidden: usize,
    /// Items in an operation with Output off (may overlap `running`).
    pub output_off: usize,
}

/// The crumb of a read-only entered group: "<page> › <group> · Read-only · Esc: leave".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnteredGroupView {
    /// The page name, the root crumb.
    pub page: String,
    /// The entered groups' names, outermost first.
    pub groups: Vec<String>,
}

/// How many items an operation's `members` hold (`specs/0111` criteria 10, 14): one per node
/// key, where a node inside a group that holds a key of the same operation is part of that group's
/// item and is not counted again; a legacy clone-member path key is not counted.
fn item_count(members: &[Vec<NodeId>], leaves: &[IndexedLeaf]) -> usize {
    let nodes: HashSet<NodeId> = members
        .iter()
        .filter_map(|key| match key.as_slice() {
            [node] => Some(*node),
            _ => None,
        })
        .collect();
    if nodes.is_empty() {
        return 0;
    }
    let mut inside = HashSet::new();
    for leaf in leaves {
        // The chain runs from the leaf outward: every member key below an outer member key is
        // inside that one.
        let Some(outer) = leaf
            .chain
            .iter()
            .rposition(|key| matches!(key.as_slice(), [node] if nodes.contains(node)))
        else {
            continue;
        };
        for key in &leaf.chain[..outer] {
            if let [node] = key.as_slice()
                && nodes.contains(node)
            {
                inside.insert(*node);
            }
        }
    }
    nodes.len() - inside.len()
}

/// The view kind of a stored kind.
pub(super) fn kind_view(kind: KindRead) -> KindView {
    match kind.known() {
        Some(OperationKind::Cut) => KindView::Cut,
        Some(OperationKind::Score) => KindView::Score,
        Some(OperationKind::Engrave) => KindView::Engrave,
        Some(OperationKind::EngraveOutline) => KindView::EngraveOutline,
        Some(OperationKind::CutOut) => KindView::CutOut,
        None => KindView::Unknown,
    }
}

fn section_view(section: Section) -> SectionView {
    match section {
        Section::Marking => SectionView::Marking,
        Section::Cutting => SectionView::Cutting,
    }
}

impl Session {
    /// The Operations tab of the active laser page, `None` on another page.
    #[must_use]
    pub fn operations_view(&self) -> Option<OperationsView> {
        let active = self.active_operations()?;
        let manufacturing = self.active_manufacturing()?;
        let list = OperationList::new(&active.operations, active.cuts_last);
        let leaves = self.output_leaves(&manufacturing);
        let summary = summarize(
            &list,
            leaves.iter().map(|leaf| Leaf {
                chain: &leaf.chain,
                hidden: leaf.hidden,
                shape: leaf.shape,
            }),
        );
        let selection = self.selection_targets();
        let pick = pick_views(self);
        let unfolded = self.operations.tools.unfolded();
        let mut sources = super::settings_view::SourceCache::new();
        let page = super::settings_view::PageFacts::of(self);
        let rows = list
            .sequence()
            .iter()
            .map(|&at| {
                let op = &active.operations[at];
                let kind = KindRead::of(op.kind.as_deref());
                let (tick, group_in_part) = row_tick(self, &selection, &list, at, &leaves);
                RowView {
                    id: op.id,
                    name: op.name.clone(),
                    kind: kind_view(kind),
                    section: kind.known().map(|k| section_view(k.section())),
                    show: op.show,
                    output: op.output,
                    margin_mm: op.margin.as_mm(),
                    items: item_count(&op.members, &leaves),
                    tick,
                    group_in_part,
                    unfolded: unfolded == Some(op.id),
                    settings: settings_view(self, op, &pick, &page),
                    source: source_view(self, op, &mut sources),
                }
            })
            .collect();
        Some(OperationsView {
            cuts_last: active.cuts_last,
            rows,
            summary: summary_view(&summary),
            items: item_lists(&list, &leaves),
            view_pressed: self.operations_view_pressed(manufacturing.id),
            select_bar_kinds: self.select_bar_kinds(&active),
            entered_group: self.entered_group_view(),
            pick,
        })
    }

    /// The Operations tab as JSON, `null` on another page (the host's contract, as the
    /// manufacturing view).
    #[must_use]
    pub fn operations_view_json(&self) -> String {
        serde_json::to_string(&self.operations_view()).unwrap_or_else(|_| "null".to_owned())
    }
}

/// The tick of row `at` over the selection, and the line-2 words of the groups in part.
fn row_tick(
    session: &Session,
    selection: &Selection,
    list: &OperationList,
    at: usize,
    leaves: &[IndexedLeaf],
) -> (TickView, Option<String>) {
    let mut checked = 0;
    let mut mixed: Vec<NodeId> = Vec::new();
    for &target in &selection.targets {
        match target_tick(selection, target, list, at, leaves) {
            GroupTick::Checked => checked += 1,
            GroupTick::Mixed => mixed.push(target),
            GroupTick::Unchecked => {}
        }
    }
    let tick = match (checked, mixed.len()) {
        (0, 0) => TickView::None,
        (n, 0) if n == selection.targets.len() => TickView::All,
        _ => TickView::Some,
    };
    let in_part = match mixed.as_slice() {
        [] => None,
        [group] => Some(session.group_name(*group)),
        several => Some(format!("{} groups", several.len())),
    };
    (tick, in_part)
}

impl Session {
    /// A group's name in the tab and the crumb: its own name, else "Group <short id>" as the
    /// Layers tab names it.
    fn group_name(&self, group: NodeId) -> String {
        self.document
            .node_registers(group)
            .and_then(|registers| registers.name)
            .unwrap_or_else(|| format!("Group {}", group.short_id()))
    }

    /// The crumb of the read-only entered group, `None` at the root or on another page.
    fn entered_group_view(&self) -> Option<EnteredGroupView> {
        if !self.in_read_only_group() {
            return None;
        }
        let groups: Vec<String> = self
            .context_chain()
            .into_iter()
            .map(|group| self.group_name(group))
            .collect();
        (!groups.is_empty()).then(|| EnteredGroupView {
            page: self.active_page().name,
            groups,
        })
    }
}

fn summary_view(summary: &Summary) -> SummaryView {
    SummaryView {
        operations: summary.operations,
        running: summary.running,
        not_assigned: summary.not_assigned,
        runs_while_hidden: summary.runs_while_hidden,
        output_off: summary.output_off,
    }
}
