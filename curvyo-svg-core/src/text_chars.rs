//! The characters of a text (`specs/0024-svg-import-export` criteria 79, 86, 87, 97, 98): white
//! space as SVG handles it, `text-transform` applied to the string, control characters removed,
//! and the `x`, `y`, `dx`, `dy` and `rotate` lists given to the characters they address.

use crate::drawing_text::SvgChar;
use crate::text_style::TextTransform;

/// The most numbers read from one position list (criterion 98).
pub(crate) const MAX_LIST_NUMBERS: usize = 10_000;

/// How white space of a run is handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Space {
    /// Newlines removed, tabs as spaces, runs of spaces as one, the ends trimmed.
    Normal,
    /// `xml:space="preserve"`: newlines and tabs as spaces, every space kept.
    Preserve,
    /// `white-space: pre`: a newline breaks the line, every space kept.
    Pre,
}

/// One piece of character data of a text, or a line break.
#[derive(Debug, Clone)]
pub(crate) struct Segment {
    /// The raw characters; `None` is a line break (an Inkscape line, a flowed paragraph).
    pub(crate) text: Option<String>,
    pub(crate) style: usize,
    pub(crate) path: Option<usize>,
    /// The elements (indices into the list table) it lies in, outermost first.
    pub(crate) elements: Vec<usize>,
    pub(crate) space: Space,
    pub(crate) transform: TextTransform,
}

/// The position lists of one element.
#[derive(Debug, Clone, Default)]
pub(crate) struct Lists {
    pub(crate) x: Vec<f64>,
    pub(crate) y: Vec<f64>,
    pub(crate) dx: Vec<f64>,
    pub(crate) dy: Vec<f64>,
    pub(crate) rotate: Vec<f64>,
}

/// Whether `c` is removed as a control character (criterion 79).
fn control(c: char) -> bool {
    (c.is_control() && !matches!(c, '\n' | '\t' | '\r')) || matches!(c, '\u{fffe}' | '\u{ffff}')
}

/// `c` under `transform`; `word_start` says it begins a word (for `capitalize`).
fn transformed(c: char, transform: TextTransform, word_start: bool, out: &mut Vec<char>) {
    match transform {
        TextTransform::Upper => out.extend(c.to_uppercase()),
        TextTransform::Lower => out.extend(c.to_lowercase()),
        TextTransform::Capitalize if word_start => out.extend(c.to_uppercase()),
        _ => out.push(c),
    }
}

/// The characters of `segments` after white space, transform and the `budget` of characters
/// left in the file (charged), each with its segment; a line break is `'\n'`.
pub(crate) fn characters(segments: &[Segment], budget: &mut usize) -> Vec<(char, usize)> {
    let mut out: Vec<(char, usize)> = Vec::new();
    // Whether the last character kept is a space (or nothing yet): a normal space then goes.
    let mut after_space = true;
    let mut word_start = true;
    let mut chars = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let Some(text) = &segment.text else {
            out.push(('\n', index));
            after_space = true;
            word_start = true;
            continue;
        };
        for raw in text.chars() {
            if *budget == 0 {
                return trimmed(out, segments);
            }
            if control(raw) {
                continue;
            }
            let c = match (segment.space, raw) {
                (Space::Normal, '\n' | '\r') | (Space::Pre, '\r') => continue,
                (Space::Pre, '\n') => {
                    out.push(('\n', index));
                    after_space = true;
                    word_start = true;
                    continue;
                }
                (_, '\n' | '\r' | '\t') => ' ',
                (_, c) => c,
            };
            if c == ' ' && segment.space == Space::Normal && after_space {
                continue;
            }
            chars.clear();
            transformed(c, segment.transform, word_start, &mut chars);
            for &c in &chars {
                out.push((c, index));
                *budget -= 1;
            }
            after_space = c == ' ';
            word_start = !c.is_alphanumeric();
        }
    }
    trimmed(out, segments)
}

/// Removes the trailing spaces of normal runs.
fn trimmed(mut out: Vec<(char, usize)>, segments: &[Segment]) -> Vec<(char, usize)> {
    while let Some(&(' ', segment)) = out.last() {
        if segments[segment].space != Space::Normal {
            break;
        }
        out.pop();
    }
    out
}

/// The characters of a text with the lists of `elements` given to the characters each element
/// holds (line breaks take none); a nested element's values win for its characters.
pub(crate) fn placed(
    chars: &[(char, usize)],
    segments: &[Segment],
    elements: &[Lists],
) -> Vec<SvgChar> {
    let mut out: Vec<SvgChar> = chars
        .iter()
        .map(|&(ch, segment)| SvgChar {
            ch,
            x: None,
            y: None,
            dx: 0.0,
            dy: 0.0,
            rotate: None,
            style: segments[segment].style,
            path: segments[segment].path,
        })
        .collect();
    // Per element, the indices of its addressable characters in order.
    let mut held: Vec<Vec<usize>> = vec![Vec::new(); elements.len()];
    for (at, &(ch, segment)) in chars.iter().enumerate() {
        if ch == '\n' {
            continue;
        }
        for &element in &segments[segment].elements {
            held[element].push(at);
        }
    }
    for (lists, held) in elements.iter().zip(&held) {
        for (k, &at) in held.iter().enumerate() {
            let c = &mut out[at];
            if let Some(&v) = lists.x.get(k) {
                c.x = Some(v);
            }
            if let Some(&v) = lists.y.get(k) {
                c.y = Some(v);
            }
            if let Some(&v) = lists.dx.get(k) {
                c.dx = v;
            }
            if let Some(&v) = lists.dy.get(k) {
                c.dy = v;
            }
            if let Some(&v) = lists.rotate.get(k).or_else(|| lists.rotate.last()) {
                c.rotate = Some(v);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(text: &str, space: Space, elements: Vec<usize>) -> Segment {
        Segment {
            text: Some(text.to_owned()),
            style: 0,
            path: None,
            elements,
            space,
            transform: TextTransform::None,
        }
    }

    fn string(segments: &[Segment]) -> String {
        characters(segments, &mut 1000)
            .iter()
            .map(|(c, _)| c)
            .collect()
    }

    #[test]
    fn ac79_white_space() {
        assert_eq!(
            string(&[
                segment("  a\n b\t", Space::Normal, vec![]),
                segment("  c  ", Space::Normal, vec![])
            ]),
            "a b c"
        );
        assert_eq!(
            string(&[segment(" a\n\tb ", Space::Preserve, vec![])]),
            " a  b "
        );
        assert_eq!(string(&[segment("a \nb", Space::Pre, vec![])]), "a \nb");
        assert_eq!(string(&[segment("a\u{7}b", Space::Normal, vec![])]), "ab");
        let mut caps = segment("hello wORLD", Space::Normal, vec![]);
        caps.transform = TextTransform::Capitalize;
        assert_eq!(string(&[caps]), "Hello WORLD");
        let mut upper = segment("straße", Space::Normal, vec![]);
        upper.transform = TextTransform::Upper;
        assert_eq!(string(&[upper]), "STRASSE");
        let mut budget = 2;
        assert_eq!(
            characters(&[segment("abc", Space::Normal, vec![])], &mut budget).len(),
            2
        );
    }

    #[test]
    fn ac86_ac87_nested_lists_win_for_their_characters() {
        let segments = [
            segment("ab", Space::Normal, vec![0]),
            segment("cd", Space::Normal, vec![0, 1]),
        ];
        let chars = characters(&segments, &mut 100);
        let lists = [
            Lists {
                x: vec![1.0, 2.0, 3.0, 4.0],
                rotate: vec![10.0, 20.0],
                ..Lists::default()
            },
            Lists {
                x: vec![9.0],
                dy: vec![0.0, 5.0],
                ..Lists::default()
            },
        ];
        let placed = placed(&chars, &segments, &lists);
        let xs: Vec<_> = placed.iter().map(|c| c.x).collect();
        assert_eq!(xs, [Some(1.0), Some(2.0), Some(9.0), Some(4.0)]);
        assert_eq!(placed[3].dy, 5.0);
        assert_eq!(placed[3].rotate, Some(20.0));
    }
}
