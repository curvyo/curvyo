//! The words of one G-code line.

/// Comments (`;` and `( )`), spaces, tabs and a carriage return removed, upper case.
pub(crate) fn strip(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_paren = false;
    for c in line.chars() {
        if in_paren {
            in_paren = c != ')';
        } else if c == '(' {
            in_paren = true;
        } else if c == ';' {
            break;
        } else if !matches!(c, ' ' | '\t' | '\r') {
            out.push(c.to_ascii_uppercase());
        }
    }
    out
}

/// A plain decimal number: optional sign, digits, optional fraction; nothing else.
pub(crate) fn number(text: &str) -> Option<f64> {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let plain = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    let ok = plain(whole) && plain(fraction) && !(whole.is_empty() && fraction.is_empty());
    ok.then(|| text.parse().ok()).flatten()
}

/// The words of a stripped line: a letter and the number after it.
pub(crate) fn words(line: &str) -> Option<Vec<(char, &str)>> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(letter) = rest.chars().next() {
        if !letter.is_ascii_alphabetic() {
            return None;
        }
        let tail = &rest[1..];
        let end = tail
            .find(|c: char| c.is_ascii_alphabetic())
            .unwrap_or(tail.len());
        out.push((letter, &tail[..end]));
        rest = &tail[end..];
    }
    Some(out)
}
