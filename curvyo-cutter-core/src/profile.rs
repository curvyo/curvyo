//! The cutter machine profile body (`specs/0159-cutting-plotter` criterion 2, `adrs.md` PL1):
//! work area, origin corner, output format, force range, default numbers, mats and connection.

use curvyo_document_core::DocumentSize;
use curvyo_library_core::Connection;

use crate::mat::MatSize;
use crate::numbers::{PressurePct, SpeedMmS};

/// The longest profile name.
pub const MAX_NAME_CHARS: usize = 60;

/// What Export job file writes for a profile (criterion 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutputFormat {
    /// An SVG at mat scale for Cricut Design Space (criterion 11).
    DesignSpaceSvg,
    /// HPGL for an open plotter (criterion 13).
    Hpgl,
}

impl OutputFormat {
    /// Every format.
    pub const ALL: [Self; 2] = [Self::DesignSpaceSvg, Self::Hpgl];

    /// The stored spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::DesignSpaceSvg => "design_space_svg",
            Self::Hpgl => "hpgl",
        }
    }

    /// The format a stored spelling names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|format| format.key() == key)
    }

    /// The name the Output row and the save dialog show.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DesignSpaceSvg => "Design Space SVG",
            Self::Hpgl => "HPGL",
        }
    }

    /// The file extension, without the dot.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::DesignSpaceSvg => "svg",
            Self::Hpgl => "plt",
        }
    }

    /// The file's media type.
    #[must_use]
    pub const fn media_type(self) -> &'static str {
        match self {
            Self::DesignSpaceSvg => "image/svg+xml",
            Self::Hpgl => "application/vnd.hp-hpgl",
        }
    }

    /// Whether the machine sets pressure and speed itself, so the numbers are hidden (UX C13).
    #[must_use]
    pub const fn sets_numbers(self) -> bool {
        matches!(self, Self::DesignSpaceSvg)
    }
}

/// The corner of the work area the machine counts from, for a matless profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OriginCorner {
    /// Top left, Y down.
    TopLeft,
    /// Bottom left, Y up (the HPGL default).
    BottomLeft,
}

impl OriginCorner {
    /// Every corner.
    pub const ALL: [Self; 2] = [Self::TopLeft, Self::BottomLeft];

    /// The stored spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::BottomLeft => "bottom_left",
        }
    }

    /// The corner a stored spelling names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|corner| corner.key() == key)
    }
}

/// The plotter's force range that Pressure maps onto (criterion 13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ForceRange {
    min: u16,
    max: u16,
}

impl ForceRange {
    /// The highest force any profile may name.
    pub const LIMIT: u16 = 1_000;

    /// `min` to `max` when `1 <= min <= max <= LIMIT`.
    #[must_use]
    pub fn new(min: u64, max: u64) -> Option<Self> {
        let min = u16::try_from(min).ok()?;
        let max = u16::try_from(max).ok()?;
        (1 <= min && min <= max && max <= Self::LIMIT).then_some(Self { min, max })
    }

    /// The lowest force.
    #[must_use]
    pub const fn min(self) -> u16 {
        self.min
    }

    /// The highest force.
    #[must_use]
    pub const fn max(self) -> u16 {
        self.max
    }

    /// `pressure` mapped linearly from 1 to 100 % onto the range, rounded half up to a whole
    /// force (criterion 13; Proposal: 1 % is the lowest force, 100 % the highest).
    #[must_use]
    pub fn force_of(self, pressure: PressurePct) -> u16 {
        let span = u32::from(self.max - self.min);
        let step = u32::from(pressure.get() - 1);
        let scaled = (span * step * 2 + 99) / (99 * 2);
        // invariant: scaled <= span <= u16::MAX
        self.min + u16::try_from(scaled).unwrap_or(u16::MAX - self.min)
    }
}

/// The values of a cutter profile and of its copy on a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutterValues {
    /// The profile's name.
    pub name: String,
    /// The work area of a matless profile (its page); a mat profile's page is its mat.
    pub work_area: WorkArea,
    /// The origin corner of a matless profile.
    pub origin: OriginCorner,
    /// What Export job file writes.
    pub output_format: OutputFormat,
    /// The force range Pressure maps onto.
    pub force: ForceRange,
    /// The highest speed.
    pub max_speed: SpeedMmS,
    /// A new operation's Pressure.
    pub default_pressure: PressurePct,
    /// A new operation's Speed, at most `max_speed`.
    pub default_speed: SpeedMmS,
    /// The mats the profile offers, in pick order; empty for a matless profile.
    pub mats: Vec<MatSize>,
    /// How the profile reaches its machine (sending is slice 6).
    pub connection: Connection,
}

impl CutterValues {
    /// Whether the values hold together: a name of 1 to 60 characters, the default speed within
    /// the max speed, the mats distinct and the connection valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let chars = self.name.trim().chars().count();
        let distinct = self
            .mats
            .iter()
            .enumerate()
            .all(|(i, m)| !self.mats[..i].contains(m));
        (1..=MAX_NAME_CHARS).contains(&chars)
            && self.default_speed <= self.max_speed
            && distinct
            && self.connection.is_valid()
    }

    /// The page size of a new page that takes these values: the first mat's field, else the work
    /// area (PL11).
    #[must_use]
    pub fn page_size(&self) -> DocumentSize {
        self.mats
            .first()
            .map_or_else(|| self.work_area.size(), |mat| mat.page_size())
    }
}

/// A work area in whole tenths of a mm, 10 to 10 000 mm a side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkArea {
    width_tenths: u32,
    height_tenths: u32,
}

impl WorkArea {
    /// The work area of `width` x `height` mm, rounded to 0.1 mm, `None` outside 10 to 10 000 mm.
    #[must_use]
    pub fn from_mm(width: f64, height: f64) -> Option<Self> {
        let tenths = |mm: f64| {
            (10.0..=10_000.0).contains(&mm).then(|| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                // invariant: within 100 to 100 000 tenths
                let t = (mm * 10.0).round() as u32;
                t
            })
        };
        Some(Self {
            width_tenths: tenths(width)?,
            height_tenths: tenths(height)?,
        })
    }

    /// The width in mm.
    #[must_use]
    pub fn width_mm(self) -> f64 {
        f64::from(self.width_tenths) / 10.0
    }

    /// The height in mm.
    #[must_use]
    pub fn height_mm(self) -> f64 {
        f64::from(self.height_tenths) / 10.0
    }

    /// The size.
    #[must_use]
    pub fn size(self) -> DocumentSize {
        DocumentSize::from_mm(self.width_mm(), self.height_mm())
    }
}

/// A cutter profile record: its id, when it was last changed and its values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutterProfile {
    /// The record id (a UUID, or a built-in's id).
    pub id: String,
    /// Milliseconds since the Unix epoch of the last change.
    pub modified: i64,
    /// The values.
    pub values: CutterValues,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_maps_linearly_onto_the_force_range() {
        let range = ForceRange::new(1, 80).unwrap();
        let force = |p| range.force_of(PressurePct::new(p).unwrap());
        assert_eq!(force(1), 1);
        assert_eq!(force(100), 80);
        assert_eq!(force(40), 32, "1 + 39 * 79 / 99 = 32.12");
        assert_eq!(force(50), 40, "1 + 49 * 79 / 99 = 40.10");
        assert!(ForceRange::new(0, 10).is_none());
        assert!(ForceRange::new(10, 9).is_none());
    }

    #[test]
    fn the_output_formats_have_their_spellings_and_files() {
        assert_eq!(OutputFormat::parse("hpgl"), Some(OutputFormat::Hpgl));
        assert_eq!(OutputFormat::DesignSpaceSvg.extension(), "svg");
        assert_eq!(OutputFormat::Hpgl.label(), "HPGL");
        assert!(OutputFormat::DesignSpaceSvg.sets_numbers());
    }
}
