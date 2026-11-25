//! The detail bar's words on why a tile matched the search (spec `0153` criterion 14, UX notes
//! K4 and K5): 'Matches “magnifier” (keyword)', '(category)', 'Matches “bin” (related to trash)'.

use serde::Serialize;

use curvyo_library_core::Why;

/// The line and, for the preview box's tooltip, every reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WhyWords {
    /// The first reason in typed order, then " and 1 more".
    pub line: String,
    /// Every reason, one per line.
    pub all: String,
}

fn reason(why: &Why) -> Option<String> {
    match why {
        Why::Keyword(word) => Some(format!("Matches “{word}” (keyword)")),
        Why::Category(word) => Some(format!("Matches “{word}” (category)")),
        Why::Related { head, word } => Some(format!("Matches “{head}” (related to {word})")),
        Why::Name | Why::Source => None,
    }
}

/// The words for the reasons of one item, one per typed word; `None` when every typed word
/// matched a name, alias, tag or source word, or nothing matched.
#[must_use]
pub fn why_words(reasons: &[Why]) -> Option<WhyWords> {
    let lines: Vec<String> = reasons.iter().filter_map(reason).collect();
    let first = lines.first()?;
    let line = match lines.len() {
        1 => first.clone(),
        n => format!("{first} and {} more", n - 1),
    };
    Some(WhyWords {
        line,
        all: lines.join("\n"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_reason_leads_and_the_rest_are_counted() {
        assert_eq!(why_words(&[Why::Name]), None);
        assert_eq!(why_words(&[]), None);
        let words = why_words(&[
            Why::Name,
            Why::Keyword("magnifier".into()),
            Why::Related {
                head: "bin".into(),
                word: "trash".into(),
            },
            Why::Category("arrows".into()),
        ])
        .unwrap();
        assert_eq!(words.line, "Matches “magnifier” (keyword) and 2 more");
        assert_eq!(
            words.all,
            "Matches “magnifier” (keyword)\nMatches “bin” (related to trash)\nMatches “arrows” (category)"
        );
    }
}
