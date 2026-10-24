//! The Unicode `Vertical_Orientation` property (UAX #50) reduced to upright or not, from the
//! generated table in `vertical_orientation_data.rs` (`specs/0079-text-layout-controls`
//! criterion 23).

#[path = "vertical_orientation_data.rs"]
mod data;

/// Whether `c` is U or Tu (set upright in vertical text); R and Tr are set sideways.
pub(crate) fn is_upright_char(c: char) -> bool {
    let c = u32::from(c);
    data::UPRIGHT
        .binary_search_by(|&(start, end)| {
            if end < c {
                std::cmp::Ordering::Less
            } else if start > c {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_sorted_and_disjoint() {
        assert!(data::UPRIGHT.windows(2).all(|w| w[0].1 < w[1].0));
        assert!(data::UPRIGHT.iter().all(|(start, end)| start <= end));
    }

    #[test]
    fn samples_of_each_class() {
        // U: CJK ideographs, kana, the section sign; Tu: the ideographic full stop (U+3002).
        for c in ['日', '本', 'あ', 'ア', '§', '\u{3002}', '\u{3001}'] {
            assert!(is_upright_char(c), "{c:?}");
        }
        // R: Latin, digits, the space; Tr: the prolonged sound mark (U+30FC), brackets (U+300C).
        for c in ['A', 'z', '1', ' ', '\u{30fc}', '\u{300c}', '(', '\u{ff08}'] {
            assert!(!is_upright_char(c), "{c:?}");
        }
    }
}
