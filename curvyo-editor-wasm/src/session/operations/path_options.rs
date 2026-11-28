//! The path options rows of an unfolded Cut or Cut-out (`specs/0139` criteria 13, 19 to 21, 23;
//! `ux-notes.md` 2, 3.2): the operation's group view, and the item part's Path options pill,
//! own fields, Start readout with Automatic and Direction pick, each edit one step.

use curvyo_document_core::{
    AUTOMATIC_START_COMMIT_LABEL, DIRECTION_COMMIT_LABEL, EDIT_OPERATION_SETTINGS_COMMIT_LABEL,
    ItemRecord, NodeId, OPERATIONS_PATH_OPTIONS_COMMIT_LABEL, OWN_PATH_OPTIONS_COMMIT_LABEL,
    RecordWrite,
};
use curvyo_laser_core::settings::{PathField, PathOptions, read_own_path};
use curvyo_laser_core::tabs::{Direction, Entries, OutlineEntry, read_entries};
use serde::Serialize;

use super::super::Session;
use super::tab_outlines::TabOutline;

/// The Path options group of an operation or the own rows of the item part (UX 2, 3.2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PathOptionsView {
    /// "none", "line" or "arc".
    pub lead_in: &'static str,
    /// The lead-in length in mm.
    pub lead_in_mm: f64,
    /// "none", "line" or "arc".
    pub lead_out: &'static str,
    /// The lead-out length in mm.
    pub lead_out_mm: f64,
    /// The overcut in mm.
    pub overcut_mm: f64,
}

impl PathOptionsView {
    /// The view of `options`.
    #[must_use]
    pub fn of(options: PathOptions) -> Self {
        Self {
            lead_in: options.lead_in.as_str(),
            lead_in_mm: options.lead_in_length.mm(),
            lead_out: options.lead_out.as_str(),
            lead_out_mm: options.lead_out_length.mm(),
            overcut_mm: options.overcut.mm(),
        }
    }
}

/// The Path options pill of the item part (UX 3.2 row 2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ItemOptionsView {
    /// Own pressed (`true`), Operation's (`false`), mixed (`None`).
    pub own: Option<bool>,
    /// With Operation's: "Lead-in Arc 1 mm · Lead-out None · Overcut 0.5 mm".
    pub summary: Option<String>,
    /// With Own: the first own item's values for the three rows.
    pub values: Option<PathOptionsView>,
}

/// The Start row of the item part (UX 3.2 row 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ItemStartView {
    /// "Automatic", "Set", "Set on some outlines".
    pub readout: String,
    /// Whether any is set (the Automatic button shows).
    pub set: bool,
}

/// The path rows of the item part of one operation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ItemPathView {
    /// The Path options pill, only when a selected item has a closed outline.
    pub options: Option<ItemOptionsView>,
    /// The Start row, only when a selected item has a closed outline.
    pub start: Option<ItemStartView>,
    /// The Direction pick, only when a selected item has an open outline.
    pub direction: Option<ItemDirectionView>,
}

/// The Direction pick of the item part (UX 3.2 row 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ItemDirectionView {
    /// `automatic`, `as_drawn` or `reversed`; `None` when the selected paths differ.
    pub value: Option<&'static str>,
}

fn direction_name(entry: Option<OutlineEntry>) -> &'static str {
    match entry {
        Some(OutlineEntry::Direction(Direction::AsDrawn)) => "as_drawn",
        Some(OutlineEntry::Direction(Direction::Reversed)) => "reversed",
        _ => "automatic",
    }
}

/// The keys of `outlines` once each, in order.
fn keys_of<'a>(outlines: impl Iterator<Item = &'a TabOutline>) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for o in outlines {
        if !keys.contains(&o.key) {
            keys.push(o.key.clone());
        }
    }
    keys
}

impl Session {
    /// The path rows of the item part of operation `op`, `None` when no selected item runs in it.
    pub(in crate::session) fn item_path_view(&self, op: NodeId) -> Option<ItemPathView> {
        let outlines = self.tab_outlines(Some(op));
        if outlines.is_empty() {
            return None;
        }
        let closed: Vec<&TabOutline> = outlines.iter().filter(|o| o.measured.closed()).collect();
        let open: Vec<&TabOutline> = outlines.iter().filter(|o| !o.measured.closed()).collect();
        let records = self
            .document
            .operation_records(op, ItemRecord::PathOwn)
            .unwrap_or_default();
        let options = (!closed.is_empty()).then(|| {
            let keys = keys_of(closed.iter().copied());
            let owns: Vec<PathOptions> = keys
                .iter()
                .filter_map(|k| records.get(k).and_then(|v| read_own_path(v).ok()))
                .collect();
            let own = match owns.len() {
                0 => Some(false),
                n if n == keys.len() => Some(true),
                _ => None,
            };
            ItemOptionsView {
                own,
                summary: (own == Some(false)).then(|| closed[0].path.summary()),
                values: (own == Some(true)).then(|| PathOptionsView::of(owns[0])),
            }
        });
        let start = (!closed.is_empty()).then(|| {
            let set = closed.iter().filter(|o| o.entry.is_some()).count();
            let readout = match (set, closed.len()) {
                (0, _) => "Automatic".to_owned(),
                (_, 1) => "Set".to_owned(),
                _ => "Set on some outlines".to_owned(),
            };
            ItemStartView {
                readout,
                set: set > 0,
            }
        });
        let direction = (!open.is_empty()).then(|| {
            let first = direction_name(open[0].entry);
            ItemDirectionView {
                value: open
                    .iter()
                    .all(|o| direction_name(o.entry) == first)
                    .then_some(first),
            }
        });
        Some(ItemPathView {
            options,
            start,
            direction,
        })
    }

    /// Writes `writes` of operation `op` in one step `label`.
    pub(in crate::session) fn write_item_records(
        &mut self,
        op: NodeId,
        writes: &[RecordWrite],
        label: &str,
    ) -> bool {
        if writes.is_empty() {
            return false;
        }
        self.recording_unguarded(|this| {
            this.document
                .write_operation_records(op, writes, label)
                .unwrap_or(false)
        })
    }

    /// The Path options pill of operation `op` for the selected items with a closed outline
    /// (criterion 21): Own copies the operation's values ("Own path options"), Operation's drops
    /// the own blocks ("Back to the operation's"). Whether it wrote.
    pub fn set_selection_path_own(&mut self, op: NodeId, own: bool) -> bool {
        let outlines = self.tab_outlines(Some(op));
        let closed = outlines.iter().filter(|o| o.measured.closed());
        let Some(path) = outlines.first().map(|o| o.path) else {
            return false;
        };
        let records = self
            .document
            .operation_records(op, ItemRecord::PathOwn)
            .unwrap_or_default();
        let writes: Vec<RecordWrite> = keys_of(closed)
            .into_iter()
            .filter(|key| records.contains_key(key) != own)
            .filter_map(|key| {
                let value = if own {
                    Some(path.to_record().as_object()?.clone())
                } else {
                    None
                };
                Some((ItemRecord::PathOwn, key, value))
            })
            .collect();
        let label = if own {
            OWN_PATH_OPTIONS_COMMIT_LABEL
        } else {
            OPERATIONS_PATH_OPTIONS_COMMIT_LABEL
        };
        self.write_item_records(op, &writes, label)
    }

    /// Types `value` into the own field `option` (`lead_in`, `lead_in_mm`, `lead_out`,
    /// `lead_out_mm`, `overcut`, as [`Session::edit_operation_option`]) of every selected item
    /// with own path options in `op`: one step "Edit settings".
    ///
    /// # Errors
    /// The chip text of a value out of range; nothing is written.
    pub fn edit_selection_path_option(
        &mut self,
        op: NodeId,
        option: &str,
        value: f64,
    ) -> Result<bool, String> {
        let field = PathField::of(option).ok_or_else(|| "Enter a number.".to_owned())?;
        let edit = field.typed(value).map_err(str::to_owned)?;
        let outlines = self.tab_outlines(Some(op));
        let records = self
            .document
            .operation_records(op, ItemRecord::PathOwn)
            .unwrap_or_default();
        let writes: Vec<RecordWrite> = keys_of(outlines.iter().filter(|o| o.measured.closed()))
            .into_iter()
            .filter_map(|key| {
                let own = read_own_path(records.get(&key)?).ok()?;
                let next = own.with(edit);
                (next != own).then(|| {
                    let value = next.to_record().as_object().cloned();
                    (ItemRecord::PathOwn, key, value)
                })
            })
            .collect();
        Ok(self.write_item_records(op, &writes, EDIT_OPERATION_SETTINGS_COMMIT_LABEL))
    }

    /// The item's stored entries of `key` in `op` (empty when none or damaged).
    pub(in crate::session) fn item_entries(&self, op: NodeId, key: &str) -> Option<Entries> {
        let records = self
            .document
            .operation_records(op, ItemRecord::Entry)
            .unwrap_or_default();
        match records.get(key) {
            None => Some(Entries::default()),
            Some(value) => read_entries(value).ok(),
        }
    }

    /// The writes that set the entry of each `(outline, entry)`, grouped by item.
    pub(in crate::session) fn entry_writes(
        &self,
        changes: &[(&TabOutline, Option<OutlineEntry>)],
    ) -> Vec<RecordWrite> {
        let mut writes: Vec<RecordWrite> = Vec::new();
        let ops_keys = {
            let mut list: Vec<(NodeId, String)> = Vec::new();
            for (o, _) in changes {
                if !list.iter().any(|(op, key)| *op == o.op && *key == o.key) {
                    list.push((o.op, o.key.clone()));
                }
            }
            list
        };
        for (op, key) in ops_keys {
            let Some(mut entries) = self.item_entries(op, &key) else {
                continue;
            };
            let before = entries.clone();
            for (o, entry) in changes.iter().filter(|(o, _)| o.op == op && o.key == key) {
                entries.set(o.reference.clone(), *entry);
            }
            if entries == before {
                continue;
            }
            let value = if entries.is_empty() {
                None
            } else {
                entries.to_value().as_object().cloned()
            };
            writes.push((ItemRecord::Entry, key, value));
        }
        writes
    }

    /// The Automatic button of operation `op` (criterion 19): every selected closed outline
    /// returns to Automatic, one step "Automatic start". Whether it wrote.
    pub fn set_selection_start_automatic(&mut self, op: NodeId) -> bool {
        let outlines = self.tab_outlines(Some(op));
        let changes: Vec<(&TabOutline, Option<OutlineEntry>)> = outlines
            .iter()
            .filter(|o| o.measured.closed() && o.entry.is_some())
            .map(|o| (o, None))
            .collect();
        let writes = self.entry_writes(&changes);
        self.write_item_records(op, &writes, AUTOMATIC_START_COMMIT_LABEL)
    }

    /// The Direction pick of operation `op` for the selected open outlines (criterion 20):
    /// `automatic`, `as_drawn` or `reversed`, one step "Direction". Whether it wrote.
    pub fn set_selection_direction(&mut self, op: NodeId, direction: &str) -> bool {
        let entry = match direction {
            "automatic" => None,
            "as_drawn" => Some(OutlineEntry::Direction(Direction::AsDrawn)),
            "reversed" => Some(OutlineEntry::Direction(Direction::Reversed)),
            _ => return false,
        };
        let outlines = self.tab_outlines(Some(op));
        let changes: Vec<(&TabOutline, Option<OutlineEntry>)> = outlines
            .iter()
            .filter(|o| !o.measured.closed())
            .map(|o| (o, entry))
            .collect();
        let writes = self.entry_writes(&changes);
        self.write_item_records(op, &writes, DIRECTION_COMMIT_LABEL)
    }
}
