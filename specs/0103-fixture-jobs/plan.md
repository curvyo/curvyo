# Plan for Fixture jobs (0103), Part A

Branch: `story/0103-fixture-jobs`. Scope of this run: Part A only (the core crates and the
labels; no editor-wasm session work, which is Part B). Fragen 176 to 178 at their defaults:
176 A (the fixture block runs before the work), 177 B (the maker ticks each part, nothing ticked
by default), 178 B (Output turns off after an output; that is `0029`, so 0103 builds only the
job's Output switch, criterion 6). Formats: `format_version` 41 (ADR 0052 §6), profile record
format 3 (ADR 0052 §7).

## Affected crates/modules

- `curvyo-laser-core`:
  - new `fixture_part.rs`: the parts of a fixture kind in Parts-list order, the part of a stored
    key (fixture or slot plus suffix) and of a profile token, and the token of a part.
  - `operation_sequence.rs`: `FixtureJobRead` and `fixture_block()` (the run rule and order).
  - `machine_profile_fixtures.rs`: the `jobs` of each fixture entry, the limits 100 jobs and
    200 targets per set; `machine_profile_file.rs`: record format 3, `MAX_RECORD_BYTES` 128 KiB.
  - goldens in `tests/fixtures/machine_profiles/`.
- `curvyo-document-core`:
  - new `fixture_job_codec.rs` (the job node's keys and its read), new `fixture_job.rs` (the ten
    commands and their labels).
  - `manufacturing_fixtures.rs`: `FixtureSnapshot.jobs`, `FixtureRecord.jobs`, `read_fixture`
    skips job children; `manufacturing_fixture_commands.rs`: the slot sweep in `remove_slot`,
    the jobs of a record written by `insert_fixture` (pick and reset).
  - `manufacturing_codec.rs`: a `fixture_job` child is valid; `document.rs`: format 41.
  - goldens in `tests/fixtures/`.
- `curvyo-ui-core`: `history_names.rs`, `step_scope.rs` (the ten labels, FREE).

## Tasks

- [x] 1. laser-core `fixture_part.rs`: parts per kind in list order (grid lines, tapes; holder
  board then slots; clamp; stop; screw head, thread), part of a key (kind, slot or not, suffix),
  tokens `board`, `clamp`, `stop`, `grid.lines`, `grid.tapes`, `screw.head`, `screw.thread`,
  `slot:<n>` both ways; a suffix that is not a part of the kind is damage (fulfils AC 8, 11, 22,
  23). Tests first.
- [x] 2. laser-core `fixture_block()` in `operation_sequence.rs`: a job runs when its fixture is
  Active, Output is on, its kind is known and it has a ticked readable part; with Cuts last on,
  Marking before Cutting, else list order; the count of the summary is the number of jobs in the
  block (fulfils AC 13, 14, 15, 16, 17 count). Tests first.
- [x] 3. laser-core profile record format 3: `jobs` per entry (kind, name, show, targets), no
  `output` key, format 2 reads without `jobs` and a `jobs` key there damages, format 3 requires
  it; a bad token, a slot index out of range or a duplicate damages; 100 jobs, 200 targets;
  record text limit 128 KiB; writes always format 3; unknown-kind jobs and targets that are not a
  part of the kind are left out of a write. Goldens `format3_fixture_jobs.json`,
  `format3_max_set.json`, `format3_damaged_token.json`, `format3_output_key.json`,
  `format2_jobs_key.json`; `newer_format.json` moves to 4 (fulfils AC 23, 24). Tests first.
- [x] 4. document-core job node and read: `shape = fixture_job` under a fixture, `name`,
  `op_kind`, `op_show` (absent on), `op_output` (absent off), no margin; target keys
  `target.<node id>[.lines|.tapes|.head|.thread]`; a key of a node that is not the fixture or a
  readable slot is left out and is not damage; a value that is not `true` is damage; a kind that
  is missing or not a text is Unknown and damage; `read_fixture` skips job children and adds the
  jobs' damage (fulfils AC 5, 12, 21, 22).
- [x] 5. document-core commands (one commit each): add (Show on, Output off, no tick), rename,
  delete, move inside its fixture, Show on/off, Output on/off, assign and remove targets; a work
  operation, slot or fixture id is refused; `remove_slot` drops the job keys of the slot in the
  same commit (fulfils AC 2, 4, 5, 7, 9, 10, 11).
- [x] 6. document-core copies: `FixtureRecord.jobs` (kind, name, show, targets by place, sorted);
  pick and reset write each job with Output off, fresh ids and the targets mapped to the new
  slot nodes by index (fulfils AC 23).
- [x] 7. document-core format 41: `child_is_valid` accepts a `fixture_job` child without
  children; an unknown kind is kept and saved back unchanged; goldens `fixture_jobs_v41`,
  `fixture_jobs_unknown_kind_v41`, `fixture_jobs_damaged_v41`; older goldens still open; a format
  42 file is refused; superseded format 40 pins updated (fulfils AC 20, 21, 22).
- [x] 8. ui-core: the ten labels with their fixed undo names in `history_names.rs` and in
  `FREE_STEPS` of `step_scope.rs` (fulfils AC 4, 9; adrs.md §7).
- [ ] 9. Part B (not this run): editor-wasm session module `session/fixtures/jobs/`, the
  Operations view contract, the summary line, the budgets, the frontend.

## Validation

- Unit tests per module (tests first for tasks 1 to 3), golden files for the project file and
  the profile record (read, write back equal, damage cases), the existing goldens of format 35 to
  40 still opening unchanged.
- The step-scope guard test covers the new labels (each in exactly one list, each named).
- Gate for this run: fmt, workspace clippy, nextest for laser-core, document-core, ui-core and
  the editor-wasm lib plus its integration binaries that pin the format version, wasm32 build and
  clippy of every core crate and editor-wasm.
