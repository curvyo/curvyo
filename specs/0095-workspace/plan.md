# Plan for the workspace, slice 1

Implementer, 2026-11-03, on `story/0095-workspace` (format 34). Built against `specification.md`
slice 1 (criteria 1 to 49), `adrs.md`, ADR 0037 (accepted) and `ux-notes.md`. Everything marked
**(Proposal)** is mine and vetoable at the demo. One PR at the end (M10), when the customer can try
a local workspace and two instances sharing one workspace against a local server; M1 to M9 are
commits on this branch.

## Affected crates/modules

- **`curvyo-document-core`**: `workspace_names` (new: name rules, collision suffix, copy name),
  `workspace_plan` (new: scanned tree, local file operations; later disk-follows-list),
  `workspace_list` (new: Loro movable tree of the shared list, no Loro type in its API),
  `container_share.rs` (`share.json` version 2), goldens.
- **`curvyo-sync-core`**: `workspace_stream` (new: inner record header, snapshot stream and its
  parts, `Moved` payload), `snapshot_due`, frame `SnapshotPart` (relay protocol 2), staged parts in
  `RoomLog`; simulator workspaces.
- **`curvyo-sync-server`**: staging under `snapshot.part/`, multi-part `snapshot` file.
- **`curvyo-storage-io`**: `workspace` (new: marker directory, writable probe, tree scan, file
  operations inside the root, OS trash via `trash`), `recent_workspaces` (new: the recents file).
- **`curvyo-ui-core`**: workspace rows (natural sort, flattening, counts), texts of the section,
  confirms, notices, the Share tab's Workspace block.
- **`curvyo-editor-wasm`**: local workspace glue (plans, new and duplicate document bytes, rows),
  workspace session on one `ClientSession`, list glue, the background writer worker.
- **`curvyo-app`**: workspace commands (root kept in `AppState`; the page names only paths
  relative to it), recents, title, marker share files.
- **`frontend`**: File tab Workspace section (treegrid, action slot, recents, Deleted), Share tab
  Workspace block and join block, worker.
- Docs: `docs/technical-debt.md` (debts of ADR 0037), `docs/design-system.md` (new components),
  `deny.toml` unchanged.

## Milestones and tasks

### M1 local workspace core and host primitives

1. [x] `trash` in `curvyo-storage-io`; `cargo deny check` green before any code (ADR 0037
   decision 13).
2. [x] `document-core::workspace_names`: refused names (empty, `/` `\`, `: * ? " < > |`,
   control characters, trailing dot or space, leading dot, Windows reserved names, over 120
   characters) with the reason; unique name among siblings of one kind, case-insensitive
   (`to_lowercase`), " 2", " 3"; copy name "`<name>` copy" (criteria 13, 14, 18, 19).
3. [x] `document-core::workspace_plan` (local part): scanned entries to a folder tree with
   document counts; plans for New document, New folder, Rename, Move, Duplicate, Delete as file
   operations on relative paths, or a refusal (into itself, name exists, bad name, not found);
   the inverse of a move for Undo move (criteria 13, 14, 16, 17, 18, 19).
4. [x] `storage-io::workspace`: `workspace.toml` codec (`format_version = 1`, `id`), open (writable
   probe, marker created only when absent, higher version or damage refused without writing),
   scan (`.curvyo` files and folders, sizes, mtimes, no container opened, the marker directory,
   hidden entries and symbolic links left out), file operations that never replace and never
   leave the root, trash (criteria 1, 2, 4, 5, 7, 17, 18, 23, 24).
5. [x] `storage-io::recent_workspaces`: the recents file text (`format = 1`, up to 10 paths, most
   recent first, removable, a newer format never written over), found flags (criterion 20).
6. [x] Scan budget: 500 documents in nested folders within 1 s of thread CPU (criterion 5).

### M2 local workspace in the File tab

7. [x] `curvyo-app` commands: pick and open a workspace, open a recent by index, close, scan,
   apply a planned operation, read a document's bytes and its thumbnail member, write a document;
   the root lives in `AppState`, every path from the page is relative and checked (criteria 1 to
   4, 9, 13 to 22).
8. [x] `ui-core` workspace rows: natural case-insensitive order, folders first, flattening by
   expansion, `aria` data, counts, texts formatted in Rust (confirms, refusals, statuses, title
   "workspace / path") (criteria 5, 6, 11, 13, 14, 16, 17).
9. [x] `editor-wasm` glue: plans, an empty document's bytes (New of `0001`), duplicate bytes
   without share record (criteria 13, 19); view state per document for the session (10).
10. [x] Frontend Workspace section per `ux-notes.md` 2, 3, 10, 11: treegrid on `useRowsFocus`,
    lazy thumbnails, "Cannot be read" rows, inline name field, drag and Ctrl+X/V moves, Undo move,
    inline delete confirm, recents, polling every 2 s, "File missing", close and switch with the
    unsaved-changes check, a `0092` document reconnecting on click (criteria 1 to 24).
11. [x] Design system entries; CPU budget of a 500-row expand (criterion 5).

### M2 follow-ups (lead decisions, 2026-11-03)

11a. [x] Thumbnails without a CPU rasteriser: the page draws the active page with the canvas
    renderer into an offscreen canvas on Save, encodes PNG, the host puts it into the container
    as `thumbnail.png` (criterion 6).
11b. [x] A file opened with plain Open keeps today's title even inside the open workspace
    (criterion 3).

### M3 shared list and stream codecs in core

12. [x] `share.json` version 2 (`workspace`, `document`, `synced_version`), version 1 still read;
    goldens (criteria 15, 36).
13. [x] `workspace_list`: Loro tree, node meta, derived names with " 2" and replaced characters,
    moves with the loop rule and the dropped-move report, tombstones and Restore, export/import/
    snapshot; convergence tests with concurrent moves, names, loops (criteria 35 to 40).
14. [x] `workspace_plan` disk-follows-list: derived tree and scanned files keyed by document id to
    create, rename, move, write, trash; outside files, missing files, external moves (criteria
    36, 37, 40, 41).
15. [x] `sync-core::workspace_stream`: inner header, snapshot stream v1, parts of 4 MiB padded to
    64 KiB, `Moved` payload, `snapshot_due`; goldens (criteria 28, 34, 43).

### M4 relay protocol 2

16. [x] Frame `SnapshotPart`, protocol 2 with protocol 1 still accepted; `RoomLog` staging, swap on
    `last`, prune to `covers_seq`, parts served as consecutive snapshot records (criteria 28, 43).
17. [x] `sync-server` staging under `snapshot.part/`, multi-part `snapshot` file, a version 1
    single snapshot read as one part; restart (criteria 47, 48).
18. [x] Simulator: workspaces of 3 and 30 documents give server data of one structure (34); two
    workspaces of one maker (47).

### M5 share, invite `2.`, join

18a. [x] `sync-core`: `InviteLink` names its kind (`1.` document, `2.` workspace); `ClientSession`
    uploads a workspace snapshot as `SnapshotPart`s (`upload_snapshot_parts`, sent on the next
    connection when offline) and asks for one by `snapshot_due` once
    `use_workspace_snapshots` is set; the simulator test of M4 task 18 on the client side: share,
    invite, join, parts assembled, inner records, sender peer 0 (criteria 26 to 29, 34).
18b. [x] `document-core::workspace_share_file`: the marker's `share.json` (ADR 0031 decision 9
    shape, `room` = workspace id, `server_seq` = workspace cursor, `synced_version` = the list's)
    and `index.json` v1; goldens (criteria 25, 33).
19. [x] Workspace session in `editor-wasm` (`session/workspace_link.rs`): the open session's
    `SyncLink` carries a workspace channel (list replica, open stream, snapshot assembler,
    background queue). Records are routed by their inner header: the list into the replica, the
    open document's into the session as in `0092`, every other document's into the background
    queue with its sequence; exports carry the inner header and sender peer 0. The link moves
    from session to session when the maker switches documents (`take_workspace_link`,
    `attach_workspace_link`); records applied to the open document but not saved go back to the
    background queue when it is left. The marker cursor never passes a record that is not in a
    file (criteria 30, 31, 33).
20. [x] Share workspace (`WorkspaceShareBuilder`): every readable document gets a fresh id and
    `share.json` v2 in memory, the list is built from the scan, the snapshot stream is cut into
    parts and uploaded after the genesis; only when the relay took it are the files, the new
    workspace id and the marker files written (criterion 25). **(Proposal)** a document shared on
    its own is left out until M8 (take-in) and named in the confirm as "Not added".
21. [x] Invite `2.`, join into an empty folder outside any workspace, admission, writes (folders,
    then each document as soon as its bytes are complete) with "Receiving 7 of 20", reopen and
    reconnect from the marker (criteria 26 to 29, 33).
22. [x] App commands (join folder check, marker files, replace a document, joined workspace) and
    the Share tab Workspace block, join block, header line and title (criteria 25 to 28).

### M6 background writer

23. [x] `editor-wasm::background_writer`: per document the records waiting, due at most every 5 s
    (**Proposal** of criterion 32), applied to the file's bytes (unpack, import, pack) with the
    sequence they reach; a Web Worker (`workers/backgroundWriter.ts`, its own wasm instance, the
    trace worker pattern) runs it; the page reads and writes the files through the host and
    reports each write, which lets the marker cursor advance (criteria 31, 32). The open document
    is never written by the worker. The workspace snapshot due by `snapshot_due` is built from the
    written files, the open document's own bytes and the list (decision 8). CPU budget: the main
    session routing 49 busy documents' records stays inside the frame budget.

### M7 live list on disk

24. [x] Remote create, rename, move, delete applied to disk through the plan; notices; trash
    waiting for an open dirty document; Deleted list and Restore; "Not in the shared workspace"
    and Add; "Missing on this computer" (criteria 35 to 41).

### M8 take-in (WS-SEPARATE B)

25. [x] Membership check of a `0092` document against the workspace keyring; take-in in the order
    of ADR 0037 decision 10 (`Moved`, `share.json` v2); refusal texts; the old link admits nobody
    (criteria 42, 49).

### M9 removal

26. [x] `RemoveMember`, `NewEpoch`, one workspace snapshot under the new epoch by the worker; the
    removed device turns local; removal found offline (criteria 43 to 46).

### M10 journey, verification, gate

27. [x] Desktop and browser journey with two instances and one server; `docs/technical-debt.md`
    entries; full gate; PR (the lead opens it).

## Validation

Test-first per module: unit tests in each new module, tempdir tests for the host operations,
golden files for every new format (`workspace.toml`, `recent-workspaces.toml`, `share.json` v2,
`list.loro`, `index.json`, snapshot stream, inner header, `Moved`, `SnapshotPart`), convergence
tests of the list replica, the ADR 0031 simulator for workspaces, thread CPU budgets (scan,
expand, frame time), and the journey. Not in the gate: the LAN timings of criteria 28, 35 to 37
(the tester measures them).

## Dependencies (ADR 0037 decision 13)

`trash` 5.2 (MIT) in `curvyo-storage-io`, default features (`chrono`, already locked;
`coinit_apartmentthreaded` for Windows); on Linux it writes the freedesktop trash as files, no
D-Bus. `cargo deny check` green on 2026-11-03. `toml` (workspace dependency) joins
`curvyo-storage-io` for the marker and recents files.

## Results

### M1 (2026-11-03)

- `curvyo-document-core::workspace_names`: `check_entry_name`, `same_entry_name`,
  `free_entry_name`, `copy_entry_name`, `MAX_ENTRY_NAME_CHARS` (the `entry` prefix keeps them
  apart from the layer names of `node_registers`). **(Proposal)** A leading dot is refused (the
  scan hides such files); a duplicate of a taken copy name is "Lid copy 2".
- `curvyo-document-core::workspace_plan`: `WorkspaceTree` over scanned `(path, kind)`, plans
  `plan_new_document`, `plan_new_folder`, `plan_duplicate`, `plan_rename`, `plan_move`,
  `plan_delete` to one `FileOp` or a `PlanRefusal`; `FileOp::inverse` for Undo move. Names
  compare under `to_lowercase` within one kind, so a folder `Lid` and a document `Lid` may stand
  side by side. **(Proposal)** A move onto its own folder is refused as `AlreadyThere`.
- `curvyo-storage-io::workspace`: the marker codec and `open_workspace` live here, not in
  `document-core`: the host owns the folder and the page never names a path (the page gets the
  id). Open reads an existing marker first (newer or damaged: refused, nothing written), probes
  writability with a removed probe file, then creates the marker with the caller's random id.
  `scan_workspace` lists folders and `.curvyo` files without opening one, leaving out the marker,
  hidden entries, symbolic links, non-UTF-8 names and unreadable subfolders (Proposal). File
  operations take relative paths, refuse components that are empty, `.`/`..`, hold a separator
  or start with a dot, and never replace an entry (a new document reserves its name, then gets
  its bytes by atomic rename). `trash_entry` uses `trash::delete`.
- `curvyo-storage-io::recent_workspaces`: `recent-workspaces.toml` (`format = 1`,
  `paths = [...]`), most recent first, ten at most, removable, rows with the folder name and a
  found flag; a newer file is refused (never written over), a damaged one is replaced by the next
  write (Proposal: it holds only shortcuts). The app reads and writes it (M2).
- Tests: 20 unit tests in the two document-core modules; `curvyo-storage-io/tests/workspace_0095.rs`
  (22 tests on real folders, goldens `workspace_v1.toml` and `recent-workspaces_v1.toml`, the
  trash test checks the item in the freedesktop trash and purges it). Scan of 500 documents in
  nested folders: 0.5 ms thread CPU in release (budget 1 s, checked in every profile).
- Checks: fmt; clippy `-D warnings` host (document-core, storage-io, all targets) and wasm32
  (document-core); wasm32 build; nextest document-core + storage-io 2,657 passed; rustdoc
  `-D warnings`; `cargo deny check` green with `trash`.

### M2 (2026-11-03)

- `curvyo-app::workspace`: `pick_workspace` (OS folder picker), `open_recent_workspace`,
  `recent_workspaces`, `remove_recent_workspace`, `close_workspace`, `scan_open_workspace`,
  `apply_workspace_op`, `read_workspace_document`, `confirm_workspace_document_opened`,
  `probe_workspace_document`. The root lives in `AppState`; every path is relative and checked by
  `curvyo-storage-io`; the project's path follows a move of its file or a folder above it; the
  title is "Box job / Sides / Lid — Curvyo" for a document inside the open workspace. A refused
  folder keeps the open workspace. `main.rs`'s handler list moved into `commands()` (the function
  passed 100 lines).
- `curvyo-storage-io::workspace_documents`: `read_document`, `probe_document` (zip directory,
  manifest and `thumbnail.png` only; "not a Curvyo file" / "damaged"; thumbnails over 1 MiB show
  the placeholder, **Proposal**), `fresh_workspace_id`. `zip` moved from dev to normal dependency.
- `curvyo-ui-core::workspace_rows` (natural case-insensitive order, folders first, flattening by
  expansion, aria data, counts) and `workspace_text` (times, labels, confirms, refusals,
  statuses); `Viewport::saved_view`/`restore_view` (zoom and the centre point).
- `curvyo-editor-wasm`: `WorkspaceGlue` + `WasmWorkspace` (scan, rows, expanded folders, plans
  with their words), `workspace_copy` (unpack + `pack_snapshot`: no share record), the fixed texts,
  `saved_view_text`/`restore_view_text` on the session.
- Frontend: `useWorkspace`, `WorkspaceSection`, `WorkspaceTree`, `WorkspaceNameField`,
  `lib/workspaceRows.ts`; `useProject` gains a generic checked action (`replaceThen`),
  `openWorkspaceDocument`, `resetToEmpty` (keeps the File tab shown) and `setFileState` ("File
  missing"); browser stand-ins for the workspace commands.
- **(Proposal)** choices: the name field of a new entry stands above the tree, not as a row in the
  folder; New document in the header acts on a row the maker chose (not the row focus landed on
  at open); opening a workspace while one is open asks first, and the open document closes only
  when another workspace opened; with none open, the open document stays; the view kept per
  document is zoom and centre; a "File missing" row is not kept for 60 s (the project block says
  it).
- Tests: ui-core 7 (rows, texts, viewport), editor-wasm 5 (glue, view) + `workspace_tree_budget`
  (500 documents: scan and every expand with rows within 16 ms of thread CPU; 8.5 ms in debug),
  storage-io 3, curvyo-app 6 (open, refusal, operations, follow move, title), frontend
  `workspaceRows.test.ts` 7 and `projectText.test.ts` 2 more. Browser smoke with a mocked host
  (open, unreadable row, new document with " 2", rename refusal and rename, Ctrl+X/V and Undo
  move, delete confirm, duplicate, close): green.
- Open: no `.curvyo` is written with a `thumbnail.png` yet (no CPU rasteriser, debt "CPU
  rasterization"), so every row shows the placeholder until a writer exists; the 300 ms
  "Scanning..." state is not shown (a scan of 500 documents takes 0.5 ms).

### M2 follow-ups (2026-11-03)

- Thumbnail (criterion 6): `thumbnail.png` is already a container member of ADR 0004 §1, so no
  new member and no `format_version` change: `unpack` reads members by name and never looks at
  it, so every build, older ones included, ignores it (checked: `container.rs` reads
  `manifest.json`, `share.json`, `document.loro` and `assets/` only). `curvyo-storage-io::
  with_thumbnail` copies every other member raw and adds the PNG stored (replacing an older
  thumbnail); refuses a non-PNG, a PNG over `MAX_THUMBNAIL_BYTES` or bytes that are no zip, and
  `save_project_bytes` then writes the bytes as packed. `Session::thumbnail_frame` (page area and
  visible artwork of the active page, fitted with a 6 % margin, **Proposal**; no guides, grid,
  selection or tool marks); `WasmThumbnail` owns its own `Gpu` on one offscreen canvas for the
  page's lifetime (so its async attach never holds a session); `lib/pageThumbnail.ts` draws and
  reads the canvas in one task, 160 x 112 px at ratio 2 (**Proposal**). Rows pick the new
  thumbnail up by the changed mtime. Debt: "Only a Save from the window writes a thumbnail".
- Criterion 3: `ProjectState::in_workspace`, set only by `confirm_workspace_opened`; the title
  names the place in the workspace only then (`project_title_name`). The tree already dropped its
  highlight for a file opened another way.
- Tests: storage-io `container_thumbnail_0095.rs` (3), curvyo-app 2 (thumbnail on save, title of
  a plain Open), editor-wasm 2 (`session::thumbnail`).

### M3 (2026-11-03)

- `share.json` version 2 (`container_share.rs`): `WorkspaceShareRecord {workspace, document,
  synced_version}`, `Document::workspace_share`/`set_workspace_share`; the file holds one record
  at most (internal `Sharing`); version 1 is still written for a document shared on its own and
  read; version 2 beside a `keyring.log` is damage. `CURRENT_SHARE_FORMAT_VERSION` is now 2 (the
  newest read); the `0092` test of a refused newer record uses 3. Golden
  `workspace_shared_v35.curvyo`.
- `workspace_list` + `workspace_list_view`: `WorkspaceList` (create folder or document, rename,
  move, delete with `{by, at}`, restore, `place`, updates, snapshot, `version`), `ListNodeId`
  (text `counter@peer`), `Created`. Shown names derived on read by creation order with
  `free_entry_name`; a peer's refused name shown with `_`. A dropped loop move is reported in
  `ListImportReport::dropped_moves` to the replica that made it: a recorded move whose node no
  longer sits in its target and whose target now lies below the node (**Proposal**; the record
  of a move is kept until an import overturns it). A restore under a still-deleted folder lands
  at the top (**Proposal**). Golden `list_v1.loro`.
- `workspace_disk_plan` (a module of its own beside `workspace_plan`, which is about local
  operations and already 560 lines): `plan_disk(entries, tombstoned ids, DiskState{folders,
  documents by id, known places})` gives `DiskOp`s (create folder, move, write from stream,
  trash), `ListPlace`s for files another program moved, missing documents, outside files and
  blocked places. A ring of moves passes `.<name>.moving` (hidden, so the scan skips it); a
  folder is trashed only when it held workspace files and is empty after the plan
  (**Proposal**); both list and disk moved: the list wins (**Proposal**).
- `curvyo-sync-core::workspace_stream`: inner header `stream[16] peer u64`; snapshot stream
  `format_version u8, count u32` then entries (the count is an addition to ADR 0037 decision 7:
  the reader knows where the stream ends); parts with plaintext header `snapshot_id[16] index
  u32 count u32 length u32` (`length` added so padding never reads as stream bytes), 4 MiB, the
  last padded to 64 KiB; `SnapshotAssembler` hands out entries as they complete; `Moved` =
  `"CVMOVED1" ‖ workspace ‖ document`; `snapshot_due`. Golden `workspace_stream_v1.bin`.
- Tests: document-core `workspace_share_0095.rs` (6), `workspace_list_0095.rs` (13),
  `workspace_disk_plan_0095.rs` (12), 1 unit; sync-core `workspace_stream.rs` (8), including
  the 3-versus-30 documents part sizes (criterion 34).

### M4 (2026-11-03)

- Relay protocol 2: `PROTOCOL_VERSION = 2`, `OLDEST_PROTOCOL_VERSION = 1` (the relay admits
  both); `ClientFrame::SnapshotPart {covers_seq, index u32, last u8 0|1, envelope}`, tag `0x08`.
  `frames_v1.bin` stays byte for byte (its `Hello` now names protocol 1 literally); new golden
  `frames_v2.bin`.
- `RoomLog` keeps its snapshot as parts in `relay::snapshot::Snapshots` (one responsibility:
  the newest snapshot and the staged parts); part 0 starts a staging over, the others follow in
  order for one `covers_seq` (else `Protocol`), the last swaps in and prunes when it covers at
  least the stored snapshot, else the parts are dropped. Parts go out as consecutive
  `Record{seq: covers_seq, kind: snapshot}`. New `StoreOp`s `SnapshotPart`, `SnapshotParts`,
  `DropStagedParts`. `StoredRoom` gains `snapshot_rest` (the parts after the first) so
  `snapshot` keeps its shape and the `0092` tester tests compile unchanged (**Proposal**).
- `curvyo-sync-server`: `snapshot_file` (snapshot version 2 `"CVYSNAP2", covers u64, count u32,
  parts`, version 1 read as one part; staging `snapshot.part/<index:08>`, dropped at startup),
  `room_scan` (the startup scan moved out of `store.rs`, which had passed 500 lines). Golden
  `tests/fixtures/snapshot_v2.bin`.
- Simulator task 18 done at the relay level (`tests/workspace_relay.rs`): a workspace of 3 and
  one of 30 documents with the same total size and edit pattern store data of one structure,
  no stream id readable; two workspaces of one maker are two rooms without cross-talk. The
  `ClientSession` side of a workspace is M5.
- Tests: `relay_rules.rs` 4 more, `frames.rs` 1 more, `workspace_relay.rs` 2, sync-server 4
  (snapshot file 3, store restart 1).

### M5 (2026-11-03)

- `curvyo-sync-core`: `InviteKind` on `InviteLink` (`1.` document, `2.` workspace; `#3.` is the
  unknown version now); `ClientSession::use_workspace_snapshots` (due by `snapshot_due` against
  the last snapshot's bytes, counted from the received parts or the own upload) and
  `upload_snapshot_parts` (`SnapshotPart` frames, kept for the next connection when offline).
  `tests/workspace_session.rs` is M4 task 18's client side: share with parts, `2.` invite, join,
  parts reassembled, inner records, every envelope with sender peer 0 and the workspace id.
- `curvyo-document-core`: `WorkspaceRoomRecord` (the marker's `share.json`, version 1 shape,
  golden `workspace_marker_share_v1.json`). `workspace_plan.rs` split: its tests moved to
  `workspace_plan_tests.rs` (335 + 229 lines).
- `curvyo-editor-wasm`: `session::workspace_channel` (routing by inner stream, snapshot
  reassembly, join writes, background queue, the marker cursor rule) and
  `session::workspace_sync` (share, join, resume, link moved between sessions, exports with the
  inner header, list exports and acks, the marker files, `share.json` v2 on save);
  `WorkspaceShareBuilder` (fresh ids, `share.json` v2, list from the scan, parts);
  `wasm_workspace_sync` bindings. The Share view gains `workspace` (name, header line, people,
  receiving, document line). Presence carries the open document's id inside the seal; marks
  are drawn only between sessions of one document.
- `curvyo-storage-io::workspace_share_files`: marker share files, fresh id, replacing a document
  (keeps its thumbnail, so Share's repack does not drop it), `index.json` (golden
  `index_v1.json`), the join folder check and the joined marker. `serde_json` joins the crate's
  dependencies (workspace dependency, MIT OR Apache-2.0).
- `curvyo-app::workspace_share`: seven commands (marker read and write, fresh id, replace a
  document, index, pick the join folder, start the joined workspace); `AppState::join_folder`.
- Frontend: `useWorkspaceSync` (share, join, resume, joined files, marker every 2 s),
  `WorkspaceShareBlock`, the Share tab's Workspace block above the Document block, "Join shared
  workspace..." beside "Open shared document..." (the link decides the route), Choose folder...,
  the File tab header line of a shared workspace; `SyncTransport.handOver` and
  `lib/workspaceLink.ts` keep the socket while the link moves to the next document's session.
- **(Proposal)** choices: Share closes the open document (after the check), prepares every file in
  memory and writes files, the new id and the marker only once the relay admitted it and the
  parts went out; it then reopens the document. The confirm counts the scanned documents; the
  ones left out (shared on their own until M8, unreadable, a name the list refuses) are named
  after the share ("Not added: Lid."). A joined workspace's folder gets its marker only when the
  first file arrives (a refused or cancelled join leaves the folder empty). A joiner whose cursor
  is 0 receives the room's snapshot again on reconnect (the relay's rule of ADR 0031); its
  containers merge into the files.
- **Deviation from ADR 0037 decision 8 (Proposal, for the architect):** the main session opens
  every record, the parts too; the background writer gets plaintext and needs no epoch keys. Opening
  is cheap next to unpack/import/pack, which is what the worker takes off the main thread.
- Tests: sync-core `workspace_session.rs` 3, `invite_link` 1; document-core 2; editor-wasm
  `workspace_channel` 3, `workspace_share_builder` 1, `workspace_sync_tests` 5 (share, join with
  files, live edit of one document and background records of another, leaving a document unsaved,
  reopening from the marker); ui-core 2; storage-io `workspace_share_0095.rs` 5; curvyo-app 2;
  frontend `syncTransport` 2 more, `workspaceLink` 1.

### M6 (2026-11-03)

- `curvyo-editor-wasm::background_writer`: `BackgroundWriter` keeps the records of documents that
  are not open per document; a document is due when records wait and its last write lies 5 s
  back (the first record is written at once); `write` unpacks the file, imports updates, pictures
  and snapshot containers, packs it (its `share.json` v2 stays) and names the highest sequence;
  a file that does not open keeps its records for the next interval. `wasm_background_writer`:
  `WasmBackgroundWriter` and the session's snapshot calls.
- Frontend: `workers/backgroundWriter.ts` (its own instance of the editor's wasm module, the trace
  worker's pattern) and `lib/backgroundWriterJobs.ts`; `useWorkspaceSync` hands the session's
  background records to the worker, reads a due document's file, writes the worker's bytes with
  `replace_workspace_document` (thumbnail kept) and reports `background_written`, which lets the
  marker cursor advance. The open document is never written in the background: the page asks
  `workspace_may_write` before reading and again before writing, and opening a document gives its
  session every record not yet in its file.
- Workspace snapshot (decision 8): `workspace_snapshot_ready` names the covered sequence once the
  files hold every record up to it; the page reads the files of every document the list names but
  the open one, the session adds the list and the open document and uploads the parts. If the
  open document changes while the files are read, the build starts again on the next tick.
- **(Proposal / deviation, for the architect):** the parts are sealed by the main session (the one
  `ClientSession` holds the keys and the nonce stream); the worker does the file work. Sealing a
  snapshot is about 1 ms per MiB; a due snapshot comes only after at least 4 MiB of edits.
- Tests: `background_writer` 2 (written into the file with `share.json` v2 kept, 5 s interval,
  containers merged, failures keep the records, forget); `workspace_sync_tests` 2 more (a due
  snapshot waits for the files, then the relay holds it; CPU budget: one second of 49 busy
  documents plus an edit of the open one through the relay and B's frame start, 0.19 ms of thread
  CPU in release, asserted at 16 ms).
- Open: the browser has no workspace (desktop only); the worker's file reads and writes go
  through the host's commands as JSON number arrays (the existing pattern), which is slow for
  large files (debt candidate).

### Fix after M6: file bytes over the IPC (2026-11-03)

- `curvyo-app::raw_ipc`: `read_workspace_document` and `probe_workspace_document` answer a raw
  `tauri::ipc::Response` (the thumbnail's PNG, empty for none; an unreadable file is the error
  `not-curvyo` / `damaged`); `apply_workspace_op` and `replace_workspace_document` take the bytes
  as the raw request body and the other arguments as ASCII-only JSON in the `curvyo-args` header
  (a call without bytes keeps the JSON body). The page's `invoke` takes the bytes as a third
  argument (`lib/binaryArgs.ts`). Tests: `raw_ipc` 2, `binaryArgs.test.ts` 2.

### M7 (2026-11-03)

- `curvyo-editor-wasm::session::workspace_tree`: `workspace_list_edit` (create document or
  folder, rename/move, delete by this device's label, restore; paths are file paths),
  `workspace_disk_plan` (`plan_disk` over the host's scan; moves another program made go to the
  list at once; steps `create-folder`, `move`, `write`, `keep`, `trash-document` with who deleted
  it, `trash`; missing, outside, blocked, settled), `workspace_adopt_document` (a new, duplicated
  or added document gets `share.json` v2 and its whole state goes out as the first records of its
  stream, after the list), `workspace_deleted`, `workspace_document_place`,
  `workspace_download_again`. `workspace_notices`: "Ben deleted Lid.", "Someone renamed Lid to
  Cover.", "Someone moved Left to Box job.", the loop notice, into the canvas notice slot.
  `workspace_channel`: a join keeps a deleted document's state (`WorkspaceWrite::Deleted`);
  snapshots carry deleted documents from their kept copies (`DocumentPlace`).
- `background_writer`: `create` builds a document that has no file here yet from its records (a
  snapshot's container, or the updates of its stream from its creation) with `share.json` v2
  naming every imported change as synced; `postpone`. `curvyo-document-core`:
  `Document::from_updates`; opening refuses Loro changes that continue a missing history
  (pending); `WorkspaceShareRecord::from_share_json`. `curvyo-sync-core`: `ClientSession::resync`
  (Download again; `record_rejected` uses it).
- `curvyo-storage-io`: `share_members` (a file's `share.json` and whether it has a keyring,
  read without opening the document), `read_index`, `workspace_deleted` (kept copies
  `.curvyo-workspace/deleted/<id>.curvyo`: keep and trash, read, write, restore).
  `curvyo-app::workspace_shared`: `scan_shared_workspace` (scan plus sharing members, cached by
  size and time, plus the index), `keep_workspace_document`, `restore_workspace_document`,
  `read_deleted_document`, `write_deleted_document`.
- `curvyo-ui-core::workspace_tree_text`; `WorkspaceGlue::set_shared_states` (row states
  "Not in the shared workspace", "Missing on this computer", "Receiving..."; missing and
  arriving documents get rows of their own) and the shared delete confirm.
- Frontend: `useSharedTree` (every 2 s: scan, plan, steps, index, row states, Deleted list; the
  maker's operations go through `apply` into the list: New and Duplicate adopt the document,
  rename/move/new folder after the file operation, Delete as a list deletion), `lib/sharedTree.ts`,
  `WorkspaceDeleted` (disclosure with count and Restore), Add to workspace and Download again in
  the row's action slot, the project block's "Deleted by Ben. Save as to keep it.", the background
  writer by place (file, kept copy, or built anew), the join's kept copies.
- **(Proposal)** choices, several for the architect: deleted documents are kept as
  `.curvyo-workspace/deleted/<id>.curvyo` before their file goes to the OS trash (the ADR keeps
  their state in the room, but a member has no other local copy to restore from or to put into
  the next snapshot; a new file in the marker directory); renames and moves say "Someone" (the list
  records who deleted, not who renamed or moved); a notice cuts names at 20 characters (24 in the
  UX notes pass the 70-character limit with two names); Add to workspace always mints a fresh id;
  Download again re-reads the whole room from its snapshot; the state buttons stand in the row's
  action slot instead of a third line (fixed row heights); the Deleted disclosure is under the
  action slot.
- Tests: editor-wasm `workspace_tree_tests` 5 (create written from records, rename and move with
  notices, delete with who and Restore from the kept copy, outside/missing/external move,
  Download again), `background_writer` 1, `workspace_tree_glue` 3, `workspace_glue_tests` 1;
  document-core 2; sync-core `sim_offline` 1; storage-io `workspace_shared_tree_0095.rs` 3;
  ui-core 2; curvyo-app 1; frontend `sharedTree.test.ts` 2, `projectText` 1.
- Open: a joiner's kept copies of deleted documents are written, not tested end to end; a
  document missing here whose records keep arriving holds the marker cursor until Download again.

### M8 (2026-11-03)

- `curvyo-editor-wasm::session::workspace_take_in`: `take_in_check` (plain; allowed when every
  current participant of the document's own keyring is a current participant of the workspace
  keyring, with the count; refused otherwise), `Session::workspace_take_in_check`, `push_moved`
  and `moved_stored` (a short-lived session of the old room appends the sealed `Moved` record),
  and `moved_away`: a member that reads `Moved` drops the share record (its copy is a local file
  from its next save), closes the room for good (`SyncEnd::Moved`, the session no longer
  reconnects, so it never writes a `RewrapEpoch` there again) and says "This document is now part
  of a shared workspace. This copy is a local file.".
- Share workspace: `WorkspaceShareBuilder::set_member` (the device's identity) takes in a
  document shared on its own when this device is its only participant (the workspace's only
  member at Share); others stay out as "Not added" (criterion 49). After the relay took the
  snapshot, each taken-in document's old room gets `Moved` (`lib/movedPush.ts`).
- Add to workspace on a row outside the workspace: plain files join with a fresh id; a document
  shared on its own asks "Lid is shared on its own with 2 people. Take it into Box job? ..." (Take
  in), then its state goes into the workspace, `Moved` goes to its old room, and its file gets
  `share.json` v2; refused ones show the alert of criterion 49 and keep syncing on their room.
- **(Proposal)** choices: the take-in order is the ADR's (workspace first, `Moved`, then the
  file); when the old room cannot be reached within 15 s the document stays taken in and the
  alert says its old link may still work; a member of the old room is told "a shared workspace"
  (it may not know the workspace's name); the Share confirm does not yet name the documents taken
  in beforehand (they are named after the share as before; open point).
- Tests: `workspace_take_in_tests` 3 (membership rule; `Moved` ends the old room for its other
  member and an older invite admits nobody; Share takes in a document shared with this device
  alone and leaves one shared with another out).

### M9 (2026-11-03)

- The workspace room's removal is `0092`'s (`ClientSession::remove`: `RemoveMember`, `NewEpoch`
  wrapped to every remaining participant, then `SnapshotDue`); in a workspace the due snapshot is
  the one the background writer's files build (M6), sealed under the new epoch, so the relay's
  pruning drops every old-epoch record. No new frame, no new code path in `sync-core`.
- `Session::workspace_removed`; on removal (live or found on reconnect) the open document's
  `share.json` v2 is dropped (a local file from its next save) and the link ends, so no marker
  is written any more. `curvyo-storage-io::remove_marker_share` and the command
  `clear_workspace_marker_share`: the marker keeps `workspace.toml`, documents and kept copies
  stay. The File tab shows "You were removed from this workspace. Your copies stay on this
  computer." with OK (`role="alert"`) until dismissed.
- Tests: `workspace_removal_tests` 2 (one rotation for the workspace, A builds one fresh snapshot
  the relay keeps, B removed with its open document local, a member sees no Remove; a removal
  found on reconnect ends B's workspace and B's offline edit does not reach A); storage-io 1.
- Open: the other files of a removed device keep `share.json` v2 until they are saved from a
  session that drops it (a session opened in the now-local workspace keeps it; harmless, a later
  Share mints new ids); the Share tab's Workspace block shows the ended link ("removed") rather
  than "Not shared" with Share workspace until the workspace is reopened; C across the rotation
  is covered by `0092`'s rotation tests, not again here.

### Checks after M7 to M9 (2026-11-03)

fmt; clippy `-D warnings` host (all targets) for document-core, sync-core, storage-io, ui-core,
editor-wasm, curvyo-app, and wasm32 for document-core, sync-core, ui-core, editor-wasm; nextest
editor-wasm (lib 794, integration 3,800) and document-core, sync-core, storage-io, ui-core,
curvyo-app, sync-server (5,302) all passed; rustdoc `-D warnings`; `cargo deny check` green;
frontend `build:wasm`, `tsc -b`, `npm test` (728), `oxlint` (no errors).

### Gaps after M9 and M10 (2026-11-03)

- Rename and move notices name the member (`ux-notes.md` section 7; criteria 36, 37 do not ask):
  presence carries `list_peer`, the list replica's Loro peer; the inner header of a list record
  names its author, so B reads "Anna renamed Lid to Cover.". **(Proposal)** "Someone" when the
  author was not seen online in this session (debt entry).
- Criterion 42: the Share confirm names the documents taken in beforehand ("Lid is shared on its
  own. It becomes part of the workspace and its own link stops working.") and those left out for
  outside members; the page classifies them with `workspace_share_take_in` before the confirm.
- Take-in with the old room unreachable (ADR 0037 decision 10 order, **Proposal**): the document's
  own room must be reachable (15 s) before anything changes. Add to workspace refuses with "Lid was
  not added: its own room cannot be reached. Try again when connected."; Share leaves such a
  document out as "Not added". Only a failure between connecting and storing `Moved` keeps the old
  alert ("Its old link may still work").
- Criterion 44: removal drops `share.json` v2 naming the workspace from every document
  (`curvyo-storage-io::remove_marker_share`, raw member copy, no new crate edge: the v2 shape is
  read as JSON); the Share tab of a removed device shows the local view ("Not shared", Share
  workspace available); the canvas notice uses the workspace's words.
- Criterion 41 / marker cursor: a missing document whose records keep arriving holds the cursor back
  until Download again; recorded as debt (no criterion asks for more).
- Found by the journey: `WorkspaceSection.tsx` called the wasm bindings at module load, which
  stopped the production page (browser and desktop) from starting; now read on first use.
- M10 journey `frontend/tests/browser/workspace.cjs` (faked desktop host on real folders, local
  relay, two contexts): PASS; join 1.6 s, create 1.2 s, rename 1.1 s, move 0.6 s, live edit
  3.3 s (wall time with saves in between, not a gate figure); no name or label in the relay data.
- Docs: `docs/requirements.md` R-SYS-011, `docs/design-system.md` "Properties panel: shared
  workspace", `docs/technical-debt.md` (seven `0095` entries), ADR index note on the build
  deviations.

### Sizes after M6

`session/workspace_sync.rs` (link) and `session/workspace_files.rs` (files, marker, snapshot)
split at 344 and 206 lines; `workspace_channel.rs` 408 with its tests in
`workspace_channel_tests.rs`; `useWorkspaceSync.ts` 453 with the writer in
`useWorkspaceWriter.ts`. Over 500 and justified: `ShareTab.tsx` (688, was 594 before this story:
the Workspace block went to `WorkspaceShareBlock.tsx`; what remains is the `0092` tab plus the
join block's folder row; splitting its shared buttons and fields is a refactor for the lead) and
`useWorkspace.ts` (701, was 669 after M2; 32 lines of accessors for the sharing hook).

### Sizes after the review fixes (architect item 3)

Counted after the fixes; each over 500 lines is justified here, splits are follow-ups for the
lead:

- `frontend/src/hooks/useWorkspace.ts` 787: the one state owner of the open workspace (rows,
  slot, naming, thumbnails, plans, the shared tree's accessors). Splitting the action slot and the
  thumbnails into hooks of their own is the natural cut; it touches every caller of the API, so it
  is left for a refactor PR.
- `frontend/src/hooks/useWorkspaceSync.ts` 590: Share, join, resume and the marker of one shared
  workspace; the writer (`useWorkspaceWriter.ts`) and the shared tree (`useSharedTree.ts`) are
  already out. The next cut is Share (scan, builder, upload) into its own hook.
- `frontend/src/components/panel/WorkspaceTree.tsx` 693: the virtualised treegrid with its
  keyboard, drag and row; the row (`TreeRow`, `StateButton`, `RowButton`, `Thumbnail`, ~230
  lines) can move to `WorkspaceTreeRow.tsx` without other changes.
- `frontend/src/components/panel/ShareTab.tsx` 688 (594 before this story): see above.
- `curvyo-app/src/project.rs` 527 (411 before this story): the project commands plus the
  workspace's follow of a moved open file (`follow_move`) and the thumbnail write on Save; the
  follow belongs to `workspace.rs` once `AppState` exposes the project path there.
- `curvyo-editor-wasm/src/session/sync_events.rs`: `sync_event` went from about 131 to 66 lines
  (Refused, Removed, Presence, Acked and SnapshotDue are helpers now).
