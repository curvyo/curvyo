//! The connector's own stop of a running job or a go to point before it exits or resets.

use std::time::{Duration, Instant};

use curvyo_connector_core::{Event, Message, Peer, State};

use super::Hub;

/// The longest a stop at exit may take: G12's hold and banner limits plus `M5` and `M9`.
const EXIT_STOP: Duration = Duration::from_secs(6);

impl Hub {
    /// Stops a running or paused job as a client's Stop would (G16, A19); the job's name if one
    /// ran, "Focus point" for a go to point. A paused job whose resume waits for an owner counts
    /// as paused.
    pub fn stop_for_exit(&self, now: Instant) -> Option<String> {
        let mut inner = self.lock();
        if matches!(inner.machine.state(), State::Focusing | State::Pulsing) {
            // A go to point ends with the connector: the move by the jog cancel, the pulse by the
            // stop of G12 before the port closes (`specs/0118` 35 c).
            let stop = Event::Message(Message::Stop, Peer::host());
            self.apply(&mut inner, stop, now, &mut Vec::new());
            return Some("Focus point".to_owned());
        }
        let paused = inner.machine.waits_for_resume();
        if !paused && !matches!(inner.machine.state(), State::Running | State::Paused) {
            return None;
        }
        let name = inner.job_name.clone();
        // The job is written as interrupted at its line, then the list stays as it is.
        inner.sync_store(self.at(now), true);
        inner.store.flush();
        inner.frozen = true;
        self.apply(
            &mut inner,
            Event::Message(Message::Stop, Peer::host()),
            now,
            &mut Vec::new(),
        );
        Some(name)
    }

    /// After [`Hub::stop_for_exit`] named something: waits until the controller confirmed the
    /// stop, at most G12's hold and banner limits, so its reset reaches it before the port closes.
    pub fn await_exit_stop(&self) {
        let until = Instant::now() + EXIT_STOP;
        let stopping = |state| {
            matches!(
                state,
                State::Running | State::Paused | State::Focusing | State::Pulsing
            )
        };
        while stopping(self.state()) && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
