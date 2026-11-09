//! Tester's acceptance tests for `specs/0095-workspace` in `curvyo-sync-core`, written from the
//! specification and the public API before reading the implementation: the invite version `2.`
//! (criteria 26, 27, 29), the workspace snapshot stream and its parts (criteria 28, 34), the
//! inner record header (criterion 34; `Moved` retired by ADR 0044), the snapshot rule (ADR 0037 decision
//! 7), and removal with key rotation through the in-process relay (criteria 43 to 46), with two
//! workspaces on one relay (criterion 47).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod support;

use std::collections::VecDeque;

use curvyo_crypto_core::{EnvelopeHeader, EnvelopeKind, open};
use curvyo_sync_core::{
    ClientFrame, ClientOutput, ClientSession, ConnectionId, InviteKind, InviteLink, Millis,
    PART_BYTES, PART_PADDING, RecordKind, RefusalReason, Relay, RelayOutput, RoomId,
    SNAPSHOT_STREAM_VERSION, SessionConfig, SessionEvent, SessionStart, SnapshotAssembler,
    StreamEntry, StreamError, decode_inner_record, encode_inner_record, encode_snapshot_stream,
    snapshot_due, split_snapshot,
};
use support::{device, member, seed};

const WS: RoomId = RoomId([0x57; 16]);
const WS2: RoomId = RoomId([0x58; 16]);
const START: Millis = Millis(1_800_000_000_000);

// ---------------------------------------------------------------------------------------------
// Invite links (criteria 26, 27, 29)
// ---------------------------------------------------------------------------------------------

/// Criterion 26: a workspace link is `curvyo://join?server=<url>#2.<base64url(id ‖ seed)>`; the
/// secret is only after `#`; version `1.` keeps its meaning; other versions are refused.
#[test]
fn c26_workspace_invite_is_version_2_with_the_secret_after_the_hash() {
    let secret = curvyo_sync_core::InviteSecret {
        room: WS,
        seed: [0xab; 32],
    };
    let link = InviteLink {
        server: "ws://192.168.1.20:8787".to_owned(),
        kind: InviteKind::Workspace,
        secret: secret.clone(),
    };
    let text = link.to_text();
    let (before, after) = text.split_once('#').unwrap();
    assert!(before.starts_with("curvyo://join?server="), "{text}");
    assert!(after.starts_with("2."), "{text}");
    assert!(
        !before.contains(&after[2..]),
        "no secret before #: {before}"
    );
    // 48 bytes: 16 id + 32 seed, base64url without padding.
    assert_eq!(after.len() - 2, 64, "{after}");
    assert!(
        after[2..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    );
    let parsed = InviteLink::parse(&text).unwrap();
    assert_eq!(parsed.kind, InviteKind::Workspace);
    assert_eq!(parsed.secret.room, WS);
    assert_eq!(parsed.secret.seed, [0xab; 32]);
    assert_eq!(
        InviteLink::parse(&text.replace("#2.", "#1.")).unwrap().kind,
        InviteKind::Document
    );
    for bad in ["#3.", "#0.", "#2", "#22."] {
        assert!(
            InviteLink::parse(&text.replace("#2.", bad)).is_err(),
            "{bad}"
        );
    }
    // Truncated or overlong payloads.
    assert!(InviteLink::parse(&text[..text.len() - 1]).is_err());
    assert!(InviteLink::parse(&format!("{text}AA")).is_err());
    // Spaces around a pasted link are ignored.
    assert_eq!(
        InviteLink::parse(&format!("  {text}\n")).unwrap().kind,
        InviteKind::Workspace
    );
}

// ---------------------------------------------------------------------------------------------
// Snapshot stream (criteria 28, 34)
// ---------------------------------------------------------------------------------------------

fn entries(sizes: &[usize]) -> Vec<StreamEntry> {
    sizes
        .iter()
        .enumerate()
        .map(|(i, &n)| StreamEntry {
            stream: [u8::try_from(i).unwrap(); 16],
            bytes: (0..n)
                .map(|j| u8::try_from((i * 7 + j) % 251).unwrap())
                .collect(),
        })
        .collect()
}

/// Criterion 28: the stream round-trips through parts; an entry is handed out with the part that
/// completes it, so a joiner writes each document as soon as its bytes are there.
#[test]
fn c28_parts_round_trip_and_entries_come_as_soon_as_complete() {
    let sizes = [100, PART_BYTES - 200, 10, PART_BYTES * 2 + 5, 0, 1];
    let input = entries(&sizes);
    let stream = encode_snapshot_stream(&input);
    assert_eq!(stream[0], SNAPSHOT_STREAM_VERSION);
    let parts = split_snapshot(&stream, [1; 16]);
    for part in &parts[..parts.len() - 1] {
        assert!(part.len() >= PART_BYTES, "{}", part.len());
    }
    assert_eq!(parts.last().unwrap().len() % PART_PADDING, 0);
    let mut assembler = SnapshotAssembler::default();
    let mut out = Vec::new();
    let mut first_after_part = Vec::new();
    for part in &parts {
        let got = assembler.push_part(part).unwrap();
        first_after_part.push(got.len());
        out.extend(got);
    }
    assert!(assembler.is_complete());
    assert_eq!(out, input);
    assert!(
        first_after_part[0] >= 1,
        "the small first entries come with part 0"
    );
    // A part after the last is refused.
    assert!(assembler.push_part(&parts[0]).is_err());
}

/// Criterion 28: parts out of order, of another snapshot, truncated or of a newer version are
/// refused.
#[test]
fn c28_bad_parts_are_refused() {
    let stream = encode_snapshot_stream(&entries(&[PART_BYTES + 10, 20]));
    let parts = split_snapshot(&stream, [1; 16]);
    assert_eq!(parts.len(), 2);
    let mut a = SnapshotAssembler::default();
    assert!(matches!(
        a.push_part(&parts[1]),
        Err(StreamError::OutOfOrder)
    ));
    let other = split_snapshot(&stream, [2; 16]);
    let mut a = SnapshotAssembler::default();
    a.push_part(&parts[0]).unwrap();
    assert!(a.push_part(&other[1]).is_err(), "part of another snapshot");
    let mut a = SnapshotAssembler::default();
    assert!(a.push_part(&parts[0][..10]).is_err());
    let mut a = SnapshotAssembler::default();
    assert!(a.push_part(&[]).is_err());
    let mut newer = stream.clone();
    newer[0] = SNAPSHOT_STREAM_VERSION + 1;
    let mut a = SnapshotAssembler::default();
    assert!(matches!(
        a.push_part(&split_snapshot(&newer, [3; 16])[0]),
        Err(StreamError::FormatTooNew)
    ));
    // An entry that claims more bytes than the stream holds.
    let mut lying = encode_snapshot_stream(&entries(&[10]));
    let len_at = 1 + 4 + 16;
    lying[len_at..len_at + 8].copy_from_slice(&u64::MAX.to_le_bytes());
    let mut a = SnapshotAssembler::default();
    let result = a.push_part(&split_snapshot(&lying, [4; 16])[0]);
    assert!(
        result.is_err() || !a.is_complete() || result.unwrap().is_empty(),
        "a lying length is not handed out"
    );
}

/// Criterion 34: a workspace of 3 documents and one of 30 of the same total size give parts of
/// the same number and the same sizes: the count cannot be read from them.
#[test]
fn c34_three_or_thirty_documents_give_the_same_parts() {
    let total = 3 * 1024 * 1024 + 12_345;
    let list = 2_000;
    let mut three = vec![list];
    three.extend([total / 3; 3]);
    let mut thirty = vec![list];
    thirty.extend([total / 30; 30]);
    let a = split_snapshot(&encode_snapshot_stream(&entries(&three)), [1; 16]);
    let b = split_snapshot(&encode_snapshot_stream(&entries(&thirty)), [2; 16]);
    assert_eq!(a.len(), b.len());
    assert_eq!(
        a.iter().map(Vec::len).collect::<Vec<_>>(),
        b.iter().map(Vec::len).collect::<Vec<_>>()
    );
    // Over several parts too.
    let big = 9 * 1024 * 1024;
    let mut three = vec![list];
    three.extend([big / 3; 3]);
    let mut thirty = vec![list];
    thirty.extend([big / 30; 30]);
    let a = split_snapshot(&encode_snapshot_stream(&entries(&three)), [1; 16]);
    let b = split_snapshot(&encode_snapshot_stream(&entries(&thirty)), [2; 16]);
    assert_eq!(
        a.iter().map(Vec::len).collect::<Vec<_>>(),
        b.iter().map(Vec::len).collect::<Vec<_>>()
    );
}

/// Criteria 30 and 34: the inner header round-trips; short bytes are refused.
#[test]
fn c34_inner_records_round_trip() {
    let record = encode_inner_record([7; 16], u64::MAX, b"payload");
    let (header, payload) = decode_inner_record(&record).unwrap();
    assert_eq!(
        (header.stream, header.peer, payload),
        ([7; 16], u64::MAX, &b"payload"[..])
    );
    let empty = encode_inner_record([7; 16], 0, b"");
    assert_eq!(decode_inner_record(&empty).unwrap().1, b"");
    assert!(decode_inner_record(&record[..23]).is_err());
    assert!(decode_inner_record(&[]).is_err());
}

/// ADR 0037 decision 7: a snapshot is due at max(4 MiB, half the last snapshot) of tail bytes or
/// max(500, last / 8 KiB) tail records.
#[test]
fn c32_snapshot_due_thresholds() {
    const MIB: u64 = 1024 * 1024;
    assert!(!snapshot_due(0, 0, 0));
    assert!(!snapshot_due(4 * MIB - 1, 499, 0));
    assert!(snapshot_due(4 * MIB, 0, 0));
    assert!(snapshot_due(0, 500, 0));
    let last = 100 * MIB;
    assert!(!snapshot_due(50 * MIB - 1, 12_799, last));
    assert!(snapshot_due(50 * MIB, 0, last));
    assert!(snapshot_due(0, 12_800, last));
    // Overflow-safe at the extremes.
    assert!(snapshot_due(u64::MAX, u64::MAX, u64::MAX));
    let _ = snapshot_due(0, 0, u64::MAX);
}

// ---------------------------------------------------------------------------------------------
// Relay: share, join, removal (criteria 25 to 29, 43 to 47)
// ---------------------------------------------------------------------------------------------

struct Dev {
    session: ClientSession,
    conn: Option<ConnectionId>,
    events: Vec<SessionEvent>,
    sent: Vec<Vec<u8>>,
    online: bool,
    /// While set, frames this device sends wait here instead of reaching the relay.
    hold: Option<Vec<Vec<u8>>>,
    /// The snapshot stream this device uploads when a snapshot is due.
    stream: Vec<u8>,
}

struct Net {
    relay: Relay,
    now: Millis,
    devs: Vec<Dev>,
    nonce: u8,
}

fn config(n: u8, room: RoomId, start: SessionStart) -> SessionConfig {
    SessionConfig {
        room,
        identity: device(n),
        peer_id: 0,
        nonce_seed: seed(n, 1),
        entropy_seed: seed(n, 2),
        since_seq: 0,
        start,
    }
}

impl Net {
    fn new() -> Self {
        Self {
            relay: Relay::new(),
            now: START,
            devs: Vec::new(),
            nonce: 0,
        }
    }

    fn add(&mut self, n: u8, room: RoomId, start: SessionStart, stream: Vec<u8>) -> usize {
        let mut session = ClientSession::new(config(n, room, start), self.now).unwrap();
        session.use_workspace_snapshots();
        self.devs.push(Dev {
            session,
            conn: None,
            events: Vec::new(),
            sent: Vec::new(),
            online: true,
            hold: None,
            stream,
        });
        self.devs.len() - 1
    }

    fn pump(&mut self) {
        let mut queue: VecDeque<(usize, Option<Vec<u8>>)> = VecDeque::new();
        loop {
            let mut moved = false;
            for i in 0..self.devs.len() {
                for output in self.devs[i].session.drain() {
                    moved = true;
                    match output {
                        ClientOutput::Connect => {
                            if self.devs[i].online {
                                let conn = self.relay.connect();
                                self.devs[i].conn = Some(conn);
                                self.devs[i].session.connected(self.now);
                            } else {
                                self.devs[i].session.disconnected(self.now);
                            }
                        }
                        ClientOutput::Send(bytes) => {
                            self.devs[i].sent.push(bytes.clone());
                            if let Some(held) = self.devs[i].hold.as_mut() {
                                held.push(bytes);
                                continue;
                            }
                            let Some(conn) = self.devs[i].conn else {
                                continue;
                            };
                            self.nonce = self.nonce.wrapping_add(1);
                            let out = self.relay.receive(conn, &bytes, self.now, [self.nonce; 32]);
                            self.route(out, &mut queue);
                        }
                        ClientOutput::Close => {
                            if let Some(conn) = self.devs[i].conn.take() {
                                let out = self.relay.disconnect(conn);
                                self.route(out, &mut queue);
                            }
                        }
                        ClientOutput::Event(event) => {
                            if let SessionEvent::SnapshotDue { covers_seq } = &event {
                                let parts = split_snapshot(
                                    &self.devs[i].stream,
                                    [u8::try_from(i).unwrap(); 16],
                                );
                                self.devs[i]
                                    .session
                                    .upload_snapshot_parts(&parts, *covers_seq)
                                    .unwrap();
                            }
                            self.devs[i].events.push(event);
                        }
                    }
                }
            }
            while let Some((to, bytes)) = queue.pop_front() {
                moved = true;
                match bytes {
                    Some(bytes) => self.devs[to].session.received(&bytes, self.now),
                    None => self.devs[to].session.disconnected(self.now),
                }
            }
            if !moved {
                return;
            }
        }
    }

    fn route(&mut self, out: Vec<RelayOutput>, queue: &mut VecDeque<(usize, Option<Vec<u8>>)>) {
        for o in out {
            match o {
                RelayOutput::Send { connection, frame } => {
                    if let Some(to) = self.devs.iter().position(|d| d.conn == Some(connection)) {
                        queue.push_back((to, Some(frame.encode())));
                    }
                }
                RelayOutput::Close { connection } => {
                    if let Some(to) = self.devs.iter().position(|d| d.conn == Some(connection)) {
                        self.devs[to].conn = None;
                        queue.push_back((to, None));
                    }
                }
                RelayOutput::Store(_) => {}
            }
        }
    }

    /// Hands one held frame of device `i` to the relay.
    fn deliver(&mut self, i: usize, bytes: &[u8]) {
        let Some(conn) = self.devs[i].conn else {
            return;
        };
        self.nonce = self.nonce.wrapping_add(1);
        let out = self.relay.receive(conn, bytes, self.now, [self.nonce; 32]);
        let mut queue = VecDeque::new();
        self.route(out, &mut queue);
        while let Some((to, bytes)) = queue.pop_front() {
            match bytes {
                Some(bytes) => self.devs[to].session.received(&bytes, self.now),
                None => self.devs[to].session.disconnected(self.now),
            }
        }
    }

    fn set_online(&mut self, i: usize, up: bool) {
        self.devs[i].online = up;
        if !up && let Some(conn) = self.devs[i].conn.take() {
            let mut q = VecDeque::new();
            let out = self.relay.disconnect(conn);
            self.route(out, &mut q);
            self.devs[i].session.disconnected(self.now);
        }
    }

    fn run(&mut self, ms: u64) {
        let until = self.now.plus(ms);
        while self.now < until {
            self.now = self.now.plus(10);
            for d in &mut self.devs {
                d.session.tick(self.now);
            }
            self.pump();
        }
    }

    fn events(&mut self, i: usize) -> Vec<SessionEvent> {
        std::mem::take(&mut self.devs[i].events)
    }

    fn invite(&mut self, i: usize, kind: InviteKind, valid_s: u64) -> InviteLink {
        let expires = self.now.unix_seconds() + valid_s;
        let secret = self.devs[i].session.invite(expires).unwrap();
        self.pump();
        InviteLink {
            server: "ws://relay.test:8787".to_owned(),
            kind,
            secret,
        }
    }

    fn remote_updates(&mut self, i: usize) -> Vec<Vec<u8>> {
        self.events(i)
            .into_iter()
            .filter_map(|e| match e {
                SessionEvent::Remote {
                    kind: RecordKind::Update,
                    bytes,
                    ..
                } => Some(bytes),
                _ => None,
            })
            .collect()
    }
}

fn ws_stream(docs: usize) -> Vec<u8> {
    let mut sizes = vec![500];
    sizes.extend(std::iter::repeat_n(3_000, docs));
    encode_snapshot_stream(&entries(&sizes))
}

/// A shares workspace `room`; B (and C) join by `2.` links.
fn workspace_of(net: &mut Net, room: RoomId, owner: u8, joiners: &[u8]) -> Vec<usize> {
    let a = net.add(owner, room, SessionStart::Share, ws_stream(3));
    net.pump();
    net.run(200);
    let mut ids = vec![a];
    for &n in joiners {
        let link = net.invite(a, InviteKind::Workspace, 3_600);
        let parsed = InviteLink::parse(&link.to_text()).unwrap();
        let j = net.add(n, room, SessionStart::Join(parsed.secret), ws_stream(3));
        net.pump();
        net.run(500);
        assert!(
            net.devs[j].events.contains(&SessionEvent::Admitted),
            "device {n} admitted: {:?}",
            net.devs[j].events
        );
        ids.push(j);
    }
    ids
}

/// Criteria 28 and 29: a used workspace link and an expired one admit nobody, and the refused
/// device sees no record.
#[test]
fn c29_a_used_or_expired_workspace_link_is_refused_without_content() {
    let mut net = Net::new();
    let a = net.add(1, WS, SessionStart::Share, ws_stream(3));
    net.pump();
    net.run(200);
    let link = net.invite(a, InviteKind::Workspace, 3_600);
    let b = net.add(2, WS, SessionStart::Join(link.secret.clone()), Vec::new());
    net.pump();
    net.run(500);
    assert!(net.events(b).contains(&SessionEvent::Admitted));
    let c = net.add(3, WS, SessionStart::Join(link.secret.clone()), Vec::new());
    net.pump();
    net.run(500);
    let events = net.events(c);
    assert!(
        events.contains(&SessionEvent::Refused(RefusalReason::Used)),
        "{events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::Remote { .. })),
        "no content for C"
    );
    let expired = net.invite(a, InviteKind::Workspace, 1);
    net.run(3_000);
    let d = net.add(4, WS, SessionStart::Join(expired.secret), Vec::new());
    net.pump();
    net.run(500);
    let events = net.events(d);
    assert!(
        events.contains(&SessionEvent::Refused(RefusalReason::Expired)),
        "{events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::Remote { .. }))
    );
}

/// Criteria 43 and 44: A removes B; the keyring gains `RemoveMember` and `NewEpoch`; B is
/// told and closed; after the new snapshot nothing the room keeps opens with B's keys; C keeps
/// editing across the rotation and loses nothing.
#[test]
fn c43_c44_removal_rotates_the_key_and_closes_out_the_removed_device() {
    let mut net = Net::new();
    let ids = workspace_of(&mut net, WS, 1, &[2, 3]);
    let (a, b, c) = (ids[0], ids[1], ids[2]);
    for i in [a, b, c] {
        net.events(i);
    }
    let b_keys = net.devs[b].session.epoch_keys();
    let epoch_before = net.devs[a].session.keyring().unwrap().current_epoch();
    let entries_before = net.devs[a].session.keyring_entries().len();
    net.devs[a].session.remove(member(2)).unwrap();
    net.pump();
    net.run(1_000);
    let entries_after = net.devs[a].session.keyring_entries().len();
    assert_eq!(
        entries_after,
        entries_before + 2,
        "RemoveMember then NewEpoch"
    );
    assert_ne!(
        net.devs[a].session.keyring().unwrap().current_epoch(),
        epoch_before
    );
    let b_events = net.events(b);
    assert!(b_events.contains(&SessionEvent::Removed), "{b_events:?}");
    assert!(net.devs[b].conn.is_none(), "B's connection is closed");
    // C edits across the rotation; A receives it.
    net.devs[c]
        .session
        .push(
            RecordKind::Update,
            encode_inner_record([0x33; 16], 3, b"C after rotation"),
        )
        .unwrap();
    net.pump();
    net.run(200);
    let got: Vec<Vec<u8>> = net.remote_updates(a);
    assert!(
        got.iter()
            .any(|r| decode_inner_record(r).unwrap().1 == b"C after rotation"),
        "A got C's edit"
    );
    // B reconnects: refused, no content.
    net.devs[b].session.network_back(net.now);
    net.pump();
    net.run(2_000);
    let b_events = net.events(b);
    assert!(
        !b_events
            .iter()
            .any(|e| matches!(e, SessionEvent::Remote { .. })),
        "{b_events:?}"
    );
    assert!(
        !net.relay.room(&WS).unwrap().online().contains(&member(2)),
        "B not online in the room"
    );
    // A writes the fresh snapshot of the removal (ADR 0031 decision 6a): either the session
    // asked for it, or the host does it now.
    let covers = net.devs[a].session.cursor();
    let asked = net.devs[a]
        .sent
        .iter()
        .any(|s| matches!(ClientFrame::decode(s), Ok(ClientFrame::SnapshotPart { .. })));
    if !asked {
        let parts = split_snapshot(&ws_stream(3), [0xee; 16]);
        net.devs[a]
            .session
            .upload_snapshot_parts(&parts, covers)
            .unwrap();
        net.pump();
        net.run(200);
    }
    let stored = net.relay.room(&WS).unwrap().stored();
    let mut envelopes: Vec<(Vec<u8>, EnvelopeKind)> = Vec::new();
    if let Some((_, first)) = &stored.snapshot {
        envelopes.push((first.clone(), EnvelopeKind::Snapshot));
    }
    for part in &stored.snapshot_rest {
        envelopes.push((part.clone(), EnvelopeKind::Snapshot));
    }
    for record in &stored.records {
        let kind = match record.kind {
            RecordKind::Asset => EnvelopeKind::Asset,
            RecordKind::Snapshot => EnvelopeKind::Snapshot,
            RecordKind::Update => EnvelopeKind::Update,
        };
        envelopes.push((record.envelope.clone(), kind));
    }
    assert!(!envelopes.is_empty());
    for (envelope, kind) in &envelopes {
        let header = EnvelopeHeader::parse(envelope).unwrap();
        for (epoch, key) in &b_keys {
            assert!(
                open(envelope, key, WS.document_id(), *kind).is_err(),
                "an envelope of epoch {} opens with B's key of epoch {epoch}",
                header.epoch
            );
        }
    }
}

/// Criterion 45: B is offline during its removal, edits offline and reconnects: it learns of the
/// removal and its offline edits are not stored in the room.
#[test]
fn c45_an_offline_removed_device_cannot_push_its_offline_edits() {
    let mut net = Net::new();
    let ids = workspace_of(&mut net, WS, 1, &[2]);
    let (a, b) = (ids[0], ids[1]);
    net.set_online(b, false);
    net.run(100);
    net.devs[a].session.remove(member(2)).unwrap();
    net.pump();
    net.run(500);
    let head_before = net.relay.room(&WS).unwrap().head_seq();
    net.devs[b]
        .session
        .push(
            RecordKind::Update,
            encode_inner_record([0x22; 16], 2, b"offline rename by B"),
        )
        .unwrap();
    net.events(b);
    net.devs[b].online = true;
    net.devs[b].session.network_back(net.now);
    net.pump();
    net.run(15_000);
    let events = net.events(b);
    assert!(
        events.contains(&SessionEvent::Removed)
            || events.contains(&SessionEvent::Refused(RefusalReason::Removed))
            || events.contains(&SessionEvent::Refused(RefusalReason::NotAMember)),
        "B learns of its removal: {events:?}"
    );
    let room = net.relay.room(&WS).unwrap();
    assert!(
        room.head_seq() == head_before
            || room
                .records()
                .iter()
                .all(|r| EnvelopeHeader::parse(&r.envelope).unwrap().epoch
                    > net.devs[b]
                        .session
                        .epoch_keys()
                        .iter()
                        .map(|k| k.0)
                        .max()
                        .unwrap_or(0)),
        "B's offline edit was stored"
    );
    assert!(net.remote_updates(a).is_empty(), "A got nothing from B");
}

/// Criterion 46: an invite after a removal admits the new member to the whole workspace under
/// the new epoch.
#[test]
fn c46_an_invite_after_a_removal_reaches_the_whole_workspace() {
    let mut net = Net::new();
    let ids = workspace_of(&mut net, WS, 1, &[2]);
    let (a, b) = (ids[0], ids[1]);
    net.devs[a].session.remove(member(2)).unwrap();
    net.pump();
    net.run(500);
    net.events(b);
    let link = net.invite(a, InviteKind::Workspace, 3_600);
    let c = net.add(3, WS, SessionStart::Join(link.secret), Vec::new());
    net.pump();
    net.run(1_000);
    assert!(net.events(c).contains(&SessionEvent::Admitted));
    net.devs[a]
        .session
        .push(
            RecordKind::Update,
            encode_inner_record([0x11; 16], 1, b"for C"),
        )
        .unwrap();
    net.pump();
    net.run(200);
    assert!(
        net.remote_updates(c)
            .iter()
            .any(|r| decode_inner_record(r).unwrap().1 == b"for C")
    );
    assert!(net.remote_updates(b).is_empty(), "removed B gets nothing");
}

/// Criterion 47: two workspaces of A on one relay: own invites, own participants, no cross-talk.
#[test]
fn c47_two_workspaces_on_one_relay_do_not_talk_to_each_other() {
    let mut net = Net::new();
    let one = workspace_of(&mut net, WS, 1, &[2]);
    let two = workspace_of(&mut net, WS2, 1, &[3]);
    for i in one.iter().chain(&two) {
        net.events(*i);
    }
    net.devs[one[0]]
        .session
        .push(
            RecordKind::Update,
            encode_inner_record([1; 16], 1, b"in one"),
        )
        .unwrap();
    net.pump();
    net.run(200);
    assert_eq!(net.remote_updates(one[1]).len(), 1);
    assert!(net.remote_updates(two[1]).is_empty());
    assert!(net.remote_updates(two[0]).is_empty());
    let p1 = net.relay.room(&WS).unwrap().keyring().participants().len();
    let p2 = net.relay.room(&WS2).unwrap().keyring().participants().len();
    assert_eq!((p1, p2), (2, 2));
    // A link of workspace one does not open workspace two.
    let link = net.invite(one[0], InviteKind::Workspace, 3_600);
    let mut forged = link.secret.clone();
    forged.room = WS2;
    let x = net.add(9, WS2, SessionStart::Join(forged), Vec::new());
    net.pump();
    net.run(1_000);
    let events = net.events(x);
    assert!(!events.contains(&SessionEvent::Admitted), "{events:?}");
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, SessionEvent::Remote { .. }))
    );
}

/// Criterion 34 at the relay: every envelope of a workspace room names the workspace and peer 0
/// in the clear, whoever wrote it, for updates and snapshot parts.
#[test]
fn c34_the_relay_sees_no_peer_or_document_in_the_clear() {
    let mut net = Net::new();
    let ids = workspace_of(&mut net, WS, 1, &[2]);
    for (k, &i) in ids.iter().enumerate() {
        net.devs[i]
            .session
            .push(
                RecordKind::Update,
                encode_inner_record([u8::try_from(k).unwrap() + 0x40; 16], 77, b"x"),
            )
            .unwrap();
    }
    net.pump();
    net.run(300);
    let stored = net.relay.room(&WS).unwrap().stored();
    let all = stored
        .records
        .iter()
        .map(|r| r.envelope.clone())
        .chain(stored.snapshot.iter().map(|s| s.1.clone()))
        .chain(stored.snapshot_rest.iter().cloned());
    for envelope in all {
        let header = EnvelopeHeader::parse(&envelope).unwrap();
        assert_eq!(header.sender_peer, 0);
        assert_eq!(header.document_id, WS.document_id());
    }
}

/// ADR 0037 decision 7 with two members: in a busy workspace both members reach the snapshot
/// threshold at once and upload their multi-part snapshots at the same time, so their parts
/// interleave at the relay. Neither member may be refused for it, and the room must still hold
/// one complete snapshot a joiner can read.
#[test]
fn c31_c32_concurrent_multi_part_snapshots_of_two_members() {
    let mut net = Net::new();
    let big = {
        let mut sizes = vec![500];
        sizes.extend([PART_BYTES / 2; 3]);
        encode_snapshot_stream(&entries(&sizes))
    };
    let a = net.add(1, WS, SessionStart::Share, big.clone());
    net.pump();
    net.run(200);
    let link = net.invite(a, InviteKind::Workspace, 3_600);
    let b = net.add(2, WS, SessionStart::Join(link.secret), big.clone());
    net.pump();
    net.run(500);
    assert!(net.events(b).contains(&SessionEvent::Admitted));
    net.events(a);
    let mut n = 0_u32;
    let mut parts_a: Vec<Vec<u8>> = Vec::new();
    let mut parts_b: Vec<Vec<u8>> = Vec::new();
    // Both count every record; hold their frames just before the threshold.
    loop {
        let due = net.devs[..2].iter().all(|d| {
            d.events
                .iter()
                .any(|e| matches!(e, SessionEvent::SnapshotDue { .. }))
        });
        if due || n > 2_000 {
            break;
        }
        if n == 700 {
            net.devs[a].hold = Some(Vec::new());
            net.devs[b].hold = Some(Vec::new());
        }
        let who = if n.is_multiple_of(2) { a } else { b };
        net.devs[who]
            .session
            .push(
                RecordKind::Update,
                encode_inner_record([3; 16], 0, &[1; 40]),
            )
            .unwrap();
        net.pump();
        n += 1;
        if net.devs[a].hold.is_some() {
            // Deliver pushes as they come; snapshot parts wait until both members uploaded.
            let is_part = |f: &Vec<u8>| {
                matches!(ClientFrame::decode(f), Ok(ClientFrame::SnapshotPart { .. }))
            };
            let (pa, ha): (Vec<_>, Vec<_>) = std::mem::take(net.devs[a].hold.as_mut().unwrap())
                .into_iter()
                .partition(is_part);
            let (pb, hb): (Vec<_>, Vec<_>) = std::mem::take(net.devs[b].hold.as_mut().unwrap())
                .into_iter()
                .partition(is_part);
            parts_a.extend(pa);
            parts_b.extend(pb);
            let mut ia = ha.into_iter();
            let mut ib = hb.into_iter();
            loop {
                let fa = ia.next();
                let fb = ib.next();
                if fa.is_none() && fb.is_none() {
                    break;
                }
                if let Some(f) = fa {
                    net.deliver(a, &f);
                }
                if let Some(f) = fb {
                    net.deliver(b, &f);
                }
            }
            net.pump();
        }
    }
    // The two uploads reach the relay interleaved, part by part.
    net.devs[a].hold.as_mut().unwrap().splice(0..0, parts_a);
    net.devs[b].hold.as_mut().unwrap().splice(0..0, parts_b);
    // Keep interleaving whatever is still held, then let go.
    for _ in 0..50 {
        let ha = std::mem::take(net.devs[a].hold.as_mut().unwrap());
        let hb = std::mem::take(net.devs[b].hold.as_mut().unwrap());
        let mut ia = ha.into_iter();
        let mut ib = hb.into_iter();
        loop {
            let fa = ia.next();
            let fb = ib.next();
            if fa.is_none() && fb.is_none() {
                break;
            }
            if let Some(f) = fa {
                net.deliver(a, &f);
            }
            if let Some(f) = fb {
                net.deliver(b, &f);
            }
        }
        net.pump();
        net.now = net.now.plus(10);
        for d in &mut net.devs {
            d.session.tick(net.now);
        }
    }
    net.devs[a].hold = None;
    net.devs[b].hold = None;
    net.run(2_000);
    let parts_sent = |d: &Dev| {
        d.sent
            .iter()
            .filter(|s| matches!(ClientFrame::decode(s), Ok(ClientFrame::SnapshotPart { .. })))
            .count()
    };
    let (pa, pb) = (parts_sent(&net.devs[a]), parts_sent(&net.devs[b]));
    assert_eq!((pa, pb), (4, 2), "A shared, then both uploaded at once");
    for i in [a, b] {
        let events = net.events(i);
        assert!(
            !events.iter().any(|e| matches!(e, SessionEvent::Refused(_))),
            "device {i} refused after concurrent snapshot uploads (parts sent {pa}/{pb}): {:?}",
            events
                .iter()
                .filter(|e| !matches!(e, SessionEvent::Acked { .. } | SessionEvent::Remote { .. }))
                .collect::<Vec<_>>()
        );
    }
    // A joiner reads one complete snapshot.
    let link = net.invite(a, InviteKind::Workspace, 3_600);
    let c = net.add(3, WS, SessionStart::Join(link.secret), Vec::new());
    net.pump();
    net.run(1_000);
    let mut assembler = SnapshotAssembler::default();
    let mut streams = Vec::new();
    for e in net.events(c) {
        if let SessionEvent::Remote {
            kind: RecordKind::Snapshot,
            bytes,
            ..
        } = e
        {
            streams.extend(assembler.push_part(&bytes).unwrap());
        }
    }
    assert!(assembler.is_complete(), "parts sent {pa}/{pb}");
    assert_eq!(streams.len(), 4);
}
