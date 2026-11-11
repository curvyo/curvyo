//! The parts of a fixture that a fixture job can tick (`specs/0103-fixture-jobs` criteria 8, 11,
//! 22 and 23; ADR 0052 §2, §7): the list per kind, the stored key form and the profile tokens.

use curvyo_document_core::JobTarget;

use crate::fixture_kind::FixtureKind;
use crate::operation_kind_run::LeafShape;

/// One tickable part of a fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FixturePart {
    /// A holder's board (the holder node).
    Board,
    /// One slot of a holder, by its 0-based index among the readable slots.
    Slot(usize),
    /// A clamp's outline.
    Clamp,
    /// A stop's L, both legs as one part.
    Stop,
    /// The grid's lines.
    GridLines,
    /// The grid's switched-on tapes.
    GridTapes,
    /// A screw's head circle.
    ScrewHead,
    /// A screw's thread circle.
    ScrewThread,
}

const LINES: &str = "lines";
const TAPES: &str = "tapes";
const HEAD: &str = "head";
const THREAD: &str = "thread";
const SLOT_TOKEN: &str = "slot:";

impl FixturePart {
    /// The parts of a fixture of `kind` with `slots` readable slots, in Parts-list order; none
    /// for an unknown kind.
    #[must_use]
    pub fn of_kind(kind: FixtureKind, slots: usize) -> Vec<Self> {
        match kind {
            FixtureKind::EngravedGrid => vec![Self::GridLines, Self::GridTapes],
            FixtureKind::Holder => std::iter::once(Self::Board)
                .chain((0..slots).map(Self::Slot))
                .collect(),
            FixtureKind::Clamp => vec![Self::Clamp],
            FixtureKind::Stop => vec![Self::Stop],
            FixtureKind::Screw => vec![Self::ScrewHead, Self::ScrewThread],
            FixtureKind::Unknown => Vec::new(),
        }
    }

    /// The part a stored target names on a fixture of `kind` with `slots` readable slots; `None`
    /// when it is not a part of that kind (damage, ADR 0052 §3).
    #[must_use]
    pub fn from_target(kind: FixtureKind, slots: usize, target: &JobTarget) -> Option<Self> {
        let part = match (target.slot, target.suffix.as_deref()) {
            (Some(index), None) if index < slots => Self::Slot(index),
            (None, None) if kind == FixtureKind::Holder => Self::Board,
            (None, None) if kind == FixtureKind::Clamp => Self::Clamp,
            (None, None) if kind == FixtureKind::Stop => Self::Stop,
            (None, Some(LINES)) => Self::GridLines,
            (None, Some(TAPES)) => Self::GridTapes,
            (None, Some(HEAD)) => Self::ScrewHead,
            (None, Some(THREAD)) => Self::ScrewThread,
            _ => return None,
        };
        Self::of_kind(kind, slots).contains(&part).then_some(part)
    }

    /// The part by place, as a record and a copy store it.
    #[must_use]
    pub fn target(self) -> JobTarget {
        let (slot, suffix) = match self {
            Self::Slot(index) => (Some(index), None),
            Self::Board | Self::Clamp | Self::Stop => (None, None),
            Self::GridLines => (None, Some(LINES)),
            Self::GridTapes => (None, Some(TAPES)),
            Self::ScrewHead => (None, Some(HEAD)),
            Self::ScrewThread => (None, Some(THREAD)),
        };
        JobTarget {
            slot,
            suffix: suffix.map(str::to_owned),
        }
    }

    /// The profile token (`board`, `slot:0`, `grid.lines`, ...).
    #[must_use]
    pub fn token(self) -> String {
        match self {
            Self::Board => "board".to_owned(),
            Self::Slot(index) => format!("{SLOT_TOKEN}{index}"),
            Self::Clamp => "clamp".to_owned(),
            Self::Stop => "stop".to_owned(),
            Self::GridLines => "grid.lines".to_owned(),
            Self::GridTapes => "grid.tapes".to_owned(),
            Self::ScrewHead => "screw.head".to_owned(),
            Self::ScrewThread => "screw.thread".to_owned(),
        }
    }

    /// The part a profile token names on an entry of `kind` with `slots` slots; `None` for a
    /// token that is no part of it (the record is damaged, ADR 0052 §7).
    #[must_use]
    pub fn from_token(kind: FixtureKind, slots: usize, token: &str) -> Option<Self> {
        let part = match token {
            "board" => Self::Board,
            "clamp" => Self::Clamp,
            "stop" => Self::Stop,
            "grid.lines" => Self::GridLines,
            "grid.tapes" => Self::GridTapes,
            "screw.head" => Self::ScrewHead,
            "screw.thread" => Self::ScrewThread,
            other => {
                let digits = other.strip_prefix(SLOT_TOKEN)?;
                // A canonical index only: "slot:01" and "slot:+1" are no tokens.
                if digits.is_empty()
                    || !digits.bytes().all(|b| b.is_ascii_digit())
                    || (digits.len() > 1 && digits.starts_with('0'))
                {
                    return None;
                }
                Self::Slot(digits.parse().ok()?)
            }
        };
        Self::of_kind(kind, slots).contains(&part).then_some(part)
    }

    /// The outline topology of the part, for `kind_run`: a closed outline for the board, a slot,
    /// a clamp, a stop and each screw circle; open lines for the grid's lines and tapes.
    #[must_use]
    pub const fn shape(self) -> LeafShape {
        match self {
            Self::GridLines | Self::GridTapes => LeafShape::Outlines { closed: 0, open: 1 },
            _ => LeafShape::Outlines { closed: 1, open: 0 },
        }
    }
}

/// The parts of a job's ticked targets `ticked` (the reads whose value is `true`) on a fixture
/// of `kind` with `slots` readable slots, in Parts-list order, and how many of them name no part
/// of the kind (each one damaged value, ADR 0052 §3; a value that is not `true` is counted by the
/// document).
#[must_use]
pub fn job_parts<'a>(
    kind: FixtureKind,
    slots: usize,
    ticked: impl IntoIterator<Item = &'a JobTarget>,
) -> (Vec<FixturePart>, usize) {
    let mut damaged = 0;
    let mut parts = Vec::new();
    for target in ticked {
        match FixturePart::from_target(kind, slots, target) {
            Some(part) if !parts.contains(&part) => parts.push(part),
            Some(_) => {}
            None => damaged += 1,
        }
    }
    let order = FixturePart::of_kind(kind, slots);
    parts.sort_by_key(|part| order.iter().position(|p| p == part));
    (parts, damaged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use FixtureKind::{Clamp, EngravedGrid, Holder, Screw, Stop, Unknown};

    fn target(slot: Option<usize>, suffix: Option<&str>) -> JobTarget {
        JobTarget {
            slot,
            suffix: suffix.map(str::to_owned),
        }
    }

    #[test]
    fn each_kind_lists_its_parts_in_the_parts_list_order() {
        assert_eq!(
            FixturePart::of_kind(EngravedGrid, 3),
            [FixturePart::GridLines, FixturePart::GridTapes]
        );
        assert_eq!(
            FixturePart::of_kind(Holder, 2),
            [
                FixturePart::Board,
                FixturePart::Slot(0),
                FixturePart::Slot(1)
            ]
        );
        assert_eq!(FixturePart::of_kind(Clamp, 0), [FixturePart::Clamp]);
        assert_eq!(FixturePart::of_kind(Stop, 0), [FixturePart::Stop]);
        assert_eq!(
            FixturePart::of_kind(Screw, 0),
            [FixturePart::ScrewHead, FixturePart::ScrewThread]
        );
        assert_eq!(FixturePart::of_kind(Unknown, 4), []);
    }

    #[test]
    fn a_stored_target_names_a_part_only_of_its_kind() {
        assert_eq!(
            FixturePart::from_target(Holder, 2, &target(None, None)),
            Some(FixturePart::Board)
        );
        assert_eq!(
            FixturePart::from_target(Holder, 2, &target(Some(1), None)),
            Some(FixturePart::Slot(1))
        );
        assert_eq!(
            FixturePart::from_target(Holder, 2, &target(None, Some("tapes"))),
            None,
            "a holder has no tapes"
        );
        assert_eq!(
            FixturePart::from_target(Holder, 2, &target(Some(0), Some("lines"))),
            None
        );
        assert_eq!(
            FixturePart::from_target(EngravedGrid, 0, &target(None, None)),
            None,
            "a grid has no whole part"
        );
        assert_eq!(
            FixturePart::from_target(Screw, 0, &target(None, Some("thread"))),
            Some(FixturePart::ScrewThread)
        );
        for part in FixturePart::of_kind(Holder, 3)
            .into_iter()
            .chain(FixturePart::of_kind(Screw, 0))
            .chain(FixturePart::of_kind(EngravedGrid, 0))
        {
            let kind = match part {
                FixturePart::ScrewHead | FixturePart::ScrewThread => Screw,
                FixturePart::GridLines | FixturePart::GridTapes => EngravedGrid,
                _ => Holder,
            };
            assert_eq!(
                FixturePart::from_target(kind, 3, &part.target()),
                Some(part)
            );
            assert_eq!(FixturePart::from_token(kind, 3, &part.token()), Some(part));
        }
    }

    #[test]
    fn tokens_read_only_for_their_kind_and_a_slot_in_range() {
        let tokens = [
            (Holder, "board"),
            (Clamp, "clamp"),
            (Stop, "stop"),
            (EngravedGrid, "grid.lines"),
            (EngravedGrid, "grid.tapes"),
            (Screw, "screw.head"),
            (Screw, "screw.thread"),
            (Holder, "slot:1"),
        ];
        for (kind, token) in tokens {
            let part = FixturePart::from_token(kind, 2, token).unwrap();
            assert_eq!(part.token(), token);
        }
        assert_eq!(FixturePart::from_token(Holder, 2, "grid.tapes"), None);
        assert_eq!(FixturePart::from_token(Holder, 2, "slot:2"), None);
        assert_eq!(FixturePart::from_token(Holder, 2, "slot:01"), None);
        assert_eq!(FixturePart::from_token(Holder, 2, "slot:"), None);
        assert_eq!(FixturePart::from_token(Holder, 2, "slot:-1"), None);
        assert_eq!(FixturePart::from_token(Clamp, 0, "board"), None);
        assert_eq!(FixturePart::from_token(Unknown, 0, "board"), None);
    }

    #[test]
    fn a_jobs_ticked_parts_read_in_list_order_and_a_bad_suffix_counts() {
        let targets = [
            target(Some(1), None),
            target(None, Some("tapes")),
            target(None, None),
            target(Some(1), None),
        ];
        let (parts, damaged) = job_parts(Holder, 2, &targets);
        assert_eq!(parts, [FixturePart::Board, FixturePart::Slot(1)]);
        assert_eq!(damaged, 1, "the tapes of a holder");
        assert_eq!(job_parts(Unknown, 0, &targets).0, []);
    }

    #[test]
    fn grid_parts_are_open_lines_and_the_others_closed_outlines() {
        assert_eq!(
            FixturePart::GridTapes.shape(),
            LeafShape::Outlines { closed: 0, open: 1 }
        );
        assert_eq!(
            FixturePart::ScrewHead.shape(),
            LeafShape::Outlines { closed: 1, open: 0 }
        );
    }
}
