//! The link commands of sketch instances (`specs/0099-sketch-pages/adrs.md` A13, criteria 32 and
//! 35): Link and Unlink of exposed values on the sketch, and the clean-up of the clones' unlinked
//! marks when a value leaves its group. One commit each.

use crate::document::{Document, OBJECTS_TREE};
use crate::instance_codec::{self, decode_entry};
use crate::instance_commands::{
    InstanceError, LINK_PARAMETERS_COMMIT_LABEL, UNLINK_PARAMETERS_COMMIT_LABEL,
};
use crate::instance_value::{ExposedKey, ExposedParam};
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::primitive_model::ObjectSnapshot;

impl Document {
    /// Links the exposed values `a` and `b` of `sketch` (`0099` criterion 32), one commit "Link
    /// parameters": neither in a group starts a new one, one in a group brings the other into it,
    /// two groups merge into `a`'s (the marks of `b`'s members go, they left their group).
    ///
    /// # Errors
    /// [`InstanceError::NoSuchSketch`] and [`InstanceError::NotLinkable`]; nothing is written.
    pub fn link_exposed(
        &self,
        sketch: NodeId,
        a: &ExposedKey,
        b: &ExposedKey,
    ) -> Result<(), InstanceError> {
        let snapshot = self.sketch(sketch).ok_or(InstanceError::NoSuchSketch)?;
        let meta = self
            .sketch_meta(sketch)
            .map_err(|_| InstanceError::NoSuchSketch)?;
        let find = |key: &ExposedKey| snapshot.exposed.iter().find(|param| &param.key == key);
        let (Some(first), Some(second)) = (find(a), find(b)) else {
            return Err(InstanceError::NotLinkable);
        };
        if a == b
            || first.kind != second.kind
            || (first.group.is_some() && first.group == second.group)
        {
            return Err(InstanceError::NotLinkable);
        }
        let mut items = instance_codec::read_exposed_raw(&meta);
        let mut unmarked = Vec::new();
        let link = match (first.group, second.group) {
            (Some(_), _) => first.link.clone(),
            (None, Some(_)) => second.link.clone(),
            (None, None) => Some(fresh_link(&items, a)),
        };
        let Some(link) = link else {
            return Err(InstanceError::NotLinkable);
        };
        let mut joining = vec![a.clone(), b.clone()];
        if first.group.is_some() && second.group.is_some() {
            joining.extend(group_partners(&snapshot.exposed, b));
            unmarked.extend(joining.iter().skip(1).cloned());
        }
        for key in &joining {
            set_link(&mut items, key, Some(&link));
        }
        instance_codec::write_exposed_raw(&meta, &items);
        self.drop_marks(sketch, &unmarked);
        self.commit_with_label(LINK_PARAMETERS_COMMIT_LABEL);
        Ok(())
    }

    /// Takes the exposed `key` of `sketch` out of its link group, one commit "Unlink parameters";
    /// a group left with one member is dissolved, and the clones' marks of the values that left
    /// go in the same commit (`0099` criterion 35).
    ///
    /// # Errors
    /// [`InstanceError::NoSuchSketch`] and [`InstanceError::NotLinkable`] for a value in no group;
    /// nothing is written.
    pub fn unlink_exposed(&self, sketch: NodeId, key: &ExposedKey) -> Result<(), InstanceError> {
        let snapshot = self.sketch(sketch).ok_or(InstanceError::NoSuchSketch)?;
        let meta = self
            .sketch_meta(sketch)
            .map_err(|_| InstanceError::NoSuchSketch)?;
        if snapshot
            .exposed
            .iter()
            .find(|param| &param.key == key)
            .is_none_or(|param| param.group.is_none())
        {
            return Err(InstanceError::NotLinkable);
        }
        let mut items = instance_codec::read_exposed_raw(&meta);
        let mut unmarked = vec![key.clone()];
        set_link(&mut items, key, None);
        if let [last] = &group_partners(&snapshot.exposed, key)[..] {
            set_link(&mut items, last, None);
            unmarked.push(last.clone());
        }
        instance_codec::write_exposed_raw(&meta, &items);
        self.drop_marks(sketch, &unmarked);
        self.commit_with_label(UNLINK_PARAMETERS_COMMIT_LABEL);
        Ok(())
    }

    /// Removes `keys` from the unlinked marks of every clone of `sketch`. Uncommitted.
    pub(crate) fn drop_marks(&self, sketch: NodeId, keys: &[ExposedKey]) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for object in self.read_objects().objects.iter() {
            let ObjectSnapshot::Clone(clone) = object else {
                continue;
            };
            if clone.chain.root != sketch {
                continue;
            }
            let Ok(meta) = tree.get_meta(tree_id_of(clone.id())) else {
                continue;
            };
            let mut marks = instance_codec::read_unlinked(&meta);
            let before = marks.clone();
            marks.retain(|key| !keys.contains(key));
            if marks != before {
                instance_codec::write_unlinked(&meta, &marks);
            }
        }
    }
}

/// The other members of `key`'s link group, in exposed order; empty when it is in none.
pub(crate) fn group_partners(exposed: &[ExposedParam], key: &ExposedKey) -> Vec<ExposedKey> {
    let Some(group) = exposed
        .iter()
        .find(|param| &param.key == key)
        .and_then(|param| param.group)
    else {
        return Vec::new();
    };
    exposed
        .iter()
        .filter(|param| param.group == Some(group) && &param.key != key)
        .map(|param| param.key.clone())
        .collect()
}

/// A link string no stored entry uses: the key's own when free, else `<key>#2`, `#3`, ...
fn fresh_link(items: &[serde_json::Value], key: &ExposedKey) -> String {
    let used = instance_codec::used_links(items);
    let mut candidate = key.as_str().to_owned();
    let mut n = 2;
    while used.contains(&candidate) {
        candidate = format!("{}#{n}", key.as_str());
        n += 1;
    }
    candidate
}

/// Sets (or with `None` removes) the `link` of the first stored entry of `key`, keeping its other
/// fields.
pub(crate) fn set_link(items: &mut [serde_json::Value], key: &ExposedKey, link: Option<&str>) {
    let Some(item) = items
        .iter_mut()
        .find(|item| decode_entry(item).is_some_and(|entry| &entry.key == key))
    else {
        return;
    };
    if let Some(map) = item.as_object_mut() {
        match link {
            Some(link) => {
                map.insert("link".into(), serde_json::Value::from(link));
            }
            None => {
                map.remove("link");
            }
        }
    }
}
