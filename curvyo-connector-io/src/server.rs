//! The connector server: the listener, one thread per connection, the supervisor of the
//! controller's thread (the fake's streamer or the serial port) and the handle the binary and the
//! tests drive, in foreground mode or as a service from its state directory.

use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::clients::ClientsError;
use curvyo_connector_core::{ConnectorConfig, State};

use crate::config_file::{self, ConfigFileError};
use crate::grbl::Timing;
use crate::hub::{ControllerChoice, ExitReason, Hub, HubMode, HubSettings};
use crate::identity::{self, IdentityError};
use crate::maintenance::{MaintenanceWindow, WINDOW};
use crate::server_tls::server_config;
use crate::session;
use crate::spki::SpkiHash;
use crate::state_dir::{self, Lock, StateDirError};
use crate::supervisor;

/// Most connections at once; the next one is closed after accept.
pub const MAX_CONNECTIONS: usize = 8;

/// What a server is started with.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Where to listen; port 0 picks a free one.
    pub listen: SocketAddr,
    /// The state directory.
    pub state_dir: PathBuf,
    /// The controller.
    pub controller: ControllerChoice,
    /// `Start` waits for the connector's terminal.
    pub require_local_start: bool,
    /// The name `Welcome` carries.
    pub connector_name: String,
    /// Issue a pairing code at start even when an app is paired.
    pub issue_code: bool,
}

/// What a connector run as a service is started with: everything else is in `connector.toml`.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Where to listen unless `connector.toml` says; port 0 picks a free one.
    pub listen: SocketAddr,
    /// The state directory.
    pub state_dir: PathBuf,
    /// The name `Welcome` carries before a configuration names one (the host name).
    pub host_name: String,
    /// The limits (the tests shorten them).
    pub timing: ServerTiming,
}

/// The limits of a service connector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerTiming {
    /// How long maintenance mode takes `ApplyConfig` (30 minutes).
    pub maintenance_window: Duration,
    /// The GRBL controller's limits.
    pub serial: Timing,
}

impl Default for ServerTiming {
    fn default() -> Self {
        Self {
            maintenance_window: WINDOW,
            serial: Timing::default(),
        }
    }
}

/// The controller a configuration names.
#[must_use]
pub fn choice_of(
    config: &curvyo_connector_core::ControllerConfig,
    timing: Timing,
) -> ControllerChoice {
    use curvyo_connector_core::ControllerConfig as C;
    match config {
        C::GrblSerial { port, baud } => ControllerChoice::Serial(crate::hub::SerialSettings {
            port: port.clone(),
            baud: *baud,
            spindle: false,
            timing,
        }),
        C::Fake {
            lines_per_second,
            alarm_at_line,
        } => ControllerChoice::Fake(crate::hub::FakeSettings {
            lines_per_second: *lines_per_second,
            alarm_at_line: *alarm_at_line,
        }),
    }
}

/// Why the server did not start.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// The state directory is held by another connector or cannot be used.
    #[error(transparent)]
    StateDir(#[from] StateDirError),
    /// `connector.toml` is unreadable, damaged or invalid; the start is refused (addendum D8).
    #[error("{}: {source}", .path.display())]
    Config {
        /// The file.
        path: PathBuf,
        /// The fault.
        source: ConfigFileError,
    },
    /// The key file.
    #[error(transparent)]
    Identity(#[from] IdentityError),
    /// The list of paired apps.
    #[error(transparent)]
    Clients(#[from] ClientsError),
    /// The TLS setup.
    #[error("TLS setup failed: {0}")]
    Tls(#[from] rustls::Error),
    /// The address could not be bound.
    #[error("cannot listen on {addr}: {source}")]
    Bind {
        /// The address.
        addr: SocketAddr,
        /// The cause.
        source: std::io::Error,
    },
    /// No random bytes for the pairing code.
    #[error("no random bytes are available")]
    Random,
}

/// A running connector.
pub struct RunningServer {
    hub: Arc<Hub>,
    addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
    threads: Mutex<Vec<JoinHandle<()>>>,
    notices: Mutex<Option<Receiver<String>>>,
    started: Instant,
    exits: Mutex<Option<Receiver<ExitReason>>>,
    exit_sender: Sender<ExitReason>,
    state_dir: PathBuf,
    _lock: Lock,
}

/// What `RunningServer::start_with` needs beyond the hub's settings.
struct Start<'a> {
    listen: SocketAddr,
    settings: HubSettings,
    mode: HubMode,
    issue_code: bool,
    timing: Timing,
    closes_at: String,
    state_dir: &'a std::path::Path,
}

impl RunningServer {
    /// Starts in foreground mode: makes or reads the key, binds and starts serving with the
    /// controller and start rule of the flags.
    ///
    /// # Errors
    /// [`ServerError`]; a damaged key file stops the start (Q3).
    pub fn start(config: &ServerConfig) -> Result<Self, ServerError> {
        let identity = identity::load_or_create(&config.state_dir)?;
        let settings = HubSettings {
            state_dir: config.state_dir.clone(),
            controller: config.controller.clone(),
            require_local_start: config.require_local_start,
            connector_name: config.connector_name.clone(),
            spki: identity.spki,
        };
        Self::start_with(
            &identity,
            Start {
                listen: config.listen,
                settings,
                mode: HubMode::Foreground,
                issue_code: config.issue_code,
                timing: Timing::default(),
                closes_at: String::new(),
                state_dir: &config.state_dir,
            },
        )
    }

    /// Starts as a service: configured from `connector.toml`, else in maintenance mode with a
    /// fresh setup window (A1, A31). A value `connector.toml` sets wins over `service.listen`.
    ///
    /// # Errors
    /// [`ServerError`]: a held lock, a bad `connector.toml` or key, a failed bind.
    pub fn start_service(service: &ServiceConfig) -> Result<Self, ServerError> {
        let dir = &service.state_dir;
        let stored = config_file::load(dir).map_err(|source| ServerError::Config {
            path: dir.join(config_file::CONFIG_FILE),
            source,
        })?;
        let identity = identity::load_or_create(dir)?;
        let opened = Instant::now();
        let (controller, listen, mode) = match stored {
            Some(config) => (
                choice_of(&config.controller, service.timing.serial),
                config.listen_addr().unwrap_or(service.listen),
                HubMode::Configured(Box::new(config)),
            ),
            None => (
                ControllerChoice::None,
                service.listen,
                HubMode::Maintenance(MaintenanceWindow {
                    opened,
                    length: service.timing.maintenance_window,
                }),
            ),
        };
        let settings = HubSettings {
            state_dir: dir.clone(),
            controller,
            require_local_start: false,
            connector_name: service.host_name.clone(),
            spki: identity.spki,
        };
        let closes_at = utc_clock(service.timing.maintenance_window);
        Self::start_with(
            &identity,
            Start {
                listen,
                settings,
                mode,
                issue_code: false,
                timing: service.timing.serial,
                closes_at,
                state_dir: dir,
            },
        )
    }

    fn start_with(identity: &identity::Identity, start: Start<'_>) -> Result<Self, ServerError> {
        let lock = state_dir::lock(start.state_dir)?;
        let tls = server_config(identity)?;
        let listener = TcpListener::bind(start.listen).map_err(|source| ServerError::Bind {
            addr: start.listen,
            source,
        })?;
        let bind_error = |source| ServerError::Bind {
            addr: start.listen,
            source,
        };
        let addr = listener.local_addr().map_err(bind_error)?;
        listener.set_nonblocking(true).map_err(bind_error)?;
        let started = Instant::now();
        let controller = start.settings.controller.clone();
        let foreground = start.mode == HubMode::Foreground;
        let (hub, notices) = Hub::new(start.settings, started);
        hub.set_mode(start.mode);
        let hub = Arc::new(hub);
        if foreground && (start.issue_code || !hub.has_clients()?) {
            hub.issue_code(started).ok_or(ServerError::Random)?;
        }
        let (exit_sender, exits) = channel();
        hub.set_exit(exit_sender.clone());
        let shutdown = Arc::new(AtomicBool::new(false));
        let threads = vec![
            spawn_acceptor(listener, tls, Arc::clone(&hub), Arc::clone(&shutdown)),
            supervisor::spawn(
                Arc::clone(&hub),
                controller,
                start.timing,
                start.closes_at,
                Arc::clone(&shutdown),
            ),
        ];
        Ok(Self {
            hub,
            addr,
            shutdown,
            threads: Mutex::new(threads),
            notices: Mutex::new(Some(notices)),
            started,
            exits: Mutex::new(Some(exits)),
            exit_sender,
            state_dir: start.state_dir.to_owned(),
            _lock: lock,
        })
    }

    /// The requests to end the program: an owner's `Reset`, or what [`Self::exit_sender`] sends;
    /// once.
    pub fn take_exits(&self) -> Option<Receiver<ExitReason>> {
        self.exits
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    /// Where a signal handler sends [`ExitReason::Signal`].
    #[must_use]
    pub fn exit_sender(&self) -> Sender<ExitReason> {
        self.exit_sender.clone()
    }

    /// The configuration, in configured mode.
    #[must_use]
    pub fn config(&self) -> Option<ConnectorConfig> {
        self.hub.config()
    }

    /// The mode.
    #[must_use]
    pub fn mode(&self) -> HubMode {
        self.hub.mode()
    }

    /// After a `Reset`: stops every thread and empties the state directory (A19); the lock goes
    /// last, with this value.
    ///
    /// # Errors
    /// The first file that could not be deleted.
    pub fn wipe(self) -> Result<(), StateDirError> {
        self.close();
        self.hub.freeze_store();
        state_dir::empty(&self.state_dir)
    }

    /// The address it listens on.
    #[must_use]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Hash of its key.
    #[must_use]
    pub fn spki(&self) -> SpkiHash {
        self.hub.spki()
    }

    /// The valid pairing code as printed (`XXXX-XXXX`), if there is one.
    #[must_use]
    pub fn current_code(&self) -> Option<String> {
        self.hub.current_code(Instant::now())
    }

    /// Issues a new code, replacing a valid one.
    #[must_use]
    pub fn new_code(&self) -> Option<String> {
        self.hub.issue_code(Instant::now())
    }

    /// The press of the start button on this host; the job's name if one started.
    #[must_use]
    pub fn local_start(&self) -> Option<String> {
        self.hub.local_start(Instant::now())
    }

    /// Stops a running or paused job or a go to point before the program exits (G16); the job's
    /// name, or "Focus point".
    #[must_use]
    pub fn stop_for_exit(&self) -> Option<String> {
        self.hub.stop_for_exit(Instant::now())
    }

    /// After [`RunningServer::stop_for_exit`] named something: waits until the controller
    /// confirmed the stop (at most G12's limits), so its reset goes out before the port closes.
    pub fn await_exit_stop(&self) {
        self.hub.await_exit_stop();
    }

    /// The job state.
    #[must_use]
    pub fn state(&self) -> State {
        self.hub.state()
    }

    /// The lines for the terminal (job outcomes, "Press Enter to start ..."); once.
    pub fn take_notices(&self) -> Option<Receiver<String>> {
        self.notices
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }

    /// Time since the server started.
    #[must_use]
    pub fn uptime(&self) -> Duration {
        self.started.elapsed()
    }

    /// Stops the threads and waits for them.
    pub fn shutdown(self) {
        self.stop();
    }

    /// Stops the threads, waits for them (the serial thread's last write and flush included)
    /// and ends the notices, for a program that exits while others still hold the server.
    pub fn close(&self) {
        self.stop();
        self.hub.close_notices();
    }

    fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let threads =
            std::mem::take(&mut *self.threads.lock().unwrap_or_else(PoisonError::into_inner));
        for thread in threads {
            let _ = thread.join();
        }
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.stop();
    }
}

struct Slot(Arc<AtomicUsize>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn spawn_acceptor(
    listener: TcpListener,
    tls: Arc<rustls::ServerConfig>,
    hub: Arc<Hub>,
    shutdown: Arc<AtomicBool>,
) -> JoinHandle<()> {
    let open = Arc::new(AtomicUsize::new(0));
    std::thread::spawn(move || {
        while !shutdown.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, peer)) => {
                    if open.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS
                        || stream.set_nonblocking(false).is_err()
                    {
                        open.fetch_sub(1, Ordering::SeqCst);
                        continue;
                    }
                    let (tls, hub, shutdown) =
                        (Arc::clone(&tls), Arc::clone(&hub), Arc::clone(&shutdown));
                    let slot = Slot(Arc::clone(&open));
                    std::thread::spawn(move || {
                        let _slot = slot;
                        session::serve(stream, peer.ip(), &tls, hub, &shutdown);
                    });
                }
                Err(_) => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    })
}

/// "14:32 UTC": the wall clock `after` from now, for the maintenance lines (A1, A5).
#[must_use]
pub fn utc_clock(after: Duration) -> String {
    let at = time::OffsetDateTime::now_utc() + after;
    format!("{:02}:{:02} UTC", at.hour(), at.minute())
}
