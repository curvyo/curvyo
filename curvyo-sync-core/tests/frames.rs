//! The wire frames of ADR 0031 decision 5: golden bytes, round trips, refusal of malformed
//! messages and of messages over 64 MiB (`specs/0092` criterion 25, `too-large`).

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use curvyo_crypto_core::MemberId;
use curvyo_sync_core::{
    ClientFrame, ConnectionId, FrameError, MAX_MESSAGE_LEN, PROTOCOL_VERSION, RecordKind,
    RefusalReason, RelayFrame, RoomId,
};

fn client_frames() -> Vec<ClientFrame> {
    vec![
        // `frames_v1.bin` pins the protocol 1 bytes; protocol 2 adds `SnapshotPart` only.
        ClientFrame::Hello {
            protocol: 1,
            room: RoomId([0xd0; 16]),
            since_seq: 42,
            identity: MemberId([0x11; 32]),
        },
        ClientFrame::Proof {
            signature: [0x22; 64],
        },
        ClientFrame::Keyring {
            entry: vec![1, 2, 3],
        },
        ClientFrame::Push {
            client_id: 7,
            kind: RecordKind::Update,
            envelope: vec![0xe0; 5],
        },
        ClientFrame::Push {
            client_id: 8,
            kind: RecordKind::Asset,
            envelope: vec![0xe1; 3],
        },
        ClientFrame::Snapshot {
            covers_seq: 500,
            envelope: vec![0xe2; 4],
        },
        ClientFrame::Awareness {
            envelope: vec![0xe3; 2],
        },
        ClientFrame::Ping,
    ]
}

fn relay_frames() -> Vec<RelayFrame> {
    vec![
        RelayFrame::Challenge { nonce: [0x33; 32] },
        RelayFrame::Welcome {
            head_seq: 9,
            snapshot_seq: 4,
            connection: ConnectionId(3),
        },
        RelayFrame::Keyring {
            entry: vec![4, 5, 6],
        },
        RelayFrame::Record {
            seq: 9,
            kind: RecordKind::Update,
            envelope: vec![0xe4; 5],
        },
        RelayFrame::Record {
            seq: 4,
            kind: RecordKind::Snapshot,
            envelope: vec![0xe5; 5],
        },
        RelayFrame::Ack {
            client_id: 7,
            seq: 10,
        },
        RelayFrame::Peer {
            connection: ConnectionId(5),
            identity: Some(MemberId([0x44; 32])),
        },
        RelayFrame::Peer {
            connection: ConnectionId(5),
            identity: None,
        },
        RelayFrame::Awareness {
            connection: ConnectionId(5),
            envelope: vec![0xe6; 2],
        },
        RelayFrame::Refused {
            reason: RefusalReason::Used,
        },
        RelayFrame::Refused {
            reason: RefusalReason::Expired,
        },
        RelayFrame::Refused {
            reason: RefusalReason::NotAMember,
        },
        RelayFrame::Refused {
            reason: RefusalReason::TooLarge,
        },
        RelayFrame::Refused {
            reason: RefusalReason::Protocol,
        },
        RelayFrame::Refused {
            reason: RefusalReason::Removed,
        },
        RelayFrame::Pong,
    ]
}

/// Every frame, each as `length u32 LE ‖ bytes`.
fn all_encoded() -> Vec<u8> {
    let mut out = Vec::new();
    let messages = client_frames()
        .iter()
        .map(ClientFrame::encode)
        .chain(relay_frames().iter().map(RelayFrame::encode))
        .collect::<Vec<_>>();
    for message in messages {
        out.extend_from_slice(&u32::try_from(message.len()).unwrap().to_le_bytes());
        out.extend_from_slice(&message);
    }
    out
}

#[test]
fn every_frame_matches_its_golden_bytes() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/frames_v1.bin");
    let bytes = all_encoded();
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &bytes).unwrap();
    }
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes,
        "frames_v1.bin differs"
    );
}

/// Relay protocol 2 (ADR 0036 decision 7): `Hello` naming 2 and `SnapshotPart`, golden in
/// `frames_v2.bin`; regenerate with `CURVYO_UPDATE_GOLDEN=1`.
#[test]
fn protocol_2_frames_match_their_golden_bytes() {
    assert_eq!(PROTOCOL_VERSION, 2);
    let frames = [
        ClientFrame::Hello {
            protocol: PROTOCOL_VERSION,
            room: RoomId([0xd0; 16]),
            since_seq: 42,
            identity: MemberId([0x11; 32]),
        },
        ClientFrame::SnapshotPart {
            covers_seq: 500,
            index: 0,
            last: false,
            envelope: vec![0xe7; 4],
        },
        ClientFrame::SnapshotPart {
            covers_seq: 500,
            index: 1,
            last: true,
            envelope: vec![0xe8; 3],
        },
    ];
    let mut bytes = Vec::new();
    for frame in &frames {
        let message = frame.encode();
        assert_eq!(ClientFrame::decode(&message).unwrap(), *frame);
        bytes.extend_from_slice(&u32::try_from(message.len()).unwrap().to_le_bytes());
        bytes.extend_from_slice(&message);
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/frames_v2.bin");
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &bytes).unwrap();
    }
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes,
        "frames_v2.bin differs"
    );
    // A `last` flag other than 0 or 1 is malformed.
    let mut odd = frames[1].encode();
    odd[13] = 2;
    assert_eq!(ClientFrame::decode(&odd), Err(FrameError::Malformed));
}

#[test]
fn the_layout_is_tag_then_little_endian_fields() {
    assert_eq!(
        ClientFrame::Push {
            client_id: 0x0102,
            kind: RecordKind::Asset,
            envelope: vec![0xaa],
        }
        .encode(),
        [0x04, 0x02, 0x01, 0, 0, 0, 0, 0, 0, 0x02, 0xaa]
    );
    assert_eq!(
        RelayFrame::Ack {
            client_id: 1,
            seq: 2
        }
        .encode(),
        [0x85, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(RelayFrame::Pong.encode(), [0x89]);
    assert_eq!(ClientFrame::Ping.encode(), [0x07]);
}

#[test]
fn every_frame_round_trips() {
    for frame in client_frames() {
        assert_eq!(ClientFrame::decode(&frame.encode()).unwrap(), frame);
    }
    for frame in relay_frames() {
        assert_eq!(RelayFrame::decode(&frame.encode()).unwrap(), frame);
    }
}

#[test]
fn truncated_extended_and_unknown_messages_are_malformed() {
    for frame in client_frames() {
        let bytes = frame.encode();
        if bytes.len() > 1 && !matches!(frame, ClientFrame::Keyring { .. }) {
            let cut = &bytes[..bytes.len() - 1];
            // A trailing bytes field may lose a byte and stay a frame; a fixed one may not.
            if matches!(frame, ClientFrame::Hello { .. } | ClientFrame::Proof { .. }) {
                assert_eq!(ClientFrame::decode(cut), Err(FrameError::Malformed));
            }
        }
    }
    let mut ping = ClientFrame::Ping.encode();
    ping.push(0);
    assert_eq!(ClientFrame::decode(&ping), Err(FrameError::Malformed));
    assert_eq!(ClientFrame::decode(&[]), Err(FrameError::Malformed));
    assert_eq!(ClientFrame::decode(&[0x55]), Err(FrameError::Malformed));
    assert_eq!(RelayFrame::decode(&[0x07]), Err(FrameError::Malformed));
    // Empty trailing bytes, an unknown kind, a pushed snapshot, an unknown refusal.
    assert_eq!(ClientFrame::decode(&[0x06]), Err(FrameError::Malformed));
    let mut push = vec![0x04];
    push.extend_from_slice(&[0; 8]);
    assert_eq!(
        ClientFrame::decode(&[push.clone(), vec![9, 1]].concat()),
        Err(FrameError::Malformed)
    );
    assert_eq!(
        ClientFrame::decode(&[push, vec![3, 1]].concat()),
        Err(FrameError::Malformed)
    );
    assert_eq!(RelayFrame::decode(&[0x88, 7]), Err(FrameError::Malformed));
    assert_eq!(
        RelayFrame::decode(&[0x86, 0, 0, 0, 0, 0, 0, 0, 0, 2]),
        Err(FrameError::Malformed)
    );
}

/// Criterion 25 (`too-large`): a message over 64 MiB is refused before it is read; one of
/// exactly 64 MiB is read.
#[test]
fn messages_over_64_mib_are_too_large() {
    assert_eq!(MAX_MESSAGE_LEN, 64 * 1024 * 1024);
    let mut bytes = vec![0xe7; MAX_MESSAGE_LEN + 1];
    bytes[0] = 0x06;
    assert_eq!(ClientFrame::decode(&bytes), Err(FrameError::TooLarge));
    bytes.pop();
    assert!(matches!(
        ClientFrame::decode(&bytes),
        Ok(ClientFrame::Awareness { .. })
    ));
    let mut relay = vec![0xe7; MAX_MESSAGE_LEN + 1];
    relay[0] = 0x89;
    assert_eq!(RelayFrame::decode(&relay), Err(FrameError::TooLarge));
}
