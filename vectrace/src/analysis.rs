//! The automatic analysis of a picture (`specs/0089-trace-quality-and-colour` slice F, `adrs.md`
//! §9): heuristic and explainable, no model. It measures properties with a confidence, names an
//! image type and a capture condition, and proposes image-dependent values only (the selection
//! method with its background and Colour tolerance, and the Threshold), never a preset, Smooth,
//! Corners, Ignore below, Output or picked colours (Frage 86 A). Global measures run on a reduced
//! copy, the noise on full-resolution crops; the result is deterministic.

mod classify;
mod shapes;
mod tones;

use crate::TraceError;
use crate::params::Selection;
use crate::raster::Raster;

/// A count of pixels, edges or regions as `f64`: a picture holds at most 2^32 pixels (ADR 0023
/// §2), far below the 2^52 an `f64` holds exactly.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn real(n: usize) -> f64 {
    n as f64
}

/// A `u64` count as [`real`] does.
#[allow(clippy::cast_precision_loss)]
pub(crate) fn real64(n: u64) -> f64 {
    n as f64
}

/// Below this confidence the analysis falls back to Brightness at the automatic Threshold and
/// says so (criterion 32).
pub const LOW_CONFIDENCE: f64 = 0.5;

/// What a property measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyKind {
    /// Colour groups of the flat content (count).
    Colours,
    /// Share of the content with a visible chroma (0 to 1).
    Chroma,
    /// Share of the opaque border near the background colour (0 to 1).
    BackgroundUniformity,
    /// Share of the border's background pixels that are not exactly the background colour:
    /// paper grain or scanner dust (0 to 1; 0 for a digital background).
    BackgroundGrain,
    /// Share of pixels that are not opaque (0 to 1).
    Transparency,
    /// Mean luma residual of light flat areas at full resolution (luma units, 0 to 255).
    Noise,
    /// Spread of the paper level across the picture (0 to 1).
    Gradient,
    /// Share of the not-paper pixels inside areas of mid tones (0 to 1).
    Midtones,
    /// Share of the picture that is flat, unlike the background and lighter than the automatic
    /// Threshold: content Brightness misses (0 to 1).
    PaleContent,
    /// Share of the picture darker than the automatic Threshold (0 to 1).
    Ink,
    /// Share of the content boundary on straight row or column runs (0 to 1).
    StraightEdges,
    /// Content regions (count, specks left out).
    Components,
    /// Holes in the content (count, specks left out).
    Holes,
    /// The median content region's area (source px²).
    ComponentSize,
    /// The mean stroke width of the content (source px).
    StrokeWidth,
    /// Rows of letter-like components (count; reported only, `0088` reads text).
    TextRows,
    /// Share of the picture in a wide middle tone class between ink and paper, such as a shadow
    /// (0 to 1).
    Shadow,
}

/// One measured property.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Property {
    /// What it measures.
    pub kind: PropertyKind,
    /// The value in the unit of its kind.
    pub value: f64,
    /// How much of the picture supported it, 0 to 1.
    pub confidence: f64,
}

/// What the picture shows (criterion 31).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageType {
    /// Graphic or clipart: shapes, silhouettes, coloured areas.
    Graphic,
    /// Logo: lettering or a compact sign in few colours.
    Logo,
    /// Line drawing: thin strokes, little area.
    LineDrawing,
    /// Sketch: pencil strokes and shading in mid tones.
    Sketch,
}

/// How the picture was made (criterion 31: a capture condition, not a type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capture {
    /// A digital picture: flat areas without noise, or a transparent background.
    Digital,
    /// A scan of paper: an even light background with grain.
    Scan,
    /// A photo of paper: uneven light, a shadow or a desk around the sheet.
    PaperPhoto,
}

/// Why the proposal differs from Brightness at the automatic Threshold; the host words it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Pale or light colours that Brightness misses are kept: Remove background.
    PaleColours,
    /// The background is transparent: Remove background takes every opaque pixel.
    TransparentBackground,
    /// A shadow or dark paper between ink and paper is left out: the lower Threshold.
    ShadowLeftOut,
    /// The type is unclear: Brightness at the automatic Threshold.
    LowConfidence,
}

/// The image-dependent values the analysis proposes (Frage 86 A).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Proposal {
    /// The method with its background and Colour tolerance.
    pub selection: Selection,
    /// The Threshold of Brightness.
    pub threshold: u8,
}

/// The analysis of one picture.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisReport {
    /// Every measured property, in [`PropertyKind`] order.
    pub properties: Vec<Property>,
    /// The image type (its best guess also under low confidence).
    pub image_type: ImageType,
    /// The capture condition.
    pub capture: Capture,
    /// How sure the type is, 0 to 1; below [`LOW_CONFIDENCE`] the proposal is the fallback.
    pub confidence: f64,
    /// The proposed values.
    pub proposal: Proposal,
    /// Why they differ from Brightness at the automatic Threshold, in the order to say them.
    pub reasons: Vec<Reason>,
}

impl AnalysisReport {
    /// The value of the property `kind`.
    #[must_use]
    pub fn value(&self, kind: PropertyKind) -> Option<f64> {
        self.properties
            .iter()
            .find(|p| p.kind == kind)
            .map(|p| p.value)
    }

    /// Whether the type is unclear and the proposal is the fallback (criterion 32).
    #[must_use]
    pub fn low_confidence(&self) -> bool {
        self.confidence < LOW_CONFIDENCE
    }
}

/// Analyses `raster` (criteria 31 to 34): measures its properties, names its type and capture,
/// and proposes the image-dependent values with their reasons.
///
/// # Errors
/// [`TraceError::Invalid`] when the bytes do not match the size.
pub fn analyse(raster: Raster<'_>) -> Result<AnalysisReport, TraceError> {
    if raster.checked_pixels().is_none() {
        return Err(TraceError::Invalid("the pixels do not match the size"));
    }
    let reduced = tones::reduce(raster);
    let tones = tones::tones(raster, &reduced);
    Ok(classify::report(&reduced, &tones))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::Rgb8;
    use crate::raster::auto_threshold;

    /// A `width` x `height` picture with the colour `paint(x, y)` (RGBA) per pixel.
    fn picture(width: u32, height: u32, paint: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .flat_map(|(x, y)| paint(x, y))
            .collect()
    }

    fn raster(width: u32, height: u32, rgba: &[u8]) -> Raster<'_> {
        Raster {
            width,
            height,
            rgba,
        }
    }

    /// A deterministic grain of 0 to 15 per pixel.
    fn grain(x: u32, y: u32) -> u8 {
        let h =
            (x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663)).wrapping_mul(2_654_435_761);
        u8::try_from(h >> 28).unwrap()
    }

    #[test]
    fn a_picture_whose_bytes_do_not_match_its_size_is_refused() {
        let rgba = vec![0_u8; 10];
        assert!(matches!(
            analyse(raster(4, 4, &rgba)),
            Err(TraceError::Invalid(_))
        ));
    }

    /// Black bars on digital white: Brightness at the automatic Threshold, no reason, digital,
    /// thick shapes; the same report twice (criterion 34).
    #[test]
    fn black_shapes_on_white_keep_brightness_at_the_automatic_threshold() {
        let rgba = picture(200, 200, |x, y| {
            if (40..160).contains(&x) && (60..140).contains(&y) {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            }
        });
        let r = analyse(raster(200, 200, &rgba)).unwrap();
        assert_eq!(r.capture, Capture::Digital);
        assert_eq!(r.proposal.selection, Selection::Brightness);
        assert_eq!(
            r.proposal.threshold,
            auto_threshold(raster(200, 200, &rgba))
        );
        assert_eq!(r.reasons, []);
        assert!(!r.low_confidence());
        assert_eq!(r.value(PropertyKind::Components), Some(1.0));
        assert!(r.value(PropertyKind::StrokeWidth).unwrap() > 40.0);
        assert_eq!(r.properties.len(), 17);
        assert_eq!(analyse(raster(200, 200, &rgba)).unwrap(), r);
    }

    /// A pale yellow panel beside a black bar on white: Brightness misses it, so Remove
    /// background with white and a tolerance at half the yellow's distance (criteria 31, 33).
    #[test]
    fn pale_colours_on_white_propose_remove_background() {
        let rgba = picture(200, 200, |x, y| {
            if (20..60).contains(&x) && (20..180).contains(&y) {
                [0, 0, 0, 255]
            } else if (90..180).contains(&x) && (40..160).contains(&y) {
                [0xFF, 0xF2, 0xA8, 255]
            } else {
                [255, 255, 255, 255]
            }
        });
        let r = analyse(raster(200, 200, &rgba)).unwrap();
        assert_eq!(r.reasons, [Reason::PaleColours]);
        let Selection::RemoveBackground {
            background,
            tolerance,
        } = r.proposal.selection
        else {
            panic!("{:?}", r.proposal.selection);
        };
        assert_eq!(background, Some(Rgb8::new(255, 255, 255)));
        // #FFF2A8 is 88 bytes from white: the gate reaches 44 bytes at 59, not at 58.
        assert_eq!(tolerance, 59);
        assert!(r.value(PropertyKind::PaleContent).unwrap() > 0.2);
    }

    /// On a transparent background a pale coloured shape is content (Remove background takes
    /// every opaque pixel), a white one is a cut-out and keeps Brightness.
    #[test]
    fn a_transparent_background_keeps_white_as_a_cut_out() {
        let paint = |inner: [u8; 4]| {
            move |x: u32, y: u32| {
                if (20..180).contains(&x) && (20..180).contains(&y) {
                    if (70..130).contains(&x) && (70..130).contains(&y) {
                        inner
                    } else {
                        [0, 0, 0, 255]
                    }
                } else {
                    [0, 0, 0, 0]
                }
            }
        };
        let yellow = picture(200, 200, paint([0xFF, 0xE0, 0x40, 255]));
        let r = analyse(raster(200, 200, &yellow)).unwrap();
        assert_eq!(r.reasons, [Reason::TransparentBackground]);
        assert_eq!(
            r.proposal.selection,
            Selection::RemoveBackground {
                background: None,
                tolerance: crate::select::DEFAULT_BACKGROUND_TOLERANCE
            }
        );
        assert!(r.value(PropertyKind::Transparency).unwrap() > 0.3);
        let white = picture(200, 200, paint([255, 255, 255, 255]));
        let r = analyse(raster(200, 200, &white)).unwrap();
        assert_eq!(r.proposal.selection, Selection::Brightness);
        assert_eq!(r.value(PropertyKind::Holes), Some(1.0));
    }

    /// A photographed sheet: a desk around it, grain, a wide shadow over a corner and thin
    /// strokes. The capture is a paper photo and the Threshold drops to the lower cut, below
    /// the shadow (criterion 31: "shadow left out").
    #[test]
    fn a_shadow_on_a_photographed_sheet_is_left_out() {
        let rgba = picture(400, 300, |x, y| {
            let desk = x < 20 || y < 20 || x >= 380 || y >= 280;
            let v = if desk {
                [120, 90, 60]
            } else if x % 40 < 2 || y % 50 < 2 {
                [30, 30, 30]
            } else if x + y > 550 {
                [150, 150, 145]
            } else {
                [225, 225, 220]
            };
            let g = grain(x, y);
            [v[0] + g, v[1] + g, v[2] + g, 255]
        });
        let r = analyse(raster(400, 300, &rgba)).unwrap();
        assert_eq!(r.capture, Capture::PaperPhoto);
        assert_eq!(r.reasons, [Reason::ShadowLeftOut]);
        assert!(r.proposal.threshold < auto_threshold(raster(400, 300, &rgba)));
        // Still above the ink (30 plus up to 15 of grain).
        assert!(r.proposal.threshold > 45, "{}", r.proposal.threshold);
        assert!(r.value(PropertyKind::Noise).unwrap() >= 0.75);
        assert!(r.value(PropertyKind::Shadow).unwrap() > 0.1);
    }

    /// Thin strokes on even, grainy paper: a line drawing, scanned, nothing changed.
    #[test]
    fn thin_strokes_on_grainy_paper_are_a_scanned_line_drawing() {
        let rgba = picture(300, 300, |x, y| {
            let inside = (30..270).contains(&x) && (30..270).contains(&y);
            let ink = inside && (x % 37 < 2 || y % 41 < 2);
            let v = if ink { 20 } else { 230 } + grain(x, y) / 2;
            [v, v, v, 255]
        });
        let r = analyse(raster(300, 300, &rgba)).unwrap();
        assert_eq!(r.image_type, ImageType::LineDrawing);
        assert_eq!(r.capture, Capture::Scan);
        assert_eq!(r.reasons, []);
        assert!(r.value(PropertyKind::StrokeWidth).unwrap() < 5.0);
        assert!(r.value(PropertyKind::StraightEdges).unwrap() > 0.5);
    }

    /// Rows of letter-like blocks make a Logo; text is only counted (criterion 31).
    #[test]
    fn rows_of_letters_make_a_logo() {
        let rgba = picture(400, 300, |x, y| {
            let row = [40, 120, 200]
                .into_iter()
                .find(|top| (*top..top + 20).contains(&y));
            let letter = (x % 24) < 14 && (20..380).contains(&x);
            let counter = (5..9).contains(&(x % 24))
                && row.is_some_and(|top| (top + 5..top + 15).contains(&y));
            if row.is_some() && letter && !counter {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            }
        });
        let r = analyse(raster(400, 300, &rgba)).unwrap();
        assert_eq!(r.value(PropertyKind::TextRows), Some(3.0));
        assert_eq!(r.image_type, ImageType::Logo);
        assert_eq!(r.proposal.selection, Selection::Brightness);
    }

    /// A grainy picture that is mostly dark (a photo, no paper) is unclear: the fallback with
    /// its reason (criterion 32).
    #[test]
    fn a_dark_photo_falls_back_with_low_confidence() {
        let rgba = picture(300, 200, |x, y| {
            let v = u8::try_from((x + y) % 120).unwrap() + 20 + grain(x, y);
            [v, v.saturating_sub(10), v / 2, 255]
        });
        let r = analyse(raster(300, 200, &rgba)).unwrap();
        assert!(r.low_confidence());
        assert_eq!(r.reasons, [Reason::LowConfidence]);
        assert_eq!(r.proposal.selection, Selection::Brightness);
        assert_eq!(
            r.proposal.threshold,
            auto_threshold(raster(300, 200, &rgba))
        );
    }
}
