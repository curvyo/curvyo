//! The embroidery profile copy of an embroidery page (`specs/0160-embroidery` criterion 20, ADR
//! 0039 via `adrs.md` EM5): the opaque `mfg_profile` map, every value read on its own with the
//! default built-in's value on damage.

use serde_json::{Map, Value};

use crate::builtins::{DEFAULT_BUILTIN, builtin};
use crate::hoop::HoopSize;
use crate::profile::{
    DST_MAX_STITCH, EmbroideryProfile, EmbroideryValues, MAX_NAME_CHARS, MIN_MAX_STITCH,
    OutputFormat, StitchesPerMin,
};
use crate::stitch::TenthMm;

const ID: &str = "id";
const NAME: &str = "name";
const OUTPUT_FORMAT: &str = "output_format";
const HOOPS: &str = "hoops";
const MAX_STITCH: &str = "max_stitch_mm";
const SPEED: &str = "speed_spm";

/// The copy an embroidery page uses: the library id it came from and the copied values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbroideryCopy {
    /// The record it came from; `None` for a missing id.
    pub id: Option<String>,
    /// The copied values.
    pub values: EmbroideryValues,
}

impl EmbroideryCopy {
    /// The copy of `profile`.
    #[must_use]
    pub fn of(profile: &EmbroideryProfile) -> Self {
        Self {
            id: Some(profile.id.clone()),
            values: profile.values.clone(),
        }
    }
}

/// A copy as read and how many of its values were damaged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyRead {
    /// The copy.
    pub copy: EmbroideryCopy,
    /// How many present values did not read.
    pub damaged: usize,
}

/// The copy `stored` holds; no copy reads as the default built-in. A damaged value reads as the
/// built-in's (the copy's own built-in when its id names one), counted.
#[must_use]
pub fn read_copy(stored: Option<&Map<String, Value>>) -> CopyRead {
    let default = DEFAULT_BUILTIN.profile();
    let Some(stored) = stored else {
        return CopyRead {
            copy: EmbroideryCopy::of(&default),
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
    take(HOOPS, &mut |x| {
        hoops_of(x)
            .filter(|h| !h.is_empty())
            .map(|h| v.hoops = h)
            .is_some()
    });
    take(MAX_STITCH, &mut |x| {
        x.as_f64()
            .map(TenthMm::of_mm)
            .filter(|t| (MIN_MAX_STITCH..=DST_MAX_STITCH).contains(t))
            .map(|t| v.max_stitch = t)
            .is_some()
    });
    take(SPEED, &mut |x| {
        x.as_u64()
            .and_then(StitchesPerMin::new)
            .map(|s| v.speed = s)
            .is_some()
    });
    CopyRead {
        copy: EmbroideryCopy { id, values: v },
        damaged,
    }
}

/// The `mfg_profile` map of `copy`.
#[must_use]
pub fn write_copy(copy: &EmbroideryCopy) -> Map<String, Value> {
    let v = &copy.values;
    let mut map = Map::new();
    if let Some(id) = &copy.id {
        map.insert(ID.to_owned(), Value::from(id.as_str()));
    }
    map.insert(NAME.to_owned(), Value::from(v.name.as_str()));
    map.insert(OUTPUT_FORMAT.to_owned(), Value::from(v.output_format.key()));
    map.insert(
        HOOPS.to_owned(),
        Value::from(v.hoops.iter().map(|h| h.key()).collect::<Vec<_>>()),
    );
    map.insert(MAX_STITCH.to_owned(), Value::from(v.max_stitch.mm()));
    map.insert(SPEED.to_owned(), Value::from(v.speed.get()));
    map
}

fn hoops_of(value: &Value) -> Option<Vec<HoopSize>> {
    let hoops: Vec<HoopSize> = value
        .as_array()?
        .iter()
        .map(|h| h.as_str().and_then(HoopSize::parse))
        .collect::<Option<_>>()?;
    let distinct = hoops
        .iter()
        .enumerate()
        .all(|(i, h)| !hoops[..i].contains(h));
    distinct.then_some(hoops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::W6_EU7;

    #[test]
    fn a_copy_writes_and_reads_back() {
        let copy = EmbroideryCopy::of(&W6_EU7.profile());
        let read = read_copy(Some(&write_copy(&copy)));
        assert_eq!(read, CopyRead { copy, damaged: 0 });
        assert_eq!(read_copy(None).copy.values.name, "W6 EU7");
    }

    #[test]
    fn a_damaged_value_reads_as_its_builtin_value() {
        let copy = EmbroideryCopy::of(&W6_EU7.profile());
        let mut map = write_copy(&copy);
        map.insert(SPEED.into(), Value::from(5_000));
        map.insert(HOOPS.into(), Value::from(vec!["9x9"]));
        let read = read_copy(Some(&map));
        assert_eq!(read.damaged, 2);
        assert_eq!(read.copy.values.speed.get(), 600);
        assert_eq!(read.copy.values.hoops.len(), 2);
    }
}
