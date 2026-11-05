//! The membership lifecycle through the simulator with real document replicas
//! (`specs/0097-membership-lifecycle` criteria 2 to 5, 10 to 14, 18 to 22, 25 and 26; ADR 0041
//! decisions 1, 2, 5 to 7): roles, leave, add back with a history choice and the return check.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use curvyo_crypto_core::keyring::{Entry, EntryHash, Payload, RecipientWrap, Role};
use curvyo_crypto_core::{EpochKey, wrap_epoch_key};
use curvyo_document_core::{Document, Length, NodeId, Point, RectBounds};
use curvyo_sync_core::{
    CheckOutcome, ClientFrame, ClientOutput, ClientSession, ConnectionId, History, Millis,
    RelayFrame, SessionConfig, SessionError, SessionEvent, SessionStart, left_by_own_hand,
};
use support::sim::{Net, shared};
use support::{ROOM, device, doc_id, member, seed};

fn rect(doc: &Document, x: f64) -> NodeId {
    doc.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    })
}

/// A (admin, device 1), B (device 2) and C (device 3), converged.
fn three(seed: u64) -> (Net, [usize; 3]) {
    let doc = Document::new(100);
    rect(&doc, 0.0);
    let (net, c) = shared(seed, ROOM, doc, &[2, 3]);
    (net, [c[0], c[1], c[2]])
}

fn session(net: &mut Net, i: usize) -> &mut ClientSession {
    net.clients[i].session.as_mut().unwrap()
}

fn act(net: &mut Net, i: usize, f: impl FnOnce(&mut ClientSession) -> Result<(), SessionError>) {
    f(session(net, i)).unwrap();
    net.pump(i);
}

/// The frames client `i` sent from message index `from` on.
fn sent_frames(net: &Net, i: usize, from: usize) -> Vec<ClientFrame> {
    net.clients[i].sent[from..]
        .iter()
        .map(|m| ClientFrame::decode(m).unwrap())
        .collect()
}

fn role_seen_by(net: &Net, i: usize, who: u8) -> Option<Role> {
    net.clients[i]
        .session()
        .keyring()
        .unwrap()
        .role(&member(who))
}

/// Criteria 2 to 5: any admin promotes and demotes; the only admin cannot step down, leave or
/// be demoted; a member changes no role.
#[test]
fn admins_promote_and_demote_and_the_last_admin_stays() {
    let (mut net, [a, b, c]) = three(1);
    assert_eq!(
        session(&mut net, a).set_role(member(1), Role::Member),
        Err(SessionError::LastAdmin)
    );
    assert_eq!(session(&mut net, a).leave(), Err(SessionError::LastAdmin));
    assert_eq!(
        session(&mut net, c).set_role(member(2), Role::Admin),
        Err(SessionError::NotAdmin)
    );
    act(&mut net, a, |s| s.set_role(member(2), Role::Admin));
    net.run(1_000);
    for i in [a, b, c] {
        assert_eq!(role_seen_by(&net, i, 2), Some(Role::Admin));
    }
    // B now invites and changes roles; its connection went on.
    assert!(session(&mut net, b).invite(u64::MAX).is_ok());
    assert!(net.clients[b].session().is_online());
    // B demotes A, then the only admin B cannot be demoted or step down.
    act(&mut net, b, |s| s.set_role(member(1), Role::Member));
    net.run(1_000);
    assert_eq!(role_seen_by(&net, c, 1), Some(Role::Member));
    assert_eq!(
        session(&mut net, b).set_role(member(2), Role::Member),
        Err(SessionError::LastAdmin)
    );
    assert_eq!(
        session(&mut net, a).set_role(member(2), Role::Member),
        Err(SessionError::NotAdmin)
    );
    // Step down once A is an admin again.
    act(&mut net, b, |s| s.set_role(member(1), Role::Admin));
    net.run(1_000);
    act(&mut net, b, |s| s.set_role(member(2), Role::Member));
    net.run(1_000);
    assert_eq!(role_seen_by(&net, a, 2), Some(Role::Member));
    assert_eq!(
        session(&mut net, b).invite(u64::MAX),
        Err(SessionError::NotAdmin)
    );
}

/// Criteria 10 to 12: B's leave is its own `RemoveMember`; A sees B left with the key not
/// changed until A rotates; B's session ends and its keys stay.
#[test]
fn a_leave_is_shown_until_an_admin_rotates() {
    let (mut net, [a, b, _]) = three(2);
    act(&mut net, b, ClientSession::leave);
    net.run(1_000);
    assert!(net.clients[b].saw(&SessionEvent::Removed));
    assert!(net.clients[b].session().has_ended());
    assert!(!net.clients[b].epoch_keys.is_empty(), "B keeps its keys");
    let keyring = net.clients[a].session().keyring().unwrap();
    assert!(keyring.was_removed(&member(2)));
    assert!(left_by_own_hand(keyring, &member(2)));
    assert_eq!(keyring.rotation_pending(), vec![member(2)]);
    act(&mut net, a, ClientSession::rotate);
    net.run(1_000);
    let keyring = net.clients[a].session().keyring().unwrap();
    assert_eq!(keyring.rotation_pending(), Vec::new());
}

/// B was removed from a room in which epochs 1 (the removal's) and 2 began while B was away,
/// with C editing under both; returns B's cursor when it was removed and the keyring heads B
/// knew then (its ended record's).
fn away(net: &mut Net, [a, b, c]: [usize; 3]) -> (u64, Vec<EntryHash>) {
    act(net, a, |s| s.remove(member(2)));
    net.run(1_000);
    assert!(net.clients[b].saw(&SessionEvent::Removed));
    let cursor = net.clients[b].session().cursor();
    let heads = known_heads(net, b);
    net.edit(c, |d| rect(d, 10.0));
    net.run(500);
    act(net, a, ClientSession::rotate);
    net.run(1_000);
    net.edit(c, |d| rect(d, 20.0));
    net.run(500);
    let keyring = net.clients[a].session().keyring().unwrap();
    assert_eq!(keyring.current_epoch().number, 2);
    (cursor, heads)
}

/// The keyring heads client `i` knows now: what its ended record keeps.
fn known_heads(net: &Net, i: usize) -> Vec<EntryHash> {
    net.clients[i].session().keyring().unwrap().heads()
}

/// A return check of B; the frames B sent during it and the answer.
fn check(
    net: &mut Net,
    b: usize,
    then_leave: bool,
    heads: &[EntryHash],
) -> (Vec<ClientFrame>, Option<CheckOutcome>) {
    let from = net.clients[b].sent.len();
    let epoch_keys = net.clients[b].epoch_keys.clone();
    net.start(
        b,
        ROOM,
        0,
        SessionStart::ReturnCheck {
            epoch_keys,
            then_leave,
            heads: heads.to_vec(),
        },
    );
    net.run(2_000);
    let outcome = net.clients[b].events.iter().rev().find_map(|e| match e {
        SessionEvent::CheckResult(outcome) => Some(*outcome),
        _ => None,
    });
    assert!(net.clients[b].session().has_ended());
    (sent_frames(net, b, from), outcome)
}

/// B syncs again from its old cursor with the keyring and keys the check fetched.
fn sync_again(net: &mut Net, b: usize, cursor: u64) {
    let session = net.clients[b].session();
    let start = SessionStart::Member {
        keyring: session.keyring_entries(),
        epoch_keys: session.epoch_keys(),
    };
    net.start(b, ROOM, cursor, start);
}

/// Whether any client sent a `RewrapEpoch` of `epoch` for device `to`.
fn rewrapped(net: &Net, epoch: u32, to: u8) -> bool {
    net.clients.iter().any(|client| {
        client.sent.iter().any(|m| match ClientFrame::decode(m) {
            Ok(ClientFrame::Keyring { entry }) => matches!(
                Entry::decode(&entry).unwrap().payload,
                Payload::RewrapEpoch { epoch: e, member: m, .. } if e == epoch && m == member(to)
            ),
            _ => false,
        })
    })
}

/// Criteria 18 to 20, 22 and 26, "From now on": B gets the current key only, nobody repairs
/// epoch 1 for B, and B still converges from the fresh snapshot.
#[test]
fn from_now_on_withholds_the_epochs_of_the_time_away() {
    let (mut net, clients) = three(3);
    let [a, b, c] = clients;
    let (cursor, heads) = away(&mut net, clients);
    act(&mut net, a, |s| s.add_back(member(2), History::FromNow));
    net.run(1_000);
    assert_eq!(role_seen_by(&net, c, 2), Some(Role::Member));
    let (frames, outcome) = check(&mut net, b, false, &heads);
    assert_eq!(outcome, Some(CheckOutcome::AddedBack));
    assert!(
        frames.iter().all(|f| matches!(
            f,
            ClientFrame::Hello {
                since_seq: u64::MAX,
                ..
            } | ClientFrame::Proof { .. }
                | ClientFrame::Ping
        )),
        "a check sends nothing else: {frames:?}"
    );
    sync_again(&mut net, b, cursor);
    let done = net.run_until(10_000, |n| {
        n.clients[b].saw(&SessionEvent::Online) && n.converged(&[a, b, c])
    });
    assert!(done.is_some(), "B converges: {:?}", net.clients[b].events);
    let epochs: Vec<u32> = net.clients[b]
        .session()
        .epoch_keys()
        .iter()
        .map(|(n, _)| *n)
        .collect();
    assert!(epochs.contains(&2) && epochs.contains(&0), "{epochs:?}");
    assert!(!epochs.contains(&1), "epoch 1 is withheld: {epochs:?}");
    net.run(2_000);
    assert!(!rewrapped(&net, 1, 2), "no member repairs epoch 1 for B");
    let keyring = net.clients[c].session().keyring().unwrap();
    let withheld = keyring
        .epochs()
        .into_iter()
        .find(|e| e.number == 1)
        .unwrap();
    assert!(!keyring.holds(&member(2), withheld));
}

/// Criteria 18 and 22, "Full history": B gets every epoch A holds, epoch 1 included.
#[test]
fn full_history_gives_every_epoch_the_admin_holds() {
    let (mut net, clients) = three(4);
    let [a, b, c] = clients;
    let (cursor, heads) = away(&mut net, clients);
    act(&mut net, a, |s| s.add_back(member(2), History::Full));
    net.run(1_000);
    let (_, outcome) = check(&mut net, b, false, &heads);
    assert_eq!(outcome, Some(CheckOutcome::AddedBack));
    sync_again(&mut net, b, cursor);
    let done = net.run_until(10_000, |n| {
        n.clients[b].saw(&SessionEvent::Online) && n.converged(&[a, b, c])
    });
    assert!(done.is_some());
    let epochs: Vec<u32> = net.clients[b]
        .session()
        .epoch_keys()
        .iter()
        .map(|(n, _)| *n)
        .collect();
    assert!(epochs.contains(&1) && epochs.contains(&2), "{epochs:?}");
}

/// Criteria 17 and 18: Add back takes only a former participant and returns a member.
#[test]
fn add_back_takes_former_participants_only() {
    let (mut net, [a, _, _]) = three(5);
    assert_eq!(
        session(&mut net, a).add_back(member(2), History::Full),
        Err(SessionError::AlreadyMember)
    );
    assert_eq!(
        session(&mut net, a).add_back(member(9), History::Full),
        Err(SessionError::NotAMember)
    );
    act(&mut net, a, |s| s.remove(member(2)));
    net.run(1_000);
    let former = net.clients[a]
        .session()
        .keyring()
        .unwrap()
        .former_participants();
    assert_eq!(former.len(), 1);
    assert_eq!(former[0].member, member(2));
}

/// Criteria 20, 21, 25 and 33: the check answers "not yet" while B is removed; "Keep as it is"
/// after an add back sends exactly one own `RemoveMember` and the admins see B left.
#[test]
fn the_check_answers_and_keep_as_it_is_leaves_once() {
    let (mut net, [a, b, _]) = three(6);
    act(&mut net, b, ClientSession::leave);
    net.run(1_000);
    let heads = known_heads(&net, b);
    let (frames, outcome) = check(&mut net, b, false, &heads);
    assert_eq!(outcome, Some(CheckOutcome::NotYet));
    assert!(frames.iter().all(|f| matches!(
        f,
        ClientFrame::Hello { .. } | ClientFrame::Proof { .. } | ClientFrame::Ping
    )));
    act(&mut net, a, |s| s.add_back(member(2), History::Full));
    net.run(1_000);
    let (frames, outcome) = check(&mut net, b, true, &heads);
    assert_eq!(outcome, Some(CheckOutcome::LeftAgain));
    let entries: Vec<Entry> = frames
        .iter()
        .filter_map(|f| match f {
            ClientFrame::Keyring { entry } => Some(Entry::decode(entry).unwrap()),
            ClientFrame::Hello { .. } | ClientFrame::Proof { .. } | ClientFrame::Ping => None,
            other => panic!("Keep as it is sent {other:?}"),
        })
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].author, member(2));
    assert!(matches!(entries[0].payload, Payload::RemoveMember { member: m } if m == member(2)));
    let keyring = net.clients[a].session().keyring().unwrap();
    assert!(keyring.was_removed(&member(2)));
    assert!(left_by_own_hand(keyring, &member(2)));
    assert_eq!(keyring.rotation_pending(), vec![member(2)]);
}

/// ADR 0041 decision 6: a relay that serves a keyring of its own making, listing B, does not
/// make B's check say "added back": it holds no wrap of a key B held, so the answer is
/// "could not be checked" (ux-notes 7.1).
#[test]
fn a_keyring_the_relay_made_up_is_not_an_add_back() {
    let now = Millis(1_800_000_000_000);
    let kept = EpochKey::from_bytes([5; 32]);
    let mut b = ClientSession::new(
        SessionConfig {
            room: ROOM,
            identity: device(2),
            peer_id: 7,
            nonce_seed: seed(2, 1),
            entropy_seed: seed(2, 2),
            since_seq: 0,
            start: SessionStart::ReturnCheck {
                epoch_keys: vec![(0, kept)],
                then_leave: false,
                heads: Vec::new(),
            },
        },
        now,
    )
    .unwrap();
    b.drain();
    b.connected(now);
    let fake = device(9);
    let key = EpochKey::from_bytes([6; 32]);
    let wrap = |d: &curvyo_crypto_core::IdentityKeys| RecipientWrap {
        member: d.identity().member,
        wrap: wrap_epoch_key(&key, 0, doc_id(ROOM), &d.identity(), [1; 32]).unwrap(),
    };
    let genesis = Entry::sign(
        Vec::new(),
        &fake,
        Payload::Genesis {
            document_id: doc_id(ROOM),
            admins: vec![fake.identity(), device(2).identity()],
            epoch0_wraps: vec![wrap(&fake), wrap(&device(2))],
        },
    )
    .unwrap();
    for frame in [
        RelayFrame::Challenge { nonce: [3; 32] },
        RelayFrame::Welcome {
            head_seq: 0,
            snapshot_seq: 0,
            connection: ConnectionId(1),
        },
        RelayFrame::Keyring {
            entry: genesis.encode().unwrap(),
        },
        RelayFrame::Pong,
    ] {
        b.received(&frame.encode(), now);
    }
    let out = b.drain();
    assert!(out.contains(&ClientOutput::Event(SessionEvent::CheckResult(
        CheckOutcome::Unverified
    ))));
    assert!(b.has_ended());
}

/// ADR 0041 decision 6: a keyring that wraps B a key B held and lists B, but holds no admission
/// of B after the heads B's ended record kept (the room's keyring served cut off before B's
/// removal), is no add back: "could not be checked". So is a check without heads.
#[test]
fn a_keyring_cut_off_before_the_removal_is_not_an_add_back() {
    let now = Millis(1_800_000_000_000);
    let kept = EpochKey::from_bytes([5; 32]);
    let a = device(1);
    let genesis = Entry::sign(
        Vec::new(),
        &a,
        Payload::Genesis {
            document_id: doc_id(ROOM),
            admins: vec![a.identity(), device(2).identity()],
            epoch0_wraps: [&a, &device(2)]
                .iter()
                .map(|d| RecipientWrap {
                    member: d.identity().member,
                    wrap: wrap_epoch_key(&kept, 0, doc_id(ROOM), &d.identity(), [1; 32]).unwrap(),
                })
                .collect(),
        },
    )
    .unwrap();
    let removal = Entry::sign(
        vec![genesis.try_hash().unwrap()],
        &a,
        Payload::RemoveMember { member: member(2) },
    )
    .unwrap();
    for heads in [vec![removal.try_hash().unwrap()], Vec::new()] {
        let mut b = ClientSession::new(
            SessionConfig {
                room: ROOM,
                identity: device(2),
                peer_id: 7,
                nonce_seed: seed(2, 1),
                entropy_seed: seed(2, 2),
                since_seq: 0,
                start: SessionStart::ReturnCheck {
                    epoch_keys: vec![(0, EpochKey::from_bytes([5; 32]))],
                    then_leave: false,
                    heads,
                },
            },
            now,
        )
        .unwrap();
        b.drain();
        b.connected(now);
        for frame in [
            RelayFrame::Challenge { nonce: [3; 32] },
            RelayFrame::Welcome {
                head_seq: 0,
                snapshot_seq: 0,
                connection: ConnectionId(1),
            },
            RelayFrame::Keyring {
                entry: genesis.encode().unwrap(),
            },
            RelayFrame::Pong,
        ] {
            b.received(&frame.encode(), now);
        }
        let out = b.drain();
        assert!(out.contains(&ClientOutput::Event(SessionEvent::CheckResult(
            CheckOutcome::Unverified
        ))));
    }
}

/// UX notes 7.1 row 5: a relay that holds no such room answers "not a member"; the check says
/// there is nothing to find, and sends one ping only.
#[test]
fn a_room_the_relay_does_not_hold_is_nothing_to_find() {
    let mut net = Net::new(11);
    let b = net.add_client(2);
    net.start(
        b,
        ROOM,
        0,
        SessionStart::ReturnCheck {
            epoch_keys: Vec::new(),
            then_leave: false,
            heads: Vec::new(),
        },
    );
    net.run(2_000);
    assert!(net.clients[b].saw(&SessionEvent::CheckResult(CheckOutcome::NotFound)));
    let pings = sent_frames(&net, b, 0)
        .iter()
        .filter(|f| matches!(f, ClientFrame::Ping))
        .count();
    assert_eq!(pings, 1);
}
