//! Where one item's paths are entered in one operation and with which own path options
//! (`specs/0139` criteria 19 to 22, adrs L3): the `entry.<key>` register, its damage, and both
//! records resolved per outline of a leaf.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::cut_item_records::{PathFraction, RecordDamage, outline_ref};
use crate::operation_path_options::{PathOptions, read_own_path};

/// The fixed direction of an open path (criterion 20); Automatic has no record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// From its first stored point.
    AsDrawn,
    /// From its last stored point.
    Reversed,
}

impl Direction {
    /// The stored text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AsDrawn => "as_drawn",
            Self::Reversed => "reversed",
        }
    }

    /// The direction of a stored text.
    #[must_use]
    pub fn of_str(text: &str) -> Option<Self> {
        match text {
            "as_drawn" => Some(Self::AsDrawn),
            "reversed" => Some(Self::Reversed),
            _ => None,
        }
    }
}

/// How one outline is entered when it is not Automatic (Frage 414 A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutlineEntry {
    /// A closed outline's set start point.
    Start(PathFraction),
    /// An open outline's fixed direction.
    Direction(Direction),
}

/// The entries of one item in one operation, by outline reference (adrs L3). A missing outline
/// is Automatic.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Entries(BTreeMap<String, OutlineEntry>);

const FIELD_START: &str = "start";
const FIELD_DIRECTION: &str = "direction";

impl Entries {
    /// The entry of the outline `reference`, `None` when it is Automatic.
    #[must_use]
    pub fn get(&self, reference: &str) -> Option<OutlineEntry> {
        self.0.get(reference).copied()
    }

    /// Sets (`Some`) or clears (`None`, Automatic) the entry of the outline `reference`.
    pub fn set(&mut self, reference: String, entry: Option<OutlineEntry>) {
        match entry {
            Some(entry) => {
                self.0.insert(reference, entry);
            }
            None => {
                self.0.remove(&reference);
            }
        }
    }

    /// Whether every outline is Automatic (the register goes, adrs L3).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The stored register.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let mut map = Map::new();
        for (reference, entry) in &self.0 {
            let value = match entry {
                OutlineEntry::Start(at) => json!({ FIELD_START: at.ppb() }),
                OutlineEntry::Direction(d) => json!({ FIELD_DIRECTION: d.as_str() }),
            };
            map.insert(reference.clone(), value);
        }
        Value::Object(map)
    }
}

/// Reads a stored `entry.<key>` register.
///
/// # Errors
/// The damage of a register that does not read.
pub fn read_entries(value: &Value) -> Result<Entries, RecordDamage> {
    let map = value.as_object().ok_or(RecordDamage::NotAMap)?;
    let mut out = BTreeMap::new();
    for (reference, entry) in map {
        let entry = entry.as_object().ok_or(RecordDamage::NotAMap)?;
        if entry.len() != 1 {
            return Err(RecordDamage::UnknownField);
        }
        let read = if let Some(start) = entry.get(FIELD_START) {
            let ppb = start.as_u64().ok_or(RecordDamage::WrongType)?;
            OutlineEntry::Start(PathFraction::new(ppb).ok_or(RecordDamage::BadFraction)?)
        } else if let Some(direction) = entry.get(FIELD_DIRECTION) {
            let text = direction.as_str().ok_or(RecordDamage::WrongType)?;
            OutlineEntry::Direction(Direction::of_str(text).ok_or(RecordDamage::BadValue)?)
        } else {
            return Err(RecordDamage::UnknownField);
        };
        out.insert(reference.clone(), read);
    }
    Ok(Entries(out))
}

/// The own path options and the entries one leaf (or Cut-out item) runs with in one operation,
/// from its item's `path_own.<key>` and `entry.<key>` registers (`specs/0139` criteria 19 to 22).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct LeafPath {
    /// The item's own path options, `None` for the operation's.
    pub own: Option<PathOptions>,
    /// The entries that are not Automatic, by outline index (contour index of a Cut-out item).
    pub entries: Vec<(u32, OutlineEntry)>,
    /// A register is damaged: the leaf runs nothing in the operation (adrs L3).
    pub damaged: bool,
}

impl LeafPath {
    /// The records of a leaf with `outlines` outlines, `below` as in [`outline_ref`].
    #[must_use]
    pub fn of(
        own: Option<&Value>,
        entries: Option<&Value>,
        below: &[String],
        outlines: usize,
    ) -> Self {
        Self::resolve(own, entries, |read| {
            (0..outlines)
                .filter_map(|index| {
                    let entry = read.get(&outline_ref(below, index))?;
                    Some((u32::try_from(index).ok()?, entry))
                })
                .collect()
        })
    }

    /// The records of a Cut-out item, by contour index (the references `"<n>"`).
    #[must_use]
    pub fn of_contours(own: Option<&Value>, entries: Option<&Value>) -> Self {
        Self::resolve(own, entries, |read| {
            read.0
                .iter()
                .filter_map(|(reference, entry)| Some((reference.parse().ok()?, *entry)))
                .collect()
        })
    }

    fn resolve(
        own: Option<&Value>,
        entries: Option<&Value>,
        pick: impl Fn(&Entries) -> Vec<(u32, OutlineEntry)>,
    ) -> Self {
        let own = own.map(read_own_path).transpose();
        let entries = entries.map(read_entries).transpose();
        match (own, entries) {
            (Ok(own), Ok(entries)) => Self {
                own,
                entries: entries.as_ref().map(pick).unwrap_or_default(),
                damaged: false,
            },
            _ => Self {
                own: None,
                entries: Vec::new(),
                damaged: true,
            },
        }
    }

    /// The entry of outline `index`, `None` when it is Automatic.
    #[must_use]
    pub fn entry(&self, index: u32) -> Option<OutlineEntry> {
        self.entries
            .iter()
            .find(|(at, _)| *at == index)
            .map(|(_, entry)| *entry)
    }

    /// The path options the leaf runs with: its own, else the operation's `operation`.
    #[must_use]
    pub fn options(&self, operation: PathOptions) -> PathOptions {
        self.own.unwrap_or(operation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation_path_options::LeadKind;

    #[test]
    fn a_register_reads_back_equal_and_automatic_removes_an_entry() {
        let mut entries = Entries::default();
        entries.set(
            "0".to_owned(),
            Some(OutlineEntry::Start(PathFraction::new(125).unwrap())),
        );
        entries.set(
            "g:1".to_owned(),
            Some(OutlineEntry::Direction(Direction::Reversed)),
        );
        let value = entries.to_value();
        assert_eq!(
            value,
            json!({ "0": { "start": 125 }, "g:1": { "direction": "reversed" } })
        );
        assert_eq!(read_entries(&value), Ok(entries.clone()));
        entries.set("0".to_owned(), None);
        entries.set("g:1".to_owned(), None);
        assert_eq!(entries, Entries::default());
    }

    #[test]
    fn a_damaged_register_names_its_damage() {
        let read = |v: Value| read_entries(&v);
        assert_eq!(read(json!(3)), Err(RecordDamage::NotAMap));
        assert_eq!(read(json!({ "0": 3 })), Err(RecordDamage::NotAMap));
        assert_eq!(
            read(json!({ "0": { "start": 1, "direction": "reversed" } })),
            Err(RecordDamage::UnknownField)
        );
        assert_eq!(
            read(json!({ "0": { "end": 1 } })),
            Err(RecordDamage::UnknownField)
        );
        assert_eq!(
            read(json!({ "0": { "start": 1_000_000_000_u64 } })),
            Err(RecordDamage::BadFraction)
        );
        assert_eq!(
            read(json!({ "0": { "start": "a" } })),
            Err(RecordDamage::WrongType)
        );
        assert_eq!(
            read(json!({ "0": { "direction": "sideways" } })),
            Err(RecordDamage::BadValue)
        );
    }

    #[test]
    fn a_leaf_takes_its_outlines_and_a_damaged_register_damages_it() {
        let entries =
            json!({ "g:1": { "start": 10 }, "g:5": { "start": 20 }, "0": { "start": 1 } });
        let below = vec!["g".to_owned()];
        let path = LeafPath::of(None, Some(&entries), &below, 3);
        assert_eq!(
            path.entry(1),
            Some(OutlineEntry::Start(PathFraction::new(10).unwrap()))
        );
        assert_eq!(path.entry(0), None, "another leaf's reference");
        assert!(!path.damaged);
        let own = PathOptions::DEFAULT
            .with(crate::operation_path_options::PathEdit::LeadIn(
                LeadKind::Line,
            ))
            .to_record();
        let path = LeafPath::of(Some(&own), None, &[], 1);
        assert_eq!(path.options(PathOptions::DEFAULT).lead_in, LeadKind::Line);
        assert!(LeafPath::of(Some(&json!(1)), None, &[], 1).damaged);
        assert!(LeafPath::of(None, Some(&json!({ "0": 1 })), &[], 1).damaged);
        let contours = LeafPath::of_contours(None, Some(&json!({ "2": { "start": 5 } })));
        assert!(contours.entry(2).is_some() && contours.entry(0).is_none());
        assert_eq!(
            LeafPath::default().options(PathOptions::DEFAULT),
            PathOptions::DEFAULT
        );
    }
}
