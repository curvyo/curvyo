//! The parts of a path box from its polygon: wall pieces between the corners, inner base, cover,
//! ring and lid (ADR 0083 §4 to §8).

use curvyo_document_core::{Length, Point, Vec2};
use curvyo_extension_core::{Mark, Warning};

use super::corner::{Corner, End, steps};
use super::describe::{describe, inner_length};
use super::plates::plates;
use super::shape::{Shapes, Vertex, shapes, vertices};
use crate::footprint::{Footprint, Shape, Span, Tab};
use crate::rounded_box::layout::Piece;
use crate::slit_bend::SlitPattern;
use crate::tab_layout::WindowSpec;
use crate::wall::{Ends, Joint, LocalSpacing, Wall};

/// Everything the geometry needs, after the values were clamped.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct Measures {
    pub(crate) wall: f64,
    pub(crate) base: f64,
    pub(crate) cover: f64,
    pub(crate) ring: f64,
    pub(crate) lid: f64,
    pub(crate) kerf: f64,
    pub(crate) inset: f64,
    pub(crate) height: f64,
    pub(crate) corner_degrees: f64,
    pub(crate) lid_on: bool,
    pub(crate) seat: bool,
    pub(crate) cover_on: bool,
    pub(crate) hollow: Option<(f64, f64)>,
    pub(crate) pattern: SlitPattern,
    /// Set while the spacing is automatic.
    pub(crate) local: Option<LocalSpacing>,
    pub(crate) draft: bool,
}

/// A piece's badge on the inner base: its number, the middle of its stretch, the direction into
/// the box and the stretch's length.
pub(crate) type Badge = (u32, Point, Vec2, f64);

/// What `build` returns: parts to place and what the shell is told about them.
pub(crate) struct Built {
    pub(crate) strips: Vec<Piece>,
    pub(crate) plates: Option<Vec<Piece>>,
    pub(crate) warnings: Vec<Warning>,
    pub(crate) marks: Vec<Mark>,
    /// The piece badges on the inner base: `(number, middle, inward, length)`.
    pub(crate) badges: Vec<Badge>,
    /// The smallest gap between two slit columns laid, `None` without a bent span.
    pub(crate) smallest_gap: Option<f64>,
    pub(crate) pieces: usize,
}

pub(crate) fn warning(id: &'static str, pieces: Vec<u32>) -> Warning {
    Warning {
        id,
        pieces,
        value: None,
        limit: None,
    }
}

/// Where a loop without a corner is cut: the middle of its longest straight, else the point with
/// the least screen y (ADR 0083 §6, revision 3).
fn seam(shapes: &[Shape]) -> (usize, f64, bool) {
    // The middle of the longest straight, never inside a bent span (lead decision, Proposal);
    // ties go to the least y, then the least x.
    let longest = shapes
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match *s {
            Shape::Straight { from, to } => {
                Some((i, inner_length(s), from.y.min(to.y), from.x.min(to.x)))
            }
            Shape::Bend { .. } => None,
        })
        .max_by(|a, b| {
            a.1.total_cmp(&b.1)
                .then(b.2.total_cmp(&a.2))
                .then(b.3.total_cmp(&a.3))
        });
    if let Some((index, length, ..)) = longest {
        return (index, length / 2.0, true);
    }
    seam_at_top(shapes)
}

/// Without a straight the seam falls on the point with the least screen y.
fn seam_at_top(shapes: &[Shape]) -> (usize, f64, bool) {
    let mut best: Option<(f64, f64, usize, f64, bool)> = None;
    let mut offer = |y: f64, x: f64, index: usize, at: f64, level: bool| {
        let better = best.is_none_or(|(by, bx, ..)| y < by - 1e-9 || (y < by + 1e-9 && x < bx));
        if better {
            best = Some((y, x, index, at, level));
        }
    };
    for (index, shape) in shapes.iter().enumerate() {
        match *shape {
            Shape::Straight { from, to } if (from.y - to.y).abs() < 1e-9 => {
                offer(
                    from.y,
                    from.x.min(to.x),
                    index,
                    inner_length(shape) / 2.0,
                    true,
                );
            }
            Shape::Straight { .. } => {}
            Shape::Bend {
                centre,
                radius,
                from,
                sweep,
            } => {
                let top = -std::f64::consts::FRAC_PI_2;
                let tau = std::f64::consts::TAU;
                let turn = (top - from).rem_euclid(tau);
                let along = if sweep > 0.0 {
                    turn
                } else {
                    (tau - turn) % tau
                };
                let (at, point) = if along <= sweep.abs() {
                    (along * radius, Point::new(centre.x, centre.y - radius))
                } else {
                    (0.0, crate::outline::polar(centre, radius, from))
                };
                offer(point.y, point.x, index, at, false);
            }
        }
    }
    best.map_or((0, 0.0, false), |(_, _, index, at, level)| {
        (index, at, level)
    })
}

/// The footprint without tabs and the corner joints of the polygon.
pub(crate) struct Plan {
    pub(crate) built: Shapes,
    pub(crate) footprint: Footprint,
    pub(crate) corners: Vec<Corner>,
}

fn plan(verts: &[Vertex], wall: f64, min_arc: f64) -> Plan {
    let built = shapes(verts, wall, min_arc);
    let corners: Vec<Corner> = built
        .corners
        .iter()
        .map(|v| Corner::new(v.turn, wall))
        .collect();
    let z = corners.iter().map(|c| c.margin(wall)).fold(wall, f64::max);
    let footprint = if built.corners.is_empty() {
        Footprint::from_loop(&built.shapes, wall, seam(&built.shapes))
    } else {
        Footprint::from_shapes(&built.shapes, wall, z)
    };
    Plan {
        built,
        footprint,
        corners,
    }
}

/// The spans of `footprint` split into pieces: a new piece starts at every corner point.
fn split_pieces(footprint: &Footprint, corners: &[Vertex]) -> Vec<(usize, usize)> {
    let at_corner = |span: &Span| {
        let start = match span.shape {
            Shape::Straight { from, .. } => from,
            Shape::Bend {
                centre,
                radius,
                from,
                ..
            } => crate::outline::polar(centre, radius, from),
        };
        corners
            .iter()
            .any(|c| libm::hypot(c.at.x - start.x, c.at.y - start.y) < 1e-6)
    };
    let starts: Vec<usize> = (0..footprint.spans.len())
        .filter(|&i| at_corner(&footprint.spans[i]))
        .collect();
    (0..starts.len())
        .map(|k| {
            (
                starts[k],
                starts.get(k + 1).copied().unwrap_or(footprint.spans.len()),
            )
        })
        .collect()
}

/// The slots of the plates' tabs and the strip height they make (ADR 0082 §5).
fn joints_and_height(m: &Measures) -> (Vec<Joint>, f64) {
    let mut joints = vec![Joint {
        thickness: Length::from_mm(m.base),
        inset: Length::from_mm(m.inset),
        from_top: false,
    }];
    if m.lid_on {
        joints.push(Joint {
            thickness: Length::from_mm(if m.seat { m.ring } else { m.lid }),
            inset: Length::from_mm(0.0),
            from_top: true,
        });
    }
    let top = match (m.lid_on, m.seat) {
        (true, true) => m.ring,
        (true, false) => m.lid,
        (false, _) => 0.0,
    };
    (joints, m.inset + m.base + m.height + top)
}

/// The finger ends of piece `number`: it mates at its start corner and owns its end corner.
fn finger_ends(plan: &Plan, number: usize, height: f64, t: f64, length: f64) -> Option<Ends> {
    let count = plan.corners.len();
    if count == 0 {
        return None;
    }
    let (start, finish) = (
        &plan.corners[number % count],
        &plan.corners[(number + 1) % count],
    );
    Some(Ends {
        start: steps(height, t, false, start, (End::Start, 0.0)),
        end: steps(height, t, true, finish, (End::Finish, length)),
        lead: start.margin(t),
        trail: finish.margin(t),
    })
}

/// The spans of `inner` from `a` to `b` as a footprint that starts at 0, with its tabs.
fn piece_footprint(
    inner: &Footprint,
    (a, b): (usize, usize),
    tabs: &[Tab],
) -> (Footprint, Vec<Tab>) {
    let s0 = inner.spans[a].s0;
    let spans: Vec<Span> = inner.spans[a..b]
        .iter()
        .map(|span| Span {
            s0: span.s0 - s0,
            ..*span
        })
        .collect();
    let footprint = Footprint {
        spans,
        wall: inner.wall,
        margin: inner.margin,
    };
    let length = footprint.total();
    let local = tabs
        .iter()
        .filter(|tab| tab.s >= s0 - 1e-9 && tab.s < s0 + length + 1e-9)
        .map(|tab| Tab {
            s: tab.s - s0,
            ..*tab
        })
        .collect();
    (footprint, local)
}

/// One strip per piece with its finger ends, tab slots and slits.
fn strips(plan: &Plan, inner: &Footprint, tabs: &[Tab], m: &Measures) -> (Vec<Piece>, Vec<f64>) {
    let (joints, height) = joints_and_height(m);
    let ranges = if plan.corners.is_empty() {
        vec![(0, inner.spans.len())]
    } else {
        split_pieces(inner, &plan.built.corners)
    };
    let (mut pieces, mut pitches) = (Vec::new(), Vec::new());
    for (number, &range) in ranges.iter().enumerate() {
        let (footprint, local) = piece_footprint(inner, range, tabs);
        let wall = Wall {
            footprint: &footprint,
            height: Length::from_mm(height),
            pattern: m.pattern,
            slits_everywhere: false,
            joints: joints.clone(),
            draft: m.draft,
            ends: finger_ends(plan, number, height, m.wall, footprint.total()),
            local_spacing: m.local,
        };
        pitches.extend(wall.pitches());
        let mut piece = Piece::new(wall.contours(&local), m.wall, "wall");
        piece.number = u32::try_from(number + 1).unwrap_or(u32::MAX);
        pieces.push(piece);
    }
    (pieces, pitches)
}

impl Built {
    /// The plates, once.
    pub(crate) fn plates_owned(&mut self) -> Vec<Piece> {
        self.plates.take().unwrap_or_default()
    }
}

/// Builds the parts of the box of `points` (clockwise on screen).
pub(crate) fn build(points: &[Point], m: &Measures) -> (Built, Footprint, Vec<Tab>) {
    let verts = vertices(points, m.corner_degrees);
    let least = if m.pattern.kind == crate::slit_bend::SlitKind::Wave {
        (m.pattern.width.as_mm() + 1.0) / 0.88
    } else {
        m.pattern.width.as_mm() + 1.0
    };
    let plan = plan(&verts, m.wall, 2.0 * least);
    let widest = [m.base, m.ring, m.lid].into_iter().fold(0.0, f64::max);
    let spec = WindowSpec {
        half_width: widest,
        min_band: 2.0 * (m.pattern.width.as_mm() + 1.0) / 0.88,
    };
    let (inner, tabs) = plan.footprint.with_tabs(spec);
    let (strips, pitches) = strips(&plan, &inner, &tabs, m);
    let mut warnings = Vec::new();
    let plates = plates(&plan, &inner, &tabs, m, &mut warnings);
    let count = strips.len();
    let (more, marks, badges) = describe(&plan, m, count);
    warnings.extend(more);
    let built = Built {
        strips,
        plates: Some(plates),
        warnings,
        marks,
        badges,
        smallest_gap: pitches.into_iter().reduce(f64::min),
        pieces: count,
    };
    (built, inner, tabs)
}
