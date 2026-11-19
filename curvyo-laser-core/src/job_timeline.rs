//! The job's lengths, its time estimate and the head position at a time (ADR 0064 §10; criteria
//! 10, 37): burn at the programmed feed, travel at the machine's maximum feed, no acceleration.

use curvyo_document_core::Point;

use crate::job_input::{PointUm, mm_of_um};
use crate::job_moves::{Job, MoveKind};
use crate::machine_profile::grouped;

/// The cumulative lengths and times of a job's moves.
#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    /// After move `i`: seconds since the start.
    time_s: Vec<f64>,
    /// After move `i`: burn length in mm.
    burn_mm: Vec<f64>,
    /// After move `i`: travel length in mm (travel and run-out).
    travel_mm: Vec<f64>,
    /// Seconds per block.
    block_s: Vec<f64>,
}

/// Where the head is at one moment of the run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Head {
    /// The moves done in full before it.
    pub done: usize,
    /// The head's page point in mm.
    pub at: Point,
    /// Seconds since the start.
    pub elapsed_s: f64,
    /// Burn length up to the head, in mm.
    pub burn_mm: f64,
    /// Travel length up to the head, in mm.
    pub travel_mm: f64,
}

impl Timeline {
    /// The timeline of `job`.
    #[must_use]
    pub fn of(job: &Job) -> Self {
        let max_feed = f64::from(job.machine.max_feed.get());
        let mut line = Self {
            time_s: Vec::with_capacity(job.moves.len()),
            burn_mm: Vec::with_capacity(job.moves.len()),
            travel_mm: Vec::with_capacity(job.moves.len()),
            block_s: vec![0.0; job.blocks.len()],
        };
        let (mut time, mut burn, mut travel) = (0.0, 0.0, 0.0);
        let mut from = job.home;
        let mut z = 0_i64;
        for m in &job.moves {
            let length = from.distance_mm(m.to);
            let block = job.blocks.get(m.block);
            let feed = |role| block.map_or(max_feed, |b| f64::from(b.of(role).feed.get()));
            let seconds = match m.kind {
                // A sweep runs its short gaps at the burn feed too (ADR 0069 §4).
                MoveKind::Burn { role, .. } | MoveKind::Sweep { role, .. } => {
                    burn += length;
                    length / feed(role) * 60.0
                }
                MoveKind::Travel => {
                    travel += length;
                    length / max_feed * 60.0
                }
                MoveKind::RunOut(role) => {
                    travel += length;
                    length / feed(role) * 60.0
                }
                MoveKind::Z(to) => {
                    let dz = mm_of_um((to - z).abs());
                    z = to;
                    dz / feed(crate::operation_settings::Role::Main) * 60.0
                }
            };
            time += seconds;
            if let Some(slot) = line.block_s.get_mut(m.block) {
                *slot += seconds;
            }
            line.time_s.push(time);
            line.burn_mm.push(burn);
            line.travel_mm.push(travel);
            from = m.to;
        }
        line
    }

    /// The estimated total in seconds.
    #[must_use]
    pub fn total_s(&self) -> f64 {
        self.time_s.last().copied().unwrap_or(0.0)
    }

    /// The total burn length in mm.
    #[must_use]
    pub fn burn_mm(&self) -> f64 {
        self.burn_mm.last().copied().unwrap_or(0.0)
    }

    /// The total travel length in mm.
    #[must_use]
    pub fn travel_mm(&self) -> f64 {
        self.travel_mm.last().copied().unwrap_or(0.0)
    }

    /// The seconds of block `index` (its own moves; the travel to it counts in it).
    #[must_use]
    pub fn block_s(&self, index: usize) -> f64 {
        self.block_s.get(index).copied().unwrap_or(0.0)
    }

    /// The head at `fraction` (0 to 1) of the total time: the move it falls in is split, so no
    /// rebuild is needed (criterion 36 f).
    #[must_use]
    pub fn head_at(&self, job: &Job, fraction: f64) -> Head {
        let total = self.total_s();
        let target = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0) * total
        } else {
            0.0
        };
        let done = self.time_s.partition_point(|&t| t <= target);
        let before = |list: &[f64], i: usize| if i == 0 { 0.0 } else { list[i - 1] };
        let Some(m) = job.moves.get(done) else {
            return Head {
                done: job.moves.len(),
                at: job.moves.last().map_or(job.home, |m| m.to).to_mm(),
                elapsed_s: total,
                burn_mm: self.burn_mm(),
                travel_mm: self.travel_mm(),
            };
        };
        let from = if done == 0 {
            job.home
        } else {
            job.moves[done - 1].to
        };
        let (t0, t1) = (before(&self.time_s, done), self.time_s[done]);
        let part = if t1 > t0 {
            (target - t0) / (t1 - t0)
        } else {
            0.0
        };
        let length = from.distance_mm(m.to) * part;
        let (burn, travel) = (before(&self.burn_mm, done), before(&self.travel_mm, done));
        let (burn, travel) = match m.kind {
            MoveKind::Burn { .. } | MoveKind::Sweep { .. } => (burn + length, travel),
            MoveKind::Travel | MoveKind::RunOut(_) => (burn, travel + length),
            MoveKind::Z(_) => (burn, travel),
        };
        Head {
            done,
            at: lerp(from, m.to, part),
            elapsed_s: target,
            burn_mm: burn,
            travel_mm: travel,
        }
    }
}

fn lerp(a: PointUm, b: PointUm, t: f64) -> Point {
    let (a, b) = (a.to_mm(), b.to_mm());
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// A duration as criterion 10 writes it: "21 s", "12 min 40 s", "1 h 5 min" (whole seconds).
#[must_use]
pub fn duration_text(seconds: f64) -> String {
    let whole = if seconds.is_finite() && seconds > 0.0 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // invariant: positive and finite; `as` saturates
        let s = seconds.round() as u64;
        s
    } else {
        0
    };
    let (h, m, s) = (whole / 3600, whole / 60 % 60, whole % 60);
    match (h, m) {
        (0, 0) => format!("{s} s"),
        (0, _) => format!("{m} min {s} s"),
        _ => format!("{h} h {m} min"),
    }
}

/// The summary's total: "About 21 s".
#[must_use]
pub fn estimate_text(seconds: f64) -> String {
    format!("About {}", duration_text(seconds))
}

/// A length as criterion 10 writes it: metres with two decimals above 1 m, else whole mm.
#[must_use]
pub fn length_text(mm: f64) -> String {
    let mm = if mm.is_finite() { mm.max(0.0) } else { 0.0 };
    if mm > 1000.0 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // invariant: positive and finite
        let cm = (mm / 10.0).round() as u64;
        format!("{}.{:02} m", grouped(cm / 100), cm % 100)
    } else {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // invariant: positive and finite
        let whole = mm.round() as u64;
        format!("{} mm", grouped(whole))
    }
}

/// The summary's lengths: "Burn 38.29 m · travel 2.26 m".
#[must_use]
pub fn lengths_text(burn_mm: f64, travel_mm: f64) -> String {
    format!(
        "Burn {} · travel {}",
        length_text(burn_mm),
        length_text(travel_mm)
    )
}

/// The play bar's readout, time only: "6 min 6 s of 9 min 51 s" (criterion 37, amended
/// 2026-11-17: no length).
#[must_use]
pub fn readout_text(head: &Head, timeline: &Timeline) -> String {
    format!(
        "{} of {}",
        duration_text(head.elapsed_s),
        duration_text(timeline.total_s())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_as_criterion_ten() {
        assert_eq!(duration_text(21.0), "21 s");
        assert_eq!(duration_text(760.4), "12 min 40 s");
        assert_eq!(duration_text(3_900.0), "1 h 5 min");
        assert_eq!(duration_text(f64::NAN), "0 s");
        assert_eq!(estimate_text(21.0), "About 21 s");
    }

    #[test]
    fn lengths_read_in_mm_or_m() {
        assert_eq!(length_text(640.4), "640 mm");
        assert_eq!(length_text(1000.0), "1,000 mm");
        assert_eq!(length_text(38_290.0), "38.29 m");
        assert_eq!(
            lengths_text(38_290.0, 2_260.0),
            "Burn 38.29 m · travel 2.26 m"
        );
    }
}
