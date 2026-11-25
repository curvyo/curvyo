//! The fuzzy matcher of the library browser (spec `0152` criteria 18 to 20 and 22, ADR 0086
//! decision 2): a dictionary of the distinct folded field words, scored once per query word.

use std::collections::HashMap;

use crate::fold::fold_char_into;

/// The weight of a field (ADR 0086 decision 2, Proposal).
pub type Weight = u8;
/// The item's name.
pub const WEIGHT_NAME: Weight = 8;
/// An alias of an icon.
pub const WEIGHT_ALIAS: Weight = 4;
/// A tag of a template.
pub const WEIGHT_TAG: Weight = 4;
/// A category.
pub const WEIGHT_CATEGORY: Weight = 2;
/// The source, the set name or a page name.
pub const WEIGHT_SOURCE: Weight = 1;

/// The query is cut at this many characters (`0138` criterion 10).
pub const MAX_QUERY_CHARS: usize = 100;

/// The strengths of a match, strongest first.
const EXACT: u8 = 16;
const PREFIX: u8 = 8;
const SUBSTRING: u8 = 4;
const TYPO: u8 = 2;
const SUBSEQUENCE: u8 = 1;
/// Longer words get no typo distance (its rows live on the stack).
const MAX_TYPO_CHARS: usize = 32;

fn is_separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, '-' | '_' | '.')
}

/// The letter mask of a word: one bit per `a` to `z` and `0` to `9`, one for anything else.
fn mask_of(chars: &[char]) -> u64 {
    chars.iter().fold(0, |mask, c| {
        mask | match c {
            'a'..='z' => 1 << (*c as u32 - 'a' as u32),
            '0'..='9' => 1 << (26 + *c as u32 - '0' as u32),
            _ => 1 << 36,
        }
    })
}

/// Splits folded `text` into words, calling `word` with each.
fn for_each_word(text: &str, buffer: &mut Vec<char>, mut word: impl FnMut(&[char])) {
    buffer.clear();
    for c in text.chars() {
        fold_char_into(c, buffer);
    }
    for part in buffer.split(|c| is_separator(*c)) {
        if !part.is_empty() {
            word(part);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DictWord {
    start: u32,
    len: u32,
    mask: u64,
}

/// The searchable words of every item, by item index in push order.
#[derive(Debug, Default, Clone)]
pub struct SearchIndex {
    chars: Vec<char>,
    words: Vec<DictWord>,
    lookup: HashMap<Box<[char]>, u32>,
    postings: Vec<(u32, Weight)>,
    ends: Vec<u32>,
}

/// The buffers a query reuses, so the hot path allocates nothing per item.
#[derive(Debug, Default, Clone)]
pub struct Scratch {
    strengths: Vec<u8>,
    scores: Vec<u32>,
    query: Vec<char>,
    words: Vec<(usize, usize)>,
}

impl SearchIndex {
    /// An empty index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ends.len()
    }

    /// Whether no item was pushed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ends.is_empty()
    }

    /// Adds the next item with its field texts and their weights.
    pub fn push(&mut self, fields: &[(&str, Weight)]) {
        let start = self.postings.len();
        let mut buffer = Vec::new();
        for (text, weight) in fields {
            for_each_word(text, &mut buffer, |word| {
                let id = self.word_id(word);
                let mine = &mut self.postings[start..];
                if let Some(seen) = mine.iter_mut().find(|(w, _)| *w == id) {
                    seen.1 = seen.1.max(*weight);
                } else {
                    self.postings.push((id, *weight));
                }
            });
        }
        // invariant: an index holds far fewer than 2^32 postings (50,000 items budgeted)
        self.ends
            .push(u32::try_from(self.postings.len()).unwrap_or(u32::MAX));
    }

    fn word_id(&mut self, word: &[char]) -> u32 {
        if let Some(id) = self.lookup.get(word) {
            return *id;
        }
        let id = u32::try_from(self.words.len()).unwrap_or(u32::MAX);
        self.words.push(DictWord {
            start: u32::try_from(self.chars.len()).unwrap_or(u32::MAX),
            len: u32::try_from(word.len()).unwrap_or(u32::MAX),
            mask: mask_of(word),
        });
        self.chars.extend_from_slice(word);
        self.lookup.insert(word.into(), id);
        id
    }

    fn word(&self, word: DictWord) -> &[char] {
        &self.chars[word.start as usize..(word.start + word.len) as usize]
    }

    /// The score of every item for `query` (0: no match), or `None` when the query has no word
    /// and every item is shown. Every query word must match one field word of an item (AND).
    pub fn scores<'s>(&self, query: &str, scratch: &'s mut Scratch) -> Option<&'s [u32]> {
        let Scratch {
            strengths,
            scores,
            query: chars,
            words,
        } = scratch;
        chars.clear();
        for c in query.chars().take(MAX_QUERY_CHARS) {
            fold_char_into(c, chars);
        }
        words.clear();
        let mut at = 0;
        for part in chars.split(|c| is_separator(*c)) {
            if !part.is_empty() {
                words.push((at, part.len()));
            }
            at += part.len() + 1;
        }
        if words.is_empty() {
            return None;
        }
        scores.clear();
        scores.resize(self.ends.len(), 0);
        strengths.resize(self.words.len(), 0);
        for (n, (from, len)) in words.iter().enumerate() {
            let q = &chars[*from..from + len];
            let qmask = mask_of(q);
            for (slot, dict) in strengths.iter_mut().zip(&self.words) {
                *slot = strength(q, qmask, self.word(*dict), dict.mask);
            }
            let mut start = 0;
            for (score, end) in scores.iter_mut().zip(&self.ends) {
                let end = *end as usize;
                if n == 0 || *score > 0 {
                    let best = self.postings[start..end]
                        .iter()
                        .map(|(w, weight)| u32::from(*weight) * u32::from(strengths[*w as usize]))
                        .max()
                        .unwrap_or(0);
                    *score = if best == 0 { 0 } else { *score + best };
                }
                start = end;
            }
        }
        Some(scores.as_slice())
    }

    /// The matching items as `(score, item)`, best first, equal scores in item order.
    pub fn rank(&self, query: &str, scratch: &mut Scratch, out: &mut Vec<(u32, u32)>) {
        out.clear();
        match self.scores(query, scratch) {
            None => out.extend((0..self.ends.len()).map(|i| (0, item_id(i)))),
            Some(scores) => {
                out.extend(
                    scores
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| **s > 0)
                        .map(|(i, s)| (*s, item_id(i))),
                );
                out.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
            }
        }
    }
}

/// An item index as the result carries it.
fn item_id(index: usize) -> u32 {
    // invariant: an index holds far fewer than 2^32 items
    u32::try_from(index).unwrap_or(u32::MAX)
}

/// The edits a query word of `len` letters may have (criterion 20).
fn typo_budget(len: usize) -> usize {
    match len {
        0..=3 => 0,
        4..=7 => 1,
        _ => 2,
    }
}

/// The strength of query word `q` against field word `w`; 0 for none.
fn strength(q: &[char], qmask: u64, w: &[char], wmask: u64) -> u8 {
    let missing = (qmask & !wmask).count_ones() as usize;
    let budget = typo_budget(q.len());
    if missing == 0 {
        if q == w {
            return EXACT;
        }
        if w.starts_with(q) {
            return PREFIX;
        }
        if q.len() <= w.len() && w.windows(q.len()).any(|part| part == q) {
            return SUBSTRING;
        }
    }
    if q.len() == 3 && w.len() >= 3 && missing == 0 && is_one_swap(q, &w[..3]) {
        return TYPO;
    }
    if budget > 0
        && missing <= budget
        && w.len() + budget >= q.len()
        && q.len() <= MAX_TYPO_CHARS
        && prefix_distance(q, &w[..w.len().min(q.len() + budget)]) <= budget
    {
        return TYPO;
    }
    if missing == 0 && q.len() >= 3 && w.first() == q.first() && is_subsequence(q, w) {
        return SUBSEQUENCE;
    }
    0
}

/// Whether `w` is `q` with two neighbouring letters swapped.
fn is_one_swap(q: &[char], w: &[char]) -> bool {
    // No allocation: the first differing position decides (ADR 0086 decision 2, hot path).
    let Some(a) = (0..q.len()).find(|i| q[*i] != w[*i]) else {
        return false;
    };
    let b = a + 1;
    b < q.len() && q[a] == w[b] && q[b] == w[a] && q[b + 1..] == w[b + 1..q.len()]
}

/// Whether the letters of `q` appear in `w` in order.
fn is_subsequence(q: &[char], w: &[char]) -> bool {
    let mut rest = w.iter();
    q.iter().all(|c| rest.any(|x| x == c))
}

/// The optimal string alignment distance of `q` to the closest prefix of `w` (insert, delete,
/// substitute, swap neighbours); `w` is at most `MAX_TYPO_CHARS + 2` letters.
fn prefix_distance(q: &[char], w: &[char]) -> usize {
    const CELLS: usize = MAX_TYPO_CHARS + 3;
    let n = w.len().min(CELLS - 1);
    let mut before = [0u8; CELLS];
    let mut last: [u8; CELLS] = std::array::from_fn(|j| u8::try_from(j).unwrap_or(u8::MAX));
    let mut row = [0u8; CELLS];
    for i in 1..=q.len() {
        row[0] = u8::try_from(i).unwrap_or(u8::MAX);
        for j in 1..=n {
            let cost = u8::from(q[i - 1] != w[j - 1]);
            let mut best = (last[j] + 1).min(row[j - 1] + 1).min(last[j - 1] + cost);
            if i > 1 && j > 1 && q[i - 1] == w[j - 2] && q[i - 2] == w[j - 1] {
                best = best.min(before[j - 2] + 1);
            }
            row[j] = best;
        }
        before = last;
        last = row;
    }
    last[..=n]
        .iter()
        .copied()
        .min()
        .map_or(usize::MAX, usize::from)
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
