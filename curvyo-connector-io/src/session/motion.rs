//! How a connection relays the motion commands Home and Jog to the hub (`specs/0118`).

use std::time::Instant;

use curvyo_connector_core::{Message, Peer};

use super::{Session, Socket, send};

impl Session {
    /// `Home`, `Jog`, `JogKeep` or `JogStop` from the logged-in `peer`. A connection that started
    /// a hold remembers it, so closing it ends the hold at once (criterion 18).
    pub(super) fn on_motion(
        &mut self,
        socket: &mut Socket,
        message: Message,
        peer: &Peer,
        now: Instant,
    ) -> bool {
        let is_home = message == Message::Home;
        let answers = match message {
            Message::Home => self.hub.home(peer, now),
            other => {
                match other {
                    Message::Jog { hold, .. } => self.holding |= hold,
                    Message::JogStop => self.holding = false,
                    _ => {}
                }
                self.hub.jog(other, peer, now)
            }
        };
        if is_home || answers.iter().any(|m| matches!(m, Message::Status { .. })) {
            self.limiter.answered(now);
        }
        answers.into_iter().all(|m| send(socket, m))
    }
}
