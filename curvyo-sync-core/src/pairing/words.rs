//! The check words: the 2,048-word BIP-39 English list (MIT, `data/bip39-english.LICENSE`) as
//! embedded data, indexed by the 11-bit values of `PairChannel::check_words`.

/// The list, one word per line, in BIP-39 order.
const LIST: &str = include_str!("../../data/bip39-english.txt");

/// The number of words in the list.
pub const CHECK_WORD_COUNT: usize = 2048;

/// The word at `index`; `None` at or above [`CHECK_WORD_COUNT`].
#[must_use]
pub fn check_word(index: u16) -> Option<&'static str> {
    LIST.lines().nth(usize::from(index))
}

/// The four words of four indices; an index out of range gives an empty word.
pub(crate) fn check_words(indices: [u16; 4]) -> [&'static str; 4] {
    indices.map(|index| check_word(index).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_has_2048_distinct_lower_case_words() {
        let words: Vec<&str> = LIST.lines().collect();
        assert_eq!(words.len(), CHECK_WORD_COUNT);
        let distinct: std::collections::BTreeSet<&str> = words.iter().copied().collect();
        assert_eq!(distinct.len(), CHECK_WORD_COUNT);
        assert!(
            words
                .iter()
                .all(|word| word.bytes().all(|b| b.is_ascii_lowercase()))
        );
        assert_eq!(check_word(0), Some("abandon"));
        assert_eq!(check_word(2047), Some("zoo"));
        assert_eq!(check_word(2048), None);
    }
}
