//! The relay's pairing slots (ADR 0040 decision 3): it hands an opener a free random slot and
//! forwards pairing messages between the opener and its one current joiner, by the relay's
//! clock, in memory only.
//!
//! Every join gets a fresh random `CPace` session id, sent to both sides in `PairJoined`, so no
//! two attempts on one slot share a `sid` (draft-irtf-cfrg-cpace-21 asks for unique sids).
//!
//! A slot answers joins until its lifetime ends; after a finished pairing it answers `used`, after
//! a lock `locked`, after its lifetime `expired`, and after two lifetimes it is forgotten
//! (`unknown`). A cancelled slot, or one whose opener left, is forgotten at once. Nothing here is
//! stored or logged: the outputs are sends and closes only.

use std::collections::BTreeMap;

use super::RelayOutput;
use crate::entropy::Entropy;
use crate::frame::{
    ClientFrame, ConnectionId, FrameError, PairCloseReason, RefusalReason, RelayFrame,
};
use crate::time::Millis;

/// How long a slot answers joins: 5 minutes (Proposal).
pub const PAIR_SLOT_LIFETIME_MS: u64 = 5 * 60 * 1000;
/// How many joins one slot takes before it is locked (Proposal).
pub const PAIR_JOINS_PER_SLOT: u8 = 5;
/// How many slots, open or remembered, one relay holds (Proposal).
pub const PAIR_SLOTS_PER_SERVER: usize = 4096;
/// The largest pairing message the relay forwards: 4 KiB, far above the pairing messages v1
/// (Proposal).
pub const PAIR_MESSAGE_MAX: usize = 4096;

/// The number of slots, `2^15`.
const SLOT_COUNT: u16 = 1 << 15;
/// Random draws before a linear search for a free slot.
const DRAWS: usize = 64;

/// How far a slot's pairing got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotState {
    Open,
    Used,
    Locked,
}

#[derive(Debug)]
struct Slot {
    opener: Option<ConnectionId>,
    joiner: Option<ConnectionId>,
    opened: Millis,
    joins: u8,
    state: SlotState,
}

impl Slot {
    fn expired(&self, now: Millis) -> bool {
        now.since(self.opened) >= PAIR_SLOT_LIFETIME_MS
    }

    fn forgotten(&self, now: Millis) -> bool {
        now.since(self.opened) >= 2 * PAIR_SLOT_LIFETIME_MS
    }
}

/// Which side of which slot a connection is.
#[derive(Debug, Clone, Copy)]
enum Party {
    Opener(u16),
    Joiner(u16),
}

/// Every pairing slot and every pairing connection of one relay.
#[derive(Debug, Default)]
pub struct PairSlots {
    slots: BTreeMap<u16, Slot>,
    parties: BTreeMap<ConnectionId, Party>,
}

impl PairSlots {
    /// A relay without slots.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of slots held, open or remembered.
    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Handles one message from a pairing connection (`connection` is numbered by the server).
    /// `now` is the relay's clock; `entropy` is 32 fresh bytes, used if a slot is opened or
    /// joined.
    pub fn receive(
        &mut self,
        connection: ConnectionId,
        bytes: &[u8],
        now: Millis,
        entropy: [u8; 32],
    ) -> Vec<RelayOutput> {
        let mut out = self.expire(now);
        let frame = match ClientFrame::decode(bytes) {
            Ok(frame) => frame,
            Err(FrameError::TooLarge) => {
                self.refuse(connection, RefusalReason::TooLarge, &mut out);
                return out;
            }
            Err(FrameError::Malformed) => {
                self.refuse(connection, RefusalReason::Protocol, &mut out);
                return out;
            }
        };
        let verdict = match (self.parties.get(&connection).copied(), frame) {
            (None, ClientFrame::PairOpen) => self.open(connection, now, entropy, &mut out),
            (None, ClientFrame::PairJoin { slot }) => {
                self.join(connection, slot, now, entropy, &mut out)
            }
            (Some(party), ClientFrame::PairMsg { bytes }) => self.forward(party, bytes, &mut out),
            (Some(Party::Opener(slot)), ClientFrame::PairClose { reason }) => {
                self.opener_close(slot, reason, &mut out)
            }
            (Some(Party::Joiner(slot)), ClientFrame::PairClose { reason }) => {
                self.joiner_close(slot, reason, &mut out)
            }
            _ => Err(RefusalReason::Protocol),
        };
        if let Err(reason) = verdict {
            self.refuse(connection, reason, &mut out);
        }
        out
    }

    /// Forgets a closed connection and tells the other side of its slot.
    pub fn disconnect(&mut self, connection: ConnectionId) -> Vec<RelayOutput> {
        let mut out = Vec::new();
        self.leave(connection, &mut out);
        out
    }

    /// Ends the slots whose lifetime passed by `now` (refusing both their parties as expired)
    /// and forgets those two lifetimes old.
    pub fn expire(&mut self, now: Millis) -> Vec<RelayOutput> {
        let mut out = Vec::new();
        let mut ended = Vec::new();
        for slot in self.slots.values_mut() {
            if slot.expired(now) {
                ended.extend(slot.opener.take());
                ended.extend(slot.joiner.take());
            }
        }
        for connection in ended {
            self.parties.remove(&connection);
            send_refused(connection, RefusalReason::Expired, &mut out);
        }
        self.slots.retain(|_, slot| !slot.forgotten(now));
        out
    }

    fn open(
        &mut self,
        connection: ConnectionId,
        now: Millis,
        entropy: [u8; 32],
        out: &mut Vec<RelayOutput>,
    ) -> Result<(), RefusalReason> {
        if self.slots.len() >= PAIR_SLOTS_PER_SERVER {
            return Err(RefusalReason::Busy);
        }
        let mut entropy = Entropy::new(entropy);
        let number = self.free_slot(&mut entropy);
        self.slots.insert(
            number,
            Slot {
                opener: Some(connection),
                joiner: None,
                opened: now,
                joins: 0,
                state: SlotState::Open,
            },
        );
        self.parties.insert(connection, Party::Opener(number));
        let expires_in_s = u32::try_from(PAIR_SLOT_LIFETIME_MS / 1000).unwrap_or(u32::MAX);
        out.push(RelayOutput::Send {
            connection,
            frame: RelayFrame::PairSlot {
                slot: number,
                expires_in_s,
            },
        });
        Ok(())
    }

    /// A random slot not held; the caller made sure one is free.
    fn free_slot(&self, entropy: &mut Entropy) -> u16 {
        let mut candidate = 0;
        for _ in 0..DRAWS {
            let bytes = entropy.secret();
            candidate = u16::from_le_bytes([bytes[0], bytes[1]]) % SLOT_COUNT;
            if !self.slots.contains_key(&candidate) {
                return candidate;
            }
        }
        (0..SLOT_COUNT)
            .map(|step| (candidate + step) % SLOT_COUNT)
            .find(|number| !self.slots.contains_key(number))
            .unwrap_or(candidate)
    }

    fn join(
        &mut self,
        connection: ConnectionId,
        number: u16,
        now: Millis,
        entropy: [u8; 32],
        out: &mut Vec<RelayOutput>,
    ) -> Result<(), RefusalReason> {
        let slot = self.slots.get_mut(&number).ok_or(RefusalReason::Unknown)?;
        if slot.expired(now) {
            return Err(RefusalReason::Expired);
        }
        match slot.state {
            SlotState::Used => return Err(RefusalReason::Used),
            SlotState::Locked => return Err(RefusalReason::Locked),
            SlotState::Open => {}
        }
        let opener = slot.opener.ok_or(RefusalReason::Unknown)?;
        if slot.joiner.is_some() {
            return Err(RefusalReason::Busy);
        }
        if slot.joins >= PAIR_JOINS_PER_SLOT {
            slot.state = SlotState::Locked;
            slot.opener = None;
            self.parties.remove(&opener);
            send_close(opener, PairCloseReason::Locked, out);
            out.push(RelayOutput::Close { connection: opener });
            return Err(RefusalReason::Locked);
        }
        slot.joins += 1;
        slot.joiner = Some(connection);
        let mut sid = [0; 16];
        sid.copy_from_slice(&Entropy::new(entropy).secret()[..16]);
        self.parties.insert(connection, Party::Joiner(number));
        for to in [connection, opener] {
            out.push(RelayOutput::Send {
                connection: to,
                frame: RelayFrame::PairJoined { sid },
            });
        }
        Ok(())
    }

    fn forward(
        &self,
        party: Party,
        bytes: Vec<u8>,
        out: &mut Vec<RelayOutput>,
    ) -> Result<(), RefusalReason> {
        if bytes.len() > PAIR_MESSAGE_MAX {
            return Err(RefusalReason::TooLarge);
        }
        let to = match party {
            Party::Opener(number) => self.slots.get(&number).and_then(|slot| slot.joiner),
            Party::Joiner(number) => self.slots.get(&number).and_then(|slot| slot.opener),
        };
        if let Some(connection) = to {
            out.push(RelayOutput::Send {
                connection,
                frame: RelayFrame::PairMsg { bytes },
            });
        }
        Ok(())
    }

    fn opener_close(
        &mut self,
        number: u16,
        reason: PairCloseReason,
        out: &mut Vec<RelayOutput>,
    ) -> Result<(), RefusalReason> {
        let Some(slot) = self.slots.get_mut(&number) else {
            return Err(RefusalReason::Protocol);
        };
        let state = match reason {
            PairCloseReason::Gone => return Err(RefusalReason::Protocol),
            PairCloseReason::Wrong => None,
            PairCloseReason::Cancelled => Some(SlotState::Open),
            PairCloseReason::Locked => Some(SlotState::Locked),
            PairCloseReason::Done => Some(SlotState::Used),
        };
        if let Some(joiner) = slot.joiner.take() {
            self.parties.remove(&joiner);
            send_close(joiner, reason, out);
            out.push(RelayOutput::Close { connection: joiner });
        }
        let Some(state) = state else {
            return Ok(());
        };
        if let Some(opener) = slot.opener.take() {
            self.parties.remove(&opener);
            out.push(RelayOutput::Close { connection: opener });
        }
        if state == SlotState::Open {
            self.slots.remove(&number);
        } else {
            slot.state = state;
        }
        Ok(())
    }

    fn joiner_close(
        &mut self,
        number: u16,
        reason: PairCloseReason,
        out: &mut Vec<RelayOutput>,
    ) -> Result<(), RefusalReason> {
        if reason == PairCloseReason::Gone {
            return Err(RefusalReason::Protocol);
        }
        let Some(slot) = self.slots.get_mut(&number) else {
            return Err(RefusalReason::Protocol);
        };
        if let Some(joiner) = slot.joiner.take() {
            self.parties.remove(&joiner);
            out.push(RelayOutput::Close { connection: joiner });
        }
        if let Some(opener) = slot.opener {
            send_close(opener, reason, out);
        }
        Ok(())
    }

    /// Detaches `connection` from its slot and tells the other side it is gone.
    fn leave(&mut self, connection: ConnectionId, out: &mut Vec<RelayOutput>) {
        let Some(party) = self.parties.remove(&connection) else {
            return;
        };
        match party {
            Party::Opener(number) => {
                let Some(slot) = self.slots.remove(&number) else {
                    return;
                };
                if let Some(joiner) = slot.joiner {
                    self.parties.remove(&joiner);
                    send_close(joiner, PairCloseReason::Gone, out);
                    out.push(RelayOutput::Close { connection: joiner });
                }
            }
            Party::Joiner(number) => {
                if let Some(slot) = self.slots.get_mut(&number) {
                    slot.joiner = None;
                    if let Some(opener) = slot.opener {
                        send_close(opener, PairCloseReason::Gone, out);
                    }
                }
            }
        }
    }

    /// Sends `Refused`, closes `connection` and detaches it from its slot.
    fn refuse(
        &mut self,
        connection: ConnectionId,
        reason: RefusalReason,
        out: &mut Vec<RelayOutput>,
    ) {
        send_refused(connection, reason, out);
        self.leave(connection, out);
    }
}

fn send_close(connection: ConnectionId, reason: PairCloseReason, out: &mut Vec<RelayOutput>) {
    out.push(RelayOutput::Send {
        connection,
        frame: RelayFrame::PairClose { reason },
    });
}

fn send_refused(connection: ConnectionId, reason: RefusalReason, out: &mut Vec<RelayOutput>) {
    out.push(RelayOutput::Send {
        connection,
        frame: RelayFrame::Refused { reason },
    });
    out.push(RelayOutput::Close { connection });
}
