//! The words of pairing a device (`specs/0096-own-devices-sync` criteria 1 to 4, `ux-notes.md`
//! sections 3 and 4): the code, its countdown, the compare step and why a pairing ended.

use crate::share_text::{cut_label, server_shown};

/// The sentence under the pairing code (criterion 1).
pub const PAIR_SENTENCE: &str = "The device that enters this code can sync the workspaces you \
                                 choose, and you can remove it again. The server can tell which \
                                 devices belong together, not their names.";

/// The opener's status while the code is shown.
pub const WAITING_FOR_OTHER: &str = "Waiting for the other device.";

/// The code spelled for a screen reader: "K, 7, Q, 2, 9, M, 4, X".
#[must_use]
pub fn code_spelled(code: &str) -> String {
    code.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(String::from)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The countdown under the code: "Valid for 4:52".
#[must_use]
pub fn valid_for_text(seconds_left: u64) -> String {
    format!("Valid for {}:{:02}", seconds_left / 60, seconds_left % 60)
}

/// The opener's status after wrong tries: "Someone entered a wrong code 2 times."
#[must_use]
pub fn wrong_tries_seen_text(wrong: u8) -> String {
    match wrong {
        1 => "Someone entered a wrong code 1 time.".to_owned(),
        n => format!("Someone entered a wrong code {n} times."),
    }
}

/// The opener's compare heading: "Laptop wants to join".
#[must_use]
pub fn wants_to_join_text(label: &str) -> String {
    format!("{} wants to join", cut_label(label, 24))
}

/// The sentence under the check words: "They must match on Laptop."
#[must_use]
pub fn words_must_match_text(label: &str) -> String {
    format!("They must match on {}.", cut_label(label, 24))
}

/// The joiner's wait: "Waiting for Desktop to confirm".
#[must_use]
pub fn waiting_to_confirm_text(label: &str) -> String {
    format!("Waiting for {} to confirm", cut_label(label, 24))
}

/// The opener after Pair: "Pairing Laptop...".
#[must_use]
pub fn pairing_text(label: &str) -> String {
    format!("Pairing {}...", cut_label(label, 24))
}

/// Why a pairing ended, as both sides learn it (criterion 4, `ux-notes.md` 3 and 4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairProblem {
    /// The code's secret did not match (joiner side).
    WrongCode,
    /// Too many wrong tries locked the code.
    Locked,
    /// The code expired.
    Expired,
    /// The code was used.
    Used,
    /// No such code on the server.
    Unknown,
    /// Another device is using the code.
    Busy,
    /// The other side cancelled or pressed Words differ.
    Stopped,
    /// The connection was lost.
    Lost,
    /// The server could not be reached.
    Unreachable,
    /// This side pressed Words differ.
    WordsDiffer,
}

/// The joiner's message for `problem` (4.5); `tries_left` for a wrong code, `other` names the
/// device that shows the code.
#[must_use]
pub fn joiner_problem_text(
    problem: PairProblem,
    tries_left: u8,
    other: &str,
    server: &str,
) -> String {
    let other = if other.is_empty() {
        "the other device".to_owned()
    } else {
        cut_label(other, 24)
    };
    match problem {
        PairProblem::WrongCode => match tries_left {
            0 => "That code does not match.".to_owned(),
            1 => "That code does not match. 1 try left.".to_owned(),
            n => format!("That code does not match. {n} tries left."),
        },
        PairProblem::Locked => "Too many wrong tries. Ask for a new code.".to_owned(),
        PairProblem::Expired => "This code has expired. Ask for a new one.".to_owned(),
        PairProblem::Used => "This code was already used.".to_owned(),
        PairProblem::Unknown => "There is no such code. Check it and try again.".to_owned(),
        PairProblem::Busy => "Another device is using this code. Try again shortly.".to_owned(),
        PairProblem::Stopped => format!("Pairing was stopped on {other}. Nothing was shared."),
        PairProblem::Lost => "Lost the connection. Pairing stopped. Nothing was shared.".to_owned(),
        PairProblem::Unreachable => {
            format!("Cannot reach the server at {}.", server_shown(server))
        }
        PairProblem::WordsDiffer => "Pairing stopped. Nothing was shared.".to_owned(),
    }
}

/// The opener's alert for `problem` (section 3); `label` names the joiner when known.
#[must_use]
pub fn opener_problem_text(problem: PairProblem, label: &str) -> String {
    match problem {
        PairProblem::Locked => "Too many wrong tries. This code no longer works.".to_owned(),
        PairProblem::Expired => "This code has expired.".to_owned(),
        PairProblem::Stopped if !label.is_empty() => format!(
            "The words did not match on {}. Pairing stopped.",
            cut_label(label, 24)
        ),
        PairProblem::WordsDiffer
        | PairProblem::Stopped
        | PairProblem::WrongCode
        | PairProblem::Used => "Pairing stopped. The code no longer works.".to_owned(),
        PairProblem::Unreachable => "Could not pair: cannot reach the server.".to_owned(),
        PairProblem::Unknown | PairProblem::Busy | PairProblem::Lost => {
            "Lost the connection. Pairing stopped. Nothing was shared.".to_owned()
        }
    }
}

/// The polite announcement of the code's countdown (section 3): at one minute, else nothing.
#[must_use]
pub fn code_left_announcement(seconds_left: u64) -> &'static str {
    if seconds_left > 0 && seconds_left <= 60 {
        "1 minute left"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_code_reads_spelled_and_counts_down() {
        assert_eq!(code_spelled("K7Q2-9M4X"), "K, 7, Q, 2, 9, M, 4, X");
        assert_eq!(valid_for_text(292), "Valid for 4:52");
        assert_eq!(valid_for_text(59), "Valid for 0:59");
        assert_eq!(
            wrong_tries_seen_text(2),
            "Someone entered a wrong code 2 times."
        );
    }

    #[test]
    fn pairing_problems_name_the_case() {
        assert_eq!(
            joiner_problem_text(PairProblem::WrongCode, 3, "", ""),
            "That code does not match. 3 tries left."
        );
        assert_eq!(
            joiner_problem_text(PairProblem::Stopped, 0, "Desktop", ""),
            "Pairing was stopped on Desktop. Nothing was shared."
        );
        assert_eq!(
            joiner_problem_text(PairProblem::Unreachable, 0, "", "ws://192.168.1.20:8080"),
            "Cannot reach the server at 192.168.1.20:8080."
        );
        assert_eq!(
            opener_problem_text(PairProblem::Locked, ""),
            "Too many wrong tries. This code no longer works."
        );
        assert_eq!(
            opener_problem_text(PairProblem::Stopped, "Laptop"),
            "The words did not match on Laptop. Pairing stopped."
        );
    }

    #[test]
    fn the_opener_says_a_used_code_no_longer_works_and_announces_the_last_minute() {
        assert_eq!(
            opener_problem_text(PairProblem::Used, ""),
            "Pairing stopped. The code no longer works."
        );
        assert_eq!(code_left_announcement(60), "1 minute left");
        assert_eq!(code_left_announcement(61), "");
    }
}
