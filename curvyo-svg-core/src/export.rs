//! What the writer takes (`specs/0024-svg-import-export` slice 4, `adrs.md` export §2): a drawing
//! already in the file's millimetres (the export origin subtracted), with styles resolved, the
//! shadow values the mapping chose and references between elements by key. The writer judges
//! nothing; it only writes.

use std::sync::Arc;

use kurbo::{Affine, BezPath, Point, Rect};

use crate::drawing::{ArcData, Cap, DisplayUnitName, Join, PageBackground, Paint, StarData};
use crate::drawing_text::TextAnchor;

/// Radii this close are one radius: the ellipse is written as a `<circle>` (criterion 55).
pub const CIRCLE_TOLERANCE_MM: f64 = 1e-9;

/// One drawing to write.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgExport {
    /// The root's `width` and `height` in millimetres (the `viewBox` is `0 0 w h`).
    pub size: (f64, f64),
    /// The pages relative to the export origin; written as `inkscape:page` when there are two
    /// or more (criterion 128).
    pub pages: Vec<ExportPage>,
    /// `inkscape:document-units`.
    pub display_unit: DisplayUnitName,
    /// `pagecolor` and `inkscape:pageopacity`.
    pub background: PageBackground,
    /// The top-level nodes in z-order, first at the bottom.
    pub nodes: Vec<ExportNode>,
    /// The leaves of the hidden, locked Guides layer (criterion 143); empty writes no layer.
    pub guides: Vec<ExportNode>,
}

/// One page.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportPage {
    /// Its name.
    pub name: String,
    /// Its rectangle in millimetres, relative to the export origin.
    pub rect: Rect,
}

/// The registers every node carries.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportState {
    /// `inkscape:label`; `None` writes none.
    pub name: Option<String>,
    /// `display:none`.
    pub hidden: bool,
    /// `sodipodi:insensitive="true"`.
    pub locked: bool,
    /// `opacity`, written below 1.
    pub opacity: f64,
    /// The key a `use` or a `textPath` names this node by.
    pub key: Option<usize>,
}

impl Default for ExportState {
    fn default() -> Self {
        Self {
            name: None,
            hidden: false,
            locked: false,
            opacity: 1.0,
            key: None,
        }
    }
}

/// One node of the tree.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportNode {
    /// A group or, with `layer`, a layer.
    Group {
        /// Its registers.
        state: ExportState,
        /// Whether it is a layer.
        layer: bool,
        /// Its children in z-order.
        children: Vec<ExportNode>,
    },
    /// A drawn leaf.
    Leaf {
        /// Its registers.
        state: ExportState,
        /// Its geometry, style and shadow data.
        leaf: Box<ExportLeaf>,
    },
    /// A live text.
    Text {
        /// Its registers.
        state: ExportState,
        /// Its values.
        text: Box<ExportText>,
    },
    /// An embedded picture.
    Image {
        /// Its registers.
        state: ExportState,
        /// Its bytes and map.
        image: Box<ExportImage>,
    },
}

impl ExportNode {
    /// The registers of this node.
    #[must_use]
    pub const fn state(&self) -> &ExportState {
        match self {
            Self::Group { state, .. }
            | Self::Leaf { state, .. }
            | Self::Text { state, .. }
            | Self::Image { state, .. } => state,
        }
    }
}

/// A leaf: geometry, style and the shadow data written next to it.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportLeaf {
    /// The geometry.
    pub kind: ExportKind,
    /// The style; a `use` writes none.
    pub style: ExportStyle,
    /// The `curvyo:` values.
    pub shadow: Shadow,
}

/// What a `<path>` is in the model: decides its id prefix (criterion 53).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathId {
    /// `path`.
    Path,
    /// `compound`.
    Compound,
    /// `rect` (a rectangle that only a path can draw).
    Rect,
    /// `ellipse`.
    Ellipse,
    /// `polygon`.
    Polygon,
    /// `star`.
    Star,
}

/// The geometry of a leaf, in the file's millimetres.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportKind {
    /// A `<path>` with `sodipodi:nodetypes` (empty writes none).
    Path {
        /// The outlines.
        path: BezPath,
        /// One letter per node.
        node_types: String,
        /// Its id prefix.
        id: PathId,
    },
    /// A `<rect>` with one radius, rotated about its centre by `rotation_deg`.
    Rect {
        /// The unrotated rectangle.
        rect: Rect,
        /// The corner radius.
        radius: f64,
        /// The rotation in degrees.
        rotation_deg: f64,
    },
    /// An `<ellipse>`, or a `<circle>` when the radii are equal.
    Ellipse {
        /// The centre.
        center: Point,
        /// The radius along x, unrotated.
        rx: f64,
        /// The radius along y, unrotated.
        ry: f64,
        /// The rotation in degrees.
        rotation_deg: f64,
    },
    /// An Inkscape star or polygon: `sodipodi:type="star"` plus its drawn `d`.
    Star {
        /// The star numbers.
        star: StarData,
        /// The outline.
        path: BezPath,
        /// One letter per node.
        node_types: String,
    },
    /// An Inkscape arc: `sodipodi:type="arc"` plus its drawn `d`, both unrotated.
    Arc {
        /// The arc numbers; `start` and `end` are set.
        arc: ArcData,
        /// The outline, unrotated.
        path: BezPath,
        /// The rotation in degrees about the centre.
        rotation_deg: f64,
    },
    /// A `<use>` of the node with this key, with the clone matrix.
    Use {
        /// The key of the target.
        target: usize,
        /// The matrix.
        matrix: Affine,
    },
}

/// A resolved style.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExportStyle {
    /// The fill; `None` is off.
    pub fill: Option<Paint>,
    /// The stroke; `None` is off.
    pub stroke: Option<ExportStroke>,
}

/// A marker shape SVG draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerShape {
    /// The triangle of `0018`.
    Arrow,
    /// The dot of `0018`.
    Dot,
}

/// A stroke that is on.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportStroke {
    /// Colour and opacity.
    pub paint: Paint,
    /// The width in millimetres.
    pub width_mm: f64,
    /// The dash list in millimetres; empty is solid.
    pub dash_mm: Vec<f64>,
    /// The cap.
    pub cap: Cap,
    /// The join.
    pub join: Join,
    /// `marker-start`.
    pub start: Option<MarkerShape>,
    /// `marker-mid`.
    pub mid: Option<MarkerShape>,
    /// `marker-end`.
    pub end: Option<MarkerShape>,
}

/// The `curvyo:` values of one element (criteria 57, 140, 141, 149); `None` writes nothing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Shadow {
    /// `curvyo:dash`: the dash list in multiples of the width.
    pub dash: Option<Vec<f64>>,
    /// `curvyo:stroke-width`: the exact width in millimetres.
    pub stroke_width: Option<f64>,
    /// `curvyo:radii`: the four corner radii, top left first, clockwise.
    pub radii: Option<[f64; 4]>,
    /// `curvyo:kind`.
    pub kind: Option<ShadowKind>,
    /// `curvyo:frame`: the primitive's numbers, by kind (see `docs/svg-import.md`).
    pub frame: Option<Vec<f64>>,
    /// `curvyo:markers`.
    pub markers: Option<ShadowMarkers>,
}

/// The primitive kind a `<path>` holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowKind {
    /// `rectangle`.
    Rectangle,
    /// `ellipse`.
    Ellipse,
    /// `polygon`.
    Polygon,
    /// `star`.
    Star,
}

impl ShadowKind {
    /// The attribute value.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Polygon => "polygon",
            Self::Star => "star",
        }
    }

    /// The kind an attribute value names.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "rectangle" => Some(Self::Rectangle),
            "ellipse" => Some(Self::Ellipse),
            "polygon" => Some(Self::Polygon),
            "star" => Some(Self::Star),
            _ => None,
        }
    }
}

/// The markers of a stroke as the model stores them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShadowMarkers {
    /// The start slot.
    pub start: Option<MarkerShape>,
    /// The middle slot.
    pub middle: Option<MarkerShape>,
    /// Middle markers at every node (`nodes`) or spaced (`spaced`).
    pub at_nodes: bool,
    /// The spaced count.
    pub count: u32,
    /// The end slot.
    pub end: Option<MarkerShape>,
}

/// A live text (criteria 132 to 135), positions in its own frame.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportText {
    /// Its frame in the file; the identity writes no `transform`.
    pub transform: Affine,
    /// `font-family`.
    pub family: String,
    /// `font-weight`.
    pub weight: u16,
    /// `font-style: italic`.
    pub italic: bool,
    /// `font-stretch` keyword; `None` for normal.
    pub stretch: Option<&'static str>,
    /// `font-size` in millimetres.
    pub font_size: f64,
    /// `text-anchor`.
    pub anchor: TextAnchor,
    /// `letter-spacing` in millimetres; 0 writes none.
    pub letter_spacing: f64,
    /// `word-spacing` in millimetres; 0 writes none.
    pub word_spacing: f64,
    /// `line-height` as a factor of the font size, shortest round-trip text; `None` writes none.
    pub line_height: Option<f64>,
    /// `writing-mode`; `None` writes none (horizontal).
    pub writing_mode: Option<&'static str>,
    /// `text-orientation`; `None` writes none (mixed).
    pub orientation: Option<&'static str>,
    /// Fill and stroke.
    pub style: ExportStyle,
    /// `curvyo:size`: the exact Size in millimetres.
    pub size: f64,
    /// `curvyo:face`: `family|style|hash`.
    pub face: String,
    /// The lines.
    pub lines: Vec<ExportLine>,
}

/// One line of a text: a `tspan`, or a `textPath` on a carrier.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportLine {
    /// The anchor of the line.
    pub x: f64,
    /// The baseline of the line.
    pub y: f64,
    /// The characters.
    pub text: String,
    /// `dx` per character; empty writes none.
    pub dx: Vec<f64>,
    /// `dy` per character; empty writes none.
    pub dy: Vec<f64>,
    /// `rotate` per character in degrees, shortest round-trip text; empty writes none.
    pub rotate: Vec<f64>,
    /// The carrier, for a text on path.
    pub on_path: Option<ExportTextPath>,
}

/// A line on a carrier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExportTextPath {
    /// The key of the carrier.
    pub carrier: usize,
    /// `startOffset` in millimetres.
    pub start_offset: f64,
    /// `side="right"`.
    pub side_right: bool,
}

/// An embedded picture.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportImage {
    /// `image/png`, `image/jpeg` or `image/webp`.
    pub mime: &'static str,
    /// The bytes as stored.
    pub bytes: Arc<[u8]>,
    /// The unit square to the file's millimetres.
    pub placement: Affine,
}
