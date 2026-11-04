//! Admission of the maker's devices by `reconcile` (ADR 0040 decisions 5, 6 and 8,
//! `specs/0096-own-devices-sync` criteria 6, 9, 11, 13, 16, 17 and 23 to 27): the rules on single
//! keyrings, and an exhaustive exploration of three devices A, B and C that pair, switch a
//! workspace on and off, remove each other, leave and replay what they wrote while not synced,
//! in a workspace that another maker's device D is a member of.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::hash::{Hash, Hasher};

use curvyo_crypto_core::keyring::{
    Entry, EntryHash, EpochWrap, Keyring, Payload, RecipientWrap, Role,
};
use curvyo_crypto_core::wrap_epoch_key;
use curvyo_crypto_core::{DocumentId, EpochKey, Identity, IdentityKeys, KeyId, MemberId};
use curvyo_sync_core::{DeviceAction, Plan, Target, WorkspaceInput, reconcile};
use support::device;

const GROUP_ID: DocumentId = DocumentId([0x6e; 16]);
const WS_ID: DocumentId = DocumentId([0x77; 16]);
const A: u8 = 1;
const B: u8 = 2;
const C: u8 = 3;
/// Another maker's device, a member of W, never in the group.
const D: u8 = 4;
const DEVICES: [u8; 3] = [A, B, C];

fn member(n: u8) -> MemberId {
    device(n).identity().member
}

fn identity(n: u8) -> Identity {
    device(n).identity()
}

/// Deterministic 32 bytes from anything hashable, so one logical history gives one byte history.
fn bytes_of(value: &impl Hash, salt: u64) -> [u8; 32] {
    let mut out = [0; 32];
    for (i, chunk) in out.chunks_mut(8).enumerate() {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        salt.hash(&mut hasher);
        i.hash(&mut hasher);
        chunk.copy_from_slice(&hasher.finish().to_le_bytes());
    }
    out
}

/// A set of keyring entries, by hash.
#[derive(Clone, Default)]
struct Room {
    entries: BTreeMap<EntryHash, Entry>,
}

impl Room {
    fn add(&mut self, entry: Entry) -> bool {
        let hash = entry.try_hash().unwrap();
        self.entries.insert(hash, entry).is_none()
    }

    fn keyring(&self) -> Option<Keyring> {
        let entries: Vec<Entry> = self.entries.values().cloned().collect();
        Keyring::replay(&entries).ok()
    }

    fn hashes(&self) -> Vec<EntryHash> {
        self.entries.keys().copied().collect()
    }
}

/// The epoch keys of every room, as only a test can know them.
#[derive(Clone, Default)]
struct Keys(BTreeMap<KeyId, EpochKey>);

/// Writes entries for `author` on `room`'s heads.
struct Writer<'a> {
    author: &'a IdentityKeys,
    document: DocumentId,
    room: &'a mut Room,
    keys: &'a mut Keys,
    written: &'a mut Vec<Entry>,
}

impl Writer<'_> {
    fn keyring(&self) -> Keyring {
        self.room.keyring().unwrap()
    }

    fn write(&mut self, payload: Payload) {
        let entry = Entry::sign(self.keyring().heads(), self.author, payload).unwrap();
        self.room.add(entry.clone());
        self.written.push(entry);
    }

    fn salt(&self, tag: u64) -> [u8; 32] {
        bytes_of(&(self.author.identity().member, self.room.hashes()), tag)
    }

    fn add(&mut self, identity: Identity, role: Role) {
        let keyring = self.keyring();
        if keyring.role(&identity.member).is_some() {
            return;
        }
        let me = self.author.identity().member;
        let mut wraps = Vec::new();
        for (epoch, wrap) in keyring.wraps_for(&me) {
            let key = &self.keys.0[&wrap.key_id()];
            let salt = self.salt(u64::from(epoch.number) + 100);
            let wrap = wrap_epoch_key(key, epoch.number, self.document, &identity, salt).unwrap();
            if !wraps
                .iter()
                .any(|w: &EpochWrap| w.wrap.key_id() == wrap.key_id())
            {
                wraps.push(EpochWrap {
                    epoch: epoch.number,
                    wrap,
                });
            }
        }
        self.write(Payload::AddMember {
            identity,
            role,
            wraps,
        });
    }

    fn remove(&mut self, members: &[MemberId]) {
        for member in members {
            if self.keyring().role(member).is_some() {
                self.write(Payload::RemoveMember { member: *member });
            }
        }
        self.rotate();
    }

    fn rotate(&mut self) {
        let keyring = self.keyring();
        let epoch = keyring.highest_epoch_number() + 1;
        let key = EpochKey::from_bytes(self.salt(1));
        self.keys.0.insert(key.key_id(), key.clone());
        let mut wraps = Vec::new();
        for participant in keyring.participants() {
            let salt = self.salt(2 + u64::from(participant.identity.member.0[0]));
            wraps.push(RecipientWrap {
                member: participant.identity.member,
                wrap: wrap_epoch_key(&key, epoch, self.document, &participant.identity, salt)
                    .unwrap(),
            });
        }
        self.write(Payload::NewEpoch { epoch, wraps });
    }

    fn apply(&mut self, actions: &[DeviceAction]) {
        for action in actions {
            match action {
                DeviceAction::Add { identity, role } => self.add(*identity, *role),
                DeviceAction::Remove { members } => self.remove(members),
                DeviceAction::Rotate => self.rotate(),
            }
        }
    }
}

fn genesis(n: u8, document: DocumentId, keys: &mut Keys) -> Room {
    let me = device(n);
    let key = EpochKey::from_bytes(bytes_of(&(n, document.0), 0));
    keys.0.insert(key.key_id(), key.clone());
    let wrap = wrap_epoch_key(&key, 0, document, &me.identity(), [n; 32]).unwrap();
    let entry = Entry::sign(
        Vec::new(),
        &me,
        Payload::Genesis {
            document_id: document,
            admins: vec![me.identity()],
            epoch0_wraps: vec![RecipientWrap {
                member: me.identity().member,
                wrap,
            }],
        },
    )
    .unwrap();
    let mut room = Room::default();
    room.add(entry);
    room
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Status {
    Unpaired,
    Active,
    /// Removed from the group: refused by the relay, keeps its files.
    Gone,
    Left,
}

/// The devices list's switch of the workspace: a last-writer-wins register.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct Switch {
    clock: u32,
    writer: u8,
    on: bool,
}

#[derive(Clone)]
struct Dev {
    n: u8,
    status: Status,
    group: Room,
    ws: Room,
    pending_group: Vec<Entry>,
    pending_ws: Vec<Entry>,
    switch: Switch,
}

impl Dev {
    fn new(n: u8) -> Self {
        Self {
            n,
            status: Status::Unpaired,
            group: Room::default(),
            ws: Room::default(),
            pending_group: Vec::new(),
            pending_ws: Vec::new(),
            switch: Switch {
                clock: 0,
                writer: 0,
                on: false,
            },
        }
    }

    fn plans(&self) -> Vec<Plan> {
        let Some(group) = self.group.keyring() else {
            return Vec::new();
        };
        let ws = self.ws.keyring();
        let inputs: Vec<WorkspaceInput<'_>> = ws
            .iter()
            .map(|keyring| WorkspaceInput {
                on: self.switch.on,
                keyring,
            })
            .collect();
        reconcile(&group, &inputs, member(self.n))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Op {
    Pair {
        by: u8,
        new: u8,
    },
    Switch {
        by: u8,
        on: bool,
    },
    Remove {
        by: u8,
        who: u8,
    },
    Leave {
        who: u8,
    },
    /// Sync, then reconcile; what reconcile writes waits for the next sync.
    Step {
        who: u8,
    },
}

impl Op {
    fn is_user(self) -> bool {
        !matches!(self, Self::Step { .. })
    }
}

#[derive(Clone)]
struct World {
    devs: Vec<Dev>,
    relay_group: Room,
    relay_ws: Room,
    relay_switch: Switch,
    keys: Keys,
}

impl World {
    /// A holds the group and the workspace W, switched on, which D (another maker) is a member
    /// of; B and C are not paired.
    fn new() -> Self {
        Self::with_other_maker(Role::Member)
    }

    /// As [`World::new`], with D in W as `role`.
    fn with_other_maker(role: Role) -> Self {
        let mut keys = Keys::default();
        let group = genesis(A, GROUP_ID, &mut keys);
        let mut ws = genesis(A, WS_ID, &mut keys);
        let author = device(A);
        Writer {
            author: &author,
            document: WS_ID,
            room: &mut ws,
            keys: &mut keys,
            written: &mut Vec::new(),
        }
        .add(identity(D), role);
        let switch = Switch {
            clock: 1,
            writer: A,
            on: true,
        };
        let mut devs: Vec<Dev> = DEVICES.iter().map(|&n| Dev::new(n)).collect();
        devs[0].status = Status::Active;
        devs[0].group = group.clone();
        devs[0].ws = ws.clone();
        devs[0].switch = switch;
        Self {
            devs,
            relay_group: group,
            relay_ws: ws,
            relay_switch: switch,
            keys,
        }
    }

    fn dev(&self, n: u8) -> &Dev {
        &self.devs[usize::from(n - 1)]
    }

    fn dev_mut(&mut self, n: u8) -> &mut Dev {
        &mut self.devs[usize::from(n - 1)]
    }

    fn fingerprint(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.relay_group.hashes().hash(&mut hasher);
        self.relay_ws.hashes().hash(&mut hasher);
        self.relay_switch.hash(&mut hasher);
        for dev in &self.devs {
            dev.status.hash(&mut hasher);
            dev.group.hashes().hash(&mut hasher);
            dev.ws.hashes().hash(&mut hasher);
            dev.switch.hash(&mut hasher);
            for entry in dev.pending_group.iter().chain(&dev.pending_ws) {
                entry.try_hash().unwrap().hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    /// The relay keeps what its replay accepts (ADR 0010 §10).
    fn publish(relay: &mut Room, pending: Vec<Entry>) {
        for entry in pending {
            let mut candidate = relay.clone();
            candidate.add(entry.clone());
            let accepted = candidate
                .keyring()
                .is_some_and(|k| k.is_accepted(&entry.try_hash().unwrap()));
            if accepted {
                *relay = candidate;
            }
        }
    }

    /// Device `n` connects to both rooms: a removed member is refused, a member pushes what it
    /// wrote and reads the relay's state.
    fn sync(&mut self, n: u8) {
        if self.dev(n).status != Status::Active {
            return;
        }
        let me = member(n);
        let group = self.relay_group.keyring().unwrap();
        if group.was_removed(&me) {
            let dev = self.dev_mut(n);
            dev.status = Status::Gone;
            dev.pending_group.clear();
            dev.pending_ws.clear();
            return;
        }
        if group.role(&me).is_none() {
            return;
        }
        let pending = std::mem::take(&mut self.dev_mut(n).pending_group);
        Self::publish(&mut self.relay_group, pending);
        self.relay_switch = self.relay_switch.max(self.dev(n).switch);
        let (relay_group, relay_switch) = (self.relay_group.clone(), self.relay_switch);
        let dev = self.dev_mut(n);
        dev.group = relay_group;
        dev.switch = relay_switch;

        let pending = std::mem::take(&mut self.dev_mut(n).pending_ws);
        let ws = self.relay_ws.keyring().unwrap();
        if ws.role(&me).is_some() {
            Self::publish(&mut self.relay_ws, pending);
            let relay_ws = self.relay_ws.clone();
            self.dev_mut(n).ws = relay_ws;
        } else if ws.was_removed(&me) {
            // Refused as removed: the workspace is a local one here until it is admitted again.
            self.dev_mut(n).ws = Room::default();
        }
    }

    /// Device `n` writes what `reconcile` plans where it is admin.
    fn reconcile(&mut self, n: u8) {
        if self.dev(n).status != Status::Active {
            return;
        }
        let plans = self.dev(n).plans();
        let author = device(n);
        let World { devs, keys, .. } = self;
        let dev = &mut devs[usize::from(n - 1)];
        for plan in plans.iter().filter(|plan| !plan.needs_admin) {
            Self::writer(dev, &author, keys, plan.target).apply(&plan.actions);
        }
    }

    fn writer<'a>(
        dev: &'a mut Dev,
        author: &'a IdentityKeys,
        keys: &'a mut Keys,
        target: Target,
    ) -> Writer<'a> {
        let (document, room, written) = match target {
            Target::Group => (GROUP_ID, &mut dev.group, &mut dev.pending_group),
            Target::Workspace(_) => (WS_ID, &mut dev.ws, &mut dev.pending_ws),
        };
        Writer {
            author,
            document,
            room,
            keys,
            written,
        }
    }

    fn group_members(&self, n: u8) -> Vec<u8> {
        let Some(group) = self.dev(n).group.keyring() else {
            return Vec::new();
        };
        DEVICES
            .iter()
            .copied()
            .filter(|&d| group.role(&member(d)).is_some())
            .collect()
    }

    /// Carries out `op`; false when it does not apply in this state.
    fn apply(&mut self, op: Op) -> bool {
        match op {
            Op::Step { who } => {
                if self.dev(who).status != Status::Active {
                    return false;
                }
                self.sync(who);
                self.reconcile(who);
                true
            }
            Op::Pair { by, new } => {
                if self.dev(new).status != Status::Unpaired {
                    return false;
                }
                // Pairing needs both online: the opener is synced.
                self.sync(by);
                if self.dev(by).status != Status::Active {
                    return false;
                }
                let author = device(by);
                let World { devs, keys, .. } = self;
                let dev = &mut devs[usize::from(by - 1)];
                Self::writer(dev, &author, keys, Target::Group).add(identity(new), Role::Admin);
                self.sync(by);
                let (group, switch) = (self.relay_group.clone(), self.relay_switch);
                let dev = self.dev_mut(new);
                dev.status = Status::Active;
                dev.group = group;
                dev.switch = switch;
                true
            }
            Op::Switch { by, on } => {
                self.sync(by);
                if self.dev(by).status != Status::Active || self.dev(by).switch.on == on {
                    return false;
                }
                let dev = self.dev_mut(by);
                dev.switch = Switch {
                    clock: dev.switch.clock + 1,
                    writer: by,
                    on,
                };
                self.reconcile(by);
                self.sync(by);
                true
            }
            Op::Remove { by, who } => {
                self.sync(by);
                if self.dev(by).status != Status::Active
                    || by == who
                    || !self.group_members(by).contains(&who)
                {
                    return false;
                }
                let author = device(by);
                let World { devs, keys, .. } = self;
                let dev = &mut devs[usize::from(by - 1)];
                Self::writer(dev, &author, keys, Target::Group).remove(&[member(who)]);
                self.reconcile(by);
                self.sync(by);
                true
            }
            Op::Leave { who } => {
                self.sync(who);
                if self.dev(who).status != Status::Active || self.group_members(who).len() < 2 {
                    return false;
                }
                let author = device(who);
                let me = member(who);
                let dev = self.dev_mut(who);
                let mut written = Vec::new();
                let entry = Entry::sign(
                    dev.group.keyring().unwrap().heads(),
                    &author,
                    Payload::RemoveMember { member: me },
                )
                .unwrap();
                dev.group.add(entry.clone());
                dev.pending_group.push(entry);
                if let Some(ws) = dev.ws.keyring().filter(|k| k.role(&me).is_some()) {
                    let entry =
                        Entry::sign(ws.heads(), &author, Payload::RemoveMember { member: me })
                            .unwrap();
                    written.push(entry);
                }
                dev.pending_ws.extend(written);
                self.sync(who);
                self.dev_mut(who).status = Status::Left;
                true
            }
        }
    }

    /// Every device still in the group steps until nothing changes.
    fn quiesce(&mut self) {
        assert!(self.try_quiesce(), "the devices do not settle");
    }

    fn try_quiesce(&mut self) -> bool {
        let mut quiet_rounds = 0;
        for _ in 0..12 {
            let before = self.fingerprint();
            for n in DEVICES {
                self.apply(Op::Step { who: n });
            }
            quiet_rounds = if self.fingerprint() == before {
                quiet_rounds + 1
            } else {
                0
            };
            if quiet_rounds == 2 {
                return true;
            }
        }
        false
    }

    /// The invariants of plan.md task 9, on the relay's state after everyone settled, also in
    /// rooms whose admin set the replay restored (ADR 0010 §5); only there may a removed device
    /// stay a workspace member (ADR 0040 decision 8, the frozen group).
    fn check(&self, path: &[Op]) {
        let group = self.relay_group.keyring().unwrap();
        let ws = self.relay_ws.keyring().unwrap();
        let admins = |k: &Keyring| {
            k.participants()
                .iter()
                .filter(|p| p.role == Role::Admin)
                .count()
        };
        assert!(admins(&group) >= 1, "no admin left in the group: {path:?}");
        assert!(admins(&ws) >= 1, "no admin left in the workspace: {path:?}");
        assert!(
            group
                .participants()
                .iter()
                .all(|p| !group.was_removed(&p.identity.member)),
            "{path:?}"
        );
        // The other maker's device is never removed by reconcile and always holds the key.
        assert_eq!(ws.role(&member(D)), Some(Role::Member), "{path:?}");
        assert!(
            ws.holds(&member(D), ws.current_epoch()),
            "D lost the current key: {path:?}"
        );
        // A workspace whose maker's devices all left the group is out of the group's reach.
        let reached = ws
            .participants()
            .iter()
            .any(|p| group.role(&p.identity.member).is_some());
        for n in DEVICES {
            let me = member(n);
            if !group.was_removed(&me) {
                continue;
            }
            assert!(
                !group.holds(&me, group.current_epoch()),
                "removed device {n} holds the group's current key: {path:?}"
            );
            if reached && !ws.admin_set_restored() {
                assert!(
                    ws.role(&me).is_none(),
                    "removed device {n} is a workspace member: {path:?}"
                );
                assert!(
                    !ws.holds(&me, ws.current_epoch()),
                    "removed device {n} holds the workspace's current key: {path:?}"
                );
            }
        }
        let in_group: Vec<MemberId> = group
            .participants()
            .iter()
            .map(|p| p.identity.member)
            .collect();
        let in_both = in_group.iter().filter(|m| ws.role(m).is_some()).count();
        if !reached {
        } else if self.relay_switch.on {
            assert_eq!(
                in_both,
                in_group.len(),
                "a group member is missing from the switched-on workspace: {path:?}"
            );
        } else if !ws.admin_set_restored() {
            assert!(
                in_both <= 1,
                "the switched-off workspace keeps devices: {path:?}"
            );
        }
        for dev in self.devs.iter().filter(|d| d.status == Status::Active) {
            assert_eq!(dev.group.hashes(), self.relay_group.hashes(), "{path:?}");
            assert!(
                dev.plans()
                    .iter()
                    .all(|plan| plan.needs_admin || plan.actions.is_empty()),
                "device {} still plans {:?} after settling: {path:?}",
                dev.n,
                dev.plans()
            );
        }
    }

    fn ops(&self) -> Vec<Op> {
        let mut ops = Vec::new();
        let active: Vec<u8> = DEVICES
            .iter()
            .copied()
            .filter(|&n| self.dev(n).status == Status::Active)
            .collect();
        let next_unpaired = DEVICES
            .iter()
            .copied()
            .find(|&n| self.dev(n).status == Status::Unpaired);
        for &n in &active {
            ops.push(Op::Step { who: n });
            if let Some(new) = next_unpaired {
                ops.push(Op::Pair { by: n, new });
            }
            ops.push(Op::Switch {
                by: n,
                on: !self.dev(n).switch.on,
            });
            for &who in &active {
                if who != n {
                    ops.push(Op::Remove { by: n, who });
                }
            }
            ops.push(Op::Leave { who: n });
        }
        ops
    }
}

/// Depth-first over every sequence of operations up to `depth` steps with at most `user_ops`
/// maker actions, skipping states already seen; checks the invariants after settling each.
fn explore(
    world: &World,
    path: &mut Vec<Op>,
    depth: usize,
    user_ops: usize,
    seen: &mut HashSet<u64>,
) {
    let mut settled = world.clone();
    if !settled.try_quiesce() {
        for n in DEVICES {
            eprintln!(
                "device {n}: {:?} plans {:?}",
                settled.dev(n).status,
                settled.dev(n).plans()
            );
        }
        panic!("the devices do not settle after {path:?}");
    }
    settled.check(path);
    if path.len() == depth {
        return;
    }
    let used = path.iter().filter(|op| op.is_user()).count();
    for op in world.ops() {
        if op.is_user() && used == user_ops {
            continue;
        }
        let mut next = world.clone();
        if !next.apply(op) || !seen.insert(next.fingerprint()) {
            continue;
        }
        path.push(op);
        explore(&next, path, depth, user_ops, seen);
        path.pop();
    }
}

/// Criteria 6, 13 and 23 to 27, plan.md task 9: every interleaving of pairing, switching,
/// removing, leaving and unsynced writes over three devices settles with no removed device in
/// the workspace or holding a current key (except where the admin set was restored), every
/// group member in the switched-on workspace, at least one admin, the other maker's device
/// kept with the current key, and nothing left to reconcile.
#[test]
fn every_interleaving_of_three_devices_settles_safely() {
    let mut seen = HashSet::new();
    let world = World::new();
    seen.insert(world.fingerprint());
    explore(&world, &mut Vec::new(), DEPTH, USER_OPS, &mut seen);
    assert!(seen.len() > 100, "explored only {} states", seen.len());
}

/// How deep the exploration goes, and how many maker actions one path takes; a release build
/// explores about 9,000 states (85 s).
#[cfg(debug_assertions)]
const DEPTH: usize = 6;
#[cfg(debug_assertions)]
const USER_OPS: usize = 3;
#[cfg(not(debug_assertions))]
const DEPTH: usize = 8;
#[cfg(not(debug_assertions))]
const USER_OPS: usize = 4;

// The rules one by one.

/// A world where A paired B (and C if `with_c`), everyone settled, W switched on.
fn paired(with_c: bool) -> World {
    let mut world = World::new();
    assert!(world.apply(Op::Pair { by: A, new: B }));
    if with_c {
        assert!(world.apply(Op::Pair { by: B, new: C }));
    }
    world.quiesce();
    world
}

fn ws_members(world: &World) -> BTreeSet<MemberId> {
    world
        .relay_ws
        .keyring()
        .unwrap()
        .participants()
        .iter()
        .map(|p| p.identity.member)
        .filter(|m| *m != member(D))
        .collect()
}

/// Criteria 6 and 11: a paired device is added to every switched-on workspace with the
/// admitting device's role, also one paired through another device.
#[test]
fn a_paired_device_is_added_with_the_writers_role() {
    let world = World::new();
    let mut paired_b = world.clone();
    assert!(paired_b.apply(Op::Pair { by: A, new: B }));
    let plans = paired_b.dev(A).plans();
    assert_eq!(
        plans,
        vec![Plan {
            target: Target::Workspace(WS_ID),
            actions: vec![DeviceAction::Add {
                identity: identity(B),
                role: Role::Admin,
            }],
            needs_admin: false,
            withheld: Vec::new(),
        }]
    );
    let world = paired(true);
    assert_eq!(
        ws_members(&world),
        [member(A), member(B), member(C)].into_iter().collect()
    );
}

/// Criterion 13: switching off removes the other group devices with one rotation; switching on
/// again admits them again.
#[test]
fn switching_off_removes_the_other_devices_and_on_admits_them_again() {
    let mut world = paired(true);
    assert!(world.apply(Op::Switch { by: B, on: false }));
    world.quiesce();
    assert_eq!(ws_members(&world), [member(B)].into_iter().collect());
    let ws = world.relay_ws.keyring().unwrap();
    assert!(!ws.holds(&member(A), ws.current_epoch()));
    assert!(world.apply(Op::Switch { by: B, on: true }));
    world.quiesce();
    assert_eq!(ws_members(&world).len(), 3);
}

/// Criteria 23 to 25: a removed device leaves every workspace and holds no later key, also when
/// it was not synced during the removal.
#[test]
fn a_removed_device_leaves_the_workspace_and_its_keys() {
    let mut world = paired(true);
    assert!(world.apply(Op::Remove { by: A, who: B }));
    world.quiesce();
    assert_eq!(world.dev(B).status, Status::Gone);
    assert!(!ws_members(&world).contains(&member(B)));
    let ws = world.relay_ws.keyring().unwrap();
    assert!(!ws.holds(&member(B), ws.current_epoch()));
}

/// Criterion 27: after a device left, the next admin device rotates the group and the
/// workspace ("Rotation pending" until then).
#[test]
fn a_device_that_left_is_rotated_out() {
    let mut world = paired(false);
    assert!(world.apply(Op::Leave { who: B }));
    let plans = world.dev(A).plans();
    assert!(plans.is_empty(), "A has not synced yet");
    world.sync(A);
    let plans = world.dev(A).plans();
    assert_eq!(
        plans
            .iter()
            .map(|p| (p.target, p.actions.clone()))
            .collect::<Vec<_>>(),
        vec![
            (Target::Group, vec![DeviceAction::Rotate]),
            (Target::Workspace(WS_ID), vec![DeviceAction::Rotate]),
        ]
    );
    world.quiesce();
    let ws = world.relay_ws.keyring().unwrap();
    assert!(!ws.holds(&member(B), ws.current_epoch()));
}

/// Criteria 17 and 26: where this device is a member but not an admin it writes nothing and the
/// plan says what an admin has to do.
#[test]
fn a_member_that_is_not_admin_only_reports() {
    let mut world = paired(false);
    // A makes B a plain member of W, then C pairs through A.
    let author = device(A);
    {
        let World { devs, keys, .. } = &mut world;
        let mut writer = World::writer(&mut devs[0], &author, keys, Target::Workspace(WS_ID));
        writer.write(Payload::SetRole {
            member: member(B),
            role: Role::Member,
        });
    }
    world.sync(A);
    world.sync(B);
    let mut group_writer_world = world.clone();
    {
        let World { devs, keys, .. } = &mut group_writer_world;
        World::writer(&mut devs[0], &author, keys, Target::Group).add(identity(C), Role::Admin);
    }
    group_writer_world.sync(A);
    group_writer_world.sync(B);
    let plans = group_writer_world.dev(B).plans();
    assert_eq!(
        plans,
        vec![Plan {
            target: Target::Workspace(WS_ID),
            actions: vec![DeviceAction::Add {
                identity: identity(C),
                role: Role::Member,
            }],
            needs_admin: true,
            withheld: Vec::new(),
        }]
    );
}

/// A device that is not (or no longer) in the group plans nothing.
#[test]
fn a_device_outside_the_group_plans_nothing() {
    let mut world = paired(true);
    assert!(world.apply(Op::Remove { by: A, who: C }));
    // C has not synced: its own view still lists it, so it would plan; once it reads the
    // removal it plans nothing.
    let mut stale = world.dev(C).clone();
    stale.group = world.relay_group.clone();
    assert_eq!(stale.plans(), Vec::new());
}

/// The session carries the plans out through the relay: `admit_device` brings back a device a
/// switch-off removed (which `add_member` refuses), `remove_members` removes with one rotation,
/// `leave` writes the own removal and `rotate` renews the key it still holds (criteria 13, 23
/// and 27).
#[test]
fn the_session_carries_the_plans_out_through_the_relay() {
    use curvyo_document_core::Document;
    use curvyo_sync_core::{RoomId, SessionError, SessionEvent};
    use support::sim::Net;

    const ROOM: RoomId = RoomId([0x77; 16]);
    // The group: A and B; A's removal of B below is a switch-off's.
    let group = paired(false).relay_group.keyring().unwrap();
    let mut net = Net::new(40);
    let a = net.add_client(A);
    net.share(a, ROOM, Document::new(1));
    net.run(500);
    let admit = |net: &mut Net| {
        net.clients[a]
            .session
            .as_mut()
            .unwrap()
            .admit_device(&group, identity(B), Role::Admin)
            .unwrap();
        net.pump(a);
        let b = net.add_client(B);
        net.connect_member(b, ROOM);
        let online = net.run_until(5_000, |n| {
            n.clients[b].saw(&SessionEvent::Online)
                && !n.clients[b].session().epoch_keys().is_empty()
        });
        assert!(
            online.is_some(),
            "B reads the room: {:?}",
            net.clients[b].events
        );
        b
    };
    let b = admit(&mut net);

    let session = net.clients[a].session.as_mut().unwrap();
    assert_eq!(session.remove_members(&[]), Err(SessionError::NotAMember));
    session.remove_members(&[member(B)]).unwrap();
    net.pump(a);
    net.run(1_000);
    assert!(net.clients[b].saw(&SessionEvent::Removed));
    let session = net.clients[a].session.as_mut().unwrap();
    assert_eq!(
        session.add_member(identity(B), Role::Admin),
        Err(SessionError::AlreadyMember)
    );

    let b = admit(&mut net);
    net.clients[b].session.as_mut().unwrap().leave().unwrap();
    net.pump(b);
    net.run(1_000);
    let keyring = net.clients[a].session().keyring().unwrap();
    assert!(keyring.was_removed(&member(B)));
    assert!(keyring.holds(&member(B), keyring.current_epoch()));
    net.clients[a].session.as_mut().unwrap().rotate().unwrap();
    net.pump(a);
    net.run(500);
    let keyring = net.clients[a].session().keyring().unwrap();
    assert!(!keyring.holds(&member(B), keyring.current_epoch()));
}

/// ADR 0040 decision 5: a group device that an admin of another maker removed from a shared
/// workspace is not added back by reconcile, the plan names it, and `admit_device` refuses it;
/// D keeps the current key throughout.
#[test]
fn a_device_another_maker_removed_is_not_added_back() {
    let mut world = World::with_other_maker(Role::Admin);
    assert!(world.apply(Op::Pair { by: A, new: B }));
    world.quiesce();
    assert!(ws_members(&world).contains(&member(B)));
    // D, an admin of W from another maker, removes B with a rotation.
    let author = device(D);
    let mut relay_ws = world.relay_ws.clone();
    Writer {
        author: &author,
        document: WS_ID,
        room: &mut relay_ws,
        keys: &mut world.keys,
        written: &mut Vec::new(),
    }
    .remove(&[member(B)]);
    world.relay_ws = relay_ws;
    world.quiesce();
    assert!(!ws_members(&world).contains(&member(B)), "B stays out");
    let plans = world.dev(A).plans();
    assert_eq!(
        plans,
        vec![Plan {
            target: Target::Workspace(WS_ID),
            actions: Vec::new(),
            needs_admin: false,
            withheld: vec![member(B)],
        }]
    );
    let ws = world.relay_ws.keyring().unwrap();
    let group = world.relay_group.keyring().unwrap();
    assert!(!curvyo_sync_core::may_readmit(&group, &ws, &member(B)));
    assert!(ws.holds(&member(D), ws.current_epoch()));
    // A switch-off and on by the maker does not bring B back either.
    assert!(world.apply(Op::Switch { by: A, on: false }));
    world.quiesce();
    assert!(world.apply(Op::Switch { by: A, on: true }));
    world.quiesce();
    assert!(!ws_members(&world).contains(&member(B)));
    let ws = world.relay_ws.keyring().unwrap();
    assert!(ws.holds(&member(D), ws.current_epoch()));
}

/// ADR 0040 decision 8 (Frage 146 default A): when the only two devices remove each other at
/// once, ADR 0010 §5 keeps both, and a later removal of either does not take effect while they
/// are the only admins; pairing a third device lets both removals take effect at once.
#[test]
fn a_mutual_removal_of_the_only_two_devices_freezes_the_group() {
    let mut world = paired(false);
    let remove = |world: &mut World, by: u8, who: u8| {
        let author = device(by);
        let World { devs, keys, .. } = world;
        World::writer(&mut devs[usize::from(by - 1)], &author, keys, Target::Group)
            .remove(&[member(who)]);
    };
    remove(&mut world, A, B);
    remove(&mut world, B, A);
    let pending: Vec<Entry> = [A, B]
        .iter()
        .flat_map(|&n| std::mem::take(&mut world.dev_mut(n).pending_group))
        .collect();
    World::publish(&mut world.relay_group, pending);
    world.quiesce();
    let group = world.relay_group.keyring().unwrap();
    assert!(group.admin_set_restored());
    assert_eq!(group.participants().len(), 2, "both stay");
    // A tries again: dropped as well.
    remove(&mut world, A, B);
    world.sync(A);
    world.quiesce();
    let group = world.relay_group.keyring().unwrap();
    assert!(group.role(&member(B)).is_some(), "B cannot be removed");
    // A third device makes both removals take effect: A and B leave the group.
    assert!(world.apply(Op::Pair { by: A, new: C }));
    world.quiesce();
    let group = world.relay_group.keyring().unwrap();
    assert!(group.role(&member(A)).is_none() && group.role(&member(B)).is_none());
    assert!(group.role(&member(C)).is_some());
}
