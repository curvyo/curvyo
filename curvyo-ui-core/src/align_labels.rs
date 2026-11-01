//! The history labels of the align, distribute and grid presses
//! (`specs/0061-align-and-distribute/` criteria 12, 17 and 21), named for
//! the maker in `history_names.rs`.

use crate::align_plan::{AlignAxis, AlignCommand};

/// Align left edges onto the key object or the selection box.
pub const ALIGN_LEFT_COMMIT_LABEL: &str = "align_left";
/// Align horizontal centres onto the key object or the selection box.
pub const ALIGN_CENTRE_H_COMMIT_LABEL: &str = "align_centre_h";
/// Align right edges onto the key object or the selection box.
pub const ALIGN_RIGHT_COMMIT_LABEL: &str = "align_right";
/// Align top edges onto the key object or the selection box.
pub const ALIGN_TOP_COMMIT_LABEL: &str = "align_top";
/// Align vertical centres onto the key object or the selection box.
pub const ALIGN_CENTRE_V_COMMIT_LABEL: &str = "align_centre_v";
/// Align bottom edges onto the key object or the selection box.
pub const ALIGN_BOTTOM_COMMIT_LABEL: &str = "align_bottom";
/// Align left edges onto the active page.
pub const ALIGN_LEFT_PAGE_COMMIT_LABEL: &str = "align_left_page";
/// Align horizontal centres onto the active page.
pub const ALIGN_CENTRE_H_PAGE_COMMIT_LABEL: &str = "align_centre_h_page";
/// Align right edges onto the active page.
pub const ALIGN_RIGHT_PAGE_COMMIT_LABEL: &str = "align_right_page";
/// Align top edges onto the active page.
pub const ALIGN_TOP_PAGE_COMMIT_LABEL: &str = "align_top_page";
/// Align vertical centres onto the active page.
pub const ALIGN_CENTRE_V_PAGE_COMMIT_LABEL: &str = "align_centre_v_page";
/// Align bottom edges onto the active page.
pub const ALIGN_BOTTOM_PAGE_COMMIT_LABEL: &str = "align_bottom_page";
/// Distribute horizontally between the outermost objects.
pub const DISTRIBUTE_H_COMMIT_LABEL: &str = "distribute_h";
/// Distribute vertically between the outermost objects.
pub const DISTRIBUTE_V_COMMIT_LABEL: &str = "distribute_v";
/// Distribute horizontally across the active page.
pub const DISTRIBUTE_H_PAGE_COMMIT_LABEL: &str = "distribute_h_page";
/// Distribute vertically across the active page.
pub const DISTRIBUTE_V_PAGE_COMMIT_LABEL: &str = "distribute_v_page";
/// Arrange in a grid.
pub const ARRANGE_GRID_COMMIT_LABEL: &str = "arrange_grid";

impl AlignCommand {
    /// The history label of this align, relative to the page when `on_page`.
    #[must_use]
    pub const fn commit_label(self, on_page: bool) -> &'static str {
        match (self, on_page) {
            (Self::Left, false) => ALIGN_LEFT_COMMIT_LABEL,
            (Self::CentreH, false) => ALIGN_CENTRE_H_COMMIT_LABEL,
            (Self::Right, false) => ALIGN_RIGHT_COMMIT_LABEL,
            (Self::Top, false) => ALIGN_TOP_COMMIT_LABEL,
            (Self::CentreV, false) => ALIGN_CENTRE_V_COMMIT_LABEL,
            (Self::Bottom, false) => ALIGN_BOTTOM_COMMIT_LABEL,
            (Self::Left, true) => ALIGN_LEFT_PAGE_COMMIT_LABEL,
            (Self::CentreH, true) => ALIGN_CENTRE_H_PAGE_COMMIT_LABEL,
            (Self::Right, true) => ALIGN_RIGHT_PAGE_COMMIT_LABEL,
            (Self::Top, true) => ALIGN_TOP_PAGE_COMMIT_LABEL,
            (Self::CentreV, true) => ALIGN_CENTRE_V_PAGE_COMMIT_LABEL,
            (Self::Bottom, true) => ALIGN_BOTTOM_PAGE_COMMIT_LABEL,
        }
    }
}

/// The history label of a distribute on `axis`, per axis and not per mode
/// (criteria 12, 17), relative to the page when `on_page`.
#[must_use]
pub const fn distribute_commit_label(axis: AlignAxis, on_page: bool) -> &'static str {
    match (axis, on_page) {
        (AlignAxis::Horizontal, false) => DISTRIBUTE_H_COMMIT_LABEL,
        (AlignAxis::Vertical, false) => DISTRIBUTE_V_COMMIT_LABEL,
        (AlignAxis::Horizontal, true) => DISTRIBUTE_H_PAGE_COMMIT_LABEL,
        (AlignAxis::Vertical, true) => DISTRIBUTE_V_PAGE_COMMIT_LABEL,
    }
}
