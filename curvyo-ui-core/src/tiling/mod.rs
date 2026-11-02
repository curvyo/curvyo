//! Tiling (`specs/0069-tiling`, ADR 0030): the seeded generator, the layout of the cells and the
//! variance of each cell, all pure; the document writes what they compute.

mod handles;
mod layout;
mod pcg32;
mod plan;
mod variance;

pub use handles::{CircleHandle, CircleView, normalised_start};
pub use layout::{
    CellBox, converted_spacing, default_circle, default_recipe, grid_pitch, tiling_layout,
};
pub use pcg32::Pcg32;
pub use plan::{
    CountStatus, TileButton, cell_box, count_status, images_become_copies, tile_button,
    tiling_request,
};
pub use variance::{CellVariance, cell_variance};
