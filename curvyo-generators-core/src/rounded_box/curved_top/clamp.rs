//! The curved top's clamps: the values read from the stored parameters, moved into the ranges the
//! parts need, each change reported with its notice (spec `0147` criterion 20, ADR 0082 §11).
//!
//! Only the rows Curved shows are touched; the hidden Flat rows keep their stored values.

use curvyo_document_core::Length;
use curvyo_extension_core::ParamValue;

use super::Sizes;
use crate::joint_layout::{self, Joints};
use crate::rounded_box::{own, slit_values_for};
use crate::slit_bend::{SlitKind, SlitPattern};
use crate::values::Values;

/// The values, clamped in the order the limits depend on each other.
pub(super) fn clamp(v: &mut Values<'_>) -> Sizes {
    let (width, depth) = (v.mm("inner_width"), v.mm("inner_depth"));
    v.limit_mm("thickness", 0.0, width.min(depth) / 6.0);
    let wall = v.mm("thickness");
    let used = |v: &Values<'_>, key: &str| own(v, key).unwrap_or(wall);
    let (end, front, base) = (
        used(v, "end_plate_thickness"),
        used(v, "front_thickness"),
        used(v, "plate_thickness"),
    );
    // The back straight must hold a tab with its margins, so the radius range is never empty.
    v.limit_mm("inner_height", 2.0 * end + 6.0 * wall, f64::MAX);
    let height = v.mm("inner_height");
    let radius_max = (depth - 2.0 * wall).min(height - 2.0 * end - 4.0 * wall);
    v.limit_mm("corner_radius", 2.0 * wall, radius_max);
    v.limit_mm("kerf", 0.0, wall.min(end).min(front).min(base) / 2.0);
    let mm = |value: f64| ParamValue::Length(Length::from_mm(value));
    let mut limits = vec![("corner_radius", mm(radius_max))];
    if v.switch("finger_hollow") {
        // Clear of the front's side fingers, and no wider than the height: a U notch is as deep
        // as half its width at least, and no deeper than half the height, so it never runs out of
        // the front's bottom edge.
        let widest = (width - 2.0 * end).min(height).max(5.0);
        v.limit_mm("hollow_width", 5.0, widest);
        let half = v.mm("hollow_width") / 2.0;
        let deepest = height / 2.0;
        v.limit_mm("hollow_depth", half, deepest);
        limits.push(("hollow_width", mm(widest)));
        limits.push(("hollow_depth", mm(deepest)));
    }
    let radius = v.mm("corner_radius");
    let joints = joints(v, (width, depth, height), (wall, end, front, base), radius);
    Sizes {
        width,
        depth,
        height,
        radius,
        wall,
        end,
        front,
        base,
        kerf: v.mm("kerf"),
        limits,
        joints,
        used_count: 0,
    }
}

/// The joint layout fitted to the tightest finger joint and end-plate tab run, with notices.
fn joints(
    v: &mut Values<'_>,
    (width, depth, height): (f64, f64, f64),
    (wall, end, front, base): (f64, f64, f64, f64),
    radius: f64,
) -> Joints {
    if own(v, "joint_width").is_some() {
        v.limit_mm(
            "joint_width",
            joint_layout::least_width(v.mm("kerf")),
            f64::MAX,
        );
    }
    // The widest automatic finger of the four joints, and the spans they run along.
    let auto = 2.0
        * [front.min(end), base.min(end), base.min(wall)]
            .into_iter()
            .fold(front.min(base), f64::max);
    let tabs = height - radius - 4.0 * wall;
    let fitted = joint_layout::fit_fingers(
        &[height, width, depth, tabs],
        auto,
        Joints::read(v, "joint_width"),
    );
    joint_layout::apply(v, &fitted.limits, "joint_width");
    fitted.joints
}

/// The slit values over the arc. The arc keeps two columns: a radius too small for two columns at
/// the least spacing is raised, and a typed spacing the arc cannot hold twice is clamped down to
/// half the arc's neutral length, pi/8 (2 r + `t_w`), with a notice naming Spacing (criterion 19).
pub(super) fn slit_pattern(v: &mut Values<'_>, s: &mut Sizes) -> SlitPattern {
    let strip_width = s.outer_width();
    let mut pattern = slit_values_for(v, (s.wall, s.kerf, s.radius), strip_width).0;
    let least = if pattern.kind == SlitKind::Wave {
        (pattern.width.as_mm() + 1.0) / 0.88
    } else {
        pattern.width.as_mm() + 1.0
    };
    // Two columns of `least` on a quarter circle of neutral radius r + t / 2.
    let two_columns = 4.0 * least / std::f64::consts::PI - s.wall / 2.0;
    if s.radius < two_columns {
        let upper = s
            .limits
            .iter()
            .find(|(key, _)| *key == "corner_radius")
            .and_then(|(_, value)| value.length())
            .map_or(two_columns, Length::as_mm);
        v.limit_mm("corner_radius", two_columns.min(upper), f64::MAX);
        s.radius = v.mm("corner_radius");
        pattern = slit_values_for(v, (s.wall, s.kerf, s.radius), strip_width).0;
    }
    let half_arc = std::f64::consts::PI / 8.0 * (2.0 * s.radius + s.wall);
    if own(v, "slit_spacing").is_some() && pattern.spacing.as_mm() > half_arc {
        v.limit_mm("slit_spacing", 0.0, half_arc);
        pattern = slit_values_for(v, (s.wall, s.kerf, s.radius), strip_width).0;
    }
    pattern
}
