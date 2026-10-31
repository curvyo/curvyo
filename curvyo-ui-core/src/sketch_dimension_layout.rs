//! Places a sketch dimension's extension lines, dimension line, arrows and text at screen-constant sizes (AC31).
//!
//! Input and output are document points; every size below is in screen pixels and is turned into
//! millimetres with the view scale, so the marks keep their size at every zoom (UX notes revision 2,
//! section 7, "Dimension anatomy").
use curvyo_document_core::{Length, Point, Vec2, ViewTransform};

/// Gap between the measured geometry and the start of an extension line.
const EXTENSION_GAP_PX: f64 = 4.0;
/// How far an extension line runs past the dimension line.
const EXTENSION_OVERRUN_PX: f64 = 4.0;
/// The offset of a new linear dimension from its geometry.
const FIRST_RANK_PX: f64 = 24.0;
/// The distance between two stacked ranks.
const RANK_STEP_PX: f64 = 20.0;
/// Below this span the arrows flip outside and the text moves beside the line end.
const INSIDE_ARROWS_MIN_PX: f64 = 56.0;
/// Below this span the arrows become ticks.
const ARROWS_MIN_PX: f64 = 24.0;
/// Length of the outside part of a flipped dimension line.
const FLIPPED_TAIL_PX: f64 = 12.0;
/// Distance of the text centre beyond the line end on a small span.
const BESIDE_TEXT_PX: f64 = 30.0;
/// Half length of a tick on a very small span.
const TICK_HALF_PX: f64 = 3.0;
/// Distance of radius and diameter text beyond the circle.
const LEADER_TEXT_PX: f64 = 28.0;
/// Default radius of an angle arc.
const ANGLE_RADIUS_PX: f64 = 40.0;
/// Distance of angle text beyond its arc.
const ANGLE_TEXT_PX: f64 = 14.0;
/// Text box estimate: width per character plus padding, and the box height (12px text).
const TEXT_CHAR_PX: f64 = 7.0;
const TEXT_PAD_PX: f64 = 8.0;
const TEXT_HEIGHT_PX: f64 = 16.0;
/// Below this length in millimetres a direction is undefined.
const DIRECTION_TOLERANCE_MM: f64 = 1e-9;

/// The measured geometry of one dimension, in document coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DimensionGeometry {
    /// A length or distance between two points, measured along the line through them.
    Linear {
        /// First measured point.
        from: Point,
        /// Second measured point.
        to: Point,
    },
    /// A circle or arc radius, with the default leader direction.
    Radius {
        /// Circle centre.
        center: Point,
        /// Radius in document millimetres.
        radius: Length,
        /// Leader direction when no label position is stored.
        toward: Vec2,
    },
    /// A circle or arc diameter, with the default line direction.
    Diameter {
        /// Circle centre.
        center: Point,
        /// Radius in document millimetres.
        radius: Length,
        /// Line direction when no label position is stored.
        toward: Vec2,
    },
    /// An angle between two rays from a vertex.
    Angle {
        /// Where the two lines meet.
        vertex: Point,
        /// Direction of the first ray.
        first: Vec2,
        /// Direction of the second ray.
        second: Vec2,
    },
}

/// One dimension to lay out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DimensionItem {
    /// What is measured.
    pub geometry: DimensionGeometry,
    /// The stored text position, if the maker placed or dragged it.
    pub label: Option<Point>,
    /// The number of characters of the text, for the overlap estimate.
    pub text_chars: usize,
}

/// A filled arrowhead: its tip and the unit direction from the tip toward its tail.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DimensionArrow {
    /// Where the arrow points to.
    pub tip: Point,
    /// Unit direction from the tip back to the tail.
    pub back: Vec2,
}

/// The marks of one laid-out dimension, in document coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionMarks {
    /// Extension lines, dimension line, leaders and ticks.
    pub lines: Vec<(Point, Point)>,
    /// Arrowheads; empty when the span is drawn with ticks.
    pub arrows: Vec<DimensionArrow>,
    /// The centre of the value text.
    pub text: Point,
    /// The stacking rank a linear dimension without stored label got.
    pub rank: usize,
}

/// Lays out every dimension, stacking unlabelled linear ones on the same line into ranks and
/// sliding a text that would overlap an earlier one along its own line.
#[must_use]
pub fn layout_dimensions(items: &[DimensionItem], view: ViewTransform) -> Vec<DimensionMarks> {
    let px = 1.0 / view.scale();
    let mut out: Vec<DimensionMarks> = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let mut rank = shared_line_rank(items, index, px);
        let mut marks = layout_dimension(item.geometry, item.label, rank, px);
        if item.label.is_none() {
            for _ in 0..4 {
                if let Some(free) = free_text(item, &marks, &out[..index], items, px) {
                    marks.text = free;
                    break;
                }
                if !matches!(item.geometry, DimensionGeometry::Linear { .. }) {
                    break;
                }
                rank += 1;
                marks = layout_dimension(item.geometry, None, rank, px);
            }
        }
        out.push(marks);
    }
    out
}

/// Lays out one dimension; `px` is document millimetres per screen pixel.
#[must_use]
fn layout_dimension(
    geometry: DimensionGeometry,
    label: Option<Point>,
    rank: usize,
    px: f64,
) -> DimensionMarks {
    match geometry {
        DimensionGeometry::Linear { from, to } => linear(from, to, label, rank, px),
        DimensionGeometry::Radius {
            center,
            radius,
            toward,
        } => radius_marks(center, radius.as_mm(), toward, label, px),
        DimensionGeometry::Diameter {
            center,
            radius,
            toward,
        } => diameter_marks(center, radius.as_mm(), toward, label, px),
        DimensionGeometry::Angle {
            vertex,
            first,
            second,
        } => angle_marks(vertex, first, second, label, px),
    }
}

fn unit(v: Vec2, fallback: Vec2) -> Vec2 {
    if v.length() <= DIRECTION_TOLERANCE_MM {
        fallback
    } else {
        v.normalized_to(1.0)
    }
}
fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}
fn at(origin: Point, direction: Vec2, distance: f64) -> Point {
    origin.translated(direction.scaled(distance))
}

/// The side a linear dimension stands on: above a horizontal line, left of a vertical one.
fn linear_frame(from: Point, to: Point) -> (Vec2, Vec2, f64) {
    let along = unit(from.vector_to(to), Vec2::new(1.0, 0.0));
    let mut normal = Vec2::new(along.y, -along.x);
    if normal.y > DIRECTION_TOLERANCE_MM
        || (normal.y.abs() <= DIRECTION_TOLERANCE_MM && normal.x > 0.0)
    {
        normal = normal.negated();
    }
    (along, normal, from.vector_to(to).length())
}

fn linear(from: Point, to: Point, label: Option<Point>, rank: usize, px: f64) -> DimensionMarks {
    let (along, normal, length) = linear_frame(from, to);
    #[allow(clippy::cast_precision_loss)] // a rank is a small count
    let offset = label.map_or((FIRST_RANK_PX + RANK_STEP_PX * rank as f64) * px, |l| {
        dot(from.vector_to(l), normal)
    });
    let side = if offset < 0.0 { -1.0 } else { 1.0 };
    let a = at(from, normal, offset);
    let b = at(to, normal, offset);
    let mut lines = Vec::new();
    if offset.abs() > EXTENSION_GAP_PX * px {
        for base in [from, to] {
            lines.push((
                at(base, normal, side * EXTENSION_GAP_PX * px),
                at(base, normal, offset + side * EXTENSION_OVERRUN_PX * px),
            ));
        }
    }
    let span = length / px;
    let beside = at(b, along, BESIDE_TEXT_PX * px);
    let mut text = label.map_or_else(
        || {
            if span < INSIDE_ARROWS_MIN_PX {
                beside
            } else {
                at(a, along, length / 2.0)
            }
        },
        |l| at(a, along, dot(from.vector_to(l), along)),
    );
    let t = dot(a.vector_to(text), along);
    let mut arrows = Vec::new();
    if span >= INSIDE_ARROWS_MIN_PX {
        lines.push((at(a, along, t.min(0.0)), at(a, along, t.max(length))));
        arrows.push(DimensionArrow {
            tip: a,
            back: along,
        });
        arrows.push(DimensionArrow {
            tip: b,
            back: along.negated(),
        });
    } else if span >= ARROWS_MIN_PX {
        let tail = FLIPPED_TAIL_PX * px;
        lines.push((
            at(a, along, (-tail).min(t)),
            at(a, along, (length + tail).max(t)),
        ));
        arrows.push(DimensionArrow {
            tip: a,
            back: along.negated(),
        });
        arrows.push(DimensionArrow {
            tip: b,
            back: along,
        });
    } else {
        lines.push((at(a, along, t.min(0.0)), at(a, along, t.max(length))));
        let slant = unit(Vec2::new(along.x + normal.x, along.y + normal.y), along);
        for end in [a, b] {
            lines.push((
                at(end, slant, -TICK_HALF_PX * px),
                at(end, slant, TICK_HALF_PX * px),
            ));
        }
    }
    if !text.x.is_finite() || !text.y.is_finite() {
        text = a;
    }
    DimensionMarks {
        lines,
        arrows,
        text,
        rank,
    }
}

fn radius_marks(
    center: Point,
    radius: f64,
    toward: Vec2,
    label: Option<Point>,
    px: f64,
) -> DimensionMarks {
    let direction = label.map_or_else(
        || unit(toward, Vec2::new(1.0, -1.0).normalized_to(1.0)),
        |l| unit(center.vector_to(l), toward),
    );
    let rim = at(center, direction, radius);
    let text = label.unwrap_or_else(|| at(rim, direction, LEADER_TEXT_PX * px));
    let outside = center.vector_to(text).length() >= radius;
    let (line, back) = if outside {
        ((rim, text), direction)
    } else {
        ((center, rim), direction.negated())
    };
    DimensionMarks {
        lines: vec![line],
        arrows: vec![DimensionArrow { tip: rim, back }],
        text,
        rank: 0,
    }
}

fn diameter_marks(
    center: Point,
    radius: f64,
    toward: Vec2,
    label: Option<Point>,
    px: f64,
) -> DimensionMarks {
    let direction = label.map_or_else(
        || unit(toward, Vec2::new(1.0, -1.0).normalized_to(1.0)),
        |l| unit(center.vector_to(l), toward),
    );
    let near = at(center, direction, -radius);
    let far = at(center, direction, radius);
    let text = label.unwrap_or_else(|| at(far, direction, LEADER_TEXT_PX * px));
    let reach = dot(center.vector_to(text), direction).max(radius);
    DimensionMarks {
        lines: vec![(near, at(center, direction, reach))],
        arrows: vec![
            DimensionArrow {
                tip: near,
                back: direction,
            },
            DimensionArrow {
                tip: far,
                back: direction.negated(),
            },
        ],
        text,
        rank: 0,
    }
}

fn angle_marks(
    vertex: Point,
    first: Vec2,
    second: Vec2,
    label: Option<Point>,
    px: f64,
) -> DimensionMarks {
    let mut e1 = unit(first, Vec2::new(1.0, 0.0));
    let mut e2 = unit(second, Vec2::new(0.0, -1.0));
    let mut sweep = (e1.x * e2.y - e1.y * e2.x).atan2(dot(e1, e2));
    if sweep < 0.0 {
        std::mem::swap(&mut e1, &mut e2);
        sweep = -sweep;
    }
    let radius = label.map_or(ANGLE_RADIUS_PX * px, |l| {
        vertex.vector_to(l).length().max(ANGLE_RADIUS_PX * px / 2.0)
    });
    let start = e1.y.atan2(e1.x);
    let point_at = |angle: f64| at(vertex, Vec2::new(angle.cos(), angle.sin()), radius);
    // invariant: a sweep in [0, PI] gives at most 32 segments of at most 0.1 rad.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let steps = ((sweep / 0.1).ceil() as usize).clamp(2, 32);
    #[allow(clippy::cast_precision_loss)] // at most 32 steps
    let lines = (0..steps)
        .map(|i| {
            let a = start + sweep * i as f64 / steps as f64;
            let b = start + sweep * (i + 1) as f64 / steps as f64;
            (point_at(a), point_at(b))
        })
        .collect();
    let end = start + sweep;
    let tangent_start = Vec2::new(-start.sin(), start.cos());
    let tangent_end = Vec2::new(end.sin(), -end.cos());
    let middle = start + sweep / 2.0;
    let text = label.unwrap_or_else(|| {
        at(
            vertex,
            Vec2::new(middle.cos(), middle.sin()),
            radius + ANGLE_TEXT_PX * px,
        )
    });
    DimensionMarks {
        lines,
        arrows: vec![
            DimensionArrow {
                tip: point_at(start),
                back: tangent_start,
            },
            DimensionArrow {
                tip: point_at(end),
                back: tangent_end,
            },
        ],
        text,
        rank: 0,
    }
}

/// The rank of an unlabelled linear dimension: one more than the earlier unlabelled linear
/// dimensions measured along the same line with overlapping spans.
fn shared_line_rank(items: &[DimensionItem], index: usize, px: f64) -> usize {
    let DimensionGeometry::Linear { from, to } = items[index].geometry else {
        return 0;
    };
    if items[index].label.is_some() {
        return 0;
    }
    let (along, normal, length) = linear_frame(from, to);
    items[..index]
        .iter()
        .filter(|other| other.label.is_none())
        .filter(|other| {
            let DimensionGeometry::Linear { from: f, to: t } = other.geometry else {
                return false;
            };
            let on_line = |p: Point| dot(from.vector_to(p), normal).abs() <= px;
            let (s0, s1) = (dot(from.vector_to(f), along), dot(from.vector_to(t), along));
            on_line(f) && on_line(t) && s0.max(s1) > 0.0 && s0.min(s1) < length
        })
        .count()
}

/// A text centre for `marks` that clears every earlier text box, sliding along the dimension
/// line by up to a third of its length each way; `None` when no slide clears.
fn free_text(
    item: &DimensionItem,
    marks: &DimensionMarks,
    earlier: &[DimensionMarks],
    items: &[DimensionItem],
    px: f64,
) -> Option<Point> {
    let half = |chars: usize| {
        #[allow(clippy::cast_precision_loss)] // a short text's character count
        let width = TEXT_CHAR_PX * chars as f64 + TEXT_PAD_PX;
        (width / 2.0 * px, TEXT_HEIGHT_PX / 2.0 * px)
    };
    let (w, h) = half(item.text_chars);
    let clear = |p: Point| {
        earlier.iter().zip(items).all(|(other, other_item)| {
            let (ow, oh) = half(other_item.text_chars);
            (p.x - other.text.x).abs() >= w + ow || (p.y - other.text.y).abs() >= h + oh
        })
    };
    if clear(marks.text) {
        return Some(marks.text);
    }
    let (along, reach) = match item.geometry {
        DimensionGeometry::Linear { from, to } => {
            let (along, _, length) = linear_frame(from, to);
            (along, length / 3.0)
        }
        _ => return None,
    };
    (1..=6)
        .flat_map(|step| [f64::from(step), -f64::from(step)])
        .map(|step| at(marks.text, along, reach * step / 6.0))
        .find(|p| clear(*p))
}

#[cfg(test)]
#[path = "sketch_dimension_layout_tests.rs"]
mod tests;
