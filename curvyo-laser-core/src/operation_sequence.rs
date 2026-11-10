//! The run order of an operation list under the Cuts last switch (criteria 5 to 7, ADR 0046 §5).

use curvyo_document_core::ManufacturingSnapshot;

use crate::manifest::PROCESS_ID;
use crate::operation_kind::{KindRead, Section};

/// Whether Cuts last is on for the manufacturing: its `x.curvyo.laser.cuts_last` register, on
/// when absent or not a boolean (ADR 0046 §3).
#[must_use]
pub fn cuts_last(manufacturing: &ManufacturingSnapshot) -> bool {
    manufacturing
        .process_values
        .get(&format!("x.{PROCESS_ID}.cuts_last"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

/// The run order as positions in the stored list: with Cuts last on a stable partition, the
/// Marking section (and Unknown operations, which keep their place) then the Cutting section;
/// with it off the list order. The list shows this order too, so a merge that left a cutting
/// operation above a marking one is shown in run order and nothing is rewritten.
#[must_use]
pub fn run_sequence(kinds: &[KindRead], cuts_last: bool) -> Vec<usize> {
    let all = 0..kinds.len();
    if !cuts_last {
        return all.collect();
    }
    let (marking, cutting): (Vec<usize>, Vec<usize>) =
        all.partition(|&at| kinds[at].section() == Section::Marking);
    marking.into_iter().chain(cutting).collect()
}

/// Where a new operation of section `section` goes in the stored list: the end of the list with
/// Cuts last off or for Cutting, else just after the last stored Marking operation (criteria 5,
/// 6).
#[must_use]
pub fn insert_index(kinds: &[KindRead], section: Section, cuts_last: bool) -> usize {
    if !cuts_last || section == Section::Cutting {
        return kinds.len();
    }
    kinds
        .iter()
        .rposition(|kind| kind.section() == Section::Marking)
        .map_or(0, |at| at + 1)
}

/// Whether a row at `from` of the shown list `shown` (kinds in run order) may land at `to`:
/// always with Cuts last off, else only inside its own section (criterion 5; Alt+Up stops at the
/// line).
#[must_use]
pub fn may_move(shown: &[KindRead], from: usize, to: usize, cuts_last: bool) -> bool {
    let (Some(moved), true) = (shown.get(from), to < shown.len()) else {
        return false;
    };
    if !cuts_last {
        return true;
    }
    let (low, high) = (from.min(to), from.max(to));
    shown[low..=high]
        .iter()
        .all(|kind| kind.section() == moved.section())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation_kind::OperationKind::{Cut, CutOut, Engrave, EngraveOutline, Score};

    const UNKNOWN: KindRead = KindRead::Unknown;

    fn k(kind: crate::operation_kind::OperationKind) -> KindRead {
        KindRead::Known(kind)
    }

    #[test]
    fn cuts_last_puts_cutting_after_marking_keeping_each_order() {
        let kinds = [
            k(Cut),
            k(Engrave),
            k(CutOut),
            k(Score),
            UNKNOWN,
            k(EngraveOutline),
        ];
        assert_eq!(run_sequence(&kinds, true), [1, 3, 4, 5, 0, 2]);
        assert_eq!(run_sequence(&kinds, false), [0, 1, 2, 3, 4, 5]);
        assert_eq!(run_sequence(&[], true), Vec::<usize>::new());
    }

    #[test]
    fn a_new_operation_lands_at_the_end_of_its_section() {
        let kinds = [k(Engrave), k(Score), k(Cut), k(CutOut)];
        assert_eq!(insert_index(&kinds, Section::Marking, true), 2);
        assert_eq!(insert_index(&kinds, Section::Cutting, true), 4);
        assert_eq!(insert_index(&kinds, Section::Marking, false), 4);
        assert_eq!(insert_index(&[k(Cut)], Section::Marking, true), 0);
        assert_eq!(insert_index(&[], Section::Marking, true), 0);
    }

    #[test]
    fn with_cuts_last_a_row_moves_only_inside_its_section() {
        let shown = [k(Engrave), k(Score), k(Cut), k(CutOut)];
        assert!(may_move(&shown, 1, 0, true));
        assert!(may_move(&shown, 3, 2, true));
        assert!(
            !may_move(&shown, 2, 1, true),
            "a Cut above the line is refused"
        );
        assert!(!may_move(&shown, 0, 3, true));
        assert!(
            may_move(&shown, 2, 0, false),
            "any place with Cuts last off"
        );
        assert!(!may_move(&shown, 4, 0, false));
        assert!(!may_move(&shown, 0, 4, false));
    }
}
