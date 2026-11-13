//! The maker's synced library list of the device group (ADR 0062 decisions 1, 2 and 4,
//! `specs/0116-library-sync`): a Loro document of registers, with no Loro type in its API.
//!
//! The root map `library` holds `format_version`; the maps `machines` and `materials` map a
//! record id (a lowercase UUID v4) to `{sync, text}`. Each value is one register: the last
//! writer wins. `text` is the exact record text the writers of ADR 0038/0055/0058 produce;
//! `sync` is the per-item toggle that travels. An item switched off keeps its entry as a marker
//! without `text`, and every device keeps its local copy; deleting an item removes the entry.
//! Entries whose key or value has another shape are skipped on read, so a damaged or newer entry
//! hides only itself; fields a reader does not know are ignored. The record texts themselves are
//! never gated here: the envelope rules of ADR 0055 §7 apply on apply, on every device.

use std::collections::HashMap;

use loro::{
    ContainerID, ContainerType, ExportMode, JsonMapOp, JsonOpContent, LoroDoc, LoroMap, LoroValue,
    ValueOrContainer, VersionVector,
};

use crate::document::DocumentVersion;

/// The library list format this build writes and the newest it reads.
pub const LIBRARY_LIST_FORMAT_VERSION: i64 = 1;
/// The largest machine record text an entry holds: the machine record reader's limit
/// (`curvyo-laser-core`'s `MAX_RECORD_BYTES`, plan P2).
pub const MAX_MACHINE_TEXT_BYTES: usize = 128 * 1024;
/// The largest material record text an entry holds: the material record reader's limit
/// (`curvyo-laser-core`'s `MAX_RECORD_TEXT`, plan P2).
pub const MAX_MATERIAL_TEXT_BYTES: usize = 512 * 1024;

const ROOT: &str = "library";
const MACHINES: &str = "machines";
const MATERIALS: &str = "materials";
const KEY_FORMAT_VERSION: &str = "format_version";
const KEY_SYNC: &str = "sync";
const KEY_TEXT: &str = "text";

/// Why a library list operation or import did nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LibraryListError {
    /// An entry id is not a lowercase UUID v4.
    #[error("a library item needs a record id")]
    BadId,
    /// A record text is longer than its entry's limit.
    #[error("the record text is too large for the library")]
    TooLarge,
    /// The list could not be encoded or written.
    #[error("the library list could not be encoded")]
    Encode,
    /// The bytes are not a library list this build reads.
    #[error("the bytes are not a library list")]
    Malformed,
    /// The list was written by a newer build.
    #[error("the library list was written by a newer version")]
    FormatTooNew,
}

/// Which of the maker's libraries an entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LibraryKind {
    /// Machine profiles: `machines.<uuid>`, the record texts of ADR 0038.
    Machine,
    /// Material records: `materials.<uuid>`, the record texts of ADR 0055/0058.
    Material,
}

impl LibraryKind {
    /// The root map's name of the kind.
    #[must_use]
    pub const fn map(self) -> &'static str {
        match self {
            Self::Machine => MACHINES,
            Self::Material => MATERIALS,
        }
    }

    /// The largest record text an entry of the kind holds.
    #[must_use]
    pub const fn max_text_bytes(self) -> usize {
        match self {
            Self::Machine => MAX_MACHINE_TEXT_BYTES,
            Self::Material => MAX_MATERIAL_TEXT_BYTES,
        }
    }
}

/// One library item the stream knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryEntry {
    /// The record id, a lowercase UUID v4.
    pub id: String,
    /// Whether the item's sync is on.
    pub sync: bool,
    /// The record text; absent while the item is switched off (the marker entry).
    pub text: Option<String>,
}

/// One replica of the library list.
pub struct LibraryList {
    doc: LoroDoc,
}

impl LibraryList {
    /// An empty list bound to the Loro peer `peer` (drawn by the caller).
    ///
    /// # Panics
    /// Does not panic in practice: a fresh root map takes a known-valid key.
    #[must_use]
    pub fn new(peer: u64) -> Self {
        let list = Self::bound(peer);
        // invariant: a fresh, attached root map takes a known-valid key.
        #[allow(clippy::unwrap_used)]
        list.doc
            .get_map(ROOT)
            .insert(KEY_FORMAT_VERSION, LIBRARY_LIST_FORMAT_VERSION)
            .unwrap();
        list.doc.commit();
        list
    }

    /// The list a snapshot holds, bound to the Loro peer `peer`.
    ///
    /// # Errors
    /// [`LibraryListError::Malformed`], [`LibraryListError::FormatTooNew`].
    pub fn from_snapshot(peer: u64, bytes: &[u8]) -> Result<Self, LibraryListError> {
        let list = Self::bound(peer);
        list.doc
            .import(bytes)
            .map_err(|_| LibraryListError::Malformed)?;
        list.check_format()?;
        Ok(list)
    }

    fn bound(peer: u64) -> Self {
        let doc = LoroDoc::new();
        // invariant: a fresh doc with no operations takes any peer id.
        #[allow(clippy::unwrap_used)]
        doc.set_peer_id(peer).unwrap();
        Self { doc }
    }

    fn check_format(&self) -> Result<(), LibraryListError> {
        check_format(&self.doc)
    }

    /// The list's version, for [`LibraryList::export_updates`].
    #[must_use]
    pub fn version(&self) -> DocumentVersion {
        DocumentVersion(self.doc.oplog_frontiers())
    }

    /// Everything this replica holds since `since`.
    ///
    /// # Errors
    /// [`LibraryListError::Encode`].
    pub fn export_updates(&self, since: &DocumentVersion) -> Result<Vec<u8>, LibraryListError> {
        self.doc.commit();
        let known = self.doc.frontiers_to_vv(&since.0).unwrap_or_default();
        self.doc
            .export(ExportMode::updates(&known))
            .map_err(|_| LibraryListError::Encode)
    }

    /// The whole list, for `library.loro` and the group room's snapshot.
    ///
    /// # Errors
    /// [`LibraryListError::Encode`].
    pub fn snapshot(&self) -> Result<Vec<u8>, LibraryListError> {
        self.doc.commit();
        self.doc
            .export(ExportMode::Snapshot)
            .map_err(|_| LibraryListError::Encode)
    }

    /// Merges updates of another replica. Updates that write a newer `format_version` are
    /// refused and leave the list unchanged (ADR 0040 decision 11), also when the write would
    /// not win the register; so are bytes that do not import.
    ///
    /// # Errors
    /// [`LibraryListError::Malformed`], [`LibraryListError::FormatTooNew`].
    pub fn import_updates(&self, bytes: &[u8]) -> Result<(), LibraryListError> {
        // The bytes on their own (changes whose history is not in them stay pending there), then
        // merged into a copy of this replica (changes it already holds are not new there).
        let alone = LoroDoc::new();
        alone
            .import(bytes)
            .map_err(|_| LibraryListError::Malformed)?;
        let trial = self.doc.fork();
        let before = trial.oplog_vv();
        trial
            .import(bytes)
            .map_err(|_| LibraryListError::Malformed)?;
        if writes_newer_format(&alone, &VersionVector::default())
            || writes_newer_format(&trial, &before)
        {
            return Err(LibraryListError::FormatTooNew);
        }
        check_format(&trial)?;
        self.doc
            .import(bytes)
            .map_err(|_| LibraryListError::Malformed)?;
        self.check_format()
    }

    /// Records the item `id` of `kind` as switched on with `text`, the exact record text.
    ///
    /// # Errors
    /// [`LibraryListError::BadId`], [`LibraryListError::TooLarge`],
    /// [`LibraryListError::Encode`].
    pub fn set(&self, kind: LibraryKind, id: &str, text: &str) -> Result<(), LibraryListError> {
        if !is_record_id(id) {
            return Err(LibraryListError::BadId);
        }
        if text.len() > kind.max_text_bytes() {
            return Err(LibraryListError::TooLarge);
        }
        let mut value = HashMap::new();
        value.insert(KEY_SYNC.to_owned(), LoroValue::from(true));
        value.insert(KEY_TEXT.to_owned(), LoroValue::from(text));
        self.write(kind.map(), id, value)
    }

    /// Switches the item `id` of `kind` off: its entry stays as a marker without text (nothing
    /// is deleted anywhere, criterion 10); an item that never synced is not written at all
    /// (criterion 4).
    ///
    /// # Errors
    /// [`LibraryListError::BadId`], [`LibraryListError::Encode`].
    pub fn switch_off(&self, kind: LibraryKind, id: &str) -> Result<(), LibraryListError> {
        if !is_record_id(id) {
            return Err(LibraryListError::BadId);
        }
        if self.entry(kind, id).is_none() {
            return Ok(());
        }
        let mut value = HashMap::new();
        value.insert(KEY_SYNC.to_owned(), LoroValue::from(false));
        self.write(kind.map(), id, value)
    }

    /// Deletes the item `id` of `kind`: the entry goes, and synced devices delete their local
    /// copy (criterion 11); an absent one does nothing.
    ///
    /// # Errors
    /// [`LibraryListError::Encode`].
    pub fn remove(&self, kind: LibraryKind, id: &str) -> Result<(), LibraryListError> {
        let map = self.doc.get_map(kind.map());
        if map.get(id).is_none() {
            return Ok(());
        }
        map.delete(id).map_err(|_| LibraryListError::Encode)?;
        self.doc.commit();
        Ok(())
    }

    /// Every entry of `kind` the list knows, on, off or damaged-skipped, sorted by id.
    #[must_use]
    pub fn entries(&self, kind: LibraryKind) -> Vec<LibraryEntry> {
        let mut out: Vec<LibraryEntry> = entries(&self.doc.get_map(kind.map()))
            .into_iter()
            .filter_map(|(key, fields)| {
                if !is_record_id(&key) {
                    return None;
                }
                let sync = match fields.get(KEY_SYNC) {
                    Some(LoroValue::Bool(on)) => *on,
                    _ => return None,
                };
                let text = match fields.get(KEY_TEXT) {
                    Some(LoroValue::String(text)) => Some(text.to_string()),
                    None => None,
                    _ => return None,
                };
                Some(LibraryEntry {
                    id: key,
                    sync,
                    text,
                })
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// The entry of the item `id` of `kind`, if the list knows it.
    #[must_use]
    pub fn entry(&self, kind: LibraryKind, id: &str) -> Option<LibraryEntry> {
        self.entries(kind).into_iter().find(|entry| entry.id == id)
    }

    fn write(
        &self,
        map: &str,
        key: &str,
        value: HashMap<String, LoroValue>,
    ) -> Result<(), LibraryListError> {
        self.doc
            .get_map(map)
            .insert(key, LoroValue::from(value))
            .map_err(|_| LibraryListError::Encode)?;
        self.doc.commit();
        Ok(())
    }
}

/// The map's entries whose value is a plain map.
fn entries(map: &LoroMap) -> Vec<(String, HashMap<String, LoroValue>)> {
    let mut out = Vec::new();
    map.for_each(|key, value| {
        if let ValueOrContainer::Value(LoroValue::Map(fields)) = value {
            out.push((
                key.to_owned(),
                fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            ));
        }
    });
    out
}

/// Whether `id` is a lowercase UUID v4: the id of a user record (the rule of ADR 0038 §5, kept
/// here because this crate does not read `curvyo-laser-core`).
fn is_record_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, &b)| match index {
            8 | 13 | 18 | 23 => b == b'-',
            14 => b == b'4',
            19 => matches!(b, b'8' | b'9' | b'a' | b'b'),
            _ => b.is_ascii_digit() || (b'a'..=b'f').contains(&b),
        })
}

/// The list's `format_version` must be one this build reads.
fn check_format(doc: &LoroDoc) -> Result<(), LibraryListError> {
    match doc.get_map(ROOT).get(KEY_FORMAT_VERSION) {
        Some(ValueOrContainer::Value(LoroValue::I64(version))) => {
            if version > LIBRARY_LIST_FORMAT_VERSION {
                Err(LibraryListError::FormatTooNew)
            } else {
                Ok(())
            }
        }
        _ => Err(LibraryListError::Malformed),
    }
}

/// Whether a change `doc` holds beyond `before` writes a `format_version` newer than this build
/// reads, whether or not that write wins the register.
fn writes_newer_format(doc: &LoroDoc, before: &VersionVector) -> bool {
    let root = ContainerID::new_root(ROOT, ContainerType::Map);
    doc.export_json_updates(before, &doc.oplog_vv())
        .changes
        .iter()
        .flat_map(|change| &change.ops)
        .any(|op| {
            op.container == root
                && matches!(
                    &op.content,
                    JsonOpContent::Map(JsonMapOp::Insert {
                        key,
                        value: LoroValue::I64(version),
                    }) if key == KEY_FORMAT_VERSION && *version > LIBRARY_LIST_FORMAT_VERSION
                )
        })
}
