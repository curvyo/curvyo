//! The play bar's axis over a cutter job (`specs/0159-cutting-plotter` UX C14, ADR 0098 UX unify
//! 7): the job's time on an HPGL profile, its path length on a Design Space one, and the head's
//! place at a point of that axis.

use curvyo_geometry_core::PointUm;

use crate::job::{CutterJob, MoveKind};

/// What the play bar's slider counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayAxis {
    /// Seconds at each operation's speed (HPGL).
    Seconds,
    /// Millimetres of every move, travel included (Design Space sets the speed itself).
    Millimetres,
}

impl CutterJob {
    /// Each move's share of `axis`, in move order.
    fn move_costs(&self, axis: PlayAxis) -> Vec<f64> {
        let moves = self.moves();
        let mut next_op = 0;
        let mut out = Vec::with_capacity(moves.len());
        for (index, m) in moves.iter().enumerate() {
            let length = m.from.distance_mm(m.to);
            if axis == PlayAxis::Millimetres {
                out.push(length);
                continue;
            }
            // A travel runs at the speed of the operation it leads into.
            let op = match m.kind {
                MoveKind::Work { op } => op,
                MoveKind::Travel => moves[index..]
                    .iter()
                    .find_map(|n| match n.kind {
                        MoveKind::Work { op } => Some(op),
                        MoveKind::Travel => None,
                    })
                    .unwrap_or(next_op),
            };
            next_op = op;
            out.push(length / f64::from(self.operations[op].settings.speed.get()));
        }
        out
    }

    /// The whole job on `axis`.
    #[must_use]
    pub fn axis_total(&self, axis: PlayAxis) -> f64 {
        self.move_costs(axis).iter().sum()
    }

    /// The head at `value` along `axis`: the index of the first move not done in full and the
    /// point the head has reached on it; `None` for an empty job or a value at or past the end.
    #[must_use]
    pub fn head_at(&self, axis: PlayAxis, value: f64) -> Option<(usize, PointUm)> {
        let moves = self.moves();
        let mut left = value.max(0.0);
        for (index, (m, cost)) in moves.iter().zip(self.move_costs(axis)).enumerate() {
            if left < cost {
                let t = if cost > 0.0 { left / cost } else { 0.0 };
                #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                // invariant: a point between two µm points stays inside the i64 range
                let lerp = |a: i64, b: i64| a + ((b - a) as f64 * t).round() as i64;
                return Some((
                    index,
                    PointUm::new(lerp(m.from.x, m.to.x), lerp(m.from.y, m.to.y)),
                ));
            }
            left -= cost;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::OperationRun;
    use crate::{CutterSettings, GENERIC_HPGL_CUTTER, OpKind};

    /// One Draw operation at 100 mm/s: a 30 mm line from 40 mm right of the origin.
    fn job() -> CutterJob {
        let mut settings = CutterSettings::defaults(&GENERIC_HPGL_CUTTER.profile().values);
        settings.speed = crate::SpeedMmS::new(100).unwrap();
        CutterJob {
            operations: vec![OperationRun {
                name: "Draw 1".to_owned(),
                kind: OpKind::Draw,
                settings,
                paths: vec![vec![PointUm::new(40_000, 0), PointUm::new(70_000, 0)]],
            }],
        }
    }

    #[test]
    fn the_axis_is_time_at_the_operations_speed_or_the_path_length() {
        let job = job();
        assert!((job.axis_total(PlayAxis::Millimetres) - 70.0).abs() < 1e-9);
        assert!((job.axis_total(PlayAxis::Seconds) - 0.7).abs() < 1e-9);
        assert_eq!(job.axis_total(PlayAxis::Seconds), job.seconds());
    }

    #[test]
    fn the_head_sits_on_the_move_the_value_reaches() {
        let job = job();
        assert_eq!(
            job.head_at(PlayAxis::Millimetres, 20.0),
            Some((0, PointUm::new(20_000, 0))),
            "half the travel"
        );
        assert_eq!(
            job.head_at(PlayAxis::Seconds, 0.55),
            Some((1, PointUm::new(55_000, 0))),
            "half the line"
        );
        assert_eq!(job.head_at(PlayAxis::Millimetres, 70.0), None, "the end");
        assert_eq!(CutterJob::default().head_at(PlayAxis::Seconds, 0.0), None);
    }
}
