# Plan for the sharing concept (workspace only)

Implementer, 2026-11-09. One PR, commits in the customer's order: A join rule, B Share tab
stays, C reachability poll, D the removal and the new model, E status.

## Affected crates/modules

- `curvyo-sync-core`: `relay/admission.rs`, `relay/room.rs` (A); `client/*`, `workspace_stream.rs`,
  `lib.rs` (D, M4: History, return check, `Moved`).
- `curvyo-crypto-core`: `keyring/mod.rs` (`withheld`, `admitted_after`).
- `curvyo-document-core`: `container_share.rs`, `container_ended.rs`, `container.rs` (legacy read
  rules, legacy flag), goldens.
- `curvyo-storage-io`: `workspace_documents.rs` (no keyring member), `workspace_create.rs` (new:
  Make a workspace for this file).
- `curvyo-app`: one command `make_workspace_for_file`.
- `curvyo-editor-wasm`: workspace Share in place (B), document entry points, take-in, return
  check, `SyncEnd::Moved`, wasm bindings; `sync_test_support` ported to a workspace pair.
- `curvyo-ui-core`: Share tab texts (one Join, device word, `1.` and browser refusals, Sync
  confirm, people rows, legacy notice).
- `frontend`: `ShareTab`, `WorkspaceShareBlock`, `ShareInviteJoin`, `ParticipantList`,
  `useCollaboration`, `useWorkspaceSync`, `workspaceShare`, `movedPush` (gone), journeys.

## Tasks

- [x] 1. Relay: a removed claimant with a claimed invite is refused as removed; a claim admits
  only when the replayed keyring lists the claimant; three relay tests (review findings 1-3).
- [x] 2. Share workspace and Join run on the open session: the open document takes its record
  in place, an attempt's link is dropped, Join keeps the document; `keptTab` gone; journey
  records every tab selection (AC 20).
- [x] 3. Reachability poll every 5 s while the Share tab shows a server and no link (AC 21, 24).
- [x] 4. Frontend: no Document block, no Open shared document, no take-in or `Moved`; one Sync
  switch with one confirm, Invite person turns Sync on in its confirm; Sync off greyed with
  others; people grouped by label; one Join field (link or code); `1.` and browser `2.`
  refusals; "Make a workspace for this file" (AC 1-3, 7-19, 22).
- [x] 5. `curvyo-editor-wasm` and `curvyo-ui-core`: port `shared_pair` and the kept session tests
  to a workspace pair; delete the document entry points, take-in, return check, `Moved`,
  document ended write (AC 1, 23).
- [x] 6. `curvyo-sync-core` and `curvyo-crypto-core`: `History`, return check, `Moved` codec,
  `withheld`, `admitted_after` out; a returning member gets an epoch only a third member held
  (AC 23).
- [x] 7. `curvyo-document-core` and `curvyo-storage-io`: legacy share members read as local with
  a notice flag, save drops them; goldens retargeted; `ShareMembers.keyring` out (AC 4-6).
- [x] 8. Journeys: `cooperative-editing.cjs` on a shared workspace (AC 23); `workspace.cjs`,
  `membership.cjs`, `own-devices.cjs` without document sharing; tab recording for Make a
  workspace, Sync on, Invite person, Join and Add device (AC 20).
- [x] 9. `ux-notes.md`, `design-system.md`; status of 0092, 0095, 0096, 0097, 0105; debt.

## Notes from the build

- Share workspace runs on the open document's session and the document takes its record in
  place (`adopt_workspace_document`); Join closes only the tree. That removes the cause of the
  Document-tab flash (a replaced session starts on Document, `0043` criterion 5).
- Sync off ends this device's link too and drops the marker's share files; Sync on again is a
  new workspace, so an old copy elsewhere arrives as a second folder (debt entry).
- Share adopts the record into the open session before it writes the files, and a file saved
  meanwhile keeps its bytes and only gets its record (`restamp_workspace_record`); a Save keeps a
  workspace document's place in the tree.
- The Sync confirm says "leave this device encrypted" (criterion 18 forbids "computer").
- A workspace join's refusals (used, expired) now show as the join's phase; "not a member" stays
  a notice.
- Offline pictures of a workspace document go out again on a reopen with unsynced edits, as the
  document resume did.

- Customer feedback on PR #298: the Sync switch is gone; Invite person starts sharing and is
  greyed with a reason from `invite_blocker` (Rust, ui-core texts) whenever it cannot work, also
  with no workspace open; "Sync to my devices" stays for a device group.

## Validation

Golden files `shared_v34.curvyo`, `ended_v36.curvyo` (legacy inputs, `open_writes_nothing`),
`workspace_marker_share_v1.json`, `workspace_shared_v35.curvyo` (unchanged). The five browser
journeys of `frontend/tests/browser` (cooperative-editing, workspace, membership, own-devices,
key-backup) against a local `curvyo-sync-server`; the full gate of `CLAUDE.md` §7 once.
