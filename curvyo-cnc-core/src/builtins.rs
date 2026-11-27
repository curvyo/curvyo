//! The built-in machine profiles of CNC milling, in code only and never written to a file (ADR 0038
//! §4, ADR 0097 §11 step G2, `specs/0161-cnc-output` criterion 2).

use curvyo_document_core::{DocumentSize, Length};
use curvyo_library_core::Connection;

use crate::profile::{CncProfile, CncValues, OutputFormat};
use crate::units::{AccelMmS2, FeedMmMin, Rpm};

/// One built-in profile: its id (not a UUID, as `generic-laser` is not) and its name; its values
/// come from [`Builtin::profile`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Builtin {
    /// The stable id.
    pub id: &'static str,
    /// The name the profile pick shows.
    pub name: &'static str,
}

/// GRBL 1.1's `defaults.h` (`DEFAULTS_GENERIC`): 500 mm/min and 10 mm/s² on every axis.
const GRBL_MAX_RATE: f64 = 500.0;
const GRBL_ACCELERATION: f64 = 10.0;

impl Builtin {
    /// The built-in as a profile (criterion 2, Proposal): GRBL, 300 x 300 mm, Z travel 60 mm,
    /// spindle up to 12 000 rpm, GRBL's default rates and accelerations, safe height 5 mm.
    #[must_use]
    pub fn profile(&self) -> CncProfile {
        // invariant: the GRBL defaults lie within the ranges of their units
        let rate = FeedMmMin::new(GRBL_MAX_RATE).unwrap_or_else(|| unreachable!());
        let accel = AccelMmS2::new(GRBL_ACCELERATION).unwrap_or_else(|| unreachable!());
        CncProfile {
            id: self.id.to_owned(),
            modified: 0,
            values: CncValues {
                name: self.name.to_owned(),
                output_format: OutputFormat::Grbl,
                work_area: DocumentSize::from_mm(300.0, 300.0),
                z_travel: Length::from_mm(60.0),
                // invariant: 12 000 is below 100 000
                spindle_max: Rpm::new(12_000).unwrap_or_else(|| unreachable!()),
                max_rate: [rate; 3],
                acceleration: [accel; 3],
                safe_height: Length::from_mm(5.0),
                connection: Connection::default(),
            },
        }
    }

    /// The size of a new page that takes this profile: its work area.
    #[must_use]
    pub fn page_size(&self) -> DocumentSize {
        self.profile().values.work_area
    }
}

/// The built-in Generic GRBL router (criterion 2).
pub const GENERIC_GRBL_ROUTER: Builtin = Builtin {
    id: "generic-grbl-router",
    name: "Generic GRBL router",
};

/// Every built-in, in list order.
pub const BUILTINS: &[Builtin] = &[GENERIC_GRBL_ROUTER];

/// The profile a new page takes (criterion 1).
pub const DEFAULT_BUILTIN: Builtin = GENERIC_GRBL_ROUTER;

/// The built-in of id `id`.
#[must_use]
pub fn builtin(id: &str) -> Option<CncProfile> {
    BUILTINS.iter().find(|b| b.id == id).map(Builtin::profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generic_grbl_router_has_the_values_of_criterion_2() {
        let router = builtin("generic-grbl-router").unwrap();
        let v = &router.values;
        assert_eq!(v.name, "Generic GRBL router");
        assert_eq!(v.output_format, OutputFormat::Grbl);
        assert_eq!(v.work_area, DocumentSize::from_mm(300.0, 300.0));
        assert_eq!(v.z_travel.as_mm(), 60.0);
        assert_eq!(v.spindle_max.get(), 12_000);
        assert!(v.max_rate.iter().all(|r| r.get() == 500.0));
        assert!(v.acceleration.iter().all(|a| a.get() == 10.0));
        assert_eq!(v.safe_height.as_mm(), 5.0);
        assert!(v.is_valid());
        assert_eq!(
            DEFAULT_BUILTIN.page_size(),
            DocumentSize::from_mm(300.0, 300.0)
        );
    }
}
