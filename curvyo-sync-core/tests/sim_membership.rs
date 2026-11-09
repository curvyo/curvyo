//! The membership lifecycle through the simulator with real document replicas
//! (`specs/0097-membership-lifecycle` criteria 2 to 5, 10 to 14, 18 to 22, 25 and 26; ADR 0041
//! decisions 1, 2, 5; ADR 0044 decision 4): roles, leave, and add back with every held epoch.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use curvyo_crypto_core::keyring::Role;
use curvyo_document_core::{Document, Length, NodeId, Point, RectBounds};
use curvyo_sync_core::{ClientSession, SessionError, SessionEvent, SessionStart, left_by_own_hand};
use support::sim::{Net, shared};
use support::{ROOM, member};

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
/// with C editing under both; returns B's cursor when it was removed.
fn away(net: &mut Net, [a, b, c]: [usize; 3]) -> u64 {
    act(net, a, |s| s.remove(member(2)));
    net.run(1_000);
    assert!(net.clients[b].saw(&SessionEvent::Removed));
    let cursor = net.clients[b].session().cursor();
    net.edit(c, |d| rect(d, 10.0));
    net.run(500);
    act(net, a, ClientSession::rotate);
    net.run(1_000);
    net.edit(c, |d| rect(d, 20.0));
    net.run(500);
    let keyring = net.clients[a].session().keyring().unwrap();
    assert_eq!(keyring.current_epoch().number, 2);
    cursor
}

/// B syncs again from its old cursor with the keyring and keys it kept.
fn sync_again(net: &mut Net, b: usize, cursor: u64) {
    let session = net.clients[b].session();
    let start = SessionStart::Member {
        keyring: session.keyring_entries(),
        epoch_keys: session.epoch_keys(),
    };
    net.start(b, ROOM, cursor, start);
}

/// Criteria 18 and 22 and ADR 0044 decision 4: an add back gives B every epoch A holds, epoch 1
/// included, and B converges.
#[test]
fn an_add_back_gives_every_epoch_the_admin_holds() {
    let (mut net, clients) = three(4);
    let [a, b, c] = clients;
    let cursor = away(&mut net, clients);
    act(&mut net, a, |s| s.add_back(member(2)));
    net.run(1_000);
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
        session(&mut net, a).add_back(member(2)),
        Err(SessionError::AlreadyMember)
    );
    assert_eq!(
        session(&mut net, a).add_back(member(9)),
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
