//! Which state changes of the job machine reach `jobs.log` and the terminal, in which words
//! (`0064` Q18; `specs/0127` G7, G13, G15). Pure.

use curvyo_connector_core::{JobResult, Message, RejectReason, State};

use crate::grbl::texts::{LASER_MODE_OFF, LASER_MODE_UNKNOWN};
use crate::job_log::LogResult;

/// The terminal line for a refusal that names the controller's settings (G34, UX notes
/// section 10: `Job refused: <text>`); `None` for the others, which the log's own words cover.
#[must_use]
pub fn rejection_line(reason: RejectReason) -> Option<String> {
    let text = match reason {
        RejectReason::LaserModeOff => LASER_MODE_OFF,
        RejectReason::LaserModeUnknown => LASER_MODE_UNKNOWN,
        _ => return None,
    };
    Some(format!("Job refused: {text}"))
}

fn detail_of(status: &Message) -> Option<&str> {
    match status {
        Message::Status { detail, .. } => detail.as_deref(),
        _ => None,
    }
}

fn alarm_of(status: &Message) -> Option<&str> {
    match status {
        Message::Status { alarm, .. } => alarm.as_deref(),
        _ => None,
    }
}

/// The log's words for an app sentence: "Failed at line 12: X" is "line 12, X", "Lost the
/// controller at line 120" is "controller lost at line 120", "Lost the controller: X" is
/// "controller lost, X" (`specs/0142` HC3), any other starts in lower case.
#[must_use]
pub fn log_text(detail: &str) -> String {
    if let Some(rest) = detail.strip_prefix("Failed at ") {
        return rest.replacen(": ", ", ", 1);
    }
    if let Some(rest) = detail.strip_prefix("Lost the controller: ") {
        return format!("controller lost, {rest}");
    }
    if let Some(rest) = detail.strip_prefix("Lost the controller") {
        return format!("controller lost{rest}");
    }
    let mut chars = detail.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_lowercase().chain(chars).collect()
    })
}

/// What the log records for a change from `before` to `after`, and the terminal's sentence when
/// it differs from the log's words; `None` when the change is not a job outcome.
#[must_use]
pub fn logged_outcome(
    before: State,
    after: State,
    status: &Message,
) -> Option<(LogResult, Option<String>)> {
    let was_active = matches!(
        before,
        State::Running | State::Paused | State::AwaitingLocalStart
    );
    let started = matches!(before, State::Running | State::Paused);
    let detail = detail_of(status);
    match after {
        State::Done { result } => Some(match result {
            JobResult::Completed => (LogResult::Completed, None),
            JobResult::Stopped => (LogResult::Stopped, None),
            JobResult::Failed => (
                LogResult::Failed(detail.map(log_text)),
                detail.map(str::to_owned),
            ),
        }),
        State::Alarm if was_active => {
            let text = alarm_of(status).unwrap_or_default().to_owned();
            Some((LogResult::Alarm(text), None))
        }
        State::NoController if started => {
            let detail = detail?;
            Some((
                LogResult::Failed(Some(log_text(detail))),
                Some(detail.to_owned()),
            ))
        }
        _ => None,
    }
}

/// The terminal line of an alarm outside a job (G14); a job's alarm is in its log line.
#[must_use]
pub fn terminal_alarm(before: State, after: State, status: &Message) -> Option<String> {
    (after == State::Alarm && before != State::Alarm)
        .then(|| format!("Alarm: {}", alarm_of(status).unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_terminal_names_a_refusal_for_the_laser_mode() {
        assert_eq!(
            rejection_line(RejectReason::LaserModeOff).as_deref(),
            Some("Job refused: Laser mode is off ($32=0)")
        );
        assert_eq!(
            rejection_line(RejectReason::LaserModeUnknown).as_deref(),
            Some("Job refused: Laser mode unknown ($32 not read)")
        );
        assert_eq!(rejection_line(RejectReason::Busy), None);
    }

    fn status(state: State, alarm: Option<&str>, detail: Option<&str>) -> Message {
        Message::Status {
            state,
            job: None,
            position: None,
            alarm: alarm.map(str::to_owned),
            detail: detail.map(str::to_owned),
            pause_pending: false,
            requested_by: None,
            job_name: None,
            sent_by: None,
        }
    }

    #[test]
    fn the_log_words_of_each_sentence() {
        assert_eq!(
            log_text("Failed at line 12: No feed rate (error 22)"),
            "line 12, No feed rate (error 22)"
        );
        assert_eq!(
            log_text("Lost the controller at line 120"),
            "controller lost at line 120"
        );
        assert_eq!(
            log_text("Lost the controller: homing took longer than 3600 s"),
            "controller lost, homing took longer than 3600 s"
        );
        assert_eq!(
            log_text("Could not confirm the machine stopped"),
            "could not confirm the machine stopped"
        );
        assert_eq!(
            log_text("Unexpected answer from the controller"),
            "unexpected answer from the controller"
        );
    }

    #[test]
    fn outcomes_and_their_terminal_sentences() {
        let failed = State::Done {
            result: JobResult::Failed,
        };
        let line = "Failed at line 12: No feed rate (error 22)";
        assert_eq!(
            logged_outcome(State::Running, failed, &status(failed, None, Some(line))),
            Some((
                LogResult::Failed(Some("line 12, No feed rate (error 22)".into())),
                Some(line.into())
            ))
        );
        let lost = status(
            State::NoController,
            None,
            Some("Lost the controller at line 120"),
        );
        assert_eq!(
            logged_outcome(State::Paused, State::NoController, &lost).map(|o| o.0),
            Some(LogResult::Failed(Some(
                "controller lost at line 120".into()
            )))
        );
        // An unstarted job leaves no entry.
        let unstarted = status(State::NoController, None, Some("Lost the controller"));
        assert_eq!(
            logged_outcome(State::Ready, State::NoController, &unstarted),
            None
        );
        let alarm = status(State::Alarm, Some("Locked"), None);
        assert_eq!(logged_outcome(State::Idle, State::Alarm, &alarm), None);
        assert_eq!(
            terminal_alarm(State::Idle, State::Alarm, &alarm).as_deref(),
            Some("Alarm: Locked")
        );
    }
}
