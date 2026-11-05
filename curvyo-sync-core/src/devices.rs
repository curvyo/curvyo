//! Admission of the maker's devices by reconciliation (ADR 0040 decisions 5, 6 and 8): one pure
//! function compares the device group's keyring with each synced workspace's keyring and says
//! which keyring entries this device should write.
//!
//! The caller replays the keyrings, runs [`reconcile`] on start, after every group or workspace
//! keyring change and after a switch, and carries each [`Plan`] out through its
//! [`ClientSession`](crate::ClientSession): [`DeviceAction::Add`] by
//! [`admit_device`](crate::ClientSession::admit_device), [`DeviceAction::Remove`] by
//! [`remove_members`](crate::ClientSession::remove_members) (removals, then one rotation and a
//! snapshot), [`DeviceAction::Rotate`] by [`rotate`](crate::ClientSession::rotate) and
//! [`Plan::leave`] by [`leave`](crate::ClientSession::leave). Two
//! devices reconciling at once write duplicate adds or concurrent rotations; replay merges them
//! (ADR 0010 §5).

use curvyo_crypto_core::keyring::{Keyring, Payload, Role};
use curvyo_crypto_core::{DocumentId, Identity, MemberId};

/// One synced or formerly synced workspace as this device knows it.
pub struct WorkspaceInput<'a> {
    /// Whether the devices list marks it on.
    pub on: bool,
    /// The workspace room's replayed keyring.
    pub keyring: &'a Keyring,
}

/// Which room a plan is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    /// The device group's room.
    Group,
    /// A workspace room, by its document id.
    Workspace(DocumentId),
}

/// One keyring change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceAction {
    /// Add a device with this role and every epoch key the writer holds.
    Add {
        /// The device.
        identity: Identity,
        /// Its role: the writer's role in that room (DEV-ROLE A).
        role: Role,
    },
    /// Remove these devices, then one `NewEpoch` and a snapshot under it.
    Remove {
        /// The devices, ordered by member id.
        members: Vec<MemberId>,
    },
    /// A `NewEpoch` only: a former member still holds the current epoch.
    Rotate,
}

/// What one room needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The room.
    pub target: Target,
    /// The changes, adds before removals.
    pub actions: Vec<DeviceAction>,
    /// This device is not an admin there: it writes nothing and reports the room (criteria 17
    /// and 26); `actions` say what an admin still has to do.
    pub needs_admin: bool,
    /// Group devices an admin outside the maker's devices removed from this room: they are not
    /// added again (ADR 0040 decision 5); the room is reported, nothing is written for them.
    pub withheld: Vec<MemberId>,
    /// The maker left this workspace (its devices list entry is marked `left`, ADR 0041
    /// decision 3): after `actions` this device writes its own `RemoveMember` by
    /// [`leave`](crate::ClientSession::leave), also where it is not an admin.
    pub leave: bool,
}

/// The plans of every room that needs a change or has a withheld device; rooms that need none
/// are left out. Nothing is planned while `me` is not a member of the group (removed, left or
/// never paired). In a room whose admin set the replay restored (ADR 0010 §5: the removals that
/// emptied it are ignored, so a removal there may never take effect) adds and rotations are
/// planned, removals are not (ADR 0040 decision 8).
///
/// In a room with held-back denials (ADR 0041 decision 4) group devices are added as members
/// only, so no automatic write makes the admin that would release them.
#[must_use]
pub fn reconcile(group: &Keyring, workspaces: &[WorkspaceInput<'_>], me: MemberId) -> Vec<Plan> {
    reconcile_with_left(group, workspaces, &[], me)
}

/// [`reconcile`] with the workspaces whose devices list entry is marked `left`, the person
/// leave of ADR 0041 decision 3: such a workspace plans no add; an admin device there removes
/// every other group device still a member (one rotation and a snapshot), then leaves itself
/// ([`Plan::leave`]); a member device only leaves itself.
#[must_use]
pub fn reconcile_with_left(
    group: &Keyring,
    workspaces: &[WorkspaceInput<'_>],
    left: &[DocumentId],
    me: MemberId,
) -> Vec<Plan> {
    if group.role(&me).is_none() {
        return Vec::new();
    }
    let mut plans = Vec::new();
    if !group.rotation_pending().is_empty() {
        plans.push(plan(Target::Group, group, me, vec![DeviceAction::Rotate]));
    }
    for workspace in workspaces {
        let id = workspace.keyring.document_id();
        let target = Target::Workspace(id);
        if left.contains(&id) {
            if let Some(plan) = leave_plan(group, workspace.keyring, target, me) {
                plans.push(plan);
            }
            continue;
        }
        let (actions, withheld) = workspace_actions(group, workspace, me);
        if !actions.is_empty() || !withheld.is_empty() {
            let mut plan = plan(target, workspace.keyring, me, actions);
            plan.withheld = withheld;
            plans.push(plan);
        }
    }
    plans
}

/// The person leave of one workspace for this device, `None` once it is no member there.
fn leave_plan(group: &Keyring, keyring: &Keyring, target: Target, me: MemberId) -> Option<Plan> {
    let my_role = keyring.role(&me)?;
    let mut actions = Vec::new();
    // As a switch-off's removals; none in a restored room, where they may never take effect
    // (ADR 0040 decision 8): each sibling leaves by itself there.
    if my_role == Role::Admin && !keyring.admin_set_restored() {
        let members: Vec<MemberId> = keyring
            .participants()
            .into_iter()
            .map(|participant| participant.identity.member)
            .filter(|member| {
                *member != me && (group.role(member).is_some() || group.was_removed(member))
            })
            .collect();
        if !members.is_empty() {
            actions.push(DeviceAction::Remove { members });
        }
    }
    Some(Plan {
        target,
        actions,
        needs_admin: false,
        withheld: Vec::new(),
        leave: true,
    })
}

/// Whether a device of the maker's group that is not a member of `keyring` may be added there:
/// every removal of it since it was last added was written by a device that ever was in the
/// group (a switch-off, its own leave, or a device the maker has since removed, such as a stolen
/// one). A removal by an identity that never was in the group, an admin of another maker, stands
/// (ADR 0040 decision 5). Order is the keyring's causal order, the same on every device.
#[must_use]
pub fn may_readmit(group: &Keyring, keyring: &Keyring, member: &MemberId) -> bool {
    let entries = keyring.accepted_entries();
    let last_add = entries.iter().rposition(|entry| match &entry.payload {
        Payload::Genesis { admins, .. } => admins.iter().any(|a| a.member == *member),
        Payload::AddMember { identity, .. } | Payload::ClaimInvite { identity, .. } => {
            identity.member == *member
        }
        _ => false,
    });
    let since = last_add.map_or(0, |i| i + 1);
    entries[since..].iter().all(|entry| match &entry.payload {
        Payload::RemoveMember { member: removed } if removed == member => {
            group.role(&entry.author).is_some() || group.was_removed(&entry.author)
        }
        _ => true,
    })
}

fn plan(target: Target, keyring: &Keyring, me: MemberId, actions: Vec<DeviceAction>) -> Plan {
    Plan {
        target,
        actions,
        needs_admin: keyring.role(&me) != Some(Role::Admin),
        withheld: Vec::new(),
        leave: false,
    }
}

/// The changes one workspace needs to match the group and its switch, and the group devices
/// withheld there.
fn workspace_actions(
    group: &Keyring,
    workspace: &WorkspaceInput<'_>,
    me: MemberId,
) -> (Vec<DeviceAction>, Vec<MemberId>) {
    let keyring = workspace.keyring;
    let Some(my_role) = keyring.role(&me) else {
        return (Vec::new(), Vec::new());
    };
    let mut actions = Vec::new();
    let mut withheld = Vec::new();
    if workspace.on {
        for device in group.participants() {
            let member = device.identity.member;
            if member == me || keyring.role(&member).is_some() {
                continue;
            }
            if may_readmit(group, keyring, &member) {
                // The lower of the writer's role there and the device's group role (DEV-ROLE A
                // makes every group device an admin, so this is the writer's role); a member
                // while denials are held back (ADR 0041 decision 4).
                let role = if keyring.admin_set_restored() {
                    Role::Member
                } else {
                    my_role.min(device.role)
                };
                actions.push(DeviceAction::Add {
                    identity: device.identity,
                    role,
                });
            } else {
                withheld.push(member);
            }
        }
    }
    // ADR 0040 decision 8: in a restored room a removal may never take effect, so none is
    // planned there; adds and rotations still settle.
    let members: Vec<MemberId> = if keyring.admin_set_restored() {
        Vec::new()
    } else {
        keyring
            .participants()
            .into_iter()
            .map(|participant| participant.identity.member)
            .filter(|member| {
                *member != me
                    && (group.was_removed(member)
                        || (!workspace.on && group.role(member).is_some()))
            })
            .collect()
    };
    if !members.is_empty() {
        actions.push(DeviceAction::Remove { members });
    } else if !keyring.rotation_pending().is_empty() {
        actions.push(DeviceAction::Rotate);
    }
    (actions, withheld)
}
