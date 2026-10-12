//! The words of a row in the Object scope of the History tab (`specs/0041-object-history` UX notes,
//! "Rows that exist only here"): its name and the summary on its second line.

use curvyo_document_core::{ObjectRow, ObjectRowClass, Role, StepKind};

use crate::history_names::operation_name;

/// The name of a row: "Resize", "Undo Resize", "Duplicate of 3f9a1c", "Created by Union",
/// "History wiped".
#[must_use]
pub fn object_row_name(row: &ObjectRow) -> String {
    let name = operation_name(&row.label);
    match &row.class {
        ObjectRowClass::Floor => "History wiped".to_owned(),
        ObjectRowClass::Reverts { label, redo, .. } => {
            let verb = if *redo { "Redo" } else { "Undo" };
            format!("{verb} {}", operation_name(label))
        }
        ObjectRowClass::CloneStart { source } => match &row.version {
            // `specs/0042-history-branches` criterion 23.
            Some(version) => format!(
                "Clone of {} at version {}",
                source.short_id(),
                version.shown_id
            ),
            None => format!("Duplicate of {}", source.short_id()),
        },
        ObjectRowClass::Step if row.version.is_some() => {
            let id = row.version.as_ref().map_or("", |v| v.shown_id.as_str());
            format!("{name} {id}")
        }
        ObjectRowClass::Step => match (row.kind, row.role) {
            (StepKind::Replace, Role::Created) => format!("Created by {name}"),
            (StepKind::Replace, Role::Deleted) => format!("Deleted by {name}"),
            _ => name.to_owned(),
        },
    }
}

/// The summary of the second line of a row, before the author and the time: "with 2 other
/// objects", "from 3 objects", "reverts a1b2c3d", "Earlier steps hidden". Empty for a step that
/// touched this object alone.
#[must_use]
pub fn object_row_summary(row: &ObjectRow) -> String {
    match &row.class {
        ObjectRowClass::Floor => "Earlier steps hidden".to_owned(),
        ObjectRowClass::Reverts { shown_id, redo, .. } => {
            let verb = if *redo { "restores" } else { "reverts" };
            format!("{verb} {shown_id}")
        }
        ObjectRowClass::CloneStart { .. } => String::new(),
        // A go to names the step whose version it went to: "Resize" (criterion 11's row).
        ObjectRowClass::Step if row.version.is_some() => row
            .version
            .as_ref()
            .map_or_else(String::new, |v| operation_name(&v.label).to_owned()),
        ObjectRowClass::Step => {
            if !row.sources.is_empty() {
                return format!("from {}", plural(row.sources.len(), "object"));
            }
            match row.others {
                0 => String::new(),
                n => format!("with {}", plural_other(n)),
            }
        }
    }
}

fn plural(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

fn plural_other(count: usize) -> String {
    format!("{count} other object{}", if count == 1 { "" } else { "s" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{Author, Document, NodeId, ObjectStatus, Point, RectBounds, Vec2};

    fn row(document: &Document, id: NodeId) -> ObjectRow {
        document.object_history_page(id, 0, 1).remove(0)
    }

    fn rect(document: &Document, x: f64) -> NodeId {
        document.create_rect(RectBounds::from_corners(
            Point::new(x, 0.0),
            Point::new(x + 10.0, 10.0),
        ))
    }

    #[test]
    fn a_step_that_touched_other_objects_says_so() {
        let document = Document::new(1);
        let ids: Vec<NodeId> = (0..3)
            .map(|n| rect(&document, f64::from(n) * 20.0))
            .collect();
        document
            .translate_objects(&ids, Vec2::new(1.0, 0.0))
            .unwrap();
        let moved = row(&document, ids[0]);
        assert_eq!(object_row_name(&moved), "Move");
        assert_eq!(object_row_summary(&moved), "with 2 other objects");
        document
            .translate_objects(&ids[..2], Vec2::new(1.0, 0.0))
            .unwrap();
        assert_eq!(
            object_row_summary(&row(&document, ids[0])),
            "with 1 other object"
        );
        assert_eq!(
            object_row_summary(&row(&document, ids[2]).clone()),
            "with 2 other objects"
        );
    }

    #[test]
    fn object_undo_rows_name_the_step_they_reverted() {
        let document = Document::new(1);
        let a = rect(&document, 0.0);
        document
            .translate_objects(&[a], Vec2::new(1.0, 0.0))
            .unwrap();
        let done = document.undo_object_step(a).unwrap().unwrap();
        let undo = row(&document, a);
        assert_eq!(object_row_name(&undo), "Undo Move");
        assert_eq!(
            object_row_summary(&undo),
            format!("reverts {}", done.target.shown_id)
        );
        assert_eq!(undo.status, ObjectStatus::Applied);
        assert_eq!(undo.author, Author::You);
    }

    #[test]
    fn a_clone_and_a_floor_have_their_own_names() {
        let document = Document::new(1);
        let a = rect(&document, 0.0);
        let clone = document
            .duplicate_objects(
                &[curvyo_document_core::CopySource {
                    id: a,
                    anchor_ids: vec![],
                }],
                Vec2::new(5.0, 5.0),
            )
            .unwrap()[0];
        assert_eq!(
            object_row_name(&row(&document, clone)),
            format!("Duplicate of {}", a.short_id())
        );
        document.wipe_object_history(clone).unwrap();
        let floor = row(&document, clone);
        assert_eq!(object_row_name(&floor), "History wiped");
        assert_eq!(object_row_summary(&floor), "Earlier steps hidden");
    }
}
