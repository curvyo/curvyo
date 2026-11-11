//! The Curvyo geometry kernel (ADR 0003 §1): the narrow slice of
//! operations that need to know what a cubic Bézier is, for
//! `path-node-editing`'s hit-testing — nearest-point-on-segment and de
//! Casteljau subdivision (`specs/0002-path-node-editing/adrs.md`, "the
//! path/node crate boundary").
//!
//! Pure and wasm-compatible (`CLAUDE.md` §6): no filesystem, network,
//! clock, threads or UI. `curvyo-document-core`'s newtypes
//! ([`curvyo_document_core::Point`], [`curvyo_document_core::Vec2`],
//! [`curvyo_document_core::Length`], [`curvyo_document_core::Tolerance`])
//! are this crate's only input/output types besides [`Subdivision`] — no
//! `kurbo` type crosses this crate's public API. `Tolerance` itself lives
//! in `document-core` (ADR 0002 §3 names it there, alongside `Length`), so
//! `curvyo-ui-core`'s hit-testing and this crate's own nearest/subdivide
//! share the exact same type without this crate inventing one.

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod boolean;
mod boolean_cleanup;
mod boolean_grid;
mod bspline;
mod curve_chords;
mod curve_contacts;
mod curve_intersections;
mod curve_settle;
mod flatten;
mod fracture;
mod interior;
mod offset;
mod offset_finish;
mod offset_pieces;
mod offset_polyline;
mod offset_rebuild;
mod outline_nesting;
mod outline_touch;
mod path_measure;
mod pattern;
mod region_contact;
mod segment;
mod spiro;
mod spiro_approx;
mod spiro_solver;
mod stack_regions;
mod stroke_dash;
mod stroke_loops;
mod stroke_markers;
mod stroke_outline;
mod variable_stroke;
mod visible_region;
mod weld;
mod weld_lines;
mod width_profile;

pub use boolean::{BooleanError, BooleanOp, BooleanResult, Outline, boolean, signed_area_mm2};
pub use boolean_grid::MAX_COORDINATE_MM;
pub use bspline::{SplineControl, SplineCubic, SplineError, evaluate_bspline};
pub use curve_intersections::{CutParameter, cut_parameters};
pub use curvyo_document_core::Tolerance;
pub use fracture::{Fracture, fracture_cells};
pub use interior::{OutlineTriple, contains_point_in_outlines};
pub use offset::{
    OffsetAnchor, OffsetError, OffsetJoin, OffsetOutlines, offset, offset_distance_is_zero,
};
pub use offset_pieces::miter_within_limit;
pub use outline_nesting::{Nesting, outline_area_mm2, outline_has_area, outline_nesting};
pub use outline_touch::{TOUCH_DISTANCE_MM, touching_outlines};
pub use path_measure::{PathMeasure, PathSample};
pub use pattern::{
    PATTERN_MAX_PIECES, PATTERN_MAX_WORK, PATTERN_NOTE_PIECES, PatternError, PatternFrame,
    PatternOutline, PatternPiece, pattern_pieces, pattern_pieces_of,
};
pub use segment::{
    BentHandles, Subdivision, bend_segment_handles, nearest_point_on_segment, segment_bounds,
    subdivide_at_parameter,
};
pub use spiro::{
    SPIRO_MAX_NEW_SWEEP, SPIRO_MAX_TURN_CHANGE, SpiroControl, SpiroCubic, SpiroError, SpiroShape,
    evaluate_spiro, solve_spiro, solve_spiro_near, spiro_cubics,
};
pub use stack_regions::StackError;
pub use stroke_outline::{StrokeOutlineError, StrokeShape, stroke_outline};
pub use variable_stroke::{VariableStrokeError, variable_stroke_outline};
pub use visible_region::{Visible, visible_regions};
pub use weld::{Welded, weld};
pub use width_profile::{WidthPoint, WidthProfile};
