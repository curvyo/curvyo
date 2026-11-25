//! The list of one Iconify set (`/collection?prefix=`, ADR 0086 decision 1): its icon names in
//! listing order with the groups each sits in, and its aliases resolved to listed names.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Deserialize;

use crate::error::IconifyError;
use crate::iconify::{limit, name_ok};

/// How many alias links are followed to reach a listed name.
const MAX_ALIAS_LINKS: usize = 4;

/// What a set lists: names, the groups of each name and the aliases that reach a listed name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetListing {
    /// The icon names, each once: `uncategorized` first, then the groups in key order.
    pub names: Vec<String>,
    /// The set's own groups ("Arrows", "Home"), in key order.
    pub groups: Vec<String>,
    /// Per name (same index as `names`), the indices into `groups` it sits in.
    pub name_groups: Vec<Vec<u16>>,
    /// `(alias, listed name)`, in alias order; aliases of hidden or missing names are dropped.
    pub aliases: Vec<(String, String)>,
}

impl SetListing {
    /// The groups name `index` sits in.
    pub fn groups_of(&self, index: usize) -> impl Iterator<Item = &str> {
        self.name_groups
            .get(index)
            .into_iter()
            .flatten()
            .filter_map(|g| self.groups.get(usize::from(*g)).map(String::as_str))
    }

    /// The aliases of every listed name, by the name's index.
    #[must_use]
    pub fn aliases_by_name(&self) -> Vec<Vec<&str>> {
        let at: HashMap<&str, usize> = self
            .names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        let mut out = vec![Vec::new(); self.names.len()];
        for (alias, name) in &self.aliases {
            if let Some(i) = at.get(name.as_str()) {
                out[*i].push(alias.as_str());
            }
        }
        out
    }
}

#[derive(Deserialize)]
struct Wire {
    #[serde(default)]
    uncategorized: Vec<String>,
    #[serde(default)]
    categories: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    prefix: Option<String>,
    #[serde(default)]
    hidden: Vec<String>,
    #[serde(default)]
    aliases: BTreeMap<String, serde_json::Value>,
}

/// The alias map's parent of one entry: a plain name, or `{"parent": name}` in older answers.
fn parent_of(value: &serde_json::Value) -> Option<&str> {
    value
        .as_str()
        .or_else(|| value.get("parent").and_then(serde_json::Value::as_str))
}

/// The set list of `/collection`: names in listing order, groups per name, resolved aliases;
/// hidden names are never listed.
///
/// # Errors
/// [`IconifyError`] for an answer that is not a set's list or is over the limit.
pub fn parse_set_listing(bytes: &[u8]) -> Result<SetListing, IconifyError> {
    limit(bytes)?;
    let wire: Wire = serde_json::from_slice(bytes).map_err(|_| IconifyError::Invalid)?;
    if wire.prefix.is_none() && wire.uncategorized.is_empty() && wire.categories.is_empty() {
        return Err(IconifyError::Invalid);
    }
    let hidden: HashSet<&str> = wire.hidden.iter().map(String::as_str).collect();
    let mut listing = SetListing::default();
    let mut at: HashMap<String, usize> = HashMap::new();
    let mut push = |listing: &mut SetListing, name: &str, group: Option<u16>| {
        if !name_ok(name) || hidden.contains(name) {
            return;
        }
        let index = *at.entry(name.to_owned()).or_insert_with(|| {
            listing.names.push(name.to_owned());
            listing.name_groups.push(Vec::new());
            listing.names.len() - 1
        });
        if let Some(group) = group
            && !listing.name_groups[index].contains(&group)
        {
            listing.name_groups[index].push(group);
        }
    };
    for name in &wire.uncategorized {
        push(&mut listing, name, None);
    }
    for (group, names) in &wire.categories {
        let Ok(id) = u16::try_from(listing.groups.len()) else {
            break;
        };
        listing.groups.push(group.clone());
        for name in names {
            push(&mut listing, name, Some(id));
        }
    }
    for (alias, value) in &wire.aliases {
        if !name_ok(alias) || at.contains_key(alias) {
            continue;
        }
        let mut next = parent_of(value);
        for _ in 0..MAX_ALIAS_LINKS {
            let Some(name) = next else { break };
            if at.contains_key(name) {
                listing.aliases.push((alias.clone(), name.to_owned()));
                break;
            }
            next = wire.aliases.get(name).and_then(parent_of);
        }
    }
    Ok(listing)
}
