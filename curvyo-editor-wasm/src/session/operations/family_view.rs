//! The Operations tab of a plotter, embroidery or CNC page in the laser's view shape (ADR 0098
//! decision 3): the family's rows, Select-bar kinds and unfolded body as `OperationsView`, so the
//! laser components draw every family.

use curvyo_document_core::NodeId;
use curvyo_laser_core::operations::OperationList;
use serde_json::Value;

use super::super::Session;
use super::super::output::{NumbersLineView, RowRunView};
use super::super::output_page::{FamilyBodyView, FamilyPage};
use super::ActiveOperations;
use super::items::ItemLists;
use super::kind_press::KindButtonView;
use super::view_json::{KindView, OperationsView, RowView};

/// The Select-bar buttons of a family in bar order (ADR 0098 decision 3, UX unify 5).
const fn bar_kinds(family: &FamilyPage) -> &'static [KindView] {
    match family {
        FamilyPage::Plotter(_) => &[KindView::Cut, KindView::Draw],
        FamilyPage::Embroidery(_) => &[
            KindView::RunningStitch,
            KindView::TatamiFill,
            KindView::Satin,
        ],
        FamilyPage::Cnc(_) => &[KindView::ProfileOutside, KindView::ProfileInside],
    }
}

/// The view kind of a family row as its family view names it.
fn row_kind(family: &FamilyPage, row: &Value) -> KindView {
    let key = |name: &str| row.get(name).and_then(Value::as_str).unwrap_or_default();
    match (family, key("kind"), key("side")) {
        (FamilyPage::Plotter(_), "cut", _) => KindView::Cut,
        (FamilyPage::Plotter(_), "draw", _) => KindView::Draw,
        (FamilyPage::Embroidery(_), "running", _) => KindView::RunningStitch,
        (FamilyPage::Embroidery(_), "tatami", _) => KindView::TatamiFill,
        (FamilyPage::Embroidery(_), "satin", _) => KindView::Satin,
        (FamilyPage::Cnc(_), _, "outside") => KindView::ProfileOutside,
        (FamilyPage::Cnc(_), _, "inside") => KindView::ProfileInside,
        _ => KindView::Unknown,
    }
}

/// The description of a family kind button from the family view's `kinds` entry.
fn description(view: &Value, family: &FamilyPage, kind: KindView) -> String {
    let wanted = match kind {
        KindView::Cut => "cut",
        KindView::Draw => "draw",
        KindView::RunningStitch => "running",
        KindView::TatamiFill => "tatami",
        KindView::Satin => "satin",
        KindView::ProfileOutside => "outside",
        KindView::ProfileInside => "inside",
        _ => "",
    };
    let key = if matches!(family, FamilyPage::Cnc(_)) {
        "side"
    } else {
        "kind"
    };
    view["kinds"]
        .as_array()
        .and_then(|kinds| kinds.iter().find(|k| k[key] == wanted))
        .and_then(|k| k["description"].as_str())
        .unwrap_or_default()
        .to_owned()
}

/// The unfolded body of a family row.
fn body(family: &FamilyPage, row: &Value, view: &Value) -> Option<FamilyBodyView> {
    let fields = row.get("numbers").filter(|n| !n.is_null())?.clone();
    let kind = row["kind"].as_str().unwrap_or_default().to_owned();
    Some(match family {
        FamilyPage::Plotter(_) => FamilyBodyView::Plotter { kind, fields },
        FamilyPage::Embroidery(_) => {
            let mut fields = fields;
            if let Some(map) = fields.as_object_mut() {
                map.insert("thread".to_owned(), row["thread"].clone());
            }
            FamilyBodyView::Embroidery { kind, fields }
        }
        FamilyPage::Cnc(_) => FamilyBodyView::Cnc {
            fields,
            tools: view["tools"].clone(),
        },
    })
}

impl Session {
    /// The operations of the active output page of any family, for the commands a row of the
    /// shared Operations tab sends (rename, delete, Show, Output, reorder, ticks, unfold):
    /// Cuts last only on a laser page.
    pub(in crate::session) fn active_row_operations(&self) -> Option<ActiveOperations> {
        if let Some(active) = self.active_operations() {
            return Some(active);
        }
        let family = self.active_family()?;
        let manufacturing = family.manufacturing();
        Some(ActiveOperations {
            manufacturing,
            operations: self.document.operations(manufacturing).ok()?,
            cuts_last: false,
        })
    }

    /// Whether `id` is an operation of the active output page of any family.
    pub(in crate::session) fn is_active_row_operation(&self, id: NodeId) -> bool {
        self.active_row_operations()
            .is_some_and(|active| active.operations.iter().any(|op| op.id == id))
    }

    /// The Operations tab of the family page `family` in the laser's shape.
    pub(in crate::session) fn family_operations_view(&self, family: &FamilyPage) -> OperationsView {
        let view = self.family_view(family).unwrap_or(Value::Null);
        let manufacturing = family.manufacturing();
        let operations = self.document.operations(manufacturing).unwrap_or_default();
        let list = OperationList::new(&operations, false);
        let leaves = self.document.manufacturing(manufacturing).map_or_else(
            || std::sync::Arc::from(Vec::new()),
            |m| self.output_leaves(&m),
        );
        let selection = self.selection_targets();
        let unfolded = self.operations.tools.unfolded();
        let rows = view["operations"].as_array().cloned().unwrap_or_default();
        let rows = operations
            .iter()
            .enumerate()
            .map(|(at, op)| {
                let row = rows.get(at).cloned().unwrap_or(Value::Null);
                let (tick, group_in_part) =
                    super::view_json::row_tick(self, &selection, &list, at, &leaves);
                let digit = u32::try_from(at + 1).unwrap_or(u32::MAX);
                let time = row["time"].as_str().map(str::to_owned);
                let run = (op.output && time.is_some()).then(|| RowRunView {
                    digit,
                    time: time.clone().unwrap_or_default(),
                });
                let line3 = row["line3"].as_str().unwrap_or_default().to_owned();
                RowView {
                    id: op.id,
                    name: op.name.clone(),
                    kind: row_kind(family, &row),
                    section: None,
                    show: op.show,
                    output: op.output,
                    margin_mm: op.margin.as_mm(),
                    items: super::view_json::item_count(&op.members, &leaves),
                    tick,
                    group_in_part,
                    unfolded: unfolded == Some(op.id),
                    settings: None,
                    item_tabs: None,
                    item_path: None,
                    source: None,
                    accessible_name: run.as_ref().map_or_else(
                        || op.name.clone(),
                        |run| format!("{}, run {}, {}", op.name, run.digit, run.time),
                    ),
                    run,
                    numbers: Some(vec![NumbersLineView {
                        text: line3,
                        typed: Vec::new(),
                    }]),
                    thread: row["thread"].as_str().map(str::to_owned),
                    body: (row["unfolds"] == true)
                        .then(|| body(family, &row, &view))
                        .flatten(),
                }
            })
            .collect();
        let select_bar_kinds = self.family_bar_shown().then(|| {
            bar_kinds(family)
                .iter()
                .map(|&kind| KindButtonView {
                    kind,
                    description: description(&view, family, kind),
                })
                .collect()
        });
        OperationsView {
            // Plotter Cuts last is not built yet: the list order is the run order on every
            // family page (UX unify 6, Proposal).
            cuts_last: None,
            rows,
            items: ItemLists::default(),
            view_pressed: self.operations_view_pressed(manufacturing)
                && !self.toolpath_view_pressed(manufacturing),
            select_bar_kinds,
            entered_group: None,
            pick: Vec::new(),
            tabs_tool: None,
            tab_chip: None,
        }
    }

    /// Whether the family kind buttons show in the Select bar: something is selected in the
    /// edit scope (`specs/0111` criterion 1).
    fn family_bar_shown(&self) -> bool {
        !self.selection.ids().is_empty() && self.edit_scope() == curvyo_ui_core::Scope::Edit
    }
}
