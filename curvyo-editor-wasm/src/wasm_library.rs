//! The document library's `wasm-bindgen` surface (`specs/0137-document-library`, ADR 0084 §5,
//! decision 10): the read-only source of the preview pane and the import, the session's import
//! entries and Came from lines, and the browser build's index over its stored workspaces.
//! Strings, bytes and scalars only (ADR 0001 §5); node ids travel as their JSON.

use std::collections::HashSet;
use std::io::Cursor;

use curvyo_document_core::{CameFrom, NodeId, ObjectSnapshot, Point};
use curvyo_ui_core::library_index::DocumentState;
use curvyo_ui_core::library_index::LibraryIndex;
use curvyo_ui_core::library_index_text::{SourceReason, document_error_text};
use curvyo_ui_core::library_tree::{LibrarySource, SourceKind, TreeView, library_rows};
use wasm_bindgen::prelude::*;

use crate::session::{ImportOptions, PreviewView, Session};
use crate::wasm_api::WasmSession;
use crate::wasm_thumbnail::WasmThumbnail;

/// A document opened read-only for the page preview pane and the import (ADR 0084 §5): never
/// packed, never committed into, no recovery copy, no room.
#[wasm_bindgen]
pub struct WasmLibrarySource {
    pub(crate) session: Session,
    /// The pane's size in CSS pixels.
    size: (f64, f64),
    /// The pane's zoom and pan.
    view: PreviewView,
}

pub(crate) fn ids_of(json: &str) -> Vec<NodeId> {
    serde_json::from_str(json).unwrap_or_default()
}

fn id_json(id: NodeId) -> String {
    serde_json::to_string(&id).unwrap_or_default()
}

#[wasm_bindgen]
impl WasmLibrarySource {
    /// Opens container `bytes` read-only; a newer or damaged file is refused with the sentence
    /// to show.
    ///
    /// # Errors
    /// A `JsValue` string: "Cannot be read: damaged." and the like.
    pub fn open(bytes: &[u8], peer: u64) -> Result<WasmLibrarySource, JsValue> {
        Session::open(peer, bytes)
            .map(|mut session| {
                session.resize_viewport(800.0, 600.0);
                Self {
                    session,
                    size: (800.0, 600.0),
                    view: PreviewView::default(),
                }
            })
            .map_err(|error| {
                let state = match error {
                    curvyo_document_core::OpenError::FormatTooNew { .. } => DocumentState::Newer,
                    curvyo_document_core::OpenError::NotAProject => DocumentState::NotCurvyo,
                    curvyo_document_core::OpenError::Damaged => DocumentState::Damaged,
                };
                JsValue::from_str(document_error_text(state).unwrap_or_default())
            })
    }

    /// Opens an icon's SVG text as a read-only source (`specs/0138` criterion 3): the same SVG
    /// reader and limits as Import SVG, its drawing on the first page.
    ///
    /// # Errors
    /// A `JsValue` string when the text gives no drawing.
    pub fn open_svg(svg: &str, peer: u64) -> Result<WasmLibrarySource, JsValue> {
        let mut session = Session::open_file(peer, svg.as_bytes(), "icon.svg")
            .map_err(|error| JsValue::from_str(&error))?;
        session.continue_open_svg(true);
        session.resize_viewport(800.0, 600.0);
        Ok(Self {
            session,
            size: (800.0, 600.0),
            view: PreviewView::default(),
        })
    }

    /// Sets the pane's size (CSS pixels), zoom over the fit and pan (CSS pixels).
    pub fn set_view(&mut self, width: f64, height: f64, zoom: f64, pan_x: f64, pan_y: f64) {
        self.size = (width, height);
        self.view = PreviewView {
            zoom,
            pan: (pan_x, pan_y),
        };
    }

    /// Draws page `page` into `view`'s canvas under the pane's view.
    ///
    /// # Errors
    /// A `JsValue` (a plain string) if the frame could not be submitted.
    pub fn render_page(&mut self, view: &mut WasmThumbnail, page: &str) -> Result<bool, JsValue> {
        let Some(found) = self.page(page) else {
            return Ok(false);
        };
        let Some(transform) = self.session.preview_transform(page, self.size, self.view) else {
            return Ok(false);
        };
        let (mut list, transform) = self.session.page_frame_in(&found, transform);
        let pictures = self.session.frame_images(&mut list);
        view.gpu.render(&list, transform, &pictures)?;
        Ok(true)
    }

    fn page(&self, id: &str) -> Option<curvyo_document_core::Page> {
        self.session
            .source_document()
            .pages()
            .into_iter()
            .find(|p| p.id == id)
    }

    /// The pickable objects of `page` in stacking order, front first: `[{id, name, hidden}]`;
    /// `name` is the name the Layers row shows (its own, or "Rectangle 3f9a1c").
    #[must_use]
    pub fn page_objects(&self, page: &str) -> String {
        let document = self.session.source_document();
        let names = curvyo_ui_core::shown_names(document);
        let mut objects: Vec<serde_json::Value> = self
            .session
            .page_top_level(page)
            .into_iter()
            .map(|id| {
                let registers = document.node_registers(id);
                serde_json::json!({
                    "id": id,
                    "name": names.get(&id).cloned().unwrap_or_else(|| format!("Object {}", id.short_id())),
                    "hidden": registers.is_some_and(|r| !r.visible),
                })
            })
            .collect();
        objects.reverse();
        serde_json::Value::Array(objects).to_string()
    }

    /// The pane's announcement of the ringed object (UX notes 6.4): "Rectangle 3f9a1c, 2 of 42,
    /// picked".
    #[must_use]
    pub fn pick_announcement(name: &str, at: usize, of: usize, picked: bool) -> String {
        curvyo_ui_core::library_index_text::pick_announcement(name, at, of, picked)
    }

    /// The pane footer's count (UX notes 6.2): "Nothing picked", "1 of 1 element picked".
    #[must_use]
    pub fn picked_text(picked: usize, of: usize) -> String {
        curvyo_ui_core::library_index_text::picked_text(picked, of)
    }

    /// The pane boxes `[x0, y0, x1, y1]` of `ids` (JSON), for the hover and pick boxes.
    #[must_use]
    pub fn boxes(&self, page: &str, ids: &str) -> String {
        let Some(view) = self.session.preview_transform(page, self.size, self.view) else {
            return "[]".to_owned();
        };
        let boxes: Vec<[f64; 4]> = ids_of(ids)
            .into_iter()
            .map(|id| {
                let (min, max) = self.box_of(id);
                let (x0, y0) = view.document_to_screen(min);
                let (x1, y1) = view.document_to_screen(max);
                [x0, y0, x1, y1]
            })
            .collect();
        serde_json::to_string(&boxes).unwrap_or_default()
    }

    fn box_of(&self, id: NodeId) -> (Point, Point) {
        let document = self.session.source_document();
        let leaves = if document.is_group(id) {
            document.leaves_of(id)
        } else {
            vec![id]
        };
        leaves
            .into_iter()
            .filter_map(|leaf| document.object(leaf))
            .map(|o: ObjectSnapshot| curvyo_ui_core::object_outline_bounds(&o))
            .fold(
                (
                    Point::new(f64::INFINITY, f64::INFINITY),
                    Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
                ),
                |(lo, hi), (min, max)| {
                    (
                        Point::new(lo.x.min(min.x), lo.y.min(min.y)),
                        Point::new(hi.x.max(max.x), hi.y.max(max.y)),
                    )
                },
            )
    }

    /// The top-level object a click at the pane point `x`, `y` picks (by outline), as its JSON
    /// id, or the empty string.
    #[must_use]
    pub fn hit(&self, page: &str, x: f64, y: f64) -> String {
        self.session
            .preview_hit(page, self.size, self.view, (x, y))
            .map(id_json)
            .unwrap_or_default()
    }

    /// The top-level objects the marquee `[x0, y0, x1, y1]` picks (rightward contains, leftward
    /// touches), as a JSON array.
    #[must_use]
    pub fn marquee(&self, page: &str, corners: &[f64]) -> String {
        let [x0, y0, x1, y1] = corners else {
            return "[]".to_owned();
        };
        let picked =
            self.session
                .preview_marquee(page, self.size, self.view, ((*x0, *y0), (*x1, *y1)));
        serde_json::to_string(&picked).unwrap_or_default()
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Imports from `source` (ADR 0084 §6): the page `page` when given, else the objects `roots`
    /// (JSON ids) onto the active page centred on the canvas point `drop_x`, `drop_y` (or on the
    /// visible canvas when `has_drop` is false). `came_from` is the record as JSON (`at` from
    /// the page's clock). Returns `{"page", "notice"}` or `{"error"}` as JSON.
    #[allow(clippy::needless_pass_by_value, clippy::too_many_arguments)]
    pub fn import_from_library(
        &mut self,
        source: &WasmLibrarySource,
        roots: &str,
        page: Option<String>,
        came_from: &str,
        drop_x: f64,
        drop_y: f64,
        has_drop: bool,
    ) -> String {
        let Ok(record) = serde_json::from_str::<CameFromWire>(came_from) else {
            return serde_json::json!({ "error": "the record is not readable" }).to_string();
        };
        let options = ImportOptions {
            label: (record.step.as_deref() == Some("icon"))
                .then_some(curvyo_document_core::IMPORT_ICON_COMMIT_LABEL),
            without_fonts: record.without_fonts,
        };
        let record = record.into_record();
        let drop = has_drop.then_some((drop_x, drop_y));
        match self.session.import_with(
            &source.session,
            &ids_of(roots),
            page.as_deref(),
            &record,
            drop,
            options,
        ) {
            Ok(done) => serde_json::to_string(&done).unwrap_or_default(),
            Err(text) => serde_json::json!({ "error": text }).to_string(),
        }
    }

    /// Loads every page of `source` as new pages after the last page in one step (`specs/0138`
    /// criterion 3); `came_from` is the record as JSON with the online keys. Returns `{"page",
    /// "notice"}` or `{"error"}` as JSON.
    #[allow(clippy::needless_pass_by_value)]
    pub fn load_online_template(&mut self, source: &WasmLibrarySource, came_from: &str) -> String {
        let Ok(record) = serde_json::from_str::<CameFromWire>(came_from) else {
            return serde_json::json!({ "error": "the record is not readable" }).to_string();
        };
        let without_fonts = record.without_fonts;
        let record = record.into_record();
        match self
            .session
            .load_online_template(&source.session, &record, without_fonts)
        {
            Ok(done) => serde_json::to_string(&done).unwrap_or_default(),
            Err(text) => serde_json::json!({ "error": text }).to_string(),
        }
    }

    /// The credits of the document's objects as JSON (`specs/0138` criterion 4).
    #[must_use]
    pub fn credits_json(&self) -> String {
        self.session.credits_json()
    }

    /// Sets the page's clock (Unix ms, local offset in minutes) for the Came from lines of the
    /// Layers rows.
    pub fn set_came_from_clock(&self, now: f64, offset_minutes: i32) {
        #[allow(clippy::cast_possible_truncation)] // a clock in ms fits
        self.session.set_came_from_clock(now as i64, offset_minutes);
    }

    /// The Came from line of the selection, as JSON, or the empty string.
    #[must_use]
    pub fn came_from_view(&self, now: f64, offset_minutes: i32) -> String {
        #[allow(clippy::cast_possible_truncation)] // a clock in ms fits
        self.session.came_from_view(now as i64, offset_minutes)
    }

    /// The Came from line of the page `id`, as JSON, or the empty string.
    #[must_use]
    pub fn page_came_from_view(&self, id: &str, now: f64, offset_minutes: i32) -> String {
        #[allow(clippy::cast_possible_truncation)] // a clock in ms fits
        self.session
            .page_came_from_view(id, now as i64, offset_minutes)
    }
}

/// The record as the page sends it: the Came from fields, the online keys of an online item and
/// how the import is run (`step` `icon`, fonts left out).
#[derive(serde::Deserialize)]
pub(crate) struct CameFromWire {
    document: String,
    page: String,
    page_id: String,
    at: i64,
    #[serde(default)]
    online: Option<curvyo_document_core::OnlineOrigin>,
    #[serde(default)]
    without_fonts: bool,
    #[serde(default)]
    step: Option<String>,
}

impl CameFromWire {
    pub(crate) fn into_record(self) -> CameFrom {
        CameFrom {
            document: self.document,
            page: self.page,
            page_id: self.page_id,
            item: None,
            at: curvyo_document_core::UnixMillis::new(self.at),
            online: self.online,
        }
    }
}

/// Joins `thumbnail` and the page pictures into packed container `bytes` (the browser's save,
/// ADR 0084 decision 10); `pages` is a JSON array of page ids in the order of `pictures`, the
/// PNG bytes laid end to end with their lengths in `lengths`. The bytes as they were when they
/// are no container.
#[wasm_bindgen]
#[must_use]
pub fn library_with_pictures(
    bytes: &[u8],
    thumbnail: &[u8],
    pages: &str,
    pictures: &[u8],
    lengths: &[u32],
) -> Vec<u8> {
    let ids: Vec<String> = serde_json::from_str(pages).unwrap_or_default();
    let mut rest = pictures;
    let mut joined = Vec::new();
    for (id, length) in ids.into_iter().zip(lengths) {
        let length = (*length as usize).min(rest.len());
        let (png, others) = rest.split_at(length);
        joined.push((id, png.to_vec()));
        rest = others;
    }
    let thumbnail = (!thumbnail.is_empty()).then_some(thumbnail);
    curvyo_document_core::with_pictures(bytes, thumbnail, &joined)
        .unwrap_or_else(|_| bytes.to_vec())
}

/// The PNG of page `page`'s picture in container `bytes`; empty when it has none (the browser
/// loads a stored entry whole, ADR 0084 decision 10).
#[wasm_bindgen]
#[must_use]
pub fn library_page_picture(bytes: &[u8], page: &str) -> Vec<u8> {
    curvyo_document_core::read_page_picture(Cursor::new(bytes), page).unwrap_or_default()
}

/// The browser build's library index (ADR 0084 decision 10): the same records, cache text,
/// search and rows as the desktop host, over the workspaces the browser stores.
#[wasm_bindgen]
pub struct WasmLibraryIndex {
    pub(crate) index: LibraryIndex,
    pub(crate) sources: Vec<LibrarySource>,
}

#[derive(serde::Deserialize)]
struct SourceWire {
    key: String,
    name: String,
    open: bool,
}

#[wasm_bindgen]
impl WasmLibraryIndex {
    /// The index of the cache `text` (empty when damaged or newer).
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(text: &str) -> WasmLibraryIndex {
        Self {
            index: LibraryIndex::parse(text),
            sources: Vec::new(),
        }
    }

    /// The cache text to store.
    #[must_use]
    pub fn to_text(&self) -> String {
        self.index.to_text()
    }

    /// Sets the sources in order, JSON `[{key, name, open}]`; records of other sources go.
    pub fn set_sources(&mut self, json: &str) {
        let wire: Vec<SourceWire> = serde_json::from_str(json).unwrap_or_default();
        self.sources = wire
            .into_iter()
            .map(|s| LibrarySource {
                key: s.key,
                name: s.name,
                kind: SourceKind::Stored(s.open),
                error: None::<SourceReason>,
            })
            .collect();
        let keys: Vec<String> = self.sources.iter().map(|s| s.key.clone()).collect();
        self.index.order_by_sources(&keys);
    }

    /// Whether the entry at `path` (JSON array) of `source` must be read again.
    #[must_use]
    pub fn needs_read(&self, source: &str, path: &str, size: f64, modified: f64) -> bool {
        let path: Vec<String> = serde_json::from_str(path).unwrap_or_default();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        self.index.needs_read(
            source,
            &path,
            size as u64,
            curvyo_document_core::UnixMillis::new(modified as i64),
        )
    }

    /// Reads the members of the stored container `bytes` into the record of `path` of `source`.
    pub fn put(&mut self, source: &str, path: &str, modified: f64, bytes: &[u8]) {
        let path: Vec<String> = serde_json::from_str(path).unwrap_or_default();
        #[allow(clippy::cast_possible_truncation)]
        let modified = curvyo_document_core::UnixMillis::new(modified as i64);
        self.index
            .put(curvyo_ui_core::library_index::record_of_container(
                source, path, modified, bytes,
            ));
    }

    /// Drops the records of `source` whose path is not among `paths` (JSON array of arrays).
    pub fn keep_present(&mut self, source: &str, paths: &str) {
        let paths: Vec<Vec<String>> = serde_json::from_str(paths).unwrap_or_default();
        self.index.keep_present(source, &paths);
    }

    /// The number of documents.
    #[must_use]
    pub fn count(&self) -> usize {
        self.index.documents().len()
    }

    /// The tree rows as JSON for the search, the expanded keys (JSON) and the clock; `saved` is
    /// the JSON `[source, path]` of this tab's dirty document, or empty.
    #[must_use]
    pub fn rows(
        &self,
        query: &str,
        expanded: &str,
        now: f64,
        offset_minutes: i32,
        saved: &str,
    ) -> String {
        let expanded: HashSet<String> = serde_json::from_str(expanded).unwrap_or_default();
        let saved: Option<(String, Vec<String>)> = serde_json::from_str(saved).ok();
        #[allow(clippy::cast_possible_truncation)]
        let rows = library_rows(
            &self.sources,
            &self.index,
            &TreeView {
                expanded: &expanded,
                query,
                now: (now as i64, offset_minutes),
                saved_version: saved.as_ref().map(|(s, p)| (s.as_str(), p.as_slice())),
            },
        );
        serde_json::to_string(&rows).unwrap_or_default()
    }

    /// Where Open source finds the record JSON: `{source, path}` or the empty string.
    #[must_use]
    pub fn find_source(&self, record: &str) -> String {
        let Ok(wire) = serde_json::from_str::<CameFromWire>(record) else {
            return String::new();
        };
        let record = CameFrom {
            document: wire.document,
            page: wire.page,
            page_id: wire.page_id,
            item: None,
            at: curvyo_document_core::UnixMillis::new(wire.at),
            online: None,
        };
        self.index
            .source_of(&record)
            .map(|d| serde_json::json!({ "source": d.source, "path": d.path }).to_string())
            .unwrap_or_default()
    }
}

/// The notice of Open source when the source has the page no more (UX notes section 8.3).
#[wasm_bindgen]
#[must_use]
pub fn library_page_gone_text(document: &str, page: &str) -> String {
    curvyo_ui_core::library_index_text::page_gone_text(document, page)
}

/// The alert of Open source when the file went (UX notes section 8.3).
#[wasm_bindgen]
#[must_use]
pub fn library_source_gone_text(document: &str) -> String {
    curvyo_ui_core::library_index_text::source_gone_text(document)
}

/// The alert of an import that failed (UX notes section 10).
#[wasm_bindgen]
#[must_use]
pub fn library_import_failed_text(name: &str, reason: &str) -> String {
    curvyo_ui_core::library_index_text::import_failed_text(name, reason)
}

/// The search's empty result (UX notes section 5).
#[wasm_bindgen]
#[must_use]
pub fn library_no_match_text(query: &str) -> String {
    curvyo_ui_core::library_index_text::no_match_text(query)
}

/// The armed Remove's name (UX notes section 4.4).
#[wasm_bindgen]
#[must_use]
pub fn library_remove_armed_text(name: &str) -> String {
    curvyo_ui_core::library_index_text::remove_armed_text(name)
}

/// What Add sends, under the Address field of the Add source form (`specs/0138` UX F6).
#[wasm_bindgen]
#[must_use]
pub fn library_online_add_sent_text(iconify: bool) -> String {
    curvyo_ui_core::library_online::add_sent_text(iconify).to_owned()
}

/// The fixed texts of the access token field, the Lock and the token form as JSON
/// (`specs/0157-library-git-token`, UX notes sections 2 to 4).
#[wasm_bindgen]
#[must_use]
pub fn library_token_words() -> String {
    serde_json::to_string(&curvyo_ui_core::library_online::token_words()).unwrap_or_default()
}

/// The announcement after Enter in the search: "Searching 2 Iconify sources".
#[wasm_bindgen]
#[must_use]
pub fn library_online_searching_text(sources: usize) -> String {
    curvyo_ui_core::library_online::searching_text(sources)
}
