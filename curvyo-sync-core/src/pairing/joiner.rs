//! The joiner's side of a pairing (device B, ADR 0040 decision 4): it joins the slot the code
//! names, runs `CPace` as initiator, sends its confirmation and sealed hello, shows the check
//! words and waits for the opener's Pair, which names the group room.

use curvyo_crypto_core::{
    IdentityKeys, PairChannel, PairCode, PairMessage, PairRole, PairSecret, PairStart,
};

use super::{
    PairEvent, PairFailure, PairOutput, PairingError, check_label, close, closed_by_peer,
    send_frame, send_message, words,
};
use crate::frame::{ClientFrame, PairCloseReason, RelayFrame};
use crate::server_address::normalize_server_address;

/// Where the joiner stands.
enum Stage {
    /// Created; `start` not called.
    Idle,
    /// `PairJoin` sent, waiting for the relay.
    Joining,
    /// Share sent, waiting for the opener's share.
    Sharing { start: Box<PairStart> },
    /// Confirmation sent, waiting for the opener's.
    Confirming { channel: Box<PairChannel> },
    /// Words shown, waiting for the opener's Pair.
    Waiting {
        channel: Box<PairChannel>,
        peer: curvyo_crypto_core::Identity,
        label: String,
    },
    /// Paired, cancelled or failed.
    Ended,
}

/// The joiner's state machine.
pub struct PairJoiner {
    server: String,
    slot: u16,
    secret: PairSecret,
    identity: IdentityKeys,
    label: String,
    entropy: [u8; 32],
    stage: Stage,
}

impl PairJoiner {
    /// A joiner for the code `code` (as typed) on the relay at `server` (normalised before it is
    /// bound into the channel), with this device's own keys and name; `entropy` is 32 fresh
    /// bytes for the `CPace` scalar.
    ///
    /// # Errors
    /// `ServerAddress`, `Label`, `Crypto(Malformed)` for a text that is not a code.
    pub fn new(
        server: &str,
        code: &str,
        identity: IdentityKeys,
        label: &str,
        entropy: [u8; 32],
    ) -> Result<Self, PairingError> {
        let server = normalize_server_address(server).ok_or(PairingError::ServerAddress)?;
        check_label(label)?;
        let code = PairCode::parse(code)?;
        Ok(Self {
            server,
            slot: code.slot(),
            secret: code.secret(),
            identity,
            label: label.to_owned(),
            entropy,
            stage: Stage::Idle,
        })
    }

    /// Joins the code's slot; call once the pairing connection is open.
    ///
    /// # Errors
    /// `NotNow` after the first call.
    pub fn start(&mut self) -> Result<Vec<PairOutput>, PairingError> {
        if !matches!(self.stage, Stage::Idle) {
            return Err(PairingError::NotNow);
        }
        self.stage = Stage::Joining;
        let mut out = Vec::new();
        send_frame(&ClientFrame::PairJoin { slot: self.slot }, &mut out);
        Ok(out)
    }

    /// The maker cancelled on this device.
    pub fn cancel(&mut self) -> Vec<PairOutput> {
        let mut out = Vec::new();
        if !matches!(self.stage, Stage::Ended) {
            self.stage = Stage::Ended;
            close(
                Some(PairCloseReason::Cancelled),
                PairFailure::Cancelled,
                &mut out,
            );
        }
        out
    }

    /// The pairing connection closed under the joiner.
    pub fn closed(&mut self) -> Vec<PairOutput> {
        let mut out = Vec::new();
        if !matches!(self.stage, Stage::Ended) {
            self.stage = Stage::Ended;
            out.push(PairOutput::Event(PairEvent::Failed(PairFailure::Gone)));
        }
        out
    }

    /// Handles one message from the relay.
    pub fn receive(&mut self, bytes: &[u8]) -> Vec<PairOutput> {
        if matches!(self.stage, Stage::Ended) {
            return Vec::new();
        }
        let Ok(frame) = RelayFrame::decode(bytes) else {
            return self.fail(Some(PairCloseReason::Cancelled), PairFailure::Protocol);
        };
        match frame {
            RelayFrame::PairJoined { sid } if matches!(self.stage, Stage::Joining) => {
                self.joined(sid)
            }
            RelayFrame::PairMsg { bytes } => self.on_message(&bytes),
            RelayFrame::PairClose { reason } => self.fail(None, closed_by_peer(reason)),
            RelayFrame::Refused { reason } => self.fail(None, PairFailure::Refused(reason)),
            _ => self.fail(Some(PairCloseReason::Cancelled), PairFailure::Protocol),
        }
    }

    fn joined(&mut self, sid: [u8; 16]) -> Vec<PairOutput> {
        match PairStart::new(
            PairRole::Joiner,
            self.secret,
            &self.server,
            sid,
            &self.entropy,
        ) {
            Ok(start) => {
                let mut out = Vec::new();
                send_message(&start.share_message(), &mut out);
                self.stage = Stage::Sharing {
                    start: Box::new(start),
                };
                out
            }
            Err(_) => self.fail(Some(PairCloseReason::Cancelled), PairFailure::Protocol),
        }
    }

    fn on_message(&mut self, bytes: &[u8]) -> Vec<PairOutput> {
        let Ok(message) = PairMessage::parse(bytes) else {
            return self.fail(Some(PairCloseReason::Cancelled), PairFailure::Protocol);
        };
        match (std::mem::replace(&mut self.stage, Stage::Ended), message) {
            (Stage::Sharing { start }, PairMessage::Share(share)) => self.confirm(*start, &share),
            (Stage::Confirming { mut channel }, confirm @ PairMessage::Confirm { .. }) => {
                let Ok(hello) = channel.open_confirm(&confirm) else {
                    return self.fail(Some(PairCloseReason::Cancelled), PairFailure::Unauthentic);
                };
                let Ok(indices) = channel.check_words(&self.identity.identity()) else {
                    return self.fail(Some(PairCloseReason::Cancelled), PairFailure::Protocol);
                };
                let out = vec![PairOutput::Event(PairEvent::Compare {
                    peer: hello.identity,
                    label: hello.label.clone(),
                    words: words::check_words(indices),
                })];
                self.stage = Stage::Waiting {
                    channel,
                    peer: hello.identity,
                    label: hello.label,
                };
                out
            }
            (
                Stage::Waiting {
                    channel,
                    peer,
                    label,
                },
                welcome @ PairMessage::Welcome { .. },
            ) => match channel.open_welcome(&welcome) {
                Ok(group_room) => vec![
                    PairOutput::Close,
                    PairOutput::Event(PairEvent::Paired {
                        peer,
                        label,
                        group_room,
                    }),
                ],
                Err(_) => self.fail(Some(PairCloseReason::Cancelled), PairFailure::Unauthentic),
            },
            _ => self.fail(Some(PairCloseReason::Cancelled), PairFailure::Protocol),
        }
    }

    /// Completes `CPace` with the opener's share and sends the confirmation and sealed hello.
    fn confirm(&mut self, start: PairStart, opener_share: &[u8; 32]) -> Vec<PairOutput> {
        let confirm = start.finish(opener_share).and_then(|mut channel| {
            let message = channel.confirm_message(&self.identity, &self.label)?;
            Ok((channel, message))
        });
        let Ok((channel, message)) = confirm else {
            return self.fail(Some(PairCloseReason::Cancelled), PairFailure::Unauthentic);
        };
        let mut out = Vec::new();
        send_message(&message, &mut out);
        self.stage = Stage::Confirming {
            channel: Box::new(channel),
        };
        out
    }

    fn fail(&mut self, reason: Option<PairCloseReason>, failure: PairFailure) -> Vec<PairOutput> {
        self.stage = Stage::Ended;
        let mut out = Vec::new();
        close(reason, failure, &mut out);
        out
    }
}
