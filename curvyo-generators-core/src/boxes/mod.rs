//! The laser box: plates with finger joints, kerf compensation, an optional lid, laid out side by
//! side in one row.

mod plate;
mod schema;

use curvyo_document_core::{Length, Point, Vec2};
use curvyo_extension_core::{
    Edit, GenerateRequest, Generated, Handle, Param, ParamValue, Part, StoredParams,
};

use crate::outline::translated;
use crate::values::Values;
use plate::{Corner, Edge, Fingers, Plate};

pub(crate) use schema::groups;

/// Every box parameter, in order.
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

/// The stored values with a resize applied before the schema clamps them.
fn edited(request: &GenerateRequest<'_>) -> StoredParams {
    let mut raw = request.params.clone();
    if let Some(Edit::Scale(factor)) = request.edit {
        for (key, default) in [
            ("inner_width", 100.0),
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

/// The number of fingers on an edge of `length`: odd, as close to `length / width` as possible.
pub(crate) fn finger_count(length: f64, width: f64) -> u32 {
    let pairs = ((length / width - 1.0) / 2.0).round().max(0.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pairs = pairs as u32;
    2 * pairs + 1
}

/// How the lid is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lid {
    None,
    Flat,
    Fingers,
}

/// The dimensions every plate is built from.
#[derive(Debug, Clone, Copy)]
struct Sizes {
    width: f64,
    depth: f64,
    wall: f64,
    thickness: f64,
    clearance: f64,
    finger: (u32, u32, u32),
    lid: Lid,
    fingers: bool,
}

impl Sizes {
    fn horizontal(&self) -> Plate {
        let (t, w, d) = (self.thickness, self.width, self.depth);
        if !self.fingers {
            return plain(w, d, t, self.clearance);
        }
        let edge = |length: f64, count| {
            Edge::Fingered(Fingers {
                from: t,
                to: length - t,
                segments: count,
                even_solid: false,
            })
        };
        Plate {
            width: w,
            height: d,
            corners: [Corner::Owned; 4],
            edges: [
                edge(w, self.finger.0),
                edge(d, self.finger.1),
                edge(w, self.finger.0),
                edge(d, self.finger.1),
            ],
            depth: t,
            clearance: self.clearance,
        }
    }

    /// A wall: `across` is its horizontal length, `along` the finger count of that direction,
    /// `side_even_solid` whether it owns the even pieces of its vertical edges.
    fn wall(&self, across: f64, along: u32, side_even_solid: bool) -> Plate {
        let (t, h) = (self.thickness, self.wall);
        if !self.fingers {
            return plain(across, h, t, self.clearance);
        }
        let capped = self.lid == Lid::Fingers;
        let top = if capped { t } else { 0.0 };
        let horizontal = Edge::Fingered(Fingers {
            from: t,
            to: across - t,
            segments: along,
            even_solid: true,
        });
        let vertical = Edge::Fingered(Fingers {
            from: top,
            to: h - t,
            segments: self.finger.2,
            even_solid: side_even_solid,
        });
        let upper = if capped {
            Corner::Notched
        } else {
            Corner::Open
        };
        Plate {
            width: across,
            height: h,
            corners: [upper, upper, Corner::Notched, Corner::Notched],
            edges: [
                if capped { horizontal } else { Edge::Plain },
                vertical,
                horizontal,
                vertical,
            ],
            depth: t,
            clearance: self.clearance,
        }
    }
}

fn plain(width: f64, height: f64, depth: f64, clearance: f64) -> Plate {
    Plate {
        width,
        height,
        corners: [Corner::Open; 4],
        edges: [Edge::Plain; 4],
        depth,
        clearance,
    }
}

/// The plates in layout order: bottom, front, back, left, right and the lid when there is one.
fn plates(sizes: &Sizes) -> Vec<Plate> {
    let t = sizes.thickness;
    let side = if sizes.fingers {
        sizes.depth
    } else {
        sizes.depth - 2.0 * t
    };
    let mut plates = vec![
        sizes.horizontal(),
        sizes.wall(sizes.width, sizes.finger.0, true),
        sizes.wall(sizes.width, sizes.finger.0, true),
        sizes.wall(side, sizes.finger.1, false),
        sizes.wall(side, sizes.finger.1, false),
    ];
    match sizes.lid {
        Lid::None => {}
        Lid::Flat => plates.push(plain(sizes.width, sizes.depth, t, sizes.clearance)),
        Lid::Fingers => plates.push(sizes.horizontal()),
    }
    plates
}

/// The plates in one row, top aligned, centred on the origin, `spacing` apart: their parts and the
/// top-left corner of each.
fn layout(plates: &[Plate], spacing: f64, kerf: f64) -> (Vec<Part>, Vec<Point>) {
    let total: f64 = plates.iter().map(|p| p.width).sum::<f64>()
        + spacing * f64::from(u32::try_from(plates.len() - 1).unwrap_or(0));
    let tallest = plates.iter().map(|p| p.height).fold(0.0, f64::max);
    let (mut x, y) = (-total / 2.0, -tallest / 2.0);
    let mut parts = Vec::new();
    let mut origins = Vec::new();
    for plate in plates {
        origins.push(Point::new(x, y));
        parts.push(Part {
            paths: vec![translated(&plate.outline(kerf), x, y)],
        });
        x += plate.width + spacing;
    }
    (parts, origins)
}

/// The three size handles on the bottom and the front plate, `half` outside the cut edge.
fn handles(plates: &[Plate], origins: &[Point], half: f64) -> Vec<Handle> {
    let (bottom, front) = (origins[0], origins[1]);
    vec![
        Handle {
            key: "inner_width",
            anchor: Point::new(
                bottom.x + plates[0].width + half,
                bottom.y + plates[0].height / 2.0,
            ),
            direction: Vec2::new(1.0, 0.0),
            rate: 1.0,
        },
        Handle {
            key: "inner_depth",
            anchor: Point::new(bottom.x + plates[0].width / 2.0, bottom.y - half),
            direction: Vec2::new(0.0, -1.0),
            rate: 1.0,
        },
        Handle {
            key: "inner_height",
            anchor: Point::new(
                front.x + plates[1].width / 2.0,
                front.y + plates[1].height + half,
            ),
            direction: Vec2::new(0.0, 1.0),
            rate: 1.0,
        },
    ]
}

/// Generates the box `request` asks for.
pub(crate) fn generate(request: &GenerateRequest<'_>) -> Generated {
    let schema = params();
    let mut v = Values::read(&schema, &edited(request));
    let (iw, id, ih) = (
        v.mm("inner_width"),
        v.mm("inner_depth"),
        v.mm("inner_height"),
    );
    let smallest = iw.min(id).min(ih);
    v.limit_mm("thickness", 0.0, 0.5 * smallest);
    let fingers = v.choice("joint") == "fingers";
    let mut kerf_limit = v.mm("thickness");
    if fingers {
        v.limit_mm("finger_width", 0.0, smallest);
        let width = v.mm("finger_width");
        kerf_limit = kerf_limit.min(0.4 * width);
        v.limit_mm("clearance", 0.0, 0.4 * width);
    }
    v.limit_mm("kerf", 0.0, kerf_limit);
    let (t, kerf, gap) = (v.mm("thickness"), v.mm("kerf"), v.mm("gap"));
    let lid = match (v.choice("lid").as_str(), fingers) {
        ("flat", _) | ("fingers", false) => Lid::Flat,
        ("fingers", true) => Lid::Fingers,
        _ => Lid::None,
    };
    let finger_width = v.mm("finger_width");
    let count = |length: f64| {
        if fingers {
            finger_count(length, finger_width)
        } else {
            1
        }
    };
    let sizes = Sizes {
        width: iw + 2.0 * t,
        depth: id + 2.0 * t,
        wall: ih + t + if lid == Lid::Fingers { t } else { 0.0 },
        thickness: t,
        clearance: if fingers { v.mm("clearance") } else { 0.0 },
        finger: (count(iw), count(id), count(ih)),
        lid,
        fingers,
    };
    let plates = plates(&sizes);

    let (parts, origins) = layout(&plates, gap + kerf, kerf);
    let handles = handles(&plates, &origins, kerf / 2.0);
    let (params, clamps) = v.finish();
    Generated {
        params,
        parts,
        handles,
        clamps,
    }
}

#[cfg(test)]
pub(crate) fn plates_for_tests(sizes_json: &StoredParams) -> Vec<Plate> {
    let schema = params();
    let mut v = Values::read(&schema, sizes_json);
    let (iw, id, ih) = (
        v.mm("inner_width"),
        v.mm("inner_depth"),
        v.mm("inner_height"),
    );
    let t = v.mm("thickness");
    let fw = v.mm("finger_width");
    let lid = match v.choice("lid").as_str() {
        "flat" => Lid::Flat,
        "fingers" => Lid::Fingers,
        _ => Lid::None,
    };
    v.limit_mm("kerf", 0.0, t);
    plates(&Sizes {
        width: iw + 2.0 * t,
        depth: id + 2.0 * t,
        wall: ih + t + if lid == Lid::Fingers { t } else { 0.0 },
        thickness: t,
        clearance: v.mm("clearance"),
        finger: (
            finger_count(iw, fw),
            finger_count(id, fw),
            finger_count(ih, fw),
        ),
        lid,
        fingers: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(entries: &[(&str, ParamValue)]) -> StoredParams {
        entries
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.to_stored()))
            .collect()
    }

    fn mm(value: f64) -> ParamValue {
        ParamValue::Length(Length::from_mm(value))
    }

    fn assert_on_one_line(a: &[f64], b: &[f64], tolerance: f64) {
        assert_eq!(a.len(), b.len(), "the mated edges have the same fingers");
        assert_ne!(a.len(), 0);
        for (x, y) in a.iter().zip(b) {
            assert!((x - y).abs() <= tolerance + 1e-9, "{x} against {y}");
        }
    }

    #[test]
    fn mated_edges_lie_on_one_line_within_the_clearance() {
        for clearance in [0.0, 0.2] {
            for lid in ["none", "fingers"] {
                let plates = plates_for_tests(&stored(&[
                    ("clearance", mm(clearance)),
                    ("finger_width", mm(7.0)),
                    ("lid", ParamValue::Choice(lid.into())),
                ]));
                let (bottom, front, left) = (&plates[0], &plates[1], &plates[3]);
                // Front's bottom edge against the bottom's top edge, along x.
                assert_on_one_line(&front.steps(2), &bottom.steps(0), clearance / 2.0);
                // Left's bottom edge against the bottom's left edge, along y.
                assert_on_one_line(&left.steps(2), &bottom.steps(3), clearance / 2.0);
                // Front's vertical edges against the left plate's, along the height.
                assert_on_one_line(&front.steps(3), &left.steps(1), clearance / 2.0);
                assert_on_one_line(&front.steps(1), &left.steps(3), clearance / 2.0);
            }
        }
    }

    #[test]
    fn the_kerf_moves_every_outline_point_by_at_most_half_the_kerf_diagonal() {
        let plates = plates_for_tests(&stored(&[]));
        for plate in &plates {
            let nominal = plate.outline(0.0);
            let kerfed = plate.outline(0.2);
            assert_eq!(nominal.segments.len(), kerfed.segments.len());
            for (a, b) in nominal.segments.iter().zip(&kerfed.segments) {
                let d = (a.to.x - b.to.x).hypot(a.to.y - b.to.y);
                assert!((d - 0.1 * 2.0_f64.sqrt()).abs() < 1e-9, "{d}");
            }
        }
    }

    #[test]
    fn a_hundred_fingers_per_edge_still_build() {
        let plates = plates_for_tests(&stored(&[
            ("inner_width", mm(400.0)),
            ("inner_depth", mm(400.0)),
            ("inner_height", mm(400.0)),
            ("finger_width", mm(4.0)),
        ]));
        assert_eq!(finger_count(400.0, 4.0), 101);
        assert!(plates[0].outline(0.1).segments.len() > 300);
    }
}
