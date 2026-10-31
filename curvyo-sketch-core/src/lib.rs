//! Constraint sketches retain geometric intent through typed numerical solving.
#![forbid(unsafe_code)]
mod error;
mod expression;
mod ids;
mod parameters;
mod parser;
mod units;
pub use error::SketchError;
pub use expression::Expression;
pub use ids::{ConstraintId, EntityId, ParameterId};
pub use parameters::{Parameter, Parameters};
pub use units::{Angle, Dimension, Length, Position, Quantity};
mod model;
mod remap;
mod validation;
pub use model::{Axis, Constraint, EllipseAxis, Entity, Relation, Sketch};
mod drag_chords;
mod drag_reach;
mod measure;
mod residuals;
mod solver;
mod solver_input;
mod solver_relations;
mod tangency;
pub use measure::measure;
pub use solver::{SolveMode, SolveResult, SolveStatus, solve};

mod construction;
mod construction_api;
mod contact;
mod fillet;
mod outline;
pub use construction_api::{create_slot, fillet};

pub use outline::outline;

mod primitives;
pub use construction_api::{
    create_circle, create_circle_three_points, create_ellipse, create_ellipse_three_points,
    create_polyline,
};

pub use construction_api::{create_arc_three_points, create_rectangle};

mod topology;

mod checkpoint;
pub use checkpoint::{solve_seeded, validate_candidate};
pub use validation::validate_records;

mod operations;
pub use operations::{
    Construction, ConstructionResult, construct, construct_joined, construct_preview,
    construct_preview_joined,
};
mod joins;

mod references;
pub use references::remove_entities;

mod handle_goals;
mod handles;
pub use handles::{SketchHandle, handle_positions};

mod expression_display;

mod continuation;
pub use continuation::{ContinuationMode, arc_radius_endpoint, endpoint_tangent};

mod arc_center;

mod axis_implication;
pub use axis_implication::axis_is_implied;
