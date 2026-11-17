//! The connector server: the listener, one thread per connection, the streamer thread and the
//! handle the binary and the tests drive.

use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::clients::ClientsError;
use crate::hub::{FakeSettings, Hub, HubSettings, TICK};
use crate::identity::{self, IdentityError};
use crate::server_tls::server_config;
use crate::session;
use crate::spki::SpkiHash;

/// Most connections at once; the next one is closed after accept.
pub const MAX_CONNECTIONS: usize = 8;

/// What a server is started with.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Where to listen; port 0 picks a free one.
    pub listen: SocketAddr,
    /// The state directory.
    pub state_dir: PathBuf,
    /// The fake controller, if one is attached.
    pub fake: Option<FakeSettings>,
    /// `Start` waits for the connector's terminal.
    pub require_local_start: bool,
    /// The name `Welcome` carries.
    pub connector_name: String,
    /// Issue a pairing code at start even when an app is paired.
    pub issue_code: bool,
}

/// Why the server did not start.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
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
    threads: Vec<JoinHandle<()>>,
    notices: Mutex<Option<Receiver<String>>>,
    started: Instant,
}

impl RunningServer {
    /// Makes or reads the key, binds and starts serving.
    ///
    /// # Errors
    /// [`ServerError`]; a damaged key file stops the start (Q3).
    pub fn start(config: &ServerConfig) -> Result<Self, ServerError> {
        let identity = identity::load_or_create(&config.state_dir)?;
        let tls = server_config(&identity)?;
        let listener = TcpListener::bind(config.listen).map_err(|source| ServerError::Bind {
            addr: config.listen,
            source,
        })?;
        let bind_error = |source| ServerError::Bind {
            addr: config.listen,
            source,
        };
        let addr = listener.local_addr().map_err(bind_error)?;
        listener.set_nonblocking(true).map_err(bind_error)?;
        let started = Instant::now();
        let (hub, notices) = Hub::new(
            HubSettings {
                state_dir: config.state_dir.clone(),
                fake: config.fake,
                require_local_start: config.require_local_start,
                connector_name: config.connector_name.clone(),
                spki: identity.spki,
            },
            started,
        );
        let hub = Arc::new(hub);
        if config.issue_code || !hub.has_clients()? {
            hub.issue_code(started).ok_or(ServerError::Random)?;
        }
        let shutdown = Arc::new(AtomicBool::new(false));
        let threads = vec![
            spawn_acceptor(listener, tls, Arc::clone(&hub), Arc::clone(&shutdown)),
            spawn_streamer(Arc::clone(&hub), Arc::clone(&shutdown)),
        ];
        Ok(Self {
            hub,
            addr,
            shutdown,
            threads,
            notices: Mutex::new(Some(notices)),
            started,
        })
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
    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        for thread in self.threads.drain(..) {
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
                Ok((stream, _)) => {
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
                        session::serve(stream, &tls, hub, &shutdown);
                    });
                }
                Err(_) => std::thread::sleep(Duration::from_millis(20)),
            }
        }
    })
}

fn spawn_streamer(hub: Arc<Hub>, shutdown: Arc<AtomicBool>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        while !shutdown.load(Ordering::Relaxed) {
            std::thread::sleep(TICK);
            hub.tick(Instant::now());
        }
    })
}
