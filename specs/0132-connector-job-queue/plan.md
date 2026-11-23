# Plan for the job list on the connector (slice 1)

Implementer, 2026-11-23. Base: `story/connector-job-queue` on the 0132 Ready commit. Slice 1 =
criteria 1 to 9, 11 (without "restart pending") and 14. Criteria 12 and 13 are slice 2.
Customer defaults applied: Frage 301 A, 302 A, 303 A.

## Affected crates/modules

- `curvyo-connector-core`: new `src/job_list/` (`mod.rs` order and limits, `rights.rs`, `chain.rs`,
  `stored.rs`), `machine/list.rs` (glue), `message.rs` (additive messages), `access.rs`
  (`Kind::List`), `send_watch.rs` (L7), `assembly.rs` (`Job.text: Arc<str>`, whole-text check).
- `curvyo-connector-io`: new `src/job_store.rs` (ordered writer thread, restore), `hub.rs` and
  `hub/journal.rs` (list push, per-session transfers, store sync, `admin.log` lines), `session*.rs`
  (JobList before the first Status, list messages), `client.rs`, `watch.rs`, `send_tracked.rs`,
  `state_dir.rs` (Reset removes `jobs/`).
- `curvyo-connector-server`: nothing but the tests (fake controller flow, restart).
- `curvyo-app`: `connector.rs` commands (move, remove, run next), watcher forwards the list.
- `curvyo-editor-wasm`, `curvyo-ui-core`: send button gets "List full" and the waiting count.
- Frontend: `MachineControls.tsx` list block, `jobList.ts`, hooks.
- Docs: ADR 0071 and ADR 0035 section 7 amendment lines, ADR index row, `docs/design-system.md`
  row, `docs/technical-debt.md` entry (RAM use of waiting jobs), service guide line, spec lines
  (`0064` P13, `0125` S8).

## Tasks

Phase 1: core, io, server, goldens

- [x] 1. core: messages `JobAccepted.position`, `ListFull`, `JobList`, `MoveJob`, `RemoveJob`,
  `SetRunNext`; golden files `job_accepted_waiting`, `job_list_*`, move/remove/set_run_next; every
  old golden re-encodes byte for byte (AC 1, 2, 9).
- [x] 2. core: `job_list` order, limits, move table tests (AC 1, 3, 9).
- [x] 3. core: `rights::may` and `access` `Kind::List` tables whole (AC 5, 9).
- [x] 4. core: machine arrival in the list, waiting transfers, `Start` of the first waiting job,
  `Stop` only current, `JobList` push on change (AC 1, 2, 3, 4, 7, 9).
- [x] 5. core: `chain::next` table and machine chain (AC 11, 9).
- [x] 6. core: `stored` format and `restore` table (AC 6, 9).
- [x] 7. core: `SendWatch` reads `JobList` (L7, AC 8).
- [x] 8. io: `job_store` writer thread, restore at hub start, Reset removes `jobs/`, `admin.log`
  lines (AC 6, 7).
- [x] 9. io: hub and session: list push before the first Status, per-session transfers, rights
  (AC 2, 3, 5).
- [x] 10. io/server integration on the fake controller: three jobs, reorder, start, restart, chain
  with a failure (AC 4, 6, 9, 11).
- [x] 11. existing tests: "Connector busy" answers become "added to the list" (changes accepted
  behaviour, spec section).

Phase 2: app and frontend per `ux-notes.md`

- [x] 12. app: watcher forwards `JobList`; commands move, remove, run next; `Start` takes the first
  waiting job (AC 2, 3, 4, 5, 8).
- [x] 13. wasm/ui-core: send button states with a held job and "List full" (AC 1, ux 5).
- [x] 14. frontend: list block in the Machine controls footer, drag, keys, row actions, texts
  (AC 2, 3, 5, 11, ux 1 to 8).

Phase 3: docs

- [x] 15. ADR 0071 and 0035 section 7 amendment lines, ADR index row, design-system row, debt
  entry, guide line (AC 14).

## Validation

Golden files for every new message; table tests for rights, chain and restore; integration tests
on the fake controller without wall-clock sleeps (waits on events, flush barrier of the writer);
new tests flake-checked 20 times under CPU burners; full gate of `CLAUDE.md` section 7.
