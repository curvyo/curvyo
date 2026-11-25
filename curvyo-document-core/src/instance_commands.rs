//! The commands of sketch instances (`specs/0099-sketch-pages/adrs.md` A6): expose a value on the
//! sketch, give a clone own values, and the outline a clone would draw with values not yet
//! written. One commit each.

use crate::clone_model::CloneSnapshot;
use crate::document::{Document, OBJECTS_TREE};
use crate::instance_codec::{
    self, ExposedEntry, KEY_INSTANCE_FALLBACK, KEY_INSTANCE_PARAMS, decode_entry,
};
use crate::instance_value::{ExposedKey, ExposedParam, InstanceValue, ValueSet};
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::primitive_model::ObjectSnapshot;

/// The label of [`Document::expose_parameter`].
pub const EXPOSE_PARAMETER_COMMIT_LABEL: &str = "expose_parameter";
/// The label of [`Document::unexpose_parameter`].
pub const UNEXPOSE_PARAMETER_COMMIT_LABEL: &str = "unexpose_parameter";
/// The label of [`Document::set_exposed_limits`].
pub const CHANGE_EXPOSED_PARAMETER_COMMIT_LABEL: &str = "change_exposed_parameter";
/// The label of dropping one own value of a clone.
pub const RESET_PARAMETER_COMMIT_LABEL: &str = "reset_parameter";
/// The label of dropping every own value of the clones of a selection.
pub const RESET_PARAMETERS_COMMIT_LABEL: &str = "reset_parameters";
/// The label of writing every current value as an own value.
pub const OWN_PARAMETERS_COMMIT_LABEL: &str = "own_parameters";

/// Why an instance command did nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InstanceError {
    /// The sketch is gone or is not a sketch.
    #[error("no such sketch")]
    NoSuchSketch,
    /// The clone is gone, is not a clone or does not follow a sketch.
    #[error("no such sketch clone")]
    NoSuchClone,
    /// The key names no parameter or driving dimension of a length or angle unit, or is exposed
    /// already.
    #[error("this value cannot be exposed")]
    NotExposable,
    /// The key is not exposed.
    #[error("this value is not exposed")]
    NotExposed,
    /// Another exposed value has this name, or the name is not 1 to 64 characters.
    #[error("the name is in use or invalid")]
    BadName,
}

/// What one clone gets written by [`Document::write_instance_values`].
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceWrite {
    /// The clone.
    pub clone: NodeId,
    /// Its whole new set of own values.
    pub own: ValueSet,
    /// The last value set a drag step solved, saved as the fallback when `own` has no solution.
    pub last_solvable: Option<ValueSet>,
}

impl Document {
    /// The clone `id` with its chain read, when it follows a sketch.
    fn sketch_clone(&self, id: NodeId) -> Option<(CloneSnapshot, crate::SketchSnapshot)> {
        let clone = self.read_cached_clone(id)?;
        let sketch = self.sketch(clone.chain.root)?;
        Some((clone, sketch))
    }

    /// Exposes `key` of `sketch` so clones may set their own value, in one commit. A dimension
    /// takes `name` as the label of the clone's field.
    ///
    /// # Errors
    /// [`InstanceError::NoSuchSketch`], [`InstanceError::NotExposable`] and
    /// [`InstanceError::BadName`]; nothing is written.
    pub fn expose_parameter(
        &self,
        sketch: NodeId,
        key: &ExposedKey,
        name: Option<&str>,
    ) -> Result<(), InstanceError> {
        let snapshot = self.sketch(sketch).ok_or(InstanceError::NoSuchSketch)?;
        let meta = self
            .sketch_meta(sketch)
            .map_err(|_| InstanceError::NoSuchSketch)?;
        if snapshot.exposed.iter().any(|param| &param.key == key) {
            return Err(InstanceError::NotExposable);
        }
        let target = crate::sketch_instance::exposed_of(
            &snapshot.graph,
            &[ExposedEntry {
                key: key.clone(),
                name: name.map(str::to_owned),
                min: None,
                max: None,
            }],
        );
        if target.is_empty() {
            return Err(InstanceError::NotExposable);
        }
        let stored_name = if key.as_str().starts_with("d:") {
            let name = name.ok_or(InstanceError::BadName)?;
            check_name(&snapshot.exposed, key, name)?;
            Some(name.to_owned())
        } else {
            None
        };
        let mut items = instance_codec::read_exposed_raw(&meta);
        items.push(
            ExposedEntry {
                key: key.clone(),
                name: stored_name,
                min: None,
                max: None,
            }
            .to_json(),
        );
        instance_codec::write_exposed_raw(&meta, &items);
        self.commit_with_label(EXPOSE_PARAMETER_COMMIT_LABEL);
        Ok(())
    }

    /// Stops exposing `key`: the entry and, in the same commit, every own value and fallback of the
    /// clones of `sketch` for it go (Undo brings them back, `0099` criterion 27).
    ///
    /// # Errors
    /// [`InstanceError::NoSuchSketch`] and [`InstanceError::NotExposed`]; nothing is written.
    pub fn unexpose_parameter(
        &self,
        sketch: NodeId,
        key: &ExposedKey,
    ) -> Result<(), InstanceError> {
        self.sketch(sketch).ok_or(InstanceError::NoSuchSketch)?;
        let meta = self
            .sketch_meta(sketch)
            .map_err(|_| InstanceError::NoSuchSketch)?;
        let mut items = instance_codec::read_exposed_raw(&meta);
        let before = items.len();
        items.retain(|item| decode_entry(item).is_none_or(|entry| &entry.key != key));
        if items.len() == before {
            return Err(InstanceError::NotExposed);
        }
        instance_codec::write_exposed_raw(&meta, &items);
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let followers: Vec<NodeId> = self
            .read_objects()
            .objects
            .iter()
            .filter_map(|object| match object {
                ObjectSnapshot::Clone(clone) if clone.chain.root == sketch => Some(clone.id()),
                _ => None,
            })
            .collect();
        for clone in followers {
            let Ok(clone_meta) = tree.get_meta(tree_id_of(clone)) else {
                continue;
            };
            for register in [KEY_INSTANCE_PARAMS, KEY_INSTANCE_FALLBACK] {
                let mut values = instance_codec::read_values(&clone_meta, register);
                if values.get(key).is_some() {
                    values.remove(key);
                    instance_codec::write_values(&clone_meta, register, &values);
                    remove_raw(&clone_meta, register, key);
                }
            }
        }
        self.commit_with_label(UNEXPOSE_PARAMETER_COMMIT_LABEL);
        Ok(())
    }

    /// Sets the limits (`min`, `max` in millimetres or degrees of the value's unit, `None` for
    /// unlimited) and, for a dimension, the name of the exposed `key`, in one commit.
    ///
    /// # Errors
    /// [`InstanceError::NoSuchSketch`], [`InstanceError::NotExposed`] and
    /// [`InstanceError::BadName`]; nothing is written.
    pub fn set_exposed_limits(
        &self,
        sketch: NodeId,
        key: &ExposedKey,
        min: Option<f64>,
        max: Option<f64>,
        name: Option<&str>,
    ) -> Result<(), InstanceError> {
        let snapshot = self.sketch(sketch).ok_or(InstanceError::NoSuchSketch)?;
        let meta = self
            .sketch_meta(sketch)
            .map_err(|_| InstanceError::NoSuchSketch)?;
        let param = snapshot
            .exposed
            .iter()
            .find(|param| &param.key == key)
            .ok_or(InstanceError::NotExposed)?;
        let value = |number: f64| match param.kind {
            crate::InstanceKind::Length => InstanceValue::from_mm(number),
            crate::InstanceKind::Angle => InstanceValue::from_deg(number),
        };
        let mut items = instance_codec::read_exposed_raw(&meta);
        let at = items
            .iter()
            .position(|item| decode_entry(item).is_some_and(|entry| &entry.key == key))
            .ok_or(InstanceError::NotExposed)?;
        let mut entry = decode_entry(&items[at]).ok_or(InstanceError::NotExposed)?;
        if key.as_str().starts_with("d:")
            && let Some(name) = name
        {
            check_name(&snapshot.exposed, key, name)?;
            entry.name = Some(name.to_owned());
        }
        entry.min = min.filter(|n| n.is_finite()).map(value);
        entry.max = max.filter(|n| n.is_finite()).map(value);
        items[at] = entry.to_json();
        instance_codec::write_exposed_raw(&meta, &items);
        self.commit_with_label(CHANGE_EXPOSED_PARAMETER_COMMIT_LABEL);
        Ok(())
    }

    /// Writes the own values of every clone of `writes` in one commit under `label`
    /// (`subject` names the value of a single-value step for the history row). A set with no
    /// solution also writes the fallback (`0099` criterion 26): the write's last solvable set, else
    /// the clone's previous own set when that solved, else the fallback it has; a set that solves
    /// removes it.
    ///
    /// # Errors
    /// [`InstanceError::NoSuchClone`]; nothing is written.
    pub fn write_instance_values(
        &self,
        writes: &[InstanceWrite],
        label: &str,
        subject: Option<&ExposedKey>,
    ) -> Result<(), InstanceError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut plan = Vec::with_capacity(writes.len());
        for write in writes {
            let (clone, sketch) = self
                .sketch_clone(write.clone)
                .ok_or(InstanceError::NoSuchClone)?;
            let solvable = |own: &ValueSet| {
                let effective = clone.chain.inherited_values.overlaid(own);
                effective.is_empty() || self.instance_solves.solved(&sketch, &effective).is_ok()
            };
            let fallback = if solvable(&write.own) {
                None
            } else if let Some(last) = write.last_solvable.as_ref().filter(|set| solvable(set)) {
                Some(last.clone())
            } else if clone.chain.own_params && solvable(&clone.chain.own_values) {
                Some(clone.chain.own_values.clone())
            } else {
                let meta = tree
                    .get_meta(tree_id_of(write.clone))
                    .map_err(|_| InstanceError::NoSuchClone)?;
                let kept = instance_codec::read_values(&meta, KEY_INSTANCE_FALLBACK);
                (!kept.is_empty()).then_some(kept)
            };
            plan.push((write.clone, write.own.clone(), fallback));
        }
        for (clone, own, fallback) in plan {
            let meta = tree
                .get_meta(tree_id_of(clone))
                .map_err(|_| InstanceError::NoSuchClone)?;
            instance_codec::write_values(&meta, KEY_INSTANCE_PARAMS, &own);
            match fallback {
                Some(set) if !own.is_empty() => {
                    instance_codec::write_values(&meta, KEY_INSTANCE_FALLBACK, &set);
                }
                _ => {
                    instance_codec::write_values(
                        &meta,
                        KEY_INSTANCE_FALLBACK,
                        &ValueSet::default(),
                    );
                }
            }
        }
        match subject {
            Some(key) => self.commit_with_label_and(
                label,
                &[(crate::KEY_SUBJECT_PARAM, key.as_str().to_owned())],
            ),
            None => self.commit_with_label(label),
        }
        Ok(())
    }

    /// The clone `id` as it would read with `own` as its own values, without writing: the one
    /// clone a field or knob drag previews. `None` when the clone does not follow a sketch or the
    /// values have no solution (the caller keeps the last solvable one).
    #[must_use]
    pub fn instance_preview(&self, id: NodeId, own: &ValueSet) -> Option<CloneSnapshot> {
        let (clone, sketch) = self.sketch_clone(id)?;
        let kept = crate::sketch_instance::filtered(&sketch.exposed, own);
        let set = clone.chain.inherited_values.overlaid(&kept);
        let composed = clone.chain.composed;
        let stretch = (composed.a * composed.a
            + composed.b * composed.b
            + composed.c * composed.c
            + composed.d * composed.d)
            .sqrt();
        let outline = if set.is_empty() {
            std::sync::Arc::new(sketch.resolved.clone())
        } else {
            self.instance_solves
                .preview_outline(&sketch, &set, stretch)?
        };
        let mut chain = (*clone.chain).clone();
        chain.own_params = !kept.is_empty();
        chain.own_values = kept;
        chain.no_solution = false;
        chain.root_outline = outline;
        let mut shown = CloneSnapshot::new(id, chain);
        shown.resolved.style = clone.resolved.style.clone();
        Some(shown)
    }

    /// The exposed values of the sketch `id`, in stored order.
    #[must_use]
    pub fn exposed_params(&self, id: NodeId) -> Vec<ExposedParam> {
        self.sketch(id)
            .map(|sketch| sketch.exposed)
            .unwrap_or_default()
    }

    /// How many value sets the instance cache holds (a diagnostic for tests).
    #[must_use]
    pub fn instance_cached_sets(&self) -> usize {
        self.instance_solves.cached_sets()
    }

    /// How many sketch solves the instance cache has run (a diagnostic for tests and the PR).
    #[must_use]
    pub fn instance_solve_count(&self) -> usize {
        self.instance_solves.solve_count()
    }
}

/// Removes the raw entry `key` from the register `register` of `meta` (one that decoded and was
/// rewritten by `write_values` has already been replaced; this drops the raw copy of a value that
/// the typed write kept).
fn remove_raw(meta: &loro::LoroMap, register: &str, key: &ExposedKey) {
    let mut raw = instance_codec::read_values_raw(meta, register);
    if raw.remove(key.as_str()).is_some() {
        let value = serde_json::Value::Object(raw.clone());
        if raw.is_empty() {
            let _ = meta.delete(register);
        } else {
            // invariant: a plain map value under a known key on an attached map cannot fail.
            #[allow(clippy::unwrap_used)]
            meta.insert(register, loro::LoroValue::from(value)).unwrap();
        }
    }
}

fn check_name(exposed: &[ExposedParam], key: &ExposedKey, name: &str) -> Result<(), InstanceError> {
    let length = name.chars().count();
    if !(1..=64).contains(&length) || name.chars().any(char::is_control) {
        return Err(InstanceError::BadName);
    }
    if exposed
        .iter()
        .any(|param| &param.key != key && param.label == name)
    {
        return Err(InstanceError::BadName);
    }
    Ok(())
}
