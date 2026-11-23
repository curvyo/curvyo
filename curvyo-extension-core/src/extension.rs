//! The `Extension` trait and the generator contract (ADR 0079 §1, §2): what the registry holds and
//! what a generator receives and returns.

use curvyo_document_core::{Point, Vec2};
use serde::Serialize;

use crate::manifest::Manifest;
use crate::notice::NoticeContext;
use crate::param::{ParamValue, StoredParams};

/// One compiled-in extension. The laser implements `manifest` and `notice`, the generators
/// `manifest` and `generate`; a contribution kind is added with its method (ADR 0034 §4).
pub trait Extension: Sync {
    /// What the extension declares.
    fn manifest(&self) -> Manifest;

    /// The text of the notice `id`, `None` while it does not apply or for an extension without
    /// notices.
    fn notice(&self, _id: &str, _context: &NoticeContext) -> Option<String> {
        None
    }

    /// Generates the object `request` names, `None` for an extension without generators or a
    /// kind it does not have.
    fn generate(&self, _request: &GenerateRequest<'_>) -> Option<Generated> {
        None
    }
}

/// The seed of a generated object: set once from the node id at creation and kept by copies
/// (ADR 0029 §5). Box and gear ignore it; it is part of the contract and the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Seed(pub u32);

/// A factor a generated object is scaled by (proportional resize).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ScaleFactor(pub f64);

/// What the maker did to a generated object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Edit<'a> {
    /// The value of this parameter was typed or dragged: it wins over a value derived from it
    /// (Pitch diameter over Module).
    Param(&'a str),
    /// The object was resized proportionally; the generator decides which lengths scale.
    Scale(ScaleFactor),
}

/// A request to generate: pure and deterministic, no I/O, clock or randomness source.
#[derive(Debug, Clone, Copy)]
pub struct GenerateRequest<'a> {
    /// The kind id inside the extension (`gear`).
    pub kind: &'a str,
    /// The stored parameter values; a missing or invalid one reads as its default.
    pub params: &'a StoredParams,
    /// The seed.
    pub seed: Seed,
    /// What changed, `None` for a fresh object or a plain regeneration.
    pub edit: Option<Edit<'a>>,
}

/// One cubic segment; a line is written with `c1 = start` and `c2 = to`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Cubic {
    /// First control point.
    pub c1: Point,
    /// Second control point.
    pub c2: Point,
    /// End point.
    pub to: Point,
}

/// A closed outline: closed by construction (the last segment ends at `start`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClosedPath {
    /// Where the outline starts and ends.
    pub start: Point,
    /// The segments in order.
    pub segments: Vec<Cubic>,
}

/// One compound outline in frame-local mm: its subpaths, holes by the compound-path rule (`0035`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Part {
    /// The outer outline and its holes.
    pub paths: Vec<ClosedPath>,
}

/// A drag handle of a length parameter, in the object's frame.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Handle {
    /// The parameter it changes.
    pub key: &'static str,
    /// Where the knob sits.
    pub anchor: Point,
    /// The drag direction (a unit vector) in which the value grows.
    pub direction: Vec2,
    /// How much the value changes per millimetre of drag along `direction` (2 for a diameter
    /// whose knob sits on the radius **(Proposal)**).
    pub rate: f64,
}

/// Why a value was changed from what was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Bound {
    /// The schema's minimum.
    Minimum,
    /// The schema's maximum.
    Maximum,
    /// A bound that depends on other parameters, the value was lowered to it ("the most").
    Dependent,
    /// A bound that depends on other parameters, the value was raised to it ("the least").
    DependentMinimum,
}

/// A parameter the generator moved to the nearest valid value (permit over refuse); the shell
/// writes the notice sentence from it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Clamp {
    /// The parameter.
    pub key: &'static str,
    /// The value it now has.
    pub value: ParamValue,
    /// Which kind of bound.
    pub bound: Bound,
}

/// The result of one generation.
#[derive(Debug, Clone, PartialEq)]
pub struct Generated {
    /// The normalised values the host writes.
    pub params: StoredParams,
    /// The outlines, one part per piece (a gear is one, a box one per plate).
    pub parts: Vec<Part>,
    /// The drag handles, empty allowed.
    pub handles: Vec<Handle>,
    /// What was clamped.
    pub clamps: Vec<Clamp>,
}
