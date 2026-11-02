//! Lists the characters OCR confuses with each character (`specs/0091-trace-text-recognition`
//! `adrs.md` decision 4).
//!
//! A hinted line is matched against the characters of its OCR string and their look-alikes only,
//! so a misread umlaut, a dotless `i` or an `l` read as `I` can still be fitted right: the OCR
//! narrows, the matcher decides.

/// Groups of characters an OCR model reads for one another (Proposal). A character in a group
/// brings the whole group; a character in no group brings itself.
const GROUPS: &[&str] = &[
    "lI1|!iíìîïj",
    "O0oQDöÖòóôõøÒÓÔÕØ",
    "ßB8",
    "AÄÀÁÂÃÅÆ4",
    "aäàáâãåæ",
    "uüùúûUÜÙÚÛvV",
    "eéèêëcçEÉÈÊË",
    "nñmrh",
    "S5$sšŠ",
    "Z2zžŽ7",
    "g9qy",
    "CcÇç(",
    "yýÿY",
    ".,'`´:;",
    "-–—_",
    "&8",
    "6b",
    "tf",
    "xX×kK",
    "wW",
    "pP",
];

/// The characters a line hinted with `text` may hold: those of `text` and their look-alikes,
/// each once, in first-seen order.
#[must_use]
pub(crate) fn hint_chars(text: &str) -> Vec<char> {
    let mut out: Vec<char> = Vec::new();
    let mut add = |c: char| {
        if !out.contains(&c) {
            out.push(c);
        }
    };
    for c in text.chars().filter(|c| !c.is_whitespace()) {
        add(c);
        for group in GROUPS.iter().filter(|g| g.contains(c)) {
            group.chars().for_each(&mut add);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umlauts_bring_their_base_letters_and_back() {
        let chars = hint_chars("BÂCK");
        for c in ['Ä', 'A', 'ß', '8', 'C', 'K'] {
            assert!(chars.contains(&c), "{c}");
        }
        assert!(hint_chars("a").contains(&'ä'));
        assert!(hint_chars("ü").contains(&'u'));
        assert!(!hint_chars("a b").contains(&' '));
    }

    #[test]
    fn a_character_in_no_group_is_kept() {
        assert_eq!(hint_chars("@"), vec!['@']);
    }
}
