//! Pairing a device with the maker's device group (ADR 0040 decision 4): the opener's and the
//! joiner's state machines over relay protocol 3 frames and the pairing channel of
//! `curvyo-crypto-core`, and the check words.
//!
//! Both machines are pure: the caller opens the pairing WebSocket, feeds each received message
//! to `receive` and carries the [`PairOutput`]s out. The opener's [`PairOpener::pair`] is called
//! after the caller wrote `AddMember` for the joiner in the group room
//! ([`ClientSession::add_member`](crate::ClientSession::add_member)), so the joiner is a member
//! when it connects there.

mod joiner;
mod opener;
mod words;

use curvyo_crypto_core::{CryptoError, DocumentId, Identity, PairMessage};

pub use joiner::PairJoiner;
pub use opener::{PAIR_WRONG_TRIES, PairOpener};
pub use words::{CHECK_WORD_COUNT, check_word};

use crate::frame::{ClientFrame, PairCloseReason, RefusalReason};

/// What the caller does next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairOutput {
    /// Send one binary message on the pairing connection.
    Send(Vec<u8>),
    /// Close the pairing connection.
    Close,
    /// Something to show.
    Event(PairEvent),
}

/// Something the pairing screens show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairEvent {
    /// The opener's code to show, with its countdown (criterion 1).
    Code {
        /// The code, shown as `XXXX-XXXX`.
        code: String,
        /// Seconds until the relay lets the slot expire.
        expires_in_s: u32,
    },
    /// Both sides confirmed the code: show the other device and the check words (criterion 2).
    /// The opener then offers Pair and Cancel; the joiner waits for the opener.
    Compare {
        /// The other device's identity (its fingerprint is shown).
        peer: Identity,
        /// The other device's name.
        label: String,
        /// The four check words.
        words: [&'static str; 4],
    },
    /// A joiner typed a wrong code; the opener's code stays valid for `tries_left` more tries.
    WrongCode {
        /// Wrong tries left before the code locks.
        tries_left: u8,
    },
    /// The joiner left before the pairing finished; the opener's code stays valid.
    JoinerLeft,
    /// The pairing finished: the opener admitted the joiner to the group room.
    Paired {
        /// The other device's identity.
        peer: Identity,
        /// The other device's name.
        label: String,
        /// The device group's room.
        group_room: DocumentId,
    },
    /// The pairing ended without a pairing; the connection is closed.
    Failed(PairFailure),
}

/// Why a pairing ended without a pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairFailure {
    /// The relay refused the code or the connection: `unknown`, `expired`, `used`, `locked`,
    /// `busy` and the protocol refusals.
    Refused(RefusalReason),
    /// The opener found the code wrong (joiner side).
    WrongCode,
    /// The code was locked after too many wrong tries.
    Locked,
    /// The other side cancelled.
    Cancelled,
    /// The other side's connection ended.
    Gone,
    /// A check of the other side's message failed: a wrong code seen from the joiner, or a relay
    /// that changed a message.
    Unauthentic,
    /// The other side or the relay broke the protocol.
    Protocol,
}

/// Why a pairing call was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PairingError {
    /// The server address is not one (`normalize_server_address`).
    #[error("not a server address")]
    ServerAddress,
    /// The label is empty or over 255 bytes.
    #[error("a device name needs 1 to 255 bytes")]
    Label,
    /// The call does not fit the pairing's state (Pair before the comparison, a second start).
    #[error("not now")]
    NotNow,
    /// A key operation failed.
    #[error(transparent)]
    Crypto(#[from] CryptoError),
}

/// A pairing message wrapped in its relay frame.
fn send_message(message: &PairMessage, out: &mut Vec<PairOutput>) {
    send_frame(
        &ClientFrame::PairMsg {
            bytes: message.to_bytes(),
        },
        out,
    );
}

fn send_frame(frame: &ClientFrame, out: &mut Vec<PairOutput>) {
    out.push(PairOutput::Send(frame.encode()));
}

/// Sends `PairClose`, closes and reports `failure`.
fn close(reason: Option<PairCloseReason>, failure: PairFailure, out: &mut Vec<PairOutput>) {
    if let Some(reason) = reason {
        send_frame(&ClientFrame::PairClose { reason }, out);
    }
    out.push(PairOutput::Close);
    out.push(PairOutput::Event(PairEvent::Failed(failure)));
}

/// The failure a `PairClose` from the other side means.
fn closed_by_peer(reason: PairCloseReason) -> PairFailure {
    match reason {
        PairCloseReason::Wrong => PairFailure::WrongCode,
        PairCloseReason::Cancelled => PairFailure::Cancelled,
        PairCloseReason::Locked => PairFailure::Locked,
        PairCloseReason::Gone => PairFailure::Gone,
        PairCloseReason::Done => PairFailure::Protocol,
    }
}

/// Checks a device name as a hello carries it.
fn check_label(label: &str) -> Result<(), PairingError> {
    if label.trim().is_empty() || label.len() > 255 {
        Err(PairingError::Label)
    } else {
        Ok(())
    }
}
