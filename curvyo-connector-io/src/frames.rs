//! The frames of this crate beside core's messages: the two `PairSpake` frames each way and
//! `PairRefused` (ADR 0035 §6, addendum A6), and the dispatch of one text frame.

use curvyo_connector_core::{CodecError, MAX_FRAME_BYTES, Message};
use serde::{Deserialize, Serialize};

/// The path the WebSocket is served on; any other path gets HTTP 404.
pub const PATH: &str = "/connector";

/// The size limit of the WebSocket itself: twice the frame limit of the protocol, so a frame
/// between 1 and 2 MiB gets an `Error` and the connection stays open (P5), and a far larger one
/// ends the connection.
pub const SOCKET_LIMIT: usize = 2 * MAX_FRAME_BYTES;

/// Why a pairing attempt got no `Paired`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefusalReason {
    /// The confirmation did not verify: a wrong code, or another key in between.
    WrongCode,
    /// The code is older than 10 minutes.
    CodeExpired,
    /// There is no valid code, it is spent, or another exchange is running.
    NoCode,
}

/// A frame only `-io` knows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IoFrame {
    /// One SPAKE2 step: round 1 carries the public message (and the app's name), round 2 the
    /// confirmation.
    PairSpake {
        /// 1 or 2.
        round: u8,
        /// The app's name, in the app's round 1 only.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_name: Option<String>,
        /// Lower-case hex.
        message: String,
    },
    /// The connector ends the exchange.
    PairRefused {
        /// Why.
        reason: RefusalReason,
        /// Attempts the code still allows.
        attempts_left: u8,
    },
}

/// What one text frame holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// A message of core's protocol.
    Core(Message),
    /// A frame of this crate.
    Io(IoFrame),
}

#[derive(Deserialize)]
struct Peek<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
}

/// Reads one text frame: the size check first, then `PairSpake` and `PairRefused` as `-io`
/// frames, everything else through core's decoder.
///
/// # Errors
/// As [`curvyo_connector_core::decode`].
pub fn decode(text: &str) -> Result<Frame, CodecError> {
    if text.len() > MAX_FRAME_BYTES {
        return Err(CodecError::FrameTooLarge { len: text.len() });
    }
    let io = serde_json::from_str::<Peek>(text)
        .is_ok_and(|p| p.kind == "PairSpake" || p.kind == "PairRefused");
    if io {
        return serde_json::from_str(text)
            .map(Frame::Io)
            .map_err(|e| CodecError::Malformed(e.to_string()));
    }
    curvyo_connector_core::decode(text).map(Frame::Core)
}

/// Writes one frame.
///
/// # Errors
/// [`CodecError::Encode`].
pub fn encode(frame: &Frame) -> Result<String, CodecError> {
    match frame {
        Frame::Core(message) => curvyo_connector_core::encode(message),
        Frame::Io(io) => serde_json::to_string(io).map_err(|e| CodecError::Encode(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_sends_each_type_to_its_decoder() {
        let spake = r#"{"type":"PairSpake","round":2,"message":"ab"}"#;
        assert!(matches!(
            decode(spake),
            Ok(Frame::Io(IoFrame::PairSpake { round: 2, .. }))
        ));
        assert!(matches!(
            decode(r#"{"type":"GetStatus"}"#),
            Ok(Frame::Core(Message::GetStatus))
        ));
        assert!(matches!(
            decode(r#"{"type":"PairSpake","round":"x"}"#),
            Err(CodecError::Malformed(_))
        ));
        assert!(matches!(
            decode(r#"{"type":"Nope"}"#),
            Err(CodecError::UnknownType(_))
        ));
        assert!(matches!(decode("not json"), Err(CodecError::Malformed(_))));
    }

    #[test]
    fn the_size_limit_is_checked_first() {
        let big = format!(
            r#"{{"type":"PairSpake","round":1,"message":"{}"}}"#,
            "a".repeat(MAX_FRAME_BYTES)
        );
        assert!(matches!(
            decode(&big),
            Err(CodecError::FrameTooLarge { .. })
        ));
    }
}
