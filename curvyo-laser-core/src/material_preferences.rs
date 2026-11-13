//! Reads and writes `material-order.json`, the maker's order of the Materials tab (ADR 0055 §5).

use serde::{Deserialize, Serialize};

use crate::record_envelope::pretty;

/// The most machines `test-grid-choice.json` remembers a recipe for (ADR 0055 §6).
pub const MAX_GRID_CHOICE_MACHINES: usize = 100;
/// The format of `material-order.json`.
const PREFERENCE_FORMAT_VERSION: u64 = 1;
/// The longest preference file that is read.
const MAX_PREFERENCE_BYTES: usize = 256 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderJson {
    format_version: u64,
    order: Vec<String>,
}

/// The ids `material-order.json` lists, in order; empty for a damaged or newer file (ADR 0055
/// §5). Ids no record has are kept, so they survive until the next order write.
#[must_use]
pub fn read_order(text: &str) -> Vec<String> {
    if text.len() > MAX_PREFERENCE_BYTES {
        return Vec::new();
    }
    serde_json::from_str::<OrderJson>(text)
        .ok()
        .filter(|order| order.format_version == PREFERENCE_FORMAT_VERSION)
        .map(|order| order.order)
        .unwrap_or_default()
}

/// The text of `material-order.json` listing `ids`.
#[must_use]
pub fn write_order(ids: &[String]) -> String {
    pretty(&OrderJson {
        format_version: PREFERENCE_FORMAT_VERSION,
        order: ids.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_order_reads_back_and_a_damaged_or_newer_file_reads_as_empty() {
        let ids = vec!["a".to_owned(), "b".to_owned()];
        assert_eq!(read_order(&write_order(&ids)), ids);
        assert_eq!(read_order("{"), Vec::<String>::new());
        assert_eq!(
            read_order(r#"{"format_version": 2, "order": ["a"]}"#),
            Vec::<String>::new()
        );
    }
}
