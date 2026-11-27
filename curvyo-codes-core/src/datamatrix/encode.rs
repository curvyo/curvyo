//! The only file that names the `datamatrix` crate: a text to a [`ModuleGrid`] of the smallest
//! square ECC 200 symbol it fits, or the reason it cannot be encoded.

use datamatrix::{DataMatrix, SymbolList};

use crate::grid::ModuleGrid;
use crate::qr::EncodeError;

/// The most bytes the largest symbol (144 x 144) holds in byte mode, the limit a refusal names.
pub(crate) const CAPACITY_BYTES: usize = 1555;

/// The side of one region of a symbol of `width` modules: each region carries its own finder and
/// clock edge, so these are the function patterns.
const fn region(width: usize) -> usize {
    match width {
        0..=26 => width,
        27..=52 => width / 2,
        53..=104 => width / 4,
        _ => width / 6,
    }
}

/// Encodes `content` as the smallest square symbol, 10 x 10 to 144 x 144: the same on every platform.
///
/// # Errors
/// [`EncodeError::Empty`] for an empty text, [`EncodeError::TooLong`] when it does not fit 144 x 144.
pub(crate) fn encode(content: &str) -> Result<ModuleGrid, EncodeError> {
    if content.is_empty() {
        return Err(EncodeError::Empty);
    }
    let symbols = SymbolList::default().enforce_square();
    let code = DataMatrix::encode_str(content, symbols).map_err(|_| EncodeError::TooLong {
        bytes: content.len(),
        limit: CAPACITY_BYTES,
    })?;
    let bitmap = code.bitmap();
    let width = bitmap.width();
    let mut dark = vec![false; width * width];
    for (x, y) in bitmap.pixels() {
        dark[y * width + x] = true;
    }
    let side = region(width);
    let function = (0..width * width)
        .map(|at| {
            let (x, y) = (at % width, at / width);
            x % side == 0 || (x + 1) % side == 0 || y % side == 0 || (y + 1) % side == 0
        })
        .collect();
    Ok(ModuleGrid::new(width, dark, function, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_digits_are_ten_by_ten() {
        let grid = encode("123456").unwrap();
        assert_eq!(grid.width(), 10);
    }

    #[test]
    fn the_l_finder_is_solid_and_the_clock_alternates() {
        let grid = encode("123456").unwrap();
        for i in 0..10 {
            assert!(grid.is_dark(0, i), "left column {i}");
            assert!(grid.is_dark(i, 9), "bottom row {i}");
            assert_eq!(grid.is_dark(i, 0), i % 2 == 0, "top row {i}");
            assert_eq!(grid.is_dark(9, i), i % 2 == 1, "right column {i}");
        }
    }

    #[test]
    fn function_cells_are_the_border_and_the_alignment_patterns() {
        let small = encode("123456").unwrap();
        let border = (0..10 * 10).filter(|at| small.is_function(at % 10, at / 10));
        assert_eq!(border.count(), 36, "the ring of a 10 x 10 symbol");
        let big = encode(&"a".repeat(100)).unwrap();
        assert!(big.width() >= 32, "{}", big.width());
        let side = big.width() / 2;
        assert!(big.is_function(side - 1, 5) && big.is_function(side, 5));
        assert!(!big.is_function(5, 5));
    }

    #[test]
    fn the_largest_symbol_and_the_limit() {
        // The crate refuses an input over 1555 characters; incompressible ones need the whole 144 x 144.
        let high: String = (0..1555)
            .map(|i| char::from(0xa1 + u8::try_from(i * 7 % 90).unwrap()))
            .collect();
        assert_eq!(encode(&high).unwrap().width(), 144);
        assert!(encode(&"a".repeat(1555)).is_ok());
        assert_eq!(
            encode(&"a".repeat(1556)),
            Err(EncodeError::TooLong {
                bytes: 1556,
                limit: CAPACITY_BYTES
            })
        );
        assert_eq!(encode(""), Err(EncodeError::Empty));
    }
}
