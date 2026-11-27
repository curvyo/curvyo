//! The only file that names the `qrcode` crate: a text and an error-correction level to a
//! [`ModuleGrid`], or the reason it cannot be encoded.

use qrcode::{Color, EcLevel as Level, QrCode};

use crate::grid::ModuleGrid;

/// The four error-correction levels of a QR code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EcLevel {
    /// About 7 % of the codewords.
    L,
    /// About 15 %.
    M,
    /// About 25 %.
    Q,
    /// About 30 %.
    H,
}

impl EcLevel {
    /// Every level, lowest first.
    pub const ALL: [Self; 4] = [Self::L, Self::M, Self::Q, Self::H];

    /// The stored option id (`"m"`).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::L => "l",
            Self::M => "m",
            Self::Q => "q",
            Self::H => "h",
        }
    }

    /// The level a stored option id names.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.id() == id)
    }

    /// The letter ("M").
    #[must_use]
    pub const fn letter(self) -> char {
        match self {
            Self::L => 'L',
            Self::M => 'M',
            Self::Q => 'Q',
            Self::H => 'H',
        }
    }

    const fn qr(self) -> Level {
        match self {
            Self::L => Level::L,
            Self::M => Level::M,
            Self::Q => Level::Q,
            Self::H => Level::H,
        }
    }

    /// The most bytes version 40 holds at this level (byte mode).
    #[must_use]
    pub const fn capacity_bytes(self) -> usize {
        match self {
            Self::L => 2953,
            Self::M => 2331,
            Self::Q => 1663,
            Self::H => 1273,
        }
    }
}

/// Why a text could not be encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    /// The text is empty.
    #[error("the content is empty")]
    Empty,
    /// The text does not fit version 40 at this level.
    #[error("{bytes} bytes do not fit, at most {limit}")]
    TooLong {
        /// The size of the text in bytes.
        bytes: usize,
        /// The most bytes version 40 holds at this level.
        limit: usize,
    },
}

/// Encodes `content` at `level`: the module grid, the same on every platform.
///
/// # Errors
/// [`EncodeError::Empty`] for an empty text, [`EncodeError::TooLong`] when it does not fit
/// version 40 at `level`.
pub fn encode(content: &str, level: EcLevel) -> Result<ModuleGrid, EncodeError> {
    if content.is_empty() {
        return Err(EncodeError::Empty);
    }
    let code =
        QrCode::with_error_correction_level(content.as_bytes(), level.qr()).map_err(|_| {
            EncodeError::TooLong {
                bytes: content.len(),
                limit: level.capacity_bytes(),
            }
        })?;
    let width = code.width();
    let dark: Vec<bool> = code
        .to_colors()
        .into_iter()
        .map(|color| color == Color::Dark)
        .collect();
    let function = (0..width * width)
        .map(|at| code.is_functional(at % width, at / width))
        .collect();
    Ok(ModuleGrid::new(
        width,
        dark,
        function,
        code.max_allowed_errors(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_at_m_is_version_one() {
        let grid = encode("Hello", EcLevel::M).unwrap();
        assert_eq!((grid.width(), grid.version()), (21, 1));
        assert!(grid.is_dark(0, 0), "the finder corner");
        assert!(grid.is_function(0, 0));
        assert!(grid.in_finder(6, 6) && !grid.in_finder(7, 7));
    }

    #[test]
    fn the_capacity_of_version_40_is_the_stated_limit_at_every_level() {
        for level in EcLevel::ALL {
            let fits = "a".repeat(level.capacity_bytes());
            assert_eq!(encode(&fits, level).unwrap().version(), 40, "{level:?}");
            let over = "a".repeat(level.capacity_bytes() + 1);
            assert_eq!(
                encode(&over, level),
                Err(EncodeError::TooLong {
                    bytes: level.capacity_bytes() + 1,
                    limit: level.capacity_bytes()
                })
            );
        }
        assert_eq!(
            encode("", EcLevel::M),
            Err(EncodeError::Empty),
            "empty is its own reason"
        );
    }

    #[test]
    fn the_same_text_gives_the_same_grid() {
        assert_eq!(
            encode("https://example.com", EcLevel::M),
            encode("https://example.com", EcLevel::M)
        );
    }
}
