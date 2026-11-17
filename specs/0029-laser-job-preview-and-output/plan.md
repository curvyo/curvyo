# Plan for Laser job preview and output (0029), Parts A to E

Branch `story/laser-job-preview-and-output`, on `main` after `0113` (PR #365, format 47, profile
record format 5). Built on `specification.md` (criteria 1 to 39), `adrs.md`, ADR 0064 (job builder,
gate, output writers), ADR 0065 (page registers, `air_assist`, `output_format`, **project
`format_version` 48**, **profile record format 6**) and `ux-notes.md` (sections 1 to 16, P1 to
P23). Numbers checked free on `main` (2026-11-16: `CURRENT_FORMAT_VERSION` 47,
`machine_profile_file::RECORD_FORMAT_VERSION` 5); re-check before the PR. One PR for the slice
(the customer can try it only after Part D); Parts A to D are commits on this branch, each ending
with the gate of its crates; Part E runs the full gate of `CLAUDE.md` §7.

| Part | What | Crates | Model |
|---|---|---|---|
| A | job builder, order, hatch, moves, timeline, gate, GRBL writer, output registers read | `curvyo-laser-core`, `curvyo-geometry-core` | Opus |
| B | page registers and their commands, profile record 6, page copy keys, format 48, Lasered hook, export effects | `curvyo-document-core`, `curvyo-laser-core`, `curvyo-ui-core` | Opus |
| C | session: job cache, Output tab view JSON, toolpath draw data, play slider, export commands | `curvyo-editor-wasm`, `curvyo-render-core` | Opus |
| D | frontend: Output tab, Route toggle, play bar, Machine block rows, export flow | `frontend/` | Sonnet |
| E | full gate, docs, spec status, PR | all | Sonnet |

Where the documents differ, this plan follows (all **(Proposal)**, for the PO and architect to
align before the PR):
- Criterion 37's test reads as `adrs.md` and UX M2 say (the slider maps time).
- Criterion 10's "summary" holds only the total (UX M1); each run's time is in its gate row.
- The Head position warn is the 53-character text of UX P22.
- The fixture jobs of one kind form one block with one run number (UX P3); the session groups
  them, the core receives one block per kind in run order.
- Lengths in the summary: run-out moves count as travel, Z moves count in no length (ADR 0064
  feature note "Run-out and Z lines": both only in the time, at the burn feed).
- Laser-off `G1` lines (run-out, Z) carry `S0` where S changes, in M3 and M4, so a Z move in M4
  never burns.

## Affected crates/modules

- `curvyo-geometry-core`: `flatten.rs` exported as `flatten_outline(anchors, closed, Tolerance)`
  (bit-identical to the existing `flatten`); new `direction.rs` "the unit vector of an angle in
  degrees, by `libm`" (`direction_of_degrees`). Architect review at build.
- `curvyo-laser-core` (pure, wasm32, `#![forbid(unsafe_code)]`), new `pub mod job` with
  `job_input.rs`, `job_numbers.rs`, `job_order.rs`, `job_hatch.rs`, `job_plan.rs` (+ `job_plan_lines.rs`
  if it passes 500 lines), `job_moves.rs`, `job_timeline.rs`, `job_gate.rs`, `job_gate_text.rs`,
  `job_fingerprint.rs`, `job_write.rs`, `job_write_grbl.rs`, `output_settings.rs`, plus
  `job_build.rs` ("builds the job of one page from its input") and `job_gate_rows.rs` ("the run
  rows of the gate") to keep each module under 500 lines;
  `machine_profile.rs` gains `OutputFormat` and `ProfileValues::{air_assist, output_format}`
  (Part A; their record and copy keys are Part B). Part B: `machine_profile_file.rs` (format 6),
  `machine_profile_copy.rs` (`air_assist`, `output_format`, damage), new `job_effects.rs`.
- `curvyo-document-core` (Part B): `document.rs` (`CURRENT_FORMAT_VERSION` 48), the three page
  register commands beside `set_cuts_last` in `operation.rs` or a new `laser_output_commands.rs`,
  the `export_job` step and the reserved non-step `mark_test_grid_lasered` commit.
- `curvyo-ui-core` (Part B): `history_names.rs` ("Export job file", "Pick fixture material",
  "Start from machine origin" / "head position", "Home first on/off", "Air assist on/off"; drops
  "Mark lasered").
- `curvyo-editor-wasm` (Part C): new `session/output/` (job cache, view JSON, export), toolpath
  draw cache; `session/manufacturing/machine.rs` (Air assist step, Output line).
- `curvyo-render-core` (Part C): new `toolpath_look.rs` "draws a cached toolpath up to a head
  position".
- `frontend/` (Part D): `OutputTab.tsx`, `PlayBar.tsx`, `PageFrames.tsx` (Route toggle),
  `PanelStrip.tsx`, `MachineBlock.tsx`, export flow beside the SVG export.

## Tasks

### Part A: the job in `curvyo-laser-core` (Opus)
- [x] A1. `curvyo-geometry-core`: export `flatten_outline`, add `direction_of_degrees`; unit tests
  (bit-identical to `flatten`, 0/90/180 exact axes) (AC 14, 17, 34).
- [x] A2. `OutputFormat` (`Grbl`, `"grbl"`, "GRBL", `ALL`) and `ProfileValues::{air_assist,
  output_format}` with defaults (false, `Grbl`); every construction updated (AC 25, 39).
- [x] A3. `output_settings.rs`: read `fixture_material`, `start_from`, `home_first` from the
  process registers, damage counted, the Home first rule as one function (AC 38; ADR 0065 §1, §2).
- [x] A4. `job_input.rs`: the session's input types (blocks in run order, leaves in page mm with
  node id and stacking order, images as frames, Cut-out items with margin) (AC 13, 20, 34).
- [x] A5. `job_numbers.rs`: effective numbers per block by `resolve(..).run()`, fixture blocks by
  the page's pick and the material's set of the kind for the profile id; feed clamp, over-feed
  flag; Unknown, damaged settings and damaged options run nothing (AC 2, 3, 4 a, 21, 33).
- [x] A6. `job_order.rs`: inner first (closed by `outline_nesting`, open by all nodes inside), then
  nearest start (closed any node, open either end, hatch units four entries), ties by stacking,
  outline, node; tested against a brute-force reference on random fixtures (AC 22).
- [x] A7. `job_hatch.rs`: scan lines Interval apart at the Scan angle, first half an Interval
  inside, nonzero spans, bidirectional, Crosshatch (AC 14, 15).
- [x] A8. `job_plan.rs`: per block the ordered burn paths by kind (Cut, Score, Engrave, Engrave +
  Outline with hatch then outline, Cut-out by `cut_out_contours`; Engrave images dropped, Cut and
  Score images as frames), flattened at 0.0095 mm and rounded to µm; `PlanCache` keyed by
  `job_fingerprint.rs` (FNV-1a 64) and the entry point (AC 13 to 18, 22, 36 d).
- [x] A9. `job_moves.rs`: `Job` and its `Move`s (burn, travel, run-out, Z), passes repeated whole
  (Frage 223 A), Z offset only with `motorised_z`, overscan run-out at 1,000 mm/s² clamped to the
  bed in Machine origin mode, start and end by Start from, `machine_point` for the four corners
  (AC 15, 19, 20, 21, 23, 25, 26, 38).
- [x] A10. `job_timeline.rs`: burn and travel lengths, time estimate, the texts of criterion 10, the
  head at a fraction of the time (binary search, move split) (AC 9, 10, 37).
- [x] A11. `job_gate.rs` + `job_gate_text.rs`: machine and mode lines, fixture and operation lines
  with source words and run time, the ordered `Cause` list with counts and targets, bed check at
  10 µm, `export_allowed`, `first_block` (AC 2, 4 to 7, 21, 33, 38, 39).
- [x] A12. `job_write.rs` + `job_write_grbl.rs`: `write(format, &job)`, name, extension, file type,
  mode line; GRBL header, `$H`, G21/G90/G91/G17, M3/M4, M8/M9, F and S modal, coordinates from µm,
  ASCII names (AC 24, 25, 26, 38, 39).
- [x] A13. Goldens `curvyo-laser-core/tests/fixtures/output/grbl/*.gcode` (the 20 files of
  criterion 31) built from `job_input` fixtures in `tests/job_output_goldens.rs`;
  `CURVYO_UPDATE_GOLDEN=1` rewrites them; determinism (two builds byte-identical), flatten
  deviation ≤ 0.01 mm, the view/file length equality of criterion 9 by a small test-only reader
  (AC 9, 17, 24, 31, 39).
- [x] A14. Budgets 36 (a) to (c) and the gate part of (d) in
  `curvyo-editor-wasm/tests/job_builder_budget.rs` (thread CPU time, asserted in release) (AC 36).

### Part B: stored values and the export step (Opus)
- [x] B1. Page register commands `set_fixture_material`, `set_start_from`, `set_home_first_on/off`
  (steps, labels of ADR 0065 §1) (AC 2, 38).
- [x] B2. Profile record format 6 (`air_assist`, `output_format`), format 5 read as defaults,
  damage rules, goldens of ADR 0065 §6 (AC 25, 32, 39).
- [x] B3. Page copy `mfg_profile.air_assist` / `.output_format`, damaged reads, Differs (AC 4 c, 39).
- [x] B4. `CURRENT_FORMAT_VERSION` 48, goldens `laser_output_v48.curvyo`,
  `laser_output_damaged_v48.curvyo`, format-47 goldens open unchanged (AC 32).
- [x] B5. `job_effects.rs` and the `export_job` step (fixture jobs off, `refresh(.., guard:
  false)`), the reserved non-step `mark_test_grid_lasered` commit, test records' `lasered`;
  `history_names.rs` (AC 28, 29, 30).
- Part B notes (Proposal): labels follow UX notes §12 ("Pick fixture material", "Start from
  machine origin" / "head position"), so Start from has two labels
  (`set_start_from_machine_origin`, `_head_position`) instead of ADR 0065 §1's one
  `set_start_from`; Air assist is `ProfileChange::AirAssist` (`air_assist_on` / `_off`).
  `BlockSource::Fixture` carries its job ids and `BlockSource::Operation` its test grid, so
  `export_effects` reads both from the exported job. An export that changes nothing writes no
  step.

### Part C: the session (Opus)
- [x] C1. Job input from the page (leaves in page space, run order, fixture blocks per kind,
  summary facts for the gate), built lazily per frame, `PlanCache` reused (AC 3, 12, 36).
- [x] C2. Output tab view JSON (machine lines, Start, runs, checks with targets, total, Export
  state and tooltip) and the commands of B1 (AC 1 to 7, 10, 38).
- [x] C3. Toolpath view toggle state per page, draw cache and `toolpath_look.rs` (kind looks,
  thinning, rings, arrowheads, travel, pale after the head) (AC 8, 9, 11, 18, 37).
- [x] C4. Play slider position (fraction of time), readout, step by 1 % (AC 37).
- [x] C5. `export_job` (bytes, name, file type, token) and `export_job_done(token, now_ms)`;
  Air assist step and Output line in the Machine block (AC 25, 27 to 30, 39).
- [x] C6. Budgets 36 (d) to (f) and 12 in `curvyo-editor-wasm/tests/cpu_time/` (AC 12, 36).

- Part C notes (Proposal): `session/output/` holds one state field (`OutputState`) and the
  submodules `input.rs` (job input; the leaf index gains a `LeafSource` per leaf),
  `leaf_geometry.rs`, `job_cache.rs` (keyed by document version, drawing read, library and
  extension switches; one `PlanCache` keeps every operation's plan), `view_json.rs`,
  `toolpath.rs`, `play.rs`, `export.rs`, `commands.rs`; the running fixture jobs come from
  `fixtures/jobs/runs.rs`. The toolpath look is built once per job (the plans are cached per
  operation), the draw list per view and head position; no frame skip. Ring numbers are DOM
  labels (`toolpath_ring_labels_json`, canvas CSS px); render-core draws the ring, tick,
  arrowheads and head. A check line carries `before`, `button`, `after` so the frontend makes the
  name or count a button. `export_job(project_file)` hands out bytes, name, file type and a
  token; `export_job_done(token, name, downloaded, now_ms)` writes the step and Lasered and
  returns `"<ms>:<notice>"`. Fixture part circles (pins, screws, centre marks, round slots) enter the job as
  exact cubic arcs, flattened like every curve (criterion 17). Budgets in `tests/output_budget.rs` (release:
  36 d 5.5 ms, 36 e 5.2 ms, 36 f 5.0 ms, 12 3.4 ms); contract fixtures in
  `tests/fixtures/output_view/` with the decoder `frontend/src/lib/outputView.ts`.

### Part D: frontend (Sonnet)
- [x] D1. Output tab after Fixtures (`FileOutput`), sections of UX 6 (AC 1 to 7, 10, 38).
- [x] D2. Route toggle beside ListTree, exclusive (UX 3) (AC 8).
- [x] D3. Play bar with Play/Pause, slider, readout, keys (UX 5) (AC 37).
- [x] D4. Export flow (save dialog or download, notices, error dialog) (AC 27, 28).
- [x] D5. Machine block: Air assist switch row, read-only "Output: GRBL" (AC 25, 39).
- [x] D6. Vitest for the tab's view mapping and the bar's keys.

- Part D notes (Proposal): Part C had no `PanelTab::Output` and no `toolpathView` in the page rows,
  so Part D adds both (`curvyo-ui-core` panel tab and content, `pages_json`, which also reads
  `operationsView` unpressed while Route is pressed), plus `select_output_items` and
  `reveal_operation_row` for the check buttons. The desktop save is `curvyo-app/src/job_export.rs`
  (`pick_job_export_path` with the file type and extension, `write_job_export` with a raw body,
  sharing `AppState`'s export path with Export SVG…); the browser downloads. One hook per feature:
  `useOutput` (tab, commands, Route; composes `useJobExport`) and `usePlayBar`; `RingLabels`,
  `PlayBar` and the Route toggle sit in `PageFrames`. The slider is the `0028` look with
  `useSliderPointer` and no number entry. The browser script is
  `frontend/tests/browser/laser-output.cjs`.

### Debt: long functions kept (review 2026-11-17)

`build_job`, `operation_looked_artwork` and `build_leaves` were split under 60 lines. Three
functions stay longer, on purpose (Proposal):
- `Cause::text` (`curvyo-laser-core/src/job_gate_text.rs`, about 86 lines): one `match` arm per
  gate cause, one sentence each; the arms are the table of criteria 4, 5, 33 and 38, and splitting
  it would scatter that table over helpers that each hold one string.
- `fixture_blocks` (`curvyo-editor-wasm/src/session/output/input.rs`, about 61 lines): one pass
  over the fixture runs that maps each part to page mm and builds its block; the steps share the
  run's material, numbers and map, so a split passes all of them along.
- `read_copy` (`curvyo-laser-core/src/machine_profile_copy.rs`, about 61 lines): the ordered read
  of the stored copy's fields with the damaged flag (ADR 0039 §5, ADR 0065 §3); each field is a
  line, and it grew by `air_assist` and `output_format` only.

### Rework after the customer's try (2026-11-17)

The customer tried PR #371; `ux-notes.md` (changes table, §15 M9 to M16, P24 to P32) and ADR 0064/0065
(amended 2026-11-17) set the rework. It supersedes, in "Where the documents differ" above, the
Head position warn bullet (removed) and the fixture run-number bullet (rings are row digits, P26),
and in Part D the Output tab (D1). Same branch, commits per R task, each ending with the gate of its
crates; then Part E. Format 48 and profile record format 6 stay; no migration, no dependency.

- [x] R1. `curvyo-laser-core` (Opus): `Start` becomes one enum `MachineOrigin | HeadPosition |
  HomeThenOrigin` in `output_settings.rs` (register `start`, absent = Machine origin, a damaged
  value reads Machine origin and counts damaged; `StartFrom`, the Home first rule and its tests
  go); `job_moves.rs` and `job_write_grbl.rs`: `HomeThenOrigin` = `$H` after the header, then the
  Machine origin file; goldens keep their bytes (`home_first.gcode`, `start_head_position.gcode`,
  fixtures in `tests/job_fixtures/` spell the one value). Gate: remove `Cause::HeadPosition`; bed
  check in Head position from the job's own box (work area anchored at the box's origin corner);
  add the block `StartDamaged` with `Targets::StartPick` after `MachineDamaged`; `Block` carries
  `Ring::Operation(digit) | Ring::Fixture` from `job_input`; `Gate { machine, runs, issues }` text-free
  (causes carry ids, not names), `RunFacts` per run (ring, time, numbers, no settings);
  `job_gate_text.rs` renders `CheckText { before, button, after }` with a names lookup, the numbers
  line and the machine line "<profile> · <format>[ · air assist]"; `ModeLine` and
  `job_write::mode_line` go; `numbers_lines` public for rows that do not run (AC 2 to 7, 24, 26,
  31, 38, 39).
- R1 notes (Proposal): `Cause` and `Severity` moved to `job_gate_cause.rs` ("the causes of
  the gate's checks") so `job_gate.rs` stays under 500 lines. Causes carry the operation's id;
  material names stay as the settings rule resolves them (the live record's, or the one stored
  with the link; a missing material has no library record to look up). `Names` holds the
  operation names the session passes. The GRBL header numbers its blocks by run order (index + 1),
  so every golden keeps its bytes. `readout_text` is time only. `check_text` (about 95 lines) is
  one `match` arm per cause, the table of section 10, as `Cause::text` was.
- [x] R2. `curvyo-document-core` + `curvyo-ui-core` (Opus): one register `x.curvyo.laser.start`
  with `machine_origin` (default, absent), `head_position`, `home_then_origin`; one command
  `set_start(value)` with labels `set_start_machine_origin`, `_head_position`, `_home_then_origin`
  ("Start: Machine origin" / "Start: Head position" / "Start: Home, then machine origin"); a pick
  writes whenever the stored value differs, a damaged value included; remove `set_start_from`,
  `set_home_first`, their labels in `laser_output_commands.rs`, `history/rows.rs`,
  `step_scope.rs`, `history_names.rs`; rewrite `laser_output_v48.curvyo` and
  `laser_output_damaged_v48.curvyo` (ADR 0065 §6); a PR #371 file with `start_from` /
  `home_first` opens as Machine origin and saves them back untouched (test). Remove
  `PanelTab::Output` and `PanelContent::Output` (AC 1, 32, 38).
- R2 notes (Proposal): `Document::set_start(manufacturing, spelling)` takes the stored spelling
  (`Start::as_str`), refuses another with the new `OperationError::BadStart`, and picks the label
  from the spelling. The PR #371 registers are kept as unknown process registers (test
  `a_file_of_the_first_build_keeps_its_two_registers_untouched`).
- [x] R3. `curvyo-editor-wasm` (Sonnet): `session/output/` answers `row_runs()`,
  `fixture_runs()` and `output_group_json` (machine line, Start with `start_damaged`, checks with
  `before`/`button`/`after` and targets incl. `start_pick`, total with "Runs N operations, M
  items.", Export); remove `OutputView`, `output_view_json`, runs heading, mode line, material and
  source word of runs, `HOME_FIRST_TOOLTIP`, `set_start_from`, `set_home_first` (wasm and
  session); add `set_start(value)`. Operations view JSON: per row the run ring digit and time
  (rows that run), line 3 numbers with typed fields on every row (via `numbers_lines`), the
  accessible name "Cut 1, run 3, 3 min 21 s"; drop the `summary` counts (they are checks).
  Fixture jobs section view: the pick, one run line per kind, header time. Job built while the
  Operations tab of a laser page shows or Route is pressed. Ring labels JSON: digit or fixture.
  Play readout time only. Contract fixtures in `tests/fixtures/output_view/` and
  `frontend/src/lib/outputView.ts` follow; budgets 36 d, e (Operations tab), f, 12 re-run (AC 1
  to 7, 10, 12, 36, 37, 38).
- R3 notes (Proposal): bindings for R4: `output_group_json()` (machine, start {value, damaged,
  options}, checks with before/button/after and target kinds none/items/operations/fixture_pick/
  start_pick, total {time, runs, lengths}, export, toolpath_view) replaces `output_view_json`;
  `set_start(value)` replaces `set_start_from` / `set_home_first`; `toolpath_ring_labels_json`
  gives `[{digit, x, y}]`, `digit` `null` for the fixture glyph; `operations_view_json` rows
  gain `run` ({digit, time} or `null`), `numbers` (lines with `text`, `typed`; `null` for
  Unknown or damaged, empty for No settings) and `accessible_name`, and the view drops
  `summary` (its `items` lists stay); `fixture_jobs_view_json` gains `output` ({pick, runs:
  [{kind, time, numbers}], time} or `null`); the play readout is time only. Contract fixtures:
  `tests/fixtures/output_view/` (states empty, cut_and_engrave, blocked, start_head_position,
  start_home_then_origin, toolpath_half, each with output, rows, ring_labels, play),
  `operations_view/`, `fixture_jobs_view/` regenerated. The rows' line 3 is
  `job_numbers::block_numbers` + `numbers_lines` on the row's block without leaves.
  `session/output/fixture_pick.rs` holds the Fixture jobs section's output part.
- [x] R4. `frontend/` (Sonnet): delete `OutputTab.tsx` and the `output` strip entry; Operations
  tab output group after the Fixture jobs section (machine line, Start pick, checks, total,
  Export; P24, P28 to P30); operation rows 56 px (64 coarse; 70/78 Engrave + Outline) with run
  ring, time and line 3 (P25 to P27), windowing and reorder ghost at the new height; Fixture jobs
  section pick and run lines, header time (P32); check buttons: items select, an operation
  selects, unfolds and scrolls its row, Fixture jobs expands and focuses the pick, Start opens the
  Start pick (P15); Start pick `MaterialPick` look with three options, no greying (P29); play
  readout time only; `MachineBlock.tsx`: the state row keeps its 28 px place, switches at the
  row's right edge (P31), browser check pressing twice at one point; Vitest for the group mapping
  and the Start pick (AC 1 to 7, 10, 37 to 39).
- R4 notes (Proposal): `OutputGroup.tsx` (machine line, Start pick, checks, total, Export) is the
  `outputGroup` slot of `OperationsSection`, after the Fixture jobs section; `OutputPick.tsx` is the one
  inline pick of both the Start pick and the fixture pick (`FixtureOutputPart.tsx`, top of the section
  body); `numbersLine.ts` decodes numbers lines for rows and fixture runs; the view-level `items` and
  `summary` are no longer decoded (the Rust JSON still carries `items`). The accessible name of a running
  row keeps ", Group N in part" after the session's name. Rows use 56 px (64 coarse via `matchMedia`),
  Engrave + Outline 70 (78). The `0103` summary helpers (`summaryWithFixtures`, `runningLine`) and the
  tests of the old Output tab and summary were removed or rewritten, as the amended spec replaces them.
  The browser check `frontend/tests/browser/laser-output.cjs` presses Air assist twice at one point and
  covers the Start and fixture pick keys.
- [ ] R5. Tester (Haiku) re-runs the 0029 acceptance tests against the amended spec (criteria 4,
  10, 24, 31, 37, 38), then the full gate; UX review of R4; architect review of R1 to R3.

### Part E: finish (Sonnet)
- [x] E1. Full gate of `CLAUDE.md` §7; `specs/README.md` format row 48; ADR 0038/0039 notes;
  `docs/design-system.md` rows of UX 15; spec `Status: Done (PR #n)`.

## Validation

- Goldens of criterion 31 in `curvyo-laser-core/tests/fixtures/output/grbl/`, one `job_input`
  fixture per file, plus the tester's independent GRBL parser (extent, lengths, operation order).
- Determinism: every golden is built twice and compared byte for byte; no `HashMap` iteration,
  clock or path reaches a writer.
- Flatten tolerance: sampled cubics compared with their written polyline (≤ 0.01 mm).
- Order: brute-force reference against `job_order` on random rectangles and circles.
- Budgets of criterion 36 in thread CPU time, release builds (`performance-budgets` job).
- Part B goldens of ADR 0065 §6; Part C session tests for view JSON and the export step; manual
  browser check of the Output tab, the Toolpath view and the play bar after Part D.
