//! Where the plates' tabs stand: on the straights of the footprint, or, where those hold fewer
//! than four, in flat windows cut into the bends (ADR 0082 §5, Frage 360 B).

use curvyo_document_core::Point;

use crate::footprint::{Footprint, Shape, Span, Tab};
use crate::outline::polar;

/// The aimed distance between two tabs.
const TAB_PITCH: f64 = 30.0;
/// A straight must hold at least this many wall thicknesses to carry a tab.
const MIN_TAB_STRAIGHT: f64 = 4.0;
/// Fewer tabs than this on the straights, and the bends take tabs in windows.
const MIN_STRAIGHT_TABS: usize = 4;
/// A footprint without a straight takes at least this many tabs, as far as its stretches allow.
const MIN_CIRCLE_TABS: usize = 4;

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
}

impl Footprint {
    /// The tabs of straights that hold at least four wall thicknesses, `half_width` being the
    /// widest tab's half width (ADR 0082 §5).
    pub(crate) fn straight_tabs(&self, half_width: f64) -> Vec<Tab> {
        let mut tabs = Vec::new();
        for span in &self.spans {
            let (Some(whole), Shape::Straight { from, to }) = (span.whole, span.shape) else {
                continue;
            };
            if whole.length < MIN_TAB_STRAIGHT * self.wall {
                continue;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let mut count = ((whole.length / TAB_PITCH).round() as usize).max(1);
            if whole.seam && count % 2 == 1 {
                count += 1;
            }
            #[allow(clippy::cast_precision_loss)]
            let pitch = whole.length / count as f64;
            let (dx, dy) = (to.x - from.x, to.y - from.y);
            let length = libm::hypot(dx, dy);
            let along = (dx / length, dy / length);
            for index in 0..count {
                #[allow(clippy::cast_precision_loss)]
                let u = pitch * (index as f64 + 0.5);
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
        let straight = self.straight_tabs(spec.half_width);
        if straight.len() >= MIN_STRAIGHT_TABS {
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
                    let wanted = ((whole / TAB_PITCH).round() as usize).max(minimum);
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
struct Layout {
    wall: f64,
    margin: f64,
    spans: Vec<Span>,
    tabs: Vec<Tab>,
    s0: f64,
    /// The angle the next window's chord subtends on its circle.
    turn: f64,
}

impl Layout {
    fn new(footprint: &Footprint) -> Self {
        Self {
            wall: footprint.wall,
            margin: footprint.margin,
            spans: Vec::new(),
            tabs: Vec::new(),
            s0: 0.0,
            turn: 0.0,
        }
    }

    fn finish(self) -> (Footprint, Vec<Tab>) {
        (
            Footprint {
                spans: self.spans,
                wall: self.wall,
                margin: self.margin,
            },
            self.tabs,
        )
    }

    /// A straight as it is, with its tabs moved to its new place on the strip.
    fn keep_straight(&mut self, span: &Span, straight: &[Tab]) {
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
    fn push(&mut self, shape: Shape, window: bool) {
        let length = match shape {
            Shape::Bend { radius, sweep, .. } => {
                sweep.abs() * (radius + sweep.signum() * self.wall / 2.0)
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
        #[allow(clippy::cast_precision_loss)]
        let step = sweep / count as f64;
        let mut cursor = from;
        for index in 0..count {
            #[allow(clippy::cast_precision_loss)]
            let middle = from + step * (index as f64 + 0.5);
            let (start, end) = (
                middle - direction * half_angle,
                middle + direction * half_angle,
            );
            if (start - cursor) * direction > 1e-9 {
                self.push_arc(centre, radius, cursor, start - cursor);
            }
            let (a, b) = (polar(centre, radius, start), polar(centre, radius, end));
            let length = libm::hypot(b.x - a.x, b.y - a.y);
            self.turn = 2.0 * half_angle;
            self.tabs.push(Tab {
                s: self.s0 + f64::midpoint(length, self.wall * half_angle),
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
