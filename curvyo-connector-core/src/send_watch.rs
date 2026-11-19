//! Follows one sent job until the connector has accepted or lost it (`specs/0125` S11, S19;
//! architect addendum B4): the one place that decides "commit at most once per job id".

use uuid::Uuid;

use crate::message::{Message, RejectReason, State};

/// How a sent job ended its way to the connector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// The connector holds the job (`JobAccepted`, or a `Status` that carries its progress).
    Accepted,
    /// The connector refused it.
    Rejected(RejectReason),
    /// After the transfer ended, the connector shows no such job.
    NotReceived,
}

/// The watch of one sent job; answers once, then never again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendWatch {
    job_id: Uuid,
    transfer_ended: bool,
    resolved: bool,
}

impl SendWatch {
    /// Watches the job `job_id`, whose transfer is about to start.
    #[must_use]
    pub const fn new(job_id: Uuid) -> Self {
        Self {
            job_id,
            transfer_ended: false,
            resolved: false,
        }
    }

    /// The job this watch follows.
    #[must_use]
    pub const fn job_id(&self) -> Uuid {
        self.job_id
    }

    /// The connection that carried the transfer ended before an answer: the next `Status`
    /// without the job decides.
    pub fn transfer_ended(&mut self) {
        self.transfer_ended = true;
    }

    /// What `message` decides, `None` while it decides nothing and for ever after the first
    /// answer.
    pub fn observe(&mut self, message: &Message) -> Option<Resolution> {
        if self.resolved {
            return None;
        }
        let resolution = match message {
            Message::JobAccepted { job_id } if *job_id == self.job_id => Resolution::Accepted,
            Message::JobRejected { job_id, reason } if *job_id == self.job_id => {
                Resolution::Rejected(*reason)
            }
            Message::Status { state, job, .. } => {
                let ours = job.as_ref().is_some_and(|p| p.job_id == self.job_id);
                if ours && holds_progress(*state) {
                    Resolution::Accepted
                } else if self.transfer_ended {
                    Resolution::NotReceived
                } else {
                    return None;
                }
            }
            _ => return None,
        };
        self.resolved = true;
        Some(resolution)
    }
}

/// The states that carry the progress of a job the connector assembled (`0064` P20).
const fn holds_progress(state: State) -> bool {
    matches!(
        state,
        State::Ready
            | State::AwaitingLocalStart
            | State::Homing
            | State::Running
            | State::Paused
            | State::Done { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{JobResult, Progress};

    const ID: Uuid = Uuid::from_u128(7);
    const OTHER: Uuid = Uuid::from_u128(8);

    fn status(state: State, id: Option<Uuid>) -> Message {
        Message::Status {
            state,
            job: id.map(|job_id| Progress {
                job_id,
                line: 0,
                lines: 3,
                elapsed_ms: 0,
            }),
            position: None,
            alarm: None,
            detail: None,
        }
    }

    #[test]
    fn job_accepted_and_rejected_decide_at_any_time() {
        let mut accepted = SendWatch::new(ID);
        assert_eq!(
            accepted.observe(&Message::JobAccepted { job_id: ID }),
            Some(Resolution::Accepted)
        );
        let mut rejected = SendWatch::new(ID);
        assert_eq!(
            rejected.observe(&Message::JobRejected {
                job_id: ID,
                reason: RejectReason::Busy
            }),
            Some(Resolution::Rejected(RejectReason::Busy))
        );
    }

    #[test]
    fn an_answer_for_another_job_decides_nothing() {
        let mut watch = SendWatch::new(ID);
        assert_eq!(watch.observe(&Message::JobAccepted { job_id: OTHER }), None);
        assert_eq!(
            watch.observe(&Message::JobRejected {
                job_id: OTHER,
                reason: RejectReason::Busy
            }),
            None
        );
    }

    #[test]
    fn while_the_transfer_runs_other_statuses_are_ignored() {
        let mut watch = SendWatch::new(ID);
        for state in [
            State::Idle,
            State::Receiving,
            State::NoController,
            State::Alarm,
        ] {
            assert_eq!(watch.observe(&status(state, None)), None, "{state:?}");
        }
        assert_eq!(watch.observe(&status(State::Ready, Some(OTHER))), None);
    }

    #[test]
    fn a_status_with_the_job_in_a_progress_state_accepts_it() {
        for state in [
            State::Ready,
            State::AwaitingLocalStart,
            State::Running,
            State::Paused,
            State::Done {
                result: JobResult::Stopped,
            },
        ] {
            let mut watch = SendWatch::new(ID);
            assert_eq!(
                watch.observe(&status(state, Some(ID))),
                Some(Resolution::Accepted),
                "{state:?}"
            );
        }
    }

    #[test]
    fn after_the_transfer_ended_a_status_without_the_job_is_not_received() {
        let cases = [
            status(State::Idle, None),
            status(State::Receiving, None),
            status(State::Receiving, Some(ID)),
            status(State::NoController, None),
            status(State::Alarm, None),
            status(State::Ready, Some(OTHER)),
        ];
        for case in cases {
            let mut watch = SendWatch::new(ID);
            watch.transfer_ended();
            assert_eq!(
                watch.observe(&case),
                Some(Resolution::NotReceived),
                "{case:?}"
            );
        }
    }

    #[test]
    fn a_reconnect_that_shows_the_job_accepts_it() {
        let mut watch = SendWatch::new(ID);
        watch.transfer_ended();
        assert_eq!(
            watch.observe(&status(State::Running, Some(ID))),
            Some(Resolution::Accepted)
        );
    }

    #[test]
    fn a_resolved_watch_answers_nothing_so_a_replayed_status_commits_once() {
        let mut watch = SendWatch::new(ID);
        watch.transfer_ended();
        assert_eq!(
            watch.observe(&Message::JobAccepted { job_id: ID }),
            Some(Resolution::Accepted)
        );
        assert_eq!(watch.observe(&status(State::Ready, Some(ID))), None);
        assert_eq!(watch.observe(&Message::JobAccepted { job_id: ID }), None);
        assert_eq!(watch.observe(&status(State::Idle, None)), None);

        let mut lost = SendWatch::new(ID);
        lost.transfer_ended();
        assert_eq!(
            lost.observe(&status(State::Idle, None)),
            Some(Resolution::NotReceived)
        );
        assert_eq!(lost.observe(&status(State::Ready, Some(ID))), None);
    }

    #[test]
    fn other_messages_decide_nothing() {
        let mut watch = SendWatch::new(ID);
        watch.transfer_ended();
        assert_eq!(watch.observe(&Message::GetStatus), None);
        assert_eq!(watch.job_id(), ID);
    }
}
