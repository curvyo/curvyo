//! The rounded box: a box with rounded corners whose wall is one slit strip that bends round the
//! corners, with an inner base, a cover, a seat ring and a lid (ADR 0082 §5, spec `0147`).

mod curved_top;
pub(crate) mod layout;
pub(crate) mod schema;

use curvyo_document_core::{Length, Point, Vec2};
use curvyo_extension_core::{
    Edit, GenerateRequest, Generated, Handle, Param, ParamValue, StoredParams,
};

use crate::footprint::{Footprint, Tab};
use crate::slit_bend::{SlitKind, SlitPattern};
use crate::tab_layout::WindowSpec;
use crate::values::Values;
use crate::wall::{Joint, Wall};
use crate::wall_plates::{Hollow, opening, plain, tabbed};
use layout::{Piece, place};

pub(crate) use schema::groups;

/// A plate part's thickness that follows the wall: stored 0 means automatic.
const FOLLOWING: [&str; 4] = [
    "plate_thickness",
    "cover_thickness",
    "ring_thickness",
    "lid_thickness",
];

/// Every rounded box parameter, in order.
pub(crate) fn params() -> Vec<Param> {
    groups()
        .into_iter()
        .flat_map(|group| group.params)
        .collect()
}

fn raw_mm(params: &StoredParams, key: &str, default: f64) -> f64 {
    params
        .get(key)
        .and_then(ParamValue::from_stored)
        .and_then(|value| value.length().map(Length::as_mm))
        .unwrap_or(default)
}

/// The stored values with a resize applied before the schema clamps them: only the three inner
/// sizes scale, so the slit count follows the new perimeter (spec `0147` criterion 7).
fn edited(request: &GenerateRequest<'_>) -> StoredParams {
    let mut raw = request.params.clone();
    if let Some(Edit::Scale(factor)) = request.edit {
        for (key, default) in [
            ("inner_width", 120.0),
            ("inner_depth", 80.0),
            ("inner_height", 50.0),
        ] {
            let scaled = raw_mm(request.params, key, default) * factor.0;
            raw.insert(
                key.to_owned(),
                ParamValue::Length(Length::from_mm(scaled)).to_stored(),
            );
        }
    }
    raw
}

/// The measures every part is built from, after all clamps.
struct Sizes {
    width: f64,
    depth: f64,
    height: f64,
    radius: f64,
    wall: f64,
    base: f64,
    cover: f64,
    ring: f64,
    lid: f64,
    kerf: f64,
    inset: f64,
    /// The largest values the dependent bounds allow now (`Generated::limits`).
    limits: Vec<(&'static str, ParamValue)>,
}

/// `key` as the maker typed it: `Some` unless it is stored 0 (automatic).
pub(crate) fn own(v: &Values<'_>, key: &str) -> Option<f64> {
    let value = v.mm(key);
    (value > 0.0).then_some(value)
}

/// The values, clamped in the order the limits depend on each other (spec `0147` criterion 4).
fn clamp(v: &mut Values<'_>) -> Sizes {
    let (width, depth, height) = (
        v.mm("inner_width"),
        v.mm("inner_depth"),
        v.mm("inner_height"),
    );
    let smallest = width.min(depth);
    // A flat window for a tab is 4 wall thicknesses wide: it must fit a small circle (ADR 0082 §5).
    v.limit_mm(
        "thickness",
        0.0,
        (0.25 * smallest.min(height)).min(smallest / 6.0),
    );
    let wall = v.mm("thickness");
    v.limit_mm("corner_radius", 2.0 * wall, smallest / 2.0);
    let radius = v.mm("corner_radius");
    let seat = v.switch("lid") && v.switch("lid_seat");
    // The ring's opening keeps two wall thicknesses across (a following value is limited quietly).
    let ring_limit = ((smallest - 2.0 * wall) / 4.0).max(0.1);
    if own(v, "ring_thickness").is_some() {
        v.limit_mm("ring_thickness", 0.1, ring_limit);
    }
    let used = |v: &Values<'_>, key: &str| own(v, key).unwrap_or(wall);
    let (base, cover, lid) = (
        used(v, "plate_thickness"),
        used(v, "cover_thickness"),
        used(v, "lid_thickness"),
    );
    let ring = used(v, "ring_thickness").min(ring_limit);
    let mut thinnest = wall.min(base);
    if v.switch("base_cover") {
        thinnest = thinnest.min(cover);
    }
    if v.switch("lid") {
        thinnest = thinnest.min(if seat { ring } else { lid });
    }
    v.limit_mm("kerf", 0.0, thinnest / 2.0);
    let mm = |value: f64| ParamValue::Length(Length::from_mm(value));
    let mut limits = vec![("corner_radius", mm(smallest / 2.0))];
    // The finger hollow stays on the front edge and ends over the ring (ADR 0082 §5).
    if v.switch("lid") && v.switch("lid_seat") && v.switch("finger_hollow") {
        let front = width - 2.0 * radius;
        // The hollow is as wide as the front straight, once; on a circle as the radius.
        let widest = if front > 1e-2 { front } else { radius };
        v.limit_mm("hollow_width", 5.0, widest.max(5.0));
        let half = v.mm("hollow_width") / 2.0;
        v.limit_mm("hollow_depth", half, (2.0 * ring + lid).max(half));
        limits.push(("hollow_width", mm(widest.max(5.0))));
        limits.push(("hollow_depth", mm((2.0 * ring + lid).max(half))));
    }
    // A raised base keeps a bridge of one wall thickness to the edge.
    let inset = v.mm("base_inset");
    if inset > 0.0 && inset < wall {
        v.limit_mm("base_inset", wall, f64::MAX);
    }
    Sizes {
        width,
        depth,
        height,
        radius,
        wall,
        base,
        cover,
        ring,
        lid,
        kerf: v.mm("kerf"),
        inset: v.mm("base_inset"),
        limits,
    }
}

/// The slit values: typed ones are clamped, automatic ones derived and reported.
fn slit_values(v: &mut Values<'_>, s: &Sizes, strip_height: f64) -> (SlitPattern, bool) {
    slit_values_for(v, (s.wall, s.kerf, s.radius), strip_height)
}

/// The slit values for a wall of thickness `wall`, kerf `kerf` and bends of `radius` (the radius
/// the automatic spacing is derived from).
pub(crate) fn slit_values_for(
    v: &mut Values<'_>,
    (wall, kerf, radius): (f64, f64, f64),
    strip_height: f64,
) -> (SlitPattern, bool) {
    if own(v, "slit_width").is_some() {
        v.limit_mm("slit_width", kerf + 0.2, f64::MAX);
    }
    let width = own(v, "slit_width").unwrap_or(kerf + 0.3);
    let kind = match v.choice("slit_pattern").as_str() {
        "offset" => SlitKind::Offset,
        "wave" => SlitKind::Wave,
        _ => SlitKind::Straight,
    };
    // Wave slots lean 28 degrees: neighbours need (width + 1 mm) / 0.88 (ADR 0082 §5).
    let least = if kind == SlitKind::Wave {
        (width + 1.0) / 0.88
    } else {
        width + 1.0
    };
    if own(v, "slit_spacing").is_some() {
        if kind == SlitKind::Wave {
            v.limit_min_for("slit_spacing", least, "Wave pattern");
        } else {
            v.limit_mm("slit_spacing", least, f64::MAX);
        }
    }
    let spacing = own(v, "slit_spacing").unwrap_or((0.22 * radius).max(least));
    let rows = match v.count("slit_rows") {
        0 => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let rows = (strip_height / (10.0 * wall)).round().max(1.0) as u32;
            rows
        }
        typed => typed,
    };
    (
        SlitPattern {
            kind,
            width: Length::from_mm(width),
            spacing: Length::from_mm(spacing),
            bridge: Length::from_mm(wall),
            rows,
        },
        v.choice("wall_slits") == "all",
    )
}

/// The plates: cover, lid, inner base and ring, in the order of the parts of the layout.
fn plates(
    v: &Values<'_>,
    s: &Sizes,
    (inner, outer): (&Footprint, &Footprint),
    tabs: &[Tab],
) -> Vec<Piece> {
    let (lid_on, seat, cover_on) = (
        v.switch("lid"),
        v.switch("lid_seat"),
        v.switch("base_cover"),
    );
    let mm = Length::from_mm;
    let mut plates: Vec<Piece> = Vec::new();
    if cover_on {
        plates.push(Piece::new(vec![plain(outer, None)], s.cover, "cover"));
    }
    if lid_on {
        let hollow = (seat && v.switch("finger_hollow")).then(|| Hollow {
            width: v.mm("hollow_width"),
            depth: v.mm("hollow_depth"),
        });
        if seat {
            plates.push(Piece::new(vec![plain(outer, hollow)], s.lid, "lid"));
        } else {
            plates.push(Piece::new(
                vec![tabbed(inner, tabs, s.lid, s.wall)],
                s.lid,
                "lid",
            ));
        }
    }
    plates.push(Piece::new(
        vec![tabbed(inner, tabs, s.base, s.wall)],
        s.base,
        "base",
    ));
    if lid_on && seat {
        let ring_width = 2.0 * s.ring;
        let hole = Footprint::rounded_rect(
            mm(s.width - 2.0 * ring_width),
            mm(s.depth - 2.0 * ring_width),
            mm((s.radius - ring_width).max(0.0)),
            mm(s.wall),
        );
        plates.push(Piece::new(
            vec![tabbed(inner, tabs, s.ring, s.wall), opening(&hole)],
            s.ring,
            "ring",
        ));
    }
    plates
}

/// Generates the rounded box `request` asks for.
pub(crate) fn generate(request: &GenerateRequest<'_>) -> Generated {
    let schema = params();
    let mut v = Values::read(&schema, &edited(request));
    if v.choice("top_shape") == "curved" {
        return curved_top::generate(request, v);
    }
    let s = clamp(&mut v);
    let (lid_on, seat) = (v.switch("lid"), v.switch("lid_seat"));
    let strip_height = s.inset
        + s.base
        + s.height
        + match (lid_on, seat) {
            (true, true) => s.ring,
            (true, false) => s.lid,
            (false, _) => 0.0,
        };
    let (pattern, everywhere) = slit_values(&mut v, &s, strip_height);
    let mm = Length::from_mm;
    let inner = Footprint::rounded_rect(mm(s.width), mm(s.depth), mm(s.radius), mm(s.wall));
    let outer = Footprint::rounded_rect(
        mm(s.width + 2.0 * s.wall),
        mm(s.depth + 2.0 * s.wall),
        mm(s.radius + s.wall),
        mm(s.wall),
    );
    let widest = [s.base, s.ring, s.lid].into_iter().fold(0.0, f64::max);
    let spec = WindowSpec {
        half_width: widest,
        // Two slit columns of the least spacing between two windows.
        min_band: 2.0 * (pattern.width.as_mm() + 1.0) / 0.88,
    };
    let (inner, tabs) = inner.with_tabs(spec);

    let mut joints = vec![Joint {
        thickness: mm(s.base),
        inset: mm(s.inset),
        from_top: false,
    }];
    if lid_on {
        joints.push(Joint {
            thickness: mm(if seat { s.ring } else { s.lid }),
            inset: mm(0.0),
            from_top: true,
        });
    }
    let wall = Wall {
        footprint: &inner,
        height: mm(strip_height),
        pattern,
        slits_everywhere: everywhere,
        joints,
        draft: matches!(request.edit, Some(Edit::Draft)),
        ends: None,
        local_spacing: None,
        runs: false,
        seam_margin: 0.0,
    };
    let strip = Piece::new(wall.contours(&tabs), s.wall, "strip");

    let plates = plates(&v, &s, (&inner, &outer), &tabs);
    let placed = place(
        std::slice::from_ref(&strip),
        plates,
        s.kerf,
        v.mm("gap") + s.kerf,
    );
    let handles = handles(&placed, &inner, &s);
    let automatic = used_values(&v, &s, &pattern);
    let states = if s.radius >= s.width.min(s.depth) / 2.0 - 0.005 {
        vec![("corner_radius", "full round")]
    } else {
        Vec::new()
    };
    let (params, clamps) = v.finish();
    Generated {
        params,
        parts: placed.into_iter().map(|p| p.part).collect(),
        handles,
        clamps,
        automatic,
        states,
        limits: s.limits.clone(),
        warnings: Vec::new(),
        marks: curvyo_extension_core::Marks::default(),
    }
}

/// The values the generator used where the maker stored 0.
fn used_values(
    v: &Values<'_>,
    s: &Sizes,
    pattern: &SlitPattern,
) -> Vec<(&'static str, ParamValue)> {
    let mut used = Vec::new();
    let length = |mm: f64| ParamValue::Length(Length::from_mm(mm));
    for (key, value) in FOLLOWING.into_iter().zip([s.base, s.cover, s.ring, s.lid]) {
        if v.mm(key) == 0.0 {
            used.push((key, length(value)));
        }
    }
    if v.mm("slit_width") == 0.0 {
        used.push(("slit_width", length(pattern.width.as_mm())));
    }
    if v.mm("slit_spacing") == 0.0 {
        used.push(("slit_spacing", length(pattern.spacing.as_mm())));
    }
    if v.count("slit_rows") == 0 {
        used.push(("slit_rows", ParamValue::Count(pattern.rows)));
    }
    used
}

/// The size handles on the inner base, the height handle on the strip and the corner radius
/// handle on the inner base's top-right corner (UX notes of `0147` §6).
fn handles(placed: &[layout::Placed], inner: &Footprint, s: &Sizes) -> Vec<Handle> {
    let origin = |role: &str| {
        placed
            .iter()
            .find(|p| p.role == role)
            .map_or((0.0, 0.0), |p| p.offset)
    };
    let (base, strip) = (origin("base"), origin("strip"));
    let (hw, hd) = (s.width / 2.0, s.depth / 2.0);
    let diagonal = std::f64::consts::FRAC_1_SQRT_2;
    // The first straight after the seam: its middle on the top edge of the strip. Without one
    // (a circle), the bridge beside the first tab window.
    let along = inner
        .spans
        .iter()
        .find(|span| {
            matches!(span.shape, crate::footprint::Shape::Straight { .. })
                && span.whole.is_some_and(|w| !w.seam)
        })
        .map(|span| span.s0 + span.length / 2.0)
        .or_else(|| {
            inner
                .spans
                .iter()
                .find(|span| span.window)
                .map(|span| span.s0 + s.wall / 2.0)
        })
        .unwrap_or(inner.total() / 2.0);
    // The arc's middle point moves 0.414 mm per mm of radius along the diagonal.
    let rate = 1.0 / (1.0 - diagonal) / std::f64::consts::SQRT_2;
    vec![
        Handle {
            key: "inner_width",
            anchor: Point::new(base.0 + hw, base.1),
            direction: Vec2::new(1.0, 0.0),
            rate: 2.0,
        },
        Handle {
            key: "inner_depth",
            anchor: Point::new(base.0, base.1 - hd),
            direction: Vec2::new(0.0, -1.0),
            rate: 2.0,
        },
        Handle {
            key: "inner_height",
            anchor: Point::new(strip.0 + along, strip.1),
            direction: Vec2::new(0.0, -1.0),
            rate: 1.0,
        },
        Handle {
            key: "corner_radius",
            anchor: Point::new(
                base.0 + hw - (1.0 - diagonal) * s.radius,
                base.1 - hd + (1.0 - diagonal) * s.radius,
            ),
            direction: Vec2::new(-diagonal, diagonal),
            rate,
        },
    ]
}
