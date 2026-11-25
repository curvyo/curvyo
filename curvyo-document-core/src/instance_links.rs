//! Applies one value change to a clone's own values through the sketch's link groups
//! (`specs/0099-sketch-pages/adrs.md` A14, criteria 33 and 34).

use crate::instance_value::{ExposedKey, ExposedParam, KeySet, ValueSet};

/// The limit that held an edited value, for the notice.
#[derive(Debug, Clone, PartialEq)]
pub struct Held {
    /// The label of the value whose limit held: the edited one's own, or a partner's.
    pub by: String,
    /// Whether it was the lower limit.
    pub minimum: bool,
    /// The number the value was set to (millimetres or degrees).
    pub number: f64,
}

/// The outcome of [`linked_change`].
#[derive(Debug, Clone, PartialEq)]
pub struct LinkedChange {
    /// The clone's whole new set of own values.
    pub own: ValueSet,
    /// The keys it holds unlinked afterwards.
    pub unlinked: KeySet,
    /// The limit that held the edited value, `None` when none did.
    pub held: Option<Held>,
}

fn param<'a>(exposed: &'a [ExposedParam], key: &ExposedKey) -> Option<&'a ExposedParam> {
    exposed.iter().find(|param| &param.key == key)
}

/// `number` clamped into the limits of every target at once, and the limit that held; `None`
/// when the targets' limits do not overlap.
fn clamp_all(
    targets: &[&ExposedParam],
    edited: &ExposedParam,
    number: f64,
) -> Option<(f64, Option<Held>)> {
    let low = targets
        .iter()
        .filter_map(|param| param.min.map(|min| (min, *param)))
        .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| prefer(edited, a.1, b.1)));
    let high = targets
        .iter()
        .filter_map(|param| param.max.map(|max| (max, *param)))
        .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| prefer(edited, b.1, a.1)));
    if let (Some((min, _)), Some((max, _))) = (low, high)
        && min > max
    {
        return None;
    }
    let held = match (low, high) {
        (Some((min, by)), _) if number < min => Some(Held {
            by: by.label.clone(),
            minimum: true,
            number: min,
        }),
        (_, Some((max, by))) if number > max => Some(Held {
            by: by.label.clone(),
            minimum: false,
            number: max,
        }),
        _ => None,
    };
    Some((held.as_ref().map_or(number, |held| held.number), held))
}

/// Among two members with the same bound the edited value's own limit is named (its own notice).
fn prefer(edited: &ExposedParam, a: &ExposedParam, b: &ExposedParam) -> std::cmp::Ordering {
    (a.key == edited.key).cmp(&(b.key == edited.key))
}

/// Sets the exposed `key` of one clone to `number` (millimetres or degrees). Without `only_this`
/// every member of its link group that this clone does not hold unlinked takes the same value,
/// clamped into the intersection of their limits (each into its own when they do not overlap);
/// with `only_this` only `key` changes, and it is marked unlinked when it is in a group and its
/// value changes from the one the clone shows (`own`, else `inherited`, else the sketch's). An
/// unknown key leaves everything as it is.
#[must_use]
pub fn linked_change(
    exposed: &[ExposedParam],
    own: &ValueSet,
    inherited: &ValueSet,
    unlinked: &KeySet,
    key: &ExposedKey,
    number: f64,
    only_this: bool,
) -> LinkedChange {
    let unchanged = LinkedChange {
        own: own.clone(),
        unlinked: unlinked.clone(),
        held: None,
    };
    let Some(edited) = param(exposed, key) else {
        return unchanged;
    };
    let alone = only_this || edited.group.is_none() || unlinked.contains(key);
    let targets: Vec<&ExposedParam> = if alone {
        vec![edited]
    } else {
        exposed
            .iter()
            .filter(|other| {
                other.key == *key || (other.group == edited.group && !unlinked.contains(&other.key))
            })
            .collect()
    };
    let mut next = own.clone();
    let held = if let Some((clamped, held)) = clamp_all(&targets, edited, number) {
        for target in &targets {
            next.insert(target.key.clone(), target.value(clamped));
        }
        held
    } else {
        let mut held = None;
        for target in &targets {
            let (clamped, was) = target.clamp(number);
            if target.key == *key && was {
                held = Some(Held {
                    by: target.label.clone(),
                    minimum: clamped > number,
                    number: clamped,
                });
            }
            next.insert(target.key.clone(), target.value(clamped));
        }
        held
    };
    let mut marks = unlinked.clone();
    if only_this && edited.group.is_some() {
        let before = own
            .get(key)
            .or_else(|| inherited.get(key))
            .map_or(edited.default, crate::InstanceValue::number);
        let after = next.get(key).map_or(before, crate::InstanceValue::number);
        if (after - before).abs() > 1e-9 {
            marks.insert(key.clone());
        }
    }
    LinkedChange {
        own: next,
        unlinked: marks,
        held,
    }
}

/// Back to the sketch's value of `key` (its own value and its unlinked mark go, the partners keep
/// theirs), or with `None` Original's: no own value and no mark.
#[must_use]
pub fn linked_reset(
    own: &ValueSet,
    unlinked: &KeySet,
    key: Option<&ExposedKey>,
) -> (ValueSet, KeySet) {
    match key {
        Some(key) => {
            let (mut values, mut marks) = (own.clone(), unlinked.clone());
            values.remove(key);
            marks.remove(key);
            (values, marks)
        }
        None => (ValueSet::default(), KeySet::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance_value::{InstanceKind, InstanceValue};

    fn key(name: &str) -> ExposedKey {
        ExposedKey::parse(&format!("d:{name}")).unwrap()
    }

    fn param(name: &str, group: Option<usize>, min: Option<f64>, max: Option<f64>) -> ExposedParam {
        ExposedParam {
            key: key(name),
            label: name.to_owned(),
            kind: InstanceKind::Length,
            min,
            max,
            default: 40.0,
            link: group.map(|g| g.to_string()),
            group,
        }
    }

    fn set(pairs: &[(&str, f64)]) -> ValueSet {
        let mut values = ValueSet::default();
        for (name, mm) in pairs {
            values.insert(key(name), InstanceValue::from_mm(*mm));
        }
        values
    }

    fn number(values: &ValueSet, name: &str) -> Option<f64> {
        values.get(&key(name)).map(InstanceValue::number)
    }

    fn three() -> Vec<ExposedParam> {
        vec![
            param("r1", Some(0), None, Some(60.0)),
            param("r2", Some(0), None, Some(50.0)),
            param("w", None, None, None),
        ]
    }

    /// Criterion 33: a coupled edit gives every member the same value, clamped into the
    /// intersection of their limits, and names the member whose limit held.
    #[test]
    fn a_coupled_edit_sets_every_member_within_the_tightest_limit() {
        let none = KeySet::default();
        let empty = ValueSet::default();
        let change = linked_change(&three(), &empty, &empty, &none, &key("r1"), 45.0, false);
        assert_eq!(
            (number(&change.own, "r1"), number(&change.own, "r2")),
            (Some(45.0), Some(45.0))
        );
        assert_eq!(number(&change.own, "w"), None, "not in the group");
        assert!(change.held.is_none() && change.unlinked.is_empty());
        let held = linked_change(&three(), &empty, &empty, &none, &key("r1"), 90.0, false);
        assert_eq!(number(&held.own, "r2"), Some(50.0));
        assert_eq!(
            held.held,
            Some(Held {
                by: "r2".into(),
                minimum: false,
                number: 50.0
            }),
            "the partner's maximum held"
        );
    }

    /// Criterion 33, finding F2: members whose limits do not overlap are each clamped into their
    /// own, so they differ; nothing is refused.
    #[test]
    fn limits_that_do_not_overlap_clamp_each_member_into_its_own() {
        let exposed = vec![
            param("a", Some(0), Some(60.0), None),
            param("b", Some(0), None, Some(50.0)),
        ];
        let empty = ValueSet::default();
        let change = linked_change(
            &exposed,
            &empty,
            &empty,
            &KeySet::default(),
            &key("a"),
            55.0,
            false,
        );
        assert_eq!(
            (number(&change.own, "a"), number(&change.own, "b")),
            (Some(60.0), Some(50.0))
        );
        assert_eq!(change.held.map(|held| held.by), Some("a".into()));
    }

    /// Criterion 34: only this changes one value and marks it unlinked; an unchanged value is not
    /// marked; an unlinked value edits alone; its partners keep coupling.
    #[test]
    fn only_this_unlinks_a_changed_value_and_an_unlinked_value_edits_alone() {
        let empty = ValueSet::default();
        let none = KeySet::default();
        let alone = linked_change(&three(), &empty, &empty, &none, &key("r1"), 30.0, true);
        assert_eq!(
            (number(&alone.own, "r1"), number(&alone.own, "r2")),
            (Some(30.0), None)
        );
        assert!(alone.unlinked.contains(&key("r1")));
        let same = linked_change(&three(), &empty, &empty, &none, &key("r1"), 40.0, true);
        assert!(
            same.unlinked.is_empty(),
            "back to the old value unlinks nothing"
        );
        let ungrouped = linked_change(&three(), &empty, &empty, &none, &key("w"), 30.0, true);
        assert!(
            ungrouped.unlinked.is_empty(),
            "no mark the read would ignore"
        );
        let later = linked_change(
            &three(),
            &alone.own,
            &empty,
            &alone.unlinked,
            &key("r1"),
            35.0,
            false,
        );
        assert_eq!(
            (number(&later.own, "r1"), number(&later.own, "r2")),
            (Some(35.0), None)
        );
        let partner = linked_change(
            &three(),
            &later.own,
            &empty,
            &later.unlinked,
            &key("r2"),
            20.0,
            false,
        );
        assert_eq!(
            (number(&partner.own, "r1"), number(&partner.own, "r2")),
            (Some(35.0), Some(20.0))
        );
        let inherited = set(&[("r1", 30.0)]);
        let from_up = linked_change(&three(), &empty, &inherited, &none, &key("r1"), 30.0, true);
        assert!(
            from_up.unlinked.is_empty(),
            "compared with the inherited value"
        );
    }

    /// Criterion 34: Back resets only that value and relinks it; Original's drops all.
    #[test]
    fn back_resets_one_value_and_its_mark_and_originals_drops_all() {
        let own = set(&[("r1", 30.0), ("r2", 45.0)]);
        let marks: KeySet = [key("r1")].into_iter().collect();
        let (values, left) = linked_reset(&own, &marks, Some(&key("r1")));
        assert_eq!(
            (number(&values, "r1"), number(&values, "r2")),
            (None, Some(45.0))
        );
        assert!(left.is_empty());
        let (values, left) = linked_reset(&own, &marks, None);
        assert!(values.is_empty() && left.is_empty());
    }
}
