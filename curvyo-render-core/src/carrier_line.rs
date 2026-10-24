//! The outlines of the carriers of texts on path (`specs/0080-text-on-path` UX notes §2, §4): the
//! carrier line of a selected text (1 px solid `--member-box` over the guide dashes) and the
//! hover outline of a carrier in pick mode (2 px `--hover-box` on its casing) and a carrier in
//! the pick set (2 px `--accent` on its casing).

use curvyo_document_core::{AnchorId, CarrierCurve, NewAnchor, ViewTransform};

use crate::glyphs::DrawList;
use crate::stroke;
use crate::theme;

/// The width of a carrier line, screen px.
const CARRIER_LINE_PX: f64 = 1.0;
/// The width of a carrier's hover outline in pick mode, screen px.
const CARRIER_HOVER_PX: f64 = 2.0;

fn anchors(curve: &CarrierCurve) -> Vec<NewAnchor> {
    curve
        .anchors
        .iter()
        .map(|a| NewAnchor {
            id: AnchorId::new(0, 0),
            point: a.point,
            handle_in: a.handle_in,
            handle_out: a.handle_out,
            kind: a.kind,
        })
        .collect()
}

/// The carrier line of each curve: what a selected text on path is attached to.
#[must_use]
pub fn build_carrier_lines(curves: &[&CarrierCurve], view: ViewTransform) -> DrawList {
    let tolerance = theme::DISPLAY_TOLERANCE_PX / view.scale();
    let width = CARRIER_LINE_PX / view.scale();
    let mut list = DrawList::default();
    for curve in curves {
        list.extend(stroke::path_stroke(
            &anchors(curve),
            curve.closed,
            width,
            theme::MEMBER_BOX,
            tolerance,
        ));
    }
    list
}

/// The hover outline of the carrier under the pointer in pick mode, on its white casing.
#[must_use]
pub fn build_carrier_hover(curve: &CarrierCurve, view: ViewTransform) -> DrawList {
    outline_on_casing(curve, view, theme::HOVER_BOX)
}

/// A carrier in the pick set (slice 2, UX notes §9): 2 px `--accent`, the selected look, on its
/// white casing.
#[must_use]
pub fn build_carrier_picked(curve: &CarrierCurve, view: ViewTransform) -> DrawList {
    outline_on_casing(curve, view, theme::ACCENT)
}

fn outline_on_casing(
    curve: &CarrierCurve,
    view: ViewTransform,
    color: crate::RgbaColor,
) -> DrawList {
    let tolerance = theme::DISPLAY_TOLERANCE_PX / view.scale();
    let width = CARRIER_HOVER_PX / view.scale();
    let anchors = anchors(curve);
    let mut list = stroke::path_stroke(
        &anchors,
        curve.closed,
        width * theme::CASING_WIDTH_FACTOR,
        theme::HOVER_BOX_CASING,
        tolerance,
    );
    list.extend(stroke::path_stroke(
        &anchors,
        curve.closed,
        width,
        color,
        tolerance,
    ));
    list
}
