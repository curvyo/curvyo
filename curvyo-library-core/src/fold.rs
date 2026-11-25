//! The folding of search text (ADR 0086 decision 3): lower case, Latin letters without their
//! diacritics ("ä" is "a", "ß" is "ss"), other scripts only lower-cased.

use crate::fold_table::FOLDS;

/// Combining diacritical marks (U+0300 to U+036F), dropped after lower-casing ("İ" lowers to
/// "i" and a dot above).
fn is_combining(c: char) -> bool {
    ('\u{0300}'..='\u{036f}').contains(&c)
}

/// Appends the folded `c` to `out`.
pub(crate) fn fold_char_into(c: char, out: &mut impl Extend<char>) {
    for lower in c.to_lowercase() {
        if is_combining(lower) {
            continue;
        }
        if lower.is_ascii() {
            out.extend([lower]);
        } else if let Ok(at) = FOLDS.binary_search_by_key(&lower, |(k, _)| *k) {
            out.extend(FOLDS[at].1.chars());
        } else {
            out.extend([lower]);
        }
    }
}

/// `text` folded.
#[must_use]
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        fold_char_into(c, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin_letters_lose_their_marks_and_other_scripts_only_lower() {
        let pairs = [
            ("Ä", "a"),
            ("äöü", "aou"),
            ("Straße", "strasse"),
            ("Æsir", "aesir"),
            ("Œuvre", "oeuvre"),
            ("Øre", "ore"),
            ("Łódź", "lodz"),
            ("Đorđe", "dorde"),
            ("Þór", "thor"),
            ("ẞ", "ss"),
            ("Crème Brûlée", "creme brulee"),
            ("İstanbul", "istanbul"),
            ("Ελλάδα", "ελλάδα"),
            ("arrow-left_2.svg", "arrow-left_2.svg"),
        ];
        for (text, want) in pairs {
            assert_eq!(fold(text), want, "{text}");
        }
    }
}
