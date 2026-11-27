//! The GRBL 1.1 G-code of a CNC job (`specs/0161-cnc-output` criteria 12 to 14, `adrs.md` CN4):
//! a comment header, `G21 G90 G17 G94`, the spindle on, the moves as `G0` and `G1` with only the
//! changed axes, the spindle off and `M30`. No `M6`, no tool length offset, no canned cycles.

use std::fmt::Write as _;

use curvyo_document_core::Length;

use crate::job::{CncJob, MoveKind};
use crate::stock::Stock;
use crate::tools::Tool;
use crate::units::Rpm;

/// The longest line written (the connector's check, `0127`).
pub const MAX_LINE: usize = 79;

/// What the header names.
#[derive(Debug, Clone, Copy)]
pub struct GrblHeader<'a> {
    /// The page's name.
    pub page: &'a str,
    /// The job's tool.
    pub tool: &'a Tool,
    /// The stock.
    pub stock: &'a Stock,
    /// The safe height above the stock top.
    pub safe: Length,
}

/// `value` with at most 3 decimals, no trailing zeros, no exponent, `-0` as `0`.
#[must_use]
pub fn number(value: f64) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    match text {
        "-0" | "" => "0".to_owned(),
        other => other.to_owned(),
    }
}

/// A comment line of `text`: every character GRBL would act on or that is not plain ASCII
/// becomes `_`, cut to fit [`MAX_LINE`].
fn comment(text: &str) -> String {
    let clean: String = text
        .chars()
        .map(|c| match c {
            '(' | ')' | '?' | '!' | '~' | ';' => '_',
            c if (' '..='}').contains(&c) => c,
            _ => '_',
        })
        .take(MAX_LINE - 2)
        .collect();
    format!("({clean})")
}

fn header_lines(h: &GrblHeader<'_>, out: &mut String) {
    let s = h.stock;
    for line in [
        format!("Curvyo CNC job: {}", h.page),
        format!("Tool: {}, {} flutes", h.tool.name(), h.tool.flutes.get()),
        format!(
            "Stock: {} x {} x {} mm, {}",
            number(s.width.as_mm()),
            number(s.height.as_mm()),
            number(s.thickness.as_mm()),
            s.material
        ),
        "Work zero: front left corner of the stock, Z 0 on top".to_owned(),
    ] {
        out.push_str(&comment(&line));
        out.push('\n');
    }
}

/// The G-code of `job` with the header `header` and the spindle at `rpm`; the same job writes
/// the same bytes (criterion 14).
#[must_use]
pub fn write_grbl(job: &CncJob, header: &GrblHeader<'_>, rpm: Rpm) -> String {
    write_grbl_mapped(job, header, rpm).0
}

/// As [`write_grbl`], with the line (counted from 0) each move of the job ends on: a move that
/// changes no written number ends on the line before it.
#[must_use]
pub fn write_grbl_mapped(job: &CncJob, header: &GrblHeader<'_>, rpm: Rpm) -> (String, Vec<usize>) {
    let mut out = String::new();
    let mut lines = Vec::with_capacity(job.moves.len());
    header_lines(header, &mut out);
    out.push_str("G21 G90 G17 G94\n");
    let _ = writeln!(out, "M3 S{}", rpm.get());
    let mut at = [number(0.0), number(0.0), number(0.0)];
    let mut feed = String::new();
    let mut count = out.matches('\n').count();
    for m in &job.moves {
        let to = [number(m.to.x()), number(m.to.y()), number(m.to.z())];
        let mut line = String::from(if m.kind == MoveKind::Rapid {
            "G0"
        } else {
            "G1"
        });
        let mut moved = false;
        for (axis, (now, was)) in ["X", "Y", "Z"].iter().zip(to.iter().zip(&at)) {
            if now != was {
                let _ = write!(line, " {axis}{now}");
                moved = true;
            }
        }
        if !moved {
            lines.push(count.saturating_sub(1));
            continue;
        }
        if let Some(f) = m.feed {
            let f = number(f.get());
            if f != feed {
                let _ = write!(line, " F{f}");
                feed = f;
            }
        }
        out.push_str(&line);
        out.push('\n');
        lines.push(count);
        count += 1;
        at = to;
    }
    out.push_str("M5\n");
    let _ = writeln!(out, "G0 Z{}", number(header.safe.as_mm()));
    out.push_str("M30\n");
    (out, lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_have_at_most_three_decimals_and_no_negative_zero() {
        assert_eq!(number(1.587_54), "1.588");
        assert_eq!(number(-0.000_1), "0");
        assert_eq!(number(10.0), "10");
        assert_eq!(number(-2.5), "-2.5");
        assert_eq!(number(1e-7), "0");
        assert_eq!(number(123_456.0), "123456");
    }

    #[test]
    fn comments_lose_what_grbl_would_act_on() {
        let line = comment(&format!("Page (a)? ~! {}", "x".repeat(100)));
        assert!(line.len() <= MAX_LINE);
        assert!(line.starts_with("(Page _a__ __ x"));
    }
}
