//! Cutting the raw pieces at the region (`specs/0071-pattern-fill` criteria 13 to 16, 29): each
//! piece kept, cut or dropped by where it lies, or with whole pieces only kept or dropped whole,
//! its work charged to the pattern's budget before it is done (acceptance report finding 2).

use curvyo_document_core::{Length, PatternMode, Point};

use super::clip::{corner, folds, grid_line, grid_points, intersect, merge, strip, union};
use super::cost::{Budget, Step, Strips};
use super::local::LocalRegions;
use super::region::{Place, Region};
use super::string_clip::StringClipper;
use super::{PatternError, PatternOutline, PatternPiece, Raw};

/// Lines whose strips are cut in one overlay, at most: overlays of many separate strips grow
/// faster than their size (`plan.md`, "Measurements").
const BATCH: usize = 32;
use crate::boolean_grid::{Path64, Paths64, Point64, double_area, snap, to_millimetres};

/// The winding number of the closed `ring` (first point repeated last) around `p`, which does not
/// lie on it.
fn winding(ring: &Path64, p: Point64) -> i32 {
    let mut w = 0;
    for pair in ring.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let side = i128::from(b.x - a.x) * i128::from(p.y - a.y)
            - i128::from(p.x - a.x) * i128::from(b.y - a.y);
        if a.y <= p.y {
            if b.y > p.y && side > 0 {
                w += 1;
            }
        } else if b.y <= p.y && side < 0 {
            w -= 1;
        }
    }
    w
}

/// The cutting of one pattern's pieces at its region.
pub(crate) struct Cutter<'r> {
    region: &'r Region,
    mode: PatternMode,
    width_units: i64,
    flatten_mm: f64,
    /// How the strips of Shapes mode are cut.
    strips: Strips,
    local: LocalRegions<'r>,
    clipper: StringClipper<'r>,
    /// Every piece left to cut was found whole (criterion 29): it lies inside, nothing is cut.
    all_whole: bool,
}

fn edges(paths: &Paths64) -> u64 {
    paths.iter().map(|p| p.len() as u64).sum()
}

fn units(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// A grid contour as a closed outline of corners in millimetres.
fn grid_outline(contour: &Path64) -> PatternOutline {
    (
        contour.iter().map(|p| corner(to_millimetres(p))).collect(),
        true,
    )
}

fn shape_piece(shape: &Paths64) -> PatternPiece {
    PatternPiece {
        outlines: shape.iter().map(grid_outline).collect(),
    }
}

/// The smallest and largest point of `paths`.
fn grid_box(paths: &Paths64) -> Option<(Point64, Point64)> {
    let points = || paths.iter().flatten();
    Some((
        Point64::new(points().map(|p| p.x).min()?, points().map(|p| p.y).min()?),
        Point64::new(points().map(|p| p.x).max()?, points().map(|p| p.y).max()?),
    ))
}

impl<'r> Cutter<'r> {
    pub(crate) fn new(
        region: &'r Region,
        mode: PatternMode,
        line_width: Length,
        flatten_mm: f64,
        strips: Strips,
    ) -> Self {
        Self {
            region,
            mode,
            width_units: snap(Point::new(line_width.as_mm(), 0.0)).x,
            flatten_mm,
            strips,
            local: LocalRegions::new(region),
            clipper: StringClipper::new(region),
            all_whole: false,
        }
    }

    const fn strip(&self, raw: &Raw) -> bool {
        !raw.closed && matches!(self.mode, PatternMode::Shapes)
    }

    /// Where `raw` lies; a strip by its line's box grown by the strip's width.
    fn place(&self, raw: &Raw) -> Place {
        if self.all_whole || (raw.on_boundary && !self.strip(raw)) {
            return Place::Inside;
        }
        let (lo, hi) = raw.grid_box();
        let w = if self.strip(raw) { self.width_units } else { 0 };
        self.region.place(
            Point64::new(lo.x - w, lo.y - w),
            Point64::new(hi.x + w, hi.y + w),
        )
    }

    /// The pieces of `raw` the region leaves whole (criterion 29), in generation order, after
    /// which every piece is taken as inside; a piece that follows another is kept only with it.
    ///
    /// # Errors
    /// [`PatternError::TooDetailed`] as soon as the work would go over `budget`.
    pub(crate) fn keep_whole(
        &mut self,
        raw: Vec<Raw>,
        budget: &mut Budget,
    ) -> Result<Vec<Raw>, PatternError> {
        let mut out: Vec<Raw> = Vec::new();
        let (mut group_start, mut group_whole) = (0, true);
        for raw in raw {
            if !raw.follows {
                if !group_whole {
                    out.truncate(group_start);
                }
                (group_start, group_whole) = (out.len(), true);
            }
            if group_whole && self.is_whole(&raw, budget)? {
                out.push(raw);
            } else {
                group_whole = false;
            }
        }
        if !group_whole {
            out.truncate(group_start);
        }
        self.all_whole = true;
        Ok(out)
    }

    /// Whether the region leaves `raw` whole: it does not meet the boundary, lies inside, and (a
    /// closed piece) holds no hole or island of the region; a strip also keeps its width inside.
    fn is_whole(&mut self, raw: &Raw, budget: &mut Budget) -> Result<bool, PatternError> {
        match self.place(raw) {
            Place::Inside => return Ok(true),
            Place::Outside => return Ok(false),
            Place::Boundary => {}
        }
        let mut path = grid_points(&raw.anchors, raw.closed, self.flatten_mm);
        let Some(&first) = path.first() else {
            return Ok(false);
        };
        if raw.closed {
            path.push(first);
        }
        let tests = self.clipper.tests;
        budget.spend(&[(Step::OverlayEdge, 2 * units(path.len()))])?;
        let met = self.clipper.crossings(&path) > 0;
        let inside = !met && self.clipper.contains(first);
        budget.spend(&[(Step::EdgeTest, self.clipper.tests - tests)])?;
        if !inside {
            return Ok(false);
        }
        if raw.closed {
            return self.holds_no_contour(&path, budget);
        }
        if self.strip(raw) {
            return self.strip_inside(&path, budget);
        }
        Ok(true)
    }

    /// Whether no contour of the region lies inside the closed `ring` (first point repeated last),
    /// which meets no contour: a hole or island inside a piece would be cut out of it.
    fn holds_no_contour(&self, ring: &Path64, budget: &mut Budget) -> Result<bool, PatternError> {
        let Some((lo, hi)) = grid_box(&vec![ring.clone()]) else {
            return Ok(true);
        };
        let contours = &self.region.contours;
        budget.spend(&[(Step::EdgeTest, units(contours.len()))])?;
        for contour in contours {
            let Some(&p) = contour.first() else {
                continue;
            };
            if p.x < lo.x || p.y < lo.y || p.x > hi.x || p.y > hi.y {
                continue;
            }
            budget.spend(&[(Step::EdgeTest, units(ring.len()))])?;
            if winding(ring, p) != 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether the strip along `string`, whose line lies inside, keeps its width inside too.
    fn strip_inside(&mut self, string: &Path64, budget: &mut Budget) -> Result<bool, PatternError> {
        let shape = strip(string, self.width_units);
        let Some((lo, hi)) = grid_box(&shape) else {
            return Ok(false);
        };
        let (around, scanned) = self.local.around(lo, hi);
        budget.spend(&[
            (Step::Overlay, 2),
            (Step::StripOverlayEdge, 2 * edges(&shape) + edges(&around)),
            (Step::Scan, units(scanned)),
        ])?;
        let area = |shapes: Vec<Paths64>| -> i128 {
            shapes.iter().flatten().map(|c| double_area(c)).sum()
        };
        Ok(area(intersect(&shape, &around)) == area(union(&shape)))
    }

    /// The pieces of `raw` within the region, in generation order; strips cut together last.
    ///
    /// # Errors
    /// [`PatternError::TooDetailed`] as soon as the work would go over `budget`.
    pub(crate) fn cut(
        &mut self,
        raw: Vec<Raw>,
        budget: &mut Budget,
    ) -> Result<Vec<PatternPiece>, PatternError> {
        let mut out = Vec::new();
        // The strips cut together, in batches of lines in generation order, with their crossings
        // of the boundary.
        let mut batches: Vec<(Paths64, u64)> = Vec::new();
        let mut in_batch = 0;
        for raw in raw {
            let place = self.place(&raw);
            match place {
                Place::Outside => {}
                _ if self.strip(&raw) => {
                    let string = grid_points(&raw.anchors, false, self.flatten_mm);
                    budget.spend(&[
                        (Step::Strip, 1),
                        (Step::StripPoint, units(string.len())),
                        (Step::StripFold, folds(&string, self.width_units)),
                    ])?;
                    let crossings = if place == Place::Inside {
                        0
                    } else {
                        let tests = self.clipper.tests;
                        let crossings = self.clipper.crossings(&string);
                        budget.spend(&[(Step::EdgeTest, self.clipper.tests - tests)])?;
                        crossings
                    };
                    let shapes = strip(&string, self.width_units);
                    if self.strips == Strips::EachOwn {
                        self.cut_strip(&shapes, place, crossings, budget, &mut out)?;
                        continue;
                    }
                    let in_batches = matches!(self.strips, Strips::Batches { .. });
                    if batches.is_empty() || (in_batches && in_batch == BATCH) {
                        batches.push((Paths64::new(), 0));
                        in_batch = 0;
                    }
                    in_batch += 1;
                    if let Some((batch, batch_crossings)) = batches.last_mut() {
                        batch.extend(shapes);
                        *batch_crossings += crossings;
                    }
                }
                Place::Inside => out.push(PatternPiece {
                    outlines: vec![(raw.anchors, raw.closed)],
                }),
                Place::Boundary if raw.closed => {
                    let polygon = grid_points(&raw.anchors, true, self.flatten_mm);
                    let (lo, hi) = raw.grid_box();
                    let (around, scanned) = self.local.around(lo, hi);
                    budget.spend(&[
                        (Step::Overlay, 1),
                        (Step::OverlayEdge, units(polygon.len())),
                        (Step::AroundEdge, edges(&around)),
                        (Step::AroundPair, edges(&around).saturating_pow(2)),
                        (Step::Scan, units(scanned)),
                    ])?;
                    out.extend(intersect(&vec![polygon], &around).iter().map(shape_piece));
                }
                Place::Boundary => {
                    let (grid, exact) = grid_line(&raw.anchors, self.flatten_mm);
                    budget.spend(&[(Step::Line, 1), (Step::LinePoint, units(grid.len()))])?;
                    let tests = self.clipper.tests;
                    for part in self.clipper.clip(&grid, &exact) {
                        out.push(PatternPiece {
                            outlines: vec![(part.into_iter().map(corner).collect(), false)],
                        });
                    }
                    budget.spend(&[(Step::EdgeTest, self.clipper.tests - tests)])?;
                }
            }
        }
        self.cut_batches(batches, budget, &mut out)?;
        Ok(out)
    }

    /// The strips cut together: crossing strips merged into one outline (a laser cuts each
    /// crossing once, acceptance report finding 4); parallel strips batch by batch, each batch's
    /// pieces merged with the next's when strips can overlap.
    fn cut_batches(
        &self,
        batches: Vec<(Paths64, u64)>,
        budget: &mut Budget,
        out: &mut Vec<PatternPiece>,
    ) -> Result<(), PatternError> {
        let region = &self.region.contours;
        match self.strips {
            Strips::EachOwn => {}
            Strips::Merged(crossings) => {
                for (strips, strip_crossings) in batches {
                    budget.spend(&[
                        crossings,
                        (Step::StripEdge, strip_crossings),
                        (Step::MergedEdge, edges(&strips) + edges(region)),
                    ])?;
                    let rounded = crossings.0 == Step::RoundedCrossing;
                    out.extend(merge(&strips, region, rounded).iter().map(shape_piece));
                }
            }
            Strips::Batches { overlap } => {
                let mut cut: Vec<Paths64> = Vec::new();
                for (strips, strip_crossings) in batches {
                    budget.spend(&[
                        (Step::Overlay, 1),
                        (Step::StripEdge, strip_crossings),
                        (Step::MergedEdge, edges(&strips) + edges(region)),
                    ])?;
                    let pieces = intersect(&strips, region);
                    if overlap {
                        cut.push(pieces.into_iter().flatten().collect());
                    } else {
                        out.extend(pieces.iter().map(shape_piece));
                    }
                }
                // Neighbouring batches merged pairwise until one is left.
                while cut.len() > 1 {
                    let mut next = Vec::with_capacity(cut.len().div_ceil(2));
                    let mut pairs = cut.into_iter();
                    while let Some(mut first) = pairs.next() {
                        if let Some(second) = pairs.next() {
                            first.extend(second);
                            budget
                                .spend(&[(Step::Overlay, 1), (Step::MergedEdge, edges(&first))])?;
                            first = union(&first).into_iter().flatten().collect();
                        }
                        next.push(first);
                    }
                    cut = next;
                }
                if let Some(merged) = cut.pop() {
                    out.extend(union(&merged).iter().map(shape_piece));
                }
            }
        }
        Ok(())
    }

    /// One strip of a line placed at `place`: kept whole when its line lies inside, cut when it
    /// meets the boundary (about `crossings` times).
    fn cut_strip(
        &mut self,
        shape: &Paths64,
        place: Place,
        crossings: u64,
        budget: &mut Budget,
        out: &mut Vec<PatternPiece>,
    ) -> Result<(), PatternError> {
        if place == Place::Inside {
            budget.spend(&[(Step::Overlay, 1), (Step::StripOverlayEdge, edges(shape))])?;
            out.extend(union(shape).iter().map(shape_piece));
            return Ok(());
        }
        let Some((lo, hi)) = grid_box(shape) else {
            return Ok(());
        };
        let (around, scanned) = self.local.around(lo, hi);
        budget.spend(&[
            (Step::Overlay, 1),
            (Step::StripOverlayEdge, edges(shape) + edges(&around)),
            (Step::StripEdge, crossings),
            (Step::Scan, units(scanned)),
        ])?;
        out.extend(intersect(shape, &around).iter().map(shape_piece));
        Ok(())
    }
}
