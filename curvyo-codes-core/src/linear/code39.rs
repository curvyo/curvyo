//! Code 39 (ISO/IEC 16388): `*` start and stop, no check character, wide:narrow 2.0 to 3.0.

use super::{LinearCode, LinearError, MAX_CHARACTERS, Printed, bars_of, flatten_lines};

/// Each character as nine elements (bar, space, bar, ...), `w` wide and `n` narrow.
const TABLE: [(char, &str); 44] = [
    ('0', "nnnwwnwnn"),
    ('1', "wnnwnnnnw"),
    ('2', "nnwwnnnnw"),
    ('3', "wnwwnnnnn"),
    ('4', "nnnwwnnnw"),
    ('5', "wnnwwnnnn"),
    ('6', "nnwwwnnnn"),
    ('7', "nnnwnnwnw"),
    ('8', "wnnwnnwnn"),
    ('9', "nnwwnnwnn"),
    ('A', "wnnnnwnnw"),
    ('B', "nnwnnwnnw"),
    ('C', "wnwnnwnnn"),
    ('D', "nnnnwwnnw"),
    ('E', "wnnnwwnnn"),
    ('F', "nnwnwwnnn"),
    ('G', "nnnnnwwnw"),
    ('H', "wnnnnwwnn"),
    ('I', "nnwnnwwnn"),
    ('J', "nnnnwwwnn"),
    ('K', "wnnnnnnww"),
    ('L', "nnwnnnnww"),
    ('M', "wnwnnnnwn"),
    ('N', "nnnnwnnww"),
    ('O', "wnnnwnnwn"),
    ('P', "nnwnwnnwn"),
    ('Q', "nnnnnnwww"),
    ('R', "wnnnnnwwn"),
    ('S', "nnwnnnwwn"),
    ('T', "nnnnwnwwn"),
    ('U', "wwnnnnnnw"),
    ('V', "nwwnnnnnw"),
    ('W', "wwwnnnnnn"),
    ('X', "nwnnwnnnw"),
    ('Y', "wwnnwnnnn"),
    ('Z', "nwwnwnnnn"),
    ('-', "nwnnnnwnw"),
    ('.', "wwnnnnwnn"),
    (' ', "nwwnnnwnn"),
    ('*', "nwnnwnwnn"),
    ('$', "nwnwnwnnn"),
    ('/', "nwnwnnnwn"),
    ('+', "nwnnnwnwn"),
    ('%', "nnnwnwnwn"),
];

fn pattern(ch: char) -> Option<&'static str> {
    TABLE.iter().find(|(c, _)| *c == ch).map(|(_, p)| *p)
}

/// Encodes `content` (upper-cased; A to Z, 0 to 9, space and - . $ / + %).
pub(super) fn encode(content: &str, wide_narrow: f64) -> Result<LinearCode, LinearError> {
    let text = flatten_lines(content).to_ascii_uppercase();
    if text.is_empty() {
        return Err(LinearError::Empty);
    }
    let count = text.chars().count();
    if text.chars().any(|c| c == '*' || pattern(c).is_none()) {
        return Err(LinearError::NotEncodable);
    }
    if count > MAX_CHARACTERS {
        return Err(LinearError::TooLong {
            characters: count,
            limit: MAX_CHARACTERS,
        });
    }
    let mut runs = Vec::with_capacity((count + 2) * 10);
    for ch in std::iter::once('*')
        .chain(text.chars())
        .chain(std::iter::once('*'))
    {
        if !runs.is_empty() {
            runs.push(1.0); // the narrow gap between characters
        }
        for element in pattern(ch).into_iter().flat_map(str::bytes) {
            runs.push(if element == b'w' { wide_narrow } else { 1.0 });
        }
    }
    let (bars, width) = bars_of(&runs, |_| false);
    Ok(LinearCode {
        bars,
        width,
        fixed_quiet: None,
        printed: Printed::Line(text.clone()),
        normalised: text,
        corrected: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_character_has_three_wide_elements_and_nine_in_all() {
        for (ch, pattern) in TABLE {
            assert_eq!(pattern.len(), 9, "{ch}");
            assert_eq!(pattern.matches('w').count(), 3, "{ch}");
        }
    }

    #[test]
    fn curvyo_at_three_to_one_is_127_modules() {
        let code = encode("curvyo", 3.0).unwrap();
        assert!((code.width - 127.0).abs() < 1e-9);
        assert_eq!(code.normalised, "CURVYO");
    }

    #[test]
    fn the_default_content_is_191_modules_and_a_ratio_of_2_5_is_fractional() {
        assert!((encode("EXAMPLE-01", 3.0).unwrap().width - 191.0).abs() < 1e-9);
        let narrow = encode("EXAMPLE-01", 2.5).unwrap().width;
        assert!((narrow - (12.0 * (6.0 + 3.0 * 2.5) + 11.0)).abs() < 1e-9);
    }

    #[test]
    fn a_star_or_a_foreign_character_is_refused() {
        assert_eq!(encode("A*B", 3.0), Err(LinearError::NotEncodable));
        assert_eq!(encode("a\u{e9}", 3.0), Err(LinearError::NotEncodable));
        assert_eq!(encode("", 3.0), Err(LinearError::Empty));
    }
}
