//! The commands of the Connector group in the Machine block (`specs/0064-machine-connector`): test,
//! pair, forget, send job, job controls and the status watcher. The pinned key and the token stay
//! in the credential store and in this process; the webview gets fingerprints and states only.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use curvyo_connector_core::{ControllerInfo, MAX_JOB_BYTES, Message, State as JobState};
use curvyo_connector_io::Uuid;
use curvyo_connector_io::client::{
    CONNECT_TIMEOUT, ClientError, DEFAULT_PORT, connect, connect_on, open_paired_on,
};
use curvyo_connector_io::job_send::SendError;
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::record::PairingRecord;
use curvyo_connector_io::watch::{WatchEvent, watch};
use curvyo_storage_io::CredentialStore;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::connector_jog::JogRequest;
use crate::connector_link::{
    CommandAnswer, FileSendError, LinkError, OTHER_TEXT, PairAnswer, PairingView, SendAnswer,
    TestAnswer, effective_role, owner_label,
};
use crate::connector_point::PointRequest;
use crate::connector_send::{PendingSends, report_resolution, send_pending};
use crate::device_keys::{OpenStore, StoreProblem};

/// The pause between two tries of the watcher (`ux-notes.md` section 4).
const RETRY: Duration = Duration::from_secs(5);
/// The longest a job transfer may take.
const SEND_TIMEOUT: Duration = Duration::from_secs(120);
/// The file remembering where Send job last looked.
const JOB_DIR_FILE: &str = "connector-job-dir.txt";

/// What the app remembers between commands: the running watcher of each profile and the sends
/// that await their answer.
#[derive(Default)]
pub(crate) struct ConnectorState {
    watchers: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub(crate) pending: PendingSends,
    /// The held jogs (`specs/0118` J8).
    pub(crate) holds: crate::connector_jog::Holds,
}

/// The name of a profile's record in the credential store (addendum A8).
pub(crate) fn record_name(profile_id: &str) -> String {
    format!("connector/{profile_id}")
}

/// Removes the record of `profile_id`; nothing happens when there is none.
pub(crate) fn forget_record(
    store: &CredentialStore,
    profile_id: &str,
) -> Result<(), curvyo_storage_io::CredentialError> {
    store.remove(&record_name(profile_id))
}

fn valid_id(profile_id: &str) -> bool {
    curvyo_laser_core::is_profile_id(profile_id)
}

pub(crate) fn load_record(
    app: &AppHandle,
    open: &OpenStore,
    profile_id: &str,
) -> Result<PairingRecord, LinkError> {
    if !valid_id(profile_id) {
        return Err(LinkError::NotPaired);
    }
    let stored = open
        .with_keychain(app, |store| store.get(&record_name(profile_id)))
        .map_err(store_error)?
        .map_err(|e| LinkError::other(&e))?;
    let bytes = stored.ok_or(LinkError::NotPaired)?;
    PairingRecord::from_bytes(&bytes).map_err(|e| LinkError::other(&e))
}

fn store_error(problem: StoreProblem) -> LinkError {
    match problem {
        StoreProblem::NoStore => LinkError::NoCredentialStore,
        StoreProblem::Failed(message) => LinkError::other(&message),
    }
}

pub(crate) fn job_state(status: &Message) -> JobState {
    match status {
        Message::Status { state, .. } => *state,
        _ => JobState::NoController,
    }
}

/// The id of the job a `Ready` status carries, or of a start a member requested (an owner's
/// Confirm start, `specs/0128` A26): what `Start` needs, from any connection.
fn ready_job_id(status: &Message, list: Option<&Message>) -> Option<Uuid> {
    ready_job_of_status(status).or_else(|| first_waiting_id(status, list))
}

/// The first waiting job that is not interrupted, when the connector holds no job that is not
/// over: what Start takes then (`specs/0132` criterion 4).
fn first_waiting_id(status: &Message, list: Option<&Message>) -> Option<Uuid> {
    let free = matches!(
        status,
        Message::Status {
            state: JobState::Idle | JobState::Done { .. },
            ..
        }
    );
    match list {
        Some(Message::JobList { waiting, .. }) if free => waiting
            .iter()
            .find(|e| e.interrupted_at.is_none())
            .map(|e| e.job_id),
        _ => None,
    }
}

fn ready_job_of_status(status: &Message) -> Option<Uuid> {
    match status {
        Message::Status {
            state: JobState::Ready,
            job: Some(progress),
            ..
        }
        | Message::Status {
            state: JobState::AwaitingLocalStart,
            job: Some(progress),
            requested_by: Some(_),
            ..
        } => Some(progress.job_id),
        _ => None,
    }
}

/// What the row shows of a stored pairing without any connection.
#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum PairingState {
    None,
    Paired(PairingView),
    NoStore,
    Failed { message: String },
}

/// The stored pairing of `profile_id`, for the trust block.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_pairing(
    app: AppHandle,
    open: State<'_, OpenStore>,
    profile_id: String,
) -> PairingState {
    match load_record(&app, &open, &profile_id) {
        Ok(record) => PairingState::Paired(PairingView {
            connector_name: record.connector_name,
            fingerprint: record.spki.fingerprint(),
        }),
        Err(LinkError::NotPaired) => PairingState::None,
        Err(LinkError::NoCredentialStore) => PairingState::NoStore,
        Err(e) => PairingState::Failed {
            message: match e {
                LinkError::Other { message } => message,
                _ => OTHER_TEXT.to_owned(),
            },
        },
    }
}

/// Test connection, and the status read on opening the block (Q11, Q16).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_test(
    app: AppHandle,
    open: State<'_, OpenStore>,
    profile_id: String,
    address: String,
) -> TestAnswer {
    match load_record(&app, &open, &profile_id) {
        Ok(record) => run_test(&address, DEFAULT_PORT, &record),
        Err(LinkError::NotPaired) => run_test_unpaired(&address, DEFAULT_PORT),
        Err(error) => TestAnswer::Failed { error },
    }
}

/// The test of a profile without a pairing: only a connector in maintenance mode is told apart
/// (`specs/0128` UX notes 8.4); anything else is "Not paired".
fn run_test_unpaired(address: &str, default_port: u16) -> TestAnswer {
    let not_paired = TestAnswer::Failed {
        error: LinkError::NotPaired,
    };
    let Ok(connection) = connect_on(address, default_port, PinMode::Capture, CONNECT_TIMEOUT)
    else {
        return not_paired;
    };
    match connection.greeting.mode {
        Some(curvyo_connector_core::ConnectorMode::Maintenance { .. }) => TestAnswer::Maintenance {
            protocol: connection.greeting.protocol,
            version: connection.greeting.version.clone(),
        },
        _ => not_paired,
    }
}

/// The test itself; `default_port` is what an address without a port means.
fn run_test(address: &str, default_port: u16, record: &PairingRecord) -> TestAnswer {
    match open_paired_on(
        address,
        default_port,
        record.spki,
        &record.token,
        CONNECT_TIMEOUT,
    ) {
        Ok((connection, status)) => TestAnswer::Connected {
            controller: TestAnswer::controller_name(connection.greeting.controller.clone()),
            controller_info: connection.greeting.controller.clone().map(Box::new),
            protocol: connection.greeting.protocol,
            version: connection.greeting.version.clone(),
            connector_name: connection.greeting.name.clone(),
            status: Box::new(status),
            role: effective_role(
                connection.greeting.mode.as_ref(),
                connection.session.as_ref().map(|(_, role)| *role),
            ),
            client_name: connection.session.as_ref().map(|(name, _)| name.clone()),
            owner_label: owner_label(connection.greeting.mode.as_ref()),
        },
        Err(e) => TestAnswer::Failed { error: e.into() },
    }
}

fn client_name() -> String {
    crate::devices::device_host_name()
        .chars()
        .take(64)
        .collect()
}

/// Pairs with the typed code (Q5 to Q7).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_pair(
    app: AppHandle,
    open: State<'_, OpenStore>,
    profile_id: String,
    address: String,
    code: String,
) -> PairAnswer {
    if !valid_id(&profile_id) {
        return PairAnswer::Failed {
            error: LinkError::NotPaired,
        };
    }
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    let mut connection = match connect(&address, PinMode::Capture, CONNECT_TIMEOUT) {
        Ok(connection) => connection,
        Err(e) => return PairAnswer::Failed { error: e.into() },
    };
    // A connector in maintenance mode takes no pairing (`specs/0128` A3, A26).
    if matches!(
        connection.greeting.mode,
        Some(curvyo_connector_core::ConnectorMode::Maintenance { .. })
    ) {
        return PairAnswer::Failed {
            error: LinkError::InMaintenance,
        };
    }
    let record = match connection.pair(&code, &client_name(), deadline) {
        Ok(record) => record,
        Err(e) => return e.into(),
    };
    let saved = open.with_keychain(&app, |store| {
        store.set(&record_name(&profile_id), &record.to_bytes())
    });
    match saved {
        Ok(Ok(())) => PairAnswer::Paired {
            connector_name: record.connector_name,
            fingerprint: record.spki.fingerprint(),
        },
        Ok(Err(e)) => PairAnswer::Failed {
            error: LinkError::other(&e),
        },
        Err(problem) => PairAnswer::Failed {
            error: store_error(problem),
        },
    }
}

/// Forget pairing (Q10a): the record goes, the watcher stops.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_forget(
    app: AppHandle,
    open: State<'_, OpenStore>,
    state: State<'_, ConnectorState>,
    profile_id: String,
) -> Result<(), String> {
    if !valid_id(&profile_id) {
        return Err("not a machine profile id".to_owned());
    }
    stop_watcher(&state, &profile_id);
    match open.with_keychain(&app, |store| forget_record(store, &profile_id)) {
        Ok(result) => result.map_err(|e| e.to_string()),
        Err(StoreProblem::NoStore) => Ok(()),
        Err(StoreProblem::Failed(message)) => Err(message),
    }
}

fn job_dir_file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join(JOB_DIR_FILE))
}

fn pick_job_file(app: &AppHandle) -> Option<PathBuf> {
    let mut dialog = app
        .dialog()
        .file()
        .set_title("Send job")
        .add_filter("G-code and text", &["gcode", "nc", "txt"])
        .add_filter("All files", &["*"]);
    if let Some(dir) = job_dir_file(app)
        .and_then(|file| std::fs::read_to_string(file).ok())
        .map(PathBuf::from)
        .filter(|dir| dir.is_dir())
    {
        dialog = dialog.set_directory(dir);
    }
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&window);
    }
    let path = dialog.blocking_pick_file()?.into_path().ok()?;
    if let (Some(file), Some(dir)) = (job_dir_file(app), path.parent()) {
        let _ = curvyo_storage_io::ensure_parent_dir(&file);
        let _ = curvyo_storage_io::write_atomic(&file, dir.to_string_lossy().as_bytes());
    }
    Some(path)
}

fn send_file(
    connection: &mut curvyo_connector_io::client::Connection,
    path: &std::path::Path,
    name: &str,
) -> Result<curvyo_connector_io::job_send::SentJob, FileSendError> {
    let deadline = Instant::now() + SEND_TIMEOUT;
    let io = |_: std::io::Error| FileSendError::Unreadable;
    let size = std::fs::metadata(path).map_err(io)?.len();
    if size > MAX_JOB_BYTES {
        connection.announce_oversized(name, size, deadline)?;
        return Err(FileSendError::Send(SendError::Client(
            ClientError::Protocol("the connector accepted an oversized job".to_owned()),
        )));
    }
    let bytes = std::fs::read(path).map_err(io)?;
    // A byte outside UTF-8 becomes U+FFFD, which the connector refuses as "not ASCII".
    let text = String::from_utf8_lossy(&bytes);
    Ok(connection.send_job(name, &text, deadline)?)
}

/// Send job (Q13): the OS file dialog, then the transfer.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_send_job(
    app: AppHandle,
    open: State<'_, OpenStore>,
    profile_id: String,
    address: String,
) -> SendAnswer {
    let record = match load_record(&app, &open, &profile_id) {
        Ok(record) => record,
        Err(error) => return SendAnswer::Failed { error },
    };
    let Some(path) = pick_job_file(&app) else {
        return SendAnswer::Cancelled;
    };
    let name = path
        .file_name()
        .map_or_else(|| "job".to_owned(), |n| n.to_string_lossy().into_owned());
    emit(&app, &profile_id, WatchView::Sending { name: name.clone() });
    let mut connection = match open_paired_on(
        &address,
        DEFAULT_PORT,
        record.spki,
        &record.token,
        CONNECT_TIMEOUT,
    ) {
        Ok((connection, _)) => connection,
        Err(e) => return SendAnswer::Failed { error: e.into() },
    };
    match send_file(&mut connection, &path, &name) {
        Ok(sent) => SendAnswer::Ready {
            name,
            lines: sent.lines,
            bytes: sent.bytes,
        },
        Err(e) => SendAnswer::from_error(&name, e),
    }
}

/// The frame that waits or runs, from the status.
fn frame_id(status: &Message) -> Option<Uuid> {
    match status {
        Message::Status { frame_id, .. } => *frame_id,
        _ => None,
    }
}

/// The message a job button sends; `None` for an unknown command or a Start with no Ready job.
fn command_message(
    command: &str,
    status: &Message,
    list: Option<&Message>,
    jog: Option<JogRequest>,
    point: Option<PointRequest>,
) -> Option<Message> {
    Some(match command {
        "start" => Message::Start {
            job_id: ready_job_id(status, list)?,
        },
        "pause" => Message::Pause,
        "resume" => Message::Resume,
        "stop" => Message::Stop,
        // Leaves GRBL's alarm lock (`specs/0127` G14, Frage 239 B); only in `Alarm`.
        "unlock" => Message::Unlock,
        // The homing cycle (`specs/0118`); the connector refuses it while a job runs.
        "home" => Message::Home,
        // One step of the jog pad (`specs/0118` 16); a held press has its own commands.
        "jog" => jog?.message(false),
        // A press of the Focus point mode (`specs/0118` 34, adrs P10); owners only.
        "go_to_point" => point?.message()?,
        // The owner confirms a member's waiting frame with the frame's id (`specs/0118` 2b, F5).
        "confirm_frame" => Message::Start {
            job_id: frame_id(status)?,
        },
        _ => return None,
    })
}

/// Start, pause, resume, stop, unlock, home or go to point, over a fresh connection (Q15, Q16; `0127` G14;
/// `0118`).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_command(
    app: AppHandle,
    open: State<'_, OpenStore>,
    profile_id: String,
    address: String,
    command: String,
    jog: Option<JogRequest>,
    point: Option<PointRequest>,
) -> CommandAnswer {
    let record = match load_record(&app, &open, &profile_id) {
        Ok(record) => record,
        Err(error) => return CommandAnswer::Failed { error },
    };
    let (mut connection, status) = match open_paired_on(
        &address,
        DEFAULT_PORT,
        record.spki,
        &record.token,
        CONNECT_TIMEOUT,
    ) {
        Ok(opened) => opened,
        Err(e) => return CommandAnswer::Failed { error: e.into() },
    };
    let list = connection.job_list.as_ref();
    let Some(message) = command_message(&command, &status, list, jog, point) else {
        return CommandAnswer::WrongState;
    };
    match connection.command(&message, job_state(&status)) {
        Ok(_) => CommandAnswer::Done,
        Err(e) => CommandAnswer::from_error(e),
    }
}

/// A press in the job list (`specs/0132`): move, remove or run next automatically.
#[derive(Debug, serde::Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ListCall {
    Move { job_id: Uuid, from: u32, to: u32 },
    Remove { job_id: Uuid },
    RunNext { job_id: Uuid, on: bool },
}

impl ListCall {
    fn message(&self) -> Message {
        match *self {
            Self::Move { job_id, from, to } => Message::MoveJob { job_id, from, to },
            Self::Remove { job_id } => Message::RemoveJob { job_id },
            Self::RunNext { job_id, on } => Message::SetRunNext { job_id, on },
        }
    }
}

/// How a press in the list ended.
#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ListAnswer {
    /// The connector took it; the list arrives on the watcher.
    Done,
    /// The job is not where the app saw it, or it is gone: the list is read again.
    Stale,
    /// Only an owner may.
    OwnerRequired,
    Failed {
        error: LinkError,
    },
}

/// Moves, removes or flags a job of the list over a fresh connection.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_list_command(
    app: AppHandle,
    open: State<'_, OpenStore>,
    profile_id: String,
    address: String,
    call: ListCall,
) -> ListAnswer {
    let record = match load_record(&app, &open, &profile_id) {
        Ok(record) => record,
        Err(error) => return ListAnswer::Failed { error },
    };
    let (mut connection, status) = match open_paired_on(
        &address,
        DEFAULT_PORT,
        record.spki,
        &record.token,
        CONNECT_TIMEOUT,
    ) {
        Ok(opened) => opened,
        Err(e) => return ListAnswer::Failed { error: e.into() },
    };
    match connection.command(&call.message(), job_state(&status)) {
        Ok(_) => ListAnswer::Done,
        Err(ClientError::Protocol(code)) if code == "WrongState" || code == "UnknownJob" => {
            ListAnswer::Stale
        }
        Err(ClientError::Protocol(code)) if code == "OwnerRequired" => ListAnswer::OwnerRequired,
        Err(e) => ListAnswer::Failed { error: e.into() },
    }
}

/// What the watcher sends to the page.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatchPayload {
    profile_id: String,
    #[serde(flatten)]
    event: WatchView,
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum WatchView {
    Status {
        status: Message,
    },
    /// The controller changed (`specs/0127`: identified after a reconnect, lost).
    Controller {
        controller: Option<ControllerInfo>,
    },
    /// The connector's job list: the first one after connecting, then every change.
    JobList {
        list: Message,
    },
    Sending {
        name: String,
    },
    Lost,
    Failed {
        error: LinkError,
    },
}

fn emit(app: &AppHandle, profile_id: &str, event: WatchView) {
    let _ = app.emit(
        crate::CONNECTOR_EVENT,
        WatchPayload {
            profile_id: profile_id.to_owned(),
            event,
        },
    );
}

fn stop_watcher(state: &ConnectorState, profile_id: &str) {
    if let Some(stop) = state
        .watchers
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(profile_id)
    {
        stop.store(true, Ordering::Relaxed);
    }
}

/// Starts the standing connection of `profile_id` while the page shows it; a running one of the
/// same profile is replaced. It tries again only while a job is in flight or a send awaits its
/// answer (`0125` B5).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_watch(
    app: AppHandle,
    open: State<'_, OpenStore>,
    state: State<'_, ConnectorState>,
    profile_id: String,
    address: String,
) {
    stop_watcher(&state, &profile_id);
    let Ok(record) = load_record(&app, &open, &profile_id) else {
        return;
    };
    let stop = Arc::new(AtomicBool::new(false));
    state
        .watchers
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(profile_id.clone(), Arc::clone(&stop));
    std::thread::spawn(move || {
        let state = app.state::<ConnectorState>();
        watch(
            &address,
            record.spki,
            &record.token,
            RETRY,
            &stop,
            &|| send_pending(&state.pending, &profile_id),
            |event| {
                let event = match event {
                    WatchEvent::Status(status) => {
                        let view = WatchView::Status {
                            status: status.clone(),
                        };
                        emit(&app, &profile_id, view);
                        report_resolution(&app, &state.pending, &profile_id, &status);
                        return;
                    }
                    WatchEvent::JobList(list) => {
                        emit(&app, &profile_id, WatchView::JobList { list: list.clone() });
                        report_resolution(&app, &state.pending, &profile_id, &list);
                        return;
                    }
                    WatchEvent::Controller(controller) => WatchView::Controller { controller },
                    WatchEvent::Lost => WatchView::Lost,
                    WatchEvent::Failed(e) => WatchView::Failed { error: e.into() },
                };
                emit(&app, &profile_id, event);
            },
        );
    });
}

/// Ends the standing connection of `profile_id` (its last viewer is gone).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn connector_unwatch(state: State<'_, ConnectorState>, profile_id: String) {
    stop_watcher(&state, &profile_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_connector_core::{JogAxis, JogDirection, JogStep};
    use curvyo_connector_io::hub::{ControllerChoice, FakeSettings};
    use curvyo_connector_io::server::{RunningServer, ServerConfig};

    #[test]
    fn the_record_is_kept_under_the_profile_id_and_forgetting_removes_only_it() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::file(dir.path(), "pass").unwrap();
        store.set(&record_name("aaa"), b"one").unwrap();
        store.set(&record_name("bbb"), b"two").unwrap();
        forget_record(&store, "aaa").unwrap();
        forget_record(&store, "never").unwrap();
        assert!(store.get("connector/aaa").unwrap().is_none());
        assert!(store.get("connector/bbb").unwrap().is_some());
    }

    /// The customer's path: a bare IP literal in the address field, the connector on the port
    /// the app falls back to (a free one here instead of 7447, the same code otherwise).
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
        let address = server.addr().to_string();
        let record = connect(&address, PinMode::Capture, CONNECT_TIMEOUT)
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
    fn test_connection_with_a_bare_ip_reaches_the_connector_on_the_default_port() {
        let (server, _dir, record) = rig();
        let answer = run_test("127.0.0.1", server.addr().port(), &record);
        assert!(matches!(answer, TestAnswer::Connected { .. }), "{answer:?}");
    }

    #[test]
    fn test_connection_of_an_unpaired_profile_tells_maintenance_mode_from_not_paired() {
        let (server, _dir, _record) = rig();
        let answer = run_test_unpaired("127.0.0.1", server.addr().port());
        assert!(
            matches!(
                answer,
                TestAnswer::Failed {
                    error: LinkError::NotPaired
                }
            ),
            "{answer:?}"
        );
        let dir = tempfile::tempdir().unwrap();
        let service = RunningServer::start_service(&curvyo_connector_io::server::ServiceConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            state_dir: dir.path().to_owned(),
            host_name: "workshop-pi".into(),
            timing: curvyo_connector_io::server::ServerTiming::default(),
        })
        .unwrap();
        let answer = run_test_unpaired("127.0.0.1", service.addr().port());
        let TestAnswer::Maintenance { protocol, version } = answer else {
            panic!("{answer:?}");
        };
        assert_eq!(protocol, 0);
        assert_ne!(version, "");
    }

    #[test]
    fn test_connection_with_garbage_says_not_a_valid_address() {
        let (_server, _dir, record) = rig();
        let answer = run_test("not an address", DEFAULT_PORT, &record);
        assert!(matches!(
            answer,
            TestAnswer::Failed {
                error: LinkError::BadAddress
            }
        ));
    }

    #[test]
    fn test_connection_with_nothing_listening_says_nothing_answers_at_the_dialled_address() {
        let (server, _dir, record) = rig();
        let port = server.addr().port();
        drop(server);
        let answer = run_test("127.0.0.1", port, &record);
        let TestAnswer::Failed {
            error: LinkError::Unreachable { address },
        } = answer
        else {
            panic!("{answer:?}");
        };
        assert_eq!(address, format!("127.0.0.1:{port}"));
    }

    fn status(state: JobState, job_id: Option<Uuid>) -> Message {
        Message::Status {
            state,
            job: job_id.map(|job_id| curvyo_connector_core::Progress {
                job_id,
                line: 0,
                lines: 3,
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
            frame_id: None,
            target: None,
        }
    }

    #[test]
    fn start_takes_the_job_id_from_a_ready_status_only() {
        let id = Uuid::nil();
        assert_eq!(
            ready_job_id(&status(JobState::Ready, Some(id)), None),
            Some(id)
        );
        assert_eq!(ready_job_id(&status(JobState::Idle, None), None), None);
        assert_eq!(
            ready_job_id(&status(JobState::Running, Some(id)), None),
            None
        );
        // `specs/0128` A26: an owner's Confirm start takes the id of the start a member requested;
        // the foreground wait for Enter carries no requester and is not confirmed from the app.
        let mut waiting = status(JobState::AwaitingLocalStart, Some(id));
        assert_eq!(ready_job_id(&waiting, None), None);
        if let Message::Status { requested_by, .. } = &mut waiting {
            *requested_by = Some("Phone".into());
        }
        assert_eq!(ready_job_id(&waiting, None), Some(id));
    }

    #[test]
    fn jog_maps_to_one_step_message_and_needs_its_request() {
        let request = JogRequest {
            axis: JogAxis::Y,
            direction: JogDirection::Minus,
            step: JogStep::Tenth,
        };
        let idle = status(JobState::Idle, None);
        assert_eq!(
            command_message("jog", &idle, None, Some(request), None),
            Some(Message::Jog {
                axis: JogAxis::Y,
                direction: JogDirection::Minus,
                step: JogStep::Tenth,
                hold: false,
            })
        );
        assert_eq!(command_message("jog", &idle, None, None, None), None);
        // The request's words are the wire's: closed sets, never a number.
        let parsed: JogRequest =
            serde_json::from_str(r#"{"axis":"X","direction":"Plus","step":"0.1"}"#).unwrap();
        assert_eq!(parsed.step, JogStep::Tenth);
        assert!(
            serde_json::from_str::<JogRequest>(r#"{"axis":"X","direction":"Plus","step":"7"}"#)
                .is_err()
        );
    }

    #[test]
    fn home_maps_to_the_home_message_in_any_state_and_unknown_words_to_none() {
        for state in [
            JobState::Idle,
            JobState::Ready,
            JobState::Alarm,
            JobState::Running,
        ] {
            assert_eq!(
                command_message("home", &status(state, None), None, None, None),
                Some(Message::Home)
            );
        }
        let id = Uuid::nil();
        assert_eq!(
            command_message(
                "start",
                &status(JobState::Ready, Some(id)),
                None,
                None,
                None
            ),
            Some(Message::Start { job_id: id })
        );
        assert_eq!(
            command_message("start", &status(JobState::Idle, None), None, None, None),
            None
        );
        assert_eq!(
            command_message("reboot", &status(JobState::Idle, None), None, None, None),
            None
        );
    }
}
