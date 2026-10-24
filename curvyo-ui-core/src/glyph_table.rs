//! Groups a face's listed characters by Unicode block and filters them by a search query
//! (`specs/0082-glyph-table` criteria 6 and 16, `adrs.md` decision 6).

use curvyo_text_core::{ListedChar, UnicodeNames, block_of};

/// The group of characters in no Unicode block.
pub const NO_BLOCK: &str = "No block";

/// The longest search query (criterion 16).
pub const MAX_GLYPH_QUERY_CHARS: usize = 64;

/// One block header and its cells, as indices into the face's list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphGroup {
    /// The block's name, or [`NO_BLOCK`].
    pub name: &'static str,
    /// The cells, as indices into the listed characters.
    pub cells: Vec<usize>,
}

/// The table of a face for a query: its groups and how many cells they hold.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GlyphTable {
    /// The groups in table order.
    pub groups: Vec<GlyphGroup>,
    /// The cells of all groups.
    pub shown: usize,
}

/// A parsed search query.
struct Query {
    /// The query is one character.
    single: Option<char>,
    /// The query names a code point (`U+00E4`, `u+e4`, 4 to 6 hex digits).
    code: Option<u32>,
    /// The query in lower case, for the glyph and block names.
    lower: String,
    /// The query's words in upper case, for the character names.
    words: Vec<String>,
}

fn parse(query: &str) -> Query {
    let trimmed = query.trim();
    let mut chars = trimmed.chars();
    let single = match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    };
    let hex = trimmed
        .strip_prefix("U+")
        .or_else(|| trimmed.strip_prefix("u+"))
        .filter(|h| (1..=6).contains(&h.len()))
        .or_else(|| Some(trimmed).filter(|h| (4..=6).contains(&h.len())));
    let code = hex
        .filter(|h| h.chars().all(|c| c.is_ascii_hexdigit()))
        .and_then(|h| u32::from_str_radix(h, 16).ok());
    Query {
        single,
        code,
        lower: trimmed.to_lowercase(),
        words: trimmed.split_whitespace().map(str::to_uppercase).collect(),
    }
}

/// Whether `word` of a character name is matched by the query word `part`: the name word, or a
/// hyphen-joined piece of it, starts with it ("minus" matches "HYPHEN-MINUS").
fn word_matches(word: &str, part: &str) -> bool {
    word.starts_with(part) || word.split('-').any(|piece| piece.starts_with(part))
}

/// Matches character names against the query's words, the dictionary looked at once per query.
struct NameMatcher<'a> {
    names: &'a UnicodeNames,
    words: &'a [String],
    /// Per query word, which dictionary words it matches.
    hits: Vec<Vec<bool>>,
}

impl<'a> NameMatcher<'a> {
    fn new(names: &'a UnicodeNames, words: &'a [String]) -> Self {
        let hits = words
            .iter()
            .map(|part| {
                names
                    .words()
                    .iter()
                    .map(|word| word_matches(word, part))
                    .collect()
            })
            .collect();
        Self { names, words, hits }
    }

    fn matches(&self, c: char) -> bool {
        if self.words.is_empty() {
            return false;
        }
        if let Some(ids) = self.names.stored_words(c) {
            return self.hits.iter().all(|hit| {
                ids.iter()
                    .any(|id| usize::try_from(*id).is_ok_and(|id| hit.get(id) == Some(&true)))
            });
        }
        self.names.name_of(c).is_some_and(|name| {
            let name_words: Vec<&str> = name.split(' ').collect();
            self.words
                .iter()
                .all(|part| name_words.iter().any(|word| word_matches(word, part)))
        })
    }
}

/// The table of `list` (a face's listed characters, code point order) for `query`: every
/// character under its block when the query is blank; else the matches (criterion 16), the blocks
/// holding an exact character or code point match first, an exact match first in its block, the
/// rest in code point order; a one-character query matches that character only. The characters in
/// no block form one last group. `names` are the character names once loaded.
#[must_use]
pub fn glyph_table(list: &[ListedChar], query: &str, names: Option<&UnicodeNames>) -> GlyphTable {
    let blank = query.trim().is_empty();
    let parsed = parse(query);
    let matcher = names
        .filter(|_| !blank)
        .map(|names| NameMatcher::new(names, &parsed.words));
    let exact = |listed: &ListedChar| {
        parsed.single == Some(listed.c) || parsed.code == Some(u32::from(listed.c))
    };
    let mut groups: Vec<(bool, GlyphGroup)> = Vec::new();
    // The characters in no block, wherever they are in code point order: one last group.
    let mut no_block: (bool, Vec<usize>) = (false, Vec::new());
    let mut shown = 0;
    for (index, listed) in list.iter().enumerate() {
        let block = block_of(listed.c).unwrap_or(NO_BLOCK);
        let is_exact = !blank && exact(listed);
        // One character matches as that character only (criterion 16).
        let keep = blank
            || is_exact
            || parsed.single.is_none()
                && (matcher.as_ref().is_some_and(|m| m.matches(listed.c))
                    || listed
                        .name
                        .as_deref()
                        .is_some_and(|name| name.to_lowercase().contains(&parsed.lower))
                    || block.to_lowercase().contains(&parsed.lower));
        if !keep {
            continue;
        }
        shown += 1;
        if block == NO_BLOCK {
            no_block.0 |= is_exact;
            no_block.1.push(index);
            continue;
        }
        match groups.last_mut() {
            Some((has_exact, group)) if group.name == block => {
                if is_exact {
                    *has_exact = true;
                    let first_plain = group
                        .cells
                        .iter()
                        .position(|&at| !exact(&list[at]))
                        .unwrap_or(group.cells.len());
                    group.cells.insert(first_plain, index);
                } else {
                    group.cells.push(index);
                }
            }
            _ => groups.push((
                is_exact,
                GlyphGroup {
                    name: block,
                    cells: vec![index],
                },
            )),
        }
    }
    if !no_block.1.is_empty() {
        let (has_exact, mut cells) = no_block;
        // An exact match first in the group, the rest in code point order.
        cells.sort_by_key(|&at| !exact(&list[at]));
        groups.push((
            has_exact,
            GlyphGroup {
                name: NO_BLOCK,
                cells,
            },
        ));
    }
    // A stable sort: the blocks with an exact match first, each part in code point order.
    groups.sort_by_key(|(has_exact, _)| !*has_exact);
    GlyphTable {
        groups: groups.into_iter().map(|(_, group)| group).collect(),
        shown,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn listed(chars: &str) -> Vec<ListedChar> {
        let mut all: Vec<char> = chars.chars().collect();
        all.sort_unstable();
        all.dedup();
        all.into_iter()
            .map(|c| ListedChar {
                c,
                glyph: 1,
                name: match c {
                    'ä' => Some("adieresis".into()),
                    '\u{E000}' => Some("swash.A".into()),
                    _ => None,
                },
            })
            .collect()
    }

    fn names() -> UnicodeNames {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(dir.join("../frontend/public/unicode/names-17.0.0.bin")).unwrap();
        UnicodeNames::parse(&bytes).unwrap()
    }

    fn shown(table: &GlyphTable, list: &[ListedChar]) -> Vec<(&'static str, String)> {
        table
            .groups
            .iter()
            .map(|g| (g.name, g.cells.iter().map(|&i| list[i].c).collect()))
            .collect()
    }

    const FACE: &str = "AaÄä→←♥❤\u{E000}\u{2FFFF}";

    #[test]
    fn a_blank_query_lists_every_character_under_its_block() {
        let list = listed(FACE);
        let table = glyph_table(&list, "  ", None);
        assert_eq!(table.shown, list.len());
        assert_eq!(
            shown(&table, &list),
            vec![
                ("Basic Latin", "Aa".to_owned()),
                ("Latin-1 Supplement", "Ää".to_owned()),
                ("Arrows", "←→".to_owned()),
                ("Miscellaneous Symbols", "♥".to_owned()),
                ("Dingbats", "❤".to_owned()),
                ("Private Use Area", "\u{E000}".to_owned()),
                (NO_BLOCK, "\u{2FFFF}".to_owned()),
            ]
        );
    }

    #[test]
    fn words_match_the_character_names() {
        let list = listed(FACE);
        let names = names();
        let heart = glyph_table(&list, "heart", Some(&names));
        assert_eq!(
            shown(&heart, &list),
            vec![
                ("Miscellaneous Symbols", "♥".to_owned()),
                ("Dingbats", "❤".to_owned())
            ]
        );
        let umlaut = glyph_table(&list, "A Diaeresis", Some(&names));
        assert_eq!(
            shown(&umlaut, &list),
            vec![("Latin-1 Supplement", "Ää".to_owned())]
        );
        assert_eq!(umlaut.shown, 2);
        let none = glyph_table(&list, "xyz", Some(&names));
        assert_eq!(none.shown, 0);
        assert_eq!(none.shown, 0);
    }

    #[test]
    fn without_names_a_word_finds_nothing_by_name() {
        let list = listed(FACE);
        assert_eq!(glyph_table(&list, "heart", None).shown, 0);
    }

    #[test]
    fn an_exact_character_or_code_point_comes_first() {
        let list = listed(FACE);
        let names = names();
        for query in ["U+00E4", "u+e4", "00e4", "ä"] {
            let table = glyph_table(&list, query, Some(&names));
            assert_eq!(table.groups[0].name, "Latin-1 Supplement", "{query}");
            assert_eq!(list[table.groups[0].cells[0]].c, 'ä', "{query}");
        }
        // One character matches as that character only, not as a word or by substring.
        let table = glyph_table(&list, "a", Some(&names));
        assert_eq!(shown(&table, &list), vec![("Basic Latin", "a".to_owned())]);
        assert_eq!(
            glyph_table(&list, "w", None).shown,
            0,
            "not in \"Arrows\" or \"swash.A\""
        );
        // Three hex digits are no code point without "U+".
        assert_eq!(glyph_table(&list, "0e4", None).shown, 0);
    }

    #[test]
    fn glyph_and_block_names_match_by_substring() {
        let list = listed(FACE);
        let swash = glyph_table(&list, "swash", None);
        assert_eq!(
            shown(&swash, &list),
            vec![("Private Use Area", "\u{E000}".to_owned())]
        );
        let arrows = glyph_table(&list, "arrows", None);
        assert_eq!(shown(&arrows, &list), vec![("Arrows", "←→".to_owned())]);
        let dier = glyph_table(&list, "dieresis", None);
        assert_eq!(
            shown(&dier, &list),
            vec![("Latin-1 Supplement", "ä".to_owned())]
        );
    }

    #[test]
    fn a_hyphenated_name_word_matches_by_its_pieces() {
        let list = listed("-");
        let names = names();
        assert_eq!(glyph_table(&list, "minus", Some(&names)).shown, 1);
        assert_eq!(glyph_table(&list, "hyphen-minus", Some(&names)).shown, 1);
    }

    #[test]
    fn computed_names_match_too() {
        let list = listed("\u{4E00}\u{AC00}");
        let names = names();
        assert_eq!(glyph_table(&list, "cjk ideograph", Some(&names)).shown, 1);
        assert_eq!(glyph_table(&list, "hangul ga", Some(&names)).shown, 1);
    }
}
