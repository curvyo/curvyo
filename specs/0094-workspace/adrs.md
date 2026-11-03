# ADRs for the workspace

Architect, 2026-11-03, revised the same day for the customer's answers WS-VISIBLE A,
WS-DELETE-ALL A, WS-SEPARATE B and WS-META ("nothing but ciphertext"). Covers slice 1 (local
workspace, sharing the whole workspace). Constants marked Proposal are vetoable at the demo.

**One ADR (0036, Accepted), no new crate, one new dependency (`trash`), relay protocol 2 (one new
frame `SnapshotPart`, protocol 1 still accepted), `share.json` version 2; the `.curvyo` container
`format_version` stays 34. One room per workspace; documents are streams inside the sealed
records.**

## Depends on

- [ADR 0036](../../docs/adr/0036-shared-workspace-list-rooms-and-keys.md) (new): marker directory,
  list replica, one room per workspace, inner stream header, multi-part workspace snapshots,
  background worker, invite `2.`, take-in, formats.
- [ADR 0031](../../docs/adr/0031-sync-engine-relay-protocol-and-transport.md): frames, auth,
  join by `RewrapEpoch`, removal with rotation (6a), snapshots and pruning, server store. Extended
  by 0036 decision 7 (`SnapshotPart`, staged parts).
- [ADR 0010](../../docs/adr/0010-document-keyring-admins-and-revocation.md): applies to the
  workspace keyring unchanged; `document_id` = workspace id.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §1, §3, §9. Unchanged.
- [ADR 0008](../../docs/adr/0008-end-to-end-encryption-of-sync-and-collaboration.md): everything
  sealed; visible metadata in 0036 decision 11.
- [ADR 0023](../../docs/adr/0023-raster-trace-crate-pipeline-and-worker.md) §5: Web Worker pattern.

## Per criterion

- **1, 2, 23, 24**: `workspace.toml` holds `format_version = 1` and a random 16-byte `id`; higher
  version refused without writing; Share writes a fresh id once (Proposal); shared state beside
  it: `share.json`, `keyring.log`, `list.loro`, `index.json` (decisions 1, 4).
- **3, 12**: a `0092` document opens on its own room by Open or by a click in a local workspace;
  a workspace document opened by Open alone opens local and syncs later (Proposal, decision 3).
- **5, 7**: scan of names, sizes, mtimes; polling every 2 s (Proposal, decision 2).
- **15, 36**: identity is `document` in `share.json` v2, never name or path (decision 3).
- **17, 40**: OS trash via `trash`; shared delete is a tombstone; the document's state stays in
  the room and every snapshot, Restore rewrites the file from it (decision 6).
- **20**: `recent-workspaces.toml` in the app data directory (decision 1).
- **25, 26, 27, 29**: genesis in the workspace room; invite `2.`; join as COOP-JOIN A into an
  empty folder outside any workspace (decision 9).
- **28, 35**: join reads one multi-part snapshot and writes each document as soon as its bytes
  are complete (decisions 7, 8).
- **30, 31, 32, 33**: one connection per workspace; open document and list applied on the main
  thread, every other document written by a background worker (Proposal: at most every 5 s per
  document); no limit of 4 rooms any more (decision 8).
- **34**: the relay sees one room per workspace, record sizes and times, snapshot part count and
  total size, member identities and connect times; no document count, grouping, names, folders
  or which document a member edits (decisions 5, 11).
- **37, 38, 39**: Loro movable tree; loop-closing move dropped everywhere; name registers,
  collision suffix derived on read (decision 6).
- **41**: outside files are "Not in the shared workspace"; files moved by another program follow
  their `document`; a removed file is "Missing on this computer" (Proposal, decision 6).
- **42 (WS-SEPARATE B)**: take-in only if every member of the document's own keyring is a
  workspace member, else refused (Proposal); then `Moved` in the old room, which stops all
  admission there (decision 10).
- **43, 44, 45, 46**: one epoch sequence; one workspace snapshot under the new epoch; the relay
  closes the removed identity in the one room (decision 9).
- **47, 48**: several workspaces and single documents are separate rooms; one reconnect per
  workspace (decisions 4, 8).

## Notes

- 2026-11-03: WS-META answered "towards B": the draft's `Bind`, `Heads`, `Head`, per-room keys
  (`derive_room_key`) and the 4-room background schedule are gone; WS-SHARE B's "one room per
  document" becomes one stream per document inside the workspace room.
- 2026-11-03: the relay keeps accepting protocol 1, since protocol 2 only adds `SnapshotPart`
  (replaces the draft's "refuses 1").
