//! The status rate limit (P20, Q14): the newest pending status wins and at most one goes out
//! per 200 ms, so every change reaches the app in time and never more than 5 a second.

use std::time::{Duration, Instant};

use curvyo_connector_core::Message;

/// The shortest time between two pushed statuses.
pub const MIN_INTERVAL: Duration = Duration::from_millis(200);

/// One connection's pending status.
#[derive(Debug, Default)]
pub struct StatusLimiter {
    pending: Option<Message>,
    last_sent: Option<Instant>,
}

impl StatusLimiter {
    /// A status that has to reach the app; it replaces an older one that was not sent yet.
    pub fn push(&mut self, status: Message) {
        self.pending = Some(status);
    }

    /// The status to send now, if one is waiting and 200 ms have passed since the last send.
    pub fn due(&mut self, now: Instant) -> Option<Message> {
        let ready = self
            .last_sent
            .is_none_or(|last| now.saturating_duration_since(last) >= MIN_INTERVAL);
        if ready && self.pending.is_some() {
            self.last_sent = Some(now);
            return self.pending.take();
        }
        None
    }

    /// A status was answered directly (`GetStatus`): it is the newest, so nothing older waits.
    pub fn answered(&mut self, now: Instant) {
        self.pending = None;
        self.last_sent = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_connector_core::State;

    fn status(line: u32) -> Message {
        Message::Status {
            state: State::Running,
            job: Some(curvyo_connector_core::Progress {
                job_id: uuid::Uuid::nil(),
                line,
                lines: 10_000,
                elapsed_ms: 0,
            }),
            position: None,
            alarm: None,
            detail: None,
            pause_pending: false,
            requested_by: None,
            job_name: None,
            sent_by: None,
            homed: false,
            machine_position: None,
            work_offset: None,
        }
    }

    fn line_of(message: &Message) -> u32 {
        match message {
            Message::Status { job: Some(job), .. } => job.line,
            _ => panic!("not a status"),
        }
    }

    #[test]
    fn a_burst_of_fast_lines_goes_out_at_most_five_times_a_second() {
        let base = Instant::now();
        let mut limiter = StatusLimiter::default();
        let mut sent = Vec::new();
        // One status per millisecond for 3 seconds: 1000 lines a second.
        for ms in 0..3000u32 {
            let now = base + Duration::from_millis(u64::from(ms));
            limiter.push(status(ms));
            if let Some(message) = limiter.due(now) {
                sent.push((ms, line_of(&message)));
            }
        }
        for second in 0..3u32 {
            let in_window = sent
                .iter()
                .filter(|(ms, _)| (second * 1000..(second + 1) * 1000).contains(ms))
                .count();
            assert!(in_window <= 5, "{in_window} in second {second}");
            assert!(in_window >= 4);
        }
        assert!(sent.windows(2).all(|w| w[1].0 - w[0].0 >= 200));
    }

    #[test]
    fn the_newest_status_wins_and_the_last_change_is_never_lost() {
        let base = Instant::now();
        let mut limiter = StatusLimiter::default();
        limiter.push(status(1));
        assert_eq!(line_of(&limiter.due(base).unwrap()), 1);
        limiter.push(status(2));
        limiter.push(status(3));
        assert!(limiter.due(base + Duration::from_millis(199)).is_none());
        assert_eq!(
            line_of(&limiter.due(base + Duration::from_millis(200)).unwrap()),
            3
        );
        assert!(limiter.due(base + Duration::from_secs(5)).is_none());
    }

    #[test]
    fn a_direct_answer_replaces_what_waited() {
        let base = Instant::now();
        let mut limiter = StatusLimiter::default();
        limiter.push(status(1));
        limiter.answered(base);
        assert!(limiter.due(base + Duration::from_secs(1)).is_none());
    }
}
