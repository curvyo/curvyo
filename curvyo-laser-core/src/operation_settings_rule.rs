//! Which numbers an operation runs with and how its `op_settings` changes (ADR 0054 §2, §3;
//! `specs/0028` criteria 24 to 27; ADR 0056 §5): one pure rule over the stored value and the
//! loaded library.

use crate::machine_profile::FeedRate;
use crate::material::{Interval, Material, MaterialRead, Passes, PowerPercent, SetKind, Settings};
use crate::operation_kind::OperationKind;
use crate::operation_settings::{
    Field, MaterialLink, Overrides, Role, SettingsDamage, SettingsRead, StoredSettings,
};

/// The numbers that run: the kind's set and, for Engrave + Outline only, the outline's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunSettings {
    /// The kind's set.
    pub main: Settings,
    /// The outline set, only for Engrave + Outline.
    pub outline: Option<Settings>,
    /// The hatch interval, only for Engrave and Engrave + Outline.
    pub interval: Option<Interval>,
}

impl RunSettings {
    /// The set of `role`, `None` for the outline of a kind without one.
    #[must_use]
    pub const fn of(&self, role: Role) -> Option<Settings> {
        match role {
            Role::Main => Some(self.main),
            Role::Outline => self.outline,
        }
    }

    /// Whether a speed lies above the machine's maximum feed: named on the line, not damage
    /// (`0029` refuses it).
    #[must_use]
    pub fn over_feed(&self, max_feed: FeedRate) -> bool {
        std::iter::once(self.main)
            .chain(self.outline)
            .any(|set| set.speed.get() > max_feed.get())
    }

    fn with(&self, overrides: &Overrides) -> Self {
        Self {
            main: overrides.main.apply(self.main),
            outline: self.outline.map(|outline| overrides.outline.apply(outline)),
            interval: self
                .interval
                .map(|interval| overrides.main.interval.unwrap_or(interval)),
        }
    }
}

/// Why a linked operation runs its stored numbers (criterion 26 (b)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredReason {
    /// No record with the id is loaded here.
    Missing,
    /// The record is damaged.
    Damaged,
    /// The record is of a newer format.
    Newer,
    /// The record is tied to another machine.
    OtherMachine,
    /// The record has no set of this kind.
    NoSet(SetKind),
}

impl StoredReason {
    /// The state line of UX 8.4.
    #[must_use]
    pub fn text(self) -> String {
        match self {
            Self::NoSet(kind) => {
                format!("Material has no {} set, using stored numbers", kind.label())
            }
            Self::Missing | Self::Damaged | Self::Newer | Self::OtherMachine => {
                "Material not found, using stored numbers".to_owned()
            }
        }
    }
}

/// The state of an operation's settings (criterion 26).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsState {
    /// No settings: nothing runs.
    NoSettings,
    /// Damaged settings: nothing runs.
    Damaged(SettingsDamage),
    /// Own numbers.
    Own(RunSettings),
    /// Linked and live: the material's current numbers, typed numbers on top.
    Live {
        /// The material's current name.
        name: String,
        /// Its current numbers.
        material: RunSettings,
        /// What runs: `material` with the typed numbers put over it.
        run: RunSettings,
        /// The typed fields, per role.
        overridden: Vec<(Role, Field)>,
    },
    /// Linked, but the stored numbers stand in for the material, typed numbers on top.
    Stored {
        /// The name stored with the link.
        name: String,
        /// Why.
        reason: StoredReason,
        /// The stored numbers.
        material: RunSettings,
        /// What runs.
        run: RunSettings,
        /// The typed fields, per role.
        overridden: Vec<(Role, Field)>,
    },
}

impl SettingsState {
    /// The numbers that run, `None` for No settings and Damaged settings. `0029` reads this.
    #[must_use]
    pub const fn run(&self) -> Option<&RunSettings> {
        match self {
            Self::NoSettings | Self::Damaged(_) => None,
            Self::Own(run) | Self::Live { run, .. } | Self::Stored { run, .. } => Some(run),
        }
    }

    /// The material's numbers (live or stored) of a linked operation.
    #[must_use]
    pub const fn material(&self) -> Option<&RunSettings> {
        match self {
            Self::Live { material, .. } | Self::Stored { material, .. } => Some(material),
            Self::NoSettings | Self::Damaged(_) | Self::Own(_) => None,
        }
    }

    /// "Differs from `<name>`" while a number is typed over a linked operation (criterion 24).
    #[must_use]
    pub fn differs_text(&self) -> Option<String> {
        match self {
            Self::Live {
                name, overridden, ..
            }
            | Self::Stored {
                name, overridden, ..
            } if !overridden.is_empty() => Some(format!("Differs from {name}")),
            _ => None,
        }
    }
}

/// The set kind an operation kind's main numbers come from (criterion 25).
#[must_use]
pub const fn main_set(kind: OperationKind) -> SetKind {
    match kind {
        OperationKind::Cut | OperationKind::CutOut => SetKind::Cut,
        OperationKind::Score => SetKind::Score,
        OperationKind::Engrave | OperationKind::EngraveOutline => SetKind::Engrave,
    }
}

/// The numbers `material` gives an operation of kind `kind` whose stored interval is `interval`,
/// and the set that fed the outline; `Err` names the missing kind set. A material set holds no
/// interval in this slice, so the stored one stands for it (ADR 0056 §5 link rule).
fn material_run(
    material: &Material,
    kind: OperationKind,
    interval: Option<Interval>,
) -> Result<(RunSettings, Option<SetKind>), SetKind> {
    let needed = main_set(kind);
    let main = *material.set(needed).ok_or(needed)?;
    if kind != OperationKind::EngraveOutline {
        return Ok((
            RunSettings {
                main,
                outline: None,
                interval,
            },
            None,
        ));
    }
    let (outline, from) = material
        .set(SetKind::Score)
        .map_or((main, SetKind::Engrave), |score| (*score, SetKind::Score));
    Ok((
        RunSettings {
            main,
            outline: Some(outline),
            interval,
        },
        Some(from),
    ))
}

/// The stored `main.interval_mm` of `read`, `None` when absent or not readable.
fn kept_interval(read: &SettingsRead) -> Option<Interval> {
    match read {
        SettingsRead::Stored(stored) => stored.interval,
        SettingsRead::None | SettingsRead::Damaged(_) => None,
    }
}

/// The value Use material writes for an operation of kind `kind` whose settings read `previous`:
/// the link with the record's `modified` and its numbers as they are now, no typed numbers
/// (criterion 25, also when the linked material is picked again), the stored interval kept
/// (`specs/0111` criteria 19, 23); `Err` names the set the material lacks, and the operation is
/// skipped.
///
/// # Errors
/// The missing set kind.
pub fn use_material(
    material: &Material,
    kind: OperationKind,
    previous: &SettingsRead,
) -> Result<StoredSettings, SetKind> {
    let interval = kept_interval(previous);
    let (run, outline_from) = material_run(material, kind, interval)?;
    Ok(StoredSettings {
        main: run.main,
        interval,
        outline: run.outline,
        material: Some(MaterialLink {
            id: material.id.clone(),
            name: material.name.clone(),
            outline_from,
            modified: material.modified,
        }),
        overrides: Overrides::default(),
    })
}

/// The live material behind `link` on a page whose profile copy has the id `profile_id`, or why
/// the stored numbers stand in.
fn live_material<'a>(
    link: &MaterialLink,
    kind: OperationKind,
    materials: &'a [MaterialRead],
    profile_id: &str,
    interval: Option<Interval>,
) -> Result<(&'a Material, RunSettings, Option<SetKind>), StoredReason> {
    let material = match materials.iter().find(|m| m.id() == link.id) {
        None => return Err(StoredReason::Missing),
        Some(MaterialRead::Damaged { .. }) => return Err(StoredReason::Damaged),
        Some(MaterialRead::Newer { .. }) => return Err(StoredReason::Newer),
        Some(MaterialRead::Loaded(material)) => material,
    };
    if material.machine.id != profile_id {
        return Err(StoredReason::OtherMachine);
    }
    let (run, from) = material_run(material, kind, interval).map_err(StoredReason::NoSet)?;
    Ok((material, run, from))
}

fn overridden(overrides: &Overrides, kind: OperationKind) -> Vec<(Role, Field)> {
    let roles: &[Role] = if kind == OperationKind::EngraveOutline {
        &[Role::Main, Role::Outline]
    } else {
        &[Role::Main]
    };
    roles
        .iter()
        .flat_map(|&role| {
            overrides
                .of(role)
                .fields()
                .into_iter()
                .map(move |f| (role, f))
        })
        .collect()
}

/// The state of a stored value of an operation of kind `kind` on a page whose profile copy has
/// the id `profile_id`, with the library `materials` (ADR 0054 §2). Pure: the caller passes the
/// loaded library in.
#[must_use]
pub fn resolve(
    read: &SettingsRead,
    kind: OperationKind,
    materials: &[MaterialRead],
    profile_id: &str,
) -> SettingsState {
    let stored = match read {
        SettingsRead::None => return SettingsState::NoSettings,
        SettingsRead::Damaged(damage) => return SettingsState::Damaged(*damage),
        SettingsRead::Stored(stored) => stored,
    };
    let stored_run = RunSettings {
        main: stored.main,
        outline: stored.stored_set(kind, Role::Outline),
        interval: stored.stored_interval(kind),
    };
    let Some(link) = &stored.material else {
        return SettingsState::Own(stored_run);
    };
    let overridden = overridden(&stored.overrides, kind);
    match live_material(
        link,
        kind,
        materials,
        profile_id,
        stored.stored_interval(kind),
    ) {
        Ok((material, numbers, _)) => SettingsState::Live {
            name: material.name.clone(),
            material: numbers,
            run: numbers.with(&stored.overrides),
            overridden,
        },
        Err(reason) => SettingsState::Stored {
            name: link.name.clone(),
            reason,
            material: stored_run,
            run: stored_run.with(&stored.overrides),
            overridden,
        },
    }
}

/// The value to write so the stored numbers beside a live link are the material's again
/// (Frage 187 B+C): `Some` only when the operation is live and the material's numbers, name or
/// `outline_from` differ from the stored ones; the typed numbers are kept. With `guard` (the save
/// refresh) only when the record's `modified` is above the link's, so two computers whose library
/// sync lags do not rewrite each other's numbers at every save (ADR 0054 §3).
#[must_use]
pub fn refresh(
    read: &SettingsRead,
    kind: OperationKind,
    materials: &[MaterialRead],
    profile_id: &str,
    guard: bool,
) -> Option<StoredSettings> {
    let SettingsRead::Stored(stored) = read else {
        return None;
    };
    let link = stored.material.as_ref()?;
    let (material, numbers, outline_from) =
        live_material(link, kind, materials, profile_id, stored.interval).ok()?;
    if guard && material.modified <= link.modified {
        return None;
    }
    let same = stored.main == numbers.main
        && stored.stored_set(kind, Role::Outline) == numbers.outline
        && link.name == material.name
        && link.outline_from == outline_from;
    if same {
        return None;
    }
    Some(StoredSettings {
        main: numbers.main,
        interval: stored.interval,
        outline: numbers.outline,
        material: Some(MaterialLink {
            id: link.id.clone(),
            name: material.name.clone(),
            outline_from,
            modified: material.modified,
        }),
        overrides: stored.overrides,
    })
}

/// One typed number of the Settings block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedNumber {
    /// A power.
    Power(PowerPercent),
    /// A speed.
    Speed(FeedRate),
    /// Passes.
    Passes(Passes),
}

impl TypedNumber {
    const fn put(self, set: Settings) -> Settings {
        match self {
            Self::Power(power) => Settings { power, ..set },
            Self::Speed(speed) => Settings { speed, ..set },
            Self::Passes(passes) => Settings { passes, ..set },
        }
    }

    fn equals(self, set: Settings) -> bool {
        self.put(set) == set
    }
}

/// The value Edit settings writes when `number` is typed into the `role` set of an operation of
/// kind `kind` whose state is `state` (criterion 24, Frage 188 B): own numbers change that
/// number; a linked operation keeps its link and stores the number as typed, or drops the typed
/// number when it equals the material's. `None` when nothing changes, for a role the kind does
/// not have, and for No settings and Damaged settings (the first commit of No settings is
/// [`StoredSettings::own`] once power and speed are set).
#[must_use]
pub fn edit_number(
    stored: &SettingsRead,
    state: &SettingsState,
    kind: OperationKind,
    role: Role,
    number: TypedNumber,
) -> Option<StoredSettings> {
    let SettingsRead::Stored(stored) = stored else {
        return None;
    };
    let current = stored.stored_set(kind, role)?;
    let mut next = stored.clone();
    if stored.material.is_none() {
        let changed = number.put(current);
        match role {
            Role::Main => next.main = changed,
            Role::Outline => next.outline = Some(changed),
        }
    } else {
        let material = state.material()?.of(role)?;
        let typed = next.overrides.of_mut(role);
        let keep = !number.equals(material);
        match number {
            TypedNumber::Power(power) => typed.power = keep.then_some(power),
            TypedNumber::Speed(speed) => typed.speed = keep.then_some(speed),
            TypedNumber::Passes(passes) => typed.passes = keep.then_some(passes),
        }
    }
    (next != *stored).then_some(next)
}

/// The value Edit settings writes when `interval` is typed into an operation of kind `kind`
/// (`specs/0111` criterion 19): `main.interval_mm` on the operation, also when it is linked (no
/// material holds an interval in this slice), and an interval typed over the link before goes.
/// `None` when nothing changes, for a kind that does not hatch, and for No settings and Damaged
/// settings.
#[must_use]
pub fn edit_interval(
    stored: &SettingsRead,
    kind: OperationKind,
    interval: Interval,
) -> Option<StoredSettings> {
    let SettingsRead::Stored(stored) = stored else {
        return None;
    };
    stored.stored_interval(kind)?;
    let mut next = stored.clone();
    next.interval = Some(interval);
    next.overrides.main.interval = None;
    (next != *stored).then_some(next)
}

/// The value Reset to material writes: the link and stored numbers without typed numbers;
/// `None` when nothing is typed.
#[must_use]
pub fn reset_to_material(stored: &SettingsRead) -> Option<StoredSettings> {
    let SettingsRead::Stored(stored) = stored else {
        return None;
    };
    (stored.material.is_some() && !stored.overrides.is_empty()).then(|| StoredSettings {
        overrides: Overrides::default(),
        ..stored.clone()
    })
}

/// The value Use own numbers writes: no link, the numbers that ran (typed ones included) as own
/// numbers (criterion 25); `None` when the operation has no link.
#[must_use]
pub fn use_own_numbers(
    stored: &SettingsRead,
    state: &SettingsState,
    kind: OperationKind,
) -> Option<StoredSettings> {
    let SettingsRead::Stored(stored) = stored else {
        return None;
    };
    stored.material.as_ref()?;
    let run = state.run()?;
    let mut own = StoredSettings::own(kind, run.main, run.outline);
    own.interval = stored.overrides.main.interval.or(stored.interval);
    Some(own)
}

#[cfg(test)]
#[path = "operation_settings_rule_tests.rs"]
mod tests;
