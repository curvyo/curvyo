//! Tester acceptance tests for `specs/0105-sharing-concept` (ADR 0044 decision 5 and
//! `arch-review-join-rule.md` findings 1 to 3) and criterion 3 at the link level, written from
//! the specification and the review before the implementation was read: a removed device gets
//! nothing with its old invite, a backdated claim is neither stored nor admitted nor forwarded,
//! the way back is a fresh invite on the current heads, and a `1.` link parses offline as a
//! document link (the refusal needs no network).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use curvyo_crypto_core::keyring::{Entry, Payload, RecipientWrap, Role};
use curvyo_crypto_core::{EpochKey, IdentityKeys, InviteKey, wrap_epoch_key};
use curvyo_sync_core::{
    ClientFrame, ConnectionId, InviteKind, InviteLink, Millis, PROTOCOL_VERSION, RefusalReason,
    Relay, RelayFrame, RelayOutput, RoomId,
};
use support::{ROOM, device};

const NOW: Millis = Millis(1_800_000_000_000);
const DAY: u64 = 24 * 3600;

fn genesis(admin: &IdentityKeys) -> Entry {
    let me = admin.identity();
    let key = EpochKey::from_bytes([0x41; 32]);
    Entry::sign(
        Vec::new(),
        admin,
        Payload::Genesis {
            document_id: ROOM.document_id(),
            admins: vec![me],
            epoch0_wraps: vec![RecipientWrap {
                member: me.member,
                wrap: wrap_epoch_key(&key, 0, ROOM.document_id(), &me, [7; 32]).unwrap(),
            }],
        },
    )
    .unwrap()
}

fn frames(out: &[RelayOutput], to: ConnectionId) -> Vec<RelayFrame> {
    out.iter()
        .filter_map(|o| match o {
            RelayOutput::Send { connection, frame } if *connection == to => Some(frame.clone()),
            _ => None,
        })
        .collect()
}

fn stored(out: &[RelayOutput]) -> usize {
    out.iter()
        .filter(|o| matches!(o, RelayOutput::Store(_)))
        .count()
}

fn refusal(out: &[RelayOutput], to: ConnectionId) -> Option<RefusalReason> {
    frames(out, to).iter().find_map(|f| match f {
        RelayFrame::Refused { reason } => Some(*reason),
        _ => None,
    })
}

fn got_keyring_or_welcome(out: &[RelayOutput], to: ConnectionId) -> bool {
    frames(out, to)
        .iter()
        .any(|f| matches!(f, RelayFrame::Keyring { .. } | RelayFrame::Welcome { .. }))
}

fn send_at(
    relay: &mut Relay,
    conn: ConnectionId,
    frame: &ClientFrame,
    at: Millis,
) -> Vec<RelayOutput> {
    relay.receive(conn, &frame.encode(), at, [9; 32])
}

fn entry_frame(entry: &Entry) -> ClientFrame {
    ClientFrame::Keyring {
        entry: entry.encode().unwrap(),
    }
}

/// Hello from `since` and the proof; the outputs of the proof.
fn connect(
    relay: &mut Relay,
    who: &IdentityKeys,
    since: u64,
    at: Millis,
) -> (ConnectionId, Vec<RelayOutput>) {
    let conn = relay.connect();
    let out = send_at(
        relay,
        conn,
        &ClientFrame::Hello {
            protocol: PROTOCOL_VERSION,
            room: ROOM,
            since_seq: since,
            identity: who.identity().member,
        },
        at,
    );
    let Some(RelayFrame::Challenge { nonce }) = frames(&out, conn).first().cloned() else {
        panic!("a challenge");
    };
    let mut message = b"curvyo-relay-v1".to_vec();
    message.extend_from_slice(&ROOM.0);
    message.extend_from_slice(&nonce);
    let out = send_at(
        relay,
        conn,
        &ClientFrame::Proof {
            signature: who.sign(&message),
        },
        at,
    );
    (conn, out)
}

fn invite(admin: &IdentityKeys, parents: &[&Entry], n: u8, expires: u64) -> Entry {
    Entry::sign(
        parents.iter().map(|e| e.try_hash().unwrap()).collect(),
        admin,
        Payload::AddPendingInvite {
            invite_id: InviteKey::from_seed(&[n; 32]).unwrap().invite_id(),
            role: Role::Member,
            expires,
        },
    )
    .unwrap()
}

fn claim(joiner: &IdentityKeys, parents: &[&Entry], n: u8) -> Entry {
    let key = InviteKey::from_seed(&[n; 32]).unwrap();
    Entry::sign(
        parents.iter().map(|e| e.try_hash().unwrap()).collect(),
        joiner,
        Payload::ClaimInvite {
            invite_id: key.invite_id(),
            identity: joiner.identity(),
            proof: key.claim_proof(ROOM.document_id(), &joiner.identity()),
        },
    )
    .unwrap()
}

/// The room after: admin 1 made invites 20 (for B, valid 24 h), 21 (still open, 24 h) and 22
/// (for C); B (device 2) and C (device 3) claimed theirs; the admin removed B.
struct Room {
    relay: Relay,
    admin: ConnectionId,
    c: ConnectionId,
    own: Entry,
    open: Entry,
    head: Entry,
    entries: usize,
}

fn room() -> Room {
    let mut relay = Relay::new();
    let admin_keys = device(1);
    let (admin, _) = connect(&mut relay, &admin_keys, 0, NOW);
    let g = genesis(&admin_keys);
    send_at(&mut relay, admin, &entry_frame(&g), NOW);
    let own = invite(&admin_keys, &[&g], 20, NOW.unix_seconds() + DAY);
    let open = invite(&admin_keys, &[&g], 21, NOW.unix_seconds() + DAY);
    let for_c = invite(&admin_keys, &[&g], 22, NOW.unix_seconds() + DAY);
    for e in [&own, &open, &for_c] {
        send_at(&mut relay, admin, &entry_frame(e), NOW);
    }
    let (b, _) = connect(&mut relay, &device(2), 0, NOW);
    let b_claim = claim(&device(2), &[&own, &open, &for_c], 20);
    let out = send_at(&mut relay, b, &entry_frame(&b_claim), NOW);
    assert!(matches!(frames(&out, b)[0], RelayFrame::Welcome { .. }));
    let (c, _) = connect(&mut relay, &device(3), 0, NOW);
    let c_claim = claim(&device(3), &[&b_claim], 22);
    let out = send_at(&mut relay, c, &entry_frame(&c_claim), NOW);
    assert!(matches!(frames(&out, c)[0], RelayFrame::Welcome { .. }));
    let removal = Entry::sign(
        vec![c_claim.try_hash().unwrap()],
        &admin_keys,
        Payload::RemoveMember {
            member: device(2).identity().member,
        },
    )
    .unwrap();
    let out = send_at(&mut relay, admin, &entry_frame(&removal), NOW);
    assert_eq!(refusal(&out, b), Some(RefusalReason::Removed));
    relay.disconnect(b);
    Room {
        relay,
        admin,
        c,
        own,
        open,
        head: removal,
        entries: 7,
    }
}

fn keyring_len(relay: &Relay) -> usize {
    relay.room(&ROOM).unwrap().stored().keyring.len()
}

/// Finding 1 / test (a): the removed device at cursor 0 with its own old secret, within 24 h,
/// as a probe and as a real claim on the current heads: `Removed`, no keyring frame, nothing
/// stored, nothing forwarded.
#[test]
fn join_rule_a_old_secret_is_refused_as_removed_with_nothing_sent() {
    let mut r = room();
    let b = device(2);
    let head = r.head.clone();
    for (label, entry) in [
        ("probe", claim(&b, &[], 20)),
        ("claim on current heads", claim(&b, &[&head], 20)),
    ] {
        let (conn, out) = connect(&mut r.relay, &b, 0, NOW);
        assert!(
            frames(&out, conn).is_empty(),
            "{label}: nothing on the proof"
        );
        let out = send_at(&mut r.relay, conn, &entry_frame(&entry), NOW);
        assert_eq!(refusal(&out, conn), Some(RefusalReason::Removed), "{label}");
        assert!(!got_keyring_or_welcome(&out, conn), "{label}");
        assert_eq!(stored(&out), 0, "{label}");
        assert!(
            frames(&out, r.admin).is_empty(),
            "{label}: admin hears nothing"
        );
        assert!(frames(&out, r.c).is_empty(), "{label}: C hears nothing");
        r.relay.disconnect(conn);
    }
    assert_eq!(keyring_len(&r.relay), r.entries);
}

/// Finding 1, past the 24 h: the removed device's own expired invite still says `Removed`
/// (the device is told the truth, not `Expired`), and nothing is sent.
#[test]
fn join_rule_a_expired_old_secret_still_says_removed() {
    let mut r = room();
    let later = Millis(NOW.0 + (DAY + 3600) * 1000);
    let (conn, _) = connect(&mut r.relay, &device(2), 0, later);
    let out = send_at(
        &mut r.relay,
        conn,
        &entry_frame(&claim(&device(2), &[], 20)),
        later,
    );
    assert_eq!(refusal(&out, conn), Some(RefusalReason::Removed));
    assert!(!got_keyring_or_welcome(&out, conn));
}

/// What holds: a removed device at cursor 0 that sends anything but a keyring entry gets
/// `Removed` and does not hang; from a cursor it is refused at once.
#[test]
fn join_rule_a_ping_or_a_cursor_from_a_removed_device_is_refused() {
    let mut r = room();
    let (conn, _) = connect(&mut r.relay, &device(2), 0, NOW);
    let out = send_at(&mut r.relay, conn, &ClientFrame::Ping, NOW);
    assert_eq!(refusal(&out, conn), Some(RefusalReason::Removed));
    assert!(!got_keyring_or_welcome(&out, conn));
    let (conn, out) = connect(&mut r.relay, &device(2), 3, NOW);
    assert_eq!(refusal(&out, conn), Some(RefusalReason::Removed));
    assert!(!got_keyring_or_welcome(&out, conn));
}

/// Finding 2 / test (b): backdated claims (heads from before the removal) of its own used
/// invite and of the still open invite: not admitted, no Welcome, nothing stored, and a member
/// that connects afterwards never receives such an entry.
#[test]
fn join_rule_b_a_backdated_claim_is_not_admitted_stored_or_forwarded() {
    let mut r = room();
    let b = device(2);
    let (own, open) = (r.own.clone(), r.open.clone());
    for (n, parents) in [(20, vec![&own]), (21, vec![&own, &open]), (21, vec![&open])] {
        let entry = claim(&b, &parents, n);
        let (conn, _) = connect(&mut r.relay, &b, 0, NOW);
        let out = send_at(&mut r.relay, conn, &entry_frame(&entry), NOW);
        assert!(refusal(&out, conn).is_some(), "invite {n}: refused");
        assert!(
            !frames(&out, conn)
                .iter()
                .any(|f| matches!(f, RelayFrame::Welcome { .. })),
            "invite {n}: no Welcome"
        );
        assert_eq!(stored(&out), 0, "invite {n}: nothing stored");
        assert!(frames(&out, r.admin).is_empty(), "invite {n}");
        assert!(frames(&out, r.c).is_empty(), "invite {n}");
        // A push on that connection is not taken either.
        let out = send_at(&mut r.relay, conn, &ClientFrame::Ping, NOW);
        assert_eq!(stored(&out), 0);
        r.relay.disconnect(conn);
    }
    assert_eq!(keyring_len(&r.relay), r.entries);
    // A fresh member connection from cursor 0 gets exactly the stored keyring.
    let (conn, out) = connect(&mut r.relay, &device(3), 0, NOW);
    let keyring_frames = frames(&out, conn)
        .iter()
        .filter(|f| matches!(f, RelayFrame::Keyring { .. }))
        .count();
    assert_eq!(keyring_frames, r.entries, "no refused entry reaches C");
}

/// Test (c): the way back is a fresh invite on the current heads: probe gets the keyring,
/// the claim admits with a Welcome and is forwarded to the members.
#[test]
fn join_rule_c_a_fresh_invite_admits_the_removed_device_again() {
    let mut r = room();
    let admin_keys = device(1);
    let head = r.head.clone();
    let fresh = invite(&admin_keys, &[&head], 23, NOW.unix_seconds() + DAY);
    send_at(&mut r.relay, r.admin, &entry_frame(&fresh), NOW);
    let b = device(2);
    let (conn, _) = connect(&mut r.relay, &b, 0, NOW);
    let out = send_at(&mut r.relay, conn, &entry_frame(&claim(&b, &[], 23)), NOW);
    let probe = frames(&out, conn);
    assert!(!probe.is_empty(), "the probe gets the keyring");
    assert!(
        probe
            .iter()
            .all(|f| matches!(f, RelayFrame::Keyring { .. }))
    );
    assert_eq!(stored(&out), 0);
    let real = claim(&b, &[&fresh], 23);
    let out = send_at(&mut r.relay, conn, &entry_frame(&real), NOW);
    assert!(matches!(frames(&out, conn)[0], RelayFrame::Welcome { .. }));
    assert_eq!(stored(&out), 1);
    assert!(
        frames(&out, r.admin).contains(&RelayFrame::Keyring {
            entry: real.encode().unwrap()
        }),
        "the members get the claim"
    );
    assert_eq!(keyring_len(&r.relay), r.entries + 2);
}

/// Accepted risk of the review (bearer token): a removed device that holds a still open
/// invite meant for someone else and claims it honestly on the current heads is admitted.
#[test]
fn join_rule_c_an_open_invite_on_current_heads_admits() {
    let mut r = room();
    let b = device(2);
    let head = r.head.clone();
    let (conn, _) = connect(&mut r.relay, &b, 0, NOW);
    let out = send_at(
        &mut r.relay,
        conn,
        &entry_frame(&claim(&b, &[&head], 21)),
        NOW,
    );
    assert!(matches!(frames(&out, conn)[0], RelayFrame::Welcome { .. }));
}

/// Regression guard: a current member that connects from cursor 0 is admitted on its proof,
/// and a member's own claimed invite is no way in for another device (`Used`).
#[test]
fn a_current_member_still_connects_and_a_used_invite_stays_used() {
    let mut r = room();
    r.relay.disconnect(r.c);
    let (conn, out) = connect(&mut r.relay, &device(3), 0, NOW);
    assert!(matches!(frames(&out, conn)[0], RelayFrame::Welcome { .. }));
    let (conn, _) = connect(&mut r.relay, &device(4), 0, NOW);
    let out = send_at(
        &mut r.relay,
        conn,
        &entry_frame(&claim(&device(4), &[], 22)),
        NOW,
    );
    assert_eq!(refusal(&out, conn), Some(RefusalReason::Used));
    assert!(!got_keyring_or_welcome(&out, conn));
}

/// Criterion 3 at the link level: a `curvyo://join?…#1.…` link parses offline as a document
/// link, so the device refuses it without a request; a `2.` link is a workspace link.
#[test]
fn c3_a_version_1_link_parses_offline_as_a_document_link() {
    let room = RoomId([0x07; 16]);
    let secret = curvyo_sync_core::InviteSecret {
        room,
        seed: [5; 32],
    };
    for (kind, version) in [(InviteKind::Document, "1."), (InviteKind::Workspace, "2.")] {
        let link = InviteLink {
            kind,
            server: "ws://192.0.2.1:8787".to_owned(),
            secret: secret.clone(),
        };
        let text = link.to_text();
        assert!(text.starts_with("curvyo://join?"), "{text}");
        assert!(text.contains(&format!("#{version}")), "{text}");
        assert_eq!(InviteLink::parse(&text).unwrap().kind, kind);
    }
}
