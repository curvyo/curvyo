//! The devices list of a maker's device group (ADR 0040 decision 1, `specs/0096-own-devices-sync`):
//! a Loro document of registers, with no Loro type in its API.
//!
//! The root map `group` holds `format_version`; the map `devices` maps a member id (64 lower-case
//! hex characters) to `{label, browser?}`; the map `workspaces` maps a workspace id (32 hex
//! characters) to `{name, on}`. Each value is one register: the last writer wins. Entries whose
//! key or value has another shape are skipped on read, so a damaged or newer entry hides only
//! itself; fields a reader does not know are ignored, so `browser` (added in `format_version` 1,
//! optional, `false` when absent) needs no new version. A build that does not know it drops it
//! when it renames the device, which shows that device as a computer again.

use std::collections::HashMap;

use loro::{
    ContainerID, ContainerType, ExportMode, JsonMapOp, JsonOpContent, LoroDoc, LoroMap, LoroValue,
    ValueOrContainer, VersionVector,
};

use crate::document::DocumentVersion;

/// The devices list format this build writes and the newest it reads.
pub const DEVICES_LIST_FORMAT_VERSION: i64 = 1;

const ROOT: &str = "group";
const DEVICES: &str = "devices";
const WORKSPACES: &str = "workspaces";
const KEY_FORMAT_VERSION: &str = "format_version";
const KEY_LABEL: &str = "label";
const KEY_BROWSER: &str = "browser";
const KEY_NAME: &str = "name";
const KEY_ON: &str = "on";

/// Why a devices list operation or import did nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DevicesListError {
    /// A device label is empty or only white space.
    #[error("a device needs a name")]
    EmptyLabel,
    /// The list could not be encoded or written.
    #[error("the devices list could not be encoded")]
    Encode,
    /// The bytes are not a devices list this build reads.
    #[error("the bytes are not a devices list")]
    Malformed,
    /// The list was written by a newer build.
    #[error("the devices list was written by a newer version")]
    FormatTooNew,
}

/// One paired device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEntry {
    /// The device's member id (its ed25519 public key).
    pub member: [u8; 32],
    /// The name the maker gave it.
    pub label: String,
    /// Whether it is the app in a browser (it marks itself, ADR 0040 decision 1).
    pub browser: bool,
}

/// One workspace the group knows, switched on or off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncedWorkspace {
    /// The workspace id.
    pub id: [u8; 16],
    /// The workspace's name when it was last switched.
    pub name: String,
    /// Whether "Sync to my devices" is on.
    pub on: bool,
}

/// One replica of the devices list.
pub struct DevicesList {
    doc: LoroDoc,
}

impl DevicesList {
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
            .insert(KEY_FORMAT_VERSION, DEVICES_LIST_FORMAT_VERSION)
            .unwrap();
        list.doc.commit();
        list
    }

    /// The list a snapshot holds, bound to the Loro peer `peer`.
    ///
    /// # Errors
    /// [`DevicesListError::Malformed`], [`DevicesListError::FormatTooNew`].
    pub fn from_snapshot(peer: u64, bytes: &[u8]) -> Result<Self, DevicesListError> {
        let list = Self::bound(peer);
        list.doc
            .import(bytes)
            .map_err(|_| DevicesListError::Malformed)?;
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

    fn check_format(&self) -> Result<(), DevicesListError> {
        check_format(&self.doc)
    }

    /// The list's version, for [`DevicesList::export_updates`].
    #[must_use]
    pub fn version(&self) -> DocumentVersion {
        DocumentVersion(self.doc.oplog_frontiers())
    }

    /// Everything this replica holds since `since`.
    ///
    /// # Errors
    /// [`DevicesListError::Encode`].
    pub fn export_updates(&self, since: &DocumentVersion) -> Result<Vec<u8>, DevicesListError> {
        self.doc.commit();
        let known = self.doc.frontiers_to_vv(&since.0).unwrap_or_default();
        self.doc
            .export(ExportMode::updates(&known))
            .map_err(|_| DevicesListError::Encode)
    }

    /// The whole list, for `devices.loro` and the group room's snapshot.
    ///
    /// # Errors
    /// [`DevicesListError::Encode`].
    pub fn snapshot(&self) -> Result<Vec<u8>, DevicesListError> {
        self.doc.commit();
        self.doc
            .export(ExportMode::Snapshot)
            .map_err(|_| DevicesListError::Encode)
    }

    /// Merges updates of another replica. Updates that write a newer `format_version` are
    /// refused and leave the list unchanged (ADR 0040 decision 11), also when the write would
    /// not win the register; so are bytes that do not import.
    ///
    /// # Errors
    /// [`DevicesListError::Malformed`], [`DevicesListError::FormatTooNew`].
    pub fn import_updates(&self, bytes: &[u8]) -> Result<(), DevicesListError> {
        // The bytes on their own (changes whose history is not in them stay pending there), then
        // merged into a copy of this replica (changes it already holds are not new there).
        let alone = LoroDoc::new();
        alone
            .import(bytes)
            .map_err(|_| DevicesListError::Malformed)?;
        let trial = self.doc.fork();
        let before = trial.oplog_vv();
        trial
            .import(bytes)
            .map_err(|_| DevicesListError::Malformed)?;
        if writes_newer_format(&alone, &VersionVector::default())
            || writes_newer_format(&trial, &before)
        {
            return Err(DevicesListError::FormatTooNew);
        }
        check_format(&trial)?;
        self.doc
            .import(bytes)
            .map_err(|_| DevicesListError::Malformed)?;
        self.check_format()
    }

    /// Sets the label of the device `member`, adding it if absent.
    ///
    /// # Errors
    /// [`DevicesListError::EmptyLabel`], [`DevicesListError::Encode`].
    pub fn set_label(&self, member: [u8; 32], label: &str) -> Result<(), DevicesListError> {
        if label.trim().is_empty() {
            return Err(DevicesListError::EmptyLabel);
        }
        let browser = self.device(&member).is_some_and(|d| d.browser);
        self.write_device(member, label, browser)
    }

    /// Marks the listed device `member` as a browser; nothing when it is not listed (yet).
    ///
    /// # Errors
    /// [`DevicesListError::Encode`].
    pub fn mark_browser(&self, member: [u8; 32]) -> Result<(), DevicesListError> {
        match self.device(&member) {
            Some(device) if !device.browser => self.write_device(member, &device.label, true),
            _ => Ok(()),
        }
    }

    fn write_device(
        &self,
        member: [u8; 32],
        label: &str,
        browser: bool,
    ) -> Result<(), DevicesListError> {
        let mut value = HashMap::new();
        value.insert(KEY_LABEL.to_owned(), LoroValue::from(label));
        if browser {
            value.insert(KEY_BROWSER.to_owned(), LoroValue::from(true));
        }
        self.write(DEVICES, &hex(&member), value)
    }

    /// Removes the device `member`; nothing when it is not listed.
    ///
    /// # Errors
    /// [`DevicesListError::Encode`].
    pub fn remove_device(&self, member: &[u8; 32]) -> Result<(), DevicesListError> {
        let devices = self.doc.get_map(DEVICES);
        let key = hex(member);
        if devices.get(&key).is_none() {
            return Ok(());
        }
        devices.delete(&key).map_err(|_| DevicesListError::Encode)?;
        self.doc.commit();
        Ok(())
    }

    /// Records the workspace `id` under `name` as switched on or off.
    ///
    /// # Errors
    /// [`DevicesListError::Encode`].
    pub fn switch_workspace(
        &self,
        id: [u8; 16],
        name: &str,
        on: bool,
    ) -> Result<(), DevicesListError> {
        let mut value = HashMap::new();
        value.insert(KEY_NAME.to_owned(), LoroValue::from(name));
        value.insert(KEY_ON.to_owned(), LoroValue::from(on));
        self.write(WORKSPACES, &hex(&id), value)
    }

    fn write(
        &self,
        map: &str,
        key: &str,
        value: HashMap<String, LoroValue>,
    ) -> Result<(), DevicesListError> {
        self.doc
            .get_map(map)
            .insert(key, LoroValue::from(value))
            .map_err(|_| DevicesListError::Encode)?;
        self.doc.commit();
        Ok(())
    }

    /// Every listed device, sorted by member id.
    #[must_use]
    pub fn devices(&self) -> Vec<DeviceEntry> {
        let mut out: Vec<DeviceEntry> = entries(&self.doc.get_map(DEVICES))
            .into_iter()
            .filter_map(|(key, fields)| {
                Some(DeviceEntry {
                    member: unhex(&key)?,
                    label: string_field(&fields, KEY_LABEL)?,
                    browser: matches!(fields.get(KEY_BROWSER), Some(LoroValue::Bool(true))),
                })
            })
            .collect();
        out.sort_by_key(|device| device.member);
        out
    }

    /// The entry of the device `member`, if it is listed.
    #[must_use]
    pub fn device(&self, member: &[u8; 32]) -> Option<DeviceEntry> {
        self.devices()
            .into_iter()
            .find(|device| device.member == *member)
    }

    /// The label of the device `member`, if it is listed.
    #[must_use]
    pub fn label(&self, member: &[u8; 32]) -> Option<String> {
        self.device(member).map(|device| device.label)
    }

    /// Every workspace the list knows, on or off, sorted by id.
    #[must_use]
    pub fn workspaces(&self) -> Vec<SyncedWorkspace> {
        let mut out: Vec<SyncedWorkspace> = entries(&self.doc.get_map(WORKSPACES))
            .into_iter()
            .filter_map(|(key, fields)| {
                let on = match fields.get(KEY_ON) {
                    Some(LoroValue::Bool(on)) => *on,
                    _ => return None,
                };
                Some(SyncedWorkspace {
                    id: unhex(&key)?,
                    name: string_field(&fields, KEY_NAME)?,
                    on,
                })
            })
            .collect();
        out.sort_by_key(|workspace| workspace.id);
        out
    }

    /// The entry of the workspace `id`, if the list knows it.
    #[must_use]
    pub fn workspace(&self, id: &[u8; 16]) -> Option<SyncedWorkspace> {
        self.workspaces()
            .into_iter()
            .find(|workspace| workspace.id == *id)
    }

    /// The ids of every workspace switched on, sorted.
    #[must_use]
    pub fn switched_on(&self) -> Vec<[u8; 16]> {
        self.workspaces()
            .into_iter()
            .filter(|workspace| workspace.on)
            .map(|workspace| workspace.id)
            .collect()
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

fn string_field(fields: &HashMap<String, LoroValue>, key: &str) -> Option<String> {
    match fields.get(key) {
        Some(LoroValue::String(text)) => Some(text.to_string()),
        _ => None,
    }
}

/// The list's `format_version` must be one this build reads.
fn check_format(doc: &LoroDoc) -> Result<(), DevicesListError> {
    match doc.get_map(ROOT).get(KEY_FORMAT_VERSION) {
        Some(ValueOrContainer::Value(LoroValue::I64(version))) => {
            if version > DEVICES_LIST_FORMAT_VERSION {
                Err(DevicesListError::FormatTooNew)
            } else {
                Ok(())
            }
        }
        _ => Err(DevicesListError::Malformed),
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
                    }) if key == KEY_FORMAT_VERSION && *version > DEVICES_LIST_FORMAT_VERSION
                )
        })
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, byte| {
        // Writing to a `String` never fails.
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// The bytes of lower-case hex text of exactly `N` bytes; `None` for any other text.
fn unhex<const N: usize>(text: &str) -> Option<[u8; N]> {
    if text.len() != 2 * N {
        return None;
    }
    let mut out = [0; N];
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let (pairs, _) = text.as_bytes().as_chunks::<2>();
    for (place, [high, low]) in out.iter_mut().zip(pairs) {
        *place = (digit(*high)? << 4) | digit(*low)?;
    }
    Some(out)
}
