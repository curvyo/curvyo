//! The shared workspace's `wasm-bindgen` surface (`specs/0095-workspace` criteria 25 to 33):
//! Share workspace's preparation, joining and resuming the workspace room, the link that moves
//! between document sessions, the files and the marker the host writes, and the fixed words.
//! Pass-throughs to `session::workspace_sync`, the share builder and `curvyo-ui-core`.

use curvyo_sync_core::{InviteKind, InviteLink};
use curvyo_ui_core::share_text::{self, JoinRefusal};
use curvyo_ui_core::workspace_share_text as texts;
use wasm_bindgen::prelude::*;

use crate::session::{
    BackgroundKind, PreparedShare, SyncSeeds, WorkspaceLink, WorkspaceWrite, hex, unhex,
};
use crate::wasm_api::WasmSession;
use crate::{NotAdded, WorkspaceShareBuilder};

fn millis(now: f64) -> curvyo_sync_core::Millis {
    // Date.now() is a whole number of milliseconds well inside u64.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    curvyo_sync_core::Millis(now.max(0.0) as u64)
}

fn seeds(identity: &[u8], nonce: &[u8], entropy: &[u8]) -> Option<SyncSeeds> {
    Some(SyncSeeds {
        identity: identity.try_into().ok()?,
        nonce: nonce.try_into().ok()?,
        entropy: entropy.try_into().ok()?,
    })
}

fn path_of(json: &str) -> Vec<String> {
    serde_json::from_str(json).unwrap_or_default()
}

/// Share workspace's preparation: documents added one by one, then prepared; the files stay
/// here for the host to write once the relay took the snapshot.
#[wasm_bindgen]
pub struct WasmWorkspaceShare {
    builder: Option<WorkspaceShareBuilder>,
    prepared: Option<PreparedShare>,
    files: crate::workspace_share_builder::ShareFiles,
    not_added: Vec<String>,
    taken_in: Vec<(Vec<String>, [u8; 16])>,
}

#[wasm_bindgen]
impl WasmWorkspaceShare {
    /// A preparation of the fresh workspace id `workspace` (16 bytes) at `now` (ms), the list
    /// bound to `peer`, creation stamps from `seed` (16 random bytes).
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(workspace: &[u8], peer: u64, now: f64, seed: &[u8]) -> Self {
        let workspace: [u8; 16] = workspace.try_into().unwrap_or_default();
        let seed: [u8; 16] = seed.try_into().unwrap_or_default();
        #[allow(clippy::cast_possible_truncation)]
        let at = (now / 1000.0).floor() as i64;
        Self {
            builder: Some(WorkspaceShareBuilder::new(workspace, peer, at, seed)),
            prepared: None,
            files: Vec::new(),
            not_added: Vec::new(),
            taken_in: Vec::new(),
        }
    }

    /// The device identity (64 bytes of entropy, as the sessions take it): a document shared on
    /// its own with this device alone is taken in (criterion 42).
    pub fn set_identity(&mut self, identity: &[u8]) {
        let (Some(builder), Ok(entropy)) = (self.builder.as_mut(), identity.try_into()) else {
            return;
        };
        builder.set_member(curvyo_crypto_core::IdentityKeys::from_entropy(&entropy).identity());
    }

    /// The documents shared on their own that were taken in: `[{path, document}]` (hex ids),
    /// each needs its `Moved` record once the workspace is shared.
    #[must_use]
    pub fn taken_in_json(&self) -> String {
        let rows: Vec<serde_json::Value> = self
            .taken_in
            .iter()
            .map(|(path, id)| serde_json::json!({"path": path, "document": hex(id)}))
            .collect();
        serde_json::Value::Array(rows).to_string()
    }

    /// Adds a scanned folder (`path` as a JSON array).
    pub fn add_folder(&mut self, path: &str) {
        if let Some(builder) = self.builder.as_mut() {
            builder.add_folder(&path_of(path));
        }
    }

    /// Adds a scanned document under the fresh id `document` (16 bytes): empty when added, else
    /// `shared-on-its-own`, `unreadable` or `name`.
    pub fn add_document(&mut self, path: &str, bytes: &[u8], document: &[u8]) -> String {
        let (Some(builder), Ok(document)) = (self.builder.as_mut(), document.try_into()) else {
            return "unreadable".to_owned();
        };
        let path = path_of(path);
        match builder.add_document(&path, bytes, document) {
            Ok(()) => String::new(),
            Err(reason) => {
                let name = path
                    .last()
                    .map_or("", |n| curvyo_document_core::document_name(n));
                self.not_added.push(name.to_owned());
                match reason {
                    NotAdded::SharedOnItsOwn => "shared-on-its-own",
                    NotAdded::Unreadable => "unreadable",
                    NotAdded::Name => "name",
                }
                .to_owned()
            }
        }
    }

    /// The documents added so far.
    #[must_use]
    pub fn document_count(&self) -> usize {
        self.builder
            .as_ref()
            .map_or(0, WorkspaceShareBuilder::document_count)
    }

    /// The names of the documents left out, as a JSON array.
    #[must_use]
    pub fn not_added_json(&self) -> String {
        serde_json::to_string(&self.not_added).unwrap_or_default()
    }

    /// Cuts the snapshot (`snapshot_id`: 16 random bytes); the share is then ready.
    pub fn prepare(&mut self, snapshot_id: &[u8]) {
        let id: [u8; 16] = snapshot_id.try_into().unwrap_or_default();
        if let Some(builder) = self.builder.take() {
            self.taken_in = builder.taken_in().to_vec();
            let (prepared, files) = builder.finish(id);
            self.prepared = Some(prepared);
            self.files = files;
        }
    }

    /// How many files the host writes after the share.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// The path of file `index` as a JSON array.
    #[must_use]
    pub fn file_path(&self, index: usize) -> String {
        self.files
            .get(index)
            .and_then(|(path, _)| serde_json::to_string(path).ok())
            .unwrap_or_default()
    }

    /// The bytes of file `index`.
    #[must_use]
    pub fn file_bytes(&self, index: usize) -> Vec<u8> {
        self.files
            .get(index)
            .map(|(_, bytes)| bytes.clone())
            .unwrap_or_default()
    }
}

/// A shared workspace's link between two document sessions.
#[wasm_bindgen]
pub struct WasmWorkspaceLink {
    link: WorkspaceLink,
}

/// One file of a join to write.
#[wasm_bindgen]
pub struct WasmWorkspaceWrite {
    /// `folder`, `document` or `deleted` (a deleted document's kept copy).
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// The path as a JSON array.
    #[wasm_bindgen(getter_with_clone)]
    pub path: String,
    /// The document id as hex (documents).
    #[wasm_bindgen(getter_with_clone)]
    pub document: String,
    /// The container bytes (documents).
    #[wasm_bindgen(getter_with_clone)]
    pub bytes: Vec<u8>,
}

/// One record for the background writer.
#[wasm_bindgen]
pub struct WasmBackgroundItem {
    /// The room sequence.
    pub seq: f64,
    /// The document id as hex.
    #[wasm_bindgen(getter_with_clone)]
    pub document: String,
    /// `update`, `asset` or `container`.
    #[wasm_bindgen(getter_with_clone)]
    pub kind: String,
    /// The payload.
    #[wasm_bindgen(getter_with_clone)]
    pub bytes: Vec<u8>,
}

/// The marker's files of a shared workspace.
#[wasm_bindgen(getter_with_clone)]
pub struct WasmMarkerFiles {
    /// `share.json`.
    pub share_json: Vec<u8>,
    /// `keyring.log`.
    pub keyring_log: Vec<u8>,
    /// `list.loro`.
    pub list: Vec<u8>,
}

#[wasm_bindgen]
impl WasmSession {
    /// Shares the workspace whose preparation is `share` (criterion 25). Empty on success,
    /// else the sentence of the failure.
    #[allow(clippy::too_many_arguments)] // The host's bytes cross as flat arguments (ADR 0001 §5).
    pub fn share_workspace(
        &mut self,
        server: &str,
        label: &str,
        name: &str,
        share: &mut WasmWorkspaceShare,
        identity: &[u8],
        nonce: &[u8],
        entropy: &[u8],
        now: f64,
    ) -> String {
        let (Some(seeds), Some(prepared)) =
            (seeds(identity, nonce, entropy), share.prepared.take())
        else {
            return "Could not share.".to_owned();
        };
        match self
            .session
            .share_workspace(server, label, name, prepared, &seeds, millis(now))
        {
            Ok(()) => String::new(),
            Err(_) => "Could not share.".to_owned(),
        }
    }

    /// Joins the workspace of a `2.` link (criterion 27). Empty when the join started, else
    /// the refusal.
    #[allow(clippy::too_many_arguments)] // The host's bytes cross as flat arguments (ADR 0001 §5).
    pub fn join_workspace(
        &mut self,
        link: &str,
        label: &str,
        name: &str,
        identity: &[u8],
        nonce: &[u8],
        entropy: &[u8],
        now: f64,
    ) -> String {
        let Some(seeds) = seeds(identity, nonce, entropy) else {
            return share_text::join_refusal_text(JoinRefusal::Invalid, "");
        };
        match self
            .session
            .join_workspace(link, label, name, &seeds, millis(now))
        {
            Ok(()) => String::new(),
            Err(refusal) => share_text::join_refusal_text(refusal, ""),
        }
    }

    /// Reconnects the shared workspace from its marker's files (criterion 33); `keys_json` is
    /// `[[epoch, "hex"], ...]` from the credential store.
    #[allow(clippy::too_many_arguments)] // The host's bytes cross as flat arguments (ADR 0001 §5).
    pub fn resume_workspace(
        &mut self,
        share_json: &[u8],
        keyring_log: &[u8],
        list: &[u8],
        name: &str,
        label: &str,
        keys_json: &str,
        identity: &[u8],
        nonce: &[u8],
        entropy: &[u8],
        now: f64,
    ) -> bool {
        let Some(seeds) = seeds(identity, nonce, entropy) else {
            return false;
        };
        let keys: Vec<(u32, String)> = serde_json::from_str(keys_json).unwrap_or_default();
        let keys = keys
            .into_iter()
            .filter_map(|(n, hex)| Some((n, unhex::<32>(&hex)?)))
            .collect();
        self.session.resume_workspace(
            share_json,
            keyring_log,
            list,
            name,
            label,
            keys,
            &seeds,
            millis(now),
        )
    }

    /// Whether this session carries the shared workspace's link.
    #[must_use]
    pub fn has_workspace_link(&self) -> bool {
        self.session.has_workspace_link()
    }

    /// Takes the workspace's link out (before this session is replaced).
    pub fn take_workspace_link(&mut self) -> Option<WasmWorkspaceLink> {
        self.session
            .take_workspace_link()
            .map(|link| WasmWorkspaceLink { link })
    }

    /// Gives this session the workspace's link.
    pub fn attach_workspace_link(&mut self, link: WasmWorkspaceLink) {
        self.session.attach_workspace_link(link.link);
    }

    /// The next file of a join to write, `undefined` when there is none.
    pub fn next_workspace_write(&mut self) -> Option<WasmWorkspaceWrite> {
        let write = self.session.next_workspace_write()?;
        Some(match write {
            WorkspaceWrite::Folder { path } => WasmWorkspaceWrite {
                kind: "folder".to_owned(),
                path: serde_json::to_string(&path).unwrap_or_default(),
                document: String::new(),
                bytes: Vec::new(),
            },
            WorkspaceWrite::Document {
                path,
                document,
                bytes,
                ..
            } => WasmWorkspaceWrite {
                kind: "document".to_owned(),
                path: serde_json::to_string(&path).unwrap_or_default(),
                document: hex(&document),
                bytes,
            },
            WorkspaceWrite::Deleted { document, bytes } => WasmWorkspaceWrite {
                kind: "deleted".to_owned(),
                path: "[]".to_owned(),
                document: hex(&document),
                bytes,
            },
        })
    }

    /// The host wrote the joined document `document` (hex).
    pub fn workspace_document_written(&mut self, document: &str) {
        if let Some(id) = unhex::<16>(document) {
            self.session.workspace_document_written(id);
        }
    }

    /// The next record for the background writer, `undefined` when there is none.
    pub fn next_background_item(&mut self) -> Option<WasmBackgroundItem> {
        let item = self.session.take_background_one()?;
        #[allow(clippy::cast_precision_loss)] // Sequences stay far below 2^53.
        Some(WasmBackgroundItem {
            seq: item.seq as f64,
            document: hex(&item.document),
            kind: match item.kind {
                BackgroundKind::Update => "update",
                BackgroundKind::Asset => "asset",
                BackgroundKind::Container => "container",
            }
            .to_owned(),
            bytes: item.bytes,
        })
    }

    /// Whether the background writer may write `document`'s (hex) file: never the open one.
    #[must_use]
    pub fn workspace_may_write(&self, document: &str) -> bool {
        unhex::<16>(document).is_some_and(|id| self.session.workspace_may_write(id))
    }

    /// The background writer wrote `document`'s (hex) file with every record up to `through`.
    pub fn background_written(&mut self, document: &str, through: f64) {
        if let Some(id) = unhex::<16>(document) {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            self.session.background_written(id, through.max(0.0) as u64);
        }
    }

    /// The marker's files when they changed, `undefined` otherwise.
    pub fn take_workspace_marker(&mut self) -> Option<WasmMarkerFiles> {
        let files = self.session.take_workspace_marker()?;
        Some(WasmMarkerFiles {
            share_json: files.share_json,
            keyring_log: files.keyring_log,
            list: files.list,
        })
    }
}

/// What an invite link opens: `document`, `workspace`, or empty for a text that is no link.
#[wasm_bindgen]
#[must_use]
pub fn invite_link_kind(text: &str) -> String {
    match InviteLink::parse(text).map(|link| link.kind) {
        Ok(InviteKind::Document) => "document",
        Ok(InviteKind::Workspace) => "workspace",
        Err(_) => "",
    }
    .to_owned()
}

/// The workspace id of a `2.` link as hex, empty for anything else.
#[wasm_bindgen]
#[must_use]
pub fn invite_link_workspace(text: &str) -> String {
    match InviteLink::parse(text) {
        Ok(link) if link.kind == InviteKind::Workspace => hex(&link.secret.room.0),
        _ => String::new(),
    }
}

/// The server address of an invite link, empty for anything else.
#[wasm_bindgen]
#[must_use]
pub fn invite_link_server(text: &str) -> String {
    InviteLink::parse(text).map_or_else(|_| String::new(), |link| link.server)
}

/// The inline confirm of Share workspace; `taken_in_json` names the documents shared on their
/// own that are taken in, `not_added_json` the ones left out (JSON arrays of names).
#[wasm_bindgen]
#[must_use]
pub fn workspace_share_confirm_text(
    name: &str,
    documents: usize,
    server: &str,
    taken_in_json: &str,
    not_added_json: &str,
) -> String {
    let taken_in: Vec<String> = serde_json::from_str(taken_in_json).unwrap_or_default();
    let not_added: Vec<String> = serde_json::from_str(not_added_json).unwrap_or_default();
    texts::workspace_share_confirm_text(name, documents, server, &taken_in, &not_added)
}

/// The sentence of criterion 26.
#[wasm_bindgen]
#[must_use]
pub fn workspace_invite_sentence() -> String {
    texts::WORKSPACE_INVITE_SENTENCE.to_owned()
}

/// "Not added: Lid, Base." for the names in `names_json`.
#[wasm_bindgen]
#[must_use]
pub fn workspace_not_added_text(names_json: &str) -> String {
    let names: Vec<String> = serde_json::from_str(names_json).unwrap_or_default();
    texts::not_added_text(&names)
}

/// "Uploading 3 of 14".
#[wasm_bindgen]
#[must_use]
pub fn workspace_uploading_text(done: usize, total: usize) -> String {
    texts::uploading_text(done, total)
}

/// The refusal of a folder chosen for a joined workspace, by the host's code.
#[wasm_bindgen]
#[must_use]
pub fn join_folder_refusal_text(code: &str) -> String {
    texts::join_folder_refusal_text(code).to_owned()
}
