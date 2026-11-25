//! The item model of the library browser (`specs/0152-library-browser`, ADR 0086 decision 4):
//! one list of what the Library tab already holds, as tiles, the sources they come from and the
//! cards of what is not loaded yet.
//!
//! The list is built once per fetch ([`browse_list`]) and crosses the IPC as JSON; the browser in
//! the page ([`Browser`]) filters, ranks and words it per keystroke.

use serde::{Deserialize, Serialize};

use curvyo_library_core::Licence;

/// The keyword file the browser's search takes (spec `0153`), re-exported for the page's host.
pub use curvyo_library_core::Keywords;

pub use crate::library_browser_guide::{
    EXAMPLE_REPOSITORY, Guide, PUBLIC_ONLY_TEXT, hosting_guide,
};
pub use crate::library_browser_items::{LocalView, browse_list};
pub use crate::library_browser_places::{
    Place, PlaceHistory, PlaceView, back_name, narrowed_to, restored, select_rows,
};
pub use crate::library_browser_query::{
    Answer, BrowseState, Browser, ColourFilter, FacetRow, Sort,
};
pub use crate::library_browser_sets::{SetsAnswer, SetsGroup, SetsState};
pub use crate::library_browser_sets_text::CardWords;
pub use crate::library_browser_text::TileWords;
pub use crate::library_browser_why::{WhyWords, why_words};

/// What one tile is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// An icon of a loaded Iconify set or of the Enter results.
    #[default]
    Icon,
    /// A template of a catalogue.
    Template,
    /// One page: of a local document, or of a downloaded template.
    Page,
    /// A local document with several pages; its pages are the second level.
    Document,
}

/// Where items come from: one row of the Source filter and what every item of it shares.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowseSource {
    /// The Source filter's key: `local:<source>`, `online:<address>`, `set:<address>#<prefix>`
    /// or `search`.
    pub key: String,
    /// The filter row's name and the tile's source line: a set's, catalogue's or folder's name.
    pub label: String,
    /// A workspace, folder or stored workspace, under "My documents".
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub local: bool,
    /// The key the host's commands take: the online source key, or the local source key.
    pub host: String,
    /// The Iconify set's prefix.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prefix: String,
    /// "Iconify › Material Symbols", "Curvyo catalogue", "Box job".
    pub place: String,
    /// The Iconify set's category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// The Iconify set's palette flag: `true` colour, `false` monochrome.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<bool>,
    /// The Iconify set's licence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub licence: Option<Licence>,
    /// The Iconify set's author.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
}

/// One tile (`Default` is an icon without a name, so a test can name only what it needs).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowseItem {
    /// What it is.
    pub kind: ItemKind,
    /// The name on the tile.
    pub name: String,
    /// Its source, an index into [`BrowseList::sources`].
    pub source: u32,
    /// An icon's aliases.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// A template's tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// The page names of a local document (searched with the lightest weight).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub page_names: Vec<String>,
    /// An icon's groups in its set ("Arrows").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<String>,
    /// The path of a template or a local document inside its source.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<String>,
    /// A page's id.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub page_id: String,
    /// A page's chip word (`drawing`, `sketch`, `laser`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub page_kind: String,
    /// A page's size ("210 × 297 mm") or a template's ("24 KB").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub size: String,
    /// The document's modification time (local items), Unix ms.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub modified: i64,
    /// The page count of a document or a downloaded template.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub pages: u32,
    /// The document or template a page belongs to, an index into [`BrowseList::items`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    /// Whether a picture (page picture, thumbnail) is there to load.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub picture: bool,
    /// A template tagged `customizable` (Frage 371 A).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tagged_customizable: bool,
    /// Why it cannot be loaded; empty when it can.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub refusal: String,
    /// "Font not included".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub font_note: String,
    /// A template's own licence (icons take their set's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub licence: Option<Licence>,
    /// A template's own author.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_zero(value: &i64) -> bool {
    *value == 0
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

/// What a card offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    /// An added source whose list is not fetched: opening fetches it.
    Source,
    /// The Curvyo catalogue, not added: opening adds it.
    Ready,
    /// An Iconify set, loaded or not: opening loads it if needed and shows its icons.
    Set,
    /// A Git catalogue whose list is fetched: opening shows its templates.
    Catalogue,
    /// A local workspace or folder: opening shows its documents.
    Folder,
}

/// One card of the Sets view (spec criteria 26 to 28, UX notes 0.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowseCard {
    /// What it offers.
    pub kind: CardKind,
    /// The name.
    pub name: String,
    /// The online source key (empty for the ready entry).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key: String,
    /// The host the requests go to.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub host: String,
    /// `git` or `iconify`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub glyph: String,
    /// A set's prefix.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prefix: String,
    /// A set's sample icon names (up to three).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub samples: Vec<String>,
    /// A set's icon count.
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub total: u64,
    /// A set's category.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// A set's palette flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<bool>,
    /// A set's licence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub licence: Option<Licence>,
    /// A set's author.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    /// The last request's failure ("Offline", "The server answered 500."); empty when none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub failure: String,
    /// Whether the failure is Offline (its glyph).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub offline: bool,
    /// A set's icon names or a catalogue's list are held this session.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub loaded: bool,
    /// The group the card sits in: `local`, `ready` or the online source key.
    #[serde(default)]
    pub group: String,
    /// The Items view's Source key opening narrows to; empty when opening stays in Sets.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub narrow: String,
    /// A catalogue's templates or a folder's documents.
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub count: u64,
    /// A folder's pages.
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub pages: u64,
}

/// One group of the Sets view: a source with its cards (UX notes 0.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowseGroup {
    /// `local`, `ready` or the online source key.
    pub key: String,
    /// "My documents", "Curvyo catalogue", a catalogue's name, an Iconify source's host.
    pub label: String,
    /// The order: My documents, the Curvyo catalogue, Git catalogues, Iconify sources.
    pub rank: u8,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

/// Everything the browser lists, as the host builds it once per fetch.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowseList {
    /// The sources, in Source-filter order: local, catalogues, sets, the Enter results.
    pub sources: Vec<BrowseSource>,
    /// The tiles; pages after their document.
    pub items: Vec<BrowseItem>,
    /// The cards of the Sets view: every set, catalogue, folder and unfetched source.
    pub cards: Vec<BrowseCard>,
    /// The Sets view's groups, in order.
    #[serde(default)]
    pub groups: Vec<BrowseGroup>,
    /// The outcome of the last Enter: 'Searching "home"…', '"home": 48 icons', '"home": Offline'.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub search_status: String,
}

#[cfg(test)]
#[path = "library_browser_tests.rs"]
mod tests;
