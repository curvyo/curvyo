//! The Grid & Rulers panel's host API for the active page's grid and origin corner
//! (`specs/0090-grid` criteria 2, 3, 16 and 18, `adrs.md` decision 7).
//!
//! Every edit goes through `Document::set_page_grid` as one history step; refusals come back as
//! `invalid:<key>` outcomes the host maps to its notices, writing nothing.

use curvyo_document_core::{Angle, GridMode, GridScale, OriginCorner, PageGrid};
use curvyo_ui_core::{format_field_length, parse_entry_number, parse_length};

use super::Session;

/// The active page's grid as the Grid panel shows it, lengths in the display unit.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
#[derive(Debug, Clone, PartialEq)]
pub struct GridRecord {
    /// The active page's identity.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub page_id: String,
    /// The active page's name, for the panel header row.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub page_name: String,
    /// Whether the grid is shown.
    pub shown: bool,
    /// `"adaptive"` or `"fixed"`.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub mode: String,
    /// `"linear"` or `"log"`.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub scale: String,
    /// The base spacing in the display unit, as the field shows it.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub spacing: String,
    /// The subdivision, as the field shows it.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub subdivision: String,
    /// The angle in degrees, shown in -180 to 180 with at most one decimal.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub angle: String,
    /// The origin corner: `"top_left"`, `"top_right"`, `"bottom_left"` or `"bottom_right"`.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub origin: String,
    /// The display unit's symbol, for the field suffixes.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(getter_with_clone))]
    pub unit: String,
}

/// The angle in degrees for the field: `(-180, 180]`, at most one decimal, never `-0`.
fn angle_text(angle: Angle) -> String {
    let degrees = (angle.normalized().as_radians().to_degrees() * 10.0).round() / 10.0;
    if degrees.abs() < 0.05 {
        "0".to_owned()
    } else {
        format!("{degrees}")
    }
}

/// What one Grid panel edit did, as the host's field outcome.
fn outcome(result: Result<bool, &'static str>) -> String {
    match result {
        Ok(true) => "committed".to_owned(),
        Ok(false) => "unchanged".to_owned(),
        Err(key) => format!("invalid:{key}"),
    }
}

impl Session {
    /// The active page's grid for the Grid panel and the rail entry.
    #[must_use]
    pub fn active_page_grid(&self) -> GridRecord {
        let page = self.active_page();
        let unit = self.display_unit();
        let grid = page.grid;
        GridRecord {
            page_id: page.id,
            page_name: page.name,
            shown: grid.shown,
            mode: match grid.mode {
                GridMode::Adaptive => "adaptive",
                GridMode::Fixed => "fixed",
            }
            .to_owned(),
            scale: match grid.scale {
                GridScale::Linear => "linear",
                GridScale::Logarithmic => "log",
            }
            .to_owned(),
            spacing: format_field_length(grid.spacing, unit),
            subdivision: grid.subdivision.to_string(),
            angle: angle_text(grid.angle),
            origin: grid.origin.as_str().to_owned(),
            unit: unit.symbol().to_owned(),
        }
    }

    /// Writes `edit` of the active page's grid as one history step. `Ok(false)` when nothing
    /// changed or a drag is in flight.
    fn edit_active_grid(&mut self, edit: impl FnOnce(&mut PageGrid)) -> Result<bool, &'static str> {
        if self.operation_in_flight() || self.button_down {
            return Ok(false);
        }
        let page = self.active_page();
        let mut grid = page.grid;
        edit(&mut grid);
        match self.recording(|this| this.document.set_page_grid(&page.id, grid)) {
            Ok(changed) => Ok(changed),
            // The active page always exists; should it not, nothing is written and no notice
            // shows.
            Err(curvyo_document_core::PageGridError::NotFound) => Ok(false),
            Err(curvyo_document_core::PageGridError::Spacing) => Err("spacing_positive"),
            Err(curvyo_document_core::PageGridError::Subdivision) => Err("subdivision"),
            Err(curvyo_document_core::PageGridError::NotFinite) => Err("number"),
        }
    }

    /// Shift+click on the rail entry (criterion 2): shows or hides the active page's grid as one
    /// history step. Returns the new state.
    pub fn toggle_active_page_grid(&mut self) -> bool {
        let shown = !self.active_page().grid.shown;
        let _ = self.edit_active_grid(|grid| grid.shown = shown);
        self.active_page().grid.shown
    }

    /// The panel's Grid switch. Returns whether anything changed.
    pub fn set_active_page_grid_shown(&mut self, shown: bool) -> bool {
        self.edit_active_grid(|grid| grid.shown = shown)
            .unwrap_or(false)
    }

    /// The Mode radios: `"adaptive"` or `"fixed"`. Returns whether anything changed.
    pub fn set_active_page_grid_mode(&mut self, mode: &str) -> bool {
        let mode = match mode {
            "adaptive" => GridMode::Adaptive,
            "fixed" => GridMode::Fixed,
            _ => return false,
        };
        self.edit_active_grid(|grid| grid.mode = mode)
            .unwrap_or(false)
    }

    /// The Scale radios: `"linear"` or `"log"`. Returns whether anything changed.
    pub fn set_active_page_grid_scale(&mut self, scale: &str) -> bool {
        let scale = match scale {
            "linear" => GridScale::Linear,
            "log" => GridScale::Logarithmic,
            _ => return false,
        };
        self.edit_active_grid(|grid| grid.scale = scale)
            .unwrap_or(false)
    }

    /// The origin corner buttons (criterion 18): `"top_left"`, `"top_right"`, `"bottom_left"` or
    /// `"bottom_right"`, as one step. Returns whether anything changed.
    pub fn set_active_page_origin(&mut self, corner: &str) -> bool {
        let Some(corner) = OriginCorner::parse(corner) else {
            return false;
        };
        self.edit_active_grid(|grid| grid.origin = corner)
            .unwrap_or(false)
    }

    /// A typed Grid panel field (criterion 3): `"spacing"`, `"subdivision"` or `"angle"`. Returns `"committed"`, `"unchanged"` or `"invalid:<key>"`
    /// with key `spacing_number`, `spacing_positive`, `subdivision` or `number`.
    pub fn set_active_page_grid_field(&mut self, field: &str, text: &str) -> String {
        let unit = self.display_unit();
        let result = match field {
            "spacing" => match parse_length(text, unit).filter(|l| l.as_mm().is_finite()) {
                None => Err("spacing_number"),
                Some(length) if length.as_mm() <= 0.0 => Err("spacing_positive"),
                Some(length) => self.edit_active_grid(|grid| grid.spacing = length),
            },
            "subdivision" => match parse_entry_number(text, false)
                .filter(|n| n.fract() == 0.0 && *n >= 2.0 && *n <= f64::from(u32::MAX))
            {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Checked above.
                Some(n) => self.edit_active_grid(|grid| grid.subdivision = n as u32),
                None => Err("subdivision"),
            },
            "angle" => match parse_entry_number(text, true) {
                Some(degrees) => self.edit_active_grid(|grid| {
                    grid.angle = Angle::from_radians(degrees.to_radians());
                }),
                None => Err("number"),
            },
            _ => Ok(false),
        };
        outcome(result)
    }
}
