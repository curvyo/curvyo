//! The lines a frame may hold (`specs/0118` 2b, adrs F3): a closed list of words, so a frame can
//! never turn the laser on, and the check of its targets against the travel.

use crate::gcode_words::{number, strip, words};
use crate::jog::{EDGE, TravelBox};
use crate::millimetres::Millimetres;

/// A frame line the connector refuses, counted from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotAFrame {
    /// The line.
    pub line: u32,
}

/// One `G1` of a frame: the distance mode it runs in and the X and Y it names.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Target {
    relative: bool,
    x: Option<f64>,
    y: Option<f64>,
}

/// The moves of a checked frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameMoves {
    targets: Vec<Target>,
}

/// What the lines before this one switched on.
#[derive(Default)]
struct Modal {
    millimetres: bool,
    mode: Option<bool>,
    laser_off: bool,
}

/// Checks `text` line by line.
///
/// # Errors
/// [`NotAFrame`] with the first line that holds a word outside the list: `G21`, `G90`, `G91`,
/// `G17`, `M5`, `M9` and `G1` with `X`, `Y`, `F` above 0 and `S0`; `G21` and a distance mode
/// before the first `G1`, and `M5` before it.
pub fn check(text: &str) -> Result<FrameMoves, NotAFrame> {
    let mut modal = Modal::default();
    let mut targets = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let refused = NotAFrame {
            line: u32::try_from(i + 1).unwrap_or(u32::MAX),
        };
        let line = strip(raw);
        let words = words(&line).ok_or(refused)?;
        let mut motion = false;
        let (mut x, mut y, mut feed, mut s) = (None, None, None, None);
        for (letter, value) in &words {
            let given = |slot: &mut Option<f64>| -> Result<(), NotAFrame> {
                let n = number(value).ok_or(refused)?;
                slot.replace(n).map_or(Ok(()), |_| Err(refused))
            };
            match (*letter, *value) {
                ('G', "21") => modal.millimetres = true,
                ('G', "90") => modal.mode = Some(false),
                ('G', "91") => modal.mode = Some(true),
                ('G', "17") | ('M', "9") => {}
                ('M', "5") => modal.laser_off = true,
                ('G', "1") if !motion => motion = true,
                ('X', _) => given(&mut x)?,
                ('Y', _) => given(&mut y)?,
                ('F', _) => given(&mut feed)?,
                ('S', _) => given(&mut s)?,
                _ => return Err(refused),
            }
        }
        if !motion {
            if x.or(y).or(feed).or(s).is_some() {
                return Err(refused);
            }
            continue;
        }
        let ready = modal.millimetres && modal.laser_off;
        let moves = x.or(y).is_some() && feed.is_some_and(|f| f > 0.0) && s == Some(0.0);
        let Some(relative) = modal.mode.filter(|_| ready && moves) else {
            return Err(refused);
        };
        targets.push(Target { relative, x, y });
    }
    Ok(FrameMoves { targets })
}

impl FrameMoves {
    /// The number of `G1` moves.
    #[must_use]
    pub fn len(&self) -> usize {
        self.targets.len()
    }

    /// Whether the frame moves nowhere.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// The machine position after each move, starting from `mpos` with the work offset `wco`
    /// (a word a move leaves out keeps its axis where it was).
    #[must_use]
    pub fn points(&self, mpos: [Millimetres; 2], wco: [Millimetres; 2]) -> Vec<[Millimetres; 2]> {
        let mut at = [mpos[0].0, mpos[1].0];
        let mut points = Vec::with_capacity(self.targets.len());
        for target in &self.targets {
            for (i, word) in [target.x, target.y].into_iter().enumerate() {
                if let Some(value) = word {
                    at[i] = if target.relative {
                        at[i] + value
                    } else {
                        value + wco[i].0
                    };
                }
            }
            points.push([Millimetres(at[0]), Millimetres(at[1])]);
        }
        points
    }

    /// Whether every target lies within `within` (machine coordinates), starting from the machine
    /// position `mpos` with the work offset `wco`; the way between two targets stays inside a box
    /// that holds both. `Err` carries the largest distance by which a target misses it.
    ///
    /// # Errors
    /// The overshoot in millimetres beyond the `EDGE` tolerance.
    pub fn fits(
        &self,
        within: &TravelBox,
        mpos: [Millimetres; 2],
        wco: [Millimetres; 2],
    ) -> Result<(), Millimetres> {
        let mut worst = 0.0_f64;
        for point in self.points(mpos, wco) {
            for (i, value) in point.iter().enumerate() {
                worst = worst
                    .max(within.min[i] - value.0)
                    .max(value.0 - within.max[i]);
            }
        }
        if worst > EDGE {
            Err(Millimetres(worst))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: &str = "; Curvyo frame: a\nG21\nG90\nG17\nM5\nM9\nG1 X10.000 Y10.000 F3000 S0\nG1 X20.000 Y10.000 F3000 S0\n";

    #[test]
    fn a_frame_of_the_writer_passes() {
        assert_eq!(check(FRAME).unwrap().len(), 2);
        assert!(check("").unwrap().is_empty());
    }

    #[test]
    fn words_outside_the_list_name_their_line() {
        let head = "G21\nG90\nM5\n";
        let table = [
            ("M3 S0", 4),
            ("M4 S0", 4),
            ("M8", 4),
            ("G0 X1 Y1", 4),
            ("G1 X1 Y1 F100 S1", 4),
            ("G1 X1 Y1 F100", 4),
            ("G1 X1 Y1 S0", 4),
            ("G1 X1 Y1 F0 S0", 4),
            ("G1 X1 Y1 F-5 S0", 4),
            ("G1 Z5 F100 S0", 4),
            ("G1 F100 S0", 4),
            ("G1 X1 X2 Y1 F100 S0", 4),
            ("G1 X#1 Y1 F100 S0", 4),
            ("G1 X1e3 Y1 F100 S0", 4),
            ("$H", 4),
            ("$32=0", 4),
            ("G10 L20 P1 X0", 4),
            ("G92 X0", 4),
            ("G54", 4),
            ("G2 X1 Y1 I1 F100 S0", 4),
            ("S100", 4),
            ("X5", 4),
            ("M5 M3", 4),
        ];
        for (line, at) in table {
            assert_eq!(
                check(&format!("{head}{line}\n")),
                Err(NotAFrame { line: at }),
                "{line}"
            );
        }
    }

    #[test]
    fn units_a_distance_mode_and_the_laser_off_come_before_the_first_move() {
        let go = "G1 X1 Y1 F100 S0\n";
        for before in ["", "G21\nG90\n", "G90\nM5\n", "G21\nM5\n"] {
            assert_eq!(
                check(&format!("{before}{go}")),
                Err(NotAFrame {
                    line: u32::try_from(before.lines().count() + 1).unwrap()
                }),
                "{before:?}"
            );
        }
        assert!(check(&format!("G21\nG91\nM5\n{go}")).is_ok());
        assert!(check("G21 G90 M5\nG1 X1 Y1 F100 S0.0\n").is_ok());
    }

    #[test]
    fn comments_hide_nothing_and_lower_case_is_read() {
        assert!(check("g21\ng90\nm5\ng1 x1 y1 f100 s0 ; M3 S900\n(M3) g1 x2 y2 f100 s0\n").is_ok());
        assert_eq!(
            check("G21\nG90\nM5\nG1 X1 Y1 F100 S0 (note\nM3\n"),
            Err(NotAFrame { line: 5 }),
            "an unclosed comment ends with its line"
        );
        assert!(check("G21\nG90\nM5\nG1 X1 Y1 F100 S0 (note) M3\n").is_err());
    }

    fn boxed() -> TravelBox {
        TravelBox::new([400.0, 300.0], [false, false])
    }

    fn mm(x: f64, y: f64) -> [Millimetres; 2] {
        [Millimetres(x), Millimetres(y)]
    }

    #[test]
    fn every_corner_of_the_travel_is_in_and_a_hair_beyond_is_out() {
        // The machine box is [-400, 0] x [-300, 0]; work (0, 0) is machine (-400, -300).
        let wco = mm(-400.0, -300.0);
        let frame =
            |x: f64, y: f64| check(&format!("G21\nG90\nM5\nG1 X{x} Y{y} F100 S0\n")).unwrap();
        for (x, y) in [(0.0, 0.0), (400.0, 0.0), (0.0, 300.0), (400.0, 300.0)] {
            assert_eq!(
                frame(x, y).fits(&boxed(), mm(-200.0, -100.0), wco),
                Ok(()),
                "{x} {y}"
            );
        }
        for (x, y, over) in [
            (-0.5, 0.0, 0.5),
            (400.5, 0.0, 0.5),
            (0.0, -2.0, 2.0),
            (0.0, 303.0, 3.0),
        ] {
            let miss = frame(x, y)
                .fits(&boxed(), mm(-200.0, -100.0), wco)
                .unwrap_err();
            assert!((miss.0 - over).abs() < 1e-9, "{x} {y}: {miss:?}");
        }
        // Within the 0.01 mm tolerance of an edge.
        assert_eq!(
            frame(400.005, 0.0).fits(&boxed(), mm(-200.0, -100.0), wco),
            Ok(())
        );
    }

    #[test]
    fn relative_moves_add_up_from_the_head() {
        let frame =
            check("G21\nG91\nM5\nG1 X-50 Y0 F100 S0\nG1 X0 Y-40 F100 S0\nG1 X100 Y0 F100 S0\n")
                .unwrap();
        let wco = mm(0.0, 0.0);
        assert_eq!(frame.fits(&boxed(), mm(-100.0, -45.0), wco), Ok(()));
        let miss = frame.fits(&boxed(), mm(-100.0, -270.0), wco);
        assert_eq!(
            miss.map_err(|m| m.0.round()),
            Err(10.0),
            "Y ends 10 mm below -300"
        );
        let past = frame.fits(&boxed(), mm(-20.0, -45.0), wco);
        assert_eq!(
            past.map_err(|m| m.0.round()),
            Err(30.0),
            "X ends 30 mm beyond 0"
        );
    }
}
