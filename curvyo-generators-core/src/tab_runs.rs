//! Flat windows on runs of bends (ADR 0083 §13.8): where the straights hold fewer than four tabs,
//! each run of consecutive arcs takes windows spread evenly along it, each a chord the tab stands
//! on that replaces its stretch.

use curvyo_document_core::Point;

use crate::footprint::{Footprint, Shape, Span, Tab};
use crate::shape_ops::Chain;
use crate::tab_layout::{
    Layout, MIN_CIRCLE_TABS, MIN_STRAIGHT_TABS, MIN_TAB_STRAIGHT, TAB_PITCH, WindowSpec,
};

/// A run as the windows see it: its shapes measured as one.
type Run = Chain;

impl Run {
    fn of(spans: &[Span]) -> Self {
        Chain::new(spans.iter().map(|s| s.shape).collect())
    }

    /// The half length of the window of chord `chord` centred at `centre`, `None` when no window
    /// of that chord fits round it.
    fn half_span(&self, centre: f64, chord: f64) -> Option<f64> {
        let reach = centre.min(self.total - centre);
        let chord_at = |a: f64| {
            let (p, q) = (self.at(centre - a), self.at(centre + a));
            libm::hypot(q.x - p.x, q.y - p.y)
        };
        if chord_at(reach) < chord {
            return None;
        }
        let (mut low, mut high) = (0.0, reach);
        for _ in 0..50 {
            let mid = f64::midpoint(low, high);
            if chord_at(mid) < chord {
                low = mid;
            } else {
                high = mid;
            }
        }
        Some(high)
    }
}

/// How the windows of a run are asked for.
struct Ask {
    spec: WindowSpec,
    wall: f64,
    /// Solid stretch kept at the run's start and end: the corner margin, or the seam's 2 t.
    margins: (f64, f64),
}

impl Run {
    /// `count` windows as `(start, end)` distances, or `None` when they do not fit with the slit
    /// stretches between them and to the run's ends.
    fn windows(&self, count: usize, ask: &Ask) -> Option<Vec<(f64, f64)>> {
        let chord = 2.0 * ask.spec.half_width + 2.0 * ask.wall;
        let (lo, hi) = (ask.margins.0, self.total - ask.margins.1);
        let mut out = Vec::new();
        for k in 0..count {
            #[allow(clippy::cast_precision_loss)]
            let centre = lo + (k as f64 + 0.5) / count as f64 * (hi - lo);
            let a = self.half_span(centre, chord)?;
            out.push((centre - a, centre + a));
        }
        let between_ok = out
            .windows(2)
            .all(|w| w[1].0 - w[0].1 >= ask.spec.min_band + 2.0 * ask.wall);
        let ends = ask.spec.min_band + ask.wall;
        let first = out[0].0 - lo;
        let last = hi - out[out.len() - 1].1;
        (between_ok && first >= ends && last >= ends).then_some(out)
    }
}

impl Footprint {
    /// The tabs of the whole wall and the footprint they stand on, runs of bends taking their
    /// windows as whole (ADR 0083 §13.8): the straights' tabs when they are at least four.
    pub(crate) fn with_run_tabs(&self, spec: WindowSpec) -> (Self, Vec<Tab>) {
        let straight = self.straight_tabs(spec.half_width);
        if straight.len() >= MIN_STRAIGHT_TABS {
            return (self.clone(), straight);
        }
        let spans = &self.spans;
        let is_bend = |i: usize| matches!(spans[i].shape, Shape::Bend { .. });
        let corners = spans.iter().any(|s| s.piece_start);
        let lone_loop = !corners && (0..spans.len()).all(is_bend);
        let mut out = Layout::new(self);
        let mut first = 0;
        while first < spans.len() {
            if !is_bend(first) {
                out.keep_straight(&spans[first], &straight);
                first += 1;
                continue;
            }
            let mut last = first;
            while last + 1 < spans.len() && is_bend(last + 1) && !spans[last + 1].piece_start {
                last += 1;
            }
            let run = Run::of(&spans[first..=last]);
            let ends_at_corner = spans.get(last + 1).map_or(corners, |s| s.piece_start);
            let seam = 2.0 * self.wall;
            let margin = |at_corner: bool, at_seam: bool| match (at_corner, at_seam) {
                (true, _) => self.margin,
                (false, true) => seam,
                (false, false) => 0.0,
            };
            let ask = Ask {
                spec,
                wall: self.wall,
                margins: (
                    margin(spans[first].piece_start, first == 0 && !corners),
                    margin(ends_at_corner, last + 1 == spans.len() && !corners),
                ),
            };
            let wanted = {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let by_length = (run.total / TAB_PITCH).round() as usize;
                by_length.max(if lone_loop { MIN_CIRCLE_TABS } else { 1 })
            };
            let enough = lone_loop || run.total >= MIN_TAB_STRAIGHT * self.wall;
            let windows = if enough {
                (1..=wanted)
                    .rev()
                    .find_map(|count| run.windows(count, &ask))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            let at = out.spans.len();
            out.run(&run, &windows);
            if let Some(span) = out.spans.get_mut(at) {
                span.piece_start = spans[first].piece_start;
            }
            first = last + 1;
        }
        out.finish()
    }
}

impl Layout {
    /// The run as arcs between its `windows`, each window a chord with its tab.
    fn run(&mut self, run: &Run, windows: &[(f64, f64)]) {
        let mut cursor = 0.0;
        for &(start, end) in windows {
            for shape in run.part(cursor, start) {
                self.push(shape, false);
            }
            let (a, b) = (run.at(start), run.at(end));
            let chord = libm::hypot(b.x - a.x, b.y - a.y);
            let replaced = run.turn_between(start, end);
            self.turn = replaced;
            self.tabs.push(Tab {
                s: self.s0 + f64::midpoint(chord, self.wall / 2.0 * replaced),
                centre: Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
                along: ((b.x - a.x) / chord, (b.y - a.y) / chord),
            });
            self.push(Shape::Straight { from: a, to: b }, true);
            cursor = end;
        }
        for shape in run.part(cursor, run.total) {
            self.push(shape, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A footprint of one concave arc (the centre outside), 30 mm radius, 60 degrees.
    fn concave_run() -> Footprint {
        let shape = Shape::Bend {
            centre: Point::new(0.0, 0.0),
            radius: 30.0,
            from: 0.0,
            sweep: -1.05,
        };
        let mut footprint = Footprint::from_shapes(&[shape], 3.0, 3.0);
        footprint.spans[0].piece_start = true;
        footprint
    }

    /// ADR 0083 §13.5: a concave window's neutral length is its chord less half a wall thickness
    /// times the turn it replaces, so it is shorter than the chord; a convex one is longer.
    #[test]
    fn a_concave_window_is_shorter_than_its_chord_on_the_neutral_line() {
        let spec = WindowSpec {
            half_width: 3.0,
            min_band: 3.0,
        };
        let fp = concave_run();
        let (fitted, tabs) = fp.with_run_tabs(spec);
        assert!(!tabs.is_empty(), "a window was cut");
        for span in fitted.spans.iter().filter(|s| s.window) {
            let Shape::Straight { from, to } = span.shape else {
                panic!("a window is a chord");
            };
            let chord = libm::hypot(to.x - from.x, to.y - from.y);
            assert!(span.length < chord, "{} against {chord}", span.length);
            let turn = 2.0 * libm::asin(chord / 60.0);
            assert!((span.length - (chord - 1.5 * turn)).abs() < 1e-3);
        }
    }
}
