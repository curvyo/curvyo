# Plan for own-devices sync, slice 1 (with the browser, BROWSER-SLICE A)

Implementer, 2026-11-04, on `story/0096-own-devices-sync`. Built against `specification.md`
(criteria 1 to 40), `adrs.md`, ADR 0040 (accepted 2026-11-04) and `ux-notes.md`. Lead decisions on
the UX Proposals: the UI says "device"; the device name is separate from "Name shown to others";
the one-device warning is persistent; switching sync off on any device stops it for all devices
(the confirm names them). Browser key backup (Frage 145) is open: its tasks come last (M9) behind
one narrow interface, everything else is built first. One PR at the end (M10); M1 to M9 are
commits on this branch. **(Proposal)** marks my own choices, vetoable at the demo.

## Affected crates/modules

- **`curvyo-crypto-core`**: `pake` (new: CPace-ristretto255-SHA512, draft-irtf-cfrg-cpace-21),
  `pair_code` (new: the `XXXX-XXXX` code), `pair_channel` (new: key schedule after CPace, confirm
  MACs, sealed pairing messages v1, check-word indices). No new dependency (`curve25519-dalek`,
  `sha2`, `hkdf`, `chacha20poly1305` are already there).
- **`curvyo-document-core`**: `devices_list` (new: Loro map of devices and workspaces, no Loro type
  in its API, `format_version = 1`, goldens).
- **`curvyo-sync-core`**: frames of relay protocol 3 (`PairOpen`, `PairJoin`, `PairMsg`,
  `PairClose`, `PairSlot`, `PairJoined`, `Refused{unknown|expired|used|locked}`),
  `relay::PairSlots` (clock passed in), `pairing` (opener and joiner state machines, BIP-39 word
  list as embedded data), `devices` (`reconcile`), simulator group rooms.
- **`curvyo-sync-server`**: pairing route on its own WebSocket, slots in memory only.
- **`curvyo-storage-io`**: `devices` (new: `devices/group.json` v1, `keyring.log`, `devices.loro`,
  temp-then-rename; group epoch keys in the credential store in the room-keys encoding).
- **`curvyo-ui-core`**: My devices texts, states, confirms, notices (formatted in Rust).
- **`curvyo-editor-wasm`**: pairing glue, group room session, `reconcile` driver, synced-workspace
  arrival, one connection per synced workspace; browser workspace store glue.
- **`curvyo-app`**: commands for pairing, group files, synced workspaces folder, device name.
- **`frontend`**: Share tab My devices block, pairing blocks, switch row; browser workspace store
  (IndexedDB), browser key custody (M9).
- Docs: `docs/technical-debt.md`, `docs/design-system.md` (new components), privacy statement
  (criterion 30), `docs/requirements.md` (pairing requirement, with the PO).

## Word list licence (checked 2026-11-04)

BIP-39 (`bitcoin/bips`, `bip-0039.mediawiki`) header: "License: MIT", section Copyright: "This
BIP falls under the MIT License." The English list `bip-0039/english.txt` (2,048 words, SHA-256
`2f5eed53a4727b4bf8880d8f3f199efc90e58503646d9ff8eff3a2ed3b24dbda`) is used; the EFF list is not
needed. The file keeps its MIT notice next to it in `curvyo-sync-core`.

## Milestones and tasks

### M1 CPace and the pairing codec in `curvyo-crypto-core`

1. [x] `pake`: `prepend_len`/`lv_cat`/`generator_string`/`transcript_ir` and CPace on
   ristretto255 with SHA-512; scalars from 32 passed-in bytes (bits above 252 cleared, zero
   refused); `scalar_mult_vfy` returns an error for an undecodable or identity share; ISK. Tests:
   every draft-21 vector of A.1 to A.3 and B.3 (generator string, hash, generator, Ya, Yb, K, ISK
   initiator/responder, valid scalar_mult, invalid shares Y_i1 and G.I) (criteria 2, 3, 4).
2. [x] `pair_code`: 8 Crockford base-32 characters, 3 for the relay slot (15 bits) and 5 for the
   secret (25 bits); parse folds case, drops spaces and dashes, maps I/L to 1 and O to 0, refuses
   U and any other length; format `XXXX-XXXX`; the secret from 4 passed-in bytes. Property tests:
   format then parse is the identity (criteria 1, 4).
3. [x] `pair_channel`: CPace with password = the 25-bit secret, `sid` from the relay, channel
   identifier `lv_cat("curvyo-pair-v1", server URL)` **(Proposal: lv_cat instead of plain
   concatenation)**; joiner B is the CPace initiator. HKDF-SHA-256 from ISK and `sid` to
   `confirm_a`, `confirm_b` (MACs over the transcript), `seal_a`, `seal_b` and the check-word
   bits; constant-time MAC check; sealed messages v1 (XChaCha20-Poly1305 with counter nonces per
   direction: keys are fresh per pairing): `Hello{identity, label, signature over the transcript}`
   and `Welcome{group room id}`; four 11-bit word indices over the transcript and both identities.
   Tests: both sides agree; a wrong code, swapped share, swapped identity, changed label or replayed
   message fails; golden files `pairing_*_v1.bin` (criteria 2, 3, 4, 30).

### M2 devices list in `curvyo-document-core`

4. [x] `devices_list`: `devices.<member hex> = {label}`, `workspaces.<id hex> = {name, on}`, last
   writer wins; read, set label, switch on/off, remove device; snapshot and update bytes; refuses
   a higher `format_version`; goldens `devices_list_v1.loro` and damaged variants (criteria 6, 8,
   9, 11, 12, 13, 22).

### M3 relay protocol 3 in `curvyo-sync-core`

5. [x] Frames `PairOpen`, `PairJoin{slot}`, `PairMsg{bytes}`, `PairClose{reason}`, `PairSlot{slot,
   expires_in_s}`, `PairJoined{sid}`, `Refused{unknown|expired|used|locked}`;
   `PROTOCOL_VERSION = 3`, 1 and 2 still admitted; goldens (criteria 1, 4, 5).
6. [x] `relay::PairSlots`: random free slot below 32,768 and a fresh `sid` per join from passed-in
   entropy (follow-up after M4: unique sids per `CPace` run, `PairSlot` carries no sid), 5-minute
   lifetime by the passed-in clock, one slot per connection, 5 joins per slot, 4,096 slots per
   server, `used` until the lifetime ends, forwarding between the opener and the one current
   joiner; nothing stored or logged (criteria 4, 5, 30).

### M4 pairing state machines in `curvyo-sync-core`

7. [x] `pairing`: opener (A) and joiner (B) machines over `pair_channel` and the frames; A counts
   wrong MACs and locks after 5 in a row; Pair writes the group genesis if needed and
   `AddMember{B, admin}` with wraps of every group epoch, then `Welcome`; Cancel and Words differ
   close; the word list (BIP-39 English, MIT) maps indices to words. Simulator tests: A and B pair,
   wrong code, lock, expiry, A offline while B waits, a third device pairs through B, no private
   key byte in any frame (criteria 1 to 6). As built: the server address is normalised before it
   enters the channel id (`normalize_server_address` moved from `ui-core::share_text` to
   `sync-core::server_address`, which also folds an upper-case scheme); `AddMember` is written by
   the new `ClientSession::add_member` (genesis by `SessionStart::Share`), then
   `PairOpener::pair` sends `Welcome`.

### M5 `reconcile` in `curvyo-sync-core::devices`

8. [x] `reconcile(group keyring, switched-on ids, workspace keyrings, own identity) -> entries`:
   add group members missing in a workspace with this device's role and every held epoch; remove
   former group members, then `NewEpoch` and a snapshot request; rotate when a current epoch is
   wrapped to a non-member; remove the other group devices in a workspace switched off; nothing
   where this device is not admin, reported per workspace (criteria 6, 9, 11, 13, 16, 17, 23, 26,
   27).
9. [x] Exhaustive three-device tests: every interleaving of pair, switch on/off, remove, leave and
   offline replay over devices A, B, C; invariants: a removed or left device is never re-admitted
   and holds no later epoch, every group member of a switched-on workspace ends a member, at least
   one admin remains, two devices reconciling at once converge by ADR 0010 §5 (criteria 23 to 27).
   As built: `reconcile` also rotates the group room after a device left; it plans nothing for a
   room whose admin set the replay restored (ADR 0010 §5 ignores the removals there, so writing
   more would never settle). `ClientSession` gained `admit_device` (re-admits a device a
   switch-off removed), `remove_members` (one rotation for several), `rotate` and `leave`; a
   session counts its own removal only after the catch-up, so a re-admitted device is not ended
   by its old removal. The exploration: depth 6 with 3 maker actions in debug builds, depth 8
   with 4 in release (about 9,000 states).

### M6 server

10. [x] `curvyo-sync-server`: pairing WebSocket route (first frame not `Hello`) driving
    `PairSlots`; integration tests with real sockets: pair, refusals, limits, slots never on disk,
    no code in the log (criteria 1 to 5, 30). As built: the hub hands a connection whose first
    frame is `PairOpen` or `PairJoin` to `PairSlots` for its whole life (same `/` route); a
    one-second tick expires slots without traffic.

### M7 desktop end to end: storage-io, editor-wasm, app, ui-core, Share tab

11. [x] `storage-io::devices`: `group.json` v1 codec and goldens, keyring log, devices list file,
    group keys in the credential store (criteria 2, 31).
12. [x] `ui-core`: My devices rows, states (5.1), confirms (6.2, 6.3, 7.1), notices, reasons,
    the persistent one-device warning (criteria 1, 2, 5, 22, 23, 26, 27, 28).
13. [x] `editor-wasm` and `curvyo-app`: pairing glue, group connection, `reconcile` driver after
    start, keyring change and switch; switch on (Share if needed, reconcile, then list entry);
    arrival under the synced workspaces folder with " 2", connect-by-id offer, safe file names;
    remove, leave, removed notice; one connection per synced workspace (criteria 1 to 28, 31).
14. [x] Share tab: My devices block, Add a device / Join my devices blocks, compare step, workspace
    pick list, switch row, Remove and Leave confirms (UX notes 2 to 8, 10).

    As built (M7), with the gaps the lead should know:
    - `editor-wasm`: `PairingGlue` and `DeviceGroupGlue` (group room session, devices list,
      add/remove/leave, files and keys, join retry while the opener's `AddMember` is on its way),
      `Session::reconcile_workspace` and `join_synced_workspace`; bindings in `wasm_devices`.
    - Page: `useDevices` keeps the group connection and one pairing connection on their own
      `SyncTransport`s; `MyDevicesBlock` in the Share tab; the switch row in
      `WorkspaceShareBlock`.
    - **Gap: only the open workspace is reconciled and synced.** ADR 0040 decision 7 (one
      connection per synced workspace, opened or not) is not built: the 0095 link lives in the
      open document's session. A removal or a newly paired device reaches a workspace when it is
      next open on an admin device; "Removal incomplete" and "Waiting for first copy" are not
      shown yet.
    - **(Proposal)** The pick list after Pair offers the open workspace only; others are switched
      on from their Workspace block.
    - **(Proposal)** Arrival on the new device runs when no workspace is open there; it opens the
      arrived workspace (the 0095 join does), retries every 30 s while the admin device has not
      admitted it yet, and reuses an empty folder left by a try that did not get in.
    - Not built yet: the decision row "Connect this folder" (criterion 14), "Rotation pending" /
      "Key renewal pending" lines, per-workspace detail lines of a row, last seen across
      restarts, safe document file names on arrival (criterion 15; folder names are safe).

    M7 gaps closed (2026-11-04, second pass):
    - ADR 0040 decision 7: `lib/backgroundWorkspace.ts` runs one session and socket per
      switched-on workspace that is not open (resume from its marker, or arrive), driven like the
      open workspace (0095 background writer worker, disk plan, marker, keys); `hooks/
      useSyncedWorkspaces.ts` starts, reconciles (every admin device, every synced workspace),
      retries and stops them; the editor claims a workspace before it resumes it
      (`claimOpenRoom`), so one connection per workspace. Host commands take an optional
      `workspace` id (`AppState` registers synced folders by id; `register_synced_workspace`,
      `create_synced_workspace`, `find_workspace_copy`, `workspace_summary`); an arriving
      workspace is created closed and listed in Recent (ux 4.4).
    - After Pair: open and recent workspaces offered, all checked, synced ones named above the
      list; one that is not open is shared in the background (`lib/workspaceShare.ts`, shared
      with Share workspace) and switched on (criterion 8).
    - Connect this folder / Make a second copy (criterion 14): a copy found by id in Recent or
      the Synced workspaces folder raises the decision row; Connect joins into it and each
      document that is there meets the room's in its file through the background writer.
    - Rows: per-workspace detail lines, own state (worst wins), "Waiting for first copy",
      "Removal incomplete" rows with their alerts, "Key renewal pending" (`Session::
      device_facts`, `DeviceGroupGlue::view_json` in `device_group_view.rs`); Leave writes the
      own removal in every synced workspace (`leave_synced_workspace`).
    - Criterion 15: names are made safe by the list view already; arrival now says "Renamed a:b
      to a_b on this device." (`WorkspaceList::stored_name`).
    - Budget: `device_budget_tests.rs`, 20 documents of 1,000 objects arriving in a background
      session and live edits: slowest received frame 3.8 ms in release (16 ms asserted there).

### M8 browser as a device: workspace store and pairing

15. [x] Browser workspace store in IndexedDB (documents and list per synced workspace, quota
    reporting) and the wasm glue to open and edit from it (criteria 35, 37, 38, 39).
16. [x] Browser pairing as joiner and opener over `wss://`, device name default browser and OS,
    no folder step, one device per profile across tabs (criteria 33, 36, 38, 39, 40).

    As built: `platform/browserWorkspaces.ts` answers every workspace and device-group host
    command from IndexedDB `curvyo-workspaces` (so the page's workspace code runs unchanged;
    the File tab's Workspace section shows once the browser holds a workspace; Save of a
    workspace document writes it there); the scan reads each file's share members through
    `container_share_members` (wasm, `document-core`). A full store raises the alert with the
    numbers; My devices shows what the browser holds. Pairing: no folder step, the loss
    sentence, `navigator.storage.persist()` at join, Add a device only with a group and a
    `wss://` server; a Web Lock makes one tab of a profile the device, other tabs say so.
    **(Proposal)** The browser kind is not stored in the devices list (no format change): its
    default name ("Firefox on Linux") says it; the "Browser, " prefix on other rows is not
    shown. A browser switches on only workspaces it holds (all synced ones).
    Key custody (M9 task 17, custody A) is built with it: `lib/deviceKeyStore.ts`.

### M9 browser key custody and passkey backup (Frage 145 = A)

17. [x] One narrow frontend interface `DeviceKeyStore { names(), load(name), save(name, bytes),
    remove(name) }` with the custody A implementation: IndexedDB `curvyo-keys`, one
    non-extractable AES-GCM-256 key, records `{format_version: 1, name, iv, ciphertext}`,
    `navigator.storage.persist()` at pairing, the loss sentence (criteria 34, 36). The passkey
    backup or accounts of Frage 145 plug in behind this interface once decided. Golden file
    `frontend/tests/fixtures/key_record_v1.json`.
18. Frage 145 answered A (customer, 2026-11-04): passkey backup and restore behind
    `DeviceKeyStore`. No criterion covers it yet: criteria 29 ("no Restore action anywhere"),
    34 and 36 ("a cleared browser is a lost device") need the PO's amendment; the tasks cite
    Frage 145 A and ADR 0040 decision 13. What is backed up (checked in `sync-core::client::
    keys`): a member's epoch keys come back from the keyring wraps on every connect
    (`unwrap_mine`), and the group's local state (`folders`, `server_seq`, keyring log, list)
    is rebuilt by the joiner's path of pairing. So the backup holds the identity entropy and
    the group pointer (server, group room, device name): written once, never stale.
    - 18a. [x] `crypto-core::backup`: the PRF input, HKDF from the PRF output to wrap key,
      record id and write-key entropy; backup record v1 (XChaCha20-Poly1305, the id bound);
      write signatures under their own context over relay nonce, id and blob hash. Golden
      `backup_record_v1.bin`, wrong passkey, changed byte, newer version.
    - 18b. [x] `sync-core`: protocol 3 frames `BackupGet`, `BackupBegin`, `BackupPut`,
      `BackupDelete`, `BackupBlob`, `BackupDone`, refusal `Forbidden`, goldens;
      `relay::Backups` (sans-I/O: the first put pins the write key, a size cap, challenge per
      write, store ops); `backup::BackupExchange` (the client side of one exchange). Tests:
      put, get, replace, delete, replay of a signed put, another write key, too large.
    - 18c. [x] `sync-server`: `backups/` in the data directory (temp then rename), the route by
      first frame, restored at start; socket tests and a scan of the data directory for the
      identity bytes, room id and names.
    - 18d. [x] `editor-wasm` bindings and `ui-core` texts; frontend `lib/keyBackup.ts`
      (WebAuthn create/get with PRF), `hooks/useKeyBackup.ts`, `KeyBackupRow.tsx`: Back up with
      passkey on a paired browser, the honest no-PRF sentence, Restore with passkey on a browser
      without a group, Remove backup (signed by the passkey again).
    - 18e. [x] Browser test `frontend/tests/browser/key-backup.cjs`: Playwright's virtual
      authenticator over CDP (`WebAuthn.addVirtualAuthenticator` with `hasPrf: true`, real PRF
      evaluation, no seam); the desktop opener runs on the wasm bindings in node; a TLS proxy
      gives the browser `wss://`. Pair, back up, clear site data, restore: same fingerprint,
      back in the group without pairing; remove; a passkey without PRF stays custody A.

    As built (M9): the page renders once before the wasm module is ready, and two M8 calls
    formatted words then (`normalize_server_address` at render, the storage line's first
    measure); the browser build crashed on load. Fixed with `wasmLoaded()` / `wasmReady()` in
    `useDevices`; the key-backup check is the first run of the browser build end to end.
    **(Proposal)** Restore uses the device name stored in the backup (the name at backup time);
    the relay keeps records up to 4 KiB; the PRF input is SHA-256 of `"curvyo-backup-v1"`.
### Cleanup after M8 (2026-11-04)

- [x] Criterion 14 end to end: `session::device_sync_tests::
  connect_this_folder_merges_a_copy_with_offline_edits_into_the_room`: B's copy of W with an
  offline edit, Connect this folder (B joins into it, the room's container and the background
  records meet the file in `BackgroundWriter`), B opens the document and A sees B's edit; B's
  file holds A's edit too.
- [x] Module sizes: `device_group_glue.rs` 644 to 519 lines (the session's events moved to
  `device_group_events.rs`; the rest is one type's commands, files and keys: justified);
  `MyDevicesBlock.tsx` 734 to 398 (pairing blocks in `PairingBlocks.tsx`, shared looks and the
  destructive confirm in `devicesUi.tsx`, the backup in `KeyBackupBlock.tsx`); `useDevices.ts`
  837 to 751 (its types in `lib/devicesApi.ts`, the backup in `useKeyBackup.ts`). The rest of
  `useDevices.ts` is justified: one hook owns the group session, its socket and the pairing
  attempt that writes into that session (Pair adds the joiner to the group the hook holds), and
  splitting the pairing out would pass the group, its record and the attach callback across a
  hook boundary for no second user.
- [x] The `0051` tester commit that failed on the app commit is made moot by the squash (M10).

### M10 journey, docs, PR

19. [x] Journey `frontend/tests/browser/own-devices.cjs`: a local relay, two desktop devices
    with a faked desktop host (as `workspace.cjs`): pair with code and check words (both screens
    show the same four), Sync these, arrival in the Synced workspaces folder, edits both ways,
    the switch off (the confirm names B) and on again with Connect this folder sending B's edit
    of a closed document, Remove, the one-device warning, and a scan of the relay's data and log
    for the `ZQX-*` markers and the code (criteria 1 to 3, 8 to 10, 13, 14, 18, 23, 24, 28, 30). The browser device,
    its passkey backup and restore and its browser mark are `key-backup.cjs` (criteria 33, 34,
    36, 40). Three instances, offline, restart and leave stay with the Rust simulator tests
    (`device_sync_tests`, `devices_reconcile`, `device_group_glue_tests`).
20. [x] Docs: debts (frozen group, slot quota), design system (My devices), ADR 0040 amended,
    `adrs.md` accepted, requirement R-SYS-012, spec criteria 29, 34, 36, 38 amended (Proposal),
    `specs/README.md` row. Full gate; the PR is the lead's.

### Review fixes (2026-11-04: architecture, crypto, UX reviews)

- Key custody: `load` throws `UnreadableKeyRecord` for a record that does not open; the
  identity is made only when absent, with IndexedDB `add` and a read-back (two tabs keep one);
  `read_device_keys` answers `failed` instead of minting a new identity. Restore refuses on a
  browser in a group before writing; Back up again asks the marked passkey; Leave deletes the
  backup; a passkey that reports PRF but gives none is forgotten.
- `reconcile`: a group device removed from a workspace is added again only when every removal
  since its last add was written by an identity that ever was in the group (tester finding 1,
  2026-11-05; first the review's "current group member"; the exhaustive run passes with it in
  debug and release); others are `withheld` in the plan, and
  `admit_device` refuses them (`may_readmit`). Restored rooms get
  adds and rotations, no removals; the exhaustive invariants run in restored rooms too, with a
  member D of another maker that must keep the current key on every path. The model publishes a
  removal at once, so it reaches restored workspace rooms (53 states in debug) but no restored
  group; the focused test covers the group. The added role is the
  lower of the writer's and the group's. `a_mutual_removal_of_the_only_two_devices_freezes_the
  _group` shows the frozen group of decision 8 (Frage 146 default A).
- Criterion 14 for a copy taken before the workspace was shared: the background writer gives
  the merged file its `share.json` v2 (`write_for`), returns the file's own changes the room
  lacks, and the page sends them through `Session::workspace_send_local`, which marks the file
  synced **(Proposal: a push still queued when the app closes is not sent again; the old
  `write` stays for the `0095` tester tests)**. Tests for a document opened afterwards and one
  never opened.
- UX: own-row rename (F2, double-click, `EntryField`), focus returns for every confirm and
  pairing block, the joiner's compare group takes focus, the code group is named by its
  characters with a polite "1 minute left", the switch-off confirm is the destructive confirm
  with the offline refusal, "Uploading 3 of 14", the switch failure alert, the off confirm names
  the people who keep sharing, the on confirm names documents taken in, "Synced, 2 devices" in
  the Workspace block and the File tab header, "my device" in the participant list without
  Remove, member-only workspaces unchecked with "Not an admin here", the removed alert in both
  tabs with one OK, joiner errors (locked clears and locks the field, wrong selects, Try again),
  the browser glyph from an optional `browser` field in the devices list (no new
  `format_version`; old readers ignore it), "A device" for an unnamed participant.
- Found by the journey and fixed: a fresh devices list exported its changes from its own first
  version, so a joined device's names never reached the others; a device that was added writes
  its own name only after reading the adder's entry (last writer wins); Recent workspaces is
  read again after an arrival; the group files are not written after the group was let go; the
  name typed at Join is the one sent; the open workspace switched off shows "Box job is no
  longer synced. Your copy stays here." (not the other-maker removal block) and drops its folder
  from the group record, so switching on again offers Connect this folder (ux-notes 6.4).
- Open: `switchOff` checks the group's connection only; with the workspace room offline the row
  stays on "Stopping..." until the room reconnects.
- Sizes: `devices_text.rs` split (pairing words in `pairing_text.rs`, 455 and 155 lines without
  tests); `device_group_glue.rs` 542 lines (one type's commands, files and keys, justified as
  before); `MyDevicesBlock.tsx` split (`DeviceChoices.tsx`). Not done: decoding the first frame
  twice in the server's hub (optional in the review).

### Tester fixes (2026-11-05)

- [x] Finding 1 (criteria 16, 24): `may_readmit` accepts a removal whose author ever was in the
  group (`role` or `was_removed` in the group keyring); ADR 0040 decision 5 amended; the
  exhaustive run passes in debug and release.
- [x] Finding 2 (decision 11): `DevicesList::import_updates` scans the bytes on their own and
  merged into a fork for a newer `format_version` and refuses them before merging; the group
  glue keeps its list, shows "Another device uses a newer Curvyo. Update this one to see changes
  to your devices." once **(Proposal)**, and uploads no group snapshot in that session (after a
  restart it would again, without the refused change: open point for the lead).
- [x] Finding 3 (criterion 15): `safe_folder_name` cuts to 244 bytes on a character boundary,
  leaving room for the longest " N" suffix.

## Validation

- Draft-21 test vectors for CPace and its string helpers; golden files for every new format
  (pairing messages v1, frames of protocol 3, devices list v1, `group.json` v1, browser key
  records v1).
- Exhaustive three-device tests for `reconcile`; simulator tests for pairing.
- Timing tests in thread CPU time; budgets asserted in release only.
- Manual demo: three desktop instances and one browser against one server.
