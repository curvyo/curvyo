//! The burnt pieces of one cut outline with tabs (`specs/0139` criteria 8 to 10, adrs L7, L8,
//! L12): where it may be entered, the pieces in run order from an entry and the marks of its gaps.

use crate::cut_entry_records::{Direction, OutlineEntry};
use crate::cut_item_records::PathFraction;
use crate::cut_tabs::{Measured, TabSpan, automatic};
use crate::job_input::PointUm;
use crate::operation_cut_options::CutSettings;
use crate::operation_path_options::PathOptions;

/// A stretch shorter than this (µm of arc) is no piece: it rounds onto one point.
pub(crate) const SPECK: f64 = 0.5;

/// The tabs of one outline unit before the order knows its parent.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct UnitCut {
    /// The outline with its arc lengths.
    pub measured: Measured,
    /// The own tabs' spans, `None` when the outline follows the operation.
    pub own: Option<Vec<TabSpan>>,
    /// The operation's cut settings.
    pub settings: CutSettings,
    /// The lead-in, lead-out and overcut that run (the item's own or the operation's).
    pub path: PathOptions,
    /// The outline's set start or fixed direction, `None` for Automatic (Frage 414 A).
    pub entry: Option<OutlineEntry>,
}

impl UnitCut {
    /// The spans the outline runs with: its own, else the operation's placement unless it has a
    /// parent under Skip inner shapes (criterion 3).
    pub(crate) fn spans(&self, has_parent: bool) -> Vec<TabSpan> {
        match &self.own {
            Some(own) => own.clone(),
            None if has_parent && self.settings.tab_skip_inner => Vec::new(),
            None => automatic(
                self.measured.length(),
                self.settings.tab_count,
                self.settings.tab_width,
                self.measured.closed(),
            ),
        }
    }
}

/// One gap of the Operations view (criterion 7): its stretch of the outline, its centre and the
/// outline's direction there in millionths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabGap {
    /// The outline points of the gap, from its start to its end.
    pub points: Vec<PointUm>,
    /// Its centre.
    pub centre: PointUm,
    /// The outline's unit direction at the centre, times 1,000,000.
    pub direction: (i32, i32),
}

/// An outline whose tabs, path options and entry are resolved.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CutRun {
    pub(crate) measured: Measured,
    pub(crate) spans: Vec<TabSpan>,
    /// The path options; inert on an open outline (criterion 13).
    pub(crate) path: PathOptions,
    /// A set start (closed) or a fixed direction (open).
    pub(crate) entry: Option<OutlineEntry>,
    /// The nesting depth among the closed paths of the block: even is a part edge, odd a hole.
    pub(crate) depth: usize,
}

/// How a cut run is entered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CutEntry {
    /// A closed outline at this arc, once around in its drawn direction.
    Closed(f64),
    /// An open outline from its first burnt point, or reversed from its last.
    Open { reversed: bool },
}

impl CutRun {
    /// The run of `measured` with `spans`, `path` and `entry`; `None` without a tab, a lead, an
    /// overcut or an entry (today's code path, criterion 24). Leads and overcut only apply to a
    /// closed outline, a start only to a closed one, a direction only to an open one.
    pub(crate) fn new(
        measured: Measured,
        spans: Vec<TabSpan>,
        path: PathOptions,
        entry: Option<OutlineEntry>,
    ) -> Option<Self> {
        let closed = measured.closed();
        let path = if closed { path } else { PathOptions::DEFAULT };
        let entry = entry.filter(|entry| matches!(entry, OutlineEntry::Start(_)) == closed);
        (!spans.is_empty() || !path.is_inert() || entry.is_some()).then_some(Self {
            measured,
            spans,
            path,
            entry,
            depth: 0,
        })
    }

    /// The burnt stretches in arc order from the outline's first point; on a closed outline a
    /// stretch that runs over the start ends past its length.
    fn stretches(&self) -> Vec<(f64, f64)> {
        self.stretches_from(0.0)
    }

    /// The burnt stretches of the outline as arcs relative to `entry` (0 to its length).
    pub(crate) fn stretches_from(&self, entry: f64) -> Vec<(f64, f64)> {
        let length = self.measured.length();
        let mut cut: Vec<(f64, f64)> = Vec::new();
        for span in &self.spans {
            if self.measured.closed() {
                let start = (span.start() - entry).rem_euclid(length);
                let end = start + span.width.min(length);
                if end > length {
                    cut.push((start, length));
                    cut.push((0.0, end - length));
                } else {
                    cut.push((start, end));
                }
            } else {
                cut.push((span.start().max(0.0), span.end().min(length)));
            }
        }
        cut.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut out = Vec::new();
        let mut from = 0.0_f64;
        for (start, end) in cut {
            if start - from > SPECK {
                out.push((from, start));
            }
            from = from.max(end);
        }
        if length - from > SPECK {
            out.push((from, length));
        }
        out
    }

    /// Whether no piece burns (criterion 10).
    pub(crate) fn covered(&self) -> bool {
        self.stretches().is_empty()
    }

    fn inside_tab(&self, arc: f64) -> bool {
        let length = self.measured.length();
        self.spans.iter().any(|span| {
            let d = if self.measured.closed() {
                (arc - span.start()).rem_euclid(length)
            } else {
                arc - span.start()
            };
            d > 1e-6 && d < span.width - 1e-6
        })
    }

    /// The arc where a set start at `at` begins to burn: the start itself, or the end of the tab
    /// that covers it (criterion 19).
    pub(crate) fn start_arc(&self, at: PathFraction) -> f64 {
        let m = &self.measured;
        let arc = m.arc_of(at);
        let covering = self.spans.iter().find(|span| {
            let d = (arc - span.start()).rem_euclid(m.length());
            d < span.width - SPECK
        });
        covering.map_or(arc, |span| m.wrap(span.end()))
    }

    /// The entries in tie order (adrs L7, L10): a set start or a fixed direction is the one
    /// entry; else a closed outline's burnt vertices that are no piece start (else the middle of
    /// each piece), so every tab is one gap between two pieces, every vertex without tabs; an open
    /// one's first burnt point forward and last reversed.
    pub(crate) fn entries(&self) -> Vec<(PointUm, CutEntry)> {
        let stretches = self.stretches();
        if stretches.is_empty() {
            return Vec::new();
        }
        let m = &self.measured;
        if !m.closed() {
            let first = stretches[0].0;
            let last = stretches[stretches.len() - 1].1;
            let forward = (m.point_um(first), CutEntry::Open { reversed: false });
            let back = (m.point_um(last), CutEntry::Open { reversed: true });
            return match self.entry {
                Some(OutlineEntry::Direction(Direction::AsDrawn)) => vec![forward],
                Some(OutlineEntry::Direction(Direction::Reversed)) => vec![back],
                _ => vec![forward, back],
            };
        }
        if let Some(OutlineEntry::Start(at)) = self.entry {
            let arc = self.start_arc(at);
            return vec![(m.point_um(arc), CutEntry::Closed(arc))];
        }
        if self.spans.is_empty() {
            return m
                .vertex_arcs()
                .iter()
                .zip(m.points())
                .map(|(&arc, &p)| (p, CutEntry::Closed(arc)))
                .collect();
        }
        let out: Vec<(PointUm, CutEntry)> = m
            .vertex_arcs()
            .iter()
            .zip(m.points())
            .filter(|(arc, _)| !self.inside_tab(**arc) && !self.at_tab_end(**arc))
            .map(|(&arc, &p)| (p, CutEntry::Closed(arc)))
            .collect();
        if !out.is_empty() {
            return out;
        }
        // No burnt vertex: the middle of each piece, so the run starts and ends inside a piece
        // and every tab is one gap between two pieces (never the travel into the outline).
        stretches
            .iter()
            .map(|&(a, b)| {
                let arc = f64::midpoint(a, b).rem_euclid(m.length());
                (m.point_um(arc), CutEntry::Closed(arc))
            })
            .collect()
    }

    /// Whether `arc` is where a tab ends (a piece start): an entry there would make that tab the
    /// travel into the outline instead of a gap.
    fn at_tab_end(&self, arc: f64) -> bool {
        let length = self.measured.length();
        self.spans.iter().any(|span| {
            let d = (arc - span.end()).rem_euclid(length);
            d < SPECK || length - d < SPECK
        })
    }

    /// The pieces in run order from `entry`, each a µm polyline.
    pub(crate) fn pieces(&self, entry: CutEntry) -> Vec<Vec<PointUm>> {
        match entry {
            CutEntry::Closed(at) => self
                .stretches_from(at)
                .into_iter()
                .map(|(a, b)| self.polyline(at + a, at + b))
                .filter(|p| p.len() >= 2)
                .collect(),
            CutEntry::Open { reversed } => {
                let mut pieces: Vec<Vec<PointUm>> = self
                    .stretches()
                    .into_iter()
                    .map(|(a, b)| self.polyline(a, b))
                    .filter(|p| p.len() >= 2)
                    .collect();
                if reversed {
                    pieces.reverse();
                    for piece in &mut pieces {
                        piece.reverse();
                    }
                }
                pieces
            }
        }
    }

    /// The outline from arc `a` to arc `b` (b ≥ a; past the length wraps on a closed outline, up
    /// to three laps for an overcut), its ends interpolated and every vertex between, rounded to
    /// µm with repeats removed.
    pub(crate) fn polyline(&self, a: f64, b: f64) -> Vec<PointUm> {
        let m = &self.measured;
        let length = m.length();
        let mut out = vec![m.point_um(a)];
        let arcs = m.vertex_arcs();
        let n = arcs.len();
        let laps: &[f64] = if m.closed() { &[0.0, 1.0, 2.0] } else { &[0.0] };
        for &lap in laps {
            let base = lap * length;
            for (i, &arc) in arcs.iter().enumerate() {
                let at = base + arc;
                if at > a + 1e-9 && at < b - 1e-9 {
                    out.push(m.points()[i % n]);
                }
            }
        }
        out.push(m.point_um(b));
        out.dedup();
        out
    }

    /// The gaps for the Operations view (criterion 7).
    pub(crate) fn gaps(&self) -> Vec<TabGap> {
        let m = &self.measured;
        self.spans
            .iter()
            .map(|span| {
                let (start, end) = if m.closed() {
                    let start = span.start().rem_euclid(m.length());
                    (start, start + span.width.min(m.length()))
                } else {
                    (span.start().max(0.0), span.end().min(m.length()))
                };
                let (dx, dy) = m.tangent_at(span.centre);
                TabGap {
                    points: self.polyline(start, end.max(start)),
                    centre: m.point_um(span.centre),
                    direction: (millionths(dx), millionths(dy)),
                }
            })
            .collect()
    }
}

fn millionths(value: f64) -> i32 {
    #[allow(clippy::cast_possible_truncation)]
    // invariant: a unit vector component times 10^6
    let v = (value * 1_000_000.0).round() as i32;
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation_cut_options::{CutLength, TabCount};

    fn settings(count: u64, hundredths: i64) -> CutSettings {
        CutSettings {
            tab_count: TabCount::new(count).unwrap(),
            tab_width: CutLength::tab(hundredths).unwrap(),
            tab_skip_inner: false,
        }
    }

    fn run(points: Vec<PointUm>, closed: bool, count: u64, width: i64) -> CutRun {
        let unit = UnitCut {
            measured: Measured::new(points, closed),
            own: None,
            settings: settings(count, width),
            path: PathOptions::DEFAULT,
            entry: None,
        };
        let spans = unit.spans(false);
        CutRun::new(unit.measured, spans, unit.path, unit.entry).unwrap()
    }

    fn length(piece: &[PointUm]) -> f64 {
        piece.windows(2).map(|p| p[0].distance_mm(p[1])).sum()
    }

    #[test]
    fn a_line_with_three_tabs_burns_ninety_four_millimetres_in_four_pieces() {
        let line = run(
            vec![PointUm::new(0, 0), PointUm::new(100_000, 0)],
            false,
            3,
            200,
        );
        let pieces = line.pieces(CutEntry::Open { reversed: false });
        assert_eq!(pieces.len(), 4);
        let burn: f64 = pieces.iter().map(|p| length(p)).sum();
        assert!((burn - 94.0).abs() < 0.015, "burn {burn}");
        let gaps: f64 = pieces
            .windows(2)
            .map(|w| w[0].last().unwrap().distance_mm(w[1][0]))
            .sum();
        assert!((gaps - 6.0).abs() < 0.015, "gaps {gaps}");
        let back = line.pieces(CutEntry::Open { reversed: true });
        assert_eq!(back[0][0], PointUm::new(100_000, 0));
    }

    #[test]
    fn a_closed_square_entered_at_a_corner_runs_its_pieces_once_around() {
        let square = run(
            vec![
                PointUm::new(0, 40_000),
                PointUm::new(40_000, 40_000),
                PointUm::new(40_000, 0),
                PointUm::new(0, 0),
            ],
            true,
            4,
            200,
        );
        let entries = square.entries();
        assert_eq!(entries.len(), 4, "the four burnt corners");
        let pieces = square.pieces(entries[0].1);
        // From the corner: half a side, three side pieces, half a side back to the corner.
        assert_eq!(pieces.len(), 5);
        assert_eq!(pieces[0][0], PointUm::new(0, 40_000));
        assert_eq!(*pieces[4].last().unwrap(), PointUm::new(0, 40_000));
        assert_eq!(*pieces[0].last().unwrap(), PointUm::new(19_000, 40_000));
        assert_eq!(pieces[1][0], PointUm::new(21_000, 40_000));
        let burn: f64 = pieces.iter().map(|p| length(p)).sum();
        assert!((burn - 152.0).abs() < 0.01);
        let gaps = square.gaps();
        assert_eq!(gaps.len(), 4);
        assert_eq!(gaps[0].centre, PointUm::new(20_000, 40_000));
        assert_eq!(gaps[0].direction, (1_000_000, 0));
        assert_eq!(
            gaps[0].points,
            [PointUm::new(19_000, 40_000), PointUm::new(21_000, 40_000)]
        );
    }

    #[test]
    fn tabs_that_cover_the_path_leave_no_piece() {
        let measured = Measured::new(vec![PointUm::new(0, 0), PointUm::new(4_000, 0)], false);
        let covered = CutRun::new(
            measured,
            vec![TabSpan {
                centre: 2_000.0,
                width: 4_000.0,
            }],
            PathOptions::DEFAULT,
            None,
        )
        .unwrap();
        assert!(covered.covered());
        assert_eq!(covered.entries(), Vec::new());
    }

    fn square() -> Measured {
        Measured::new(
            vec![
                PointUm::new(0, 0),
                PointUm::new(40_000, 0),
                PointUm::new(40_000, 40_000),
                PointUm::new(0, 40_000),
            ],
            true,
        )
    }

    #[test]
    fn a_set_start_or_a_fixed_direction_is_the_one_entry_and_defaults_take_no_run() {
        assert_eq!(
            CutRun::new(square(), Vec::new(), PathOptions::DEFAULT, None),
            None
        );
        let start = OutlineEntry::Start(PathFraction::new(62_500_000).unwrap());
        let set = CutRun::new(square(), Vec::new(), PathOptions::DEFAULT, Some(start)).unwrap();
        assert_eq!(set.entries().len(), 1);
        assert_eq!(set.entries()[0].0, PointUm::new(10_000, 0));
        // A direction on a closed outline and a start on an open one are ignored.
        let reversed = OutlineEntry::Direction(Direction::Reversed);
        assert_eq!(
            CutRun::new(square(), Vec::new(), PathOptions::DEFAULT, Some(reversed)),
            None
        );
        let line = Measured::new(vec![PointUm::new(0, 0), PointUm::new(9_000, 0)], false);
        let open = CutRun::new(line, Vec::new(), PathOptions::DEFAULT, Some(reversed)).unwrap();
        assert_eq!(
            open.entries(),
            [(PointUm::new(9_000, 0), CutEntry::Open { reversed: true })]
        );
        // A start inside a tab begins at the tab's end.
        let tab = TabSpan {
            centre: 10_000.0,
            width: 2_000.0,
        };
        let covered = CutRun::new(square(), vec![tab], PathOptions::DEFAULT, Some(start)).unwrap();
        assert_eq!(covered.entries()[0].0, PointUm::new(11_000, 0));
    }
}
