//! The metadata of a font family as it travels between the maker's devices
//! (`specs/0116-library-sync` criteria 16 and 17): read from the library, applied to it.
//!
//! Own tags and the maker's overrides of the built-in tags other than Cuttable travel as one
//! tag list (`own:<name>`, `add:<id>`, `del:<id>`); the star and the Cuttable override travel as
//! fields of their own. The automatic parts (the declared class, the check, the shipped tags)
//! are not sent: every device derives them from the same file.

use std::collections::BTreeSet;

use crate::font_library::{BuiltInTag, FontLibrary, MAX_LIST_KEYS, MAX_OWN_TAGS};

/// What of a family's tags and star travels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FontMeta {
    /// The own tags (`own:<name>`) and the overrides of the built-in tags but Cuttable
    /// (`add:<id>`, `del:<id>`), sorted.
    pub tags: Vec<String>,
    /// The family is starred.
    pub favourite: bool,
    /// The Cuttable override: `Some(true)` added by hand, `Some(false)` removed by hand.
    pub cuttable: Option<bool>,
}

impl FontLibrary {
    /// The metadata of the family `key` as it travels.
    #[must_use]
    pub fn font_meta(&self, key: &str) -> FontMeta {
        let mut tags: Vec<String> = self
            .tags
            .get(key)
            .map(|own| own.iter().map(|name| format!("own:{name}")).collect())
            .unwrap_or_default();
        let mut cuttable = None;
        for tag in BuiltInTag::ALL {
            let Some(over) = self.overrides.get(&tag) else {
                continue;
            };
            let state = if over.added.contains(key) {
                Some(true)
            } else if over.removed.contains(key) {
                Some(false)
            } else {
                None
            };
            if tag == BuiltInTag::Cuttable {
                cuttable = state;
            } else if let Some(on) = state {
                tags.push(format!("{}:{}", if on { "add" } else { "del" }, tag.id()));
            }
        }
        FontMeta {
            tags,
            favourite: self.favourites.contains(key),
            cuttable,
        }
    }

    /// Makes the family `key` carry `meta` (what arrived from another device). Returns whether
    /// anything changed. An own tag past the library's limit is left out, as an Add would
    /// refuse it.
    pub fn apply_font_meta(&mut self, key: &str, meta: &FontMeta) -> bool {
        let before = self.font_meta(key);
        let mut own = BTreeSet::new();
        let mut added = Vec::new();
        let mut removed = Vec::new();
        for entry in &meta.tags {
            match entry.split_once(':') {
                Some(("own", name)) => {
                    own.insert(name.to_owned());
                }
                Some(("add", id)) => added.extend(BuiltInTag::from_id(id)),
                Some(("del", id)) => removed.extend(BuiltInTag::from_id(id)),
                _ => {}
            }
        }
        self.set_own_tags(key, own);
        for tag in BuiltInTag::ALL {
            let state = if tag == BuiltInTag::Cuttable {
                meta.cuttable
            } else if added.contains(&tag) {
                Some(true)
            } else if removed.contains(&tag) {
                Some(false)
            } else {
                None
            };
            self.set_override(tag, key, state);
        }
        if meta.favourite {
            if self.favourites.len() < MAX_LIST_KEYS {
                self.favourites.insert(key.to_owned());
            }
        } else {
            self.favourites.remove(key);
        }
        self.font_meta(key) != before
    }

    fn set_own_tags(&mut self, key: &str, own: BTreeSet<String>) {
        let others: BTreeSet<&String> = self
            .tags
            .iter()
            .filter(|(family, _)| family.as_str() != key)
            .flat_map(|(_, tags)| tags)
            .collect();
        let room = MAX_OWN_TAGS.saturating_sub(others.len());
        let fresh = own.iter().filter(|name| !others.contains(name)).count();
        let own: BTreeSet<String> = if fresh > room {
            own.into_iter()
                .filter(|name| others.contains(name))
                .collect()
        } else {
            own
        };
        if own.is_empty() {
            self.tags.remove(key);
        } else {
            self.tags.insert(key.to_owned(), own);
        }
    }

    fn set_override(&mut self, tag: BuiltInTag, key: &str, state: Option<bool>) {
        let over = self.overrides.entry(tag).or_default();
        over.added.remove(key);
        over.removed.remove(key);
        match state {
            Some(true) if over.added.len() < MAX_LIST_KEYS => {
                over.added.insert(key.to_owned());
            }
            Some(false) if over.removed.len() < MAX_LIST_KEYS => {
                over.removed.insert(key.to_owned());
            }
            _ => {}
        }
        if over.added.is_empty() && over.removed.is_empty() {
            self.overrides.remove(&tag);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_library::FamilyFacts;

    /// Criterion 16: a family's own tags, star, a hand-set Initials and a hand-set Cuttable
    /// read as meta and make another library carry the same.
    #[test]
    fn meta_read_from_one_library_makes_another_carry_the_same() {
        let facts = FamilyFacts::default();
        let mut a = FontLibrary::default();
        a.toggle_favourite("lobster");
        a.add_tag("lobster", "Wedding", &facts).unwrap();
        a.add_tag("lobster", "Initials", &facts).unwrap();
        a.add_tag("lobster", "Cuttable", &facts).unwrap();
        let meta = a.font_meta("lobster");
        assert!(meta.favourite);
        assert_eq!(meta.cuttable, Some(true));
        assert_eq!(meta.tags, ["own:Wedding", "add:initials"]);

        let mut b = FontLibrary::default();
        assert!(b.apply_font_meta("lobster", &meta));
        assert_eq!(b.font_meta("lobster"), meta);
        assert!(
            !b.apply_font_meta("lobster", &meta),
            "nothing changed twice"
        );
        assert!(b.is_favourite("lobster"));
        assert_eq!(b.own_tags(), ["Wedding"]);
    }

    /// Criterion 17: a star and a tag taken away on one device go away on the other; a removed
    /// Script override is carried as `del:`.
    #[test]
    fn a_removed_tag_and_star_travel_too() {
        let facts = FamilyFacts::default();
        let mut a = FontLibrary::default();
        a.toggle_favourite("lobster");
        a.add_tag("lobster", "Wedding", &facts).unwrap();
        let mut b = FontLibrary::default();
        b.apply_font_meta("lobster", &a.font_meta("lobster"));
        a.toggle_favourite("lobster");
        a.remove_tag("lobster", "Wedding", &facts);
        assert!(b.apply_font_meta("lobster", &a.font_meta("lobster")));
        assert!(!b.is_favourite("lobster"));
        assert_eq!(b.own_tags().len(), 0);

        let mut c = FontLibrary::default();
        c.apply_font_meta(
            "x",
            &FontMeta {
                tags: vec!["del:script".to_owned()],
                favourite: false,
                cuttable: Some(false),
            },
        );
        let meta = c.font_meta("x");
        assert_eq!(meta.tags, ["del:script"]);
        assert_eq!(meta.cuttable, Some(false));
    }
}
