//! The text of numbers and strings in a written SVG (criteria 53, 54, 57 and 72): pure Rust
//! formatting, no locale, the same bytes on every target.

use std::fmt::Write as _;

/// A coordinate or length: at most 6 decimals, trailing zeros and a bare point trimmed, never
/// `-0`, never an exponent. A non-finite number is written as `0`.
pub(crate) fn coord(value: f64, out: &mut String) {
    debug_assert!(value.is_finite(), "the mapping writes finite numbers");
    if !value.is_finite() {
        out.push('0');
        return;
    }
    let start = out.len();
    let _ = write!(out, "{value:.6}");
    trim_decimals(out, start);
    fix_negative_zero(out, start);
}

/// A number that must read back to the same `f64`: Rust's shortest round-trip text (never an
/// exponent), never `-0`. A non-finite number is written as `0`.
pub(crate) fn exact(value: f64, out: &mut String) {
    debug_assert!(value.is_finite(), "the mapping writes finite numbers");
    if !value.is_finite() {
        out.push('0');
        return;
    }
    let start = out.len();
    let _ = write!(out, "{value}");
    fix_negative_zero(out, start);
}

/// Removes trailing zeros after a decimal point and the point itself when nothing follows it.
fn trim_decimals(out: &mut String, start: usize) {
    if out[start..].contains('.') {
        while out.ends_with('0') {
            out.pop();
        }
        if out.ends_with('.') {
            out.pop();
        }
    }
}

/// `-0` becomes `0`.
fn fix_negative_zero(out: &mut String, start: usize) {
    if &out[start..] == "-0" {
        out.truncate(start);
        out.push('0');
    }
}

/// Whether `c` is a control or bidirectional control character that names drop (criterion 53).
const fn is_name_control(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{1f}' | '\u{7f}'..='\u{9f}' | '\u{200e}' | '\u{200f}'
        | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Whether XML 1.0 allows `c` in a document.
const fn is_xml_char(c: char) -> bool {
    matches!(c, '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}'
        | '\u{10000}'..='\u{10ffff}')
}

fn escape_char(c: char, out: &mut String) {
    match c {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '"' => out.push_str("&quot;"),
        '\'' => out.push_str("&apos;"),
        _ => out.push(c),
    }
}

/// A name (label, page name, font family): escaped, without control or bidirectional control
/// characters.
pub(crate) fn escape_name(text: &str, out: &mut String) {
    for c in text
        .chars()
        .filter(|c| !is_name_control(*c) && is_xml_char(*c))
    {
        escape_char(c, out);
    }
}

/// Text content: escaped, without the characters XML forbids, control and bidirectional
/// control characters (criterion 132).
pub(crate) fn escape_text(text: &str, out: &mut String) {
    escape_name(text, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(v: f64) -> String {
        let mut s = String::new();
        coord(v, &mut s);
        s
    }

    fn e(v: f64) -> String {
        let mut s = String::new();
        exact(v, &mut s);
        s
    }

    #[test]
    fn ac54_coordinates_have_at_most_six_decimals_and_no_negative_zero() {
        assert_eq!(c(10.0), "10");
        assert_eq!(c(-0.0), "0");
        assert_eq!(c(-1e-7), "0");
        assert_eq!(c(1e-7), "0");
        assert_eq!(c(0.1 + 0.2), "0.3");
        assert_eq!(c(1e7), "10000000");
        assert_eq!(c(-1e7), "-10000000");
        assert_eq!(c(11.5), "11.5");
        assert_eq!(c(1.234_567_89), "1.234568");
    }

    #[test]
    fn ac57_exact_numbers_read_back_and_have_no_exponent() {
        assert_eq!(e(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(e(-0.0), "0");
        assert_eq!(e(1e-7), "0.0000001");
        assert_eq!(e(1e21), "1000000000000000000000");
        assert_eq!(e(std::f64::consts::FRAC_PI_2), "1.5707963267948966");
        for v in [0.1, 1.0 / 3.0, 6.943, -45.0_f64.to_radians()] {
            assert_eq!(e(v).parse::<f64>().unwrap_or(f64::NAN), v);
        }
    }

    #[test]
    fn ac53_names_are_escaped_and_lose_controls() {
        let mut s = String::new();
        escape_name("a&b<c>\"d'\u{202e}e\u{7}f", &mut s);
        assert_eq!(s, "a&amp;b&lt;c&gt;&quot;d&apos;ef");
    }
}
