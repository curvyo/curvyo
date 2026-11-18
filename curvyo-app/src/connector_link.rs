//! What the connector commands answer: plain data for the webview, and the mapping from the
//! client's errors. No Tauri here, so the tables are tested without a window.

use curvyo_connector_core::{ControllerInfo, Message, RejectReason};
use curvyo_connector_io::client::ClientError;
use curvyo_connector_io::frames::RefusalReason;
use curvyo_connector_io::job_send::SendError;
use curvyo_connector_io::pairing::PairError;
use serde::Serialize;

/// Why the connector could not be used; one line of the row each (`ux-notes.md` section 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum LinkError {
    /// No pairing is stored for this profile.
    NotPaired,
    /// This computer has no credential store to keep a pairing in.
    NoCredentialStore,
    /// The typed address is none of `host`, `host:port`, `[v6]:port`.
    BadAddress,
    /// Nothing listens at the address.
    Unreachable { address: String },
    /// No answer in 5 s.
    TimedOut,
    /// The key changed; both fingerprints are in the printed form.
    KeyChanged { stored: String, presented: String },
    /// The connector no longer knows the token.
    NoLongerPaired,
    /// Another protocol version.
    ProtocolMismatch { connector: u32 },
    /// Anything else, as the one fixed text [`OTHER_TEXT`]; the detail goes to the log only.
    Other { message: String },
}

/// What the row says of an error it has no words for.
pub(crate) const OTHER_TEXT: &str = "Connector error";

impl LinkError {
    /// An unmapped error: the detail is logged, the row gets the fixed short text.
    pub(crate) fn other(detail: &impl std::fmt::Display) -> Self {
        eprintln!("curvyo connector: {detail}");
        Self::Other {
            message: OTHER_TEXT.to_owned(),
        }
    }
}

impl From<ClientError> for LinkError {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::BadAddress(_) => Self::BadAddress,
            ClientError::Unreachable(address) => Self::Unreachable { address },
            ClientError::TimedOut => Self::TimedOut,
            ClientError::KeyChanged { stored, presented } => Self::KeyChanged {
                stored: stored.fingerprint(),
                presented: presented.fingerprint(),
            },
            ClientError::NoLongerPaired => Self::NoLongerPaired,
            ClientError::ProtocolMismatch { connector } => Self::ProtocolMismatch { connector },
            other => Self::other(&other),
        }
    }
}

/// The answer of a connection test or a status read.
#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum TestAnswer {
    /// Connected and authenticated.
    Connected {
        controller: Option<String>,
        /// What the controller reported (`specs/0127` G19): the Controller block's rows.
        controller_info: Option<Box<ControllerInfo>>,
        protocol: u32,
        version: String,
        connector_name: String,
        status: Message,
    },
    /// Not connected.
    Failed { error: LinkError },
}

impl TestAnswer {
    pub(crate) fn controller_name(controller: Option<ControllerInfo>) -> Option<String> {
        controller.map(|c| c.name)
    }
}

/// The answer of pairing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum PairAnswer {
    /// Paired; the row shows the name and the key.
    Paired {
        connector_name: String,
        fingerprint: String,
    },
    /// A wrong code (or another key in between).
    WrongCode { attempts_left: u8 },
    /// The third wrong attempt: the code is spent.
    CodeSpent,
    /// The code is older than 10 minutes.
    CodeExpired,
    /// There is no such code (used, or none issued).
    CodeNotValid,
    /// The connection failed.
    Failed { error: LinkError },
}

impl From<PairError> for PairAnswer {
    fn from(error: PairError) -> Self {
        match error {
            PairError::Refused {
                reason: RefusalReason::WrongCode,
                attempts_left: 0,
            } => Self::CodeSpent,
            PairError::Refused {
                reason: RefusalReason::WrongCode,
                attempts_left,
            } => Self::WrongCode { attempts_left },
            PairError::Refused {
                reason: RefusalReason::CodeExpired,
                ..
            } => Self::CodeExpired,
            PairError::Refused {
                reason: RefusalReason::NoCode,
                ..
            }
            | PairError::BadCode => Self::CodeNotValid,
            PairError::Unverified => Self::WrongCode { attempts_left: 0 },
            PairError::Client(e) => Self::Failed { error: e.into() },
        }
    }
}

/// The answer of Send job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum SendAnswer {
    /// The file dialog was cancelled.
    Cancelled,
    /// The connector holds the job.
    Ready {
        name: String,
        lines: u32,
        bytes: u64,
    },
    /// The connector refused it.
    Rejected { name: String, reason: String },
    /// The chosen file could not be read; nothing was sent.
    Unreadable { name: String },
    /// The connection failed.
    Failed { error: LinkError },
}

/// Why sending a picked file failed: the file itself, or the transfer.
#[derive(Debug)]
pub(crate) enum FileSendError {
    /// Reading the file failed (missing, no permission); the connector was not involved.
    Unreadable,
    Send(SendError),
}

impl From<SendError> for FileSendError {
    fn from(error: SendError) -> Self {
        Self::Send(error)
    }
}

/// The words of a refusal, for the wire (`RejectReason` names).
pub(crate) fn reject_name(reason: RejectReason) -> String {
    format!("{reason:?}")
}

impl SendAnswer {
    pub(crate) fn from_error(name: &str, error: FileSendError) -> Self {
        match error {
            FileSendError::Unreadable => Self::Unreadable {
                name: name.to_owned(),
            },
            FileSendError::Send(SendError::Rejected(reason)) => Self::Rejected {
                name: name.to_owned(),
                reason: reject_name(reason),
            },
            FileSendError::Send(SendError::Client(e)) => Self::Failed { error: e.into() },
        }
    }
}

/// The answer of a job command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum CommandAnswer {
    /// Sent; the new state arrives as a status.
    Done,
    /// The state does not allow it.
    WrongState,
    /// The job is not the one the connector holds, or this app does not know it.
    UnknownJob,
    /// The connection failed.
    Failed { error: LinkError },
}

impl CommandAnswer {
    pub(crate) fn from_error(error: ClientError) -> Self {
        match &error {
            ClientError::Protocol(text) if text == "WrongState" => Self::WrongState,
            ClientError::Protocol(text) if text == "UnknownJob" => Self::UnknownJob,
            _ => Self::Failed {
                error: error.into(),
            },
        }
    }
}

/// What the row shows of a stored pairing without a connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingView {
    pub(crate) connector_name: String,
    pub(crate) fingerprint: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_connector_io::spki::SpkiHash;

    #[test]
    fn pairing_refusals_become_the_texts_of_the_row() {
        let refused = |reason, attempts_left| PairError::Refused {
            reason,
            attempts_left,
        };
        assert_eq!(
            PairAnswer::from(refused(RefusalReason::WrongCode, 2)),
            PairAnswer::WrongCode { attempts_left: 2 }
        );
        assert_eq!(
            PairAnswer::from(refused(RefusalReason::WrongCode, 0)),
            PairAnswer::CodeSpent
        );
        assert_eq!(
            PairAnswer::from(refused(RefusalReason::CodeExpired, 0)),
            PairAnswer::CodeExpired
        );
        assert_eq!(
            PairAnswer::from(refused(RefusalReason::NoCode, 0)),
            PairAnswer::CodeNotValid
        );
    }

    #[test]
    fn an_unreadable_file_is_not_a_lost_connection() {
        let answer = SendAnswer::from_error("job.gcode", FileSendError::Unreadable);
        assert_eq!(
            answer,
            SendAnswer::Unreadable {
                name: "job.gcode".to_owned()
            }
        );
        let json = serde_json::to_value(answer).unwrap();
        assert_eq!(json["kind"], "unreadable");
    }

    #[test]
    fn an_unmapped_error_shows_the_fixed_text_not_the_rust_text() {
        let error = LinkError::from(ClientError::Protocol("secret detail".to_owned()));
        assert_eq!(
            error,
            LinkError::Other {
                message: "Connector error".to_owned()
            }
        );
    }

    #[test]
    fn every_enumerated_failure_reaches_its_own_variant() {
        use curvyo_connector_io::spki::SpkiHash;
        let key = SpkiHash::of_spki_der(b"a");
        let cases = [
            (ClientError::BadAddress("x y".to_owned()), "badAddress"),
            (ClientError::Unreachable("h:1".to_owned()), "unreachable"),
            (ClientError::TimedOut, "timedOut"),
            (
                ClientError::KeyChanged {
                    stored: key,
                    presented: key,
                },
                "keyChanged",
            ),
            (ClientError::NoLongerPaired, "noLongerPaired"),
            (
                ClientError::ProtocolMismatch { connector: 3 },
                "protocolMismatch",
            ),
            (ClientError::Tls("x".to_owned()), "other"),
            (ClientError::Io("x".to_owned()), "other"),
        ];
        for (error, kind) in cases {
            let json = serde_json::to_value(LinkError::from(error)).unwrap();
            assert_eq!(json["kind"], kind);
        }
    }

    #[test]
    fn a_changed_key_carries_both_fingerprints() {
        let stored = SpkiHash::of_spki_der(b"a");
        let presented = SpkiHash::of_spki_der(b"b");
        let error = LinkError::from(ClientError::KeyChanged { stored, presented });
        assert_eq!(
            error,
            LinkError::KeyChanged {
                stored: stored.fingerprint(),
                presented: presented.fingerprint()
            }
        );
    }

    #[test]
    fn the_wire_shape_is_tagged_camel_case() {
        let json = serde_json::to_value(PairAnswer::WrongCode { attempts_left: 2 }).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"kind": "wrongCode", "attemptsLeft": 2})
        );
    }

    #[test]
    fn command_errors_map_to_their_answers() {
        assert_eq!(
            CommandAnswer::from_error(ClientError::Protocol("WrongState".to_owned())),
            CommandAnswer::WrongState
        );
        assert_eq!(
            CommandAnswer::from_error(ClientError::Protocol("UnknownJob".to_owned())),
            CommandAnswer::UnknownJob
        );
        assert!(matches!(
            CommandAnswer::from_error(ClientError::TimedOut),
            CommandAnswer::Failed {
                error: LinkError::TimedOut
            }
        ));
    }
}
