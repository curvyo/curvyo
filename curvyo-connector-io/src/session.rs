//! One connection of the connector: the peer check, TLS, the WebSocket, the hello, the access
//! gate, login and pairing, and the relay of messages to the hub.

#[cfg(test)]
mod gate_tests;
mod motion;
mod owner;
mod pairing;

use std::net::{IpAddr, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use curvyo_connector_core::{
    ErrorCode, Handshake, Message, Mode, Outcome, PROTOCOL_VERSION, Peer, Role, admit, refusal,
};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{HandshakeError, WebSocket};

use crate::frames::{self, Frame, IoFrame, PATH, RefusalReason, SOCKET_LIMIT};
use crate::hub::Hub;
use crate::maintenance::is_local;
use crate::spake::SessionKey;
use crate::status_limit::StatusLimiter;

/// How long the TLS and WebSocket handshakes and the first frame may take.
const FIRST_FRAME: Duration = Duration::from_secs(10);
/// How long a pairing exchange may wait for the app's next frame.
const PAIRING_IDLE: Duration = Duration::from_secs(10);
/// How long a connection that is neither authenticated nor pairing may stay silent.
const UNAUTHENTICATED_IDLE: Duration = Duration::from_secs(10);
/// The read timeout: how often the loop looks at the status channel.
const POLL: Duration = Duration::from_millis(50);
/// The longest `client_name`.
const MAX_NAME: usize = 64;

type Socket = WebSocket<StreamOwned<ServerConnection, TcpStream>>;

struct Pairing {
    key: SessionKey,
    client_name: String,
    deadline: Instant,
}

struct Session {
    hub: Arc<Hub>,
    handshake: Handshake,
    greeted: bool,
    updates: Option<Receiver<Message>>,
    limiter: StatusLimiter,
    pairing: Option<Pairing>,
    /// The jobs this connection announced.
    submitted: Vec<uuid::Uuid>,
    /// When the last frame came in.
    last_frame: Instant,
    /// The logged-in client: its name and role.
    peer: Option<Peer>,
    /// The address the connection came from.
    peer_ip: IpAddr,
    /// Set by a revoke of this client: the connection closes at its next turn (A11).
    close: Option<Arc<AtomicBool>>,
    /// The hub's maintenance epoch this connection was greeted in; it closes when it moves (A6).
    maintenance_epoch: Option<u64>,
    /// This connection started a held jog and has not released it: closing ends it (J4).
    holding: bool,
}

#[allow(clippy::result_large_err)] // the signature tungstenite's callback asks for
fn not_found(_: &Request, _: Response) -> Result<Response, ErrorResponse> {
    Err(tungstenite::http::Response::builder()
        .status(404)
        .body(None)
        .unwrap_or_default())
}

#[allow(clippy::result_large_err)] // the signature tungstenite's callback asks for
fn accept_path(request: &Request, response: Response) -> Result<Response, ErrorResponse> {
    if request.uri().path() == PATH {
        Ok(response)
    } else {
        not_found(request, response)
    }
}

/// Serves one accepted connection from `peer_ip` until it ends or `shutdown` is set. In
/// maintenance mode a peer that is not local is closed before TLS (A4).
pub fn serve(
    stream: TcpStream,
    peer_ip: IpAddr,
    tls: &Arc<ServerConfig>,
    hub: Arc<Hub>,
    shutdown: &AtomicBool,
) {
    let maintenance = matches!(hub.access_mode(Instant::now()), Mode::Maintenance { .. });
    if maintenance && !is_local(peer_ip) {
        return;
    }
    if stream.set_read_timeout(Some(POLL)).is_err() || stream.set_nodelay(true).is_err() {
        return;
    }
    let Ok(connection) = ServerConnection::new(Arc::clone(tls)) else {
        return;
    };
    let config = WebSocketConfig::default()
        .max_frame_size(Some(SOCKET_LIMIT))
        .max_message_size(Some(SOCKET_LIMIT));
    let deadline = Instant::now() + FIRST_FRAME;
    let mut attempt = tungstenite::accept_hdr_with_config(
        StreamOwned::new(connection, stream),
        accept_path,
        Some(config),
    );
    let mut socket = loop {
        match attempt {
            Ok(socket) => break socket,
            Err(HandshakeError::Interrupted(mid)) => {
                if Instant::now() >= deadline || shutdown.load(Ordering::Relaxed) {
                    return;
                }
                attempt = mid.handshake();
            }
            Err(HandshakeError::Failure(_)) => return,
        }
    };
    let mut session = Session {
        hub,
        handshake: Handshake::new(),
        greeted: false,
        updates: None,
        limiter: StatusLimiter::default(),
        pairing: None,
        submitted: Vec::new(),
        last_frame: Instant::now(),
        peer: None,
        peer_ip,
        close: None,
        maintenance_epoch: None,
        holding: false,
    };
    session.run(&mut socket, deadline, shutdown);
    if !session.submitted.is_empty() {
        session
            .hub
            .client_disconnected(&session.submitted, Instant::now());
    }
    if session.holding
        && let Some(peer) = &session.peer
    {
        // A closed connection ends the hold it started at once (`specs/0118` criterion 18).
        let _ = session.hub.jog(Message::JogStop, peer, Instant::now());
    }
    if session.pairing.take().is_some() {
        session.hub.pair_failed();
    }
}

/// Whether a connection without a session has been silent for `limit` (a pure check, so the
/// timing is tested without waiting).
fn silent_too_long(
    authenticated_or_pairing: bool,
    last_frame: Instant,
    now: Instant,
    limit: Duration,
) -> bool {
    !authenticated_or_pairing && now.saturating_duration_since(last_frame) >= limit
}

fn is_idle(error: &tungstenite::Error) -> bool {
    matches!(error, tungstenite::Error::Io(e)
        if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut))
}

fn send_frame(socket: &mut Socket, frame: &Frame) -> bool {
    match frames::encode(frame) {
        Ok(text) => socket.send(tungstenite::Message::text(text)).is_ok(),
        Err(_) => false,
    }
}

fn send(socket: &mut Socket, message: Message) -> bool {
    send_frame(socket, &Frame::Core(message))
}

fn error(code: ErrorCode, text: &str) -> Message {
    Message::Error {
        code,
        message: text.to_owned(),
    }
}

impl Session {
    fn run(&mut self, socket: &mut Socket, first_frame: Instant, shutdown: &AtomicBool) {
        let hello = Message::Hello {
            protocol: PROTOCOL_VERSION,
            software: "curvyo-connector-server".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        };
        if !send(socket, hello) {
            return;
        }
        while !shutdown.load(Ordering::Relaxed) {
            let keep = match socket.read() {
                // A revoked client or a maintenance connection after the setup takes nothing more.
                Ok(tungstenite::Message::Text(_)) if self.must_close() => false,
                Ok(tungstenite::Message::Text(text)) => {
                    self.last_frame = Instant::now();
                    self.on_text(socket, text.as_str())
                }
                Ok(tungstenite::Message::Binary(_)) => send(
                    socket,
                    error(
                        ErrorCode::Malformed,
                        "Frames are text, one JSON object each.",
                    ),
                ),
                Ok(tungstenite::Message::Close(_)) => false,
                Ok(_) => true,
                Err(e) if is_idle(&e) => true,
                Err(_) => false,
            };
            let now = Instant::now();
            if !keep
                || !self.flush_statuses(socket, now)
                || self.timed_out(socket, first_frame, now)
            {
                break;
            }
        }
        let _ = socket.close(None);
        let _ = socket.flush();
    }

    fn timed_out(&mut self, socket: &mut Socket, first_frame: Instant, now: Instant) -> bool {
        if !self.greeted && now >= first_frame {
            return true;
        }
        if self.must_close() {
            return true;
        }
        let has_session = self.updates.is_some() || self.pairing.is_some();
        if silent_too_long(has_session, self.last_frame, now, UNAUTHENTICATED_IDLE) {
            return true;
        }
        if self.pairing.as_ref().is_some_and(|p| now >= p.deadline) {
            self.pairing = None;
            let left = self.hub.pair_failed();
            return !Self::refuse(socket, RefusalReason::WrongCode, left);
        }
        false
    }

    /// Whether a revoke of this client or the end of maintenance mode closes the connection.
    fn must_close(&self) -> bool {
        self.close
            .as_ref()
            .is_some_and(|c| c.load(Ordering::Relaxed))
            || self
                .maintenance_epoch
                .is_some_and(|epoch| epoch != self.hub.epoch())
    }

    fn flush_statuses(&mut self, socket: &mut Socket, now: Instant) -> bool {
        if let Some(updates) = &self.updates {
            while let Ok(message) = updates.try_recv() {
                if matches!(message, Message::Welcome { .. } | Message::JobList { .. }) {
                    // A changed controller and every change of the list go out at once, in
                    // order, before the status that follows.
                    if !send(socket, message) {
                        return false;
                    }
                } else {
                    self.limiter.push(message);
                }
            }
        }
        match self.limiter.due(now) {
            Some(status) => send(socket, status),
            None => true,
        }
    }

    fn refuse(socket: &mut Socket, reason: RefusalReason, attempts_left: u8) -> bool {
        send_frame(
            socket,
            &Frame::Io(IoFrame::PairRefused {
                reason,
                attempts_left,
            }),
        )
    }

    /// Returns false when the connection is to be closed.
    fn on_text(&mut self, socket: &mut Socket, text: &str) -> bool {
        match frames::decode(text) {
            Err(e) => send(socket, e.to_error_message()),
            Ok(Frame::Core(message)) => self.on_core(socket, message),
            Ok(Frame::Io(frame)) if !self.greeted => {
                let _ = frame;
                let _ = send(
                    socket,
                    error(ErrorCode::HelloRequired, "The first message must be Hello."),
                );
                false
            }
            Ok(Frame::Io(frame)) => self.on_io(socket, frame),
        }
    }

    fn on_core(&mut self, socket: &mut Socket, message: Message) -> bool {
        match self.handshake.on_message(&message) {
            Outcome::Hello => {
                self.greeted = true;
                if matches!(
                    self.hub.access_mode(Instant::now()),
                    Mode::Maintenance { .. }
                ) {
                    self.maintenance_epoch = Some(self.hub.epoch());
                }
                return send(socket, self.hub.welcome());
            }
            Outcome::Close(reply) => {
                let _ = send(socket, reply);
                return false;
            }
            Outcome::Closed => return false,
            Outcome::Pass => {}
        }
        let role = self.peer.as_ref().map(|p| p.role);
        if let Err(code) = admit(self.hub.access_mode(Instant::now()), role, &message) {
            if role.is_none()
                && code == ErrorCode::OwnerRequired
                && matches!(message, Message::ApplyConfig { .. })
            {
                self.hub.setup_refused(self.peer_ip);
            }
            return send(socket, refusal(code));
        }
        match message {
            Message::Auth { token } => self.on_auth(socket, &token),
            Message::ApplyConfig {
                config,
                client_name,
            } => self.on_apply(socket, config, &client_name),
            Message::ListPorts => send(
                socket,
                Message::Ports {
                    paths: crate::ports::list_ports(std::path::Path::new("/dev")),
                },
            ),
            Message::Hello { .. } | Message::Pair { .. } => send(
                socket,
                error(
                    ErrorCode::UnexpectedMessage,
                    "The connector does not accept this message.",
                ),
            ),
            other => match self.peer.clone() {
                None => send(socket, refusal(ErrorCode::PairingRequired)),
                Some(peer) => self.on_logged_in(socket, other, &peer),
            },
        }
    }

    fn on_logged_in(&mut self, socket: &mut Socket, message: Message, peer: &Peer) -> bool {
        let now = Instant::now();
        match message {
            Message::Home
            | Message::Jog { .. }
            | Message::JogKeep
            | Message::JogStop
            | Message::GoToPoint { .. } => self.on_motion(socket, message, peer, now),
            Message::Unlock => {
                let status = self.hub.unlock(peer, now);
                self.limiter.answered(now);
                send(socket, status)
            }
            owner_call @ (Message::GetConfig
            | Message::IssueCode { .. }
            | Message::ListClients
            | Message::RevokeClient { .. }
            | Message::GetLog { .. }
            | Message::Reset { .. }) => self.on_owner_call(socket, owner_call, peer),
            other => {
                if let Message::SubmitJob { job_id, .. } = &other {
                    self.submitted.push(*job_id);
                }
                let answers = self.hub.handle_message(other, peer, now);
                if answers.iter().any(|m| matches!(m, Message::Status { .. })) {
                    self.limiter.answered(now);
                }
                answers.into_iter().all(|m| send(socket, m))
            }
        }
    }

    fn on_auth(&mut self, socket: &mut Socket, token: &str) -> bool {
        let Some((name, role)) = self.hub.login(token) else {
            return send(
                socket,
                error(ErrorCode::PairingRequired, "This token is not paired."),
            );
        };
        self.limiter.answered(Instant::now());
        self.become_authenticated(socket, name, role) && self.send_list_and_status(socket)
    }

    /// The job list, then the current status: a connection starts with both (`specs/0132` L3).
    pub(super) fn send_list_and_status(&mut self, socket: &mut Socket) -> bool {
        send(socket, self.hub.job_list()) && send(socket, self.hub.status())
    }

    /// The connection is now `name` with `role`: it gets the statuses and is told who it is.
    fn become_authenticated(&mut self, socket: &mut Socket, name: String, role: Role) -> bool {
        if self.updates.is_none() {
            self.updates = Some(self.hub.subscribe());
        }
        self.close = Some(self.hub.register_session(&name));
        self.maintenance_epoch = None;
        self.peer = Some(Peer::new(name.clone(), role));
        send(
            socket,
            Message::Authenticated {
                client_name: name,
                role,
            },
        )
    }

    fn on_io(&mut self, socket: &mut Socket, frame: IoFrame) -> bool {
        match frame {
            IoFrame::PairSpake { .. }
                if admit(
                    self.hub.access_mode(Instant::now()),
                    self.peer.as_ref().map(|p| p.role),
                    &Message::Pair {
                        code: String::new(),
                        client_name: String::new(),
                    },
                )
                .is_err() =>
            {
                // A pairing frame is judged as `Pair` (addendum D4).
                send(socket, refusal(ErrorCode::Maintenance))
            }
            IoFrame::PairSpake {
                round: 1,
                client_name: Some(name),
                message,
            } if self.updates.is_none() => self.pair_round_one(socket, &name, &message),
            IoFrame::PairSpake {
                round: 2, message, ..
            } => self.pair_round_two(socket, &message),
            IoFrame::PairSpake { .. } => send(
                socket,
                error(ErrorCode::Malformed, "This pairing frame does not fit."),
            ),
            IoFrame::PairRefused { .. } => send(
                socket,
                error(
                    ErrorCode::UnexpectedMessage,
                    "The connector does not accept this message.",
                ),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_silent_connection_without_a_session_is_closed_after_the_limit() {
        let base = Instant::now();
        let limit = Duration::from_secs(10);
        let just_before = base + Duration::from_millis(9_999);
        let at_limit = base + limit;
        assert!(!silent_too_long(false, base, just_before, limit));
        assert!(silent_too_long(false, base, at_limit, limit));
        // Authenticated or pairing connections have their own rules.
        assert!(!silent_too_long(
            true,
            base,
            base + Duration::from_secs(100),
            limit
        ));
        // A frame resets the clock.
        assert!(!silent_too_long(false, at_limit, at_limit, limit));
    }
}
