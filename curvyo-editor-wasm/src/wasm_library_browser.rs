//! The library browser's `wasm-bindgen` surface (`specs/0152-library-browser`, ADR 0086 decision
//! 5): the list is set once per fetch, and a keystroke is one synchronous `query` here, never an
//! IPC round trip. JSON texts, numbers and `Uint32Array`s only (ADR 0001 §5). The keyword file
//! of `specs/0153-library-keywords` arrives once as bytes and is kept across lists (ADR 0087
//! decision 5).

use std::sync::Arc;

use curvyo_ui_core::library_browser::{
    Answer, BrowseList, BrowseState, Browser, Keywords, LocalView, Place, PlaceHistory, SetsState,
    back_name, hosting_guide, narrowed_to, restored, select_rows,
};
use curvyo_ui_core::library_online::OnlineView;
use wasm_bindgen::prelude::*;

use crate::wasm_library::WasmLibraryIndex;

/// The browser's list, index and last answer for this page.
#[wasm_bindgen]
pub struct WasmLibraryBrowser {
    browser: Browser,
    answer: Answer,
    /// The places left this session; it outlives a new list (UX notes 0.3).
    history: PlaceHistory,
}

impl Default for WasmLibraryBrowser {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmLibraryBrowser {
    /// An empty browser.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> WasmLibraryBrowser {
        Self {
            browser: Browser::new(BrowseList::default()),
            answer: Answer::default(),
            history: PlaceHistory::default(),
        }
    }

    /// Takes the list JSON of `library_browser_items` or `browse_items_json`; `false` when it
    /// does not read (the list is kept).
    pub fn set_items(&mut self, json: &str) -> bool {
        match serde_json::from_str::<BrowseList>(json) {
            Ok(list) => {
                self.browser = Browser::with_keywords(list, self.browser.keywords().cloned());
                true
            }
            Err(_) => false,
        }
    }

    /// Takes the fetched keyword file (gzip or plain); `false` when it does not decode, and the
    /// search goes on without keywords. On success the index is rebuilt once; the page re-runs
    /// its query.
    pub fn set_keywords(&mut self, bytes: &[u8]) -> bool {
        match Keywords::decode(bytes) {
            Ok(keywords) => {
                self.browser.set_keywords(Arc::new(keywords));
                true
            }
            Err(_) => false,
        }
    }

    /// Why item `item` matched the search `text` as JSON `{line, all}` (spec `0153` criterion
    /// 14), asked for the one item the detail bar shows; empty when its name matched or nothing.
    pub fn why(&mut self, item: u32, text: &str) -> String {
        self.browser
            .why(item, text)
            .and_then(|words| serde_json::to_string(&words).ok())
            .unwrap_or_default()
    }

    /// The answer for the state JSON (`BrowseState`), without its order (see [`Self::order`]).
    pub fn query(&mut self, state: &str) -> String {
        let state: BrowseState = serde_json::from_str(state).unwrap_or_default();
        self.answer = self.browser.query(&state);
        serde_json::to_string(&self.answer).unwrap_or_default()
    }

    /// The items of the last answer, in order.
    #[must_use]
    pub fn order(&self) -> Vec<u32> {
        self.answer.order.clone()
    }

    /// The words of the items `indices` (the visible tiles), as a JSON array.
    #[must_use]
    pub fn tiles(&self, indices: &[u32]) -> String {
        let words: Vec<_> = indices
            .iter()
            .filter_map(|i| self.browser.tile(*i))
            .collect();
        serde_json::to_string(&words).unwrap_or_default()
    }

    /// The words of the cards `indices`, as a JSON array.
    #[must_use]
    pub fn cards(&self, indices: &[u32]) -> String {
        let words: Vec<_> = indices
            .iter()
            .filter_map(|i| self.browser.card(*i))
            .collect();
        serde_json::to_string(&words).unwrap_or_default()
    }

    /// The Sets view's answer for the state JSON (`SetsState`, criterion 26).
    pub fn query_sets(&mut self, state: &str) -> String {
        let state: SetsState = serde_json::from_str(state).unwrap_or_default();
        serde_json::to_string(&self.browser.query_sets(&state)).unwrap_or_default()
    }

    /// The maker left the place JSON (`Place`); `by_opening` when something was opened from it
    /// (criterion 30).
    pub fn push_place(&mut self, place: &str, by_opening: bool) {
        if let Ok(place) = serde_json::from_str::<Place>(place) {
            self.history.push(place, by_opening);
        }
    }

    /// Back: the newest place left as JSON, what is no longer fetched dropped; empty when none.
    pub fn back(&mut self) -> String {
        self.history
            .back()
            .map(|place| restored(self.browser.list(), place))
            .and_then(|place| serde_json::to_string(&place).ok())
            .unwrap_or_default()
    }

    /// The Back button's name ("Back to Sets"); empty while there is no place to go back to.
    #[must_use]
    pub fn back_name(&self) -> String {
        self.history
            .peek()
            .map(|place| back_name(self.browser.list(), place))
            .unwrap_or_default()
    }

    /// Whether Escape goes Back (the newest place was left by opening something), else closes.
    #[must_use]
    pub fn escape_goes_back(&self) -> bool {
        self.history.escape_goes_back()
    }

    /// A body arrived for icon `item`; `fixed_colour` when it draws its own colours.
    pub fn note_body(&mut self, item: u32, fixed_colour: bool) {
        self.browser.note_body(item, fixed_colour);
    }
}

#[wasm_bindgen]
impl WasmLibraryIndex {
    /// The browser build's list (Frage 372 A): its stored documents; it has no Online section.
    #[must_use]
    pub fn browse_items_json(&self) -> String {
        let expanded = std::collections::HashSet::new();
        let list = curvyo_ui_core::library_browser::browse_list(
            Some(LocalView {
                sources: &self.sources,
                index: &self.index,
            }),
            &OnlineView {
                sources: Vec::new(),
                ready_offered: false,
                expanded: &expanded,
                query: "",
            },
        );
        serde_json::to_string(&list).unwrap_or_default()
    }
}

/// The selection of a multi-select facet list after a press on row `key` (criterion 31); JSON
/// arrays of keys in and out.
#[wasm_bindgen]
#[must_use]
pub fn library_facet_select(selected: &str, key: &str, children: &str, shift: bool) -> String {
    let selected: Vec<String> = serde_json::from_str(selected).unwrap_or_default();
    let children: Vec<String> = serde_json::from_str(children).unwrap_or_default();
    serde_json::to_string(&select_rows(&selected, key, &children, shift)).unwrap_or_default()
}

/// The Items view's state that opening the set, catalogue or folder `key` shows (criterion 27).
#[wasm_bindgen]
#[must_use]
pub fn library_narrowed_items(key: &str) -> String {
    let state: BrowseState = narrowed_to(key);
    serde_json::to_string(&state).unwrap_or_default()
}

/// The "Host your own library" guide as JSON (criterion 32); it ships with the app.
#[wasm_bindgen]
#[must_use]
pub fn library_hosting_guide() -> String {
    serde_json::to_string(&hosting_guide()).unwrap_or_default()
}
