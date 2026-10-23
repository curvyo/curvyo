//! Takes the sample a font preview follows from a text: its first line, at most 200 characters
//! (`specs/0077-font-preview` criteria 7 to 9, `adrs.md` decision 5).

/// The most characters of a preview sample (criterion 7).
pub const MAX_SAMPLE_CHARS: usize = 200;

/// The first line of `text` (up to the first line break), cut after [`MAX_SAMPLE_CHARS`]
/// characters at a character boundary.
#[must_use]
pub fn sample_line(text: &str) -> &str {
    let line = text.split(['\n', '\r']).next().unwrap_or_default();
    line.char_indices()
        .nth(MAX_SAMPLE_CHARS)
        .map_or(line, |(at, _)| &line[..at])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_line_is_the_sample() {
        assert_eq!(sample_line("Anna & Jonas\n2026"), "Anna & Jonas");
        assert_eq!(sample_line("Lea\r\nTom"), "Lea");
        assert_eq!(sample_line(""), "");
        assert_eq!(sample_line("\nsecond"), "");
    }

    #[test]
    fn a_long_line_is_cut_after_200_characters() {
        let long = "ä".repeat(201);
        assert_eq!(sample_line(&long).chars().count(), 200);
        assert_eq!(sample_line(&"a".repeat(200)), "a".repeat(200));
    }
}
