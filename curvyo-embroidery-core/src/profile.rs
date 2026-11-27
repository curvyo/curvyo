//! The embroidery machine profile body (`specs/0160-embroidery` criterion 2, `adrs.md` EM2): output
//! format, hoops, maximum stitch and speed.

use curvyo_document_core::DocumentSize;

use crate::hoop::{HoopSize, page_mm};
use crate::stitch::TenthMm;

/// The longest profile name.
pub const MAX_NAME_CHARS: usize = 60;
/// The DST limit of one record, 12.1 mm (criterion 15).
pub const DST_MAX_STITCH: TenthMm = TenthMm::new(121);
/// The shortest maximum stitch a profile may name, 1 mm.
pub const MIN_MAX_STITCH: TenthMm = TenthMm::new(10);

/// What Export job file writes for a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputFormat {
    /// Tajima DST (criterion 14).
    Dst,
}

impl OutputFormat {
    /// Every format.
    pub const ALL: [Self; 1] = [Self::Dst];

    /// The stored spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Dst => "dst",
        }
    }

    /// The format a stored spelling names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|format| format.key() == key)
    }

    /// The Output row's word ("DST").
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Dst => "DST",
        }
    }

    /// The save dialog's file type (UX 7: "Tajima DST").
    #[must_use]
    pub const fn file_type(self) -> &'static str {
        match self {
            Self::Dst => "Tajima DST",
        }
    }

    /// The file extension, without the dot.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Dst => "dst",
        }
    }
}

/// A machine speed in whole stitches per minute, 100 to 1 500 (UX 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StitchesPerMin(u16);

impl StitchesPerMin {
    /// The lowest speed.
    pub const MIN: u16 = 100;
    /// The highest speed.
    pub const MAX: u16 = 1_500;

    /// `n` when it is within 100 to 1 500.
    #[must_use]
    pub fn new(n: u64) -> Option<Self> {
        u16::try_from(n)
            .ok()
            .filter(|n| (Self::MIN..=Self::MAX).contains(n))
            .map(Self)
    }

    /// The speed.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// The values of an embroidery profile and of its copy on a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbroideryValues {
    /// The profile's name.
    pub name: String,
    /// What Export job file writes.
    pub output_format: OutputFormat,
    /// The hoops the profile offers, in pick order; never empty.
    pub hoops: Vec<HoopSize>,
    /// The longest stitch, at most the DST limit.
    pub max_stitch: TenthMm,
    /// The machine speed: the simulation's 1x and the time.
    pub speed: StitchesPerMin,
}

impl EmbroideryValues {
    /// Whether the values hold together: a name of 1 to 60 characters, at least one hoop and no
    /// hoop twice, a maximum stitch of 1 to 12.1 mm.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let chars = self.name.trim().chars().count();
        let distinct = self
            .hoops
            .iter()
            .enumerate()
            .all(|(i, h)| !self.hoops[..i].contains(h));
        (1..=MAX_NAME_CHARS).contains(&chars)
            && !self.hoops.is_empty()
            && distinct
            && (MIN_MAX_STITCH..=DST_MAX_STITCH).contains(&self.max_stitch)
    }

    /// The page size: the largest sewing field plus 20 mm on every side (criterion 1).
    #[must_use]
    pub fn page_size(&self) -> DocumentSize {
        let (w, h) = page_mm(&self.hoops);
        DocumentSize::from_mm(w, h)
    }

    /// The hoop a new page takes: the first.
    #[must_use]
    pub fn default_hoop(&self) -> HoopSize {
        self.hoops.first().copied().unwrap_or_default()
    }
}

/// An embroidery profile record: its id, when it was last changed and its values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbroideryProfile {
    /// The record id (a UUID, or a built-in's id).
    pub id: String,
    /// Milliseconds since the Unix epoch of the last change.
    pub modified: i64,
    /// The values.
    pub values: EmbroideryValues,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_speed_has_its_range() {
        assert!(StitchesPerMin::new(99).is_none());
        assert_eq!(StitchesPerMin::new(600).map(StitchesPerMin::get), Some(600));
        assert!(StitchesPerMin::new(1_501).is_none());
        assert_eq!(OutputFormat::parse("dst"), Some(OutputFormat::Dst));
    }
}
