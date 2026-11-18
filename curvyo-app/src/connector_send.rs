//! Send to machine (`specs/0125-send-to-machine`; architect addendum B4, B5, B7): the job bytes of a
//! laser page reach the connector as the raw body of one command, and the send is followed by its
//! job id until the connector has accepted or lost it, also across a dropped connection. The
//! webview never gets the token; it gets the job id, counts and the resolution.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use curvyo_connector_core::{Message, RejectReason, Resolution, SendWatch};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, DEFAULT_PORT, open_paired_on};
use curvyo_connector_io::job_send::new_job_id;
use curvyo_connector_io::record::PairingRecord;
use curvyo_connector_io::send_tracked::{
    SendEnd, SharedWatch, observe_shared, send_tracked, watched_job,
};
use serde::{Deserialize, Serialize};
use tauri::ipc::Request;
use tauri::{AppHandle, Emitter, State};

use crate::connector::{ConnectorState, load_record};
use crate::connector_link::{LinkError, reject_name};
use crate::device_keys::OpenStore;
use crate::raw_ipc::call_args;

/// The longest a job transfer may take.
const SEND_TIMEOUT: Duration = Duration::from_secs(120);

/// A send that has not been answered yet: its watch and the counts the page shows.
pub(crate) struct Pending {
    watch: SharedWatch,
}

/// The sends in flight, by machine profile id.
pub(crate) type PendingSends = Mutex<HashMap<String, Arc<Pending>>>;

/// How the connector stands to a sent job, as the page reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ResolutionView {
    /// The connector holds the job.
    Accepted,
    /// The connector refused it, with the name of the reason.
    Rejected { reason: String },
    /// The connector shows no such job after the transfer.
    NotReceived,
}

impl From<Resolution> for ResolutionView {
    fn from(resolution: Resolution) -> Self {
        match resolution {
            Resolution::Accepted => Self::Accepted,
            Resolution::Rejected(reason) => Self::Rejected {
                reason: reject_name(reason),
            },
            Resolution::NotReceived => Self::NotReceived,
        }
    }
}

/// The answer of `connector_send_page_job`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum PageSendAnswer {
    /// This call got the answer.
    Resolved {
        job_id: String,
        resolution: ResolutionView,
    },
    /// The watcher got the answer first; its `sendResolved` event carries it.
    ResolvedElsewhere,
    /// The connection dropped and could not be re-read; the watcher resolves the send later.
    Unresolved { error: LinkError },
    /// Nothing was sent.
    Failed { error: LinkError },
}

/// What the page is told besides the watcher's statuses.
#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum SendEvent {
    /// A send began: its id, to match the resolution, and its counts.
    SendStarted {
        job_id: String,
        lines: u32,
        bytes: u64,
    },
    /// The send ended; emitted by the watcher when it got the answer.
    SendResolved {
        job_id: String,
        resolution: ResolutionView,
    },
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SendPayload {
    profile_id: String,
    #[serde(flatten)]
    event: SendEvent,
}

fn emit(app: &AppHandle, profile_id: &str, event: SendEvent) {
    let _ = app.emit(
        crate::CONNECTOR_EVENT,
        SendPayload {
            profile_id: profile_id.to_owned(),
            event,
        },
    );
}

/// Whether a send of `profile_id` still awaits its answer: the watcher keeps trying meanwhile.
pub(crate) fn send_pending(pending: &PendingSends, profile_id: &str) -> bool {
    pending
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(profile_id)
        .is_some_and(|held| watched_job(&held.watch).is_some())
}

/// The watcher saw `status`: if it settles the send of `profile_id`, the event to tell the page.
pub(crate) fn resolve_by_status(
    pending: &PendingSends,
    profile_id: &str,
    status: &Message,
) -> Option<(String, ResolutionView)> {
    let held = pending
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(profile_id)
        .cloned()?;
    let job_id = watched_job(&held.watch)?;
    let resolution = observe_shared(&held.watch, status)?;
    Some((job_id.to_string(), resolution.into()))
}

/// Tells the page what the watcher's `status` settled, if anything.
pub(crate) fn report_resolution(
    app: &AppHandle,
    pending: &PendingSends,
    profile_id: &str,
    status: &Message,
) {
    if let Some((job_id, resolution)) = resolve_by_status(pending, profile_id, status) {
        emit(
            app,
            profile_id,
            SendEvent::SendResolved { job_id, resolution },
        );
    }
}

/// The send itself, without a window: opens fresh connections to `address`, sends `text` and
/// follows the job of `held`.
fn run_send(
    address: &str,
    default_port: u16,
    record: &PairingRecord,
    held: &Pending,
    name: &str,
    text: &str,
) -> PageSendAnswer {
    let job_id = watched_job(&held.watch);
    let mut open = || {
        open_paired_on(
            address,
            default_port,
            record.spki,
            &record.token,
            CONNECT_TIMEOUT,
        )
    };
    match send_tracked(
        &mut open,
        &held.watch,
        name,
        text,
        Instant::now() + SEND_TIMEOUT,
    ) {
        Err(error) => PageSendAnswer::Failed {
            error: error.into(),
        },
        Ok(SendEnd::Resolved(resolution)) => PageSendAnswer::Resolved {
            job_id: job_id.map(|id| id.to_string()).unwrap_or_default(),
            resolution: resolution.into(),
        },
        Ok(SendEnd::ResolvedElsewhere) => PageSendAnswer::ResolvedElsewhere,
        Ok(SendEnd::Unresolved(error)) => PageSendAnswer::Unresolved {
            error: error.into(),
        },
    }
}

/// The arguments besides the job's bytes.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendArgs {
    profile_id: String,
    address: String,
    name: String,
}

/// Send to machine: the raw body is the job text of the page (the file Export would write).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_send_page_job(
    app: AppHandle,
    open: State<'_, OpenStore>,
    state: State<'_, ConnectorState>,
    request: Request<'_>,
) -> PageSendAnswer {
    let (args, bytes): (SendArgs, Vec<u8>) = match call_args(&request) {
        Ok(parsed) => parsed,
        Err(detail) => {
            return PageSendAnswer::Failed {
                error: LinkError::other(&detail),
            };
        }
    };
    let record = match load_record(&app, &open, &args.profile_id) {
        Ok(record) => record,
        Err(error) => return PageSendAnswer::Failed { error },
    };
    let Ok(text) = String::from_utf8(bytes) else {
        // A byte outside UTF-8 can only be a damaged job: the connector would refuse it too.
        return PageSendAnswer::Resolved {
            job_id: String::new(),
            resolution: ResolutionView::Rejected {
                reason: reject_name(RejectReason::NotAscii),
            },
        };
    };
    let Ok(job_id) = new_job_id() else {
        return PageSendAnswer::Failed {
            error: LinkError::other(&"no random bytes are available"),
        };
    };
    let held = Arc::new(Pending {
        watch: Mutex::new(Some(SendWatch::new(job_id))),
    });
    state
        .pending
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(args.profile_id.clone(), Arc::clone(&held));
    emit(
        &app,
        &args.profile_id,
        SendEvent::SendStarted {
            job_id: job_id.to_string(),
            lines: u32::try_from(text.lines().count()).unwrap_or(u32::MAX),
            bytes: text.len() as u64,
        },
    );
    let answer = run_send(
        &args.address,
        DEFAULT_PORT,
        &record,
        &held,
        &args.name,
        &text,
    );
    if !matches!(answer, PageSendAnswer::Unresolved { .. }) {
        state
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&args.profile_id);
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_connector_io::client::connect;
    use curvyo_connector_io::hub::{ControllerChoice, FakeSettings};
    use curvyo_connector_io::pin_verifier::PinMode;
    use curvyo_connector_io::server::{RunningServer, ServerConfig};

    const JOB: &str = "G21\nG90\nG1 X1 F100 S10\n";

    fn rig() -> (RunningServer, tempfile::TempDir, PairingRecord) {
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

    fn pending() -> Arc<Pending> {
        Arc::new(Pending {
            watch: Mutex::new(Some(SendWatch::new(new_job_id().unwrap()))),
        })
    }

    #[test]
    fn a_send_is_accepted_and_the_answer_names_the_job() {
        let (server, _dir, record) = rig();
        let held = pending();
        let id = watched_job(&held.watch).unwrap().to_string();
        let answer = run_send(
            &server.addr().to_string(),
            DEFAULT_PORT,
            &record,
            &held,
            "page",
            JOB,
        );
        assert_eq!(
            answer,
            PageSendAnswer::Resolved {
                job_id: id,
                resolution: ResolutionView::Accepted
            }
        );
    }

    #[test]
    fn a_second_send_is_refused_busy_with_the_reason_name() {
        let (server, _dir, record) = rig();
        let address = server.addr().to_string();
        run_send(&address, DEFAULT_PORT, &record, &pending(), "one", JOB);
        let answer = run_send(&address, DEFAULT_PORT, &record, &pending(), "two", JOB);
        let PageSendAnswer::Resolved { resolution, .. } = answer else {
            panic!("{answer:?}");
        };
        assert_eq!(
            resolution,
            ResolutionView::Rejected {
                reason: "Busy".to_owned()
            }
        );
    }

    #[test]
    fn nothing_listening_fails_before_anything_is_sent() {
        let (server, _dir, record) = rig();
        let port = server.addr().port();
        drop(server);
        let answer = run_send("127.0.0.1", port, &record, &pending(), "page", JOB);
        assert!(
            matches!(
                answer,
                PageSendAnswer::Failed {
                    error: LinkError::Unreachable { .. }
                }
            ),
            "{answer:?}"
        );
    }

    #[test]
    fn the_watcher_resolves_a_registered_send_once() {
        let held = pending();
        let id = watched_job(&held.watch).unwrap();
        let map: PendingSends = Mutex::new(HashMap::from([("p".to_owned(), Arc::clone(&held))]));
        let status = Message::Status {
            state: curvyo_connector_core::State::Ready,
            job: Some(curvyo_connector_core::Progress {
                job_id: id,
                line: 0,
                lines: 3,
                elapsed_ms: 0,
            }),
            position: None,
            alarm: None,
            detail: None,
        };
        assert!(send_pending(&map, "p"));
        assert_eq!(
            resolve_by_status(&map, "p", &status),
            Some((id.to_string(), ResolutionView::Accepted))
        );
        assert_eq!(resolve_by_status(&map, "p", &status), None, "once");
        assert!(!send_pending(&map, "p"));
        assert_eq!(resolve_by_status(&map, "other", &status), None);
    }

    #[test]
    fn the_wire_shape_is_tagged_camel_case() {
        let json = serde_json::to_value(PageSendAnswer::Resolved {
            job_id: "x".to_owned(),
            resolution: ResolutionView::Rejected {
                reason: "Busy".to_owned(),
            },
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({"kind": "resolved", "jobId": "x",
                "resolution": {"kind": "rejected", "reason": "Busy"}})
        );
    }
}
