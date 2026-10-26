//! An SVG drawing as document content (`specs/0024-svg-import-export`, ADR 0018 §2): the reader's
//! tree as groups, layers, paths, primitives and linked clones, for Open (at the file's own
//! coordinates, layers kept) or for Import SVG… (moved as one onto a point, layers as groups).

use curvyo_document_core::{
    Affine, BackgroundPaint, Color, DisplayUnit, DocumentBackground, DocumentSize, DrawingNode,
    DrawingPage, DrawingRegisters, MAX_GROUP_DEPTH, Opacity, Point, clean_name,
    validated_document_side,
};
use curvyo_svg_core::{DisplayUnitName, LeafKind, NodeState, PageBackground, SvgDrawing, SvgNode};

use crate::svg_import_shapes::{LeafGeometry, leaf_geometry};
use crate::svg_import_style::{model_style, rewind_even_odd};

/// Where the drawing goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SvgPlacement {
    /// Open: the file's own coordinates; its layers stay layers.
    File,
    /// Import SVG…: moved as one so the centre of its bounding box lies on this point (criterion
    /// 3); its layers become groups inside the import's layer.
    CentredOn(Point),
}

/// An SVG drawing ready to write.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgImport {
    /// The page, display unit and background for Open (criteria 15 to 20).
    pub page: DrawingPage,
    /// The content in z-order.
    pub nodes: Vec<DrawingNode>,
    /// How many objects came in; 0 means nothing could be imported (criterion 8).
    pub leaves: usize,
}

/// The document content of `drawing` placed by `placement`.
#[must_use]
pub fn svg_import(drawing: &SvgDrawing, placement: SvgPlacement) -> SvgImport {
    let (matrix, max_groups) = match placement {
        SvgPlacement::File => (kurbo::Affine::IDENTITY, MAX_GROUP_DEPTH),
        SvgPlacement::CentredOn(target) => {
            let shift = drawing.bounds().map_or(kurbo::Vec2::ZERO, |b| {
                kurbo::Point::new(target.x, target.y) - b.center()
            });
            // The import's own layer is one group above everything.
            (kurbo::Affine::translate(shift), MAX_GROUP_DEPTH - 1)
        }
    };
    let mapper = Mapper {
        matrix,
        keep_layers: placement == SvgPlacement::File,
        max_groups,
    };
    let mut nodes = Vec::new();
    mapper.map(&drawing.nodes, 0, &mut nodes);
    SvgImport {
        page: page_of(drawing),
        leaves: drawing.leaf_count(),
        nodes,
    }
}

/// Reads the SVG `bytes` and maps them by `placement`.
///
/// # Errors
/// The error dialog's sentence when the file gives no drawing (criterion 6).
pub fn import_svg_bytes(bytes: &[u8], placement: SvgPlacement) -> Result<SvgImport, String> {
    let drawing = curvyo_svg_core::read_svg(bytes).map_err(crate::svg_refusal_text)?;
    Ok(svg_import(&drawing, placement))
}

/// The page settings of `drawing`: its page (a new document's size when it gives none), its
/// display unit and its background.
fn page_of(drawing: &SvgDrawing) -> DrawingPage {
    let size = drawing
        .page
        .and_then(|(w, h)| {
            Some(DocumentSize::new(
                validated_document_side(curvyo_document_core::Length::from_mm(w)).ok()?,
                validated_document_side(curvyo_document_core::Length::from_mm(h)).ok()?,
            ))
        })
        .unwrap_or_default();
    let display_unit = drawing.display_unit.map(|unit| match unit {
        DisplayUnitName::Mm => DisplayUnit::Mm,
        DisplayUnitName::Cm => DisplayUnit::Cm,
        DisplayUnitName::In => DisplayUnit::In,
    });
    let background = drawing.background.map(|paint| match paint {
        PageBackground::None => DocumentBackground {
            paint: BackgroundPaint::None,
            ..DocumentBackground::DEFAULT
        },
        PageBackground::Colour(rgb, opacity) => DocumentBackground {
            paint: BackgroundPaint::Solid,
            color: Color {
                r: rgb[0],
                g: rgb[1],
                b: rgb[2],
            },
            opacity: Opacity::new(opacity).unwrap_or(Opacity::OPAQUE),
        },
    });
    DrawingPage {
        size,
        display_unit,
        background,
    }
}

/// The node registers of an imported node; a layer always has a name.
fn registers(state: &NodeState) -> DrawingRegisters {
    DrawingRegisters {
        name: state.name.as_deref().and_then(clean_name),
        visible: !state.hidden,
        locked: state.locked,
        opacity: Opacity::new(state.opacity).unwrap_or(Opacity::OPAQUE),
    }
}

/// The tree mapping of one import.
struct Mapper {
    /// The placement translation (Import) or the identity (Open).
    matrix: kurbo::Affine,
    keep_layers: bool,
    /// The most groups on one chain; deeper groups merge into their parent (criterion 44).
    max_groups: usize,
}

impl Mapper {
    /// Maps `nodes` at group depth `depth` into `out`. Every leaf of the reader gives exactly one
    /// leaf, in order, so clone numbers stay valid.
    fn map(&self, nodes: &[SvgNode], depth: usize, out: &mut Vec<DrawingNode>) {
        for (index, node) in nodes.iter().enumerate() {
            match node {
                SvgNode::Group {
                    state,
                    layer,
                    children,
                } => {
                    if depth >= self.max_groups {
                        self.map(children, depth, out);
                        continue;
                    }
                    let mut inner = Vec::new();
                    self.map(children, depth + 1, &mut inner);
                    let layer = *layer && self.keep_layers && depth == 0;
                    if inner.is_empty() && !layer {
                        continue;
                    }
                    let mut registers = registers(state);
                    if layer && registers.name.is_none() {
                        registers.name = Some(format!("Layer {}", index + 1));
                    }
                    out.push(DrawingNode::Group {
                        registers,
                        layer,
                        children: inner,
                    });
                }
                SvgNode::Leaf { state, leaf } => {
                    let registers = registers(state);
                    let style = model_style(&leaf.style);
                    out.push(match (&leaf.kind, leaf_geometry(&leaf.kind, self.matrix)) {
                        (LeafKind::Clone { original, matrix }, _) => DrawingNode::Clone {
                            registers,
                            original: *original,
                            matrix: self.clone_matrix(*matrix),
                        },
                        (_, Some(LeafGeometry::Path(mut outlines))) => {
                            if leaf.style.even_odd && leaf.style.fill.is_some() {
                                rewind_even_odd(&mut outlines);
                            }
                            DrawingNode::Path {
                                registers,
                                outlines,
                                style,
                            }
                        }
                        (_, Some(LeafGeometry::Primitive(shape, rotation))) => {
                            DrawingNode::Primitive {
                                registers,
                                shape,
                                rotation,
                                style,
                            }
                        }
                        // invariant: only a clone has no geometry, and it matched above.
                        (_, None) => continue,
                    });
                }
            }
        }
    }

    /// A clone's matrix after the placement T: T·M·T⁻¹, so the clone stays where the file put it
    /// relative to its original (criterion 3).
    fn clone_matrix(&self, m: kurbo::Affine) -> Affine {
        Affine::from_array((self.matrix * m * self.matrix.inverse()).as_coeffs())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use curvyo_document_core::{RectBounds, Shape};

    fn read(text: &str) -> SvgDrawing {
        curvyo_svg_core::read_svg(text.as_bytes()).unwrap()
    }

    #[test]
    fn ac3_the_import_is_centred_on_the_point_and_not_scaled() {
        let drawing = read(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="400mm" height="400mm" viewBox="0 0 400 400">
            <rect x="200" y="200" width="40" height="20"/><circle cx="300" cy="210" r="10"/></svg>"#,
        );
        let import = svg_import(&drawing, SvgPlacement::CentredOn(Point::new(50.0, 40.0)));
        let DrawingNode::Primitive {
            shape: Shape::Rect { bounds, .. },
            ..
        } = &import.nodes[0]
        else {
            panic!()
        };
        assert_eq!(bounds.width.as_mm(), 40.0);
        let DrawingNode::Primitive {
            shape: Shape::Ellipse { frame, .. },
            ..
        } = &import.nodes[1]
        else {
            panic!()
        };
        let left = bounds.origin.x;
        let right = frame.center.x + frame.rx.as_mm();
        assert!((f64::midpoint(left, right) - 50.0).abs() < 1e-9);
        assert!((frame.center.x - bounds.origin.x - 100.0).abs() < 1e-9);
        let RectBounds { origin, .. } = *bounds;
        assert!((origin.y + 10.0 - 40.0).abs() < 1e-9);
    }
}
