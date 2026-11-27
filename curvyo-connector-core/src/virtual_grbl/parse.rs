//! How the virtual GRBL reads one G-code line (`specs/0161-cnc-output` criterion 16): the words
//! GRBL 1.1 takes, the modal state they change, and the error number of what it refuses.

use super::codes;

/// The spindle as `M3`, `M4` and `M5` leave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Spindle {
    /// `M5`.
    #[default]
    Off,
    /// `M3`, clockwise.
    Cw,
    /// `M4`, counter-clockwise.
    Ccw,
}

/// The parser's modal state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Modal {
    /// The motion mode is `G0`.
    pub rapid: bool,
    /// `G90` (else `G91`).
    pub absolute: bool,
    /// `G20` (else `G21`).
    pub inches: bool,
    /// The feed rate in mm/min; `None` until an `F` word.
    pub feed: Option<f64>,
    /// The spindle.
    pub spindle: Spindle,
    /// The spindle speed `S`.
    pub rpm: f64,
}

impl Default for Modal {
    /// GRBL's power-up state: `G0 G90 G21 M5`, no feed.
    fn default() -> Self {
        Self {
            rapid: true,
            absolute: true,
            inches: false,
            feed: None,
            spindle: Spindle::Off,
            rpm: 0.0,
        }
    }
}

/// What one line asks the machine to do, in order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    /// A move to `target` (machine mm): a rapid, or a feed at `feed_mm_min`.
    Move {
        /// The end point.
        target: [f64; 3],
        /// `G0`.
        rapid: bool,
        /// The feed of a `G1`.
        feed_mm_min: f64,
    },
    /// The spindle set.
    Spindle(Spindle, f64),
    /// `G4 P<seconds>`.
    Dwell(f64),
}

fn words(line: &str) -> Result<Vec<(char, f64)>, u16> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let letter = chars[i];
        if !letter.is_ascii_uppercase() {
            return Err(codes::EXPECTED_COMMAND_LETTER);
        }
        i += 1;
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_digit() || matches!(chars[i], '.' | '-' | '+'))
        {
            i += 1;
        }
        let text: String = chars[start..i].iter().collect();
        let value = text.parse::<f64>().map_err(|_| codes::BAD_NUMBER_FORMAT)?;
        out.push((letter, value));
    }
    Ok(out)
}

/// The G codes GRBL 1.1 has (no canned cycles, no cutter compensation, no tool length offset
/// beyond `G49`).
const G_CODES: &[i64] = &[
    0, 1, 2, 3, 4, 17, 18, 19, 20, 21, 40, 49, 54, 80, 90, 91, 94,
];
/// The M codes GRBL 1.1 has (no `M6`).
const M_CODES: &[i64] = &[0, 1, 2, 3, 4, 5, 7, 8, 9, 30];

/// A G or M number as a whole code; `None` for a fraction (`G38.2` and the like are not had).
fn code(value: f64) -> Option<i64> {
    // invariant: a parsed number checked to be whole and small before the cast
    #[allow(clippy::cast_possible_truncation)]
    let whole = (value.fract() == 0.0 && value.abs() < 1e6).then_some(value as i64);
    whole
}
/// The value words GRBL 1.1 takes.
const VALUE_WORDS: &str = "FIJKNPRSTXYZ";

/// The actions of the stripped, upper-case `line` from `pos` (machine mm), the modal state
/// updated.
///
/// # Errors
/// The GRBL 1.1 error number of a line GRBL refuses; the modal state is then unchanged.
pub fn parse_line(line: &str, modal: &mut Modal, pos: [f64; 3]) -> Result<Vec<Action>, u16> {
    let mut next = *modal;
    let mut seen = String::new();
    let mut motion: Option<bool> = None;
    let mut axes: [Option<f64>; 3] = [None; 3];
    let (mut dwell, mut seconds, mut spindle_word) = (false, 0.0, None);
    for (letter, value) in words(line)? {
        match letter {
            'G' => {
                let g = code(value)
                    .filter(|g| G_CODES.contains(g))
                    .ok_or(codes::UNSUPPORTED_COMMAND)?;
                match g {
                    0..=3 => {
                        if motion.is_some() {
                            return Err(codes::MODAL_GROUP_VIOLATION);
                        }
                        motion = Some(g == 0);
                    }
                    4 => dwell = true,
                    20 => next.inches = true,
                    21 => next.inches = false,
                    90 => next.absolute = true,
                    91 => next.absolute = false,
                    _ => {}
                }
            }
            'M' => {
                let m = code(value)
                    .filter(|m| M_CODES.contains(m))
                    .ok_or(codes::UNSUPPORTED_COMMAND)?;
                spindle_word = match m {
                    3 => Some(Spindle::Cw),
                    4 => Some(Spindle::Ccw),
                    2 | 5 | 30 => Some(Spindle::Off),
                    _ => spindle_word,
                };
            }
            l if VALUE_WORDS.contains(l) => {
                if seen.contains(l) {
                    return Err(codes::REPEATED_WORD);
                }
                seen.push(l);
                let scale = if next.inches { 25.4 } else { 1.0 };
                match l {
                    'F' => next.feed = Some(value * scale),
                    'S' => next.rpm = value,
                    'P' => seconds = value,
                    'X' => axes[0] = Some(value * scale),
                    'Y' => axes[1] = Some(value * scale),
                    'Z' => axes[2] = Some(value * scale),
                    _ => {}
                }
            }
            _ => return Err(codes::UNSUPPORTED_COMMAND),
        }
    }
    if let Some(rapid) = motion {
        next.rapid = rapid;
    }
    let moves = axes.iter().any(Option::is_some) && !dwell;
    if moves && !next.rapid && next.feed.is_none_or(|f| f <= 0.0) {
        return Err(codes::UNDEFINED_FEED_RATE);
    }
    let mut actions = Vec::new();
    if let Some(s) = spindle_word {
        next.spindle = s;
    }
    if spindle_word.is_some() || (next.rpm - modal.rpm).abs() > 0.0 {
        actions.push(Action::Spindle(next.spindle, next.rpm));
    }
    if dwell {
        actions.push(Action::Dwell(seconds));
    }
    if moves {
        let mut target = pos;
        for (axis, value) in axes.iter().enumerate() {
            if let Some(v) = value {
                target[axis] = if next.absolute { *v } else { pos[axis] + v };
            }
        }
        actions.push(Action::Move {
            target,
            rapid: next.rapid,
            feed_mm_min: next.feed.unwrap_or(0.0),
        });
    }
    *modal = next;
    Ok(actions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(line: &str) -> Result<Vec<Action>, u16> {
        parse_line(line, &mut Modal::default(), [0.0; 3])
    }

    #[test]
    fn grbl_1_1_refuses_tool_changes_canned_cycles_and_feedless_moves() {
        assert_eq!(run("M6T1"), Err(20));
        assert_eq!(run("G81X1Y1Z-1R1F100"), Err(20));
        assert_eq!(run("G43H1"), Err(20));
        assert_eq!(run("G1X10"), Err(22));
        assert_eq!(run("G0G1X1"), Err(21));
        assert_eq!(run("G1X1X2F100"), Err(25));
        assert_eq!(run("1X"), Err(1));
        assert_eq!(run("GX"), Err(2));
    }

    #[test]
    fn moves_and_the_spindle_follow_the_modal_state() {
        let mut modal = Modal::default();
        assert_eq!(
            parse_line("M3S12000", &mut modal, [0.0; 3]),
            Ok(vec![Action::Spindle(Spindle::Cw, 12_000.0)])
        );
        let a = parse_line("G1X10Y5F800", &mut modal, [0.0; 3]).unwrap();
        assert_eq!(
            a,
            [Action::Move {
                target: [10.0, 5.0, 0.0],
                rapid: false,
                feed_mm_min: 800.0
            }]
        );
        let a = parse_line("Z-1", &mut modal, [10.0, 5.0, 0.0]).unwrap();
        assert!(matches!(
            a[0],
            Action::Move {
                target: [10.0, 5.0, -1.0],
                rapid: false,
                ..
            }
        ));
    }
}
