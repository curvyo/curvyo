//! The opener's side of a pairing (device A, ADR 0040 decision 4): it shows the code, answers
//! each joiner's `CPace` share, checks the joiner's confirmation, counts wrong codes, and after
//! the maker's Pair sends the group room.

use curvyo_crypto_core::{
    DocumentId, Identity, IdentityKeys, PairChannel, PairCode, PairMessage, PairRole, PairSecret,
    PairStart,
};

use super::{
    PairEvent, PairFailure, PairOutput, PairingError, check_label, close, closed_by_peer,
    send_frame, send_message, words,
};
use crate::entropy::Entropy;
use crate::frame::{ClientFrame, PairCloseReason, RelayFrame};
use crate::server_address::normalize_server_address;

/// Wrong codes in a row after which the opener locks its code (criterion 4).
pub const PAIR_WRONG_TRIES: u8 = 5;

/// Where the opener stands.
enum Stage {
    /// Created; `start` not called.
    Idle,
    /// `PairOpen` sent, waiting for the slot.
    Opening,
    /// The code is shown; waiting for a joiner.
    Open,
    /// A joiner joined under a fresh `sid`; waiting for its share.
    Waiting { sid: [u8; 16] },
    /// Share answered, waiting for the joiner's confirmation.
    Confirming { channel: Box<PairChannel> },
    /// Both confirmed; waiting for the maker's Pair or Cancel.
    Comparing {
        channel: Box<PairChannel>,
        peer: Identity,
        label: String,
    },
    /// Paired, cancelled or failed.
    Ended,
}

/// The opener's state machine.
pub struct PairOpener {
    server: String,
    identity: IdentityKeys,
    label: String,
    secret: PairSecret,
    entropy: Entropy,
    wrong: u8,
    stage: Stage,
}

impl PairOpener {
    /// An opener for the relay at `server` (normalised before it is bound into the channel), with
    /// this device's keys and name; `entropy` is 32 fresh bytes for the code's secret and every
    /// `CPace` scalar.
    ///
    /// # Errors
    /// `ServerAddress`, `Label`.
    pub fn new(
        server: &str,
        identity: IdentityKeys,
        label: &str,
        entropy: [u8; 32],
    ) -> Result<Self, PairingError> {
        let server = normalize_server_address(server).ok_or(PairingError::ServerAddress)?;
        check_label(label)?;
        let mut entropy = Entropy::new(entropy);
        let drawn = entropy.secret();
        let secret = PairSecret::from_entropy([drawn[0], drawn[1], drawn[2], drawn[3]]);
        Ok(Self {
            server,
            identity,
            label: label.to_owned(),
            secret,
            entropy,
            wrong: 0,
            stage: Stage::Idle,
        })
    }

    /// Asks the relay for a slot; call once the pairing connection is open.
    ///
    /// # Errors
    /// `NotNow` after the first call.
    pub fn start(&mut self) -> Result<Vec<PairOutput>, PairingError> {
        if !matches!(self.stage, Stage::Idle) {
            return Err(PairingError::NotNow);
        }
        self.stage = Stage::Opening;
        let mut out = Vec::new();
        send_frame(&ClientFrame::PairOpen, &mut out);
        Ok(out)
    }

    /// The maker pressed Pair: sends the group room sealed and ends the pairing (criterion 3).
    /// Call it after `AddMember` for the joiner was written to `group_room`.
    ///
    /// # Errors
    /// `NotNow` before the comparison; `Crypto` if sealing fails.
    pub fn pair(&mut self, group_room: DocumentId) -> Result<Vec<PairOutput>, PairingError> {
        if !matches!(self.stage, Stage::Comparing { .. }) {
            return Err(PairingError::NotNow);
        }
        let Stage::Comparing {
            mut channel,
            peer,
            label,
            ..
        } = std::mem::replace(&mut self.stage, Stage::Ended)
        else {
            return Err(PairingError::NotNow);
        };
        let welcome = channel.welcome_message(group_room)?;
        let mut out = Vec::new();
        send_message(&welcome, &mut out);
        send_frame(
            &ClientFrame::PairClose {
                reason: PairCloseReason::Done,
            },
            &mut out,
        );
        out.push(PairOutput::Close);
        out.push(PairOutput::Event(PairEvent::Paired {
            peer,
            label,
            group_room,
        }));
        Ok(out)
    }

    /// The maker cancelled: the relay forgets the slot and tells the joiner.
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

    /// The pairing connection closed under the opener.
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
        let mut out = Vec::new();
        if matches!(self.stage, Stage::Ended) {
            return out;
        }
        let Ok(frame) = RelayFrame::decode(bytes) else {
            return self.fail(PairFailure::Protocol);
        };
        match frame {
            RelayFrame::PairSlot { slot, expires_in_s } if matches!(self.stage, Stage::Opening) => {
                let Ok(code) = PairCode::new(slot, self.secret) else {
                    return self.fail(PairFailure::Protocol);
                };
                self.stage = Stage::Open;
                out.push(PairOutput::Event(PairEvent::Code {
                    code: code.to_string(),
                    expires_in_s,
                }));
            }
            // A new joiner under its own `sid`: its share follows. A pairing in progress was
            // ended by the relay's `PairClose` first, so the stage is `Open` here.
            RelayFrame::PairJoined { sid } if matches!(self.stage, Stage::Open) => {
                self.stage = Stage::Waiting { sid };
            }
            RelayFrame::PairMsg { bytes } => return self.on_message(&bytes),
            RelayFrame::PairClose { reason } => return self.on_close(reason),
            RelayFrame::Refused { reason } => {
                self.stage = Stage::Ended;
                out.push(PairOutput::Close);
                out.push(PairOutput::Event(PairEvent::Failed(PairFailure::Refused(
                    reason,
                ))));
            }
            _ => return self.fail(PairFailure::Protocol),
        }
        out
    }

    fn on_message(&mut self, bytes: &[u8]) -> Vec<PairOutput> {
        let message = PairMessage::parse(bytes).ok();
        match (std::mem::replace(&mut self.stage, Stage::Ended), message) {
            (Stage::Waiting { sid }, Some(PairMessage::Share(share))) => {
                self.answer_share(sid, &share)
            }
            (Stage::Confirming { mut channel }, Some(confirm @ PairMessage::Confirm { .. })) => {
                if let Ok(hello) = channel.open_confirm(&confirm) {
                    self.confirmed(channel, hello.identity, hello.label)
                } else {
                    self.wrong_try()
                }
            }
            (Stage::Waiting { .. } | Stage::Confirming { .. }, _) => self.wrong_try(),
            // Nothing is expected from the joiner while the maker compares the words.
            (stage, _) => {
                self.stage = stage;
                Vec::new()
            }
        }
    }

    /// Answers a joiner's share with this side's share; a share that aborts `CPace` is a wrong
    /// try.
    fn answer_share(&mut self, sid: [u8; 16], peer_share: &[u8; 32]) -> Vec<PairOutput> {
        let entropy = self.entropy.secret();
        let channel = PairStart::new(PairRole::Opener, self.secret, &self.server, sid, &entropy)
            .and_then(|start| {
                let share = start.share_message();
                start.finish(peer_share).map(|channel| (share, channel))
            });
        let Ok((share, channel)) = channel else {
            return self.wrong_try();
        };
        let mut out = Vec::new();
        send_message(&share, &mut out);
        self.stage = Stage::Confirming {
            channel: Box::new(channel),
        };
        out
    }

    /// The joiner's confirmation checked out: send this side's and show the comparison.
    fn confirmed(
        &mut self,
        mut channel: Box<PairChannel>,
        peer: Identity,
        label: String,
    ) -> Vec<PairOutput> {
        self.wrong = 0;
        let own = self.identity.identity();
        let reply = channel
            .confirm_message(&self.identity, &self.label)
            .and_then(|reply| Ok((reply, channel.check_words(&own)?)));
        let Ok((reply, indices)) = reply else {
            return self.fail(PairFailure::Protocol);
        };
        let mut out = Vec::new();
        send_message(&reply, &mut out);
        out.push(PairOutput::Event(PairEvent::Compare {
            peer,
            label: label.clone(),
            words: words::check_words(indices),
        }));
        self.stage = Stage::Comparing {
            channel,
            peer,
            label,
        };
        out
    }

    /// A joiner's try failed: `PairClose{wrong}`, or after [`PAIR_WRONG_TRIES`] in a row
    /// `PairClose{locked}` and the end. The next joiner comes with a fresh `sid`.
    fn wrong_try(&mut self) -> Vec<PairOutput> {
        self.stage = Stage::Open;
        self.wrong = self.wrong.saturating_add(1);
        let mut out = Vec::new();
        if self.wrong >= PAIR_WRONG_TRIES {
            self.stage = Stage::Ended;
            close(Some(PairCloseReason::Locked), PairFailure::Locked, &mut out);
        } else {
            send_frame(
                &ClientFrame::PairClose {
                    reason: PairCloseReason::Wrong,
                },
                &mut out,
            );
            out.push(PairOutput::Event(PairEvent::WrongCode {
                tries_left: PAIR_WRONG_TRIES - self.wrong,
            }));
        }
        out
    }

    /// The joiner or the relay ended the joiner's part: wait for the next joiner, or end when
    /// the relay locked the slot.
    fn on_close(&mut self, reason: PairCloseReason) -> Vec<PairOutput> {
        match reason {
            PairCloseReason::Locked => self.fail(PairFailure::Locked),
            PairCloseReason::Cancelled | PairCloseReason::Gone => match self.stage {
                Stage::Open
                | Stage::Waiting { .. }
                | Stage::Confirming { .. }
                | Stage::Comparing { .. } => {
                    self.stage = Stage::Open;
                    vec![PairOutput::Event(PairEvent::JoinerLeft)]
                }
                Stage::Idle | Stage::Opening | Stage::Ended => self.fail(closed_by_peer(reason)),
            },
            PairCloseReason::Wrong | PairCloseReason::Done => self.fail(PairFailure::Protocol),
        }
    }

    fn fail(&mut self, failure: PairFailure) -> Vec<PairOutput> {
        self.stage = Stage::Ended;
        let mut out = Vec::new();
        close(None, failure, &mut out);
        out
    }
}
