//! Code 128 (ISO/IEC 15417) in the sets B and C with the shortest switching, mod-103 check
//! symbol. Printable ASCII only, so set A (control characters) is never reached.

use super::{LinearCode, LinearError, MAX_CHARACTERS, Printed, bars_of, flatten_lines};

/// The 107 symbols as six element widths (bar, space, ...); the stop has seven (ISO/IEC 15417).
const PATTERNS: [&str; 107] = [
    "212222", "222122", "222221", "121223", "121322", "131222", "122213", "122312", "132212",
    "221213", "221312", "231212", "112232", "122132", "122231", "113222", "123122", "123221",
    "223211", "221132", "221231", "213212", "223112", "312131", "311222", "321122", "321221",
    "312212", "322112", "322211", "212123", "212321", "232121", "111323", "131123", "131321",
    "112313", "132113", "132311", "211313", "231113", "231311", "112133", "112331", "132131",
    "113123", "113321", "133121", "313121", "211331", "231131", "213113", "213311", "213131",
    "311123", "311321", "331121", "312113", "312311", "332111", "314111", "221411", "431111",
    "111224", "111422", "121124", "121421", "141122", "141221", "112214", "112412", "122114",
    "122411", "142112", "142211", "241211", "221114", "413111", "241112", "134111", "111242",
    "121142", "121241", "114212", "124112", "124211", "411212", "421112", "421211", "212141",
    "214121", "412121", "111143", "111341", "131141", "114113", "114311", "411113", "411311",
    "113141", "114131", "311141", "411131", "211412", "211214", "211232", "2331112",
];
const START_B: usize = 104;
const START_C: usize = 105;
const CODE_B: usize = 100;
const CODE_C: usize = 99;
const STOP: usize = 106;

/// The widths of a symbol in modules.
fn widths(symbol: usize) -> impl Iterator<Item = f64> {
    PATTERNS[symbol]
        .bytes()
        .map(|digit| f64::from(digit - b'0'))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Set {
    B,
    C,
}

/// The fewest symbols (switches included) that encode `chars[at..]` starting in `set`.
fn cost(chars: &[u8], at: usize, set: Set, memo: &mut [[Option<usize>; 2]]) -> usize {
    if at == chars.len() {
        return 0;
    }
    let slot = usize::from(set == Set::C);
    if let Some(known) = memo[at][slot] {
        return known;
    }
    let pair = chars.get(at + 1).is_some_and(u8::is_ascii_digit) && chars[at].is_ascii_digit();
    let stay = match set {
        Set::B => Some(1 + cost(chars, at + 1, Set::B, memo)),
        Set::C if pair => Some(1 + cost(chars, at + 2, Set::C, memo)),
        Set::C => None,
    };
    // Switching: one symbol, then the same position in the other set (never switch back at once).
    let switched = match set {
        Set::B if pair => Some(1 + cost_after_switch(chars, at, Set::C, memo)),
        Set::C => Some(1 + cost_after_switch(chars, at, Set::B, memo)),
        Set::B => None,
    };
    let best = match (stay, switched) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => usize::MAX / 2,
    };
    memo[at][slot] = Some(best);
    best
}

/// The cost of `chars[at..]` in `set` entered by a switch: the first step must consume a character.
fn cost_after_switch(chars: &[u8], at: usize, set: Set, memo: &mut [[Option<usize>; 2]]) -> usize {
    match set {
        Set::B => 1 + cost(chars, at + 1, Set::B, memo),
        Set::C => 1 + cost(chars, at + 2, Set::C, memo),
    }
}

/// The symbol values of `chars` (without the check symbol and stop), the shortest, a tie staying
/// in the current set.
fn symbols(chars: &[u8]) -> Vec<usize> {
    let mut memo = vec![[None; 2]; chars.len() + 1];
    let paired = chars.len() >= 2 && chars[0].is_ascii_digit() && chars[1].is_ascii_digit();
    let b = cost(chars, 0, Set::B, &mut memo);
    let c = if paired {
        cost(chars, 0, Set::C, &mut memo)
    } else {
        usize::MAX / 2
    };
    let mut set = if c < b { Set::C } else { Set::B };
    let mut out = vec![if set == Set::C { START_C } else { START_B }];
    let mut at = 0;
    while at < chars.len() {
        let pair = chars.get(at + 1).is_some_and(u8::is_ascii_digit) && chars[at].is_ascii_digit();
        let here = cost(chars, at, set, &mut memo);
        let stay_ok = set == Set::B || pair;
        let stay_cost = if stay_ok {
            Some(match set {
                Set::B => 1 + cost(chars, at + 1, Set::B, &mut memo),
                Set::C => 1 + cost(chars, at + 2, Set::C, &mut memo),
            })
        } else {
            None
        };
        if stay_cost == Some(here) {
            match set {
                Set::B => {
                    out.push(usize::from(chars[at] - b' '));
                    at += 1;
                }
                Set::C => {
                    out.push(usize::from((chars[at] - b'0') * 10 + chars[at + 1] - b'0'));
                    at += 2;
                }
            }
        } else if set == Set::B {
            out.push(CODE_C);
            set = Set::C;
        } else {
            out.push(CODE_B);
            set = Set::B;
        }
    }
    out
}

/// The symbols of `chars` with the mod-103 check symbol and the stop appended.
fn with_check(chars: &[u8]) -> Vec<usize> {
    let mut values = symbols(chars);
    let weighted: usize = values
        .iter()
        .enumerate()
        .map(|(index, value)| value * index.max(1))
        .sum();
    values.push(weighted % 103);
    values.push(STOP);
    values
}

/// Encodes `content` as Code 128 (printable ASCII, at most [`MAX_CHARACTERS`]).
pub(super) fn encode(content: &str) -> Result<LinearCode, LinearError> {
    let text = flatten_lines(content);
    if text.is_empty() {
        return Err(LinearError::Empty);
    }
    let count = text.chars().count();
    if !text.chars().all(|c| (' '..='~').contains(&c)) {
        return Err(LinearError::NotEncodable);
    }
    if count > MAX_CHARACTERS {
        return Err(LinearError::TooLong {
            characters: count,
            limit: MAX_CHARACTERS,
        });
    }
    let values = with_check(text.as_bytes());
    let runs: Vec<f64> = values.iter().flat_map(|v| widths(*v)).collect();
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

/// The runs of one symbol value, for the tests that walk the whole table.
#[cfg(test)]
pub(super) fn pattern_sum(symbol: usize) -> f64 {
    widths(symbol).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_symbol_is_eleven_modules_and_the_stop_thirteen() {
        for symbol in 0..STOP {
            assert!((pattern_sum(symbol) - 11.0).abs() < 1e-9, "symbol {symbol}");
        }
        assert!((pattern_sum(STOP) - 13.0).abs() < 1e-9);
    }

    #[test]
    fn curvyo_is_101_modules_in_set_b_with_the_known_check_symbol() {
        let code = encode("Curvyo").unwrap();
        assert!((code.width - 101.0).abs() < 1e-9);
        // Start B (104) + C 35 + u 85 + r 82 + v 86 + y 89 + o 79: weights 1, 1..6; 1818 mod 103 = 67.
        assert_eq!(
            with_check(b"Curvyo"),
            [104, 35, 85, 82, 86, 89, 79, 67, STOP]
        );
    }

    #[test]
    fn a_tie_stays_in_the_current_set_and_digits_alone_start_in_set_c() {
        // "EXAMPLE-01": two digits at the end cost the same in B or after a switch to C.
        let symbols = symbols(b"EXAMPLE-01");
        assert_eq!(symbols[0], START_B);
        assert!(!symbols.contains(&CODE_C));
        assert_eq!(symbols.len(), 11);
        assert_eq!(super::symbols(b"12345678")[0], START_C);
        assert_eq!(super::symbols(b"12345678").len(), 5);
        // Four digits in the middle pay for the switch there and back.
        let mid = super::symbols(b"AB123456CD");
        assert!(mid.contains(&CODE_C) && mid.contains(&CODE_B));
    }

    #[test]
    fn the_default_content_is_145_modules() {
        assert!((encode("EXAMPLE-01").unwrap().width - 145.0).abs() < 1e-9);
    }

    #[test]
    fn invalid_content_names_its_reason() {
        assert_eq!(encode(""), Err(LinearError::Empty));
        assert_eq!(encode("caf\u{e9}"), Err(LinearError::NotEncodable));
        let long = "A".repeat(92);
        assert_eq!(
            encode(&long),
            Err(LinearError::TooLong {
                characters: 92,
                limit: 80
            })
        );
    }
}
