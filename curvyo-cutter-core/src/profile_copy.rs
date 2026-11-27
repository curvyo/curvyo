//! The cutter profile copy of a plotter page (`specs/0159-cutting-plotter` criterion 17, ADR 0039
//! via `adrs.md` PL3): the opaque `mfg_profile` map, every value read on its own with the default
//! built-in's value on damage.

use serde_json::{Map, Value};

use crate::builtins::{DEFAULT_BUILTIN, builtin};
use crate::mat::MatSize;
use crate::numbers::{PressurePct, SpeedMmS};
use crate::profile::{
    CutterProfile, CutterValues, ForceRange, OriginCorner, OutputFormat, WorkArea,
};

const ID: &str = "id";
const NAME: &str = "name";
const WIDTH: &str = "work_width_mm";
const HEIGHT: &str = "work_height_mm";
const ORIGIN: &str = "origin";
const OUTPUT_FORMAT: &str = "output_format";
const FORCE_MIN: &str = "force_min";
const FORCE_MAX: &str = "force_max";
const MAX_SPEED: &str = "max_speed_mm_s";
const DEFAULT_PRESSURE: &str = "default_pressure_pct";
const DEFAULT_SPEED: &str = "default_speed_mm_s";
const MATS: &str = "mats";

/// The copy a plotter page uses: the library id it came from and the copied values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutterCopy {
    /// The record it came from; `None` for a missing id.
    pub id: Option<String>,
    /// The copied values.
    pub values: CutterValues,
}

impl CutterCopy {
    /// The copy of `profile`.
    #[must_use]
    pub fn of(profile: &CutterProfile) -> Self {
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
    pub copy: CutterCopy,
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
            copy: CutterCopy::of(&default),
            damaged: 0,
        };
    };
    let id = stored.get(ID).and_then(Value::as_str).map(str::to_owned);
    let base = id.as_deref().and_then(builtin).unwrap_or(default).values;
    let mut damaged = 0;
    let mut v = base;
    let mut read = |key: &str, parse: &dyn Fn(&Value) -> Option<Read>| -> Option<Read> {
        let value = stored.get(key)?;
        let read = parse(value);
        if read.is_none() {
            damaged += 1;
        }
        read
    };
    let name = read(NAME, &|x| {
        x.as_str()
            .filter(|n| valid_name(n))
            .map(|n| Read::Text(n.to_owned()))
    });
    let mm = |x: &Value| x.as_f64().map(Read::Mm);
    let num = |x: &Value| x.as_u64().map(Read::Number);
    let area = (read(WIDTH, &mm), read(HEIGHT, &mm));
    let origin = read(ORIGIN, &|x| {
        x.as_str().and_then(OriginCorner::parse).map(Read::Origin)
    });
    let format = read(OUTPUT_FORMAT, &|x| {
        x.as_str().and_then(OutputFormat::parse).map(Read::Format)
    });
    let force = (read(FORCE_MIN, &num), read(FORCE_MAX, &num));
    let max_speed = read(MAX_SPEED, &|x| {
        x.as_u64().and_then(SpeedMmS::new).map(Read::Speed)
    });
    let pressure = read(DEFAULT_PRESSURE, &|x| {
        x.as_u64().and_then(PressurePct::new).map(Read::Pressure)
    });
    let speed = read(DEFAULT_SPEED, &|x| {
        x.as_u64().and_then(SpeedMmS::new).map(Read::Speed)
    });
    let mats = read(MATS, &|x| mats_of(x).map(Read::Mats));
    if let Some(Read::Text(name)) = name {
        v.name = name;
    }
    if let (Some(Read::Mm(w)), Some(Read::Mm(h))) = area {
        match WorkArea::from_mm(w, h) {
            Some(area) => v.work_area = area,
            None => damaged += 1,
        }
    }
    if let Some(Read::Origin(origin)) = origin {
        v.origin = origin;
    }
    if let Some(Read::Format(format)) = format {
        v.output_format = format;
    }
    if let (Some(Read::Number(min)), Some(Read::Number(max))) = force {
        match ForceRange::new(min, max) {
            Some(force) => v.force = force,
            None => damaged += 1,
        }
    }
    if let Some(Read::Speed(s)) = max_speed {
        v.max_speed = s;
    }
    if let Some(Read::Pressure(p)) = pressure {
        v.default_pressure = p;
    }
    if let Some(Read::Speed(s)) = speed {
        v.default_speed = s;
    }
    if let Some(Read::Mats(m)) = mats {
        v.mats = m;
    }
    if v.default_speed > v.max_speed {
        v.default_speed = v.max_speed;
        damaged += 1;
    }
    CopyRead {
        copy: CutterCopy { id, values: v },
        damaged,
    }
}

/// The `mfg_profile` map of `copy`.
#[must_use]
pub fn write_copy(copy: &CutterCopy) -> Map<String, Value> {
    let v = &copy.values;
    let mut map = Map::new();
    if let Some(id) = &copy.id {
        map.insert(ID.to_owned(), Value::from(id.as_str()));
    }
    map.insert(NAME.to_owned(), Value::from(v.name.as_str()));
    map.insert(WIDTH.to_owned(), Value::from(v.work_area.width_mm()));
    map.insert(HEIGHT.to_owned(), Value::from(v.work_area.height_mm()));
    map.insert(ORIGIN.to_owned(), Value::from(v.origin.key()));
    map.insert(OUTPUT_FORMAT.to_owned(), Value::from(v.output_format.key()));
    map.insert(FORCE_MIN.to_owned(), Value::from(v.force.min()));
    map.insert(FORCE_MAX.to_owned(), Value::from(v.force.max()));
    map.insert(MAX_SPEED.to_owned(), Value::from(v.max_speed.get()));
    map.insert(
        DEFAULT_PRESSURE.to_owned(),
        Value::from(v.default_pressure.get()),
    );
    map.insert(DEFAULT_SPEED.to_owned(), Value::from(v.default_speed.get()));
    map.insert(
        MATS.to_owned(),
        Value::from(v.mats.iter().map(|m| m.key()).collect::<Vec<_>>()),
    );
    map
}

/// One value as read.
enum Read {
    Text(String),
    Mm(f64),
    Number(u64),
    Origin(OriginCorner),
    Format(OutputFormat),
    Speed(SpeedMmS),
    Pressure(PressurePct),
    Mats(Vec<MatSize>),
}

fn valid_name(name: &str) -> bool {
    (1..=crate::profile::MAX_NAME_CHARS).contains(&name.trim().chars().count())
}

fn mats_of(value: &Value) -> Option<Vec<MatSize>> {
    value
        .as_array()?
        .iter()
        .map(|m| m.as_str().and_then(MatSize::parse))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::GENERIC_HPGL_CUTTER;

    #[test]
    fn a_copy_writes_and_reads_back() {
        let copy = CutterCopy::of(&GENERIC_HPGL_CUTTER.profile());
        let read = read_copy(Some(&write_copy(&copy)));
        assert_eq!(read, CopyRead { copy, damaged: 0 });
        assert_eq!(read_copy(None).copy.values.name, "Cricut Maker 3");
    }

    #[test]
    fn a_damaged_value_reads_as_its_builtin_value() {
        let copy = CutterCopy::of(&GENERIC_HPGL_CUTTER.profile());
        let mut map = write_copy(&copy);
        map.insert(OUTPUT_FORMAT.into(), Value::from("gpgl"));
        map.insert(DEFAULT_PRESSURE.into(), Value::from(400));
        let read = read_copy(Some(&map));
        assert_eq!(read.damaged, 2);
        assert_eq!(read.copy.values.output_format, OutputFormat::Hpgl);
        assert_eq!(read.copy.values.default_pressure.get(), 40);
    }
}
