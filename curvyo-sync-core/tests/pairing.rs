//! Pairing a device with the maker's devices (`specs/0096-own-devices-sync` criteria 1 to 6,
//! ADR 0040 decision 4): the opener's and joiner's state machines through the relay's pairing
//! slots, then admission to the device group room in the simulator.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::collections::VecDeque;

use curvyo_crypto_core::keyring::Role;
use curvyo_crypto_core::{IdentityKeys, PairCode, PairSecret};
use curvyo_document_core::Document;
use curvyo_sync_core::{
    CHECK_WORD_COUNT, ConnectionId, Millis, PAIR_SLOT_LIFETIME_MS, PAIR_WRONG_TRIES, PairEvent,
    PairFailure, PairJoiner, PairOpener, PairOutput, PairSlots, PairingError, RefusalReason,
    RelayOutput, RoomId, SessionError, SessionEvent, check_word,
};
use support::device;
use support::sim::Net;

const SERVER: &str = "ws://192.168.1.20:8787";
const GROUP: RoomId = RoomId([0x6e; 16]);
const START: Millis = Millis(1_800_000_000_000);

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

/// The relay's pairing slots and the parties of one opener's code, delivering in order.
struct World {
    slots: PairSlots,
    now: Millis,
    next_conn: u64,
    /// Every message any party sent to the relay.
    captured: Vec<Vec<u8>>,
    opener: Option<Party<PairOpener>>,
    joiners: Vec<Party<PairJoiner>>,
    to_relay: VecDeque<(ConnectionId, Vec<u8>)>,
}

impl World {
    fn new() -> Self {
        Self {
            slots: PairSlots::new(),
            now: START,
            next_conn: 0,
            captured: Vec::new(),
            opener: None,
            joiners: Vec::new(),
            to_relay: VecDeque::new(),
        }
    }

    fn conn(&mut self) -> ConnectionId {
        self.next_conn += 1;
        ConnectionId(self.next_conn)
    }

    /// Device `n` opens a code on `server`; returns the code as shown.
    fn open(&mut self, server: &str, n: u8, label: &str) -> String {
        let mut machine = PairOpener::new(server, device(n), label, [n; 32]).unwrap();
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
            .find_map(|event| match event {
                PairEvent::Code { code, expires_in_s } => {
                    assert_eq!(u64::from(*expires_in_s) * 1000, PAIR_SLOT_LIFETIME_MS);
                    Some(code.clone())
                }
                _ => None,
            })
            .expect("a code")
    }

    /// Device `n` types `code` on `server`; returns its index.
    fn join(&mut self, server: &str, code: &str, keys: IdentityKeys, label: &str) -> usize {
        let salt = u8::try_from(self.joiners.len()).unwrap();
        let mut machine = PairJoiner::new(server, code, keys, label, [0x40 ^ salt; 32]).unwrap();
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

    fn joiner_events(&self, index: usize) -> &[PairEvent] {
        &self.joiners[index].events
    }

    fn opener_do(&mut self, act: impl FnOnce(&mut PairOpener) -> Vec<PairOutput>) {
        let out = act(&mut self.opener.as_mut().unwrap().machine);
        self.handle(Who::Opener, out);
        self.settle();
    }

    fn joiner_do(&mut self, index: usize, act: impl FnOnce(&mut PairJoiner) -> Vec<PairOutput>) {
        let out = act(&mut self.joiners[index].machine);
        self.handle(Who::Joiner(index), out);
        self.settle();
    }

    /// The relay's clock moves to `now` and it ends what expired.
    fn advance(&mut self, ms: u64) {
        self.now = self.now.plus(ms);
        let out = self.slots.expire(self.now);
        self.route(out);
        self.settle();
    }

    fn party_conn(&self, who: Who) -> ConnectionId {
        match who {
            Who::Opener => self.opener.as_ref().unwrap().conn,
            Who::Joiner(i) => self.joiners[i].conn,
        }
    }

    fn handle(&mut self, who: Who, out: Vec<PairOutput>) {
        let conn = self.party_conn(who);
        for output in out {
            match output {
                PairOutput::Send(bytes) => {
                    if !self.is_closed(who) {
                        self.captured.push(bytes.clone());
                        self.to_relay.push_back((conn, bytes));
                    }
                }
                PairOutput::Close => {
                    if !self.is_closed(who) {
                        self.set_closed(who);
                        // What was sent before the close still reaches the relay.
                        self.settle();
                        let out = self.slots.disconnect(conn);
                        self.route(out);
                    }
                }
                PairOutput::Event(event) => match who {
                    Who::Opener => self.opener.as_mut().unwrap().events.push(event),
                    Who::Joiner(i) => self.joiners[i].events.push(event),
                },
            }
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

    fn who(&self, conn: ConnectionId) -> Option<Who> {
        if self.opener.as_ref().is_some_and(|party| party.conn == conn) {
            return Some(Who::Opener);
        }
        self.joiners
            .iter()
            .position(|party| party.conn == conn)
            .map(Who::Joiner)
    }

    fn route(&mut self, out: Vec<RelayOutput>) {
        for output in out {
            match output {
                RelayOutput::Send { connection, frame } => {
                    let Some(who) = self.who(connection) else {
                        continue;
                    };
                    if self.is_closed(who) {
                        continue;
                    }
                    let bytes = frame.encode();
                    let out = match who {
                        Who::Opener => self.opener.as_mut().unwrap().machine.receive(&bytes),
                        Who::Joiner(i) => self.joiners[i].machine.receive(&bytes),
                    };
                    self.handle(who, out);
                }
                RelayOutput::Close { connection } => {
                    let Some(who) = self.who(connection) else {
                        continue;
                    };
                    if !self.is_closed(who) {
                        self.set_closed(who);
                        let out = match who {
                            Who::Opener => self.opener.as_mut().unwrap().machine.closed(),
                            Who::Joiner(i) => self.joiners[i].machine.closed(),
                        };
                        self.handle(who, out);
                    }
                }
                RelayOutput::Store(op) => panic!("pairing stored {op:?}"),
            }
        }
    }

    fn settle(&mut self) {
        while let Some((conn, bytes)) = self.to_relay.pop_front() {
            let entropy = [u8::try_from(conn.0 & 0xff).unwrap(); 32];
            let out = self.slots.receive(conn, &bytes, self.now, entropy);
            self.route(out);
        }
    }
}

fn compare(
    events: &[PairEvent],
) -> Option<(curvyo_crypto_core::Identity, String, [&'static str; 4])> {
    events.iter().find_map(|event| match event {
        PairEvent::Compare { peer, label, words } => Some((*peer, label.clone(), *words)),
        _ => None,
    })
}

fn paired(events: &[PairEvent]) -> Option<RoomId> {
    events.iter().find_map(|event| match event {
        PairEvent::Paired { group_room, .. } => Some(RoomId(group_room.0)),
        _ => None,
    })
}

fn failed(events: &[PairEvent]) -> Option<PairFailure> {
    events.iter().find_map(|event| match event {
        PairEvent::Failed(failure) => Some(*failure),
        _ => None,
    })
}

/// A code with the same slot and another secret.
fn wrong_code(code: &str, n: u8) -> String {
    let parsed = PairCode::parse(code).unwrap();
    PairCode::new(
        parsed.slot(),
        PairSecret::from_entropy([n, 0x5a, 0x5a, 0x5a]),
    )
    .unwrap()
    .to_string()
}

/// Asserts no captured message holds a 32-byte half of a device's private entropy.
fn assert_no_private_key(messages: &[Vec<u8>], devices: &[u8]) {
    for &n in devices {
        let entropy = device(n).to_entropy();
        for secret in [&entropy[..32], &entropy[32..]] {
            for message in messages {
                assert!(
                    !message.windows(32).any(|window| window == secret),
                    "a private key byte string of device {n} travels"
                );
            }
        }
    }
}

/// Pairs device `joiner` with device `opener` (client `opener_client` in `net`, admin of the
/// group room) and connects the joiner's group session; returns the joiner's client.
fn pair_into_group(net: &mut Net, opener_client: usize, opener: u8, joiner: u8) -> (usize, World) {
    let mut world = World::new();
    let code = world.open(SERVER, opener, "Opener");
    let b = world.join(SERVER, &code, device(joiner), "Joiner");
    let (peer, label, words_a) = compare(world.opener_events()).expect("the opener compares");
    let (_, _, words_b) = compare(world.joiner_events(b)).expect("the joiner compares");
    assert_eq!(peer, device(joiner).identity());
    assert_eq!(label, "Joiner");
    assert_eq!(words_a, words_b);
    // Nothing before Pair (criterion 2).
    assert_eq!(paired(world.joiner_events(b)), None);

    net.add_member(opener_client, device(joiner).identity(), Role::Admin);
    net.run(200);
    world.opener_do(|machine| machine.pair(GROUP.document_id()).unwrap());
    assert_eq!(paired(world.opener_events()), Some(GROUP));
    assert_eq!(paired(world.joiner_events(b)), Some(GROUP));

    let client = net.add_client(joiner);
    net.connect_member(client, GROUP);
    let online = net.run_until(5_000, |n| {
        n.clients[client].saw(&SessionEvent::Online)
            && !n.clients[client].session().epoch_keys().is_empty()
    });
    assert!(online.is_some(), "the paired device reads the group room");
    (client, world)
}

/// Criteria 1 to 3 and 6: A pairs B; B, admitted by A's `AddMember`, reads the group room with
/// its own keys; B pairs C; A learns C from the group keyring. No private key byte travels.
#[test]
fn devices_pair_and_a_third_pairs_through_the_second() {
    let mut net = Net::new(96);
    let a = net.add_client(1);
    net.share(a, GROUP, Document::new(1));
    net.run(500);

    let (b, first) = pair_into_group(&mut net, a, 1, 2);
    let (_c, second) = pair_into_group(&mut net, b, 2, 3);
    net.run(500);
    let participants: Vec<_> = net.clients[a]
        .session()
        .keyring()
        .unwrap()
        .participants()
        .into_iter()
        .map(|p| (p.identity.member, p.role))
        .collect();
    for n in [1, 2, 3] {
        assert!(
            participants.contains(&(device(n).identity().member, Role::Admin)),
            "device {n} is an admin of the group"
        );
    }
    let mut messages = first.captured.clone();
    messages.extend(second.captured.iter().cloned());
    for client in &net.clients {
        messages.extend(client.sent.iter().cloned());
    }
    assert_no_private_key(&messages, &[1, 2, 3]);
}

#[test]
fn a_member_or_removed_device_is_not_added_twice() {
    let mut net = Net::new(7);
    let a = net.add_client(1);
    net.share(a, GROUP, Document::new(1));
    net.run(500);
    net.add_member(a, device(2).identity(), Role::Admin);
    let again = net.clients[a]
        .session
        .as_mut()
        .unwrap()
        .add_member(device(2).identity(), Role::Admin);
    assert_eq!(again, Err(SessionError::AlreadyMember));
}

/// The server address is normalised before it is bound: two spellings of one address pair, two
/// addresses do not (the confirmation fails as a wrong code).
#[test]
fn the_server_address_is_normalised_before_it_is_bound() {
    let mut world = World::new();
    let code = world.open("192.168.1.20:8787", 1, "Workshop");
    let b = world.join(" WS://192.168.1.20:8787/ ", &code, device(2), "Laptop");
    assert!(compare(world.joiner_events(b)).is_some());

    let mut world = World::new();
    let code = world.open(SERVER, 1, "Workshop");
    let b = world.join("ws://192.168.1.21:8787", &code, device(2), "Laptop");
    assert_eq!(failed(world.joiner_events(b)), Some(PairFailure::WrongCode));
    assert!(
        world
            .opener_events()
            .contains(&PairEvent::WrongCode { tries_left: 4 })
    );
}

/// Criterion 4: a wrong code is refused, the opener's code stays good for the right one. The
/// second joiner runs `CPace` under the fresh sid of its own join (ADR 0040 decision 3).
#[test]
fn a_wrong_code_fails_and_the_right_one_still_pairs() {
    let mut world = World::new();
    let code = world.open(SERVER, 1, "Workshop");
    let wrong = world.join(SERVER, &wrong_code(&code, 1), device(3), "Intruder");
    assert_eq!(
        failed(world.joiner_events(wrong)),
        Some(PairFailure::WrongCode)
    );
    assert_eq!(compare(world.joiner_events(wrong)), None);
    let right = world.join(SERVER, &code, device(2), "Laptop");
    assert!(compare(world.joiner_events(right)).is_some());
}

/// Criterion 4 and crypto review item 7: the opener counts the joiners whose confirmation does
/// not check out and locks its code after 5; the next joiner is refused as locked.
#[test]
fn five_wrong_codes_lock_the_code() {
    let mut world = World::new();
    let code = world.open(SERVER, 1, "Workshop");
    for n in 0..PAIR_WRONG_TRIES {
        let joiner = world.join(SERVER, &wrong_code(&code, n), device(3), "Intruder");
        let expected = if n + 1 < PAIR_WRONG_TRIES {
            PairFailure::WrongCode
        } else {
            PairFailure::Locked
        };
        assert_eq!(failed(world.joiner_events(joiner)), Some(expected));
    }
    let tries: Vec<_> = world
        .opener_events()
        .iter()
        .filter_map(|event| match event {
            PairEvent::WrongCode { tries_left } => Some(*tries_left),
            _ => None,
        })
        .collect();
    assert_eq!(tries, vec![4, 3, 2, 1]);
    assert_eq!(failed(world.opener_events()), Some(PairFailure::Locked));
    let late = world.join(SERVER, &code, device(2), "Laptop");
    assert_eq!(
        failed(world.joiner_events(late)),
        Some(PairFailure::Refused(RefusalReason::Locked))
    );
}

/// Criterion 4: an expired code and a used code are refused with their reasons; an unknown slot
/// as unknown.
#[test]
fn expired_used_and_unknown_codes_are_refused() {
    let mut world = World::new();
    let code = world.open(SERVER, 1, "Workshop");
    world.advance(PAIR_SLOT_LIFETIME_MS);
    assert_eq!(
        failed(world.opener_events()),
        Some(PairFailure::Refused(RefusalReason::Expired))
    );
    let late = world.join(SERVER, &code, device(2), "Laptop");
    assert_eq!(
        failed(world.joiner_events(late)),
        Some(PairFailure::Refused(RefusalReason::Expired))
    );

    let mut world = World::new();
    let code = world.open(SERVER, 1, "Workshop");
    let b = world.join(SERVER, &code, device(2), "Laptop");
    world.opener_do(|machine| machine.pair(GROUP.document_id()).unwrap());
    assert_eq!(paired(world.joiner_events(b)), Some(GROUP));
    let again = world.join(SERVER, &code, device(3), "Office");
    assert_eq!(
        failed(world.joiner_events(again)),
        Some(PairFailure::Refused(RefusalReason::Used))
    );

    let mut world = World::new();
    world.open(SERVER, 1, "Workshop");
    let nobody = PairCode::new(12_345, PairSecret::from_entropy([1; 4])).unwrap();
    let stray = world.join(SERVER, &nobody.to_string(), device(2), "Laptop");
    let failure = failed(world.joiner_events(stray));
    assert!(
        failure == Some(PairFailure::Refused(RefusalReason::Unknown))
            || failure == Some(PairFailure::WrongCode),
        "{failure:?}"
    );
}

/// Criterion 5: a joiner whose opener is gone does not pair on its own; the pairing needs both.
#[test]
fn the_joiner_never_pairs_without_the_opener() {
    let mut world = World::new();
    let code = world.open(SERVER, 1, "Workshop");
    let b = world.join(SERVER, &code, device(2), "Laptop");
    assert!(compare(world.joiner_events(b)).is_some());
    // The opener's app closes while the joiner waits for Pair.
    let conn = world.opener.as_ref().unwrap().conn;
    world.opener.as_mut().unwrap().closed = true;
    let out = world.slots.disconnect(conn);
    world.route(out);
    world.settle();
    assert_eq!(failed(world.joiner_events(b)), Some(PairFailure::Gone));
    assert_eq!(paired(world.joiner_events(b)), None);
}

/// Cancel on either side: the other side is told; the opener's code stays for the next joiner
/// when the joiner cancels.
#[test]
fn either_side_can_cancel() {
    let mut world = World::new();
    let code = world.open(SERVER, 1, "Workshop");
    let b = world.join(SERVER, &code, device(2), "Laptop");
    world.joiner_do(b, PairJoiner::cancel);
    assert_eq!(failed(world.joiner_events(b)), Some(PairFailure::Cancelled));
    assert!(world.opener_events().contains(&PairEvent::JoinerLeft));
    assert_eq!(failed(world.opener_events()), None);
    let c = world.join(SERVER, &code, device(3), "Office");
    assert!(compare(world.joiner_events(c)).is_some());
    world.opener_do(PairOpener::cancel);
    assert_eq!(failed(world.joiner_events(c)), Some(PairFailure::Cancelled));
    assert_eq!(failed(world.opener_events()), Some(PairFailure::Cancelled));
}

#[test]
fn calls_out_of_place_and_bad_inputs_are_refused() {
    assert!(matches!(
        PairOpener::new("http://x:1", device(1), "Workshop", [1; 32]),
        Err(PairingError::ServerAddress)
    ));
    assert!(matches!(
        PairOpener::new(SERVER, device(1), " ", [1; 32]),
        Err(PairingError::Label)
    ));
    assert!(matches!(
        PairJoiner::new(SERVER, "ABCD-EFG", device(2), "Laptop", [1; 32]),
        Err(PairingError::Crypto(_))
    ));
    let mut opener = PairOpener::new(SERVER, device(1), "Workshop", [1; 32]).unwrap();
    assert_eq!(opener.pair(GROUP.document_id()), Err(PairingError::NotNow));
    opener.start().unwrap();
    assert_eq!(opener.start(), Err(PairingError::NotNow));
}

/// The check words come from the BIP-39 English list: 2,048 words.
#[test]
fn check_words_come_from_the_word_list() {
    assert_eq!(CHECK_WORD_COUNT, 2048);
    assert_eq!(check_word(0), Some("abandon"));
    assert_eq!(check_word(2047), Some("zoo"));
    let mut world = World::new();
    let code = world.open(SERVER, 1, "Workshop");
    let b = world.join(SERVER, &code, device(2), "Laptop");
    let (_, _, words) = compare(world.joiner_events(b)).unwrap();
    assert!(words.iter().all(|word| !word.is_empty()));
}
