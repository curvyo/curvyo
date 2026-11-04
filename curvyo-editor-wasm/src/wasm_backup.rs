//! The passkey key backup's `wasm-bindgen` surface (ADR 0040 decision 13, Frage 145 A): one
//! exchange with the relay over its own socket, the PRF input, and the backup's words. The page
//! drives `WebAuthn` and the socket; sealing, signing and the frames are Rust's.

use curvyo_crypto_core::{BackupContent, DocumentId, IdentityKeys, backup_prf_input};
use curvyo_sync_core::{BackupExchange, BackupOutcome, RefusalReason};
use curvyo_ui_core::key_backup_text as texts;
use wasm_bindgen::prelude::*;

use crate::session::{hex, unhex};

/// The 32 bytes every browser asks its passkey's PRF to evaluate.
#[wasm_bindgen]
#[must_use]
pub fn key_backup_prf_input() -> Vec<u8> {
    backup_prf_input().to_vec()
}

/// One read, write or delete of the passkey's backup record.
#[wasm_bindgen]
pub struct WasmKeyBackup {
    exchange: BackupExchange,
}

#[wasm_bindgen]
impl WasmKeyBackup {
    /// Reads the record of the passkey whose PRF output is `prf` (32 bytes).
    #[must_use]
    pub fn read(prf: &[u8]) -> Option<WasmKeyBackup> {
        Some(Self {
            exchange: BackupExchange::read(prf.try_into().ok()?),
        })
    }

    /// Backs up `identity` (64 bytes) and the group (`room` hex on `server`, this device named
    /// `label`), sealed with the 24 fresh bytes `nonce`. `undefined` for a bad argument.
    #[must_use]
    pub fn write(
        prf: &[u8],
        identity: &[u8],
        room: &str,
        server: &str,
        label: &str,
        nonce: &[u8],
    ) -> Option<WasmKeyBackup> {
        let identity: [u8; 64] = identity.try_into().ok()?;
        let content = BackupContent {
            identity: IdentityKeys::from_entropy(&identity).to_entropy(),
            group_room: DocumentId(unhex(room)?),
            server: server.to_owned(),
            label: label.to_owned(),
        };
        BackupExchange::write(prf.try_into().ok()?, &content, nonce.try_into().ok()?)
            .ok()
            .map(|exchange| Self { exchange })
    }

    /// Deletes the passkey's record.
    #[must_use]
    pub fn delete(prf: &[u8]) -> Option<WasmKeyBackup> {
        Some(Self {
            exchange: BackupExchange::delete(prf.try_into().ok()?),
        })
    }

    /// The frame to send once the socket is open.
    #[must_use]
    pub fn first_frame(&self) -> Vec<u8> {
        self.exchange.first_frame()
    }

    /// One received message; the answer to send, if any.
    pub fn receive(&mut self, bytes: &[u8]) -> Option<Vec<u8>> {
        self.exchange.receive(bytes)
    }

    /// The socket closed or did not open.
    pub fn closed(&mut self) {
        self.exchange.closed();
    }

    /// Whether the exchange ended.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.exchange.finished()
    }

    /// The outcome as JSON, taken once: `{"kind":"read","identity":hex,"room":hex,"server",
    /// "label"}`, `{"kind":"done"}`, or a failure kind with its sentence (`refused-unknown`,
    /// `refused-forbidden`, `damaged`, `failed`, ...); `""` while running.
    pub fn take_outcome(&mut self) -> String {
        let Some(outcome) = self.exchange.take_outcome() else {
            return String::new();
        };
        let value = match outcome {
            BackupOutcome::Read(content) => serde_json::json!({
                "kind": "read",
                "identity": hex(content.identity.as_slice()),
                "room": hex(&content.group_room.0),
                "server": content.server,
                "label": content.label,
            }),
            BackupOutcome::Done => serde_json::json!({ "kind": "done" }),
            other => {
                let kind = failure_kind(&other);
                serde_json::json!({ "kind": kind, "text": texts::outcome_text(&kind) })
            }
        };
        value.to_string()
    }
}

fn failure_kind(outcome: &BackupOutcome) -> String {
    match outcome {
        BackupOutcome::Refused(reason) => format!(
            "refused-{}",
            match reason {
                RefusalReason::Unknown => "unknown",
                RefusalReason::Forbidden => "forbidden",
                RefusalReason::TooLarge => "too-large",
                _ => "protocol",
            }
        ),
        BackupOutcome::Damaged => "damaged".to_owned(),
        BackupOutcome::Read(_) | BackupOutcome::Done | BackupOutcome::Failed => "failed".to_owned(),
    }
}

/// A fixed word of the backup by key: `backed-up`, `back-up`, `remove`, `restore`, `waiting`,
/// `working`, `stored`, `removed`, `restored`, `remove-confirm`, or an
/// outcome kind (`no-prf`, `cancelled`, ...); empty for another key.
#[wasm_bindgen]
#[must_use]
pub fn key_backup_text(key: &str) -> String {
    match key {
        "backed-up" => texts::BACKED_UP_LINE,
        "back-up" => texts::BACK_UP_LABEL,
        "remove" => texts::REMOVE_LABEL,
        "restore" => texts::RESTORE_LABEL,
        "waiting" => texts::WAITING,
        "working" => texts::WORKING,
        "stored" => texts::STORED_NOTICE,
        "removed" => texts::REMOVED_NOTICE,
        "restored" => texts::RESTORED_NOTICE,
        "remove-confirm" => texts::REMOVE_CONFIRM,
        other => texts::outcome_text(other),
    }
    .to_owned()
}

/// The consequence sentence before a backup, naming `server`.
#[wasm_bindgen]
#[must_use]
pub fn key_backup_sentence(server: &str) -> String {
    texts::back_up_sentence(server)
}
