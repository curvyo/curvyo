//! The background writer's `wasm-bindgen` surface (`specs/0094-workspace` criteria 31 and 32):
//! the worker adds records, asks which documents are due, and writes a document's file bytes.
//! Also the session's side of a workspace snapshot the host builds from the files (ADR 0036
//! decision 8). Pass-throughs to `background_writer` and `session::workspace_sync`.

use wasm_bindgen::prelude::*;

use crate::session::{BackgroundKind, DocumentPlace, hex, unhex};

/// A document's place as the page reads it.
fn place_json(id: &[u8; 16], place: &DocumentPlace) -> serde_json::Value {
    match place {
        DocumentPlace::File(path) => serde_json::json!({"document": hex(id), "path": path}),
        DocumentPlace::Deleted => serde_json::json!({"document": hex(id), "deleted": true}),
    }
}
use crate::wasm_api::WasmSession;
use crate::{BackgroundWriter, Waiting};

fn whole(value: f64) -> u64 {
    // Times and sequences from the page are whole numbers well inside u64.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let whole = value.max(0.0) as u64;
    whole
}

/// The worker's writer of documents that are not open.
#[wasm_bindgen]
pub struct WasmBackgroundWriter {
    writer: BackgroundWriter,
}

/// A document's file written with its waiting records.
#[wasm_bindgen]
pub struct WasmWrittenFile {
    /// The new file bytes.
    #[wasm_bindgen(getter_with_clone)]
    pub bytes: Vec<u8>,
    /// The highest sequence they hold.
    pub through: f64,
}

#[wasm_bindgen]
impl WasmBackgroundWriter {
    /// A writer whose unpacked documents use the Loro peer `peer`.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(peer: u64) -> Self {
        Self {
            writer: BackgroundWriter::new(peer),
        }
    }

    /// A record of `document` (hex): `kind` is `update`, `asset` or `container`.
    pub fn add(&mut self, document: &str, seq: f64, kind: &str, bytes: Vec<u8>) {
        let kind = match kind {
            "update" => BackgroundKind::Update,
            "asset" => BackgroundKind::Asset,
            "container" => BackgroundKind::Container,
            _ => return,
        };
        if let Some(id) = unhex::<16>(document) {
            self.writer.add(
                id,
                Waiting {
                    seq: whole(seq),
                    kind,
                    bytes,
                },
            );
        }
    }

    /// The documents (hex, JSON array) whose file the worker asks for now (`now` in ms).
    pub fn due(&mut self, now: f64) -> String {
        let due: Vec<String> = self
            .writer
            .due(whole(now))
            .iter()
            .map(|id| hex(id))
            .collect();
        serde_json::to_string(&due).unwrap_or_default()
    }

    /// Writes the waiting records of `document` (hex) into its file `bytes`; `undefined` when
    /// the file does not open (the records keep waiting).
    pub fn write(&mut self, document: &str, bytes: &[u8], now: f64) -> Option<WasmWrittenFile> {
        let id = unhex::<16>(document)?;
        let (bytes, through) = self.writer.write(id, bytes, whole(now)).ok()?;
        #[allow(clippy::cast_precision_loss)] // Sequences stay far below 2^53.
        Some(WasmWrittenFile {
            bytes,
            through: through as f64,
        })
    }

    /// Builds the first file of `document` (hex), a document of `workspace` (hex) with no file
    /// here yet, from its records; `undefined` while they do not start a document.
    pub fn create(&mut self, document: &str, workspace: &str, now: f64) -> Option<WasmWrittenFile> {
        let id = unhex::<16>(document)?;
        let workspace = unhex::<16>(workspace)?;
        let (bytes, through) = self.writer.create(id, workspace, whole(now)).ok()?;
        #[allow(clippy::cast_precision_loss)] // Sequences stay far below 2^53.
        Some(WasmWrittenFile {
            bytes,
            through: through as f64,
        })
    }

    /// The file of `document` (hex) is not there yet: asked for again after the interval.
    pub fn postpone(&mut self, document: &str, now: f64) {
        if let Some(id) = unhex::<16>(document) {
            self.writer.postpone(id, whole(now));
        }
    }

    /// Drops what waits for `document` (hex); the highest sequence dropped, `-1` for none.
    pub fn forget(&mut self, document: &str) -> f64 {
        #[allow(clippy::cast_precision_loss)] // Sequences stay far below 2^53.
        unhex::<16>(document)
            .and_then(|id| self.writer.forget(id))
            .map_or(-1.0, |seq| seq as f64)
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// The sequence a due workspace snapshot covers once the files hold every record up to it;
    /// `-1` otherwise.
    #[must_use]
    pub fn workspace_snapshot_ready(&self) -> f64 {
        #[allow(clippy::cast_precision_loss)] // Sequences stay far below 2^53.
        self.session
            .workspace_snapshot_ready()
            .map_or(-1.0, |seq| seq as f64)
    }

    /// The documents whose files go into the snapshot: `[{"document", "path"}]`, or
    /// `{"document", "deleted": true}` for a deleted document's kept copy.
    #[must_use]
    pub fn workspace_snapshot_documents_json(&self) -> String {
        let documents: Vec<serde_json::Value> = self
            .session
            .workspace_snapshot_documents()
            .into_iter()
            .map(|(id, place)| place_json(&id, &place))
            .collect();
        serde_json::Value::Array(documents).to_string()
    }

    /// One document's (hex) file bytes for the snapshot being built.
    pub fn add_workspace_snapshot_document(&mut self, document: &str, bytes: Vec<u8>) {
        if let Some(id) = unhex::<16>(document) {
            self.session.add_workspace_snapshot_document(id, bytes);
        }
    }

    /// Uploads the due snapshot with the documents added; `snapshot_id` is 16 random bytes.
    pub fn finish_workspace_snapshot(&mut self, snapshot_id: &[u8]) -> bool {
        let Ok(id) = snapshot_id.try_into() else {
            return false;
        };
        self.session.finish_workspace_snapshot(id).is_ok()
    }

    /// Where the workspace document `document` (hex) keeps its file: `{"document", "path"}`
    /// or `{"document", "deleted": true}`; empty when the list does not know it yet.
    #[must_use]
    pub fn workspace_document_place(&self, document: &str) -> String {
        unhex::<16>(document)
            .and_then(|id| {
                let place = self.session.workspace_document_place(id)?;
                Some(place_json(&id, &place).to_string())
            })
            .unwrap_or_default()
    }
}
