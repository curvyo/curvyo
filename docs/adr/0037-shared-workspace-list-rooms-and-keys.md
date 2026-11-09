# ADR 0037: Workspace on disk, shared workspace list, one sealed room per workspace

**Status:** Accepted 2026-11-03 (customer), revised for WS-META/WS-SEPARATE. Customer answers
(2026-11-03): option A of the first draft in principle; WS-VISIBLE A (one membership for the whole
workspace), WS-DELETE-ALL A (any member deletes, restorable), WS-SEPARATE B (a document shared on
its own is taken into the workspace), WS-META "the server sees nothing but ciphertext, towards B".
This revision replaces the draft's bound rooms (`Bind`, `Heads`, per-room keys) with one room per
workspace. Asked by `specs/0095-workspace` slice 1. Extends ADR 0031 (rooms, frames, snapshots,
invite versions) and ADR 0004 §1 (files beside the documents); amends nothing accepted. Choices
marked (Proposal) are the architect's, vetoable at the demo.

**Superseded in part by [ADR 0044](0044-workspace-only-unit-of-sharing.md)** (customer, 2026-11-06 DOC-SHARE-REMOVE B, accepted 2026-11-09): WS-SEPARATE B and decision 10 (take-in) entirely; in decision 11 the sentences on a member also in a `0092` room and on take-in timing; in decision 12 the `Moved` payload. A `0092` file in a workspace folder is an ordinary workspace document.

## Context

ADR 0031 shares one document per room; the relay checks every envelope against the room's keyring
(ADR 0010) and prunes the log at the newest client snapshot. A shared workspace needs one
membership over many documents, a sealed list of names and folders that several members change at
once, and documents that keep their identity when a file is renamed or moved. Constraints:

- WS-META: the relay may learn no grouping of rooms, no document count, no name or folder and not
  which document a member works on. Only sizes and timing of ciphertext may remain.
- The relay stays blind (ADR 0008) and a thin shell around `RoomLog` (ADR 0031 decision 5).
- `document-core` holds the only Loro dependency (ADR 0004 §3).
- Every member keeps every document on disk (WS-VISIBLE A, criterion 28), so every member needs
  every change anyway; selective fetching saves nothing.

### Options — how documents travel

**A. One room per document, bound to a workspace room** (first draft). Rejected by WS-META: the
relay learns which rooms belong together, the document count and which room a member syncs when.
**B. Unlinkable per-document rooms** (no `Bind`, independent auth per room). Rejected: the same
identity key authenticates in every room (ADR 0031 auth), so the relay links the rooms by member;
hiding that needs a fresh identity per room, i.e. N keyrings kept identical (the draft's rejected
option B), and a member still opens room 7 when editing document 7.
**C. One room per workspace; every document is a stream inside the sealed records** (chosen). The
relay sees one log of ciphertext. Cost: one log carries all documents, so a snapshot carries the
whole workspace; decision 7 bounds that cost. A joiner downloads the whole workspace, which A does
too.

### Options — the shared document list

**A. A Loro movable tree replica, carried as one stream of the workspace room** (chosen; ADR 0004's
reason for Loro). **B. One record file per document with a `path` field.** Rejected: two concurrent
folder moves and a concurrent loop have no rule (criteria 37 to 39).

### Options — compaction of the one log

**A. Whole-workspace snapshot in fixed-size parts, triggered by a size ratio** (chosen). The relay
prunes as in ADR 0031; upload cost stays proportional to the bytes edited.
**B. Per-document snapshots inside the log.** The relay cannot tell which older records a document
snapshot supersedes without being told, and telling it (`keep`/`drop` lists of sequences) reveals
which records form one document, hence the count. Rejected.

## Decision

1. **Marker directory** `.curvyo-workspace/` in the workspace folder. Always present:
   `workspace.toml` = `format_version = 1` and `id = "<32 hex>"` (16 random bytes), nothing else;
   the workspace's name is the folder's name. A reader refuses a higher `format_version` without
   writing. Present only once shared (ADR 0004 §1 pattern): `share.json` (ADR 0031 decision 9's
   shape, `room` = workspace id, `server_seq` = the workspace cursor), `keyring.log`, `list.loro`
   (list snapshot, temp-then-rename), `index.json` (cache of document id → path, size, mtime,
   `format_version = 1`; rebuilt from the documents' `share.json` when missing, damaged or newer)
   and `deleted/<document id hex>.curvyo` (the kept copy of each document a member deleted, written
   before its file goes to the trash, read by Restore).
   Recent workspaces: `recent-workspaces.toml` in the app data directory, beside
   `font-library.toml`. No library folder (WS-LIBRARY A).
2. **A local workspace is a folder scan; a shared workspace is its list.** Before sharing, the tree
   is the recursive scan of `.curvyo` files (names, sizes, mtimes; no container opened), polled
   every 2 s (Proposal: no watcher dependency). After sharing, the list decides which documents
   exist and where; the disk follows it (decision 6).
3. **Document identity is a document id in the document's `share.json`.** A workspace document's
   container holds `share.json` **version 2**: `{format_version: 2, workspace, document,
   synced_version}`; no `server`, `room`, `server_seq` or `keyring.log`. `document` is 16 fresh
   random bytes minted when the document enters a shared workspace (Share, New, Duplicate, Add to
   workspace, take-in) and never derived from name or path, so rename and move keep it (criterion
   15). Two files with the same `document` (a copy by another program): the one the list's path
   names keeps it, the other shows "Not in the shared workspace" and gets a fresh id on "Add to
   workspace". A workspace document opened by Open outside its workspace opens as a local file;
   its `synced_version` makes its edits go out the next time the workspace syncs it (Proposal).
4. **One room, one keyring, one key per epoch.** Share writes a fresh workspace id into
   `workspace.toml` (Proposal: a folder copied before sharing cannot collide on the relay) and a
   `Genesis` with `document_id` = workspace id. The workspace room is an ADR 0031 room unchanged:
   one keyring, one admin set, one epoch sequence (criterion 43), envelopes sealed with the epoch
   key and `NonceStream` nonces. No per-document key: every member may read every document
   (WS-VISIBLE A), and one room has one nonce space as in ADR 0031. `crypto-core` is unchanged.
5. **Streams inside the records.** The sealed plaintext of every workspace `update` or `asset`
   record starts with an inner header `stream [16], peer u64`: `stream` is the document id, or the
   workspace id for the list replica; `peer` is the Loro peer id. The cleartext envelope header
   carries `document_id` = workspace id and `sender_peer = 0` for every workspace record, so the
   relay cannot group records by session or document. Presence frames carry the open document's id
   only inside the seal. Codec: module `workspace_stream` in `curvyo-sync-core`.
6. **The list replica and the disk** (unchanged from the draft except keys).
   - Module `workspace_list` in `curvyo-document-core`, no Loro type in its API: a Loro tree; node
     meta `kind` (folder | document), `name` (register, no extension), `document` (document only),
     `created` (`[unix seconds, 16-byte id]`, written once), `deleted` (absent, or `{by, at}`);
     root register `format_version = 1`. Seam: `export_updates(since)`, `import_updates`, snapshot.
   - Moves: Loro tree moves; concurrent moves converge to one parent; a cycle-closing move is not
     applied, identically everywhere (criterion 39); the client whose move was dropped says so.
   - Names are registers; the shown name is derived on read: among visible siblings of one kind
     equal under `to_lowercase`, ordered by `created`, the first keeps the name, the next get " 2",
     " 3" (criterion 38), never written back. Names with `/ \ : * ? " < > |`, control characters,
     a trailing dot or space or a Windows reserved name are refused at input on every OS; one
     arriving from a peer is written to disk with `_` in place of each.
   - Delete sets `deleted` (tombstone); a node under a deleted folder is hidden with it; Restore
     clears it. A tombstoned document's state stays in the room and in every workspace snapshot,
     so Restore rewrites the file from it. Tombstones are never purged in slice 1 (debt).
   - Disk follows the list: module `workspace_plan` in `curvyo-document-core`, one pure function
     from the derived tree and the scanned files, keyed by document id, to file operations
     (create, rename, move, write from stream, trash). The host executes them (`curvyo-storage-io`
     module `workspace`; Tauri commands in `curvyo-app`). A remote deletion moves the local file
     to the OS trash, never erases it, and waits while the document is open with unsaved changes
     (criterion 40). A file another program renamed or moved is recognised by its `document` and
     written to the list as that move; one another program removed is "Missing on this computer"
     with "Download again", not a shared deletion; a new file is "Not in the shared workspace"
     (criterion 41) (Proposal).
7. **Workspace snapshots.** The snapshot is one byte stream: `format_version u8, count u32`, then
   `count` entries `(stream [16], length u64, bytes)`: the list snapshot first, then every document's container
   bytes (`container::pack`, tombstoned ones included). It is cut into parts of 4 MiB, the last
   padded to a multiple of 64 KiB (Proposal); each part is a signed snapshot envelope whose sealed
   plaintext begins `snapshot_id [16], index u32, count u32, length u32` (`length` = the stream
   bytes the part carries before its padding), so part boundaries do not follow
   document boundaries. New frame client → relay `SnapshotPart{covers_seq, index, last, envelope}`
   (relay protocol 2; the relay keeps accepting protocol 1, of which 2 is a superset). The relay
   stages parts under `snapshot.part/`, one upload at a time: the connection whose part 0 started
   the staging owns it; parts of another connection meanwhile are dropped without a refusal (two
   members can reach the threshold at once; the other uploads again at its next threshold), and
   the staging is dropped when its owner disconnects or sends a new part 0. On `last` it swaps
   them in and prunes records up to
   `covers_seq` as in ADR 0031 decision 10, keeping only the highest complete `covers_seq`; it
   serves the parts as consecutive `Record{seq: covers_seq, kind: snapshot}` frames (client codec
   unchanged). A snapshot is due (pure function `snapshot_due` in `curvyo-sync-core`) when the
   tail since the last snapshot reaches max(4 MiB, half the last snapshot's size) or
   max(500, last snapshot size / 8 KiB) records (Proposal): upload stays at most about twice the
   bytes edited, a join reads at most about 1.5 times the workspace.
8. **One connection, a background worker.** A shared workspace keeps one WebSocket open, to its
   room. The main wasm session opens and seals everything: it opens update records and snapshot
   parts, applies those of the open document and the list, and passes the others' plaintext with
   their document id to a Web Worker with its own wasm session (the trace worker pattern, ADR
   0023), which imports them into the files (load, import, save at most every 5 s per document;
   Proposal) and advances the cursor in the marker's `share.json` only after the files are saved
   (Loro import is idempotent, so a crash replays). The worker holds no keys. The main session
   also builds and seals the workspace snapshot (sealing cost: debt). The open document's frame budget is untouched (criteria 31, 32).
   Join writes each document as soon as its bytes in the snapshot stream are complete.
9. **Share, invite, join, removal.**
   - *Share*: genesis; the list built from the scan; one snapshot (decision 7) holding every
     document; each file gets `share.json` v2; the marker gets its files.
   - *Invite* `curvyo://join?server=<url>#2.<base64url(workspace id ‖ invite seed[32])>`: bytes,
     keyring payloads and proof as version `1.` (ADR 0031 decision 8) with `document_id` =
     workspace id. Version `1.` keeps its meaning.
   - *Join*: probe and claim, admission by any online member's `RewrapEpoch` (COOP-JOIN A); the
     joiner picks an empty folder outside any workspace, writes the marker, reads the snapshot and
     the tail.
   - *Removal*: `RemoveMember`, `NewEpoch` (ADR 0031 decision 6a), then one workspace snapshot
     under the new epoch, written by the worker; the relay's pruning then drops every old-epoch
     record. The relay closes the removed identity's connection to the one room (criterion 44a).
     The removed device deletes the marker's share files and keeps its documents as local files;
     a document whose workspace has no share record drops `share.json` on its next save.
10. **Take-in of a document shared on its own (WS-SEPARATE B).** A `0092` document (`share.json`
    v1 with `keyring.log`) that lies in a shared workspace's folder at Share, or is added by "Add
    to workspace", is taken in only if every current member of its own keyring is a current
    member of the workspace keyring (Proposal); otherwise it is refused, stays "Not in the shared
    workspace" and keeps syncing on its own room (criterion 12). The acting device must be a
    member of both, no admin right needed, since nobody loses access. Take-in, in this order:
    first the old room must be reachable within 15 s (Proposal), else nothing changes and the
    document keeps syncing on its own; then the document's merged state is written into the workspace as a new document stream (same
    Loro document, so later merges work); then a sealed `Moved` record, plaintext
    `"CVMOVED1" ‖ workspace [16] ‖ document [16]`, is appended to the old room; then `share.json` v2 replaces v1 and `keyring.log` is dropped. A
    client that reads `Moved` stops syncing that room and never again writes a `RewrapEpoch`
    there, so the old link admits nobody new; a member of the workspace is told where the
    document now is; its old copy becomes a local file. Changes pushed to the old room after
    `Moved` stay in their author's local copy (debt; needs a push across the take-in instant).
    The old room's data stays on the relay (no room deletion in ADR 0031; debt).
11. **What the relay sees** (criterion 34): per workspace one room id; per record its sequence,
    size and time; snapshot parts, their count and the total snapshot size to 64 KiB, hence the
    workspace's approximate size; the keyring entries and which member identities connect when,
    as in ADR 0031; that the room is a workspace (snapshot parts, zero sender peer). A member who
    is also in a `0092` room is linkable across both by its identity key, as in ADR 0031. A
    take-in links the old room to the workspace room by timing: the `Moved` record and the burst
    of workspace records arrive within seconds of each other from the same identity. Presence
    carries the member's list replica peer (`list_peer`, to name the author of a rename) only
    inside the seal. It sees
    no document count, no grouping of records into documents, no names, folders, tree shape, or
    which document a member has open or edits. Sizes and timing cannot be hidden: a patient
    observer sees bursts of activity and, from a record's size, roughly how large one change was.
    Padding live records is not done (Proposal: it hides little while timing stays visible).
12. **Formats.** `.curvyo` container `format_version` unchanged (34); the only container change is
    `share.json` version 2, which an older build refuses by its own version field. New:
    `workspace.toml` v1, `list.loro` (list `format_version` 1), `index.json` v1 (cache), the
    snapshot stream v1, the inner record header, the `Moved` payload, the marker's
    `deleted/<id>.curvyo` kept copies (plain containers), relay protocol 2 with `SnapshotPart`, server staging `snapshot.part/` and a multi-part `snapshot` file (a v1 single
    snapshot reads as one part). Golden files for each.
13. **Crates and dependencies.** No new crate. `sync-core` + `SnapshotPart`, staged parts in
    `RoomLog`, `workspace_stream`, `snapshot_due` (edges unchanged); `sync-server` + staging;
    `document-core` + `workspace_list`, `workspace_plan`, `share.json` v2; `storage-io` +
    `workspace`; `curvyo-app` + commands; `editor-wasm` + list glue and the worker; `crypto-core`
    unchanged. One dependency: `trash` (MIT) in `storage-io`, to pass `cargo deny` with no D-Bus
    stack; if it does not, the implementer stops and reports.

## Consequences

- The relay learns nothing about the documents of a workspace beyond sizes and timing; the 0092
  keyring and room code is reused unchanged and needs one new frame.
- WS-SHARE B's "one room per document" becomes one stream per document inside the workspace room;
  documents still sync, merge and are restored one by one.
- Every member receives every change of every document; with one open document the others are
  written in the background. Fine for the target (tens of documents, LAN); a workspace with large,
  busy documents costs every member the bandwidth of all of them.
- A snapshot carries the whole workspace; the ratio rule of decision 7 keeps that bounded, and a
  removal costs one whole-workspace upload instead of one per room.
- Debt, entered with the build: tombstoned documents never purged; folder scan by polling; names
  without Unicode normalisation; retired `0092` rooms stay on the relay; changes pushed to an old
  room after `Moved` stay local.
- Pure parts (list, plan, stream codec, `snapshot_due`, frames) are testable in the ADR 0031
  simulator; the worker and the file operations need the desktop app.
