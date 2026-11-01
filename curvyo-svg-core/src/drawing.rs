//! What the reader gives back: the page, the display unit, the background and the tree of groups
//! and leaves, every length in millimetres (ADR 0018 §1).

use kurbo::{Affine, BezPath, Point, Rect, Shape};

use crate::drawing_text::{SvgImage, SvgText};

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
    /// Every `inkscape:page` in file order, in millimetres of the drawing (criterion 129); empty
    /// when the file has none.
    pub pages: Vec<SvgPage>,
    /// Whether the root declares the `curvyo:` profile v1 (`curvyo:format="1"`): the file was
    /// written by Curvyo's export, and a closed outline whose last segment ends on its first node
    /// ends there (the writer's form of a curved closing segment).
    pub profile: bool,
}

/// One `inkscape:page`.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgPage {
    /// `inkscape:label`, when present.
    pub name: Option<String>,
    /// The page in the drawing's millimetres.
    pub rect: Rect,
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
#[allow(clippy::struct_excessive_bools)] // independent registers of the file
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
    /// A carrier a text sits on that SVG does not draw (criterion 94): it becomes a guide.
    pub guide: bool,
    /// The Inkscape object it is, from `sodipodi:type`, when the importer names it after one
    /// (criteria 119 to 121).
    pub inkscape: Option<InkscapeObject>,
    /// A top-level layer with `curvyo:guides="1"` (criterion 144): its children are guides.
    pub guides_layer: bool,
}

/// An Inkscape object that has no primitive and is imported as paths (slice 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InkscapeObject {
    /// `sodipodi:type="spiral"`: a path.
    Spiral,
    /// `sodipodi:type="inkscape:box3d"`: a group of faces.
    Box3d,
    /// `sodipodi:type="inkscape:box3dside"`: one face of a 3D box.
    Box3dSide,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            name: None,
            hidden: false,
            locked: false,
            opacity: 1.0,
            guide: false,
            inkscape: None,
            guides_layer: false,
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
    /// A live text (criterion 78); it takes no leaf number.
    Text {
        /// Its registers.
        state: NodeState,
        /// Its characters and styles.
        text: Box<SvgText>,
    },
    /// An embedded picture (criterion 100); it takes no leaf number.
    Image {
        /// Its registers.
        state: NodeState,
        /// Its bytes and frame.
        image: Box<SvgImage>,
    },
}

/// A drawn object: geometry and the style resolved for it.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgLeaf {
    /// The geometry.
    pub kind: LeafKind,
    /// The style; a clone shows its original's and ignores this one.
    pub style: LeafStyle,
    /// The `curvyo:` values next to it as written (criterion 57); empty without the profile.
    pub shadow: crate::export::Shadow,
    /// Whether `marker-start`, `marker-mid` and `marker-end` reference a marker (criterion 142).
    pub marker_refs: [bool; 3],
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
    /// An Inkscape arc (`sodipodi:type="arc"`), with its drawn path.
    Arc {
        /// The arc numbers in user units.
        arc: ArcData,
        /// The drawn `d` in millimetres, arcs in eighth turns as for a path (criterion 22).
        path: BezPath,
        /// The same `d` with its arcs within 1e-5 mm, for the 0.001 mm check of criterion 111;
        /// `None` when it cannot be read within the point budget.
        fine: Option<BezPath>,
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
    /// The inner radius `sodipodi:r2`, when the file gives one (a polygon ignores it).
    pub r2: Option<f64>,
    /// The angle of the first outer vertex `sodipodi:arg1`, radians.
    pub arg1: f64,
    /// The angle of the first inner vertex `sodipodi:arg2`, radians, when the file gives one.
    pub arg2: Option<f64>,
    /// `inkscape:flatsided`: a polygon.
    pub flat: bool,
}

/// The numbers of Inkscape's arc tool, in user units; each is finite.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcData {
    /// `sodipodi:cx`, `sodipodi:cy`.
    pub center: Point,
    /// `sodipodi:rx`.
    pub rx: f64,
    /// `sodipodi:ry`.
    pub ry: f64,
    /// `sodipodi:start`, radians, when present.
    pub start: Option<f64>,
    /// `sodipodi:end`, radians, when present.
    pub end: Option<f64>,
    /// `sodipodi:arc-type` as written (trimmed), when present.
    pub arc_type: Option<String>,
    /// `sodipodi:open="true"`.
    pub open: bool,
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

impl SvgDrawing {
    /// The bounding box in millimetres of the whole drawing: the true outlines (clones through
    /// their chain), the stroke excluded, hidden objects included; `None` when it has no leaf.
    #[must_use]
    pub fn bounds(&self) -> Option<Rect> {
        crate::read::bounds_of(&self.nodes)
    }

    /// How many objects the drawing has: leaves, texts and images.
    #[must_use]
    pub fn leaf_count(&self) -> usize {
        fn count(nodes: &[SvgNode]) -> usize {
            nodes
                .iter()
                .map(|n| match n {
                    SvgNode::Group { children, .. } => count(children),
                    SvgNode::Leaf { .. } | SvgNode::Text { .. } | SvgNode::Image { .. } => 1,
                })
                .sum()
        }
        count(&self.nodes)
    }
}

impl SvgNode {
    /// The registers of this node.
    #[must_use]
    pub const fn state(&self) -> &NodeState {
        match self {
            Self::Group { state, .. }
            | Self::Leaf { state, .. }
            | Self::Text { state, .. }
            | Self::Image { state, .. } => state,
        }
    }
}

impl SvgNode {
    /// The registers of this node, to change.
    pub fn state_mut(&mut self) -> &mut NodeState {
        match self {
            Self::Group { state, .. }
            | Self::Leaf { state, .. }
            | Self::Text { state, .. }
            | Self::Image { state, .. } => state,
        }
    }
}

impl LeafKind {
    /// The geometry in millimetres as one path (primitives through their matrix); a clone has
    /// none of its own.
    #[must_use]
    pub fn outline_mm(&self) -> Option<BezPath> {
        match self {
            Self::Path { path, .. } | Self::Star { path, .. } | Self::Arc { path, .. } => {
                Some(path.clone())
            }
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
/// from the start of the top right corner, each corner a quarter of an ellipse in two cubics;
/// the closing segment is the top edge, so no node repeats.
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
    for (index, (center, turn)) in corners.into_iter().enumerate() {
        if index == 0 {
            path.move_to(Point::new(rect.x1 - rx, rect.y0));
        } else {
            path.line_to(at(center, turn));
        }
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
