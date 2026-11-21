//! The visible part of a picture (ADR 0072 §3; `specs/0135` criterion 5): the unit rectangle its
//! switched-on Crop steps keep, the one stack kind this crate interprets because the frame is
//! geometry.

use crate::image_adjust_model::AdjustEntry;

/// The stored kind of a Crop step.
pub const CROP_KIND: &str = "crop";

/// A rectangle of the picture's unit square, y down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitRect {
    /// The left edge, 0 to 1.
    pub left: f64,
    /// The top edge, 0 to 1.
    pub top: f64,
    /// The width, above 0.
    pub width: f64,
    /// The height, above 0.
    pub height: f64,
}

impl UnitRect {
    /// The whole picture.
    pub const WHOLE: Self = Self {
        left: 0.0,
        top: 0.0,
        width: 1.0,
        height: 1.0,
    };

    /// The four corners in the unit square: top-left, top-right, bottom-right, bottom-left.
    #[must_use]
    pub fn corners(self) -> [(f64, f64); 4] {
        let (r, b) = (self.left + self.width, self.top + self.height);
        [(self.left, self.top), (r, self.top), (r, b), (self.left, b)]
    }

    /// The part `pct` (left, top, width, height in % of this rectangle) of this rectangle.
    fn part(self, [l, t, w, h]: [f64; 4]) -> Self {
        Self {
            left: self.left + self.width * l / 100.0,
            top: self.top + self.height * t / 100.0,
            width: self.width * w / 100.0,
            height: self.height * h / 100.0,
        }
    }
}

/// The fields of a stored Crop entry when they read as the laser extension reads them: left and
/// top 0 to 99 %, width and height 1 to 100 %, left + width and top + height at most 100.
fn crop_pct(entry: &AdjustEntry) -> Option<[f64; 4]> {
    let step = entry.step()?;
    if step.kind != CROP_KIND || !step.enabled {
        return None;
    }
    let field = |key: &str, min: i64, max: i64| {
        step.fields
            .get(key)
            .copied()
            .filter(|v| (min..=max).contains(v))
    };
    let (l, t) = (field("left_pct", 0, 99)?, field("top_pct", 0, 99)?);
    let (w, h) = (field("width_pct", 1, 100)?, field("height_pct", 1, 100)?);
    if l + w > 100 || t + h > 100 {
        return None;
    }
    // invariant: whole percents, exact in f64.
    #[allow(clippy::cast_precision_loss)]
    let pct = [l, t, w, h].map(|v| v as f64);
    Some(pct)
}

/// The visible part of a picture with the stack `steps`: each switched-on Crop that reads keeps
/// its part of what the crops before it kept, in stored order; the whole picture without one.
/// The drawn frame can differ from the cropped pixels by less than a pixel (the run floors and
/// ceils to whole pixels, `curvyo-image-core`'s `crop_rect`).
#[must_use]
pub fn visible_rect(steps: &[AdjustEntry]) -> UnitRect {
    steps
        .iter()
        .filter_map(crop_pct)
        .fold(UnitRect::WHOLE, UnitRect::part)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::image_adjust_model::AdjustStep;

    fn crop(enabled: bool, [l, t, w, h]: [i64; 4]) -> AdjustEntry {
        AdjustEntry::Step(AdjustStep {
            kind: CROP_KIND.to_owned(),
            enabled,
            fields: BTreeMap::from(
                [
                    ("left_pct", l),
                    ("top_pct", t),
                    ("width_pct", w),
                    ("height_pct", h),
                ]
                .map(|(k, v)| (k.to_owned(), v)),
            ),
        })
    }

    #[test]
    fn crops_compose_and_off_or_damaged_ones_keep_the_whole() {
        assert_eq!(visible_rect(&[]), UnitRect::WHOLE);
        let one = visible_rect(&[crop(true, [20, 10, 60, 50])]);
        assert_eq!(
            one,
            UnitRect {
                left: 0.2,
                top: 0.1,
                width: 0.6,
                height: 0.5
            }
        );
        let two = visible_rect(&[crop(true, [20, 10, 60, 50]), crop(true, [50, 0, 50, 100])]);
        assert!((two.left - 0.5).abs() < 1e-12 && (two.width - 0.3).abs() < 1e-12);
        assert_eq!(
            visible_rect(&[crop(false, [20, 10, 60, 50])]),
            UnitRect::WHOLE
        );
        assert_eq!(
            visible_rect(&[crop(true, [50, 0, 60, 100])]),
            UnitRect::WHOLE,
            "past the edge"
        );
    }
}
