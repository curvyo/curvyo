//! The stored form of a laser entry's pick sets: their JSON shape, read with the damage rules of
//! ADR 0093 decision 5, and written back in canonical order.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::material::{MachineTie, PickKind, SetKind};
use crate::material_record::{Set, SetSource, TestRecord};
use crate::material_record_file::{
    NumbersJson, RecordFileError, SetJson, damaged, numbers_json, read_numbers, read_source,
    source_json,
};

/// The pick sets of one laser entry, per kind in the order cut, score, engrave, image.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PicksJson {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cut: Option<PickSetsJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    score: Option<PickSetsJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    engrave: Option<PickSetsJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image: Option<PickSetsJson>,
}

/// The pick sets of one kind; `best` is not a key, because Best is the entry in `sets`.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PickSetsJson {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    second_best: Option<SetJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fastest: Option<SetJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deepest: Option<SetJson>,
}

impl PickSetsJson {
    fn get(&mut self, pick: PickKind) -> Option<SetJson> {
        match pick {
            PickKind::SecondBest => self.second_best.take(),
            PickKind::Fastest => self.fastest.take(),
            PickKind::Deepest => self.deepest.take(),
            PickKind::Best => None,
        }
    }

    fn is_empty(&self) -> bool {
        self.second_best.is_none() && self.fastest.is_none() && self.deepest.is_none()
    }
}

/// Whether a set of `kind` sourced from the test `id` fits: a test that is not in the record is a
/// deleted test (valid, ADR 0093 decision 4), one that is must have the laser and the kind.
pub(crate) fn source_fits(
    tests: &[TestRecord],
    id: crate::test_id::TestId,
    machine: &MachineTie,
    kind: SetKind,
) -> bool {
    tests
        .iter()
        .filter(|t| t.id == id)
        .all(|t| t.machine.id == machine.id && t.recipe.kind() == kind)
}

/// The pick sets of a laser entry of a record of format `version`.
pub(crate) fn read_picks(
    picks: Option<PicksJson>,
    machine: &MachineTie,
    tests: &[TestRecord],
    version: u64,
) -> Result<BTreeMap<(SetKind, PickKind), Set>, RecordFileError> {
    let mut out = BTreeMap::new();
    let Some(picks) = picks else { return Ok(out) };
    if version < 5 {
        return Err(damaged(format!(
            "A format-{version} record holds pick sets."
        )));
    }
    let PicksJson {
        cut,
        score,
        engrave,
        image,
    } = picks;
    let kinds = [
        (SetKind::Cut, cut),
        (SetKind::Score, score),
        (SetKind::Engrave, engrave),
        (SetKind::Image, image),
    ];
    let mut any = false;
    for (kind, sets) in kinds {
        let Some(mut sets) = sets else { continue };
        any = true;
        if sets.is_empty() {
            return Err(damaged(format!(
                "The {} picks of {} are empty.",
                kind.label(),
                machine.name
            )));
        }
        for pick in PickKind::OTHERS {
            let Some(set) = sets.get(pick) else { continue };
            let numbers = read_numbers(
                kind,
                &NumbersJson {
                    power_pct: set.power_pct,
                    speed_mm_min: set.speed_mm_min,
                    passes: set.passes,
                    interval_mm: set.interval_mm,
                },
            )?;
            let source = read_source(&set.source)?;
            let SetSource::Test(id) = source else {
                return Err(damaged(format!(
                    "The {} {} pick of {} has no test as its source.",
                    kind.label(),
                    pick.label(),
                    machine.name
                )));
            };
            if !source_fits(tests, id, machine, kind) {
                return Err(damaged(format!(
                    "The {} {} pick of {} names no fitting test.",
                    kind.label(),
                    pick.label(),
                    machine.name
                )));
            }
            out.insert((kind, pick), Set { numbers, source });
        }
    }
    if !any {
        return Err(damaged(format!("The picks of {} are empty.", machine.name)));
    }
    Ok(out)
}

/// The stored picks of a laser entry, `None` when it has none.
pub(crate) fn picks_json(picks: &BTreeMap<(SetKind, PickKind), Set>) -> Option<PicksJson> {
    let of_kind = |kind: SetKind| {
        let set = |pick: PickKind| {
            picks.get(&(kind, pick)).map(|set| {
                let n = numbers_json(&set.numbers);
                SetJson {
                    power_pct: n.power_pct,
                    speed_mm_min: n.speed_mm_min,
                    passes: n.passes,
                    interval_mm: n.interval_mm,
                    source: source_json(&set.source),
                }
            })
        };
        let sets = PickSetsJson {
            second_best: set(PickKind::SecondBest),
            fastest: set(PickKind::Fastest),
            deepest: set(PickKind::Deepest),
        };
        (!sets.is_empty()).then_some(sets)
    };
    let json = PicksJson {
        cut: of_kind(SetKind::Cut),
        score: of_kind(SetKind::Score),
        engrave: of_kind(SetKind::Engrave),
        image: of_kind(SetKind::Image),
    };
    (!picks.is_empty()).then_some(json)
}
