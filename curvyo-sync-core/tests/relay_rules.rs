//! The relay's room rules at the frame level (ADR 0031 decisions 5, 6a, 10; `specs/0092`
//! criteria 4, 25, 27, 33, 38, 40): admission, keyring before records, gapless sequence,
//! envelope checks, snapshot replace and prune, removal, size limit, restore.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use curvyo_crypto_core::keyring::{Entry, Payload, RecipientWrap, Role};
use curvyo_crypto_core::{
    EnvelopeKind, EpochKey, IdentityKeys, InviteKey, Sealing, seal, wrap_epoch_key,
};
use curvyo_sync_core::{
    ClientFrame, ConnectionId, MAX_MESSAGE_LEN, Millis, PROTOCOL_VERSION, RecordKind,
    RefusalReason, Relay, RelayFrame, RelayOutput, RestoreError, RoomId, StoreOp, StoredRoom,
};
use support::{ROOM, device};

const NOW: Millis = Millis(1_800_000_000_000);

fn key() -> EpochKey {
    EpochKey::from_bytes([0x41; 32])
}

fn genesis(admin: &IdentityKeys, room: RoomId) -> Entry {
    let me = admin.identity();
    Entry::sign(
        Vec::new(),
        admin,
        Payload::Genesis {
            document_id: room.document_id(),
            admins: vec![me],
            epoch0_wraps: vec![RecipientWrap {
                member: me.member,
                wrap: wrap_epoch_key(&key(), 0, room.document_id(), &me, [7; 32]).unwrap(),
            }],
        },
    )
    .unwrap()
}

fn envelope(kind: EnvelopeKind, room: RoomId, signer: Option<&IdentityKeys>, n: u8) -> Vec<u8> {
    let key = key();
    seal(
        &Sealing {
            kind,
            epoch: 0,
            key: &key,
            document_id: room.document_id(),
            sender_peer: 1,
            signer,
        },
        [n; 24],
        b"sealed bytes",
    )
    .unwrap()
}

fn sends(out: &[RelayOutput], to: ConnectionId) -> Vec<RelayFrame> {
    out.iter()
        .filter_map(|o| match o {
            RelayOutput::Send { connection, frame } if *connection == to => Some(frame.clone()),
            _ => None,
        })
        .collect()
}

fn stores(out: &[RelayOutput]) -> usize {
    out.iter()
        .filter(|o| matches!(o, RelayOutput::Store(_)))
        .count()
}

fn refused(out: &[RelayOutput], to: ConnectionId) -> Option<RefusalReason> {
    sends(out, to).iter().find_map(|f| match f {
        RelayFrame::Refused { reason } => Some(*reason),
        _ => None,
    })
}

fn send(relay: &mut Relay, conn: ConnectionId, frame: &ClientFrame) -> Vec<RelayOutput> {
    relay.receive(conn, &frame.encode(), NOW, [9; 32])
}

/// Hello and proof for `who`; returns the outputs of the proof.
fn handshake(
    relay: &mut Relay,
    who: &IdentityKeys,
    room: RoomId,
    since: u64,
) -> (ConnectionId, Vec<RelayOutput>) {
    let conn = relay.connect();
    let out = send(
        relay,
        conn,
        &ClientFrame::Hello {
            protocol: PROTOCOL_VERSION,
            room,
            since_seq: since,
            identity: who.identity().member,
        },
    );
    let Some(RelayFrame::Challenge { nonce }) = sends(&out, conn).first().cloned() else {
        panic!("a challenge");
    };
    let mut message = b"curvyo-relay-v1".to_vec();
    message.extend_from_slice(&room.0);
    message.extend_from_slice(&nonce);
    let out = send(
        relay,
        conn,
        &ClientFrame::Proof {
            signature: who.sign(&message),
        },
    );
    (conn, out)
}

/// A relay holding `ROOM` created by device 1, connected as `conn`.
fn room_with_admin() -> (Relay, ConnectionId, Entry) {
    let mut relay = Relay::new();
    let admin = device(1);
    let (conn, out) = handshake(&mut relay, &admin, ROOM, 0);
    assert!(
        sends(&out, conn).is_empty(),
        "a stranger waits for its genesis"
    );
    let g = genesis(&admin, ROOM);
    let out = send(
        &mut relay,
        conn,
        &ClientFrame::Keyring {
            entry: g.encode().unwrap(),
        },
    );
    assert!(matches!(
        sends(&out, conn)[0],
        RelayFrame::Welcome { head_seq: 0, .. }
    ));
    assert_eq!(stores(&out), 1);
    (relay, conn, g)
}

#[test]
fn a_genesis_creates_the_room_and_a_member_is_admitted_on_its_proof() {
    let (mut relay, conn, g) = room_with_admin();
    for n in 1..=3 {
        let out = send(
            &mut relay,
            conn,
            &ClientFrame::Push {
                client_id: n,
                kind: RecordKind::Update,
                envelope: envelope(EnvelopeKind::Update, ROOM, None, u8::try_from(n).unwrap()),
            },
        );
        assert_eq!(
            sends(&out, conn),
            vec![RelayFrame::Ack {
                client_id: n,
                seq: n
            }]
        );
    }
    // The same identity again, with cursor 1: welcome, keyring, records 2 and 3, the peer.
    let (again, out) = handshake(&mut relay, &device(1), ROOM, 1);
    let frames = sends(&out, again);
    assert!(matches!(
        frames[0],
        RelayFrame::Welcome {
            head_seq: 3,
            snapshot_seq: 0,
            ..
        }
    ));
    assert_eq!(
        frames[1],
        RelayFrame::Keyring {
            entry: g.encode().unwrap()
        }
    );
    assert!(matches!(frames[2], RelayFrame::Record { seq: 2, .. }));
    assert!(matches!(frames[3], RelayFrame::Record { seq: 3, .. }));
    assert_eq!(
        frames[4],
        RelayFrame::Peer {
            connection: conn,
            identity: Some(device(1).identity().member)
        }
    );
    assert_eq!(
        sends(&out, conn),
        vec![RelayFrame::Peer {
            connection: again,
            identity: Some(device(1).identity().member)
        }]
    );
    let room = relay.room(&ROOM).unwrap();
    assert_eq!(
        room.records().iter().map(|r| r.seq).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    // Leaving is reported to the others.
    let out = relay.disconnect(again);
    assert_eq!(
        sends(&out, conn),
        vec![RelayFrame::Peer {
            connection: again,
            identity: None
        }]
    );
}

#[test]
fn handshake_errors_are_refused() {
    let (mut relay, _, _) = room_with_admin();
    let conn = relay.connect();
    let out = send(&mut relay, conn, &ClientFrame::Ping);
    assert_eq!(refused(&out, conn), Some(RefusalReason::Protocol));
    let conn = relay.connect();
    let out = send(
        &mut relay,
        conn,
        &ClientFrame::Hello {
            protocol: PROTOCOL_VERSION + 1,
            room: ROOM,
            since_seq: 0,
            identity: device(1).identity().member,
        },
    );
    assert_eq!(refused(&out, conn), Some(RefusalReason::Protocol));
    // A proof by another key than the one named.
    let conn = relay.connect();
    send(
        &mut relay,
        conn,
        &ClientFrame::Hello {
            protocol: PROTOCOL_VERSION,
            room: ROOM,
            since_seq: 0,
            identity: device(1).identity().member,
        },
    );
    let out = send(
        &mut relay,
        conn,
        &ClientFrame::Proof {
            signature: device(2).sign(b"x"),
        },
    );
    assert_eq!(refused(&out, conn), Some(RefusalReason::NotAMember));
    assert!(out.contains(&RelayOutput::Close { connection: conn }));
}

/// Criterion 27: an identity that is not a member gets nothing but a refusal, whatever it sends.
#[test]
fn a_stranger_gets_nothing_but_a_refusal() {
    let (mut relay, _, g) = room_with_admin();
    let stranger = device(9);
    let (conn, out) = handshake(&mut relay, &stranger, ROOM, 0);
    assert_eq!(sends(&out, conn), [] as [curvyo_sync_core::RelayFrame; 0]);
    let out = send(
        &mut relay,
        conn,
        &ClientFrame::Push {
            client_id: 1,
            kind: RecordKind::Update,
            envelope: envelope(EnvelopeKind::Update, ROOM, None, 1),
        },
    );
    assert_eq!(
        sends(&out, conn),
        vec![RelayFrame::Refused {
            reason: RefusalReason::NotAMember
        }]
    );
    // A genesis for an existing room, or a claim of an invite that does not exist.
    for entry in [genesis(&stranger, ROOM), claim(&stranger, &[&g], 5)] {
        let (conn, _) = handshake(&mut relay, &stranger, ROOM, 0);
        let out = send(
            &mut relay,
            conn,
            &ClientFrame::Keyring {
                entry: entry.encode().unwrap(),
            },
        );
        assert_eq!(refused(&out, conn), Some(RefusalReason::NotAMember));
        assert_eq!(stores(&out), 0);
    }
}

fn claim(joiner: &IdentityKeys, parents: &[&Entry], n: u8) -> Entry {
    let invite = InviteKey::from_seed(&[n; 32]).unwrap();
    Entry::sign(
        parents.iter().map(|e| e.try_hash().unwrap()).collect(),
        joiner,
        Payload::ClaimInvite {
            invite_id: invite.invite_id(),
            identity: joiner.identity(),
            proof: invite.claim_proof(ROOM.document_id(), &joiner.identity()),
        },
    )
    .unwrap()
}

/// Criteria 3 and 4: a probe with a valid invite proof receives the keyring only; the claim on
/// its heads admits; a second claim is `used`, an expired invite `expired`.
#[test]
fn invites_admit_once_and_until_they_expire() {
    let (mut relay, admin_conn, g) = room_with_admin();
    let admin = device(1);
    let invite = |n: u8, expires: u64| {
        Entry::sign(
            vec![g.try_hash().unwrap()],
            &admin,
            Payload::AddPendingInvite {
                invite_id: InviteKey::from_seed(&[n; 32]).unwrap().invite_id(),
                role: Role::Member,
                expires,
            },
        )
        .unwrap()
    };
    let open = invite(5, NOW.unix_seconds() + 60);
    let stale = invite(6, NOW.unix_seconds());
    for entry in [&open, &stale] {
        send(
            &mut relay,
            admin_conn,
            &ClientFrame::Keyring {
                entry: entry.encode().unwrap(),
            },
        );
    }
    let b = device(2);
    let (conn, _) = handshake(&mut relay, &b, ROOM, 0);
    let probe = claim(&b, &[], 5);
    let out = send(
        &mut relay,
        conn,
        &ClientFrame::Keyring {
            entry: probe.encode().unwrap(),
        },
    );
    let frames = sends(&out, conn);
    assert_eq!(frames.len(), 3, "the keyring only");
    assert!(
        frames
            .iter()
            .all(|f| matches!(f, RelayFrame::Keyring { .. }))
    );
    assert_eq!(stores(&out), 0);
    let real = claim(&b, &[&open, &stale], 5);
    let out = send(
        &mut relay,
        conn,
        &ClientFrame::Keyring {
            entry: real.encode().unwrap(),
        },
    );
    assert!(matches!(sends(&out, conn)[0], RelayFrame::Welcome { .. }));
    assert_eq!(
        sends(&out, admin_conn)[0],
        RelayFrame::Keyring {
            entry: real.encode().unwrap()
        }
    );
    let c = device(3);
    let (conn, _) = handshake(&mut relay, &c, ROOM, 0);
    let out = send(
        &mut relay,
        conn,
        &ClientFrame::Keyring {
            entry: claim(&c, &[], 5).encode().unwrap(),
        },
    );
    assert_eq!(refused(&out, conn), Some(RefusalReason::Used));
    let (conn, _) = handshake(&mut relay, &c, ROOM, 0);
    let out = send(
        &mut relay,
        conn,
        &ClientFrame::Keyring {
            entry: claim(&c, &[], 6).encode().unwrap(),
        },
    );
    assert_eq!(refused(&out, conn), Some(RefusalReason::Expired));
}

/// Pushes and snapshots are checked against the room: kind, document and a known key id.
#[test]
fn envelopes_of_another_kind_room_or_key_are_refused() {
    let other = RoomId([0xee; 16]);
    let bad = [
        envelope(EnvelopeKind::Presence, ROOM, None, 1),
        envelope(EnvelopeKind::Update, other, None, 1),
        {
            let foreign = EpochKey::from_bytes([3; 32]);
            seal(
                &Sealing {
                    kind: EnvelopeKind::Update,
                    epoch: 0,
                    key: &foreign,
                    document_id: ROOM.document_id(),
                    sender_peer: 1,
                    signer: None,
                },
                [1; 24],
                b"x",
            )
            .unwrap()
        },
    ];
    for envelope in bad {
        let (mut relay, conn, _) = room_with_admin();
        let out = send(
            &mut relay,
            conn,
            &ClientFrame::Push {
                client_id: 1,
                kind: RecordKind::Update,
                envelope,
            },
        );
        assert_eq!(refused(&out, conn), Some(RefusalReason::Protocol));
        assert_eq!(relay.room(&ROOM).unwrap().head_seq(), 0);
    }
}

/// Decision 10: a signed snapshot replaces the stored one and prunes the records it covers; a
/// newcomer receives it first; one beyond the head or unsigned is refused.
#[test]
fn snapshots_replace_and_prune() {
    let (mut relay, conn, _) = room_with_admin();
    let admin = device(1);
    for n in 1..=4u8 {
        send(
            &mut relay,
            conn,
            &ClientFrame::Push {
                client_id: u64::from(n),
                kind: RecordKind::Update,
                envelope: envelope(EnvelopeKind::Update, ROOM, None, n),
            },
        );
    }
    let snapshot = envelope(EnvelopeKind::Snapshot, ROOM, Some(&admin), 9);
    let out = send(
        &mut relay,
        conn,
        &ClientFrame::Snapshot {
            covers_seq: 3,
            envelope: snapshot.clone(),
        },
    );
    assert!(out.iter().any(|o| matches!(
        o,
        RelayOutput::Store(StoreOp::Snapshot { covers_seq: 3, .. })
    )));
    let room = relay.room(&ROOM).unwrap();
    assert_eq!(room.snapshot_seq(), 3);
    assert_eq!(
        room.records().iter().map(|r| r.seq).collect::<Vec<_>>(),
        vec![4]
    );
    let (fresh, out) = handshake(&mut relay, &admin, ROOM, 0);
    let records: Vec<_> = sends(&out, fresh)
        .into_iter()
        .filter_map(|f| match f {
            RelayFrame::Record { seq, kind, .. } => Some((seq, kind)),
            _ => None,
        })
        .collect();
    assert_eq!(
        records,
        vec![(3, RecordKind::Snapshot), (4, RecordKind::Update)]
    );
    let out = send(
        &mut relay,
        conn,
        &ClientFrame::Snapshot {
            covers_seq: 9,
            envelope: snapshot,
        },
    );
    assert_eq!(refused(&out, conn), Some(RefusalReason::Protocol));
    let (conn, _) = handshake(&mut relay, &admin, ROOM, 4);
    let unsigned = envelope(EnvelopeKind::Update, ROOM, None, 3);
    let out = send(
        &mut relay,
        conn,
        &ClientFrame::Snapshot {
            covers_seq: 4,
            envelope: unsigned,
        },
    );
    assert_eq!(refused(&out, conn), Some(RefusalReason::Protocol));
}

/// Presence is relayed to the others and never stored.
#[test]
fn awareness_is_relayed_and_never_stored() {
    let (mut relay, conn, _) = room_with_admin();
    let (other, _) = handshake(&mut relay, &device(1), ROOM, 0);
    let presence = envelope(EnvelopeKind::Presence, ROOM, None, 4);
    let out = send(
        &mut relay,
        conn,
        &ClientFrame::Awareness {
            envelope: presence.clone(),
        },
    );
    assert_eq!(stores(&out), 0);
    assert_eq!(
        sends(&out, other),
        vec![RelayFrame::Awareness {
            connection: conn,
            envelope: presence
        }]
    );
    assert_eq!(sends(&out, conn), [] as [curvyo_sync_core::RelayFrame; 0]);
}

/// Criterion 25 (`too-large`): a message over 64 MiB is refused and the connection closed.
#[test]
fn a_message_over_64_mib_is_refused() {
    let (mut relay, conn, _) = room_with_admin();
    let mut bytes = vec![0; MAX_MESSAGE_LEN + 1];
    bytes[0] = 0x04;
    let out = relay.receive(conn, &bytes, NOW, [0; 32]);
    assert_eq!(refused(&out, conn), Some(RefusalReason::TooLarge));
    assert!(out.contains(&RelayOutput::Close { connection: conn }));
}

/// Criteria 20 and 38: a relay restored from its store holds the same room, head and keyring;
/// a damaged store is refused.
#[test]
fn a_restored_relay_continues_the_sequence() {
    let (mut relay, conn, _) = room_with_admin();
    for n in 1..=2u8 {
        send(
            &mut relay,
            conn,
            &ClientFrame::Push {
                client_id: u64::from(n),
                kind: RecordKind::Asset,
                envelope: envelope(EnvelopeKind::Asset, ROOM, None, n),
            },
        );
    }
    let stored = relay.room(&ROOM).unwrap().stored();
    let mut restored = Relay::restore(vec![stored.clone()]).unwrap();
    assert_eq!(restored.room(&ROOM).unwrap().head_seq(), 2);
    assert_eq!(restored.room(&ROOM).unwrap().stored(), stored);
    let (conn, _) = handshake(&mut restored, &device(1), ROOM, 2);
    let out = send(
        &mut restored,
        conn,
        &ClientFrame::Push {
            client_id: 1,
            kind: RecordKind::Update,
            envelope: envelope(EnvelopeKind::Update, ROOM, None, 3),
        },
    );
    assert_eq!(
        sends(&out, conn),
        vec![RelayFrame::Ack {
            client_id: 1,
            seq: 3
        }]
    );
    let mut damaged = stored.clone();
    damaged.keyring[0][5] ^= 1;
    assert_eq!(
        Relay::restore(vec![damaged]).err(),
        Some(RestoreError::DamagedKeyring)
    );
    let mut unordered = stored;
    unordered.records.reverse();
    assert_eq!(
        Relay::restore(vec![unordered]).err(),
        Some(RestoreError::DamagedLog)
    );
    let empty = StoredRoom {
        room: ROOM,
        keyring: Vec::new(),
        snapshot: None,
        records: Vec::new(),
    };
    assert_eq!(
        Relay::restore(vec![empty]).err(),
        Some(RestoreError::DamagedKeyring)
    );
}
