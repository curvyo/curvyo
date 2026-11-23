//! The Print and cut register on a placement (ADR 0081 §4), its model, read and write.
//!
//! ```text
//! meta map of a placement (format 55) may hold:
//!   x.curvyo.laser.print_and_cut : one atomic map
//!     marks        : {"1": [x_um, y_um], ...}  integer µm, machine work coordinates, "1".."4"
//!     tolerance_um : integer 50..5000 (default 500)
//!     reset        : [a, b, c, d, e, f] the matrix before the last Apply (optional)
//! ```
//!
//! Absent means no measure. A register that does not read this way is `Unreadable`: the group
//! says so with empty rows and the next measure write replaces it; the file still opens.

use std::collections::{BTreeMap, HashMap};

use loro::{LoroMap, LoroValue, ValueOrContainer};

use crate::affine::Affine;
use crate::path_codec::as_f64;

/// The register key.
pub(crate) const KEY_PRINT_AND_CUT: &str = "x.curvyo.laser.print_and_cut";
const MARKS: &str = "marks";
const TOLERANCE: &str = "tolerance_um";
const RESET: &str = "reset";

/// The default largest-residual tolerance in µm (criterion 13).
pub const PRINT_AND_CUT_DEFAULT_TOLERANCE_UM: i64 = 500;
/// The smallest tolerance in µm.
pub const PRINT_AND_CUT_MIN_TOLERANCE_UM: i64 = 50;
/// The largest tolerance in µm.
pub const PRINT_AND_CUT_MAX_TOLERANCE_UM: i64 = 5000;
/// The largest measured coordinate in µm either way (UX notes P15: 10,000 mm).
pub const PRINT_AND_CUT_MAX_MEASURE_UM: i64 = 10_000_000;

/// The label of a row edit, Clear or Take head position; its history name is "Measure mark".
pub const PRINT_AND_CUT_MEASURE_COMMIT_LABEL: &str = "print_and_cut_measure";
/// The label of a tolerance edit; its history name is "Print and cut tolerance".
pub const PRINT_AND_CUT_TOLERANCE_COMMIT_LABEL: &str = "print_and_cut_tolerance";
/// The label of Apply; its history name is "Fit sheet to marks".
pub const APPLY_PRINT_AND_CUT_COMMIT_LABEL: &str = "apply_print_and_cut";
/// The label of Reset placement; its history name is "Reset sheet placement".
pub const RESET_PRINT_AND_CUT_COMMIT_LABEL: &str = "reset_print_and_cut";

/// The measures stored on one placement.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintAndCutMeasures {
    /// Mark number to its measured position `[x, y]` in µm (machine work coordinates).
    pub marks: BTreeMap<u8, [i64; 2]>,
    /// The largest-residual tolerance in µm.
    pub tolerance_um: i64,
    /// The placement matrix before the last Apply.
    pub reset: Option<Affine>,
}

impl Default for PrintAndCutMeasures {
    fn default() -> Self {
        Self {
            marks: BTreeMap::new(),
            tolerance_um: PRINT_AND_CUT_DEFAULT_TOLERANCE_UM,
            reset: None,
        }
    }
}

/// What a placement's register holds.
#[derive(Debug, Clone, PartialEq)]
pub enum PrintAndCutRegister {
    /// No register: nothing measured.
    Absent,
    /// A register that cannot be read ("The stored measures could not be read.").
    Unreadable,
    /// The measures.
    Measures(PrintAndCutMeasures),
}

/// The register of the placement `meta`.
pub(crate) fn read_register(meta: &LoroMap) -> PrintAndCutRegister {
    match meta.get(KEY_PRINT_AND_CUT) {
        None => PrintAndCutRegister::Absent,
        Some(ValueOrContainer::Value(LoroValue::Map(map))) => read_measures(&map).map_or(
            PrintAndCutRegister::Unreadable,
            PrintAndCutRegister::Measures,
        ),
        Some(_) => PrintAndCutRegister::Unreadable,
    }
}

fn read_measures(map: &loro::LoroMapValue) -> Option<PrintAndCutMeasures> {
    let mut measures = PrintAndCutMeasures::default();
    if let Some(value) = map.get(MARKS) {
        let LoroValue::Map(marks) = value else {
            return None;
        };
        for (key, value) in marks.iter() {
            let number: u8 = key.parse().ok().filter(|n| (1..=4).contains(n))?;
            let LoroValue::List(xy) = value else {
                return None;
            };
            let [LoroValue::I64(x), LoroValue::I64(y)] = xy.as_slice() else {
                return None;
            };
            let limit = PRINT_AND_CUT_MAX_MEASURE_UM;
            if x.abs() > limit || y.abs() > limit {
                return None;
            }
            measures.marks.insert(number, [*x, *y]);
        }
    }
    if let Some(value) = map.get(TOLERANCE) {
        let LoroValue::I64(t) = value else {
            return None;
        };
        if !(PRINT_AND_CUT_MIN_TOLERANCE_UM..=PRINT_AND_CUT_MAX_TOLERANCE_UM).contains(t) {
            return None;
        }
        measures.tolerance_um = *t;
    }
    if let Some(value) = map.get(RESET) {
        let LoroValue::List(entries) = value else {
            return None;
        };
        if entries.len() != 6 {
            return None;
        }
        let mut m = [0.0; 6];
        for (slot, entry) in m.iter_mut().zip(entries.iter()) {
            *slot = as_f64(entry).filter(|n| n.is_finite() && n.abs() <= 1e7)?;
        }
        measures.reset = Some(Affine::from_array(m));
    }
    Some(measures)
}

/// Writes `measures` whole, or removes the register when it holds nothing but defaults.
pub(crate) fn write_register(meta: &LoroMap, measures: &PrintAndCutMeasures) {
    if measures == &PrintAndCutMeasures::default() {
        // invariant: deleting a key of an attached map cannot fail.
        #[allow(clippy::unwrap_used)]
        meta.delete(KEY_PRINT_AND_CUT).unwrap();
        return;
    }
    let marks: HashMap<String, LoroValue> = measures
        .marks
        .iter()
        .map(|(n, [x, y])| {
            (
                n.to_string(),
                LoroValue::from(vec![LoroValue::I64(*x), LoroValue::I64(*y)]),
            )
        })
        .collect();
    let mut value: HashMap<String, LoroValue> = HashMap::from([
        (MARKS.to_owned(), LoroValue::from(marks)),
        (TOLERANCE.to_owned(), LoroValue::I64(measures.tolerance_um)),
    ]);
    if let Some(reset) = measures.reset {
        let entries: Vec<LoroValue> = reset
            .to_array()
            .into_iter()
            .map(LoroValue::Double)
            .collect();
        value.insert(RESET.to_owned(), LoroValue::from(entries));
    }
    // invariant: inserting a plain map value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_PRINT_AND_CUT, LoroValue::from(value))
        .unwrap();
}
