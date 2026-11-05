# Plan for the membership lifecycle, slice 1

Implementer, 2026-11-05, on `story/0097-membership-lifecycle`. Built against `specification.md`
(slice 1 = criteria 1 to 36; slice 2, criteria 37 to 41, is not built here), `adrs.md`,
ADR 0041 (accepted 2026-11-05 with the defaults A of Frage 160 to 162), `ux-notes.md`, and
ADR 0010, 0031, 0037, 0040 where cited. Lead decisions: two roles; any admin promotes and
demotes with a confirm; the second-admin line is dismissable per tier, per document, per
device; the last admin cannot leave until a successor exists; leave per person in synced
workspaces; re-add with "Full history" (preselected) / "From now on" for documents, no choice
for workspaces; "Keep as it is" sends one leave entry; REJOIN-PATH default B; the UI says
"device". No keyring payload or validity change, relay protocol stays 3, no `.curvyo`
`format_version` bump, no new crate or dependency. One PR at the end (M8); M1 to M7 are commits
on this branch. **(Proposal)** marks my own choices, vetoable at the demo.

## Affected crates/modules

- **`curvyo-crypto-core`** `keyring`: `Keyring::held_back()` (new `HeldBack`, `HeldBackKind`),
  `admin_set_restored()` derived from it, `rotation_pending()` (moved from
  `sync-core::devices::stale_epoch`), `former_participants()`, the withholding rule in
  `missing_wraps` (ADR 0041 decisions 2, 4, 5, 7, 10).
- **`curvyo-sync-core`**: `membership` (new module: the last-admin check, `membership_notices`,
  ADR 0041 decisions 1 and 9), `devices::reconcile` (`left` plan, member clamp in rooms with
  held-back denials; decisions 3 and 4), client `set_role`, add back with a history choice, the
  return check mode (`Hello{since_seq: u64::MAX}`, decision 6).
- **`curvyo-document-core`**: `container_share` gains the optional member `share.ended.json` v1
  (not listed in `manifest.json`); `devices_list` gains the optional `left` field (no version
  change). Goldens for both.
- **`curvyo-storage-io`**: `workspace_share_files`: the marker's `share.ended.json`; documents
  keep `share.json` v2 while the marker holds an ended record.
- **`curvyo-editor-wasm`**: `session::sync_members` (roles, leave, add back, check, Sync again,
  Keep as it is, the kept label), `session::workspace_sync`/`workspace_notices` (the same for a
  workspace), `device_sync` (person leave), `wasm_sync`/`wasm_workspace_sync` glue.
- **`curvyo-app`**: commands for the marker ended record and the per-data-directory notice and
  dismissal state.
- **`curvyo-ui-core`**: texts of every flow (rows, confirms, notices, Removed block, add-back
  dialog) and the wording test (criteria 10, 34).
- **`frontend`**: `ShareTab.tsx`, `WorkspaceShareBlock.tsx`, `MyDevicesBlock.tsx`, the tab mark.
- Docs: `docs/technical-debt.md` ("From now on is a client rule", "Mutual removal freezes a
  device group" now detected), privacy statement (criterion 31), `docs/design-system.md`
  (participant row with role line and two slots, tab mark), ADR 0010/0031/0037/0040 amendment
  notes named by ADR 0041.

## Milestones and tasks

### M1 keyring rules in `curvyo-crypto-core` and the last-admin check in `curvyo-sync-core`

1. [x] `Keyring::held_back()`: the removals and demotions that "the admin set never becomes
   empty" ignored, each with entry hash, author, subject and kind (`Removal`, `Demotion`);
   `admin_set_restored()` is `!held_back().is_empty()`. Tests first: the linear case (the sole
   admin removes itself while a member exists: one held-back entry with author = subject, the
   admin stays; no test had it), the linear self-demotion, the mutual removal of two admins
   (two entries, authors and subjects crossed), a removal that applies is not held back
   (criteria 5, 14, 27, 28, 29).
2. [x] `Keyring::rotation_pending()`: the former participants that still hold the current epoch,
   ordered by member id (moved from `devices::stale_epoch`, which `reconcile` now calls). Tests:
   a leaver holds it, a rotation clears it, a removal with rotation never shows it (criteria 11,
   12).
3. [x] `Keyring::former_participants()`: the identities (both public keys) of everyone ever
   added and no longer a participant, ordered by member id. Tests: removed, left, re-added and
   never-added identities (criteria 17, 24).
4. [x] `missing_wraps` withholds on purpose (Frage 160 A, ADR 0041 decision 7): for a member whose
   latest admission is a direct `AddMember`, an epoch that is a causal ancestor of that entry
   and not wrapped by it is not repaired. Epochs are compared as `EpochRef` (number and key id).
   Tests: remove B, rotate twice, add B back with the current epoch only: no member owes B the
   intermediate epoch; a rotation concurrent with the add back is still owed; a member admitted
   by invite still gets every epoch; a full-history add is unchanged (criteria 18, 19, 22, 23).
   (Genesis admins need no test: they hold epoch 0 through the genesis and every `NewEpoch`
   covers all members, so nothing can be withheld from them.)
5. [x] `curvyo-sync-core::membership::admin_remains(keyring, acted)`: whether an admin remains
   outside the devices an act removes or demotes (ADR 0041 decision 1). One pure function, so
   the per-device and per-person readings are one caller change (open point 1). Tests: sole
   admin, two admins, every admin in the acted set, acted set with members only (criteria 5, 14,
   16).

### M2 the rest of `curvyo-sync-core`

6. [x] Client `set_role(member, role)` (`SetRole`, admin only, refused with
   `SessionError::LastAdmin` when [`admin_remains`] says no; Step down is `set_role(me, Member)`);
   `leave()` refused the same way (criteria 2 to 5, 14).
7. [x] Client add back: `add_back(member, History::Full | History::FromNow)`; Full = every held
   epoch, From now on = the current epoch only, then `SnapshotDue` under it (ADR 0010 §9);
   refuses a current participant. Simulator test (adrs.md, criterion 22): after a from-now-on
   add back no online member's session writes `RewrapEpoch` for epoch k, and B's replica cannot
   open a record sealed under k (criteria 18, 19, 22, 23, 24).
8. [x] Client return check: a session start that sends `Hello{since_seq: u64::MAX}`, `Proof`,
   `Ping`, reads keyring entries until `Pong`, replays them and ends with
   `CheckResult{added_back | not_yet}`; sends nothing else (no `Awareness`, outbox, keyring
   entry, rewrap or snapshot) (criteria 20, 26, 31, 33). Done ahead in M1: relay test
   `relay_rules::a_return_check_cursor_gets_the_keyring_and_no_content` (`admit` with
   `since_seq = u64::MAX` sends `Welcome`, keyring and peers, no snapshot, no record; no relay
   change needed).
9. [x] `reconcile_with_left(group, workspaces, left, me)` (`reconcile` delegates with none, so
   `WorkspaceInput` and the tester's literals stay); a `left` workspace plans no add, and for an
   admin device the removal of every other group device still a member, one rotation, then its
   own removal (`Plan::leave`, decision 3; a field, not a `DeviceAction` variant, since the
   0096 tester matches `DeviceAction` exhaustively, Proposal); a member device only its own; in a room with
   held-back denials group devices are added as members (decision 4). Exhaustive three-device
   tests with another maker's admin, a person leave racing a sibling's re-add, a pairing into a
   frozen room (criteria 16, 28).
10. [x] `membership::membership_notices(previous admin set, keyring, own group)`: "is now an
    admin", "is no longer an admin", "left", "was removed", rotation pending, mutual removal
    (two or more concurrent held-back denials between admins) versus the refused linear act,
    the second-admin tiers (criteria 7, 8, 11, 27 to 30).

### M3 formats in `curvyo-document-core`

11. [x] `share.ended.json` v1 = `{format_version, server, room, server_seq, synced_version,
    ended: "removed" | "left"}` as an optional container member, never in `manifest.json`; a
    file with it opens as local; a higher `format_version` is ignored as "no way back" (not an
    open error, Proposal). Goldens `share_ended_v1.json`, a container with it, damaged variants;
    a container test that an older reader (no knowledge of the member) opens it (criteria 13,
    20, 33).
12. [x] `devices_list`: optional `left` on `workspaces.<id>`; read, set, clear; goldens
    `devices_list_left_v1.loro`; an older reader sees a switch-off (criterion 16).
    Built: `Document::ended_share`/`set_ended_share` (a third `Sharing` kind, never in a relay
    snapshot), `EndedRecord::{to_bytes, from_bytes}` for the marker (M5),
    `DevicesList::leave_workspace` and `SyncedWorkspace::left`; a `share.ended.json` beside a
    `share.json` is ignored (Proposal). The older-reader claim is checked against the v35
    reader's source (members read by name, assets by `assets/` names) and by opening a container
    with an unknown member; no v35 binary is run.

### M4 session ops in `curvyo-editor-wasm`

13. [x] Document: Make admin, Make member, Step down, Leave (ended record `left`, keys kept),
    Change keys now, Add back (history), the kept label (`remove_participant` stops dropping it),
    Removed or left rows, Check, Sync again (rebuild `share.json`/`keyring.log` from the ended
    record and the checked keyring), Keep as it is (one own leave). Removal writes the ended
    record `removed` instead of dropping the share (criteria 1 to 5, 9 to 14, 17 to 22, 25, 26).
14. [x] Workspace: the same through the workspace channel, the marker's ended record, documents
    keep `share.json` v2; person leave writes the `left` mark, waits for the `Ack`, then runs the
    plan; Sync again and switch-on clear it before connecting (criteria 15, 16, 23, 24, 29, 35).
15. [x] Notices state per data directory (last seen admin set, unread mark, dismissals,
    acknowledged held-back hashes), never in the file or on the wire (criteria 7, 8, 27, 30,
    31).
    Built (M4): `set_participant_role`, `rotate_share_keys`, `add_back_participant`,
    `leave_share`, `check_added_back`/`check_workspace_added_back`, `sync_again`/
    `workspace_sync_again`, `keep_as_it_is`, `reconcile_left_workspace`,
    `person_leave_allowed`, `share_admin_set`, `membership_notices_json` (structured; words in
    M6), the share view's role flags, `former` rows and `check` state, and
    `DeviceGroupLink::leave_workspace`/`list_acked`; wasm glue in `wasm_membership.rs`. The
    person leave holds its own `RemoveMember` until the rotation's workspace snapshot went out.
    **Left for M5:** the open workspace document still drops `share.json` v2 on removal (the
    0095 test `removal_rotates_the_workspace_key_and_writes_one_fresh_snapshot` pins it) until
    storage-io writes the marker's `share.ended.json` and stops `drop_document_shares`; the
    host keeps the last seen admin set and dismissals (the session takes them as inputs).

### M5 `curvyo-app` and `curvyo-storage-io`

16. [x] Marker `share.ended.json` read and write (temp then rename); the notice and dismissal
    state file beside the label cache; commands for the M4 ops (criteria 15, 20, 30, 31).
    Built: `curvyo-storage-io::workspace_ended` (`end_marker_share` writes the record, drops the
    marker's `share.json` and `keyring.log`, keeps `list.loro` (the record's cursor counts its
    records), `index.json` and every document's `share.json` v2; `read_marker_ended`; the record
    wins over left-over share files, so a crash never reconnects; `write_marker_share` and the
    full `remove_marker_share` drop it), app commands `workspace_marker_ended` and
    `end_workspace_marker_share`, the same in the browser store. A plain switch-off of a synced
    workspace still drops the share (`clear_workspace_marker_share`); a removal, a leave and a
    workspace the devices list marks `left` keep the record. The session's `end_share` no longer
    drops the open document's `share.json` v2: the 0095 pin in
    `workspace_removal_tests.rs` now expects it kept (a legitimate expectation change, ADR 0041
    decision 6 amends ADR 0037 decision 9). The notice state (last seen admin set, unread mark,
    dismissed tiers, acknowledged mutual removals) is kept in local storage beside the label
    cache, `frontend/src/lib/membershipState.ts`, as ADR 0041 decision 9 says ("beside the label
    cache"; the label cache is local storage, so no file in `curvyo-app`, Proposal); the device
    group view gains `left` per workspace.

### M6 texts in `curvyo-ui-core`

17. [x] Every text of ux-notes sections 2 to 8 with "device", names cut as the notes say; a test
    over every text of the share flow finds no "revoked" and no "lost access", and no
    "can no longer read" without what stays (criteria 10, 18, 34).
    Built: `curvyo-ui-core::membership_text` (rows, confirms, notices, leave) and
    `membership_return_text` (Removed block, check, add-back prompt and dialog), the wording test
    `no_text_of_the_share_flow_says_revoked_or_lost_access` over both plus the `0092` removal
    sentence and notices; `wasm_membership_text` passes them to the page; the refusals of
    `wasm_membership` come from there. `LEFT_NOTICE` says "device" (still "this document": the
    session does not know the file name; the host shows the named `left_notice`). "Leave my
    devices" refused as the group's only admin device says "This device is the only one that can
    manage your devices. Pair another device first." (a change of `0096` behaviour, test
    `the_groups_only_device_cannot_leave_it`). `REMOVED_NOTICE` keeps "computer": it is `0092`'s
    accepted text and its browser script matches it (Proposal, open point).

### M7 Share tab

18. [x] `ShareTab.tsx`, `WorkspaceShareBlock.tsx`: participant rows with role line and two slots,
    the confirms, the second-admin line, the Notices block, Removed or left, the Removed block
    with Check and the add-back prompt, the tab mark; `MyDevicesBlock.tsx`: the "You left" line
    (criteria 1 to 30, 35).
    Built: `useMembership` (commands, keys, the kept notice state, the mark, the ended state of a
    document from the share view and of a workspace from its marker), `ParticipantList` (rows,
    role and action slots, Make admin / Make member / Step down / Remove / Leave confirms, the
    frozen-room warning inside the one Make admin confirm with Make admin anyway, the only-admin
    refusal that moves focus to a member's Make admin, the second-admin line with Dismiss per
    tier), `MembershipNotices` (admin-change line, rotation pending with Change keys now, mutual
    removal with OK and its held-back lines), `EndedBlock` (heading, Check whether I was added
    back with "Contacts ...", the add-back prompt focused as a group, Sync again, Keep as it is),
    `FormerList` (Removed or left disclosure, Add back with History for documents, one sentence
    for workspaces), the Share tab mark in `PanelStrip` (cleared after 1 s of the Share body).
    The "You left Box job." line sits under the switch of the Workspace block (where the
    switch is, not in `MyDevicesBlock`). Every confirm returns focus to its opener; destructive
    actions keep the 400 ms guard (`DestructiveConfirm`, now with an Apply look for Make admin);
    focus after Leave, Sync again and the last Add back goes through `lib/shareFocus.ts`.
    The session view gains `endedRoom` (where Check goes, the keys' slot) and `addedBy`; an
    ended file names its server from its ended record; the workspace leave's notice says
    "You left Box job. The documents stay in this folder." instead of "removed". ShareTab's
    server reach check is not run for a removed or left file (criterion 33). The siblings of a
    person leave run `reconcile_left_workspace` in `useDevices` (open) and
    `useSyncedWorkspaces` (background). Sync again of a workspace the maker left clears the mark
    to a plain switch-off and waits for `list_acked()` (10 s, else "Not connected...") before it
    connects (Proposal: off, so no sibling acts on it; the switch turns it on again).
    `ShareTab.tsx` (734 lines) was split: Invite and Join to `ShareInviteJoin.tsx`, the name
    field to `ShareSettingField.tsx`, the disc to `ParticipantDisc.tsx`; `useWorkspaceSync.ts`
    (548) and `useDevices.ts` (793) were over 500 before and grow by about 15 and 60 lines
    (the person leave and the left reconcile belong with the group they drive; splitting them
    is a refactoring outside this story, noted for the lead).

### M8 journey and PR

19. [x] Three desktop instances and A2 with one server (criterion 36): the journey of criteria 3,
    10 to 12, 18 to 22, 26 to 28 for D and W; docs (privacy sentence, debts, design system, ADR
    amendment notes); full gate; PR.
    Built: `frontend/tests/browser/membership.cjs` (three browser contexts on a document, two
    faked desktop hosts on a workspace; the fake host moved to `support/fake-workspace-host.cjs`
    and is shared with `workspace.cjs`). Criteria 2, 3, 7, 11 to 14, 17, 18, 20, 21, 23 and 31
    run end to end. Criterion 27 cannot be produced through one relay (the relay closes a removed
    admin's connection before its concurrent entry is read; debt entry). The `0092`, `0095` and
    `0096` journeys know the kept ended record (their fake hosts gained the two marker commands;
    the `0092` removal confirm is found by its short name).

### M9 review fixes (2026-11-05)

20. [x] Architect and crypto review, blocking: `heads` in `share.ended.json` v1 and
    `Keyring::admitted_after`, so a relay cannot pass off an older keyring as an add back; Keep as
    it is rewrites the heads (criteria 20, 21, 33). `admin_remains` does not count an admin kept
    only by a held-back denial (criteria 5, 14, 27). The person leave counts the devices the
    group removed as the maker's, and a sibling whose own leave is refused later says so
    (criterion 16). Complete marker share files win over an ended record (criterion 33). Check
    sends one `Ping`; "not a member" is "Nothing to find here" (`CheckOutcome::NotFound`).
    Not done: `left: bool` on `WorkspaceInput` (the `0096` tester builds `WorkspaceInput`
    literals that must stay unchanged; `reconcile_with_left` stays).
21. [x] UX review must-fix 1 to 9 and the cheap nice-to-haves: focus after Dismiss, OK and Change
    keys now; the Removed block keeps the focus on its prompt and never takes it when only shown
    (focus requests expire after 3 s); Keep as it is refuses offline with words; the check's
    "could not be checked" and "nothing to find"; live regions mounted while empty; busy limits
    for Add back and Change keys now; the refusal under a row clears when the room changes;
    tooltips name why Step down and Leave are unavailable; "this device", "my device",
    "Participants", "Key epoch n" and the time come from Rust; Share stays available on a
    removed file (ux-notes 7.1.3). Kept: the bespoke History radios (`PanelRadioGroup` has no
    consequence line and returns focus to the canvas), and the Removed block stays after
    "Nothing to find here" (the reference is not dropped: a relay's word alone never deletes the
    way back, Proposal). Share workspace stays available beside a removed workspace's Removed block, as Share
    does for a document (hidden only while the add-back prompt asks).
    Open: in about one of six journey runs the first check right after an add back answers "Not
    added back yet." and the next one, 2 s later, "added back" (criterion 26 asks for the prompt
    within 5 s); not yet explained. Criterion 26's timings have no budget test.
    **Explained and fixed (M10):** the check did answer "added back"; the host did not read the
    Share tab again because the new check's link restarted its revision at 1 and could end on the
    first check's number. A new link now continues the revision of the one it replaces
    (`Session::install_link`). The journey asserts the first check (10 of 10 runs).

### M10 tester findings and ADR 0041 decision 11 (2026-11-05)

22. [x] `sync-core` `client::confirm`: a `Ping` after every own `Keyring` frame, one batch of own
    entries per `Ping` in flight, each `Pong` confirms its batch against the replay of the
    confirmed entries (an entry on an unconfirmed parent stays unconfirmed; one the replay
    refuses leaves the local log and is never resent); the confirmed set is every entry the relay
    sent, rebuilt on each `Welcome` (staged while the relay's keyring arrives and swapped in at
    the catch-up's first `Pong`, so the view never empties or shrinks in between). After the catch-up `Pong` every local entry the relay lacks
    goes out again, parents first, then a `Ping`; the session is online (rewraps, outbox,
    snapshots) only after that `Pong`. It ends as removed only by the confirmed keyring or
    `Refused{removed}`, and then keeps only confirmed entries. `confirmed_keyring()`,
    `confirmed_heads()`, `pending_acts()` (criteria 10, 13, 20, 26, 29a).
    Tests: the tester's five ignored findings pass; two changed expectation with the architect's
    amendment (`a_leaver_whose_leave_was_lost_can_be_added_back`: the leave now arrives, so A's
    removal of B is `NotAMember`; `mutual_removal_through_one_relay_after_both_were_cut_off`: the
    first one back wins, the confirmed views and the loser's ended heads match the relay).
    `tests/sim_confirmation.rs`: every act into a dead link reaches the room, a leave is pending
    until the relay has it, a reopened device sends its pending leave first, resends are stored
    once, an add back is pending until the relay holds it, and the reachability of criteria 27 to
    29 through one relay: (a) a mutual removal removes the admin whose removal arrived second,
    who is told so; (b) crossed demotions, and a removal arriving after its author's demotion,
    freeze the room with the notice; (c) removals merged offline freeze it.
23. [x] `document-core`: an ended record beside a partial share set opens as not shared (ADR
    0041 decision 6; tester low finding 3).
24. [x] `editor-wasm`: Leave keeps syncing and the file shared until the leave is confirmed
    ("Leaving..."; a reopened file with a pending leave ends on its next open as "left"); the
    ended record's `heads` are `confirmed_heads()`; rows carry `pending` (making admin, making
    member, removing, leaving), a removed or leaving participant stays listed until confirmed, an
    added-back one stays in Removed or left (`adding`) and "was added back" shows only once the
    relay holds it; notices and the admin set read the confirmed keyring; a mutual notice with a
    demotion says so (`demoted`). `confirmation_tests.rs`.
25. [x] `ui-core`: "Goes out when the connection is back" (after 10 s pending, Proposal), the
    crossed-demotions notice and Make admin warning (criteria 27, 28). Frontend: rows and Removed
    or left take the busy line from the view, 10 s timer in `usePendingLine`; the leaver's file
    is saved again when "left" arrives.
    Not done: the sync-server tester test `a_check_right_after_the_add_back_says_added_back`
    stays ignored: a check started 0 ms after A's socket took the add still races it; decision
    11 answers it on A's screen ("Added" only once the relay holds it), not on the wire.

### PR #281 feedback (customer, 2026-11-06)

- [x] F1 Rebase on `main` (ADR index rows 0041 to 0043 kept, spec 0097 and ADR 0041 unchanged).
- [x] F2 Share tab button rows wrap (`flex-wrap`) in My devices, pairing, decision, ended, invite,
  former-participant and confirm rows; `tests/browser/support/panel-overflow.cjs` asserts in
  `own-devices.cjs` and `membership.cjs` that nothing sticks out and no button clips its text.
- [x] F3 Sync these failures: `lib/unreachableWorkspaces.ts` keeps only the workspaces the server
  did not let in, one grouped alert (`could_not_sync_text(names)`), none without a server address
  or while unpaired; a workspace whose group was offline, or an already shared one, is retried when the group is online again (at once on network back); cleared
  when they get in, the list names them or the group ends; no 15 s wait while the group is
  offline. Node tests `tests/unreachableWorkspaces.test.ts`, journey phase in `own-devices.cjs`.
- [x] F4 Latency measured in the journeys (they print their timings); `networkBack` also clears
  the 30 s retry of background workspaces.

## Validation

- Core rules test-first in `curvyo-crypto-core/tests/keyring_lifecycle.rs` (new) and
  `curvyo-sync-core/src/membership.rs` unit tests; existing `keyring_forks.rs`,
  `devices_reconcile.rs` and the tester files stay green unchanged.
- Formats: golden files in `tests/fixtures/` of the crate.
- Every changed core crate builds and lints for `wasm32-unknown-unknown`.
- Timing budgets (criterion 26) assert in release only, thread CPU time.

## Open points for the lead

1. **Last-admin count for a document Leave.** Criterion 14 counts per person ("A's own devices
   together are one admin"); ADR 0041 decision 1 counts one device for a document Leave and per
   person only for the person leave of a synced workspace. Built as ADR 0041 (the client passes
   the acted-on set to `admin_remains`); per person is one caller change.
   **Resolved (lead, 2026-11-05):** per device for a document Leave (ADR 0041 decision 1), per
   person for the synced-workspace leave; criterion 14's sentence aligned (Proposal).
2. **`DeviceAction::Leave`** is a new variant of a public enum in `sync-core` (decision 3 needs
   the device's own removal after its siblings', and `write_removal` refuses `me`).
   **Built as `Plan::leave: bool`** instead (M2): `acceptance_0096_tester.rs` matches
   `DeviceAction` exhaustively and must stay untouched (Proposal).
3. **Return check anchoring (M2).** The ended record keeps no `keyring.log`, so the check's
   keyring is fetched fresh. The client accepts "added back" only when that keyring lists this
   device and wraps to it a key it still holds from the credential store, the wrap opening to
   that same key: this stops a keyring the relay made up (test
   `a_keyring_the_relay_made_up_is_not_an_add_back`). It does **not** stop a relay that serves
   the real keyring with this device's `RemoveMember` withheld: replay then lists the device and
   its old wraps satisfy the anchor. Impact is limited (no new key, an honest relay refuses the
   Sync again connection, nothing reaches anyone new), but a real fix needs a local anchor on the
   keyring at removal (its heads, say) in the ended record: a format question for the architect.
   A device whose credential store lost the room's keys never sees "added back".
   **Resolved (M9):** the ended record keeps the keyring heads; see debt "Return check heads".
4. **`LastAdmin` reaches the 0096 paths.** `ClientSession::leave` now refuses the only admin, so
   the group's last device cannot leave its group (`WasmDeviceGroup::leave` says so) and
   `leave_synced_workspace` returns `false` for a sole admin. Consistent with ADR 0041
   decision 1, but a change of accepted 0096 behaviour.
5. **Host routing (M5/M7).** `leave_share` on a workspace synced to own devices leaves one
   device, which siblings with the switch on re-add (`may_readmit`); the host offers only the
   person leave there. Sync again for a `left` workspace: the host clears the mark and waits for
   `list_acked()` before `workspace_sync_again`; nothing enforces it in the session.
6. **Synced workspace removed by another maker's admin (M5).** The open and background removal
   sites treat every end of a workspace the group lists (`inGroupList`) as a switch-off and drop
   the share, so such a removal keeps no way back (criterion 20). Telling it apart needs the
   list's off state, which races the workspace room; left as is.
   **Resolved:** `isSwitchOff` decides by who signed the removal (`frontend/src/lib/endedRecord.ts`).
7. **Outcomes the core does not tell apart.** "Keys changed after Ben left." (a reconcile that
   rotated), "The server's answer could not be checked" and "Nothing to find here" have no
   distinct outcome in the session (`NotYet` covers them); their texts were not kept.
   **Resolved (M9):** `CheckOutcome::Unverified` and `NotFound` with their texts; "Keys changed
   after Ben left." stays unbuilt (the rotation notice simply goes).
8. **Not built in M7:** the frozen-room sentence in the `0096` pairing confirm (ux-notes 3.2),
   the "Leaving..." busy state on the switch row (5.4), a `LEFT_NOTICE` with the file's name;
   `REMOVED_NOTICE` still says "computer" (`0092` text, its browser script matches it). The
   switch turned on for a removed or left workspace only clears the `left` mark (written off,
   as Sync again does, Proposal); it never shares anew, and Check stays the way back.
   **Resolved:** all four built; `REMOVED_NOTICE` says "device" and the `0092` journey matches.
9. **No UI run yet:** M5 to M7 are checked by unit tests and the static gate only; the
   three-instance journey is M8.
   **Resolved (M8):** `membership.cjs` and the four regression journeys pass; again after M10.
11. **Offline acts (M10).** Criterion 29a says an act "written while offline" goes out later; the
    session still refuses an act while it knows it is offline (`SessionError::Offline`, ux-notes
    "Not connected. Try again when connected."). What 29a covers is an act written into a link
    that is dead but not yet noticed; whether the client should queue acts it knows to be offline
    is a product question.
12. **Before the first `Welcome` of a session** the client counts the entries of `keyring.log`
    as confirmed except its own acts (its genesis and claim count): a device refused as removed
    at its handshake keeps its own older acts out of its ended heads, which only makes them older
    (Proposal).
10. **Sizes (CLAUDE.md §5).** Over 500 lines and justified in the debt entry "Sync files over 500
    lines": `useDevices.ts` 884, `useSyncedWorkspaces.ts` 675 (600 before the PR #281 feedback), `backgroundWorkspace.ts` 595,
    `PropertiesPanel.tsx` 586, `useCollaboration.ts` 571, `useWorkspaceSync.ts` 563,
    `device_group_glue.rs` 594, `pairing_glue.rs` 576 (half of it tests). `reconcile_open_workspace` is back under 60 lines. M10:
    `curvyo-ui-core/src/membership_text.rs` 681 (was 633; half of it the wording tests of
    criterion 34, one module for the words of one flow).

## PR #281 review follow-ups

- A removed (or left) device paired again with its identity is added back to the group (ADR 0041
  decision 5, every key) and made admin, instead of `AddMember`, which refuses a removed identity
  and left the opener at "Pairing ..."; any other refusal ends the pairing with "Could not add
  ... to your devices." (Proposal: no new identity, so the device keeps its other rooms.)
- Sync again of a document saves once the room admitted it; the client's liveness does not count
  a stall of the host between two ticks as silence (ADR 0031 decision 9 timing otherwise as is).
- The Sync these alert names why: unreachable, another server than the Share tab's, wss:// needed
  in a browser on https, refused by the server, or a share that failed on this device.
