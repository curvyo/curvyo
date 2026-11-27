//! A 1D code as closed outlines: the bars widened or narrowed by Bar width adjust, the guard bars
//! of EAN and UPC reaching into the text band, and the human-readable text as glyph outlines in
//! the same part (ADR 0092 §3). Outlines run clockwise on the screen, the winding of ink.

use curvyo_document_core::{Length, Point};
use curvyo_extension_core::{ClosedPath, Cubic, Glyph, GlyphTable};

use super::{LinearCode, Printed};

/// How far the guard bars of EAN and UPC reach below the data bars at least, in modules.
pub(crate) const GUARD_REACH: f64 = 5.0;
/// The gap between the data bars and the top of EAN and UPC digits, in modules.
const EAN_TEXT_GAP: f64 = 1.0;
/// The gap between the data bars and the top of the Code 128 and Code 39 text, in modules.
const LINE_TEXT_GAP: f64 = 2.0;
/// The ground of an inverted code reaches this far above and below what it holds, in modules
/// **(Proposal)**.
const GROUND_PAD: f64 = 2.0;
/// The advance of a digit at size 1 when the table has none (an em fraction).
const FALLBACK_DIGIT: f64 = 0.55;

/// What the style needs besides the code.
#[derive(Debug, Clone, Copy)]
pub struct LinearStyle<'a> {
    /// One module (the narrow bar) in mm.
    pub module: Length,
    /// The height of the data bars in mm.
    pub bar_height: Length,
    /// The quiet zones in modules, left and right.
    pub quiet: (f64, f64),
    /// Bar width adjust as a fraction of the module (signed).
    pub adjust: f64,
    /// The ink is the spaces: one ground with the bars knocked out.
    pub inverted: bool,
    /// A Background is on: [`Drawn::background`] holds its rectangle and the box reaches it.
    pub background: bool,
    /// The human-readable text: its size in mm and the face's outlines; `None` for no text.
    pub text: Option<TextLayout<'a>>,
}

/// The text of a code: the type size and the outlines to set it with.
#[derive(Debug, Clone, Copy)]
pub struct TextLayout<'a> {
    /// The type size (a cap height).
    pub size: Length,
    /// The glyph outlines at 1 mm.
    pub glyphs: Option<&'a GlyphTable>,
}

/// The outlines and the numbers the generator needs about them.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawn {
    /// The outlines in mm, the origin at the centre of the data bars.
    pub paths: Vec<ClosedPath>,
    /// The highest y of the outlines (the box's top).
    pub top: Length,
    /// The lowest y of the outlines (the box's bottom; descenders included).
    pub bottom: Length,
    /// The Background's rectangle (the bars, the quiet zones and the text band), `None` while it is off.
    pub background: Option<ClosedPath>,
    /// The text could not be set in full: the face lacks a character, or no face is there.
    pub missing: bool,
}

fn line(from: Point, to: Point) -> Cubic {
    Cubic {
        c1: from,
        c2: to,
        to,
    }
}

/// A clockwise rectangle (y down).
fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> ClosedPath {
    let corners = [
        Point::new(x0, y0),
        Point::new(x1, y0),
        Point::new(x1, y1),
        Point::new(x0, y1),
    ];
    let mut segments = Vec::with_capacity(4);
    for at in 0..4 {
        segments.push(line(corners[at], corners[(at + 1) % 4]));
    }
    ClosedPath {
        start: corners[0],
        segments,
    }
}

/// `path` run the other way round, from the same start.
fn reversed(path: &ClosedPath) -> ClosedPath {
    let mut points = vec![path.start];
    points.extend(path.segments.iter().map(|segment| segment.to));
    let n = path.segments.len();
    let segments = (0..n)
        .map(|j| {
            let source = &path.segments[n - 1 - j];
            Cubic {
                c1: source.c2,
                c2: source.c1,
                to: points[n - 1 - j],
            }
        })
        .collect();
    ClosedPath {
        start: path.start,
        segments,
    }
}

/// `glyph` set at `size` mm with its pen start at `(x, baseline)`.
fn placed(glyph: &Glyph, size: f64, x: f64, baseline: f64) -> Vec<ClosedPath> {
    let map = |p: Point| Point::new(x + p.x * size, baseline + p.y * size);
    glyph
        .paths
        .iter()
        .map(|path| ClosedPath {
            start: map(path.start),
            segments: path
                .segments
                .iter()
                .map(|s| Cubic {
                    c1: map(s.c1),
                    c2: map(s.c2),
                    to: map(s.to),
                })
                .collect(),
        })
        .collect()
}

/// The height of a capital (or a digit) of the face at size 1: how far its top lies above the
/// baseline.
fn ascent(table: &GlyphTable, ch: char) -> f64 {
    table
        .glyphs
        .get(&ch)
        .or_else(|| table.glyphs.get(&'0'))
        .and_then(|glyph| {
            glyph
                .paths
                .iter()
                .flat_map(|p| std::iter::once(p.start.y).chain(p.segments.iter().map(|s| s.to.y)))
                .reduce(f64::min)
        })
        .map_or(0.7, |top| -top)
}

/// The widest advance of a digit at size 1 (what limits a digit's size under its 7 modules).
#[must_use]
pub fn digit_advance(table: Option<&GlyphTable>) -> f64 {
    table
        .map(|table| {
            ('0'..='9')
                .filter_map(|d| table.glyphs.get(&d))
                .map(|glyph| glyph.advance.as_mm())
                .fold(0.0, f64::max)
        })
        .filter(|advance| *advance > 0.0)
        .unwrap_or(FALLBACK_DIGIT)
}

/// The bars as rectangles `(top, bottom, guard bottom)` high, widened or narrowed centred on each
/// edge pair; inverted the spaces are what widens.
fn bar_paths(
    code: &LinearCode,
    style: &LinearStyle<'_>,
    (top, bottom, guards): (f64, f64, f64),
) -> Vec<ClosedPath> {
    let m = style.module.as_mm();
    let x_of = |modules: f64| (modules - code.width / 2.0) * m;
    let grow = if style.inverted {
        -style.adjust * m
    } else {
        style.adjust * m
    };
    code.bars
        .iter()
        .map(|bar| {
            let y1 = if bar.guard { guards } else { bottom };
            rect(
                x_of(bar.x) - grow / 2.0,
                top,
                x_of(bar.x + bar.width) + grow / 2.0,
                y1,
            )
        })
        .collect()
}

/// The glyph outlines of the human-readable text of `code` with their baseline at `baseline`, and
/// whether some of it could not be set (a character the face lacks, or no face at all).
fn set_text(
    code: &LinearCode,
    style: &LinearStyle<'_>,
    table: Option<&GlyphTable>,
    baseline: f64,
) -> (Vec<ClosedPath>, bool) {
    let x_of = |modules: f64| (modules - code.width / 2.0) * style.module.as_mm();
    let mut missing = style.text.is_some() && table.is_none();
    let mut text: Vec<ClosedPath> = Vec::new();
    if let (Some(layout), Some(table)) = (style.text, table) {
        let mut set = |ch: char| {
            let glyph = table.glyphs.get(&ch);
            if glyph.is_none() {
                missing = true;
            }
            glyph
        };
        match &code.printed {
            Printed::Cells(cells) => {
                for cell in cells {
                    if let Some(glyph) = set(cell.ch) {
                        let left = x_of(cell.x) - glyph.advance.as_mm() * layout.size.as_mm() / 2.0;
                        text.extend(placed(glyph, layout.size.as_mm(), left, baseline));
                    }
                }
            }
            Printed::Line(line) => {
                let advances: Vec<f64> = line
                    .chars()
                    .map(|ch| {
                        table
                            .glyphs
                            .get(&ch)
                            .map_or(0.0, |glyph| glyph.advance.as_mm() * layout.size.as_mm())
                    })
                    .collect();
                let mut x = -advances.iter().sum::<f64>() / 2.0;
                for (ch, advance) in line.chars().zip(advances) {
                    if let Some(glyph) = set(ch) {
                        text.extend(placed(glyph, layout.size.as_mm(), x, baseline));
                    }
                    x += advance;
                }
            }
        }
    }
    (text, missing)
}

/// Draws `code` in `style`.
#[must_use]
pub fn draw(code: &LinearCode, style: &LinearStyle<'_>) -> Drawn {
    let m = style.module.as_mm();
    let half = code.width / 2.0;
    let x_of = |modules: f64| (modules - half) * m;
    let (top, bottom) = (
        -style.bar_height.as_mm() / 2.0,
        style.bar_height.as_mm() / 2.0,
    );
    let table = style.text.and_then(|text| text.glyphs);
    let ean = matches!(code.printed, Printed::Cells(_));
    let (gap, size) = style.text.map_or((0.0, 0.0), |t| {
        (
            if ean { EAN_TEXT_GAP } else { LINE_TEXT_GAP },
            t.size.as_mm(),
        )
    });
    let cap = table.map_or(0.7, |t| ascent(t, if ean { '0' } else { 'H' })) * size;
    let baseline = bottom + gap * m + cap;
    let guard_bottom = if ean {
        baseline.max(bottom + GUARD_REACH * m)
    } else {
        bottom
    };
    let mut bars = bar_paths(code, style, (top, bottom, guard_bottom));
    let (mut text, missing) = set_text(code, style, table, baseline);
    let mut ink: Vec<ClosedPath> = Vec::new();
    let lowest = |paths: &[ClosedPath], floor: f64| {
        paths
            .iter()
            .flat_map(|p| std::iter::once(p.start.y).chain(p.segments.iter().map(|s| s.to.y)))
            .fold(floor, f64::max)
    };
    let low = lowest(&text, lowest(&bars, bottom));
    let ground = || {
        rect(
            x_of(-style.quiet.0),
            top - GROUND_PAD * m,
            x_of(code.width + style.quiet.1),
            low + GROUND_PAD * m,
        )
    };
    let background = style.background.then(ground);
    if style.inverted {
        // One ground over the bars and the quiet zones; the bars and the text are knocked out.
        ink.push(ground());
        ink.extend(bars.drain(..).map(|b| reversed(&b)));
        ink.extend(text.iter().map(reversed));
        return Drawn {
            paths: ink,
            top: Length::from_mm(top - GROUND_PAD * m),
            bottom: Length::from_mm(low + GROUND_PAD * m),
            background,
            missing,
        };
    }
    ink.append(&mut bars);
    ink.append(&mut text);
    // The box reaches the Background's rectangle while it is on.
    let pad = if style.background {
        GROUND_PAD * m
    } else {
        0.0
    };
    Drawn {
        paths: ink,
        top: Length::from_mm(top - pad),
        bottom: Length::from_mm(low + pad),
        background,
        missing,
    }
}
