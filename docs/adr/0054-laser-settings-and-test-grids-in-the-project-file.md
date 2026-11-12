# ADR 0054: Laser settings and test grids in the project file: one atomic settings register per operation, a material link with stored numbers and own overrides, a test grid as a tagged group with one grid row, format 43

**Status:** Accepted 2026-11-12 (PR #333), revised 2026-11-11, `needs-customer` (document model and file format). Built on the customer's answers of 2026-11-11: Frage 183 A (one folded Test grid row), 185 B (an operation points to its material by id, with the numbers stored beside the id), 186 B (the label settings are the maker's, as in LightBurn), and the changed answers 187 B+C (the stored numbers refresh on every material edit and on every save), 188 B (a typed number keeps the link and differs from the material), 189 B (material edits are undo steps; the record side is [ADR 0055](0055-material-records.md) §9). Frage 190 open with a default (ADR 0055). Extends [ADR 0046](0046-laser-operations-record.md) §1 (one more register on an operation, one more kind of row) and §4 (copy of a grid), follows the atomic-map precedent of [ADR 0039](0039-machine-profile-copy-in-the-project-file.md) §1, the tagged generated children of [ADR 0030](0030-stored-tiling-nodes.md) §2 and the reserved non-step label of [ADR 0042](0042-page-kinds.md) §9. The material record itself is [ADR 0055](0055-material-records.md). Spec: [`0028`](../../specs/0028-material-test-library/specification.md). Feature notes: [`0028` adrs.md](../../specs/0028-material-test-library/adrs.md). On acceptance, a note goes on ADR 0046's Status line (§1: `op_settings`, `op_grid`; §4: a grid copy gets its own row).

**Note (ADR 0056, proposed 2026-11-12):** §1 and §2: `main` of `engrave` and of the hatch of `engrave_outline` may hold `interval_mm` (0.01 to 2.00); operation-only settings live in the new `op_options` register (0056 §5, §6).

## Context

`0101` and `0103` stored no power, speed or passes. `0028` adds them to the work operations, adds a generated test grid whose cells each run with their own numbers, and lets an operation take its numbers from a material in the maker's library, with numbers the maker types on top. Three things must hold for a laser: a set of numbers that runs was chosen by someone (never mixed from two concurrent edits); a project opened on a computer without the maker's library still runs with known numbers; the numbers that will run are visible before the run.

## Options

**Where an operation's settings live**

- **A. One atomic register `op_settings` per operation, holding the numbers, the material link and the overrides (chosen).** A merge of two edits keeps one whole value: never the power of one material with the speed of another, never an override typed for another material. A pick is one write of link and numbers together.
- **B. One register per number (`op_power`, `op_speed`, ...).** Two concurrent picks of two materials produce a set no test proved. Rejected.
- **C. Settings on the assigned items.** Refused by concept §1 (no per-object override). Rejected.

**How an operation refers to a material (Frage 185 B)**

- **A. Link plus stored numbers in the same register (chosen).** The link names the record; the stored numbers run whenever the record cannot be read here.
- **B. Link only.** A file opened elsewhere cannot run. Rejected by the customer's condition.

**A typed number on a linked operation (Frage 188 B)**

- **A. `overrides` beside the stored numbers, per field (chosen).** The stored numbers stay the material's as last seen, so "Reset to material" works also where the record is missing, and a refresh never touches what the maker typed.
- **B. Overrides in their own register.** Survives a concurrent refresh, but a concurrent pick of another material would keep an override typed for the first one: a set nobody chose. Rejected.
- **C. The typed number replaces the stored number, plus a list of "own" fields.** The stored numbers would no longer be the material's; Reset to material could not work without the record. Rejected.

**When the stored numbers are refreshed (Frage 187 B+C)**

- **A. At every material edit in this project (in its step), every save, every pick and every output, guarded by the record's `modified` (chosen).** The guard stops two computers whose library sync lags from rewriting each other's stored numbers at every save.
- **B. The same without the guard.** Ping-pong commits between the maker's computers until the library has synced. Rejected.

**The test grid**

- **A. An ordinary group tagged under `x.curvyo.laser.*`, its items tagged per cell and label, and one operation node with `op_grid` that is a member of the group (chosen).** The row is an ADR 0046 operation, so Show, Output, rename, armed trash, order, Cuts last, the nearest rule, the delete sweep and undo come with it. Cells keep their numbers on themselves, so a cell moved by hand still runs with what it was made with.
- **B. A new node shape `test_grid` with its own children.** A second drawing walk, delete path and row type for one feature. Rejected.

## Decision

1. **`op_settings`, one atomic map on an operation node** (ADR 0046 §1). Absent means **No settings**. Shape:
   ```text
   op_settings = {
     main:      { power_pct: 60.0, speed_mm_min: 3000, passes: 1 },
     outline:   { power_pct: 25.0, speed_mm_min: 6000, passes: 1 },   // engrave_outline only
     material:  { id: "<material uuid>", name: "Birch 3 mm",
                  outline_from: "score" | "engrave", modified: 1762862400000 },  // absent: own numbers
     overrides: { main: { power_pct: 65.0 }, outline: { passes: 2 } }  // only with material; absent: none
   }
   ```
   Without `material`, `main` and `outline` are the operation's own numbers. With it, they are the material's numbers as last seen and `material.modified` is the record's `modified` (ADR 0055 §2) they were taken from; `overrides` holds the numbers the maker typed on this operation, each field optional. Ranges: `power_pct` 1 to 100 with at most one decimal (percent of the profile copy's max power, ADR 0039); `speed_mm_min` an integer of at least 1; `passes` 1 to 20; `name` at most 60 characters; `modified` an integer of at least 0; `outline_from` says which material set fed the outline (Engrave when the material has no Score set). The main set is the kind's set: Cut for `cut` and `cut_out`, Score for `score`, Engrave for `engrave` and the hatch of `engrave_outline`. A speed above the machine's max feed is not damage: the line names it and `0029` refuses it.
   **Damage:** not a map, an unknown key, a missing or out-of-range number, an `outline` on another kind, a link without an id or `modified`, `overrides` without a link, an empty `overrides` or role map, an unknown key in it, an `overrides.outline` on another kind: the operation reads **Damaged settings**, runs nothing, is saved back unchanged and never repaired on read (ADR 0033 §4). An Unknown operation keeps its register untouched (R-MFG-010).
2. **Which numbers run (one pure rule, `curvyo-laser-core/src/operation_settings.rs`).** Given the stored value, the kind, the loaded materials and the page's profile id, the **material numbers** are:
   - **Linked, live:** the record with that id loads, is tied to this profile id (ADR 0055 §2) and has the needed set: its current sets. Shown "<name>".
   - **Linked, stored:** the record is missing, damaged, newer, of another machine, or lacks the set: the stored `main` and `outline`; the line says which.
   The numbers that **run** are the material numbers with each override put over its field; with any override the line says "Differs from <name>" and names the material's number beside each typed one. **Own:** no link, the stored numbers run. **None** and **Damaged:** nothing runs.
   In code: `MaterialLink` gains `modified: u64`; `StoredSettings` gains `overrides: Overrides` (`main`, `outline`: `SetOverride { power: Option<PowerPercent>, speed: Option<FeedRate>, passes: Option<Passes> }`); `SettingsDamage` gains `BadOverride`; `SettingsState::Live` and `Stored` carry `material: RunSettings` (the material numbers), `run: RunSettings` (with overrides) and the overridden fields; `Live.stored` and `behind()` go (the line no longer shows a stored copy). New pure functions: `refresh(read, kind, materials, profile_id, guard) -> Option<StoredSettings>` (the value to write when the operation is live and the material numbers, name or `outline_from` differ from the stored ones; overrides kept; with the guard only when the record's `modified` is above the link's), `edit_number(...)` and `reset_to_material(...)` (decision 3). `curvyo-document-core` hands the map over as plain values and decides nothing (ADR 0033 §3); the session passes the loaded library in. **`0029` reads `resolve(..).run()`**, overrides included.
3. **Writes of `op_settings`** (`curvyo-document-core/src/operation.rs`):
   - **Use material** (step "Use material") writes the link with the record's `modified` and the material numbers, without overrides; picking the linked material again also clears them. Several operations: one step; an operation whose kind has no set in the material is skipped.
   - **Edit settings** (a typed number, step "Edit settings"): own numbers, the whole map with the new number; linked, the link stays and the field goes into `overrides`, or leaves it when the number equals the material number.
   - **Reset to material** (step "Reset to material", shown only with overrides) removes `overrides`. **Own numbers** in the pick (step "Use own numbers") removes link and overrides and keeps the numbers that ran as own numbers.
   - **A material edit made in this project** (ADR 0055 §9) writes `refresh` without the guard into every linked operation of the project in the material edit's own step, and writes `x.curvyo.laser.material_edit` = `{ id, modified }` on the document root so the step exists also when no operation points to the material. Nothing reads the marker; every build keeps it.
   - **Save:** before the file is written, `refresh` with the guard for every linked operation, as one commit with the reserved label `refresh_material_numbers`: no step on the stacks, no row in the history list (as `migrate_format`, ADR 0042 §9); nothing is committed when nothing differs. Undo after a save does not bring the old stored numbers back.
   - **Output** (`0029`, contract): `refresh` without the guard, so the file carries what last ran.
   - **Nothing else writes it:** no open, no library load, no remote import, no history-list action on the library.
4. **The test grid, on the laser page:**
   - **Top group** ("Test grid <N>", an ordinary group) carries `x.curvyo.laser.test_grid`, one atomic map: `kind` (`cut`, `score`, `engrave`), `columns`, `rows` (2 to 12), `power_from_pct`, `power_to_pct`, `speed_from_mm_min`, `speed_to_mm_min`, `cell_mm`, `gap_mm`, and `labels` (on or off, `kind` `engrave` or `score`, never `cut`, `height_mm`, `power_pct`, `speed_mm_min`, `passes`, per Frage 186 B). Atomic, as ADR 0030's `tiling`.
   - **The mark** is `x.curvyo.laser.test_grid_worked` = `{ column, row }` on the top group, absent when nothing is marked. A rebuild keeps it when a new cell has the same power and speed (rewritten to that cell's index) and removes it otherwise, in the same commit.
   - **Cells** carry `x.curvyo.laser.grid_cell` = `{ column, row, power_pct, speed_mm_min }` (one pass). **Labels** carry `x.curvyo.laser.grid_label` = `"power"`, `"speed"` or `"unit"`. A tag counts only on a descendant of a top group with a readable `test_grid`; elsewhere it is inert and kept.
   - **Hand edited** is derived, never stored: the pure layout of `curvyo-laser-core/src/test_grid.rs` is compared, with an explicit tolerance, to the tagged items. Rebuild replaces the tagged items; untagged items are left alone.
   - **The grid row** is an operation node (ADR 0046 §1) with `op_kind` the grid's kind, `op_grid` = `true`, no `op_settings`, and exactly one membership key `member.<top group id>`. Its section and place follow its kind.
   - **Running a grid row:** for each leaf whose nearest key (ADR 0046 §7) is the grid row's, a label runs with the label settings, a cell with its own `grid_cell` numbers and the grid's kind, any other leaf runs nothing and is counted as not assigned. Labels run before cells; cells row by row from the top left (`0029` may reorder inside the grid).
5. **Grid lifetime** (session batches, one undo step each):
   - **Make grid** writes the group, its items and the row in one commit.
   - **Deleting the top group** (delete, cut, ungroup) also deletes its row, in the same step: the membership-key sweep of ADR 0046 §3 deletes a grid row whose key died instead of removing only the key. A row whose group is gone by a merge (a peer deleted the group) reads "no grid" and runs nothing until the next local step, whose sweep deletes it like any dead key (amended 2026-11-12 to match the build: keeping it renamable would need the sweep to tell a key that died in this step from one a merge left dead, recorded on every import path; the row has nothing left to run, so removing it loses nothing; undo of that step brings it back with its key).
   - **Deleting the grid row** keeps the items, which run nothing until a row is added again.
   - **Copy, Duplicate, Paste of a top group** (amends ADR 0046 §4): the copy's mark is removed, its key is not added to the original's row, and a new row with the copy's key is added, in the commit that creates the copy.
6. **`format_version` 43.** A build of 42 would read a grid row as an ordinary operation over the whole group: its copy would share the original's row and its delete would leave a row without a grid, silently. ADR 0033 §7 refuses that by a bump. **Migration: none.** A format 42 file opens with "No settings" on every operation and is not rewritten. Goldens in `curvyo-document-core/tests/fixtures/`:
   - `laser_settings_v43.curvyo`: an Engrave + Outline operation linked to a material (both sets, `modified`) with a power override on the hatch, a linked Cut without overrides, an own-numbers Score, an operation with no settings, a `material_edit` marker, a 3 x 2 Cut grid with labels, a mark and its row;
   - `laser_settings_damaged_v43.curvyo`: settings not a map, power 0, passes 21, a link without an id, a link without `modified`, an `outline` on a Cut, `overrides` without a link, an empty `overrides`, an `overrides.outline` on a Cut; each reads Damaged settings and saves back unchanged;
   - `test_grid_hand_edited_v43.curvyo` (a moved cell, a deleted label) and `test_grid_orphan_row_v43.curvyo` (a row whose group is gone).

## Consequences

- What runs depends on the maker's library: the same file runs the edited material on the computer that has it and the stored numbers elsewhere, always with the overrides. The line shows which and why; `0029` freezes them at output.
- A save can change the document (Frage 187 C): the refresh commit is not an undo step and not a history row, as the format migration. It writes only where the record changed outside this project's own steps (another project, another computer), so it rarely runs; when it does, an undo of an older step on that operation is skipped as changed since (ADR 0014 §1).
- A collaborator's library never holds the maker's material ids (random UUIDs), so their computer never refreshes. A refresh and a typed number made concurrently on two replicas keep one whole value; the other is visible in the History list, never mixed.
- The "stored: 60 %" / "Changed since saved" state goes: the stored copy is a fallback the session keeps current, not a state the maker manages.
- Published file keys: `op_settings` and its sub-keys (`overrides`, `material.modified` included), `op_grid`, `x.curvyo.laser.test_grid`, `test_grid_worked`, `grid_cell`, `grid_label`, the root `x.curvyo.laser.material_edit`, the label `refresh_material_numbers`. Renaming one is a format change.
- A rebuild is up to 144 cells and 25 labels in one commit; budgets of criterion 30 in the feature notes.
- Rejected and why: per-number registers (mixed sets), link only (cannot run elsewhere), overrides in their own register (an override over a material it was not typed for), refresh without guard (ping-pong), a new grid node shape (a second walk), a stored "edited by hand" flag (wrong after undo and merge).
