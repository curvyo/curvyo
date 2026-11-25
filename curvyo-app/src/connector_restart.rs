//! How the app asks the connector to restart an interrupted job from a line, its answer read
//! (`specs/0118` 2e, adrs R7, ux-notes W71).

use std::time::{Duration, Instant};

use curvyo_connector_core::{ErrorCode, Message, State};
use curvyo_connector_io::Uuid;
use curvyo_connector_io::client::Connection;
use curvyo_connector_io::frames::Frame;

use crate::connector::ListAnswer;
use crate::connector_link::LinkError;

/// How long the app waits for the connector's answer (W71: "Timed out" after 5 s).
const RESTART_WAIT: Duration = Duration::from_secs(5);

/// The generic words of a `WrongState`: the entry changed under the press, not a cause.
const STALE: &str = "This does not fit the current state.";

/// The answer `frame` gives to a restart of `job_id`, if it is one: the connector's `Error`, or
/// a status that shows the job taken (moving to its line, or waiting for the start at the
/// machine).
pub(crate) fn answer(frame: &Message, job_id: Uuid) -> Option<ListAnswer> {
    match frame {
        Message::Error {
            code: ErrorCode::WrongState,
            message,
        } if message != STALE => Some(ListAnswer::Refused {
            cause: message.clone(),
        }),
        Message::Error {
            code: ErrorCode::WrongState | ErrorCode::UnknownJob,
            ..
        } => Some(ListAnswer::Stale),
        Message::Error {
            code: ErrorCode::OwnerRequired,
            ..
        } => Some(ListAnswer::OwnerRequired),
        Message::Error { code, .. } => Some(ListAnswer::Refused {
            cause: format!("{code:?}"),
        }),
        Message::Status {
            state: State::Running | State::AwaitingLocalStart,
            job: Some(progress),
            ..
        } if progress.job_id == job_id && progress.from_line.is_some() => Some(ListAnswer::Done),
        _ => None,
    }
}

/// Sends `RestartFromLine { job_id, line }` on `connection` and waits for its answer.
pub(crate) fn restart_from_line(
    connection: &mut Connection,
    job_id: Uuid,
    line: u32,
) -> ListAnswer {
    if let Err(e) = connection.send(&Message::RestartFromLine { job_id, line }) {
        return ListAnswer::Failed { error: e.into() };
    }
    let until = Instant::now() + RESTART_WAIT;
    loop {
        match connection.try_recv(until) {
            Ok(None) => {
                return ListAnswer::Failed {
                    error: LinkError::TimedOut,
                };
            }
            Err(e) => return ListAnswer::Failed { error: e.into() },
            Ok(Some(Frame::Core(message))) => {
                if let Some(answer) = answer(&message, job_id) {
                    return answer;
                }
            }
            Ok(Some(_)) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_connector_core::Progress;

    fn status(state: State, from_line: Option<u32>) -> Message {
        Message::Status {
            state,
            job: Some(Progress {
                job_id: Uuid::from_u128(1),
                line: 1233,
                lines: 4000,
                elapsed_ms: 0,
                from_line,
                entering: true,
            }),
            position: None,
            alarm: None,
            detail: None,
            pause_pending: false,
            requested_by: None,
            job_name: None,
            sent_by: None,
            homed: true,
            machine_position: None,
            work_offset: None,
            frame_id: None,
            target: None,
        }
    }

    fn error(code: ErrorCode, message: &str) -> Message {
        Message::Error {
            code,
            message: message.to_owned(),
        }
    }

    #[test]
    fn the_answers_of_a_restart() {
        let id = Uuid::from_u128(1);
        assert_eq!(
            answer(&error(ErrorCode::WrongState, "Not homed"), id),
            Some(ListAnswer::Refused {
                cause: "Not homed".into()
            })
        );
        assert_eq!(
            answer(&error(ErrorCode::WrongState, STALE), id),
            Some(ListAnswer::Stale)
        );
        assert_eq!(
            answer(&error(ErrorCode::UnknownJob, "x"), id),
            Some(ListAnswer::Stale)
        );
        assert_eq!(
            answer(&error(ErrorCode::OwnerRequired, "x"), id),
            Some(ListAnswer::OwnerRequired)
        );
        assert_eq!(
            answer(&status(State::Running, Some(1234)), id),
            Some(ListAnswer::Done)
        );
        assert_eq!(
            answer(&status(State::AwaitingLocalStart, Some(1234)), id),
            Some(ListAnswer::Done)
        );
        assert_eq!(
            answer(&status(State::Running, None), id),
            None,
            "another job"
        );
        assert_eq!(answer(&status(State::Idle, Some(1234)), id), None);
    }
}
