//! A workspace room seen from the clients (`specs/0095-workspace` criteria 25 to 29 and 34, ADR
//! 0037 decisions 5, 7 and 9): sharing uploads the snapshot in parts, a `2.` invite admits the
//! joiner, the joiner reassembles every stream, and live records carry their stream inside the
//! seal with sender peer 0 in the clear.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::collections::VecDeque;

use curvyo_crypto_core::EnvelopeHeader;
use curvyo_sync_core::{
    ClientFrame, ClientOutput, ClientSession, ConnectionId, InviteKind, InviteLink, Millis,
    PART_BYTES, RecordKind, Relay, RelayOutput, RoomId, SessionConfig, SessionEvent, SessionStart,
    SnapshotAssembler, StreamEntry, decode_inner_record, encode_inner_record,
    encode_snapshot_stream, split_snapshot,
};
use support::{device, seed};

const WORKSPACE: RoomId = RoomId([0x57; 16]);
const LIST: [u8; 16] = [0x57; 16];
const LID: [u8; 16] = [0x11; 16];
const SIDES: [u8; 16] = [0x22; 16];
const START: Millis = Millis(1_800_000_000_000);

struct Device {
    session: ClientSession,
    conn: Option<ConnectionId>,
    events: Vec<SessionEvent>,
    sent: Vec<Vec<u8>>,
}

struct Net {
    relay: Relay,
    now: Millis,
    devices: Vec<Device>,
    nonce: u8,
}

impl Net {
    fn new() -> Self {
        Self {
            relay: Relay::new(),
            now: START,
            devices: Vec::new(),
            nonce: 0,
        }
    }

    fn add(&mut self, n: u8, start: SessionStart) -> usize {
        let mut session = ClientSession::new(
            SessionConfig {
                room: WORKSPACE,
                identity: device(n),
                // Every workspace record names peer 0 in the clear (decision 5).
                peer_id: 0,
                nonce_seed: seed(n, 1),
                entropy_seed: seed(n, 2),
                since_seq: 0,
                start,
            },
            self.now,
        )
        .unwrap();
        session.use_workspace_snapshots();
        self.devices.push(Device {
            session,
            conn: None,
            events: Vec::new(),
            sent: Vec::new(),
        });
        self.devices.len() - 1
    }

    fn pump(&mut self) {
        let mut queue: VecDeque<(usize, Vec<u8>)> = VecDeque::new();
        loop {
            let mut moved = false;
            for i in 0..self.devices.len() {
                for output in self.devices[i].session.drain() {
                    moved = true;
                    match output {
                        ClientOutput::Connect => {
                            let conn = self.relay.connect();
                            self.devices[i].conn = Some(conn);
                            self.devices[i].session.connected(self.now);
                        }
                        ClientOutput::Send(bytes) => {
                            self.devices[i].sent.push(bytes.clone());
                            let Some(conn) = self.devices[i].conn else {
                                continue;
                            };
                            self.nonce = self.nonce.wrapping_add(1);
                            let out = self.relay.receive(conn, &bytes, self.now, [self.nonce; 32]);
                            for o in out {
                                if let RelayOutput::Send { connection, frame } = o
                                    && let Some(to) =
                                        self.devices.iter().position(|d| d.conn == Some(connection))
                                {
                                    queue.push_back((to, frame.encode()));
                                }
                            }
                        }
                        ClientOutput::Close => {
                            if let Some(conn) = self.devices[i].conn.take() {
                                let _ = self.relay.disconnect(conn);
                            }
                        }
                        ClientOutput::Event(event) => self.devices[i].events.push(event),
                    }
                }
            }
            while let Some((to, bytes)) = queue.pop_front() {
                moved = true;
                self.devices[to].session.received(&bytes, self.now);
            }
            if !moved {
                return;
            }
        }
    }

    fn run(&mut self, ms: u64) {
        let until = self.now.plus(ms);
        while self.now < until {
            self.now = self.now.plus(10);
            for d in &mut self.devices {
                d.session.tick(self.now);
            }
            self.pump();
        }
    }

    fn take_events(&mut self, i: usize) -> Vec<SessionEvent> {
        std::mem::take(&mut self.devices[i].events)
    }
}

fn workspace_stream() -> Vec<u8> {
    encode_snapshot_stream(&[
        StreamEntry {
            stream: LIST,
            bytes: b"list snapshot".to_vec(),
        },
        StreamEntry {
            stream: LID,
            bytes: vec![0x4c; PART_BYTES],
        },
        StreamEntry {
            stream: SIDES,
            bytes: b"sides".to_vec(),
        },
    ])
}

/// A shares the workspace and uploads its snapshot in parts when the relay asks for the first
/// snapshot; B joins by a `2.` link.
fn shared_pair() -> (Net, usize, usize, String) {
    let mut net = Net::new();
    let a = net.add(1, SessionStart::Share);
    net.pump();
    net.run(100);
    let due = net
        .take_events(a)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::SnapshotDue { covers_seq } => Some(covers_seq),
            _ => None,
        })
        .expect("sharing asks for the first snapshot");
    let parts = split_snapshot(&workspace_stream(), [0xab; 16]);
    assert_eq!(parts.len(), 2, "a stream over 4 MiB takes two parts");
    net.devices[a]
        .session
        .upload_snapshot_parts(&parts, due)
        .unwrap();
    net.pump();
    let expires = net.now.unix_seconds() + 3600;
    let secret = net.devices[a].session.invite(expires).unwrap();
    let link = InviteLink {
        server: "ws://relay.test:8787".to_owned(),
        kind: InviteKind::Workspace,
        secret,
    }
    .to_text();
    net.pump();
    let parsed = InviteLink::parse(&link).unwrap();
    assert_eq!(parsed.kind, InviteKind::Workspace);
    let b = net.add(2, SessionStart::Join(parsed.secret));
    net.pump();
    net.run(500);
    (net, a, b, link)
}

/// Criteria 25, 27 and 28: the joiner is admitted by the online member, receives the snapshot's
/// parts as consecutive records of one sequence and reassembles every stream from them.
#[test]
fn a_joiner_reassembles_the_workspace_snapshot_from_its_parts() {
    let (mut net, a, b, link) = shared_pair();
    assert!(link.contains("#2."));
    let events = net.take_events(b);
    assert!(events.contains(&SessionEvent::Admitted), "{events:?}");
    let parts: Vec<(u64, Vec<u8>)> = events
        .into_iter()
        .filter_map(|e| match e {
            SessionEvent::Remote {
                seq,
                kind: RecordKind::Snapshot,
                bytes,
            } => Some((seq, bytes)),
            _ => None,
        })
        .collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(
        parts[0].0, parts[1].0,
        "the parts are records of one sequence"
    );
    let mut assembler = SnapshotAssembler::default();
    let mut streams = Vec::new();
    for (_, part) in &parts {
        streams.extend(assembler.push_part(part).unwrap());
    }
    assert!(assembler.is_complete());
    let ids: Vec<[u8; 16]> = streams.iter().map(|s| s.stream).collect();
    assert_eq!(ids, vec![LIST, LID, SIDES]);
    assert_eq!(streams[1].bytes.len(), PART_BYTES);
    assert_eq!(
        net.devices[a]
            .session
            .keyring()
            .unwrap()
            .participants()
            .len(),
        2
    );
}

/// Criteria 30 and 34: a live record carries its stream inside the seal; in the clear every
/// workspace envelope names the workspace and peer 0, whoever wrote it.
#[test]
fn live_records_carry_their_stream_inside_the_seal() {
    let (mut net, a, b, _) = shared_pair();
    net.take_events(b);
    let record = encode_inner_record(SIDES, 0xa1, b"update of Sides");
    net.devices[a]
        .session
        .push(RecordKind::Update, record.clone())
        .unwrap();
    net.pump();
    let received: Vec<Vec<u8>> = net
        .take_events(b)
        .into_iter()
        .filter_map(|e| match e {
            SessionEvent::Remote {
                kind: RecordKind::Update,
                bytes,
                ..
            } => Some(bytes),
            _ => None,
        })
        .collect();
    assert_eq!(received, vec![record]);
    let (header, payload) = decode_inner_record(&received[0]).unwrap();
    assert_eq!((header.stream, header.peer), (SIDES, 0xa1));
    assert_eq!(payload, b"update of Sides");
    for device in &net.devices {
        for bytes in &device.sent {
            let Ok(ClientFrame::Push { envelope, .. } | ClientFrame::SnapshotPart { envelope, .. }) =
                ClientFrame::decode(bytes)
            else {
                continue;
            };
            let header = EnvelopeHeader::parse(&envelope).unwrap();
            assert_eq!(header.sender_peer, 0);
            assert_eq!(header.document_id, WORKSPACE.document_id());
        }
    }
}

/// ADR 0037 decision 7: a snapshot made while offline goes out as parts on the next connection.
#[test]
fn snapshot_parts_made_offline_go_out_on_reconnect() {
    let (mut net, a, _, _) = shared_pair();
    net.devices[a].session.disconnected(net.now);
    net.pump();
    let parts = split_snapshot(&workspace_stream(), [0xcd; 16]);
    let cursor = net.devices[a].session.cursor();
    net.devices[a]
        .session
        .upload_snapshot_parts(&parts, cursor)
        .unwrap();
    let before = net.devices[a].sent.len();
    net.pump();
    assert_eq!(
        net.devices[a].sent.len(),
        before,
        "nothing goes out offline"
    );
    net.run(15_000);
    let sent_parts = net.devices[a].sent[before..]
        .iter()
        .filter(|bytes| {
            matches!(
                ClientFrame::decode(bytes),
                Ok(ClientFrame::SnapshotPart { .. })
            )
        })
        .count();
    assert_eq!(sent_parts, 2);
}
