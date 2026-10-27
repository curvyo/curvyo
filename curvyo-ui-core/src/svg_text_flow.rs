//! Flowed text as plain lines (`specs/0024-svg-import-export` criterion 96, Proposal): the
//! paragraphs of a `flowRoot`, a `shape-inside` or an `inline-size` text broken greedily at spaces
//! by the shaped widths, and the first character of each line placed for the box's alignment.

use curvyo_svg_core::{FlowAlign, SvgChar, SvgText, TextAnchor};

use crate::svg_text_layout::{Resolved, line_height_user, spacing};

/// A flowed text's characters with its lines broken at the box width and the first character
/// of each line placed (criterion 96), and the anchor its alignment gives; `None` when the text
/// is not flowed. Positions given in the file are dropped.
pub(crate) fn flowed(
    text: &SvgText,
    advances: &[f64],
    resolved: &[Option<Resolved<'_>>],
) -> Option<(Vec<SvgChar>, TextAnchor, bool)> {
    let flow = text.flow?;
    let root = &text.styles[text.root_style];
    let r = resolved.iter().flatten().next()?;
    let (anchor, justify) = if flow.by_anchor {
        (root.anchor, false)
    } else {
        match root.flow_align {
            FlowAlign::Start => (TextAnchor::Start, false),
            FlowAlign::Centre => (TextAnchor::Middle, false),
            FlowAlign::End => (TextAnchor::End, false),
            FlowAlign::Justify => (TextAnchor::Start, true),
        }
    };
    let x = if flow.by_anchor {
        flow.left
    } else {
        match anchor {
            TextAnchor::Start => flow.left,
            TextAnchor::Middle => flow.left + flow.width / 2.0,
            TextAnchor::End => flow.left + flow.width,
        }
    };
    let first_baseline = flow
        .first_baseline
        .unwrap_or(flow.top + r.face.metrics().ascender * root.font_size);
    let lh = line_height_user(root, r);
    let mut chars: Vec<SvgChar> = text
        .chars
        .iter()
        .map(|c| SvgChar {
            x: None,
            y: None,
            dx: 0.0,
            dy: 0.0,
            path: None,
            ..*c
        })
        .collect();
    // Greedy breaks at spaces by the shaped widths (a word wider than the box on its own line).
    let mut width = 0.0;
    let mut last_space: Option<usize> = None;
    let mut line_begin = 0;
    for i in 0..chars.len() {
        let c = chars[i];
        if c.ch == '\n' {
            width = 0.0;
            last_space = None;
            line_begin = i + 1;
            continue;
        }
        let advance = advances[i] + spacing(&text.styles[c.style], c.ch);
        if c.ch == ' ' {
            last_space = Some(i);
        } else if flow.width > 0.0
            && width + advances[i] > flow.width
            && i > line_begin
            && let Some(at) = last_space.filter(|at| *at > line_begin)
        {
            chars[at].ch = '\n';
            width = (at + 1..i)
                .map(|j| advances[j] + spacing(&text.styles[chars[j].style], chars[j].ch))
                .sum();
            line_begin = at + 1;
            last_space = None;
        }
        width += advance;
    }
    let mut line = 0_u32;
    let mut at_line_start = true;
    for c in &mut chars {
        if c.ch == '\n' {
            line += 1;
            at_line_start = true;
            continue;
        }
        if at_line_start {
            c.x = Some(x);
            c.y = Some(first_baseline + lh * f64::from(line));
            at_line_start = false;
        }
    }
    Some((chars, anchor, justify))
}
