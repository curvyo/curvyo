//! The words of text recognition in Trace (`specs/0091-trace-text-recognition` UX notes §3 and §5),
//! with the counts and letters as arguments.

/// The bar notice when the reader could not be used (UX notes §3, permit over refuse).
pub const TEXT_FAILED_NOTICE: &str = "Text recognition failed. Traced without text.";

/// The most guessed letters the warning lists before "and N more" (`0088` criterion 7).
const GUESSED_LISTED: usize = 5;
/// The longest quoted line in characters before "…" (UX notes §5).
const QUOTE_LENGTH: usize = 24;
/// The most lines the nearest-face part quotes before "and N more".
const QUOTED_LINES: usize = 2;

/// One text a Create paths wrote, as the warning needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarnedText<'a> {
    /// The string.
    pub string: &'a str,
    /// The guessed letters: character and 1-based position.
    pub guessed: &'a [(char, usize)],
    /// Set in the nearest face, every letter guessed (Frage 114 B).
    pub nearest: bool,
}

/// "2 letters guessed: e (5), l (9)"; past five, "and 3 more" (the `0088` wording).
fn guessed_letters(guessed: &[(char, usize)]) -> String {
    let listed: Vec<String> = guessed
        .iter()
        .take(GUESSED_LISTED)
        .map(|(c, at)| format!("{c} ({at})"))
        .collect();
    let noun = if guessed.len() == 1 {
        "letter"
    } else {
        "letters"
    };
    let more = guessed.len().saturating_sub(GUESSED_LISTED);
    let tail = if more > 0 {
        format!(" and {more} more")
    } else {
        String::new()
    };
    format!(
        "{} {noun} guessed: {}{tail}",
        guessed.len(),
        listed.join(", ")
    )
}

/// `"text"`, cut at [`QUOTE_LENGTH`] characters with "…".
fn quoted(text: &str) -> String {
    if text.chars().count() > QUOTE_LENGTH {
        format!(
            "\"{}…\"",
            text.chars().take(QUOTE_LENGTH).collect::<String>()
        )
    } else {
        format!("\"{text}\"")
    }
}

/// The warning after a Create paths that set text with guessed letters (UX notes §5): the partial
/// guesses first, then the lines set in the nearest face; `None` when every letter is sure (no
/// success notice).
#[must_use]
pub fn traced_text_warning(texts: &[WarnedText<'_>]) -> Option<String> {
    let partial: Vec<(char, usize)> = texts
        .iter()
        .filter(|t| !t.nearest)
        .flat_map(|t| t.guessed.iter().copied())
        .collect();
    let nearest: Vec<&WarnedText<'_>> = texts.iter().filter(|t| t.nearest).collect();
    let mut parts: Vec<String> = Vec::new();
    if !partial.is_empty() {
        parts.push(format!("{}.", guessed_letters(&partial)));
    }
    if !nearest.is_empty() {
        let quotes: Vec<String> = nearest
            .iter()
            .take(QUOTED_LINES)
            .map(|t| quoted(t.string))
            .collect();
        let more = nearest.len().saturating_sub(QUOTED_LINES);
        let list = if more > 0 {
            format!("{} and {more} more", quotes.join(", "))
        } else {
            quotes.join(", ")
        };
        let head = if nearest.len() == 1 {
            "1 line set in the nearest font".to_owned()
        } else {
            format!("{} lines set in the nearest font", nearest.len())
        };
        parts.push(format!("{head}, all letters guessed: {list}."));
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text<'a>(string: &'a str, guessed: &'a [(char, usize)], nearest: bool) -> WarnedText<'a> {
        WarnedText {
            string,
            guessed,
            nearest,
        }
    }

    #[test]
    fn every_letter_sure_warns_nothing() {
        assert_eq!(traced_text_warning(&[text("Hello", &[], false)]), None);
        assert_eq!(traced_text_warning(&[]), None);
    }

    #[test]
    fn partial_guesses_come_first_then_the_nearest_face_lines() {
        assert_eq!(
            traced_text_warning(&[
                text("Hello", &[('e', 2), ('l', 3)], false),
                text("Open daily and every Sunday too", &[('O', 1)], true),
            ])
            .as_deref(),
            Some(
                "2 letters guessed: e (2), l (3). 1 line set in the nearest font, all letters \
                 guessed: \"Open daily and every Sun…\"."
            )
        );
        assert_eq!(
            traced_text_warning(&[text("Hi", &[('i', 2)], false)]).as_deref(),
            Some("1 letter guessed: i (2).")
        );
    }

    #[test]
    fn long_lists_are_cut_with_and_more() {
        let letters: Vec<(char, usize)> = "abcdefg".chars().zip(1..).collect();
        assert_eq!(
            traced_text_warning(&[text("abcdefg", &letters, false)]).as_deref(),
            Some("7 letters guessed: a (1), b (2), c (3), d (4), e (5) and 2 more.")
        );
        let one = [('A', 1)];
        let lines: Vec<WarnedText<'_>> = ["A", "B", "C", "D"]
            .iter()
            .map(|s| text(s, &one, true))
            .collect();
        assert_eq!(
            traced_text_warning(&lines).as_deref(),
            Some("4 lines set in the nearest font, all letters guessed: \"A\", \"B\" and 2 more.")
        );
    }
}
