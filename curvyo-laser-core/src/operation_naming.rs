//! The name of a new operation, `<Kind> <N>` (criterion 2), or of a new test grid, `Test grid <N>`
//! (`specs/0028` criterion 4).

use crate::operation_kind::OperationKind;

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
    use super::*;

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
