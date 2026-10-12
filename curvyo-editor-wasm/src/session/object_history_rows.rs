//! `Session`'s Object scope as the History tab reads it: the subject, the buttons' steps and the rows
//! of the selected object's timeline as JSON (`specs/0041-object-history` criteria 1 to 3, 16).

use curvyo_document_core::{
    Author, NodeId, ObjectRow, ObjectRowClass, ObjectStatus, ObjectTarget, Role, StepKind,
};
use curvyo_ui_core::{
    known_operation_name, object_kind_name, object_row_name, object_row_summary, operation_name,
};
use serde_json::{Value, json};

use super::Session;

fn author_word(author: Author) -> &'static str {
    match author {
        Author::You => "you",
        Author::Earlier => "earlier",
    }
}

impl Session {
    /// The subject of the Object scope as one JSON object: the kind and ids of the selected object,
    /// the steps its buttons would act on, and what a wipe would hide. Empty for none or several
    /// selected objects.
    pub fn object_history_info_json(&mut self) -> String {
        self.sync_history();
        let Some(id) = self.history_object() else {
            return String::new();
        };
        let Some(object) = self.document.object(id) else {
            return String::new();
        };
        let targets = self.document.object_history_targets(id);
        let target = |target: Option<ObjectTarget>| -> Value {
            target.map_or(Value::Null, |target| {
                json!({
                    "id": target.shown_id,
                    "name": operation_name(&target.label),
                    "author": author_word(target.author),
                    "time": target.time.as_secs(),
                })
            })
        };
        json!({
            "key": id.full_id(),
            "kind": object_kind_name(&object),
            "id": id.short_id(),
            "fullId": id.full_id(),
            "undo": target(targets.undo),
            "redo": target(targets.redo),
            "wipeCount": self.document.object_wipe_count(id),
            "total": self.document.object_history_len(id),
        })
        .to_string()
    }

    /// The number of rows in the Object scope for the selected object.
    pub fn object_history_len(&mut self) -> usize {
        self.sync_history();
        self.history_object()
            .map_or(0, |id| self.document.object_history_len(id))
    }

    /// `count` rows of the selected object's timeline from the `start`-th newest, as a JSON array.
    pub fn object_history_rows_json(&mut self, start: usize, count: usize) -> String {
        self.sync_history();
        Value::Array(self.object_history_row_values(start, count)).to_string()
    }

    /// `count` rows of the selected object's timeline from the `start`-th newest; none without
    /// exactly one selected object.
    pub(super) fn object_history_row_values(&mut self, start: usize, count: usize) -> Vec<Value> {
        self.sync_history();
        let Some(id) = self.history_object() else {
            return Vec::new();
        };
        self.document
            .object_history_page(id, start, count)
            .iter()
            .map(object_row_json)
            .collect()
    }
}

fn kind_word(kind: StepKind) -> &'static str {
    match kind {
        StepKind::Create => "create",
        StepKind::Delete => "delete",
        StepKind::Replace => "replace",
        StepKind::Change => "change",
        StepKind::Document => "document",
    }
}

fn role_word(role: Role) -> &'static str {
    match role {
        Role::Created => "created",
        Role::Deleted => "deleted",
        Role::Changed => "changed",
        Role::Moved => "moved",
    }
}

fn object_row_json(row: &ObjectRow) -> Value {
    let (class, reverts) = match &row.class {
        ObjectRowClass::Step => ("step", Value::Null),
        ObjectRowClass::Reverts { shown_id, redo, .. } => (
            if *redo { "redoes" } else { "undoes" },
            Value::String(shown_id.clone()),
        ),
        ObjectRowClass::CloneStart { source } => ("clone-start", Value::String(source.short_id())),
        ObjectRowClass::Floor => ("floor", Value::Null),
    };
    let (status, undone_by) = match row.status {
        ObjectStatus::Applied => ("applied", ""),
        ObjectStatus::Undone => ("undone", ""),
        ObjectStatus::OnBranch => ("branch", ""),
        ObjectStatus::UndoneBy(author) => ("undone-by", author_word(author)),
    };
    json!({
        "id": row.shown_id,
        "step": format!("{}@{}", row.step.peer(), row.step.counter()),
        "label": row.label,
        "name": object_row_name(row),
        "known": known_operation_name(&row.label).is_some() || !matches!(row.class, ObjectRowClass::Step),
        "author": author_word(row.author),
        "time": row.time.as_secs(),
        "class": class,
        "target": reverts,
        "kind": kind_word(row.kind),
        "role": role_word(row.role),
        "summary": object_row_summary(row),
        "status": status,
        "undoneBy": undone_by,
        "from": row.inherited_from.map(NodeId::short_id).unwrap_or_default(),
        "sources": row.sources.iter().map(|id| id.full_id()).collect::<Vec<_>>(),
        "objects": [],
    })
}
