//! Take-in of a document shared on its own (`specs/0094-workspace` criteria 42 and 49): the
//! membership rule, and the `Moved` record that ends the document's own room for every member
//! and admits nobody new there.

use curvyo_crypto_core::IdentityKeys;

use super::Session;
use super::sync_test_support::{seeds, shared_pair, view};
use super::workspace_take_in::{TakeIn, take_in_check};

fn identity(device: u8) -> curvyo_crypto_core::Identity {
    IdentityKeys::from_entropy(&seeds(device, 0).identity).identity()
}

/// Criteria 42 and 49: a document whose own participants are all workspace members may be
/// taken in; one shared with someone outside is refused; an unshared one is plain.
#[test]
fn take_in_needs_every_own_participant_in_the_workspace() {
    let (mut net, a, _b) = shared_pair();
    let bytes = net.s(a).pack_for_save().unwrap();
    assert_eq!(
        take_in_check(&bytes, &[identity(1), identity(2), identity(3)]),
        TakeIn::Allowed { people: 2 }
    );
    assert_eq!(take_in_check(&bytes, &[identity(1)]), TakeIn::Refused);
    let plain = Session::new(5).pack_for_save().unwrap();
    assert_eq!(take_in_check(&plain, &[identity(1)]), TakeIn::Plain);
}

/// Criterion 49: once `Moved` is in the old room, its other member stops syncing it and says
/// where the document went; its copy is a local file. A third device with an older invite is
/// admitted by nobody.
#[test]
fn moved_ends_the_old_room_and_its_link_admits_nobody() {
    let (mut net, a, b) = shared_pair();
    let now = net.now;
    let (link, _) = net.s(a).create_invite(24 * 3600, now).unwrap();
    net.pump();
    assert!(net.s(a).push_moved([0x57; 16], [0x11; 16]));
    net.pump();
    net.run(500);
    assert!(net.s(a).moved_stored());
    // The take-in's short-lived session of the old room ends with it.
    net.devices[a].session = Session::new(0xa9);
    net.pump();
    assert_eq!(view(net.s(b))["ended"], "moved");
    assert!(
        net.s(b).document.share().is_none(),
        "B's copy is a local file"
    );
    let notice = net.s(b).take_sync_notice().unwrap().0;
    assert!(
        notice.contains("now part of a shared workspace"),
        "{notice}"
    );

    let c = net.add(Session::new(0xc3));
    let now = net.now;
    net.s(c)
        .join_document(&link, "Cleo", &seeds(3, 0), now)
        .unwrap();
    net.pump();
    net.run(5_000);
    assert_ne!(view(net.s(c))["join"]["phase"], "done", "nobody admits C");
}

/// Criterion 42 at Share workspace: a document shared on its own with this device alone is
/// taken in (its file loses its own room); one shared with someone else stays out (49).
#[test]
fn share_workspace_takes_in_a_document_shared_with_this_device_alone() {
    use super::sync_test_support::{Net, ROOM, SERVER};
    use crate::{NotAdded, WorkspaceShareBuilder};

    let mut net = Net::new();
    let a = net.add(Session::new(0xa1));
    let now = net.now;
    net.s(a)
        .share_document(SERVER, "Anna", ROOM, &seeds(1, 0), now)
        .unwrap();
    net.pump();
    net.run(200);
    let alone = net.s(a).pack_for_save().unwrap();
    let (mut pair, pa, _) = shared_pair();
    let with_ben = pair.s(pa).pack_for_save().unwrap();

    let path = |name: &str| vec![name.to_owned()];
    let mut builder = WorkspaceShareBuilder::new([0x57; 16], 1, 1_800_000_000, [3; 16]);
    builder.set_member(identity(1));
    builder
        .add_document(&path("Lid.curvyo"), &alone, [0x11; 16])
        .unwrap();
    assert_eq!(
        builder.add_document(&path("Base.curvyo"), &with_ben, [0x22; 16]),
        Err(NotAdded::SharedOnItsOwn)
    );
    assert_eq!(builder.taken_in(), &[(path("Lid.curvyo"), [0x11; 16])]);
    let (_, files) = builder.finish([9; 16]);
    let taken = curvyo_document_core::unpack(4, &files[0].1).unwrap();
    assert!(taken.share().is_none(), "no own room any more");
    assert_eq!(taken.workspace_share().unwrap().document, [0x11; 16]);
}
