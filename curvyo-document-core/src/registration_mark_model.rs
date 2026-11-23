//! The registration mark leaf as read from the document (ADR 0081 §1): its number, its exact
//! centre, its size and line width, with the limits and labels of its commands.

use serde::Serialize;

use crate::path_model::NodeId;
use crate::style_model::{Fill, Style};
use crate::units::{Length, Point};

/// The label of Add registration marks; its history name is "Add registration marks".
pub const INSERT_REGISTRATION_MARKS_COMMIT_LABEL: &str = "insert_registration_marks";
/// The label of Add mark n; its history name is "Add registration mark".
pub const ADD_REGISTRATION_MARK_COMMIT_LABEL: &str = "add_registration_mark";
/// The label of a Size or Line width edit; its history name is "Registration mark size".
pub const REGISTRATION_MARK_SIZE_COMMIT_LABEL: &str = "registration_mark_size";

/// The default circle diameter in mm (criterion 2).
pub const MARK_DEFAULT_DIAMETER_MM: f64 = 6.0;
/// The smallest circle diameter in mm.
pub const MARK_MIN_DIAMETER_MM: f64 = 3.0;
/// The largest circle diameter in mm.
pub const MARK_MAX_DIAMETER_MM: f64 = 20.0;
/// The default line width in mm.
pub const MARK_DEFAULT_LINE_MM: f64 = 0.25;
/// The thinnest line in mm.
pub const MARK_MIN_LINE_MM: f64 = 0.05;
/// The thickest line in mm.
pub const MARK_MAX_LINE_MM: f64 = 1.0;
/// How far the crosshair reaches past the circle, in mm in total (1 mm on every side).
pub const MARK_CROSS_EXTRA_MM: f64 = 2.0;
/// The most marks one marks object holds; numbers run 1 to this.
pub const MARK_MAX_COUNT: u8 = 4;
/// The fewest marks one marks object keeps (criterion 3).
pub const MARK_MIN_COUNT: usize = 2;
/// The inset of a mark centre from a page edge and the margin around the artwork box (criterion
/// 1), in mm.
pub const MARK_INSET_MM: f64 = 10.0;

/// One registration mark (ADR 0081 §1): never burnt, always printed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegistrationMarkSnapshot {
    /// The leaf's identity.
    pub id: NodeId,
    /// Its number, 1 to [`MARK_MAX_COUNT`].
    pub number: u8,
    /// The crosshair centre in document mm: what the fit reads.
    pub centre: Point,
    /// The circle's diameter.
    pub diameter: Length,
    /// The line width of circle, cross and digit.
    pub line: Length,
    /// The fixed look: a black stroke of [`RegistrationMarkSnapshot::line`], no fill. Not stored.
    pub style: Style,
}

impl RegistrationMarkSnapshot {
    /// A mark with its fixed look.
    #[must_use]
    pub fn new(id: NodeId, number: u8, centre: Point, diameter: Length, line: Length) -> Self {
        let mut style = Style::default();
        style.stroke.enabled = true;
        style.stroke.width = line;
        style.fill = Fill::default();
        style.fill.enabled = false;
        Self {
            id,
            number,
            centre,
            diameter,
            line,
            style,
        }
    }

    /// The same mark with its centre mapped by `map`; size and line stay (criterion 2).
    #[must_use]
    pub fn mapped_by(&self, map: crate::affine::Affine) -> Self {
        self.moved_to(map.apply_point(self.centre))
    }

    /// The same mark with its centre at `centre` (a transform maps the centre only, criterion 2).
    #[must_use]
    pub fn moved_to(&self, centre: Point) -> Self {
        Self {
            centre,
            ..self.clone()
        }
    }
}

/// Whether `diameter` (mm) is a valid mark diameter.
#[must_use]
pub fn mark_diameter_is_valid(diameter: f64) -> bool {
    (MARK_MIN_DIAMETER_MM..=MARK_MAX_DIAMETER_MM).contains(&diameter)
}

/// Whether `line` (mm) is a valid mark line width.
#[must_use]
pub fn mark_line_is_valid(line: f64) -> bool {
    (MARK_MIN_LINE_MM..=MARK_MAX_LINE_MM).contains(&line)
}
