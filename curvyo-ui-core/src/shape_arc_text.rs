//! The words of the arc and curve edits (`specs/0021-ellipse-arcs-and-shaping/` criteria
//! 6, 25, 43 and 65): the pointer readout of a drag and the polite announcement after a
//! release or a bar commit.

use curvyo_document_core::{ArcKind, Shape, effective_curve};

use crate::param_entry::shown_arc_angles;
use crate::shape_arc_edit::{ArcEnd, arc_capable};

/// The readout of an arc-handle drag on `shape` as it would commit: "start 30.0° · sweep
/// 120.0°" for the dragged `end` and the resulting sweep (360.0° for a whole ellipse).
#[must_use]
pub fn arc_readout(shape: &Shape, end: ArcEnd) -> Option<String> {
    let (start, stop) = shown_arc_angles(shape)?;
    let sweep = arc_capable(shape)?.map_or(360.0, |arc| arc.sweep().as_radians().to_degrees());
    let (name, angle) = match end {
        ArcEnd::Start => ("start", start),
        ArcEnd::End => ("end", stop),
    };
    Some(format!(
        "{name} {angle:.1}\u{b0} \u{b7} sweep {sweep:.1}\u{b0}"
    ))
}

/// The readout of a curve-handle drag: "curve -35.0 %", the effective value.
#[must_use]
pub fn curve_readout(shape: &Shape) -> Option<String> {
    Some(format!("curve {:.1} %", effective_curve(shape)?.percent()))
}

const fn kind_word(kind: ArcKind) -> &'static str {
    match kind {
        ArcKind::Pie => "pie",
        ArcKind::Arc => "arc",
        ArcKind::Chord => "chord",
    }
}

/// The announcement of `shape`'s arc after an edit: "Arc start 30.0°, end 150.0°, pie",
/// or "Whole ellipse".
#[must_use]
pub fn arc_announcement(shape: &Shape) -> Option<String> {
    let arc = arc_capable(shape)?;
    let (start, end) = shown_arc_angles(shape)?;
    Some(arc.map_or_else(
        || "Whole ellipse".to_string(),
        |arc| {
            format!(
                "Arc start {start:.1}\u{b0}, end {end:.1}\u{b0}, {}",
                kind_word(arc.kind)
            )
        },
    ))
}

/// The announcement of `shape`'s curve after an edit: "Curve −35.0%" (a typographic
/// minus).
#[must_use]
pub fn curve_announcement(shape: &Shape) -> Option<String> {
    let percent = effective_curve(shape)?.percent();
    let text = format!("{percent:.1}");
    Some(format!("Curve {}%", text.replace('-', "\u{2212}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{Angle, Curve, EllipseArc, EllipseFrame, Length, Point};

    fn ellipse(arc: Option<EllipseArc>, curve: f64) -> Shape {
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(0.0, 0.0),
                rx: Length::from_mm(10.0),
                ry: Length::from_mm(10.0),
            },
            arc,
            curve: Curve::new(curve).unwrap(),
        }
    }

    fn deg(v: f64) -> Angle {
        Angle::from_radians(v.to_radians())
    }

    /// Criteria 6, 25 and 65: the readouts and announcements as the spec writes them.
    #[test]
    fn the_words_match_the_specification() {
        let pie = ellipse(EllipseArc::new(deg(30.0), deg(150.0), ArcKind::Pie), 1.0);
        assert_eq!(
            arc_readout(&pie, ArcEnd::Start).unwrap(),
            "start 30.0\u{b0} \u{b7} sweep 120.0\u{b0}"
        );
        assert_eq!(
            arc_announcement(&pie).unwrap(),
            "Arc start 30.0\u{b0}, end 150.0\u{b0}, pie"
        );
        let curved = ellipse(None, -0.35);
        assert_eq!(curve_readout(&curved).unwrap(), "curve -35.0 %");
        assert_eq!(curve_announcement(&curved).unwrap(), "Curve \u{2212}35.0%");
        assert_eq!(
            arc_announcement(&ellipse(None, 1.0)).unwrap(),
            "Whole ellipse"
        );
    }
}
