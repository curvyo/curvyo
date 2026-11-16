//! The numbers Find centre lines sends between the page and its worker (ADR 0063 amendment
//! 2026-11-18, W): the subjects' outlines in, each subject's result out, as flat `f64` lists that
//! travel as transferred `Float64Array`s.
//!
//! Subjects: `count`, then per subject `outlines`, per outline `closed, n` and `n` anchors of six
//! numbers (point, handle in, handle out). Results: `count`, then per subject `built, failed`,
//! its paths (`role, closed, width, n` anchors, `m` profile points `(position, multiplier)`,
//! `k` painted polygons of `n` points), its kept parts (`regions`, per region `rings`, per ring `n` anchors) and
//! its inner areas (`areas`, per area `n` points of two numbers).

use curvyo_document_core::{Length, Point, Vec2};
use curvyo_geometry_core::{
    CentreLines, CentrePath, CentreRole, OutlineTriple, WidthPoint, WidthProfile,
};

/// A wire that does not hold what its layout says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireError;

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("malformed centre lines wire")
    }
}

impl std::error::Error for WireError {}

/// One subject's outlines `(anchors, closed)`.
pub type SubjectOutlines = Vec<(Vec<OutlineTriple>, bool)>;

/// One subject's result: `None` when its skeleton could not be built (kept as it is).
#[derive(Debug, Clone, Default)]
pub struct SubjectLines {
    /// The kernel's result, `None` when the build failed.
    pub lines: Option<CentreLines>,
    /// Per path its painted outline (the variable-width artwork) as polygons, for the preview;
    /// empty in a result for Apply.
    pub painted: Vec<Vec<Vec<Point>>>,
}

/// Reads numbers in order.
struct Reader<'a> {
    wire: &'a [f64],
    at: usize,
}

impl Reader<'_> {
    fn number(&mut self) -> Result<f64, WireError> {
        let value = *self.wire.get(self.at).ok_or(WireError)?;
        self.at += 1;
        Ok(value)
    }

    fn count(&mut self) -> Result<usize, WireError> {
        let value = self.number()?;
        if value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= 1e9 {
            // invariant: a whole number in 0..=1e9.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Ok(value as usize)
        } else {
            Err(WireError)
        }
    }

    fn flag(&mut self) -> Result<bool, WireError> {
        Ok(self.number()? != 0.0)
    }

    fn triples(&mut self) -> Result<Vec<OutlineTriple>, WireError> {
        let n = self.count()?;
        if self.wire.len().saturating_sub(self.at) < n.saturating_mul(6) {
            return Err(WireError);
        }
        (0..n)
            .map(|_| {
                let mut v = [0.0; 6];
                for x in &mut v {
                    *x = self.number()?;
                }
                Ok((
                    Point::new(v[0], v[1]),
                    Vec2::new(v[2], v[3]),
                    Vec2::new(v[4], v[5]),
                ))
            })
            .collect()
    }

    fn points(&mut self) -> Result<Vec<Point>, WireError> {
        let n = self.count()?;
        if self.wire.len().saturating_sub(self.at) < n.saturating_mul(2) {
            return Err(WireError);
        }
        (0..n)
            .map(|_| Ok(Point::new(self.number()?, self.number()?)))
            .collect()
    }

    fn end(&self) -> Result<(), WireError> {
        (self.at == self.wire.len()).then_some(()).ok_or(WireError)
    }
}

#[allow(clippy::cast_precision_loss)] // counts far below 2^53
fn push_count(out: &mut Vec<f64>, n: usize) {
    out.push(n as f64);
}

fn push_triples(out: &mut Vec<f64>, triples: &[OutlineTriple]) {
    push_count(out, triples.len());
    for (p, i, o) in triples {
        out.extend([p.x, p.y, i.x, i.y, o.x, o.y]);
    }
}

/// The subjects as a wire.
#[must_use]
pub fn encode_subjects(subjects: &[SubjectOutlines]) -> Vec<f64> {
    let mut out = Vec::new();
    push_count(&mut out, subjects.len());
    for outlines in subjects {
        push_count(&mut out, outlines.len());
        for (anchors, closed) in outlines {
            out.push(f64::from(u8::from(*closed)));
            push_triples(&mut out, anchors);
        }
    }
    out
}

/// The subjects of a wire.
///
/// # Errors
/// [`WireError`] when the wire does not follow the layout.
pub fn decode_subjects(wire: &[f64]) -> Result<Vec<SubjectOutlines>, WireError> {
    let mut read = Reader { wire, at: 0 };
    let count = read.count()?;
    let mut subjects = Vec::with_capacity(count.min(wire.len()));
    for _ in 0..count {
        let outlines = read.count()?;
        let mut subject = Vec::with_capacity(outlines.min(wire.len()));
        for _ in 0..outlines {
            let closed = read.flag()?;
            subject.push((read.triples()?, closed));
        }
        subjects.push(subject);
    }
    read.end()?;
    Ok(subjects)
}

/// The profile of `path` as the document stores it: `(position, multiplier)` at each point.
fn profile_points(path: &CentrePath) -> Vec<(f64, f64)> {
    path.profile
        .positions()
        .map(|at| (at, path.profile.sample(at).unwrap_or(1.0)))
        .collect()
}

/// The results as a wire; `painted` rings are written where the subject has them.
#[must_use]
pub fn encode_results(results: &[SubjectLines]) -> Vec<f64> {
    let mut out = Vec::new();
    push_count(&mut out, results.len());
    for result in results {
        let Some(lines) = &result.lines else {
            out.extend([0.0, 0.0, 0.0, 0.0, 0.0]);
            continue;
        };
        out.push(1.0);
        push_count(&mut out, lines.failed);
        push_count(&mut out, lines.paths.len());
        for (k, path) in lines.paths.iter().enumerate() {
            out.push(match path.role {
                CentreRole::Main => 0.0,
                CentreRole::Branch => 1.0,
            });
            out.push(f64::from(u8::from(path.closed)));
            out.push(path.width.as_mm());
            push_triples(&mut out, &path.anchors);
            let points = profile_points(path);
            push_count(&mut out, points.len());
            for (at, multiplier) in points {
                out.extend([at, multiplier]);
            }
            let rings = result.painted.get(k).map_or(&[][..], Vec::as_slice);
            push_count(&mut out, rings.len());
            for ring in rings {
                push_count(&mut out, ring.len());
                for p in ring {
                    out.extend([p.x, p.y]);
                }
            }
        }
        push_count(&mut out, lines.filled.len());
        for region in &lines.filled {
            push_count(&mut out, region.len());
            for ring in region {
                push_triples(&mut out, ring);
            }
        }
        push_count(&mut out, lines.inner_areas.len());
        for area in &lines.inner_areas {
            push_count(&mut out, area.len());
            for p in area {
                out.extend([p.x, p.y]);
            }
        }
    }
    out
}

/// The results of a wire.
///
/// # Errors
/// [`WireError`] when the wire does not follow the layout or a profile is invalid.
pub fn decode_results(wire: &[f64]) -> Result<Vec<SubjectLines>, WireError> {
    let mut read = Reader { wire, at: 0 };
    let count = read.count()?;
    let mut results = Vec::with_capacity(count.min(wire.len()));
    for _ in 0..count {
        let built = read.flag()?;
        let failed = read.count()?;
        let mut lines = CentreLines {
            failed,
            ..CentreLines::default()
        };
        let mut painted = Vec::new();
        for _ in 0..read.count()? {
            let role = if read.flag()? {
                CentreRole::Branch
            } else {
                CentreRole::Main
            };
            let closed = read.flag()?;
            let width = Length::from_mm(read.number()?);
            let anchors = read.triples()?;
            let mut points = Vec::new();
            for id in 0..read.count()? {
                let (at, multiplier) = (read.number()?, read.number()?);
                points.push(WidthPoint::new(id as u128, at, multiplier, closed).ok_or(WireError)?);
            }
            let profile = WidthProfile::new(points, closed).ok_or(WireError)?;
            let mut rings = Vec::new();
            for _ in 0..read.count()? {
                rings.push(read.points()?);
            }
            painted.push(rings);
            lines.paths.push(CentrePath {
                role,
                anchors,
                closed,
                width,
                profile,
            });
        }
        for _ in 0..read.count()? {
            let mut region = Vec::new();
            for _ in 0..read.count()? {
                region.push(read.triples()?);
            }
            lines.filled.push(region);
        }
        for _ in 0..read.count()? {
            lines.inner_areas.push(read.points()?);
        }
        if !painted.iter().any(|rings: &Vec<_>| !rings.is_empty()) {
            painted.clear();
        }
        results.push(SubjectLines {
            lines: built.then_some(lines),
            painted,
        });
    }
    read.end()?;
    Ok(results)
}
