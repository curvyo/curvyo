//! The selection method of a trace and the colours it takes.

/// A colour as its stored sRGB bytes (no colour management, `0058` D10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb8 {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
}

impl Rgb8 {
    /// The colour `#RRGGBB` of `r`, `g`, `b`.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// The most picked colours Pick colours takes (**Proposal**, criterion 17).
pub const MAX_SWATCHES: usize = 8;

/// The picked colours of Pick colours: 0 to [`MAX_SWATCHES`], kept in a fixed array so the trace
/// values stay `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Swatches {
    colours: [Rgb8; MAX_SWATCHES],
    len: u8,
}

impl Swatches {
    /// The first [`MAX_SWATCHES`] of `colours`.
    #[must_use]
    pub fn from_slice(colours: &[Rgb8]) -> Self {
        let mut out = Self::default();
        for (slot, &colour) in out.colours.iter_mut().zip(colours) {
            *slot = colour;
            out.len += 1;
        }
        out
    }

    /// The picked colours in order.
    #[must_use]
    pub fn as_slice(&self) -> &[Rgb8] {
        &self.colours[..usize::from(self.len)]
    }
}

/// How pixels become content (`specs/0089` Terms, criterion 14, `adrs.md` §2, §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Selection {
    /// Pixels darker than the Threshold (`TraceParams::threshold`).
    #[default]
    Brightness,
    /// Pixels farther than the colour tolerance from the background colour; `None` is a
    /// transparent background, where every opaque pixel is content.
    RemoveBackground {
        /// The background colour.
        background: Option<Rgb8>,
        /// The colour tolerance, 0 to 100.
        tolerance: u8,
    },
    /// Pixels made of the picked colours and their blends toward the background.
    PickColours {
        /// The picked colours.
        swatches: Swatches,
        /// The background colour the blends run toward, resolved once on the full working
        /// image by the host so the reduced preview and the full-size trace agree; `None` is a
        /// transparent background.
        background: Option<Rgb8>,
        /// The colour tolerance, 0 to 100.
        tolerance: u8,
    },
}

impl Selection {
    /// The methods in menu order (the schema's `method` choice).
    pub const LABELS: &'static [&'static str] =
        &["Brightness", "Remove background", "Pick colours"];

    /// The method's schema index.
    #[must_use]
    pub const fn index(&self) -> u8 {
        match self {
            Self::Brightness => 0,
            Self::RemoveBackground { .. } => 1,
            Self::PickColours { .. } => 2,
        }
    }
}
