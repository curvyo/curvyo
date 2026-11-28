//! The CNC profile copy of a CNC page (`specs/0161-cnc-output` criterion 20, ADR 0039 via
//! `adrs.md` CN2): the opaque `mfg_profile` map, every value read on its own with the default
//! built-in's value on damage. The connection stays on the record.

use curvyo_document_core::{DocumentSize, Length, PageAnchor, PageCorner};
use serde_json::{Map, Value};

use crate::builtins::{DEFAULT_BUILTIN, builtin};
use crate::profile::{
    CncProfile, CncValues, MAX_NAME_CHARS, MIN_SAFE_HEIGHT_MM, OutputFormat, WORK_AREA_MM,
    Z_TRAVEL_MM,
};
use crate::units::{AccelMmS2, FeedMmMin, Rpm};

const ID: &str = "id";
const NAME: &str = "name";
const OUTPUT_FORMAT: &str = "output_format";
const WIDTH: &str = "work_width_mm";
const HEIGHT: &str = "work_height_mm";
const Z_TRAVEL: &str = "z_travel_mm";
const SPINDLE_MAX: &str = "spindle_max_rpm";
const MAX_RATE: &str = "max_rate_mm_min";
const ACCELERATION: &str = "acceleration_mm_s2";
const SAFE_HEIGHT: &str = "safe_height_mm";

/// The copy a CNC page uses: the library id it came from and the copied values.
#[derive(Debug, Clone, PartialEq)]
pub struct CncCopy {
    /// The record it came from; `None` for a missing id.
    pub id: Option<String>,
    /// The copied values (the connection is the default: it lives on the record).
    pub values: CncValues,
}

impl CncCopy {
    /// The copy of `profile`.
    #[must_use]
    pub fn of(profile: &CncProfile) -> Self {
        Self {
            id: Some(profile.id.clone()),
            values: profile.values.clone(),
        }
    }
}

/// The point a CNC page keeps when its size changes (ADR 0098 decision 1): the top-left corner,
/// where the stock and the G-code origin sit.
#[must_use]
pub fn resize_anchor(copy: &CncCopy) -> PageAnchor {
    let _ = copy;
    PageAnchor::Corner(PageCorner::default())
}

/// Whether the page's `copy` differs from the library `record` it names (ADR 0098 decision 5):
/// the values, the work area among them.
#[must_use]
pub fn differs(copy: &CncCopy, record: &CncProfile) -> bool {
    copy.values != record.values
}

/// A copy as read and how many of its values were damaged.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyRead {
    /// The copy.
    pub copy: CncCopy,
    /// How many present values did not read.
    pub damaged: usize,
}

fn mm_in(value: &Value, (min, max): (f64, f64)) -> Option<Length> {
    value
        .as_f64()
        .filter(|v| (min..=max).contains(v))
        .map(Length::from_mm)
}

fn triple<T>(value: &Value, make: fn(f64) -> Option<T>) -> Option<[T; 3]> {
    let items = value.as_array().filter(|a| a.len() == 3)?;
    let mut out = items.iter().map(|v| v.as_f64().and_then(make));
    Some([out.next()??, out.next()??, out.next()??])
}

/// The copy `stored` holds; no copy reads as the default built-in. A damaged value reads as the
/// built-in's (the copy's own built-in when its id names one), counted.
#[must_use]
pub fn read_copy(stored: Option<&Map<String, Value>>) -> CopyRead {
    let default = DEFAULT_BUILTIN.profile();
    let Some(stored) = stored else {
        return CopyRead {
            copy: CncCopy::of(&default),
            damaged: 0,
        };
    };
    let id = stored.get(ID).and_then(Value::as_str).map(str::to_owned);
    let mut v = id.as_deref().and_then(builtin).unwrap_or(default).values;
    let mut damaged = 0;
    let mut take = |key: &str, ok: &mut dyn FnMut(&Value) -> bool| {
        if let Some(value) = stored.get(key)
            && !ok(value)
        {
            damaged += 1;
        }
    };
    take(NAME, &mut |x| {
        x.as_str()
            .filter(|n| (1..=MAX_NAME_CHARS).contains(&n.trim().chars().count()))
            .map(|n| n.clone_into(&mut v.name))
            .is_some()
    });
    take(OUTPUT_FORMAT, &mut |x| {
        x.as_str()
            .and_then(OutputFormat::parse)
            .map(|f| v.output_format = f)
            .is_some()
    });
    let (mut width, mut height) = (v.work_area.width, v.work_area.height);
    take(WIDTH, &mut |x| {
        mm_in(x, WORK_AREA_MM).map(|l| width = l).is_some()
    });
    take(HEIGHT, &mut |x| {
        mm_in(x, WORK_AREA_MM).map(|l| height = l).is_some()
    });
    v.work_area = DocumentSize::from_mm(width.as_mm(), height.as_mm());
    take(Z_TRAVEL, &mut |x| {
        mm_in(x, Z_TRAVEL_MM).map(|l| v.z_travel = l).is_some()
    });
    take(SPINDLE_MAX, &mut |x| {
        x.as_u64()
            .and_then(Rpm::new)
            .map(|r| v.spindle_max = r)
            .is_some()
    });
    take(MAX_RATE, &mut |x| {
        triple(x, FeedMmMin::new).map(|r| v.max_rate = r).is_some()
    });
    take(ACCELERATION, &mut |x| {
        triple(x, AccelMmS2::new)
            .map(|a| v.acceleration = a)
            .is_some()
    });
    let z = v.z_travel.as_mm();
    take(SAFE_HEIGHT, &mut |x| {
        mm_in(x, (MIN_SAFE_HEIGHT_MM, z))
            .map(|l| v.safe_height = l)
            .is_some()
    });
    CopyRead {
        copy: CncCopy { id, values: v },
        damaged,
    }
}

/// The `mfg_profile` map of `copy`.
#[must_use]
pub fn write_copy(copy: &CncCopy) -> Map<String, Value> {
    let v = &copy.values;
    let mut map = Map::new();
    if let Some(id) = &copy.id {
        map.insert(ID.to_owned(), Value::from(id.as_str()));
    }
    map.insert(NAME.to_owned(), Value::from(v.name.as_str()));
    map.insert(OUTPUT_FORMAT.to_owned(), Value::from(v.output_format.key()));
    map.insert(WIDTH.to_owned(), Value::from(v.work_area.width.as_mm()));
    map.insert(HEIGHT.to_owned(), Value::from(v.work_area.height.as_mm()));
    map.insert(Z_TRAVEL.to_owned(), Value::from(v.z_travel.as_mm()));
    map.insert(SPINDLE_MAX.to_owned(), Value::from(v.spindle_max.get()));
    map.insert(
        MAX_RATE.to_owned(),
        Value::from(v.max_rate.map(FeedMmMin::get).to_vec()),
    );
    map.insert(
        ACCELERATION.to_owned(),
        Value::from(v.acceleration.map(AccelMmS2::get).to_vec()),
    );
    map.insert(SAFE_HEIGHT.to_owned(), Value::from(v.safe_height.as_mm()));
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::GENERIC_GRBL_ROUTER;

    #[test]
    fn a_cnc_page_keeps_its_top_left_and_differs_by_value() {
        let router = GENERIC_GRBL_ROUTER.profile();
        let mut copy = CncCopy::of(&router);
        assert_eq!(
            resize_anchor(&copy),
            PageAnchor::Corner(PageCorner::default())
        );
        assert!(!differs(&copy, &router));
        copy.values.name = "Mine".to_owned();
        assert!(differs(&copy, &router));
    }

    #[test]
    fn a_copy_writes_and_reads_back() {
        let copy = CncCopy::of(&GENERIC_GRBL_ROUTER.profile());
        let read = read_copy(Some(&write_copy(&copy)));
        assert_eq!(read, CopyRead { copy, damaged: 0 });
        assert_eq!(read_copy(None).copy.values.name, "Generic GRBL router");
    }

    #[test]
    fn a_damaged_value_reads_as_its_builtin_value() {
        let copy = CncCopy::of(&GENERIC_GRBL_ROUTER.profile());
        let mut map = write_copy(&copy);
        map.insert(SAFE_HEIGHT.into(), Value::from(500.0));
        map.insert(MAX_RATE.into(), Value::from(vec![1.0, 0.0]));
        let read = read_copy(Some(&map));
        assert_eq!(read.damaged, 2);
        assert_eq!(read.copy.values.safe_height.as_mm(), 5.0);
        assert_eq!(read.copy.values.max_rate[0].get(), 500.0);
    }
}
