//! The Settings block of selected operations (`specs/0028` criteria 24 to 27; ADR 0054 §2, §3):
//! the state each operation's numbers are in through the laser rule, and the writes of Use
//! material, Own numbers, Edit settings and Reset to material, each one step.

use curvyo_document_core::{NodeId, OperationSnapshot, SettingsStep, SettingsWrite};
use curvyo_laser_core::FeedRate;
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::{
    Field, Interval, Passes, PowerPercent, Role, SetOverride, Settings, SettingsDamage,
    SettingsRead, SettingsState, StoredSettings, TypedNumber, edit_number, read_options,
    read_settings, reset_to_material, resolve, use_material, use_own_numbers,
};

use super::super::Session;
use super::super::materials::short_name;

/// The longest skip notice (criterion 27).
const SKIP_NOTICE_CHARS: usize = 70;

/// The number typed into a settings field, or its chip text (UX notes §8.3): power 1 to 100
/// rounded to 0.1 %, speed a whole 1 to the machine's maximum feed, passes 1 to 20.
///
/// # Errors
/// The chip text.
pub(in crate::session) fn typed_number(
    field: Field,
    value: f64,
    max_feed: FeedRate,
) -> Result<TypedNumber, String> {
    if !value.is_finite() {
        return Err(match field {
            Field::Power => "Enter a number.".to_owned(),
            Field::Speed | Field::Passes => "Enter a whole number.".to_owned(),
        });
    }
    let whole = || {
        // a finite, whole, non-negative number below 2^53
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        (value.fract() == 0.0 && (0.0..9.0e15).contains(&value)).then_some(value as u64)
    };
    match field {
        Field::Power => (1.0..=100.0)
            .contains(&value)
            .then(|| PowerPercent::rounded(value))
            .flatten()
            .map(TypedNumber::Power)
            .ok_or_else(|| "Power is 1 to 100 %.".to_owned()),
        Field::Speed => {
            let n = whole().ok_or_else(|| "Enter a whole number.".to_owned())?;
            FeedRate::new(n)
                .filter(|speed| speed.get() <= max_feed.get())
                .map(TypedNumber::Speed)
                .ok_or_else(|| format!("Speed is 1 to {} mm/min.", max_feed.get()))
        }
        Field::Passes => {
            let n = whole().ok_or_else(|| "Enter a whole number.".to_owned())?;
            Passes::new(n)
                .map(TypedNumber::Passes)
                .ok_or_else(|| "Passes are 1 to 20.".to_owned())
        }
    }
}

/// The field of its stored name.
pub(in crate::session) fn field_of(name: &str) -> Option<Field> {
    match name {
        "power" => Some(Field::Power),
        "speed" => Some(Field::Speed),
        "passes" => Some(Field::Passes),
        _ => None,
    }
}

/// What is typed into an operation with No settings before power and speed are both there
/// (`specs/0028` UX notes §8.3): the numbers, and the Interval of a hatching kind (`specs/0111`
/// criterion 19), written together with them.
#[derive(Debug, Clone, Copy, Default)]
pub(in crate::session) struct SettingsDraft {
    set: SetOverride,
    interval: Option<Interval>,
}

/// An operation of the active laser page with what its settings read.
pub(in crate::session) struct SettingsTarget {
    pub(in crate::session) id: NodeId,
    pub(in crate::session) name: String,
    pub(in crate::session) kind: OperationKind,
    pub(in crate::session) read: SettingsRead,
    pub(in crate::session) state: SettingsState,
}

/// What operation `op` of kind `kind` reads for its numbers: damaged `op_options` make the whole
/// operation read as Damaged settings, so nothing runs and no material command writes it (ADR 0056
/// §6; the kind of damage is not shown).
pub(in crate::session) fn operation_read(
    op: &OperationSnapshot,
    kind: OperationKind,
) -> SettingsRead {
    if read_options(op.options.as_ref(), kind).is_err() {
        return SettingsRead::Damaged(SettingsDamage::NotAMap);
    }
    read_settings(op.settings.as_ref(), kind)
}

fn plain(stored: &StoredSettings) -> Option<serde_json::Map<String, serde_json::Value>> {
    match stored.to_value() {
        serde_json::Value::Object(map) => Some(map),
        _ => None,
    }
}

impl Session {
    /// The operation `id` of the active laser page with its settings, `None` for another page, a
    /// grid row or an unknown kind (those have no settings line).
    pub(in crate::session) fn settings_target(&self, id: NodeId) -> Option<SettingsTarget> {
        let active = self.active_operations()?;
        let op = active.operations.iter().find(|op| op.id == id)?;
        if op.grid {
            return None;
        }
        let kind = OperationKind::parse(op.kind.as_deref()?)?;
        let read = operation_read(op, kind);
        let profile = self.active_profile_id().unwrap_or_default();
        let state = resolve(&read, kind, self.materials.library().reads(), &profile);
        Some(SettingsTarget {
            id,
            name: op.name.clone(),
            kind,
            read,
            state,
        })
    }

    /// The machine's maximum feed of the active laser page.
    pub(in crate::session) fn active_max_feed(&self) -> FeedRate {
        self.active_copy()
            .map_or(FeedRate::DEFAULT, |copy| copy.read.copy.values.max_feed)
    }

    /// Picks material `material` for the operations `ids` (criteria 25, 27), one step "Use
    /// material", typed numbers cleared, also on a re-pick. An operation whose kind has no set in
    /// the material is skipped and named in the notice. Whether anything was written.
    pub fn use_material_on(&mut self, ids: &[NodeId], material: &str) -> bool {
        self.operations.notice = None;
        let Some(found) = self.materials.library().find(material).cloned() else {
            return false;
        };
        let profile = self.active_profile_id().unwrap_or_default();
        let mut writes = Vec::new();
        let mut skipped: Vec<(String, &'static str)> = Vec::new();
        for target in ids.iter().filter_map(|&id| self.settings_target(id)) {
            match use_material(&found, &profile, target.kind, &target.read) {
                Ok(mut stored) => {
                    if target.read == SettingsRead::None {
                        stored.interval = self.draft_interval(target.id).or(stored.interval);
                    }
                    writes.push(SettingsWrite {
                        operation: target.id,
                        value: plain(&stored),
                    });
                }
                Err(kind) => skipped.push((target.name, kind.label())),
            }
        }
        self.operations.notice = skip_notice(&skipped, &found.name);
        if writes.iter().any(|write| {
            self.operations
                .settings_draft
                .is_some_and(|(op, _)| op == write.operation)
        }) {
            self.operations.settings_draft = None;
        }
        self.write_settings_step(SettingsStep::UseMaterial, &writes)
    }

    /// "Own numbers" (or "No material") in the pick (criterion 25): the link and typed numbers of
    /// the operations `ids` go, the numbers that ran stay as own numbers. One step "Use own
    /// numbers". Whether anything was written.
    pub fn use_own_numbers_on(&mut self, ids: &[NodeId]) -> bool {
        self.operations.notice = None;
        let writes: Vec<SettingsWrite> = ids
            .iter()
            .filter_map(|&id| self.settings_target(id))
            .filter_map(|target| {
                let stored = use_own_numbers(&target.read, &target.state, target.kind)?;
                Some(SettingsWrite {
                    operation: target.id,
                    value: plain(&stored),
                })
            })
            .collect();
        self.write_settings_step(SettingsStep::UseOwnNumbers, &writes)
    }

    /// Types `value` into the `field` of the `role` set (`main`, `outline`) of operation `id`
    /// (criterion 24, Frage 188 B), one step "Edit settings". Own numbers change; a linked
    /// operation keeps its link and stores the typed number, or drops it when it equals the
    /// material's. With No settings the numbers are kept until power and speed are both typed
    /// (passes 1 when empty). `Ok(false)` when nothing is written.
    ///
    /// # Errors
    /// The chip text of UX notes §8.3.
    pub fn edit_operation_setting(
        &mut self,
        id: NodeId,
        role: &str,
        field: &str,
        value: f64,
    ) -> Result<bool, String> {
        self.operations.notice = None;
        let role = match role {
            "main" => Role::Main,
            "outline" => Role::Outline,
            _ => return Ok(false),
        };
        let field = field_of(field).ok_or_else(|| "Enter a number.".to_owned())?;
        let number = typed_number(field, value, self.active_max_feed())?;
        let Some(target) = self.settings_target(id) else {
            return Ok(false);
        };
        if target.read == SettingsRead::None {
            return Ok(self.type_into_draft(&target, role, number));
        }
        let Some(next) = edit_number(&target.read, &target.state, target.kind, role, number) else {
            return Ok(false);
        };
        let write = SettingsWrite {
            operation: id,
            value: plain(&next),
        };
        Ok(self.write_settings_step(SettingsStep::EditSettings, &[write]))
    }

    /// A number typed into an operation with No settings: kept in the draft until power and
    /// speed are both there, then written as own numbers (UX notes §8.3).
    fn type_into_draft(
        &mut self,
        target: &SettingsTarget,
        role: Role,
        number: TypedNumber,
    ) -> bool {
        if role != Role::Main {
            return false;
        }
        let mut draft = self.draft_of(target.id);
        match number {
            TypedNumber::Power(power) => draft.set.power = Some(power),
            TypedNumber::Speed(speed) => draft.set.speed = Some(speed),
            TypedNumber::Passes(passes) => draft.set.passes = Some(passes),
        }
        let (Some(power), Some(speed)) = (draft.set.power, draft.set.speed) else {
            self.operations.settings_draft = Some((target.id, draft));
            return false;
        };
        self.operations.settings_draft = None;
        let main = Settings {
            power,
            speed,
            passes: draft.set.passes.unwrap_or(Passes::ONE),
        };
        let mut stored = StoredSettings::own(target.kind, main, None);
        stored.interval = draft.interval;
        let write = SettingsWrite {
            operation: target.id,
            value: plain(&stored),
        };
        self.write_settings_step(SettingsStep::EditSettings, &[write])
    }

    /// Reset to material (criterion 24): the typed numbers of operation `id` go, one step.
    /// Whether anything was written.
    pub fn reset_operation_to_material(&mut self, id: NodeId) -> bool {
        self.operations.notice = None;
        let Some(target) = self.settings_target(id) else {
            return false;
        };
        let Some(next) = reset_to_material(&target.read) else {
            return false;
        };
        let write = SettingsWrite {
            operation: id,
            value: plain(&next),
        };
        self.write_settings_step(SettingsStep::ResetToMaterial, &[write])
    }

    /// The numbers typed into an operation with No settings, while power or speed is missing.
    pub(in crate::session) fn settings_draft(&self, id: NodeId) -> Option<SetOverride> {
        self.operations
            .settings_draft
            .and_then(|(op, draft)| (op == id).then_some(draft.set))
    }

    /// The Interval typed into an operation with No settings, until its numbers are written.
    pub(in crate::session) fn draft_interval(&self, id: NodeId) -> Option<Interval> {
        self.operations
            .settings_draft
            .and_then(|(op, draft)| (op == id).then_some(draft.interval)?)
    }

    /// The draft of operation `id`, empty when another operation's draft is kept.
    fn draft_of(&self, id: NodeId) -> SettingsDraft {
        match self.operations.settings_draft {
            Some((op, draft)) if op == id => draft,
            _ => SettingsDraft::default(),
        }
    }

    /// Keeps `interval` typed into operation `id` with No settings until power and speed are
    /// typed (criterion 19): nothing is written, so the result is always `false`.
    pub(in crate::session) fn type_interval_into_draft(
        &mut self,
        id: NodeId,
        interval: Interval,
    ) -> bool {
        let mut draft = self.draft_of(id);
        draft.interval = Some(interval);
        self.operations.settings_draft = Some((id, draft));
        false
    }

    fn write_settings_step(&mut self, step: SettingsStep, writes: &[SettingsWrite]) -> bool {
        if writes.is_empty() {
            return false;
        }
        self.recording_unguarded(|this| {
            let done = this
                .document
                .write_operation_settings(step, writes)
                .unwrap_or(false);
            // A settings or material edit of a Lasered grid's row gives the grid a new id
            // (`specs/0112` criterion 11).
            if done {
                let rows: Vec<NodeId> = writes.iter().map(|w| w.operation).collect();
                this.renew_lasered_grids_of(&rows);
            }
            done
        })
    }
}

/// The skip notice of criterion 27, at most 70 characters: one operation by name, several by
/// count.
fn skip_notice(skipped: &[(String, &'static str)], material: &str) -> Option<String> {
    let (first, kind) = skipped.first()?;
    let text = if skipped.len() == 1 {
        format!(
            "{} skipped: {} has no {kind} set.",
            short_name(first),
            short_name(material)
        )
    } else if skipped.iter().all(|(_, k)| k == kind) {
        format!("{} operations skipped: no {kind} set.", skipped.len())
    } else {
        format!("{} operations skipped: no fitting set.", skipped.len())
    };
    Some(text.chars().take(SKIP_NOTICE_CHARS).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_numbers_are_checked_against_their_ranges() {
        let max = FeedRate::new(6000).unwrap();
        assert_eq!(
            typed_number(Field::Power, 0.5, max),
            Err("Power is 1 to 100 %.".to_owned())
        );
        assert_eq!(
            typed_number(Field::Power, 60.25, max),
            Ok(TypedNumber::Power(PowerPercent::from_tenths(603).unwrap()))
        );
        assert_eq!(
            typed_number(Field::Speed, 6001.0, max),
            Err("Speed is 1 to 6000 mm/min.".to_owned())
        );
        assert_eq!(
            typed_number(Field::Speed, 10.5, max),
            Err("Enter a whole number.".to_owned())
        );
        assert_eq!(
            typed_number(Field::Passes, 21.0, max),
            Err("Passes are 1 to 20.".to_owned())
        );
    }

    #[test]
    fn the_skip_notice_names_one_and_counts_several() {
        assert_eq!(
            skip_notice(&[("Cut 2".to_owned(), "Cut")], "Birch 3 mm").as_deref(),
            Some("Cut 2 skipped: Birch 3 mm has no Cut set.")
        );
        let two = [("Cut 1".to_owned(), "Cut"), ("Cut 2".to_owned(), "Cut")];
        assert_eq!(
            skip_notice(&two, "Birch").as_deref(),
            Some("2 operations skipped: no Cut set.")
        );
        let long = "x".repeat(60);
        assert!(
            skip_notice(&[(long.clone(), "Cut")], &long)
                .unwrap()
                .chars()
                .count()
                <= 70
        );
    }
}
