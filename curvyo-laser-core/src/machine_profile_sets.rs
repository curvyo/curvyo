//! Reads and writes the named fixture sets of a profile record (record format 5, ADR 0060 §1,
//! §2, amended 2026-11-16: ids and locks).

use curvyo_document_core::FixtureSetRecord;
use serde::{Deserialize, Serialize};

use crate::machine_profile_fixtures::{FixtureJson, read_fixture, valid_name, write_fixture};
use crate::record_envelope::is_record_id;

/// The most sets a profile record holds.
pub const MAX_RECORD_SETS: usize = 16;
/// The most fixtures in all sets of a record together.
pub const MAX_RECORD_FIXTURES: usize = 64;
/// The most slots in all sets of a record together.
pub const MAX_RECORD_SLOTS: usize = 200;
/// The most fixture jobs in all sets of a record together.
pub const MAX_RECORD_JOBS: usize = 200;
/// The most part tokens of all jobs in all sets of a record together.
pub const MAX_RECORD_TARGETS: usize = 400;

/// One set as the record stores it: its id, name, Lock and fixture entries, never On this page.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SetJson {
    id: String,
    name: String,
    locked: bool,
    fixtures: Vec<FixtureJson>,
}

/// The sets as written, in order: the page's sets only (ADR 0060 §1, §6); a fixture of an
/// unknown kind is left out of its set.
pub(crate) fn write_sets(sets: &[FixtureSetRecord]) -> Vec<SetJson> {
    sets.iter()
        .map(|set| SetJson {
            id: set.id.clone(),
            name: set.name.clone(),
            locked: set.locked,
            fixtures: set.fixtures.iter().filter_map(write_fixture).collect(),
        })
        .collect()
}

/// Whether every id of `sets` (sets, fixtures and jobs) is a lowercase UUID v4 and no id is given
/// twice in the record (ADR 0060 §1 amended 2026-11-16).
fn ids_hold(sets: &[SetJson]) -> bool {
    let mut ids: Vec<&str> = Vec::new();
    for set in sets {
        ids.push(&set.id);
        for fixture in &set.fixtures {
            ids.extend(fixture.ids());
        }
    }
    let all_valid = ids.iter().all(|id| is_record_id(id));
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    all_valid && ids.len() == before
}

/// The sets a record holds; `None` when any of it is damage: more than 16 sets, more than 64
/// fixtures, 200 slots, 200 jobs or 400 targets in all sets, a set name that is empty or over 40
/// characters, an id that is no UUID v4 or is given twice, or a damaged entry (ADR 0060 §1, §2).
pub(crate) fn read_sets(sets: Vec<SetJson>) -> Option<Vec<FixtureSetRecord>> {
    let entries = || sets.iter().flat_map(|set| &set.fixtures);
    let fixtures = entries().count();
    let slots: usize = entries().map(FixtureJson::slot_count).sum();
    let jobs: usize = entries().map(FixtureJson::job_count).sum();
    let targets: usize = entries().map(FixtureJson::target_count).sum();
    if sets.len() > MAX_RECORD_SETS
        || fixtures > MAX_RECORD_FIXTURES
        || slots > MAX_RECORD_SLOTS
        || jobs > MAX_RECORD_JOBS
        || targets > MAX_RECORD_TARGETS
        || sets.iter().any(|set| !valid_name(&set.name))
        || !ids_hold(&sets)
    {
        return None;
    }
    sets.into_iter()
        .map(|set| {
            Some(FixtureSetRecord {
                id: set.id,
                name: set.name,
                locked: set.locked,
                fixtures: set
                    .fixtures
                    .into_iter()
                    .map(read_fixture)
                    .collect::<Option<Vec<_>>>()?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sets(value: serde_json::Value) -> Option<Vec<FixtureSetRecord>> {
        read_sets(serde_json::from_value(value).unwrap())
    }

    /// A lowercase UUID v4 made of `n`.
    fn id(n: usize) -> String {
        format!("00000000-0000-4000-8000-{n:012}")
    }

    fn clamp(n: usize) -> serde_json::Value {
        json!({
            "id": id(1000 + n), "kind": "clamp", "name": format!("Clamp {n}"), "locked": false,
            "params": {"width_mm": 40.0, "height_mm": 40.0}, "slots": [], "jobs": []
        })
    }

    fn set_json(n: usize, name: &str, fixtures: &[serde_json::Value]) -> serde_json::Value {
        json!({"id": id(n), "name": name, "locked": false, "fixtures": fixtures})
    }

    #[test]
    fn sets_read_in_order_with_their_fixtures_and_an_empty_set_is_kept() {
        let read = sets(json!([
            set_json(1, "Bed", &[clamp(1), clamp(2)]),
            {"id": id(2), "name": "Coaster jig", "locked": true, "fixtures": []},
        ]))
        .unwrap();
        assert_eq!(read.len(), 2);
        assert_eq!(read[0].name, "Bed");
        assert_eq!(read[0].id, id(1));
        assert_eq!(read[0].fixtures[1].name, "Clamp 2");
        assert!(read[1].locked && !read[0].locked);
        assert_eq!(read[1].fixtures, []);
        let text = serde_json::to_value(write_sets(&read)).unwrap();
        assert_eq!(sets(text).unwrap(), read, "round trip");
    }

    #[test]
    fn a_bad_set_name_or_a_damaged_entry_damages_the_sets() {
        assert_eq!(sets(json!([set_json(1, "", &[])])), None);
        assert_eq!(sets(json!([set_json(1, &"n".repeat(41), &[])])), None);
        let mut on_page = set_json(1, "Bed", &[]);
        on_page["on_page"] = json!(true);
        assert!(
            serde_json::from_value::<Vec<SetJson>>(json!([on_page])).is_err(),
            "a record never holds On this page"
        );
        let mut broken = clamp(1);
        broken["params"]["width_mm"] = json!("wide");
        assert_eq!(sets(json!([set_json(1, "Bed", &[broken])])), None);
    }

    #[test]
    fn a_missing_repeated_or_malformed_id_a_missing_lock_or_an_active_key_damages_the_sets() {
        let mut no_id = set_json(1, "Bed", &[]);
        no_id.as_object_mut().unwrap().remove("id");
        assert!(serde_json::from_value::<Vec<SetJson>>(json!([no_id])).is_err());
        let mut no_lock = clamp(1);
        no_lock.as_object_mut().unwrap().remove("locked");
        assert!(
            serde_json::from_value::<Vec<SetJson>>(json!([set_json(1, "B", &[no_lock])])).is_err()
        );
        let mut active = clamp(1);
        active["active"] = json!(true);
        assert!(
            serde_json::from_value::<Vec<SetJson>>(json!([set_json(1, "B", &[active])])).is_err(),
            "On the bed is a choice of each page"
        );
        let twice = json!([set_json(1, "A", &[clamp(1)]), set_json(2, "B", &[clamp(1)])]);
        assert_eq!(sets(twice), None, "one fixture id in two sets");
        let set_and_fixture = json!([set_json(1001, "A", &[clamp(1)])]);
        assert_eq!(
            sets(set_and_fixture),
            None,
            "a set and a fixture share an id"
        );
        let mut upper = clamp(1);
        upper["id"] = json!("00000000-0000-4000-8000-00000000000A");
        assert_eq!(
            sets(json!([set_json(1, "A", &[upper])])),
            None,
            "not lowercase"
        );
    }

    #[test]
    fn more_than_sixteen_sets_or_sixty_four_fixtures_damage_the_sets() {
        let empty: Vec<serde_json::Value> = (0..=MAX_RECORD_SETS)
            .map(|n| set_json(n, &format!("Set {n}"), &[]))
            .collect();
        assert_eq!(sets(json!(empty[..MAX_RECORD_SETS])).unwrap().len(), 16);
        assert_eq!(sets(json!(empty)), None, "17 sets");
        let many: Vec<serde_json::Value> = (0..=MAX_RECORD_FIXTURES).map(clamp).collect();
        let split = json!([
            set_json(1, "A", &many[..32]),
            set_json(2, "B", &many[32..MAX_RECORD_FIXTURES]),
        ]);
        assert_eq!(sets(split).unwrap().len(), 2, "64 fixtures in all");
        let over = json!([set_json(1, "A", &many[..32]), set_json(2, "B", &many[32..])]);
        assert_eq!(sets(over), None, "65 fixtures in all");
    }
}
