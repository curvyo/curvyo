//! The Came from record of an imported page or object (ADR 0084 §7, `specs/0137` criteria 6, 7
//! and 9): where a copy was taken from, stored as one atomic map on the node or the page.
//!
//! The record never changes how anything draws or outputs, so a damaged one reads as absent and
//! is kept untouched; the file is never refused for it. Keys this build does not know are kept,
//! because the stored map is copied as it is and only ever replaced whole.

use loro::{LoroMap, LoroValue, ValueOrContainer};

use crate::document::{Document, OBJECTS_TREE, ROOT_MAP};
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

/// The node meta key and the page register field of the record.
pub(crate) const KEY_CAME_FROM: &str = "came_from";

const FIELD_DOCUMENT: &str = "document";
const FIELD_PAGE: &str = "page";
const FIELD_PAGE_ID: &str = "page_id";
const FIELD_ITEM: &str = "item";
const FIELD_AT: &str = "at";

/// A moment as Unix milliseconds in UTC. The caller reads the clock; this crate has none.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct UnixMillis(i64);

impl UnixMillis {
    /// The moment `millis` milliseconds after 1970-01-01T00:00:00Z.
    #[must_use]
    pub const fn new(millis: i64) -> Self {
        Self(millis)
    }

    /// The milliseconds since 1970-01-01T00:00:00Z.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

/// Where an imported page or object came from (ADR 0084 §7).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CameFrom {
    /// The source document's file name without its extension.
    pub document: String,
    /// The source page's name.
    pub page: String,
    /// The source page's id, by which Open source finds the document again.
    pub page_id: String,
    /// The copied root's own name, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// When the import was made.
    pub at: UnixMillis,
}

impl CameFrom {
    /// The record as the one plain map it is stored as.
    pub(crate) fn to_value(&self) -> LoroValue {
        let mut map = std::collections::HashMap::new();
        map.insert(
            FIELD_DOCUMENT.to_owned(),
            LoroValue::from(self.document.as_str()),
        );
        map.insert(FIELD_PAGE.to_owned(), LoroValue::from(self.page.as_str()));
        map.insert(
            FIELD_PAGE_ID.to_owned(),
            LoroValue::from(self.page_id.as_str()),
        );
        if let Some(item) = &self.item {
            map.insert(FIELD_ITEM.to_owned(), LoroValue::from(item.as_str()));
        }
        map.insert(FIELD_AT.to_owned(), LoroValue::I64(self.at.get()));
        LoroValue::from(map)
    }

    /// The record of a stored value; `None` for a value of the wrong type or with a required
    /// field missing or of the wrong type.
    pub(crate) fn of_value(value: &LoroValue) -> Option<Self> {
        let LoroValue::Map(map) = value else {
            return None;
        };
        let text = |field: &str| match map.get(field) {
            Some(LoroValue::String(text)) => Some(text.to_string()),
            _ => None,
        };
        let item = match map.get(FIELD_ITEM) {
            None => None,
            Some(LoroValue::String(text)) => Some(text.to_string()),
            Some(_) => return None,
        };
        let at = match map.get(FIELD_AT) {
            Some(LoroValue::I64(millis)) => UnixMillis::new(*millis),
            _ => return None,
        };
        Some(Self {
            document: text(FIELD_DOCUMENT)?,
            page: text(FIELD_PAGE)?,
            page_id: text(FIELD_PAGE_ID)?,
            item,
            at,
        })
    }

    /// This record for a copied root named `item`.
    #[must_use]
    pub fn with_item(&self, item: Option<String>) -> Self {
        Self {
            item,
            ..self.clone()
        }
    }
}

/// The stored record value of a node's meta map, as it is (damaged or not).
pub(crate) fn stored(meta: &LoroMap) -> Option<LoroValue> {
    match meta.get(KEY_CAME_FROM)? {
        ValueOrContainer::Value(value) => Some(value),
        ValueOrContainer::Container(_) => None,
    }
}

/// The readable record of a node's meta map.
pub(crate) fn read(meta: &LoroMap) -> Option<CameFrom> {
    CameFrom::of_value(&stored(meta)?)
}

/// Writes `record` on a node's meta map, uncommitted.
pub(crate) fn write(meta: &LoroMap, record: &CameFrom) {
    // invariant: an attached meta map takes a plain value under any key.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_CAME_FROM, record.to_value()).unwrap();
}

/// The one stored value every one of `metas` carries, when they all carry an equal one (ADR 0084
/// §7: a result of several sources keeps the record only then).
pub(crate) fn shared(metas: impl IntoIterator<Item = LoroMap>) -> Option<LoroValue> {
    let mut common: Option<LoroValue> = None;
    for meta in metas {
        let value = stored(&meta)?;
        match &common {
            Some(seen) if *seen != value => return None,
            Some(_) => {}
            None => common = Some(value),
        }
    }
    common
}

/// Writes the stored `value` as it is on a node's meta map, uncommitted.
pub(crate) fn write_stored(meta: &LoroMap, value: LoroValue) {
    // invariant: an attached meta map takes a plain value under any key.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_CAME_FROM, value).unwrap();
}

/// Writes `record` as the page register of `page_id`, uncommitted.
pub(crate) fn write_page(root: &LoroMap, page_id: &str, record: &CameFrom) {
    crate::pages::write(root, page_id, KEY_CAME_FROM, record.to_value());
}

impl Document {
    /// The Came from record of the object `id`; `None` when it has none, it is damaged, or the
    /// object does not exist.
    #[must_use]
    pub fn came_from(&self, id: NodeId) -> Option<CameFrom> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        read(&tree.get_meta(tree_id_of(id)).ok()?)
    }

    /// The Came from record of the page `id`; `None` when it has none or it is damaged.
    #[must_use]
    pub fn page_came_from(&self, id: &str) -> Option<CameFrom> {
        let root = self.loro().get_map(ROOT_MAP);
        match root.get(&crate::pages::key(id, KEY_CAME_FROM))? {
            ValueOrContainer::Value(value) => CameFrom::of_value(&value),
            ValueOrContainer::Container(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> CameFrom {
        CameFrom {
            document: "Lid".into(),
            page: "Front".into(),
            page_id: "7-3".into(),
            item: Some("Handle".into()),
            at: UnixMillis::new(1_764_000_000_000),
        }
    }

    #[test]
    fn a_record_reads_back_from_its_value() {
        assert_eq!(CameFrom::of_value(&record().to_value()), Some(record()));
        let plain = record().with_item(None);
        assert_eq!(CameFrom::of_value(&plain.to_value()), Some(plain));
    }

    #[test]
    fn a_wrong_type_or_a_missing_field_reads_as_absent() {
        assert_eq!(CameFrom::of_value(&LoroValue::from("Lid")), None);
        let LoroValue::Map(map) = record().to_value() else {
            panic!("a map")
        };
        for field in [FIELD_DOCUMENT, FIELD_PAGE, FIELD_PAGE_ID, FIELD_AT] {
            let mut partial = (*map).clone();
            partial.remove(field);
            assert_eq!(CameFrom::of_value(&LoroValue::from(partial)), None);
        }
        let mut wrong_item = (*map).clone();
        wrong_item.insert(FIELD_ITEM.into(), LoroValue::I64(3));
        assert_eq!(CameFrom::of_value(&LoroValue::from(wrong_item)), None);
        let mut wrong_at = (*map).clone();
        wrong_at.insert(FIELD_AT.into(), LoroValue::Double(3.0));
        assert_eq!(CameFrom::of_value(&LoroValue::from(wrong_at)), None);
    }

    #[test]
    fn unknown_keys_do_not_stop_the_read() {
        let LoroValue::Map(map) = record().to_value() else {
            panic!("a map")
        };
        let mut more = (*map).clone();
        more.insert("source_url".into(), LoroValue::from("https://example.org"));
        assert_eq!(CameFrom::of_value(&LoroValue::from(more)), Some(record()));
    }
}
