//! The session state of the active laser page's output (`specs/0029-laser-job-preview-and-output`;
//! ADR 0064 §9 to §11, ADR 0065): the job built from the page, its caches, the Toolpath view, the
//! play bar, the exports not yet saved.

mod commands;
mod cut_keys;
mod export;
mod fixture_pick;
mod focus;
mod frame;
mod gate_facts;
mod input;
mod job_cache;
mod leaf_geometry;
mod pane_picture;
mod pixels;
mod play;
mod raster_picture;
mod raster_view;
mod send;
mod tab_facts;
#[cfg(test)]
mod tests;
mod toolpath;
mod user_origin;
mod view_json;

use std::collections::HashSet;

use curvyo_document_core::NodeId;

pub(in crate::session) use cut_keys::CutKey;
pub use export::ExportedJob;
pub use fixture_pick::FixtureOutputView;
#[cfg(target_arch = "wasm32")]
pub use frame::shape_of;
pub(in crate::session) use job_cache::PageJob;
pub use play::PlayView;
pub use send::{SendView, SentJobFile, send_notice};
pub use tab_facts::RowRunView;
pub(in crate::session) use toolpath::shifted;
pub(in crate::session) use view_json::STALE_TIME;
pub use view_json::{NumbersLineView, OutputGroupView};

/// What the session keeps for the output group and the Toolpath view: the job and geometry caches,
/// the pressed Toolpath views, the play position and the exported jobs not yet saved.
#[derive(Debug, Default)]
pub(in crate::session) struct OutputState {
    /// The active laser page's job (`job_cache.rs`).
    job: job_cache::JobCache,
    /// The page-mm geometry of the output leaves (`input.rs`).
    geometry: input::GeometryCache,
    /// The working images of the output's image leaves (`pixels.rs`).
    pixels: pixels::PixelCache,
    /// The dot pictures of the raster items (`raster_view.rs`).
    raster: raster_view::RasterCache,
    /// The laser pages whose Toolpath view is pressed: session state, never saved.
    toolpath: HashSet<NodeId>,
    /// The toolpath's draw caches (`toolpath.rs`).
    draw: toolpath::DrawCache,
    /// The play bar (`play.rs`).
    play: play::PlayState,
    /// The jobs handed out for saving, by token (`export.rs`).
    exports: export::Exports,
    /// The one pending send and the link to the connector (`send.rs`).
    send: send::SendState,
    /// The head ring of a running frame (`frame.rs`); session state, never saved.
    frame_head: Option<frame::FrameHead>,
    /// The target ring of a go to point (`focus.rs`); session state, never saved.
    focus_ring: Option<frame::FrameHead>,
}
