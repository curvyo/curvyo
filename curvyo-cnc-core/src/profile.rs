//! The CNC machine profile body (`specs/0161-cnc-output` criterion 2, `adrs.md` CN1): output
//! format, work area, Z travel, spindle, rates and accelerations per axis, safe height and the
//! connection.

use curvyo_document_core::{DocumentSize, Length};
use curvyo_library_core::Connection;

use crate::units::{AccelMmS2, FeedMmMin, Rpm};

/// The longest profile name.
pub const MAX_NAME_CHARS: usize = 60;
/// The smallest and largest side of a work area in mm.
pub const WORK_AREA_MM: (f64, f64) = (10.0, 5_000.0);
/// The smallest and largest Z travel in mm.
pub const Z_TRAVEL_MM: (f64, f64) = (1.0, 1_000.0);
/// The smallest safe height in mm.
pub const MIN_SAFE_HEIGHT_MM: f64 = 0.5;
/// GRBL's default junction deviation, `$11` (criterion 18).
pub const JUNCTION_DEVIATION_MM: f64 = 0.01;

/// What Export job file writes for a profile: a closed list, GRBL only in slice 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputFormat {
    /// GRBL 1.1 G-code (criterion 12).
    Grbl,
}

impl OutputFormat {
    /// Every format.
    pub const ALL: [Self; 1] = [Self::Grbl];

    /// The stored spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Grbl => "grbl",
        }
    }

    /// The format a stored spelling names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|format| format.key() == key)
    }

    /// The Output row's word.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Grbl => "GRBL",
        }
    }

    /// The save dialog's file type (UX 8).
    #[must_use]
    pub const fn file_type(self) -> &'static str {
        match self {
            Self::Grbl => "GRBL G-code",
        }
    }

    /// The file extension, without the dot (criterion 12).
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Grbl => "nc",
        }
    }
}

/// The values of a CNC profile and of its copy on a page.
#[derive(Debug, Clone, PartialEq)]
pub struct CncValues {
    /// The profile's name.
    pub name: String,
    /// What Export job file writes.
    pub output_format: OutputFormat,
    /// The work area: the page size.
    pub work_area: DocumentSize,
    /// How far Z can travel.
    pub z_travel: Length,
    /// The highest spindle speed (`$30`).
    pub spindle_max: Rpm,
    /// The maximum rate of X, Y and Z (`$110` to `$112`): G0 runs at it, feeds are clamped to it.
    pub max_rate: [FeedMmMin; 3],
    /// The acceleration of X, Y and Z (`$120` to `$122`).
    pub acceleration: [AccelMmS2; 3],
    /// Rapids run this far above the stock top.
    pub safe_height: Length,
    /// How the profile reaches its machine (stored on the record only).
    pub connection: Connection,
}

impl CncValues {
    /// Whether the values hold together: a name of 1 to 60 characters, a work area of 10 to
    /// 5 000 mm a side, a Z travel of 1 to 1 000 mm, a safe height from 0.5 mm to the Z travel
    /// and a valid connection.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let chars = self.name.trim().chars().count();
        let side = |l: Length| (WORK_AREA_MM.0..=WORK_AREA_MM.1).contains(&l.as_mm());
        let z = self.z_travel.as_mm();
        (1..=MAX_NAME_CHARS).contains(&chars)
            && side(self.work_area.width)
            && side(self.work_area.height)
            && (Z_TRAVEL_MM.0..=Z_TRAVEL_MM.1).contains(&z)
            && (MIN_SAFE_HEIGHT_MM..=z).contains(&self.safe_height.as_mm())
            && self.connection.is_valid()
    }
}

/// A CNC profile record: its id, when it was last changed and its values.
#[derive(Debug, Clone, PartialEq)]
pub struct CncProfile {
    /// The record id (a UUID, or a built-in's id).
    pub id: String,
    /// Milliseconds since the Unix epoch of the last change.
    pub modified: i64,
    /// The values.
    pub values: CncValues,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_output_format_is_grbl_written_as_nc() {
        assert_eq!(OutputFormat::parse("grbl"), Some(OutputFormat::Grbl));
        assert_eq!(OutputFormat::Grbl.extension(), "nc");
        assert!(OutputFormat::parse("mach3").is_none());
    }
}
