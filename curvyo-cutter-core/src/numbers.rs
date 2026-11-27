//! The cutter's number newtypes with their ranges (`specs/0159-cutting-plotter` criterion 8,
//! `adrs.md` PL1): pressure, speed and passes.

use serde::{Deserialize, Serialize};

/// A blade or pen pressure in whole percent, 1 to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct PressurePct(u8);

impl PressurePct {
    /// The lowest pressure.
    pub const MIN: u8 = 1;
    /// The highest pressure.
    pub const MAX: u8 = 100;

    /// `pct` when it is within 1 to 100.
    #[must_use]
    pub fn new(pct: u64) -> Option<Self> {
        u8::try_from(pct)
            .ok()
            .filter(|p| (Self::MIN..=Self::MAX).contains(p))
            .map(Self)
    }

    /// The pressure in percent.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u64> for PressurePct {
    type Error = &'static str;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or("pressure out of range")
    }
}

impl From<PressurePct> for u64 {
    fn from(value: PressurePct) -> Self {
        Self::from(value.0)
    }
}

/// A speed in whole mm/s, 1 to [`SpeedMmS::LIMIT`]; a profile's Max speed limits it further.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct SpeedMmS(u16);

impl SpeedMmS {
    /// The highest speed any profile may name.
    pub const LIMIT: u16 = 2_000;

    /// `mm_s` when it is within 1 to [`Self::LIMIT`].
    #[must_use]
    pub fn new(mm_s: u64) -> Option<Self> {
        u16::try_from(mm_s)
            .ok()
            .filter(|s| (1..=Self::LIMIT).contains(s))
            .map(Self)
    }

    /// The speed in mm/s.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl TryFrom<u64> for SpeedMmS {
    type Error = &'static str;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or("speed out of range")
    }
}

impl From<SpeedMmS> for u64 {
    fn from(value: SpeedMmS) -> Self {
        Self::from(value.0)
    }
}

/// How often a Cut runs each path, 1 to 10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct Passes(u8);

impl Passes {
    /// One pass.
    pub const ONE: Self = Self(1);
    /// The most passes.
    pub const MAX: u8 = 10;

    /// `n` when it is within 1 to 10.
    #[must_use]
    pub fn new(n: u64) -> Option<Self> {
        u8::try_from(n)
            .ok()
            .filter(|n| (1..=Self::MAX).contains(n))
            .map(Self)
    }

    /// The count.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u64> for Passes {
    type Error = &'static str;
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or("passes out of range")
    }
}

impl From<Passes> for u64 {
    fn from(value: Passes) -> Self {
        Self::from(value.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_number_has_its_range() {
        assert!(PressurePct::new(0).is_none());
        assert_eq!(PressurePct::new(100).unwrap().get(), 100);
        assert!(PressurePct::new(101).is_none());
        assert!(SpeedMmS::new(0).is_none());
        assert_eq!(SpeedMmS::new(2_000).unwrap().get(), 2_000);
        assert!(SpeedMmS::new(2_001).is_none());
        assert!(Passes::new(0).is_none());
        assert_eq!(Passes::new(10).unwrap().get(), 10);
        assert!(
            Passes::new(11).is_none(),
            "criterion 8: Passes 11 is refused"
        );
        assert!(serde_json::from_str::<Passes>("11").is_err());
        assert_eq!(serde_json::to_string(&Passes::ONE).unwrap(), "1");
    }
}
