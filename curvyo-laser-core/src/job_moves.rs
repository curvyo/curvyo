//! The job's format-neutral moves (ADR 0064 §7, ADR 0069 §1): burn, sweep, travel, run-out and
//! Z moves in page µm, the blocks they run with, and the machine frame every writer maps them
//! through.

use std::sync::Arc;

use curvyo_document_core::NodeId;

use crate::job_input::{PointUm, Ring, um_of_mm};
use crate::job_numbers::{BlockNumbers, RunNumbers};
use crate::job_plan::Plan;
use crate::job_raster_rows::Sweep;
use crate::machine_profile::{OriginCorner, ProfileValues};
use crate::operation_kind::OperationKind;
use crate::operation_settings::Role;
use crate::output_settings::Start;

/// The acceleration the run-out of Overscanning assumes, in mm/s² (ADR 0064 §6, Proposal: the
/// profile holds none).
pub const RUN_OUT_ACCELERATION: f64 = 1_000.0;

/// What one move does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    /// Laser on along a path of `node`, with the set of `role`.
    Burn {
        /// The set.
        role: Role,
        /// The leaf or Cut-out item.
        node: NodeId,
    },
    /// One raster sweep of `node` with the set of `role` (`specs/0123` criterion 14): the head
    /// runs from the sweep's start to its end at the burn feed, its runs at their power.
    Sweep {
        /// The set.
        role: Role,
        /// The image or gradient-filled leaf.
        node: NodeId,
        /// That leaf's stacking place in its block (leaves of one clone share their node).
        stacking: u32,
        /// The sweep's index in [`Job::sweeps`].
        sweep: u32,
    },
    /// Laser off at the machine's maximum feed.
    Travel,
    /// Laser off at the burn feed of `role`: the run-out of Overscanning.
    RunOut(Role),
    /// Laser off to the height `z` µm (absolute), at the block's feed; X and Y stay.
    Z(i64),
}

/// One move: to a page point, of a kind, in a block (`usize::MAX` for the job's first and last
/// travel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    /// Where the head goes, in page µm.
    pub to: PointUm,
    /// What it does on the way.
    pub kind: MoveKind,
    /// The block's index in [`Job::blocks`].
    pub block: usize,
}

/// One run of the job with its numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// The operation or first fixture job.
    pub id: NodeId,
    /// Its ring: the operation row's digit or the fixture glyph (UX P26).
    pub ring: Ring,
    /// The name.
    pub name: String,
    /// The kind.
    pub kind: OperationKind,
    /// The numbers.
    pub numbers: BlockNumbers,
    /// The Z offset in µm that runs (only with a motorised Z axis and a non-zero offset).
    pub z: Option<i64>,
}

impl Block {
    /// The numbers of `role`.
    #[must_use]
    pub fn of(&self, role: Role) -> RunNumbers {
        self.numbers.of(role)
    }
}

/// The bed as the file sees it: the origin corner and the page size in µm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    /// The corner the machine homes to.
    pub corner: OriginCorner,
    /// The page width in µm.
    pub width: i64,
    /// The page height in µm.
    pub height: i64,
}

impl Frame {
    /// The page point of the machine origin.
    #[must_use]
    pub const fn origin(&self) -> PointUm {
        PointUm::new(
            if self.corner.is_right() {
                self.width
            } else {
                0
            },
            if self.corner.is_bottom() {
                self.height
            } else {
                0
            },
        )
    }

    /// The machine coordinates of page point `p` (criterion 26): measured from the origin corner
    /// with axes into the bed.
    #[must_use]
    pub const fn machine_point(&self, p: PointUm) -> PointUm {
        PointUm::new(
            if self.corner.is_right() {
                self.width - p.x
            } else {
                p.x
            },
            if self.corner.is_bottom() {
                self.height - p.y
            } else {
                p.y
            },
        )
    }

    /// The page point of machine coordinates `m`: the inverse of [`Frame::machine_point`], which
    /// mirrors each axis it flips and so is its own inverse.
    #[must_use]
    pub const fn page_point(&self, m: PointUm) -> PointUm {
        self.machine_point(m)
    }

    /// Whether `p` lies on the bed within `tolerance` µm.
    #[must_use]
    pub const fn holds(&self, p: PointUm, tolerance: i64) -> bool {
        p.x >= -tolerance
            && p.y >= -tolerance
            && p.x <= self.width + tolerance
            && p.y <= self.height + tolerance
    }
}

/// The job of one page: everything a writer, the Toolpath view and the estimate read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    /// The job's name (the page name).
    pub name: String,
    /// The machine's values (the page's profile copy).
    pub machine: ProfileValues,
    /// The bed.
    pub frame: Frame,
    /// How the job starts.
    pub start: Start,
    /// Where the head stands when the job starts and where it returns, in page µm.
    pub home: PointUm,
    /// The blocks that run, in run order.
    pub blocks: Vec<Block>,
    /// The moves; the last one is the travel back to [`Job::home`] when any move exists.
    pub moves: Vec<Move>,
    /// The raster sweeps the `Sweep` moves point into; each pass reuses the same sweeps.
    pub sweeps: Vec<Arc<Sweep>>,
}

impl Job {
    /// Whether any move burns.
    #[must_use]
    pub fn burns(&self) -> bool {
        self.moves
            .iter()
            .any(|m| matches!(m.kind, MoveKind::Burn { .. } | MoveKind::Sweep { .. }))
    }

    /// The sweep of a `Sweep` move.
    #[must_use]
    pub fn sweep(&self, index: u32) -> Option<&Sweep> {
        self.sweeps
            .get(usize::try_from(index).ok()?)
            .map(AsRef::as_ref)
    }
}

/// The moves of a job, where it starts and the sweeps its moves point into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltMoves {
    /// The moves.
    pub moves: Vec<Move>,
    /// The home.
    pub home: PointUm,
    /// The sweeps.
    pub sweeps: Vec<Arc<Sweep>>,
}

/// Builds the moves of `blocks` with their plans (ADR 0064 §7): per block a Z move to its offset,
/// its runs once per pass (Frage 223 A), the Z move back; Machine origin starts and ends at the
/// origin (Home, then machine origin gives the same moves), Head position at the first point.
#[must_use]
pub fn build_moves(frame: Frame, start: Start, blocks: &[(Block, &Plan)]) -> BuiltMoves {
    let origin = frame.origin();
    let mut moves = Moves {
        list: Vec::new(),
        sweeps: Vec::new(),
        head: origin,
        frame,
        clamp: !start.relative(),
    };
    for (index, (block, plan)) in blocks.iter().enumerate() {
        moves.block(index, block, plan);
    }
    let mut list = moves.list;
    let home = if start.relative() {
        match list.first() {
            Some(first) if first.kind == MoveKind::Travel => {
                let point = first.to;
                list.remove(0);
                point
            }
            _ => origin,
        }
    } else {
        origin
    };
    if let Some(end) = list.last().map(|m| m.to)
        && end != home
    {
        list.push(Move {
            to: home,
            kind: MoveKind::Travel,
            block: usize::MAX,
        });
    }
    BuiltMoves {
        moves: list,
        home,
        sweeps: moves.sweeps,
    }
}

struct Moves {
    list: Vec<Move>,
    sweeps: Vec<Arc<Sweep>>,
    head: PointUm,
    frame: Frame,
    clamp: bool,
}

impl Moves {
    fn push(&mut self, to: PointUm, kind: MoveKind, block: usize) {
        if to == self.head && !matches!(kind, MoveKind::Z(_)) {
            return;
        }
        self.list.push(Move { to, kind, block });
        self.head = to;
    }

    fn block(&mut self, index: usize, block: &Block, plan: &Plan) {
        let mut z_pending = block.z;
        for run in &plan.runs {
            let numbers = block.of(run.role);
            let run_out = run_out_um(block.numbers.options.overscan.percent(), numbers);
            let first_sweep = self.sweeps.len();
            self.sweeps
                .extend(run.sweeps.iter().map(|s| Arc::clone(&s.sweep)));
            for _ in 0..numbers.passes.get() {
                for path in &run.paths {
                    let Some(&first) = path.points.first() else {
                        continue;
                    };
                    let (lead_in, lead_out) = if path.hatch && run_out > 0.0 {
                        let last = path.points[path.points.len() - 1];
                        (
                            Some(self.extend(first, last, run_out)),
                            Some(self.extend(last, first, run_out)),
                        )
                    } else {
                        (None, None)
                    };
                    self.push(lead_in.unwrap_or(first), MoveKind::Travel, index);
                    if let Some(z) = z_pending.take() {
                        self.push(self.head, MoveKind::Z(z), index);
                    }
                    if lead_in.is_some() {
                        self.push(first, MoveKind::RunOut(run.role), index);
                    }
                    for &point in &path.points[1..] {
                        let kind = MoveKind::Burn {
                            role: run.role,
                            node: path.node,
                        };
                        self.push(point, kind, index);
                    }
                    if let Some(end) = lead_out {
                        self.push(end, MoveKind::RunOut(run.role), index);
                    }
                }
                for (offset, planned) in run.sweeps.iter().enumerate() {
                    let sweep = u32::try_from(first_sweep + offset).unwrap_or(u32::MAX);
                    let kind = MoveKind::Sweep {
                        role: run.role,
                        node: planned.node,
                        stacking: planned.stacking,
                        sweep,
                    };
                    self.sweep(index, &planned.sweep, kind, run_out, &mut z_pending);
                }
            }
        }
        if block.z.is_some() && z_pending.is_none() {
            self.push(self.head, MoveKind::Z(0), index);
        }
    }

    /// The moves of one sweep: travel to its start (or to its run-in with Overscanning), the
    /// sweep itself, the run-out after it.
    fn sweep(
        &mut self,
        index: usize,
        sweep: &Sweep,
        kind: MoveKind,
        run_out: f64,
        z_pending: &mut Option<i64>,
    ) {
        let MoveKind::Sweep { role, .. } = kind else {
            return;
        };
        let (lead_in, lead_out) = if run_out > 0.0 {
            (
                Some(self.extend(sweep.start, sweep.end, run_out)),
                Some(self.extend(sweep.end, sweep.start, run_out)),
            )
        } else {
            (None, None)
        };
        self.push(lead_in.unwrap_or(sweep.start), MoveKind::Travel, index);
        if let Some(z) = z_pending.take() {
            self.push(self.head, MoveKind::Z(z), index);
        }
        if lead_in.is_some() {
            self.push(sweep.start, MoveKind::RunOut(role), index);
        }
        self.push(sweep.end, kind, index);
        if let Some(end) = lead_out {
            self.push(end, MoveKind::RunOut(role), index);
        }
    }

    /// The point `length` µm beyond `end` along `from -> end`, shortened to the bed in Machine
    /// origin mode (ADR 0064 §6).
    fn extend(&self, end: PointUm, from: PointUm, length: f64) -> PointUm {
        let (dx, dy) = (as_f64(from.x - end.x), as_f64(from.y - end.y));
        // Not `f64::hypot`: its result depends on the platform's libm, while `sqrt` is correctly
        // rounded by IEEE 754 on every target, so native and wasm32 write the same bytes.
        let norm = (dx * dx + dy * dy).sqrt();
        if norm == 0.0 {
            return end;
        }
        // Beyond `end`, away from `from`.
        let (ux, uy) = (-dx / norm, -dy / norm);
        let mut length = length;
        if self.clamp {
            length = length.min(room(as_f64(end.x), ux, as_f64(self.frame.width)));
            length = length.min(room(as_f64(end.y), uy, as_f64(self.frame.height)));
            length = length.max(0.0);
        }
        PointUm::new(end.x + round(ux * length), end.y + round(uy * length))
    }
}

/// How far one may go from `at` along `u` before leaving `0..=size`.
fn room(at: f64, u: f64, size: f64) -> f64 {
    if u > 0.0 {
        (size - at) / u
    } else if u < 0.0 {
        -at / u
    } else {
        f64::INFINITY
    }
}

/// The run-out length in µm: `overscan % × v² / (2 a)` with `v` the feed in mm/s.
fn run_out_um(overscan_percent: u8, numbers: RunNumbers) -> f64 {
    let v = f64::from(numbers.feed.get()) / 60.0;
    f64::from(overscan_percent) / 100.0 * v * v / (2.0 * RUN_OUT_ACCELERATION) * 1000.0
}

/// The Z offset in µm a block runs: a non-zero stored offset with a motorised Z axis.
#[must_use]
pub fn running_z_um(numbers: &BlockNumbers, motorised_z: bool) -> Option<i64> {
    numbers
        .options
        .running_z_offset(motorised_z)
        .map(|z| um_of_mm(z.mm()))
        .filter(|&z| z != 0)
}

fn as_f64(um: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    // exact: page coordinates in µm are far below 2^53
    let value = um as f64;
    value
}

fn round(value: f64) -> i64 {
    #[allow(clippy::cast_possible_truncation)]
    // invariant: `as` saturates; run-outs are millimetres
    let um = value.round() as i64;
    um
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_corners_map_the_page_into_the_bed() {
        let p = PointUm::new(10_000, 20_000);
        let frame = |corner| Frame {
            corner,
            width: 400_000,
            height: 300_000,
        };
        assert_eq!(
            frame(OriginCorner::BottomLeft).machine_point(p),
            PointUm::new(10_000, 280_000)
        );
        assert_eq!(frame(OriginCorner::TopLeft).machine_point(p), p);
        assert_eq!(
            frame(OriginCorner::BottomRight).machine_point(p),
            PointUm::new(390_000, 280_000)
        );
        assert_eq!(
            frame(OriginCorner::TopRight).machine_point(p),
            PointUm::new(390_000, 20_000)
        );
        for corner in OriginCorner::ALL {
            assert_eq!(
                frame(corner).machine_point(frame(corner).origin()),
                PointUm::new(0, 0)
            );
        }
    }

    #[test]
    fn the_run_out_is_the_distance_to_reach_the_feed() {
        let numbers = RunNumbers {
            power: crate::material::PowerPercent::from_tenths(500).unwrap(),
            speed: crate::machine_profile::FeedRate::new(6_000).unwrap(),
            feed: crate::machine_profile::FeedRate::new(6_000).unwrap(),
            passes: crate::material::Passes::ONE,
        };
        // 100 mm/s: v² / 2a = 5 mm; at 50 % 2.5 mm.
        assert!((run_out_um(50, numbers) - 2_500.0).abs() < 1e-9);
        assert!(run_out_um(0, numbers).abs() < 1e-12);
    }
}
