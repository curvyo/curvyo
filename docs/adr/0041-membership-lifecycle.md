# ADR 0041: Membership lifecycle: roles, leave, add back, the return check, verified join

**Status:** Accepted 2026-11-05 (customer: "weiter mit plan und bau von slice 1" after Frage 160 to 163, with the defaults A of Frage 160 to 162; vetoable at the demo). Originally proposed 2026-11-05 (architect). **needs-customer (answered by default):** Frage 160 (what "From now on"
can promise), Frage 161 (a new optional file member for the return path, a format we write) and
Frage 162 (what "Keep as it is" sends). On acceptance it amends ADR 0010 §7 and §9 (wording,
no rule change), ADR 0031 decision 6a (removed device), ADR 0037 decision 9 (removal keeps an
ended record) and ADR 0040 decisions 5 and 8 (`reconcile`). Asked by
`specs/0097-membership-lifecycle`. Executes the customer decisions of 2026-11-05: ROLES two
(admin, member); WHO-PROMOTES A (any admin promotes and demotes anyone, with a confirm);
LAST-ADMIN A; LEAVE-WORDING honest about what the leaver keeps; READD-DEFAULT A (full history
preselected, keys only; HIST-MEANING dropped, no shallow snapshot); WS-HISTORY B (a returning
workspace member gets everything, no choice); FP-FLOW A and FP-LINK A (the `0096` code and four
check words, beside the invite link); DEVICE-OR-PERSON A (roles per device, leave of a synced
workspace per person); NOTIFY A; REJOIN-PATH B (no background traffic). Choices marked
(Proposal) are the architect's, vetoable at the demo.
**Amended 2026-11-05** (architect, after the tester's findings on the `0097` branch; Proposal,
no customer question: no frame, no protocol version, no format and no server change): decision
11 (own keyring entries are confirmed by the relay and resent until they are), decision 2
(Leave ends on confirmation), decision 4 (which concurrent acts freeze a room through one
relay), decision 6 (`heads` are confirmed heads). On acceptance this also amends ADR 0031
decision 5 (the client's resend after the catch-up, wording only).

## Context

Slice 1 of `0092` and `0095` built invite, admission and removal with rotation. ADR 0010 already
has every payload the lifecycle needs (`SetRole`, `RemoveMember` by the subject itself,
`AddMember`, `NewEpoch`, `RewrapEpoch`) and its replay already drops a removal that would empty
the admin set. What the built code does not answer:

1. **"From now on" is undone by repair.** `Keyring::missing_wraps` makes every online member
   write `RewrapEpoch` for every epoch it holds that a current member lacks, and the validity
   rule only checks that author and recipient are members. A returning member added with the
   current epoch only gets the intermediate epochs from the next online member.
2. **A removed device forgets where it was.** `sync_removed` drops `share.json` and
   `keyring.log` (`set_share(None)`); a 0092 document has no other room id. A workspace keeps
   its id in `workspace.toml` but drops the marker's `share.json` (server, cursor).
3. **A frozen room is invisible.** Replay exposes one bool, `admin_set_restored`, which also
   fires for a linear sole-admin self-removal. `reconcile` adds group devices with the writer's
   role (admin) in such a room, which releases the held-back removals and ejects both old
   admins (ADR 0040 decision 8).
4. **`reconcile` undoes a person's leave.** With `on: false` it removes the *other* group devices
   and keeps the writer (0040's switch-off), and with `on: true` a sibling re-adds a device whose
   last removal was its own leave (`may_readmit`).
5. **The verified join has no transport.** A fingerprint is a hash; the admin needs both public
   keys to write wraps.
6. **A keyring entry is sent once and never confirmed** (since `0092`). An entry written into a
   link that is dead but not yet noticed is lost: a Leave that never arrives (the leaver thinks it
   left, the room still lists it with the current key), a removal's `NewEpoch` (the writer then
   seals under a key the relay refuses), a role change. The writer's own keyring and its ended
   record's `heads` keep the lost entry, so its return check can never say "added back". And an
   admin's list shows a change before the relay holds it, so the other side's check can say
   "not yet" for something the admin already sees as done.

## Options

**From now on (Frage 160).** (A) A client rule in `missing_wraps`: an epoch that is an ancestor
of a member's latest direct `AddMember` and not wrapped by it is withheld on purpose and not
repaired. (B) The same rule as keyring validity. (C) Drop the choice; documents always get full
history, as workspaces do. B makes the relay's and old builds' replay disagree with new builds
about which entries are valid, the costliest kind of change ADR 0010 has, and buys nothing
against a hostile member, who can hand over a key or the plaintext outside the app anyway. C is
honest but removes a choice the customer kept. **A chosen**, with its limit stated.

**Where the return reference lives (Frage 161).** (A) A new optional container member
`share.ended.json` in place of `share.json` and `keyring.log`, and the same file in a
workspace's marker. (B) Keep `share.json` and `keyring.log` and list ended rooms in the app data
directory. (C) App data keyed by room id only. B means "has `share.json`" no longer means
"shared": an older build, or this build after its app data is lost, reconnects silently, and a
leaver who was added back syncs again without being asked (criterion 25). C cannot find a 0092
document again, since its room id lives only in `share.json`. **A chosen**: the reference
travels with the file, every build treats the file as local, and readers open members by name
while `manifest.json` lists none, so older builds ignore it.

**Person leave of a synced workspace.** (A) An additive `left` mark on the devices list entry
that `reconcile` turns into removals. (B) The leaving device removes its siblings by itself.
(C) Leave per device and `reconcile` withholds a device's own leave (Frage 155 B2). B fails where
the maker's devices are members of another maker's workspace (members cannot remove) and for a
sibling that re-adds the leaver before it sees its own removal. C contradicts DEVICE-OR-PERSON A.
**A chosen.**

**Frozen room.** (A) `reconcile` adds group devices as members while held-back denials exist,
and the human paths that add an admin warn. (B) `reconcile` adds nobody there. (C) Warn only.
B leaves a newly paired device out of every frozen workspace; C has no human to warn.
**A chosen.**

**Verified join transport.** (A) The ADR 0040 pairing connection and CPace, a join purpose tag,
the admin shows the code. (B) A new "knock" frame: the invitee posts its keys to the room, the
admin compares a digit string. (C) An exchanged file or QR code. B gives non-members a way to
write to a room and the relay a per-room queue of unauthenticated requests, and a skipped digit
comparison loses to a swapping relay; C needs a camera or a second channel for files.
**A chosen** (FP-FLOW A).

**Confirming own keyring entries** (amendment). (A) A `Ping` after the entries: the relay
handles a connection's frames in order and writes a store op to disk before it sends the next
frame (`curvyo-sync-server` `Hub::carry_out`; a failed write stops the hub), so the `Pong`
proves each entry was handled, and whether it was stored is the client's own replay verdict,
which ADR 0010 makes the same on every replica (an entry's validity depends on its ancestors
only). (B) The relay also sends an accepted entry back to its writer as a `Keyring` frame.
(C) A new `KeyringAck{hash}` frame. B and C still need the `Ping` to learn of a dropped entry,
and both need relay protocol 4 so that a new client can tell a relay that never confirms from a
slow one, plus "upgrade the server before the apps"; their only gain is not relying on the
same-verdict rule, which ADR 0010 already requires. **A chosen.** For "not yet" right after an
add back: a check that retries for a few seconds would hide nothing (an add stuck behind a dead
link stays stuck longer than any retry) and would make more connections per press than
criterion 33 allows; the admin's screen showing the add only once it is confirmed is honest
and costs nothing. **Chosen.**

## Amendment (customer feedback on PR #281, 2026-11-09)

Add back by the "Removed or left" list (decision 5) and the return check with Sync again and Keep
as it is (decision 6, the way back) are dropped from the app: the customer never asked for them.
The way back is a new invite: the relay lets a removed identity that starts from nothing
(`since_seq` 0) offer a claim of a new invite, and refuses it as removed for anything else (a
reconnect from a cursor, the old link); the keyring already counts an add after the removal
(ADR 0010 §8). A joiner is not ended as removed while its claim waits for the relay. The core's
`add_back` and the check outcome stay in `curvyo-sync-core` until the architect retires them.
Armed acts have no sentence; Step down and Leave of the only admin are greyed.

## Decision

1. **Roles.** Make admin, Make member and Step down are `SetRole`, as ADR 0010 §4 already
   validates; one row is one device (DEVICE-OR-PERSON A). Any admin may change any role
   (WHO-PROMOTES A). Exception (customer 2026-11-09): "Leave my devices" on the group's last
   device is never refused; it ends the group on that device (ADR 0040 decision 8). Step down and
   Leave of the only admin are greyed with the reason as tooltip and description instead of
   refusing a press with a sentence. The client refuses Step down, Leave and a demotion when afterwards no admin
   would remain outside the devices the act removes or demotes: one device for Step down,
   Leave of a document or of an unsynced workspace, and demotion; **every device of the maker's
   group** for the person leave of decision 3 (LAST-ADMIN A). Otherwise a maker whose own devices
   are the only admins could leave from all of them at once, replay would drop every one of those
   self-removals and they would stay admins while believing they left. Replay stays the
   backstop; a unit test names the linear case (one admin removes itself, no fork), which
   today restores the admin set and is untested. An admin that is one only because replay held
   back its removal or demotion (decision 4) is no successor: replay would hold back the act as
   well, so the client refuses it instead of letting the maker believe it applied (amendment on
   review, 2026-11-05). For the person leave the maker's devices are the group's participants
   **and** the devices the group removed (a stolen one that is still an admin of the workspace
   is no outside admin); a sibling that finds its own leave refused later says so instead of
   staying silent. Security: a stolen admin device can demote
   every other admin as it can already remove them; the defence stays removing it promptly.
2. **Leave of one device** (document, workspace not synced to own devices). The client writes its
   own `RemoveMember` (ADR 0010 §4) and keeps syncing state until the relay confirms it
   (decision 11) or answers `Refused{removed}`; only then does it replace the share record with
   the ended record of decision 6 (reason `left`), keep its room keys in the credential store and
   stop syncing. Until then the file shows "Leaving…" and stays shared on disk (its
   `keyring.log` holds the leave); if the app closes first, the next open reconnects, resends the
   leave and ends there (amendment).
   `Keyring::rotation_pending()` moves from `sync-core::devices::stale_epoch` into `crypto-core`
   and drives "B left. The keys have not been changed yet." for every participant; Change keys now
   is the existing rotation with snapshot (ADR 0031 decision 6a). `reconcile` keeps rotating by
   itself for **any** former member that holds the current epoch, not only the maker's own
   devices, in every workspace it serves where this device is admin (Proposal: rotating fails
   closed, and the notice then reads "Keys changed").
3. **Leave of a synced workspace is per person.** The devices list entry gains an optional field:
   `workspaces.<id> = {name, on: false, left: true}`. Readers ignore unknown fields, so the
   devices list keeps `format_version` 1 (the precedent is `browser`, ADR 0040 decision 1).
   `reconcile` plans, for a workspace marked `left`: no add; if this device is admin there,
   `RemoveMember` of every other group device still a member, one `NewEpoch` and a snapshot
   (ADR 0031 6a); then this device's own `RemoveMember`. A member device only writes its own.
   The leaving device writes the mark, waits for the relay's `Ack`, then runs the same plan. A
   sibling that re-added the leaver in the moment before it read the mark removes it again when
   it does, and a sibling that stays offline is removed by any admin sibling instead of keeping
   the current key until it returns. The person-level rule of decision 1 guarantees an admin
   outside the group, so the siblings' concurrent removals of each other all apply. A device
   that has read the mark writes its ended record with `ended: "left"` however its removal
   reaches it (own entry, a sibling's removal or `Refused{removed}`). The mark is cleared only by
   a person: Sync again after an add back (decision 6) and switching the workspace on rewrite the
   entry without `left` and wait for its `Ack` before the room connection opens, so the returning
   device and a sibling that reads late do not leave again. An older
   build reads the mark as a plain switch-off and keeps the workspace on that device until it is
   updated (stated in the release notes).
4. **Held-back denials and frozen rooms.** `Keyring::held_back()` returns the removals and
   demotions the "admin set never becomes empty" rule ignored: entry hash, author, subject,
   kind. `admin_set_restored()` becomes `!held_back().is_empty()`. No format or rule change.
   - Two or more held-back denials between admins that are concurrent with each other are the
     mutual removal of criterion 27; one linear self-removal or self-demotion is the refused
     last-admin act of decision 1. The client tells them apart by the entries' authors and
     subjects.
   - Every path that would make a new admin while `held_back()` is non-empty states first that
     the held-back removals then apply (criterion 28): Make admin, and pairing a new own device
     into the group room. Add back and the verified join add members and release nothing.
   - `reconcile` adds group devices with role **member** in a workspace whose keyring has
     held-back denials, so no automatic write ever releases them (amends ADR 0040 decision 8,
     which added them as admins). They become admins only through a person's Make admin.
   - The acknowledgement of the notice is kept on this computer, keyed by the held-back entry
     hashes, so a new mutual removal shows again.
   - **Through one relay** (amendment, by code reading; a simulator test pins it): the relay
     closes a removed member's connection as soon as it stores the removal, so of two admins who
     remove each other, the removal that reaches the relay first applies and the other admin is
     removed and told so; its own removal is never stored (decision 11 keeps it out of its
     `heads`). A demotion closes no connection and an entry is judged by its ancestors, so two
     admins who demote each other, or a removal that reaches the relay after the remover's own
     demotion by the other admin, are both stored and both held back: that is the frozen room
     of criteria 27 to 29. Two removals that met outside one relay (merged offline keyrings) are
     held back the same way.
5. **Add back.** `Keyring::former_participants()` returns the identities (both public keys) of
   everyone `was_removed`; an admin needs no invite and no fingerprint for them. Add back writes
   `AddMember{identity, member, wraps}` (Make admin is a separate act).
   - **Document:** Full history (preselected, READD-DEFAULT A) wraps every epoch the admin holds.
     From now on wraps the current epoch only and uploads a fresh signed snapshot under it
     (ADR 0010 §9). Under decision 7 the other members do not repair the withheld epochs.
   - **Workspace:** every epoch the admin holds, no choice (WS-HISTORY B); a first add keeps
     `0095`'s rule.
   - The returning device keeps every epoch key it held before (nothing deletes them), so From
     now on withholds only epochs that began while it was away; the dialog says so.
   - The last known label comes from the per-data-directory label cache that already keeps the
     labels other participants announced (`curvyo.share-labels`). Removal no longer drops the
     entry (`remove_participant` stops calling `labels.remove`). Labels never reach a file or the
     server (ADR 0031 decision 7).
6. **The ended record and the return check** (REJOIN-PATH B, Frage 161 A).
   - On removal or leave the document's container replaces `share.json` and `keyring.log` with
     `share.ended.json` = `{format_version: 1, server, room, server_seq, synced_version, ended:
     "removed" | "left", heads}`; `heads` are the heads of the keyring entries the relay
     confirmed to this device (decision 11) when the share ended (public entry hashes, hex). A
     subset of what the relay holds only makes the rule below weaker by older heads, never
     wrong; an own entry the relay never stored is never a head (amendment). A file that holds it is a local file to every build. A
     workspace's marker keeps `share.ended.json` in place of `share.json` and `keyring.log`;
     `list.loro` stays (the record's cursor counts its records), `index.json` and `deleted/` go
     as today. Complete share files beside an ended record win, in a marker as in a document
     (`share.json` beats `share.ended.json`); an ended record beside a partial set means not
     shared. A record this build cannot read (newer version, malformed) is no way back, never an
     open error. Its documents keep `share.json` v2 while the marker
     holds an ended record, so a rejoin matches them by document id (amends ADR 0037 decision 9,
     which dropped them on the next save). An older build that saves the file drops the unknown
     member; the way back is then an invite link.
   - "Check whether I was added back" opens one room connection: `Hello{room, since_seq:
     u64::MAX, identity}`, `Proof`, `Ping`. `u64::MAX` makes the relay's `admit` send `Welcome`,
     the keyring and the peers but no snapshot and no record (a relay test pins this; no relay
     change). `Refused{removed}` means not yet. After `Welcome` the client reads keyring entries
     until `Pong`, replays them itself and closes. It sends nothing else: no `Awareness` (it
     would carry the label), no outbox, no `Keyring` entry, no `RewrapEpoch`, no snapshot. The
     answer is the client's own replay of signed entries: a relay can deny a returning member
     (a refusal), not fake an add. "Added back" needs three things: the fetched keyring lists the
     device, wraps to it a key it still holds, and holds an accepted `AddMember` (or invite
     claim) of it that descends from every stored `heads` entry. A current member cannot be
     added, so such an add implies a removal after what the device knew; a keyring cut off
     before the removal, or an earlier add back served again, fails the rule and the check says
     "could not be checked". After Keep as it is the record's `heads` are rewritten to include
     the new leave. A relay that knows no such room answers "not a member", which the check
     shows as "nothing to find here" (Proposal: a room that never listed the device reads the
     same).
   - What leaves the machine on a check: one connection by an identity the relay already lists
     for that room, its time and IP. If the device was added back, the online members get a
     `Peer` for it for the length of the check (identity only, no label).
   - Sync again rebuilds `share.json` and `keyring.log` from the ended record and the checked
     keyring and resumes as a reopened shared file: edits made while away go out as offline edits
     (`0092` criteria 17 to 19); for a workspace marked `left` it first clears the mark
     (decision 3). Keep as it is: Frage 162.
7. **Repair respects a withheld epoch** (Frage 160 A). `missing_wraps` leaves out, for a member
   whose latest admission is a direct `AddMember`, every epoch that is a causal ancestor of that
   entry and not wrapped by it. Concurrent and later epochs are still repaired (ADR 0031 6a), a
   member admitted by an invite still gets every epoch (COOP-JOIN A), and a full-history add
   wraps every epoch the adder holds, so nothing else changes. An epoch the adding admin did not
   hold is no longer filled in by another member; Full history means the adder's epochs
   (criterion 18 says so). **Security:** this is a rule of honest clients, not a guarantee. A
   build without it, or a member who wants to, can still hand the returning person the
   withheld keys or the content. The dialog says "the app does not give B the keys", never
   "B cannot read".
8. **Verified join** (slice 2; FP-FLOW A, FP-LINK A). The relay's pairing connection, frames,
   slots, limits and the code format are ADR 0040 decisions 3 and 4, unchanged, relay protocol
   3. **The admin opens the slot and shows the code; the invitee types the server address and
   the code** (UX MEM-VERIFIED-DIRECTION confirmed: one pattern for makers). Join messages v1
   are the pairing messages with the channel identifier `lv_cat("curvyo-join-v1", server URL)`
   and `curvyo-join-v1` in every HKDF info, so a device-pairing code never completes a join and
   a join code never pairs a device; the wrong kind fails as a wrong code. Steps: CPace; the
   invitee sends its sealed identity, label and signature over the transcript; both screens show
   the four check words, the admin also the invitee's label and fingerprint. On Add the admin
   writes `AddMember{X, member, wraps}` (documents: the history choice of decision 5;
   workspaces: every epoch) and sends sealed `{kind: document | workspace, room, name}`. The
   invitee connects with `Hello`, is admitted as a member and reads the snapshot as a link joiner
   does (a new file, or an empty folder for a workspace). Words differ sends
   `PairClose{cancelled}` and nothing is written. Any admin of a room can add a maker's new
   computer this way (criterion 41).
   **Security:** a relay that swaps keys must guess the 25-bit secret within 5 tries; someone who
   overhears the code and races the invitee shows up with another label, fingerprint and words
   than the invitee's screen. The code is a one-time, five-minute, online-only PAKE password, not
   a credential: nothing that grants access exists before the admin presses Add. That is how
   ADR 0010 §7's "no secret ever transits a channel" reads from now on (amendment on
   acceptance). The debt "Joining needs a member online" stays (an admin, here).
9. **Notices come from the keyring.** Admin-set changes are the difference between the admin
   set this device last showed for a room and the current replay. The last seen set and the
   unread mark are kept per data directory beside the label cache (Proposal), never in the file,
   never on the wire. "Rotation pending" is `rotation_pending()`; the mutual-removal notice is
   decision 4. A pure function `membership_notices(previous, keyring, own group)` in a new module
   `membership` of `curvyo-sync-core` returns the lines; `curvyo-ui-core` formats the texts.
10. **Formats, protocol, crates.** Keyring: no payload, no validity rule, no `format_version`
    change. Relay protocol 3 unchanged, no frame, no server code change. New formats, each with
    golden files: `share.ended.json` v1 (container member and marker file), join messages v1,
    the devices list's `left` field (no version change). `.curvyo` `format_version` and
    `share.json` versions unchanged. No new crate, no new dependency. `crypto-core`:
    `held_back`, `former_participants`, `rotation_pending`, `admitted_after`, the `missing_wraps`
    rule.
    `sync-core`: `reconcile` (`left`, member clamp), the client's check mode, the join purpose in
    `pairing`, `membership`. `document-core`: the ended record, `left` in `devices_list`.
    `storage-io`: the marker's ended record. `editor-wasm`, `curvyo-app`, `ui-core`: glue and
    texts. Every changed core crate still builds for `wasm32-unknown-unknown`.
11. **Own keyring entries reach the relay** (amendment; Proposal). In a room session (not the
    return check, not the genesis or claim of sharing and joining, which keep their own retry)
    the client confirms and resends what it writes. No frame, no protocol version, no server
    change, no format change.
    - *Confirmed set*, in memory per session: every entry the relay sends (the whole keyring
      after each `Welcome`, every broadcast; counted also when the entry is already known
      locally), plus own entries confirmed by a `Pong`. It is rebuilt on every `Welcome`, so
      nothing is persisted; `keyring.log` keeps every local entry as today.
    - *Barrier:* after a `Keyring` frame the client sends a `Ping`. `Pong`s answer `Ping`s in
      order, so the client keeps a queue of the entries sent before each `Ping`; on its `Pong`
      each entry its own replay still accepts is confirmed, and one it refuses is dropped from
      the local log (the relay dropped it too). A connection that ends first leaves them
      unconfirmed.
    - *Resend:* after the catch-up `Pong`, the client sends every local entry the confirmed set
      lacks, parents first, then one `Ping`, and only then writes rewraps, flushes the outbox and
      uploads snapshots, so a push sealed under a resent `NewEpoch` reaches a relay that knows
      the epoch. The relay ignores an entry it holds (`append_entry`), so a resend is idempotent
      by entry hash. An entry the relay refused at its `Pong` is never resent.
    - *Removed:* the session ends as removed only by the confirmed keyring or
      `Refused{removed}`, never by an own unconfirmed `RemoveMember`, so a reopened device sends
      its pending leave first. A device that `Refused{removed}` cut off keeps its unconfirmed
      entries out of the ended record's `heads`.
    - *Screen:* a role change, removal, add back or leave shows as pending ("Adding…",
      "Removing…", "Leaving…"; texts per UX) until confirmed, and the success line ("B was added
      back, B can now check") only after; after 10 s unconfirmed (Proposal) the row says the
      change goes out when the connection is back. Notices (decision 9) read the confirmed
      keyring.
    - *API:* `ClientSession` gains the pending entries per participant and `confirmed_heads()`;
      the ended record takes its `heads` from it (`editor-wasm` `ended_record`), `ui-core` the
      pending texts. Pure `sync-core`, wasm-safe.
    - *Dependency:* the barrier relies on every replica giving an entry the same verdict from its
      ancestors (ADR 0010). Any later change to keyring validity bumps the relay protocol.

## Consequences

- No change to ADR 0010's rules, the relay or its protocol: the lifecycle is existing entries
  written, confirmed and resent by new client rules (decision 11), and the relay learns only what ADR 0010 §10 already names (roles
  and when they change), plus one connection per check.
- "From now on" is a client promise, not cryptography (decision 7); debt entry "From now on is
  a client rule".
- `reconcile` grows two rules (`left`, member clamp in held-back rooms). Mitigation as ADR 0040:
  exhaustive three-device graphs plus another maker's admin, now with a person leave racing a
  sibling's re-add and a pairing into a frozen room.
- A mutual removal of the only admins still freezes a room; it is now visible and no automatic
  write releases it. The keyring rule that would settle it stays debt (Frage 146 default A).
- A removed or left file carries its server and room id in `share.ended.json`, which a shared
  file carried in `share.json` anyway (plaintext at rest, ADR 0008 §7), plus the keyring head
  hashes (public, no keys).
- An own entry written into a dead link reaches the room when the link is back, in the order it
  was written, however much later; the admin's screen says it is pending until then. `heads`
  hold only confirmed entries, so a lost or refused own entry no longer blocks the way back
  (debt entry "Return check heads" resolved).
- Through one relay, a mutual removal removes one admin (the subject of the removal that
  arrives first); only crossed demotions freeze a room (decision 4).
- A stolen sibling device can write `left` into the devices list and so push all the maker's
  devices out of a synced workspace: the same power as a switch-off, which it already has
  (ADR 0040). The defence stays removing it from the group promptly.
- Tests the build needs: the linear last-admin case; `held_back` for mutual and linear cases;
  `missing_wraps` withholding exactly the ancestor epochs; `reconcile` with `left` and with
  held-back denials; a person leave, add back and Sync again after which the returning device
  stays a member; the relay sending no record for `since_seq = u64::MAX`; a check that sends
  no frame after `Proof` but `Ping`; join versus pairing domain separation and a swapping relay;
  golden files for the three formats. For decision 11: a leave, a removal with rotation and a
  role change written into a silently dead link reach the relay after it returns and the
  writer's edits converge; a reopened device with a pending leave sends it before it ends; the
  cut-off admin of a same-step mutual removal and a leaver whose leave was lost are added back
  and their checks say so; resends are idempotent; an admin's list shows an add back as pending
  until the relay holds it; a mutual demotion through one relay ends held back.

## Questions for the customer

**Frage 160 (FROM-NOW-ON): What may "From now on" promise when someone is added back?**
- A: The app does not hand the person the keys of the time they were away; anyone still in the
  share could pass them on outside the app. No rule change, ships in slice 1.
- B: The keyring refuses those keys to the person (a change to ADR 0010's rules; the server and
  older app versions then disagree about valid entries until all are updated). A determined
  member can still pass on the content.
- C: No choice for documents either: adding back always gives full history.
- Recommendation: A. Default if unanswered: A.

**Frage 161 (ENDED-RECORD): Where does a removed or left file keep the way back?**
- A: In the file, as a small new part `share.ended.json` (fields: `server`, `room`,
  `server_seq`, `synced_version`, `ended`, `heads`; no keys); older app
  versions ignore it, and if they save the file the way back is an invite link again.
- B: The file stays "shared" and a list on this computer says "do not connect". If that list is
  lost or an older version opens the file, it connects by itself and may sync again unasked.
- C: Only on this computer, by room; a removed document cannot be found again, only workspaces.
- Recommendation: A. Default if unanswered: A.

**Frage 162 (DECLINE): What does "Keep as it is" do after someone was added back?**
- A: The device leaves again (one keyring entry, as Leave): the admins see "B left" and that the
  keys should change. The list stays true.
- B: Nothing is sent. The person stays listed as a member and holds the current key, though they
  never sync.
- Recommendation: A. Default if unanswered: A.
