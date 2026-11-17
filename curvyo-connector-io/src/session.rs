//! One connection of the connector: TLS, the WebSocket, the hello, the auth gate, pairing and the
//! relay of messages to the hub.

use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use curvyo_connector_core::{ErrorCode, Handshake, Message, Outcome, PROTOCOL_VERSION};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{HandshakeError, WebSocket};

use crate::frames::{self, Frame, IoFrame, PATH, RefusalReason, SOCKET_LIMIT};
use crate::hex;
use crate::hub::Hub;
use crate::pairing_code::Refusal;
use crate::spake::{self, SessionKey};
use crate::status_limit::StatusLimiter;
use crate::token::Token;

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
    /// Whether this connection sent a `SubmitJob`.
    submitted: bool,
    /// When the last frame came in.
    last_frame: Instant,
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

/// Serves one accepted connection until it ends or `shutdown` is set.
pub fn serve(stream: TcpStream, tls: &Arc<ServerConfig>, hub: Arc<Hub>, shutdown: &AtomicBool) {
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
        submitted: false,
        last_frame: Instant::now(),
    };
    session.run(&mut socket, deadline, shutdown);
    if session.submitted {
        session.hub.client_disconnected(Instant::now());
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

    fn flush_statuses(&mut self, socket: &mut Socket, now: Instant) -> bool {
        if let Some(updates) = &self.updates {
            while let Ok(status) = updates.try_recv() {
                self.limiter.push(status);
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
                return send(socket, self.hub.welcome());
            }
            Outcome::Close(reply) => {
                let _ = send(socket, reply);
                return false;
            }
            Outcome::Closed => return false,
            Outcome::Pass => {}
        }
        match message {
            Message::Auth { token } => self.on_auth(socket, &token),
            Message::Hello { .. } | Message::Pair { .. } => send(
                socket,
                error(
                    ErrorCode::UnexpectedMessage,
                    "The connector does not accept this message.",
                ),
            ),
            other if self.updates.is_none() => {
                let _ = other;
                send(
                    socket,
                    error(ErrorCode::PairingRequired, "Pair or authenticate first."),
                )
            }
            other => {
                let now = Instant::now();
                self.submitted |= matches!(other, Message::SubmitJob { .. });
                let answers = self.hub.handle_message(other, now);
                if answers.iter().any(|m| matches!(m, Message::Status { .. })) {
                    self.limiter.answered(now);
                }
                answers.into_iter().all(|m| send(socket, m))
            }
        }
    }

    fn on_auth(&mut self, socket: &mut Socket, token: &str) -> bool {
        if !self.hub.accepts(token) {
            return send(
                socket,
                error(ErrorCode::PairingRequired, "This token is not paired."),
            );
        }
        self.become_authenticated();
        self.limiter.answered(Instant::now());
        send(socket, self.hub.status())
    }

    fn become_authenticated(&mut self) {
        if self.updates.is_none() {
            self.updates = Some(self.hub.subscribe());
        }
    }

    fn on_io(&mut self, socket: &mut Socket, frame: IoFrame) -> bool {
        match frame {
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

    fn pair_round_one(&mut self, socket: &mut Socket, name: &str, message: &str) -> bool {
        if name.is_empty() || name.chars().count() > MAX_NAME || name.chars().any(char::is_control)
        {
            return send(
                socket,
                error(ErrorCode::Malformed, "The client name is not valid."),
            );
        }
        let now = Instant::now();
        let password = match self.hub.begin_pair(now) {
            Ok(password) => password,
            Err(Refusal::Expired) => return Self::refuse(socket, RefusalReason::CodeExpired, 0),
            Err(Refusal::NoCode) => return Self::refuse(socket, RefusalReason::NoCode, 0),
        };
        let (exchange, reply) = spake::start_connector(&password, &self.hub.spki());
        let key = hex::decode(message).and_then(|m| exchange.finish(&m).ok());
        let Some(key) = key else {
            let left = self.hub.pair_failed();
            return Self::refuse(socket, RefusalReason::WrongCode, left);
        };
        self.pairing = Some(Pairing {
            key,
            client_name: name.to_owned(),
            deadline: now + PAIRING_IDLE,
        });
        send_frame(
            socket,
            &Frame::Io(IoFrame::PairSpake {
                round: 1,
                client_name: None,
                message: hex::encode(&reply),
            }),
        )
    }

    fn pair_round_two(&mut self, socket: &mut Socket, message: &str) -> bool {
        let Some(pairing) = self.pairing.take() else {
            return Self::refuse(socket, RefusalReason::NoCode, 0);
        };
        let verified = hex::decode(message).is_some_and(|tag| pairing.key.verify_app_tag(&tag));
        if !verified {
            let left = self.hub.pair_failed();
            return Self::refuse(socket, RefusalReason::WrongCode, left);
        }
        let Some(token) = Token::generate() else {
            self.hub.pair_failed();
            return Self::refuse(socket, RefusalReason::NoCode, 0);
        };
        if self
            .hub
            .pair_succeeded(&pairing.client_name, &token.sha256_hex())
            .is_err()
        {
            return Self::refuse(socket, RefusalReason::NoCode, 0);
        }
        let confirmation = Frame::Io(IoFrame::PairSpake {
            round: 2,
            client_name: None,
            message: hex::encode(&pairing.key.connector_tag()),
        });
        self.become_authenticated();
        self.limiter.answered(Instant::now());
        send_frame(socket, &confirmation)
            && send(
                socket,
                Message::Paired {
                    token: token.to_hex().to_string(),
                },
            )
            && send(socket, self.hub.status())
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
