//! The read model of a text on path (ADR 0022 §1, `specs/0080-text-on-path/adrs.md` decision 1):
//! the reference to its carrier, its placement values on it and the outline it sits on.

use serde::Serialize;

use crate::path_model::NodeId;
use crate::primitive_outline::OutlineAnchor;
use crate::units::Length;

/// Which carrier and which of its outlines a text sits on (`text_carrier`, one atomic value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CarrierRef {
    /// The carrier node.
    pub node: NodeId,
    /// The outline of a compound path, in storage order; 0 for every other carrier.
    pub outline: u32,
}

/// On which side of the carrier direction the letters lie (spec "Terms", Side).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathSide {
    /// On the left of the carrier direction (outside a clockwise circle); absent key.
    #[default]
    Above,
    /// On the right (`text_path_side = "below"`).
    Below,
}

impl PathSide {
    /// The other side.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::Above => Self::Below,
            Self::Below => Self::Above,
        }
    }
}

/// Which line of the text touches the carrier (spec "Terms", Line on path).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LineOnPath {
    /// The baseline; absent key.
    #[default]
    Baseline,
    /// The top of the capital H, Size above the baseline (`text_path_line = "top"`).
    TopEdge,
}

impl LineOnPath {
    /// The other line.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::Baseline => Self::TopEdge,
            Self::TopEdge => Self::Baseline,
        }
    }
}

/// An outline in document mm: what a text sits on, read from its carrier or its kept copy.
#[derive(Debug, Clone, PartialEq)]
pub struct CarrierCurve {
    /// Whether a segment joins the last node to the first.
    pub closed: bool,
    /// The nodes in carrier direction.
    pub anchors: Vec<OutlineAnchor>,
    /// Whether the carrier is an ellipse or an arc of one (Position reads degrees, criterion
    /// 17; the rotate handles move the text, criterion 20).
    pub ellipse: Option<EllipseCarrier>,
}

/// The frame of an ellipse carrier, for the degree readout and the rotation (criteria 17, 20).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct EllipseCarrier {
    /// The centre, document mm.
    pub centre: crate::units::Point,
    /// The half axes.
    pub rx: f64,
    /// The half axes.
    pub ry: f64,
    /// The ellipse's own rotation, radians, clockwise on screen.
    pub rotation: f64,
}

/// The placement values of a text on its carrier, as stored (criterion 17).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct OnPathValues {
    /// The side the letters lie on.
    pub side: PathSide,
    /// The line that touches the carrier.
    pub line: LineOnPath,
    /// The arc length from the carrier start to the text's middle (customer 2026-10-24).
    pub position: Length,
    /// The gap between the carrier and the line on path, positive towards the letters.
    pub distance: Length,
    /// Every letter mirrored about its own axis, the reading order reversed (criterion 19).
    pub mirror_letters: bool,
}

impl OnPathValues {
    /// The defaults of criterion 5 at `position`.
    #[must_use]
    pub const fn at(position: Length) -> Self {
        Self {
            side: PathSide::Above,
            line: LineOnPath::Baseline,
            position,
            distance: Length::from_mm(0.0),
            mirror_letters: false,
        }
    }

    /// Whether the text reads in carrier direction: Above with Baseline, Below with Top edge.
    #[must_use]
    pub fn reads_with_carrier(&self) -> bool {
        matches!(
            (self.side, self.line),
            (PathSide::Above, LineOnPath::Baseline) | (PathSide::Below, LineOnPath::TopEdge)
        )
    }

    /// These values mirrored across the carrier (criterion 18): both Side and Line switch, so the
    /// text crosses the path and keeps its reading direction.
    #[must_use]
    pub const fn mirrored(self) -> Self {
        Self {
            side: self.side.other(),
            line: self.line.other(),
            ..self
        }
    }
}

/// A text on its carrier as read (`TextSnapshot::on_path`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextOnPath {
    /// The carrier as stored.
    pub carrier: CarrierRef,
    /// The placement values.
    pub values: OnPathValues,
    /// The outline the text sits on: its live carrier's, else its kept copy; `None` when neither
    /// is readable, and the text then draws straight at its stored placement (criterion 35).
    #[serde(skip)]
    pub curve: Option<std::sync::Arc<CarrierCurve>>,
    /// Whether `curve` is the kept copy of a deleted carrier (criterion 25).
    pub kept: bool,
    /// The kept copy as stored, if any (written when the carrier was removed).
    #[serde(skip)]
    pub kept_curve: Option<std::sync::Arc<CarrierCurve>>,
}

/// The step an on-path change is (criteria 16 to 19, 21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnPathLabel {
    /// "Change text side".
    Side,
    /// "Change line on path".
    Line,
    /// "Move text on path".
    Move,
    /// "Change distance on path".
    Distance,
    /// "Mirror text on path".
    Mirror,
    /// "Flip": Mirror letters toggled (criterion 19).
    Flip,
    /// "Change text on path": several values at once from the handle's entry box (customer
    /// 2026-10-24).
    Values,
}

impl OnPathLabel {
    /// The commit label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Side => "change_text_side",
            Self::Line => "change_line_on_path",
            Self::Move => "move_text_on_path",
            Self::Distance => "change_distance_on_path",
            Self::Mirror => "mirror_text_on_path",
            Self::Flip => "flip_letters",
            Self::Values => "change_text_on_path",
        }
    }
}

/// The commit label of "Put text on path".
pub const PUT_ON_PATH_LABEL: &str = "put_text_on_path";
/// The commit label of "Release from path".
pub const RELEASE_FROM_PATH_LABEL: &str = "release_from_path";
