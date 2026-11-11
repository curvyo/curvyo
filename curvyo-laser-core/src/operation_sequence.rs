//! The run order of the laser's runs: the fixture block, then the operations under the Cuts last
//! switch (`0101` criteria 5 to 7, ADR 0046 §5; `0103` criteria 13 to 17).

use curvyo_document_core::ManufacturingSnapshot;

use crate::fixture_part::FixturePart;
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

/// A fixture job as the run rule needs it (`0103` criterion 13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureJobRead {
    /// Its kind; Unknown runs nothing.
    pub kind: KindRead,
    /// Its Output switch.
    pub output: bool,
    /// Whether its fixture is Active (`0102` criterion 5).
    pub fixture_active: bool,
    /// Its ticked readable parts, in Parts-list order.
    pub parts: Vec<FixturePart>,
}

impl FixtureJobRead {
    /// Whether the job runs: its fixture is Active, its Output is on, its kind is known and it
    /// has a ticked part. Show and the fixture's eye do not matter.
    #[must_use]
    pub fn runs(&self) -> bool {
        self.fixture_active && self.output && self.kind.known().is_some() && !self.parts.is_empty()
    }
}

/// The fixture block (Frage 176 A: it runs before the work): the positions of the running jobs in
/// `jobs` (fixture creation order, then job order), in run order; with Cuts last on Marking jobs
/// before Cutting jobs, each keeping its order, else list order. Its length is the summary's
/// count of running fixture jobs (criterion 17).
#[must_use]
pub fn fixture_block(jobs: &[FixtureJobRead], cuts_last: bool) -> Vec<usize> {
    let running: Vec<usize> = (0..jobs.len()).filter(|&at| jobs[at].runs()).collect();
    let kinds: Vec<KindRead> = running.iter().map(|&at| jobs[at].kind).collect();
    run_sequence(&kinds, cuts_last)
        .into_iter()
        .map(|at| running[at])
        .collect()
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

    fn job(kind: KindRead, output: bool, active: bool, parts: usize) -> FixtureJobRead {
        FixtureJobRead {
            kind,
            output,
            fixture_active: active,
            parts: (0..parts).map(FixturePart::Slot).collect(),
        }
    }

    #[test]
    fn a_fixture_job_runs_only_active_with_output_a_known_kind_and_a_part() {
        assert!(job(k(Cut), true, true, 1).runs());
        assert!(!job(k(Cut), false, true, 1).runs(), "Output off");
        assert!(!job(k(Cut), true, false, 1).runs(), "not on the bed");
        assert!(!job(UNKNOWN, true, true, 1).runs(), "Unknown kind");
        assert!(!job(k(Cut), true, true, 0).runs(), "no tick");
    }

    #[test]
    fn the_fixture_block_puts_marking_before_cutting_with_cuts_last_on() {
        let jobs = [
            job(k(Cut), true, true, 1),
            job(k(Engrave), false, true, 1),
            job(k(CutOut), true, true, 2),
            job(k(EngraveOutline), true, true, 1),
            job(k(Score), true, false, 1),
            job(UNKNOWN, true, true, 1),
        ];
        assert_eq!(fixture_block(&jobs, true), [3, 0, 2]);
        assert_eq!(fixture_block(&jobs, false), [0, 2, 3]);
        assert_eq!(
            fixture_block(&jobs, true).len(),
            3,
            "jobs are counted, not parts"
        );
        assert_eq!(fixture_block(&[], true), Vec::<usize>::new());
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
