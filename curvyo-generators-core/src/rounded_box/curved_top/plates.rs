//! The four plates of the curved top: two end plates, the front and the base, with the Box's
//! corner rule (ADR 0082 §1, §11): the base owns every bottom corner cube, the front owns the end
//! pieces of the vertical joint.

use super::Sizes;
use super::strip::tab_centres;
use crate::fingers::Split;
use crate::joint_layout;
use crate::plate::{Corner, Edge, Fingers, Plate};
use crate::rounded_box::layout::Piece;
use crate::wall_plates::Hollow;

/// The finger pieces of each joint, by the thinner of its two parts (spec criterion 18) or the
/// joint layout (spec `0155` criterion 25).
pub(super) struct Counts {
    /// Front and end plate, along the height.
    pub(super) front_end: Split,
    /// Front and base, across the width.
    pub(super) front_base: Split,
    /// Base and end plate, along the depth.
    pub(super) base_end: Split,
    /// Base and the strip's end, across the width.
    pub(super) base_strip: Split,
}

impl Counts {
    pub(super) fn of(s: &Sizes) -> Self {
        let split = |length: f64, a: f64, b: f64| {
            joint_layout::finger_split(length, 2.0 * a.min(b), &s.joints)
        };
        Self {
            front_end: split(s.height, s.front, s.end),
            front_base: split(s.width, s.front, s.base),
            base_end: split(s.depth, s.base, s.end),
            base_strip: split(s.width, s.base, s.wall),
        }
    }

    /// The most fingers on one joint.
    pub(super) fn most_fingers(&self) -> u32 {
        [
            &self.front_end,
            &self.front_base,
            &self.base_end,
            &self.base_strip,
        ]
        .into_iter()
        .map(Split::fingers)
        .max()
        .unwrap_or(0)
    }
}

fn fingers(from: f64, to: f64, split: &Split, even_solid: bool) -> Edge {
    Edge::Fingered(Fingers::new(from, to, split, even_solid))
}

/// An end plate: the profile, front at the left, the back-top corner round, tabs on the back
/// edge, fingers to the front and to the base.
pub(super) fn end_plate(s: &Sizes, counts: &Counts) -> Plate {
    let (w, h) = (s.profile_width(), s.profile_height());
    let tabs = tab_centres(s)
        .into_iter()
        .map(|y| (y - s.joints.half(s.end), y + s.joints.half(s.end)))
        .collect();
    Plate {
        width: w,
        height: h,
        corners: [
            Corner::Open,
            Corner::Round(s.radius),
            Corner::Open,
            Corner::Notched,
        ],
        edges: [
            Edge::Plain,
            Edge::Tabs(tabs),
            fingers(s.front, w, &counts.base_end, false),
            fingers(0.0, h - s.base, &counts.front_end, false),
        ],
        depths: [0.0, s.wall, s.base, s.front],
        clearance: 0.0,
        hollow: None,
    }
}

/// The front: the Box's uncapped front wall, its top edge the seat, with the finger hollow.
pub(super) fn front(s: &Sizes, counts: &Counts, hollow: Option<Hollow>) -> Plate {
    let (w, h) = (s.outer_width(), s.profile_height());
    let side = fingers(0.0, h - s.base, &counts.front_end, true);
    Plate {
        width: w,
        height: h,
        corners: [Corner::Open, Corner::Open, Corner::Notched, Corner::Notched],
        edges: [
            Edge::Plain,
            side.clone(),
            fingers(s.end, w - s.end, &counts.front_base, false),
            side,
        ],
        depths: [0.0, s.end, s.base, s.end],
        clearance: 0.0,
        hollow,
    }
}

/// The base: the Box's bottom, owning every corner cube, fingered on all four edges.
pub(super) fn base(s: &Sizes, counts: &Counts) -> Plate {
    let (w, d) = (s.outer_width(), s.depth + s.front + s.wall);
    let side = fingers(s.front, d - s.wall, &counts.base_end, true);
    Plate {
        width: w,
        height: d,
        corners: [Corner::Owned; 4],
        edges: [
            fingers(s.end, w - s.end, &counts.front_base, true),
            side.clone(),
            fingers(s.end, w - s.end, &counts.base_strip, true),
            side,
        ],
        depths: [s.front, s.end, s.wall, s.end],
        clearance: 0.0,
        hollow: None,
    }
}

/// The plates in layout and numbering order: 2 and 3 the end plates, 4 the front, 5 the base.
pub(super) fn plates(s: &Sizes, hollow: Option<Hollow>) -> Vec<Piece> {
    let counts = Counts::of(s);
    let piece = |plate: Plate, thickness: f64, role: &'static str, number: u32| {
        let mut piece = Piece::new(vec![plate.contour()], thickness, role);
        piece.number = number;
        piece
    };
    vec![
        piece(end_plate(s, &counts), s.end, "left", 2),
        piece(end_plate(s, &counts), s.end, "right", 3),
        piece(front(s, &counts, hollow), s.front, "front", 4),
        piece(base(s, &counts), s.base, "base", 5),
    ]
}
