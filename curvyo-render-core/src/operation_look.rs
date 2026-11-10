//! The look a laser operation's leaves are drawn in on the output page under "View: Operations"
//! (`specs/0101-laser-operations` criterion 18, ux-notes section 13): the kind decides the look,
//! there is no colour per operation. Red is for lines to be cut, pale orange for hatched areas.

use curvyo_document_core::{
    Color, DashPattern, Fill, Length, LineCap, Opacity, Stroke, Style, ViewTransform,
};

use crate::color::RgbaColor;
use crate::theme::{OP_CUT, OP_INK, OP_LANE_LINE};

/// How a leaf is drawn under "View: Operations". The kind of its operation picks the variant (the
/// caller maps `OperationKind` to it, ADR 0046 decision 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationLook {
    /// Cut: a solid red line, one device pixel, on each outline.
    SolidLine,
    /// Score: a dotted ink line, 1.5 px, a round dot every 3 px, on each outline.
    DashedLine,
    /// Engrave: the closed areas hatched (`operation_hatch`); no outline.
    AreaFill,
    /// Engrave + Outline: the hatch, and a 1 px solid ink outline.
    AreaWithLine,
    /// Cut-out: a dashed red line, 2 px, dash 6 on 3, on the outer contour only.
    OuterLine,
    /// Not assigned, Unknown and Output off: a muted dashed outline, no hatch, no symbol.
    MutedDashed,
}

/// The stroke of `look` as a style, widths in screen pixels converted with `view`, so a line keeps
/// its device width at every zoom; dash lengths are multiples of the width. A hatched look has no
/// stroke or fill of its own: its hatch is drawn by `operation_hatch`.
#[must_use]
pub fn look_style(look: OperationLook, view: ViewTransform) -> Style {
    let px = |count: f64| Length::from_mm(count / view.scale());
    let ink = |c: RgbaColor| Color {
        r: c.r,
        g: c.g,
        b: c.b,
    };
    let line = |width: Length, dash: &[f64], colour: RgbaColor, cap: LineCap| Stroke {
        enabled: true,
        width,
        color: ink(colour),
        dash: DashPattern::new(dash.to_vec()).unwrap_or_else(|_| DashPattern::solid()),
        cap,
        ..Stroke::default()
    };
    let none_stroke = Stroke {
        enabled: false,
        ..Stroke::default()
    };
    let no_fill = Fill {
        enabled: false,
        opacity: Opacity::OPAQUE,
        ..Fill::default()
    };
    let stroke = match look {
        OperationLook::SolidLine => line(px(1.0), &[], OP_CUT, LineCap::Butt),
        // Dots: a zero-length dash with round caps, one every 3 px (the width is 1.5 px).
        OperationLook::DashedLine => line(px(1.5), &[0.0, 2.0], OP_INK, LineCap::Round),
        OperationLook::AreaFill => none_stroke,
        OperationLook::AreaWithLine => line(px(1.0), &[], OP_INK, LineCap::Butt),
        // 6 px on, 3 px off, in units of the 2 px width.
        OperationLook::OuterLine => line(px(2.0), &[3.0, 1.5], OP_CUT, LineCap::Butt),
        OperationLook::MutedDashed => line(px(1.0), &[4.0, 3.0], OP_LANE_LINE, LineCap::Butt),
    };
    Style {
        stroke,
        fill: no_fill,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{CANVAS_BG, OP_ENGRAVE_HATCH};

    fn luminance(color: RgbaColor) -> f64 {
        let channel = |c: u8| {
            let c = f64::from(c) / 255.0;
            if c <= 0.039_28 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
    }

    fn contrast(a: RgbaColor, b: RgbaColor) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn the_two_role_colours_are_at_least_3_to_1_on_the_page_ground() {
        for (name, colour) in [("--cut", OP_CUT), ("--engrave-hatch", OP_ENGRAVE_HATCH)] {
            let ratio = contrast(colour, CANVAS_BG);
            assert!(ratio >= 3.0, "{name}: {ratio:.2}:1");
        }
    }

    #[test]
    fn cut_is_a_solid_red_line_one_pixel_wide() {
        let style = look_style(OperationLook::SolidLine, ViewTransform::identity());
        assert!(style.stroke.enabled && style.stroke.dash.is_solid());
        assert!((style.stroke.width.as_mm() - 1.0).abs() < 1e-9);
        assert_eq!(
            (style.stroke.color.r, style.stroke.color.g),
            (OP_CUT.r, OP_CUT.g)
        );
        assert!(!style.fill.enabled);
    }

    #[test]
    fn score_is_dotted_ink_with_round_dots() {
        let style = look_style(OperationLook::DashedLine, ViewTransform::identity());
        assert_eq!(style.stroke.dash.as_slice(), &[0.0, 2.0]);
        assert_eq!(style.stroke.cap, LineCap::Round);
        assert!((style.stroke.width.as_mm() - 1.5).abs() < 1e-9);
        assert_eq!(style.stroke.color.r, OP_INK.r);
    }

    #[test]
    fn cut_out_is_dashed_red_two_pixels_dash_six_on_three() {
        let style = look_style(OperationLook::OuterLine, ViewTransform::identity());
        assert!((style.stroke.width.as_mm() - 2.0).abs() < 1e-9);
        // 6 px on and 3 px off at a 2 px width: 3 and 1.5 widths.
        assert_eq!(style.stroke.dash.as_slice(), &[3.0, 1.5]);
        assert_eq!(style.stroke.color.r, OP_CUT.r);
    }

    #[test]
    fn engrave_has_no_stroke_and_its_hatch_is_drawn_apart() {
        let style = look_style(OperationLook::AreaFill, ViewTransform::identity());
        assert!(!style.stroke.enabled && !style.fill.enabled);
        let both = look_style(OperationLook::AreaWithLine, ViewTransform::identity());
        assert!(both.stroke.enabled && both.stroke.color.r == OP_INK.r);
    }

    #[test]
    fn not_assigned_is_muted_dashed_in_the_lane_line_ink() {
        let muted = look_style(OperationLook::MutedDashed, ViewTransform::identity());
        assert_eq!(muted.stroke.color.r, OP_LANE_LINE.r);
        assert_eq!(muted.stroke.dash.as_slice(), &[4.0, 3.0]);
        assert!(!muted.fill.enabled);
    }
}
