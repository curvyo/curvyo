//! A session's setup and owner calls (A6, A9 to A11, A16, A18, A19): the access gate has let them
//! through; this sends each to the hub and answers.

use std::time::{Duration, Instant};

use curvyo_connector_core::{ConnectorConfig, Message, Peer, Role, Secret, State};

use super::{Session, Socket, send};
use crate::hub::ExitReason;

/// The longest a stop before a reset may take: G12's hold and banner limits plus `M5` and `M9`.
const RESET_STOP: Duration = Duration::from_secs(6);

impl Session {
    /// `ApplyConfig`: in maintenance mode the sender becomes the owner, and this connection goes
    /// on as an owner's session without a new login (addendum D6).
    pub(super) fn on_apply(
        &mut self,
        socket: &mut Socket,
        config: ConnectorConfig,
        client_name: &str,
    ) -> bool {
        let owner = self.peer.clone();
        let applied = self.hub.apply_config(
            config,
            client_name,
            owner.as_ref(),
            self.peer_ip,
            Instant::now(),
        );
        match applied {
            Err(refused) => send(socket, refused.message()),
            Ok(None) => send(socket, Message::ConfigApplied { token: None }),
            Ok(Some(token)) => {
                let answer = Message::ConfigApplied {
                    token: Some(Secret::new(token)),
                };
                if !send(socket, answer) {
                    return false;
                }
                if self.updates.is_none() {
                    self.updates = Some(self.hub.subscribe());
                }
                self.close = Some(self.hub.register_session(client_name));
                self.maintenance_epoch = None;
                self.peer = Some(Peer::new(client_name, Role::Owner));
                true
            }
        }
    }

    /// An owner call of `peer` (the gate checked the role); false when the connection closes.
    pub(super) fn on_owner_call(
        &mut self,
        socket: &mut Socket,
        message: Message,
        peer: &Peer,
    ) -> bool {
        let answer = match message {
            Message::GetConfig => match self.hub.config() {
                Some(config) => Message::Config { config },
                None => {
                    curvyo_connector_core::refusal(curvyo_connector_core::ErrorCode::Maintenance)
                }
            },
            Message::IssueCode { client_name, role } => {
                match self
                    .hub
                    .issue_code_for(&client_name, role, &peer.name, Instant::now())
                {
                    Ok((code, expires_in_s)) => Message::CodeIssued {
                        code: Secret::new(code),
                        expires_in_s,
                    },
                    Err(refused) => refused.message(),
                }
            }
            Message::ListClients => Message::Clients {
                clients: self.hub.list_clients(),
            },
            Message::RevokeClient { name } => match self.hub.revoke(&name, &peer.name) {
                Ok(()) => Message::ClientRevoked { name },
                Err(refused) => refused.message(),
            },
            Message::GetLog { log, last } => Message::LogLines {
                lines: self.hub.log_lines(log, last),
            },
            Message::Reset { connector_name } => {
                return self.on_reset(socket, &connector_name, peer);
            }
            other => other,
        };
        send(socket, answer)
    }

    /// `Reset`: a running job is stopped as on SIGTERM, `ResetDone` goes out, then the main thread
    /// is asked to shut down and empty the state directory (ADR 0071 decision 4, A19).
    fn on_reset(&mut self, socket: &mut Socket, connector_name: &str, peer: &Peer) -> bool {
        if let Err(refused) = self.hub.check_reset(connector_name) {
            return send(socket, refused.message());
        }
        if self.hub.stop_for_exit(Instant::now()).is_some() {
            let until = Instant::now() + RESET_STOP;
            while matches!(self.hub.state(), State::Running | State::Paused)
                && Instant::now() < until
            {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        let _ = send(socket, Message::ResetDone);
        let _ = socket.flush();
        self.hub.request_exit(ExitReason::Reset {
            by: peer.name.clone(),
        });
        false
    }
}
