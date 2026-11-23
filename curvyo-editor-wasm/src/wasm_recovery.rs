//! The recovery copy's `wasm-bindgen` surface (`specs/0136-autosave-and-restore`, ADR 0079): the
//! session's hand-over calls, the worker's replica, the schedule of `curvyo-ui-core`, the record
//! codec, the container check and assemble, the prune plan and the words of the File tab block.
//! Pass-throughs; the rules are Rust's, the page keeps the clock and the storage.

use curvyo_document_core::{assemble_recovery_copy, check_recovery_copy};
use curvyo_ui_core as ui;
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

use crate::RecoveryWriter;
use crate::wasm_api::WasmSession;

fn whole(value: f64) -> u64 {
    // Times from the page are whole numbers well inside u64.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let whole = value.max(0.0) as u64;
    whole
}

/// What the replica is handed.
#[wasm_bindgen]
pub struct WasmRecoveryDelta {
    /// A snapshot that seeds the replica, not updates.
    pub full: bool,
    /// The bytes.
    #[wasm_bindgen(getter_with_clone)]
    pub bytes: Vec<u8>,
}

#[wasm_bindgen]
impl WasmSession {
    /// Whether a step was committed since the last call.
    pub fn recovery_changed(&mut self) -> bool {
        self.session.recovery_changed()
    }

    /// What the worker's replica is handed next.
    ///
    /// # Errors
    /// A plain string when the bytes cannot be encoded.
    pub fn recovery_export(&mut self) -> Result<WasmRecoveryDelta, JsValue> {
        let delta = self
            .session
            .recovery_export()
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;
        Ok(WasmRecoveryDelta {
            full: delta.full,
            bytes: delta.bytes,
        })
    }

    /// The replica is gone or went its own way: the next export is a snapshot.
    pub fn recovery_reseed(&mut self) {
        self.session.recovery_reseed();
    }

    /// A new session key: nothing has been handed yet.
    pub fn recovery_start_over(&mut self) {
        self.session.recovery_start_over();
    }

    /// The names (JSON array) of the assets the host has not been handed.
    #[must_use]
    pub fn recovery_asset_names(&self) -> String {
        let names: Vec<String> = self
            .session
            .recovery_new_assets()
            .into_iter()
            .map(|a| a.name)
            .collect();
        json!(names).to_string()
    }

    /// One pending asset's bytes; empty for a name that is not pending.
    #[must_use]
    pub fn recovery_asset(&self, name: &str) -> Vec<u8> {
        self.session
            .recovery_new_assets()
            .into_iter()
            .find(|a| a.name == name)
            .map_or_else(Vec::new, |a| a.bytes.to_vec())
    }

    /// The host has these assets (JSON array of names).
    pub fn recovery_assets_done(&mut self, names: &str) {
        let names: Vec<String> = serde_json::from_str(names).unwrap_or_default();
        self.session.recovery_assets_done(names);
    }
}

/// A packed copy and its page count.
#[wasm_bindgen]
pub struct WasmPackedCopy {
    /// The container without assets.
    #[wasm_bindgen(getter_with_clone)]
    pub bytes: Vec<u8>,
    /// The page count.
    pub pages: u32,
}

/// The recovery worker's replica.
#[wasm_bindgen]
pub struct WasmRecoveryWriter {
    peer: u64,
    writer: RecoveryWriter,
}

#[wasm_bindgen]
impl WasmRecoveryWriter {
    /// A writer whose replica uses the Loro peer `peer`.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(peer: u64) -> Self {
        Self {
            peer,
            writer: RecoveryWriter::new(),
        }
    }

    /// Takes a delta: empty on success, `reseed` when the page has to send a snapshot,
    /// `rejected` for bytes it cannot read.
    pub fn apply(&mut self, full: bool, bytes: &[u8]) -> String {
        match self.writer.apply(self.peer, full, bytes) {
            Ok(()) => String::new(),
            Err(crate::RecoveryWriterError::Reseed) => "reseed".to_owned(),
            Err(_) => "rejected".to_owned(),
        }
    }

    /// Forgets the replica (a new session key).
    pub fn reset(&mut self) {
        self.writer.reset();
    }

    /// The replica packed; `undefined` while it holds nothing or cannot be packed.
    pub fn pack(&mut self) -> Option<WasmPackedCopy> {
        let packed = self.writer.pack().ok().flatten()?;
        Some(WasmPackedCopy {
            bytes: packed.bytes,
            pages: packed.pages,
        })
    }
}

/// The schedule of one session's recovery copy (`curvyo-ui-core`'s rule, criterion 1).
#[wasm_bindgen]
pub struct WasmRecoverySchedule {
    schedule: ui::RecoverySchedule,
}

#[wasm_bindgen]
impl WasmRecoverySchedule {
    /// A schedule with no copy.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            schedule: ui::RecoverySchedule::new(),
        }
    }

    /// A step was committed.
    pub fn step(&mut self) {
        self.schedule.step();
    }

    /// `nothing`, `write` or `delete` at `now` (ms, monotonic) for a `dirty` project.
    pub fn due(&self, now: f64, dirty: bool) -> String {
        match self.schedule.due(whole(now), dirty) {
            ui::Due::Nothing => "nothing",
            ui::Due::Write => "write",
            ui::Due::DeleteCopy => "delete",
        }
        .to_owned()
    }

    /// A write starts.
    pub fn begin(&mut self) {
        self.schedule.begin();
    }

    /// The write succeeded.
    pub fn written(&mut self, now: f64) {
        self.schedule.written(whole(now));
    }

    /// The write failed; whether the canvas notice shows.
    pub fn failed(&mut self, now: f64) -> bool {
        self.schedule.failed(whole(now))
    }

    /// The copy was deleted because the project is clean.
    pub fn deleted(&mut self) {
        self.schedule.deleted();
    }

    /// Save, Discard or Close ended the copy.
    pub fn ended(&mut self) {
        self.schedule.ended();
    }

    /// Whether the File tab shows the failed-write line.
    #[must_use]
    pub fn failure_line(&self) -> bool {
        self.schedule.failure_line()
    }
}

impl Default for WasmRecoverySchedule {
    fn default() -> Self {
        Self::new()
    }
}

/// The text of a record: `base` is `{path, modified_ms, length}` JSON or empty.
#[wasm_bindgen]
#[must_use]
pub fn recovery_record_text(
    key: &str,
    name: Option<String>,
    base: &str,
    last_step_ms: f64,
    pages: u32,
) -> String {
    let record = ui::RecoveryRecord {
        format_version: ui::RECOVERY_FORMAT,
        key: key.to_owned(),
        name,
        base: serde_json::from_str(base).ok(),
        last_step_ms: whole(last_step_ms),
        pages,
    };
    String::from_utf8(ui::encode_record(&record)).unwrap_or_default()
}

/// What a stored copy says: `{name, pages, last_step_ms, problem}` from its record text and
/// container bytes (`problem` is `damaged`, `newer` or null), the browser's listing of criterion 7.
#[wasm_bindgen]
#[must_use]
pub fn recovery_check_entry(record: &str, copy: &[u8]) -> String {
    let decoded = ui::decode_record(record.as_bytes());
    let problem = match (&decoded, check_recovery_copy(copy)) {
        (Err(ui::RecordError::Newer), _)
        | (_, Err(curvyo_document_core::OpenError::FormatTooNew { .. })) => Some("newer"),
        (Err(_), _) | (_, Err(_)) => Some("damaged"),
        (Ok(_), Ok(())) => None,
    };
    let record = decoded.ok();
    json!({
        "name": record.as_ref().and_then(|r| r.name.clone()),
        "pages": record.as_ref().map_or(0, |r| r.pages),
        "last_step_ms": record.as_ref().map(|r| r.last_step_ms),
        "problem": problem,
    })
    .to_string()
}

/// The keys to delete (JSON array, oldest first) from `[{key, bytes, last_step_ms, locked}]`.
#[wasm_bindgen]
#[must_use]
pub fn recovery_prune_plan(candidates: &str) -> String {
    let list: Vec<Value> = serde_json::from_str(candidates).unwrap_or_default();
    let copies: Vec<ui::PruneCandidate> = list
        .iter()
        .filter_map(|v| {
            Some(ui::PruneCandidate {
                key: v["key"].as_str()?.to_owned(),
                bytes: v["bytes"].as_u64()?,
                last_step_ms: v["last_step_ms"].as_u64().unwrap_or(0),
                locked: v["locked"].as_bool().unwrap_or(false),
            })
        })
        .collect();
    json!(ui::prune_plan(&copies)).to_string()
}

/// A normal `.curvyo` from a recovery copy and its assets: the page `add`s each asset.
#[wasm_bindgen]
pub struct WasmAssembler {
    copy: Vec<u8>,
    assets: Vec<(String, Vec<u8>)>,
}

#[wasm_bindgen]
impl WasmAssembler {
    /// Starts from the copy's bytes.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(copy: Vec<u8>) -> Self {
        Self {
            copy,
            assets: Vec::new(),
        }
    }

    /// One asset by its name below `assets/`.
    pub fn add(&mut self, name: String, bytes: Vec<u8>) {
        self.assets.push((name, bytes));
    }

    /// The assembled bytes; `undefined` when the copy is no container.
    pub fn finish(&mut self) -> Option<Vec<u8>> {
        let assets: Vec<(&str, &[u8])> = self
            .assets
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        assemble_recovery_copy(&self.copy, &assets).ok()
    }
}

fn local_time(value: &Value) -> ui::LocalTime {
    let part = |key: &str| value[key].as_i64().unwrap_or(0);
    ui::LocalTime {
        year: i32::try_from(part("year")).unwrap_or(0),
        month: u32::try_from(part("month")).unwrap_or(1),
        day: u32::try_from(part("day")).unwrap_or(1),
        hour: u32::try_from(part("hour")).unwrap_or(0),
        minute: u32::try_from(part("minute")).unwrap_or(0),
    }
}

/// The words of one row. `row` is `{name, pages, changed, problem}`, `at` and `now` are local
/// `{year, month, day, hour, minute}`.
#[wasm_bindgen]
#[must_use]
pub fn recovery_row_view(row: &str, at: &str, now: &str) -> String {
    let row: Value = serde_json::from_str(row).unwrap_or_default();
    let at: Value = serde_json::from_str(at).unwrap_or_default();
    let now: Value = serde_json::from_str(now).unwrap_or_default();
    let (at, now) = (local_time(&at), local_time(&now));
    let name = row["name"].as_str();
    let pages = u32::try_from(row["pages"].as_u64().unwrap_or(0)).unwrap_or(0);
    let changed = row["changed"].as_bool().unwrap_or(false);
    let problem = match row["problem"].as_str() {
        Some("damaged") => Some(ui::Unrestorable::Damaged),
        Some("newer") => Some(ui::Unrestorable::Newer),
        _ => None,
    };
    let time = ui::time_text(at, now);
    let subject = ui::row_subject(name, &time);
    json!({
        "name": ui::row_name(name),
        "line2": ui::row_line_two(at, now, pages, problem),
        "line3": (changed && problem.is_none()).then_some(ui::FILE_CHANGED_LINE),
        "label": ui::row_label(name, &time, pages, changed, problem),
        "restore_label": ui::restore_label(&subject),
        "discard_label": ui::discard_label(&subject),
        "discard_armed_label": ui::discard_armed_label(&subject),
        "subject": subject,
        "recovered_name": ui::recovered_name(name),
    })
    .to_string()
}

/// The block's heading.
#[wasm_bindgen]
#[must_use]
pub fn recovery_heading() -> String {
    ui::RECOVERY_BLOCK_HEADING.to_owned()
}

/// The help line: "Kept in `<path>`" or, with no path, "Kept in this browser".
#[wasm_bindgen]
#[must_use]
#[allow(clippy::needless_pass_by_value)] // wasm-bindgen takes the page's option by value
pub fn recovery_help_line(path: Option<String>) -> String {
    ui::help_line(path.as_deref())
}

/// The failed-write sentence for a code the page got: `disk-full`, `no-permission`,
/// `browser-full`, or the system's reason.
#[wasm_bindgen]
#[must_use]
pub fn recovery_failure_sentence(code: &str) -> String {
    let failure = match code {
        "disk-full" => ui::WriteFailure::DiskFull,
        "no-permission" => ui::WriteFailure::NoPermission,
        "browser-full" => ui::WriteFailure::BrowserFull,
        other => ui::WriteFailure::Other(other.to_owned()),
    };
    ui::failure_sentence(&failure)
}

/// The name a restored project gets (`Poster (recovered)` or `Untitled`).
#[wasm_bindgen]
#[must_use]
#[allow(clippy::needless_pass_by_value)] // wasm-bindgen takes the page's option by value
pub fn recovery_recovered_name(name: Option<String>) -> String {
    ui::recovered_name(name.as_deref())
}
