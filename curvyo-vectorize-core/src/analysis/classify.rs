//! The analysis report of measured tones: the content mask, its shapes, the proposal, the image
//! type, the capture and their confidence.

use super::shapes::{Shapes, shapes};
use super::tones::{Reduced, Tones};
use super::{
    AnalysisReport, Capture, ImageType, LOW_CONFIDENCE, Property, PropertyKind, Proposal, Reason,
    real,
};
use crate::params::{Rgb8, Selection};
use crate::raster::luma;
use crate::select::{DEFAULT_BACKGROUND_TOLERANCE, OPAQUE, gate};

/// The report of `reduced` with its `tones`.
pub(super) fn report(reduced: &Reduced, tones: &Tones) -> AnalysisReport {
    let capture = capture(tones);
    let pale = pale_method(tones);
    let mask = content_mask(reduced, tones, pale.is_some());
    let shapes = shapes(&mask, reduced.width, reduced.height);
    let middle = middle(reduced, tones);
    let f = real(reduced.factor);
    let short = real(reduced.width.min(reduced.height)) * f;
    let (image_type, confidence) = image_type(tones, &shapes, capture, f, short);
    let mut reasons = Vec::new();
    let mut proposal = Proposal {
        selection: Selection::Brightness,
        threshold: tones.threshold,
    };
    if confidence < LOW_CONFIDENCE {
        reasons.push(Reason::LowConfidence);
    } else {
        if let Some((selection, reason)) = pale {
            proposal.selection = selection;
            reasons.push(reason);
        }
        // A wide middle class on a photo of paper is a shadow or dark paper, not strokes.
        if capture == Capture::PaperPhoto
            && middle.share >= 0.03
            && middle.width * f >= 8.0
            && middle.width >= 3.0 * middle.ink_width
        {
            proposal.threshold = tones.lower_cut;
            reasons.push(Reason::ShadowLeftOut);
        }
    }
    AnalysisReport {
        properties: properties(tones, &shapes, f, middle.share),
        image_type,
        capture,
        confidence,
        proposal,
        reasons,
    }
}

/// The capture condition: digital with a transparent background; with paper grain (noise of
/// at least 0.75) a photo of paper when the border is mostly not background or the paper level
/// spreads over 0.12, else a scan; without grain a scan when the border is almost but not
/// entirely background (the sheet's edge or dust along it, 90 to 99.5 %); else digital.
fn capture(tones: &Tones) -> Capture {
    if tones.transparent > 0.02 {
        Capture::Digital
    } else if tones.noise >= 0.75 {
        if tones.border_uniformity < 0.7 || tones.gradient > 0.12 {
            Capture::PaperPhoto
        } else {
            Capture::Scan
        }
    } else if (0.9..0.995).contains(&tones.border_uniformity) {
        Capture::Scan
    } else {
        Capture::Digital
    }
}

/// Remove background with its reason when Brightness misses flat content on an even, quiet
/// background (border at least 90 % background, noise under 0.75): pale content of at least 0.2 % of the picture and a tenth of the dark content. The
/// background colour and a Colour tolerance whose distance is half the palest colour's.
fn pale_method(tones: &Tones) -> Option<(Selection, Reason)> {
    let even = tones.background.is_none() || (tones.border_uniformity >= 0.9 && tones.noise < 0.75);
    if !even || tones.pale < 0.002 || tones.pale < 0.1 * tones.ink {
        return None;
    }
    match tones.background {
        None => Some((
            Selection::RemoveBackground {
                background: None,
                tolerance: DEFAULT_BACKGROUND_TOLERANCE,
            },
            Reason::TransparentBackground,
        )),
        Some(bg) => Some((
            Selection::RemoveBackground {
                background: Some(bg),
                tolerance: half_distance_tolerance(tones.palest),
            },
            Reason::PaleColours,
        )),
    }
}

/// The smallest Colour tolerance whose distance (`0.5 · (t/100)² · 255` bytes) reaches half of
/// `palest`,
/// at least 10.
fn half_distance_tolerance(palest: f64) -> u8 {
    (10..100).find(|&t| gate(t) >= palest / 2.0).unwrap_or(100)
}

/// The content of the reduced copy: darker than the Threshold, or with `pale` unlike the
/// background (every opaque pixel on a transparent one).
fn content_mask(reduced: &Reduced, tones: &Tones, pale: bool) -> Vec<bool> {
    reduced
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, a]| {
            a >= OPAQUE
                && (luma(r, g, b) < tones.threshold
                    || (pale && tones.background.is_none_or(|bg| unlike(r, g, b, bg))))
        })
        .collect()
}

fn unlike(r: u8, g: u8, b: u8, bg: Rgb8) -> bool {
    let d = [
        f64::from(r) - f64::from(bg.r),
        f64::from(g) - f64::from(bg.g),
        f64::from(b) - f64::from(bg.b),
    ];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2] > 15.3 * 15.3
}

/// The middle tone class between the lower cut and the Threshold (a shadow, dark paper, or
/// the anti-aliased rims of the ink): its share of the picture, its mean width and the ink's
/// below the lower cut (reduced px).
struct Middle {
    share: f64,
    width: f64,
    ink_width: f64,
}

fn middle(reduced: &Reduced, tones: &Tones) -> Middle {
    let levels: Vec<Option<u8>> = reduced
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&[r, g, b, a]| (a >= OPAQUE).then(|| luma(r, g, b)))
        .collect();
    let mask: Vec<bool> = levels
        .iter()
        .map(|l| l.is_some_and(|l| l >= tones.lower_cut && l < tones.threshold))
        .collect();
    let ink: Vec<bool> = levels
        .iter()
        .map(|l| l.is_some_and(|l| l < tones.lower_cut))
        .collect();
    Middle {
        share: real(mask.iter().filter(|&&m| m).count()) / real(mask.len()),
        width: shapes(&mask, reduced.width, reduced.height).stroke,
        ink_width: shapes(&ink, reduced.width, reduced.height).stroke,
    }
}

/// How far `value` lies past `limit` on the deciding side, as a share of `limit`, 0 to 1.
fn margin(value: f64, limit: f64) -> f64 {
    ((value - limit).abs() / limit).min(1.0)
}

/// The confidence of a decision with `margin`: 0.6 at its limit, 0.95 at twice (or half) it.
fn sure(margin: f64) -> f64 {
    0.6 + 0.35 * margin
}

/// The image type and its confidence from the tones and the shapes (`f` source px per mask px,
/// `short` the picture's short side in source px), first rule that holds: three or more text
/// rows make a Logo; mid tones with paper grain a Sketch; thin strokes (at most 5 px or 1.2 %
/// of the short side) a Line drawing; colour a Graphic; straight edges a Logo; else a Graphic.
/// A picture that is mostly dark and not digital (a photo, no paper) is unclear.
fn image_type(
    tones: &Tones,
    shapes: &Shapes,
    capture: Capture,
    f: f64,
    short: f64,
) -> (ImageType, f64) {
    if capture != Capture::Digital && tones.ink >= 0.45 {
        return (ImageType::Graphic, 0.3);
    }
    if shapes.text_rows >= 3 {
        return (ImageType::Logo, sure(margin(real(shapes.text_rows), 2.0)));
    }
    if tones.midtones >= 0.1 && tones.noise >= 1.0 {
        let m = margin(tones.midtones, 0.1).min(margin(tones.noise, 1.0));
        return (ImageType::Sketch, sure(m));
    }
    let stroke = shapes.stroke * f;
    let thin = 5.0f64.max(0.012 * short);
    if stroke <= thin {
        return (ImageType::LineDrawing, sure(margin(stroke, thin)));
    }
    let m = margin(stroke, thin);
    if tones.colours >= 3 || tones.chroma >= 0.2 {
        return (ImageType::Graphic, sure(m));
    }
    let kind = if shapes.straight >= 0.35 {
        ImageType::Logo
    } else {
        ImageType::Graphic
    };
    (kind, sure(m.min(margin(shapes.straight, 0.35))))
}

/// The report's properties in [`PropertyKind`] order.
fn properties(tones: &Tones, shapes: &Shapes, f: f64, shadow: f64) -> Vec<Property> {
    let noise_confidence = (real(tones.noise_samples) / 2000.0).min(1.0);
    let opaque = 1.0 - tones.transparent;
    let p = |kind, value, confidence| Property {
        kind,
        value,
        confidence,
    };
    vec![
        p(PropertyKind::Colours, real(tones.colours), opaque),
        p(PropertyKind::Chroma, tones.chroma, opaque),
        p(
            PropertyKind::BackgroundUniformity,
            tones.border_uniformity,
            if tones.background.is_some() { 1.0 } else { 0.0 },
        ),
        p(
            PropertyKind::BackgroundGrain,
            tones.grain,
            if tones.background.is_some() { 1.0 } else { 0.0 },
        ),
        p(PropertyKind::Transparency, tones.transparent, 1.0),
        p(PropertyKind::Noise, tones.noise, noise_confidence),
        p(PropertyKind::Gradient, tones.gradient, opaque),
        p(PropertyKind::Midtones, tones.midtones, opaque),
        p(PropertyKind::PaleContent, tones.pale, opaque),
        p(PropertyKind::Ink, tones.ink, opaque),
        p(PropertyKind::StraightEdges, shapes.straight, opaque),
        p(PropertyKind::Components, real(shapes.components), opaque),
        p(PropertyKind::Holes, real(shapes.holes), opaque),
        p(
            PropertyKind::ComponentSize,
            shapes.component_area * f * f,
            opaque,
        ),
        p(PropertyKind::StrokeWidth, shapes.stroke * f, opaque),
        p(PropertyKind::TextRows, real(shapes.text_rows), opaque),
        p(PropertyKind::Shadow, shadow, opaque),
    ]
}
