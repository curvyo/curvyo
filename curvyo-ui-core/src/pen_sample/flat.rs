//! The flat record of a [`PenSample`] that crosses the wasm boundary once per frame (ADR 0095
//! decision 1): `PEN_SAMPLE_STRIDE` numbers per sample in field order.

use super::{PenEnd, PenPhase, PenSample, Pressure, Tilt};
use crate::ScreenPx;

/// Numbers per flat sample record: `x`, `y` (CSS px), `pressure` (NaN = none), `tilt_x`,
/// `tilt_y` (NaN = none), `end` (0 tip, 1 eraser), `phase` (0 hover, 1 contact, 2 out),
/// `barrel` bits, `time_ms`.
pub const PEN_SAMPLE_STRIDE: usize = 9;

impl PenSample {
    /// The samples of a run of flat records; a short trailing record, or one with a non-finite
    /// position or time or an unknown end or phase, is skipped.
    #[must_use]
    pub fn decode_flat(flat: &[f64]) -> Vec<Self> {
        let (records, _) = flat.as_chunks::<PEN_SAMPLE_STRIDE>();
        records.iter().filter_map(Self::decode_record).collect()
    }

    fn decode_record(record: &[f64; PEN_SAMPLE_STRIDE]) -> Option<Self> {
        let [x, y, pressure, tilt_x, tilt_y, end, phase, barrel, time_ms] = *record;
        if !(x.is_finite() && y.is_finite() && time_ms.is_finite()) {
            return None;
        }
        let end = match end {
            0.0 => PenEnd::Tip,
            1.0 => PenEnd::Eraser,
            _ => return None,
        };
        let phase = match phase {
            0.0 => PenPhase::Hover,
            1.0 => PenPhase::Contact,
            2.0 => PenPhase::Out,
            _ => return None,
        };
        let tilt = (tilt_x.is_finite() && tilt_y.is_finite()).then(|| Tilt::new(tilt_x, tilt_y));
        // Barrel bits are 0 to 3; anything else reads as none.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let barrel = if (0.0..=255.0).contains(&barrel) {
            barrel as u8
        } else {
            0
        };
        Some(Self {
            x: ScreenPx(x),
            y: ScreenPx(y),
            pressure: pressure.is_finite().then(|| Pressure::new(pressure)),
            tilt,
            end,
            phase,
            barrel,
            time_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(values: [f64; PEN_SAMPLE_STRIDE]) -> Vec<f64> {
        values.to_vec()
    }

    #[test]
    fn a_record_decodes_in_field_order() {
        let flat = record([10.0, 20.0, 0.25, 0.5, -0.5, 0.0, 1.0, 2.0, 1234.5]);
        let samples = PenSample::decode_flat(&flat);
        assert_eq!(samples.len(), 1);
        let s = samples[0];
        assert_eq!((s.x.0, s.y.0), (10.0, 20.0));
        assert_eq!(s.pressure.map(Pressure::value), Some(0.25));
        assert_eq!(s.tilt.map(|t| (t.x(), t.y())), Some((0.5, -0.5)));
        assert_eq!(
            (s.end, s.phase, s.barrel),
            (PenEnd::Tip, PenPhase::Contact, 2)
        );
        assert!((s.time_ms - 1234.5).abs() < f64::EPSILON);
    }

    #[test]
    fn nan_means_none_and_bad_records_are_skipped() {
        let mut flat = record([1.0, 2.0, f64::NAN, f64::NAN, f64::NAN, 1.0, 2.0, 0.0, 5.0]);
        flat.extend(record([f64::NAN, 2.0, 0.5, 0.0, 0.0, 0.0, 1.0, 0.0, 6.0]));
        flat.extend(record([1.0, 2.0, 0.5, 0.0, 0.0, 3.0, 1.0, 0.0, 7.0]));
        flat.extend(record([1.0, 2.0, 0.5, 0.0, 0.0, 0.0, 9.0, 0.0, 8.0]));
        flat.extend([1.0, 2.0, 3.0]);
        let samples = PenSample::decode_flat(&flat);
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].pressure, None);
        assert_eq!(samples[0].tilt, None);
        assert_eq!(
            (samples[0].end, samples[0].phase),
            (PenEnd::Eraser, PenPhase::Out)
        );
    }
}
