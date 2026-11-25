//! The rounded box's curved top: a strip over the top and round the back arc, two end plates, a
//! front and a base (ADR 0082 §11, spec `0147` criteria 16 to 24).
//!
//! Elevation plane: x across the width, y from the front to the back, z up. The inner faces of
//! the front, the end plates and the base bound the clear inside; the strip's inner face starts at
//! the front's outer face, runs over the top, round the arc and down to the base's underside.

mod clamp;
mod describe;
mod plates;
mod strip;
#[cfg(test)]
mod tests;

use curvyo_document_core::{Length, Point};
use curvyo_extension_core::{Edit, GenerateRequest, Generated, ParamValue};

use super::layout::{Piece, place};
use crate::slit_bend::SlitPattern;
use crate::values::Values;

/// The measures every part is built from, after all clamps (millimetres).
pub(super) struct Sizes {
    pub(super) width: f64,
    pub(super) depth: f64,
    pub(super) height: f64,
    pub(super) radius: f64,
    /// The strip's thickness.
    pub(super) wall: f64,
    pub(super) end: f64,
    pub(super) front: f64,
    pub(super) base: f64,
    pub(super) kerf: f64,
    /// The largest values the dependent bounds allow now (`Generated::limits`).
    pub(super) limits: Vec<(&'static str, ParamValue)>,
}

impl Sizes {
    /// The strip's width, and the width of the front and the base: the end plates' outer faces.
    pub(super) fn outer_width(&self) -> f64 {
        self.width + 2.0 * self.end
    }

    /// The end plate's width: the depth plus the front's thickness.
    pub(super) fn profile_width(&self) -> f64 {
        self.depth + self.front
    }

    /// The end plate's and the front's height: the height plus the base's thickness.
    pub(super) fn profile_height(&self) -> f64 {
        self.height + self.base
    }
}

/// Generates the rounded box with a curved top that `request` asks for, from the values read
/// from its stored parameters.
pub(super) fn generate(request: &GenerateRequest<'_>, mut v: Values<'_>) -> Generated {
    let mut s = clamp::clamp(&mut v);
    let pattern = clamp::slit_pattern(&mut v, &mut s);
    let draft = matches!(request.edit, Some(Edit::Draft));
    let hollow = v
        .switch("finger_hollow")
        .then(|| crate::wall_plates::Hollow {
            width: v.mm("hollow_width"),
            depth: v.mm("hollow_depth"),
        });
    let strip = strip::strip(&s, pattern, draft);
    let mut parts = vec![Piece::new(strip, s.wall, "strip")];
    parts[0].number = 1;
    let plates = plates::plates(&s, hollow);
    let placed = place(&parts, plates, s.kerf, v.mm("gap") + s.kerf);
    let handles = describe::handles(&placed, &s, hollow.map(|h| h.width));
    let marks = describe::marks(&placed, &s);
    let automatic = used_values(&v, &s, &pattern);
    // The radius at its upper bound reads "max" (ux-notes of `0147` §10.7), as any other bound.
    let at_most = s.limits.iter().any(|(key, value)| {
        *key == "corner_radius"
            && value
                .length()
                .is_some_and(|l| (l.as_mm() - s.radius).abs() < 1e-6)
    });
    let states = if at_most {
        vec![("corner_radius", "max")]
    } else {
        Vec::new()
    };
    let (params, clamps) = v.finish();
    Generated {
        params,
        parts: placed.into_iter().map(|p| p.part).collect(),
        handles,
        clamps,
        automatic,
        states,
        limits: s.limits,
        warnings: Vec::new(),
        marks,
    }
}

/// The values the generator used where the maker stored 0.
fn used_values(
    v: &Values<'_>,
    s: &Sizes,
    pattern: &SlitPattern,
) -> Vec<(&'static str, ParamValue)> {
    let length = |mm: f64| ParamValue::Length(Length::from_mm(mm));
    let mut used = Vec::new();
    for (key, value) in [
        ("plate_thickness", s.base),
        ("end_plate_thickness", s.end),
        ("front_thickness", s.front),
    ] {
        if v.mm(key) == 0.0 {
            used.push((key, length(value)));
        }
    }
    if v.mm("slit_width") == 0.0 {
        used.push(("slit_width", length(pattern.width.as_mm())));
    }
    if v.mm("slit_spacing") == 0.0 {
        used.push(("slit_spacing", length(pattern.spacing.as_mm())));
    }
    if v.count("slit_rows") == 0 {
        used.push(("slit_rows", ParamValue::Count(pattern.rows)));
    }
    used
}

/// The point `(x, y)` of the profile plane as a screen point (z up becomes y down).
pub(super) fn plane(x: f64, z: f64) -> Point {
    Point::new(x, -z)
}
