//! The plotter's operation kinds Cut and Draw and their stored numbers (`specs/0159-cutting-plotter`
//! criteria 7 to 9, `adrs.md` PL3): reading `op_settings` with damage as the default, writing it,
//! and checking a typed value against its range.

use serde_json::{Map, Value};

use crate::numbers::{Passes, PressurePct, SpeedMmS};
use crate::profile::CutterValues;

/// The `op_settings` key of Pressure.
pub const KEY_PRESSURE: &str = "pressure_pct";
/// The `op_settings` key of Speed.
pub const KEY_SPEED: &str = "speed_mm_s";
/// The `op_settings` key of Passes (Cut only).
pub const KEY_PASSES: &str = "passes";

/// What an operation of a plotter page does (Frage 428 A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpKind {
    /// The blade cuts along each item's outline.
    Cut,
    /// The pen follows each item's centre line.
    Draw,
}

impl OpKind {
    /// Both kinds, in Select-bar order.
    pub const ALL: [Self; 2] = [Self::Cut, Self::Draw];

    /// The stored `op_kind`.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Cut => "cut",
            Self::Draw => "draw",
        }
    }

    /// The kind a stored `op_kind` names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == key)
    }

    /// The name and the start of a new operation's name ("Cut 1").
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cut => "Cut",
            Self::Draw => "Draw",
        }
    }

    /// Whether the kind runs in the cutting section, last with Cuts last (criterion 9).
    #[must_use]
    pub const fn is_cutting(self) -> bool {
        matches!(self, Self::Cut)
    }
}

/// The numbers of one operation (criterion 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CutterSettings {
    /// Pressure.
    pub pressure: PressurePct,
    /// Speed.
    pub speed: SpeedMmS,
    /// Passes; always one for Draw.
    pub passes: Passes,
}

impl CutterSettings {
    /// A new operation's numbers from the profile's defaults.
    #[must_use]
    pub fn defaults(values: &CutterValues) -> Self {
        Self {
            pressure: values.default_pressure,
            speed: values.default_speed,
            passes: Passes::ONE,
        }
    }

    /// The numbers of `kind` stored in `stored`: each missing or damaged value (out of its range,
    /// a speed above the profile's Max speed) reads as the profile's default, counted (PL3).
    #[must_use]
    pub fn read(kind: OpKind, stored: &Map<String, Value>, values: &CutterValues) -> (Self, usize) {
        let defaults = Self::defaults(values);
        let mut damaged = 0;
        let mut number = |key: &str| -> Option<u64> {
            let value = stored.get(key)?;
            let n = value.as_u64();
            if n.is_none() {
                damaged += 1;
            }
            n
        };
        let pressure = number(KEY_PRESSURE);
        let speed = number(KEY_SPEED);
        let passes = match kind {
            OpKind::Cut => number(KEY_PASSES),
            OpKind::Draw => None,
        };
        let mut checked = |n: Option<u64>, ok: Option<bool>| match (n, ok) {
            (Some(_), Some(true)) | (None, _) => {}
            _ => damaged += 1,
        };
        let pressure_ok = pressure.map(|n| PressurePct::new(n).is_some());
        let speed_ok = speed.map(|n| SpeedMmS::new(n).is_some_and(|s| s <= values.max_speed));
        let passes_ok = passes.map(|n| Passes::new(n).is_some());
        checked(pressure, pressure_ok);
        checked(speed, speed_ok);
        checked(passes, passes_ok);
        let settings = Self {
            pressure: pressure
                .and_then(PressurePct::new)
                .unwrap_or(defaults.pressure),
            speed: speed
                .and_then(SpeedMmS::new)
                .filter(|s| *s <= values.max_speed)
                .unwrap_or(defaults.speed),
            passes: passes.and_then(Passes::new).unwrap_or(Passes::ONE),
        };
        (settings, damaged)
    }

    /// The `op_settings` map of `kind` (Draw writes no passes).
    #[must_use]
    pub fn write(self, kind: OpKind) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert(KEY_PRESSURE.to_owned(), Value::from(self.pressure.get()));
        map.insert(KEY_SPEED.to_owned(), Value::from(self.speed.get()));
        if kind == OpKind::Cut {
            map.insert(KEY_PASSES.to_owned(), Value::from(self.passes.get()));
        }
        map
    }
}

/// A field of an operation's numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// Pressure, %.
    Pressure,
    /// Speed, mm/s.
    Speed,
    /// Passes, Cut only.
    Passes,
}

impl Field {
    /// The field a key names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        match key {
            KEY_PRESSURE | "pressure" => Some(Self::Pressure),
            KEY_SPEED | "speed" => Some(Self::Speed),
            KEY_PASSES => Some(Self::Passes),
            _ => None,
        }
    }
}

/// The settings `settings` with `field` set to the typed `text`, or the field message
/// (`ux-notes.md` 11): a whole number within the field's range, a speed within `max_speed`.
///
/// # Errors
/// The message to show when the text is no whole number or out of range, or the field is not one
/// of `kind`.
pub fn edit_setting(
    kind: OpKind,
    settings: CutterSettings,
    field: Field,
    text: &str,
    max_speed: SpeedMmS,
) -> Result<CutterSettings, String> {
    let n: u64 = text
        .trim()
        .parse()
        .map_err(|_| "Enter a whole number.".to_owned())?;
    let mut out = settings;
    match field {
        Field::Pressure => {
            out.pressure =
                PressurePct::new(n).ok_or_else(|| "Pressure is 1 to 100 %.".to_owned())?;
        }
        Field::Speed => {
            out.speed = SpeedMmS::new(n)
                .filter(|s| *s <= max_speed)
                .ok_or_else(|| format!("Speed is 1 to {} mm/s.", max_speed.get()))?;
        }
        Field::Passes if kind == OpKind::Cut => {
            out.passes = Passes::new(n).ok_or_else(|| "Passes is 1 to 10.".to_owned())?;
        }
        Field::Passes => return Err("Draw has no passes.".to_owned()),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::GENERIC_HPGL_CUTTER;

    fn values() -> CutterValues {
        GENERIC_HPGL_CUTTER.profile().values
    }

    #[test]
    fn settings_write_and_read_back() {
        let values = values();
        let mut settings = CutterSettings::defaults(&values);
        settings.passes = Passes::new(3).unwrap();
        let map = settings.write(OpKind::Cut);
        assert_eq!(
            serde_json::to_string(&map).unwrap(),
            r#"{"passes":3,"pressure_pct":40,"speed_mm_s":200}"#
        );
        assert_eq!(
            CutterSettings::read(OpKind::Cut, &map, &values),
            (settings, 0)
        );
        assert!(!settings.write(OpKind::Draw).contains_key(KEY_PASSES));
    }

    #[test]
    fn a_damaged_number_reads_as_the_default_with_a_count() {
        let values = values();
        let mut map = Map::new();
        map.insert(KEY_PRESSURE.into(), Value::from(250));
        map.insert(KEY_SPEED.into(), Value::from("fast"));
        map.insert(KEY_PASSES.into(), Value::from(2));
        let (read, damaged) = CutterSettings::read(OpKind::Cut, &map, &values);
        assert_eq!(damaged, 2);
        assert_eq!(read.pressure.get(), 40);
        assert_eq!(read.speed.get(), 200);
        assert_eq!(read.passes.get(), 2);
    }

    #[test]
    fn a_typed_value_outside_its_range_is_refused_with_the_range() {
        let values = values();
        let s = CutterSettings::defaults(&values);
        let max = values.max_speed;
        assert_eq!(
            edit_setting(OpKind::Cut, s, Field::Passes, "11", max),
            Err("Passes is 1 to 10.".to_owned())
        );
        assert_eq!(
            edit_setting(OpKind::Cut, s, Field::Speed, "900", max),
            Err("Speed is 1 to 800 mm/s.".to_owned())
        );
        assert_eq!(
            edit_setting(OpKind::Draw, s, Field::Pressure, "4.5", max),
            Err("Enter a whole number.".to_owned())
        );
        let edited = edit_setting(OpKind::Cut, s, Field::Passes, "2", max).unwrap();
        assert_eq!(edited.passes.get(), 2);
    }
}
