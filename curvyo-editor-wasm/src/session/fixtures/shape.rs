//! The typed shape of one fixture, read from its kind and stored parameters (ADR 0049 §2,
//! ADR 0059 §4).

use curvyo_document_core::{Length, OriginCorner, Point};
use curvyo_extension_core::StoredParams;
use curvyo_laser_core::{
    CentreGrid, FixtureKind, GridSpec, MmRange, board_side_range, centre_grid, grid_spec, pin_spec,
    screw_spec, stop_leg_range, stop_thickness_range,
};
use serde_json::Value;

/// The default width and height of a new holder (criterion 2, Proposal).
pub(super) const HOLDER_SIZE_MM: (f64, f64) = (100.0, 60.0);
/// The default width and height of a new clamp (criterion 2, Proposal).
pub(super) const CLAMP_SIZE_MM: (f64, f64) = (40.0, 20.0);

/// The shape and the typed values of one fixture.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Shape {
    /// The engraved grid over the work area.
    Grid(GridSpec),
    /// A holder's board.
    Board { width: Length, height: Length },
    /// A clamp's keep-out rectangle.
    Clamp { width: Length, height: Length },
    /// An L-stop: two legs from the outer corner along +x and +y, and the thickness.
    Stop {
        leg_a: Length,
        leg_b: Length,
        thickness: Length,
    },
    /// A screw: two concentric circles about the frame's origin (criterion 37), at every centre
    /// of its grid (`0113` criterion 19).
    Screw {
        thread: Length,
        head: Length,
        grid: CentreGrid,
    },
    /// A pin: a circle about the frame's origin, at every centre of its grid (`0113` criteria
    /// 18, 19).
    Pin { diameter: Length, grid: CentreGrid },
    /// A kind this build does not know: drawn never, hit never (criterion 32).
    Unknown,
}

impl Shape {
    /// The stored parameter keys and values of this shape (ADR 0049 §2, ADR 0059 §4); none for
    /// the grid and an unknown kind, whose values are written by their own fields.
    pub(super) fn params(&self) -> Vec<(&'static str, Value)> {
        let mm = |v: Length| Value::from(v.as_mm());
        match self {
            Self::Board { width, height } | Self::Clamp { width, height } => {
                vec![("width_mm", mm(*width)), ("height_mm", mm(*height))]
            }
            Self::Stop {
                leg_a,
                leg_b,
                thickness,
            } => vec![
                ("leg_a_mm", mm(*leg_a)),
                ("leg_b_mm", mm(*leg_b)),
                ("thickness_mm", mm(*thickness)),
            ],
            Self::Screw { thread, head, grid } => {
                let mut out = vec![
                    ("thread_diameter_mm", mm(*thread)),
                    ("head_diameter_mm", mm(*head)),
                ];
                out.extend(grid_params(grid));
                out
            }
            Self::Pin { diameter, grid } => {
                let mut out = vec![("diameter_mm", mm(*diameter))];
                out.extend(grid_params(grid));
                out
            }
            Self::Grid(_) | Self::Unknown => Vec::new(),
        }
    }

    /// The grid of a pin or a screw; `None` for every other shape.
    pub(super) const fn centre_grid(&self) -> Option<CentreGrid> {
        match self {
            Self::Pin { grid, .. } | Self::Screw { grid, .. } => Some(*grid),
            _ => None,
        }
    }

    /// The outer radius of one pin or screw of a grid: the pin's, or the screw's head.
    pub(super) fn centre_radius(&self) -> f64 {
        match self {
            Self::Pin { diameter, .. } => diameter.as_mm() / 2.0,
            Self::Screw { head, .. } => head.as_mm() / 2.0,
            _ => 0.0,
        }
    }

    /// The circles drawn at each centre: the pin's, or the screw's head and thread.
    pub(super) fn centre_diameters(&self) -> Vec<Length> {
        match self {
            Self::Pin { diameter, .. } => vec![*diameter],
            Self::Screw { thread, head, .. } => vec![*head, *thread],
            _ => Vec::new(),
        }
    }
}

/// The stored grid keys of a pin or a screw (ADR 0059 §4): the counts as whole numbers.
fn grid_params(grid: &CentreGrid) -> [(&'static str, Value); 5] {
    [
        ("columns", Value::from(grid.counts.columns())),
        ("rows", Value::from(grid.counts.rows())),
        (
            "column_spacing_mm",
            Value::from(grid.column_spacing.as_mm()),
        ),
        ("row_spacing_mm", Value::from(grid.row_spacing.as_mm())),
        ("mark_mm", Value::from(grid.mark.as_mm())),
    ]
}

impl Shape {
    /// The centres of a pin or screw grid in the fixture's own frame, the first at the origin;
    /// the origin alone for any other shape.
    pub(super) fn local_centres(&self) -> Vec<Point> {
        self.centre_grid().map_or_else(
            || vec![Point::new(0.0, 0.0)],
            |grid| {
                curvyo_laser_core::centres(
                    Point::new(0.0, 0.0),
                    grid.counts,
                    grid.column_spacing,
                    grid.row_spacing,
                )
            },
        )
    }
}

/// A length parameter inside its range, or the range's default (criterion 33).
fn mm(params: &StoredParams, key: &str, range: MmRange) -> Length {
    params
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .filter(|v| (range.min.as_mm()..=range.max.as_mm()).contains(v))
        .map_or(range.default, Length::from_mm)
}

/// The typed shape of a fixture of `kind` with the stored `params`; the grid counts from the
/// page grid's origin corner `corner` (Frage 174 B).
pub(super) fn shape_of(kind: FixtureKind, params: &StoredParams, corner: OriginCorner) -> Shape {
    let side = |key, default| mm(params, key, board_side_range(default));
    match kind {
        FixtureKind::EngravedGrid => Shape::Grid(grid_spec(params, corner).0),
        FixtureKind::Holder => Shape::Board {
            width: side("width_mm", HOLDER_SIZE_MM.0),
            height: side("height_mm", HOLDER_SIZE_MM.1),
        },
        FixtureKind::Clamp => Shape::Clamp {
            width: side("width_mm", CLAMP_SIZE_MM.0),
            height: side("height_mm", CLAMP_SIZE_MM.1),
        },
        FixtureKind::Stop => Shape::Stop {
            leg_a: mm(params, "leg_a_mm", stop_leg_range()),
            leg_b: mm(params, "leg_b_mm", stop_leg_range()),
            thickness: mm(params, "thickness_mm", stop_thickness_range()),
        },
        FixtureKind::Screw => {
            let (thread, head, _) = screw_spec(params);
            let (grid, _) = centre_grid(params);
            Shape::Screw { thread, head, grid }
        }
        FixtureKind::Pin => {
            let (diameter, _) = pin_spec(params);
            let (grid, _) = centre_grid(params);
            Shape::Pin { diameter, grid }
        }
        FixtureKind::Unknown => Shape::Unknown,
    }
}
