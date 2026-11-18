//! GRBL's `$$` settings as read at identify (`specs/0127` G19); the connector never writes one.

use std::collections::BTreeMap;

use curvyo_connector_core::{ControllerSettings, Millimetres};

use crate::grbl::texts::millimetres;

/// Millimetres per inch.
const INCH: f64 = 25.4;

/// The settings GRBL printed, by number.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    values: BTreeMap<u16, String>,
}

impl Settings {
    /// Takes one `$<n>=<value>` line.
    pub fn set(&mut self, number: u16, value: &str) {
        self.values.insert(number, value.to_owned());
    }

    fn number(&self, n: u16) -> Option<f64> {
        self.values.get(&n)?.trim().parse().ok()
    }

    /// Laser mode, `$32`.
    #[must_use]
    pub fn laser_mode(&self) -> Option<bool> {
        self.number(32).map(|v| v != 0.0)
    }

    /// The S value of full power, `$30`, rounded.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped just before
    pub fn s_max(&self) -> Option<u32> {
        self.number(30)
            .filter(|v| v.is_finite() && *v >= 0.0)
            // invariant: a finite, non-negative value below 2^32 after the clamp.
            .map(|v| v.round().min(f64::from(u32::MAX)) as u32)
    }

    /// The travel of X and Y in millimetres, `$130` and `$131`.
    #[must_use]
    pub fn travel(&self) -> Option<[f64; 2]> {
        Some([self.number(130)?, self.number(131)?])
    }

    /// Whether status reports are in inches, `$13`.
    #[must_use]
    pub fn reports_inches(&self) -> bool {
        self.number(13).is_some_and(|v| v != 0.0)
    }

    /// A reported position in millimetres.
    #[must_use]
    pub fn to_mm(&self, value: f64) -> f64 {
        if self.reports_inches() {
            value * INCH
        } else {
            value
        }
    }

    /// The settings as the protocol carries them.
    #[must_use]
    pub fn to_wire(&self) -> ControllerSettings {
        ControllerSettings {
            laser_mode: self.laser_mode(),
            s_max: self.s_max(),
            travel: self.travel().map(|t| t.map(Millimetres)),
        }
    }

    /// The terminal's identify line after "GRBL 1.1h on /dev/ttyUSB0: " (UX notes section 10).
    #[must_use]
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(on) = self.laser_mode() {
            parts.push(format!("laser mode {}", if on { "on" } else { "off" }));
        }
        if let Some(s) = self.s_max() {
            parts.push(format!("max power {s}"));
        }
        if let Some([x, y]) = self.travel() {
            parts.push(format!("travel {} x {} mm", millimetres(x), millimetres(y)));
        }
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_values_the_controller_line_shows() {
        let mut s = Settings::default();
        assert_eq!(s.to_wire().laser_mode, None);
        s.set(32, "1");
        s.set(30, "999.6");
        s.set(130, "406.400");
        s.set(131, "304.8");
        s.set(13, "0");
        assert_eq!(s.laser_mode(), Some(true));
        assert_eq!(s.s_max(), Some(1000));
        assert_eq!(s.travel(), Some([406.4, 304.8]));
        assert!(!s.reports_inches());
        assert_eq!(
            s.summary(),
            "laser mode on, max power 1000, travel 406.4 x 304.8 mm"
        );
        s.set(13, "1");
        assert!((s.to_mm(1.0) - 25.4).abs() < 1e-9);
        s.set(32, "0");
        assert_eq!(s.laser_mode(), Some(false));
    }
}
