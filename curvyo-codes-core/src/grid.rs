//! The module grid of a 2D code: which cells are dark and which belong to a function pattern.

/// A square grid of modules, row by row, as the encoder made it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleGrid {
    width: usize,
    dark: Vec<bool>,
    function: Vec<bool>,
    correctable: usize,
}

impl ModuleGrid {
    /// A grid of `width` x `width` cells (made by the encoder only); `dark` and `function` hold `width * width` flags each and
    /// `correctable` is the number of codewords the code can lose.
    ///
    /// # Panics
    /// When a flag list does not hold `width * width` entries (an encoder bug, never input).
    #[must_use]
    pub(crate) fn new(
        width: usize,
        dark: Vec<bool>,
        function: Vec<bool>,
        correctable: usize,
    ) -> Self {
        assert_eq!(dark.len(), width * width, "dark flags");
        assert_eq!(function.len(), width * width, "function flags");
        Self {
            width,
            dark,
            function,
            correctable,
        }
    }

    /// The number of modules along one side (21 for version 1).
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// The QR version, 1 to 40.
    #[must_use]
    pub const fn version(&self) -> usize {
        (self.width - 17) / 4
    }

    /// Whether the module at column `x`, row `y` is dark.
    #[must_use]
    pub fn is_dark(&self, x: usize, y: usize) -> bool {
        self.dark[y * self.width + x]
    }

    /// Whether the module belongs to a finder, separator, timing, alignment, format or version
    /// pattern.
    #[must_use]
    pub fn is_function(&self, x: usize, y: usize) -> bool {
        self.function[y * self.width + x]
    }

    /// The codewords the code can lose before it no longer reads (the clear-area budget).
    #[must_use]
    pub const fn correctable(&self) -> usize {
        self.correctable
    }

    /// Whether the module is one of the three 7 x 7 finder patterns, which the finder style draws.
    #[must_use]
    pub fn in_finder(&self, x: usize, y: usize) -> bool {
        let far = self.width - 7;
        let (near_x, near_y) = (x < 7, y < 7);
        match (near_x, near_y) {
            (true, true) => true,
            (true, false) => y >= far,
            (false, true) => x >= far,
            (false, false) => false,
        }
    }
}
