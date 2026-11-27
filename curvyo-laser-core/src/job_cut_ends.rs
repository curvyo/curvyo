//! The burn of one entered cut outline with its ends (`specs/0139` criteria 14 to 16, 19, 20,
//! adrs L8, L12): the pieces, the overcut past the closing point, the lead-in before the first
//! burnt point and the lead-out after the last, and the marks they leave.

use crate::cut_entry_records::OutlineEntry;
use crate::job_cut_checks::touches_own;
use crate::job_cut_lead::{inside_left, lead_in, lead_out, rounded, waste_normal};
use crate::job_cut_path::{CutEntry, CutRun, SPECK};
use crate::job_input::PointUm;
use crate::operation_path_options::LeadKind;

/// What one entered outline leaves for the Operations view and the Checks (adrs L11, L12).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CutEnds {
    /// The lead-in from its start to the first burnt point; empty without one.
    pub lead_in: Vec<PointUm>,
    /// The overcut along the path; empty without one.
    pub overcut: Vec<PointUm>,
    /// The lead-out from the last burnt point; empty without one.
    pub lead_out: Vec<PointUm>,
    /// Where the burn begins on the path and its unit direction there times 1,000,000, for a set
    /// start or a fixed direction (the chevron of UX 6.1); `None` for Automatic.
    pub start: Option<(PointUm, (i32, i32))>,
    /// The first burnt point on the path, where the job really enters it (C7).
    pub entered: Option<PointUm>,
    /// Whether the lead-in and the lead-out come back onto their own outline (criterion 18).
    pub(crate) own_touch: [bool; 2],
}

fn millionths(value: f64) -> i32 {
    #[allow(clippy::cast_possible_truncation)]
    // invariant: a unit vector component times 10^6
    let v = (value * 1_000_000.0).round() as i32;
    v
}

impl CutRun {
    /// The burn paths of the outline entered by `entry`, in run order, with its ends: the
    /// pieces (tabs win, criterion 15), the overcut appended to the last piece when it closes at
    /// the entry, `min(d, L)` and stopping at the first tab, the lead-in prepended to the first
    /// piece and the lead-out appended to the last (closed outlines only, criterion 13).
    pub(crate) fn run(&self, entry: CutEntry) -> (Vec<Vec<PointUm>>, CutEnds) {
        let mut pieces = self.pieces(entry);
        let mut ends = CutEnds::default();
        if pieces.is_empty() {
            return (pieces, ends);
        }
        let m = &self.measured;
        ends.entered = pieces[0].first().copied();
        let CutEntry::Closed(at) = entry else {
            if self.entry.is_some() {
                let piece = &pieces[0];
                ends.start = Some((piece[0], direction(piece[0], piece[1])));
            }
            return (pieces, ends);
        };
        let stretches = self.stretches_from(at);
        let (first, last) = (stretches[0], stretches[stretches.len() - 1]);
        let end = self.overcut_into(at, first, last, &mut pieces, &mut ends);
        if let Some(OutlineEntry::Start(_)) = self.entry {
            let (dx, dy) = m.tangent_at(at + first.0);
            ends.start = Some((pieces[0][0], (millionths(dx), millionths(dy))));
        }
        if let Some(inside) = inside_left(m.points()) {
            self.lead_in_into(at + first.0, inside, &mut pieces, &mut ends);
            self.lead_out_into(end, inside, &mut pieces, &mut ends);
        }
        (pieces, ends)
    }

    /// Appends the overcut to the last piece when it closes at the entry `at` (the first burnt
    /// stretch `first` starts there, the last `last` ends there): `min(d, L)`, stopping at the
    /// first tab. Returns the arc where the burn ends.
    fn overcut_into(
        &self,
        at: f64,
        first: (f64, f64),
        last: (f64, f64),
        pieces: &mut [Vec<PointUm>],
        ends: &mut CutEnds,
    ) -> f64 {
        let length = self.measured.length();
        let overcut = um(self.path.overcut.um());
        if overcut <= 0.0 || last.1 < length - SPECK || first.0 > SPECK {
            return at + last.1;
        }
        let run_on = overcut.min(length).min(first.1);
        if run_on <= SPECK {
            return at + last.1;
        }
        let stretch = self.polyline(at + length, at + length + run_on);
        if let Some(piece) = pieces.last_mut() {
            piece.extend(stretch.iter().skip(1).copied());
            piece.dedup();
        }
        ends.overcut = stretch;
        at + length + run_on
    }

    /// Prepends the lead-in that ends at the first burnt point, arc `from`.
    fn lead_in_into(
        &self,
        from: f64,
        inside: bool,
        pieces: &mut [Vec<PointUm>],
        ends: &mut CutEnds,
    ) {
        if self.path.lead_in == LeadKind::None {
            return;
        }
        let m = &self.measured;
        let t = junction_direction(m, from, true);
        let n = waste_normal(t, inside, self.depth);
        let length = um(self.path.lead_in_length.um());
        let mut lead = rounded(&lead_in(self.path.lead_in, length, m.point_at(from), t, n));
        lead.pop();
        lead.push(pieces[0][0]);
        lead.dedup();
        let mut first_piece = lead.clone();
        first_piece.extend(pieces[0].iter().skip(1).copied());
        pieces[0] = first_piece;
        let mut from_junction = lead.clone();
        from_junction.reverse();
        ends.own_touch[0] = touches_own(&from_junction, m.points());
        ends.lead_in = lead;
    }

    /// Appends the lead-out that starts at the last burnt point, arc `end`.
    fn lead_out_into(
        &self,
        end: f64,
        inside: bool,
        pieces: &mut [Vec<PointUm>],
        ends: &mut CutEnds,
    ) {
        if self.path.lead_out == LeadKind::None {
            return;
        }
        let m = &self.measured;
        let t = junction_direction(m, end, false);
        let n = waste_normal(t, inside, self.depth);
        let length = um(self.path.lead_out_length.um());
        let mut lead = rounded(&lead_out(self.path.lead_out, length, m.point_at(end), t, n));
        if let Some(piece) = pieces.last_mut()
            && let Some(&junction) = piece.last()
        {
            lead[0] = junction;
            lead.dedup();
            piece.extend(lead.iter().skip(1).copied());
        }
        ends.own_touch[1] = touches_own(&lead, m.points());
        ends.lead_out = lead;
    }
}

/// A turn below this at a vertex is a flattened curve, not a corner (cos 30°).
const SMOOTH_TURN_COS: f64 = 0.866_025_403_784_438_6;

/// The tangent of a flattened curve at vertex arc `v`: the mean of the arriving and the leaving
/// segment, `None` at a corner (a turn of 30° or more) or an open path's end.
fn smooth_at(m: &crate::cut_tabs::Measured, v: f64) -> Option<(f64, f64)> {
    if !m.closed() && (v <= 0.0 || v >= m.length()) {
        return None;
    }
    let out = m.tangent_at(v);
    let into = m.tangent_before(v);
    if out.0 * into.0 + out.1 * into.1 < SMOOTH_TURN_COS {
        return None;
    }
    let (x, y) = (out.0 + into.0, out.1 + into.1);
    let norm = (x * x + y * y).sqrt();
    Some((x / norm, y / norm))
}

/// The path's direction at the junction `arc` of a lead (criterion 14): at a corner the segment
/// that leaves (`leaving`, a lead-in) or arrives (a lead-out); on a flattened curve (both ends of
/// the segment turn under 30°) the curve's tangent, the smooth tangents of the segment's ends
/// blended by position, so a tangent Arc does not cross the curve it is tangent to.
fn junction_direction(m: &crate::cut_tabs::Measured, arc: f64, leaving: bool) -> (f64, f64) {
    let fallback = if leaving {
        m.tangent_at(arc)
    } else {
        m.tangent_before(arc)
    };
    let arcs = m.vertex_arcs();
    let length = m.length();
    if arcs.len() < 2 || length <= 0.0 {
        return fallback;
    }
    let pos = m.wrap(arc);
    let i = arcs.partition_point(|&v| v <= pos).saturating_sub(1);
    let start = arcs[i];
    let end = arcs.get(i + 1).copied().unwrap_or(length);
    let (Some(s0), Some(s1)) = (smooth_at(m, start), smooth_at(m, end)) else {
        return fallback;
    };
    let along = if end > start {
        ((pos - start) / (end - start)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (x, y) = (s0.0 + (s1.0 - s0.0) * along, s0.1 + (s1.1 - s0.1) * along);
    let norm = (x * x + y * y).sqrt();
    if norm > 0.0 {
        (x / norm, y / norm)
    } else {
        fallback
    }
}

fn um(value: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)] // at most 20,000 µm
    let v = value as f64;
    v
}

fn direction(a: PointUm, b: PointUm) -> (i32, i32) {
    let (dx, dy) = (um(b.x - a.x), um(b.y - a.y));
    let norm = (dx * dx + dy * dy).sqrt();
    if norm > 0.0 {
        (millionths(dx / norm), millionths(dy / norm))
    } else {
        (1_000_000, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cut_entry_records::{Direction, OutlineEntry};
    use crate::cut_item_records::PathFraction;
    use crate::cut_tabs::{Measured, TabSpan};
    use crate::operation_cut_options::CutLength;
    use crate::operation_path_options::{Overcut, PathEdit, PathOptions};

    fn burn(pieces: &[Vec<PointUm>]) -> f64 {
        pieces
            .iter()
            .map(|p| p.windows(2).map(|w| w[0].distance_mm(w[1])).sum::<f64>())
            .sum()
    }

    fn square() -> Measured {
        // 40 x 40 mm drawn from the bottom-side midpoint (+x first): the inside is +y.
        Measured::new(
            vec![
                PointUm::new(20_000, 0),
                PointUm::new(40_000, 0),
                PointUm::new(40_000, 40_000),
                PointUm::new(0, 40_000),
                PointUm::new(0, 0),
            ],
            true,
        )
    }

    /// A polygon of `n` points on a circle of `circumference` mm (as flattened).
    fn circle(circumference: f64, n: usize) -> Measured {
        let r = circumference / std::f64::consts::TAU * 1000.0;
        let points = (0..n)
            .map(|i| {
                #[allow(clippy::cast_precision_loss)]
                let a = std::f64::consts::TAU * i as f64 / n as f64;
                #[allow(clippy::cast_possible_truncation)]
                PointUm::new((r * a.cos()).round() as i64, (r * a.sin()).round() as i64)
            })
            .collect();
        Measured::new(points, true)
    }

    fn options(edits: &[PathEdit]) -> PathOptions {
        edits.iter().fold(PathOptions::DEFAULT, |o, &e| o.with(e))
    }

    #[test]
    fn a_line_lead_in_starts_two_millimetres_outside_a_part_and_one_inside_a_hole() {
        let lead = options(&[
            PathEdit::LeadIn(LeadKind::Line),
            PathEdit::LeadInLength(CutLength::tab(200).unwrap()),
        ]);
        let run = CutRun::new(square(), Vec::new(), lead, None).unwrap();
        let (pieces, ends) = run.run(CutEntry::Closed(0.0));
        assert_eq!(pieces.len(), 1);
        assert_eq!(
            pieces[0][0],
            PointUm::new(20_000, -2_000),
            "2 mm below the side"
        );
        assert_eq!(pieces[0][1], PointUm::new(20_000, 0));
        assert_eq!(
            ends.lead_in,
            [PointUm::new(20_000, -2_000), PointUm::new(20_000, 0)]
        );
        assert!((burn(&pieces) - 162.0).abs() < 0.01);
        let mut hole = CutRun::new(square(), Vec::new(), lead, None).unwrap();
        hole.depth = 1;
        let (pieces, _) = hole.run(CutEntry::Closed(0.0));
        assert_eq!(pieces[0][0], PointUm::new(20_000, 2_000), "inside the hole");
    }

    #[test]
    fn an_overcut_runs_on_by_its_length_and_stops_at_the_first_tab() {
        let ring = circle(60.0, 600);
        let length = ring.length() / 1000.0;
        let cut = options(&[PathEdit::Overcut(Overcut::new(500).unwrap())]);
        let run = CutRun::new(ring.clone(), Vec::new(), cut, None).unwrap();
        let (pieces, ends) = run.run(CutEntry::Closed(0.0));
        assert!(
            (burn(&pieces) - (length + 5.0)).abs() < 0.01,
            "{}",
            burn(&pieces)
        );
        assert_ne!(ends.overcut.len(), 0);
        // One 2 mm tab starting 1 mm after the start point: the overcut stops at it.
        let tab = TabSpan {
            centre: 2_000.0,
            width: 2_000.0,
        };
        let start = OutlineEntry::Start(PathFraction::new(0).unwrap());
        let run = CutRun::new(ring.clone(), vec![tab], cut, Some(start)).unwrap();
        let entry = run.entries()[0].1;
        let (pieces, _) = run.run(entry);
        assert!(
            (burn(&pieces) - (length - 2.0 + 1.0)).abs() < 0.01,
            "{}",
            burn(&pieces)
        );
        // A tab over the closing point: no overcut, the lead-out sits at the last burnt point.
        let over = TabSpan {
            centre: 0.0,
            width: 2_000.0,
        };
        let both = options(&[
            PathEdit::Overcut(Overcut::new(500).unwrap()),
            PathEdit::LeadOut(LeadKind::Line),
        ]);
        let run = CutRun::new(ring, vec![over], both, Some(start)).unwrap();
        let entry = run.entries()[0].1;
        let (pieces, ends) = run.run(entry);
        assert_eq!(ends.overcut.len(), 0);
        assert!((burn(&pieces) - (length - 2.0 + 1.0)).abs() < 0.02);
        assert_eq!(ends.lead_out.len(), 2);
    }

    #[test]
    fn an_arc_lead_that_hugs_a_hole_wall_from_its_junction_does_not_touch_it() {
        // A 10 mm hole flattened as the job flattens it, Arc 4 mm inside: tangent at the junction
        // and drawing away from the wall, only the chords' sagitta brings them near (criterion 18).
        let hole = circle(std::f64::consts::PI * 10.0, 52);
        for (kind, length) in [
            (LeadKind::Arc, 400),
            (LeadKind::Arc, 200),
            (LeadKind::Line, 400),
        ] {
            let lead = options(&[
                PathEdit::LeadIn(kind),
                PathEdit::LeadInLength(CutLength::tab(length).unwrap()),
                PathEdit::LeadOut(kind),
                PathEdit::LeadOutLength(CutLength::tab(length).unwrap()),
            ]);
            let mut run = CutRun::new(hole.clone(), Vec::new(), lead, None).unwrap();
            run.depth = 1;
            for arc in [0.0, 1_234.0, 17_000.0] {
                let (_, ends) = run.run(CutEntry::Closed(arc));
                assert_eq!(ends.own_touch, [false, false], "{kind:?} {length} at {arc}");
            }
        }
    }

    #[test]
    fn a_set_start_and_a_fixed_direction_leave_their_chevron() {
        let start = OutlineEntry::Start(PathFraction::new(0).unwrap());
        let run = CutRun::new(square(), Vec::new(), PathOptions::DEFAULT, Some(start)).unwrap();
        let (pieces, ends) = run.run(run.entries()[0].1);
        assert_eq!(pieces[0][0], PointUm::new(20_000, 0));
        assert_eq!(ends.start, Some((PointUm::new(20_000, 0), (1_000_000, 0))));
        let line = Measured::new(vec![PointUm::new(0, 0), PointUm::new(9_000, 0)], false);
        let reversed = OutlineEntry::Direction(Direction::Reversed);
        let open = CutRun::new(line, Vec::new(), PathOptions::DEFAULT, Some(reversed)).unwrap();
        let (pieces, ends) = open.run(open.entries()[0].1);
        assert_eq!(pieces[0], [PointUm::new(9_000, 0), PointUm::new(0, 0)]);
        assert_eq!(ends.start, Some((PointUm::new(9_000, 0), (-1_000_000, 0))));
    }
}
