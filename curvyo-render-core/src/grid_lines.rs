//! The triangles of a page grid's lines (`specs/0090-grid` criterion 4, `adrs.md` decision 6).
//!
//! The layout (positions, opacity, emphasis) comes from `curvyo-ui-core`'s grid module through
//! [`GridStroke`]; this module only turns each line into a 1 device-pixel quad in the grid ink
//! chosen from the page background, blending the minor and major opacity by the emphasis.

use curvyo_document_core::{BackgroundPaint, DocumentBackground, Point, ViewTransform};

use crate::color::RgbaColor;
use crate::glyphs::{DrawList, Vertex};
use crate::theme;

/// Relative luminance below which a page ground counts as dark and takes the white ink.
const DARK_GROUND_LUMINANCE: f64 = 0.18;
/// A line whose ends differ by at most this many millimetres on one axis is axis aligned.
const AXIS_EPS_MM: f64 = 1e-9;

/// One grid line to draw, in document mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridStroke {
    /// One end.
    pub start: Point,
    /// The other end.
    pub end: Point,
    /// `0` minor to `1` major.
    pub emphasis: f32,
    /// Opacity, `0..=1`.
    pub alpha: f32,
}

/// The grid ink for a page background: dark ink on a light ground or a checkerboard, white ink
/// on a dark ground (UX notes §4).
#[must_use]
pub fn grid_ink(background: DocumentBackground) -> RgbaColor {
    if background.paint == BackgroundPaint::None {
        return theme::GRID_INK;
    }
    let linear = |channel: u8| {
        let c = f64::from(channel) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let c = background.color;
    let luminance = 0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b);
    // A translucent ground shows the pasteboard-light checkerboard through it.
    let opacity = background.opacity.get();
    let mixed = luminance * opacity + (1.0 - opacity);
    if mixed < DARK_GROUND_LUMINANCE {
        theme::GRID_INK_ON_DARK
    } else {
        theme::GRID_INK
    }
}

/// The colour of one line: the ink at the minor-to-major opacity, times the line's alpha.
fn line_colour(ink: RgbaColor, stroke: &GridStroke) -> RgbaColor {
    let emphasis = f64::from(stroke.emphasis.clamp(0.0, 1.0));
    let alpha = f64::from(stroke.alpha.clamp(0.0, 1.0));
    let level = theme::GRID_MINOR_OPACITY
        + (theme::GRID_MAJOR_OPACITY - theme::GRID_MINOR_OPACITY) * emphasis;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=255 by the clamps.
    let a = (level * alpha * 255.0).round() as u8;
    ink.with_alpha(a)
}

/// The indices of `coloured` ordered by opacity, strongest first: a counting sort over the 256
/// opacity bytes, linear in the number of lines (criterion 9's frame budget).
fn strongest_first(coloured: &[(RgbaColor, &GridStroke)]) -> Vec<usize> {
    let rank = |colour: RgbaColor| usize::from(u8::MAX - colour.a);
    let mut next = [0_usize; 257];
    for (colour, _) in coloured {
        next[rank(*colour) + 1] += 1;
    }
    for i in 1..next.len() {
        next[i] += next[i - 1];
    }
    let mut order = vec![0; coloured.len()];
    for (index, (colour, _)) in coloured.iter().enumerate() {
        let slot = &mut next[rank(*colour)];
        order[*slot] = index;
        *slot += 1;
    }
    order
}

/// The triangles of `strokes`, each a quad one device pixel wide at every zoom. An axis-aligned
/// line is centred on a device-pixel column or row so it never blurs over two. Stronger lines
/// come first: the artwork pass's depth test keeps the first fragment of a layer, so where two
/// lines cross a minor line never cuts a gap into a major one.
#[must_use]
pub fn build_grid_lines(
    strokes: &[GridStroke],
    ink: RgbaColor,
    view: ViewTransform,
    device_pixel_ratio: f64,
) -> DrawList {
    let mut list = DrawList::default();
    let dpr = if device_pixel_ratio > 0.0 {
        device_pixel_ratio
    } else {
        1.0
    };
    let half = 0.5 / (dpr * view.scale());
    let centre = |screen: f64| ((screen * dpr).floor() + 0.5) / dpr;
    let coloured: Vec<(RgbaColor, &GridStroke)> = strokes
        .iter()
        .map(|stroke| (line_colour(ink, stroke), stroke))
        .filter(|(colour, _)| colour.a > 0)
        .collect();
    list.triangles.reserve(coloured.len() * 6);
    for index in strongest_first(&coloured) {
        let (colour, stroke) = coloured[index];
        let (mut a, mut b) = (stroke.start, stroke.end);
        if (a.x - b.x).abs() <= AXIS_EPS_MM {
            let (sx, _) = view.document_to_screen(a);
            let x = view.screen_to_document(centre(sx), 0.0).x;
            a.x = x;
            b.x = x;
        } else if (a.y - b.y).abs() <= AXIS_EPS_MM {
            let (_, sy) = view.document_to_screen(a);
            let y = view.screen_to_document(0.0, centre(sy)).y;
            a.y = y;
            b.y = y;
        }
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let length = dx.hypot(dy);
        if length <= 0.0 {
            continue;
        }
        let (nx, ny) = (-dy / length * half, dx / length * half);
        let corners = [
            Point::new(a.x + nx, a.y + ny),
            Point::new(b.x + nx, b.y + ny),
            Point::new(b.x - nx, b.y - ny),
            Point::new(a.x - nx, a.y - ny),
        ];
        for index in [0, 1, 2, 0, 2, 3] {
            list.push_vertex(Vertex {
                position: corners[index],
                color: colour,
            });
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Color, Opacity};

    use super::*;

    fn ground(r: u8, g: u8, b: u8) -> DocumentBackground {
        DocumentBackground {
            paint: BackgroundPaint::Solid,
            color: Color { r, g, b },
            opacity: Opacity::new(1.0).unwrap(),
        }
    }

    #[test]
    fn the_ink_follows_the_page_ground() {
        assert_eq!(grid_ink(DocumentBackground::DEFAULT), theme::GRID_INK);
        assert_eq!(grid_ink(ground(255, 255, 255)), theme::GRID_INK);
        assert_eq!(grid_ink(ground(0, 0, 0)), theme::GRID_INK_ON_DARK);
        assert_eq!(grid_ink(ground(0x76, 0x76, 0x76)), theme::GRID_INK);
        assert_eq!(
            grid_ink(DocumentBackground {
                paint: BackgroundPaint::None,
                ..ground(0, 0, 0)
            }),
            theme::GRID_INK
        );
    }

    #[test]
    fn lines_are_one_device_pixel_wide_and_major_lines_are_stronger() {
        for (scale, dpr) in [(0.1, 1.0), (3.78, 2.0), (300.0, 1.5)] {
            let view = ViewTransform::new(scale, Point::new(0.0, 0.0));
            let strokes = [
                GridStroke {
                    start: Point::new(1.0, 0.0),
                    end: Point::new(1.0, 10.0),
                    emphasis: 0.0,
                    alpha: 1.0,
                },
                GridStroke {
                    start: Point::new(0.0, 2.0),
                    end: Point::new(10.0, 2.0),
                    emphasis: 1.0,
                    alpha: 1.0,
                },
            ];
            let list = build_grid_lines(&strokes, theme::GRID_INK, view, dpr);
            assert_eq!(list.triangles.len(), 12);
            // The major (horizontal) line comes first, the minor (vertical) one second.
            let xs: Vec<f64> = list.triangles[6..].iter().map(|v| v.position.x).collect();
            let width = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                - xs.iter().copied().fold(f64::INFINITY, f64::min);
            assert!((width * scale * dpr - 1.0).abs() < 1e-9);
            assert!(list.triangles[0].color.a > list.triangles[6].color.a);
        }
    }

    #[test]
    fn major_lines_come_before_minor_lines() {
        let line = |x: f64, emphasis: f32| GridStroke {
            start: Point::new(x, 0.0),
            end: Point::new(x, 10.0),
            emphasis,
            alpha: 1.0,
        };
        let list = build_grid_lines(
            &[line(1.0, 1.0), line(2.0, 0.0), line(3.0, 0.5)],
            theme::GRID_INK,
            ViewTransform::identity(),
            1.0,
        );
        let alphas: Vec<u8> = list
            .triangles
            .iter()
            .step_by(6)
            .map(|v| v.color.a)
            .collect();
        assert_eq!(alphas.len(), 3);
        assert!(alphas.windows(2).all(|pair| pair[0] > pair[1]));
    }

    #[test]
    fn a_line_at_zero_opacity_draws_nothing() {
        let list = build_grid_lines(
            &[GridStroke {
                start: Point::new(0.0, 0.0),
                end: Point::new(5.0, 5.0),
                emphasis: 1.0,
                alpha: 0.0,
            }],
            theme::GRID_INK,
            ViewTransform::identity(),
            1.0,
        );
        assert_eq!(list.triangles.len(), 0);
    }
}
