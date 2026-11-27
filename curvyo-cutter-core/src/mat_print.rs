//! The printing of a mat as drawn with Mat grid on (`specs/0159-cutting-plotter` criterion 5,
//! `ux-notes.md` 5.2): grid, centre cross, cm rulers, inch numbers and the grip word, in page mm.

use curvyo_document_core::Point;

use crate::mat::{Grip, INCH_MM, MatSize, SHEET_END_MM, SHEET_SIDE_MM};

/// The width of a grid line.
pub const GRID_LINE_MM: f64 = 0.6;
/// The width of the centre cross's arms.
pub const CROSS_LINE_MM: f64 = 1.2;
/// The length of each arm of the centre cross from the centre.
pub const CROSS_ARM_MM: f64 = 9.0;
/// The length of a cm tick outward from the field edge.
pub const CM_TICK_MM: f64 = 2.5;
/// The length of an inch tick past the field edge.
pub const INCH_TICK_MM: f64 = 2.0;
/// The height of the printed digits and the grip word.
pub const TEXT_MM: f64 = 3.0;

/// One printed line from `a` to `b`, `width` mm wide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrintLine {
    /// The start.
    pub a: Point,
    /// The end.
    pub b: Point,
    /// The width in mm.
    pub width_mm: f64,
}

/// One printed text centred on `at`, [`TEXT_MM`] high, upright or turned 180 degrees.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintText {
    /// The centre.
    pub at: Point,
    /// The text.
    pub text: String,
    /// Turned 180 degrees, as the inch numbers and the second grip word are printed.
    pub turned: bool,
}

/// The printing of one mat.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MatPrint {
    /// The grid, the ticks and the cross.
    pub lines: Vec<PrintLine>,
    /// The numbers and the grip words.
    pub texts: Vec<PrintText>,
}

/// The printing of `size` in `grip`: everything Mat grid on adds (criterion 5).
#[must_use]
pub fn mat_print(size: MatSize, grip: Grip) -> MatPrint {
    let field = size.field();
    let (w_in, h_in) = size.inches();
    let mut print = MatPrint::default();
    let line = |a: Point, b: Point, width_mm: f64| PrintLine { a, b, width_mm };
    for i in 0..=w_in {
        let x = f64::from(i) * INCH_MM;
        print.lines.push(line(
            Point::new(x, 0.0),
            Point::new(x, field.h),
            GRID_LINE_MM,
        ));
    }
    for j in 0..=h_in {
        let y = f64::from(j) * INCH_MM;
        print.lines.push(line(
            Point::new(0.0, y),
            Point::new(field.w, y),
            GRID_LINE_MM,
        ));
    }
    let c = size.centre();
    print.lines.push(line(
        Point::new(c.x - CROSS_ARM_MM, c.y),
        Point::new(c.x + CROSS_ARM_MM, c.y),
        CROSS_LINE_MM,
    ));
    print.lines.push(line(
        Point::new(c.x, c.y - CROSS_ARM_MM),
        Point::new(c.x, c.y + CROSS_ARM_MM),
        CROSS_LINE_MM,
    ));
    cm_rulers(&mut print, field.w, field.h);
    inch_numbers(&mut print, size);
    print.texts.push(PrintText {
        at: Point::new(field.w - 25.0, -SHEET_END_MM / 2.0),
        text: grip.word().to_owned(),
        turned: false,
    });
    print.texts.push(PrintText {
        at: Point::new(25.0, field.h + SHEET_END_MM / 2.0 - 8.0),
        text: grip.word().to_owned(),
        turned: true,
    });
    print
}

/// The cm ticks and numbers along the top and left edges, counted from the mat origin.
fn cm_rulers(print: &mut MatPrint, width: f64, height: f64) {
    let text_off = CM_TICK_MM + 1.0 + TEXT_MM / 2.0;
    for k in 1..=whole_cm(width) {
        let x = f64::from(k) * 10.0;
        print.lines.push(PrintLine {
            a: Point::new(x, 0.0),
            b: Point::new(x, -CM_TICK_MM),
            width_mm: GRID_LINE_MM,
        });
        print.texts.push(PrintText {
            at: Point::new(x, -text_off),
            text: k.to_string(),
            turned: false,
        });
    }
    for k in 1..=whole_cm(height) {
        let y = f64::from(k) * 10.0;
        print.lines.push(PrintLine {
            a: Point::new(0.0, y),
            b: Point::new(-CM_TICK_MM, y),
            width_mm: GRID_LINE_MM,
        });
        print.texts.push(PrintText {
            at: Point::new(-text_off.min(SHEET_SIDE_MM - TEXT_MM), y),
            text: k.to_string(),
            turned: false,
        });
    }
}

/// The inch ticks and numbers along the right and bottom edges, counted from the bottom-right
/// corner and turned 180 degrees (UX C7).
fn inch_numbers(print: &mut MatPrint, size: MatSize) {
    let field = size.field();
    let (w_in, h_in) = size.inches();
    let text_off = INCH_TICK_MM + 1.0 + TEXT_MM / 2.0;
    for k in 1..=h_in {
        let y = field.h - f64::from(k) * INCH_MM;
        print.lines.push(PrintLine {
            a: Point::new(field.w, y),
            b: Point::new(field.w + INCH_TICK_MM, y),
            width_mm: GRID_LINE_MM,
        });
        print.texts.push(PrintText {
            at: Point::new(field.w + text_off, y),
            text: k.to_string(),
            turned: true,
        });
    }
    for k in 1..=w_in {
        let x = field.w - f64::from(k) * INCH_MM;
        print.lines.push(PrintLine {
            a: Point::new(x, field.h),
            b: Point::new(x, field.h + INCH_TICK_MM),
            width_mm: GRID_LINE_MM,
        });
        print.texts.push(PrintText {
            at: Point::new(x, field.h + text_off),
            text: k.to_string(),
            turned: true,
        });
    }
}

/// The whole centimetres in `mm`.
fn whole_cm(mm: f64) -> u32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // invariant: a mat side is a few hundred mm, positive
    let cm = (mm / 10.0 + 1e-9).floor() as u32;
    cm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_12x12_print_has_its_grid_cross_rulers_and_numbers() {
        let print = mat_print(MatSize::In12x12, Grip::Standard);
        let grid = print
            .lines
            .iter()
            .filter(|l| l.width_mm == GRID_LINE_MM && l.a.y == 0.0 && l.b.y == 304.8)
            .count();
        assert_eq!(grid, 13, "12 cells, border included");
        let cross: Vec<&PrintLine> = print
            .lines
            .iter()
            .filter(|l| l.width_mm == CROSS_LINE_MM)
            .collect();
        assert_eq!(cross.len(), 2);
        let mid = |l: &PrintLine| (f64::midpoint(l.a.x, l.b.x), f64::midpoint(l.a.y, l.b.y));
        for arm in cross {
            let (x, y) = mid(arm);
            assert!((x - 152.4).abs() <= 0.01 && (y - 152.4).abs() <= 0.01);
        }
        let numbers = |turned: bool| {
            print
                .texts
                .iter()
                .filter(|t| t.turned == turned && t.text.parse::<u32>().is_ok())
                .count()
        };
        assert_eq!(numbers(false), 60, "cm 1 to 30 on top and left");
        assert_eq!(numbers(true), 24, "inch 1 to 12 on right and bottom");
        assert!(print.texts.iter().any(|t| t.text == "StandardGrip"));
    }

    #[test]
    fn the_long_mat_counts_to_24_inches_and_60_cm() {
        let print = mat_print(MatSize::In12x24, Grip::Light);
        assert!(print.texts.iter().any(|t| t.turned && t.text == "24"));
        assert!(print.texts.iter().any(|t| !t.turned && t.text == "60"));
        assert!(!print.texts.iter().any(|t| t.text == "61"));
    }
}
