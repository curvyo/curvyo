//! Where the plates' tabs stand: on the straights of the footprint, or, where those hold fewer
//! than four, in flat windows cut into the bends (ADR 0082 §5, Frage 360 B).

use curvyo_document_core::Point;

use crate::footprint::{Footprint, Shape, Span, Tab};
use crate::joint_layout::{self, Edge, Joints};
use crate::outline::polar;

/// The aimed distance between two tabs.
pub(crate) const TAB_PITCH: f64 = 30.0;
/// A straight must hold at least this many wall thicknesses to carry a tab.
pub(crate) const MIN_TAB_STRAIGHT: f64 = 4.0;
/// Fewer tabs than this on the straights, and the bends take tabs in windows.
pub(crate) const MIN_STRAIGHT_TABS: usize = 4;
/// A footprint without a straight takes at least this many tabs, as far as its stretches allow.
pub(crate) const MIN_CIRCLE_TABS: usize = 4;

/// The length of a span on the inner face.
fn inner_length(shape: &Shape) -> f64 {
    match *shape {
        Shape::Straight { from, to } => libm::hypot(to.x - from.x, to.y - from.y),
        Shape::Bend { radius, sweep, .. } => radius * sweep.abs(),
    }
}

/// The measures a window needs: the chord a tab of `half_width` stands on, and how long a curved
/// stretch between two windows must be to keep its slit columns.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowSpec {
    pub(crate) half_width: f64,
    /// The least length of the slit band of a stretch, without its two bridges.
    pub(crate) min_band: f64,
    /// A tab on a bend stands on the true arc and the slits run on through it (`bend_tabs`
    /// `curve`, ADR 0089 decision 2); where it cannot, the tab keeps its window.
    pub(crate) curve: bool,
    /// The count, size and pattern the maker chose (`half_width` already follows the size).
    pub(crate) joints: Joints,
}

impl WindowSpec {
    /// Straight tabs enough to leave the bends alone: four, or two when a count was typed.
    pub(crate) fn straight_enough(&self, tabs: usize) -> bool {
        tabs >= if self.joints.typed() {
            2
        } else {
            MIN_STRAIGHT_TABS
        }
    }
}

/// Whether a tab of half width `half_width` can stand on the true arc of `radius` (a concave arc
/// must also hold the tip, which lies one wall thickness nearer the centre).
pub(crate) fn curve_fits(radius: f64, convex: bool, half_width: f64, wall: f64) -> bool {
    radius > half_width + 1.0 && (convex || radius > half_width + wall + 1.0)
}

/// The neutral length of an arc of `radius` and `sweep` on a wall of thickness `wall`.
pub(crate) fn neutral_arc(radius: f64, sweep: f64, wall: f64) -> f64 {
    sweep.abs() * (radius + sweep.signum() * wall / 2.0).max(0.0)
}

impl Footprint {
    /// The straights that carry tabs: whole straights of at least four wall thicknesses.
    pub(crate) fn tab_edges(&self) -> Vec<Edge> {
        let mut seen = Vec::new();
        let mut edges = Vec::new();
        for span in &self.spans {
            let (Some(whole), Shape::Straight { .. }) = (span.whole, span.shape) else {
                continue;
            };
            if whole.length < MIN_TAB_STRAIGHT * self.wall || seen.contains(&whole.id) {
                continue;
            }
            seen.push(whole.id);
            edges.push(Edge {
                length: whole.length,
                seam: whole.seam,
            });
        }
        edges
    }

    /// Fits the typed joint values to these straights; `widest` is the widest plate's thickness.
    pub(crate) fn joint_fit(&self, joints: Joints, widest: f64) -> joint_layout::Fitted {
        joint_layout::fit(&self.tab_edges(), self.margin, 2.0 * widest, joints)
    }

    /// The tabs of straights that hold at least four wall thicknesses, `half_width` being the
    /// widest tab's half width (ADR 0082 §5), placed by `joints`.
    pub(crate) fn straight_tabs(&self, half_width: f64, joints: &Joints) -> Vec<Tab> {
        let mut tabs = Vec::new();
        let width = 2.0 * half_width;
        for span in &self.spans {
            let (Some(whole), Shape::Straight { from, to }) = (span.whole, span.shape) else {
                continue;
            };
            if whole.length < MIN_TAB_STRAIGHT * self.wall {
                continue;
            }
            let edge = Edge {
                length: whole.length,
                seam: whole.seam,
            };
            let count = edge.count(width, self.margin, joints);
            let gap = edge.gap(count, width, self.margin, joints);
            let centres = joint_layout::centres(whole.length, count, width, joints, gap);
            let (dx, dy) = (to.x - from.x, to.y - from.y);
            let length = libm::hypot(dx, dy);
            let along = (dx / length, dy / length);
            for u in centres {
                let local = u - whole.offset;
                if local < 0.0 || local >= length {
                    continue;
                }
                if u - half_width < self.margin - 1e-9
                    || u + half_width > whole.length - self.margin + 1e-9
                {
                    continue;
                }
                tabs.push(Tab {
                    s: span.s0 + local,
                    centre: Point::new(from.x + along.0 * local, from.y + along.1 * local),
                    along,
                });
            }
        }
        tabs
    }

    /// The tabs of the whole wall and the footprint they stand on: the straights' tabs when they
    /// are at least four, else windows on the bends. A window is a flat chord of length
    /// `2 half_width + 2 wall` that replaces its stretch of the arc.
    pub(crate) fn with_tabs(&self, spec: WindowSpec) -> (Self, Vec<Tab>) {
        let straight = self.straight_tabs(spec.half_width, &spec.joints);
        if spec.straight_enough(straight.len()) {
            return (self.clone(), straight);
        }
        let no_straight = self
            .spans
            .iter()
            .all(|span| matches!(span.shape, Shape::Bend { .. }));
        let perimeter: f64 = self.spans.iter().map(|s| inner_length(&s.shape)).sum();
        let mut out = Layout::new(self);
        for span in &self.spans {
            match span.shape {
                Shape::Straight { .. } => {
                    out.keep_straight(span, &straight);
                }
                Shape::Bend { .. } => {
                    let length = inner_length(&span.shape);
                    let (whole, minimum) = if no_straight {
                        (perimeter, MIN_CIRCLE_TABS)
                    } else {
                        (length, 1)
                    };
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let by_length = ((whole / TAB_PITCH).round() as usize).max(minimum);
                    let wanted = match spec.joints.count {
                        Some(typed) if no_straight => typed as usize,
                        _ => by_length,
                    };
                    let enough = no_straight || length >= MIN_TAB_STRAIGHT * self.wall;
                    let count = if enough {
                        self.fitting_count(span, wanted, spec)
                    } else {
                        0
                    };
                    out.split_bend(span, count, spec);
                }
            }
        }
        out.finish()
    }

    /// The most windows up to `wanted` that leave every curved stretch between two of them a slit
    /// band of `spec.min_band` and its two bridges, as a bend allows; 0 when not even one chord
    /// fits the circle.
    fn fitting_count(&self, span: &Span, wanted: usize, spec: WindowSpec) -> usize {
        let Shape::Bend { radius, sweep, .. } = span.shape else {
            return 0;
        };
        let chord = 2.0 * spec.half_width + 2.0 * self.wall;
        let reach = chord / (2.0 * radius);
        if reach >= 1.0 {
            return 0;
        }
        let window_arc = radius * 2.0 * libm::asin(reach);
        let needed = spec.min_band + 2.0 * self.wall;
        let mut count = wanted;
        #[allow(clippy::cast_precision_loss)]
        while count > 1 && radius * sweep.abs() / count as f64 - window_arc < needed {
            count -= 1;
        }
        if radius * sweep.abs() > window_arc + 2.0 * self.wall {
            count
        } else {
            0
        }
    }
}

/// The spans and tabs of the footprint with its windows, built one span at a time.
pub(crate) struct Layout {
    pub(crate) wall: f64,
    margin: f64,
    pub(crate) spans: Vec<Span>,
    pub(crate) tabs: Vec<Tab>,
    pub(crate) s0: f64,
    /// The signed angle the next window's chord replaces (positive on a convex stretch).
    pub(crate) turn: f64,
    /// The tabs that kept a window although `curve` was asked for.
    pub(crate) windowed: u32,
}

impl Layout {
    pub(crate) fn new(footprint: &Footprint) -> Self {
        Self {
            wall: footprint.wall,
            margin: footprint.margin,
            spans: Vec::new(),
            tabs: Vec::new(),
            s0: 0.0,
            turn: 0.0,
            windowed: 0,
        }
    }

    pub(crate) fn finish(self) -> (Footprint, Vec<Tab>) {
        (
            Footprint {
                spans: self.spans,
                wall: self.wall,
                margin: self.margin,
                windowed: self.windowed,
            },
            self.tabs,
        )
    }

    /// A straight as it is, with its tabs moved to its new place on the strip.
    pub(crate) fn keep_straight(&mut self, span: &Span, straight: &[Tab]) {
        self.tabs.extend(
            straight
                .iter()
                .filter(|tab| tab.s >= span.s0 - 1e-9 && tab.s < span.s0 + span.length)
                .map(|tab| Tab {
                    s: tab.s - span.s0 + self.s0,
                    ..*tab
                }),
        );
        self.spans.push(Span {
            s0: self.s0,
            ..*span
        });
        self.s0 += span.length;
    }

    /// Appends a piece of the footprint: an arc between windows or a window's chord, whose
    /// neutral length also holds the kink of half a wall thickness times the turn.
    pub(crate) fn push(&mut self, shape: Shape, window: bool) {
        let length = match shape {
            Shape::Bend { radius, sweep, .. } => {
                sweep.abs() * (radius + sweep.signum() * self.wall / 2.0).max(0.0)
            }
            Shape::Straight { from, to } => {
                let chord = libm::hypot(to.x - from.x, to.y - from.y);
                chord + self.wall / 2.0 * self.turn
            }
        };
        self.spans.push(Span {
            shape,
            s0: self.s0,
            length,
            whole: None,
            window,
            piece_start: false,
        });
        self.s0 += length;
    }

    /// Cuts `count` windows into the bend `span`, evenly by arc length.
    fn split_bend(&mut self, span: &Span, count: usize, spec: WindowSpec) {
        let Shape::Bend {
            centre,
            radius,
            from,
            sweep,
        } = span.shape
        else {
            return;
        };
        if count == 0 {
            self.push(span.shape, false);
            return;
        }
        let chord = 2.0 * spec.half_width + 2.0 * self.wall;
        let half_angle = libm::asin(chord / (2.0 * radius));
        let direction = sweep.signum();
        let curve = spec.curve && curve_fits(radius, sweep > 0.0, spec.half_width, self.wall);
        #[allow(clippy::cast_precision_loss)]
        let step = sweep / count as f64;
        let mut cursor = from;
        for index in 0..count {
            #[allow(clippy::cast_precision_loss)]
            let middle = from + step * (index as f64 + 0.5);
            if curve {
                // The arc stays whole; the tab stands on it at `middle` (unpushed arc from
                // the cursor to there is part of the strip before the tab).
                let after = self.s0 + neutral_arc(radius, middle - cursor, self.wall);
                self.tabs.push(Tab {
                    s: after,
                    centre: polar(centre, radius, middle),
                    along: (
                        -direction * libm::sin(middle),
                        direction * libm::cos(middle),
                    ),
                });
                continue;
            }
            self.windowed += u32::from(spec.curve);
            let (start, end) = (
                middle - direction * half_angle,
                middle + direction * half_angle,
            );
            if (start - cursor) * direction > 1e-9 {
                self.push_arc(centre, radius, cursor, start - cursor);
            }
            let (a, b) = (polar(centre, radius, start), polar(centre, radius, end));
            let length = libm::hypot(b.x - a.x, b.y - a.y);
            self.turn = 2.0 * half_angle * direction;
            self.tabs.push(Tab {
                s: self.s0 + f64::midpoint(length, self.wall * half_angle * direction),
                centre: Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
                along: ((b.x - a.x) / length, (b.y - a.y) / length),
            });
            self.push(Shape::Straight { from: a, to: b }, true);
            cursor = end;
        }
        let end = from + sweep;
        if (end - cursor) * direction > 1e-9 {
            self.push_arc(centre, radius, cursor, end - cursor);
        }
    }

    fn push_arc(&mut self, centre: Point, radius: f64, from: f64, sweep: f64) {
        self.push(
            Shape::Bend {
                centre,
                radius,
                from,
                sweep,
            },
            false,
        );
    }
}
