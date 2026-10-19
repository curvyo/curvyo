//! The Loro keys of an ellipse's arc and a shape's curve
//! (`specs/0021-ellipse-arcs-and-shaping/adrs.md`, decisions 1 and 2): their read,
//! their validation on open and their writes, each deleted at the shape's default.
//!
//! ```text
//! ellipse:          ellipse_arc  [start°, end°, "pie" | "arc" | "chord"]  ONE LWW register,
//!                                absent = whole
//! ellipse, polygon,
//! star:             curve        fraction -1..=1, LWW register, absent = the default
//!                                (ellipse 1, polygon and star 0)
//! ```

use loro::{LoroMap, LoroValue};

use crate::ellipse_arc::{ArcKind, EllipseArc};
use crate::path_codec;
use crate::ring_curve::Curve;
use crate::units::Angle;

pub(crate) const KEY_ELLIPSE_ARC: &str = "ellipse_arc";
pub(crate) const KEY_CURVE: &str = "curve";

/// What a stored register holds: nothing, a valid value, or something a file must be
/// refused for.
enum Stored<T> {
    Absent,
    Valid(T),
    Damaged,
}

fn stored_arc(meta: &LoroMap) -> Stored<Option<EllipseArc>> {
    let Some(value) = meta.get(KEY_ELLIPSE_ARC) else {
        return Stored::Absent;
    };
    let LoroValue::List(list) = value.get_deep_value() else {
        return Stored::Damaged;
    };
    let (Some(start), Some(end), Some(LoroValue::String(kind))) =
        (list.first(), list.get(1), list.get(2))
    else {
        return Stored::Damaged;
    };
    let (Some(start), Some(end), Some(kind)) = (
        path_codec::as_f64(start),
        path_codec::as_f64(end),
        ArcKind::from_name(kind),
    ) else {
        return Stored::Damaged;
    };
    if list.len() != 3 || !start.is_finite() || !end.is_finite() {
        return Stored::Damaged;
    }
    // Equal angles (a value no writer stores) read as the whole ellipse.
    Stored::Valid(EllipseArc::new(
        Angle::from_radians(start.to_radians()),
        Angle::from_radians(end.to_radians()),
        kind,
    ))
}

fn stored_curve(meta: &LoroMap) -> Stored<Curve> {
    let Some(value) = meta.get(KEY_CURVE) else {
        return Stored::Absent;
    };
    match path_codec::as_f64(&value.get_deep_value()).and_then(Curve::new) {
        Some(curve) => Stored::Valid(curve),
        None => Stored::Damaged,
    }
}

/// The ellipse's arc, `None` for the whole ellipse (absent, or unreadable on a document
/// that was validated on open).
pub(crate) fn read_arc(meta: &LoroMap) -> Option<EllipseArc> {
    match stored_arc(meta) {
        Stored::Valid(arc) => arc,
        Stored::Absent | Stored::Damaged => None,
    }
}

/// The shape's stored curve, `default` when absent.
pub(crate) fn read_curve(meta: &LoroMap, default: Curve) -> Curve {
    match stored_curve(meta) {
        Stored::Valid(curve) => curve,
        Stored::Absent | Stored::Damaged => default,
    }
}

/// Whether an ellipse's arc and curve registers open (criteria 15, 33): a readable arc,
/// a finite curve within -1..=1, and never an arc together with a curve other than 1.
pub(crate) fn ellipse_arc_and_curve_are_valid(meta: &LoroMap) -> bool {
    let arc = match stored_arc(meta) {
        Stored::Damaged => return false,
        Stored::Absent => None,
        Stored::Valid(arc) => arc,
    };
    match stored_curve(meta) {
        Stored::Damaged => false,
        Stored::Absent => true,
        Stored::Valid(curve) => arc.is_none() || curve == Curve::ELLIPSE_DEFAULT,
    }
}

/// Whether a polygon's or star's curve register opens (criterion 50); a value below the
/// shape's limit is valid.
pub(crate) fn polygon_curve_is_valid(meta: &LoroMap) -> bool {
    !matches!(stored_curve(meta), Stored::Damaged)
}

fn delete_if_present(meta: &LoroMap, key: &str) -> bool {
    if meta.get(key).is_none() {
        return false;
    }
    // invariant: deleting a key just confirmed present on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.delete(key).unwrap();
    true
}

/// The stored form of `arc`'s angles, degrees.
fn degrees(arc: EllipseArc) -> (f64, f64) {
    (
        arc.start().as_radians().to_degrees(),
        arc.end().as_radians().to_degrees(),
    )
}

/// Writes `arc` unless the node already reads it; `None` deletes the register
/// (`adrs.md` decision 1). Returns whether it wrote.
pub(crate) fn write_arc_if_changed(meta: &LoroMap, arc: Option<EllipseArc>) -> bool {
    let Some(arc) = arc else {
        return delete_if_present(meta, KEY_ELLIPSE_ARC);
    };
    if let Stored::Valid(Some(old)) = stored_arc(meta)
        && old.kind == arc.kind
        && degrees(old) == degrees(arc)
    {
        return false;
    }
    let (start, end) = degrees(arc);
    let list = LoroValue::List(
        vec![
            LoroValue::Double(start),
            LoroValue::Double(end),
            LoroValue::String(arc.kind.name().into()),
        ]
        .into(),
    );
    // invariant: inserting a plain list under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_ELLIPSE_ARC, list).unwrap();
    true
}

/// Writes `curve` unless the node already reads it; the shape's `default` deletes the
/// register. Returns whether it wrote.
pub(crate) fn write_curve_if_changed(meta: &LoroMap, curve: Curve, default: Curve) -> bool {
    if curve == default {
        return delete_if_present(meta, KEY_CURVE);
    }
    if let Stored::Valid(old) = stored_curve(meta)
        && old == curve
    {
        return false;
    }
    // invariant: inserting a number under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_CURVE, curve.get()).unwrap();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use loro::LoroDoc;

    fn meta() -> (LoroDoc, LoroMap) {
        let doc = LoroDoc::new();
        let map = doc.get_map("m");
        (doc, map)
    }

    fn arc(start: f64, end: f64, kind: ArcKind) -> EllipseArc {
        EllipseArc::new(
            Angle::from_radians(start.to_radians()),
            Angle::from_radians(end.to_radians()),
            kind,
        )
        .unwrap()
    }

    /// Decision 1: the arc is one list, deleted for the whole ellipse; an unchanged value
    /// is not rewritten.
    #[test]
    fn the_arc_round_trips_and_whole_deletes_it() {
        let (_doc, m) = meta();
        assert!(read_arc(&m).is_none());
        let a = arc(30.0, 150.0, ArcKind::Pie);
        assert!(write_arc_if_changed(&m, Some(a)));
        assert!(!write_arc_if_changed(&m, Some(a)));
        let back = read_arc(&m).unwrap();
        assert_eq!(back.kind, ArcKind::Pie);
        assert!((back.start().as_radians().to_degrees() - 30.0).abs() < 1e-9);
        assert!(ellipse_arc_and_curve_are_valid(&m));
        assert!(write_arc_if_changed(&m, None));
        assert!(m.get(KEY_ELLIPSE_ARC).is_none());
        assert!(!write_arc_if_changed(&m, None));
    }

    /// Decision 1: the curve is deleted at the shape's default.
    #[test]
    fn the_curve_is_deleted_at_the_default() {
        let (_doc, m) = meta();
        let c = Curve::new(-0.35).unwrap();
        assert!(write_curve_if_changed(&m, c, Curve::ELLIPSE_DEFAULT));
        assert!(!write_curve_if_changed(&m, c, Curve::ELLIPSE_DEFAULT));
        assert_eq!(read_curve(&m, Curve::ELLIPSE_DEFAULT), c);
        assert!(write_curve_if_changed(
            &m,
            Curve::ELLIPSE_DEFAULT,
            Curve::ELLIPSE_DEFAULT
        ));
        assert!(m.get(KEY_CURVE).is_none());
        assert_eq!(
            read_curve(&m, Curve::POLYGON_DEFAULT),
            Curve::POLYGON_DEFAULT
        );
    }

    /// Decision 2, criteria 33 and 50: damaged registers.
    #[test]
    fn damaged_registers_are_refused() {
        let cases: Vec<(&str, LoroValue)> = vec![
            (KEY_CURVE, LoroValue::Double(2.0)),
            (KEY_CURVE, LoroValue::Double(f64::NAN)),
            (KEY_CURVE, LoroValue::String("x".into())),
            (
                KEY_ELLIPSE_ARC,
                LoroValue::List(
                    vec![
                        LoroValue::Double(1.0),
                        LoroValue::Double(2.0),
                        LoroValue::String("wedge".into()),
                    ]
                    .into(),
                ),
            ),
            (
                KEY_ELLIPSE_ARC,
                LoroValue::List(vec![LoroValue::Double(1.0), LoroValue::Double(2.0)].into()),
            ),
            (
                KEY_ELLIPSE_ARC,
                LoroValue::List(
                    vec![
                        LoroValue::Double(f64::INFINITY),
                        LoroValue::Double(2.0),
                        LoroValue::String("pie".into()),
                    ]
                    .into(),
                ),
            ),
        ];
        for (key, value) in cases {
            let (_doc, m) = meta();
            m.insert(key, value.clone()).unwrap();
            assert!(!ellipse_arc_and_curve_are_valid(&m), "{key} {value:?}");
        }
        let (_doc, m) = meta();
        write_arc_if_changed(&m, Some(arc(0.0, 90.0, ArcKind::Arc)));
        assert!(ellipse_arc_and_curve_are_valid(&m));
        m.insert(KEY_CURVE, 0.5).unwrap();
        assert!(!ellipse_arc_and_curve_are_valid(&m), "arc with a curve");
        let (_doc, m) = meta();
        m.insert(KEY_CURVE, -1.0).unwrap();
        assert!(polygon_curve_is_valid(&m));
        m.insert(KEY_CURVE, -1.5).unwrap();
        assert!(!polygon_curve_is_valid(&m));
    }
}
