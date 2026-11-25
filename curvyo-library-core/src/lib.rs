//! The protocols of the online template library (`specs/0138-template-library`, ADR 0085) as pure
//! functions: the text of the sources file, the catalogue index and its checks, the Iconify
//! request builders and parsers, and the hash check of a downloaded file.
//!
//! Pure and wasm-compatible (`CLAUDE.md` §6): no filesystem, network, clock or UI. The host sends
//! the [`Request`]s this crate builds and hands the answers back as bytes.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod catalogue;
mod error;
mod iconify;
mod request;
mod sources;
mod verify;

pub use catalogue::{
    CATALOGUE_FILE, Catalogue, FontEntry, Licence, MAX_INDEX_BYTES, MAX_PICTURE_BYTES,
    MAX_TEMPLATE_BYTES, MAX_TEMPLATES, SUPPORTED_CATALOGUE_FORMAT, TemplateEntry,
    font_licence_is_known, needs_credit,
};
pub use error::{CatalogueError, Failure, IconifyError, RequestError, SourcesError, VerifyError};
pub use iconify::{
    IconBody, IconRef, IconSet, MAX_ICONIFY_BYTES, MAX_SEARCH_RESULTS, MAX_WORDS_CHARS,
    Millimetres, SearchAnswer, collection_request, collections_request, icon_svg, icon_svg_mm,
    icons_request, parse_collection, parse_collections, parse_icons, parse_search, search_request,
    search_words,
};
pub use request::{
    READY_ADDRESS, READY_HOST, Request, SourceKind, base_address, host_of, is_ready,
};
pub use sources::{AddRefusal, Millis, SOURCES_FILE, SourceEntry, Sources};
pub use verify::{sha256_hex, verify};
