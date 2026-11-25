//! The machine state a G-code text leaves before one of its lines (`specs/0118` 2e, adrs R5).

use crate::gcode_words::{number, strip, words};

/// GRBL's millimetres per inch.
const INCH: f64 = 25.4;

/// The spindle (laser) word in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Spindle {
    /// `M5`.
    Off,
    /// `M3`, constant power.
    Constant,
    /// `M4`, dynamic power.
    Dynamic,
}

/// The modal state and the position before a line, in the job's own words.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // one flag per G-code group, each written on its own
pub(crate) struct Modal {
    /// The motion word: 0 to 3, or 80.
    pub(crate) motion: u8,
    /// The plane: 17, 18 or 19.
    pub(crate) plane: u8,
    /// `G91`.
    pub(crate) incremental: bool,
    /// `G91.1`.
    pub(crate) arc_incremental: bool,
    /// `G20`.
    pub(crate) inches: bool,
    /// `G93`.
    pub(crate) inverse_time: bool,
    /// The work system: 54 to 59.
    pub(crate) work: u8,
    /// The last `F`, in the job's units.
    pub(crate) feed: Option<f64>,
    /// The last `S`.
    pub(crate) power: f64,
    /// The spindle word.
    pub(crate) spindle: Spindle,
    /// `M7`.
    pub(crate) mist: bool,
    /// `M8`.
    pub(crate) flood: bool,
    /// X, Y and Z in work millimetres; `None` while the text has not given it.
    pub(crate) position: [Option<f64>; 3],
}

impl Default for Modal {
    /// GRBL's power-up state: `G0 G17 G90 G21 G94 G54 M5 M9`, no `F`, `S0`.
    fn default() -> Self {
        Self {
            motion: 0,
            plane: 17,
            incremental: false,
            arc_incremental: false,
            inches: false,
            inverse_time: false,
            work: 54,
            feed: None,
            power: 0.0,
            spindle: Spindle::Off,
            mist: false,
            flood: false,
            position: [None; 3],
        }
    }
}

/// Why the state before a line cannot be rebuilt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Unrebuildable {
    /// A word the rebuild does not know, as the text spells it.
    Word(String),
    /// X or Y before the line is not given by the text (a job run from the head position).
    HeadPosition,
}

impl Unrebuildable {
    /// The cause in words for a restart from `line`.
    pub(crate) fn text(&self, line: u32) -> String {
        match self {
            Self::Word(word) => format!(
                "Cannot rebuild the state before line {}: {word}",
                crate::count::count(line)
            ),
            Self::HeadPosition => "Starts at the head position".to_owned(),
        }
    }
}

/// A G or M number times ten, when it is one (`G91.1` is 911).
fn code(value: &str) -> Option<u16> {
    let tenths = number(value)? * 10.0;
    let whole = tenths.round();
    let plain = (tenths - whole).abs() < 1e-9 && (0.0..=9_999.0).contains(&whole);
    plain.then(|| format!("{whole:.0}").parse().ok()).flatten()
}

/// The word as the stripped line spells it, for the cause.
fn spelled(letter: char, value: &str) -> String {
    format!("{letter}{value}")
}

impl Modal {
    /// Applies one G word; `Err` for a word outside the groups the rebuild keeps.
    fn g(&mut self, value: &str) -> Result<(), ()> {
        match code(value).ok_or(())? {
            0 => self.motion = 0,
            10 => self.motion = 1,
            20 => self.motion = 2,
            30 => self.motion = 3,
            800 => self.motion = 80,
            170 => self.plane = 17,
            180 => self.plane = 18,
            190 => self.plane = 19,
            900 => self.incremental = false,
            910 => self.incremental = true,
            911 => self.arc_incremental = true,
            200 => self.inches = true,
            210 => self.inches = false,
            930 => self.inverse_time = true,
            940 => self.inverse_time = false,
            c @ (540 | 550 | 560 | 570 | 580 | 590) => {
                self.work = u8::try_from(c / 10).map_err(|_| ())?;
            }
            40 | 400 | 610 => {}
            _ => return Err(()),
        }
        Ok(())
    }

    /// Applies one M word.
    fn m(&mut self, value: &str) -> Result<(), ()> {
        match code(value).ok_or(())? {
            30 => self.spindle = Spindle::Constant,
            40 => self.spindle = Spindle::Dynamic,
            50 => self.spindle = Spindle::Off,
            70 => self.mist = true,
            80 => self.flood = true,
            90 => {
                self.mist = false;
                self.flood = false;
            }
            0 | 10 => {}
            _ => return Err(()),
        }
        Ok(())
    }

    /// Applies one stripped, non-empty line.
    fn line(&mut self, line: &str) -> Result<(), Unrebuildable> {
        let refuse = |word: String| Unrebuildable::Word(word);
        if line.starts_with('$') {
            return if line == "$H" {
                Ok(())
            } else {
                Err(refuse(line.to_owned()))
            };
        }
        let words = words(line).ok_or_else(|| refuse(line.to_owned()))?;
        let mut axes: [Option<f64>; 3] = [None; 3];
        for (letter, value) in words {
            let word = || spelled(letter, value);
            match letter {
                'G' => self.g(value).map_err(|()| refuse(word()))?,
                'M' => self.m(value).map_err(|()| refuse(word()))?,
                'N' => {}
                'X' | 'Y' | 'Z' => {
                    let at = match letter {
                        'X' => 0,
                        'Y' => 1,
                        _ => 2,
                    };
                    axes[at] = Some(number(value).ok_or_else(|| refuse(word()))?);
                }
                'F' => self.feed = Some(number(value).ok_or_else(|| refuse(word()))?),
                'S' => self.power = number(value).ok_or_else(|| refuse(word()))?,
                'I' | 'J' | 'K' | 'R' | 'P' => {
                    number(value).ok_or_else(|| refuse(word()))?;
                }
                _ => return Err(refuse(word())),
            }
        }
        let scale = if self.inches { INCH } else { 1.0 };
        for (slot, axis) in self.position.iter_mut().zip(axes) {
            let Some(value) = axis.map(|v| v * scale) else {
                continue;
            };
            *slot = if self.incremental {
                slot.map(|known| known + value)
            } else {
                Some(value)
            };
        }
        Ok(())
    }
}

/// The state lines 1 to `line - 1` of `text` leave: comments, blank lines, `N` words and a `$H`
/// line are skipped, the last word of each group wins.
pub(crate) fn before(text: &str, line: u32) -> Result<Modal, Unrebuildable> {
    let mut modal = Modal::default();
    let take = usize::try_from(line.saturating_sub(1)).unwrap_or(usize::MAX);
    for raw in text.lines().take(take) {
        let stripped = strip(raw);
        if !stripped.is_empty() {
            modal.line(&stripped)?;
        }
    }
    if modal.position[0].is_none() || modal.position[1].is_none() {
        return Err(Unrebuildable::HeadPosition);
    }
    if matches!(modal.motion, 2 | 3) {
        // GRBL refuses `G2`/`G3` without axis words, so the arc mode cannot be set on its own:
        // line n has to name its motion (Proposal).
        let at = text.lines().nth(take).map(strip).unwrap_or_default();
        let named = words(&at).is_some_and(|w| {
            w.iter()
                .any(|(l, v)| *l == 'G' && matches!(code(v), Some(0 | 10 | 20 | 30)))
        });
        if !named {
            return Err(Unrebuildable::Word(format!("G{}", modal.motion)));
        }
    }
    Ok(modal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(w: &str) -> Result<Modal, Unrebuildable> {
        Err(Unrebuildable::Word(w.to_owned()))
    }

    #[test]
    fn the_last_word_of_each_group_wins() {
        let text = "G21\nG90\nG17\nM4 S0\nG0 X20 Y20\nM8\nG1 X120 Y20 F600 S500\nG18\nM7\nG55\nG93\nG94\nG2 X1 Y2 I1 J0\nG2 X5 Y5 I1 J0\n";
        let modal = before(text, 14).unwrap();
        assert_eq!(modal.motion, 2);
        assert_eq!(modal.plane, 18);
        assert_eq!(modal.work, 55);
        assert!(!modal.inverse_time);
        assert_eq!(modal.feed, Some(600.0));
        assert!((modal.power - 500.0).abs() < 1e-9);
        assert_eq!(modal.spindle, Spindle::Dynamic);
        assert!(modal.mist && modal.flood);
        assert_eq!(modal.position, [Some(1.0), Some(2.0), None]);
        let off = before("G0 X1 Y1\nM7 M8\nM9\nM5\nG91.1\nX", 6).unwrap();
        assert!(!off.mist && !off.flood);
        assert_eq!(off.spindle, Spindle::Off);
        assert!(off.arc_incremental);
    }

    #[test]
    fn positions_follow_units_and_distance_mode() {
        let inch = before("G20\nG90\nG0 X1 Y2 Z0.5\nG91\nG1 X1 F10\nX", 6).unwrap();
        let mm = inch
            .position
            .map(|p| p.map(|v| (v * 1000.0).round() / 1000.0));
        assert_eq!(mm, [Some(50.8), Some(50.8), Some(12.7)]);
        assert!(inch.inches && inch.incremental);
        assert_eq!(
            before("G21\nG91\nG1 X10 Y0 F600\nX", 4),
            Err(Unrebuildable::HeadPosition),
            "incremental from an unknown head stays unknown"
        );
        assert_eq!(
            before("G90\nG0 X1\nG1 Y1\n", 3),
            Err(Unrebuildable::HeadPosition)
        );
        assert!(before("G90\nG0 X1\nG1 Y1\n", 4).is_ok());
    }

    #[test]
    fn comments_blank_lines_numbers_and_home_are_skipped() {
        let text = "; a\n(b)\n\n$H\nN10 G0 X1 Y1 (c) ; d\ng1 x2 y3 f100\nX";
        let modal = before(text, 7).unwrap();
        assert_eq!(modal.position, [Some(2.0), Some(3.0), None]);
        assert_eq!(modal.motion, 1);
        assert!(before("G4 P0.5\nG40\nG61\nM0\nM1\nG0X0Y0\nX", 7).is_ok());
    }

    #[test]
    fn words_outside_the_groups_are_refused_first_one_wins() {
        let head = "G0 X0 Y0\n";
        let table = [
            ("G92 X0", "G92"),
            ("G10 L20 P1 X0", "G10"),
            ("G28", "G28"),
            ("G30", "G30"),
            ("G38.2 Z-5", "G38.2"),
            ("G43.1 Z1", "G43.1"),
            ("G53 G0 X0", "G53"),
            ("M2", "M2"),
            ("M30", "M30"),
            ("A10", "A10"),
            ("T1", "T1"),
            ("$J=X1", "$J=X1"),
            ("$X", "$X"),
            ("G1 X#1", "X#1"),
            ("G1 X1e3", "E3"),
            ("M5 G28", "G28"),
            ("%", "%"),
        ];
        for (line, cause) in table {
            assert_eq!(
                before(&format!("{head}{line}\nG92\nX"), 4),
                word(cause),
                "{line}"
            );
        }
        assert_eq!(
            Unrebuildable::Word("G28".into()).text(1234),
            "Cannot rebuild the state before line 1,234: G28"
        );
        assert_eq!(
            Unrebuildable::HeadPosition.text(5),
            "Starts at the head position"
        );
    }

    #[test]
    fn an_arc_mode_needs_line_n_to_name_its_motion() {
        let text = "G0 X0 Y0\nG2 X10 Y0 I5 J0 F600\nX20 Y0 I5 J0\nG3 X30 Y0 I5 J0\n";
        assert_eq!(before(text, 3), word("G2"));
        assert!(before(text, 4).is_ok());
    }

    #[test]
    fn a_refused_word_at_or_after_the_line_does_not_matter() {
        assert!(before("G0 X1 Y1\nG28\n", 2).is_ok());
        assert!(before("G0 X1 Y1\nG1 X2\nM2\n", 3).is_ok());
    }
}
