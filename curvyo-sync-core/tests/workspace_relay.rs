//! Workspace rooms at the relay (`specs/0095-workspace` criteria 34, 47 and 48; ADR 0037
//! decisions 5, 7 and 11): what the relay stores of a workspace does not show how many
//! documents it holds, and two workspaces of one maker are two rooms without cross-talk.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use curvyo_crypto_core::keyring::{Entry, Payload, RecipientWrap};
use curvyo_crypto_core::{EnvelopeKind, EpochKey, IdentityKeys, Sealing, seal, wrap_epoch_key};
use curvyo_sync_core::{
    ClientFrame, ConnectionId, Millis, PROTOCOL_VERSION, RecordKind, Relay, RelayFrame,
    RelayOutput, RoomId, StoredRoom, StreamEntry, encode_inner_record, encode_snapshot_stream,
    split_snapshot,
};
use support::device;

const NOW: Millis = Millis(1_800_000_000_000);
const WORKSPACE: RoomId = RoomId([0x57; 16]);
const OTHER_WORKSPACE: RoomId = RoomId([0x58; 16]);

fn key() -> EpochKey {
    EpochKey::from_bytes([0x42; 32])
}

/// Seals a workspace record: the cleartext header names the workspace and peer 0 (decision 5).
fn sealed(kind: EnvelopeKind, room: RoomId, plaintext: &[u8], n: u64) -> Vec<u8> {
    let admin = device(1);
    let mut nonce = [0xa5; 24];
    nonce[..8].copy_from_slice(&n.to_le_bytes());
    seal(
        &Sealing {
            kind,
            epoch: 0,
            key: &key(),
            document_id: room.document_id(),
            sender_peer: 0,
            signer: (kind == EnvelopeKind::Snapshot).then_some(&admin),
        },
        nonce,
        plaintext,
    )
    .unwrap()
}

fn send(relay: &mut Relay, conn: ConnectionId, frame: &ClientFrame) -> Vec<RelayOutput> {
    relay.receive(conn, &frame.encode(), NOW, [9; 32])
}

/// Connects `who` to `room` and proves the identity.
fn connect(relay: &mut Relay, who: &IdentityKeys, room: RoomId) -> ConnectionId {
    let conn = relay.connect();
    let out = send(
        relay,
        conn,
        &ClientFrame::Hello {
            protocol: PROTOCOL_VERSION,
            room,
            since_seq: 0,
            identity: who.identity().member,
        },
    );
    let nonce = out
        .iter()
        .find_map(|o| match o {
            RelayOutput::Send {
                frame: RelayFrame::Challenge { nonce },
                ..
            } => Some(*nonce),
            _ => None,
        })
        .unwrap();
    let mut message = b"curvyo-relay-v1".to_vec();
    message.extend_from_slice(&room.0);
    message.extend_from_slice(&nonce);
    send(
        relay,
        conn,
        &ClientFrame::Proof {
            signature: who.sign(&message),
        },
    );
    conn
}

/// Device 1 creates the workspace room `room` and stays connected.
fn create(relay: &mut Relay, room: RoomId) -> ConnectionId {
    let admin = device(1);
    let conn = connect(relay, &admin, room);
    let me = admin.identity();
    let genesis = Entry::sign(
        Vec::new(),
        &admin,
        Payload::Genesis {
            document_id: room.document_id(),
            admins: vec![me],
            epoch0_wraps: vec![RecipientWrap {
                member: me.member,
                wrap: wrap_epoch_key(&key(), 0, room.document_id(), &me, [7; 32]).unwrap(),
            }],
        },
    )
    .unwrap();
    send(
        relay,
        conn,
        &ClientFrame::Keyring {
            entry: genesis.encode().unwrap(),
        },
    );
    conn
}

/// A workspace of `documents` documents of equal size and `total` document bytes in all, edited
/// 60 times in the same pattern of record sizes, then compacted by one multi-part snapshot.
fn workspace_server_data(documents: u8, total: usize, list_bytes: usize) -> StoredRoom {
    let mut relay = Relay::new();
    let conn = create(&mut relay, WORKSPACE);
    for n in 0..60_u64 {
        let document = [u8::try_from(n % u64::from(documents)).unwrap(); 16];
        let plaintext = encode_inner_record(document, 3, &[0x5a; 200]);
        send(
            &mut relay,
            conn,
            &ClientFrame::Push {
                client_id: n,
                kind: RecordKind::Update,
                envelope: sealed(EnvelopeKind::Update, WORKSPACE, &plaintext, n),
            },
        );
    }
    let mut entries = vec![StreamEntry {
        stream: WORKSPACE.0,
        bytes: vec![1; list_bytes],
    }];
    entries.extend((0..documents).map(|i| StreamEntry {
        stream: [i; 16],
        bytes: vec![2; total / usize::from(documents)],
    }));
    let parts = split_snapshot(&encode_snapshot_stream(&entries), [0x99; 16]);
    let count = parts.len();
    for (index, part) in parts.iter().enumerate() {
        send(
            &mut relay,
            conn,
            &ClientFrame::SnapshotPart {
                covers_seq: 50,
                index: u32::try_from(index).unwrap(),
                last: index + 1 == count,
                envelope: sealed(
                    EnvelopeKind::Snapshot,
                    WORKSPACE,
                    part,
                    1000 + u64::try_from(index).unwrap(),
                ),
            },
        );
    }
    relay.room(&WORKSPACE).unwrap().stored()
}

/// The shape of what the relay stores: keyring entry sizes, record sequences, kinds and sizes,
/// the snapshot's sequence and part sizes.
fn structure(stored: &StoredRoom) -> Vec<(u64, u8, usize)> {
    let mut out: Vec<(u64, u8, usize)> = stored
        .keyring
        .iter()
        .map(|entry| (0, 0, entry.len()))
        .collect();
    if let Some((seq, first)) = &stored.snapshot {
        out.push((*seq, 3, first.len()));
        out.extend(
            stored
                .snapshot_rest
                .iter()
                .map(|part| (*seq, 3, part.len())),
        );
    }
    out.extend(stored.records.iter().map(|r| {
        (
            r.seq,
            u8::from(r.kind == RecordKind::Asset),
            r.envelope.len(),
        )
    }));
    out
}

/// Criterion 34: a workspace of 3 documents and one of 30 of the same total size and edit
/// pattern give stored data of the same structure; the snapshot is in several parts and pruned
/// the records it covers.
#[test]
fn three_or_thirty_documents_look_the_same_to_the_relay() {
    let few = workspace_server_data(3, 9_000_000, 1_000);
    let many = workspace_server_data(30, 9_000_000, 5_000);
    assert_eq!(structure(&few), structure(&many));
    assert_eq!(few.snapshot_rest.len(), 2, "three parts");
    assert_eq!(few.records.first().map(|r| r.seq), Some(51));
    // Nothing of the inner header is readable: no stream id appears in the stored bytes.
    let stored: Vec<&Vec<u8>> = many
        .records
        .iter()
        .map(|r| &r.envelope)
        .chain(many.snapshot.iter().map(|(_, first)| first))
        .chain(many.snapshot_rest.iter())
        .collect();
    for document in 0..30_u8 {
        let id = [document; 16];
        assert!(
            stored
                .iter()
                .all(|bytes| !bytes.windows(16).any(|w| w == id)),
            "stream {document} readable"
        );
    }
}

/// Criterion 47: two workspaces of one maker are two rooms with their own keyrings; a record of
/// one never reaches a connection of the other.
#[test]
fn two_workspaces_of_one_maker_are_separate_rooms() {
    let mut relay = Relay::new();
    let first = create(&mut relay, WORKSPACE);
    let second = create(&mut relay, OTHER_WORKSPACE);
    let out = send(
        &mut relay,
        first,
        &ClientFrame::Push {
            client_id: 1,
            kind: RecordKind::Update,
            envelope: sealed(
                EnvelopeKind::Update,
                WORKSPACE,
                &encode_inner_record([1; 16], 3, b"edit"),
                1,
            ),
        },
    );
    assert!(out.iter().all(|o| !matches!(
        o,
        RelayOutput::Send { connection, .. } if *connection == second
    )));
    // An envelope of the first workspace is refused in the second.
    let out = send(
        &mut relay,
        second,
        &ClientFrame::Push {
            client_id: 2,
            kind: RecordKind::Update,
            envelope: sealed(EnvelopeKind::Update, WORKSPACE, b"x", 2),
        },
    );
    assert!(out.iter().any(|o| matches!(
        o,
        RelayOutput::Send {
            frame: RelayFrame::Refused { .. },
            ..
        }
    )));
    assert_eq!(relay.room(&WORKSPACE).unwrap().head_seq(), 1);
    assert_eq!(relay.room(&OTHER_WORKSPACE).unwrap().head_seq(), 0);
    assert_eq!(relay.room_ids(), vec![WORKSPACE, OTHER_WORKSPACE]);
}
