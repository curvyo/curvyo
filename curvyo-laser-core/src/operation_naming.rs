//! Which operation a kind press fills: the last one of its kind (ADR 0056 §3, `specs/0111`
//! criteria 3 to 6) or a new one named `<Kind> <N>` (criterion 2), and the name of a new test grid,
//! `Test grid <N>` (`specs/0028` criterion 4).

use curvyo_document_core::OperationSnapshot;

use crate::operation_kind::{KindRead, OperationKind};
use crate::operation_options::read_options;
use crate::operation_settings::{SettingsRead, read_settings};

/// The last operation of `kind` (`specs/0111` "Words"): the position in the stored list of the
/// operation of that kind lowest in the list that is not Unknown, has no damaged settings or
/// options and is no test grid row. `None` when there is none, so a press creates one.
#[must_use]
pub fn last_of_kind(operations: &[OperationSnapshot], kind: OperationKind) -> Option<usize> {
    operations.iter().rposition(|op| {
        KindRead::of(op.kind.as_deref()) == KindRead::Known(kind)
            && !op.grid
            && !matches!(
                read_settings(op.settings.as_ref(), kind),
                SettingsRead::Damaged(_)
            )
            && read_options(op.options.as_ref(), kind).is_ok()
    })
}

/// `<Kind> <N>` with N the lowest number from 1 that no name in `taken` (the page's operation
/// names) uses for that kind.
#[must_use]
pub fn next_operation_name<'a>(
    kind: OperationKind,
    taken: impl IntoIterator<Item = &'a str>,
) -> String {
    next_numbered(kind.label(), taken)
}

/// `Test grid <N>` with N the lowest number from 1 that no name in `taken` (the page's operation
/// names, grid rows included) uses.
#[must_use]
pub fn next_test_grid_name<'a>(taken: impl IntoIterator<Item = &'a str>) -> String {
    next_numbered("Test grid", taken)
}

fn next_numbered<'a>(word: &str, taken: impl IntoIterator<Item = &'a str>) -> String {
    let prefix = format!("{word} ");
    let mut used: Vec<u64> = taken
        .into_iter()
        .filter_map(|name| name.strip_prefix(&prefix)?.parse().ok())
        .collect();
    used.sort_unstable();
    used.dedup();
    let mut n = 1;
    for number in used {
        if number == n {
            n += 1;
        } else if number > n {
            break;
        }
    }
    format!("{prefix}{n}")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::operation_nearest::tests::op;

    #[test]
    fn the_last_operation_of_a_kind_is_the_lowest_usable_one() {
        let engrave = |n| op(n, Some("engrave"), true, &[]);
        let ops = [engrave(0), op(1, Some("cut"), true, &[]), engrave(2)];
        assert_eq!(last_of_kind(&ops, OperationKind::Engrave), Some(2));
        assert_eq!(last_of_kind(&ops, OperationKind::Cut), Some(1));
        assert_eq!(last_of_kind(&ops, OperationKind::Score), None);
        let mut grid = engrave(3);
        grid.grid = true;
        let mut damaged = engrave(4);
        damaged.settings = Some(json!(7));
        let mut bad_options = engrave(5);
        bad_options.options = Some(json!({"overscan_pct": 51}));
        let unknown = op(6, None, true, &[]);
        let only_unusable = [grid, damaged, bad_options, unknown];
        assert_eq!(
            last_of_kind(&only_unusable, OperationKind::Engrave),
            None,
            "a grid row, damaged settings or options, an Unknown operation are never reused"
        );
        let mut with_first = vec![engrave(0)];
        with_first.extend(only_unusable);
        assert_eq!(last_of_kind(&with_first, OperationKind::Engrave), Some(0));
    }

    #[test]
    fn a_test_grid_is_numbered_by_the_grid_names_only() {
        assert_eq!(next_test_grid_name([]), "Test grid 1");
        assert_eq!(
            next_test_grid_name(["Test grid 1", "Cut 2", "Test grid 3"]),
            "Test grid 2"
        );
    }

    #[test]
    fn the_first_cut_is_cut_1_and_gaps_are_filled_from_the_bottom() {
        assert_eq!(next_operation_name(OperationKind::Cut, []), "Cut 1");
        assert_eq!(
            next_operation_name(OperationKind::Cut, ["Cut 1", "Cut 3", "Score 2"]),
            "Cut 2"
        );
        assert_eq!(
            next_operation_name(OperationKind::Score, ["Cut 1", "Score 1", "Score 2"]),
            "Score 3"
        );
        assert_eq!(
            next_operation_name(OperationKind::Engrave, ["Engrave + Outline 1"]),
            "Engrave 1",
            "another kind's name does not take the number"
        );
        assert_eq!(
            next_operation_name(
                OperationKind::EngraveOutline,
                ["Outer", "Engrave + Outline 1"]
            ),
            "Engrave + Outline 2"
        );
        assert_eq!(
            next_operation_name(OperationKind::CutOut, ["Cut-out 0", "Cut-out x"]),
            "Cut-out 1"
        );
    }
}
