//! The path box: a laser-cut box built from any closed outline, its corners finger-jointed and its
//! kinks slit-bent (ADR 0083, spec `0149`).

mod build;
mod check;
mod classify;
mod corner;
mod describe;
mod plates;
mod schema;
mod shape;

use curvyo_document_core::{Length, Point, Vec2};
use curvyo_extension_core::{
    ClosedPath, Cubic, Edit, GenerateRequest, Generated, Mark, Marks, Param, ParamValue,
    StoredParams, Warning,
};

use crate::rounded_box::layout::{Placed, place};
use crate::rounded_box::{own, slit_values_for};
use crate::slit_bend::SlitKind;
use crate::values::Values;
use crate::wall::LocalSpacing;
use build::{Measures, build};
use check::{check, default_outline};

pub(crate) use schema::groups;

/// A plate part's thickness that follows the wall: stored 0 means automatic.
const FOLLOWING: [&str; 4] = [
    "plate_thickness",
    "cover_thickness",
    "ring_thickness",
    "lid_thickness",
];

/// Every path box parameter, in order.
pub(crate) fn params() -> Vec<Param> {
    groups()
        .into_iter()
        .flat_map(|group| group.params)
        .collect()
}

fn scaled(path: &ClosedPath, factor: f64) -> ClosedPath {
    let by = |p: Point| Point::new(p.x * factor, p.y * factor);
    ClosedPath {
        start: by(path.start),
        segments: path
            .segments
            .iter()
            .map(|c| Cubic {
                c1: by(c.c1),
                c2: by(c.c2),
                to: by(c.to),
            })
            .collect(),
    }
}

fn raw_mm(params: &StoredParams, key: &str, default: f64) -> f64 {
    params
        .get(key)
        .and_then(ParamValue::from_stored)
        .and_then(|value| value.length().map(Length::as_mm))
        .unwrap_or(default)
}

/// The stored values with a resize applied before the schema clamps them: the outline and Inner
/// height scale, the thicknesses stay, and the factor stops where the outline would hold less than
/// `(4 t)^2` (the warning `smallest_size`, ADR 0083 §2).
fn edited(request: &GenerateRequest<'_>) -> (StoredParams, Option<Warning>) {
    let mut raw = request.params.clone();
    let Some(Edit::Scale(factor)) = request.edit else {
        return (raw, None);
    };
    let outline = raw
        .get("outline")
        .and_then(ParamValue::from_stored)
        .and_then(|value| value.path().cloned())
        .unwrap_or_else(default_outline);
    let wall = raw_mm(request.params, "thickness", 3.0);
    let area = check(&outline).area;
    // A hair above, so the area never rounds just below (4 t)^2.
    let least = 4.0 * wall / libm::sqrt(area) * (1.0 + 1e-9);
    let effective = factor.0.max(least);
    let warning = (effective > factor.0 + 1e-12).then(|| Warning {
        id: "smallest_size",
        pieces: Vec::new(),
        value: Some(ParamValue::Length(Length::from_mm(wall))),
        limit: None,
    });
    raw.insert(
        "outline".to_owned(),
        ParamValue::Path(scaled(&outline, effective)).to_stored(),
    );
    let height = raw_mm(request.params, "inner_height", 50.0) * effective;
    raw.insert(
        "inner_height".to_owned(),
        ParamValue::Length(Length::from_mm(height)).to_stored(),
    );
    (raw, warning)
}

/// The thickness of the part on top of the strip: the ring of a seated lid, else the lid.
fn top_part(lid_on: bool, seat: bool, ring: f64, lid: f64) -> f64 {
    match (lid_on, seat) {
        (true, true) => ring,
        (true, false) => lid,
        (false, _) => 0.0,
    }
}

/// The thinnest part that is cut: the kerf may be at most half of it.
fn thinnest_part(
    v: &Values<'_>,
    [wall, base, cover, ring, lid]: [f64; 5],
    lid_on: bool,
    seat: bool,
) -> f64 {
    let mut thinnest = wall.min(base);
    if v.switch("base_cover") {
        thinnest = thinnest.min(cover);
    }
    if lid_on {
        thinnest = thinnest.min(if seat { ring } else { lid });
    }
    thinnest
}

/// The values after the dependent clamps, in the order the limits depend on each other.
fn measures(v: &mut Values<'_>, area: f64, draft: bool) -> Measures {
    let height = v.mm("inner_height");
    // The size rule is the only thing that limits the thickness on a create (ADR 0083 §2).
    v.limit_mm("thickness", 0.0, libm::sqrt(area) / 4.0);
    let wall = v.mm("thickness");
    let (seat, lid_on) = (v.switch("lid") && v.switch("lid_seat"), v.switch("lid"));
    let used = |v: &Values<'_>, key: &str| own(v, key).unwrap_or(wall);
    let (base, cover, ring, lid) = (
        used(v, "plate_thickness"),
        used(v, "cover_thickness"),
        used(v, "ring_thickness"),
        used(v, "lid_thickness"),
    );
    let thinnest = thinnest_part(v, [wall, base, cover, ring, lid], lid_on, seat);
    v.limit_mm("kerf", 0.0, thinnest / 2.0);
    let inset = v.mm("base_inset");
    if inset > 0.0 && inset < wall {
        v.limit_mm("base_inset", wall, f64::MAX);
    }
    let kerf = v.mm("kerf");
    let strip_height = v.mm("base_inset") + base + height + top_part(lid_on, seat, ring, lid);
    let (pattern, _) = slit_values_for(v, (wall, kerf, 2.0 * wall), strip_height);
    let least = if pattern.kind == SlitKind::Wave {
        (pattern.width.as_mm() + 1.0) / 0.88
    } else {
        pattern.width.as_mm() + 1.0
    };
    Measures {
        wall,
        base,
        cover,
        ring,
        lid,
        kerf,
        inset: v.mm("base_inset"),
        height,
        corner_degrees: v.degrees("corner_angle"),
        lid_on,
        seat,
        cover_on: v.switch("base_cover"),
        hollow: (seat && v.switch("finger_hollow"))
            .then(|| (v.mm("hollow_width"), v.mm("hollow_depth"))),
        pattern,
        local: (own(v, "slit_spacing").is_none()).then_some(LocalSpacing {
            factor: 0.22,
            least,
        }),
        draft,
    }
}

/// The numbers the generator derived where the maker stored 0.
fn used_values(
    v: &Values<'_>,
    m: &Measures,
    smallest_gap: Option<f64>,
) -> Vec<(&'static str, ParamValue)> {
    let length = |mm: f64| ParamValue::Length(Length::from_mm(mm));
    let mut used = Vec::new();
    for (key, value) in FOLLOWING.into_iter().zip([m.base, m.cover, m.ring, m.lid]) {
        if v.mm(key) == 0.0 {
            used.push((key, length(value)));
        }
    }
    if v.mm("slit_width") == 0.0 {
        used.push(("slit_width", length(m.pattern.width.as_mm())));
    }
    if v.mm("slit_spacing") == 0.0 {
        let fallback = (0.22 * 2.0 * m.wall).max(m.pattern.width.as_mm() + 1.0);
        used.push(("slit_spacing", length(smallest_gap.unwrap_or(fallback))));
    }
    if v.count("slit_rows") == 0 {
        used.push(("slit_rows", ParamValue::Count(m.pattern.rows)));
    }
    used
}

/// The bounding box of a placed part's outline.
fn middle(placed: &Placed) -> Option<Point> {
    let path = placed.part.paths.first()?;
    let points: Vec<Point> = std::iter::once(path.start)
        .chain(path.segments.iter().map(|c| c.to))
        .collect();
    let (min_x, max_x) = points
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
    let (min_y, max_y) = points
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.y), b.max(p.y)));
    Some(Point::new(
        f64::midpoint(min_x, max_x),
        f64::midpoint(min_y, max_y),
    ))
}

fn marks(built: &build::Built, placed: &[Placed]) -> Marks {
    let base = placed
        .iter()
        .find(|p| p.role == "base")
        .map_or((0.0, 0.0), |p| p.offset);
    let mut on_footprint = built.marks.clone();
    let mut on_parts = Vec::new();
    for &(number, at, inward, length) in &built.badges {
        on_footprint.push(Mark::Piece {
            number,
            at,
            inward,
            length: Length::from_mm(length),
        });
    }
    if built.pieces >= 2 {
        for part in placed.iter().filter(|p| p.role == "wall") {
            if let Some(at) = middle(part) {
                on_parts.push(Mark::Piece {
                    number: part.number,
                    at,
                    inward: Vec2::new(0.0, 0.0),
                    length: Length::from_mm(0.0),
                });
            }
        }
    }
    Marks {
        footprint: Vec2::new(base.0, base.1),
        on_footprint,
        on_parts,
    }
}

/// Generates the path box `request` asks for.
pub(crate) fn generate(request: &GenerateRequest<'_>) -> Generated {
    let schema = params();
    let (raw, resized) = edited(request);
    let mut v = Values::read(&schema, &raw);
    let outline = v.path("outline").unwrap_or_else(default_outline);
    let checked = check(&outline);
    let m = measures(
        &mut v,
        checked.area,
        matches!(request.edit, Some(Edit::Draft)),
    );
    let (mut built, _inner, _tabs) = build(&checked.points, &m);
    let spacing = m.kerf + v.mm("gap");
    let plates = built.plates_owned();
    let placed = place(&built.strips, plates, m.kerf, spacing);
    let marks = marks(&built, &placed);
    let automatic = used_values(&v, &m, built.smallest_gap);
    let mut warnings = checked.warnings;
    warnings.extend(built.warnings);
    warnings.extend(resized);
    let (params, clamps) = v.finish();
    Generated {
        params,
        parts: placed.into_iter().map(|p| p.part).collect(),
        handles: Vec::new(),
        clamps,
        automatic,
        states: Vec::new(),
        limits: Vec::new(),
        warnings,
        marks,
    }
}
