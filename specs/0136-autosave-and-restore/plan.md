# Plan for autosave and restore (0136)

**Status:** Built, gate run (see the PR). Branch `story/autosave-and-restore`. Design: ADR 0079 (`docs/adr/0079-recovery-copies.md`), `adrs.md`, `ux-notes.md`.
No new crate, no new dependency, `.curvyo` `format_version` unchanged (`recovery.json` is a new record at format 1).

## Affected crates and modules

- `curvyo-ui-core` (pure, no clock): `recovery_schedule`, `recovery_record`, `recovery_prune`, `recovery_text`.
- `curvyo-document-core::container`: `pack_recovery`, `assemble`, `check_recovery_copy`.
- `curvyo-storage-io::recovery_copies` (write, list with lock state, read, delete, lock) and `curvyo-app/src/recovery.rs` (commands).
- `curvyo-editor-wasm`: `recovery_writer` (worker side) and the session calls `recovery_*` (delta export, asset hand-over).
- `frontend`: `lib/recoveryStore.ts` (IndexedDB, Web Locks), `workers/recoveryWriter.ts`, `hooks/useRecovery.ts`, `components/panel/RecoveryBlock.tsx`, File tab wiring.

## Decisions taken while planning

- **Locked copies do not count against the limits** of criterion 12 (a running session's copy is not "beyond"); they are never deleted. The newest unlocked copy is kept whatever its size; of the older unlocked copies at most 20 and 200 MB (with assets) stay. So up to 21 unlocked rows can exist; UX notes section 3 says "at most 20 rows". **Open point for the lead/PO**: criterion 12 reads "at most 20 copies beyond the newest", the row list can therefore show 21. Built as the criterion reads.
- **Seeding the worker replica** always uses a full `export_updates(never)` at the first due write of a session (a reseed, run at an idle moment), also for an opened file, instead of unpacking the opened file's bytes in the worker (ADR 0079 §4). One code path for Open, New, SVG, Restore and workspace documents; the cost is one full export at the first write (measured in the CPU-time test). **(Proposal)**
- Derived assets are laid out on disk as `assets/derived/<hash>.png`, mirroring the container, so `assemble` is a plain copy plus append.
- The page mints the 16-byte session key with `crypto.getRandomValues` (no host dependency).

## Tasks (tests first; each names its criteria)

### Part A: pure rules (`curvyo-ui-core`), commit 1
- [x] A1 `recovery_schedule` tests with a fake clock, then code. Criteria 1, 8, 13. Tests: step after a gap of 5 min or more is due at once; steps within the window give exactly one trailing write at expiry; no step since the last write: nothing is due; a clean project is never due and a due write on a clean project deletes the copy (only when a copy was written); a step during `writing()` stays pending; a failure does not count as a write and retries on the next step or 5 min after the attempt; the failure notice shows once and clears on the next success, a new failure after a success shows again; `ended()` resets everything.
- [x] A2 `recovery_record` codec with goldens in `curvyo-ui-core/tests/fixtures/recovery/` (current, newer refused, truncated, with and without `base`). Criteria 4, 6, 7, 9.
- [x] A3 `recovery_prune`. Criterion 12 and 11. Tests: newest unlocked kept whatever its size; locked never deleted and not counted; the newest plus 20 stay, a 22nd drops the oldest; 200 MB counts asset bytes; oldest first.
- [x] A4 `recovery_text`: row lines, damaged lines, help line, failure sentences, accessible names, "(recovered)" name. Criteria 4, 5, 6, 7, 9, 13.
- [x] A5 `curvyo-document-core::container`: `pack_recovery` (a flag on `pack_members`, no share record, no asset members), `assemble(copy, assets)`, `check_recovery_copy` (manifest and central directory, accepts an asset-less container, same named errors as Open). Round trip `pack_recovery` + `assemble` + `unpack` equals the original (assets and derived assets included). Criteria 1, 5, 7.

### Part B: host (`curvyo-storage-io`, `curvyo-app`), commit 2
- [x] B1 `recovery_copies` over a temp dir: write order assets, copy, record (temp then rename), a crash between leaves the previous copy intact, list with lock state (a held lock is listed as locked), read, delete copy (keeps `lock`), delete folder, startup cleanup of unlocked folders without `recovery.json`. Criteria 2, 3, 9, 11.
- [x] B2 `curvyo-app/src/recovery.rs` commands (`recovery_begin`, `recovery_write_asset`, `recovery_write_copy`, `recovery_delete_copy`, `recovery_end`, `recovery_list`, `recovery_read`, `recovery_path`), `app_local_data_dir()`, raw bytes over IPC, `base` of the saved file recorded by the host. Criteria 3, 6, 9, 10.

### Part C: session and worker (`curvyo-editor-wasm`), commit 3
- [x] C1 `recovery_writer` (replica imports the deltas, packs `pack_recovery`) and the `WasmRecoveryWriter` surface; tests: the packed copy assembles and opens equal to the session's document after several deltas; a reseed after a wipe. Criteria 1, 2.
- [x] C2 Session calls: `recovery_version_changed`, `recovery_export` (delta since the last handed version, never mid gesture), asset hand-over by name. Criterion 1, 2.
- [x] C3 CPU-time test (`curvyo-editor-wasm/tests/recovery_export_budget.rs`, uses `tests/cpu_time`) for the main-thread part on 10,000 objects. Criterion 2.

### Part D: frontend, commit 4
- [x] D1 `recoveryStore.ts` (IndexedDB `curvyo-recovery`, Web Locks; guarded), `recoveryWriter.ts` worker, `useRecovery.ts` (schedule loop, key, hooks into Save success, Discard, clean Close, New, quit, Restore adopting the key). Criteria 1, 3, 5, 8, 9, 11, 13.
- [x] D2 `RecoveryBlock.tsx` in the File tab (rows, Restore, armed Discard, damaged look, roving focus, start view of Frage 317 A, failed-write line and canvas notice). Criteria 4 to 7, 13 and the UX notes.
- [x] D3 Docs (R-SYS-013 status; `adrs.md` implementation notes; the `technical-debt.md` entry was already in): `docs/requirements.md` (R-SYS-010 extended, R-SYS-013), `docs/technical-debt.md` entry if absent, this plan ticked.

## Validation

Core tests per task above; `cargo nextest` on the touched crates while working; the full gate of `CLAUDE.md` section 7 once at the end. Criteria 2 (a drag at 60 fps), 10 (nothing to the network, by construction) and the browser/desktop wiring (4, 9, 11) are covered by the tests that can run headless; what cannot is listed in the report.
