# ADRs for autosave and restore

Status: Accepted (architect and customer, 2026-11-23; Frage 318). There is one new ADR, [0079](../../docs/adr/0079-recovery-copies.md). It is marked `needs-customer` (persistence), but it rests only on Fragen 265 B, 266 A+B and 267 A, which the customer answered on 2026-11-19; the customer accepted the ADR on 2026-11-23. No new customer question. Decisions marked **(Proposal)** are the architect's and vetoable at the demo.

- [ADR 0079](../../docs/adr/0079-recovery-copies.md) decides the following:
  - A recovery worker with its own wasm instance holds a replica fed by `export_updates` deltas, so packing never runs on the canvas thread (criterion 2).
  - Each write is a container without assets (`pack_recovery`, share record left out), and each asset is written once by hash. Restore assembles a normal `.curvyo` (`assemble`) and opens it through Open.
  - Location: `app_local_data_dir()/recovery/<key>/` on the desktop, and IndexedDB `curvyo-recovery` in the browser.
  - One copy per random session key, guarded by an OS file lock or a Web Lock.
  - Writes are temp-then-rename in the order assets, copy, `recovery.json`.
  - Pure rules (schedule, record codec, prune) live in `curvyo-ui-core`.
  - `.curvyo` `format_version` is unchanged. `recovery.json` is a new record at format 1.
  - No new crate and no new dependency.
- Extended, not replaced:
  - [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §7 (atomic writes; the "no lock files" rule is for synced folders, and the recovery folder is never synced) and §9 (a new record with its own `format_version`).
  - [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §2 (one new module each in `curvyo-storage-io`, `curvyo-ui-core`, `curvyo-editor-wasm` and `curvyo-app`, no new crate).
  - [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md) §2 (content-addressed assets make "write each asset once" possible).
  - [ADR 0031](../../docs/adr/0031-sync-engine-relay-protocol-and-transport.md) decision 9 (`pack_snapshot` without the share record).
  - [ADR 0037](../../docs/adr/0037-shared-workspace-list-rooms-and-keys.md) §8 (the background writer's worker pattern, a second worker beside it).

## Feature-local decisions

- 2026-11-23: **Crate and module placement.** The pure rules have no clock; callers pass monotonic ms. They live in `curvyo-ui-core`, one module each:
  - `recovery_schedule`: the rule of criterion 1, the failure notice of criterion 13, and `INTERVAL_MS` as a constant.
  - `recovery_record`: the `recovery.json` codec.
  - `recovery_prune`: criterion 12.

  Packing goes in `curvyo-editor-wasm::recovery_writer` (worker) with two pure functions in `curvyo-document-core::container`: `pack_recovery` and `assemble`.

  Writing, listing, locking and deleting go in `curvyo-storage-io::recovery_copies`, with commands in `curvyo-app/src/recovery.rs`. In the browser they go in `frontend/src/lib/recoveryStore.ts`, and the worker is `frontend/src/workers/recoveryWriter.ts`. Nothing for the PO.
- 2026-11-23: **`app_local_data_dir()`, not `app_data_dir()`.** The other records (machines, materials, formats) use the roaming data folder. On Windows that folder is synced by domain profiles, and ADR 0004 §7 allows pointing it at a synced folder. The recovery folder deliberately differs; do not align it later. A command returns the path for criterion 9's help line.
- 2026-11-23: **Restore adopts the copy (Proposal).** The restored session takes the copy's key and lock, so the row goes and a second crash before Save still recovers. Criterion 5's "the recovery copy goes with that Save" holds as written. **For the PO:** criterion 5 could say "the row goes because the project is open again", not "the copy goes".
- 2026-11-23: **Pruning keeps the newest unlocked copy whatever its size** (Proposal) and never touches a locked one. Only older copies count against 20 copies / 200 MB, so one crashed project with large pictures still reaches its row. **For the PO:** criterion 12 reads "beyond the newest copy" (applied in the spec, 2026-11-23).
- 2026-11-23: **Folder lifecycle.** Save deletes the copy, its assets and `recovery.json` but keeps `lock`, because the session runs on under the same key. Discard and a clean Close delete the whole folder. At start, an unlocked folder without `recovery.json` is a leftover of a clean session and is removed silently, never listed.
- 2026-11-23: **A failed write is retried at the next committed step, or 5 minutes after the failed attempt** (Proposal). There is no retry every second against a full disk. The notice of criterion 13 shows once and clears with the next success.
- 2026-11-23: **A due write that finds the project clean deletes the copy** (Proposal; criterion 8, for example after an undo back to the saved state).
- 2026-11-23: **A record older than its copy** is possible after a crash between the last two renames. It still restores the newer copy (Proposal, permit over refuse). The listing reads each asset-less copy and runs one cheap container check, and only a failure reads "Cannot be restored" (criterion 7).
- 2026-11-23: **A missing asset file does not refuse Restore.** The project opens as a file with a missing asset opens (Proposal).
- 2026-11-23: **Workspace documents** (`0095`): an open, dirty workspace document gets a recovery copy like any project. It is written without `share.json`, so a restored copy is a local project and never rejoins the room (criterion 10). **For the implementer:** check whether the open workspace document is ever dirty. If its records are written into the file anyway, criterion 8 means it never gets a copy, and that is correct.
- 2026-11-23: **Browser: no `navigator.storage.persist()` for this store** (Proposal). Firefox prompts for it, and the spec rules out popups. Eviction under storage pressure is accepted. **For the PO:** criterion 6 has no counterpart in the browser build: there is no file path to compare, so the row never says "File changed since" there.
- 2026-11-23: **Criterion 1's timing test.** The rule is driven by a fake clock in `recovery_schedule` tests. The CPU-time test covers the main-thread part of a due write (`export_updates` plus handing the assets) in `curvyo-editor-wasm/tests/cpu_time/`. The worker's pack is off the canvas thread and gets no frame budget. The ADR's sizes and costs are estimates until that test runs. A reseed (a full `export_updates`, after a wipe or a worker restart) runs at an idle moment, never inside a step.
- 2026-11-23: **No `format_version` row** in `specs/README.md`: `.curvyo` is unchanged. `recovery.json` format 1 gets golden files in `tests/fixtures/recovery/`, covering current, newer (refused) and truncated.
- 2026-11-23: **Debt for the lead** (this agent edits only ADR files and this file): the second wasm instance holds a second Loro replica of the open document, but no assets. Recorded beside the ADR 0077 asset-memory entry in `docs/technical-debt.md` (2026-11-23). Trigger: a measured out-of-memory.
