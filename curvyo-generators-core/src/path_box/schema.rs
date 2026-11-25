//! The path box's parameter groups: the rounded box's without its sizes, corner radius and wall
//! slits, plus the outline and the Corner angle (ADR 0083 §8; keys are file format).

use curvyo_extension_core::Curve::{Linear, Log};
use curvyo_extension_core::{Param, ParamGroup, ParamKind, ParamValue};

use super::check::default_outline;
use crate::rounded_box::schema::{INNER, base, layout, lid, material, slits};
use crate::schema::{angle, length, tracked};

/// The outline row: stored, never drawn.
fn outline() -> Param {
    Param {
        key: "outline",
        label: "Outline",
        tooltip: "The outline the box is built from",
        kind: ParamKind::Path,
        default: ParamValue::Path(default_outline()),
        shown_when: None,
        track: None,
    }
}

fn size() -> ParamGroup {
    let mut corner = angle("corner_angle", "Corner angle", (5.0, 90.0), 25.0);
    if let ParamKind::Angle { step, decimals, .. } = &mut corner.kind {
        *step = 1.0;
        *decimals = 1;
    }
    ParamGroup {
        label: "Size",
        params: vec![
            outline(),
            tracked(
                length("inner_height", "Inner height", INNER, 50.0, (1.0, 2)),
                10.0,
                500.0,
                Log,
            ),
            tracked(corner, 5.0, 90.0, Linear),
        ],
    }
}

/// Every group of the path box, in order.
pub(crate) fn groups() -> Vec<ParamGroup> {
    vec![size(), material(), slits(false), base(), lid(), layout()]
}
