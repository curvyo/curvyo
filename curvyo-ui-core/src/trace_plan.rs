//! What Create paths writes (`specs/0025-raster-trace` criteria 13, 14 and 46): per traced
//! image one group "Trace of" and the image name holding one compound path, filled black, no stroke, ready
//! for `Document::insert_drawing_above`.

use curvyo_document_core::{
    AnchorId, Color, DrawingAnchor, DrawingNode, DrawingOutline, DrawingRegisters, Fill, NodeId,
    Opacity, Stroke, Style, clean_name,
};
use kurbo::BezPath;

use crate::AnchorIdMinter;
use crate::svg_import_shapes::{kind_from_handles, outlines};

/// The name of an image that has none (the Layers panel's default).
const IMAGE_DEFAULT_NAME: &str = "Image";

/// One image's result: the image, its name, its outlines in document millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceTarget {
    /// The traced image.
    pub image: NodeId,
    /// The image's own name, if it has one.
    pub name: Option<String>,
    /// The outlines in document millimetres.
    pub outlines: Vec<BezPath>,
}

/// The write of one Create paths.
#[derive(Debug, Clone, PartialEq)]
pub struct TracePlan {
    /// Per image with outlines: the image and the group to put directly above it.
    pub items: Vec<(NodeId, DrawingNode)>,
    /// The anchor ids of all paths, in item order.
    pub anchor_ids: Vec<AnchorId>,
    /// Outlines written.
    pub paths: usize,
    /// Anchors written.
    pub nodes: usize,
}

/// The group name of a trace of `image_name` (criterion 13): "Trace of" and the image name, cleaned and cut
/// to the name limit.
#[must_use]
pub fn trace_group_name(image_name: Option<&str>) -> String {
    let base = image_name
        .and_then(clean_name)
        .unwrap_or_else(|| IMAGE_DEFAULT_NAME.to_string());
    clean_name(&format!("Trace of {base}")).unwrap_or_else(|| "Trace".to_string())
}

/// The style of a black and white result (criterion 13, D6): black fill, opaque, no stroke.
#[must_use]
pub fn trace_style() -> Style {
    Style {
        stroke: Stroke {
            enabled: false,
            ..Stroke::default()
        },
        fill: Fill {
            enabled: true,
            color: Color::BLACK,
            opacity: Opacity::OPAQUE,
        },
    }
}

/// `path`'s closed outlines with the anchor a closing segment ends on merged into the first.
fn trace_outlines(path: &BezPath) -> Vec<DrawingOutline> {
    let mut out = outlines(path, None);
    for outline in &mut out {
        let n = outline.anchors.len();
        if outline.closed && n > 2 {
            let (first, last) = (outline.anchors[0], outline.anchors[n - 1]);
            let close = (first.point.x - last.point.x).abs() < 1e-9
                && (first.point.y - last.point.y).abs() < 1e-9;
            if close {
                outline.anchors.pop();
                let start: &mut DrawingAnchor = &mut outline.anchors[0];
                start.handle_in = last.handle_in;
                start.kind = kind_from_handles(start.handle_in, start.handle_out);
            }
        }
    }
    out.retain(|outline| outline.anchors.len() >= 2);
    out
}

/// The groups of Create paths for `targets` with fresh anchor ids from `minter`; an image whose
/// result has no outline gets no group.
pub fn plan_trace(targets: &[TraceTarget], minter: &mut AnchorIdMinter) -> TracePlan {
    let mut plan = TracePlan {
        items: Vec::new(),
        anchor_ids: Vec::new(),
        paths: 0,
        nodes: 0,
    };
    for target in targets {
        let drawn: Vec<DrawingOutline> = target.outlines.iter().flat_map(trace_outlines).collect();
        if drawn.is_empty() {
            continue;
        }
        let nodes: usize = drawn.iter().map(|o| o.anchors.len()).sum();
        plan.anchor_ids.extend((0..nodes).map(|_| minter.mint()));
        plan.paths += drawn.len();
        plan.nodes += nodes;
        let path = DrawingNode::Path {
            registers: DrawingRegisters::default(),
            outlines: drawn,
            style: trace_style(),
        };
        let group = DrawingNode::Group {
            registers: DrawingRegisters {
                name: Some(trace_group_name(target.name.as_deref())),
                ..DrawingRegisters::default()
            },
            layer: false,
            children: vec![path],
        };
        plan.items.push((target.image, group));
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64) -> BezPath {
        let mut p = BezPath::new();
        p.move_to((x, 0.0));
        p.line_to((x + 4.0, 0.0));
        p.line_to((x + 4.0, 4.0));
        p.line_to((x, 4.0));
        p.close_path();
        p
    }

    fn circle() -> BezPath {
        let mut p = kurbo::Shape::to_path(&kurbo::Circle::new((0.0, 0.0), 5.0), 0.1);
        // kurbo's circle ends its last curve on the start point.
        p.close_path();
        p
    }

    #[test]
    fn one_group_per_image_with_one_compound_path() {
        let mut minter = AnchorIdMinter::new(3);
        let doc = curvyo_document_core::Document::new(1);
        let rect = || {
            doc.create_rect(curvyo_document_core::RectBounds {
                origin: curvyo_document_core::Point::new(0.0, 0.0),
                width: curvyo_document_core::Length::from_mm(1.0),
                height: curvyo_document_core::Length::from_mm(1.0),
            })
        };
        let (first, second) = (rect(), rect());
        let targets = [
            TraceTarget {
                image: first,
                name: Some("logo.png".into()),
                outlines: vec![square(0.0), square(10.0)],
            },
            TraceTarget {
                image: second,
                name: None,
                outlines: vec![],
            },
        ];
        let plan = plan_trace(&targets, &mut minter);
        assert_eq!(plan.items.len(), 1);
        assert_eq!((plan.paths, plan.nodes, plan.anchor_ids.len()), (2, 8, 8));
        let DrawingNode::Group {
            registers,
            children,
            ..
        } = &plan.items[0].1
        else {
            panic!("a group");
        };
        assert_eq!(registers.name.as_deref(), Some("Trace of logo.png"));
        let DrawingNode::Path {
            outlines, style, ..
        } = &children[0]
        else {
            panic!("a path");
        };
        assert_eq!(outlines.len(), 2);
        assert!(outlines.iter().all(|o| o.closed && o.anchors.len() == 4));
        assert!(style.fill.enabled && !style.stroke.enabled);
        assert_eq!(style.fill.color, Color::BLACK);
    }

    #[test]
    fn a_closing_curve_does_not_repeat_its_first_anchor() {
        let drawn = trace_outlines(&circle());
        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].anchors.len(), 4);
        assert_ne!(
            drawn[0].anchors[0].handle_in,
            curvyo_document_core::Vec2::ZERO
        );
        assert!(matches!(
            drawn[0].anchors[0].kind,
            curvyo_document_core::AnchorKind::Symmetric
                | curvyo_document_core::AnchorKind::Asymmetric
        ));
    }

    #[test]
    fn names_are_cleaned_and_cut() {
        assert_eq!(trace_group_name(None), "Trace of Image");
        assert_eq!(trace_group_name(Some("  \u{202e}scan ")), "Trace of scan");
        let long = "x".repeat(300);
        assert_eq!(trace_group_name(Some(&long)).chars().count(), 128);
    }
}
