//! A client's session with the relay (ADR 0031 decisions 5 to 10): handshake, cursor, outbox,
//! sealing at send time, liveness and reconnect, snapshot trigger, presence, joining, removal.
//!
//! Sans-I/O: the caller opens and closes the socket when told ([`ClientOutput::Connect`],
//! [`ClientOutput::Close`]), feeds received messages and the time in, sends the bytes that come
//! out, and applies the [`SessionEvent`]s to its document. No document type appears here.

mod api;
mod inbound;
mod keys;
mod link;
mod outgoing;
mod presence;

use std::collections::{BTreeMap, VecDeque};

use curvyo_crypto_core::keyring::{Entry, Keyring};
use curvyo_crypto_core::{EpochKey, InviteKey, MemberId};

use crate::entropy::Entropy;
use crate::frame::{ClientFrame, ConnectionId, PROTOCOL_VERSION, RecordKind, RoomId};
use crate::time::Millis;
pub use api::{
    ClientOutput, InviteSecret, SessionConfig, SessionError, SessionEvent, SessionStart,
};
use keys::Keys;
use link::Link;
use presence::Presence;

/// A room log longer than this many records since its snapshot asks for a new one (adrs.md
/// decision 8, Proposal).
pub const SNAPSHOT_RECORDS: u64 = 500;
/// A room log larger than this many bytes since its snapshot asks for a new one.
pub const SNAPSHOT_BYTES: u64 = 4 * 1024 * 1024;

/// How the session started, as far as it still matters.
enum Start {
    Member,
    Share { genesis: Entry, uploaded: bool },
    Join { key: InviteKey, claimed: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle { retry_at: Millis },
    Connecting,
    Handshake,
    Joined { caught_up: bool },
    Ended,
}

struct OutItem {
    kind: RecordKind,
    plaintext: Vec<u8>,
    sent: bool,
}

/// One client's session with one room.
pub struct ClientSession {
    room: RoomId,
    keys: Keys,
    entropy: Entropy,
    start: Start,
    invite_expires: Option<u64>,
    awaiting_admission: bool,
    state: State,
    link: Link,
    presence: Presence,
    peers: BTreeMap<ConnectionId, MemberId>,
    delivered: u64,
    pending: VecDeque<(u64, RecordKind, Vec<u8>)>,
    outbox: BTreeMap<u64, OutItem>,
    next_client_id: u64,
    pending_snapshot: Option<(u64, Vec<u8>)>,
    /// A workspace snapshot's sealed parts waiting for the connection, with what it covers.
    pending_parts: Option<(u64, Vec<Vec<u8>>)>,
    /// For a workspace room (ADR 0036 decision 7): the bytes of the last snapshot, which set
    /// when the next one is due; `None` for a document's room.
    workspace_snapshot: Option<u64>,
    /// The sequence and the bytes so far of the snapshot being received in parts.
    receiving_snapshot: (u64, u64),
    since_snapshot: (u64, u64),
    snapshot_asked: bool,
    rejected_shown: bool,
    /// The highest sequence the room was known to hold when this session last resynced from
    /// the snapshot; a rejected record up to it starts no new resync.
    resynced_through: Option<u64>,
    /// The highest sequence the relay reported or delivered.
    head_seq: u64,
    out: Vec<ClientOutput>,
}

impl ClientSession {
    /// A session that asks to connect at once.
    ///
    /// # Errors
    /// `Crypto` if the genesis or the invite key cannot be made.
    pub fn new(config: SessionConfig, now: Millis) -> Result<Self, SessionError> {
        let mut entropy = Entropy::new(config.entropy_seed);
        let mut keys = Keys::new(
            config.identity,
            config.room.document_id(),
            config.peer_id,
            config.nonce_seed,
        );
        let awaiting_admission = matches!(config.start, SessionStart::Join(_));
        let start = match config.start {
            SessionStart::Member {
                keyring,
                epoch_keys,
            } => {
                keys.add_keys(epoch_keys);
                keys.add_entries(keyring);
                Start::Member
            }
            SessionStart::Share => Start::Share {
                genesis: keys.write_genesis(&mut entropy)?,
                uploaded: false,
            },
            SessionStart::Join(secret) => Start::Join {
                key: InviteKey::from_seed(&secret.seed)?,
                claimed: false,
            },
        };
        let mut link = Link::new();
        link.heard(now);
        Ok(Self {
            room: config.room,
            keys,
            entropy,
            start,
            invite_expires: None,
            awaiting_admission,
            state: State::Connecting,
            link,
            presence: Presence::default(),
            peers: BTreeMap::new(),
            delivered: config.since_seq,
            pending: VecDeque::new(),
            outbox: BTreeMap::new(),
            next_client_id: 1,
            pending_snapshot: None,
            pending_parts: None,
            workspace_snapshot: None,
            receiving_snapshot: (0, 0),
            since_snapshot: (0, 0),
            snapshot_asked: false,
            rejected_shown: false,
            resynced_through: None,
            head_seq: config.since_seq,
            out: vec![ClientOutput::Connect],
        })
    }

    /// Takes everything the caller must carry out, in order.
    pub fn drain(&mut self) -> Vec<ClientOutput> {
        core::mem::take(&mut self.out)
    }

    /// Makes this the session of a workspace room (ADR 0036 decision 7): a snapshot is due by
    /// [`snapshot_due`](crate::snapshot_due) against the last snapshot's size, and is uploaded
    /// with [`upload_snapshot_parts`](Self::upload_snapshot_parts).
    pub fn use_workspace_snapshots(&mut self) {
        self.workspace_snapshot.get_or_insert(0);
    }

    /// The room.
    #[must_use]
    pub fn room(&self) -> RoomId {
        self.room
    }

    /// This device's member id.
    #[must_use]
    pub fn me(&self) -> MemberId {
        self.keys.me()
    }

    /// The replayed keyring, once one is known.
    #[must_use]
    pub fn keyring(&self) -> Option<&Keyring> {
        self.keys.keyring()
    }

    /// The keyring entries to save as `keyring.log`.
    #[must_use]
    pub fn keyring_entries(&self) -> Vec<Entry> {
        self.keys.keyring().map_or_else(
            || self.keys.entries().to_vec(),
            |k| k.accepted_entries().into_iter().cloned().collect(),
        )
    }

    /// The epoch keys held, for the credential store.
    #[must_use]
    pub fn epoch_keys(&self) -> Vec<(u32, EpochKey)> {
        self.keys.held_keys()
    }

    /// The last record sequence the caller has received in order (`share.json` `server_seq`).
    #[must_use]
    pub fn cursor(&self) -> u64 {
        self.pending
            .front()
            .map_or(self.delivered, |(seq, _, _)| seq.saturating_sub(1))
    }

    /// Whether the session is connected, caught up and holds the current key.
    #[must_use]
    pub fn is_online(&self) -> bool {
        self.state == State::Joined { caught_up: true } && self.keys.holds_any_key()
    }

    /// Whether the session has ended for good (removed, refused, invite expired).
    #[must_use]
    pub fn has_ended(&self) -> bool {
        self.state == State::Ended
    }

    /// The socket opened.
    pub fn connected(&mut self, now: Millis) {
        if self.state != State::Connecting {
            return;
        }
        self.link.heard(now);
        self.state = State::Handshake;
        self.send(&ClientFrame::Hello {
            protocol: PROTOCOL_VERSION,
            room: self.room,
            since_seq: self.cursor(),
            identity: self.keys.me(),
        });
    }

    /// The socket closed or failed to open.
    pub fn disconnected(&mut self, now: Millis) {
        if matches!(self.state, State::Ended | State::Idle { .. }) {
            return;
        }
        self.lose(now, false);
    }

    /// The platform reports the network is back (the browser's `online` event): a reconnect
    /// waiting for its backoff is tried now, from the first delay again, so a merge after a long
    /// outage does not wait up to 10 s (criterion 18). Connected or ended: nothing.
    pub fn network_back(&mut self, now: Millis) {
        if let State::Idle { .. } = self.state {
            self.link.reset_backoff();
            self.state = State::Idle { retry_at: now };
            self.tick(now);
        }
    }

    /// Time passed: pings, loss detection, reconnects, presence and invite expiry.
    pub fn tick(&mut self, now: Millis) {
        match self.state {
            State::Ended => return,
            State::Idle { retry_at } => {
                if now >= retry_at {
                    self.state = State::Connecting;
                    self.link.heard(now);
                    self.out.push(ClientOutput::Connect);
                }
                return;
            }
            State::Connecting | State::Handshake | State::Joined { .. } => {
                if self.link.is_lost(now) {
                    self.lose(now, true);
                    return;
                }
            }
        }
        if let State::Joined { caught_up } = self.state {
            if self.link.ping_due(now) {
                self.send(&ClientFrame::Ping);
            }
            if caught_up {
                self.tick_joined(now);
            }
        }
    }

    fn tick_joined(&mut self, now: Millis) {
        if !self.keys.holds_any_key()
            && self
                .invite_expires
                .is_some_and(|expires| expires <= now.unix_seconds())
        {
            self.event(SessionEvent::InviteExpired);
            self.end();
            return;
        }
        self.send_presence(now);
        for connection in self.presence.expire(now) {
            self.event(SessionEvent::PresenceGone { connection });
        }
    }

    /// The link is gone: offline, peers gone, a reconnect scheduled.
    fn lose(&mut self, now: Millis, close: bool) {
        let was_online = self.is_online();
        if close {
            self.out.push(ClientOutput::Close);
        }
        if was_online {
            self.event(SessionEvent::Offline);
        }
        self.peers.clear();
        for connection in self.presence.forget_all() {
            self.event(SessionEvent::PresenceGone { connection });
        }
        for item in self.outbox.values_mut() {
            item.sent = false;
        }
        let retry_at = self.link.next_attempt(now, &mut self.entropy);
        self.state = State::Idle { retry_at };
    }

    fn end(&mut self) {
        self.state = State::Ended;
        self.out.push(ClientOutput::Close);
    }

    fn send(&mut self, frame: &ClientFrame) {
        self.out.push(ClientOutput::Send(frame.encode()));
    }

    fn event(&mut self, event: SessionEvent) {
        self.out.push(ClientOutput::Event(event));
    }
}
