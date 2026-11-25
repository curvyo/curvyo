//! The protocols of the online template library (`specs/0138-template-library`, ADR 0085) as pure
//! functions: the text of the sources file, the catalogue index and its checks, the Iconify
//! request builders and parsers, the hash check of a downloaded file, and the fuzzy matcher of the
//! library browser (`specs/0152-library-browser`, ADR 0086) with its keyword file (`specs/0153`,
//! ADR 0087).
//!
//! Pure and wasm-compatible (`CLAUDE.md` §6): no filesystem, network, clock or UI. The host sends
//! the [`Request`]s this crate builds and hands the answers back as bytes.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod catalogue;
mod error;
mod fold;
mod fold_table;
mod iconify;
mod iconify_listing;
mod iconify_tile;
mod keywords;
mod request;
mod search;
mod search_related;
mod sources;
mod verify;

pub use catalogue::{
    CATALOGUE_FILE, Catalogue, FontEntry, Licence, MAX_INDEX_BYTES, MAX_PICTURE_BYTES,
    MAX_TEMPLATE_BYTES, MAX_TEMPLATES, SUPPORTED_CATALOGUE_FORMAT, TemplateEntry,
    font_licence_is_known, needs_credit,
};
pub use error::{
    CatalogueError, Failure, IconifyError, KeywordsError, RequestError, SourcesError, VerifyError,
};
pub use fold::fold;
pub use iconify::{
    IconBody, IconRef, IconSet, MAX_ICONIFY_BYTES, MAX_SEARCH_RESULTS, MAX_WORDS_CHARS,
    Millimetres, SearchAnswer, collection_request, collections_request, icon_svg, icon_svg_mm,
    icons_request, parse_collection, parse_collections, parse_icons, parse_search, search_request,
    search_words,
};
pub use iconify_listing::{SetListing, parse_set_listing};
pub use iconify_tile::{Rgb8, has_fixed_colour, icon_tile_svg};
pub use keywords::{IconWords, KeywordSource, Keywords, MAX_KEYWORDS_BYTES, MAX_RELATED};
pub use request::{
    READY_ADDRESS, READY_HOST, Request, SourceKind, base_address, host_of, is_ready,
};
pub use search::{
    MAX_QUERY_CHARS, Scratch, SearchIndex, WEIGHT_ALIAS, WEIGHT_CATEGORY, WEIGHT_KEYWORD,
    WEIGHT_NAME, WEIGHT_RELATED, WEIGHT_SOURCE, WEIGHT_TAG, Weight, Why,
};
pub use sources::{AddRefusal, Millis, SOURCES_FILE, SourceEntry, Sources};
pub use verify::{sha256_hex, verify};
