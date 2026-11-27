//! EAN-13, EAN-8 and UPC-A (GS1 General Specifications; ISO/IEC 15420): the digit patterns, the
//! check digit and the layout of the human-readable digits. UPC-A is EAN-13 with a leading 0.

use super::{Bar, Cell, LinearCode, LinearError, Printed, Symbology, bars_of};

/// The L (odd parity) patterns of the digits 0 to 9, a 1 is a bar.
const L: [&str; 10] = [
    "0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011",
    "0110111", "0001011",
];
/// The G (even parity) patterns.
const G: [&str; 10] = [
    "0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001",
    "0001001", "0010111",
];
/// The R patterns (the complement of L).
const R: [&str; 10] = [
    "1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100",
    "1001000", "1110100",
];
/// The L or G choice of the six left digits, by the first digit of an EAN-13.
const PARITY: [&str; 10] = [
    "LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG", "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL",
    "LGGLGL",
];
const GUARD: &str = "101";
const CENTRE: &str = "01010";

/// The check digit of `digits` (without it): weights 3 and 1 from the right.
pub(super) fn check_digit(digits: &[u8]) -> u8 {
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(index, digit)| u32::from(*digit) * if index % 2 == 0 { 3 } else { 1 })
        .sum();
    // invariant: the remainder is below 10
    u8::try_from((10 - sum % 10) % 10).unwrap_or(0)
}

/// The digits of `content` with spaces and hyphens dropped.
fn digits_of(content: &str, short: usize, long: usize) -> Result<Vec<u8>, LinearError> {
    let text: String = content
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    let lengths = || LinearError::WrongLength {
        short: u32::try_from(short).unwrap_or(0),
        long: u32::try_from(long).unwrap_or(0),
    };
    let count = text.chars().count();
    if count != short && count != long {
        return Err(lengths());
    }
    text.chars()
        .map(|c| {
            c.to_digit(10)
                .and_then(|d| u8::try_from(d).ok())
                .ok_or(LinearError::DigitsOnly)
        })
        .collect()
}

fn pattern_of(bits: &str, out: &mut String) {
    out.push_str(bits);
}

/// The bars of the EAN-13 digits `ean` (or the eight of an EAN-8) and their width in modules; the
/// guard patterns are guard bars, and in a UPC-A so are its first and last characters (GS1: the
/// number system and the check digit stand outside the digits, their bars reach into the band).
fn bars_and_width(symbology: Symbology, ean: &[u8], eight: bool) -> (Vec<Bar>, f64) {
    let mut bits = String::new();
    let mut guards: Vec<(usize, usize)> = Vec::new();
    let mut mark = |bits: &str, out: &mut String| {
        guards.push((out.len(), out.len() + bits.len()));
        pattern_of(bits, out);
    };
    mark(GUARD, &mut bits);
    let (left, right): (&[u8], &[u8]) = if eight {
        (&ean[..4], &ean[4..])
    } else {
        (&ean[1..7], &ean[7..])
    };
    let parity = if eight {
        "LLLL"
    } else {
        PARITY[usize::from(ean[0])]
    };
    for (digit, side) in left.iter().zip(parity.chars()) {
        let table = if side == 'L' { &L } else { &G };
        pattern_of(table[usize::from(*digit)], &mut bits);
    }
    mark(CENTRE, &mut bits);
    for digit in right {
        pattern_of(R[usize::from(*digit)], &mut bits);
    }
    mark(GUARD, &mut bits);
    if symbology == Symbology::UpcA {
        guards.push((3, 10));
        guards.push((85, 92));
    }
    let (bars, width) = bars_of(&runs_of(&bits), |_| false);
    let bars = bars
        .into_iter()
        .map(|mut bar| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let at = bar.x as usize;
            bar.guard = guards.iter().any(|(from, to)| at >= *from && at < *to);
            bar
        })
        .collect();
    (bars, width)
}

/// Encodes `content` as `symbology` (an EAN or UPC one).
pub(super) fn encode(symbology: Symbology, content: &str) -> Result<LinearCode, LinearError> {
    let (data_len, eight) = match symbology {
        Symbology::Ean13 => (12, false),
        Symbology::Ean8 => (7, true),
        _ => (11, false),
    };
    let mut digits = digits_of(content, data_len, data_len + 1)?;
    let mut corrected = None;
    let wanted = if symbology == Symbology::UpcA {
        // UPC-A is the EAN-13 of "0" + its digits: the same weights.
        let mut full = vec![0];
        full.extend_from_slice(&digits[..data_len]);
        check_digit(&full)
    } else {
        check_digit(&digits[..data_len])
    };
    if digits.len() == data_len {
        digits.push(wanted);
        corrected = Some(false);
    } else if digits[data_len] != wanted {
        digits[data_len] = wanted;
        corrected = Some(true);
    }
    let normalised: String = digits.iter().map(|d| char::from(b'0' + d)).collect();
    // The EAN-13 digits: UPC-A gets its leading 0.
    let ean: Vec<u8> = if symbology == Symbology::UpcA {
        std::iter::once(0).chain(digits.iter().copied()).collect()
    } else {
        digits.clone()
    };
    let (bars, width) = bars_and_width(symbology, &ean, eight);
    let (fixed, printed) = layout(symbology, &digits);
    Ok(LinearCode {
        bars,
        width,
        fixed_quiet: Some(fixed),
        printed,
        normalised,
        corrected,
    })
}

/// The run widths of a bit string, first a bar.
fn runs_of(bits: &str) -> Vec<f64> {
    let mut runs: Vec<f64> = Vec::new();
    let mut previous = '1';
    for bit in bits.chars() {
        if bit == previous && !runs.is_empty() {
            if let Some(last) = runs.last_mut() {
                *last += 1.0;
            }
        } else {
            runs.push(1.0);
        }
        previous = bit;
    }
    runs
}

/// The fixed quiet zones and where each printed digit stands (GS1 layout, UX notes §11.9).
fn layout(symbology: Symbology, digits: &[u8]) -> ((f64, f64), Printed) {
    let digit = |index: usize| char::from(b'0' + digits[index]);
    let cell = |index: usize, x: f64, outside: bool| Cell {
        ch: digit(index),
        x,
        outside,
    };
    let mut cells = Vec::new();
    match symbology {
        Symbology::Ean13 => {
            cells.push(cell(0, -4.5, true));
            for i in 0..6 {
                cells.push(cell(1 + i, 3.0 + 7.0 * as_f64(i) + 3.5, false));
                cells.push(cell(7 + i, 50.0 + 7.0 * as_f64(i) + 3.5, false));
            }
            ((11.0, 7.0), Printed::Cells(cells))
        }
        Symbology::Ean8 => {
            for i in 0..4 {
                cells.push(cell(i, 3.0 + 7.0 * as_f64(i) + 3.5, false));
                cells.push(cell(4 + i, 36.0 + 7.0 * as_f64(i) + 3.5, false));
            }
            ((7.0, 7.0), Printed::Cells(cells))
        }
        _ => {
            // UPC-A: the number system digit and the check digit stand in the quiet zones.
            cells.push(cell(0, -4.5, true));
            for i in 1..6 {
                cells.push(cell(i, 3.0 + 7.0 * as_f64(i) + 3.5, false));
            }
            for i in 0..5 {
                cells.push(cell(6 + i, 50.0 + 7.0 * as_f64(i) + 3.5, false));
            }
            cells.push(cell(11, 95.0 + 4.5, true));
            ((9.0, 9.0), Printed::Cells(cells))
        }
    }
}

#[allow(clippy::cast_precision_loss)]
const fn as_f64(n: usize) -> f64 {
    n as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_check_digit_of_known_numbers() {
        assert_eq!(check_digit(&[4, 0, 0, 6, 3, 8, 1, 3, 3, 3, 9, 3]), 1);
        assert_eq!(check_digit(&[9, 6, 3, 8, 5, 0, 7]), 4);
        // The placeholders of UX notes §11.3.
        assert_eq!(
            encode(Symbology::Ean13, "200123456789").unwrap().normalised,
            "2001234567893"
        );
        assert_eq!(
            encode(Symbology::Ean8, "2012345").unwrap().normalised,
            "20123451"
        );
        assert_eq!(
            encode(Symbology::UpcA, "21234567890").unwrap().normalised,
            "212345678909"
        );
        assert_eq!(
            encode(Symbology::UpcA, "03600029145").unwrap().normalised,
            "036000291452"
        );
    }

    #[test]
    fn a_wrong_check_digit_is_replaced_and_said() {
        let code = encode(Symbology::Ean13, "4006381333930").unwrap();
        assert_eq!(code.normalised, "4006381333931");
        assert_eq!(code.corrected, Some(true));
        assert_eq!(
            encode(Symbology::Ean13, "4 006381-333931")
                .unwrap()
                .corrected,
            None
        );
    }

    #[test]
    fn the_widths_are_95_and_67_modules() {
        assert!((encode(Symbology::Ean13, "4006381333931").unwrap().width - 95.0).abs() < 1e-9);
        assert!((encode(Symbology::UpcA, "036000291452").unwrap().width - 95.0).abs() < 1e-9);
        assert!((encode(Symbology::Ean8, "96385074").unwrap().width - 67.0).abs() < 1e-9);
    }

    #[test]
    fn the_three_guards_are_six_guard_bars() {
        let code = encode(Symbology::Ean13, "4006381333931").unwrap();
        assert_eq!(code.bars.iter().filter(|b| b.guard).count(), 6);
    }

    #[test]
    fn upc_a_also_lengthens_its_first_and_last_character() {
        // 036000291452: the number system 0 (L 0001101) and the check digit 2 (R 1101100) have two
        // bars each besides the six of the three guards; an EAN-13 has only the guards.
        let upc = encode(Symbology::UpcA, "036000291452").unwrap();
        let long: Vec<f64> = upc.bars.iter().filter(|b| b.guard).map(|b| b.x).collect();
        assert_eq!(long.len(), 10, "{long:?}");
        assert!(long.iter().any(|x| (3.0..10.0).contains(x)));
        assert!(long.iter().any(|x| (85.0..92.0).contains(x)));
        let ean = encode(Symbology::Ean13, "4006381333931").unwrap();
        assert_eq!(ean.bars.iter().filter(|b| b.guard).count(), 6);
    }

    #[test]
    fn wrong_content_is_named() {
        assert_eq!(
            encode(Symbology::Ean13, "123"),
            Err(LinearError::WrongLength {
                short: 12,
                long: 13
            })
        );
        assert_eq!(
            encode(Symbology::Ean8, "1234567x"),
            Err(LinearError::DigitsOnly)
        );
    }
}
