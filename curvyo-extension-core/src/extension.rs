//! The `Extension` trait and the generator contract (ADR 0080 §1, §2): what the registry holds and
//! what a generator receives and returns.

use std::borrow::Cow;

use curvyo_document_core::{Length, Point, Vec2};
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

/// What happened to a generated object: what the maker did to it, or a request for a draft.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Edit<'a> {
    /// The value of this parameter was typed or dragged: it wins over a value derived from it
    /// (Pitch diameter over Module).
    Param(&'a str),
    /// The object was resized proportionally; the generator decides which lengths scale.
    Scale(ScaleFactor),
    /// Only the outer outlines are wanted, for a ghost under the pointer: a generator may leave
    /// out fine detail (the slits of a bending wall). The result is never stored.
    Draft,
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

impl ClosedPath {
    /// The stored encoding of one subpath: `[x0, y0, c1x, c1y, c2x, c2y, x, y, ...]`, the start
    /// point and then three points for every cubic, the closing one included (ADR 0080 §4).
    #[must_use]
    pub fn flat(&self) -> Vec<f64> {
        let mut numbers = vec![self.start.x, self.start.y];
        for cubic in &self.segments {
            numbers.extend([
                cubic.c1.x, cubic.c1.y, cubic.c2.x, cubic.c2.y, cubic.to.x, cubic.to.y,
            ]);
        }
        numbers
    }

    /// The path a stored subpath of `2 + 6n` numbers holds, `None` for any other length or a
    /// last cubic that does not end at the start (a closed path is closed by construction).
    #[must_use]
    pub fn from_flat(numbers: &[f64]) -> Option<Self> {
        if numbers.len() < 2 || !(numbers.len() - 2).is_multiple_of(6) {
            return None;
        }
        let point = |at: usize| Point::new(numbers[at], numbers[at + 1]);
        let segments: Vec<Cubic> = (0..(numbers.len() - 2) / 6)
            .map(|n| Cubic {
                c1: point(2 + 6 * n),
                c2: point(4 + 6 * n),
                to: point(6 + 6 * n),
            })
            .collect();
        let path = Self {
            start: point(0),
            segments,
        };
        let last = path.segments.last()?.to;
        ((last.x - path.start.x).abs() < 1e-6 && (last.y - path.start.y).abs() < 1e-6)
            .then_some(path)
    }
}

/// One compound outline in frame-local mm: its subpaths, holes by the compound-path rule (`0035`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Part {
    /// The part's stable key (ADR 0082 §10): unique within one result and the same on every
    /// regeneration, so an operation can hold `member.<node>#<key>`. A gear `gear`, a Box
    /// `bottom`, `front`, `back`, `left`, `right`, `lid`, a rounded box `strip`, `base`, `cover`,
    /// `ring`, `lid` (and in Curved `left`, `right`, `front`), a path box `wall-<n>` first. A key
    /// is 1 to 15 characters of `a-z`, `0-9` and `-` (`wall-4294967295` fits); the host refuses a
    /// record whose keys are longer, repeat or use other characters, so a generator must keep
    /// to this grammar.
    pub key: Cow<'static, str>,
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
    /// A bound that depends on other parameters, named by its cause: "the most error correction M
    /// allows" (the name is the subject).
    DependentFor(&'static str),
    /// The same, because of a choice that sets its own minimum ("the least the Wave pattern
    /// allows"): the name of that choice.
    DependentMinimumFor(&'static str),
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

/// A fact about the result that the shell words as a notice (ADR 0083 §11): run time only, never
/// stored. The shell writes the sentence because it formats lengths in the document's units and
/// merges warnings of one id into the plural sentence.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Warning {
    /// Which warning (`sharp_corner`, `tight_bend`, `ring_closed`, `hollow_left_out`,
    /// `smallest_size`, `outline_crosses`, `cover_rounded`, `cover_offset_failed`,
    /// `fillet_failed`, `bend_too_short`, `tab_window`, `unknown_joint_pattern`).
    pub id: &'static str,
    /// The wall pieces it names, by their numbers (1 based, walk order).
    pub pieces: Vec<u32>,
    /// The value that caused it (an interior angle, a radius).
    pub value: Option<ParamValue>,
    /// The limit it breaks (the least radius).
    pub limit: Option<ParamValue>,
}

/// One drawing the shell lays over the generated object (ADR 0083 §11): never geometry, never cut.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Mark {
    /// A corner (a finger joint): where, the unit direction of its bisector and whether it is
    /// very sharp.
    Corner {
        /// The corner point.
        at: Point,
        /// The unit vector from the corner into the box.
        bisector: Vec2,
        /// Whether the interior angle is below 30 degrees.
        sharp: bool,
    },
    /// A stretch of the inner face under a slit band, as cubics, the band is drawn inside it. One
    /// mark per stretch of one tightness inside a run: consecutive marks share their end points.
    /// Windows, the flats beside a corner and the seam margin lie outside every stretch (ADR 0083
    /// §13.16).
    Bend {
        /// Where it starts.
        start: Point,
        /// The stretch.
        segments: Vec<Cubic>,
        /// Whether the radius is too small to bend (criterion 7).
        tight: bool,
    },
    /// Where the strip of an outline with no corner is glued shut: the seam point on the inner
    /// face and the unit vector into the box.
    Seam {
        /// The seam point.
        at: Point,
        /// The unit vector from the face into the box.
        inward: Vec2,
    },
    /// A wall piece's number badge.
    Piece {
        /// The number, 1 based, walk order.
        number: u32,
        /// The middle of the stretch the piece stands on (on a footprint) or of the piece (on a
        /// part).
        at: Point,
        /// The unit vector from the stretch into the box (zero on a part).
        inward: Vec2,
        /// The stretch's length in mm.
        length: Length,
    },
}

/// The overlay marks of one generation (ADR 0083 §11).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Marks {
    /// Where the stored outline's origin lies in the layout: the inner base's offset.
    pub footprint: Vec2,
    /// Marks in the stored outline's coordinates.
    pub on_footprint: Vec<Mark>,
    /// Marks in the object's frame (the badges on the wall pieces).
    pub on_parts: Vec<Mark>,
}

impl Default for Marks {
    fn default() -> Self {
        Self {
            footprint: Vec2::ZERO,
            on_footprint: Vec::new(),
            on_parts: Vec::new(),
        }
    }
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
    /// The values the generator used for parameters stored as 0 (automatic, ADR 0082 §7); the
    /// block shows them muted. Never written to the record.
    pub automatic: Vec<(&'static str, ParamValue)>,
    /// A word for a value that sits at a limit that is a shape ("full round"), by parameter.
    pub states: Vec<(&'static str, &'static str)>,
    /// The largest value a bound that depends on other parameters allows right now, by
    /// parameter: the block's slider ends there (aria-valuemax), not at the schema's.
    pub limits: Vec<(&'static str, ParamValue)>,
    /// Facts the shell words as notices (a very sharp corner); empty for a kind without any.
    pub warnings: Vec<Warning>,
    /// The overlay marks; empty for a kind without any.
    pub marks: Marks,
}
