//! The font entries of the device group's library stream as the page calls them
//! (`specs/0116-library-sync` criteria 16 to 19): strings, scalars and byte arrays only (ADR
//! 0001 §5). The bytes of a font cross once per switch-on, the hash is the glue's own.

use curvyo_document_core::{AssetHash, CuttableChoice};
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

use crate::wasm_devices::WasmDeviceGroup;
use crate::{FontChange, FontInput};

fn cuttable(word: &str) -> CuttableChoice {
    match word {
        "yes" => CuttableChoice::Yes,
        "no" => CuttableChoice::No,
        _ => CuttableChoice::Auto,
    }
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn word(cuttable: CuttableChoice) -> &'static str {
    match cuttable {
        CuttableChoice::Auto => "auto",
        CuttableChoice::Yes => "yes",
        CuttableChoice::No => "no",
    }
}

#[wasm_bindgen]
impl WasmDeviceGroup {
    /// Hands over a licence or readme file of the font the next `library_font_set` switches on.
    pub fn library_font_stage_document(&mut self, name: &str, bytes: Vec<u8>) {
        self.glue.font_stage_document(name, bytes);
    }

    /// Switches a font on with its file bytes: `input` is JSON `{"file_name", "folder"?,
    /// "tags": [..], "favourite", "cuttable": "auto"|"yes"|"no"}`. Returns `{"hash"}` or, for a
    /// refusal (criterion 19), `{"notice"}` with its text; a refused font writes nothing.
    pub fn library_font_set(&mut self, input: &str, bytes: Vec<u8>) -> String {
        let Ok(value) = serde_json::from_str::<Value>(input) else {
            return json!({ "notice": "" }).to_string();
        };
        let input = FontInput {
            file_name: value["file_name"].as_str().unwrap_or_default().to_owned(),
            folder: value["folder"].as_str().map(str::to_owned),
            tags: strings(&value["tags"]),
            favourite: value["favourite"].as_bool().unwrap_or(false),
            cuttable: cuttable(value["cuttable"].as_str().unwrap_or("auto")),
            documents: Vec::new(),
        };
        match self.glue.font_set(input, bytes) {
            Ok(hash) => json!({ "hash": hash.as_str() }),
            Err(notice) => json!({ "notice": notice }),
        }
        .to_string()
    }

    /// The metadata of the switched-on font `hash` changed here (JSON of `FontLibrary::sync_meta`).
    pub fn library_font_meta(&mut self, hash: &str, meta: &str) {
        let (Some(hash), Ok(value)) = (AssetHash::parse(hash), serde_json::from_str::<Value>(meta))
        else {
            return;
        };
        self.glue.font_meta(
            &hash,
            strings(&value["tags"]),
            value["favourite"].as_bool().unwrap_or(false),
            cuttable(value["cuttable"].as_str().unwrap_or("auto")),
        );
    }

    /// Switches the font `hash` off: its marker stays, its bytes leave the next snapshot.
    pub fn library_font_switch_off(&mut self, hash: &str) {
        if let Some(hash) = AssetHash::parse(hash) {
            self.glue.font_switch_off(&hash);
        }
    }

    /// The font `hash` was deleted here: its entry goes if it was on.
    pub fn library_font_remove(&mut self, hash: &str) {
        if let Some(hash) = AssetHash::parse(hash) {
            self.glue.font_remove(&hash);
        }
    }

    /// The hashes of the font files this host holds (JSON array of 64 hex digit strings).
    pub fn library_font_host_hashes(&mut self, hashes: &str) {
        let hashes: Vec<String> = serde_json::from_str(hashes).unwrap_or_default();
        self.glue
            .font_host_hashes(hashes.iter().filter_map(|h| AssetHash::parse(h)).collect());
    }

    /// The hashes of switched-on fonts whose bytes this host holds but the group does not (JSON
    /// array): the page reads those files and supplies them with `library_font_supply`.
    #[must_use]
    pub fn library_font_wants(&self) -> String {
        let wants = self.glue.font_wants();
        let wants: Vec<&str> = wants.iter().map(AssetHash::as_str).collect();
        serde_json::to_string(&wants).unwrap_or_default()
    }

    /// The bytes of the font file or licence file `hash`, as asked for by `library_font_wants`.
    pub fn library_font_supply(&mut self, hash: &str, bytes: Vec<u8>) {
        if let Some(hash) = AssetHash::parse(hash) {
            self.glue.font_supply_blob(&hash, bytes);
        }
    }

    /// The held bytes of `hash` (a font or licence file that arrived), empty when none.
    #[must_use]
    pub fn library_font_blob(&self, hash: &str) -> Vec<u8> {
        AssetHash::parse(hash)
            .map(|hash| self.glue.font_blob(&hash).to_vec())
            .unwrap_or_default()
    }

    /// The font changes for the page's fonts folder, once, as JSON `[{"hash", "kind":
    /// "add"|"meta"|"removed", "file_name", "folder", "tags", "favourite", "cuttable",
    /// "documents": [{"hash", "name"}]}]`; an "add" carries bytes the page reads with
    /// `library_font_blob`.
    pub fn library_font_arrivals(&mut self) -> String {
        let rows: Vec<Value> = self
            .glue
            .take_font_arrivals()
            .into_iter()
            .map(|arrival| {
                let (kind, details) = match arrival.change {
                    FontChange::Add(details) => ("add", Some(details)),
                    FontChange::Meta(details) => ("meta", Some(details)),
                    FontChange::Removed => ("removed", None),
                };
                json!({
                    "hash": arrival.hash.as_str(),
                    "kind": kind,
                    "file_name": details.as_ref().map(|d| d.file_name.clone()),
                    "folder": details.as_ref().and_then(|d| d.folder.clone()),
                    "tags": details.as_ref().map(|d| d.tags.clone()).unwrap_or_default(),
                    "favourite": details.as_ref().is_some_and(|d| d.favourite),
                    "cuttable": details.as_ref().map_or("auto", |d| word(d.cuttable)),
                    "documents": details.as_ref().map(|d| d.documents.iter()
                        .map(|doc| json!({"hash": doc.hash.as_str(), "name": doc.name}))
                        .collect::<Vec<_>>()).unwrap_or_default(),
                })
            })
            .collect();
        serde_json::to_string(&rows).unwrap_or_default()
    }
}
