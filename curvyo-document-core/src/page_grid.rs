//! The grid registers of one page (`specs/0090-grid`, `adrs.md` decision 1).
//!
//! Each page stores its grid as independent root registers under
//! `pages::key(id, "grid_<field>")`. Reads are lenient: an absent or malformed
//! field reads as its default and the read writes nothing. [`validate`] is the
//! strict check on open, and [`Document::set_page_grid`] writes only the fields
//! that differ, in one commit.

use std::f64::consts::TAU;

use loro::{LoroDoc, LoroMap, LoroValue, ValueOrContainer};
use serde::Serialize;

use crate::document::{Document, ROOT_MAP};
use crate::pages::{key, write};
use crate::{Angle, Length, Page, Point, Vec2};

/// Below this distance from a full turn, in radians, a normalised angle is
/// stored as 0, so 360° and -0° store 0. Two normalised angles closer than this
/// around the circle are the same angle, so 390° over a stored 30° writes nothing.
const ANGLE_EPS: f64 = 1e-12;

const FIELD_SHOWN: &str = "grid_shown";
const FIELD_MODE: &str = "grid_mode";
const FIELD_SCALE: &str = "grid_scale";
const FIELD_SPACING: &str = "grid_spacing";
const FIELD_SUBDIVISION: &str = "grid_subdivision";
const FIELD_ANGLE: &str = "grid_angle";
const FIELD_ORIGIN: &str = "grid_origin";

/// Whether the shown cell size follows the zoom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GridMode {
    /// Cells refine and merge by the subdivision factor as the zoom changes.
    Adaptive,
    /// Cells are always the base spacing.
    Fixed,
}

/// How lines are laid out along each grid axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GridScale {
    /// Evenly spaced lines.
    Linear,
    /// Log paper: repeating decades with lines at `log10(n)`.
    #[serde(rename = "log")]
    Logarithmic,
}

/// The page corner where the grid starts and the rulers count from (criterion 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginCorner {
    /// The top-left corner: x counts right, y counts down.
    #[default]
    TopLeft,
    /// The top-right corner: x counts left, y counts down.
    TopRight,
    /// The bottom-left corner: x counts right, y counts up.
    BottomLeft,
    /// The bottom-right corner: x counts left, y counts up.
    BottomRight,
}

impl OriginCorner {
    /// Every corner, in panel order.
    pub const ALL: [Self; 4] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
    ];

    /// The stored and wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::TopRight => "top_right",
            Self::BottomLeft => "bottom_left",
            Self::BottomRight => "bottom_right",
        }
    }

    /// The corner with the stored or wire name `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|corner| corner.as_str() == text)
    }

    /// Whether x counts to the left (a right corner).
    #[must_use]
    pub const fn x_reversed(self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight)
    }

    /// Whether y counts upward (a bottom corner).
    #[must_use]
    pub const fn y_reversed(self) -> bool {
        matches!(self, Self::BottomLeft | Self::BottomRight)
    }

    /// The direction of each counting axis in document coordinates: `1` along, `-1` against.
    #[must_use]
    pub const fn signs(self) -> (f64, f64) {
        (
            if self.x_reversed() { -1.0 } else { 1.0 },
            if self.y_reversed() { -1.0 } else { 1.0 },
        )
    }
}

impl GridMode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Adaptive => "adaptive",
            Self::Fixed => "fixed",
        }
    }
    fn parse(text: &str) -> Option<Self> {
        match text {
            "adaptive" => Some(Self::Adaptive),
            "fixed" => Some(Self::Fixed),
            _ => None,
        }
    }
}

impl GridScale {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Logarithmic => "log",
        }
    }
    fn parse(text: &str) -> Option<Self> {
        match text {
            "linear" => Some(Self::Linear),
            "log" => Some(Self::Logarithmic),
            _ => None,
        }
    }
}

/// The grid settings of one page (criterion 1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PageGrid {
    /// Whether the grid is drawn and pulls snapping.
    pub shown: bool,
    /// Adaptive or fixed.
    pub mode: GridMode,
    /// Linear or logarithmic.
    pub scale: GridScale,
    /// The cell size at 100 % zoom (linear), or a tenth of the decade length (log).
    pub spacing: Length,
    /// The refinement factor and the major-line period, at least 2.
    pub subdivision: u32,
    /// Clockwise rotation around the grid origin, normalised to `[0, 2π)`.
    pub angle: Angle,
    /// The page corner where the grid starts and the rulers count from (criterion 16).
    pub origin: OriginCorner,
}

impl PageGrid {
    /// What a new page and a page without grid registers have.
    pub const DEFAULT: Self = Self {
        shown: false,
        mode: GridMode::Adaptive,
        scale: GridScale::Linear,
        spacing: Length::from_mm(5.0),
        subdivision: 5,
        angle: Angle::from_radians(0.0),
        origin: OriginCorner::TopLeft,
    };

    /// Whether this is [`Self::DEFAULT`]; `document.json` leaves such a grid out.
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::DEFAULT
    }

    /// This value with its angle normalised to `[0, 2π)`, or the refusal.
    ///
    /// # Errors
    /// Refuses a non-finite number, a spacing of zero or less, or a subdivision below 2.
    pub fn normalized(self) -> Result<Self, PageGridError> {
        if !(self.spacing.as_mm().is_finite() && self.angle.as_radians().is_finite()) {
            return Err(PageGridError::NotFinite);
        }
        if self.spacing.as_mm() <= 0.0 {
            return Err(PageGridError::Spacing);
        }
        if self.subdivision < 2 {
            return Err(PageGridError::Subdivision);
        }
        Ok(Self {
            angle: Angle::from_radians(normalize_angle(self.angle.as_radians())),
            ..self
        })
    }
}

impl Default for PageGrid {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A refused grid write; it writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PageGridError {
    /// The page identity is unknown.
    #[error("page not found")]
    NotFound,
    /// A spacing or angle is not finite.
    #[error("the grid value is not a finite number")]
    NotFinite,
    /// The spacing is zero or less.
    #[error("the grid spacing must be greater than 0")]
    Spacing,
    /// The subdivision is below 2.
    #[error("the grid subdivision must be 2 or more")]
    Subdivision,
}

/// Whether two angles in `[0, 2π)` are within [`ANGLE_EPS`] of each other around the circle.
fn same_angle(a: f64, b: f64) -> bool {
    let apart = (a - b).abs();
    apart.min(TAU - apart) <= ANGLE_EPS
}

fn normalize_angle(radians: f64) -> f64 {
    let wrapped = radians.rem_euclid(TAU);
    if TAU - wrapped <= ANGLE_EPS || wrapped == 0.0 {
        0.0
    } else {
        wrapped
    }
}

fn spacing_of(value: &LoroValue) -> Option<f64> {
    match value {
        LoroValue::Double(n) if n.is_finite() && *n > 0.0 => Some(*n),
        _ => None,
    }
}

fn finite_of(value: &LoroValue) -> Option<f64> {
    match value {
        LoroValue::Double(n) if n.is_finite() => Some(*n),
        _ => None,
    }
}

fn subdivision_of(value: &LoroValue) -> Option<u32> {
    match value {
        LoroValue::I64(n) => u32::try_from(*n).ok().filter(|n| *n >= 2),
        _ => None,
    }
}

fn bool_of(value: &LoroValue) -> Option<bool> {
    match value {
        LoroValue::Bool(b) => Some(*b),
        _ => None,
    }
}

fn mode_of(value: &LoroValue) -> Option<GridMode> {
    match value {
        LoroValue::String(s) => GridMode::parse(s),
        _ => None,
    }
}

fn origin_of(value: &LoroValue) -> Option<OriginCorner> {
    match value {
        LoroValue::String(s) => OriginCorner::parse(s),
        _ => None,
    }
}

fn scale_of(value: &LoroValue) -> Option<GridScale> {
    match value {
        LoroValue::String(s) => GridScale::parse(s),
        _ => None,
    }
}

/// Whether a stored value of `field` is well formed; `None` for an unknown field.
fn field_is_valid(field: &str, value: &LoroValue) -> Option<bool> {
    let valid = match field {
        FIELD_SHOWN => bool_of(value).is_some(),
        FIELD_MODE => mode_of(value).is_some(),
        FIELD_SCALE => scale_of(value).is_some(),
        FIELD_SPACING => spacing_of(value).is_some(),
        FIELD_SUBDIVISION => subdivision_of(value).is_some(),
        FIELD_ANGLE => finite_of(value).is_some(),
        FIELD_ORIGIN => origin_of(value).is_some(),
        _ => return None,
    };
    Some(valid)
}

/// Reads the grid of page `id` from a root map; each absent or malformed field
/// reads as its default and the read writes nothing.
pub(crate) fn grid_of_root(root: &LoroMap, id: &str) -> PageGrid {
    grid_of_fields(|field| match root.get(&key(id, field)) {
        Some(ValueOrContainer::Value(value)) => Some(value),
        _ => None,
    })
}

/// The grid whose stored fields `value` returns by field name (`grid_<field>`); each absent or
/// malformed field reads as its default.
pub(crate) fn grid_of_fields(value: impl Fn(&str) -> Option<LoroValue>) -> PageGrid {
    let d = PageGrid::DEFAULT;
    PageGrid {
        shown: value(FIELD_SHOWN)
            .and_then(|v| bool_of(&v))
            .unwrap_or(d.shown),
        mode: value(FIELD_MODE)
            .and_then(|v| mode_of(&v))
            .unwrap_or(d.mode),
        scale: value(FIELD_SCALE)
            .and_then(|v| scale_of(&v))
            .unwrap_or(d.scale),
        spacing: value(FIELD_SPACING)
            .and_then(|v| spacing_of(&v))
            .map_or(d.spacing, Length::from_mm),
        subdivision: value(FIELD_SUBDIVISION)
            .and_then(|v| subdivision_of(&v))
            .unwrap_or(d.subdivision),
        angle: Angle::from_radians(normalize_angle(
            value(FIELD_ANGLE)
                .and_then(|v| finite_of(&v))
                .unwrap_or(0.0),
        )),
        origin: value(FIELD_ORIGIN)
            .and_then(|v| origin_of(&v))
            .unwrap_or(d.origin),
    }
}

/// Strict validation on open: every root register `page.<id>.grid_<field>` with
/// a known field holds a plain value of its type and range, deleted pages included.
pub(crate) fn validate(loro: &LoroDoc) -> bool {
    let root = loro.get_map(ROOT_MAP);
    let mut ok = true;
    root.for_each(|name, value| {
        let Some(field) = name
            .strip_prefix("page.")
            .and_then(|rest| rest.rsplit_once('.'))
            .map(|(_, field)| field)
        else {
            return;
        };
        if !field.starts_with("grid_") {
            return;
        }
        let valid = match value {
            ValueOrContainer::Value(value) => field_is_valid(field, &value).unwrap_or(true),
            ValueOrContainer::Container(_) => false,
        };
        ok &= valid;
    });
    ok
}

/// The registers of `grid` that differ from `current`, as key fields and values.
#[allow(clippy::float_cmp)] // Exact register equality avoids writing unchanged values.
fn changed_fields(grid: PageGrid, current: PageGrid) -> Vec<(&'static str, LoroValue)> {
    let mut out = Vec::new();
    if grid.shown != current.shown {
        out.push((FIELD_SHOWN, LoroValue::Bool(grid.shown)));
    }
    if grid.mode != current.mode {
        out.push((FIELD_MODE, LoroValue::from(grid.mode.as_str())));
    }
    if grid.scale != current.scale {
        out.push((FIELD_SCALE, LoroValue::from(grid.scale.as_str())));
    }
    if grid.spacing.as_mm() != current.spacing.as_mm() {
        out.push((FIELD_SPACING, LoroValue::Double(grid.spacing.as_mm())));
    }
    if grid.subdivision != current.subdivision {
        out.push((
            FIELD_SUBDIVISION,
            LoroValue::I64(i64::from(grid.subdivision)),
        ));
    }
    if !same_angle(grid.angle.as_radians(), current.angle.as_radians()) {
        out.push((FIELD_ANGLE, LoroValue::Double(grid.angle.as_radians())));
    }
    if grid.origin != current.origin {
        out.push((FIELD_ORIGIN, LoroValue::from(grid.origin.as_str())));
    }
    out
}

/// Writes the fields of `grid` that differ from the defaults onto page `id`,
/// inside the caller's commit (Add page copies its size source's grid).
pub(crate) fn write_non_default(root: &LoroMap, id: &str, grid: PageGrid) {
    for (field, value) in changed_fields(grid, PageGrid::DEFAULT) {
        write(root, id, field, value);
    }
}

impl Page {
    /// The page's origin corner in document coordinates: where its rulers show 0
    /// (criterion 17).
    #[must_use]
    pub fn zero_point(&self) -> Point {
        let corner = self.grid.origin;
        Point::new(
            self.origin.x
                + if corner.x_reversed() {
                    self.size.width.as_mm()
                } else {
                    0.0
                },
            self.origin.y
                + if corner.y_reversed() {
                    self.size.height.as_mm()
                } else {
                    0.0
                },
        )
    }

    /// The position of a document point measured from the origin corner, positive into the
    /// page (criteria 17 and 19).
    #[must_use]
    pub fn measured(&self, point: Point) -> Vec2 {
        let d = self.zero_point().vector_to(point);
        let (sx, sy) = self.grid.origin.signs();
        Vec2::new(d.x * sx, d.y * sy)
    }

    /// The document point at a position measured from the origin corner; the inverse of
    /// [`Self::measured`].
    #[must_use]
    pub fn placed(&self, position: Vec2) -> Point {
        let (sx, sy) = self.grid.origin.signs();
        self.zero_point()
            .translated(Vec2::new(position.x * sx, position.y * sy))
    }
}

impl Document {
    /// The grid of page `id`; an unknown page reads as the default.
    #[must_use]
    pub fn page_grid(&self, id: &str) -> PageGrid {
        grid_of_root(&self.loro().get_map(ROOT_MAP), id)
    }

    /// Sets the grid of page `id` as one commit labelled `set_page_grid`,
    /// writing only the fields that differ. The angle is normalised first.
    /// Returns `false` and writes nothing when the value equals the stored one.
    ///
    /// # Errors
    /// Refuses an unknown page, a non-finite number, a spacing of zero or less
    /// or a subdivision below 2, writing nothing.
    pub fn set_page_grid(&self, id: &str, grid: PageGrid) -> Result<bool, PageGridError> {
        let grid = grid.normalized()?;
        let page = self
            .pages()
            .into_iter()
            .find(|p| p.id == id)
            .ok_or(PageGridError::NotFound)?;
        let fields = changed_fields(grid, page.grid);
        if fields.is_empty() {
            return Ok(false);
        }
        let root = self.loro().get_map(ROOT_MAP);
        write(&root, id, "deleted", false);
        for (field, value) in fields {
            write(&root, id, field, value);
        }
        self.commit_with_label("set_page_grid");
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_turn_and_negative_zero_normalise_to_zero() {
        assert_eq!(normalize_angle(TAU), 0.0);
        assert_eq!(normalize_angle(-0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(normalize_angle(TAU - 1e-13), 0.0);
        assert!(
            (normalize_angle(-std::f64::consts::FRAC_PI_2) - 1.5 * std::f64::consts::PI).abs()
                < 1e-12
        );
    }

    #[test]
    fn angles_a_full_turn_apart_are_the_same_angle() {
        let stored = normalize_angle(std::f64::consts::FRAC_PI_6);
        let typed = normalize_angle(std::f64::consts::FRAC_PI_6 + TAU);
        assert!(same_angle(stored, typed));
        assert!(same_angle(0.0, normalize_angle(TAU - 1e-13)));
        assert!(same_angle(1e-13, TAU - 1e-13));
        assert!(!same_angle(stored, stored + 1e-9));
    }

    #[test]
    fn unknown_grid_fields_are_ignored_by_validation() {
        assert_eq!(field_is_valid("grid_colour", &LoroValue::Bool(true)), None);
    }
}
