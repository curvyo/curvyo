//! The automatic analysis of a picture written as JSON for the Trace tool's page (`specs/0089`
//! slice F). Host-independent, so ordinary `cargo test` covers it.

use curvyo_ui_core::{
    TraceAnalysis, TraceCapture, TraceImageType, TraceMethod, TraceProperty, TraceReason,
};

use crate::trace_glue::hex;

/// The analysis as `{type, capture, confidence, low, method, threshold, background, tolerance,
/// reasons, properties}`: `type` one of `graphic`, `logo`, `line_drawing`, `sketch`; `capture`
/// one of `digital`, `scan`, `paper_photo`; `method` 0 Brightness or 1 Remove background with
/// its `background` (`#RRGGBB`, empty for a transparent one) and `tolerance`; `reasons` in the
/// order to say them (`pale_colours`, `transparent_background`, `shadow_left_out`,
/// `low_confidence`); `properties` each measured value by name.
#[must_use]
pub fn analysis_json(report: &TraceAnalysis) -> String {
    let (background, tolerance) = match report.proposal.selection {
        TraceMethod::RemoveBackground {
            background,
            tolerance,
        } => (background.map(hex).unwrap_or_default(), Some(tolerance)),
        _ => (String::new(), None),
    };
    let properties: serde_json::Map<String, serde_json::Value> = report
        .properties
        .iter()
        .map(|p| {
            (
                property_name(p.kind).to_string(),
                serde_json::json!(p.value),
            )
        })
        .collect();
    serde_json::json!({
        "type": type_name(report.image_type),
        "capture": capture_name(report.capture),
        "confidence": report.confidence,
        "low": report.low_confidence(),
        "method": report.proposal.selection.index(),
        "threshold": report.proposal.threshold,
        "background": background,
        "tolerance": tolerance,
        "reasons": report.reasons.iter().map(|r| reason_name(*r)).collect::<Vec<_>>(),
        "properties": properties,
    })
    .to_string()
}

fn type_name(kind: TraceImageType) -> &'static str {
    match kind {
        TraceImageType::Graphic => "graphic",
        TraceImageType::Logo => "logo",
        TraceImageType::LineDrawing => "line_drawing",
        TraceImageType::Sketch => "sketch",
    }
}

fn capture_name(capture: TraceCapture) -> &'static str {
    match capture {
        TraceCapture::Digital => "digital",
        TraceCapture::Scan => "scan",
        TraceCapture::PaperPhoto => "paper_photo",
    }
}

fn reason_name(reason: TraceReason) -> &'static str {
    match reason {
        TraceReason::PaleColours => "pale_colours",
        TraceReason::TransparentBackground => "transparent_background",
        TraceReason::ShadowLeftOut => "shadow_left_out",
        TraceReason::LowConfidence => "low_confidence",
    }
}

fn property_name(kind: TraceProperty) -> &'static str {
    match kind {
        TraceProperty::Colours => "colours",
        TraceProperty::Chroma => "chroma",
        TraceProperty::BackgroundUniformity => "background_uniformity",
        TraceProperty::BackgroundGrain => "background_grain",
        TraceProperty::Transparency => "transparency",
        TraceProperty::Noise => "noise",
        TraceProperty::Gradient => "gradient",
        TraceProperty::Midtones => "midtones",
        TraceProperty::PaleContent => "pale_content",
        TraceProperty::Ink => "ink",
        TraceProperty::StraightEdges => "straight_edges",
        TraceProperty::Components => "components",
        TraceProperty::Holes => "holes",
        TraceProperty::ComponentSize => "component_size",
        TraceProperty::StrokeWidth => "stroke_width",
        TraceProperty::TextRows => "text_rows",
        TraceProperty::Shadow => "shadow",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_ui_core::trace_analyse;

    /// A pale yellow panel on white: Remove background with white, its reason and every
    /// property by name.
    #[test]
    fn the_analysis_json_names_the_proposal_and_its_reason() {
        let rgba: Vec<u8> = (0..100 * 100)
            .flat_map(|i| {
                let (x, y) = (i % 100, i / 100);
                if (10..30).contains(&x) && (10..90).contains(&y) {
                    [0, 0, 0, 255]
                } else if (50..90).contains(&x) && (20..80).contains(&y) {
                    [0xFF, 0xF2, 0xA8, 255]
                } else {
                    [255, 255, 255, 255]
                }
            })
            .collect();
        let report = trace_analyse(&rgba, 100, 100).unwrap();
        let json: serde_json::Value = serde_json::from_str(&analysis_json(&report)).unwrap();
        assert_eq!(json["method"], 1);
        assert_eq!(json["background"], "#FFFFFF");
        assert_eq!(json["capture"], "digital");
        assert_eq!(json["reasons"], serde_json::json!(["pale_colours"]));
        assert_eq!(json["low"], false);
        assert_eq!(json["properties"].as_object().unwrap().len(), 17);
        assert!(json["properties"]["pale_content"].as_f64().unwrap() > 0.1);
    }
}
