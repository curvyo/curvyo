# ADR 0055: Material records in the per-user data directory: one JSON file per material, tied to a machine profile id, with its test history inside, edits as undo steps of the open project, model in `curvyo-laser-core`

**Status:** Proposed 2026-11-11, revised 2026-11-11, `needs-customer` (persistence: a new per-user file format; undo across the library). Built on Frage 184 A (customer, 2026-11-11: per-user files `materials/<uuid>.json` tied to a profile id) and the changed answer 189 B (material edits are undo steps, decision 9). Frage 190 (MAT-UNDO-HISTORY) open with default A (decision 9). Realises the material part of [ADR 0004](0004-persistence-and-cross-machine-sync.md) §6 for the laser family with the reading rules of [ADR 0038](0038-machine-profile-records.md) §1 to §5. Keeps the record model in `curvyo-laser-core` and defers `curvyo-library-core` again (ADR 0038 Consequences, `docs/technical-debt.md`). The operation's link to a record is [ADR 0054](0054-laser-settings-and-test-grids-in-the-project-file.md). Spec: [`0028`](../../specs/0028-material-test-library/specification.md) criteria 15 to 23, 29, 30.

## Context

A material record is a name, a thickness and, for one machine, the Cut, Score and Engrave sets that a test proved, with every test that fed it (spec criteria 16, 18). It is the maker's data, reused across projects, synced only as the data directory is (ADR 0004 §7), never sent anywhere (R-SYS-002, R-SYS-007). An operation points at it by id (ADR 0054 §1), so its id must be stable and unique across computers.

## Options

**Where the test history lives**

- **A. Inside the record, an append-only array of entries with their own ids (chosen).** One file per material, one read; the spec's shape. Two computers adding a test to the same material offline leave a sync conflict copy (the open "Record conflicts" debt); because each entry has its own id, the later merge of ADR 0004 §8 is a union by id and loses nothing.
- **B. A separate log file per record (`<uuid>.tests.jsonl`).** Still one file written by both computers, so still a conflict copy, and two files to keep consistent. Rejected.
- **C. One file per test.** Conflict-free, but 200 materials with a few tests each become thousands of files to list within the budget of criterion 30. Rejected.

**Crate**

- **A. `curvyo-laser-core`, beside the machine profiles (chosen).** Both records are laser-only, no merge exists yet, and one machine family exists (`CLAUDE.md` §5, §6). The envelope both records share moves into one module of the crate, which is the second user ADR 0038 waited for; the crate move waits for a second family or the merge.
- **B. A new `curvyo-library-core` now (ADR 0011 §2's slot).** It would hold two laser records and no merge; the file format would not change by moving later. Rejected for now.

## Decision

1. **Location and ids.** Desktop: `materials/<id>.json` under the data directory, next to `machines/` (ADR 0038 §1). Browser: `localStorage` key `curvyo.materials/<id>`. A name that is not `<uuid>.json` is ignored and never touched. `id` is a lowercase UUID v4 made in core from 16 caller bytes (ADR 0038 §5, the same function, renamed `new_record_id`); history entry ids the same way.
2. **Record format 1** (canonical writer as ADR 0038 §2: this key order, two-space indent, trailing newline; read, write, read is the identity):
   ```json
   {
     "format_version": 1,
     "id": "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c",
     "kind": "laser",
     "modified": 1762862400000,
     "name": "Birch plywood",
     "thickness_mm": 3.0,
     "machine": { "id": "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f", "name": "K40 in the garage" },
     "sets": {
       "cut": { "power_pct": 60.0, "speed_mm_min": 300, "passes": 2,
                "source": { "test": "5d1e7a2b-0c3f-4e6d-8a9b-7c6d5e4f3a2b" } }
     },
     "history": [
       { "id": "5d1e7a2b-0c3f-4e6d-8a9b-7c6d5e4f3a2b", "date": 1762862400000, "event": "tested",
         "kind": "cut", "set": { "power_pct": 60.0, "speed_mm_min": 300, "passes": 2 },
         "grid": { "columns": 5, "rows": 5, "power_from_pct": 20.0, "power_to_pct": 100.0,
                   "speed_from_mm_min": 120, "speed_to_mm_min": 1200, "cell_mm": 10.0, "gap_mm": 3.0 },
         "cell": { "column": 2, "row": 1 } }
     ]
   }
   ```
   - `machine.id` is a profile id (a UUID, or `generic-laser`); `machine.name` is the profile's name when the record was made, for display only. The record is shown and linked only on pages whose profile copy (ADR 0039) has that id.
   - `sets` has at most the keys `cut`, `score`, `engrave`; a record may have none. Numbers as ADR 0054 §1 (power 1 to 100 with at most one decimal, speed an integer of at least 1, passes 1 to 20). `source` is `{ "test": "<history id>" }` (that entry exists and is a `tested` entry of the same kind) or `"edited"`.
   - `history` is append-only. `event` is `tested` (with `set`, `grid`, `cell`), `edited` (with `set`) or `removed` (no `set`). No write removes or changes an entry; replacing or deleting a set appends one. The one exception is the undo of the step that appended the entry (decision 9, Frage 190 A): the entry goes, and redo adds it back with the same id.
   - `thickness_mm` 0.1 to 100, one decimal; `name` 1 to 60 characters; `modified`, `date` Unix milliseconds passed in by the caller (core has no clock). `modified` rises with every write: `max(now, previous + 1)`, also for an undo or redo, so an operation's link can tell a newer record from an older one (ADR 0054 §1, §3).
3. **Reading** (ADR 0038 §3 applied): text over 256 KiB, a JSON or type error, an unknown key (`deny_unknown_fields`), a value out of range, a `source` naming no fitting entry, more than 500 history entries, a kind other than `laser`, or an id that differs from the file name is **damage**: the record is skipped, the load returns **one notice per record** with its file name and reason (spec criterion 29), and the file is never rewritten, renamed or deleted by a read. A `format_version` above 1 is skipped with its own notice and never written. At most 1,000 records in file-name order. A speed above the machine's current max feed is not damage (the profile can change after the test).
4. **Writing.** Every edit returns the host write (`Write { id, text }` or `Remove { id }`, as `machine_profiles.rs`). An edit that would pass 500 history entries or 256 KiB is refused with a notice and writes nothing. Deleting a material removes its file; operations keep their stored numbers (ADR 0054 §2); Ctrl+Z in the same session writes it back (decision 9).
5. **Order.** The list order is a library preference, `material-order.json` in the data directory (`{ "format_version": 1, "order": ["<id>", ...] }`, browser key `curvyo.material-order`): listed ids first in that order, then the others by name. Unknown ids are ignored and kept until the next order write. It is not part of a record, so reordering never rewrites a record.
6. **Remembered grid values** (spec criterion 2) are this computer's preference: `test-grid-choice.json` in the config directory (`{ "format_version": 1, "by_machine": { "<profile id>": { <the recipe of ADR 0054 §4 without kind> } } }`, at most 100 machines, browser key `curvyo.test-grid-choice`). A damaged file reads as empty and is never rewritten by a read.
7. **Crates, modules, boundary.**
   - `curvyo-laser-core` (pure, wasm32, `#![forbid(unsafe_code)]`):
     `record_envelope` ("the envelope every per-user laser record shares"): id and file-name check, `format_version` gate, size limit, `new_record_id`; `machine_profile_file` moves onto it with no behaviour change (its goldens guard that).
     `material` ("the material record with its sets, history and limits"): newtypes `PowerPercent` (tenths, `u16`), `Passes` (`u8`), `Thickness` (tenths of mm), `FeedRate` reused from `machine_profile`.
     `material_file` ("reads and writes the material library's files"): the record, `material-order.json`, `test-grid-choice.json`.
     `materials` ("the maker's materials as one list"): load from `(name, text)` pairs with notices, filter by profile id, save a test, replace, edit and remove a set, rename, set thickness, delete, order, and `revert` (decision 9: write a record's earlier text back only if its current text is the expected one); each returns its host writes.
   - `curvyo-editor-wasm`: the session holds `Materials` beside `MachineProfiles` and the undo journal of decision 9 (`session/materials/undo.rs`), makes each record edit one document step, and turns the writes into host calls; strings and scalars across the boundary (ADR 0001 §5). Record text is decoded only in Rust; the frontend never parses a record.
   - `curvyo-app`: `materials.rs`, Tauri commands that move text and decide nothing: `list_materials`, `write_material`, `remove_material`, read and write of `material-order.json` and `test-grid-choice.json`, over `curvyo-storage-io`'s `read_dir_texts`, `write_atomic`, `read_optional`, `remove_file`. Browser: the same in `frontend/src/platform/host.ts` over `localStorage`.
8. **Goldens** in `curvyo-laser-core/tests/fixtures/materials/`: `full.json` (three sets, `tested`, `edited` and `removed` entries; reads, writes back byte for byte), `generic_laser.json`, `no_sets.json`, `history_max.json` (500 entries, fits 256 KiB), and one damaged file each for: an unknown key, power 0.5, power 100.5, speed 0, passes 21, thickness 0 and 100.1, a name of 61 characters, a `source` naming no entry, 501 entries, an id that differs from the file name; `newer_format.json` (format 2); `order.json`, `order_unknown_id.json`, `test_grid_choice.json`.

9. **Material edits are undo steps of the open project (Frage 189 B).**
   - **Which:** Save as material (new record, added or replaced set), a set edit or removal, rename, thickness, delete. Not undo steps (Proposal): the order (a list preference, as panel layout) and the remembered grid values.
   - **The step:** each edit is one document step of the project in which it was made, labelled "Save as material", "Edit material", "Rename material" or "Delete material". It writes the root marker and refreshes every linked operation of the project (ADR 0054 §3), so Ctrl+Z takes back the record and its effect on every operation together. Every Ctrl+Z entry stays a document step, so the stacks of ADR 0014 §6 and the history list keep agreeing (`0020` criterion 55).
   - **The journal:** the session keeps, per such step, `{ material id, before, after }`, each the record's whole text or none (created, deleted), in memory only, beside the stacks.
   - **Undo:** the document part as ADR 0014 (three-way, per container). Then the record part, by the same rule: if the loaded record's text is still `after` (or the record is still absent), `before` is written back (with a new `modified`; recreated when it was deleted, removed when it was created); otherwise the record is left as it is and the notice says "Birch 3 mm changed since, the material was not undone." **Redo** is the same with `before` and `after` swapped. History entries: decision 2.
   - **Limits, honestly:**
     (a) Only Ctrl+Z and Redo of the session where the edit was made reach the record. The journal ends with that session (New, Open, quit), as its stacks do; in a later session the edit is permanent and the record's history keeps the old set.
     (b) Another project never takes the edit back; its linked operations follow the record live and refresh at its next save.
     (c) History-list actions (go to version, object undo, branches, `0041`, `0042`) restore only the project; the next save brings the stored numbers to the record again.
     (d) A change on disk this session has not loaded (another window, folder sync from another computer) is overwritten by an undo as by any library write today (the "Record conflicts" debt).
     (e) The journal holds at most 16 MiB of record text (Proposal); the record part of the oldest steps is dropped first, and their undo restores only the project with "Birch 3 mm can no longer be undone."
     (f) A host write that fails (browser quota) leaves the record as it is with a notice; the document part is undone, and the next save refreshes the stored numbers to what the record holds.

## Consequences

- Ctrl+Z reaches a per-user file, within one session: the first undo whose effect leaves the project (decision 9). The limits (a) to (f) are what keeps it honest; persisting the journal in the project was rejected (it would copy the maker's library into a shared file).
- A third per-user record file, synced as files. Two computers editing the same material offline leave a conflict copy, which the reader ignores (not a `<uuid>.json` name); detection stays the "Record conflicts" debt, and the history entry ids make its merge a union.
- Browser `localStorage` (about 5 MiB per origin) holds about 200 materials of average size with history; a refused write leaves the record unchanged in memory and on disk and shows a notice. Desktop has no such limit.
- A material of a deleted profile stays on disk and is shown on no page ("Show all machines" is out of scope).
- `curvyo-library-core` waits for a second machine family or the record merge; the file formats of ADR 0038 and this ADR do not change by that move. The debt entry is updated.
- Budgets (criterion 30): the Materials list of 200 records within 50 ms; the load of 200 records with history within 200 ms (ADR 0038's load budget), thread CPU time under `curvyo-editor-wasm/tests/cpu_time/`.
