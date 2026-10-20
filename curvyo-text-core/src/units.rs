//! The length type of the engine's input and output.

/// A length in millimetres (`CLAUDE.md` §5: units are types).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Mm(pub f64);

impl Mm {
    /// The length in millimetres.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}
