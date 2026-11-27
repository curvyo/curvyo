//! Turns a job's text into the lines GRBL receives and checks all of them before the first byte
//! is written (`specs/0127` G5): comments, spaces and carriage returns removed, real-time
//! characters, control and non-ASCII characters, long lines, setting writes and unlocks refused.

/// GRBL's line buffer holds 80 bytes with the newline.
pub const MAX_LINE: usize = 79;

/// Why a prepared line is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// `!`, `~` or `?` outside a comment.
    RealTime,
    /// Longer than [`MAX_LINE`].
    TooLong,
    /// `$<n>=<value>`, `$RST`, `$N`, `$I=`.
    ChangesSetting,
    /// `$X`.
    Unlocks,
    /// A character outside `0x20` to `0x7E` (the job's own ASCII check comes first).
    NotPrintable,
}

impl Cause {
    /// The words after `"Failed at line <n>: "`.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Self::RealTime => "Contains !, ~ or ?",
            Self::TooLong => "Longer than 79 characters",
            Self::ChangesSetting => "Changes a machine setting",
            Self::Unlocks => "Unlocks the machine",
            Self::NotPrintable => "Not plain ASCII text",
        }
    }
}

/// The first refused line, counted from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refused {
    /// The job line.
    pub line: u32,
    /// Why.
    pub cause: Cause,
}

/// One line without its comments, spaces and carriage return; empty for a line with nothing
/// left. An unclosed `(` comments out the rest of the line.
#[must_use]
pub fn strip(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_paren = false;
    for c in line.chars() {
        if in_paren {
            in_paren = c != ')';
            continue;
        }
        match c {
            '(' => in_paren = true,
            ';' => break,
            ' ' | '\t' | '\r' => {}
            other => out.push(other),
        }
    }
    out
}

fn check(line: &str) -> Option<Cause> {
    if !line.chars().all(|c| matches!(c, ' '..='~')) {
        return Some(Cause::NotPrintable);
    }
    if line.contains(['!', '~', '?']) {
        return Some(Cause::RealTime);
    }
    if line.len() > MAX_LINE {
        return Some(Cause::TooLong);
    }
    let upper = line.to_ascii_uppercase();
    let command = upper.strip_prefix('$')?;
    let setting = command.starts_with(|c: char| c.is_ascii_digit()) && command.contains('=');
    if setting
        || command.starts_with("RST")
        || command.starts_with('N')
        || command.starts_with("I=")
    {
        return Some(Cause::ChangesSetting);
    }
    (command == "X").then_some(Cause::Unlocks)
}

/// Every line of `text` prepared, one per job line (an empty string is acknowledged at once and
/// never sent), or the first refused line.
///
/// # Errors
/// [`Refused`] for the first line that must not reach the controller.
pub fn prepare(text: &str) -> Result<Vec<String>, Refused> {
    let mut lines = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = strip(raw);
        if let Some(cause) = check(&line) {
            return Err(Refused {
                line: u32::try_from(i + 1).unwrap_or(u32::MAX),
                cause,
            });
        }
        lines.push(line);
    }
    Ok(lines)
}

/// Whether a prepared line is a `$` command, which GRBL runs alone: nothing else is sent while
/// it is in flight (homing takes minutes, `specs/0127` G9).
#[must_use]
pub fn is_system(line: &str) -> bool {
    line.starts_with('$')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(text: &str) -> Option<(u32, Cause)> {
        prepare(text).err().map(|r| (r.line, r.cause))
    }

    #[test]
    fn comments_spaces_and_carriage_returns_go_and_empty_lines_stay_as_lines() {
        let lines =
            prepare("; Curvyo job: Hello! ~?\r\nG1 X1 (go!) Y2 ; fast?\r\n\nG0 X0\n(only ~)\nM5")
                .unwrap();
        assert_eq!(lines, ["", "G1X1Y2", "", "G0X0", "", "M5"]);
    }

    #[test]
    fn an_unclosed_comment_ends_the_line() {
        assert_eq!(strip("G1 X1 (no end"), "G1X1");
        assert_eq!(strip("(a)(b)G0"), "G0");
    }

    #[test]
    fn each_refusal_names_its_line_and_cause() {
        let table = [
            ("G1 X1\nG1 X1 !\n", (2, Cause::RealTime)),
            ("M3 S100~\n", (1, Cause::RealTime)),
            ("G0\nG0\nG1 X?\n", (3, Cause::RealTime)),
            ("$32=0\n", (1, Cause::ChangesSetting)),
            ("G0\n$RST=*\n", (2, Cause::ChangesSetting)),
            ("$N0=G21\n", (1, Cause::ChangesSetting)),
            ("$I=mine\n", (1, Cause::ChangesSetting)),
            ("G0\nG0\nG0\n$x\n", (4, Cause::Unlocks)),
            ("$X\n", (1, Cause::Unlocks)),
        ];
        for (text, expected) in table {
            assert_eq!(refused(text), Some(expected), "{text:?}");
        }
        // A second defence behind the job's own ASCII check: control and non-ASCII characters.
        for odd in ["G1X1\u{18}\n", "G0\nG1X\u{e9}\n", "M3\u{7f}\n"] {
            assert_eq!(
                refused(odd).map(|(_, c)| c),
                Some(Cause::NotPrintable),
                "{odd:?}"
            );
        }
        assert_eq!(refused("G0\nG1X\u{e9}\n"), Some((2, Cause::NotPrintable)));
        let long = format!("G1X{}\n", "1".repeat(77));
        assert_eq!(refused(&long), Some((1, Cause::TooLong)));
        let fits = format!("G1X{}\n", "1".repeat(76));
        assert_eq!(prepare(&fits).unwrap()[0].len(), 79);
        // A long comment is removed before the length is measured.
        assert!(prepare(&format!("G0 ({})\n", "x".repeat(200))).is_ok());
    }

    #[test]
    fn homing_and_reading_commands_pass() {
        assert!(prepare("$H\n$$\n$#\n$G\n$I\n$J=G91X1F100\n").is_ok());
        assert!(is_system("$H"));
        assert!(!is_system("G0X0"));
    }

    #[test]
    fn the_first_refused_line_wins() {
        assert_eq!(refused("$X\n$1=2\n"), Some((1, Cause::Unlocks)));
    }

    #[test]
    fn the_texts_follow_the_ux_notes() {
        assert_eq!(Cause::RealTime.text(), "Contains !, ~ or ?");
        assert_eq!(Cause::TooLong.text(), "Longer than 79 characters");
        assert_eq!(Cause::ChangesSetting.text(), "Changes a machine setting");
        assert_eq!(Cause::Unlocks.text(), "Unlocks the machine");
        assert_eq!(Cause::NotPrintable.text(), "Not plain ASCII text");
    }
}
