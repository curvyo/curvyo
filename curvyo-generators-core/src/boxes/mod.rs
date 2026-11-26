//! The laser box: plates with finger joints, kerf compensation, an optional lid, laid out side by
//! side in one row.

mod schema;
#[cfg(test)]
mod tests;

use curvyo_document_core::{Length, Point, Vec2};
use curvyo_extension_core::{
    Edit, GenerateRequest, Generated, Handle, Param, ParamValue, Part, StoredParams,
};

use crate::fingers::Split;
use crate::joint_layout::{self, Joints};
use crate::outline::translated;
use crate::plate::{Corner, Edge, Fingers, Plate};
use crate::values::Values;

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

/// How the lid is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lid {
    None,
    Flat,
    Fingers,
}

/// The dimensions every plate is built from.
#[derive(Debug, Clone)]
struct Sizes {
    width: f64,
    depth: f64,
    wall: f64,
    thickness: f64,
    clearance: f64,
    /// The pieces of the joint edges along the width, the depth and the height.
    finger: (Split, Split, Split),
    lid: Lid,
}

impl Sizes {
    fn horizontal(&self) -> Plate {
        let (t, w, d) = (self.thickness, self.width, self.depth);
        let edge =
            |length: f64, split: &Split| Edge::Fingered(Fingers::new(t, length - t, split, true));
        Plate {
            width: w,
            height: d,
            corners: [Corner::Owned; 4],
            edges: [
                edge(w, &self.finger.0),
                edge(d, &self.finger.1),
                edge(w, &self.finger.0),
                edge(d, &self.finger.1),
            ],
            depths: [t; 4],
            clearance: self.clearance,
            hollow: None,
        }
    }

    /// A wall: `across` is its horizontal length, `along` the pieces of that direction,
    /// `side_even_solid` whether it owns the even pieces of its vertical edges.
    fn wall(&self, across: f64, along: &Split, side_even_solid: bool) -> Plate {
        let (t, h) = (self.thickness, self.wall);
        let capped = self.lid == Lid::Fingers;
        let top = if capped { t } else { 0.0 };
        let horizontal = Edge::Fingered(Fingers::new(t, across - t, along, false));
        let vertical = Edge::Fingered(Fingers::new(top, h - t, &self.finger.2, side_even_solid));
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
                if capped {
                    horizontal.clone()
                } else {
                    Edge::Plain
                },
                vertical.clone(),
                horizontal,
                vertical,
            ],
            depths: [t; 4],
            clearance: self.clearance,
            hollow: None,
        }
    }
}

fn plain(width: f64, height: f64, depth: f64, clearance: f64) -> Plate {
    Plate {
        width,
        height,
        corners: [Corner::Open; 4],
        edges: [Edge::Plain, Edge::Plain, Edge::Plain, Edge::Plain],
        depths: [depth; 4],
        clearance,
        hollow: None,
    }
}

/// The plates in layout order: bottom, front, back, left, right and the lid when there is one.
fn plates(sizes: &Sizes) -> Vec<Plate> {
    let t = sizes.thickness;
    let side = sizes.depth;
    let mut plates = vec![
        sizes.horizontal(),
        sizes.wall(sizes.width, &sizes.finger.0, true),
        sizes.wall(sizes.width, &sizes.finger.0, true),
        sizes.wall(side, &sizes.finger.1, false),
        sizes.wall(side, &sizes.finger.1, false),
    ];
    match sizes.lid {
        Lid::None => {}
        Lid::Flat => plates.push(plain(sizes.width, sizes.depth, t, sizes.clearance)),
        Lid::Fingers => plates.push(sizes.horizontal()),
    }
    plates
}

/// The part keys of the plates in layout order (ADR 0082 §10).
const PLATE_KEYS: [&str; 6] = ["bottom", "front", "back", "left", "right", "lid"];

/// The plates in one row, top aligned, centred on the origin, `spacing` apart: their parts and the
/// top-left corner of each.
fn layout(plates: &[Plate], spacing: f64, kerf: f64) -> (Vec<Part>, Vec<Point>) {
    let total: f64 = plates.iter().map(|p| p.width).sum::<f64>()
        + spacing * f64::from(u32::try_from(plates.len() - 1).unwrap_or(0));
    let tallest = plates.iter().map(|p| p.height).fold(0.0, f64::max);
    let (mut x, y) = (-total / 2.0, -tallest / 2.0);
    let mut parts = Vec::new();
    let mut origins = Vec::new();
    for (plate, key) in plates.iter().zip(PLATE_KEYS) {
        origins.push(Point::new(x, y));
        parts.push(Part {
            key: key.into(),
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
    v.limit_mm("finger_width", 0.0, smallest);
    let width = v.mm("finger_width");
    v.limit_mm("clearance", 0.0, 0.4 * width);
    v.limit_mm("kerf", 0.0, v.mm("thickness").min(0.4 * width));
    let (t, kerf, gap) = (v.mm("thickness"), v.mm("kerf"), v.mm("gap"));
    let lid = match v.choice("lid").as_str() {
        "flat" => Lid::Flat,
        "fingers" => Lid::Fingers,
        _ => Lid::None,
    };
    // Finger width is the Joint size: its own clamps stay, the layout fits count, gap and spread.
    let asked = Joints::read(&v, "finger_width");
    let mut fitted = joint_layout::fit_fingers(
        &[iw, id, ih],
        width,
        Joints {
            width: None,
            ..asked
        },
    );
    joint_layout::apply(&mut v, &fitted.limits, "finger_width");
    let joints = Joints {
        width: Some(width),
        ..fitted.joints
    };
    let split = |span: f64| joint_layout::finger_split(span, width, &joints);
    let finger = (split(iw), split(id), split(ih));
    fitted.used_count = Some(
        finger
            .0
            .fingers()
            .max(finger.1.fingers())
            .max(finger.2.fingers()),
    );
    let sizes = Sizes {
        width: iw + 2.0 * t,
        depth: id + 2.0 * t,
        wall: ih + t + if lid == Lid::Fingers { t } else { 0.0 },
        thickness: t,
        clearance: v.mm("clearance"),
        finger,
        lid,
    };
    let plates = plates(&sizes);

    let (parts, origins) = layout(&plates, gap + kerf, kerf);
    let handles = handles(&plates, &origins, kerf / 2.0);
    let automatic = joint_layout::used(&v, &fitted, ("finger_width", width));
    let warnings = joint_layout::pattern_warnings(&v);
    let (params, clamps) = v.finish();
    Generated {
        params,
        parts,
        handles,
        clamps,
        automatic,
        states: Vec::new(),
        limits: Vec::new(),
        warnings,
        marks: curvyo_extension_core::Marks::default(),
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
            Split::equal(iw, fw),
            Split::equal(id, fw),
            Split::equal(ih, fw),
        ),
        lid,
    })
}
