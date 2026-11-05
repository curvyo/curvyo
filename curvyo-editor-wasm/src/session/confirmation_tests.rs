//! Own keyring acts in the editing session wait for the relay's confirmation (ADR 0041 decision
//! 11, `specs/0097-membership-lifecycle` criteria 10, 20, 26 and 29a): the rows say so, a leave
//! keeps the file shared until it is confirmed, "was added back" waits for the relay.

use curvyo_crypto_core::keyring::Keyring;
use curvyo_document_core::EndReason;
use serde_json::Value;

use super::Session;
use super::sync_test_support::{Device, Net, seeds, shared_pair, view};

/// A, B and C, converged.
fn shared_three() -> (Net, [usize; 3]) {
    let (mut net, a, b) = shared_pair();
    let now = net.now;
    let (link, _) = net.s(a).create_invite(3_600, now).unwrap();
    net.pump();
    let c = net.add(Session::new(0xc3));
    net.s(c)
        .join_document(&link, "Cleo", &seeds(3, 0), now)
        .unwrap();
    net.pump();
    net.run(1_000);
    (net, [a, b, c])
}

fn row(session: &Session, label: &str) -> Value {
    view(session)["participants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["label"] == label)
        .cloned()
        .unwrap_or(Value::Null)
}

fn notices(session: &mut Session) -> Vec<String> {
    std::iter::from_fn(|| session.take_sync_notice().map(|(t, _)| t)).collect()
}

/// Criteria 10 and 29a: a leave written into a dead link shows "Leaving" on the own row, keeps
/// the file shared with the leave in its keyring log, and ends as "left" once the link is back;
/// the ended record's heads are entries the room holds.
#[test]
fn a_leave_stays_pending_and_shared_until_confirmed() {
    let (mut net, [a, b, _]) = shared_three();
    net.set_link(b, false);
    net.s(b).leave_share("Poster").unwrap();
    net.run(500);
    assert_eq!(row(&net.devices[b].session, "Ben")["pending"], "leaving");
    assert!(net.devices[b].session.is_shared());
    let saved = curvyo_document_core::unpack(9, &net.s(b).pack_for_save().unwrap()).unwrap();
    assert!(
        saved.share().is_some(),
        "shared on disk until the leave is confirmed"
    );
    assert!(saved.ended_share().is_none());
    net.set_link(b, true);
    net.run(15_000);
    let ended = net.devices[b].session.document.ended_share().unwrap();
    assert_eq!(ended.ended, EndReason::Left);
    assert!(
        notices(net.s(b)).contains(&"You left Poster. Your copy stays on this device.".to_owned())
    );
    assert_eq!(row(&net.devices[a].session, "Ben"), Value::Null);
    let link = net.devices[a].session.sync.as_ref().unwrap();
    let keyring: &Keyring = link.client.keyring().unwrap();
    for head in &ended.heads {
        assert!(keyring.is_accepted(&curvyo_crypto_core::keyring::EntryHash(*head)));
    }
}

/// Criterion 10: a leave not sent before the app closed goes out on the next open, which shows
/// "Leaving" until it is confirmed and then ends as "left".
#[test]
fn a_leave_saved_unsent_ends_on_the_next_open() {
    let (mut net, [_a, b, _]) = shared_three();
    let (_, keys) = net.s(b).take_key_update().unwrap();
    net.set_link(b, false);
    net.s(b).leave_share("Poster").unwrap();
    let bytes = net.s(b).pack_for_save().unwrap();
    let mut reopened = Session::open(0xb4, &bytes).unwrap();
    let now = net.now;
    assert!(reopened.resume_shared("Ben", keys, &seeds(2, 2), now));
    assert_eq!(row(&reopened, "Ben")["pending"], "leaving");
    net.devices[b] = Device {
        session: reopened,
        conn: None,
        link_up: true,
        connects: 0,
    };
    net.pump();
    net.run(2_000);
    let ended = net.devices[b].session.document.ended_share().unwrap();
    assert_eq!(ended.ended, EndReason::Left);
}

/// Criteria 20, 26 and 29a: an add back written into a dead link stays in Removed or left as
/// "adding" and says nothing; once the relay holds it the row moves to Participants and "Ben
/// was added back." shows.
#[test]
fn an_add_back_is_announced_once_the_relay_holds_it() {
    let (mut net, [a, _b, _c]) = shared_three();
    let ben = row(&net.devices[a].session, "Ben")["member"]
        .as_str()
        .unwrap()
        .to_owned();
    net.s(a).remove_participant(&ben).unwrap();
    net.run(1_000);
    let _ = notices(net.s(a));
    net.set_link(a, false);
    net.s(a).add_back_participant(&ben, true).unwrap();
    net.run(500);
    let former = view(&net.devices[a].session)["former"].clone();
    assert_eq!(former[0]["member"], ben.as_str(), "{former}");
    assert_eq!(former[0]["adding"], true);
    assert_eq!(row(&net.devices[a].session, "Ben"), Value::Null);
    assert!(!notices(net.s(a)).iter().any(|n| n.contains("added back")));
    net.set_link(a, true);
    net.run(15_000);
    assert_eq!(
        view(&net.devices[a].session)["former"],
        Value::Array(Vec::new())
    );
    assert_eq!(row(&net.devices[a].session, "Ben")["pending"], Value::Null);
    assert!(
        notices(net.s(a)).contains(&"Ben was added back.".to_owned()),
        "the success line after the confirmation"
    );
}

/// Criterion 29a: a removal and a role change into a dead link keep their rows busy, with no
/// further act offered, until the relay confirms them.
#[test]
fn a_removal_and_a_role_change_stay_pending_until_confirmed() {
    let (mut net, [a, _b, c]) = shared_three();
    let ben = row(&net.devices[a].session, "Ben")["member"]
        .as_str()
        .unwrap()
        .to_owned();
    let cleo = row(&net.devices[a].session, "Cleo")["member"]
        .as_str()
        .unwrap()
        .to_owned();
    net.set_link(a, false);
    net.s(a).set_participant_role(&ben, true).unwrap();
    net.s(a).remove_participant(&cleo).unwrap();
    net.run(500);
    let ben_row = row(&net.devices[a].session, "Ben");
    assert_eq!(ben_row["pending"], "making-admin");
    assert_eq!(ben_row["removable"], false);
    let cleo_row = row(&net.devices[a].session, "Cleo");
    assert_eq!(cleo_row["pending"], "removing");
    assert_eq!(cleo_row["removable"], false);
    net.set_link(a, true);
    net.run(15_000);
    assert_eq!(row(&net.devices[a].session, "Ben")["pending"], Value::Null);
    assert_eq!(row(&net.devices[a].session, "Ben")["admin"], true);
    assert_eq!(row(&net.devices[a].session, "Cleo"), Value::Null);
    assert!(net.devices[c].session.document.ended_share().is_some());
}
