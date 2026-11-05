//! Tester acceptance tests for `specs/0096-own-devices-sync`, written from the specification and
//! ADR 0040 before the implementation was read: pairing through the relay's pairing slots with
//! used, expired, locked, unknown and busy codes and a relay that swaps session ids or shares
//! (criteria 1 to 5, 30); `reconcile` authorization (criteria 9, 12, 13, 16, 17, 23 to 27); the
//! relay's passkey backups (criteria 34 and 36).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

mod support;

use std::collections::{BTreeMap, VecDeque};

use curvyo_crypto_core::keyring::{
    Entry, EntryHash, EpochWrap, Keyring, Payload, RecipientWrap, Role,
};
use curvyo_crypto_core::{
    BackupContent, BackupKeys, BackupOp, DocumentId, EpochKey, Identity, IdentityKeys, KeyId,
    MemberId, wrap_epoch_key,
};
use curvyo_sync_core::{
    BACKUP_RECORD_MAX, BackupExchange, BackupOutcome, Backups, ClientFrame, ConnectionId,
    DeviceAction, Millis, PAIR_MESSAGE_MAX, PAIR_SLOT_LIFETIME_MS, PAIR_WRONG_TRIES, PairEvent,
    PairFailure, PairJoiner, PairOpener, PairOutput, PairSlots, PairingError, Plan, RefusalReason,
    RelayFrame, RelayOutput, StoreOp, StoredBackup, Target, WorkspaceInput, check_word, reconcile,
};
use support::device;

const SERVER: &str = "ws://192.168.1.20:8787";
const START: Millis = Millis(1_800_000_000_000);
const GROUP_ROOM: DocumentId = DocumentId([0x6e; 16]);

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || haystack.windows(needle.len()).any(|w| w == needle)
}

// ------------------------------------------------------------------------------------------
// Pairing world: opener and joiners driven through `PairSlots`, with a relay that may tamper.
// ------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Who {
    Opener,
    Joiner(usize),
}

struct Party<M> {
    conn: ConnectionId,
    machine: M,
    events: Vec<PairEvent>,
    closed: bool,
}

type Tamper = Box<dyn FnMut(Who, &mut RelayFrame)>;

struct World {
    slots: PairSlots,
    now: Millis,
    next_conn: u64,
    next_entropy: u8,
    /// Every byte any party or the relay sent.
    captured: Vec<u8>,
    stores: usize,
    opener: Option<Party<PairOpener>>,
    joiners: Vec<Party<PairJoiner>>,
    queue: VecDeque<Step>,
    tamper: Option<Tamper>,
}

enum Step {
    ToRelay(ConnectionId, Vec<u8>),
    ClientGone(ConnectionId),
    ToClient(Who, Vec<u8>),
    RelayClosed(Who),
}

impl World {
    fn new() -> Self {
        Self {
            slots: PairSlots::new(),
            now: START,
            next_conn: 0,
            next_entropy: 0,
            captured: Vec::new(),
            stores: 0,
            opener: None,
            joiners: Vec::new(),
            queue: VecDeque::new(),
            tamper: None,
        }
    }

    fn conn(&mut self) -> ConnectionId {
        self.next_conn += 1;
        ConnectionId(self.next_conn)
    }

    fn entropy(&mut self) -> [u8; 32] {
        self.next_entropy = self.next_entropy.wrapping_add(1);
        let mut out = [0; 32];
        for (i, b) in out.iter_mut().enumerate() {
            *b = self
                .next_entropy
                .wrapping_mul(53)
                .wrapping_add(u8::try_from(i).unwrap());
        }
        out
    }

    fn who(&self, conn: ConnectionId) -> Option<Who> {
        if self.opener.as_ref().is_some_and(|p| p.conn == conn) {
            return Some(Who::Opener);
        }
        self.joiners
            .iter()
            .position(|p| p.conn == conn)
            .map(Who::Joiner)
    }

    fn conn_of(&self, who: Who) -> ConnectionId {
        match who {
            Who::Opener => self.opener.as_ref().unwrap().conn,
            Who::Joiner(i) => self.joiners[i].conn,
        }
    }

    fn is_closed(&self, who: Who) -> bool {
        match who {
            Who::Opener => self.opener.as_ref().unwrap().closed,
            Who::Joiner(i) => self.joiners[i].closed,
        }
    }

    fn set_closed(&mut self, who: Who) {
        match who {
            Who::Opener => self.opener.as_mut().unwrap().closed = true,
            Who::Joiner(i) => self.joiners[i].closed = true,
        }
    }

    fn handle(&mut self, who: Who, out: Vec<PairOutput>) {
        let conn = self.conn_of(who);
        for o in out {
            match o {
                PairOutput::Send(bytes) => {
                    if !self.is_closed(who) {
                        self.queue.push_back(Step::ToRelay(conn, bytes));
                    }
                }
                PairOutput::Close => {
                    if !self.is_closed(who) {
                        self.set_closed(who);
                        self.queue.push_back(Step::ClientGone(conn));
                    }
                }
                PairOutput::Event(e) => match who {
                    Who::Opener => self.opener.as_mut().unwrap().events.push(e),
                    Who::Joiner(i) => self.joiners[i].events.push(e),
                },
            }
        }
    }

    fn route(&mut self, out: Vec<RelayOutput>) {
        for o in out {
            match o {
                RelayOutput::Send {
                    connection,
                    mut frame,
                } => {
                    if let Some(who) = self.who(connection) {
                        if let Some(t) = self.tamper.as_mut() {
                            t(who, &mut frame);
                        }
                        self.queue.push_back(Step::ToClient(who, frame.encode()));
                    }
                }
                RelayOutput::Close { connection } => {
                    if let Some(who) = self.who(connection) {
                        self.queue.push_back(Step::RelayClosed(who));
                    }
                }
                RelayOutput::Store(_) => self.stores += 1,
            }
        }
    }

    fn settle(&mut self) {
        let mut guard = 0;
        while let Some(step) = self.queue.pop_front() {
            guard += 1;
            assert!(guard < 10_000, "pairing does not settle");
            match step {
                Step::ToRelay(conn, bytes) => {
                    self.captured.extend_from_slice(&bytes);
                    let entropy = self.entropy();
                    let out = self.slots.receive(conn, &bytes, self.now, entropy);
                    self.route(out);
                }
                Step::ClientGone(conn) => {
                    let out = self.slots.disconnect(conn);
                    self.route(out);
                }
                Step::ToClient(who, bytes) => {
                    if self.is_closed(who) {
                        continue;
                    }
                    self.captured.extend_from_slice(&bytes);
                    let out = match who {
                        Who::Opener => self.opener.as_mut().unwrap().machine.receive(&bytes),
                        Who::Joiner(i) => self.joiners[i].machine.receive(&bytes),
                    };
                    self.handle(who, out);
                }
                Step::RelayClosed(who) => {
                    if self.is_closed(who) {
                        continue;
                    }
                    self.set_closed(who);
                    let out = match who {
                        Who::Opener => self.opener.as_mut().unwrap().machine.closed(),
                        Who::Joiner(i) => self.joiners[i].machine.closed(),
                    };
                    self.handle(who, out);
                }
            }
        }
    }

    fn open(&mut self, n: u8, label: &str) -> String {
        let mut machine = PairOpener::new(SERVER, device(n), label, [n; 32]).unwrap();
        let out = machine.start().unwrap();
        let conn = self.conn();
        self.opener = Some(Party {
            conn,
            machine,
            events: Vec::new(),
            closed: false,
        });
        self.handle(Who::Opener, out);
        self.settle();
        self.opener_events()
            .iter()
            .find_map(|e| match e {
                PairEvent::Code { code, .. } => Some(code.clone()),
                _ => None,
            })
            .expect("a code")
    }

    fn join(&mut self, code: &str, keys: IdentityKeys, label: &str) -> usize {
        let salt = u8::try_from(self.joiners.len()).unwrap();
        let mut machine = PairJoiner::new(SERVER, code, keys, label, [0x90 ^ salt; 32]).unwrap();
        let out = machine.start().unwrap();
        let conn = self.conn();
        self.joiners.push(Party {
            conn,
            machine,
            events: Vec::new(),
            closed: false,
        });
        let index = self.joiners.len() - 1;
        self.handle(Who::Joiner(index), out);
        self.settle();
        index
    }

    fn opener_events(&self) -> &[PairEvent] {
        &self.opener.as_ref().unwrap().events
    }

    fn joiner_events(&self, i: usize) -> &[PairEvent] {
        &self.joiners[i].events
    }

    fn pair(&mut self) -> Result<(), PairingError> {
        let out = self.opener.as_mut().unwrap().machine.pair(GROUP_ROOM)?;
        self.handle(Who::Opener, out);
        self.settle();
        Ok(())
    }

    fn advance(&mut self, ms: u64) {
        self.now = self.now.plus(ms);
        let out = self.slots.expire(self.now);
        self.route(out);
        self.settle();
    }

    /// Sends a raw client frame on a fresh connection; returns the relay's frames to it.
    fn raw(&mut self, frames: &[ClientFrame]) -> Vec<RelayFrame> {
        let conn = self.conn();
        let mut got = Vec::new();
        for frame in frames {
            let entropy = self.entropy();
            for o in self.slots.receive(conn, &frame.encode(), self.now, entropy) {
                match o {
                    RelayOutput::Send { connection, frame } if connection == conn => {
                        got.push(frame);
                    }
                    RelayOutput::Store(_) => self.stores += 1,
                    _ => {}
                }
            }
        }
        got
    }
}

fn compare(events: &[PairEvent]) -> Option<(Identity, String, [&'static str; 4])> {
    events.iter().find_map(|e| match e {
        PairEvent::Compare { peer, label, words } => Some((*peer, label.clone(), *words)),
        _ => None,
    })
}

fn paired(events: &[PairEvent]) -> bool {
    events.iter().any(|e| matches!(e, PairEvent::Paired { .. }))
}

fn failure(events: &[PairEvent]) -> Option<PairFailure> {
    events.iter().find_map(|e| match e {
        PairEvent::Failed(f) => Some(*f),
        _ => None,
    })
}

fn wrong_code(code: &str) -> String {
    // Change the last character of the secret part to another Crockford digit.
    let mut chars: Vec<char> = code.chars().collect();
    let last = chars.len() - 1;
    chars[last] = if chars[last] == '7' { '8' } else { '7' };
    chars.into_iter().collect()
}

/// Criterion 1: the opener's code is `XXXX-XXXX` with a 5-minute countdown.
#[test]
fn criterion_1_opener_shows_a_code_and_5_minutes() {
    let mut w = World::new();
    let code = w.open(1, "Desktop");
    assert_eq!(code.len(), 9);
    assert_eq!(&code[4..5], "-");
    let expires = w
        .opener_events()
        .iter()
        .find_map(|e| match e {
            PairEvent::Code { expires_in_s, .. } => Some(*expires_in_s),
            _ => None,
        })
        .unwrap();
    assert_eq!(u64::from(expires), PAIR_SLOT_LIFETIME_MS / 1000);
    assert_eq!(PAIR_SLOT_LIFETIME_MS, 5 * 60 * 1000);
}

/// Criteria 2 and 3: both sides see the other device and the same four words; the joiner has
/// nothing before Pair; after Pair both report the pairing with the group room.
#[test]
fn criteria_2_3_honest_pairing_same_words_nothing_before_pair() {
    let mut w = World::new();
    let code = w.open(1, "ZQX-PC-7731 desktop");
    let j = w.join(&code, device(2), "ZQX-PC-7731 laptop");

    let (peer_on_opener, label_on_opener, words_on_opener) =
        compare(w.opener_events()).expect("opener compares");
    let (peer_on_joiner, label_on_joiner, words_on_joiner) =
        compare(w.joiner_events(j)).expect("joiner compares");
    assert_eq!(peer_on_opener, device(2).identity());
    assert_eq!(peer_on_joiner, device(1).identity());
    assert_eq!(label_on_opener, "ZQX-PC-7731 laptop");
    assert_eq!(label_on_joiner, "ZQX-PC-7731 desktop");
    assert_eq!(words_on_opener, words_on_joiner);
    for word in words_on_opener {
        assert!((0..2048).any(|i| check_word(i) == Some(word)));
    }
    assert!(!paired(w.joiner_events(j)), "joiner paired before Pair");
    assert!(!paired(w.opener_events()));

    w.pair().unwrap();
    assert!(w.joiner_events(j).iter().any(|e| matches!(
        e,
        PairEvent::Paired { peer, group_room, .. } if *peer == device(1).identity() && *group_room == GROUP_ROOM
    )));
    assert!(w.opener_events().iter().any(|e| matches!(
        e,
        PairEvent::Paired { peer, .. } if *peer == device(2).identity()
    )));
    assert_eq!(w.stores, 0, "pairing wrote to the relay's store");
}

/// Pair before the comparison is refused.
#[test]
fn criterion_2_pair_before_compare_is_not_now() {
    let mut w = World::new();
    let _ = w.open(1, "Desktop");
    assert_eq!(w.pair(), Err(PairingError::NotNow));
}

/// Criteria 3 and 30: no private key, no code secret, no label and no group room id in any
/// byte sent by either side or by the relay.
#[test]
fn criteria_3_30_nothing_secret_on_the_wire() {
    let mut w = World::new();
    let code = w.open(1, "ZQX-PC-7731 desktop");
    let _ = w.join(&code, device(2), "ZQX-PC-7731 laptop");
    w.pair().unwrap();
    for n in [1, 2] {
        let entropy = device(n).to_entropy();
        for window in entropy.chunks(16) {
            assert!(
                !contains(&w.captured, window),
                "private key bytes of {n} on the wire"
            );
        }
    }
    assert!(!contains(&w.captured, b"ZQX-PC-7731"));
    assert!(!contains(&w.captured, &GROUP_ROOM.0));
    assert!(!contains(&w.captured, code.as_bytes()));
    assert!(!contains(&w.captured, code.replace('-', "").as_bytes()));
    assert!(!contains(&w.captured, &code.as_bytes()[5..]));
}

/// Criterion 4: a used code is refused with `used`, then `expired`, then `unknown`.
#[test]
fn criterion_4_used_then_expired_then_unknown() {
    let mut w = World::new();
    let code = w.open(1, "Desktop");
    let _ = w.join(&code, device(2), "Laptop");
    w.pair().unwrap();
    let c = w.join(&code, device(3), "Thief");
    assert_eq!(
        failure(w.joiner_events(c)),
        Some(PairFailure::Refused(RefusalReason::Used))
    );
    assert!(compare(w.joiner_events(c)).is_none());
    w.advance(PAIR_SLOT_LIFETIME_MS + 1);
    let c = w.join(&code, device(3), "Thief");
    assert_eq!(
        failure(w.joiner_events(c)),
        Some(PairFailure::Refused(RefusalReason::Expired))
    );
    w.advance(PAIR_SLOT_LIFETIME_MS + 1);
    let c = w.join(&code, device(3), "Thief");
    assert_eq!(
        failure(w.joiner_events(c)),
        Some(PairFailure::Refused(RefusalReason::Unknown))
    );
    assert_eq!(w.stores, 0);
}

/// Criterion 4: a code nobody opened answers `unknown`.
#[test]
fn criterion_4_unknown_code() {
    let mut w = World::new();
    let i = w.join("ABCD-EFGH", device(3), "C");
    assert_eq!(
        failure(w.joiner_events(i)),
        Some(PairFailure::Refused(RefusalReason::Unknown))
    );
}

/// Criterion 4: a code not used within 5 minutes expires; the opener learns it too.
#[test]
fn criterion_4_open_code_expires_after_5_minutes() {
    let mut w = World::new();
    let code = w.open(1, "Desktop");
    w.advance(PAIR_SLOT_LIFETIME_MS - 1);
    assert!(failure(w.opener_events()).is_none(), "expired early");
    w.advance(2);
    assert!(failure(w.opener_events()).is_some(), "opener not told");
    let c = w.join(&code, device(3), "C");
    assert_eq!(
        failure(w.joiner_events(c)),
        Some(PairFailure::Refused(RefusalReason::Expired))
    );
}

/// Criterion 4: five wrong codes in a row lock the code; the right code is then refused.
#[test]
fn criterion_4_five_wrong_tries_lock_the_code() {
    let mut w = World::new();
    let code = w.open(1, "Desktop");
    let wrong = wrong_code(&code);
    for attempt in 0..PAIR_WRONG_TRIES {
        let j = w.join(&wrong, device(3), "Guess");
        assert!(
            compare(w.joiner_events(j)).is_none(),
            "try {attempt} compared"
        );
        assert!(
            failure(w.joiner_events(j)).is_some(),
            "try {attempt} not ended"
        );
    }
    let tries: Vec<u8> = w
        .opener_events()
        .iter()
        .filter_map(|e| match e {
            PairEvent::WrongCode { tries_left } => Some(*tries_left),
            _ => None,
        })
        .collect();
    assert_ne!(tries.len(), 0);
    assert!(tries.windows(2).all(|p| p[1] < p[0]), "{tries:?}");
    let right = w.join(&code, device(2), "Laptop");
    assert!(compare(w.joiner_events(right)).is_none());
    assert!(!paired(w.joiner_events(right)));
    assert_eq!(
        failure(w.joiner_events(right)),
        Some(PairFailure::Refused(RefusalReason::Locked))
    );
    assert_eq!(failure(w.opener_events()), Some(PairFailure::Locked));
    let last = w.joiners.len() - 2;
    assert_eq!(failure(w.joiner_events(last)), Some(PairFailure::Locked));
    assert_eq!(w.stores, 0);
}

/// A wrong try followed by the right code still pairs (the counter is "in a row", and the code
/// stays valid after a wrong try).
#[test]
fn criterion_4_right_code_after_a_wrong_try_pairs() {
    let mut w = World::new();
    let code = w.open(1, "Desktop");
    let bad = w.join(&wrong_code(&code), device(3), "Guess");
    assert_eq!(failure(w.joiner_events(bad)), Some(PairFailure::WrongCode));
    let good = w.join(&code, device(2), "Laptop");
    assert!(compare(w.joiner_events(good)).is_some());
    w.pair().unwrap();
    assert!(paired(w.joiner_events(good)));
}

/// A second joiner while the first is connected is `busy` and sees nothing.
#[test]
fn criterion_4_second_joiner_is_busy() {
    let mut w = World::new();
    let code = w.open(1, "Desktop");
    let first = w.join(&code, device(2), "Laptop");
    assert!(compare(w.joiner_events(first)).is_some());
    let second = w.join(&code, device(3), "Thief");
    assert!(compare(w.joiner_events(second)).is_none());
    assert_eq!(
        failure(w.joiner_events(second)),
        Some(PairFailure::Refused(RefusalReason::Busy))
    );
    // The first pairing is undisturbed.
    w.pair().unwrap();
    assert!(paired(w.joiner_events(first)));
    assert!(!paired(w.joiner_events(second)));
}

/// Criterion 2: Cancel on the opener ends the joiner's wait without a pairing.
#[test]
fn criterion_2_cancel_ends_the_joiner() {
    let mut w = World::new();
    let code = w.open(1, "Desktop");
    let j = w.join(&code, device(2), "Laptop");
    let out = w.opener.as_mut().unwrap().machine.cancel();
    w.handle(Who::Opener, out);
    w.settle();
    assert_eq!(failure(w.joiner_events(j)), Some(PairFailure::Cancelled));
    assert!(!paired(w.joiner_events(j)));
}

/// Criterion 5: the opener's connection ends while the joiner waits: the joiner does not pair
/// on its own.
#[test]
fn criterion_5_opener_gone_never_pairs() {
    let mut w = World::new();
    let code = w.open(1, "Desktop");
    let j = w.join(&code, device(2), "Laptop");
    let conn = w.conn_of(Who::Opener);
    w.set_closed(Who::Opener);
    w.queue.push_back(Step::ClientGone(conn));
    w.settle();
    w.advance(PAIR_SLOT_LIFETIME_MS * 3);
    assert!(!paired(w.joiner_events(j)));
}

/// A joiner that leaves before Pair does not use up the code.
#[test]
fn criterion_4_joiner_left_keeps_the_code() {
    let mut w = World::new();
    let code = w.open(1, "Desktop");
    let j = w.join(&code, device(2), "Laptop");
    let out = w.joiners[j].machine.cancel();
    w.handle(Who::Joiner(j), out);
    w.settle();
    assert!(
        w.opener_events()
            .iter()
            .any(|e| matches!(e, PairEvent::JoinerLeft)),
        "{:?}",
        w.opener_events()
    );
    let again = w.join(&code, device(2), "Laptop");
    assert!(compare(w.joiner_events(again)).is_some());
    w.pair().unwrap();
    assert!(paired(w.joiner_events(again)));
}

/// A relay that hands the opener and the joiner different session ids gets no pairing.
#[test]
fn criterion_2_relay_swapping_sid_gets_no_pairing() {
    let mut w = World::new();
    w.tamper = Some(Box::new(|who, frame| {
        if let (Who::Opener, RelayFrame::PairJoined { sid }) = (who, frame) {
            sid[0] ^= 0xff;
        }
    }));
    let code = w.open(1, "Desktop");
    let j = w.join(&code, device(2), "Laptop");
    assert!(compare(w.joiner_events(j)).is_none());
    assert!(compare(w.opener_events()).is_none());
    assert!(w.pair().is_err());
    assert!(!paired(w.joiner_events(j)));
}

/// A relay that replaces the joiner's `CPace` share (the first pairing message) gets no
/// pairing, and nobody shows words.
#[test]
fn criterion_2_relay_swapping_shares_gets_no_pairing() {
    let mut w = World::new();
    let mut first = true;
    w.tamper = Some(Box::new(move |who, frame| {
        if let (Who::Opener, RelayFrame::PairMsg { bytes }) = (who, frame)
            && first
        {
            first = false;
            // A valid-looking share of another party: a fresh share message.
            let fake = curvyo_crypto_core::PairStart::new(
                curvyo_crypto_core::PairRole::Joiner,
                curvyo_crypto_core::PairSecret::from_entropy([1, 2, 3, 4]),
                SERVER,
                [0; 16],
                &[0x77; 32],
            )
            .unwrap()
            .share_message()
            .to_bytes();
            *bytes = fake;
        }
    }));
    let code = w.open(1, "Desktop");
    let j = w.join(&code, device(2), "Laptop");
    assert!(compare(w.opener_events()).is_none());
    assert!(compare(w.joiner_events(j)).is_none());
    assert!(!paired(w.joiner_events(j)));
}

/// A relay that flips a byte in any forwarded pairing message gets no pairing.
#[test]
fn criterion_2_relay_changing_any_message_gets_no_pairing() {
    for target in 0..4 {
        let mut w = World::new();
        let mut seen = 0;
        w.tamper = Some(Box::new(move |_, frame| {
            if let RelayFrame::PairMsg { bytes } = frame {
                if seen == target {
                    let last = bytes.len() - 1;
                    bytes[last] ^= 0x01;
                }
                seen += 1;
            }
        }));
        let code = w.open(1, "Desktop");
        let j = w.join(&code, device(2), "Laptop");
        let _ = w.pair();
        assert!(
            !paired(w.joiner_events(j)),
            "message {target} changed, paired"
        );
    }
}

/// The relay refuses a pairing message over 4 KiB and a second `PairOpen` on one connection.
#[test]
fn criterion_4_relay_limits() {
    let mut w = World::new();
    let got = w.raw(&[ClientFrame::PairOpen, ClientFrame::PairOpen]);
    assert!(
        got.iter().any(|f| matches!(f, RelayFrame::Refused { .. })),
        "{got:?}"
    );
    let mut w = World::new();
    let code = w.open(1, "Desktop");
    let slot = curvyo_crypto_core::PairCode::parse(&code).unwrap().slot();
    let got = w.raw(&[
        ClientFrame::PairJoin { slot },
        ClientFrame::PairMsg {
            bytes: vec![1; PAIR_MESSAGE_MAX + 1],
        },
    ]);
    assert!(
        got.contains(&RelayFrame::Refused {
            reason: RefusalReason::TooLarge
        }),
        "{got:?}"
    );
    // A pairing message first (no open, no join) is refused.
    let got = w.raw(&[ClientFrame::PairMsg { bytes: vec![1] }]);
    assert!(got.iter().any(|f| matches!(f, RelayFrame::Refused { .. })));
    // A slot number out of range.
    let got = w.raw(&[ClientFrame::PairJoin { slot: u16::MAX }]);
    assert!(got.iter().any(|f| matches!(f, RelayFrame::Refused { .. })));
    assert_eq!(w.stores, 0);
}

/// At most 5 joins per slot: a sixth is refused even with the right code.
#[test]
fn criterion_4_six_joins_are_refused() {
    let mut w = World::new();
    let code = w.open(1, "Desktop");
    for _ in 0..5 {
        let j = w.join(&code, device(2), "Laptop");
        let out = w.joiners[j].machine.cancel();
        w.handle(Who::Joiner(j), out);
        w.settle();
    }
    let sixth = w.join(&code, device(2), "Laptop");
    assert!(compare(w.joiner_events(sixth)).is_none());
    assert!(failure(w.joiner_events(sixth)).is_some());
}

/// Criterion 6: a paired device can open a code itself (C pairs through B).
#[test]
fn criterion_6_second_device_opens_a_code() {
    let mut w = World::new();
    let code = w.open(2, "Laptop");
    let c = w.join(&code, device(3), "Workshop");
    let (peer, _, _) = compare(w.opener_events()).unwrap();
    assert_eq!(peer, device(3).identity());
    w.pair().unwrap();
    assert!(paired(w.joiner_events(c)));
}

/// Joiner input checks: bad server, bad label, bad code text.
#[test]
fn criterion_2_joiner_input_is_checked() {
    assert!(matches!(
        PairJoiner::new("not a server", "ABCD-EFGH", device(2), "L", [0; 32]),
        Err(PairingError::ServerAddress)
    ));
    assert!(matches!(
        PairJoiner::new(SERVER, "ABCD-EFGH", device(2), "   ", [0; 32]),
        Err(PairingError::Label)
    ));
    assert!(PairJoiner::new(SERVER, "ABCD-EFGU", device(2), "L", [0; 32]).is_err());
    assert!(matches!(
        PairOpener::new(SERVER, device(1), "", [0; 32]),
        Err(PairingError::Label)
    ));
}

// ------------------------------------------------------------------------------------------
// reconcile
// ------------------------------------------------------------------------------------------

const A: u8 = 1;
const B: u8 = 2;
const C: u8 = 3;
/// A device of another maker.
const M: u8 = 9;
/// A stolen device of the maker.
const T: u8 = 7;
const WS: DocumentId = DocumentId([0x77; 16]);

fn mid(n: u8) -> MemberId {
    device(n).identity().member
}

/// A keyring room with known epoch keys.
struct Room {
    document: DocumentId,
    entries: BTreeMap<EntryHash, Entry>,
    keys: BTreeMap<KeyId, EpochKey>,
    salt: u8,
}

impl Room {
    fn genesis(document: DocumentId, author: u8) -> Self {
        let me = device(author);
        let key = EpochKey::from_bytes([author ^ document.0[0]; 32]);
        let wrap = wrap_epoch_key(&key, 0, document, &me.identity(), [author; 32]).unwrap();
        let entry = Entry::sign(
            Vec::new(),
            &me,
            Payload::Genesis {
                document_id: document,
                admins: vec![me.identity()],
                epoch0_wraps: vec![RecipientWrap {
                    member: me.identity().member,
                    wrap,
                }],
            },
        )
        .unwrap();
        let mut room = Self {
            document,
            entries: BTreeMap::new(),
            keys: BTreeMap::new(),
            salt: 0,
        };
        room.keys.insert(key.key_id(), key);
        room.entries.insert(entry.try_hash().unwrap(), entry);
        room
    }

    fn keyring(&self) -> Keyring {
        let entries: Vec<Entry> = self.entries.values().cloned().collect();
        Keyring::replay(&entries).unwrap()
    }

    fn salt(&mut self) -> [u8; 32] {
        self.salt = self.salt.wrapping_add(1);
        [self.salt; 32]
    }

    fn write_on(&mut self, parents: Vec<EntryHash>, author: u8, payload: Payload) -> EntryHash {
        let entry = Entry::sign(parents, &device(author), payload).unwrap();
        let hash = entry.try_hash().unwrap();
        self.entries.insert(hash, entry);
        hash
    }

    fn write(&mut self, author: u8, payload: Payload) -> EntryHash {
        let heads = self.keyring().heads();
        self.write_on(heads, author, payload)
    }

    fn add_payload(&mut self, author: u8, who: u8, role: Role) -> Payload {
        let keyring = self.keyring();
        let identity = device(who).identity();
        let mut wraps: Vec<EpochWrap> = Vec::new();
        for (epoch, wrap) in keyring.wraps_for(&mid(author)) {
            if wraps.iter().any(|w| w.wrap.key_id() == wrap.key_id()) {
                continue;
            }
            let key = self.keys[&wrap.key_id()].clone();
            let salt = self.salt();
            wraps.push(EpochWrap {
                epoch: epoch.number,
                wrap: wrap_epoch_key(&key, epoch.number, self.document, &identity, salt).unwrap(),
            });
        }
        Payload::AddMember {
            identity,
            role,
            wraps,
        }
    }

    fn add(&mut self, author: u8, who: u8, role: Role) {
        let payload = self.add_payload(author, who, role);
        self.write(author, payload);
    }

    fn remove(&mut self, author: u8, who: u8) {
        self.write(author, Payload::RemoveMember { member: mid(who) });
    }

    fn rotate_payload(&mut self) -> Payload {
        let keyring = self.keyring();
        let epoch = keyring.highest_epoch_number() + 1;
        let salt = self.salt();
        let key = EpochKey::from_bytes(salt);
        self.keys.insert(key.key_id(), key.clone());
        let mut wraps = Vec::new();
        for p in keyring.participants() {
            let s = self.salt();
            wraps.push(RecipientWrap {
                member: p.identity.member,
                wrap: wrap_epoch_key(&key, epoch, self.document, &p.identity, s).unwrap(),
            });
        }
        Payload::NewEpoch { epoch, wraps }
    }

    fn rotate(&mut self, author: u8) {
        let payload = self.rotate_payload();
        self.write(author, payload);
    }

    /// Applies a plan as `author` would.
    fn apply(&mut self, author: u8, plan: &Plan) {
        for action in &plan.actions {
            match action {
                DeviceAction::Add { identity, role } => {
                    let who = [A, B, C, M, T]
                        .into_iter()
                        .find(|&n| device(n).identity() == *identity)
                        .unwrap();
                    self.add(author, who, *role);
                }
                DeviceAction::Remove { members } => {
                    for m in members {
                        self.write(author, Payload::RemoveMember { member: *m });
                    }
                    self.rotate(author);
                }
                DeviceAction::Rotate => self.rotate(author),
            }
        }
    }
}

/// A group of the given devices, all admins (DEV-ROLE A), started by the first.
fn group(devices: &[u8]) -> Room {
    let mut g = Room::genesis(GROUP_ROOM, devices[0]);
    for &d in &devices[1..] {
        g.add(devices[0], d, Role::Admin);
    }
    g
}

fn plan_for(plans: &[Plan], target: Target) -> Option<&Plan> {
    plans.iter().find(|p| p.target == target)
}

fn adds(plan: &Plan) -> Vec<(MemberId, Role)> {
    plan.actions
        .iter()
        .filter_map(|a| match a {
            DeviceAction::Add { identity, role } => Some((identity.member, *role)),
            _ => None,
        })
        .collect()
}

fn removes(plan: &Plan) -> Vec<MemberId> {
    plan.actions
        .iter()
        .flat_map(|a| match a {
            DeviceAction::Remove { members } => members.clone(),
            _ => Vec::new(),
        })
        .collect()
}

fn ws_input(ws: &Keyring, on: bool) -> WorkspaceInput<'_> {
    WorkspaceInput { on, keyring: ws }
}

/// Criteria 9 and 16: switching on admits every group device with the admin's role; once
/// applied, nothing is left to do.
#[test]
fn criterion_9_switch_on_admits_every_group_device() {
    let g = group(&[A, B, C]).keyring();
    let mut w = Room::genesis(WS, A);
    let plans = reconcile(&g, &[ws_input(&w.keyring(), true)], mid(A));
    let plan = plan_for(&plans, Target::Workspace(WS)).expect("a plan for W");
    assert!(!plan.needs_admin);
    let mut added = adds(plan);
    added.sort();
    let mut expected = vec![(mid(B), Role::Admin), (mid(C), Role::Admin)];
    expected.sort();
    assert_eq!(added, expected);
    assert_eq!(removes(plan).len(), 0);
    let plan = plan.clone();
    w.apply(A, &plan);
    let plans = reconcile(&g, &[ws_input(&w.keyring(), true)], mid(A));
    assert!(
        plan_for(&plans, Target::Workspace(WS)).is_none(),
        "{plans:?}"
    );
}

/// Criterion 12: a workspace that is off and has only this device gets nothing.
#[test]
fn criterion_12_off_workspace_gets_nothing() {
    let g = group(&[A, B]).keyring();
    let w = Room::genesis(WS, A);
    let plans = reconcile(&g, &[ws_input(&w.keyring(), false)], mid(A));
    assert!(
        plan_for(&plans, Target::Workspace(WS)).is_none(),
        "{plans:?}"
    );
}

/// Criterion 16 (role): the added role is the lower of the writer's role in W and the device's
/// group role.
#[test]
fn criterion_16_added_role_is_the_lower_one() {
    let mut g = Room::genesis(GROUP_ROOM, A);
    g.add(A, B, Role::Member);
    let g = g.keyring();
    let w = Room::genesis(WS, A);
    let plans = reconcile(&g, &[ws_input(&w.keyring(), true)], mid(A));
    let plan = plan_for(&plans, Target::Workspace(WS)).unwrap();
    assert_eq!(adds(plan), vec![(mid(B), Role::Member)]);
}

/// Criterion 17: in a workspace where this device is a member only it writes nothing and the
/// plan reports it.
#[test]
fn criterion_17_member_only_writes_nothing() {
    let g = group(&[A, B]).keyring();
    let mut w = Room::genesis(WS, M);
    w.add(M, A, Role::Member);
    let plans = reconcile(&g, &[ws_input(&w.keyring(), true)], mid(A));
    let plan = plan_for(&plans, Target::Workspace(WS)).expect("reported");
    assert!(plan.needs_admin);
}

/// Criterion 16: M (another maker, admin of W) sees B added by A with A's role; if A is admin
/// of W in a workspace M started, B gets admin.
#[test]
fn criterion_16_shared_workspace_admin_adds_with_its_role() {
    let g = group(&[A, B]).keyring();
    let mut w = Room::genesis(WS, M);
    w.add(M, A, Role::Admin);
    let plans = reconcile(&g, &[ws_input(&w.keyring(), true)], mid(A));
    let plan = plan_for(&plans, Target::Workspace(WS)).unwrap();
    assert!(!plan.needs_admin);
    assert_eq!(adds(plan), vec![(mid(B), Role::Admin)]);
    // M is never touched.
    assert!(!removes(plan).contains(&mid(M)));
}

/// ADR 0040 decision 5: a group device that another maker's admin removed is withheld, not
/// added back.
#[test]
fn criterion_16_device_removed_by_another_maker_is_withheld() {
    let g = group(&[A, B]).keyring();
    let mut w = Room::genesis(WS, M);
    w.add(M, A, Role::Admin);
    w.add(A, B, Role::Admin);
    w.remove(M, B);
    w.rotate(M);
    let plans = reconcile(&g, &[ws_input(&w.keyring(), true)], mid(A));
    let plan = plan_for(&plans, Target::Workspace(WS)).expect("reported");
    assert!(adds(plan).is_empty(), "{plan:?}");
    assert_eq!(plan.withheld, vec![mid(B)]);
}

/// Criterion 13: switching off removes the other group devices (one rotation), not this one and
/// not another maker's device.
#[test]
fn criterion_13_switch_off_removes_the_other_group_devices() {
    let g = group(&[A, B, C]).keyring();
    let mut w = Room::genesis(WS, A);
    w.add(A, B, Role::Admin);
    w.add(A, C, Role::Admin);
    w.add(A, M, Role::Member);
    let plans = reconcile(&g, &[ws_input(&w.keyring(), false)], mid(A));
    let plan = plan_for(&plans, Target::Workspace(WS)).expect("a plan");
    let mut gone = removes(plan);
    gone.sort();
    let mut expected = vec![mid(B), mid(C)];
    expected.sort();
    assert_eq!(gone, expected);
    let plan = plan.clone();
    w.apply(A, &plan);
    let k = w.keyring();
    assert!(k.role(&mid(M)).is_some());
    assert!(k.role(&mid(A)).is_some());
    let plans = reconcile(&g, &[ws_input(&k, false)], mid(A));
    assert!(
        plan_for(&plans, Target::Workspace(WS)).is_none(),
        "{plans:?}"
    );
}

/// Criterion 13: switching on again after an off re-admits the devices the switch-off removed.
#[test]
fn criterion_13_on_again_readmits() {
    let g = group(&[A, B]).keyring();
    let mut w = Room::genesis(WS, A);
    w.add(A, B, Role::Admin);
    let plans = reconcile(&g, &[ws_input(&w.keyring(), false)], mid(A));
    let plan = plan_for(&plans, Target::Workspace(WS)).unwrap().clone();
    w.apply(A, &plan);
    let plans = reconcile(&g, &[ws_input(&w.keyring(), true)], mid(A));
    let plan = plan_for(&plans, Target::Workspace(WS)).expect("re-admit");
    assert_eq!(adds(plan), vec![(mid(B), Role::Admin)]);
    assert_eq!(plan.withheld.len(), 0);
}

/// Criteria 23 and 24: a device removed from the group is removed from W with a rotation and is
/// never added again; the removed device itself plans nothing.
#[test]
fn criterion_23_removed_device_is_removed_and_never_readded() {
    let mut g = group(&[A, B, C]);
    g.remove(A, B);
    g.rotate(A);
    let gk = g.keyring();
    let mut w = Room::genesis(WS, A);
    w.add(A, B, Role::Admin);
    w.add(A, C, Role::Admin);
    let plans = reconcile(&gk, &[ws_input(&w.keyring(), true)], mid(A));
    let plan = plan_for(&plans, Target::Workspace(WS)).unwrap().clone();
    assert_eq!(removes(&plan), vec![mid(B)]);
    assert_eq!(adds(&plan).len(), 0);
    w.apply(A, &plan);
    let wk = w.keyring();
    // B holds nothing of the current epoch.
    assert!(!wk.holds(&mid(B), wk.current_epoch()));
    for me in [A, C] {
        let plans = reconcile(&gk, &[ws_input(&wk, true)], mid(me));
        assert!(
            plan_for(&plans, Target::Workspace(WS)).is_none(),
            "{plans:?}"
        );
    }
    // B, removed, plans nothing anywhere.
    assert_eq!(reconcile(&gk, &[ws_input(&wk, true)], mid(B)).len(), 0);
}

/// Criterion 23: a removed device with a stale view of the group cannot re-add itself: with the
/// current group keyring its own reconcile is empty.
#[test]
fn criterion_24_removed_device_does_not_readd_itself() {
    let mut g = group(&[A, B]);
    g.remove(A, B);
    g.rotate(A);
    let gk = g.keyring();
    let mut w = Room::genesis(WS, A);
    w.add(A, B, Role::Admin);
    w.remove(A, B);
    w.rotate(A);
    let plans = reconcile(&gk, &[ws_input(&w.keyring(), true)], mid(B));
    assert!(plans.is_empty(), "{plans:?}");
}

/// Criterion 26: removing a device where this device is not admin writes nothing there, but
/// the other workspaces are still done.
#[test]
fn criterion_26_non_admin_workspace_reported_others_done() {
    let mut g = group(&[A, B]);
    g.remove(A, B);
    g.rotate(A);
    let gk = g.keyring();
    let mut w1 = Room::genesis(WS, A);
    w1.add(A, B, Role::Admin);
    let ws2 = DocumentId([0x78; 16]);
    let mut w2 = Room::genesis(ws2, M);
    w2.add(M, A, Role::Member);
    w2.add(M, B, Role::Member);
    let k1 = w1.keyring();
    let k2 = w2.keyring();
    let plans = reconcile(&gk, &[ws_input(&k1, true), ws_input(&k2, true)], mid(A));
    let p1 = plan_for(&plans, Target::Workspace(WS)).unwrap();
    assert!(!p1.needs_admin);
    assert_eq!(removes(p1), vec![mid(B)]);
    let p2 = plan_for(&plans, Target::Workspace(ws2)).expect("reported");
    assert!(p2.needs_admin);
}

/// Criterion 27: after B leaves (own removal in group and W), A rotates both rooms.
#[test]
fn criterion_27_leave_makes_admin_rotate() {
    let mut g = group(&[A, B]);
    let mut w = Room::genesis(WS, A);
    w.add(A, B, Role::Admin);
    g.remove(B, B);
    w.remove(B, B);
    let gk = g.keyring();
    let wk = w.keyring();
    assert!(gk.role(&mid(B)).is_none());
    let plans = reconcile(&gk, &[ws_input(&wk, true)], mid(A));
    let pw = plan_for(&plans, Target::Workspace(WS)).expect("rotation pending in W");
    assert!(pw.actions.contains(&DeviceAction::Rotate), "{pw:?}");
    let pg = plan_for(&plans, Target::Group).expect("rotation pending in the group");
    assert!(pg.actions.contains(&DeviceAction::Rotate), "{pg:?}");
    // After the rotation B holds nothing current and the plans are empty.
    let pw = pw.clone();
    w.apply(A, &pw);
    let pg = pg.clone();
    g.apply(A, &pg);
    let wk = w.keyring();
    assert!(!wk.holds(&mid(B), wk.current_epoch()));
    let plans = reconcile(&g.keyring(), &[ws_input(&wk, true)], mid(A));
    assert!(plans.is_empty(), "{plans:?}");
}

/// Criterion 27 (re-pair after leave): a device that left and was paired again is admitted
/// again (its last removal was its own).
#[test]
fn criterion_27_left_device_paired_again_is_readmitted() {
    let mut g = group(&[A, B]);
    let mut w = Room::genesis(WS, A);
    w.add(A, B, Role::Admin);
    g.remove(B, B);
    w.remove(B, B);
    g.rotate(A);
    w.rotate(A);
    g.add(A, B, Role::Admin);
    let plans = reconcile(&g.keyring(), &[ws_input(&w.keyring(), true)], mid(A));
    let plan = plan_for(&plans, Target::Workspace(WS)).expect("re-admit");
    assert_eq!(adds(plan), vec![(mid(B), Role::Admin)]);
}

/// Not in the group (never paired): nothing planned.
#[test]
fn criterion_7_unpaired_device_plans_nothing() {
    let g = group(&[A, B]).keyring();
    let w = Room::genesis(WS, C);
    assert_eq!(
        reconcile(&g, &[ws_input(&w.keyring(), true)], mid(C)).len(),
        0
    );
}

/// ADR 0040 decision 8: in a workspace room whose only admins A and B removed each other at
/// once (restored admin set), reconcile still adds a newly paired device C and plans no
/// removal, even for B whom the group removed (it could never settle).
#[test]
fn decision_8_mutual_removal_restored_room_still_adds() {
    let mut g = group(&[A, B, C]);
    g.remove(A, B);
    g.rotate(A);
    let gk = g.keyring();
    let mut w = Room::genesis(WS, A);
    w.add(A, B, Role::Admin);
    let heads = w.keyring().heads();
    w.write_on(heads.clone(), A, Payload::RemoveMember { member: mid(B) });
    w.write_on(heads, B, Payload::RemoveMember { member: mid(A) });
    let wk = w.keyring();
    assert!(wk.admin_set_restored(), "test setup: restored admin set");
    assert!(wk.role(&mid(A)).is_some());
    let plans = reconcile(&gk, &[ws_input(&wk, true)], mid(A));
    let plan = plan_for(&plans, Target::Workspace(WS)).expect("C is added");
    assert!(adds(plan).iter().any(|(m, _)| *m == mid(C)), "{plan:?}");
    assert!(removes(plan).is_empty(), "{plan:?}");
}

/// ADR 0040 decision 8: a group whose two devices removed each other at once keeps both while
/// they are the only admins; neither plans anything that removes the other.
#[test]
fn decision_8_mutual_removal_in_the_group_freezes() {
    let mut g = group(&[A, B]);
    let heads = g.keyring().heads();
    g.write_on(heads.clone(), A, Payload::RemoveMember { member: mid(B) });
    g.write_on(heads, B, Payload::RemoveMember { member: mid(A) });
    let gk = g.keyring();
    assert!(gk.admin_set_restored());
    assert!(gk.role(&mid(A)).is_some() && gk.role(&mid(B)).is_some());
    let mut w = Room::genesis(WS, A);
    w.add(A, B, Role::Admin);
    let plans = reconcile(&gk, &[ws_input(&w.keyring(), true)], mid(A));
    for plan in &plans {
        assert!(removes(plan).is_empty(), "{plan:?}");
    }
}

/// Criteria 23 and 24 with a stolen device: T (the maker's own, stolen) removed B from W, then
/// the maker removes T from the group. B is the maker's own device and a current group member;
/// its removal was written by the maker's device, not by another maker (ADR 0040 decision 5:
/// "a removal by anyone else, an admin of another maker, stands"). B must be admitted again and
/// T removed.
#[test]
fn criterion_24_device_kicked_by_a_since_removed_own_device_is_readmitted() {
    let mut g = group(&[A, B, T]);
    let mut w = Room::genesis(WS, A);
    w.add(A, B, Role::Admin);
    w.add(A, T, Role::Admin);
    // The thief removes B from W.
    w.remove(T, B);
    w.rotate(T);
    // The maker removes the stolen T from the group.
    g.remove(A, T);
    g.rotate(A);
    let plans = reconcile(&g.keyring(), &[ws_input(&w.keyring(), true)], mid(A));
    let plan = plan_for(&plans, Target::Workspace(WS)).expect("a plan for W");
    assert_eq!(removes(plan), vec![mid(T)], "{plan:?}");
    assert!(
        adds(plan).iter().any(|(m, _)| *m == mid(B)),
        "the maker's own device B stays locked out of W: {plan:?}"
    );
}

// ------------------------------------------------------------------------------------------
// Backups on the relay (criteria 34, 36; ADR 0040 decision 13)
// ------------------------------------------------------------------------------------------

fn content(label: &str) -> BackupContent {
    BackupContent {
        identity: device(5).to_entropy(),
        group_room: GROUP_ROOM,
        server: "wss://relay.example".to_owned(),
        label: label.to_owned(),
    }
}

struct Relay {
    backups: Backups,
    next: u64,
    stored: Vec<StoreOp>,
}

impl Relay {
    fn new() -> Self {
        Self {
            backups: Backups::restore(Vec::new()),
            next: 100,
            stored: Vec::new(),
        }
    }

    /// One connection carrying `frames` in order; returns the relay's frames to it.
    fn connect(&mut self, frames: &[ClientFrame]) -> Vec<RelayFrame> {
        self.next += 1;
        let conn = ConnectionId(self.next);
        let mut got = Vec::new();
        for f in frames {
            let mut nonce = [0; 32];
            nonce[..8].copy_from_slice(&self.next.to_le_bytes());
            for o in self.backups.receive(conn, &f.encode(), nonce) {
                match o {
                    RelayOutput::Send { frame, .. } => got.push(frame),
                    RelayOutput::Store(op) => self.stored.push(op),
                    RelayOutput::Close { .. } => {}
                }
            }
        }
        self.backups.disconnect(conn);
        got
    }

    /// Begin a write and return the challenge and the connection, keeping it open.
    fn begin(&mut self, id: curvyo_crypto_core::BackupId) -> (ConnectionId, [u8; 32]) {
        self.next += 1;
        let conn = ConnectionId(self.next);
        let mut nonce = [0; 32];
        nonce[..8].copy_from_slice(&self.next.to_le_bytes());
        nonce[31] = 0xcc;
        let out = self
            .backups
            .receive(conn, &ClientFrame::BackupBegin { id }.encode(), nonce);
        let challenge = out
            .iter()
            .find_map(|o| match o {
                RelayOutput::Send {
                    frame: RelayFrame::Challenge { nonce },
                    ..
                } => Some(*nonce),
                _ => None,
            })
            .expect("a challenge");
        (conn, challenge)
    }

    fn send(&mut self, conn: ConnectionId, frame: &ClientFrame) -> Vec<RelayFrame> {
        let mut got = Vec::new();
        for o in self.backups.receive(conn, &frame.encode(), [0; 32]) {
            match o {
                RelayOutput::Send { frame, .. } => got.push(frame),
                RelayOutput::Store(op) => self.stored.push(op),
                RelayOutput::Close { .. } => {}
            }
        }
        got
    }

    fn put(&mut self, keys: &BackupKeys, record: &[u8]) -> Vec<RelayFrame> {
        let (conn, nonce) = self.begin(keys.id());
        let signature = keys.sign_write(BackupOp::Put, &nonce, record);
        let got = self.send(
            conn,
            &ClientFrame::BackupPut {
                write_key: keys.write_key(),
                signature,
                record: record.to_vec(),
            },
        );
        self.backups.disconnect(conn);
        got
    }

    /// Runs a `BackupExchange` against this relay to its end.
    fn exchange(&mut self, mut ex: BackupExchange) -> BackupOutcome {
        self.exchange_with(&mut ex, |_| {})
    }

    fn exchange_with(
        &mut self,
        ex: &mut BackupExchange,
        mut tamper: impl FnMut(&mut RelayFrame),
    ) -> BackupOutcome {
        self.next += 1;
        let conn = ConnectionId(self.next);
        let mut pending = vec![ex.first_frame()];
        while let Some(bytes) = pending.pop() {
            let mut nonce = [0; 32];
            nonce[..8].copy_from_slice(&self.next.to_le_bytes());
            for o in self.backups.receive(conn, &bytes, nonce) {
                match o {
                    RelayOutput::Send { mut frame, .. } => {
                        tamper(&mut frame);
                        if let Some(answer) = ex.receive(&frame.encode()) {
                            pending.push(answer);
                        }
                    }
                    RelayOutput::Store(op) => self.stored.push(op),
                    RelayOutput::Close { .. } => {}
                }
            }
        }
        self.backups.disconnect(conn);
        if !ex.finished() {
            ex.closed();
        }
        ex.take_outcome().expect("an outcome")
    }
}

fn refused(frames: &[RelayFrame]) -> Option<RefusalReason> {
    frames.iter().find_map(|f| match f {
        RelayFrame::Refused { reason } => Some(*reason),
        _ => None,
    })
}

/// Criterion 36: back up, then restore with the same passkey: the same identity comes back.
#[test]
fn criterion_36_backup_then_restore_round_trip() {
    let mut relay = Relay::new();
    let prf = [0x42; 32];
    let out = relay.exchange(BackupExchange::write(&prf, &content("Firefox"), &[1; 24]).unwrap());
    assert_eq!(out, BackupOutcome::Done);
    let out = relay.exchange(BackupExchange::read(&prf));
    let BackupOutcome::Read(back) = out else {
        panic!("{out:?}");
    };
    assert_eq!(back, content("Firefox"));
    // Another passkey finds nothing.
    let out = relay.exchange(BackupExchange::read(&[0x43; 32]));
    assert_eq!(out, BackupOutcome::Refused(RefusalReason::Unknown));
}

/// Criterion 34: the relay's stored data holds no readable identity, label, server or room.
#[test]
fn criterion_34_relay_store_holds_nothing_readable() {
    let mut relay = Relay::new();
    let prf = [0x42; 32];
    relay.exchange(BackupExchange::write(&prf, &content("ZQX-PC-7731"), &[1; 24]).unwrap());
    let mut bytes = Vec::new();
    for op in &relay.stored {
        if let StoreOp::Backup(StoredBackup {
            id,
            write_key,
            record,
        }) = op
        {
            bytes.extend_from_slice(&id.0);
            bytes.extend_from_slice(&write_key.0);
            bytes.extend_from_slice(record);
        }
    }
    assert!(!bytes.is_empty(), "nothing stored");
    let entropy = device(5).to_entropy();
    for window in entropy.chunks(16) {
        assert!(!contains(&bytes, window));
    }
    assert!(!contains(&bytes, &device(5).identity().member.0));
    assert!(!contains(&bytes, b"ZQX-PC-7731"));
    assert!(!contains(&bytes, b"relay.example"));
    assert!(!contains(&bytes, &GROUP_ROOM.0));
    assert!(!contains(&bytes, &prf));
}

/// Criterion 36: a write needs the signed challenge: a put as the first frame, a put without a
/// valid signature, and a replayed put are refused and store nothing.
#[test]
fn criterion_36_write_needs_the_signed_challenge() {
    let mut relay = Relay::new();
    let keys = BackupKeys::derive(&[0x42; 32]);
    let record = keys.seal(&content("A"), &[1; 24]).unwrap();
    // Put without Begin.
    let got = relay.connect(&[ClientFrame::BackupPut {
        write_key: keys.write_key(),
        signature: keys.sign_write(BackupOp::Put, &[0; 32], &record),
        record: record.clone(),
    }]);
    assert!(refused(&got).is_some(), "{got:?}");
    assert_eq!(relay.stored.len(), 0);
    // Put with a garbage signature.
    let (conn, _nonce) = relay.begin(keys.id());
    let got = relay.send(
        conn,
        &ClientFrame::BackupPut {
            write_key: keys.write_key(),
            signature: [0; 64],
            record: record.clone(),
        },
    );
    assert_eq!(refused(&got), Some(RefusalReason::Forbidden));
    assert_eq!(relay.stored.len(), 0);
    // Honest put, then a captured replay of it on a new connection.
    let (conn, nonce) = relay.begin(keys.id());
    let put = ClientFrame::BackupPut {
        write_key: keys.write_key(),
        signature: keys.sign_write(BackupOp::Put, &nonce, &record),
        record: record.clone(),
    };
    let got = relay.send(conn, &put);
    assert!(got.contains(&RelayFrame::BackupDone), "{got:?}");
    let stored_before = relay.stored.len();
    let (conn, _) = relay.begin(keys.id());
    let got = relay.send(conn, &put);
    assert_eq!(refused(&got), Some(RefusalReason::Forbidden));
    assert_eq!(relay.stored.len(), stored_before);
}

/// Criterion 36: nobody else can replace or delete a record: the first put pins the write key.
#[test]
fn criterion_36_record_cannot_be_replaced_or_deleted_by_others() {
    let mut relay = Relay::new();
    let owner = BackupKeys::derive(&[0x42; 32]);
    let other = BackupKeys::derive(&[0x43; 32]);
    let record = owner.seal(&content("A"), &[1; 24]).unwrap();
    assert!(relay.put(&owner, &record).contains(&RelayFrame::BackupDone));
    // Someone who knows the id puts with their own write key.
    let (conn, nonce) = relay.begin(owner.id());
    let evil = other.seal(&content("Evil"), &[2; 24]).unwrap();
    let got = relay.send(
        conn,
        &ClientFrame::BackupPut {
            write_key: other.write_key(),
            signature: other.sign_write(BackupOp::Put, &nonce, &evil),
            record: evil,
        },
    );
    assert_eq!(refused(&got), Some(RefusalReason::Forbidden));
    // ... or deletes it.
    let (conn, nonce) = relay.begin(owner.id());
    let got = relay.send(
        conn,
        &ClientFrame::BackupDelete {
            signature: other.sign_write(BackupOp::Delete, &nonce, &[]),
        },
    );
    assert_eq!(refused(&got), Some(RefusalReason::Forbidden));
    // A put signature cannot pass as a delete.
    let (conn, nonce) = relay.begin(owner.id());
    let got = relay.send(
        conn,
        &ClientFrame::BackupDelete {
            signature: owner.sign_write(BackupOp::Put, &nonce, &[]),
        },
    );
    assert_eq!(refused(&got), Some(RefusalReason::Forbidden));
    // The owner's record is still there and still the owner's.
    let out = relay.exchange(BackupExchange::read(&[0x42; 32]));
    assert_eq!(out, BackupOutcome::Read(content("A")));
    // The owner replaces it and deletes it.
    let out = relay.exchange(BackupExchange::write(&[0x42; 32], &content("B"), &[3; 24]).unwrap());
    assert_eq!(out, BackupOutcome::Done);
    assert_eq!(
        relay.exchange(BackupExchange::read(&[0x42; 32])),
        BackupOutcome::Read(content("B"))
    );
    assert_eq!(
        relay.exchange(BackupExchange::delete(&[0x42; 32])),
        BackupOutcome::Done
    );
    assert_eq!(
        relay.exchange(BackupExchange::read(&[0x42; 32])),
        BackupOutcome::Refused(RefusalReason::Unknown)
    );
    assert_eq!(relay.backups.count(), 0);
}

/// Criterion 36: 4 KiB is the cap: exactly 4096 bytes are kept, 4097 are `too-large`.
#[test]
fn criterion_36_four_kib_cap() {
    assert_eq!(BACKUP_RECORD_MAX, 4096);
    let mut relay = Relay::new();
    let keys = BackupKeys::derive(&[0x42; 32]);
    let at = vec![0xab; BACKUP_RECORD_MAX];
    assert!(
        relay.put(&keys, &at).contains(&RelayFrame::BackupDone),
        "4096 bytes refused"
    );
    let over = vec![0xab; BACKUP_RECORD_MAX + 1];
    let keys2 = BackupKeys::derive(&[0x44; 32]);
    let got = relay.put(&keys2, &over);
    assert_eq!(refused(&got), Some(RefusalReason::TooLarge), "{got:?}");
    assert_eq!(relay.backups.count(), 1);
}

/// Criterion 36: a record the relay changed is refused by the reader (`Damaged`), never taken.
#[test]
fn criterion_36_tampered_record_is_refused_on_restore() {
    let mut relay = Relay::new();
    let prf = [0x42; 32];
    relay.exchange(BackupExchange::write(&prf, &content("A"), &[1; 24]).unwrap());
    let mut ex = BackupExchange::read(&prf);
    let out = relay.exchange_with(&mut ex, |frame| {
        if let RelayFrame::BackupRecord { record } = frame {
            let i = record.len() / 2;
            record[i] ^= 1;
        }
    });
    assert_eq!(out, BackupOutcome::Damaged);
    // A relay that serves another passkey's record under this id: also damaged.
    let other = BackupKeys::derive(&[0x43; 32])
        .seal(&content("Other"), &[1; 24])
        .unwrap();
    let mut ex = BackupExchange::read(&prf);
    let out = relay.exchange_with(&mut ex, |frame| {
        if let RelayFrame::BackupRecord { record } = frame {
            record.clone_from(&other);
        }
    });
    assert_eq!(out, BackupOutcome::Damaged);
}

/// The relay's restart keeps backups (restored from the store).
#[test]
fn criterion_36_backups_survive_restore() {
    let mut relay = Relay::new();
    let prf = [0x42; 32];
    relay.exchange(BackupExchange::write(&prf, &content("A"), &[1; 24]).unwrap());
    let stored: Vec<StoredBackup> = relay
        .stored
        .iter()
        .filter_map(|op| match op {
            StoreOp::Backup(b) => Some(b.clone()),
            _ => None,
        })
        .collect();
    let mut restarted = Relay::new();
    restarted.backups = Backups::restore(stored);
    assert_eq!(
        restarted.exchange(BackupExchange::read(&prf)),
        BackupOutcome::Read(content("A"))
    );
}

/// A backup connection that ends before an answer fails, it does not hang or claim success.
#[test]
fn criterion_36_closed_exchange_fails() {
    let mut ex = BackupExchange::read(&[1; 32]);
    let _ = ex.first_frame();
    ex.closed();
    assert_eq!(ex.take_outcome(), Some(BackupOutcome::Failed));
}
