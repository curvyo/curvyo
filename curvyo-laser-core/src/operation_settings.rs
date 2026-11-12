//! The stored `op_settings` of an operation as plain values (ADR 0054 §1, `specs/0028` criteria
//! 24 to 28, ADR 0056 §5): its numbers, its material link and its typed overrides, read with their
//! damage.

use serde_json::{Map, Value, json};

use crate::machine_profile::FeedRate;
use crate::material::{Interval, MAX_MATERIAL_NAME_CHARS, Passes, PowerPercent, SetKind, Settings};
use crate::operation_kind::OperationKind;
use crate::operation_options::hatches;

/// A link of an operation to a material, with the name and `modified` it had when its numbers
/// were last taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialLink {
    /// The material's id.
    pub id: String,
    /// Its name when the numbers were taken.
    pub name: String,
    /// Which set fed the outline of Engrave + Outline (the Engrave set when there is no Score
    /// set); `None` for other kinds.
    pub outline_from: Option<SetKind>,
    /// The record's `modified` the numbers were taken from (the refresh guard).
    pub modified: u64,
}

/// The two sets an operation may hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// The kind's set (the hatch of Engrave + Outline).
    Main,
    /// The outline of Engrave + Outline.
    Outline,
}

impl Role {
    /// The stored key.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Outline => "outline",
        }
    }
}

/// The three numbers of a set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    /// Power in percent.
    Power,
    /// Speed in mm/min.
    Speed,
    /// Passes.
    Passes,
}

/// The numbers the maker typed over one set of a linked operation; each field optional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SetOverride {
    /// A typed power.
    pub power: Option<PowerPercent>,
    /// A typed speed.
    pub speed: Option<FeedRate>,
    /// Typed passes.
    pub passes: Option<Passes>,
    /// A typed interval, read from a file (`overrides.main.interval_mm`, Engrave kinds only); no
    /// command of this slice writes one (`specs/0111` criterion 19).
    pub interval: Option<Interval>,
}

impl SetOverride {
    /// Whether no number is typed.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.power.is_none()
            && self.speed.is_none()
            && self.passes.is_none()
            && self.interval.is_none()
    }

    /// `set` with each typed number put over its field.
    #[must_use]
    pub fn apply(&self, set: Settings) -> Settings {
        Settings {
            power: self.power.unwrap_or(set.power),
            speed: self.speed.unwrap_or(set.speed),
            passes: self.passes.unwrap_or(set.passes),
        }
    }

    /// The typed fields, in the order power, speed, passes (a typed interval is no [`Field`]).
    #[must_use]
    pub fn fields(&self) -> Vec<Field> {
        [
            (self.power.is_some(), Field::Power),
            (self.speed.is_some(), Field::Speed),
            (self.passes.is_some(), Field::Passes),
        ]
        .into_iter()
        .filter_map(|(typed, field)| typed.then_some(field))
        .collect()
    }
}

/// The typed numbers of a linked operation per set; both empty means none (`overrides` absent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Overrides {
    /// Over the main set.
    pub main: SetOverride,
    /// Over the outline set (Engrave + Outline only).
    pub outline: SetOverride,
}

impl Overrides {
    /// Whether nothing is typed.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.main.is_empty() && self.outline.is_empty()
    }

    /// The override of `role`.
    #[must_use]
    pub const fn of(&self, role: Role) -> &SetOverride {
        match role {
            Role::Main => &self.main,
            Role::Outline => &self.outline,
        }
    }

    pub(crate) fn of_mut(&mut self, role: Role) -> &mut SetOverride {
        match role {
            Role::Main => &mut self.main,
            Role::Outline => &mut self.outline,
        }
    }
}

/// A readable `op_settings` value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSettings {
    /// The kind's set: own numbers, or the material's as last seen.
    pub main: Settings,
    /// `main.interval_mm` of an Engrave kind; absent reads as [`Interval::DEFAULT`].
    pub interval: Option<Interval>,
    /// The outline of Engrave + Outline; absent, the outline runs with `main`.
    pub outline: Option<Settings>,
    /// The material link; absent means own numbers.
    pub material: Option<MaterialLink>,
    /// The typed numbers over a linked operation; always empty without a link.
    pub overrides: Overrides,
}

/// Why a stored `op_settings` does not read: it is kept, runs nothing and is never repaired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsDamage {
    /// Not a map.
    NotAMap,
    /// A key this build does not know.
    UnknownKey,
    /// A set is missing, not a map, or holds a missing or out-of-range number.
    BadSet,
    /// An `outline` on a kind other than Engrave + Outline.
    OutlineOnKind,
    /// A link without an id or `modified`, with a bad name or a bad `outline_from`.
    BadLink,
    /// `overrides` without a link, empty, with an empty role map, an unknown key, a bad number,
    /// or an `outline` on a kind other than Engrave + Outline.
    BadOverride,
    /// An `interval_mm` in an `outline` set or override, or on a kind that does not hatch, or one
    /// out of range (ADR 0056 §5).
    BadInterval,
}

/// The stored value as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsRead {
    /// No `op_settings`: "No settings".
    None,
    /// A damaged value: "Damaged settings".
    Damaged(SettingsDamage),
    /// A readable value.
    Stored(StoredSettings),
}

const SET_KEYS: [&str; 3] = ["power_pct", "speed_mm_min", "passes"];
/// The key of the hatch interval in `main` and `overrides.main`.
const KEY_INTERVAL: &str = "interval_mm";

/// The interval of a `main` set or override of an operation of kind `kind`, and the map without
/// it: `Ok(None)` when absent; damage on a kind that does not hatch or for a bad value.
fn split_interval(
    value: &Value,
    kind: OperationKind,
) -> Result<(Option<Interval>, Value), SettingsDamage> {
    let Some(map) = value.as_object() else {
        return Ok((None, value.clone()));
    };
    let Some(stored) = map.get(KEY_INTERVAL) else {
        return Ok((None, value.clone()));
    };
    if !hatches(kind) {
        return Err(SettingsDamage::BadInterval);
    }
    let interval = stored
        .as_f64()
        .and_then(Interval::from_mm)
        .ok_or(SettingsDamage::BadInterval)?;
    let mut rest = map.clone();
    rest.remove(KEY_INTERVAL);
    Ok((Some(interval), Value::Object(rest)))
}

/// Whether `value` is a map holding an interval (never allowed in an `outline`).
fn holds_interval(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|map| map.contains_key(KEY_INTERVAL))
}

/// A set of plain values; `None` when it is not a map of exactly the three valid numbers.
fn read_set(value: &Value) -> Option<Settings> {
    let map = value.as_object()?;
    if map.len() != SET_KEYS.len() || map.keys().any(|k| !SET_KEYS.contains(&k.as_str())) {
        return None;
    }
    Some(Settings {
        power: PowerPercent::from_percent(map.get("power_pct")?.as_f64()?)?,
        speed: FeedRate::new(map.get("speed_mm_min")?.as_u64()?)?,
        passes: Passes::new(map.get("passes")?.as_u64()?)?,
    })
}

/// The plain values of a set.
#[must_use]
pub fn set_value(set: &Settings) -> Value {
    json!({
        "power_pct": set.power.percent(),
        "speed_mm_min": set.speed.get(),
        "passes": set.passes.get(),
    })
}

fn read_override(value: &Value) -> Option<SetOverride> {
    let map = value.as_object()?;
    if map.is_empty() || map.keys().any(|k| !SET_KEYS.contains(&k.as_str())) {
        return None;
    }
    let mut typed = SetOverride::default();
    if let Some(power) = map.get("power_pct") {
        typed.power = Some(PowerPercent::from_percent(power.as_f64()?)?);
    }
    if let Some(speed) = map.get("speed_mm_min") {
        typed.speed = Some(FeedRate::new(speed.as_u64()?)?);
    }
    if let Some(passes) = map.get("passes") {
        typed.passes = Some(Passes::new(passes.as_u64()?)?);
    }
    Some(typed)
}

fn override_value(typed: &SetOverride) -> Value {
    let mut map = Map::new();
    if let Some(power) = typed.power {
        map.insert("power_pct".to_owned(), json!(power.percent()));
    }
    if let Some(speed) = typed.speed {
        map.insert("speed_mm_min".to_owned(), json!(speed.get()));
    }
    if let Some(passes) = typed.passes {
        map.insert("passes".to_owned(), json!(passes.get()));
    }
    if let Some(interval) = typed.interval {
        map.insert(KEY_INTERVAL.to_owned(), json!(interval.mm()));
    }
    Value::Object(map)
}

fn read_overrides(value: &Value, kind: OperationKind) -> Result<Overrides, SettingsDamage> {
    let bad = SettingsDamage::BadOverride;
    let map = value.as_object().ok_or(bad)?;
    if map.is_empty()
        || map
            .keys()
            .any(|k| !["main", "outline"].contains(&k.as_str()))
    {
        return Err(bad);
    }
    if map.contains_key("outline") && kind != OperationKind::EngraveOutline {
        return Err(bad);
    }
    let main = match map.get("main") {
        None => SetOverride::default(),
        Some(value) => {
            let (interval, rest) = split_interval(value, kind)?;
            let only_interval = interval.is_some() && rest.as_object().is_some_and(Map::is_empty);
            let mut typed = if only_interval {
                SetOverride::default()
            } else {
                read_override(&rest).ok_or(bad)?
            };
            typed.interval = interval;
            typed
        }
    };
    let outline = match map.get("outline") {
        None => SetOverride::default(),
        Some(value) if holds_interval(value) => return Err(SettingsDamage::BadInterval),
        Some(value) => read_override(value).ok_or(bad)?,
    };
    Ok(Overrides { main, outline })
}

fn read_link(value: &Value) -> Option<MaterialLink> {
    let map = value.as_object()?;
    if map
        .keys()
        .any(|k| !["id", "name", "outline_from", "modified"].contains(&k.as_str()))
    {
        return None;
    }
    let id = map.get("id")?.as_str().filter(|id| !id.is_empty())?;
    let name = map.get("name")?.as_str()?;
    if name.chars().count() > MAX_MATERIAL_NAME_CHARS {
        return None;
    }
    let outline_from = match map.get("outline_from") {
        None => None,
        Some(text) => Some(SetKind::parse(text.as_str()?).filter(|k| *k != SetKind::Cut)?),
    };
    Some(MaterialLink {
        id: id.to_owned(),
        name: name.to_owned(),
        outline_from,
        modified: map.get("modified")?.as_u64()?,
    })
}

/// Reads a stored `op_settings` of an operation of kind `kind` (ADR 0054 §1). `value` is the
/// register's plain values, `None` when it is absent (a register that is no plain value is handed
/// in as `null` and reads as not a map).
#[must_use]
pub fn read_settings(value: Option<&Value>, kind: OperationKind) -> SettingsRead {
    let Some(value) = value else {
        return SettingsRead::None;
    };
    match read_stored(value, kind) {
        Ok(stored) => SettingsRead::Stored(stored),
        Err(damage) => SettingsRead::Damaged(damage),
    }
}

fn read_stored(value: &Value, kind: OperationKind) -> Result<StoredSettings, SettingsDamage> {
    let map = value.as_object().ok_or(SettingsDamage::NotAMap)?;
    let known = ["main", "outline", "material", "overrides"];
    if map.keys().any(|k| !known.contains(&k.as_str())) {
        return Err(SettingsDamage::UnknownKey);
    }
    let (interval, main) = split_interval(map.get("main").ok_or(SettingsDamage::BadSet)?, kind)?;
    let main = read_set(&main).ok_or(SettingsDamage::BadSet)?;
    let outline = match map.get("outline") {
        None => None,
        Some(_) if kind != OperationKind::EngraveOutline => {
            return Err(SettingsDamage::OutlineOnKind);
        }
        Some(value) if holds_interval(value) => return Err(SettingsDamage::BadInterval),
        Some(value) => Some(read_set(value).ok_or(SettingsDamage::BadSet)?),
    };
    let material = match map.get("material") {
        None => None,
        Some(value) => Some(read_link(value).ok_or(SettingsDamage::BadLink)?),
    };
    let overrides = match (map.get("overrides"), &material) {
        (None, _) => Overrides::default(),
        (Some(_), None) => return Err(SettingsDamage::BadOverride),
        (Some(value), Some(_)) => read_overrides(value, kind)?,
    };
    Ok(StoredSettings {
        main,
        interval,
        outline,
        material,
        overrides,
    })
}

impl StoredSettings {
    /// Own numbers of an operation of kind `kind`: `main`, and for Engrave + Outline the outline
    /// (`main` when not given); no link.
    #[must_use]
    pub fn own(kind: OperationKind, main: Settings, outline: Option<Settings>) -> Self {
        Self {
            main,
            interval: None,
            outline: (kind == OperationKind::EngraveOutline).then(|| outline.unwrap_or(main)),
            material: None,
            overrides: Overrides::default(),
        }
    }

    /// The plain values to store (`op_settings`); [`read_settings`] reads them back equal.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let mut map = Map::new();
        let mut main = set_value(&self.main);
        if let (Some(interval), Value::Object(fields)) = (self.interval, &mut main) {
            fields.insert(KEY_INTERVAL.to_owned(), json!(interval.mm()));
        }
        map.insert("main".to_owned(), main);
        if let Some(outline) = &self.outline {
            map.insert("outline".to_owned(), set_value(outline));
        }
        if let Some(link) = &self.material {
            let mut fields = Map::new();
            fields.insert("id".to_owned(), json!(link.id));
            fields.insert("name".to_owned(), json!(link.name));
            if let Some(from) = link.outline_from {
                fields.insert("outline_from".to_owned(), json!(from.as_str()));
            }
            fields.insert("modified".to_owned(), json!(link.modified));
            map.insert("material".to_owned(), Value::Object(fields));
            if !self.overrides.is_empty() {
                let mut roles = Map::new();
                for role in [Role::Main, Role::Outline] {
                    let typed = self.overrides.of(role);
                    if !typed.is_empty() {
                        roles.insert(role.as_str().to_owned(), override_value(typed));
                    }
                }
                map.insert("overrides".to_owned(), Value::Object(roles));
            }
        }
        Value::Object(map)
    }

    /// The stored interval of an operation of kind `kind`: the default when none is stored,
    /// `None` for a kind that does not hatch.
    #[must_use]
    pub fn stored_interval(&self, kind: OperationKind) -> Option<Interval> {
        hatches(kind).then(|| self.interval.unwrap_or_default())
    }

    /// The stored set of `role` for an operation of kind `kind`: the outline falls back to
    /// `main`; `None` for the outline of another kind.
    #[must_use]
    pub fn stored_set(&self, kind: OperationKind, role: Role) -> Option<Settings> {
        match role {
            Role::Main => Some(self.main),
            Role::Outline => {
                (kind == OperationKind::EngraveOutline).then(|| self.outline.unwrap_or(self.main))
            }
        }
    }
}

#[cfg(test)]
#[path = "operation_settings_tests.rs"]
mod tests;
