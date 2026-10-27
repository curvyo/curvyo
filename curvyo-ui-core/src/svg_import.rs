//! An SVG drawing as document content (`specs/0024-svg-import-export`, ADR 0018 §2): the reader's
//! tree as groups, layers, paths, primitives, linked clones, texts and images, for Open (at the
//! file's own coordinates, layers kept) or for Import SVG… (moved as one onto a point, layers as
//! groups). Texts need faces (`adrs.md` note 2026-10-27, point 3): phase one,
//! [`svg_faces_needed`], names the listed faces to load; phase two, [`svg_import`], converts
//! with them loaded.

use std::sync::Arc;

use curvyo_document_core::{
    Affine, AssetHash, BackgroundPaint, Color, DisplayUnit, DocumentBackground, DocumentSize,
    DrawingNode, DrawingPage, DrawingRegisters, MAX_GROUP_DEPTH, Opacity, Point, TEXT_NAME_CHARS,
    clean_name, validated_document_side,
};
use curvyo_image_core::WorkingImage;
use curvyo_svg_core::{
    DisplayUnitName, InkscapeObject, LeafKind, NodeState, PageBackground, SvgDrawing, SvgNode,
};

use crate::svg_font_match::FaceMemo;
use crate::svg_import_images::{ImportAssets, frame_bounds};
use crate::svg_import_shapes::{LeafGeometry, leaf_geometry};
use crate::svg_import_style::{model_style, rewind_even_odd};
use crate::svg_text::{TextPiece, convert_text};
use crate::svg_text_layout::ImportFaces;

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
#[derive(Debug, Clone)]
pub struct SvgImport {
    /// The page, display unit and background for Open (criteria 15 to 20).
    pub page: DrawingPage,
    /// The content in z-order.
    pub nodes: Vec<DrawingNode>,
    /// How many objects came in; 0 means nothing could be imported (criterion 8).
    pub leaves: usize,
    /// The bytes the images name, by asset number (criterion 104).
    pub assets: Vec<Arc<[u8]>>,
    /// The decoded pictures of those assets, for the canvas.
    pub working: Vec<(AssetHash, Arc<WorkingImage>)>,
}

/// The texts and images of a drawing converted in the file's millimetres, in tree order.
enum Converted {
    /// The pieces and the whole text's string, lines joined by spaces (for a group's name).
    Text(Vec<TextPiece>, String),
    Image(Option<(usize, kurbo::Affine)>),
}

/// Phase two: the document content of `drawing` placed by `placement`, its texts in `fonts`, its
/// images within `asset_room` bytes of the project's limit.
#[must_use]
pub fn svg_import(
    drawing: &SvgDrawing,
    placement: SvgPlacement,
    fonts: &ImportFaces<'_>,
    asset_room: u64,
) -> SvgImport {
    let mut assets = ImportAssets::new(asset_room);
    let mut converted = Vec::new();
    let mut bounds = drawing.bounds();
    convert_all(
        &drawing.nodes,
        (fonts, &FaceMemo::new(fonts.faces)),
        &mut assets,
        &mut converted,
        &mut bounds,
    );
    let (matrix, max_groups) = match placement {
        SvgPlacement::File => (kurbo::Affine::IDENTITY, MAX_GROUP_DEPTH),
        SvgPlacement::CentredOn(target) => {
            let shift = bounds.map_or(kurbo::Vec2::ZERO, |b| {
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
    let mut next = converted.into_iter();
    mapper.map(&drawing.nodes, 0, &mut next, &mut nodes);
    SvgImport {
        page: svg_page(drawing),
        leaves: count_objects(&nodes),
        nodes,
        assets: assets.bytes,
        working: assets.working,
    }
}

/// Converts every text and image below `nodes`, widening `bounds` by their boxes (criterion 106).
fn convert_all(
    nodes: &[SvgNode],
    fonts: (&ImportFaces<'_>, &FaceMemo<'_>),
    assets: &mut ImportAssets,
    out: &mut Vec<Converted>,
    bounds: &mut Option<kurbo::Rect>,
) {
    let widen = |bounds: &mut Option<kurbo::Rect>, b: kurbo::Rect| {
        *bounds = Some(bounds.map_or(b, |all| all.union(b)));
    };
    for node in nodes {
        match node {
            SvgNode::Group { children, .. } => convert_all(children, fonts, assets, out, bounds),
            SvgNode::Text { text, .. } => {
                let pieces = convert_text(text, fonts.0, fonts.1);
                for b in pieces.iter().filter_map(|p| p.bounds) {
                    widen(bounds, b);
                }
                let whole = text
                    .chars
                    .iter()
                    .map(|c| if c.ch == '\n' { ' ' } else { c.ch })
                    .collect();
                out.push(Converted::Text(pieces, whole));
            }
            SvgNode::Image { image, .. } => {
                let added = assets.add(image);
                if let Some((_, map)) = added {
                    widen(bounds, frame_bounds(map));
                }
                out.push(Converted::Image(added));
            }
            SvgNode::Leaf { .. } => {}
        }
    }
}

/// How many objects `nodes` hold (leaves, texts and images; a split text counts once per
/// piece).
fn count_objects(nodes: &[DrawingNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            DrawingNode::Group { children, .. } => count_objects(children),
            _ => 1,
        })
        .sum()
}

/// Reads the SVG `bytes` and maps them by `placement` without faces: texts are left out (a
/// drawing without text needs none), images come in.
///
/// # Errors
/// The error dialog's sentence when the file gives no drawing (criterion 6).
pub fn import_svg_bytes(bytes: &[u8], placement: SvgPlacement) -> Result<SvgImport, String> {
    let drawing = read_svg_drawing(bytes)?;
    let store = curvyo_text_core::FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    Ok(svg_import(&drawing, placement, &fonts, u64::MAX))
}

/// Reads the SVG `bytes`.
///
/// # Errors
/// The error dialog's sentence when the file gives no drawing (criterion 6).
pub fn read_svg_drawing(bytes: &[u8]) -> Result<SvgDrawing, String> {
    curvyo_svg_core::read_svg(bytes).map_err(crate::svg_refusal_text)
}

/// The page settings of `drawing`: its page (a new document's size when it gives none), its
/// display unit and its background.
#[must_use]
pub fn svg_page(drawing: &SvgDrawing) -> DrawingPage {
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
        guide: state.guide,
    }
}

/// The name of a leaf without a label after the Inkscape object it is (criterion 121): "Arc"
/// for an arc kept as an ellipse arc, "Spiral", "Face N" for the `face`-th face of a 3D box;
/// `None` takes the default name.
fn inkscape_name(
    state: &NodeState,
    kind: &LeafKind,
    geometry: Option<&LeafGeometry>,
    face: usize,
) -> Option<String> {
    let arc = matches!(
        (kind, geometry),
        (
            LeafKind::Arc { .. },
            Some(LeafGeometry::Primitive(
                curvyo_document_core::Shape::Ellipse { arc: Some(_), .. },
                _
            ))
        )
    );
    match state.inkscape {
        _ if arc => Some("Arc".to_owned()),
        Some(InkscapeObject::Spiral) => Some("Spiral".to_owned()),
        Some(InkscapeObject::Box3dSide) => Some(format!("Face {face}")),
        _ => None,
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
    fn map(
        &self,
        nodes: &[SvgNode],
        depth: usize,
        next: &mut impl Iterator<Item = Converted>,
        out: &mut Vec<DrawingNode>,
    ) {
        let mut faces = 0;
        for (index, node) in nodes.iter().enumerate() {
            match node {
                SvgNode::Group {
                    state,
                    layer,
                    children,
                } => {
                    if depth >= self.max_groups {
                        self.map(children, depth, next, out);
                        continue;
                    }
                    let mut inner = Vec::new();
                    self.map(children, depth + 1, next, &mut inner);
                    let layer = *layer && self.keep_layers && depth == 0;
                    if inner.is_empty() && !layer {
                        continue;
                    }
                    let mut registers = registers(state);
                    if layer && registers.name.is_none() {
                        registers.name = Some(format!("Layer {}", index + 1));
                    }
                    if state.inkscape == Some(InkscapeObject::Box3d) && registers.name.is_none() {
                        registers.name = Some("3D box".to_owned());
                    }
                    out.push(DrawingNode::Group {
                        registers,
                        layer,
                        children: inner,
                    });
                }
                SvgNode::Leaf { state, leaf } => {
                    let mut registers = registers(state);
                    if state.inkscape == Some(InkscapeObject::Box3dSide) {
                        faces += 1;
                    }
                    let geometry = leaf_geometry(&leaf.kind, self.matrix);
                    if registers.name.is_none() {
                        registers.name = inkscape_name(state, &leaf.kind, geometry.as_ref(), faces);
                    }
                    let style = model_style(&leaf.style);
                    out.push(match (&leaf.kind, geometry) {
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
                SvgNode::Text { state, .. } => {
                    if let Some(Converted::Text(pieces, whole)) = next.next() {
                        self.text(state, pieces, &whole, depth, out);
                    }
                }
                SvgNode::Image { state, .. } => {
                    if let Some(Converted::Image(Some((asset, map)))) = next.next() {
                        let mut registers = registers(state);
                        if registers.name.is_none() {
                            registers.name = Some("Image".to_owned());
                        }
                        out.push(DrawingNode::Image {
                            registers,
                            asset,
                            placement: self.placed(map),
                        });
                    }
                }
            }
        }
    }

    /// A map of the file's millimetres after the placement.
    fn placed(&self, map: kurbo::Affine) -> Affine {
        Affine::from_array((self.matrix * map).as_coeffs())
    }

    /// The pieces of one text: one text object, or a group of them named by the whole text's
    /// first characters (criterion 92); the text's registers go on the object or the group.
    fn text(
        &self,
        state: &NodeState,
        pieces: Vec<TextPiece>,
        whole: &str,
        depth: usize,
        out: &mut Vec<DrawingNode>,
    ) {
        let mut nodes: Vec<DrawingNode> = pieces
            .into_iter()
            .map(|piece| {
                let mut text = piece.text;
                text.placement = Affine::from_array(
                    (self.matrix * kurbo::Affine::new(text.placement.to_array())).as_coeffs(),
                );
                match piece.on_path {
                    Some((carrier, values)) => DrawingNode::TextOnPath {
                        registers: DrawingRegisters::default(),
                        text,
                        carrier,
                        values,
                    },
                    None => DrawingNode::Text {
                        registers: DrawingRegisters::default(),
                        text,
                    },
                }
            })
            .collect();
        let registers = registers(state);
        match nodes.len() {
            0 => {}
            1 => {
                if let Some(
                    DrawingNode::Text { registers: r, .. }
                    | DrawingNode::TextOnPath { registers: r, .. },
                ) = nodes.first_mut()
                {
                    *r = registers;
                }
                out.append(&mut nodes);
            }
            _ if depth >= self.max_groups => out.append(&mut nodes),
            _ => {
                let mut registers = registers;
                if registers.name.is_none() {
                    let first: String = whole.chars().take(TEXT_NAME_CHARS).collect();
                    registers.name = clean_name(&first);
                }
                out.push(DrawingNode::Group {
                    registers,
                    layer: false,
                    children: nodes,
                });
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
        let store = curvyo_text_core::FaceStore::new();
        let fonts = ImportFaces {
            faces: &[],
            store: &store,
            substitute: None,
        };
        let import = svg_import(
            &drawing,
            SvgPlacement::CentredOn(Point::new(50.0, 40.0)),
            &fonts,
            u64::MAX,
        );
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
