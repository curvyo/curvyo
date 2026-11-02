//! Puts a read string's spaces where its ink has word gaps, and cuts it where the ink has much
//! wider gaps (`specs/0091-trace-text-recognition` round 3, criterion 29).
//!
//! The ink is the line's outlines projected on its direction; the string is laid out over it, so
//! each gap between ink spans falls between two characters. A gap wider than [`WORD_GAP`] em with
//! no space there gets one; a gap wider than [`SPLIT_GAP`] em cuts the line into separate texts.

use kurbo::{BezPath, Vec2};

/// A gap between ink spans this many em wide or wider is a word gap (Proposal, customer
/// 2026-11-02: "about 0.3 em").
pub const WORD_GAP: f64 = 0.3;
/// A gap this many em wide or wider separates texts (Proposal: "about 1.5 em", the chip row
/// "All | Favourites | Cuttable | Single line").
pub const SPLIT_GAP: f64 = 1.5;
/// A word gap is also this many times the line's median gap at least, and a cut twice that
/// (Proposal): letter-spaced display text ("F o n t s") has wide gaps between all its letters.
pub const OVER_MEDIAN: f64 = 2.0;

/// One part of a read line after the gaps.
#[derive(Debug, Clone, PartialEq)]
pub struct WordPart {
    /// Its string, spaces inserted, no leading or trailing space.
    pub text: String,
    /// The ink it covers along the line: from, to.
    pub span: (f64, f64),
}

/// The ink of `outlines` along the unit direction `u`: the spans of their projections, merged
/// where they touch or overlap, in order.
#[must_use]
pub fn ink_spans(outlines: &[BezPath], u: Vec2) -> Vec<(f64, f64)> {
    let mut spans: Vec<(f64, f64)> = outlines
        .iter()
        .filter_map(|path| {
            let (mut lo, mut hi) = (f64::MAX, f64::MIN);
            for el in path.elements() {
                if let Some(p) = el.end_point() {
                    let a = p.to_vec2().dot(u);
                    lo = lo.min(a);
                    hi = hi.max(a);
                }
            }
            (lo <= hi).then_some((lo, hi))
        })
        .collect();
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (lo, hi) in spans {
        match merged.last_mut() {
            Some(last) if lo <= last.1 => last.1 = last.1.max(hi),
            _ => merged.push((lo, hi)),
        }
    }
    merged
}

/// `text` cut and spaced by the gaps of `spans` (ink along the line), with `boundaries[i]` the
/// position along the line where character `i` starts as `text` is laid out over the ink
/// (`boundaries.len()` is the number of characters) and `em` the em of that layout. Each gap goes to the character boundary
/// nearest its middle. An inserted space never leaves a word of one character, unless that is a
/// digit, "&", "+" or "-" (Proposal, lead 2026-11-02: a wrong space is worse than a missing one).
#[must_use]
pub fn plan_words(text: &str, boundaries: &[f64], spans: &[(f64, f64)], em: f64) -> Vec<WordPart> {
    let chars: Vec<char> = text.chars().collect();
    let whole = || {
        vec![WordPart {
            text: text.trim().to_owned(),
            span: (
                spans.first().map_or(0.0, |s| s.0),
                spans.last().map_or(0.0, |s| s.1),
            ),
        }]
    };
    if chars.len() < 2 || boundaries.len() != chars.len() || spans.len() < 2 || em <= 0.0 {
        return whole();
    }
    // Per boundary between character i - 1 and i: a word gap, a cut, or nothing.
    let mut space_at = vec![false; chars.len()];
    let mut cut_at: Vec<(usize, f64)> = Vec::new();
    let width_of = |j: usize| spans[j + 1].0 - spans[j].1;
    let mut gaps: Vec<f64> = (0..spans.len() - 1).map(width_of).collect();
    gaps.sort_by(f64::total_cmp);
    // Two gaps or fewer tell nothing about the letter spacing.
    let median = if gaps.len() >= 3 {
        gaps[gaps.len() / 2]
    } else {
        0.0
    };
    let (word, cut) = (
        (WORD_GAP * em).max(OVER_MEDIAN * median),
        (SPLIT_GAP * em).max(2.0 * OVER_MEDIAN * median),
    );
    for (j, pair) in spans.windows(2).enumerate() {
        let (from, to) = (pair[0].1, pair[1].0);
        let gap = width_of(j);
        if gap < word {
            continue;
        }
        let middle = f64::midpoint(from, to);
        let Some(b) = (1..chars.len()).min_by(|&i, &j| {
            (boundaries[i] - middle)
                .abs()
                .total_cmp(&(boundaries[j] - middle).abs())
        }) else {
            continue;
        };
        if gap >= cut {
            if cut_at.last().is_none_or(|c| c.0 != b) {
                cut_at.push((b, middle));
            }
        } else {
            space_at[b] = true;
        }
    }
    keep_real_words(&chars, &mut space_at);
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut cuts = cut_at.iter().peekable();
    let span_of = |from: f64, to: f64| {
        let inside: Vec<&(f64, f64)> = spans
            .iter()
            .filter(|s| f64::midpoint(s.0, s.1) >= from && f64::midpoint(s.0, s.1) < to)
            .collect();
        (
            inside.first().map_or(from, |s| s.0),
            inside.last().map_or(to, |s| s.1),
        )
    };
    let mut from = f64::MIN;
    for (i, &c) in chars.iter().enumerate() {
        if let Some(&&(b, middle)) = cuts.peek()
            && b == i
        {
            cuts.next();
            let piece = current.trim().to_owned();
            if !piece.is_empty() {
                parts.push(WordPart {
                    text: piece,
                    span: span_of(from, middle),
                });
            }
            current.clear();
            from = middle;
        }
        if space_at[i] && !c.is_whitespace() && !current.ends_with(char::is_whitespace) {
            current.push(' ');
        }
        current.push(c);
    }
    let piece = current.trim().to_owned();
    if !piece.is_empty() {
        parts.push(WordPart {
            text: piece,
            span: span_of(from, f64::MAX),
        });
    }
    parts
}

/// A character that may stand alone as a word.
fn stands_alone(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '&' | '+' | '-')
}

/// Drops, left to right, every inserted space in `space_at` that would leave a word of one
/// character that cannot stand alone on either side.
fn keep_real_words(chars: &[char], space_at: &mut [bool]) {
    let breaks = |space_at: &[bool], i: usize| chars[i].is_whitespace() || space_at[i];
    for b in 1..chars.len() {
        if !space_at[b] || chars[b].is_whitespace() || chars[b - 1].is_whitespace() {
            continue;
        }
        // The word left of the boundary: back to the last break.
        let start = (0..b)
            .rev()
            .find(|&i| breaks(space_at, i))
            .map_or(0, |i| if chars[i].is_whitespace() { i + 1 } else { i });
        // The word right of it: on to the next break (an original space or a later insertion).
        let end = (b + 1..chars.len())
            .find(|&i| breaks(space_at, i))
            .unwrap_or(chars.len());
        let lone = |word: &[char]| word.len() == 1 && !stands_alone(word[0]);
        if lone(&chars[start..b]) || lone(&chars[b..end]) {
            space_at[b] = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Characters 1 mm apart from 0, an em of 1 mm.
    fn evenly(text: &str) -> Vec<f64> {
        (0..u32::try_from(text.chars().count()).unwrap_or(0))
            .map(f64::from)
            .collect()
    }

    #[test]
    fn touching_and_overlapping_shapes_are_one_span() {
        let rect =
            |x0: f64, x1: f64| kurbo::Shape::to_path(&kurbo::Rect::new(x0, 0.0, x1, 1.0), 0.1);
        let spans = ink_spans(
            &[rect(2.0, 3.0), rect(0.0, 1.0), rect(0.5, 1.5)],
            Vec2::new(1.0, 0.0),
        );
        assert_eq!(spans, vec![(0.0, 1.5), (2.0, 3.0)]);
    }

    /// A word gap with no space in the string gets one; a gap already spaced is left alone.
    #[test]
    fn a_word_gap_without_a_space_gets_one() {
        // "Embeddedfree": ink "Embedded" 0..7.9, a gap of 0.6 em, "free" 8.5..11.9.
        let text = "Embeddedfree";
        let spans = [(0.0, 7.9), (8.5, 11.9)];
        let parts = plan_words(text, &evenly(text), &spans, 1.0);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].text, "Embedded free");
        let spaced = "Embedded free";
        let parts = plan_words(spaced, &evenly(spaced), &[(0.0, 7.9), (9.0, 12.9)], 1.0);
        assert_eq!(parts[0].text, "Embedded free");
    }

    /// Letter-spaced text, every gap wide, gets no spaces.
    #[test]
    fn letter_spaced_text_gets_no_spaces() {
        let text = "Fonts";
        let spans = [(0.0, 0.6), (1.2, 1.8), (2.4, 3.0), (3.6, 4.2), (4.8, 5.4)];
        let starts: Vec<f64> = (0..5).map(|i| 1.2 * f64::from(i)).collect();
        assert_eq!(plan_words(text, &starts, &spans, 1.0)[0].text, "Fonts");
    }

    /// Letter gaps under 0.3 em add nothing.
    #[test]
    fn letter_gaps_add_nothing() {
        let text = "Hello";
        let spans = [(0.0, 0.9), (1.1, 1.9), (2.1, 2.9), (3.1, 3.9), (4.1, 4.9)];
        assert_eq!(
            plan_words(text, &evenly(text), &spans, 1.0)[0].text,
            "Hello"
        );
    }

    /// A gap over 1.5 em cuts the line into texts, each with its own ink.
    #[test]
    fn a_wide_gap_cuts_the_line() {
        // "AllFavourites": "All" 0..2.9, a gap of 2.1 em, "Favourites" 5..14.9.
        let text = "AllFavourites";
        let mut boundaries = evenly(text);
        for b in boundaries.iter_mut().skip(3) {
            *b += 2.0;
        }
        let parts = plan_words(text, &boundaries, &[(0.0, 2.9), (5.0, 14.9)], 1.0);
        assert_eq!(
            parts,
            vec![
                WordPart {
                    text: "All".to_owned(),
                    span: (0.0, 2.9)
                },
                WordPart {
                    text: "Favourites".to_owned(),
                    span: (5.0, 14.9)
                },
            ]
        );
    }

    /// A string with a wide gap after its first character: the gap sits after `first` letters.
    fn after(text: &str, first: usize) -> String {
        let chars = text.chars().count();
        let mut boundaries: Vec<f64> = (0..chars)
            .map(|i| f64::from(u32::try_from(i).unwrap_or(0)))
            .collect();
        for b in boundaries.iter_mut().skip(first) {
            *b += 0.8;
        }
        let spans: Vec<(f64, f64)> = boundaries.iter().map(|&x| (x, x + 0.9)).collect();
        plan_words(text, &boundaries, &spans, 1.0)[0].text.clone()
    }

    /// Lead 2026-11-02: an inserted space never leaves a lone letter or mark ("F onts",
    /// "C urvyo fonts", "A nak Gedong", "Q Search fonts", "Size o", "> EMS Alluwre").
    #[test]
    fn no_space_leaves_a_lone_letter_or_mark() {
        assert_eq!(after("Fonts", 1), "Fonts");
        assert_eq!(after("Curvyo", 1), "Curvyo");
        assert_eq!(after("AnakGedong", 1), "AnakGedong");
        assert_eq!(after("QSearch", 1), "QSearch");
        assert_eq!(after("Sizeo", 4), "Sizeo");
        assert_eq!(after(">EMS", 1), ">EMS");
    }

    /// A lone digit, "&", "+" or "-" may stand as a word.
    #[test]
    fn a_digit_or_ampersand_may_stand_alone() {
        assert_eq!(after("Track7", 5), "Track 7");
        assert_eq!(after("A&B", 1), "A&B");
        assert_eq!(after("Salt&", 4), "Salt &");
    }
}
