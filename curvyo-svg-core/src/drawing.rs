//! What the reader gives back: the page, the display unit, the background and the tree of groups
//! and leaves, every length in millimetres (ADR 0018 §1).

use kurbo::{Affine, BezPath, Point, Rect, Shape};

/// A colour without alpha, 8 bits per channel.
pub type Rgb = [u8; 3];

/// One drawn SVG, read.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgDrawing {
    /// The page in millimetres, clamped to 1 to 100,000 mm per side; `None` when the file gives
    /// no size, no viewBox and draws nothing (the size of a new document applies).
    pub page: Option<(f64, f64)>,
    /// `inkscape:document-units` when it is `mm`, `cm` or `in`.
    pub display_unit: Option<DisplayUnitName>,
    /// The page colour of `sodipodi:namedview`, when the file gives one.
    pub background: Option<PageBackground>,
    /// The top-level nodes in z-order, first at the bottom.
    pub nodes: Vec<SvgNode>,
    /// Whether a limit stopped the reading (everything before it is kept).
    pub truncated: bool,
}

/// A display unit the file names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayUnitName {
    /// Millimetres.
    Mm,
    /// Centimetres.
    Cm,
    /// Inches.
    In,
}

/// The page colour of the file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageBackground {
    /// No background (opacity 0 or none stated).
    None,
    /// A colour with its opacity.
    Colour(Rgb, f64),
}

/// The node registers every node carries.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeState {
    /// `inkscape:label` as written (or a layer's `id` or "Layer N"); not cleaned.
    pub name: Option<String>,
    /// `display:none` (or, on a leaf, `visibility:hidden`).
    pub hidden: bool,
    /// `sodipodi:insensitive="true"`.
    pub locked: bool,
    /// `opacity`, 0 to 1.
    pub opacity: f64,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            name: None,
            hidden: false,
            locked: false,
            opacity: 1.0,
        }
    }
}

/// One node of the tree.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)] // leaves are most of the tree; a box would cost one allocation each
pub enum SvgNode {
    /// A group or, with `layer`, an Inkscape layer at the top level.
    Group {
        /// Its registers.
        state: NodeState,
        /// Whether it is a layer.
        layer: bool,
        /// Its children in z-order.
        children: Vec<SvgNode>,
    },
    /// A drawn object.
    Leaf {
        /// Its registers.
        state: NodeState,
        /// Its geometry and style.
        leaf: SvgLeaf,
    },
}

/// A drawn object: geometry and the style resolved for it.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgLeaf {
    /// The geometry.
    pub kind: LeafKind,
    /// The style; a clone shows its original's and ignores this one.
    pub style: LeafStyle,
}

/// The geometry of a leaf. Primitives keep their own numbers (user units) and the matrix from
/// user units to millimetres, so the caller decides whether they stay primitives.
#[derive(Debug, Clone, PartialEq)]
pub enum LeafKind {
    /// A `<path>`, `<line>`, `<polyline>` or `<polygon>` in millimetres, transforms applied.
    Path {
        /// The outlines, as written.
        path: BezPath,
        /// `sodipodi:nodetypes`, when present.
        node_types: Option<String>,
    },
    /// A `<rect>`; `rx` and `ry` are clamped to half the sides.
    Rect {
        /// The rectangle in user units.
        rect: Rect,
        /// The corner radii in user units.
        rx: f64,
        /// The corner radii in user units.
        ry: f64,
        /// User units to millimetres.
        matrix: Affine,
    },
    /// A `<circle>` or `<ellipse>`.
    Ellipse {
        /// The centre in user units.
        center: Point,
        /// The radius along x in user units.
        rx: f64,
        /// The radius along y in user units.
        ry: f64,
        /// User units to millimetres.
        matrix: Affine,
    },
    /// An Inkscape star or polygon (`sodipodi:type="star"`), with its drawn path.
    Star {
        /// The star numbers in user units.
        star: StarData,
        /// The drawn `d` in millimetres.
        path: BezPath,
        /// `sodipodi:nodetypes`, when present.
        node_types: Option<String>,
        /// User units to millimetres.
        matrix: Affine,
    },
    /// A `<use>` that is a linked clone of the leaf with this index (document order of all
    /// leaves, clones included), with the clone's matrix in millimetres.
    Clone {
        /// The original's leaf index.
        original: usize,
        /// The clone matrix M.
        matrix: Affine,
    },
}

/// The numbers of Inkscape's star tool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StarData {
    /// `sodipodi:sides`.
    pub sides: u32,
    /// The centre.
    pub center: Point,
    /// The outer radius `sodipodi:r1`.
    pub r1: f64,
    /// The inner radius `sodipodi:r2`.
    pub r2: f64,
    /// The angle of the first outer vertex `sodipodi:arg1`, radians.
    pub arg1: f64,
    /// `inkscape:flatsided`: a polygon.
    pub flat: bool,
}

/// One paint: a colour and its opacity (the property's opacity times the colour's alpha).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Paint {
    /// The colour.
    pub rgb: Rgb,
    /// The opacity, 0 to 1.
    pub opacity: f64,
}

/// A line cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cap {
    /// `butt`.
    Butt,
    /// `round`.
    Round,
    /// `square`.
    Square,
}

/// A line join.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Join {
    /// `miter` (also `miter-clip` and `arcs`).
    Miter,
    /// `round`.
    Round,
    /// `bevel`.
    Bevel,
}

/// The style of a leaf, resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct LeafStyle {
    /// The fill, `None` when it is off.
    pub fill: Option<Paint>,
    /// `fill-rule: evenodd`.
    pub even_odd: bool,
    /// The stroke, `None` when it is off.
    pub stroke: Option<Paint>,
    /// The stroke width in millimetres: always with a stroke, and when the file states one.
    pub stroke_width_mm: Option<f64>,
    /// The dash list in multiples of the stroke width; empty is solid.
    pub dash: Vec<f64>,
    /// The cap, when the file states one.
    pub cap: Option<Cap>,
    /// The join, when the file states one.
    pub join: Option<Join>,
}

impl SvgNode {
    /// The registers of this node.
    #[must_use]
    pub const fn state(&self) -> &NodeState {
        match self {
            Self::Group { state, .. } | Self::Leaf { state, .. } => state,
        }
    }
}

impl LeafKind {
    /// The geometry in millimetres as one path (primitives through their matrix); a clone has
    /// none of its own.
    #[must_use]
    pub fn outline_mm(&self) -> Option<BezPath> {
        match self {
            Self::Path { path, .. } | Self::Star { path, .. } => Some(path.clone()),
            Self::Rect {
                rect,
                rx,
                ry,
                matrix,
            } => Some(*matrix * rounded_rect(*rect, *rx, *ry)),
            Self::Ellipse {
                center,
                rx,
                ry,
                matrix,
            } => Some(*matrix * kurbo::Ellipse::new(*center, (*rx, *ry), 0.0).to_path(1e-9)),
            Self::Clone { .. } => None,
        }
    }
}

/// A rectangle with elliptical corners of radii `rx`, `ry` as a path: clockwise on the screen
/// from the start of the top edge, each corner one quarter of an ellipse in two cubics.
#[must_use]
pub fn rounded_rect(rect: Rect, rx: f64, ry: f64) -> BezPath {
    if rx <= 0.0 || ry <= 0.0 {
        return rect.to_path(0.0);
    }
    let corners = [
        (Point::new(rect.x1 - rx, rect.y0 + ry), 1.5),
        (Point::new(rect.x1 - rx, rect.y1 - ry), 0.0),
        (Point::new(rect.x0 + rx, rect.y1 - ry), 0.5),
        (Point::new(rect.x0 + rx, rect.y0 + ry), 1.0),
    ];
    let at = |center: Point, turn: f64| {
        let (sin, cos) = (turn * std::f64::consts::PI).sin_cos();
        Point::new(center.x + rx * cos, center.y + ry * sin)
    };
    let mut path = BezPath::new();
    path.move_to(Point::new(rect.x0 + rx, rect.y0));
    for (center, turn) in corners {
        path.line_to(at(center, turn));
        let arc = kurbo::Arc::new(
            center,
            (rx, ry),
            turn * std::f64::consts::PI,
            std::f64::consts::FRAC_PI_2,
            0.0,
        );
        for (c1, c2, p) in crate::path_data::eighth_turn_cubics(&arc, at(center, turn + 0.5)) {
            path.curve_to(c1, c2, p);
        }
    }
    path.close_path();
    path
}
