//! The keyring state of a set of valid entries: members and roles under the fork rules of
//! ADR 0010 §5, epochs and who holds them, and invites.

use std::collections::{BTreeMap, BTreeSet};

use crate::identity::{Identity, MemberId};
use crate::invite::InviteId;
use crate::keyring::graph::Graph;
use crate::keyring::{EntryHash, Payload, Role};
use crate::wrap::Wrap;
use crate::{DocumentId, KeyId};

/// One epoch key, named by its number and `key_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EpochRef {
    /// The epoch number.
    pub number: u32,
    /// The id of its key.
    pub key_id: KeyId,
}

/// How a member came in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admission {
    Direct,
    Claimed(InviteId),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MemberState {
    pub(crate) identity: Identity,
    pub(crate) role: Role,
    pub(crate) admission: Admission,
}

#[derive(Debug, Clone)]
pub(crate) struct EpochState {
    pub(crate) epoch: EpochRef,
    pub(crate) entry: EntryHash,
    /// Every wrap of this key, in replay order (`NewEpoch` or `Genesis`, `AddMember`, `RewrapEpoch`).
    pub(crate) wraps: Vec<(MemberId, Wrap)>,
}

impl EpochState {
    pub(crate) fn holds(&self, member: &MemberId) -> bool {
        self.wraps.iter().any(|(m, _)| m == member)
    }
}

/// An invite as the keyring sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InviteState {
    /// The role its claimant gets.
    pub role: Role,
    /// Its expiry in seconds since the Unix epoch.
    pub expires: u64,
    /// The member whose claim won, if any.
    pub claimed_by: Option<MemberId>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    pub(crate) document_id: Option<DocumentId>,
    pub(crate) members: BTreeMap<MemberId, MemberState>,
    pub(crate) ever_added: BTreeSet<MemberId>,
    pub(crate) epochs: Vec<EpochState>,
    pub(crate) current: Option<usize>,
    pub(crate) invites: BTreeMap<InviteId, InviteState>,
    pub(crate) admin_set_restored: bool,
}

impl State {
    pub(crate) fn role(&self, member: &MemberId) -> Option<Role> {
        self.members.get(member).map(|m| m.role)
    }

    pub(crate) fn is_admin(&self, member: &MemberId) -> bool {
        self.role(member) == Some(Role::Admin)
    }

    pub(crate) fn epoch(&self, epoch: EpochRef) -> Option<&EpochState> {
        self.epochs.iter().find(|e| e.epoch == epoch)
    }

    pub(crate) fn highest_number(&self) -> u32 {
        self.epochs
            .iter()
            .map(|e| e.epoch.number)
            .max()
            .unwrap_or(0)
    }
}

struct Add {
    index: usize,
    identity: Identity,
    role: Role,
    admission: Admission,
}

/// The facts of a set of entries before the membership rules apply.
#[derive(Default)]
struct Facts {
    adds: Vec<Add>,
    removes: Vec<(usize, MemberId)>,
    role_sets: Vec<(usize, MemberId, Role)>,
    claims: BTreeMap<InviteId, Vec<(usize, Identity)>>,
    rewraps: Vec<(u32, MemberId, Wrap)>,
    member_wraps: Vec<(u32, MemberId, Wrap)>,
}

/// The state replayed from `set` (valid entries, closed under ancestors).
pub(crate) fn compute(graph: &Graph, set: &BTreeSet<usize>) -> State {
    let mut state = State::default();
    let mut facts = Facts::default();
    let ordered = graph.order.iter().copied().filter(|i| set.contains(i));
    for i in ordered {
        collect(graph, i, &mut state, &mut facts);
    }
    for (invite_id, claims) in &facts.claims {
        // Concurrent claims of one invite: the lowest entry hash (= lowest index) wins.
        let Some(&(index, identity)) = claims.iter().min_by_key(|(i, _)| *i) else {
            continue;
        };
        let Some(invite) = state.invites.get_mut(invite_id) else {
            continue;
        };
        invite.claimed_by = Some(identity.member);
        facts.adds.push(Add {
            index,
            identity,
            role: invite.role,
            admission: Admission::Claimed(*invite_id),
        });
    }
    facts.adds.sort_by_key(|a| graph.position[a.index]);
    state.ever_added = facts.adds.iter().map(|a| a.identity.member).collect();
    state.members = membership(graph, &facts, &BTreeSet::new());
    if !state.members.values().any(|m| m.role == Role::Admin) && !facts.adds.is_empty() {
        // ADR 0010 §5: the admin set never becomes empty; the denials that emptied it drop.
        let denials: BTreeSet<usize> = facts
            .removes
            .iter()
            .map(|(i, _)| *i)
            .chain(
                facts
                    .role_sets
                    .iter()
                    .filter(|(_, _, role)| *role == Role::Member)
                    .map(|(i, _, _)| *i),
            )
            .collect();
        let relaxed = membership(graph, &facts, &denials);
        let ignored: BTreeSet<usize> = facts
            .removes
            .iter()
            .map(|(i, m)| (*i, *m))
            .chain(facts.role_sets.iter().map(|(i, m, _)| (*i, *m)))
            .filter(|(i, m)| {
                denials.contains(i) && relaxed.get(m).is_some_and(|s| s.role == Role::Admin)
            })
            .map(|(i, _)| i)
            .collect();
        state.members = membership(graph, &facts, &ignored);
        state.admin_set_restored = true;
    }
    attach_wraps(&mut state, &facts);
    state.current = current_epoch(&state);
    state
}

fn collect(graph: &Graph, i: usize, state: &mut State, facts: &mut Facts) {
    let entry = &graph.entries[i];
    match &entry.payload {
        Payload::Genesis {
            document_id,
            admins,
            epoch0_wraps,
        } => {
            state.document_id = Some(*document_id);
            for admin in admins {
                facts.adds.push(Add {
                    index: i,
                    identity: *admin,
                    role: Role::Admin,
                    admission: Admission::Direct,
                });
            }
            push_epoch(state, graph.hashes[i], 0, epoch0_wraps);
        }
        Payload::AddMember {
            identity,
            role,
            wraps,
        } => {
            facts.adds.push(Add {
                index: i,
                identity: *identity,
                role: *role,
                admission: Admission::Direct,
            });
            for wrap in wraps {
                facts
                    .member_wraps
                    .push((wrap.epoch, identity.member, wrap.wrap));
            }
        }
        Payload::RemoveMember { member } => facts.removes.push((i, *member)),
        Payload::SetRole { member, role } => facts.role_sets.push((i, *member, *role)),
        Payload::NewEpoch { epoch, wraps } => push_epoch(state, graph.hashes[i], *epoch, wraps),
        Payload::AddPendingInvite {
            invite_id,
            role,
            expires,
        } => {
            state.invites.insert(
                *invite_id,
                InviteState {
                    role: *role,
                    expires: *expires,
                    claimed_by: None,
                },
            );
        }
        Payload::ClaimInvite {
            invite_id,
            identity,
            ..
        } => facts
            .claims
            .entry(*invite_id)
            .or_default()
            .push((i, *identity)),
        Payload::RewrapEpoch {
            epoch,
            member,
            wrap,
        } => facts.rewraps.push((*epoch, *member, *wrap)),
    }
}

fn push_epoch(
    state: &mut State,
    entry: EntryHash,
    number: u32,
    wraps: &[crate::keyring::RecipientWrap],
) {
    let Some(first) = wraps.first() else { return };
    state.epochs.push(EpochState {
        epoch: EpochRef {
            number,
            key_id: first.wrap.key_id(),
        },
        entry,
        wraps: wraps.iter().map(|w| (w.member, w.wrap)).collect(),
    });
}

fn attach_wraps(state: &mut State, facts: &Facts) {
    for (number, member, wrap) in facts.member_wraps.iter().chain(&facts.rewraps) {
        let epoch = EpochRef {
            number: *number,
            key_id: wrap.key_id(),
        };
        if let Some(target) = state.epochs.iter_mut().find(|e| e.epoch == epoch) {
            target.wraps.push((*member, *wrap));
        }
    }
}

/// Membership and roles with the denials in `ignored` left out. Deny wins: an add or role
/// counts only if every counted removal of that member is its ancestor; among the latest role
/// entries the lower role wins.
fn membership(
    graph: &Graph,
    facts: &Facts,
    ignored: &BTreeSet<usize>,
) -> BTreeMap<MemberId, MemberState> {
    let mut members = BTreeMap::new();
    let subjects: BTreeSet<MemberId> = facts.adds.iter().map(|a| a.identity.member).collect();
    for subject in subjects {
        let removes: Vec<usize> = facts
            .removes
            .iter()
            .filter(|(i, m)| *m == subject && !ignored.contains(i))
            .map(|(i, _)| *i)
            .collect();
        let after_removes = |index: usize| removes.iter().all(|&r| graph.is_ancestor(r, index));
        let adds: Vec<&Add> = facts
            .adds
            .iter()
            .filter(|a| a.identity.member == subject && after_removes(a.index))
            .collect();
        let Some(latest_add) = adds.last() else {
            continue;
        };
        let roles: Vec<(usize, Role)> = adds
            .iter()
            .map(|a| (a.index, a.role))
            .chain(
                facts
                    .role_sets
                    .iter()
                    .filter(|(i, m, _)| *m == subject && !ignored.contains(i) && after_removes(*i))
                    .map(|(i, _, role)| (*i, *role)),
            )
            .collect();
        let role = roles
            .iter()
            .filter(|(i, _)| !roles.iter().any(|(j, _)| graph.is_ancestor(*i, *j)))
            .map(|(_, role)| *role)
            .min()
            .unwrap_or(Role::Member);
        members.insert(
            subject,
            MemberState {
                identity: latest_add.identity,
                role,
                admission: latest_add.admission,
            },
        );
    }
    members
}

/// ADR 0010 §5: the highest epoch wins, ties by lowest entry hash. **(Proposal)** Deny wins
/// first: an epoch key held by someone who is not a member of the merged state (a rotation
/// concurrent with that member's removal wrapped it to them) is passed over while an epoch held
/// only by members exists.
fn current_epoch(state: &State) -> Option<usize> {
    let clean = |e: &EpochState| e.wraps.iter().all(|(m, _)| state.members.contains_key(m));
    let any_clean = state.epochs.iter().any(clean);
    state
        .epochs
        .iter()
        .enumerate()
        .filter(|(_, e)| !any_clean || clean(e))
        .max_by(|(_, a), (_, b)| {
            a.epoch
                .number
                .cmp(&b.epoch.number)
                .then_with(|| b.entry.cmp(&a.entry))
        })
        .map(|(i, _)| i)
}
