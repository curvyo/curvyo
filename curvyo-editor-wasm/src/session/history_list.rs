//! `Session`'s History list as the panel draws it: step rows, branch header rows and the "N more
//! branches" row, each with its slice of the lane gutter, in the Document scope or the selected
//! object's (`specs/0042-history-branches` criteria 3, 4, 8 and 9).

use std::collections::HashSet;

use curvyo_document_core::{HistoryBranch, StepRef};
use curvyo_ui_core::{
    BranchSpan, Edge, HistoryItem, LineStyle, ListLayout, RowLook, Segment, branch_header_text,
    branch_lane_name, more_branches_text,
};
use serde_json::{Value, json};

use super::Session;

/// What the maker folded and unfolded. View state: kept for the session, never saved.
#[derive(Debug, Default)]
pub(super) struct ListState {
    /// The branches whose header the maker pressed an odd number of times.
    toggled: HashSet<StepRef>,
    /// Whether the "more" row is unfolded.
    more_open: bool,
}

/// The list of one scope as it shows now.
struct Shown {
    branches: Vec<HistoryBranch>,
    layout: ListLayout,
}

fn edge_word(edge: Edge) -> &'static str {
    match edge {
        Edge::Top => "top",
        Edge::Mark => "mark",
        Edge::Bottom => "bottom",
    }
}

fn style_word(style: LineStyle) -> &'static str {
    match style {
        LineStyle::Main => "main",
        LineStyle::MainDotted => "dotted",
        LineStyle::MainDashed => "dashed",
        LineStyle::Branch => "branch",
    }
}

/// The gutter as compact arrays: `["line", lane, from, to, style]`, `["disc", lane]`,
/// `["ring", lane]`, `["fork", from_lane, lane]`.
fn gutter_json(segments: &[Segment]) -> Value {
    segments
        .iter()
        .map(|segment| match *segment {
            Segment::Line {
                lane,
                from,
                to,
                style,
            } => json!([
                "line",
                lane,
                edge_word(from),
                edge_word(to),
                style_word(style)
            ]),
            Segment::Disc { lane } => json!(["disc", lane]),
            Segment::Ring { lane } => json!(["ring", lane]),
            Segment::Fork { from_lane, lane } => json!(["fork", from_lane, lane]),
        })
        .collect()
}

impl Session {
    fn shown_list(&mut self, object: bool) -> Option<Shown> {
        self.sync_history();
        let (total, branches) = if object {
            let id = self.history_object()?;
            (
                self.document.object_history_len(id),
                self.document.object_history_branches(id),
            )
        } else {
            (
                self.document.history_len(),
                self.document.history_branches(),
            )
        };
        let spans: Vec<BranchSpan> = branches
            .iter()
            .map(|branch| BranchSpan {
                rows: branch.rows.clone(),
                fork_row: branch.fork_row,
            })
            .collect();
        let state = &self.history.list;
        let toggled: Vec<bool> = branches
            .iter()
            .map(|branch| state.toggled.contains(&branch.id))
            .collect();
        let layout = ListLayout::new(total, &spans, &toggled, state.more_open);
        Some(Shown { branches, layout })
    }

    /// The list's shape as JSON: the number of items, the lanes the gutter needs and the items
    /// that are not step rows (`[index, "header" | "more"]`), so the panel knows every item's
    /// height without reading them.
    pub fn history_layout_json(&mut self, object: bool) -> String {
        let Some(shown) = self.shown_list(object) else {
            return json!({"total": 0, "lanes": 0, "special": []}).to_string();
        };
        let special: Vec<Value> = shown
            .layout
            .items()
            .iter()
            .enumerate()
            .filter_map(|(index, item)| match item {
                HistoryItem::Row(_) => None,
                HistoryItem::Header(_) => Some(json!([index, "header"])),
                HistoryItem::More { .. } => Some(json!([index, "more"])),
            })
            .collect();
        json!({
            "total": shown.layout.items().len(),
            "lanes": shown.layout.lane_count(),
            "special": special,
        })
        .to_string()
    }

    /// `count` items of the list from the `start`-th, top to bottom, as a JSON array. A step row
    /// is the row of `history_rows_json` (or of the Object scope) with `"type": "row"`, its gutter
    /// and, on a branch, its lane name; a header row and the "more" row carry their text and
    /// whether they are unfolded.
    pub fn history_items_json(&mut self, object: bool, start: usize, count: usize) -> String {
        let Some(shown) = self.shown_list(object) else {
            return "[]".to_owned();
        };
        let items: Vec<(usize, HistoryItem)> = shown
            .layout
            .items()
            .iter()
            .copied()
            .enumerate()
            .skip(start)
            .take(count)
            .collect();
        let rows = self.rows_of_items(object, &items);
        let out: Vec<Value> = items
            .iter()
            .map(|(index, item)| match *item {
                HistoryItem::Row(row) => {
                    let value = rows
                        .iter()
                        .find(|(r, _)| *r == row)
                        .map_or(Value::Null, |(_, value)| value.clone());
                    self.row_item(&shown, *index, row, value)
                }
                HistoryItem::Header(b) => {
                    let branch = &shown.branches[b];
                    json!({
                        "type": "header",
                        "key": format!("{}@{}", branch.id.peer(), branch.id.counter()),
                        "text": branch_header_text(self.fork_name(branch).as_deref(), branch.rows.len()),
                        "expanded": shown.layout.is_open(b),
                        "gutter": gutter_json(&shown.layout.gutter(*index, RowLook::default())),
                    })
                }
                HistoryItem::More { count } => json!({
                    "type": "more",
                    "key": "more",
                    "text": more_branches_text(count),
                    "expanded": shown.layout.more_open(),
                    "gutter": gutter_json(&shown.layout.gutter(*index, RowLook::default())),
                }),
            })
            .collect();
        Value::Array(out).to_string()
    }

    /// A step row as an item: the row's JSON with its type, its gutter and, on a branch, the lane
    /// name a screen reader hears (criterion 9).
    fn row_item(&self, shown: &Shown, index: usize, row: usize, mut value: Value) -> Value {
        let look = RowLook {
            undone: value["status"] != "applied",
            inherited: value["from"].as_str().is_some_and(|from| !from.is_empty()),
        };
        value["type"] = json!("row");
        value["gutter"] = gutter_json(&shown.layout.gutter(index, look));
        if let Some(b) = shown.layout.branch_of_row(row) {
            let branch = &shown.branches[b];
            let of = branch.rows.len();
            let step = of - branch.rows.iter().position(|r| *r == row).unwrap_or(0);
            let fork = self.fork_name(branch);
            value["lane"] = json!(branch_lane_name(fork.as_deref(), step, of));
        }
        value
    }

    /// The shown id of a branch's fork point.
    fn fork_name(&self, branch: &HistoryBranch) -> Option<String> {
        branch
            .fork
            .and_then(|fork| self.document.history_shown_id(fork))
    }

    /// The item at `index` of the list.
    pub(super) fn history_list_item(&mut self, object: bool, index: usize) -> Option<HistoryItem> {
        self.shown_list(object)?.layout.items().get(index).copied()
    }

    /// A press or Enter on the header row or the "more" row at `index`: folds or unfolds it.
    /// Returns whether the list changed.
    pub fn toggle_history_item(&mut self, object: bool, index: usize) -> bool {
        let Some(shown) = self.shown_list(object) else {
            return false;
        };
        match shown.layout.items().get(index) {
            Some(HistoryItem::Header(b)) => {
                let id = shown.branches[*b].id;
                let toggled = &mut self.history.list.toggled;
                if !toggled.remove(&id) {
                    toggled.insert(id);
                }
                true
            }
            Some(HistoryItem::More { .. }) => {
                self.history.list.more_open = !self.history.list.more_open;
                true
            }
            _ => false,
        }
    }

    /// The JSON of the step rows among `items`, read in runs of consecutive rows.
    fn rows_of_items(
        &mut self,
        object: bool,
        items: &[(usize, HistoryItem)],
    ) -> Vec<(usize, Value)> {
        let mut wanted: Vec<usize> = items
            .iter()
            .filter_map(|(_, item)| match item {
                HistoryItem::Row(row) => Some(*row),
                _ => None,
            })
            .collect();
        wanted.sort_unstable();
        let mut out = Vec::with_capacity(wanted.len());
        let mut at = 0;
        while at < wanted.len() {
            let first = wanted[at];
            let mut last = first;
            while at + 1 < wanted.len() && wanted[at + 1] == last + 1 {
                at += 1;
                last = wanted[at];
            }
            at += 1;
            let values = if object {
                self.object_history_row_values(first, last - first + 1)
            } else {
                self.history_row_values(first, last - first + 1)
            };
            out.extend((first..=last).zip(values));
        }
        out
    }
}
