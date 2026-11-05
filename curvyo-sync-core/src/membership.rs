//! The client's membership rules over a replayed keyring (ADR 0041 decisions 1, 4 and 9):
//! whether an act that removes or demotes devices still leaves an admin, and the notices the
//! keyring gives a participant.

use curvyo_crypto_core::MemberId;
use curvyo_crypto_core::keyring::{EntryHash, HeldBack, Keyring, Payload, Role};

/// Whether an admin remains outside `acted`, the devices an act removes or demotes: one device
/// for Step down, a demotion or the Leave of a document, every device of the maker's group for
/// the person leave of a synced workspace (LAST-ADMIN A). An admin that is one only because
/// replay held back its removal or demotion does not count: it is no successor in a frozen room
/// (ADR 0041 decision 1). An act on members only always leaves the admins as they are. The
/// client refuses the act when not; replay's "the admin set never becomes empty" stays the
/// backstop.
#[must_use]
pub fn admin_remains(keyring: &Keyring, acted: &[MemberId]) -> bool {
    let admins: Vec<MemberId> = keyring
        .participants()
        .iter()
        .filter(|p| p.role == Role::Admin)
        .map(|p| p.identity.member)
        .collect();
    if !admins.iter().any(|m| acted.contains(m)) {
        return true;
    }
    let held = keyring.held_back();
    admins
        .iter()
        .any(|m| !acted.contains(m) && !held.iter().any(|h| h.subject == *m))
}

/// One line the Share tab shows for a room (ADR 0041 decision 9); `curvyo-ui-core` words it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MembershipNotice {
    /// "B is now an admin".
    NowAdmin(MemberId),
    /// "B is no longer an admin".
    NoLongerAdmin(MemberId),
    /// An admin left by its own `RemoveMember`: "B left".
    AdminLeft(MemberId),
    /// An admin was removed by another admin.
    AdminRemoved(MemberId),
    /// A former participant still holds the current epoch key (criterion 11): "B left. The
    /// keys have not been changed yet." (`left`) or the same after a removal without rotation.
    RotationPending {
        /// The former participant.
        member: MemberId,
        /// It left by its own `RemoveMember`.
        left: bool,
    },
    /// Admins removed or demoted each other while they could not see each other and replay kept
    /// them all (criterion 27): "A and C removed each other at the same time. Nobody was
    /// removed." Acknowledged per computer by `entries`.
    MutualRemoval {
        /// The admins involved, ordered by member id.
        admins: Vec<MemberId>,
        /// The held-back entries, in causal order.
        entries: Vec<EntryHash>,
    },
    /// A removal or demotion replay held back that is not part of a mutual removal, such as a
    /// later one between the same admins or a sole admin's own (criterion 28): "Removal of C was
    /// not applied: you are the only admins".
    NotApplied(HeldBack),
    /// Exactly one admin device (criterion 7): "Make a second person an admin".
    OneAdmin,
    /// Two or more admins, every one a device of this maker's group (criterion 8): "All admins
    /// are your own computers".
    OnlyOwnAdmins,
}

/// Whether `member`'s latest removal in `keyring` was its own `RemoveMember` (a leave).
#[must_use]
pub fn left_by_own_hand(keyring: &Keyring, member: &MemberId) -> bool {
    keyring
        .accepted_entries()
        .iter()
        .rev()
        .find_map(|entry| match &entry.payload {
            Payload::RemoveMember { member: removed } if removed == member => {
                Some(entry.author == *member)
            }
            _ => None,
        })
        .unwrap_or(false)
}

/// The admin devices of `keyring`, ordered by member id: what a device keeps per room as the
/// last seen admin set.
#[must_use]
pub fn admin_set(keyring: &Keyring) -> Vec<MemberId> {
    let mut admins: Vec<MemberId> = keyring
        .participants()
        .iter()
        .filter(|p| p.role == Role::Admin)
        .map(|p| p.identity.member)
        .collect();
    admins.sort();
    admins
}

/// The notices of one room (ADR 0041 decision 9): the changes of its admin set since
/// `previous` (the set this device last showed; `None` the first time, which shows none), the
/// former participants that still hold the current key, held-back denials, and the second-admin
/// line counted against `own_group` (the devices of this maker's group, this device included).
#[must_use]
pub fn membership_notices(
    previous: Option<&[MemberId]>,
    keyring: &Keyring,
    own_group: &[MemberId],
) -> Vec<MembershipNotice> {
    let mut notices = Vec::new();
    let admins = admin_set(keyring);
    if let Some(previous) = previous {
        for member in previous.iter().filter(|m| !admins.contains(m)) {
            notices.push(match keyring.role(member) {
                Some(_) => MembershipNotice::NoLongerAdmin(*member),
                None if left_by_own_hand(keyring, member) => MembershipNotice::AdminLeft(*member),
                None => MembershipNotice::AdminRemoved(*member),
            });
        }
        for member in admins.iter().filter(|m| !previous.contains(m)) {
            notices.push(MembershipNotice::NowAdmin(*member));
        }
    }
    for member in keyring.rotation_pending() {
        notices.push(MembershipNotice::RotationPending {
            member,
            left: left_by_own_hand(keyring, &member),
        });
    }
    held_back_notices(&keyring.held_back(), &mut notices);
    if admins.len() == 1 {
        notices.push(MembershipNotice::OneAdmin);
    } else if admins.len() > 1 && admins.iter().all(|a| own_group.contains(a)) {
        notices.push(MembershipNotice::OnlyOwnAdmins);
    }
    notices
}

/// A mutual removal is the first held-back denial in each direction between two admins that
/// denied each other (ADR 0041 decision 4: told apart from the refused linear act by authors
/// and subjects); every other held-back denial, a later one between them too, is not applied.
fn held_back_notices(held: &[HeldBack], notices: &mut Vec<MembershipNotice>) {
    let mut admins = Vec::new();
    let mut entries = Vec::new();
    let mut seen = Vec::new();
    let mut not_applied = Vec::new();
    for h in held {
        let direction = (h.author, h.subject);
        let first = !seen.contains(&direction);
        seen.push(direction);
        let answered = held
            .iter()
            .any(|o| o.author == h.subject && o.subject == h.author);
        if h.author != h.subject && first && answered {
            for member in [h.author, h.subject] {
                if !admins.contains(&member) {
                    admins.push(member);
                }
            }
            entries.push(h.entry);
        } else {
            not_applied.push(MembershipNotice::NotApplied(*h));
        }
    }
    if !entries.is_empty() {
        admins.sort();
        notices.push(MembershipNotice::MutualRemoval { admins, entries });
    }
    notices.extend(not_applied);
}

#[cfg(test)]
mod tests {
    use curvyo_crypto_core::keyring::{Entry, EpochWrap};
    use curvyo_crypto_core::{DocumentId, EpochKey, IdentityKeys, wrap_epoch_key};

    use super::*;

    const DOC: DocumentId = DocumentId([0x97; 16]);

    fn device(n: u8) -> IdentityKeys {
        IdentityKeys::from_entropy(&[n; 64])
    }

    fn member(d: &IdentityKeys) -> MemberId {
        d.identity().member
    }

    /// A keyring with `admins` (the first writes the genesis) and `members`.
    fn keyring(admins: &[&IdentityKeys], members: &[&IdentityKeys]) -> Keyring {
        Keyring::replay(&entries(admins, members)).unwrap()
    }

    /// `entry` signed by `author` on `parents`.
    fn on(parents: Vec<EntryHash>, author: &IdentityKeys, payload: Payload) -> Entry {
        Entry::sign(parents, author, payload).unwrap()
    }

    /// `payload` by `author` on the heads of `entries`.
    fn push(entries: &mut Vec<Entry>, author: &IdentityKeys, payload: Payload) {
        let heads = Keyring::replay(entries).unwrap().heads();
        entries.push(on(heads, author, payload));
    }

    /// The entries of [`keyring`].
    fn entries(admins: &[&IdentityKeys], members: &[&IdentityKeys]) -> Vec<Entry> {
        let key = EpochKey::from_bytes([7; 32]);
        let first = admins[0];
        let wrap = |d: &IdentityKeys| wrap_epoch_key(&key, 0, DOC, &d.identity(), [9; 32]).unwrap();
        let mut entries = vec![
            Entry::sign(
                Vec::new(),
                first,
                Payload::Genesis {
                    document_id: DOC,
                    admins: vec![first.identity()],
                    epoch0_wraps: vec![curvyo_crypto_core::keyring::RecipientWrap {
                        member: member(first),
                        wrap: wrap(first),
                    }],
                },
            )
            .unwrap(),
        ];
        let others = admins[1..]
            .iter()
            .map(|d| (*d, Role::Admin))
            .chain(members.iter().map(|d| (*d, Role::Member)));
        for (d, role) in others {
            let parents = vec![entries.last().unwrap().try_hash().unwrap()];
            let payload = Payload::AddMember {
                identity: d.identity(),
                role,
                wraps: vec![EpochWrap {
                    epoch: 0,
                    wrap: wrap(d),
                }],
            };
            entries.push(Entry::sign(parents, first, payload).unwrap());
        }
        entries
    }

    #[test]
    fn the_only_admin_cannot_step_down_or_leave() {
        let (a, b) = (device(1), device(2));
        let keyring = keyring(&[&a], &[&b]);
        assert!(!admin_remains(&keyring, &[member(&a)]));
        assert!(
            admin_remains(&keyring, &[member(&b)]),
            "a member's act leaves the admin"
        );
        assert!(admin_remains(&keyring, &[]));
    }

    #[test]
    fn a_second_admin_is_a_successor_unless_it_acts_too() {
        let (a, a2, b) = (device(1), device(2), device(3));
        let keyring = keyring(&[&a, &a2], &[&b]);
        assert!(admin_remains(&keyring, &[member(&a)]));
        assert!(admin_remains(&keyring, &[member(&a2)]));
        // The person leave of a synced workspace acts on every device of the group.
        assert!(!admin_remains(&keyring, &[member(&a), member(&a2)]));
        assert!(admin_remains(&keyring, &[member(&a), member(&b)]));
    }

    /// ADR 0041 decision 1: A removes C while C removes A, so replay keeps both; neither is a
    /// successor for the other's leave or Step down, which replay would hold back as well. A
    /// member's leave is not refused.
    #[test]
    fn an_admin_kept_only_by_a_held_back_denial_is_no_successor() {
        let (a, b, c) = (device(1), device(2), device(3));
        let mut log = entries(&[&a, &c], &[&b]);
        let heads = Keyring::replay(&log).unwrap().heads();
        log.push(on(heads.clone(), &a, remove(&c)));
        log.push(on(heads, &c, remove(&a)));
        let keyring = Keyring::replay(&log).unwrap();
        assert!(!admin_remains(&keyring, &[member(&c)]));
        assert!(!admin_remains(&keyring, &[member(&a)]));
        assert!(admin_remains(&keyring, &[member(&b)]));
    }

    fn remove(who: &IdentityKeys) -> Payload {
        Payload::RemoveMember {
            member: member(who),
        }
    }

    /// Criterion 30: the admin set's changes against the last seen one; none the first time.
    #[test]
    fn admin_set_changes_are_notices() {
        let (a, b, c, d) = (device(1), device(2), device(3), device(4));
        let mut log = entries(&[&a, &c, &d], &[&b]);
        let before = admin_set(&Keyring::replay(&log).unwrap());
        push(
            &mut log,
            &a,
            Payload::SetRole {
                member: member(&b),
                role: Role::Admin,
            },
        );
        push(
            &mut log,
            &a,
            Payload::SetRole {
                member: member(&c),
                role: Role::Member,
            },
        );
        push(&mut log, &d, remove(&d));
        let keyring = Keyring::replay(&log).unwrap();
        let notices = membership_notices(Some(&before), &keyring, &[member(&a)]);
        assert!(notices.contains(&MembershipNotice::NowAdmin(member(&b))));
        assert!(notices.contains(&MembershipNotice::NoLongerAdmin(member(&c))));
        assert!(notices.contains(&MembershipNotice::AdminLeft(member(&d))));
        assert!(notices.contains(&MembershipNotice::RotationPending {
            member: member(&d),
            left: true,
        }));
        let first = membership_notices(None, &keyring, &[member(&a)]);
        assert!(
            !first
                .iter()
                .any(|n| matches!(n, MembershipNotice::NowAdmin(_)))
        );
        push(&mut log, &a, remove(&b));
        let keyring = Keyring::replay(&log).unwrap();
        let notices = membership_notices(
            Some(&admin_set(&Keyring::replay(&log[..log.len() - 1]).unwrap())),
            &keyring,
            &[],
        );
        assert!(notices.contains(&MembershipNotice::AdminRemoved(member(&b))));
        assert!(notices.contains(&MembershipNotice::RotationPending {
            member: member(&b),
            left: false,
        }));
    }

    /// Criteria 27 and 28: two admins who remove each other at once are one mutual removal; a
    /// later removal between them is not applied, as is a sole admin's own removal.
    #[test]
    fn mutual_and_linear_held_back_denials_are_told_apart() {
        let (a, b, c) = (device(1), device(2), device(3));
        let mut log = entries(&[&a, &c], &[&b]);
        let heads = Keyring::replay(&log).unwrap().heads();
        log.push(on(heads.clone(), &a, remove(&c)));
        log.push(on(heads, &c, remove(&a)));
        let keyring = Keyring::replay(&log).unwrap();
        assert_eq!(keyring.held_back().len(), 2);
        let notices = membership_notices(None, &keyring, &[member(&a)]);
        let mut both = vec![member(&a), member(&c)];
        both.sort();
        assert!(matches!(
            &notices[..],
            [MembershipNotice::MutualRemoval { admins, entries }, ..]
                if *admins == both && entries.len() == 2
        ));
        push(&mut log, &a, remove(&c));
        let keyring = Keyring::replay(&log).unwrap();
        let notices = membership_notices(None, &keyring, &[member(&a)]);
        assert!(notices.iter().any(|n| matches!(
            n,
            MembershipNotice::NotApplied(h) if h.author == member(&a) && h.subject == member(&c)
        )));

        let mut linear = entries(&[&a], &[&b]);
        push(&mut linear, &a, remove(&a));
        let keyring = Keyring::replay(&linear).unwrap();
        let notices = membership_notices(None, &keyring, &[]);
        assert!(notices.iter().any(|n| matches!(
            n,
            MembershipNotice::NotApplied(h) if h.author == member(&a) && h.subject == member(&a)
        )));
        assert!(
            !notices
                .iter()
                .any(|n| matches!(n, MembershipNotice::MutualRemoval { .. }))
        );
    }

    /// Criteria 7 and 8: one admin device, or admins that are all the maker's own devices.
    #[test]
    fn the_second_admin_line_has_two_tiers() {
        let (a, a2, b) = (device(1), device(2), device(3));
        let one = keyring(&[&a], &[&b]);
        assert!(
            membership_notices(None, &one, &[member(&a)]).contains(&MembershipNotice::OneAdmin)
        );
        let own = keyring(&[&a, &a2], &[&b]);
        let group = [member(&a), member(&a2)];
        let notices = membership_notices(None, &own, &group);
        assert!(notices.contains(&MembershipNotice::OnlyOwnAdmins));
        assert!(!notices.contains(&MembershipNotice::OneAdmin));
        // B sees two admins of another maker: no line.
        assert_eq!(membership_notices(None, &own, &[member(&b)]), Vec::new());
        let mixed = keyring(&[&a, &b], &[]);
        assert_eq!(membership_notices(None, &mixed, &[member(&a)]), Vec::new());
    }
}
