//! Where the drag knobs of a generated object sit in the document and what a drag does to the
//! value (`specs/0141-parametric-generators` UX notes §9); the knob type both parametric
//! instances share (`specs/0099-sketch-pages/adrs.md` A12).

use curvyo_document_core::{GeneratedSnapshot, Point, Vec2};
use curvyo_extension_core::ParamValue;

use crate::extensions::ExtensionSettings;
use crate::instance::{find_kind, generate};

/// How a knob turns pointer motion into a value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KnobMotion {
    /// Along a line: the value grows by `rate` per millimetre moved along `direction`.
    Along {
        /// The unit direction in which the value grows.
        direction: Vec2,
        /// How much the value changes per millimetre of drag along `direction`.
        rate: f64,
    },
    /// Around a point: the value grows by the degrees swept counter-clockwise around `centre`,
    /// times `sign` (-1 on a mirrored clone).
    Around {
        /// The vertex the knob turns about.
        centre: Point,
        /// 1, or -1 where the frame mirrors.
        sign: f64,
    },
}

/// One knob of a parametric instance, in document space.
#[derive(Debug, Clone, PartialEq)]
pub struct Knob {
    /// The parameter it changes.
    pub key: String,
    /// The parameter's label, for the readout ("Pitch diameter").
    pub label: String,
    /// Where it sits.
    pub at: Point,
    /// How a drag changes the value.
    pub motion: KnobMotion,
    /// The value it shows: millimetres, or degrees for [`KnobMotion::Around`].
    pub value: f64,
}

/// The knobs of `snapshot`, in the generator's order, `Vec::new()` for a generator that is not
/// available. Their positions follow the object's frame.
#[must_use]
pub fn knobs(settings: &ExtensionSettings, snapshot: &GeneratedSnapshot) -> Vec<Knob> {
    let Some(found) = find_kind(settings, &snapshot.record.kind) else {
        return Vec::new();
    };
    let Some(generated) = generate(&found, &snapshot.record.params, snapshot.record.seed, None)
    else {
        return Vec::new();
    };
    generated
        .handles
        .iter()
        .filter_map(|handle| {
            let param = found.kind.params().find(|param| param.key == handle.key)?;
            let ParamValue::Length(length) = param.read(&snapshot.record.params) else {
                return None;
            };
            let direction = snapshot.frame.apply_vector(handle.direction);
            let norm = direction.length();
            (norm > 1e-9).then(|| Knob {
                key: handle.key.to_owned(),
                label: param.label.to_owned(),
                at: snapshot.frame.apply_point(handle.anchor),
                motion: KnobMotion::Along {
                    direction: Vec2::new(direction.x / norm, direction.y / norm),
                    rate: handle.rate,
                },
                value: length.as_mm(),
            })
        })
        .collect()
}

/// The value a drag of `knob` from `from` to `to` asks for, in millimetres or degrees; `snap`
/// rounds it to whole ones (the session picks the key: Shift on a generator, `0141` UX notes §9;
/// Ctrl or Cmd on a sketch clone, `0099` criterion 25).
#[must_use]
pub fn dragged(knob: &Knob, from: Point, to: Point, snap: bool) -> f64 {
    let value = match knob.motion {
        KnobMotion::Along { direction, rate } => {
            let along = from.vector_to(to);
            knob.value + (along.x * direction.x + along.y * direction.y) * rate
        }
        KnobMotion::Around { centre, sign } => {
            let (a, b) = (centre.vector_to(from), centre.vector_to(to));
            let swept = (a.x * b.y - a.y * b.x).atan2(a.x * b.x + a.y * b.y);
            knob.value + swept.to_degrees() * sign
        }
    };
    if snap { value.round() } else { value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::{find_kind, new_record};
    use curvyo_document_core::Affine;

    fn gear(frame: Affine) -> GeneratedSnapshot {
        let found =
            find_kind(&ExtensionSettings::default(), "curvyo.generators/gear").expect("gear");
        let (record, _) = new_record(&found).expect("defaults");
        GeneratedSnapshot {
            id: curvyo_document_core::NodeId::NONE,
            record,
            frame,
        }
    }

    #[test]
    fn a_default_gear_has_the_bore_knob_on_its_circle_and_no_pitch_knob() {
        let knobs = knobs(&ExtensionSettings::default(), &gear(Affine::IDENTITY));
        let keys: Vec<&str> = knobs.iter().map(|knob| knob.key.as_str()).collect();
        assert_eq!(keys, ["bore"], "solid: no hub, no rim, no pitch diameter");
        // Bore 6 mm: radius 3 mm, at 0 degrees; a diameter changes at twice the pointer.
        let bore = &knobs[0];
        assert!((bore.at.x.hypot(bore.at.y) - 3.0).abs() < 1e-6);
        assert!((bore.value - 6.0).abs() < 1e-6, "{}", bore.value);
        assert!(
            matches!(bore.motion, KnobMotion::Along { rate, .. } if (rate - 2.0).abs() < 1e-12)
        );
    }

    #[test]
    fn the_knobs_follow_the_frame_and_a_drag_along_the_radius_changes_the_value() {
        let frame = Affine::new(0.0, 1.0, -1.0, 0.0, 100.0, 50.0);
        let knobs = knobs(&ExtensionSettings::default(), &gear(frame));
        let bore = knobs.iter().find(|knob| knob.key == "bore").expect("bore");
        let turning = Knob {
            motion: KnobMotion::Around {
                centre: Point::new(0.0, 0.0),
                sign: -1.0,
            },
            value: 30.0,
            ..bore.clone()
        };
        let quarter = dragged(
            &turning,
            Point::new(10.0, 0.0),
            Point::new(0.0, 10.0),
            false,
        );
        assert!(
            (quarter - (30.0 - 90.0)).abs() < 1e-9,
            "mirrored: {quarter}"
        );
        // The bore knob sits at 0 degrees, a quarter turn later it points along +y.
        assert!(
            (bore.at.x - 100.0).abs() < 1e-6 && (bore.at.y - 53.0).abs() < 1e-6,
            "{:?}",
            bore.at
        );
        let to = Point::new(bore.at.x, bore.at.y + 1.0);
        let value = dragged(bore, bore.at, to, false);
        assert!((value - (bore.value + 2.0)).abs() < 1e-9, "{value}");
        let across = Point::new(bore.at.x + 5.0, bore.at.y);
        assert!((dragged(bore, bore.at, across, false) - bore.value).abs() < 1e-9);
        assert!(
            (dragged(bore, bore.at, to, true) - 8.0).abs() < 1e-9,
            "whole millimetres"
        );
    }
}
