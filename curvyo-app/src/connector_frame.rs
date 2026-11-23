//! Frame (`specs/0118` slice 2b, adrs F2): the frame text of the page reaches the connector as
//! the raw body of one command over a fresh connection, and the answer is the connector's
//! `JobAccepted` or its refusal. Nothing is watched, started or listed: a frame is never a job,
//! and its state comes back as the status of `Framing`.

use std::time::{Duration, Instant};

use curvyo_connector_core::{JobKind, RejectReason};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, DEFAULT_PORT, open_paired_on};
use curvyo_connector_io::job_send::{SendError, new_job_id};
use curvyo_connector_io::record::PairingRecord;
use serde::Deserialize;
use tauri::ipc::Request;
use tauri::{AppHandle, State};

use crate::connector::load_record;
use crate::connector_link::{CommandAnswer, LinkError};
use crate::device_keys::OpenStore;
use crate::raw_ipc::call_args;

/// The longest the connector may take to answer a frame (the page shows "Timed out" after it).
const FRAME_TIMEOUT: Duration = Duration::from_secs(5);

/// The arguments besides the frame text.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FrameArgs {
    profile_id: String,
    address: String,
}

/// What a refusal of the frame means to the page: a busy or absent machine is a state that does
/// not allow it ("Connector busy"); the rest is a damaged frame, which the page cannot explain.
fn answer_of(error: SendError) -> CommandAnswer {
    match error {
        SendError::Rejected(RejectReason::Busy | RejectReason::NoController) => {
            CommandAnswer::WrongState
        }
        SendError::Rejected(reason) => CommandAnswer::Failed {
            error: LinkError::other(&format!("the connector refused the frame: {reason:?}")),
        },
        SendError::Client(error) => CommandAnswer::from_error(error),
    }
}

/// Frame: the raw body is the frame text of the page (`WasmSession::frame_bytes`).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_send_frame(
    app: AppHandle,
    open: State<'_, OpenStore>,
    request: Request<'_>,
) -> CommandAnswer {
    let (args, bytes): (FrameArgs, Vec<u8>) = match call_args(&request) {
        Ok(parsed) => parsed,
        Err(detail) => {
            return CommandAnswer::Failed {
                error: LinkError::other(&detail),
            };
        }
    };
    let record = match load_record(&app, &open, &args.profile_id) {
        Ok(record) => record,
        Err(error) => return CommandAnswer::Failed { error },
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return CommandAnswer::Failed {
            error: LinkError::other(&"the frame text is not UTF-8"),
        };
    };
    run_frame(&args.address, DEFAULT_PORT, &record, &text)
}

/// The frame itself, without a window: a fresh connection to `address`, the text sent as a
/// frame and the connector's answer.
fn run_frame(address: &str, port: u16, record: &PairingRecord, text: &str) -> CommandAnswer {
    let Ok(job_id) = new_job_id() else {
        return CommandAnswer::Failed {
            error: LinkError::other(&"no random bytes are available"),
        };
    };
    let (mut connection, _) =
        match open_paired_on(address, port, record.spki, &record.token, CONNECT_TIMEOUT) {
            Ok(opened) => opened,
            Err(e) => return CommandAnswer::Failed { error: e.into() },
        };
    match connection.send_kind(
        job_id,
        JobKind::Frame,
        "Frame",
        text,
        Instant::now() + FRAME_TIMEOUT,
    ) {
        Ok(_) => CommandAnswer::Done,
        Err(error) => answer_of(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_connector_io::client::connect;
    use curvyo_connector_io::hub::{ControllerChoice, FakeSettings};
    use curvyo_connector_io::pin_verifier::PinMode;
    use curvyo_connector_io::server::{RunningServer, ServerConfig};

    const FRAME: &str = "G21\nG90\nM5\nM9\nG1 X5.000 Y0.000 F3000 S0\nG1 X0.000 Y0.000 F3000 S0\n";

    fn rig() -> (RunningServer, tempfile::TempDir, PairingRecord) {
        let dir = tempfile::tempdir().unwrap();
        let server = RunningServer::start(&ServerConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            state_dir: dir.path().to_owned(),
            controller: ControllerChoice::Fake(FakeSettings {
                lines_per_second: 2,
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

    #[test]
    fn a_frame_text_is_accepted_and_a_second_while_it_runs_is_busy() {
        let (server, _dir, record) = rig();
        let address = server.addr().to_string();
        let run = |text: &str| run_frame(&address, DEFAULT_PORT, &record, text);
        assert_eq!(run(FRAME), CommandAnswer::Done);
        assert_eq!(run(FRAME), CommandAnswer::WrongState, "Connector busy");
    }

    #[test]
    fn a_frame_that_could_light_the_laser_is_refused_and_nothing_runs() {
        let (server, _dir, record) = rig();
        let address = server.addr().to_string();
        let bad = "G21\nG90\nM5\nM3 S1000\nG1 X5 Y0 F3000 S0\n";
        let answer = run_frame(&address, DEFAULT_PORT, &record, bad);
        assert!(
            matches!(
                answer,
                CommandAnswer::Failed {
                    error: LinkError::Other { .. }
                }
            ),
            "{answer:?}"
        );
        // The refusal left the connector free: a good frame is taken.
        assert_eq!(
            run_frame(&address, DEFAULT_PORT, &record, FRAME),
            CommandAnswer::Done
        );
    }
}
