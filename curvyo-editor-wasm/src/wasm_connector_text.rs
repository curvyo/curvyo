//! The Connector row's words that are decided in Rust (`specs/0127` G19): a pass-through to
//! `curvyo-ui-core::controller_warnings`.

use wasm_bindgen::prelude::*;

/// The Controller block's warning lines for the page's profile and the reported settings.
#[wasm_bindgen]
#[must_use]
pub fn controller_warnings(
    profile_laser_mode: bool,
    profile_max_power: f64,
    laser_mode: Option<bool>,
    s_max: Option<u32>,
) -> Vec<String> {
    curvyo_ui_core::controller_warnings(profile_laser_mode, profile_max_power, laser_mode, s_max)
}
