# ADRs for own-devices sync

Architect, 2026-11-04, for the customer answers DEV-IDENTITY A, PAIR-METHOD A, SYNC-SCOPE B,
DEV-ROLE A, DEV-LOST A, DEV-UI A, DEV-BROWSER B. Covers slice 1. Constants marked Proposal are
vetoable at the demo.

**One ADR (0040, Accepted 2026-11-04: ADR-0040 A, BROWSER-SLICE A, Frage 145 A; Frage 146
default A), no new crate, no new crate in `Cargo.lock` (`subtle` made direct in `crypto-core`),
no new keyring payload, relay protocol 3 (pairing and backup frames; 1 and 2 still admitted;
upgrade the server first), `.curvyo` `format_version` unchanged. A synced workspace is an ADR 0037 shared workspace; the
maker's devices form one group room; each admin device admits and removes group members in its
workspaces by one pure `reconcile`.**

## Depends on

- [ADR 0040](../../docs/adr/0040-own-devices-pairing-and-device-group.md) (new): group room and
  devices list, pairing by CPace and check words, `reconcile`, remove and leave, browser custody,
  what the relay links, passkey backup of a browser device (decision 13).
- [ADR 0037](../../docs/adr/0037-shared-workspace-list-rooms-and-keys.md): Share, room, snapshot,
  worker, removal; unchanged.
- [ADR 0031](../../docs/adr/0031-sync-engine-relay-protocol-and-transport.md): frames and auth
  (extended by protocol 3), 6a rotation, credential store and key file (decision 12).
- [ADR 0010](../../docs/adr/0010-document-keyring-admins-and-revocation.md): every keyring entry
  used here exists; replay and §5 merge unchanged; device model of §1 kept.
- [ADR 0008](../../docs/adr/0008-end-to-end-encryption-of-sync-and-collaboration.md): §9's per-user
  sync key and recovery key not built (DEV-LOST A); note added on acceptance of 0040.
- [ADR 0007](../../docs/adr/0007-asset-library-connector-and-credential-storage.md) §1 and the
  no-accounts boundary: kept; the passkey backup is an opaque record, no account (Frage 145 A).
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §7: "cloud" sync path is
  realised by this ADR for workspaces.

## Per criterion

- **1**: code `XXXX-XXXX`, Crockford base 32; 3 characters relay slot, 5 characters secret; slot
  lifetime 5 minutes (Proposal) (decisions 3, 4).
- **2**: CPace then sealed identities and labels; check words 4 x 11 bits from the BIP-39 English
  list (Proposal, licence checked by the implementer); B gets no group key before Pair (4.3).
- **3**: B generates its keys; only public keys travel; Pair writes `AddMember{B, admin}` with
  wraps in the group room (4.4, 1).
- **4**: relay `Refused{unknown | expired | used | locked}`; 5 wrong MACs lock the code (A
  counts), 5 joins per slot (relay counts); slots in memory only (3, 4.3).
- **5**: pairing needs the relay and both devices online; nothing pairs on its own.
- **6**: B is admin of the group room and pairs C; A learns C from the group keyring; every admin
  device's `reconcile` admits C into its workspaces (1, 5).
- **7**: nothing is sent before Add, Join or a switch; group genesis at Pair (1).
- **8, 9, 12**: switch per workspace in the devices list; on = Share if needed, `reconcile`, then
  the list entry; off or absent = nothing sent (6).
- **10, 11, 14, 15**: arrival under `synced_folder` from the list entry; B already a member when it
  reads it, so no one online is needed; connect-by-id offer; names as ADR 0037 decision 6 (6).
- **13**: off removes the other group devices with one rotation (5, 6).
- **14**: a copy from before the workspace was shared gets its `share.json` v2 from the background
  writer when it merges; its own edits go out as the document's records, also for documents
  never opened on that device (6).
- **16**: a device another maker's admin removed is not added back (`may_readmit`, 5); one
  removed by an identity that ever was in the group is.
- **16**: `AddMember` with the admitting device's role; other makers see that entry and the sealed
  presence label only (5).
- **17, 26**: `reconcile` writes nothing where this device is not admin and reports it (5).
- **18 to 21**: ADR 0037 and 0031 unchanged; one connection per synced workspace plus one group
  connection while the app runs (7).
- **22**: last seen from the relay's `Peer` events and the group room; labels from the devices
  list.
- **23 to 25**: group `RemoveMember` + `NewEpoch`, then `reconcile` per workspace (0031 6a); any
  admin device online completes the rest; a mutual removal of the only admins freezes the room
  until the group is set up again (8, Frage 146 default A).
- **27**: own `RemoveMember` in group and workspaces; next admin device rotates (5, 8).
- **28**: UI only.
- **29**: no recovery code (DEV-LOST A); a browser device's passkey backup and restore (13).
- **30**: decision 10: the relay sees the group keyring, so it knows which identities are one
  maker's devices, more than co-occurrence; names, labels, workspace room ids, code secret sealed
  or never sent. The confirm text should say "the server can tell which computers belong
  together".
- **31**: group keys in the credential store; `devices/` in the app data directory; browser in
  IndexedDB wrapped by a non-extractable AES-GCM key, room keys as JSON `[[epoch, hex]]`; an
  unreadable record is refused, never replaced (2, 9).
- **33 to 40**: browser pairing over `wss://`, custody A, the browser workspace store in
  IndexedDB, the `browser` mark in the devices list (1, 9); passkey backup and restore (13): the
  modules `crypto-core::backup`, `sync-core::backup`, `relay::Backups`, the server's `backups/`.
- **32**: unchanged.

## Notes (smaller than an ADR)

- 2026-11-04: a device in a group cannot join another group; merging groups is out of scope
  (Proposal).
- 2026-11-04: a browser can join a group but not start one (Proposal), so a group always has had a
  desktop with plaintext files.
- 2026-11-04: slice 1 needs no account, including the browser (decision 9); the passkey backup
  is an opaque record under a derived id (decision 13, Frage 145 A).
- 2026-11-04: a browser switches on only workspaces it already holds (criterion 38 as built).

## Questions (in ADR 0040)

- Frage 142 (BROWSER-KEYS): replaced by Frage 145. Frage 143 (BROWSER-SLICE): answered A.
- Frage 145 (BROWSER-ACCOUNT): answered A, decision 13.
- Frage 146 (GROUP-DEADLOCK): unanswered, default A (decision 8: dissolve and set up the group
  again; debt entry; keyring rule unchanged).
