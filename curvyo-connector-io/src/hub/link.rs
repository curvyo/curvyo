//! The hub's side of the controller: its turns, the serial thread's exchange, its loss, and its
//! replacement by a new configuration (addendum D14).

use std::sync::mpsc::Sender;
use std::time::Instant;

use curvyo_connector_core::{Event, StartRule, State};

use curvyo_connector_core::ControllerConfig;

use super::{ControllerChoice, Exchange, Hub, Inner, SerialSettings, machine_for};
use crate::controller::Controller;
use crate::fake_controller::FakeController;
use crate::grbl::GrblController;

impl Hub {
    /// Lets the controller run up to `now` with nothing read (the fake controller's pace).
    pub fn tick(&self, now: Instant) {
        let mut inner = self.lock();
        self.turn(&mut inner, &[], now);
    }

    /// One turn of the serial thread: `read` from the port; what to write and whether to close.
    pub fn exchange(&self, read: &[u8], now: Instant) -> Exchange {
        let mut inner = self.lock();
        let close = self.turn(&mut inner, read, now);
        Exchange {
            write: std::mem::take(&mut inner.pending_write),
            close,
        }
    }

    /// Runs the controller once; true when it gave the link up.
    pub(super) fn turn(&self, inner: &mut Inner, read: &[u8], now: Instant) -> bool {
        let at = self.at(now);
        let Some(controller) = inner.controller.as_mut() else {
            return false;
        };
        let advance = controller.advance(read, at);
        inner.pending_write.extend_from_slice(&advance.write);
        for notice in advance.notices {
            let _ = inner.notices.send(notice);
        }
        self.publish_info(inner);
        for event in advance.events {
            self.apply(inner, event, now, &mut Vec::new());
        }
        if matches!(inner.machine.state(), State::Jogging | State::Pulsing) {
            // The dead-man of a held jog and the pulse's deadline count on the machine's clock
            // (`specs/0118` J4, adrs P6).
            self.apply(inner, Event::Clock(at), now, &mut Vec::new());
        }
        let Some(notice) = advance.close else {
            return false;
        };
        let _ = inner.notices.send(notice);
        self.drop_controller(inner, now);
        true
    }

    fn drop_controller(&self, inner: &mut Inner, now: Instant) {
        inner.controller = None;
        self.publish_info(inner);
        self.apply(inner, Event::ControllerLost, now, &mut Vec::new());
    }

    /// Sends a `Welcome` to every subscriber when the controller's info changed (addendum C6).
    pub(super) fn publish_info(&self, inner: &mut Inner) {
        let info = inner.controller.as_ref().and_then(|c| c.info());
        if info != inner.info {
            inner.info.clone_from(&info);
            self.publish_welcome(inner);
        }
    }

    /// Sends the current `Welcome` to every subscriber.
    pub(super) fn publish_welcome(&self, inner: &mut Inner) {
        let welcome = self.welcome_of(inner, Instant::now());
        inner
            .subscribers
            .retain(|s| s.send(welcome.clone()).is_ok());
    }

    /// The serial thread opened the port at `now`: a fresh GRBL controller identifies (G3).
    /// A serial thread that a new configuration replaced attaches nothing.
    pub fn attach_serial(&self, serial: &SerialSettings, now: Instant) {
        let mut inner = self.lock();
        if inner.serial_port.as_deref() != Some(serial.port.as_str()) {
            return;
        }
        let controller =
            GrblController::new(&serial.port, serial.baud, serial.timing, self.at(now))
                .with_spindle(serial.spindle);
        inner.controller = Some(Box::new(controller));
    }

    /// The port `port` failed (read or write error): the controller is lost (G15). A port a new
    /// configuration replaced loses nothing.
    pub fn lose(&self, port: &str, notice: String, now: Instant) {
        let mut inner = self.lock();
        if inner.serial_port.as_deref() == Some(port) && inner.controller.is_some() {
            let _ = inner.notices.send(notice);
            inner.pending_write.clear();
            self.drop_controller(&mut inner, now);
        }
    }

    /// The port `port` could not be opened: why, for the app (G2, G3).
    pub fn open_failed(&self, port: &str, detail: String, now: Instant) {
        let mut inner = self.lock();
        if inner.serial_port.as_deref() != Some(port) {
            return;
        }
        self.apply(
            &mut inner,
            Event::NoControllerDetail { detail },
            now,
            &mut Vec::new(),
        );
    }

    /// Where controller changes go: the thread that owns the controller's thread.
    pub fn set_supervisor(&self, supervisor: Sender<ControllerConfig>) {
        self.lock().supervisor = Some(supervisor);
    }

    /// Installs the controller of a configuration at once, under the hub's lock, so no message
    /// meets the old machine after the answer to `ApplyConfig`; then asks the supervisor to swap
    /// the controller's thread (only while no job is held, addendum D8).
    pub(super) fn install(&self, inner: &mut Inner, config: ControllerConfig) {
        let choice = crate::server::choice_of(&config, crate::grbl::Timing::default());
        self.rebuild(inner, &choice, StartRule::OwnerConfirms);
        if let Some(supervisor) = &inner.supervisor {
            let _ = supervisor.send(config);
        }
    }

    /// The machine starts afresh with the controller of `choice` under `rule`.
    pub(super) fn rebuild(&self, inner: &mut Inner, choice: &ControllerChoice, rule: StartRule) {
        let kept = inner.machine.take_list();
        inner.machine = machine_for(choice, rule);
        inner.machine.restore(kept);
        inner.controller = fake_of(choice);
        inner.serial_port = serial_port_of(choice);
        inner.pending_write.clear();
        inner.job_name.clear();
        if inner.controller.is_some() {
            inner.machine.handle(Event::ControllerConnected);
        }
        inner.info = inner.controller.as_ref().and_then(|c| c.info());
        self.publish_welcome(inner);
        let status = inner.machine.status();
        inner.subscribers.retain(|s| s.send(status.clone()).is_ok());
    }

    /// Gives the hub a controller test double (addendum D16).
    #[cfg(test)]
    pub(crate) fn set_test_controller(&self, controller: Box<dyn crate::controller::Controller>) {
        let mut inner = self.lock();
        inner.controller = Some(controller);
        inner.machine.handle(Event::ControllerConnected);
    }
}

pub(super) fn serial_port_of(choice: &ControllerChoice) -> Option<String> {
    match choice {
        ControllerChoice::Serial(serial) => Some(serial.port.clone()),
        _ => None,
    }
}

pub(super) fn fake_of(choice: &ControllerChoice) -> Option<Box<dyn Controller>> {
    match choice {
        ControllerChoice::Fake(f) => Some(Box::new(FakeController::new(
            f.lines_per_second,
            f.alarm_at_line,
        ))),
        _ => None,
    }
}
