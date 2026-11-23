# ADR 0079: Recovery copies: a worker replica writes an asset-less container plus assets once each, into the local app data folder, keyed and locked per session

**Status:** Accepted (customer, Frage 318, 2026-11-23; architect, 2026-11-23), `needs-customer` (persistence, `CLAUDE.md` §3). The customer has already decided what this ADR rests on, on 2026-11-19: **Frage 265 B** (only in the app data folder, never in the maker's, workspace or synced folders), **Frage 266 A+B** (after every step, at most every 5 minutes; the rule of spec criterion 1) and **Frage 267 A** (an inline block in the File tab). Nothing in this ADR is a new customer decision; it is accepted with the slice PR. Spec: `specs/0136-autosave-and-restore`. Choices marked **(Proposal)** are the architect's and can be vetoed at the demo.

**In plain words (for the customer).** While a project has unsaved changes, Curvyo keeps a hidden copy of it in its own local folder. On Linux that is `~/.local/share/<app id>/recovery`, on macOS `~/Library/Application Support/<app id>/recovery`, on Windows `%LOCALAPPDATA%\<app id>\recovery`, and in the browser the site's own storage. It is never put next to your file or in a synced folder. The copy is written in the background, so drawing never stutters. Each picture is written only once, not again every five minutes. When the app crashes, the copy waits for you in the File tab. When you save or discard, it is deleted. The `.curvyo` file format does not change.

## Context

Spec criteria 1 to 13 ask for a copy of the whole project (pages, history, document formats, assets). The copy is written at most every 5 minutes, without a dropped frame at 10,000 objects (criterion 2). It replaces the previous copy only when complete. It lives outside every folder that can be synced, one per open session (criterion 11), and at most 20 copies and 200 MB are kept (criterion 12).

Things already in place:
- `pack` (`curvyo-document-core::container`) writes `document.loro`, `document.json` and every asset of `document.assets` into one zip. The asset store is not part of Loro. It holds pictures, document fonts and derived assets (ADR 0017, 0047, 0077), up to `MAX_PROJECT_ASSET_BYTES` = 512 MiB (+64 MiB derived).
- `Document::export_updates(since)` gives the Loro ops since a version, cheaply, and `import_updates` merges them (`history/updates.rs`).
- The background writer of `0095` (ADR 0037 §8) already packs files in a Web Worker with its own wasm instance, so the open document's frames never wait.
- `curvyo-storage-io::filesystem::write_atomic` writes temp-then-rename (ADR 0004 §7).
- The other app records use Tauri's `app_data_dir()`. On Windows that is the *Roaming* profile, which domain profiles sync, and ADR 0004 §7 lets a maker point the data directory at Nextcloud or Syncthing.

## Options

**Where the packing runs** (criterion 2)

| Option | Why it won or lost |
|---|---|
| **A. A recovery worker with its own wasm instance and a replica of the open document, fed `export_updates` deltas (chosen)** | `export_updates` on the main thread is incremental (estimated well under a frame for a step; confirmed by the `cpu_time` test), while the snapshot, JSON export, deflate run in the worker. This is the pattern of the trace worker and the background writer. |
| B. `pack` on the main thread in `requestIdleCallback` | An idle slice is at most 50 ms. Exporting the snapshot and JSON of 10,000 objects plus deflate does not fit, and the trailing write can fall in a drag. |

**What one write contains** (the asset store is outside Loro)

| Option | Why it won or lost |
|---|---|
| A. A full `.curvyo` on every write | Restore would be a plain Open. But each write re-deflates and rewrites every asset: up to 576 MiB, estimated several seconds of worker CPU and 576 MB of disk every 5 minutes. The worker replica would also hold a second copy of all assets, which means over 1 GiB across two wasm32 instances in the worst case. One such copy alone breaks criterion 12's 200 MB. |
| **B. A container without assets on every write, plus each asset as its own file written once (chosen)** | A write is the Loro snapshot plus `document.json` (estimated a few MB for 10,000 objects). Assets are content-addressed (ADR 0017 §2), so the page sends each new hash once, and it sends it to the host, not to the worker. The worker holds no asset bytes. The cost is a step at Restore that assembles a normal `.curvyo` (§5). |

**Location** (criterion 9, Frage 265 B)

| Option | Why it won or lost |
|---|---|
| **A. `app_local_data_dir()/recovery/` (chosen)** | Local on all three systems (Windows `%LOCALAPPDATA%`, not Roaming). It is never the configurable, possibly synced data directory of ADR 0004 §7. |
| B. `app_data_dir()/recovery/` like machines and materials | Roaming on Windows, and it follows a data directory that may sit in a synced folder. That breaks criterion 9. |
| C. Beside the project file | Rejected by the customer (Frage 265). |

**Liveness** (criterion 11: do not offer a copy whose session is still running)

| Option | Why it won or lost |
|---|---|
| **A. An OS lock held by the running session: `std::fs::File::try_lock` on the desktop (std since 1.89, the toolchain is 1.99.1, no new dependency) and the Web Locks API in the browser (chosen)** | A crash or a power cut releases the lock by itself. There is no stale state to time out. The folder is never synced, so ADR 0004 §7's "no lock files" does not apply. |
| B. A heartbeat timestamp in the record | It needs a clock threshold, and it misreads a sleeping laptop as a crash. |

## Decision

1. **Pure rules in `curvyo-ui-core`**, one module each, with no clock: callers pass `now`.
   - `recovery_schedule`: the rule of criterion 1, on a *monotonic* clock in ms. `step(now)`, `due(now)`, `writing()`, `written(now)`, `failed(now)`, `ended()` (Save, Discard, Close) and the failure notice of criterion 13: shown once, cleared by the next success.
     - `INTERVAL_MS = 300_000` is a constant, not a setting.
     - A clean project is never due (criterion 8). A due write that finds the project clean deletes the copy instead (Proposal).
     - A failed write does not count as a write. It is retried at the next committed step, or 5 minutes after the failed attempt when no step comes (Proposal: no retry every second against a full disk).
   - `recovery_record`: the codec of `recovery.json` (§3), with `format_version` 1, refusing a newer version.
   - `recovery_prune`: criterion 12 over `(key, bytes, last_step, locked)`. It counts a copy's bytes with its assets and deletes oldest first beyond 20 copies or 200 MB. It **never deletes a locked copy** (criterion 11) and **always keeps the newest unlocked copy, whatever its size**: one crashed project with 250 MB of pictures must reach its row. Only the older copies count against the limits.
   - The row texts of the File tab block.
2. **One copy per session key.** A random 16-byte key (hex) is made when a session opens or starts a New project, and Restore reuses the key of the copy it restores (§6). A never-saved project is the same as any other with `base: null`. The key is not derived from the path: two windows on one file are two copies (criterion 11).
3. **Desktop layout**, `app_local_data_dir()/recovery/<key>/`:
   - `copy.curvyo`: the container without asset members (§4).
   - `assets/<sha256>.<ext>`: one file per asset, written once, before any copy that refers to it.
   - `recovery.json`: `format_version`, `key`, `name` (or none), `base` (the saved file's path plus its modification time and length at open or Save, or none), `last_step_ms` (wall clock, for display only), `pages`.
   - `lock`: held with `try_lock` while the session runs.

   Each file is written with `write_atomic`, in the order assets, `copy.curvyo`, `recovery.json`, as `devices.rs` writes `group.json` last. So `copy.curvyo` is always one complete write, old or new. A crash between the last two renames leaves the previous write's record next to the newer copy: the row shows a name, time and page count up to 5 minutes old, and Restore opens the newer copy (Proposal, permit over refuse). There is no hash, since the zip's own checks catch damage. A copy holds no assets, so it is small, and the listing reads each copy and runs one pure check in `container` (central directory readable, manifest `format_version` not newer). A failure lists the row as "Cannot be restored" with the named error of `0001` criterion 7.

   The code is a new module `curvyo-storage-io::recovery_copies` (write, list with lock state, read, delete the folder, lock), with `curvyo-app/src/recovery.rs` commands. One more command returns the folder path for the help line of criterion 9.
4. **The worker** (`frontend/src/workers/recoveryWriter.ts`, Rust in `curvyo-editor-wasm::recovery_writer` with its `wasm_recovery_writer` surface):
   - It holds one replica, seeded with a Loro snapshot of the open document (`export_loro_snapshot`, read by `Document::from_snapshot` in `curvyo-document-core`), on open, New, SVG, Restore and workspace documents alike; the opened file's bytes are not unpacked in the worker.
   - The page hands it `export_updates` since the version it last handed. It does this at the next idle moment when the schedule is due, and never for a drag in flight.
   - The worker imports the updates, packs `pack_snapshot` minus assets (one new pure entry point in `container`, e.g. `pack_recovery`, sharing `pack_members`). It posts the bytes back, and the page writes them through the host.
   - Assets do not pass through the worker. With each due write the page asks its session for the assets whose hash it has not yet handed, and hands them to the host, which writes them before the copy (one new session call, e.g. `assets_since(&handed)`).
   - It reseeds with a snapshot (not a full `export_updates`, which does not read back after a history wipe) whenever an import fails with `SyncError::Pending`, the history epoch changes (a wipe, ADR 0014), or the worker restarted. A reseed is a full `export_updates` on the main thread; it is rare and runs at an idle moment, never inside a step.
   - The share record is left out (as `pack_snapshot` does, ADR 0031 decision 9): a restored copy is a local project and must never rejoin a room as the same replica (criterion 10).
5. **Restore** reads the copy and its assets and calls one pure function in `curvyo-document-core::container`, `assemble(copy, assets) -> Vec<u8>`. That function adds the asset members and gives a normal `.curvyo`, which opens through the ordinary Open path with its named errors (criterion 7). It opens as unsaved, with no path and the name "<name> (recovered)" or "Untitled".

   An asset file that is missing does not refuse the copy. The project opens the way a file with a missing asset opens (Proposal, permit over refuse).
6. **Restore adopts the copy:** the new session takes the copy's key and lock instead of deleting it. That way a second crash before Save still recovers it, and the row goes because the copy is now locked. Save or Save as (after the file write succeeded) deletes `recovery.json`, `copy.curvyo` and `assets/`, but keeps `lock`: the session runs on under the same key, and on Windows a file held open cannot be deleted. Discard in the unsaved-changes check and a clean Close release the lock and delete the whole folder. New writes nothing until the first step (criterion 3). Writes and deletes of one key run in order on one queue, so a trailing write that lands after Save cannot bring the copy back.
7. **Start of the app:** list, skip locked copies, silently remove an unlocked folder with no `recovery.json` (the leftover of a session that was clean when it ended, including a crash right after Save; never a "Cannot be restored" row), prune (§1), and show the rest. For a copy with `base`, the row says "File changed since" when the file's time or length differs or the file is gone (criterion 6).
8. **Browser:**
   - One IndexedDB database `curvyo-recovery` with two stores. `copies` maps a key to the record and the copy in **one** transaction, so record and copy always belong together. `assets` maps `<key>/<sha256>` to bytes, written before the copy.
   - Liveness is `navigator.locks.request("curvyo-recovery-<key>")` for the life of the tab, and the listing uses `navigator.locks.query()`.
   - No `navigator.storage.persist()` request for this store (Proposal). Firefox shows a prompt for it, and the spec rules out popups. Eviction under storage pressure is accepted.
9. **No format change.** `.curvyo` `format_version` and `CURRENT_LORO_SNAPSHOT_VERSION` stay. The asset-less copy is never a file the maker sees: Restore assembles a normal container. `recovery.json` is a new internal record with its own `format_version` 1 (ADR 0004 §9) and golden tests in `tests/fixtures/` (`CLAUDE.md` §5).
10. **No new crate and no new dependency.** `zip`, `serde_json` and `thiserror` are already in the crates concerned. The lock is std on the desktop and a browser API in the browser.

## Consequences

- A second wasm instance holds a second Loro replica of the open document, but no assets: about one more document's worth of memory. This is recorded next to the ADR 0077 debt entry (whole asset store in wasm memory). Trigger: a measured out-of-memory.
- A large picture is written to the recovery folder once per session, the first time it is due. Inserting a picture in a 200 MB-plus project therefore costs one large write, not one every 5 minutes.
- Criterion 6 has no counterpart in the browser build, which has no path to compare. The row there never says "File changed since" (note for the PO in the spec's `adrs.md`).
- A crash between the last two renames on the desktop leaves a record up to 5 minutes older than its copy (§3). This is rare (two renames in microseconds) and accepted, because the alternative is one file that bundles the record with the copy, which means a format inside a format.
- The 5-minute constant lives in `curvyo-ui-core::recovery_schedule`. Changing it after the demo is one line plus its test.
