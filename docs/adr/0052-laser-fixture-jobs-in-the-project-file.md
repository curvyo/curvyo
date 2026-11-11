# ADR 0052: Laser fixture jobs in the project file and the profile record: job nodes under their fixture, part keys, format 41, record format 3

**Status:** Accepted 2026-11-12 (customer accepted PR #330; document model and file format). Extends [ADR 0049](0049-laser-fixtures-in-the-project-file.md) (a fixture may hold job nodes beside its slots) and [ADR 0050](0050-laser-fixture-set-in-machine-profile.md) (record format 3, a `jobs` key per fixture entry). Follows the child-node precedent of [ADR 0046](0046-laser-operations-record.md) §1 and [ADR 0049](0049-laser-fixtures-in-the-project-file.md) §3. Spec: [`0103`](../../specs/0103-fixture-jobs/specification.md) criteria 3 to 5, 8 to 12, 20 to 24. Feature notes: [`0103` adrs.md](../../specs/0103-fixture-jobs/adrs.md). On acceptance, amendment notes go on ADR 0049 §2, §3 and §7, and on ADR 0050 §1, §2 and §8 (the record text limit, the format 2 reading and the goldens).

## Context

A fixture job is an operation of one fixture (`0103`): a kind, a name, Show and Output, and the fixture parts it takes. It lives with its fixture, in the fixture's order, and is removed with it. Its ticks name fixture parts, which are the fixture's own node (a holder's board, a clamp, a stop), its slot nodes, or, for a grid and a screw, one of two named sub-parts of that node. The profile's fixture set (ADR 0050) must carry the jobs too, by place and not by id.

## Options

**Where a job lives**

- **A. A child node of its fixture node, `shape = "fixture_job"` (chosen).** Tree delete removes the jobs with their fixture in the same step, undo revives them (ADR 0014), the order among a fixture's jobs is the child order as for slots (ADR 0049 §3), and no sweep is needed on fixture delete. Cost: one new child shape under a fixture, so format 41.
- **B. A child of the manufacturing node with a `fixture` id register.** A fixture delete must sweep its jobs, and the order across fixtures needs a second order. Rejected.
- **C. Registers keyed by the fixture id on the manufacturing node.** Deletion, revive and "keep unknown" become bespoke code, the reason ADR 0046 rejected its option C. Rejected.

**Where the ticks live**

- **A. Keys on the job: `target.<part key>` = `true` (chosen).** A tick writes the job node only, as ADR 0046 option A does for operations. Deleting a job drops its ticks with it.
- **B. Keys on the part node, one per job.** A tick would write a shared slot node, and deleting a job would sweep every slot. Rejected.

**Which key prefix**

- **`target.` (chosen).** ADR 0046's `member.` keys are read, swept and matched by the nearest rule for work operations. Spec `0103` criterion 10 needs the two sets never to share a key or a row, so fixture parts get their own prefix. The sweep for a deleted slot is a separate, job-only scan (§4).
- **`member.` (rejected).** It would put fixture parts into the work reader by construction.

**Profile record**

- **Record format 3 with a `jobs` key per fixture entry (chosen).** One record per profile, as ADR 0038 and ADR 0050 chose.
- **A second file (rejected, ADR 0050 option B).**

## Decision

1. **Job node.** A child of a fixture node, `shape = "fixture_job"`, with no children. Registers:
   - `name` (existing, read by the registers of ADR 0033);
   - `op_kind` (the five texts of ADR 0046 §1: `cut`, `score`, `engrave`, `engrave_outline`, `cut_out`; opaque here, read by `curvyo-laser-core`);
   - `op_show` (boolean, absent = on);
   - `op_output` (boolean; **absent = off** for a job, unlike ADR 0046's operations; a new job always writes `false`).
   There is no `op_margin` register: a fixture job row has no settings field (customer, 2026-11-10), so a Cut-out job runs with margin 0.
   The order of a fixture's jobs is the child order among its job siblings. A reorder is one tree move, as ADR 0046 §1.
2. **Target keys.** A tick is one boolean register on the job, key `target.<part key>`, value `true`. The part key is the encoded id of a fixture node (`encode_node`, `peer:counter`), with an optional suffix:
   - a holder's board, a clamp's outline and a stop: the fixture node's id, no suffix (the board is the holder node);
   - a slot: the slot node's id, no suffix;
   - a grid's lines and tapes: the grid's id with `.lines` or `.tapes`;
   - a screw's head and thread: the screw's id with `.head` or `.thread`.
   Examples: `target.7:3`, `target.9:12`, `target.7:3.lines`. A key with any other value than `true` is damage (§3). The separator `.` never occurs in an encoded id (`peer` and `counter` are decimal numbers, a codec test asserts it).
   The Targets list and the run order follow the **fixture part order** (the board or the clamp, then the slots in creation order, the grid's lines then tapes, the screw's head then thread), not the key order.
3. **Reads** (`curvyo-document-core/src/fixture_job_codec.rs`, one snapshot per job, `FixtureSnapshot.jobs`):
   - A target key counts only when its node is the job's own fixture or a readable slot child of it. Any other key (a dead node, a slot that is skipped as damaged) is left out of the snapshot and is **not** damage (Frage 169 A, `0103` criterion 12). Reads never delete it.
   - A key whose suffix is not a part of the fixture's kind, or whose value is not `true`, is kept in the snapshot and counted as one damaged value, then ignored by the run rule (`curvyo-laser-core/src/fixture_part.rs`).
   - `op_kind` absent or not a string: Unknown and one damaged value. A text that is not one of the five: Unknown, not damage (`0103` criterion 21, R-MFG-010). An Unknown job runs nothing, is counted nowhere, and can only be renamed or deleted.
   - `op_show` not a boolean: reads on, one damaged value. `op_output` absent: off, no damage; not a boolean: off, one damaged value (`0103` criterion 22: never on).
   - A damaged value counts into `FixtureSnapshot.damaged` (`0102` criterion 33 gains the jobs' damage, `0103` criterion 22).
   Nothing is rewritten on read.
4. **Writes** (`curvyo-document-core/src/fixture_job.rs`, one commit each, labels in §8). The commands check the node's shape `fixture_job` and that its parent is a fixture node; a work operation id, a slot id or a fixture id passed to a job command is refused. Adding a job writes `op_output` = `false` and no target key. Ticking or unticking writes or removes one `target.` key on the job.
   **Slot removal.** `remove_slot` (ADR 0049 §3) also removes, in the same commit, every `target.<slot id>` key of the holder's job children, so one undo restores them. This is the only sweep the feature adds; it scans at most the holder's jobs (`0103` budgets). Moving, resizing or renaming a slot writes nothing here (criterion 11).
   **Fixture removal.** The tree delete of `delete_fixture` removes its jobs and their keys; no sweep.
5. **Validation (open-file check, ADR 0033 §7).** A fixture's children are `fixture_slot` nodes or `fixture_job` nodes, each with no children and readable node registers. Any other child is refused, as today. `read_fixture` skips job children; it counts only slot children that do not read. (Without this change every holder with a job would report a damaged slot.)
6. **`format_version` 41.** A `fixture_job` child is a new node shape under a fixture. Build 40 refuses a fixture with a non-slot child (`child_is_valid`, `CURRENT_FORMAT_VERSION` doc of format 40), so a build of 40 would refuse a file with jobs as damage. The bump makes it a version refusal instead, as ADR 0033 §7 requires (the same rule as 37, 38, 39, 40). **Migration: none.** A format 40 file has no job node; it opens unchanged and nothing is written on open. Goldens in `curvyo-document-core/tests/fixtures/`:
   - `fixture_jobs_v41.curvyo`: the set of `0103` criterion 20 (a holder with an Engrave + Outline job on its board and two slots, Output on, and a Cut job on a grid, Output off; a grid with no job; an inactive holder with one job; one target key whose node is dead);
   - `fixture_jobs_unknown_kind_v41.curvyo` (criterion 21);
   - `fixture_jobs_damaged_v41.curvyo` (criterion 22: `op_show` as text, `op_output` as text, a target key with value `false`, a target suffix `.tapes` on a holder).
   The older goldens stay and must open unchanged, including `fixtures_v40.curvyo` and `operations_v39.curvyo`.
7. **Profile record format 3** (ADR 0050 amendment). `format_version` 3. Each fixture entry gains `"jobs": [ ... ]` in creation order, always written (possibly `[]`). A job entry:
   ```json
   { "kind": "engrave_outline", "name": "Engrave + Outline 1", "show": true,
     "targets": ["board", "slot:0", "slot:1"] }
   ```
   - `kind` is one of the five texts of §1 (no prefix). `show` is a boolean. No `output` key: the profile never holds Output, so an `output` key is an unknown key and damages the record (`deny_unknown_fields`).
   - `targets` are part tokens: `board` (a holder), `clamp`, `stop`, `grid.lines`, `grid.tapes`, `screw.head`, `screw.thread`, and `slot:<n>` with `n` the 0-based index among the entry's `slots`. A token that is not a part of the entry's kind, a slot index out of range, or a duplicate makes the record damaged (ADR 0050 §2: a silently changed bed is refused, not defaulted). So a copy never has a target without a counterpart.
   - **Reading.** Format 1: no fixtures (ADR 0050). Format 2: fixtures without `jobs`; a `jobs` key in a format 2 entry is an unknown key and damages the record. Format 3: `jobs` required in each entry. A higher version is skipped with its notice (ADR 0038 §3). Reads never write.
   - **Writing.** Always format 3. A write of a format 1 or 2 record upgrades it. Unknown-kind jobs are left out of the written set, as unknown fixtures already are (`write_set`).
   - **Copy.** Pick profile and Reset from profile (ADR 0050 §4) write each job on the new fixture node with a fresh id, `show` from the record, `op_output` = `false`, and one `target.` key per token, mapped to the new fixture's slot nodes by index. Same labels, same undo step as the fixtures.
   - **Differs from profile** (ADR 0050 §5) compares the jobs in order: kind, name, Show and the set of target tokens. Output is ignored, so a maker switching an Output does not see "Differs".
   - **Limits.** A set holds at most 100 jobs and 200 targets (`MAX_SET_JOBS`, `MAX_SET_TARGETS` in `machine_profile_fixtures.rs`). Names at most 40 characters (ADR 0050 §1). The record text limit rises from 64 KiB to **128 KiB** (`MAX_RECORD_BYTES`), because a set of 32 fixtures, 200 slots, 100 jobs and 200 targets in the pretty form is above 64 KiB. A golden at the maximum must read and fit.
   - **Goldens** in `curvyo-laser-core/tests/fixtures/machine_profiles/`: `format3_fixture_jobs.json` (reads, writes back equal); `format3_max_set.json` (the maximum above, reads, fits 128 KiB); `format3_damaged_token.json` (`grid.tapes` on a holder: damaged); `format3_output_key.json` (an `output` key: damaged); `format2_jobs_key.json` (format 2 with `jobs`: damaged). `format2_fixtures.json` still reads; a write of it is format 3 with `"jobs": []`, so ADR 0050 §8's "byte for byte" becomes "reads, and any write is format 3". `newer_format.json` moves to format 4.
8. **Labels** (document-core constants, `fixture_job.rs`; named in `curvyo-ui-core/src/history_names.rs`, listed in `FREE_STEPS`, see `0103` `adrs.md` §7): `add_fixture_job`, `rename_fixture_job`, `delete_fixture_job`, `set_fixture_job_show_on`, `set_fixture_job_show_off`, `set_fixture_job_output_on`, `set_fixture_job_output_off`, `move_fixture_job`, `assign_fixture_target`, `remove_fixture_target`. The label names are fixed per label (`operation_name`), so the undo step names carry no job name.

## Consequences

- Every walk of a fixture's children filters by shape. Today two raw walks exist (`read_fixture` and `validate`); a new raw walk must do the same. The skip predicate `in_manufacturing_subtree` reaches a job at depth 2 under the manufacturing, so drawing walks already skip jobs; its doc comment is updated to name jobs.
- Format 41 means a build of 40 refuses a file with a job. Accepted: old-file compatibility is waived for this development build (customer, 2026-10-29).
- The profile record is larger and still one whole-record write: two computers editing one profile's jobs at once produce a conflict copy, as for its fixtures (ADR 0038).
- Keys are published file keys: `target.`, the part key form with `.lines`, `.tapes`, `.head`, `.thread`, and the five kind texts. Renaming one is a format change.
- Rejected and why: the job under the manufacturing (B above), registers keyed by the fixture (C), keys on the parts (B), the `member.` prefix, a second profile file, and an `output` key in the profile (the copy always starts Off, so the key would only be a way to store "on").
