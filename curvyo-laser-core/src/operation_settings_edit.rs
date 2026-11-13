//! The value Edit settings writes for a typed number or interval (ADR 0054 §3).

use crate::machine_profile::FeedRate;
use crate::material::{Interval, Passes, PowerPercent, Settings};
use crate::operation_kind::OperationKind;
use crate::operation_settings::{Role, SettingsRead, StoredSettings};
use crate::operation_settings_rule::SettingsState;

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
