//! The page vector that puts a job's anchor on the user origin (`specs/0118` criterion 31,
//! `adrs.md` O5): the burn box, its corner at the profile's origin corner, the shift and the
//! placement built from them.

use crate::job_input::PointUm;
use crate::job_moves::Frame;
use crate::machine_profile::OriginCorner;

/// Where an absolute job stands on the bed (`specs/0118` O5): the page point it starts and ends
/// at and the page vector its moves are placed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Placement {
    /// The start and end in page µm (the machine origin, or the anchor with User origin).
    pub home: PointUm,
    /// The shift (zero but for User origin).
    pub shift: PointUm,
}

impl Placement {
    /// At the machine origin of `frame`, unshifted.
    #[must_use]
    pub const fn at_origin(frame: Frame) -> Self {
        Self {
            home: frame.origin(),
            shift: PointUm::new(0, 0),
        }
    }
}

/// The box of a job's burn points in page µm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BurnBox {
    /// The smallest X and Y.
    pub lo: PointUm,
    /// The largest X and Y.
    pub hi: PointUm,
}

/// The box of `points`, `None` without a point.
#[must_use]
pub fn burn_box(points: impl IntoIterator<Item = PointUm>) -> Option<BurnBox> {
    let mut points = points.into_iter();
    let first = points.next()?;
    Some(points.fold(
        BurnBox {
            lo: first,
            hi: first,
        },
        |b, p| BurnBox {
            lo: PointUm::new(b.lo.x.min(p.x), b.lo.y.min(p.y)),
            hi: PointUm::new(b.hi.x.max(p.x), b.hi.y.max(p.y)),
        },
    ))
}

/// The page point of the corner of `b` at `corner` (the bottom-left box corner for a
/// bottom-left machine): in machine coordinates always the box's min corner.
#[must_use]
pub const fn anchor(b: BurnBox, corner: OriginCorner) -> PointUm {
    PointUm::new(
        if corner.is_right() { b.hi.x } else { b.lo.x },
        if corner.is_bottom() { b.hi.y } else { b.lo.y },
    )
}

/// The page vector that moves the anchor of `b` onto the user origin `origin` (machine µm of
/// `frame`): `frame.page_point(origin) - anchor`.
#[must_use]
pub const fn shift(frame: Frame, b: BurnBox, origin: PointUm) -> PointUm {
    let at = frame.page_point(origin);
    let a = anchor(b, frame.corner);
    PointUm::new(at.x - a.x, at.y - a.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i64, y: i64) -> PointUm {
        PointUm::new(x, y)
    }

    #[test]
    fn the_anchor_is_the_box_corner_at_the_origin_corner_and_the_machine_min_corner() {
        let b = burn_box([p(10, 20), p(60, 40), p(30, 70)]).unwrap();
        assert_eq!((b.lo, b.hi), (p(10, 20), p(60, 70)));
        for (corner, expected) in [
            (OriginCorner::BottomLeft, p(10, 70)),
            (OriginCorner::BottomRight, p(60, 70)),
            (OriginCorner::TopLeft, p(10, 20)),
            (OriginCorner::TopRight, p(60, 20)),
        ] {
            assert_eq!(anchor(b, corner), expected, "{corner:?}");
            let frame = Frame {
                corner,
                width: 400,
                height: 300,
            };
            let machine =
                [b.lo, b.hi, p(b.lo.x, b.hi.y), p(b.hi.x, b.lo.y)].map(|q| frame.machine_point(q));
            let min = p(
                machine.iter().map(|q| q.x).min().unwrap(),
                machine.iter().map(|q| q.y).min().unwrap(),
            );
            assert_eq!(frame.machine_point(expected), min, "{corner:?}");
        }
    }

    #[test]
    fn the_shift_puts_the_anchor_on_the_user_origin() {
        let b = burn_box([p(10, 20), p(60, 70)]).unwrap();
        for corner in OriginCorner::ALL {
            let frame = Frame {
                corner,
                width: 400,
                height: 300,
            };
            let origin = p(150, 100);
            let d = shift(frame, b, origin);
            let a = anchor(b, corner);
            assert_eq!(frame.machine_point(p(a.x + d.x, a.y + d.y)), origin);
        }
        assert_eq!(burn_box([]), None);
    }
}
