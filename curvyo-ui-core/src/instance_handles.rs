//! Where the drag knobs of a generated object sit in the document and what a drag does to the
//! value (`specs/0141-parametric-generators` UX notes §9).

use curvyo_document_core::{GeneratedSnapshot, Point, Vec2};
use curvyo_extension_core::ParamValue;

use crate::extensions::ExtensionSettings;
use crate::instance::{find_kind, generate};

/// One knob of a generated object, in document space.
#[derive(Debug, Clone, PartialEq)]
pub struct Knob {
    /// The parameter it changes.
    pub key: &'static str,
    /// The parameter's label, for the readout ("Pitch diameter").
    pub label: &'static str,
    /// Where it sits.
    pub at: Point,
    /// The unit direction in which the value grows.
    pub direction: Vec2,
    /// How much the value changes per millimetre of drag along `direction`.
    pub rate: f64,
    /// The stored value, in millimetres.
    pub value_mm: f64,
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
                key: handle.key,
                label: param.label,
                at: snapshot.frame.apply_point(handle.anchor),
                direction: Vec2::new(direction.x / norm, direction.y / norm),
                rate: handle.rate,
                value_mm: length.as_mm(),
            })
        })
        .collect()
}

/// The value a drag of `knob` from `from` to `to` asks for, in millimetres; `snap_whole` rounds it
/// to whole millimetres (Shift, UX notes §9).
#[must_use]
pub fn dragged_mm(knob: &Knob, from: Point, to: Point, snap_whole: bool) -> f64 {
    let along = from.vector_to(to);
    let along = along.x * knob.direction.x + along.y * knob.direction.y;
    let value = knob.value_mm + along * knob.rate;
    if snap_whole { value.round() } else { value }
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
    fn a_default_gear_has_the_pitch_diameter_and_the_bore_knob_on_its_circles() {
        let knobs = knobs(&ExtensionSettings::default(), &gear(Affine::IDENTITY));
        let keys: Vec<&str> = knobs.iter().map(|knob| knob.key).collect();
        assert_eq!(keys, ["pitch_diameter", "bore"], "solid: no hub, no rim");
        // Module 2 and 24 teeth: pitch radius 24 mm, at 45 degrees.
        let pitch = &knobs[0];
        let radius = pitch.at.x.hypot(pitch.at.y);
        assert!((radius - 24.0).abs() < 1e-6, "{radius}");
        assert!((pitch.value_mm - 48.0).abs() < 1e-6, "{}", pitch.value_mm);
        assert!((pitch.rate - 2.0).abs() < 1e-12);
    }

    #[test]
    fn the_knobs_follow_the_frame_and_a_drag_along_the_radius_changes_the_value() {
        let frame = Affine::new(0.0, 1.0, -1.0, 0.0, 100.0, 50.0);
        let knobs = knobs(&ExtensionSettings::default(), &gear(frame));
        let bore = knobs.iter().find(|knob| knob.key == "bore").expect("bore");
        // The bore knob sits at 0 degrees, a quarter turn later it points along +y.
        assert!(
            (bore.at.x - 100.0).abs() < 1e-6 && (bore.at.y - 53.0).abs() < 1e-6,
            "{:?}",
            bore.at
        );
        let to = Point::new(bore.at.x, bore.at.y + 1.0);
        let value = dragged_mm(bore, bore.at, to, false);
        assert!((value - (bore.value_mm + 2.0)).abs() < 1e-9, "{value}");
        let across = Point::new(bore.at.x + 5.0, bore.at.y);
        assert!((dragged_mm(bore, bore.at, across, false) - bore.value_mm).abs() < 1e-9);
        assert!(
            (dragged_mm(bore, bore.at, to, true) - 8.0).abs() < 1e-9,
            "whole millimetres"
        );
    }
}
