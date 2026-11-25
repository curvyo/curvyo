//! Converts between messages and frames (one JSON text each), with typed errors.

use crate::message::{ErrorCode, Message};

/// The largest frame, in bytes (ADR 0035 §5).
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// Every `type` of protocol version 0, to tell "unknown type" from "malformed".
const KNOWN_TYPES: [&str; 43] = [
    "Hello",
    "Pair",
    "Auth",
    "GetStatus",
    "SubmitJob",
    "JobChunk",
    "Start",
    "Pause",
    "Resume",
    "Stop",
    "Unlock",
    "Home",
    "Jog",
    "JogKeep",
    "JogStop",
    "GoToPoint",
    "ListPorts",
    "ApplyConfig",
    "GetConfig",
    "IssueCode",
    "ListClients",
    "RevokeClient",
    "GetLog",
    "Reset",
    "MoveJob",
    "RemoveJob",
    "SetRunNext",
    "JobList",
    "Welcome",
    "Authenticated",
    "Ports",
    "ConfigApplied",
    "Config",
    "CodeIssued",
    "Clients",
    "ClientRevoked",
    "LogLines",
    "ResetDone",
    "Paired",
    "JobAccepted",
    "JobRejected",
    "Status",
    "Error",
];

/// Why a frame could not be decoded or a message not encoded.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CodecError {
    /// The frame is longer than [`MAX_FRAME_BYTES`]; it was not parsed.
    #[error("frame of {len} bytes is above the limit of {MAX_FRAME_BYTES}")]
    FrameTooLarge {
        /// The frame's length in bytes.
        len: usize,
    },
    /// Not JSON, or a known type with a missing or wrongly typed field.
    #[error("malformed message: {0}")]
    Malformed(String),
    /// A JSON object whose `type` this version does not know.
    #[error("unknown message type {0:?}")]
    UnknownType(String),
    /// The message cannot be written as JSON.
    #[error("cannot encode message: {0}")]
    Encode(String),
}

impl CodecError {
    /// The `Error` message the connector sends back; the connection stays open.
    #[must_use]
    pub fn to_error_message(&self) -> Message {
        let (code, message) = match self {
            CodecError::FrameTooLarge { .. } => {
                (ErrorCode::FrameTooLarge, "A frame is at most 1 MiB.")
            }
            CodecError::UnknownType(_) => (ErrorCode::UnknownType, "Unknown message type."),
            CodecError::Malformed(_) | CodecError::Encode(_) => (
                ErrorCode::Malformed,
                "The message is not valid JSON or has a missing or wrongly typed field.",
            ),
        };
        Message::Error {
            code,
            message: message.to_owned(),
        }
    }
}

/// Writes a message as one JSON text.
///
/// # Errors
/// [`CodecError::Encode`] when a number is not finite.
pub fn encode(message: &Message) -> Result<String, CodecError> {
    serde_json::to_string(message).map_err(|e| CodecError::Encode(e.to_string()))
}

/// Reads one frame.
///
/// # Errors
/// [`CodecError::FrameTooLarge`] before any parsing, then [`CodecError::Malformed`] or
/// [`CodecError::UnknownType`].
pub fn decode(frame: &str) -> Result<Message, CodecError> {
    if frame.len() > MAX_FRAME_BYTES {
        return Err(CodecError::FrameTooLarge { len: frame.len() });
    }
    match serde_json::from_str::<Message>(frame) {
        Ok(message) => Ok(message),
        Err(error) => Err(classify(frame, &error)),
    }
}

fn classify(frame: &str, error: &serde_json::Error) -> CodecError {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(frame) else {
        return CodecError::Malformed(error.to_string());
    };
    match value.get("type").and_then(serde_json::Value::as_str) {
        Some(name) if !KNOWN_TYPES.contains(&name) => CodecError::UnknownType(name.to_owned()),
        _ => CodecError::Malformed(error.to_string()),
    }
}
