//! Checks that a connection starts with `Hello` of the right protocol version.

use crate::message::{ErrorCode, Message};

/// The protocol version this crate speaks (ADR 0035 §6, unstable).
pub const PROTOCOL_VERSION: u32 = 0;

/// What the transport does with a message after the handshake looked at it.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)] // one value per message, never stored in bulk
pub enum Outcome {
    /// `Hello` passed: answer with `Welcome`.
    Hello,
    /// The handshake is over: pass the message on.
    Pass,
    /// Send this `Error` and close the connection.
    Close(Message),
    /// The connection was closed before: accept nothing.
    Closed,
}

/// The first-message check of one connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handshake {
    phase: Phase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    AwaitingHello,
    Open,
    Closed,
}

impl Default for Handshake {
    fn default() -> Self {
        Self::new()
    }
}

impl Handshake {
    /// A new connection.
    #[must_use]
    pub fn new() -> Self {
        Self {
            phase: Phase::AwaitingHello,
        }
    }

    /// Looks at the next message from the peer.
    pub fn on_message(&mut self, message: &Message) -> Outcome {
        match self.phase {
            Phase::Closed => Outcome::Closed,
            Phase::Open => Outcome::Pass,
            Phase::AwaitingHello => match message {
                Message::Hello { protocol, .. } if *protocol == PROTOCOL_VERSION => {
                    self.phase = Phase::Open;
                    Outcome::Hello
                }
                Message::Hello { .. } => self.close(
                    ErrorCode::UnsupportedProtocol,
                    "This connector speaks protocol 0.",
                ),
                _ => self.close(ErrorCode::HelloRequired, "The first message must be Hello."),
            },
        }
    }

    fn close(&mut self, code: ErrorCode, message: &str) -> Outcome {
        self.phase = Phase::Closed;
        Outcome::Close(Message::Error {
            code,
            message: message.to_owned(),
        })
    }
}
