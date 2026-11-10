//! The Operations tab's view as JSON (`specs/0101-laser-operations` criteria 1, 5, 8, 17; ADR 0046
//! §5; `adrs.md` decision 6). The only place that builds this JSON; the shared fixtures in
//! `curvyo-editor-wasm/tests/fixtures/operations_view/` pin its shape.

use curvyo_document_core::NodeId;
use curvyo_laser_core::operations::{
    KindRead, Leaf, OperationKind, OperationList, Section, Summary, summarize,
};
use serde::Serialize;

use super::super::Session;
use super::OperationsViewMode;
use super::items::{CloneMemberView, ItemLists, clone_member_rows, item_lists};

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
    /// The clone member rows, read-only and assignable (criterion 11).
    pub clone_members: Vec<CloneMemberView>,
    /// How the output page is drawn (criterion 18).
    pub view_mode: OperationsViewMode,
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
    /// How many items (paths) it holds.
    pub items: usize,
    /// How the current selection stands in it: none, some or all of the selection (ticks).
    pub tick: TickView,
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

fn kind_view(kind: KindRead) -> KindView {
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
        let rows = list
            .sequence()
            .iter()
            .map(|&at| {
                let op = &active.operations[at];
                let kind = KindRead::of(op.kind.as_deref());
                let inside = selection
                    .targets
                    .iter()
                    .filter(|target| op.members.contains(target))
                    .count();
                RowView {
                    id: op.id,
                    name: op.name.clone(),
                    kind: kind_view(kind),
                    section: kind.known().map(|k| section_view(k.section())),
                    show: op.show,
                    output: op.output,
                    margin_mm: op.margin.as_mm(),
                    items: op.members.len(),
                    tick: match (inside, selection.targets.len()) {
                        (0, _) => TickView::None,
                        (n, total) if n == total => TickView::All,
                        _ => TickView::Some,
                    },
                }
            })
            .collect();
        Some(OperationsView {
            cuts_last: active.cuts_last,
            rows,
            summary: summary_view(&summary),
            items: item_lists(&list, &leaves),
            clone_members: clone_member_rows(&active.operations, &leaves),
            view_mode: self.operations.view_mode,
        })
    }

    /// The Operations tab as JSON, `null` on another page (the host's contract, as the
    /// manufacturing view).
    #[must_use]
    pub fn operations_view_json(&self) -> String {
        serde_json::to_string(&self.operations_view()).unwrap_or_else(|_| "null".to_owned())
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
