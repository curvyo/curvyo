//! The own tabs of one item in one operation (`specs/0139` criteria 5, 6, 11, adrs L3): the stored
//! `tabs.<key>` register, its outline references and its damage.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use curvyo_document_core::NodeId;

use crate::operation_cut_options::CutLength;

/// A position on an outline as a fraction of its flattened length, in parts per billion,
/// measured from its first stored point along its drawn direction (adrs L3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathFraction(u32);

impl PathFraction {
    /// One whole outline length in parts per billion (exclusive bound).
    pub const WHOLE: u32 = 1_000_000_000;

    /// The fraction of `ppb` parts per billion, `None` from one whole length on.
    #[must_use]
    pub const fn new(ppb: u64) -> Option<Self> {
        if ppb < Self::WHOLE as u64 {
            #[allow(clippy::cast_possible_truncation)] // below 10^9
            Some(Self(ppb as u32))
        } else {
            None
        }
    }

    /// The fraction nearest to `value` (0 to 1); 1 and above wrap to 0, below 0 reads 0.
    #[must_use]
    pub fn of(value: f64) -> Self {
        let ppb = (value.clamp(0.0, 1.0) * f64::from(Self::WHOLE)).round();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // invariant: clamped to 0..=10^9 above
        let ppb = ppb as u32;
        Self(if ppb >= Self::WHOLE { 0 } else { ppb })
    }

    /// The parts per billion.
    #[must_use]
    pub const fn ppb(self) -> u32 {
        self.0
    }

    /// The fraction as a number from 0 to 1.
    #[must_use]
    pub fn get(self) -> f64 {
        f64::from(self.0) / f64::from(Self::WHOLE)
    }
}

/// One own tab: its centre and, when it has one, its own width (criterion 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OwnTab {
    /// The centre.
    pub at: PathFraction,
    /// The own width; `None` is the operation's.
    pub width: Option<CutLength>,
}

/// Why a stored `tabs.<key>` register does not read: the item runs nothing in that operation,
/// counts as damaged settings and the register is kept unchanged (adrs L3, criterion 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordDamage {
    /// Not a map, or an outline's value is not a list of maps.
    NotAMap,
    /// A field this build does not know.
    UnknownField,
    /// A value of the wrong type.
    WrongType,
    /// A fraction outside 0 to 1.
    BadFraction,
    /// A width outside 0.10 to 20.00 mm or with more decimals.
    BadWidth,
    /// Two tabs that overlap (also two at one position).
    Overlap,
    /// A lead kind, lead length, overcut or direction outside its values (`specs/0139`
    /// criterion 22).
    BadValue,
}

/// The own tabs of one item in one operation, by outline reference (adrs L3). An outline that is
/// missing follows the operation's placement; an empty list means no tabs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OwnTabs(BTreeMap<String, Vec<OwnTab>>);

const FIELD_AT: &str = "at";
const FIELD_WIDTH: &str = "width_mm";

impl OwnTabs {
    /// The tabs of the outline `reference`, `None` when it follows the operation.
    #[must_use]
    pub fn get(&self, reference: &str) -> Option<&[OwnTab]> {
        self.0.get(reference).map(Vec::as_slice)
    }

    /// Sets the tabs of the outline `reference`, ordered by position.
    pub fn set(&mut self, reference: String, mut tabs: Vec<OwnTab>) {
        tabs.sort_by_key(|tab| tab.at);
        self.0.insert(reference, tabs);
    }

    /// How many tabs all outlines hold.
    #[must_use]
    pub fn count(&self) -> usize {
        self.0.values().map(Vec::len).sum()
    }

    /// The stored register.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let mut map = Map::new();
        for (reference, tabs) in &self.0 {
            let list: Vec<Value> = tabs
                .iter()
                .map(|tab| {
                    let mut entry = Map::new();
                    entry.insert(FIELD_AT.to_owned(), json!(tab.at.ppb()));
                    if let Some(width) = tab.width {
                        entry.insert(FIELD_WIDTH.to_owned(), json!(width.mm()));
                    }
                    Value::Object(entry)
                })
                .collect();
            map.insert(reference.clone(), Value::Array(list));
        }
        Value::Object(map)
    }
}

/// Reads a stored `tabs.<key>` register.
///
/// # Errors
/// The damage of a register that does not read.
pub fn read_own_tabs(value: &Value) -> Result<OwnTabs, RecordDamage> {
    let map = value.as_object().ok_or(RecordDamage::NotAMap)?;
    let mut out = BTreeMap::new();
    for (reference, list) in map {
        let list = list.as_array().ok_or(RecordDamage::NotAMap)?;
        let mut tabs = Vec::with_capacity(list.len());
        for entry in list {
            tabs.push(read_tab(entry)?);
        }
        if tabs.windows(2).any(|pair| pair[0].at >= pair[1].at) {
            return Err(RecordDamage::Overlap);
        }
        out.insert(reference.clone(), tabs);
    }
    Ok(OwnTabs(out))
}

fn read_tab(entry: &Value) -> Result<OwnTab, RecordDamage> {
    let entry = entry.as_object().ok_or(RecordDamage::NotAMap)?;
    if entry.keys().any(|k| k != FIELD_AT && k != FIELD_WIDTH) {
        return Err(RecordDamage::UnknownField);
    }
    let at = entry.get(FIELD_AT).ok_or(RecordDamage::WrongType)?;
    let at = at.as_u64().ok_or(RecordDamage::WrongType)?;
    let at = PathFraction::new(at).ok_or(RecordDamage::BadFraction)?;
    let width = match entry.get(FIELD_WIDTH) {
        None => None,
        Some(width) => {
            let mm = width.as_f64().ok_or(RecordDamage::WrongType)?;
            Some(CutLength::tab_of_mm(mm).ok_or(RecordDamage::BadWidth)?)
        }
    };
    Ok(OwnTab { at, width })
}

/// The reference of outline `outline` of a leaf: `"<n>"` when the target is the leaf itself
/// (`below` empty), else `"<path>:<n>"` with `below` the node texts from below the target down to
/// the leaf, joined by `/` (adrs L3). A Cut-out item's contour `n` and an image's frame (`0`) use
/// the first form.
#[must_use]
pub fn outline_ref(below: &[String], outline: usize) -> String {
    if below.is_empty() {
        outline.to_string()
    } else {
        format!("{}:{outline}", below.join("/"))
    }
}

/// The nodes from below the assigned `target` down to the leaf whose key chain is `chain`
/// (nearest first, as the nearest rule walks it, ADR 0046 §7): empty when the target is the leaf
/// itself; a clone member's ids after its clone; a group's members down to the leaf.
#[must_use]
pub fn below_target(chain: &[Vec<NodeId>], target: &[NodeId]) -> Vec<NodeId> {
    let Some(at) = chain.iter().position(|key| key.as_slice() == target) else {
        return Vec::new();
    };
    let mut below: Vec<NodeId> = Vec::new();
    let mut previous: &[NodeId] = target;
    for key in chain[..at].iter().rev() {
        if key.starts_with(previous) {
            below.extend_from_slice(&key[previous.len()..]);
        } else {
            below.extend_from_slice(key);
        }
        previous = key;
    }
    below
}

/// The tabs one leaf (or Cut-out item) runs with in one operation, resolved from its item's
/// register (`specs/0139` criteria 4, 5, 11).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum LeafTabs {
    /// The operation's placement on every outline.
    #[default]
    Operation,
    /// Own tabs on some outlines, by outline index (contour index of a Cut-out item); an outline
    /// not listed follows the operation.
    Own(Vec<(u32, Vec<OwnTab>)>),
    /// The item's register is damaged: the leaf runs nothing in the operation.
    Damaged,
}

impl LeafTabs {
    /// The tabs of a leaf with `outlines` outlines, from its item's register `record` (absent:
    /// the operation's), `below` as in [`outline_ref`].
    #[must_use]
    pub fn of(record: Option<&Value>, below: &[String], outlines: usize) -> Self {
        let Some(record) = record else {
            return Self::Operation;
        };
        let Ok(own) = read_own_tabs(record) else {
            return Self::Damaged;
        };
        let list: Vec<(u32, Vec<OwnTab>)> = (0..outlines)
            .filter_map(|index| {
                let tabs = own.get(&outline_ref(below, index))?;
                Some((u32::try_from(index).ok()?, tabs.to_vec()))
            })
            .collect();
        if list.is_empty() {
            Self::Operation
        } else {
            Self::Own(list)
        }
    }

    /// The tabs of a Cut-out item, by contour index, from its register `record` (the references
    /// `"<n>"`).
    #[must_use]
    pub fn of_contours(record: Option<&Value>) -> Self {
        let Some(record) = record else {
            return Self::Operation;
        };
        let Ok(own) = read_own_tabs(record) else {
            return Self::Damaged;
        };
        let list: Vec<(u32, Vec<OwnTab>)> = own
            .0
            .iter()
            .filter_map(|(reference, tabs)| Some((reference.parse().ok()?, tabs.clone())))
            .collect();
        if list.is_empty() {
            Self::Operation
        } else {
            Self::Own(list)
        }
    }

    /// The own tabs of outline `index`, `None` when it follows the operation.
    #[must_use]
    pub fn own(&self, index: u32) -> Option<&[OwnTab]> {
        match self {
            Self::Own(list) => list
                .iter()
                .find(|(at, _)| *at == index)
                .map(|(_, tabs)| tabs.as_slice()),
            Self::Operation | Self::Damaged => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_register_reads_back_equal_and_keeps_its_order() {
        let mut own = OwnTabs::default();
        own.set(
            "0".to_owned(),
            vec![
                OwnTab {
                    at: PathFraction::new(750_000_000).unwrap(),
                    width: None,
                },
                OwnTab {
                    at: PathFraction::new(250_000_000).unwrap(),
                    width: CutLength::tab(120),
                },
            ],
        );
        own.set("7:3/9:12:1".to_owned(), Vec::new());
        let value = own.to_value();
        assert_eq!(
            value,
            json!({ "0": [{ "at": 250_000_000, "width_mm": 1.2 }, { "at": 750_000_000 }],
                    "7:3/9:12:1": [] })
        );
        assert_eq!(read_own_tabs(&value), Ok(own.clone()));
        assert_eq!(own.count(), 2);
    }

    #[test]
    fn a_damaged_register_names_its_damage() {
        let read = |v: Value| read_own_tabs(&v);
        assert_eq!(read(json!([1])), Err(RecordDamage::NotAMap));
        assert_eq!(read(json!({ "0": 3 })), Err(RecordDamage::NotAMap));
        assert_eq!(
            read(json!({ "0": [{ "at": 1, "x": 2 }] })),
            Err(RecordDamage::UnknownField)
        );
        assert_eq!(
            read(json!({ "0": [{ "at": "a" }] })),
            Err(RecordDamage::WrongType)
        );
        assert_eq!(
            read(json!({ "0": [{ "width_mm": 1.0 }] })),
            Err(RecordDamage::WrongType)
        );
        assert_eq!(
            read(json!({ "0": [{ "at": 1_000_000_000_u64 }] })),
            Err(RecordDamage::BadFraction)
        );
        assert_eq!(
            read(json!({ "0": [{ "at": -1 }] })),
            Err(RecordDamage::WrongType)
        );
        assert_eq!(
            read(json!({ "0": [{ "at": 1, "width_mm": 20.01 }] })),
            Err(RecordDamage::BadWidth)
        );
        assert_eq!(
            read(json!({ "0": [{ "at": 5 }, { "at": 5 }] })),
            Err(RecordDamage::Overlap)
        );
    }

    #[test]
    fn the_nodes_below_the_target_follow_the_chain() {
        let n = |c: u64| crate::operation_nearest::tests::id(c);
        // A leaf in a group in a group: below the outer group are the inner group and the leaf.
        let chain = vec![vec![n(1)], vec![n(2)], vec![n(3)]];
        assert_eq!(below_target(&chain, &[n(3)]), [n(2), n(1)]);
        assert_eq!(below_target(&chain, &[n(1)]), Vec::<NodeId>::new());
        // A clone member: its clone, a group of the original, the member.
        let chain = vec![
            vec![n(7), n(8), n(9)],
            vec![n(7), n(8)],
            vec![n(7)],
            vec![n(4)],
        ];
        assert_eq!(below_target(&chain, &[n(7)]), [n(8), n(9)]);
        assert_eq!(below_target(&chain, &[n(4)]), [n(7), n(8), n(9)]);
        let record = json!({ "0": [{ "at": 1 }], "2": [], "x:1": [] });
        let tabs = LeafTabs::of_contours(Some(&record));
        assert!(tabs.own(0).is_some() && tabs.own(2).is_some() && tabs.own(1).is_none());
    }

    #[test]
    fn a_leaf_takes_its_outlines_and_ignores_references_it_does_not_have() {
        let record = json!({ "g:1": [{ "at": 10 }], "g:5": [{ "at": 20 }], "0": [] });
        let below = vec!["g".to_owned()];
        let tabs = LeafTabs::of(Some(&record), &below, 3);
        assert_eq!(tabs.own(1).map(<[OwnTab]>::len), Some(1));
        assert_eq!(tabs.own(0), None, "another leaf's reference");
        assert_eq!(LeafTabs::of(None, &below, 3), LeafTabs::Operation);
        assert_eq!(
            LeafTabs::of(Some(&json!({ "0": [] })), &[], 2).own(0),
            Some(&[][..])
        );
        assert_eq!(LeafTabs::of(Some(&json!(1)), &[], 1), LeafTabs::Damaged);
        assert_eq!(outline_ref(&[], 2), "2");
        assert_eq!(outline_ref(&["a".into(), "b".into()], 0), "a/b:0");
        assert_eq!(PathFraction::of(1.0).ppb(), 0, "a closed path wraps");
        assert_eq!(PathFraction::of(0.25).ppb(), 250_000_000);
    }
}
