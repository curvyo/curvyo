//! The app's side of the connection: connect with a pinned key, say hello, authenticate. The
//! blocking client runs on its own thread in the app.

use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

use curvyo_connector_core::{ControllerInfo, ErrorCode, Message, PROTOCOL_VERSION};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use tungstenite::client::IntoClientRequest as _;
use tungstenite::handshake::client::ClientHandshake;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{HandshakeError, WebSocket};

use crate::frames::{self, Frame, PATH, SOCKET_LIMIT};
use crate::pin_verifier::{PinMode, PinVerifier};
use crate::spki::SpkiHash;
use crate::token::Token;

/// The port a connector listens on unless told otherwise; the server's default listen port and the
/// port the app dials when the address has none.
pub const DEFAULT_PORT: u16 = 7447;

/// How long a connect, a hello and an auth may take together (Q11: 5 s).
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);

/// Why the connector could not be used; each variant is one line of the row (Q8, Q11).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClientError {
    /// The address is none of `host`, `host:port`, `[v6]:port` or an IP literal.
    #[error("not an address: {0}")]
    BadAddress(String),
    /// Nothing listens there.
    #[error("nothing answers at {0}")]
    Unreachable(String),
    /// No answer in time.
    #[error("timed out")]
    TimedOut,
    /// The connector presents another key than the pinned one; nothing was sent.
    #[error("this connector's key changed")]
    KeyChanged {
        /// The pinned key.
        stored: SpkiHash,
        /// The key it presented.
        presented: SpkiHash,
    },
    /// The connector does not know the token (revoked).
    #[error("no longer paired")]
    NoLongerPaired,
    /// The connector speaks another protocol version.
    #[error("protocol mismatch: app {PROTOCOL_VERSION}, connector {connector}")]
    ProtocolMismatch {
        /// The connector's version.
        connector: u32,
    },
    /// The TLS handshake failed, for example TLS 1.2 only.
    #[error("TLS failed: {0}")]
    Tls(String),
    /// The connector sent something the protocol does not allow here.
    #[error("unexpected answer: {0}")]
    Protocol(String),
    /// The connection broke.
    #[error("connection lost: {0}")]
    Io(String),
}

/// The connector's greeting.
#[derive(Debug, Clone, PartialEq)]
pub struct Greeting {
    /// Its protocol version.
    pub protocol: u32,
    /// Its program version.
    pub version: String,
    /// Its name.
    pub name: String,
    /// Its controller.
    pub controller: Option<ControllerInfo>,
}

/// One open connection, after `Hello` and `Welcome`.
pub struct Connection {
    socket: WebSocket<StreamOwned<ClientConnection, TcpStream>>,
    /// What the connector said about itself.
    pub greeting: Greeting,
    /// Hash of the key it presented.
    pub spki: SpkiHash,
}

fn is_idle(error: &tungstenite::Error) -> bool {
    matches!(error, tungstenite::Error::Io(e)
        if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut))
}

/// The parts of an address: the name or IP literal, and the port (the default when absent).
fn split_address(address: &str, default_port: u16) -> Option<(String, u16)> {
    let text = address.trim();
    if let Ok(socket) = text.parse::<SocketAddr>() {
        return Some((socket.ip().to_string(), socket.port()));
    }
    if let Ok(ip) = text.parse::<IpAddr>() {
        return Some((ip.to_string(), default_port));
    }
    if let Some(v6) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        return v6
            .parse::<std::net::Ipv6Addr>()
            .ok()
            .map(|ip| (ip.to_string(), default_port));
    }
    let (host, port) = match text.rsplit_once(':') {
        Some((host, port)) => (host, port.parse::<u16>().ok().filter(|p| *p != 0)?),
        None => (text, default_port),
    };
    let name_ok = !host.is_empty()
        && host.len() <= 253
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    name_ok.then(|| (host.to_owned(), port))
}

/// The `host:port` the connection dials, with `[..]` around a v6 literal.
fn authority(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn tcp(
    address: &str,
    default_port: u16,
    deadline: Instant,
) -> Result<(TcpStream, String), ClientError> {
    let bad = || ClientError::BadAddress(address.to_owned());
    let (host, port) = split_address(address, default_port).ok_or_else(bad)?;
    let dialled = authority(&host, port);
    // A well-formed name that does not resolve is a place nothing answers at.
    let addrs: Vec<_> = dialled
        .to_socket_addrs()
        .map_err(|_| ClientError::Unreachable(dialled.clone()))?
        .collect();
    let mut timed_out = false;
    for addr in addrs {
        let left = deadline.saturating_duration_since(Instant::now());
        match TcpStream::connect_timeout(&addr, left.max(Duration::from_millis(1))) {
            Ok(stream) => return Ok((stream, dialled)),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => timed_out = true,
            Err(_) => {}
        }
    }
    Err(if timed_out {
        ClientError::TimedOut
    } else {
        ClientError::Unreachable(dialled)
    })
}

/// Opens a connection to `address` and greets: `host`, `host:port`, `[v6]:port`, a name (resolved)
/// or an IP literal; a missing port is [`DEFAULT_PORT`]. With [`PinMode::Expect`] nothing is sent to
/// a connector whose key differs.
///
/// # Errors
/// [`ClientError`].
pub fn connect(address: &str, mode: PinMode, timeout: Duration) -> Result<Connection, ClientError> {
    connect_on(address, DEFAULT_PORT, mode, timeout)
}

/// [`connect`] with another port for an address that has none (tests use a free one).
///
/// # Errors
/// [`ClientError`].
pub fn connect_on(
    address: &str,
    default_port: u16,
    mode: PinMode,
    timeout: Duration,
) -> Result<Connection, ClientError> {
    let deadline = Instant::now() + timeout;
    let (stream, address) = tcp(address, default_port, deadline)?;
    stream
        .set_read_timeout(Some(POLL))
        .and_then(|()| stream.set_nodelay(true))
        .map_err(|e| ClientError::Io(e.to_string()))?;
    let verifier = PinVerifier::new(mode);
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| ClientError::Tls(e.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::clone(&verifier) as _)
        .with_no_client_auth();
    let name =
        ServerName::try_from("curvyo-connector").map_err(|e| ClientError::Tls(e.to_string()))?;
    let tls = ClientConnection::new(Arc::new(config), name)
        .map_err(|e| ClientError::Tls(e.to_string()))?;
    let request = format!("wss://{address}{PATH}")
        .into_client_request()
        .map_err(|_| ClientError::BadAddress(address.clone()))?;
    let websocket = WebSocketConfig::default()
        .max_frame_size(Some(SOCKET_LIMIT))
        .max_message_size(Some(SOCKET_LIMIT));
    let mut attempt: Result<_, HandshakeError<ClientHandshake<_>>> =
        tungstenite::client::client_with_config(
            request,
            StreamOwned::new(tls, stream),
            Some(websocket),
        );
    let socket = loop {
        match attempt {
            Ok((socket, _)) => break socket,
            Err(HandshakeError::Interrupted(mid)) => {
                if Instant::now() >= deadline {
                    return Err(ClientError::TimedOut);
                }
                attempt = mid.handshake();
            }
            Err(HandshakeError::Failure(e)) => return Err(handshake_error(&e, mode, &verifier)),
        }
    };
    let mut connection = Connection {
        socket,
        greeting: Greeting {
            protocol: 0,
            version: String::new(),
            name: String::new(),
            controller: None,
        },
        spki: verifier
            .seen()
            .ok_or_else(|| ClientError::Tls("no key was presented".to_owned()))?,
    };
    connection.greet(deadline)?;
    Ok(connection)
}

fn handshake_error(
    error: &tungstenite::Error,
    mode: PinMode,
    verifier: &PinVerifier,
) -> ClientError {
    if let (PinMode::Expect(stored), Some(presented)) = (mode, verifier.seen())
        && stored != presented
    {
        return ClientError::KeyChanged { stored, presented };
    }
    ClientError::Tls(error.to_string())
}

impl Connection {
    /// Sends one message.
    ///
    /// # Errors
    /// [`ClientError::Io`].
    pub fn send(&mut self, message: &Message) -> Result<(), ClientError> {
        let text = frames::encode(&Frame::Core(message.clone()))
            .map_err(|e| ClientError::Protocol(e.to_string()))?;
        self.send_text(text)
    }

    /// Sends one frame, of core's protocol or of this crate.
    ///
    /// # Errors
    /// [`ClientError::Io`].
    pub fn send_frame(&mut self, frame: &Frame) -> Result<(), ClientError> {
        let text = frames::encode(frame).map_err(|e| ClientError::Protocol(e.to_string()))?;
        self.send_text(text)
    }

    fn send_text(&mut self, text: String) -> Result<(), ClientError> {
        self.socket
            .send(tungstenite::Message::text(text))
            .map_err(|e| ClientError::Io(e.to_string()))
    }

    /// The next frame, or `None` when none arrived by `deadline`.
    ///
    /// # Errors
    /// [`ClientError::Io`] when the connection ended; [`ClientError::Protocol`] for a frame
    /// that does not decode.
    pub fn try_recv(&mut self, deadline: Instant) -> Result<Option<Frame>, ClientError> {
        loop {
            match self.socket.read() {
                Ok(tungstenite::Message::Text(text)) => {
                    return frames::decode(text.as_str())
                        .map(Some)
                        .map_err(|e| ClientError::Protocol(e.to_string()));
                }
                Ok(tungstenite::Message::Close(_)) => {
                    return Err(ClientError::Io("closed by the connector".to_owned()));
                }
                Ok(_) => {}
                Err(e) if is_idle(&e) => {
                    if Instant::now() >= deadline {
                        return Ok(None);
                    }
                }
                Err(e) => return Err(ClientError::Io(e.to_string())),
            }
        }
    }

    /// The next frame; a timeout is an error.
    ///
    /// # Errors
    /// As [`Connection::try_recv`], and [`ClientError::TimedOut`].
    pub fn recv(&mut self, deadline: Instant) -> Result<Frame, ClientError> {
        self.try_recv(deadline)?.ok_or(ClientError::TimedOut)
    }

    /// The next message of core's protocol; a frame of this crate is an error.
    ///
    /// # Errors
    /// As [`Connection::recv`], and [`ClientError::Protocol`].
    pub fn recv_message(&mut self, deadline: Instant) -> Result<Message, ClientError> {
        match self.recv(deadline)? {
            Frame::Core(message) => Ok(message),
            Frame::Io(_) => Err(ClientError::Protocol("a pairing frame".to_owned())),
        }
    }

    fn greet(&mut self, deadline: Instant) -> Result<(), ClientError> {
        let Message::Hello {
            protocol, version, ..
        } = self.recv_message(deadline)?
        else {
            return Err(ClientError::Protocol("no Hello".to_owned()));
        };
        if protocol != PROTOCOL_VERSION {
            return Err(ClientError::ProtocolMismatch {
                connector: protocol,
            });
        }
        self.send(&Message::Hello {
            protocol: PROTOCOL_VERSION,
            software: "curvyo-app".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        })?;
        match self.recv_message(deadline)? {
            Message::Welcome {
                connector_name,
                controller,
            } => {
                self.greeting = Greeting {
                    protocol,
                    version,
                    name: connector_name,
                    controller,
                };
                Ok(())
            }
            Message::Error {
                code: ErrorCode::UnsupportedProtocol,
                ..
            } => Err(ClientError::ProtocolMismatch {
                connector: protocol,
            }),
            other => Err(ClientError::Protocol(format!("{other:?}"))),
        }
    }

    /// Authenticates with the token; the answer is the current `Status`.
    ///
    /// # Errors
    /// [`ClientError::NoLongerPaired`] when the connector does not know the token.
    pub fn auth(&mut self, token: &Token, deadline: Instant) -> Result<Message, ClientError> {
        self.send(&Message::Auth {
            token: token.to_hex().to_string(),
        })?;
        match self.recv_message(deadline)? {
            status @ Message::Status { .. } => Ok(status),
            Message::Error {
                code: ErrorCode::PairingRequired,
                ..
            } => Err(ClientError::NoLongerPaired),
            other => Err(ClientError::Protocol(format!("{other:?}"))),
        }
    }
}

/// Connects to a paired connector: the pinned key, then the token. Returns the connection and the
/// connector's current `Status`.
///
/// # Errors
/// [`ClientError`]: `KeyChanged` before any token is sent, `NoLongerPaired` for a revoked token.
pub fn open_paired(
    address: &str,
    spki: SpkiHash,
    token: &Token,
    timeout: Duration,
) -> Result<(Connection, Message), ClientError> {
    open_paired_on(address, DEFAULT_PORT, spki, token, timeout)
}

/// [`open_paired`] with another port for an address that has none (tests use a free one).
///
/// # Errors
/// [`ClientError`].
pub fn open_paired_on(
    address: &str,
    default_port: u16,
    spki: SpkiHash,
    token: &Token,
    timeout: Duration,
) -> Result<(Connection, Message), ClientError> {
    let deadline = Instant::now() + timeout;
    let mut connection = connect_on(address, default_port, PinMode::Expect(spki), timeout)?;
    let status = connection.auth(token, deadline)?;
    Ok((connection, status))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(text: &str) -> Option<(String, u16)> {
        split_address(text, DEFAULT_PORT)
    }

    #[test]
    fn an_address_without_a_port_gets_the_default_port() {
        assert_eq!(split("127.0.0.1"), Some(("127.0.0.1".to_owned(), 7447)));
        assert_eq!(split("workshop"), Some(("workshop".to_owned(), 7447)));
        assert_eq!(split("::1"), Some(("::1".to_owned(), 7447)));
        assert_eq!(split("[::1]"), Some(("::1".to_owned(), 7447)));
    }

    #[test]
    fn host_port_and_v6_port_forms_keep_their_port() {
        assert_eq!(split("10.0.0.2:9000"), Some(("10.0.0.2".to_owned(), 9000)));
        assert_eq!(
            split("shop.local:9000"),
            Some(("shop.local".to_owned(), 9000))
        );
        assert_eq!(split("[::1]:9000"), Some(("::1".to_owned(), 9000)));
        assert_eq!(authority("::1", 9000), "[::1]:9000");
    }

    #[test]
    fn what_is_no_address_is_refused() {
        for text in [
            "",
            " ",
            "host:",
            "host:0",
            "host:99999",
            "a b",
            "[::1",
            "[host]:1",
            "a:b:c",
            "ho_st",
        ] {
            assert_eq!(split(text), None, "{text:?}");
        }
    }
}
