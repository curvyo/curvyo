//! The words of undo and redo: the touched-objects summary of a row and the one-line notice a key
//! press leaves (`specs/0020-undo-redo` criteria 1, 2, 4, 19, 24 and the UX notes, "Notices").

use curvyo_document_core::{Author, Direction, ObjectRestore, ObjectShape, RestoreReport, Role};

use crate::history_names::operation_name;

/// The hint of an undo or redo while the Pen holds an unfinished path (`specs/0020-undo-redo`
/// criterion 6e).
pub const FINISH_PEN_FIRST: &str = "Finish the path (Enter) or cancel it (Esc) first.";

/// The shapes a summary counts, from the objects a step touched with what it did to each: the
/// objects as the maker saw them change. A step that deleted objects and created others (a Boolean,
/// Combine, Break apart) counts the deleted ones, the two squares that were selected and not the
/// one result; a document command counts none (`is_document_level_label`).
#[must_use]
pub fn summary_shapes(label: &str, touched: &[(ObjectShape, Role)]) -> Vec<ObjectShape> {
    if curvyo_document_core::is_document_level_label(label) {
        return Vec::new();
    }
    let replaced = touched.iter().any(|(_, role)| *role == Role::Deleted)
        && touched.iter().any(|(_, role)| *role == Role::Created);
    let shapes: Vec<ObjectShape> = touched
        .iter()
        .filter(|(_, role)| !(replaced && *role == Role::Created))
        .map(|(shape, _)| *shape)
        .collect();
    // The group node of a Group or Ungroup is not one of the objects the maker moved: they are
    // the members. A step that touched only groups (a deleted group) counts the groups.
    if shapes.iter().any(|shape| *shape != ObjectShape::Group) {
        shapes
            .into_iter()
            .filter(|shape| *shape != ObjectShape::Group)
            .collect()
    } else {
        shapes
    }
}

/// "1 rectangle", "3 paths", "5 objects": the summary of the objects a step touched. Objects of
/// one shape are counted by that shape, a mix as "objects". `None` for a step that touched no
/// object.
#[must_use]
pub fn objects_summary(shapes: &[ObjectShape]) -> Option<String> {
    let first = *shapes.first()?;
    let count = shapes.len();
    if shapes.iter().all(|shape| *shape == first) {
        let (one, many) = match first {
            ObjectShape::Path => ("path", "paths"),
            ObjectShape::Sketch => ("sketch", "sketches"),
            ObjectShape::Rectangle => ("rectangle", "rectangles"),
            ObjectShape::Ellipse => ("ellipse", "ellipses"),
            ObjectShape::Polygon => ("polygon", "polygons"),
            ObjectShape::Star => ("star", "stars"),
            ObjectShape::Group => ("group", "groups"),
            ObjectShape::Clone => ("clone", "clones"),
            ObjectShape::Text => ("text", "texts"),
            ObjectShape::Image => ("image", "images"),
        };
        Some(format!("{count} {}", if count == 1 { one } else { many }))
    } else {
        Some(format!("{count} objects"))
    }
}

/// The notice for a press with nothing on the stack.
#[must_use]
pub const fn nothing_notice(direction: Direction) -> &'static str {
    match direction {
        Direction::Undo => "Nothing to undo.",
        Direction::Redo => "Nothing to redo.",
    }
}

/// The notice of Ctrl+U and Ctrl+Shift+U with none or several objects selected
/// (`specs/0041-object-history` criterion 5).
pub const SELECT_ONE_OBJECT: &str = "Select one object.";

/// The hint of Ctrl+U and Ctrl+Shift+U with a tool other than Select and Node.
pub const OBJECT_TOOL_NOTICE: &str = "Object undo needs the Select or Node tool.";

/// The notice of an object undo on a copy that has no step left it can take back, when its earlier
/// steps are the original's.
pub const NOTHING_FROM_ORIGINAL: &str =
    "Nothing to undo for this object: its earlier steps only moved or reshaped the original.";

/// The notice for an object undo or redo with nothing to act on (criteria 11 and 14).
#[must_use]
pub const fn object_nothing_notice(direction: Direction) -> &'static str {
    match direction {
        Direction::Undo => "Nothing to undo for this object.",
        Direction::Redo => "Nothing to redo for this object.",
    }
}

/// The notice for an object undo or redo that ran (`specs/0041-object-history` UX notes,
/// "Object undo and redo"): "Undid: Resize.", "Undid: Union. 2 other objects came back too.",
/// "Undid: Resize. 1 field kept later changes.", "Undid a step by someone else: Resize (a1b2c3d).",
/// "Undid: Draw rectangle. The object is gone. Ctrl+Z brings it back.".
#[must_use]
pub fn object_restore_notice(restore: &ObjectRestore) -> String {
    let text = object_restore_text(restore);
    match restore.passed_over {
        0 => text,
        1 if restore.report.wrote => {
            format!("{text} Skipped 1 step that only moved or reshaped the original.")
        }
        n if restore.report.wrote => {
            format!("{text} Skipped {n} steps that only moved or reshaped the original.")
        }
        _ => text,
    }
}

fn object_restore_text(restore: &ObjectRestore) -> String {
    let name = operation_name(&restore.target.label);
    let report = &restore.report;
    if !report.wrote && restore.target.inherited_from.is_some() {
        return NOTHING_FROM_ORIGINAL.to_owned();
    }
    if !report.wrote {
        return restore_notice(name, report, None, false);
    }
    let (done, others) = match report.direction {
        Direction::Undo => ("Undid", "came back too"),
        Direction::Redo => ("Redid", "changed too"),
    };
    if restore.target.author == Author::Earlier {
        return format!(
            "{done} a step by someone else: {name} ({}).",
            restore.target.shown_id
        );
    }
    let head = format!("{done}: {name}.");
    if restore.whole_step {
        let count = report.objects.len().saturating_sub(1);
        return match count {
            0 => head,
            1 => format!("{head} 1 other object {others}."),
            n => format!("{head} {n} other objects {others}."),
        };
    }
    if restore.object_gone {
        return format!("{head} The object is gone. Ctrl+Z brings it back.");
    }
    match report.objects.iter().map(|o| o.kept).sum::<usize>() {
        0 => head,
        1 => format!("{head} 1 field kept later changes."),
        n => format!("{head} {n} fields kept later changes."),
    }
}

/// The notice for a press that ran: "Undid: Move (3 paths)." and its variants when parts were
/// skipped. `off_screen` replaces the full stop of the plain notice by ", off screen.".
#[must_use]
pub fn restore_notice(
    name: &str,
    report: &RestoreReport,
    summary: Option<&str>,
    off_screen: bool,
) -> String {
    let (done, verb) = match report.direction {
        Direction::Undo => ("Undid", "undo"),
        Direction::Redo => ("Redid", "redo"),
    };
    if !report.wrote {
        return if report.created_object_kept() && report.direction == Direction::Undo {
            "Not undone: someone else changed this object after you drew it.".to_owned()
        } else {
            format!(
                "Could not {verb} {name}: everything it changed was changed or deleted by someone else since."
            )
        };
    }
    let skipped = report.skipped();
    if skipped > 0 {
        return format!(
            "{done}: {name}. {skipped} of {} objects kept later changes by someone else.",
            report.total()
        );
    }
    let objects = summary.map(|text| format!(" ({text})")).unwrap_or_default();
    let tail = if off_screen { ", off screen." } else { "." };
    format!("{done}: {name}{objects}{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{
        Document, NodeId, ObjectReport, ObjectResult, Point, RectBounds, Role, SkipReason,
    };

    fn node_ids(count: usize) -> Vec<NodeId> {
        let document = Document::new(1);
        (0..count)
            .map(|n| {
                let x = f64::from(u32::try_from(n).unwrap()) * 20.0;
                document.create_rect(RectBounds::from_corners(
                    Point::new(x, 0.0),
                    Point::new(x + 10.0, 10.0),
                ))
            })
            .collect()
    }

    fn report(direction: Direction, results: &[ObjectResult], wrote: bool) -> RestoreReport {
        let ids = node_ids(results.len());
        let objects = results
            .iter()
            .zip(ids)
            .map(|(result, id)| ObjectReport {
                id,
                shape: ObjectShape::Path,
                role: Role::Changed,
                result: *result,
                kept: 0,
            })
            .collect();
        RestoreReport {
            direction,
            objects,
            root: None,
            wrote,
            step: None,
        }
    }

    #[test]
    fn a_boolean_counts_its_operands_and_a_resize_of_the_document_counts_none() {
        use ObjectShape::{Path, Rectangle};
        let union = [
            (Rectangle, Role::Deleted),
            (Rectangle, Role::Deleted),
            (Path, Role::Created),
        ];
        assert_eq!(
            summary_shapes("boolean_union", &union),
            [Rectangle, Rectangle]
        );
        let draw = [(Rectangle, Role::Created)];
        assert_eq!(summary_shapes("create_rect", &draw), [Rectangle]);
        let moved = [(Rectangle, Role::Changed), (Path, Role::Changed)];
        assert_eq!(summary_shapes("resize_document", &moved), []);
        assert_eq!(summary_shapes("translate_objects", &moved).len(), 2);
    }

    #[test]
    fn summaries_count_by_shape_and_fall_back_to_objects() {
        use ObjectShape::{Path, Rectangle};
        assert_eq!(objects_summary(&[]), None);
        assert_eq!(
            objects_summary(&[Rectangle]).as_deref(),
            Some("1 rectangle")
        );
        assert_eq!(objects_summary(&[Path; 3]).as_deref(), Some("3 paths"));
        assert_eq!(
            objects_summary(&[Path, Rectangle]).as_deref(),
            Some("2 objects")
        );
    }

    #[test]
    fn the_notices_read_as_the_specification_words_them() {
        let ok = report(Direction::Undo, &[ObjectResult::Restored; 3], true);
        assert_eq!(
            restore_notice("Move", &ok, Some("3 paths"), false),
            "Undid: Move (3 paths)."
        );
        assert_eq!(
            restore_notice("Delete", &ok, Some("2 rectangles"), true),
            "Undid: Delete (2 rectangles), off screen."
        );
        let redo = report(Direction::Redo, &[ObjectResult::Restored], true);
        assert_eq!(restore_notice("Move", &redo, None, false), "Redid: Move.");
        let partly = report(
            Direction::Undo,
            &[
                ObjectResult::Restored,
                ObjectResult::Restored,
                ObjectResult::Skipped(SkipReason::ChangedLater),
            ],
            true,
        );
        assert_eq!(
            restore_notice("Move", &partly, Some("3 paths"), false),
            "Undid: Move. 1 of 3 objects kept later changes by someone else."
        );
        let none = report(
            Direction::Undo,
            &[ObjectResult::Skipped(SkipReason::DeletedLater)],
            false,
        );
        assert_eq!(
            restore_notice("Move", &none, None, false),
            "Could not undo Move: everything it changed was changed or deleted by someone else since."
        );
        assert_eq!(nothing_notice(Direction::Undo), "Nothing to undo.");
        assert_eq!(nothing_notice(Direction::Redo), "Nothing to redo.");
    }

    fn object_restore(
        label: &str,
        direction: Direction,
        author: Author,
        results: &[(ObjectResult, usize)],
        gone: bool,
        whole: bool,
    ) -> ObjectRestore {
        let ids = node_ids(results.len());
        let objects = results
            .iter()
            .zip(ids)
            .map(|((result, kept), id)| ObjectReport {
                id,
                shape: ObjectShape::Path,
                role: Role::Changed,
                result: *result,
                kept: *kept,
            })
            .collect();
        ObjectRestore {
            target: curvyo_document_core::ObjectTarget {
                step: curvyo_document_core::StepRef::new(1, 3),
                shown_id: "a1b2c3d".to_owned(),
                label: label.to_owned(),
                author,
                time: curvyo_document_core::Timestamp::UNKNOWN,
                inherited_from: None,
            },
            report: RestoreReport {
                direction,
                objects,
                root: None,
                wrote: true,
                step: None,
            },
            object_gone: gone,
            whole_step: whole,
            passed_over: 0,
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn the_object_notices_read_as_the_specification_words_them() {
        let ok = ObjectResult::Restored;
        let plain = object_restore(
            "set_rect_bounds",
            Direction::Undo,
            Author::You,
            &[(ok, 0)],
            false,
            false,
        );
        assert_eq!(object_restore_notice(&plain), "Undid: Resize.");
        let redo = object_restore(
            "set_rect_bounds",
            Direction::Redo,
            Author::You,
            &[(ok, 0)],
            false,
            false,
        );
        assert_eq!(object_restore_notice(&redo), "Redid: Resize.");
        let union = object_restore(
            "boolean_union",
            Direction::Undo,
            Author::You,
            &[(ok, 0); 3],
            false,
            true,
        );
        assert_eq!(
            object_restore_notice(&union),
            "Undid: Union. 2 other objects came back too."
        );
        let kept = object_restore(
            "set_rect_bounds",
            Direction::Undo,
            Author::You,
            &[(ObjectResult::Partial, 1)],
            false,
            false,
        );
        assert_eq!(
            object_restore_notice(&kept),
            "Undid: Resize. 1 field kept later changes."
        );
        let kept_many = object_restore(
            "set_rect_bounds",
            Direction::Undo,
            Author::You,
            &[(ObjectResult::Partial, 3)],
            false,
            false,
        );
        assert_eq!(
            object_restore_notice(&kept_many),
            "Undid: Resize. 3 fields kept later changes."
        );
        let other = object_restore(
            "set_rect_bounds",
            Direction::Undo,
            Author::Earlier,
            &[(ok, 0)],
            false,
            false,
        );
        assert_eq!(
            object_restore_notice(&other),
            "Undid a step by someone else: Resize (a1b2c3d)."
        );
        let gone = object_restore(
            "create_rect",
            Direction::Undo,
            Author::You,
            &[(ok, 0)],
            true,
            false,
        );
        assert_eq!(
            object_restore_notice(&gone),
            "Undid: Draw rectangle. The object is gone. Ctrl+Z brings it back."
        );
        let mut passed = gone.clone();
        passed.passed_over = 2;
        passed.object_gone = false;
        assert_eq!(
            object_restore_notice(&passed),
            "Undid: Draw rectangle. Skipped 2 steps that only moved or reshaped the original."
        );
        let mut inherited = plain.clone();
        inherited.report.wrote = false;
        inherited.target.inherited_from = Some(plain.report.objects[0].id);
        assert_eq!(
            object_restore_notice(&inherited),
            "Nothing to undo for this object: its earlier steps only moved or reshaped the original."
        );
        assert_eq!(
            object_nothing_notice(Direction::Undo),
            "Nothing to undo for this object."
        );
        assert_eq!(
            object_nothing_notice(Direction::Redo),
            "Nothing to redo for this object."
        );
        assert_eq!(SELECT_ONE_OBJECT, "Select one object.");
    }

    #[test]
    fn a_created_object_a_peer_changed_keeps_its_own_notice() {
        let mut kept = report(
            Direction::Undo,
            &[ObjectResult::Skipped(SkipReason::ChangedLater)],
            false,
        );
        kept.objects[0].role = Role::Created;
        assert_eq!(
            restore_notice("Draw rectangle", &kept, Some("1 rectangle"), false),
            "Not undone: someone else changed this object after you drew it."
        );
    }
}
