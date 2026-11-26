//! The strip: the lid run, the quarter arc of the hinge and the back run, developed flat, with
//! the edge notches of the end plates' tabs and the finger end that meets the base.

use std::f64::consts::FRAC_PI_2;

use curvyo_document_core::{Length, Point};

use super::{Sizes, plane};
use crate::contour::Contour;
use crate::fingers::{finger_count, is_notched};
use crate::footprint::{Footprint, Shape, Tab};
use crate::slit_bend::SlitPattern;
use crate::wall::{Ends, Joint, Step, Wall};

/// A tab of an end plate is about this far from the next (spec `0147` criterion 18).
const TAB_PITCH: f64 = 30.0;

/// The footprint of the strip's inner face: the run over the top, the arc, the run down the back.
pub(super) fn footprint(s: &Sizes) -> Footprint {
    let (d, h, r) = (s.depth, s.height, s.radius);
    let top = Shape::Straight {
        from: plane(-s.front, h),
        to: plane(d - r, h),
    };
    let bend = Shape::Bend {
        centre: plane(d - r, h - r),
        radius: r,
        from: -FRAC_PI_2,
        sweep: FRAC_PI_2,
    };
    let back = Shape::Straight {
        from: plane(d, h - r),
        to: plane(d, -s.base),
    };
    Footprint::from_shapes(&[top, bend, back], s.wall, 2.0 * s.wall)
}

/// Where the back run starts on the strip, along the neutral line.
pub(super) fn back_start(s: &Sizes) -> f64 {
    (s.depth - s.radius + s.front) + FRAC_PI_2 * (s.radius + s.wall / 2.0)
}

/// The centres of the end plates' tabs, as the distance from the end plate's top edge: spread
/// over the back straight, at least two wall thicknesses below the arc and above the base.
pub(super) fn tab_centres(s: &Sizes) -> Vec<f64> {
    let (from, to) = (s.radius + 2.0 * s.wall, s.height - 2.0 * s.wall);
    let length = to - from;
    let fits = (length / (2.0 * s.end + 2.0 * s.wall)).floor().max(1.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = ((length / TAB_PITCH).round().max(1.0).min(fits)) as u32;
    (0..count)
        .map(|k| from + (f64::from(k) + 0.5) / f64::from(count) * length)
        .collect()
}

/// The steps of the strip's end edge, across its width: the finger joint with the base's back
/// edge. The zones over the end plates are cut back by the base's thickness (the base owns those
/// corner cubes), between them the strip keeps the odd pieces.
pub(super) fn end_steps(s: &Sizes, length: f64) -> Vec<Step> {
    let width = s.outer_width();
    let count = finger_count(s.width, 2.0 * s.base.min(s.wall));
    let step = s.width / f64::from(count);
    let (cut, full) = (length - s.base, length);
    let mut spans = vec![(0.0, s.end, cut)];
    for index in 0..count {
        let y0 = s.end + step * f64::from(index);
        let x = if is_notched(index, false) { cut } else { full };
        spans.push((y0, y0 + step, x));
    }
    spans.push((width - s.end, width, cut));
    let mut steps: Vec<Step> = Vec::new();
    for (y0, y1, x) in spans {
        match steps.last_mut() {
            Some(last) if (last.x - x).abs() < 1e-12 => last.y1 = y1,
            _ => steps.push(Step { y0, y1, x }),
        }
    }
    steps
}

/// The strip's contours: the outline first, then the slits that stay inside it. With `draft`
/// only the plain rectangle.
pub(super) fn strip(s: &Sizes, pattern: SlitPattern, draft: bool) -> Vec<Contour> {
    let footprint = footprint(s);
    let length = footprint.total();
    let back = back_start(s);
    let tabs: Vec<Tab> = tab_centres(s)
        .into_iter()
        .map(|y| {
            let along = back + y - s.radius;
            Tab {
                s: along,
                centre: Point::new(along, 0.0),
                along: (1.0, 0.0),
            }
        })
        .collect();
    let joint = |from_top| Joint {
        thickness: Length::from_mm(s.end),
        half_width: Length::from_mm(s.end),
        inset: Length::from_mm(0.0),
        from_top,
    };
    let wall = Wall {
        footprint: &footprint,
        height: Length::from_mm(s.outer_width()),
        pattern,
        slits_everywhere: false,
        joints: vec![joint(true), joint(false)],
        draft,
        ends: (!draft).then(|| Ends {
            start: vec![Step {
                y0: 0.0,
                y1: s.outer_width(),
                x: 0.0,
            }],
            end: end_steps(s, length),
            lead: 0.0,
            trail: 0.0,
        }),
        local_spacing: None,
        runs: false,
        seam_margin: 0.0,
    };
    wall.contours(&tabs)
}
