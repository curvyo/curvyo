//! Decides whether a peer may send a message in a mode (architect addendum D4): one pure table
//! for the LAN and, later, the relay.

use crate::admin::Role;
use crate::message::{ErrorCode, Message};

/// The connector's mode as the table needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Not configured; `window_open` while `ApplyConfig` is still taken.
    Maintenance {
        /// Whether the setup window is open.
        window_open: bool,
    },
    /// Configured from `connector.toml`.
    Configured,
    /// Run with the command-line flags of slices 2 and 4: no owners.
    Foreground,
}

/// What a message is, for the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Hello,
    ListPorts,
    ApplyConfig,
    Login,
    Job,
    /// `Jog`, `JogKeep`: moving the head by hand is an owner's (Frage 308 A).
    Motion,
    /// `MoveJob`, `RemoveJob`, `SetRunNext`: the job list.
    List,
    /// `MoveJob`, `RestartFromLine`: only an owner reorders or restarts.
    OwnerList,
    OwnerCall,
}

fn kind(message: &Message) -> Kind {
    match message {
        Message::Hello { .. } => Kind::Hello,
        Message::ListPorts => Kind::ListPorts,
        Message::ApplyConfig { .. } => Kind::ApplyConfig,
        Message::Pair { .. } | Message::Auth { .. } => Kind::Login,
        Message::Jog { .. } | Message::JogKeep => Kind::Motion,
        Message::MoveJob { .. } | Message::RestartFromLine { .. } => Kind::OwnerList,
        Message::RemoveJob { .. } | Message::SetRunNext { .. } => Kind::List,
        Message::GetConfig
        | Message::IssueCode { .. }
        | Message::ListClients
        | Message::RevokeClient { .. }
        | Message::GetLog { .. }
        | Message::Reset { .. } => Kind::OwnerCall,
        // The job and control messages, `GetStatus`, `Unlock`, `Home` (a member's waits for an
        // owner, `specs/0118` 37), `JogStop`, and the answers only a connector sends (the job machine refuses those as unexpected).
        _ => Kind::Job,
    }
}

/// Whether `peer` (`None`: not logged in) may send `message` in `mode`; the refusal's code when
/// not. `Pair` and `Auth` pass outside maintenance mode: the session's login rules decide them.
/// A member's `Home` passes: the job machine holds it until an owner confirms (`specs/0118` 37);
/// a member's `Jog` and `JogKeep` are refused, `JogStop` passes.
///
/// # Errors
/// The `ErrorCode` of the refusal.
pub fn admit(mode: Mode, peer: Option<Role>, message: &Message) -> Result<(), ErrorCode> {
    let kind = kind(message);
    if kind == Kind::Hello {
        return Ok(());
    }
    match mode {
        Mode::Maintenance { window_open } => match kind {
            Kind::ListPorts => Ok(()),
            Kind::ApplyConfig if window_open => Ok(()),
            Kind::ApplyConfig => Err(ErrorCode::MaintenanceClosed),
            _ => Err(ErrorCode::Maintenance),
        },
        Mode::Configured => match (kind, peer) {
            (Kind::Login, _) | (_, Some(Role::Owner)) => Ok(()),
            (Kind::ListPorts | Kind::ApplyConfig, _)
            | (Kind::OwnerCall | Kind::Motion | Kind::OwnerList, Some(Role::Member)) => {
                Err(ErrorCode::OwnerRequired)
            }
            (_, None) => Err(ErrorCode::PairingRequired),
            (_, Some(Role::Member)) => Ok(()),
        },
        Mode::Foreground => match (kind, peer) {
            (Kind::Login, _) => Ok(()),
            (Kind::ListPorts | Kind::ApplyConfig, _) | (Kind::OwnerCall, Some(_)) => {
                Err(ErrorCode::OwnerRequired)
            }
            (_, None) => Err(ErrorCode::PairingRequired),
            (_, Some(_)) => Ok(()),
        },
    }
}

/// The `Error` the connector answers a refusal of [`admit`] with, in fixed words.
#[must_use]
pub fn refusal(code: ErrorCode) -> Message {
    let message = match code {
        ErrorCode::Maintenance => "The connector is in maintenance mode.",
        ErrorCode::MaintenanceClosed => "The setup window is closed.",
        ErrorCode::OwnerRequired => "Only an owner may send this.",
        ErrorCode::PairingRequired => "Pair or authenticate first.",
        _ => "The connector does not accept this message.",
    };
    Message::Error {
        code,
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::{LogName, Role};
    use crate::config::{ConnectorConfig, ControllerConfig};
    use crate::jog::{JogAxis, JogDirection, JogStep};

    fn config() -> ConnectorConfig {
        ConnectorConfig {
            format: 1,
            name: "a".into(),
            owner: None,
            listen: None,
            controller: ControllerConfig::Fake {
                lines_per_second: 20,
                alarm_at_line: None,
            },
        }
    }

    /// One message of each row of the D4 table, with its row name.
    fn rows() -> Vec<(&'static str, Message)> {
        let id = uuid::Uuid::nil();
        vec![
            (
                "Hello",
                Message::Hello {
                    protocol: 0,
                    software: "x".into(),
                    version: "0".into(),
                },
            ),
            ("ListPorts", Message::ListPorts),
            (
                "ApplyConfig",
                Message::ApplyConfig {
                    config: config(),
                    client_name: "c".into(),
                },
            ),
            (
                "Pair",
                Message::Pair {
                    code: "x".into(),
                    client_name: "c".into(),
                },
            ),
            ("Auth", Message::Auth { token: "x".into() }),
            ("GetStatus", Message::GetStatus),
            ("Start", Message::Start { job_id: id }),
            ("Pause", Message::Pause),
            ("Resume", Message::Resume),
            ("Stop", Message::Stop),
            ("Unlock", Message::Unlock),
            ("Home", Message::Home),
            (
                "Jog",
                Message::Jog {
                    axis: JogAxis::X,
                    direction: JogDirection::Plus,
                    step: JogStep::One,
                    hold: false,
                },
            ),
            ("JogKeep", Message::JogKeep),
            ("JogStop", Message::JogStop),
            (
                "MoveJob",
                Message::MoveJob {
                    job_id: id,
                    from: 1,
                    to: 2,
                },
            ),
            ("RemoveJob", Message::RemoveJob { job_id: id }),
            (
                "RestartFromLine",
                Message::RestartFromLine {
                    job_id: id,
                    line: 12,
                },
            ),
            (
                "SetRunNext",
                Message::SetRunNext {
                    job_id: id,
                    on: true,
                },
            ),
            ("GetConfig", Message::GetConfig),
            (
                "IssueCode",
                Message::IssueCode {
                    client_name: "c".into(),
                    role: Role::Member,
                },
            ),
            ("ListClients", Message::ListClients),
            ("RevokeClient", Message::RevokeClient { name: "c".into() }),
            (
                "GetLog",
                Message::GetLog {
                    log: LogName::Jobs,
                    last: 1,
                },
            ),
            (
                "Reset",
                Message::Reset {
                    connector_name: "a".into(),
                },
            ),
        ]
    }

    fn cell(result: Result<(), ErrorCode>) -> String {
        match result {
            Ok(()) => "yes".to_owned(),
            Err(code) => format!("{code:?}"),
        }
    }

    /// The whole table of addendum D4, one line per message: maintenance open, maintenance
    /// closed, configured not logged in, member, owner, foreground not logged in, foreground paired.
    #[test]
    fn the_whole_table() {
        let open = Mode::Maintenance { window_open: true };
        let closed = Mode::Maintenance { window_open: false };
        let columns = [
            (open, None),
            (closed, None),
            (Mode::Configured, None),
            (Mode::Configured, Some(Role::Member)),
            (Mode::Configured, Some(Role::Owner)),
            (Mode::Foreground, None),
            (Mode::Foreground, Some(Role::Member)),
        ];
        let table: Vec<String> = rows()
            .iter()
            .map(|(name, message)| {
                let cells: Vec<String> = columns
                    .iter()
                    .map(|(mode, peer)| cell(admit(*mode, *peer, message)))
                    .collect();
                format!("{name}: {}", cells.join(" "))
            })
            .collect();
        let expected = [
            "Hello: yes yes yes yes yes yes yes",
            "ListPorts: yes yes OwnerRequired OwnerRequired yes OwnerRequired OwnerRequired",
            "ApplyConfig: yes MaintenanceClosed OwnerRequired OwnerRequired yes OwnerRequired OwnerRequired",
            "Pair: Maintenance Maintenance yes yes yes yes yes",
            "Auth: Maintenance Maintenance yes yes yes yes yes",
            "GetStatus: Maintenance Maintenance PairingRequired yes yes PairingRequired yes",
            "Start: Maintenance Maintenance PairingRequired yes yes PairingRequired yes",
            "Pause: Maintenance Maintenance PairingRequired yes yes PairingRequired yes",
            "Resume: Maintenance Maintenance PairingRequired yes yes PairingRequired yes",
            "Stop: Maintenance Maintenance PairingRequired yes yes PairingRequired yes",
            "Unlock: Maintenance Maintenance PairingRequired yes yes PairingRequired yes",
            "Home: Maintenance Maintenance PairingRequired yes yes PairingRequired yes",
            "Jog: Maintenance Maintenance PairingRequired OwnerRequired yes PairingRequired yes",
            "JogKeep: Maintenance Maintenance PairingRequired OwnerRequired yes PairingRequired yes",
            "JogStop: Maintenance Maintenance PairingRequired yes yes PairingRequired yes",
            "MoveJob: Maintenance Maintenance PairingRequired OwnerRequired yes PairingRequired yes",
            "RemoveJob: Maintenance Maintenance PairingRequired yes yes PairingRequired yes",
            "RestartFromLine: Maintenance Maintenance PairingRequired OwnerRequired yes PairingRequired yes",
            "SetRunNext: Maintenance Maintenance PairingRequired yes yes PairingRequired yes",
            "GetConfig: Maintenance Maintenance PairingRequired OwnerRequired yes PairingRequired OwnerRequired",
            "IssueCode: Maintenance Maintenance PairingRequired OwnerRequired yes PairingRequired OwnerRequired",
            "ListClients: Maintenance Maintenance PairingRequired OwnerRequired yes PairingRequired OwnerRequired",
            "RevokeClient: Maintenance Maintenance PairingRequired OwnerRequired yes PairingRequired OwnerRequired",
            "GetLog: Maintenance Maintenance PairingRequired OwnerRequired yes PairingRequired OwnerRequired",
            "Reset: Maintenance Maintenance PairingRequired OwnerRequired yes PairingRequired OwnerRequired",
        ];
        assert_eq!(table, expected);
    }
}
