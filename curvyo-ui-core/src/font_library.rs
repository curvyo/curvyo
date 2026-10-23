//! The maker's font tags and favourites and the membership rule of the built-in tags
//! (`specs/0075-font-library` Parts A to C, `adrs.md` decision 2; tags replace groups,
//! customer 2026-10-20).
//!
//! Families are named by their [`crate::family_key`]. A built-in tag's member is decided by the
//! maker's override, else the shipped list (included fonts), else the automatic result (the
//! declared class for Sans, Serif and Script, the cuttable check for Cuttable, the face kind for
//! Single line, `specs/0076-single-line-fonts` criterion 4). Own tags are names
//! the maker put on families; a tag exists while a family carries it. Nothing here is a document
//! change (criterion 13).

use std::collections::{BTreeMap, BTreeSet};

use curvyo_text_core::DeclaredClass;

/// The strip cell of every listed family; not a tag.
pub const ALL_ID: &str = "all";
/// The strip cell of the starred families; not a tag.
pub const FAVOURITES_ID: &str = "favourites";
/// The most own tags (`adrs.md` decision 1).
pub const MAX_OWN_TAGS: usize = 100;
/// The longest tag name, in characters (criterion 7).
pub const MAX_TAG_NAME_CHARS: usize = 24;
/// The most family keys in one list (`adrs.md` decision 1).
pub const MAX_LIST_KEYS: usize = 10_000;

/// A built-in tag: always present, never renamed or deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BuiltInTag {
    /// Faces whose test word is one piece.
    Cuttable,
    /// Single-line faces (`0076`).
    SingleLine,
    /// Script faces.
    Script,
    /// Sans serif faces.
    Sans,
    /// Serif faces.
    Serif,
    /// Set by the maker only.
    Initials,
}

impl BuiltInTag {
    /// The tags in chip and strip order.
    pub const ALL: [Self; 6] = [
        Self::Cuttable,
        Self::SingleLine,
        Self::Script,
        Self::Sans,
        Self::Serif,
        Self::Initials,
    ];

    /// The fixed id, as in the file.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Cuttable => "cuttable",
            Self::SingleLine => "single_line",
            Self::Script => "script",
            Self::Sans => "sans",
            Self::Serif => "serif",
            Self::Initials => "initials",
        }
    }

    /// The name on chips and in the strip.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cuttable => "Cuttable",
            Self::SingleLine => "Single line",
            Self::Script => "Script",
            Self::Sans => "Sans",
            Self::Serif => "Serif",
            Self::Initials => "Initials",
        }
    }

    /// The tag of `id`.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tag| tag.id() == id)
    }

    /// The tag named `name` (or its id), ignoring case.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let lower = name.trim().to_lowercase();
        Self::ALL
            .into_iter()
            .find(|tag| tag.id() == lower || tag.name().to_lowercase() == lower)
    }

    const fn declared(self) -> Option<DeclaredClass> {
        match self {
            Self::Script => Some(DeclaredClass::Script),
            Self::Sans => Some(DeclaredClass::Sans),
            Self::Serif => Some(DeclaredClass::Serif),
            Self::Initials | Self::Cuttable | Self::SingleLine => None,
        }
    }
}

/// The cuttable check's result for a family's representative face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckResult {
    /// The test word makes this many pieces.
    Pieces(usize),
    /// The face could not be read; it is not cuttable and not checked again.
    Unreadable,
}

/// What is known of a family besides the maker's choices.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FamilyFacts {
    /// The class its representative face declares (criterion 15).
    pub declared: Option<DeclaredClass>,
    /// The cuttable check's result, once there is one.
    pub check: Option<CheckResult>,
    /// The shipped tags of an included family (criterion 14); `None` for any other family.
    pub shipped: Option<BTreeSet<BuiltInTag>>,
    /// Its faces are single-line (`0076` criterion 4).
    pub single_line: bool,
}

/// Why a family carries a tag: what its chip's tooltip names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberSource {
    /// The maker's override, or an own tag.
    ByYou,
    /// The shipped list of the included fonts.
    Shipped,
    /// The declared class.
    Declared(DeclaredClass),
    /// The cuttable check, with its piece count.
    Detected(usize),
    /// The face kind: a single-line font (`0076` criterion 4).
    SingleLine,
    /// No source: not a member.
    Nothing,
}

/// A family's state in a built-in tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Membership {
    /// Whether the family carries the tag.
    pub member: bool,
    /// What decided it.
    pub source: MemberSource,
}

/// The maker's additions to and removals from a built-in tag.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Override {
    /// Families added by hand.
    pub added: BTreeSet<String>,
    /// Families taken out by hand.
    pub removed: BTreeSet<String>,
}

/// Why a tag name was refused (criterion 7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagNameError {
    /// Nothing but whitespace.
    Empty,
    /// More than [`MAX_TAG_NAME_CHARS`] characters.
    TooLong,
    /// All or Favourites, which are not tags.
    Reserved,
    /// Another tag of this name, ignoring case, exists (Rename): its name.
    Taken(String),
    /// [`MAX_OWN_TAGS`] own tags exist.
    TooMany,
}

/// A tag of the strip, the Tags list or a chip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagEntry {
    /// The built-in id, or the own tag's name.
    pub id: String,
    /// The name.
    pub name: String,
    /// A built-in tag.
    pub builtin: bool,
}

/// The maker's font library.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FontLibrary {
    pub(crate) chosen: Option<String>,
    pub(crate) favourites: BTreeSet<String>,
    pub(crate) overrides: BTreeMap<BuiltInTag, Override>,
    /// Own tags by family key.
    pub(crate) tags: BTreeMap<String, BTreeSet<String>>,
}

/// The automatic state of `tag` for a family (no override).
#[must_use]
pub fn automatic(tag: BuiltInTag, facts: &FamilyFacts) -> Membership {
    if let Some(shipped) = &facts.shipped {
        let member = shipped.contains(&tag);
        return Membership {
            member,
            source: if member {
                MemberSource::Shipped
            } else {
                MemberSource::Nothing
            },
        };
    }
    let source = match tag {
        BuiltInTag::Cuttable => match facts.check {
            // A single-line face is never cuttable automatically (`0076` criterion 5).
            Some(CheckResult::Pieces(pieces)) if !facts.single_line => {
                MemberSource::Detected(pieces)
            }
            _ => MemberSource::Nothing,
        },
        BuiltInTag::SingleLine if facts.single_line => MemberSource::SingleLine,
        _ => match (tag.declared(), facts.declared) {
            (Some(wanted), Some(declared)) if wanted == declared => {
                MemberSource::Declared(declared)
            }
            _ => MemberSource::Nothing,
        },
    };
    let member = match source {
        MemberSource::Detected(pieces) => pieces == 1,
        MemberSource::Declared(_) | MemberSource::SingleLine => true,
        _ => false,
    };
    Membership { member, source }
}

/// The order of own tags: A to Z ignoring case, then by spelling.
fn tag_order(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b))
}

/// `name` checked against criterion 7 (1 to 24 characters, not All or Favourites); trimmed.
///
/// # Errors
/// The [`TagNameError`] of the first rule `name` breaks.
pub fn check_tag_name(name: &str) -> Result<String, TagNameError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(TagNameError::Empty);
    }
    if name.chars().count() > MAX_TAG_NAME_CHARS {
        return Err(TagNameError::TooLong);
    }
    let lower = name.to_lowercase();
    if lower == ALL_ID || lower == FAVOURITES_ID {
        return Err(TagNameError::Reserved);
    }
    Ok(name.to_owned())
}

impl FontLibrary {
    /// The strip cell the maker chose; All when none. Deleting the chosen own tag, or removing its
    /// last use, chooses All at once (criterion 6, customer bug report 2026-10-20).
    #[must_use]
    pub fn chosen(&self) -> &str {
        self.chosen.as_deref().unwrap_or(ALL_ID)
    }

    /// Chooses a strip cell: All, Favourites, a built-in id or an own tag's name.
    pub fn choose(&mut self, id: &str) -> bool {
        let known = id == ALL_ID
            || id == FAVOURITES_ID
            || BuiltInTag::from_id(id).is_some()
            || self.own_tag(id).is_some();
        if known {
            self.chosen = (id != ALL_ID).then(|| id.to_owned());
        }
        known
    }

    /// The own tag named `name` ignoring case, in its stored spelling.
    fn own_tag(&self, name: &str) -> Option<&str> {
        let lower = name.to_lowercase();
        self.tags
            .values()
            .flatten()
            .find(|tag| tag.to_lowercase() == lower)
            .map(String::as_str)
    }

    /// The own tags, A to Z ignoring case.
    #[must_use]
    pub fn own_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self
            .tags
            .values()
            .flatten()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        tags.sort_by(|a, b| tag_order(a, b));
        tags
    }

    /// Every tag in strip order: the built-in tags, then the own tags A to Z.
    #[must_use]
    pub fn all_tags(&self) -> Vec<TagEntry> {
        BuiltInTag::ALL
            .into_iter()
            .map(|tag| TagEntry {
                id: tag.id().to_owned(),
                name: tag.name().to_owned(),
                builtin: true,
            })
            .chain(self.own_tags().into_iter().map(|name| TagEntry {
                id: name.clone(),
                name,
                builtin: false,
            }))
            .collect()
    }

    /// Whether `key` is starred.
    #[must_use]
    pub fn is_favourite(&self, key: &str) -> bool {
        self.favourites.contains(key)
    }

    /// Stars or unstars `key` (criterion 9); returns the new state.
    pub fn toggle_favourite(&mut self, key: &str) -> bool {
        if self.favourites.remove(key) || self.favourites.len() >= MAX_LIST_KEYS {
            return false;
        }
        self.favourites.insert(key.to_owned());
        true
    }

    /// The state of `key` in the built-in `tag`.
    #[must_use]
    pub fn membership(&self, tag: BuiltInTag, key: &str, facts: &FamilyFacts) -> Membership {
        if let Some(over) = self.overrides.get(&tag) {
            if over.added.contains(key) {
                return Membership {
                    member: true,
                    source: MemberSource::ByYou,
                };
            }
            if over.removed.contains(key) {
                return Membership {
                    member: false,
                    source: MemberSource::ByYou,
                };
            }
        }
        automatic(tag, facts)
    }

    /// Whether `key` is in the strip cell `id` (criterion 2); All holds every family.
    #[must_use]
    pub fn is_member(&self, id: &str, key: &str, facts: &FamilyFacts) -> bool {
        if id == ALL_ID {
            return true;
        }
        if id == FAVOURITES_ID {
            return self.is_favourite(key);
        }
        if let Some(tag) = BuiltInTag::from_id(id) {
            return self.membership(tag, key, facts).member;
        }
        self.tags.get(key).is_some_and(|tags| tags.contains(id))
    }

    /// The tags `key` carries in chip order, each with its source; a removed built-in tag is not
    /// among them (criterion 10).
    #[must_use]
    pub fn tags_of(&self, key: &str, facts: &FamilyFacts) -> Vec<(TagEntry, MemberSource)> {
        let builtin = BuiltInTag::ALL.into_iter().filter_map(|tag| {
            let state = self.membership(tag, key, facts);
            state.member.then(|| {
                (
                    TagEntry {
                        id: tag.id().to_owned(),
                        name: tag.name().to_owned(),
                        builtin: true,
                    },
                    state.source,
                )
            })
        });
        let mut own: Vec<String> = self
            .tags
            .get(key)
            .map(|tags| tags.iter().cloned().collect())
            .unwrap_or_default();
        own.sort_by(|a, b| tag_order(a, b));
        builtin
            .chain(own.into_iter().map(|name| {
                (
                    TagEntry {
                        id: name.clone(),
                        name,
                        builtin: false,
                    },
                    MemberSource::ByYou,
                )
            }))
            .collect()
    }

    fn set_builtin(&mut self, tag: BuiltInTag, key: &str, on: bool, facts: &FamilyFacts) -> bool {
        if self.membership(tag, key, facts).member == on {
            return false;
        }
        let over = self.overrides.entry(tag).or_default();
        over.added.remove(key);
        over.removed.remove(key);
        // Adding an automatic tag back clears the override (UX amendment §1).
        if automatic(tag, facts).member != on {
            let list = if on {
                &mut over.added
            } else {
                &mut over.removed
            };
            if list.len() < MAX_LIST_KEYS {
                list.insert(key.to_owned());
            }
        }
        if over.added.is_empty() && over.removed.is_empty() {
            self.overrides.remove(&tag);
        }
        true
    }

    /// Puts the tag `name` on `key` (the add field, criterion 10): a built-in name is an
    /// override, an existing own tag (ignoring case) keeps its spelling, a new name makes a tag.
    /// Returns whether anything changed.
    ///
    /// # Errors
    /// A [`TagNameError`]; nothing is changed.
    pub fn add_tag(
        &mut self,
        key: &str,
        name: &str,
        facts: &FamilyFacts,
    ) -> Result<bool, TagNameError> {
        let name = check_tag_name(name)?;
        if let Some(tag) = BuiltInTag::from_name(&name) {
            return Ok(self.set_builtin(tag, key, true, facts));
        }
        let spelling = match self.own_tag(&name) {
            Some(existing) => existing.to_owned(),
            None if self.own_tags().len() >= MAX_OWN_TAGS => return Err(TagNameError::TooMany),
            None => name,
        };
        if !self.tags.contains_key(key) && self.tags.len() >= MAX_LIST_KEYS {
            return Err(TagNameError::TooMany);
        }
        Ok(self
            .tags
            .entry(key.to_owned())
            .or_default()
            .insert(spelling))
    }

    /// Takes the tag `id` off `key` (its x, criterion 10): a built-in tag becomes a "removed"
    /// override, an own tag's last use deletes it. Returns whether anything changed.
    pub fn remove_tag(&mut self, key: &str, id: &str, facts: &FamilyFacts) -> bool {
        if let Some(tag) = BuiltInTag::from_id(id) {
            return self.set_builtin(tag, key, false, facts);
        }
        let Some(tags) = self.tags.get_mut(key) else {
            return false;
        };
        let removed = tags.remove(id);
        if tags.is_empty() {
            self.tags.remove(key);
        }
        self.drop_chosen_if_gone();
        removed
    }

    /// Chooses All when the chosen own tag no longer exists (criterion 6).
    fn drop_chosen_if_gone(&mut self) {
        let gone = self.chosen.as_deref().is_some_and(|id| {
            id != FAVOURITES_ID && BuiltInTag::from_id(id).is_none() && self.own_tag(id).is_none()
        });
        if gone {
            self.chosen = None;
        }
    }

    /// Renames the own tag `id` on every family (criterion 7); a change of case only is allowed.
    ///
    /// # Errors
    /// A [`TagNameError`]; nothing is changed. A built-in or unknown id is
    /// [`TagNameError::Taken`] with its own name.
    pub fn rename_tag(&mut self, id: &str, name: &str) -> Result<(), TagNameError> {
        let name = check_tag_name(name)?;
        if BuiltInTag::from_id(id).is_some() || self.own_tag(id) != Some(id) {
            return Err(TagNameError::Taken(id.to_owned()));
        }
        if let Some(tag) = BuiltInTag::from_name(&name) {
            return Err(TagNameError::Taken(tag.name().to_owned()));
        }
        if let Some(other) = self.own_tag(&name).filter(|other| *other != id) {
            return Err(TagNameError::Taken(other.to_owned()));
        }
        for tags in self.tags.values_mut() {
            if tags.remove(id) {
                tags.insert(name.clone());
            }
        }
        if self.chosen.as_deref() == Some(id) {
            self.chosen = Some(name);
        }
        Ok(())
    }

    /// Takes the own tag `id` off every family (criterion 8); a built-in tag is never deleted.
    /// Returns whether it existed.
    pub fn delete_tag(&mut self, id: &str) -> bool {
        if BuiltInTag::from_id(id).is_some() {
            return false;
        }
        let mut found = false;
        for tags in self.tags.values_mut() {
            found |= tags.remove(id);
        }
        self.tags.retain(|_, tags| !tags.is_empty());
        self.drop_chosen_if_gone();
        found
    }

    /// The existing tags `key` does not carry whose name starts with `prefix` ignoring case, in
    /// chip order: the add field's inline completion (UX amendment §4).
    #[must_use]
    pub fn completions(&self, key: &str, prefix: &str, facts: &FamilyFacts) -> Vec<String> {
        let lower = prefix.trim_start().to_lowercase();
        if lower.is_empty() {
            return Vec::new();
        }
        let carried: BTreeSet<String> = self
            .tags_of(key, facts)
            .into_iter()
            .map(|(tag, _)| tag.id)
            .collect();
        self.all_tags()
            .into_iter()
            .filter(|tag| !carried.contains(&tag.id) && tag.name.to_lowercase().starts_with(&lower))
            .map(|tag| tag.name)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(declared: Option<DeclaredClass>, pieces: Option<usize>) -> FamilyFacts {
        FamilyFacts {
            declared,
            check: pieces.map(CheckResult::Pieces),
            shipped: None,
            single_line: false,
        }
    }

    /// `0076` criteria 4 and 5: a single-line family is in Single line, after Cuttable in the
    /// strip, never Cuttable by itself; the maker can remove and add both tags.
    #[test]
    fn ac4_ac5_single_line_tag() {
        let mut library = FontLibrary::default();
        let facts = FamilyFacts {
            single_line: true,
            ..facts(Some(DeclaredClass::Sans), Some(1))
        };
        let names: Vec<String> = library.all_tags().into_iter().map(|t| t.name).collect();
        assert_eq!(
            names[..4],
            ["Cuttable", "Single line", "Script", "Sans"].map(String::from)
        );
        assert_eq!(
            library.tags_of("hershey", &facts),
            [
                (
                    TagEntry {
                        id: "single_line".into(),
                        name: "Single line".into(),
                        builtin: true
                    },
                    MemberSource::SingleLine
                ),
                (
                    TagEntry {
                        id: "sans".into(),
                        name: "Sans".into(),
                        builtin: true
                    },
                    MemberSource::Declared(DeclaredClass::Sans)
                )
            ]
        );
        assert!(library.remove_tag("hershey", "single_line", &facts));
        assert!(!library.is_member("single_line", "hershey", &facts));
        assert_eq!(library.add_tag("hershey", "single line", &facts), Ok(true));
        assert!(library.overrides.is_empty());
        assert_eq!(library.add_tag("hershey", "Cuttable", &facts), Ok(true));
        assert!(library.is_member("cuttable", "hershey", &facts));
    }

    /// Criteria 10, 14, 15, 17: override beats shipped beats automatic; the check tags Cuttable
    /// at one piece only; a removed automatic tag is hidden and adding it back clears the
    /// override.
    #[test]
    fn ac10_ac17_membership_rule_and_overrides() {
        let mut library = FontLibrary::default();
        let script = facts(Some(DeclaredClass::Script), Some(1));
        let names = |l: &FontLibrary| -> Vec<String> {
            l.tags_of("lobster", &script)
                .into_iter()
                .map(|(t, _)| t.name)
                .collect()
        };
        assert_eq!(names(&library), ["Cuttable", "Script"]);
        assert!(!library.is_member("cuttable", "x", &facts(None, Some(7))));
        let shipped = FamilyFacts {
            shipped: Some([BuiltInTag::Sans].into()),
            ..facts(Some(DeclaredClass::Serif), None)
        };
        assert!(library.is_member("sans", "source sans 3", &shipped));
        assert!(!library.is_member("serif", "source sans 3", &shipped));
        assert!(library.remove_tag("lobster", "script", &script));
        assert_eq!(names(&library), ["Cuttable"]);
        assert_eq!(library.add_tag("lobster", "SCRIPT", &script), Ok(true));
        assert!(library.overrides.is_empty(), "back to automatic");
        library.add_tag("lobster", "initials", &script).unwrap();
        assert_eq!(
            library.tags_of("lobster", &script)[2],
            (
                TagEntry {
                    id: "initials".into(),
                    name: "Initials".into(),
                    builtin: true
                },
                MemberSource::ByYou
            )
        );
    }

    /// Criteria 6, 7, 8: own tags exist while used, keep their first spelling, sort A to Z,
    /// rename on every family, and the names All and Favourites are refused.
    #[test]
    fn ac6_ac7_ac8_own_tags() {
        let mut library = FontLibrary::default();
        let none = FamilyFacts::default();
        library.add_tag("a", "Wedding", &none).unwrap();
        library.add_tag("b", "wedding", &none).unwrap();
        library.add_tag("b", "Birthday", &none).unwrap();
        assert_eq!(library.own_tags(), ["Birthday", "Wedding"]);
        assert!(library.is_member("Wedding", "b", &none));
        assert_eq!(
            library.add_tag("a", "Favourites", &none),
            Err(TagNameError::Reserved)
        );
        assert_eq!(
            library.add_tag("a", &"x".repeat(25), &none),
            Err(TagNameError::TooLong)
        );
        assert_eq!(
            library.rename_tag("Wedding", "birthday"),
            Err(TagNameError::Taken("Birthday".into()))
        );
        assert!(library.rename_tag("Wedding", "WEDDING").is_ok());
        assert!(library.is_member("WEDDING", "a", &none));
        assert!(library.choose("WEDDING"));
        assert!(library.remove_tag("a", "WEDDING", &none));
        assert!(library.delete_tag("WEDDING"));
        assert_eq!(library.own_tags(), ["Birthday"]);
        assert_eq!(
            library.chosen(),
            "all",
            "the deleted chosen tag gives way to All"
        );
        assert!(!library.delete_tag("serif"));
        assert_eq!(library.completions("a", "b", &none), ["Birthday"]);
        assert_eq!(library.completions("b", "b", &none), Vec::<String>::new());
    }

    /// Criterion 9: a star toggles and Favourites holds the starred families.
    #[test]
    fn ac9_the_star_toggles_favourites() {
        let mut library = FontLibrary::default();
        assert!(library.toggle_favourite("lobster"));
        assert!(library.is_member("favourites", "lobster", &FamilyFacts::default()));
        assert!(!library.toggle_favourite("lobster"));
    }
}
