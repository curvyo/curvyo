//! The plotter job: one builder for the Toolpath view and both writers (`specs/0159-cutting-plotter`
//! criteria 9, 10, 14, 15, `adrs.md` PL5): items flattened to µm, ordered by `path_order`, each
//! Cut path repeated per pass.

use curvyo_document_core::Tolerance;
use curvyo_geometry_core::{OrderPath, OutlineTriple, PointUm, flatten_outline, order_paths};

use crate::operation::{CutterSettings, OpKind};

/// The flattening tolerance: with the 0.0005 mm of µm rounding a written point stays within
/// 0.01 mm of the curve (criterion 15, as the laser's).
pub const FLATTEN_MM: f64 = 0.0095;

/// One outline of an item in page mm, Y down from the mat origin.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemOutline {
    /// The anchors with their handles.
    pub anchors: Vec<OutlineTriple>,
    /// Whether it is closed.
    pub closed: bool,
}

/// One item of an operation: its outlines (Cut and Draw both follow the item's own path,
/// `adrs.md` PL8) and its stacking order.
#[derive(Debug, Clone, PartialEq)]
pub struct JobItem {
    /// The outlines.
    pub outlines: Vec<ItemOutline>,
    /// The stacking order, the order's tie-breaker.
    pub stacking: u32,
}

/// One operation of the job, in run order.
#[derive(Debug, Clone, PartialEq)]
pub struct JobOperation {
    /// The operation's name ("Cut 1").
    pub name: String,
    /// Its kind.
    pub kind: OpKind,
    /// Its numbers.
    pub settings: CutterSettings,
    /// Its items.
    pub items: Vec<JobItem>,
}

/// One operation as built: its paths in run order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRun {
    /// The name.
    pub name: String,
    /// The kind.
    pub kind: OpKind,
    /// The numbers.
    pub settings: CutterSettings,
    /// The tool-down paths in run order, each once; a closed path ends on its start.
    pub paths: Vec<Vec<PointUm>>,
}

impl OperationRun {
    /// How often each path runs in a row: Cut's passes, one for Draw (criterion 14).
    #[must_use]
    pub fn passes(&self) -> usize {
        match self.kind {
            OpKind::Cut => usize::from(self.settings.passes.get()),
            OpKind::Draw => 1,
        }
    }

    /// The paths as the machine runs them: each path [`Self::passes`] times in a row.
    pub fn runs(&self) -> impl Iterator<Item = &Vec<PointUm>> {
        self.paths
            .iter()
            .flat_map(|path| std::iter::repeat_n(path, self.passes()))
    }
}

/// What a move does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    /// The tool is down for operation `op` (its index in the job).
    Work {
        /// The operation's index.
        op: usize,
    },
    /// The tool is up.
    Travel,
}

/// One straight move of the job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobMove {
    /// What it does.
    pub kind: MoveKind,
    /// Where it starts.
    pub from: PointUm,
    /// Where it ends.
    pub to: PointUm,
}

/// The built job.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CutterJob {
    /// The operations with paths, in run order.
    pub operations: Vec<OperationRun>,
}

impl CutterJob {
    /// Every move from the mat origin: travel to each path's start, then its segments.
    #[must_use]
    pub fn moves(&self) -> Vec<JobMove> {
        let mut out = Vec::new();
        let mut head = PointUm::new(0, 0);
        for (op, run) in self.operations.iter().enumerate() {
            for path in run.runs() {
                let Some(&start) = path.first() else {
                    continue;
                };
                if start != head {
                    out.push(JobMove {
                        kind: MoveKind::Travel,
                        from: head,
                        to: start,
                    });
                }
                for pair in path.windows(2) {
                    out.push(JobMove {
                        kind: MoveKind::Work { op },
                        from: pair[0],
                        to: pair[1],
                    });
                }
                head = path.last().copied().unwrap_or(head);
            }
        }
        out
    }

    /// The tool-down length of the operations of `kind`, and the travel length, in mm.
    #[must_use]
    pub fn lengths_mm(&self) -> Lengths {
        let mut lengths = Lengths::default();
        for m in self.moves() {
            let d = m.from.distance_mm(m.to);
            match m.kind {
                MoveKind::Travel => lengths.travel += d,
                MoveKind::Work { op } => match self.operations[op].kind {
                    OpKind::Cut => lengths.cut += d,
                    OpKind::Draw => lengths.draw += d,
                },
            }
        }
        lengths
    }

    /// The time in seconds: every move at the speed of the operation it belongs to, a travel at
    /// the speed of the operation it leads into (Proposal; HPGL `VS` sets one speed for both).
    #[must_use]
    pub fn seconds(&self) -> f64 {
        self.axis_total(crate::PlayAxis::Seconds)
    }

    /// Whether the job runs nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.operations.iter().all(|run| run.paths.is_empty())
    }
}

/// The lengths of a job in mm.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Lengths {
    /// Cut, tool down.
    pub cut: f64,
    /// Draw, pen down.
    pub draw: f64,
    /// Travel, tool up.
    pub travel: f64,
}

/// The job of `operations` (in run order): each operation's outlines flattened, ordered inner
/// first then nearest from where the last operation ended, each Cut path repeated per pass.
#[must_use]
pub fn build_job(operations: &[JobOperation]) -> CutterJob {
    let mut head = PointUm::new(0, 0);
    let mut out = Vec::new();
    for operation in operations {
        let paths: Vec<OrderPath> = operation
            .items
            .iter()
            .flat_map(|item| {
                item.outlines
                    .iter()
                    .enumerate()
                    .filter_map(move |(index, outline)| {
                        let points = flat(outline)?;
                        Some(OrderPath {
                            points,
                            closed: outline.closed,
                            rank: (item.stacking, u32::try_from(index).unwrap_or(u32::MAX)),
                        })
                    })
            })
            .collect();
        let (runs, exit) = order_paths(&paths, head);
        head = exit;
        out.push(OperationRun {
            name: operation.name.clone(),
            kind: operation.kind,
            settings: operation.settings,
            paths: runs.into_iter().map(|run| run.points).collect(),
        });
    }
    CutterJob { operations: out }
}

/// The outline flattened and rounded to µm with repeats removed, `None` with fewer than two
/// distinct points.
fn flat(outline: &ItemOutline) -> Option<Vec<PointUm>> {
    let points = flatten_outline(
        &outline.anchors,
        outline.closed,
        Tolerance::from_mm(FLATTEN_MM),
    );
    let mut out: Vec<PointUm> = Vec::new();
    for point in points.into_iter().map(PointUm::of_mm) {
        if out.last() != Some(&point) {
            out.push(point);
        }
    }
    if outline.closed && out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    (out.len() >= 2).then_some(out)
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Point, Vec2};

    use super::*;
    use crate::builtins::GENERIC_HPGL_CUTTER;
    use crate::numbers::Passes;

    pub(crate) fn square(x: f64, y: f64, size: f64) -> ItemOutline {
        let p = |x, y| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO);
        ItemOutline {
            anchors: vec![
                p(x, y),
                p(x + size, y),
                p(x + size, y + size),
                p(x, y + size),
            ],
            closed: true,
        }
    }

    fn op(kind: OpKind, passes: u64, outlines: Vec<ItemOutline>) -> JobOperation {
        let mut settings = CutterSettings::defaults(&GENERIC_HPGL_CUTTER.profile().values);
        settings.passes = Passes::new(passes).unwrap();
        JobOperation {
            name: kind.label().to_owned(),
            kind,
            settings,
            items: vec![JobItem {
                outlines,
                stacking: 0,
            }],
        }
    }

    #[test]
    fn passes_repeat_each_cut_path_in_a_row_on_the_same_points() {
        let job = build_job(&[op(OpKind::Cut, 2, vec![square(10.0, 10.0, 50.0)])]);
        let paths: Vec<_> = job.operations[0].runs().collect();
        assert_eq!(paths.len(), 2, "criterion 14");
        assert_eq!(paths[0], paths[1]);
        assert_eq!(paths[0].first(), paths[0].last(), "closed");
        assert_eq!(
            paths[0][0],
            PointUm::new(10_000, 10_000),
            "nearest to the origin"
        );
    }

    #[test]
    fn the_moves_and_lengths_follow_the_paths() {
        let job = build_job(&[
            op(OpKind::Draw, 1, vec![square(0.0, 0.0, 10.0)]),
            op(OpKind::Cut, 1, vec![square(20.0, 0.0, 10.0)]),
        ]);
        let lengths = job.lengths_mm();
        assert!((lengths.draw - 40.0).abs() < 1e-9);
        assert!((lengths.cut - 40.0).abs() < 1e-9);
        assert!((lengths.travel - 20.0).abs() < 1e-9);
        let seconds = job.seconds();
        assert!((seconds - 100.0 / 200.0).abs() < 1e-9, "{seconds}");
        assert_eq!(job.moves().len(), 9);
    }

    #[test]
    #[allow(clippy::many_single_char_names)]
    fn a_flattened_curve_stays_within_a_hundredth_of_a_millimetre() {
        // A quarter circle of radius 50 mm from (0, 50) to (50, 0) as one cubic.
        let k = 0.552_284_749_8 * 50.0;
        let outline = ItemOutline {
            anchors: vec![
                (Point::new(0.0, 50.0), Vec2::ZERO, Vec2::new(0.0, -k)),
                (Point::new(50.0, 0.0), Vec2::new(-k, 0.0), Vec2::ZERO),
            ],
            closed: false,
        };
        let points = flat(&outline).unwrap();
        assert!(points.len() > 10);
        let cubic = |t: f64| {
            let (a, b, c, d) = ((0.0, 50.0), (0.0, 50.0 - k), (50.0 - k, 0.0), (50.0, 0.0));
            let u = 1.0 - t;
            let f = |p0: f64, p1: f64, p2: f64, p3: f64| {
                u * u * u * p0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * p3
            };
            (f(a.0, b.0, c.0, d.0), f(a.1, b.1, c.1, d.1))
        };
        let samples: Vec<(f64, f64)> = (0..=20_000)
            .map(|i| cubic(f64::from(i) / 20_000.0))
            .collect();
        for p in &points {
            let p = p.to_mm();
            let d = samples
                .iter()
                .map(|s| ((s.0 - p.x).powi(2) + (s.1 - p.y).powi(2)).sqrt())
                .fold(f64::MAX, f64::min);
            assert!(d <= 0.01, "criterion 15: {d}");
        }
    }
}
