//! The held jog of the jog pad: one connection per profile, opened by the press, kept alive by
//! the webview's own calls and closed by the release (`specs/0118` adrs J8, criterion 18).
//! Rust sends no keep-alive of its own, so a frozen webview stops the head.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use curvyo_connector_core::{JogAxis, JogDirection, JogStep, Message, State as JobState};
use curvyo_connector_io::client::{
    CONNECT_TIMEOUT, ClientError, Connection, DEFAULT_PORT, open_paired_on,
};
use serde::Deserialize;
use tauri::{AppHandle, State};

use crate::connector::{ConnectorState, job_state, load_record};
use crate::connector_link::CommandAnswer;
use crate::device_keys::OpenStore;

/// A press on the pad: the machine axis, the way and the step (the step is a closed set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct JogRequest {
    pub(crate) axis: JogAxis,
    pub(crate) direction: JogDirection,
    pub(crate) step: JogStep,
}

impl JogRequest {
    /// The message of a step press or, with `hold`, a held one.
    pub(crate) fn message(self, hold: bool) -> Message {
        Message::Jog {
            axis: self.axis,
            direction: self.direction,
            step: self.step,
            hold,
        }
    }
}

/// The open connections of held jogs, by profile.
#[derive(Default)]
pub(crate) struct Holds(Mutex<HashMap<String, Connection>>);

impl Holds {
    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<String, Connection>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts a hold on `connection`, which was just opened with `status` as its first read; the
    /// connection stays open only when the connector took the press.
    pub(crate) fn start(
        &self,
        profile_id: &str,
        mut connection: Connection,
        state: JobState,
        request: JogRequest,
    ) -> CommandAnswer {
        // A hold of this profile that was never released ends with its connection.
        if let Some(mut old) = self.map().remove(profile_id) {
            let _ = old.send(&Message::JogStop);
            old.close();
        }
        match connection.command(&request.message(true), state) {
            Ok(_) => {
                self.map().insert(profile_id.to_owned(), connection);
                CommandAnswer::Done
            }
            Err(e) => CommandAnswer::from_error(e),
        }
    }

    /// One keep-alive on the profile's hold. The connection is read empty each time, so the
    /// connector's statuses never pile up; an `Error` or a lost connection ends the hold.
    pub(crate) fn keep(&self, profile_id: &str) -> CommandAnswer {
        let mut map = self.map();
        let Some(connection) = map.get_mut(profile_id) else {
            return CommandAnswer::WrongState;
        };
        let sent = connection.send(&Message::JogKeep);
        let drained = sent.and_then(|()| drain(connection));
        match drained {
            Ok(()) => CommandAnswer::Done,
            Err(e) => {
                map.remove(profile_id);
                CommandAnswer::from_error(e)
            }
        }
    }

    /// The release: `JogStop` at once, then the connection closes. Nothing held answers `Done`.
    pub(crate) fn release(&self, profile_id: &str) -> CommandAnswer {
        let Some(mut connection) = self.map().remove(profile_id) else {
            return CommandAnswer::Done;
        };
        let sent = connection.send(&Message::JogStop);
        connection.close();
        match sent {
            Ok(()) => CommandAnswer::Done,
            Err(e) => CommandAnswer::from_error(e),
        }
    }
}

/// Reads what the connector sent meanwhile, without waiting; an `Error` frame is the answer.
fn drain(connection: &mut Connection) -> Result<(), ClientError> {
    while let Some(frame) = connection.try_recv(Instant::now())? {
        if let curvyo_connector_io::frames::Frame::Core(Message::Error { code, .. }) = frame {
            return Err(ClientError::Protocol(format!("{code:?}")));
        }
    }
    Ok(())
}

/// A press held past 300 ms: opens the profile's hold connection and starts the jog.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_jog_hold(
    app: AppHandle,
    open: State<'_, OpenStore>,
    connector: State<'_, ConnectorState>,
    profile_id: String,
    address: String,
    jog: JogRequest,
) -> CommandAnswer {
    let record = match load_record(&app, &open, &profile_id) {
        Ok(record) => record,
        Err(error) => return CommandAnswer::Failed { error },
    };
    match open_paired_on(
        &address,
        DEFAULT_PORT,
        record.spki,
        &record.token,
        CONNECT_TIMEOUT,
    ) {
        Ok((connection, status)) => {
            connector
                .holds
                .start(&profile_id, connection, job_state(&status), jog)
        }
        Err(e) => CommandAnswer::Failed { error: e.into() },
    }
}

/// One keep-alive of the held press (the webview sends it every 100 ms).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_jog_keep(
    connector: State<'_, ConnectorState>,
    profile_id: String,
) -> CommandAnswer {
    connector.holds.keep(&profile_id)
}

/// The release of the held press.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_jog_release(
    connector: State<'_, ConnectorState>,
    profile_id: String,
) -> CommandAnswer {
    connector.holds.release(&profile_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_connector_io::client::{connect, open_paired};
    use curvyo_connector_io::hub::{ControllerChoice, FakeSettings};
    use curvyo_connector_io::pin_verifier::PinMode;
    use curvyo_connector_io::server::{RunningServer, ServerConfig};

    fn rig() -> (
        RunningServer,
        tempfile::TempDir,
        curvyo_connector_io::record::PairingRecord,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let server = RunningServer::start(&ServerConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            state_dir: dir.path().to_owned(),
            controller: ControllerChoice::Fake(FakeSettings {
                lines_per_second: 20,
                alarm_at_line: None,
            }),
            require_local_start: false,
            connector_name: "Test connector".to_owned(),
            issue_code: false,
        })
        .unwrap();
        let record = connect(
            &server.addr().to_string(),
            PinMode::Capture,
            CONNECT_TIMEOUT,
        )
        .unwrap()
        .pair(
            &server.current_code().unwrap(),
            "test app",
            Instant::now() + CONNECT_TIMEOUT,
        )
        .unwrap();
        (server, dir, record)
    }

    fn request() -> JogRequest {
        JogRequest {
            axis: JogAxis::X,
            direction: JogDirection::Plus,
            step: JogStep::Ten,
        }
    }

    fn state_now(
        server: &RunningServer,
        record: &curvyo_connector_io::record::PairingRecord,
    ) -> JobState {
        let (_, status) = open_paired(
            &server.addr().to_string(),
            record.spki,
            &record.token,
            CONNECT_TIMEOUT,
        )
        .unwrap();
        job_state(&status)
    }

    #[test]
    fn a_press_held_keeps_the_jog_alive_and_the_release_ends_it() {
        let (server, _dir, record) = rig();
        let holds = Holds::default();
        let (connection, status) = open_paired(
            &server.addr().to_string(),
            record.spki,
            &record.token,
            CONNECT_TIMEOUT,
        )
        .unwrap();
        let answer = holds.start("p", connection, job_state(&status), request());
        assert!(matches!(answer, CommandAnswer::Done), "{answer:?}");
        assert_eq!(state_now(&server, &record), JobState::Jogging);
        for _ in 0..3 {
            assert!(matches!(holds.keep("p"), CommandAnswer::Done));
        }
        assert_eq!(state_now(&server, &record), JobState::Jogging);
        assert!(matches!(holds.release("p"), CommandAnswer::Done));
        // A keep-alive after the release finds nothing held, and a second release is no error.
        assert!(matches!(holds.keep("p"), CommandAnswer::WrongState));
        assert!(matches!(holds.release("p"), CommandAnswer::Done));
    }

    #[test]
    fn a_hold_connection_that_is_dropped_ends_the_jog_at_the_connector() {
        let (server, _dir, record) = rig();
        let holds = Holds::default();
        let (connection, status) = open_paired(
            &server.addr().to_string(),
            record.spki,
            &record.token,
            CONNECT_TIMEOUT,
        )
        .unwrap();
        holds.start("p", connection, job_state(&status), request());
        drop(holds);
        let until = Instant::now() + CONNECT_TIMEOUT;
        while state_now(&server, &record) == JobState::Jogging {
            assert!(Instant::now() < until, "the dropped hold ended");
        }
    }
}
