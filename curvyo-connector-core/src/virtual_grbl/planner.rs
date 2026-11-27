//! The virtual GRBL's motion planner (`specs/0161-cnc-output` criterion 18, `adrs.md` CN5):
//! GRBL 1.1's axis-limited rates and accelerations, junction speeds from the junction deviation,
//! and trapezoid speed profiles over a run of moves that starts and ends at rest.

use super::parse::{Action, Modal, parse_line};
use super::settings::GrblSettings;
use crate::gcode_line::strip;

/// A time in seconds.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Seconds(f64);

impl Seconds {
    /// `s` seconds.
    #[must_use]
    pub const fn new(s: f64) -> Self {
        Self(s)
    }

    /// The value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// One straight move as the planner holds it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    /// Where it ends (machine mm).
    pub target: [f64; 3],
    /// Its length in mm.
    pub length: f64,
    /// Its unit direction.
    pub unit: [f64; 3],
    /// Its nominal speed in mm/s: the feed (or the rapid rate) limited by every axis.
    pub nominal: f64,
    /// Its acceleration in mm/s², limited by every axis.
    pub accel: f64,
}

/// `limits[i] / |unit[i]|` at its smallest over the moving axes.
fn axis_limited(unit: [f64; 3], limit: impl Fn(usize) -> f64) -> f64 {
    (0..3)
        .filter(|&i| unit[i].abs() > 1e-12)
        .map(|i| limit(i) / unit[i].abs())
        .fold(f64::INFINITY, f64::min)
}

/// The block from `from` to `target`: a rapid at the machine's rates, else at `feed_mm_min`;
/// `None` for a move shorter than a micrometre.
#[must_use]
pub fn block(
    from: [f64; 3],
    target: [f64; 3],
    rapid: bool,
    feed_mm_min: f64,
    s: &GrblSettings,
) -> Option<Block> {
    let d = [
        target[0] - from[0],
        target[1] - from[1],
        target[2] - from[2],
    ];
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if length < 1e-3 {
        return None;
    }
    let unit = d.map(|v| v / length);
    let max = axis_limited(unit, |i| s.max_rate_mm_s(i));
    let nominal = if rapid {
        max
    } else {
        (feed_mm_min / 60.0).min(max)
    };
    Some(Block {
        target,
        length,
        unit,
        nominal,
        accel: axis_limited(unit, |i| s.acceleration(i)),
    })
}

/// The square of the highest speed through the junction from `a` into `b` (GRBL 1.1's
/// junction deviation rule).
#[must_use]
pub fn junction_max_sq(a: &Block, b: &Block, s: &GrblSettings) -> f64 {
    let cos = -(a.unit[0] * b.unit[0] + a.unit[1] * b.unit[1] + a.unit[2] * b.unit[2]);
    let limit = a.nominal.min(b.nominal).powi(2);
    if cos > 0.999_999 {
        return 0.0;
    }
    if cos < -0.999_999 {
        return limit;
    }
    let turn: [f64; 3] = std::array::from_fn(|i| b.unit[i] - a.unit[i]);
    let norm = (turn[0] * turn[0] + turn[1] * turn[1] + turn[2] * turn[2]).sqrt();
    let accel = axis_limited(turn.map(|v| v / norm), |i| s.acceleration(i));
    let sin_half = (0.5 * (1.0 - cos)).sqrt();
    (accel * s.junction_deviation() * sin_half / (1.0 - sin_half)).min(limit)
}

/// The time of one block entered at `v0_sq` and left at `v1_sq` (squared mm/s).
fn trapezoid(b: &Block, v0_sq: f64, v1_sq: f64) -> f64 {
    let (a, vn) = (b.accel, b.nominal);
    let (v0, v1) = (v0_sq.sqrt(), v1_sq.sqrt());
    let up = (vn * vn - v0_sq) / (2.0 * a);
    let down = (vn * vn - v1_sq) / (2.0 * a);
    if up + down <= b.length {
        (vn - v0) / a + (vn - v1) / a + (b.length - up - down) / vn
    } else {
        let peak = ((2.0 * a * b.length + v0_sq + v1_sq) / 2.0).sqrt();
        (peak - v0) / a + (peak - v1) / a
    }
}

/// Each block's time and exit speed (squared) for `blocks` run from `entry_sq` and ending at
/// rest: the backward and forward passes of GRBL's planner.
#[must_use]
pub fn durations(blocks: &[Block], entry_sq: f64, s: &GrblSettings) -> Vec<(f64, f64)> {
    let n = blocks.len();
    // `entry[i]` is the squared speed at the start of block i; `entry[n]` is the end, at rest.
    let mut entry = vec![0.0; n + 1];
    for i in 1..n {
        entry[i] = junction_max_sq(&blocks[i - 1], &blocks[i], s);
    }
    if n > 0 {
        entry[0] = entry_sq.min(blocks[0].nominal.powi(2));
    }
    for i in (0..n).rev() {
        let reach = entry[i + 1] + 2.0 * blocks[i].accel * blocks[i].length;
        entry[i] = entry[i].min(reach);
    }
    for i in 0..n {
        let reach = entry[i] + 2.0 * blocks[i].accel * blocks[i].length;
        entry[i + 1] = entry[i + 1].min(reach);
    }
    (0..n)
        .map(|i| (trapezoid(&blocks[i], entry[i], entry[i + 1]), entry[i + 1]))
        .collect()
}

/// The run time of a G-code job and when each of its lines ends.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlanTime {
    /// The whole run.
    pub total: Seconds,
    /// When line `i` (counted from 0) is done.
    pub line_end: Vec<Seconds>,
}

/// The run time of `gcode` on a machine of `settings` (criterion 18): the moves between two
/// spindle changes or dwells (which wait for the planner) run as one plan from rest to rest.
/// A line GRBL refuses takes no time.
#[must_use]
pub fn plan_time(gcode: &str, settings: &GrblSettings) -> PlanTime {
    let mut modal = Modal::default();
    let mut pos = [0.0; 3];
    let mut run: Vec<(usize, Block)> = Vec::new();
    let mut ends = vec![0.0; gcode.lines().count()];
    let mut clock = 0.0;
    let flush = |run: &mut Vec<(usize, Block)>, clock: &mut f64, ends: &mut Vec<f64>| {
        let blocks: Vec<Block> = run.iter().map(|r| r.1).collect();
        for ((line, _), (t, _)) in run.iter().zip(durations(&blocks, 0.0, settings)) {
            *clock += t;
            ends[*line] = *clock;
        }
        run.clear();
    };
    for (index, raw) in gcode.lines().enumerate() {
        let line = strip(raw).to_ascii_uppercase();
        let actions = if line.is_empty() || line.starts_with('$') {
            Vec::new()
        } else {
            parse_line(&line, &mut modal, pos).unwrap_or_default()
        };
        for action in actions {
            match action {
                Action::Move {
                    target,
                    rapid,
                    feed_mm_min,
                } => {
                    if let Some(b) = block(pos, target, rapid, feed_mm_min, settings) {
                        run.push((index, b));
                    }
                    pos = target;
                }
                Action::Spindle(..) => flush(&mut run, &mut clock, &mut ends),
                Action::Dwell(seconds) => {
                    flush(&mut run, &mut clock, &mut ends);
                    clock += seconds.max(0.0);
                }
            }
        }
        ends[index] = ends[index].max(clock);
    }
    flush(&mut run, &mut clock, &mut ends);
    let mut last = 0.0_f64;
    let line_end = ends
        .into_iter()
        .map(|t| {
            last = last.max(t);
            Seconds(last)
        })
        .collect();
    PlanTime {
        total: Seconds(clock),
        line_end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_line_runs_a_trapezoid_and_a_short_one_a_triangle() {
        let s = GrblSettings::default();
        // 500 mm/min = 8.333 mm/s, 10 mm/s²: 0.8333 s and 3.472 mm up and as much down.
        let t = plan_time("G1X100F500\n", &s).total.get();
        let v = 500.0 / 60.0;
        let expected = 2.0 * v / 10.0 + (100.0 - v * v / 10.0) / v;
        assert!((t - expected).abs() < 1e-9, "{t} {expected}");
        let short = plan_time("G1X1F500\n", &s).total.get();
        assert!((short - 2.0 * (1.0_f64 / 10.0).sqrt()).abs() < 1e-9);
    }

    #[test]
    fn a_straight_continuation_keeps_its_speed_and_a_right_angle_slows_down() {
        let s = GrblSettings::default();
        let one = plan_time("G1X100F500\n", &s).total.get();
        let two = plan_time("G1X50F500\nX100\n", &s).total.get();
        assert!((one - two).abs() < 1e-9);
        let corner = plan_time("G1X50F500\nY50\n", &s).total.get();
        assert!(corner > one);
        let times = plan_time("G1X50F500\nY50\n", &s).line_end;
        assert!(times[0].get() < times[1].get());
    }

    #[test]
    fn feeds_are_clamped_to_the_axis_rate_and_rapids_run_at_it() {
        let s = GrblSettings::default();
        let fast = plan_time("G1X100F5000\n", &s).total.get();
        let rapid = plan_time("G0X100\n", &s).total.get();
        let rated = plan_time("G1X100F500\n", &s).total.get();
        assert!((fast - rated).abs() < 1e-9 && (rapid - rated).abs() < 1e-9);
    }
}
