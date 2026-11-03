//! The server's lifecycle over its HTTP routes (`/` WebSocket, `/healthz`): bind, startup scan,
//! relay, graceful shutdown closing every socket with code 1012 (ADR 0031 decision 11, criteria
//! 35 to 38).

use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message, WebSocket, WebSocketUpgrade, close_code};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use curvyo_sync_core::{MAX_MESSAGE_LEN, Relay, RestoreError};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

use crate::config::Config;
use crate::health::Health;
use crate::hub::{Hub, HubError, Outgoing, ToHub};
use crate::log::Log;
use crate::store::{Store, StoreError};

/// Why the server could not start or did not stop cleanly.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// The listen address could not be bound.
    #[error("cannot listen on {0}: {1}")]
    Bind(SocketAddr, io::Error),
    /// The data directory could not be read.
    #[error("startup scan: {0}")]
    Store(#[from] StoreError),
    /// The stored rooms are not a valid relay state.
    #[error("startup scan: {0}")]
    Restore(#[from] RestoreError),
    /// The relay stopped on an error.
    #[error(transparent)]
    Hub(#[from] HubError),
    /// Serving HTTP failed.
    #[error("serving: {0}")]
    Serve(io::Error),
    /// A server task panicked.
    #[error("a server task panicked")]
    Panicked,
}

#[derive(Clone)]
struct AppState {
    hub: mpsc::UnboundedSender<ToHub>,
    health: Arc<Health>,
    stop: watch::Receiver<bool>,
}

/// A started server.
pub struct Server {
    addr: SocketAddr,
    stop: watch::Sender<bool>,
    serve: JoinHandle<io::Result<()>>,
    hub: JoinHandle<Result<(), HubError>>,
    log: Log,
}

impl Server {
    /// Binds the listen address, serves `/healthz` (503 meanwhile), scans the data directory and
    /// restores every room, then starts the relay and turns ready.
    ///
    /// # Errors
    /// See [`ServerError`].
    pub async fn start(config: &Config, log: Log) -> Result<Self, ServerError> {
        let listener = TcpListener::bind(config.listen)
            .await
            .map_err(|e| ServerError::Bind(config.listen, e))?;
        let addr = listener
            .local_addr()
            .map_err(|e| ServerError::Bind(config.listen, e))?;
        let (stop, stop_rx) = watch::channel(false);
        let (hub_tx, hub_rx) = mpsc::unbounded_channel();
        let health = Arc::new(Health::new(config.data.clone()));
        let state = AppState {
            hub: hub_tx,
            health: Arc::clone(&health),
            stop: stop_rx.clone(),
        };
        let serve = tokio::spawn(serve(listener, state, stop_rx));
        log.line(&format!("listening on {addr}"));

        let (store, relay) = match scan(config.data.clone()).await {
            Ok(found) => found,
            Err(e) => {
                let _ = stop.send(true);
                let _ = serve.await;
                return Err(e);
            }
        };
        log.line(&format!(
            "startup scan done: {} rooms",
            relay.room_ids().len()
        ));
        let hub = Hub::new(relay, store);
        let failed = stop.clone();
        let hub_log = log.clone();
        let hub = tokio::task::spawn_blocking(move || {
            let result = hub.run(hub_rx);
            if let Err(e) = &result {
                hub_log.line(&format!("stopping: {e}"));
                let _ = failed.send(true);
            }
            result
        });
        health.scanned();
        Ok(Self {
            addr,
            stop,
            serve,
            hub,
            log,
        })
    }

    /// The address the server listens on (the port when started on port 0).
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Completes when the relay stopped on an error, so the caller shuts down.
    pub async fn failed(&self) {
        let mut stop = self.stop.subscribe();
        let _ = stop.wait_for(|stop| *stop).await;
    }

    /// Shuts down: stops accepting, closes every socket with code 1012, finishes the appends
    /// already received, and returns.
    ///
    /// # Errors
    /// A relay or serving error.
    pub async fn shutdown(self) -> Result<(), ServerError> {
        self.log.line("shutting down");
        let _ = self.stop.send(true);
        let served = self.serve.await.map_err(|_| ServerError::Panicked)?;
        let hub = self.hub.await.map_err(|_| ServerError::Panicked)?;
        hub?;
        served.map_err(ServerError::Serve)?;
        self.log.line("stopped");
        Ok(())
    }
}

/// The startup scan, off the async threads.
async fn scan(data: PathBuf) -> Result<(Store, Relay), ServerError> {
    tokio::task::spawn_blocking(move || {
        let (store, rooms) = Store::open(&data)?;
        let relay = Relay::restore(rooms)?;
        Ok((store, relay))
    })
    .await
    .map_err(|_| ServerError::Panicked)?
}

async fn serve(
    listener: TcpListener,
    state: AppState,
    mut stop: watch::Receiver<bool>,
) -> io::Result<()> {
    let app = Router::new()
        .route("/", get(upgrade))
        .route("/healthz", get(healthz))
        .with_state(state);
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = stop.wait_for(|stop| *stop).await;
        })
        .await
}

async fn healthz(State(state): State<AppState>) -> (StatusCode, &'static str) {
    let health = Arc::clone(&state.health);
    let ready = tokio::task::spawn_blocking(move || health.is_ready())
        .await
        .unwrap_or(false);
    if ready {
        (StatusCode::OK, "ready\n")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready\n")
    }
}

async fn upgrade(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    // One over the limit still reaches the relay, which answers `Refused{too-large}`.
    let limit = MAX_MESSAGE_LEN + 1;
    ws.max_message_size(limit)
        .max_frame_size(limit)
        .on_upgrade(move |socket| connection(socket, state))
}

/// One WebSocket: messages in to the hub, the hub's frames out, closed with 1012 on shutdown.
async fn connection(mut socket: WebSocket, state: AppState) {
    let AppState { hub, mut stop, .. } = state;
    let (outgoing, mut from_hub) = mpsc::unbounded_channel();
    let (reply, number) = oneshot::channel();
    if hub.send(ToHub::Open { outgoing, reply }).is_err() {
        return;
    }
    let Ok(connection) = number.await else {
        return;
    };
    loop {
        tokio::select! {
            biased;
            () = async { let _ = stop.wait_for(|stop| *stop).await; } => {
                let frame = CloseFrame {
                    code: close_code::RESTART,
                    reason: "server restart".into(),
                };
                let _ = socket.send(Message::Close(Some(frame))).await;
                break;
            }
            out = from_hub.recv() => match out {
                Some(Outgoing::Frame(bytes)) => {
                    if socket.send(Message::Binary(bytes.into())).await.is_err() {
                        break;
                    }
                }
                Some(Outgoing::Close) | None => {
                    let _ = socket.send(Message::Close(None)).await;
                    break;
                }
            },
            received = socket.recv() => match received {
                Some(Ok(Message::Binary(bytes))) => {
                    let message = ToHub::Message { connection, bytes: bytes.to_vec() };
                    if hub.send(message).is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                // Text is not part of the protocol; a closed or failed socket ends the task.
                Some(Ok(Message::Text(_) | Message::Close(_)) | Err(_)) | None => break,
            },
        }
    }
    let _ = hub.send(ToHub::Gone { connection });
}
