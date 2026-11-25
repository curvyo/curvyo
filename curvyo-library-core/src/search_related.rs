//! The related-words step of the library search (spec `0153` criteria 2 and 3, ADR 0087
//! decision 4): each typed word's head words give up to 8 related dictionary words, one hop.

use crate::keywords::{Keywords, MAX_RELATED};
use crate::search::{
    DictWord, EXACT, Head, PREFIX, SUBSTRING, SearchIndex, WEIGHT_KEYWORD, index_u32, mask_of,
};

impl SearchIndex {
    /// Takes the related words of `keywords` (call it after the last [`Self::push`]): each
    /// related word maps to this index's dictionary once; one the index does not have as a
    /// name, alias, tag or keyword word is dropped, and a head left with none too.
    pub fn set_related(&mut self, keywords: &Keywords) {
        let mut strong = vec![false; self.words.len()];
        for (id, weight) in &self.postings {
            if *weight >= WEIGHT_KEYWORD {
                strong[*id as usize] = true;
            }
        }
        self.heads.clear();
        self.head_chars.clear();
        self.related.clear();
        let mut buffer = Vec::new();
        for (head, words) in keywords.related() {
            let from = self.related.len();
            for word in words {
                buffer.clear();
                buffer.extend(word.chars());
                if let Some(id) = self.lookup.get(buffer.as_slice())
                    && strong[*id as usize]
                    && !self.related[from..].contains(id)
                {
                    self.related.push(*id);
                }
            }
            if self.related.len() == from {
                continue;
            }
            buffer.clear();
            buffer.extend(head.chars());
            self.heads.push(Head {
                word: DictWord {
                    start: index_u32(self.head_chars.len()),
                    len: index_u32(buffer.len()),
                    mask: mask_of(&buffer),
                },
                from: index_u32(from),
                to: index_u32(self.related.len()),
            });
            self.head_chars.extend_from_slice(&buffer);
        }
    }

    /// The related words of query word `q` into `picked`: heads it matches exactly first, then
    /// then by prefix, then as a substring, each in table order, at most 8 distinct, each with its head's strength.
    pub(crate) fn pick_related(&self, q: &[char], qmask: u64, picked: &mut Vec<(u32, u8, u32)>) {
        picked.clear();
        if q.len() < 3 {
            return;
        }
        for level in [EXACT, PREFIX, SUBSTRING] {
            for (n, head) in self.heads.iter().enumerate() {
                let found = head_strength(q, qmask, self.head(head), head.word.mask);
                if found != level {
                    continue;
                }
                for id in &self.related[head.from as usize..head.to as usize] {
                    if picked.len() == MAX_RELATED {
                        return;
                    }
                    if !picked.iter().any(|(p, _, _)| p == id) {
                        picked.push((*id, found, index_u32(n)));
                    }
                }
            }
        }
    }
}

/// The strength of query word `q` against head word `h`: exact, prefix or substring only (no
/// typo or subsequence match of a head, ADR 0087 decision 4).
fn head_strength(q: &[char], qmask: u64, h: &[char], hmask: u64) -> u8 {
    if qmask & !hmask != 0 {
        return 0;
    }
    if q == h {
        EXACT
    } else if h.starts_with(q) {
        PREFIX
    } else if q.len() <= h.len() && h.windows(q.len()).any(|part| part == q) {
        SUBSTRING
    } else {
        0
    }
}
