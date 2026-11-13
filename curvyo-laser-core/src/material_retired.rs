//! Reads and writes the retired test ids (ADR 0058 §4, `specs/0112` criteria 9, 17, 25).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::record_envelope::pretty;
use crate::test_id::TestId;

/// The format of `retired-test-ids.json` this build reads and writes.
pub const RETIRED_FORMAT_VERSION: u64 = 1;
/// The most ids the file holds (Proposal).
pub const MAX_RETIRED: usize = 20_000;
/// The notice of a file that does not read; ids may then be given again.
pub const RETIRED_DAMAGED_NOTICE: &str = "Retired test ids could not be read.";

/// The ids of deleted tests, never given again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetiredIds {
    ids: BTreeSet<TestId>,
}

/// Why an id could not be retired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RetireRefused {
    /// The file holds 20,000 ids.
    #[error("The list of retired test ids is full.")]
    Full,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetiredJson {
    format_version: u64,
    retired: Vec<String>,
}

impl RetiredIds {
    /// Whether `id` is retired.
    #[must_use]
    pub fn contains(&self, id: &TestId) -> bool {
        self.ids.contains(id)
    }

    /// The retired ids, sorted.
    pub fn iter(&self) -> impl Iterator<Item = &TestId> {
        self.ids.iter()
    }

    /// The number of retired ids.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether no id is retired.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Adds `ids` (append-only: nothing is ever taken out, an undo of a delete leaves its id
    /// here); `true` when one was new, so the file is written.
    ///
    /// # Errors
    /// [`RetireRefused::Full`] when the ids would pass 20,000; nothing is added.
    pub fn retire(&mut self, ids: impl IntoIterator<Item = TestId>) -> Result<bool, RetireRefused> {
        let new: BTreeSet<TestId> = ids.into_iter().filter(|id| !self.contains(id)).collect();
        if self.ids.len() + new.len() > MAX_RETIRED {
            return Err(RetireRefused::Full);
        }
        let changed = !new.is_empty();
        self.ids.extend(new);
        Ok(changed)
    }
}

/// The retired ids of the file text, `None` for no file. A text that does not read (not JSON,
/// another format, an unknown key, a bad or repeated id, more than 20,000) reads as empty with
/// [`RETIRED_DAMAGED_NOTICE`]; it is never rewritten by a read.
#[must_use]
pub fn read_retired(text: Option<&str>) -> (RetiredIds, Option<&'static str>) {
    let Some(text) = text else {
        return (RetiredIds::default(), None);
    };
    let parsed = serde_json::from_str::<RetiredJson>(text)
        .ok()
        .filter(|json| json.format_version == RETIRED_FORMAT_VERSION)
        .filter(|json| json.retired.len() <= MAX_RETIRED)
        .and_then(|json| {
            let ids: Option<BTreeSet<TestId>> =
                json.retired.iter().map(|t| TestId::parse(t)).collect();
            ids.filter(|ids| ids.len() == json.retired.len())
        });
    match parsed {
        Some(ids) => (RetiredIds { ids }, None),
        None => (RetiredIds::default(), Some(RETIRED_DAMAGED_NOTICE)),
    }
}

/// The canonical text of `retired`: sorted, two-space indent, trailing newline.
#[must_use]
pub fn write_retired(retired: &RetiredIds) -> String {
    pretty(&RetiredJson {
        format_version: RETIRED_FORMAT_VERSION,
        retired: retired.ids.iter().map(ToString::to_string).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> TestId {
        TestId::parse(text).unwrap()
    }

    #[test]
    fn the_golden_reads_and_writes_back_sorted() {
        let text = include_str!("../tests/fixtures/materials/retired_ids.json");
        let (retired, notice) = read_retired(Some(text));
        assert_eq!(notice, None);
        assert_eq!(retired.len(), 3);
        assert!(retired.contains(&id("4H8ZP")));
        assert_eq!(write_retired(&retired), text);
    }

    #[test]
    fn a_damaged_file_reads_empty_with_one_notice() {
        let damaged = include_str!("../tests/fixtures/materials/retired_ids_damaged.json");
        for text in [
            damaged,
            "{",
            r#"{ "format_version": 2, "retired": [] }"#,
            r#"{ "format_version": 1, "retired": ["4H8ZP", "4H8ZP"] }"#,
            r#"{ "format_version": 1, "retired": [], "more": 1 }"#,
        ] {
            assert_eq!(
                read_retired(Some(text)),
                (RetiredIds::default(), Some(RETIRED_DAMAGED_NOTICE)),
                "{text}"
            );
        }
        assert_eq!(read_retired(None), (RetiredIds::default(), None));
    }

    #[test]
    fn retiring_only_adds_and_stops_at_twenty_thousand() {
        let mut retired = RetiredIds::default();
        assert_eq!(retired.retire([id("B0000"), id("A0000")]), Ok(true));
        assert_eq!(retired.retire([id("A0000")]), Ok(false));
        assert_eq!(
            retired.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["A0000", "B0000"]
        );
        let many = (0..MAX_RETIRED - 2).map(|n| id(&format!("{n:05}")));
        assert_eq!(retired.retire(many), Ok(true));
        assert_eq!(retired.retire([id("ZZZZZ")]), Err(RetireRefused::Full));
        assert_eq!(retired.len(), MAX_RETIRED);
    }
}
