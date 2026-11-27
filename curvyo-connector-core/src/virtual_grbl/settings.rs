//! The `$` settings of the virtual GRBL (`specs/0161-cnc-output` criteria 15, 18): GRBL 1.1's
//! `defaults.h` (`DEFAULTS_GENERIC`) values, set and listed as `$<n>=<value>`.

use std::collections::BTreeMap;

use super::codes;

/// The settings GRBL 1.1 has, with their `DEFAULTS_GENERIC` values.
const DEFAULTS: &[(u16, f64)] = &[
    (0, 10.0),
    (1, 25.0),
    (2, 0.0),
    (3, 0.0),
    (4, 0.0),
    (5, 0.0),
    (6, 0.0),
    (10, 1.0),
    (11, 0.010),
    (12, 0.002),
    (13, 0.0),
    (20, 0.0),
    (21, 0.0),
    (22, 0.0),
    (23, 0.0),
    (24, 25.0),
    (25, 500.0),
    (26, 250.0),
    (27, 1.0),
    (30, 1_000.0),
    (31, 0.0),
    (32, 0.0),
    (100, 250.0),
    (101, 250.0),
    (102, 250.0),
    (110, 500.0),
    (111, 500.0),
    (112, 500.0),
    (120, 10.0),
    (121, 10.0),
    (122, 10.0),
    (130, 200.0),
    (131, 200.0),
    (132, 200.0),
];

/// Settings that are whole numbers (masks and switches).
const WHOLE: &[u16] = &[0, 2, 3, 4, 5, 6, 10, 13, 20, 21, 22, 23, 26, 32];

/// The virtual GRBL's `$` settings.
#[derive(Debug, Clone, PartialEq)]
pub struct GrblSettings {
    values: BTreeMap<u16, f64>,
}

impl Default for GrblSettings {
    fn default() -> Self {
        Self {
            values: DEFAULTS.iter().copied().collect(),
        }
    }
}

impl GrblSettings {
    /// The value of `$n`; 0 for one GRBL does not have.
    #[must_use]
    pub fn get(&self, n: u16) -> f64 {
        self.values.get(&n).copied().unwrap_or(0.0)
    }

    /// `$n=value` as GRBL takes it.
    ///
    /// # Errors
    /// `3` for a setting GRBL does not have, `2` for a value that does not read, `4` for a
    /// negative one, `10` for soft limits on while homing is off.
    pub fn set(&mut self, n: u16, value: &str) -> Result<(), u16> {
        if !self.values.contains_key(&n) {
            return Err(codes::INVALID_STATEMENT);
        }
        let v: f64 = value.parse().map_err(|_| codes::BAD_NUMBER_FORMAT)?;
        if !v.is_finite() {
            return Err(codes::BAD_NUMBER_FORMAT);
        }
        if v < 0.0 {
            return Err(codes::NEGATIVE_VALUE);
        }
        let v = if WHOLE.contains(&n) { v.trunc() } else { v };
        if n == 20 && v != 0.0 && self.get(22) == 0.0 {
            return Err(codes::HOMING_NOT_ENABLED);
        }
        self.values.insert(n, v);
        Ok(())
    }

    /// The `$$` listing, one `$<n>=<value>` per setting.
    #[must_use]
    pub fn listing(&self) -> Vec<String> {
        self.values
            .iter()
            .map(|(n, v)| {
                if WHOLE.contains(n) {
                    format!("${n}={v}")
                } else {
                    format!("${n}={v:.3}")
                }
            })
            .collect()
    }

    /// The maximum rate of axis `axis` (0 to 2) in mm/s.
    #[must_use]
    pub fn max_rate_mm_s(&self, axis: usize) -> f64 {
        self.axis(110, axis) / 60.0
    }

    /// The acceleration of axis `axis` in mm/s².
    #[must_use]
    pub fn acceleration(&self, axis: usize) -> f64 {
        self.axis(120, axis)
    }

    /// The travel of axis `axis` in mm.
    #[must_use]
    pub fn travel(&self, axis: usize) -> f64 {
        self.axis(130, axis)
    }

    /// The junction deviation `$11` in mm.
    #[must_use]
    pub fn junction_deviation(&self) -> f64 {
        self.get(11)
    }

    /// Whether soft limits are on (`$20`).
    #[must_use]
    pub fn soft_limits(&self) -> bool {
        self.get(20) != 0.0
    }

    /// Whether homing is on (`$22`).
    #[must_use]
    pub fn homing(&self) -> bool {
        self.get(22) != 0.0
    }

    /// The status report mask `$10`.
    #[must_use]
    pub fn report_mask(&self) -> u8 {
        // invariant: a whole number set through `set`, small
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mask = self.get(10) as u8;
        mask
    }

    fn axis(&self, base: u16, axis: usize) -> f64 {
        let offset = u16::try_from(axis.min(2)).unwrap_or(0);
        self.get(base + offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_limits_need_homing_and_values_keep_grbl_rules() {
        let mut s = GrblSettings::default();
        assert_eq!(s.set(20, "1"), Err(10));
        assert_eq!(s.set(22, "1"), Ok(()));
        assert_eq!(s.set(20, "1"), Ok(()));
        assert_eq!(s.set(99, "1"), Err(3));
        assert_eq!(s.set(110, "x"), Err(2));
        assert_eq!(s.set(110, "-1"), Err(4));
        assert_eq!(s.max_rate_mm_s(0), 500.0 / 60.0);
        assert!(s.listing().contains(&"$11=0.010".to_owned()));
        assert!(s.listing().contains(&"$32=0".to_owned()));
    }
}
