//! The tracer's quality harness: the 0025 set with its metrics, plus the slice A reference set of
//! `specs/0089-trace-quality-and-colour` with its annotations, metrics, report.

// Each test binary uses a different part of the harness; the numeric code reads best with the
// short names of its formulas (x, y, w, h, u, v).
#![allow(
    unused_imports,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::needless_range_loop
)]

mod catalogue;
mod degrade;
mod fixtures;
mod metrics;
pub mod real;
pub mod reference;
pub mod render;
pub mod report;
pub mod run;
mod scenes;
mod scenes_more;
pub mod shape;
mod text;
pub mod topology;

pub use catalogue::reference_set;
pub use fixtures::*;
pub use metrics::*;
