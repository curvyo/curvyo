//! The time the caller passes in.

/// Milliseconds since the Unix epoch, as the caller's clock reads them. The crate never reads a
/// clock; every input that depends on time carries one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Millis(pub u64);

impl Millis {
    /// The time `ms` milliseconds later, saturating.
    #[must_use]
    pub fn plus(self, ms: u64) -> Self {
        Self(self.0.saturating_add(ms))
    }

    /// The milliseconds from `earlier` to `self`, zero if `earlier` is later.
    #[must_use]
    pub fn since(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }

    /// Whole seconds since the Unix epoch: the unit of an invite's expiry (ADR 0010 §3).
    #[must_use]
    pub fn unix_seconds(self) -> u64 {
        self.0 / 1000
    }
}
