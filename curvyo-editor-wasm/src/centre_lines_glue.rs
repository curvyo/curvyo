//! The worker's side of Find centre lines (ADR 0063 amendment 2026-11-18, W): it keeps each
//! subject's skeleton with its staged memo across option changes and answers each change with
//! the result's numbers; it calls only `geometry-core`.

use std::collections::HashMap;

use curvyo_document_core::{Length, LineCap, LineJoin, Point, Stroke, Tolerance};
use curvyo_geometry_core::{
    CentreOptions, CentrePath, CentreRole, CentreSkeleton, CentreStages, DEFAULT_THICK_PARTS,
    Outline, StrokeShape, variable_stroke_outline,
};

use crate::centre_lines_wire::{
    SubjectLines, SubjectOutlines, WireError, decode_subjects, encode_results,
};

/// The inner area offset by default, in millimetres (`ux-notes.md` §3).
pub const DEFAULT_INNER_OFFSET_MM: f64 = 0.3;
/// The largest Thick parts value of the bar, in mm: there it means no limit (only a disc stays
/// filled, criterion 12).
pub const THICK_PARTS_MAX_MM: f64 = 100.0;
/// The largest inner area offset either way, in mm (`ux-notes.md` §3: −5 to +5).
const INNER_OFFSET_MAX_MM: f64 = 5.0;
/// The smallest Thick parts value, in mm.
const THICK_PARTS_MIN_MM: f64 = 0.1;
/// The tolerance of the painted outline, as the canvas draws a profiled path (ADR 0053).
const PAINTED_TOLERANCE_MM: f64 = 0.01;

/// The options from the bar's numbers: `overlap` a fraction (-0.5 to 0.5), `thick_parts_mm` up
/// to [`THICK_PARTS_MAX_MM`] (no limit), `optimise` 0 to 1, `inner_offset_mm` used when
/// `keep_inner`. A value that is not a number takes its default (NaN passes `clamp`); an
/// infinite Thick parts is no limit, a negative infinite one the default.
#[must_use]
pub fn centre_options(
    overlap: f64,
    thick_parts_mm: f64,
    optimise: f64,
    keep_inner: bool,
    inner_offset_mm: f64,
) -> CentreOptions {
    let defaults = CentreOptions::default();
    let finite = |value: f64, default: f64| if value.is_finite() { value } else { default };
    CentreOptions {
        overlap: finite(overlap, defaults.overlap).clamp(-0.5, 0.5),
        // `+inf` is past the maximum: no limit, as at 100 mm.
        thick_parts: {
            let mm = if thick_parts_mm.is_nan() || thick_parts_mm == f64::NEG_INFINITY {
                DEFAULT_THICK_PARTS.as_mm()
            } else {
                thick_parts_mm
            };
            (mm < THICK_PARTS_MAX_MM).then(|| Length::from_mm(mm.max(THICK_PARTS_MIN_MM)))
        },
        optimise: finite(optimise, defaults.optimise).clamp(0.0, 1.0),
        inner_offset: keep_inner.then(|| {
            Length::from_mm(
                finite(inner_offset_mm, DEFAULT_INNER_OFFSET_MM)
                    .clamp(-INNER_OFFSET_MAX_MM, INNER_OFFSET_MAX_MM),
            )
        }),
    }
}

/// One subject the worker holds.
struct Subject {
    outlines: SubjectOutlines,
    /// Its staged skeleton; `None` when the build failed (it is kept as it is).
    stages: Option<CentreStages>,
    /// The box around its anchors and handles.
    bounds: (Point, Point),
}

/// A hash of the exact bits of `outlines`, the memo's lookup key.
fn key_of(outlines: &SubjectOutlines) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (anchors, closed) in outlines {
        closed.hash(&mut hasher);
        anchors.len().hash(&mut hasher);
        for (point, handle_in, handle_out) in anchors {
            for v in [
                point.x,
                point.y,
                handle_in.x,
                handle_in.y,
                handle_out.x,
                handle_out.y,
            ] {
                v.to_bits().hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

/// The box around a subject's anchors and handles.
fn bounds_of(outlines: &SubjectOutlines) -> (Point, Point) {
    let mut low = Point::new(f64::INFINITY, f64::INFINITY);
    let mut high = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for (anchors, _) in outlines {
        for &(p, i, o) in anchors {
            for q in [p, p.translated(i), p.translated(o)] {
                low = Point::new(low.x.min(q.x), low.y.min(q.y));
                high = Point::new(high.x.max(q.x), high.y.max(q.y));
            }
        }
    }
    (low, high)
}

/// Whether box `a` lies inside box `b`.
fn box_inside(a: &(Point, Point), b: &(Point, Point)) -> bool {
    a.0.x >= b.0.x && a.1.x <= b.1.x && a.0.y >= b.0.y && a.1.y <= b.1.y
}

/// The exact numbers of a path, the key of its painted outline.
fn path_key(path: &CentrePath) -> Vec<u64> {
    let mut key = vec![
        u64::from(path.role == CentreRole::Branch),
        u64::from(path.closed),
        path.width.as_mm().to_bits(),
    ];
    for (p, i, o) in &path.anchors {
        key.extend([p.x, p.y, i.x, i.y, o.x, o.y].map(f64::to_bits));
    }
    for at in path.profile.positions() {
        key.extend([
            at.to_bits(),
            path.profile.sample(at).unwrap_or(1.0).to_bits(),
        ]);
    }
    key
}

/// The filled outline `path` paints, as the canvas draws it (ADR 0053's variable-width artwork),
/// flattened to polygons within the same tolerance.
fn painted(path: &CentrePath) -> Vec<Vec<Point>> {
    let style = StrokeShape {
        width: path.width,
        cap: match path.role {
            CentreRole::Main => LineCap::Round,
            CentreRole::Branch => LineCap::Butt,
        },
        join: LineJoin::Round,
        dash: Stroke::default().dash,
        markers: Stroke::default().markers,
    };
    variable_stroke_outline(
        Outline::new(&path.anchors, path.closed),
        &style,
        &path.profile,
        Tolerance::from_mm(PAINTED_TOLERANCE_MM),
    )
    .map(|outlines| {
        outlines
            .outlines()
            .iter()
            .map(|ring| {
                let mut bez = kurbo::BezPath::new();
                let at = |p: Point| kurbo::Point::new(p.x, p.y);
                for (k, a) in ring.iter().enumerate() {
                    if k == 0 {
                        bez.move_to(at(a.point));
                    } else {
                        let b = &ring[k - 1];
                        bez.curve_to(
                            at(b.point.translated(b.handle_out)),
                            at(a.point.translated(a.handle_in)),
                            at(a.point),
                        );
                    }
                }
                if let (Some(first), Some(last)) = (ring.first(), ring.last()) {
                    bez.curve_to(
                        at(last.point.translated(last.handle_out)),
                        at(first.point.translated(first.handle_in)),
                        at(first.point),
                    );
                }
                let mut points = Vec::new();
                kurbo::flatten(&bez, PAINTED_TOLERANCE_MM, |el| match el {
                    kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) => {
                        points.push(Point::new(p.x, p.y));
                    }
                    _ => {}
                });
                points
            })
            .collect()
    })
    .unwrap_or_default()
}

/// The worker's state: the subjects with their staged skeletons, the latest result and the
/// painted outlines of its paths.
#[derive(Default)]
pub struct CentreLinesWorker {
    subjects: Vec<Subject>,
    latest: Vec<SubjectLines>,
    painted: HashMap<Vec<u64>, Vec<Vec<Point>>>,
}

impl CentreLinesWorker {
    /// An empty worker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the subjects of `wire`: a subject with the same outlines as one held keeps its
    /// skeleton and memo, the others are built, the rest are dropped. `report` hears the share
    /// of subjects built, 0 to 100. The latest result is dropped.
    ///
    /// # Errors
    /// [`WireError`] for a malformed wire; nothing is changed then.
    pub fn set_subjects(
        &mut self,
        wire: &[f64],
        report: &mut dyn FnMut(u32),
    ) -> Result<(), WireError> {
        let incoming = decode_subjects(wire)?;
        let mut held: Vec<Option<Subject>> = self.subjects.drain(..).map(Some).collect();
        // The held subjects by the hash of their outlines' bits: a lookup, not a scan per
        // subject (equal outlines are still compared exactly).
        let mut by_key: HashMap<u64, Vec<usize>> = HashMap::new();
        for (i, subject) in held.iter().enumerate() {
            if let Some(subject) = subject {
                by_key.entry(key_of(&subject.outlines)).or_default().push(i);
            }
        }
        let total = incoming.len().max(1);
        let mut subjects = Vec::with_capacity(incoming.len());
        for (index, outlines) in incoming.into_iter().enumerate() {
            let kept = by_key
                .get(&key_of(&outlines))
                .and_then(|candidates| {
                    candidates
                        .iter()
                        .copied()
                        .find(|&i| held[i].as_ref().is_some_and(|s| s.outlines == outlines))
                })
                .and_then(|i| held[i].take());
            subjects.push(kept.unwrap_or_else(|| {
                let views: Vec<Outline<'_>> =
                    outlines.iter().map(|(a, c)| Outline::new(a, *c)).collect();
                let stages = CentreSkeleton::build(&views).ok().map(CentreStages::new);
                Subject {
                    bounds: bounds_of(&outlines),
                    outlines,
                    stages,
                }
            }));
            // invariant: a share in 0..=100.
            #[allow(clippy::cast_possible_truncation)]
            report(((index + 1) * 100 / total) as u32);
        }
        self.subjects = subjects;
        self.latest.clear();
        Ok(())
    }

    /// The result for `options`, kept as the latest; with `paint` each path's painted outline
    /// goes with it (the preview), as a wire.
    pub fn finish(&mut self, options: &CentreOptions, paint: bool) -> Vec<f64> {
        let mut results = Vec::with_capacity(self.subjects.len());
        for index in 0..self.subjects.len() {
            // The memo is taken out while the other subjects' outlines are read (no copies).
            let Some(mut stages) = self.subjects[index].stages.take() else {
                results.push(SubjectLines::default());
                continue;
            };
            let here = self.subjects[index].bounds;
            let cutouts: Vec<Outline<'_>> = if options.inner_offset.is_some() {
                // Only a shape whose box lies inside this subject's box can lie in one of its
                // holes.
                self.subjects
                    .iter()
                    .enumerate()
                    .filter(|(other, s)| *other != index && box_inside(&s.bounds, &here))
                    .flat_map(|(_, s)| &s.outlines)
                    .map(|(anchors, closed)| Outline::new(anchors, *closed))
                    .collect()
            } else {
                Vec::new()
            };
            let lines = stages.finish(options, &cutouts);
            self.subjects[index].stages = Some(stages);
            results.push(SubjectLines {
                lines: Some(lines),
                painted: Vec::new(),
            });
        }
        if paint {
            self.paint(&mut results);
        }
        let wire = encode_results(&results);
        for result in &mut results {
            result.painted.clear();
        }
        self.latest = results;
        wire
    }

    /// Adds each path's painted outline, memoised on the path's exact numbers.
    fn paint(&mut self, results: &mut [SubjectLines]) {
        let mut next = HashMap::new();
        for result in results.iter_mut() {
            let Some(lines) = &result.lines else {
                continue;
            };
            result.painted = lines
                .paths
                .iter()
                .map(|path| {
                    let key = path_key(path);
                    let rings = self
                        .painted
                        .remove(&key)
                        .or_else(|| next.get(&key).cloned())
                        .unwrap_or_else(|| painted(path));
                    next.insert(key, rings.clone());
                    rings
                })
                .collect();
        }
        self.painted = next;
    }

    /// The latest result as a wire for Apply (without painted outlines); empty before the first
    /// [`CentreLinesWorker::finish`].
    #[must_use]
    pub fn result(&self) -> Vec<f64> {
        if self.latest.is_empty() && !self.subjects.is_empty() {
            return Vec::new();
        }
        encode_results(&self.latest)
    }
}
