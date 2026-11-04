//! The relay's pairing slots (ADR 0040 decision 3, `specs/0096-own-devices-sync` criteria 1, 4,
//! 5 and 30): a free random slot per opener, forwarding between the opener and the one current
//! joiner, the refusals `unknown`, `expired`, `used`, `locked` and `busy`, the limits, and
//! nothing stored.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;

use curvyo_sync_core::{
    ClientFrame, ConnectionId, Millis, PAIR_JOINS_PER_SLOT, PAIR_MESSAGE_MAX,
    PAIR_SLOT_LIFETIME_MS, PAIR_SLOTS_PER_SERVER, PairCloseReason, PairSlots, RefusalReason,
    RelayFrame, RelayOutput,
};

const NOW: Millis = Millis(1_800_000_000_000);
const OPENER: ConnectionId = ConnectionId(1);
const JOINER: ConnectionId = ConnectionId(2);
const OTHER: ConnectionId = ConnectionId(3);

fn entropy(n: u64) -> [u8; 32] {
    let mut out = [0; 32];
    out[..8].copy_from_slice(&n.to_le_bytes());
    out
}

fn sends(out: &[RelayOutput], to: ConnectionId) -> Vec<RelayFrame> {
    out.iter()
        .filter_map(|output| match output {
            RelayOutput::Send { connection, frame } if *connection == to => Some(frame.clone()),
            _ => None,
        })
        .collect()
}

fn closed(out: &[RelayOutput], which: ConnectionId) -> bool {
    out.iter()
        .any(|output| matches!(output, RelayOutput::Close { connection } if *connection == which))
}

fn no_store(out: &[RelayOutput]) {
    assert!(
        !out.iter()
            .any(|output| matches!(output, RelayOutput::Store(_))),
        "a pairing output stores something: {out:?}"
    );
}

fn send(
    slots: &mut PairSlots,
    from: ConnectionId,
    frame: &ClientFrame,
    now: Millis,
) -> Vec<RelayOutput> {
    let out = slots.receive(from, &frame.encode(), now, entropy(from.0));
    no_store(&out);
    out
}

/// Opens a slot on `OPENER` and returns its number.
fn open(slots: &mut PairSlots) -> u16 {
    let out = send(slots, OPENER, &ClientFrame::PairOpen, NOW);
    match sends(&out, OPENER).as_slice() {
        [RelayFrame::PairSlot { slot, expires_in_s }] => {
            assert_eq!(u64::from(*expires_in_s) * 1000, PAIR_SLOT_LIFETIME_MS);
            assert!(*slot < 1 << 15);
            *slot
        }
        other => panic!("no slot: {other:?}"),
    }
}

fn join(slots: &mut PairSlots, joiner: ConnectionId, slot: u16, now: Millis) -> Vec<RelayOutput> {
    send(slots, joiner, &ClientFrame::PairJoin { slot }, now)
}

fn refusal(out: &[RelayOutput], to: ConnectionId) -> Option<RefusalReason> {
    sends(out, to).into_iter().find_map(|frame| match frame {
        RelayFrame::Refused { reason } => Some(reason),
        _ => None,
    })
}

/// Criteria 1 and 2: the opener gets a slot, the joiner joins it, both learn the sid, and
/// messages pass both ways unchanged.
#[test]
fn opener_and_joiner_exchange_messages_through_a_slot() {
    let mut slots = PairSlots::new();
    let slot = open(&mut slots);
    let out = join(&mut slots, JOINER, slot, NOW);
    let [RelayFrame::PairJoined { sid }] = sends(&out, JOINER)[..] else {
        panic!("not joined: {out:?}");
    };
    assert_eq!(sends(&out, OPENER), vec![RelayFrame::PairJoined { sid }]);

    let out = send(
        &mut slots,
        JOINER,
        &ClientFrame::PairMsg {
            bytes: vec![1, 1, 7],
        },
        NOW,
    );
    assert_eq!(
        sends(&out, OPENER),
        vec![RelayFrame::PairMsg {
            bytes: vec![1, 1, 7]
        }]
    );
    let out = send(
        &mut slots,
        OPENER,
        &ClientFrame::PairMsg {
            bytes: vec![1, 3, 9],
        },
        NOW,
    );
    assert_eq!(
        sends(&out, JOINER),
        vec![RelayFrame::PairMsg {
            bytes: vec![1, 3, 9]
        }]
    );
}

/// Criterion 4: once the opener closes as done, the code answers `used` until its lifetime ends,
/// then `expired`, then `unknown`.
#[test]
fn a_used_code_is_refused_as_used_then_expired() {
    let mut slots = PairSlots::new();
    let slot = open(&mut slots);
    join(&mut slots, JOINER, slot, NOW);
    let out = send(
        &mut slots,
        OPENER,
        &ClientFrame::PairClose {
            reason: PairCloseReason::Done,
        },
        NOW,
    );
    assert_eq!(
        sends(&out, JOINER),
        vec![RelayFrame::PairClose {
            reason: PairCloseReason::Done
        }]
    );
    assert!(closed(&out, JOINER) && closed(&out, OPENER));

    let out = join(&mut slots, OTHER, slot, NOW.plus(1000));
    assert_eq!(refusal(&out, OTHER), Some(RefusalReason::Used));
    assert!(closed(&out, OTHER));
    let out = join(
        &mut slots,
        ConnectionId(4),
        slot,
        NOW.plus(PAIR_SLOT_LIFETIME_MS),
    );
    assert_eq!(refusal(&out, ConnectionId(4)), Some(RefusalReason::Expired));
    let out = join(
        &mut slots,
        ConnectionId(5),
        slot,
        NOW.plus(2 * PAIR_SLOT_LIFETIME_MS),
    );
    assert_eq!(refusal(&out, ConnectionId(5)), Some(RefusalReason::Unknown));
}

/// Criterion 4: after its lifetime the slot is refused as expired, and both parties of a slot
/// still open are told and closed when the relay's clock passes it.
#[test]
fn an_expired_code_is_refused_and_its_parties_closed() {
    let mut slots = PairSlots::new();
    let slot = open(&mut slots);
    join(&mut slots, JOINER, slot, NOW);
    let out = slots.expire(NOW.plus(PAIR_SLOT_LIFETIME_MS - 1));
    assert_eq!(out, Vec::new());
    let out = slots.expire(NOW.plus(PAIR_SLOT_LIFETIME_MS));
    no_store(&out);
    assert_eq!(refusal(&out, OPENER), Some(RefusalReason::Expired));
    assert_eq!(refusal(&out, JOINER), Some(RefusalReason::Expired));
    assert!(closed(&out, OPENER) && closed(&out, JOINER));
    let out = join(&mut slots, OTHER, slot, NOW.plus(PAIR_SLOT_LIFETIME_MS + 5));
    assert_eq!(refusal(&out, OTHER), Some(RefusalReason::Expired));
}

/// Criterion 4: a slot nobody opened is `unknown`.
#[test]
fn an_unknown_slot_is_refused() {
    let mut slots = PairSlots::new();
    let out = join(&mut slots, JOINER, 77, NOW);
    assert_eq!(refusal(&out, JOINER), Some(RefusalReason::Unknown));
    assert!(closed(&out, JOINER));
}

/// Criterion 4: a wrong code closes only the joiner; the slot stays open for the next try. The
/// opener's lock answers `locked`, and the relay locks a slot after 5 joins on its own.
#[test]
fn wrong_codes_and_locks() {
    let mut slots = PairSlots::new();
    let slot = open(&mut slots);
    join(&mut slots, JOINER, slot, NOW);
    let out = send(
        &mut slots,
        OPENER,
        &ClientFrame::PairClose {
            reason: PairCloseReason::Wrong,
        },
        NOW,
    );
    assert_eq!(
        sends(&out, JOINER),
        vec![RelayFrame::PairClose {
            reason: PairCloseReason::Wrong
        }]
    );
    assert!(closed(&out, JOINER) && !closed(&out, OPENER));
    let out = join(&mut slots, OTHER, slot, NOW);
    assert!(matches!(
        sends(&out, OTHER)[..],
        [RelayFrame::PairJoined { .. }]
    ));
    let out = send(
        &mut slots,
        OPENER,
        &ClientFrame::PairClose {
            reason: PairCloseReason::Locked,
        },
        NOW,
    );
    assert!(closed(&out, OTHER) && closed(&out, OPENER));
    let out = join(&mut slots, ConnectionId(9), slot, NOW);
    assert_eq!(refusal(&out, ConnectionId(9)), Some(RefusalReason::Locked));

    // The relay's own count: joiners that leave without a word.
    let mut slots = PairSlots::new();
    let slot = open(&mut slots);
    for n in 0..u64::from(PAIR_JOINS_PER_SLOT) {
        let joiner = ConnectionId(10 + n);
        let out = join(&mut slots, joiner, slot, NOW);
        assert!(matches!(
            sends(&out, joiner)[..],
            [RelayFrame::PairJoined { .. }]
        ));
        let out = slots.disconnect(joiner);
        assert_eq!(
            sends(&out, OPENER),
            vec![RelayFrame::PairClose {
                reason: PairCloseReason::Gone
            }]
        );
    }
    let out = join(&mut slots, ConnectionId(99), slot, NOW);
    assert_eq!(refusal(&out, ConnectionId(99)), Some(RefusalReason::Locked));
    assert!(closed(&out, OPENER));
}

/// A second joiner while one is current is refused as busy; the current pairing goes on.
#[test]
fn one_joiner_at_a_time() {
    let mut slots = PairSlots::new();
    let slot = open(&mut slots);
    join(&mut slots, JOINER, slot, NOW);
    let out = join(&mut slots, OTHER, slot, NOW);
    assert_eq!(refusal(&out, OTHER), Some(RefusalReason::Busy));
    assert!(sends(&out, OPENER).is_empty() && sends(&out, JOINER).is_empty());
    let out = send(
        &mut slots,
        JOINER,
        &ClientFrame::PairMsg { bytes: vec![5] },
        NOW,
    );
    assert_eq!(
        sends(&out, OPENER),
        vec![RelayFrame::PairMsg { bytes: vec![5] }]
    );
}

/// Cancel by the opener, or the opener leaving, tells the joiner and frees the slot.
#[test]
fn the_opener_cancelling_or_leaving_ends_the_slot() {
    let mut slots = PairSlots::new();
    let slot = open(&mut slots);
    join(&mut slots, JOINER, slot, NOW);
    let out = send(
        &mut slots,
        OPENER,
        &ClientFrame::PairClose {
            reason: PairCloseReason::Cancelled,
        },
        NOW,
    );
    assert_eq!(
        sends(&out, JOINER),
        vec![RelayFrame::PairClose {
            reason: PairCloseReason::Cancelled
        }]
    );
    assert!(closed(&out, JOINER) && closed(&out, OPENER));
    assert_eq!(slots.slot_count(), 0);

    let mut slots = PairSlots::new();
    let slot = open(&mut slots);
    join(&mut slots, JOINER, slot, NOW);
    let out = slots.disconnect(OPENER);
    assert_eq!(
        sends(&out, JOINER),
        vec![RelayFrame::PairClose {
            reason: PairCloseReason::Gone
        }]
    );
    assert!(closed(&out, JOINER));
    let out = join(&mut slots, OTHER, slot, NOW);
    assert_eq!(refusal(&out, OTHER), Some(RefusalReason::Unknown));
}

/// The joiner cancelling tells the opener; the slot stays open.
#[test]
fn the_joiner_cancelling_tells_the_opener() {
    let mut slots = PairSlots::new();
    let slot = open(&mut slots);
    join(&mut slots, JOINER, slot, NOW);
    let out = send(
        &mut slots,
        JOINER,
        &ClientFrame::PairClose {
            reason: PairCloseReason::Cancelled,
        },
        NOW,
    );
    assert_eq!(
        sends(&out, OPENER),
        vec![RelayFrame::PairClose {
            reason: PairCloseReason::Cancelled
        }]
    );
    assert!(closed(&out, JOINER) && !closed(&out, OPENER));
    assert_eq!(slots.slot_count(), 1);
}

/// One slot per connection; a session frame, a `Gone` from a client, an oversized pairing
/// message or a pairing frame out of place is a protocol error.
#[test]
fn protocol_errors() {
    let mut slots = PairSlots::new();
    open(&mut slots);
    let out = send(&mut slots, OPENER, &ClientFrame::PairOpen, NOW);
    assert_eq!(refusal(&out, OPENER), Some(RefusalReason::Protocol));
    assert_eq!(slots.slot_count(), 0);

    let mut slots = PairSlots::new();
    let out = send(&mut slots, JOINER, &ClientFrame::Ping, NOW);
    assert_eq!(refusal(&out, JOINER), Some(RefusalReason::Protocol));
    let out = send(
        &mut slots,
        JOINER,
        &ClientFrame::PairMsg { bytes: vec![1] },
        NOW,
    );
    assert_eq!(refusal(&out, JOINER), Some(RefusalReason::Protocol));

    let slot = open(&mut slots);
    join(&mut slots, JOINER, slot, NOW);
    let out = send(
        &mut slots,
        JOINER,
        &ClientFrame::PairMsg {
            bytes: vec![0; PAIR_MESSAGE_MAX + 1],
        },
        NOW,
    );
    assert_eq!(refusal(&out, JOINER), Some(RefusalReason::TooLarge));
    let out = join(&mut slots, OTHER, slot, NOW);
    let out2 = send(
        &mut slots,
        OTHER,
        &ClientFrame::PairClose {
            reason: PairCloseReason::Gone,
        },
        NOW,
    );
    assert!(matches!(
        sends(&out, OTHER)[..],
        [RelayFrame::PairJoined { .. }]
    ));
    assert_eq!(refusal(&out2, OTHER), Some(RefusalReason::Protocol));
    // A refused connection is forgotten: its next message reaches nobody.
    let out = send(
        &mut slots,
        OTHER,
        &ClientFrame::PairMsg { bytes: vec![1] },
        NOW,
    );
    assert_eq!(refusal(&out, OTHER), Some(RefusalReason::Protocol));
    assert_eq!(sends(&out, OPENER), Vec::new());
}

/// Slots are drawn at random below 2^15, distinct, and the server holds at most 4,096: the next
/// opener is refused as busy.
#[test]
fn slots_are_distinct_and_limited() {
    let mut slots = PairSlots::new();
    let mut seen = BTreeSet::new();
    let limit = u64::try_from(PAIR_SLOTS_PER_SERVER).unwrap();
    for n in 0..limit {
        let opener = ConnectionId(1000 + n);
        let out = slots.receive(opener, &ClientFrame::PairOpen.encode(), NOW, entropy(n % 7));
        let [RelayFrame::PairSlot { slot, .. }] = sends(&out, opener)[..] else {
            panic!("no slot for opener {n}");
        };
        assert!(slot < 1 << 15);
        assert!(seen.insert(slot), "slot {slot} given twice");
    }
    assert_eq!(slots.slot_count(), PAIR_SLOTS_PER_SERVER);
    let late = ConnectionId(1);
    let out = slots.receive(late, &ClientFrame::PairOpen.encode(), NOW, entropy(1));
    assert_eq!(refusal(&out, late), Some(RefusalReason::Busy));
    // After two lifetimes every slot is gone and the server takes openers again.
    slots.expire(NOW.plus(2 * PAIR_SLOT_LIFETIME_MS));
    assert_eq!(slots.slot_count(), 0);
    let out = slots.receive(late, &ClientFrame::PairOpen.encode(), NOW, entropy(1));
    assert!(matches!(
        sends(&out, late)[..],
        [RelayFrame::PairSlot { .. }]
    ));
}

/// Every join attempt on one slot gets its own sid from the entropy passed in with the join,
/// so no two `CPace` runs on a slot share a sid (ADR 0040 decision 3).
#[test]
fn every_join_gets_a_fresh_sid() {
    let mut slots = PairSlots::new();
    let slot = open(&mut slots);
    let mut sids = BTreeSet::new();
    for attempt in 0..3_u64 {
        let out = slots.receive(
            JOINER,
            &ClientFrame::PairJoin { slot }.encode(),
            NOW,
            entropy(100 + attempt),
        );
        let [RelayFrame::PairJoined { sid }] = sends(&out, JOINER)[..] else {
            panic!("not joined: {out:?}");
        };
        assert_eq!(sends(&out, OPENER), vec![RelayFrame::PairJoined { sid }]);
        assert!(sids.insert(sid), "a sid was used twice");
        send(
            &mut slots,
            OPENER,
            &ClientFrame::PairClose {
                reason: PairCloseReason::Wrong,
            },
            NOW,
        );
    }
}
