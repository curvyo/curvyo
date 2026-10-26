//! Lengths and viewports: SVG units to user units and millimetres, the viewBox map with
//! `preserveAspectRatio` (`specs/0024-svg-import-export` criteria 15 to 18, D12).

use kurbo::Affine;
use svgtypes::{Align, AspectRatio, Length, LengthUnit, ViewBox};

/// Millimetres per inch.
pub(crate) const MM_PER_INCH: f64 = 25.4;

/// What a percentage refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
    /// The viewport width.
    X,
    /// The viewport height.
    Y,
    /// The viewport diagonal divided by √2.
    Diagonal,
}

/// How lengths inside the content become user units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Units {
    /// Millimetres per CSS pixel: 25.4/96, or 25.4/72 (Illustrator), 25.4/90 (old Inkscape).
    pub(crate) px_mm: f64,
    /// The viewport in user units, for percentages.
    pub(crate) viewport: (f64, f64),
}

impl Units {
    /// `length` in user units.
    pub(crate) fn user(&self, length: Length, axis: Axis) -> f64 {
        let n = length.number;
        match length.unit {
            LengthUnit::None | LengthUnit::Px => n,
            LengthUnit::Em => n * 16.0,
            LengthUnit::Ex => n * 8.0,
            LengthUnit::Percent => {
                let (w, h) = self.viewport;
                let base = match axis {
                    Axis::X => w,
                    Axis::Y => h,
                    Axis::Diagonal => (w.mul_add(w, h * h) / 2.0).sqrt(),
                };
                n / 100.0 * base
            }
            unit => absolute_mm(n, unit).unwrap_or(0.0) / self.px_mm,
        }
    }

    /// The attribute `text` as a length in user units; `None` when it is not one.
    pub(crate) fn parse(&self, text: &str, axis: Axis) -> Option<f64> {
        let length: Length = text.trim().parse().ok()?;
        let value = self.user(length, axis);
        value.is_finite().then_some(value)
    }
}

/// A length with an absolute unit in millimetres.
fn absolute_mm(n: f64, unit: LengthUnit) -> Option<f64> {
    Some(match unit {
        LengthUnit::Mm => n,
        LengthUnit::Cm => n * 10.0,
        LengthUnit::In => n * MM_PER_INCH,
        LengthUnit::Pt => n * MM_PER_INCH / 72.0,
        LengthUnit::Pc => n * MM_PER_INCH / 6.0,
        _ => return None,
    })
}

/// A size of the root (`width`, `height`) in millimetres; `None` for a missing, `auto`,
/// percentage or unreadable value, or one that is not above zero.
pub(crate) fn root_size_mm(text: Option<&str>, px_mm: f64) -> Option<f64> {
    let length: Length = text?.trim().parse().ok()?;
    let mm = match length.unit {
        LengthUnit::None | LengthUnit::Px => length.number * px_mm,
        LengthUnit::Em => length.number * 16.0 * px_mm,
        LengthUnit::Ex => length.number * 8.0 * px_mm,
        LengthUnit::Percent => return None,
        unit => absolute_mm(length.number, unit)?,
    };
    (mm.is_finite() && mm > 0.0).then_some(mm)
}

/// A usable viewBox (finite, width and height above zero).
pub(crate) fn view_box(text: Option<&str>) -> Option<ViewBox> {
    let vb: ViewBox = text?.parse().ok()?;
    let finite = [vb.x, vb.y, vb.w, vb.h].iter().all(|n| n.is_finite());
    (finite && vb.w > 0.0 && vb.h > 0.0).then_some(vb)
}

/// The aspect rule, `xMidYMid meet` when absent or unreadable.
pub(crate) fn aspect(text: Option<&str>) -> AspectRatio {
    text.and_then(|t| t.parse().ok()).unwrap_or(AspectRatio {
        defer: false,
        align: Align::XMidYMid,
        slice: false,
    })
}

/// The map from the viewBox `vb` to a viewport of `width` × `height` at the origin, by the
/// aspect rule (all nine alignments, `meet`, `slice`, `none`).
pub(crate) fn view_box_map(vb: ViewBox, rule: AspectRatio, width: f64, height: f64) -> Affine {
    let (sx, sy) = (width / vb.w, height / vb.h);
    let (sx, sy) = if rule.align == Align::None {
        (sx, sy)
    } else {
        let s = if rule.slice { sx.max(sy) } else { sx.min(sy) };
        (s, s)
    };
    let (fx, fy) = match rule.align {
        Align::None | Align::XMinYMin => (0.0, 0.0),
        Align::XMidYMin => (0.5, 0.0),
        Align::XMaxYMin => (1.0, 0.0),
        Align::XMinYMid => (0.0, 0.5),
        Align::XMidYMid => (0.5, 0.5),
        Align::XMaxYMid => (1.0, 0.5),
        Align::XMinYMax => (0.0, 1.0),
        Align::XMidYMax => (0.5, 1.0),
        Align::XMaxYMax => (1.0, 1.0),
    };
    let tx = (width - vb.w * sx) * fx;
    let ty = (height - vb.h * sy) * fy;
    Affine::new([sx, 0.0, 0.0, sy, tx - vb.x * sx, ty - vb.y * sy])
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn root_sizes_convert_exactly() {
        let px = MM_PER_INCH / 96.0;
        assert_eq!(root_size_mm(Some("210mm"), px), Some(210.0));
        assert_eq!(root_size_mm(Some("2in"), px), Some(50.8));
        assert!((root_size_mm(Some("96"), px).unwrap() - 25.4).abs() < 1e-12);
        assert!((root_size_mm(Some("72pt"), px).unwrap() - 25.4).abs() < 1e-12);
        assert!((root_size_mm(Some("6pc"), px).unwrap() - 25.4).abs() < 1e-12);
        assert_eq!(root_size_mm(Some("100%"), px), None);
        assert_eq!(root_size_mm(Some("auto"), px), None);
        assert_eq!(root_size_mm(Some("0"), px), None);
    }

    #[test]
    fn the_aspect_rule_places_the_view_box() {
        let vb = ViewBox::new(0.0, 0.0, 200.0, 100.0);
        let meet = view_box_map(vb, aspect(None), 100.0, 100.0);
        assert_eq!(meet.as_coeffs(), [0.5, 0.0, 0.0, 0.5, 0.0, 25.0]);
        let none = view_box_map(vb, aspect(Some("none")), 100.0, 100.0);
        assert_eq!(none.as_coeffs(), [0.5, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let slice = view_box_map(vb, aspect(Some("xMaxYMax slice")), 100.0, 100.0);
        assert_eq!(slice.as_coeffs(), [1.0, 0.0, 0.0, 1.0, -100.0, 0.0]);
        let shifted = view_box_map(
            ViewBox::new(10.0, 10.0, 100.0, 100.0),
            aspect(None),
            100.0,
            100.0,
        );
        assert_eq!(shifted.as_coeffs(), [1.0, 0.0, 0.0, 1.0, -10.0, -10.0]);
    }
}
