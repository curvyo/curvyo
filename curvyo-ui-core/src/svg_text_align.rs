//! Whether an imported text keeps its file's alignment (`specs/0024-svg-import-export` criterion
//! 89): a piece of whole lines, each one text chunk anchored at one point, and an aligned layout
//! whose lines start where the file draws them, all off by one shift that the anchor takes.

use curvyo_document_core::TextSnapshot;
use curvyo_svg_core::TextAnchor;
use curvyo_text_core::TextLayout;
use kurbo::{Point, Vec2};

use crate::svg_text::Context;

/// How far a laid-out line may sit from where the file puts it before the piece is set left
/// aligned instead (mm).
const ALIGN_TOLERANCE_MM: f64 = 1e-4;

impl Context<'_, '_> {
    /// Where a centred or right-aligned run of several lines that begins or ends inside a line
    /// is cut, so its whole lines keep the alignment and the partial line goes on its own.
    pub(crate) fn partial_line_split(&self, chars: &[usize]) -> Option<usize> {
        let first = chars[0];
        let anchor = self
            .anchor
            .unwrap_or(self.text.styles[self.chars[first].style].anchor);
        if anchor == TextAnchor::Start {
            return None;
        }
        let line_starts: Vec<usize> = (1..chars.len())
            .filter(|&k| self.targets.breaks[chars[k]] > 0)
            .collect();
        let (&first_start, &last_start) = (line_starts.first()?, line_starts.last()?);
        let (starts_line, ends_line) = self.bounds_on_lines(chars);
        if !starts_line {
            Some(first_start)
        } else if !ends_line {
            Some(last_start)
        } else {
            None
        }
    }

    /// Whether `chars` begin at a line start and end at a line end of the whole text.
    fn bounds_on_lines(&self, chars: &[usize]) -> (bool, bool) {
        let real: Vec<usize> = (0..self.chars.len())
            .filter(|&i| self.chars[i].ch != '\n')
            .collect();
        let at = |i: usize| real.iter().position(|&r| r == i);
        let (Some(start), Some(end)) = (at(chars[0]), at(chars[chars.len() - 1])) else {
            return (false, false);
        };
        let starts_line = start == 0 || self.targets.breaks[chars[0]] > 0;
        let ends_line = real
            .get(end + 1)
            .is_none_or(|&next| self.targets.breaks[next] > 0);
        (starts_line, ends_line)
    }

    /// Whether `chars` are whole lines, each one text chunk, all anchored at one point
    /// (criterion 89: then the text keeps its alignment).
    pub(crate) fn whole_lines_of_one_anchor(&self, chars: &[usize]) -> bool {
        let first = chars[0];
        let anchor = self
            .anchor
            .unwrap_or(self.text.styles[self.chars[first].style].anchor);
        if anchor == TextAnchor::Start {
            return false;
        }
        let (starts_line, ends_line) = self.bounds_on_lines(chars);
        let start_along = self.targets.chunk_start[first];
        let lines_ok = chars.iter().enumerate().all(|(k, &i)| {
            let line_start = k == 0 || self.targets.breaks[i] > 0;
            self.targets.chunk[i] == line_start
                && (!line_start || (self.targets.chunk_start[i] - start_along).abs() < 1e-9)
        });
        starts_line && ends_line && lines_ok
    }

    /// Whether every line of the aligned layout starts where the file draws it, all off by one
    /// shift that the origin then takes (criterion 89).
    pub(crate) fn lands(
        &self,
        chars: &[usize],
        layout: &TextLayout,
        snapshot: &TextSnapshot,
        origin: Point,
        frame: kurbo::Affine,
        moved: &mut Point,
    ) -> bool {
        let mut first_of_line = Vec::new();
        let mut byte = 0;
        let mut k = 0;
        let mut next_starts = true;
        for c in snapshot.string.chars() {
            if c == '\n' {
                next_starts = true;
            } else {
                if next_starts {
                    first_of_line.push((byte, chars[k]));
                    next_starts = false;
                }
                k += 1;
            }
            byte += c.len_utf8();
        }
        let vertical =
            snapshot.layout.writing_mode != curvyo_document_core::TextWritingMode::Horizontal;
        let mut offset: Option<Vec2> = None;
        for (byte, i) in first_of_line {
            let Some(glyph) = layout
                .lines
                .iter()
                .flat_map(|line| &line.glyphs)
                .find(|g| g.cluster == byte)
            else {
                return false;
            };
            // SVG turns a glyph about its origin, the canvas about its centre: compare where the
            // canvas puts the turned origin (the kerns correct the same pivot, criterion 87).
            let turned = if vertical {
                glyph.origin
            } else {
                let centre = Point::new(glyph.centre_x.0, glyph.centre_y.0);
                kurbo::Affine::rotate_about(glyph.rotation_rad, centre) * glyph.origin
            };
            let actual = frame * turned;
            let wanted = (self.targets.at[i] - origin) * self.s;
            let mut residual = actual.to_vec2() - wanted;
            if vertical {
                // A column's origin sits on the file's pen across the line, as for a text
                // anchored at its start; only the alignment along the column moves it.
                residual.x = 0.0;
            }
            match offset {
                None => offset = Some(residual),
                Some(o) if (o - residual).hypot() > ALIGN_TOLERANCE_MM => return false,
                Some(_) => {}
            }
        }
        if let Some(o) = offset {
            // The placement is `translate(origin) * scale(1 / s)`: the glyphs move with the origin.
            *moved = origin - o / self.s;
        }
        true
    }
}
