# Plan for Fixture sets and pins (0113), Parts A to F

Branch `story/fixture-sets-and-pins`, on `main` after `0111` (PR #350, format 44, profile record
format 4), `0114` (format 45) and `0112` (PR #353, format 46). Built on `specification.md`
(criteria 1 to 43), `adrs.md`, ADR 0059 (project file, **format 47**), ADR 0060 (profile **record
format 5**) and `ux-notes.md` (F1 to F16, M1 to M10). Numbers checked free on `origin/main`
(2026-11-15: `CURRENT_FORMAT_VERSION` 46, `RECORD_FORMAT_VERSION` 4); re-check before the PR. With
this plan the story is Ready. One PR for the slice (the customer can try it only after Part D);
Parts A to E are commits on this branch, each ending with the gate of its crates; Part F runs the
full gate of `CLAUDE.md` §7.

**No legacy conversion (lead, 2026-11-15, Proposal, vetoable; Curvyo is not deployed).** Nothing is
converted to "Set 1" and there is no migration module, no `migrate_format` commit and no migration
budget:
- A project file of format 46 or lower opens as it is and nothing is written: its flat fixtures are
  direct children of the manufacturing, so by ADR 0059 §2 they **are job fixtures** (unlocked); the
  page has no set; a screw keeps its `0103` jobs and gets no role jobs, so its Centre mark and
  Diameters toggles are hidden (hidden, not disabled). The existing fixture goldens
  (`fixtures_*_v40`, `fixture_jobs_*_v41`, `fixtures_grid_v39`) keep "nothing written on open".
- A profile record of format 1 to 4 is skipped as **older** (`RecordError::Older`), counted in the
  load notice "1 profile has an older format. It was left as it is." / "N profiles have an older
  format. They were left as they are.", left on disk, its id skipped (Save as profile takes a new
  id), as a format-1 material is skipped in `0112`.
- Amended with one dated line each: spec criterion 39 and "Format", `adrs.md` (decision, S1,
  budgets, crate placement), ADR 0059 (status, option C, §7, §9), ADR 0060 (status, §3, §4, §9),
  `ux-notes.md` section 12 "Old files".

Customer decisions applied: Frage 207 B (sets are named groups of the machine, a page switches on
any number of them, plus job fixtures of the page), 208 A (pin), 209 A (centre mark with a Size,
engravable diameters), 210 ("Apply fixtures from page" is a plain copy, never mirrored), 211 B (a
set fixture keeps its own "On the bed" check; on the bed = set on this page **and** own check),
P16 reduced (the screw keeps its Head and Thread knobs, no fields).

Where the documents differ, this plan follows (all **(Proposal)**, for the PO and architect to
align before the PR):
- Add labels (UX M1): `add_fixture` "Add fixture" (into a set) and `add_job_fixture` "Add job
  fixture" replace the per-kind `add_<kind>` labels of `0102` for new steps; the old keys stay in
  `history_names.rs` so old history rows read; `adrs.md`'s `add_pin` is not added.
- Line 2 of a job fixture appends the jobs that are on (UX F12, M2); the Jobs toggles live in the
  unfolded body only (M5); the dashed preview variant (M8) and the pin ground (M9) as UX F1, F2.
- The Select bar of a job fixture holds only Lock (M7), a design-system exception noted in G3.

## Affected crates/modules

- `curvyo-laser-core` (pure, wasm32): `fixture_kind.rs` (`Pin`, `curvyo.laser/pin`, diameter
  range), new `fixture_grid.rs` ("the centres of a pin or screw grid and its limits"), new
  `fixture_marks.rs` ("the lines of centre marks and diameters"), new `fixture_sets.rs` ("which
  fixtures of a page lie on the bed and in what order", plus `is_locked`), `fixture_part.rs`
  (`.circle`, `.marks`), `fixture_snap.rs` / `fixture_fit.rs` (pin is no target),
  `fixture_stored.rs` (grid params, role), `machine_profile_file.rs` (format 5, `Older`; split
  because it is 523 lines today), new `machine_profile_sets.rs`, `machine_profile_fixtures.rs`
  (pin entries, roles, tokens), `machine_profile_copy.rs` (sets only, Differs by set),
  `machine_profiles.rs` (older count and notice). Goldens `tests/fixtures/machine_profiles/`.
- `curvyo-document-core`: `document.rs` (`CURRENT_FORMAT_VERSION` 47), `manufacturing_codec.rs`
  (set node, `set_on_page`, `fixture_locked`), `manufacturing_fixtures.rs` (reads at depth 2 and 3),
  `manufacturing.rs` (validator), `fixture_job_codec.rs` / `fixture_job.rs` (`job_role`), new
  `manufacturing_fixture_set_commands.rs` (+ `_tests.rs`), `manufacturing_fixture_commands.rs`
  (add into a set or as a job fixture, lock, role-job refusals), new `fixture_apply.rs` ("copies the
  job fixtures of one page to another"), the profile copy path (Pick and Reset replace sets only).
  Goldens `tests/fixtures/`.
- `curvyo-ui-core`: `history_names.rs`, `step_scope.rs` (`FREE_STEPS`), `snapping/` (one centres-only
  query on the Objects target, no new target).
- `curvyo-render-core`: none expected (the fixture look goes through the existing draw list).
- `curvyo-editor-wasm`: `session/fixtures/` new `sets.rs`, `placement.rs` (one owned struct:
  armed kind, hanging position, snap state, Apply preview; `end_placement`), `lock.rs`,
  `apply.rs`, `grid_handles.rs` (Diameter, Columns, Rows knobs and chips); edits in `hit.rs`,
  `draw.rs`, `drag.rs`, `handles.rs`, `chips.rs`, `keys.rs`, `edit.rs`, `params.rs`, `commands.rs`,
  `view_json.rs`, `jobs/view_json.rs`, `jobs/commands.rs`, `jobs/parts.rs`; `session/panel_tabs.rs`
  (`fixtures` tab), `session/layers_panel.rs` (order, muted), `select_bar.rs` (Lock only),
  `wasm_*` bindings. View fixtures `tests/fixtures/fixtures_view/`, `fixture_jobs_view/`; budget
  `tests/fixture_sets_budget.rs` on `tests/cpu_time/`.
- Frontend: `lib/fixturesView.ts`, `lib/fixtureJobsView.ts`, `lib/fixtureRows.ts`,
  `hooks/useFixtures.ts`, new `hooks/useFixtureSets.ts`; `panel/PanelStrip.tsx` (`fixtures`),
  new `panel/FixturesTab.tsx`, `panel/FixtureSetRow.tsx`, `panel/FixtureAddBar.tsx`,
  `panel/JobFixtureRow.tsx`, `panel/PinScrewBody.tsx`, `panel/ApplyFixturesList.tsx`;
  `workbench/FixturesBlock.tsx` (rows reused, block leaves the Document tab),
  `workbench/FixtureGlyphs.tsx` (Pin), `workbench/MachineBlock.tsx` ("Fixture sets in profile: N"),
  `SelectToolbar.tsx` (Lock group), `FixtureJobRow.tsx` / `FixtureJobParts.tsx` (role jobs).

## Part A: laser-core rules, pin and grids, record format 5 (Opus, medium)

Tests first in every task; each new module's doc comment one sentence without "and"; lengths are
`Length` newtypes, comparisons with `PRESET_MATCH_TOLERANCE` or a named tolerance.

- [x] A1. `fixture_kind.rs`: kind `Pin` (`curvyo.laser/pin`, word "Pin"), `pin_diameter_range()`
  0.5 to 1,000 default 6; an unknown kind still round-trips (AC 18, 22).
- [x] A2. `fixture_grid.rs`: `GridCounts { columns, rows }` (1 to 50, product ≤ 400),
  `spacing_range()` 1 to 1,000 default 20, `mark_range()` 0.5 to 50 default 4; `centres(origin,
  counts, column_spacing, row_spacing)` (+x columns, +y rows); `clamp_count(other, wanted)` gives
  the largest allowed count and a flag. Tests: 1 x 1, 50 x 8 = 400, 50 x 9 refused to 8, spacing
  limits (AC 19, 21).
- [x] A3. `fixture_marks.rs`: per centre a circle of `mark_mm` and two cross lines spanning it; the
  pin circle, the screw head and thread circles; a 400-centre grid yields 400 of each in one batch
  (AC 25, 26).
- [x] A4. `fixture_part.rs`: suffixes `.circle` (pin) and `.marks` (pin, screw); a screw's `.head`
  and `.thread` serve Diameters; `fixture_snap.rs` / `fixture_fit.rs`: a pin is no snap and no fit
  target (AC 23, 27).
- [x] A5. `fixture_sets.rs`: `on_bed(set_on_page, active)`, `listing_order(sets, job_fixtures)`
  (sets in order, each with its fixtures in creation order, then job fixtures),
  `job_runs(..)` with the set condition and the line-2 priority "not on this page", "not on the
  bed", "Output off", part count; `is_locked(fixture, is_job_fixture)` (a lock on a set fixture is
  ignored) (AC 2, 8, 16, 28, 32).
- [x] A6. `fixture_stored.rs`: grid params and `mark_mm` read field by field with damage counts; a
  product above 400 reads 1 x 1 and counts one; a pin or screw matrix that is not a translation
  reads its translation part and counts one (AC 19, 21, 40).
- [x] A7. Split `machine_profile_file.rs` (523 lines) before growing it: the version dispatch stays,
  the set layer goes to `machine_profile_sets.rs` ("reads and writes the named fixture sets of a
  profile record"). Then record format 5: `fixture_sets: [{ name, fixtures }]` after `connection`,
  always written; text limit 256 KiB; limits of ADR 0060 §2; a `fixtures` key in format 5 is
  damage; **formats 1 to 4 return `RecordError::Older`** (no read path for them); above 5 `Newer`
  (AC 10, 39, 41).
- [x] A8. `machine_profile_fixtures.rs`: pin entries, screw grid params, `role` on job entries, the
  tokens `pin.circle`, `pin.marks`, `screw.marks`; a pin or screw holds exactly one job per role
  with the fixed tokens, else damage (AC 22, 27, 40).
- [x] A9. `machine_profiles.rs`: an `older` counter, the notice of the no-legacy decision, the id
  skipped (`is_skipped`); `machine_profile_copy.rs`: the copy builds one set per record set (all on,
  fresh ids, Output off, targets mapped by place), Differs compares sets in order by name and
  fixtures, ignoring On this page, Output, locks and job fixtures; Save as and Update write sets
  only (AC 9, 10, 14).
- [x] A10. Goldens `tests/fixtures/machine_profiles/`: new `format5_fixture_sets.json` (reads, writes
  back byte for byte), `format5_max_sets.json` (fits 256 KiB), `format5_damaged_role.json`,
  `format5_damaged_pin.json`, `format5_fixtures_key.json`; `newer_format.json` to format 6; the
  format-1 to -4 goldens become "skipped as older" pins (AC 39 to 41).
- Gate A: `cargo nextest run -p curvyo-laser-core`, clippy, `cargo build -p curvyo-laser-core
  --target wasm32-unknown-unknown`.
- Done 2026-11-15 (implementer). Deviations (Proposal):
  - Document-core gains the record types the profile needs: `JobRole` (`centre_mark`,
    `diameters`), `JobRecord::role` and `FixtureSetRecord`; the `job_role` codec stays B2
    (`FixtureJobSnapshot::record` gives `role: None` until then).
  - A4: `fixture_snap.rs` and `fixture_fit.rs` are per-shape geometry with no kind dispatch, so a
    pin needs no change there; "a pin is no snap or fit target" is the kind dispatch of
    `editor-wasm` `fixtures/snap.rs` and `fit.rs` (C2, C6).
  - A5: no `job_runs`: `FixtureJobRead::runs` stays and callers pass `on_bed(home, active)` as
    `fixture_active`; line 2 is `JobState::of`. `listing_order` works on positions, as `NodeId`
    cannot be built outside document-core.
  - A6: a pin without `diameter_mm` counts one damaged value (criterion 40: "a pin with no
    diameter"); a screw's absent grid keys read as 1 x 1 without damage.
  - A7: the record goldens moved from the unit tests to `tests/machine_profile_records.rs`
    (`machine_profile_file.rs` 523 to 238 lines); limits renamed `MAX_SET_*` to `MAX_RECORD_*`
    plus `MAX_RECORD_SETS`. The six `damaged_*.json` and `generic_laser_copy.json` goldens move
    to format 5 so their damage rules stay pinned (as format 1 they would only read "older").
  - A8: the writer normalises role jobs so every write reads back: a missing role job is added
    ("Centre mark", "Diameters", Show on), a second one of a role dropped, kind and ticks fixed;
    a pin keeps no other job; a role on another kind is dropped; a pin's or screw's matrix is
    written as its translation part.
  - A9: the copy into a page (one set node per record set, fresh ids, Output off) is a document
    write and moves to B7; laser-core gives `sets_differ` for Differs.
  - Interim glue in `curvyo-editor-wasm` until Parts B and C, so the workspace builds: the page's
    flat fixtures are saved as one set "Set 1" and a pick flattens the record's sets
    (`session/manufacturing/mod.rs` `flat_sets`, `one_set`); a pin reads as an unknown kind in
    `placed.rs` and `view_json.rs`; part words "Marks" and "Circle" (UX 6).
  - Superseded pins updated (each with a "Superseded by `0113`" line): laser-core
    `acceptance_0026_tester` (format 5, newer is 6, 256 KiB, `fixture_sets` key),
    `acceptance_0102_tester` (format-2 helpers build a format-5 set, 64 fixtures, older),
    `acceptance_0103_tester` and `fixture_jobs_limits_0103` (older, `format5_max_sets.json`, 201
    jobs, 401 tokens), `acceptance_0111_tester` (format 4 older, format 5 golden); editor-wasm
    `acceptance_0026_tester` (`fixture_sets`, newer 6), `acceptance_0102_tester` and
    `acceptance_0103_tester` (format 5 writes, format 2 and 3 records older with a notice, a
    screw lists Marks).
  - Gate run: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -D warnings`;
    nextest laser-core (378), document-core (3193), ui-core and app (2744), editor-wasm (5725, in
    two partitions); clippy and build of laser-core for wasm32; rustdoc `-D warnings` for
    laser-core and document-core.

## Part B: document-core set nodes, job fixtures, roles, Apply, format 47 (Opus, medium)

- [x] B1. `CURRENT_FORMAT_VERSION` 47 with its doc line (a 46 build refuses `fixture_set` by version,
  ADR 0033 §7) (AC 38).
- [x] B2. Codec: set node `shape = "fixture_set"` with `name` and `set_on_page` (absent = on);
  `fixture_locked` on direct fixture children; `job_role` (`centre_mark`, `diameters`); a set
  elsewhere or a non-fixture child of a set is kept and never read; a bad role counts one damaged
  value and reads as an ordinary job (AC 22, 27, 40).
- [x] B3. Reads: `manufacturing_fixtures.rs` and the fixture-job reads follow the set level (fixture
  at depth 2, job at depth 3); direct fixture children read as job fixtures, **which is also how a
  format-46 file reads (no conversion)**; damage counted per set and for the job fixtures (AC 15,
  16, 39, 40).
- [x] B4. Set commands in `manufacturing_fixture_set_commands.rs`: add ("Set <N>", lowest unused,
  on), rename (1 to 40), remove (tree delete of the subtree), move, set on page on and off; a set id
  passed to a fixture command is refused (AC 3 to 6, 8).
- [x] B5. Fixture commands: add into a set (Active on, centred) or as a job fixture ("<Kind> <N>"
  per kind among job fixtures, at a given centre); a pin or screw is created with its two role jobs
  (score, Output off, fixed ticks); refusals: delete a role job, change its ticks, add a job to a pin
  or screw; lock and unlock (job fixtures only); resize commits for diameter, counts, spacings and
  `mark_mm` (AC 7, 12, 13, 19, 20, 22, 25, 27, 31).
- [x] B6. `fixture_apply.rs`: deep copy of every job fixture of the source manufacturing (slots,
  jobs, registers), appended after this page's job fixtures, fresh ids, same matrix, name, lock,
  `visible`, `active`, every `op_output` false, `target.` keys remapped to the copy's ids, never
  mirrored; returns the count of copies outside the work area (AC 35 to 37).
- [x] B7. Pick profile and Reset from profile replace the set nodes only (job fixtures untouched);
  Generic laser removes the sets, notice "Fixture sets removed (undo restores them)." (AC 9, 14).
- [x] B8. Validator accepts set nodes and their children. Labels in `curvyo-ui-core`
  `history_names.rs` and `FREE_STEPS`: `add_fixture_set`, `rename_fixture_set`,
  `remove_fixture_set`, `move_fixture_set`, `set_fixture_set_on_page`, `set_fixture_set_off_page`,
  `add_fixture`, `add_job_fixture`, `lock_fixture`, `unlock_fixture`, `apply_page_fixtures`; role
  toggles reuse `set_fixture_job_output_on`/`_off`, sizes `resize_fixture` (UX section 11).
- [x] B9. Goldens `tests/fixtures/`: `fixture_sets_v47.curvyo` (AC 38: two sets one off, a grid, a
  holder with two slots, a 4 x 1 screw grid with Centre mark on and `mark_mm` 3, a fixture with
  `active` off, a locked 2 x 2 pin grid with Diameters on, a single pin, a hand-set tick; saves and
  reopens identical); `fixture_sets_damaged_v47.curvyo` (AC 40: text in `diameter_mm`, a NaN
  spacing, a missing set name, a pin with no diameter, a count of 0, a product of 401, a third role
  job, a rotated pin matrix; nothing rewritten); `fixtures_pre_sets_v46.curvyo` (AC 39: flat
  fixtures with jobs and a screw open unchanged as job fixtures, no set, no role jobs, nothing
  written). The v39 to v41 fixture goldens keep their assertions.
- Gate B: nextest for document-core and ui-core, clippy, wasm32 build of both.
- Done 2026-11-15 (implementer). Deviations (Proposal):
  - Snapshot shape: `ManufacturingSnapshot::fixtures` stays one list, now in listing order (sets'
    fixtures, then job fixtures); each `FixtureSnapshot` gains `set` and `locked`; `sets` holds
    `FixtureSetSnapshot { id, name, on_page, damaged }`; helpers `job_fixtures`, `set_fixtures`,
    `set_on_page`, `set_records`. Set reads in new `manufacturing_fixture_sets.rs`; a missing set
    name reads "Set n" (its place).
  - Modules: set commands and `add_set_fixture` / `replace_sets` in
    `manufacturing_fixture_set_commands.rs`; `add_job_fixture` and `set_fixture_locked` in
    `job_fixture_commands.rs`; Apply in `fixture_apply.rs`. Integration tests
    `tests/fixture_sets_commands.rs`, `tests/fixture_sets_format.rs` instead of a `_tests.rs`.
  - B4/B5: the add limits (16 sets, 64 job fixtures) are also command refusals
    (`TooManySets`, `TooManyJobFixtures`), as `TooManySlots`. The `0102` `add_fixture` (job
    fixture, per-kind label, no role jobs) stays for document-core tests and old-file goldens;
    `NewFixture::add` is unused by the two new adds. Role kinds (`curvyo.laser/pin`, `/screw`) and
    role parts live in `fixture_job_codec.rs`; `PART_SUFFIXES` gains `circle`, `marks`.
  - B6: `apply_page_fixtures` returns the copies' ids; the "outside the work area" count needs the
    kind geometry and moves to the session (C7). Registers are copied raw (unknown keys too).
  - B7: `set_manufacturing_profile_and_fixtures` is renamed `set_manufacturing_profile_and_sets`
    (takes `FixtureSetRecord`s). Editor-wasm: pick, reset, Save as, Update and Differs use the
    page's sets (`sets_differ`); `flat_sets`/`one_set` removed; the notice reads "Fixture sets
    removed (undo restores them)."; interim until C: the `0102` add buttons add into the first set
    (a "Set 1" made in the same step), keeping their `add_<kind>` label.
  - B9: "a product of 401" is impossible with counts up to 50 (401 is prime); the golden uses
    20 x 21. Kind damage of that golden (text, no diameter, 0, product, turned pin) is counted by
    laser-core (`tests/fixtures_stored.rs`), document damage (NaN, set name, `set_on_page`,
    `fixture_locked`, role) by document-core.
  - Superseded tests updated: document-core `fixtures_commands` (pick keeps job fixtures),
    `fixture_jobs_commands` (sets API), `clone_repair_guard` (exempt the set module),
    `acceptance_0050_tester` (skip the 0113 goldens); every format-46 pin to 47 and "newer" to 48
    (document-core tests, editor-wasm `acceptance_0106`/`0115`, svg-core `*.document.json`);
    editor-wasm `acceptance_0103_tester` (a screw takes no further job), `tests_refusal` (notice).
  - Gate run: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -D warnings`;
    nextest document-core, laser-core, ui-core, svg-core (6374); editor-wasm lib (1146) and
    `acceptance_0026`, `0102`, `0102_rev2`, `0103`, `0106`, `0115`, `fixture_jobs_gaps_0103`,
    both view contracts; wasm32 build and clippy of document-core, laser-core, ui-core; rustdoc
    `-D warnings` for document-core, laser-core, ui-core, editor-wasm. The other editor-wasm test
    binaries were not run (Part F full gate).

## Part C: session, centre snapping, view JSON, wasm bridge, budgets (Opus, medium)

- [x] C1. `curvyo-ui-core` snapping: a centres-only query on the Objects target (circle, ellipse,
  polygon, box centre) through the existing index, with capture, release, hysteresis and Alt
  rules; test on 1,000 shapes against the index, not a scan (AC 30, 42b).
- [x] C2. `session/fixtures/sets.rs`: set commands, fold state (session only), the on-the-bed rule
  feeding draw, hit, snap, fit and jobs; a fixture of a set turned off is deselected (AC 2 to 8).
- [x] C3. `placement.rs` (Finding 1): arm a kind, hanging centre, each grid centre tried against C1
  and the nearest match moves the grid, drop is one step and stays armed; Escape, right click,
  second press, tool change and page activation call `end_placement`; while armed a click selects
  nothing; keyboard: work-area centre, arrows 1/10 mm with snapping, Enter drops. Placement runs
  through the Rust tool pipeline, not new handlers in `useEditorSession.ts` (Finding 2) (AC 13, 30).
- [x] C4. `lock.rs`: Lock gates drag, nudge, handles, knobs, Move and S chips through
  `is_locked`; a drag on a locked box does nothing and starts no marquee; lock badge and Lock
  cursor in the draw list; trash, eye and Active still work (AC 31, 32).
- [x] C5. `grid_handles.rs`: box, centre move, Diameter knob (pin; screw keeps Head and Thread),
  Columns and Rows knobs (hidden at count 1, shown at 72 px), drag changes spacing with the readout,
  double-click chip with count and spacing; no rotate, no R, no skew; S opens no chip (F11)
  (AC 19 to 21).
- [x] C6. Draw: pin circle with opaque ground, every grid centre, Centre mark and Diameters in the
  Score look when on and on the bed, fixture ink otherwise; draw batched per kind; nothing in SVG
  export, print or drawing export (AC 18, 25, 26, 43).
- [x] C7. `apply.rs`: candidates (other laser pages with job fixtures and their counts), preview of
  the picked page (dashed, not hit), Apply commit through B6 and the outside notice "<n> fixtures
  lie outside the work area." (AC 33 to 37).
- [x] C8. Views (Finding 3): `fixtures_view` gains sets (name, on page, kinds, count), job fixtures
  (lock, grid, size, roles on), Apply candidates, the "Fixture sets in profile: N" count;
  `fixture_jobs_view` gains `role` and "not on this page". Shared fixtures (Rust pins the JSON, TS
  rejects a missing and a mistyped field): `fixtures_view/two_sets_one_off.json`, `empty_page.json`
  (replaces `empty.json`), `locked_pin_grid.json`, `screw_grid_centre_mark.json`,
  `apply_candidates.json`, `damaged_set.json`, `old_file_job_fixtures.json` (no-legacy pin);
  `fixture_jobs_view/role_jobs.json`, `set_off.json`. Layers order and muted rows; Select bar of a
  job fixture: Lock only; panel tab `fixtures` on laser pages; wasm bindings (AC 1, 2, 11, 12, 16,
  28, 31).
- [x] C9. Budgets `tests/fixture_sets_budget.rs` on `tests/cpu_time/` (thread CPU time, never wall
  time): (a) hit ≤ 2 ms and draw list ≤ 16 ms on 3 sets x 20 fixtures, 64 job fixtures, a 400-centre
  pin grid with both jobs on; (b) a snapping drag move ≤ 16 ms on 1,000 shapes; (c) Apply of 64 ≤
  100 ms; (d) the Fixtures tab view of 16 sets and 64 job fixtures ≤ 100 ms (AC 42). No migration
  budget (dropped with the migration).
- Gate C: nextest for ui-core and editor-wasm, clippy, `npm run build:wasm`.
- Done 2026-11-16 (implementer). Deviations (Proposal):
  - C1: `SnapTargets::gather_centres` in new `ui-core/src/snapping/centres.rs` gathers one point
    target per visible object (an ellipse's and a polygon's frame centre, else its outline box
    centre) and resolves through the existing index and `resolve` (capture, release, hysteresis,
    Alt by `suspended`, the `0060` marker). Tested against a full scan on 1,000 shapes.
  - Feature-owned state: `FixtureState` gains `sets: SetFolds` (unfolded sets) and `placement:
    Placement` (armed kind, hanging centre, its `CentreSnap`, the Apply source); `end_fixture_placement`
    is called by Escape, a tool change and a page switch; a right click is the host's call to it.
    The session's pointer dispatch calls `fixture_placement_hover/press/release/leave`;
    `pointer_down` is over 100 lines with it (allowed, justified in the PR).
  - Removed: `Session::add_fixture(kind)` and its binding, the "Set 1 in the same step" glue, the
    pin-as-unknown shims. New: `add_set_fixture(set, kind)` ("Add fixture") and placement's
    `drop_fixture_placement` ("Add job fixture"). Tests written against the `0102` buttons use a
    test helper that adds into the first set (`tests/fixture_support/mod.rs`, `fixtures::add_fixture`
    under `cfg(test)`); where they counted one step, "+ Set" is done first.
  - C5: a pin's and a screw's grid live in `Shape::Pin`/`Shape::Screw` (`CentreGrid`), written as
    whole counts; new pins and screws are written as a 1 x 1 grid with all grid keys. Knobs:
    `FixtureHandle::{Diameter, Columns, Rows}` in new `grid_handles.rs` (Diameter at 72 px, Columns
    and Rows at a count above 1), chips `Chip::Grid` (count and spacing, or Diameter); S opens no
    chip for a pin or a grid of screws, a single screw keeps its `0102` S chip; no R for either.
    Row fields through `set_fixture_grid_value(id, key, value)`; the Size commit is "Resize
    fixture" (no "Edit fixture job" label exists, UX F8 not followed).
  - C6: `curvyo-render-core` gains `PinLook`, `FixtureLook::pins`/`marks` and
    `build_fixture_preview` (dashed `--preview-new`); Centre marks draw in fixture ink while the
    role job exists, their Score look and the Diameters come from the job looks under View:
    Operations (`jobs/look.rs` geometry for `.circle`, `.marks`, and every centre of a grid).
  - C4: the lock badge is a `FixtureLabel` of kind `lock` at the box's upper left (see G3); the Lock
    cursor is `fixture_cursor` "lock"; the Select bar's Lock is `selected_fixture_lock()`.
  - C8: `FixturesView` is `sets` (with `line2`, `can_add_grid`, `unfolded`), `job_fixtures`,
    `sets_on_page`, `can_add_set`, `can_add_job_fixture`, `armed_kind`, `apply_candidates`,
    `apply_picked`; each row gains `on_bed`, `locked` (job fixtures only), `line2` and `round`
    (pin and screw fields, role toggles; `None` toggles for an old screw). Damage notices count per
    set and for "Job fixtures" (criterion 40). `fixture_jobs_view` rows gain `role` and
    `on_this_page`; a pin or screw selected offers no add target. Layer rows gain `onPage`. The
    Machine block view gains `fixture_sets_in_profile` (its count of sets) beside
    `fixtures_in_profile`, not in `fixtures_view`. Panel tab `fixtures` in
    `curvyo-ui-core` `PanelTab`/`PanelContent`. Bridge `wasm_fixture_sets.rs`. Modules split:
    `labels.rs`, `row_lines.rs`; `placed.rs` (537) and `entry.rs` (525) stay just over 500.
  - C8 fixtures: the nine of the plan in `tests/fixture_sets_view_contract.rs`; `empty.json`
    removed; `every_kind`, `selected_holder` and the three `fixture_jobs_view` ones regenerated.
    The TypeScript side (decoders, `useFixtures.add`) is Part D: `npx tsc -b` and `npm test` fail
    until D1/D4.
  - C9: `tests/fixture_sets_budget.rs`, added to the CI `performance-budgets` job; asserted in
    release only. Debug times: press 7.3 ms, draw list 57 ms, snapping move 0.02 ms, Apply 4.3 ms,
    tab view 4.8 ms. The release run did not finish within the 10-minute limit locally (LTO build).
  - Superseded tests updated (each with a "Superseded by `0113`" line): editor-wasm
    `acceptance_0102_tester`, `_rev2` (rows of sets then job fixtures, one grid per set, notice
    "in", "+ Set" first), `acceptance_0093_tester`, `acceptance_0098_tester` (Fixtures tab),
    `acceptance_0103_tester` (notice), `acceptance_0024_slice2_tester` (format 47, E1),
    `acceptance_0020_tester_more` (the 0113 goldens' two adjacent "Add fixture set" rows), unit
    tests of `fixtures/` and `manufacturing/`.
  - Gate run: `cargo fmt --all --check`; clippy `-D warnings` all targets for ui-core, render-core,
    editor-wasm, and for wasm32; nextest ui-core and render-core (3102), editor-wasm in seven
    chunks (hash partitions 1 to 6 without `acceptance_0020_tester_more`, then that binary alone):
    all pass after the fixes above; wasm32 build of the three; rustdoc `-D warnings` of the three;
    `npm run build:wasm`.

## Part D: frontend (Sonnet, medium; after the finished UX notes)

- [x] D1. Decoders `fixturesView.ts`, `fixtureJobsView.ts` check every field; tests decode each C8
  fixture and reject a missing and a mistyped field.
- [x] D2. `PanelStrip.tsx`: `fixtures` with the Holder glyph after Operations and Materials, laser
  pages only; `FixturesTab.tsx` with Machine sets ("<n> of <m> on this page") and Job fixtures;
  the Document tab loses the Fixtures block (AC 1, 11).
- [x] D3. `FixtureSetRow.tsx` + `useFixtureSets.ts`: chevron, name, line 2, On this page, armed
  trash with its status, arrows, drag with the insertion line, Alt+arrows, rename in place, "+ Set"
  hidden at 16; windowed with `useScrollWindow` (AC 2 to 6, 17).
- [x] D4. `FixtureAddBar.tsx`: set bar (Grid hidden when present, six kinds) and job bar (five
  kinds, arms placement, `aria-pressed`, sr status "Placing Pin.", hidden at 64); set fixture rows
  reuse the `0102` rows with their own On the bed check (AC 7, 8, 12, 13, 17).
- [x] D5. `JobFixtureRow.tsx`: check, glyph, name, Lock, eye, trash, line 2 per UX 5.2; one unfolded
  fixture row in the tab (F13) (AC 12, 31, 32).
- [x] D6. `PinScrewBody.tsx`: Diameter (pin only), Columns and Rows units with Spacing hidden at 1,
  clamping notices, the 400 notice; Jobs toggles as the role jobs' Output with Size; the toggles
  hidden when a screw has no role jobs (old file) (AC 18 to 20, 25, 39).
- [x] D7. `ApplyFixturesList.tsx`: in place radiogroup, preview, "Apply <n> fixtures", Cancel and
  Escape, focus back to the button (AC 33 to 36).
- [x] D8. `SelectToolbar.tsx` Lock group; `MachineBlock.tsx` "Fixture sets in profile: N";
  `FixtureGlyphs.tsx` Pin; `FixtureJobRow.tsx`/`FixtureJobParts.tsx`: no trash or rename for role
  jobs, static ticks, "not on this page" (AC 10, 27, 28, 31).
- Gate D: `npm run build:wasm && npx tsc -b --noEmit && npm test && npx oxlint`.
- Done 2026-11-16 (implementer). Gate D: `npm run build:wasm` exit 0, `npx tsc -b --noEmit` exit 0,
  `npm test` exit 0 (1,397 tests, 0 failed), `npx oxlint` exit 0 (no warning in the new files). No
  browser run (Part F). E1: no format 46 pin remains (Part C moved them). Deviations (Proposal):
  - D1: both decoders require every field (a missing `selection_announcement` is refused now);
    `tests/contractWalk.ts` breaks every field of the nine `fixtures_view` and the five
    `fixture_jobs_view` fixtures (missing and mistyped, at every depth) and each must decode to
    `null`. `FixturesView` is `sets` and `jobFixtures`; `rows`, `canAddGrid` and `rowLine` are gone
    (line 2 is the session's `line2`).
  - D2: the manifest still lists a `fixture_list` block (Rust, `curvyo-laser-core`), so
    `ManifestTab` draws nothing for it instead of the manifest changing; the tab subject line names
    the selected fixture (F16). `FixturesBlock.tsx` is deleted, its rows, fields and slots moved to
    `FixtureRowItem.tsx`, `FixtureBody.tsx`, `FixtureFields.tsx`.
  - D3, D4, D5: set rows and job fixture rows are not windowed with `useScrollWindow` (at most 16
    and 64 rows, one list in the tab's scroll; budgets are the session's, Part C); job rows are
    `FixtureRowItem` with `job` (no `JobFixtureRow.tsx`); set drag is `hooks/useSetDrag.ts`.
  - D6: Size commits through `set_fixture_grid_value("mark_mm")`, which is "Resize fixture" in
    Rust (the C5 deviation), not "Edit fixture job".
  - D7: the "outside the work area" notice shows in the tab (role status, 5 s), not in the canvas
    notice slot.
  - D8: the Lock-only Select bar is its own component `FixtureLockBar.tsx`, chosen in `App.tsx`,
    instead of a group inside the 1,000-line `SelectToolbar.tsx`; a set fixture keeps no bar.
  - Superseded tests (each marked): `acceptance_0093` (`add_fixture`), `_0098` (page-scoped tabs),
    `_0100` (nothing follows Place page), `_0103` (row helper fields).

## Part E: mechanical follow-ups (Haiku)

- [ ] E1. Format pins 46 to 47: document-core tests, `acceptance_0106_tester`,
  `acceptance_0024_slice2_tester`, the SVG export `*.document.json` goldens of `curvyo-svg-core`;
  profile `newer_format.json` pins to 6.
- [ ] E2. Superseded tests (rows of "Changes to accepted specs"): `0102` tests that look for the
  Fixtures block in the Document tab, the `Add <kind>` step names (M1), `0103` grouping order,
  profile tests that read formats 2 to 4 into a fixture set (now skipped as older); each listed in
  the PR.
- [ ] E3. Docs at acceptance: ADR 0059 and 0060 status; notes on ADR 0049 §1 §2 §7, 0050 §1 to §5
  §8, 0052 §1 to §3 §7, 0039 §4 §5; `docs/adr/index.md`; `docs/design-system.md` rows of UX
  section 14 and the M7 exception; `docs/technical-debt.md` if a budget or deviation is left; spec
  `Status: Done (PR #n)` (lead).
- Gate E: nextest of the touched crates.

## Part F: verify (Sonnet, medium)

- [ ] F1. Chromium run of the demo path (one agent drives the browser), screenshots to the
  scratchpad `0113/`; UX review (ux-engineer); architect review (format 47, record format 5, set
  node, the no-legacy decision).
- [ ] F2. Full gate of `CLAUDE.md` §7, steps separately; report to the lead.

## Part G: UX review gaps (2026-11-16, Opus, medium)

- [x] G1 (criterion 34, UX F2). `build_fixture_preview` carries the dash phase along joined sides
  (a side starting within 1e-9 mm of the last one's end) and restarts it where the outline jumps,
  so a previewed pin or screw circle reads dashed, not solid. Render-core only; the job-look reuse
  of `jobs/look.rs` (commit 4397cc26) draws through `build_artwork` and is untouched.
- [x] G2 (UX notes 10, `0102` 4.1). A clamp or refusal notice shows under the field that was
  committed: `NumberField` claims the next command through `FieldNoticeContext`, `useFixtures`
  keeps `{ text, field }`; a notice no field gave (Apply outside the work area) stays at the
  tab's foot (`noticeUnder`, `noticeInTab`).
- [x] G3 (criterion 32, UX notes 7). The lock badge yields the upper right to the job chips: it
  stands 4 px outside the box's **upper left** corner, left of the selected fixture's tag, which
  stands above that corner to the right (Proposal). Known limit: near the canvas's right edge the
  chips flip to the upper left too and may meet the badge.
- [x] G4 (criterion 42a, review: idle frame 33 to 67 ms, a row press 0.4 to 1.5 s in software
  rendering). Cause: every animation frame rebuilt the whole frame list, fixtures included, and
  uploaded all of it as one new vertex buffer, changed or not (588,012 vertices for one 20 x 20
  pin grid). Fixes: the fixtures' page lists are cached by document version, pages, view, device
  pixel ratio, canvas size, workbench, View: Operations and extensions (bypassed during a drag;
  tests cross-check the cache against a fresh build). A frame skip in `Gpu::render` (submit
  nothing for a frame equal to the one shown) was removed after the customer demo: the webview
  shows a submitted WebGL frame only once the next one is submitted, so the last frame of every
  change stayed hidden (a marquee stayed after release, the page trailed the wheel by one step).
  Every animation frame is submitted again; per-layer GPU buffers are an architect question.

## Rework: sets in the profile (2026-11-16, architect; customer feedback on the build)

Customer: "machine sets cannot be locked yet, and they are bound to the page; they should be
persistent across all pages and instances". Decided in ADR 0059 (amended second: mirror, uids,
alignment, decision 10) and ADR 0060 (§1 ids and locks, no `active`; §5a edits write the record;
§6, §7 shrink), `adrs.md` 2026-11-16, spec criteria 3, 7, 8, 9, 10, 17, 32, 38, 40, 42 amended
and 44 to 47 new, UX notes 4.4, 4.5 (new), 7, 10, 11, 12. Still format 47 and record format 5
(both unreleased). Frage 218 open with default A (a new set is off on the other pages); build A.

What stays: the set node subtree, every set and fixture command, the readers at depth 2 and 3,
draw, hit, jobs, pins, grids, Apply, placement and snapping. The subtree becomes the page's
**mirror** of the profile's sets; content edits write the record in the same step; other pages
follow by **alignment**. Order: R1, then R2, then R3 and R4, then R5 (one agent at a time, one
checkout).

### R1. laser-core: record ids and locks (Opus, medium; redoes parts of A5, A7 to A10)

- [x] R1.1 `machine_profile_sets.rs` / `machine_profile_fixtures.rs`: set `{ id, name, locked,
  fixtures }`, entry `id` first and `locked` after `name`, job `id` first; no `active`; damage:
  missing or duplicate id (across the record), missing `locked`, an `active` key (AC 38, 40).
- [x] R1.2 `fixture_sets.rs`: `is_locked(fixture_locked, set_locked)` for set fixtures, the job
  fixture rule unchanged; `on_bed` unchanged (AC 32).
- [x] R1.3 `machine_profile_copy.rs`: `sets_differ` compares contents only (names, order, locks,
  ids, fixtures without `active`, slots, jobs and tokens) for the alignment test; Differs of the
  page copy drops sets (ADR 0039 §5 fields only); Update profile keeps the record's
  `fixture_sets` (AC 10).
- [x] R1.4 Goldens: regenerate every `format5_*.json`; `format5_fixture_sets.json` with a locked
  set and fixture; new `format5_duplicate_id.json`, `format5_missing_id.json`,
  `format5_active_key.json`, `format5_align_newer.json` (ADR 0060 §9); `format5_max_sets.json`
  fits 256 KiB.
- Gate R1: nextest, clippy and wasm32 build of `curvyo-laser-core`.

### R2. document-core: mirror registers, alignment (Opus, medium; redoes parts of B2 to B5, B7, B9)

- [x] R2.1 Codec: `set_uid`, `set_locked` on set nodes; `fixture_uid` and `fixture_locked` (now
  read) on set fixtures; `job_uid` on set fixture jobs; `mfg_fixture_sets_modified` on the
  manufacturing; `FixtureSetRecord`, `FixtureRecord`, `JobRecord` gain `id`, `locked` (set and
  fixture) and `FixtureRecord` loses `active`; `set_records` writes them. Reads: a set without
  `set_uid` counts one; a second node with the same uid is kept and not read (AC 40).
- [x] R2.2 Commands: the add commands take the uid; `lock_fixture_set` / `unlock_fixture_set`;
  lock and unlock accept set fixtures; `replace_sets` (Pick) builds the mirror from records with
  their uids, every set on, every fixture On the bed, Output off; Reset no longer touches sets;
  Generic laser deletes the mirror, the notice "Fixture sets removed" goes (AC 9, 32).
- [x] R2.3 New `manufacturing_fixture_align.rs`: `align_fixture_sets(mfg, sets, modified)` as
  ADR 0059 §10 (match by uid, keep node ids and choices, new sets off, new fixtures on, delete
  missing, follow order, slots by place, ticks from tokens, twin nodes deleted, write
  `mfg_fixture_sets_modified`); the reserved non-step label `align_fixture_sets` beside
  `migrate_format` (no stack entry, no history row); labels `lock_fixture_set`,
  `unlock_fixture_set` in `history_names.rs` and `FREE_STEPS` (AC 44, 47).
- [x] R2.4 Goldens: regenerate `fixture_sets_v47.curvyo` and `fixture_sets_damaged_v47.curvyo`
  (ADR 0059 §9 amended); new `fixture_sets_align_v47.curvyo` with the guarded and the not-newer
  case; the v39 to v41 and `fixtures_pre_sets_v46` goldens keep "nothing written on open".
- Gate R2: nextest of document-core and ui-core, clippy, wasm32 build.

### R3. session: record writes, journal, read-only gate, alignment triggers (Opus, medium; redoes parts of C2, C4, C8, C9)

- [x] R3.1 Move the material journal (`session/materials/undo.rs`) to
  `session/library_journal.rs` ("the record part of this session's library steps") with a record
  kind (material, machine); one 16 MiB budget; the material behaviour unchanged (a refactorer move
  first, Haiku, behaviour-preserving, then the machine kind here).
- [x] R3.2 New `session/fixtures/profile_sets.rs`: `editable(page)` (profile record loaded and
  readable); every set-content command goes through one `commit_set_edit` that commits on the
  mirror, aligns the project's other manufacturings on that profile, writes
  `mfg_fixture_sets_modified`, emits the host record write and the journal entry (ADR 0060 §5a);
  a drag writes at release only; choices (On this page, On the bed, eye, Output) stay plain steps.
  Undo and Redo through the journal with the three notices of UX 10 (AC 44, 45).
- [x] R3.3 Alignment triggers: after the project opens, after every library reload, at page
  activation: guarded `align_fixture_sets` when the record's `modified` is above the page's and
  `sets_differ`; nothing when not newer (AC 47).
- [x] R3.4 Read only: refuse every content command when not `editable` (Generic laser, record
  missing, damaged, older, newer); Save as profile writes the mirror with its uids and points the
  page at it, then the page is editable (AC 3, 10, 46).
- [x] R3.5 Lock: `lock.rs` asks `is_locked(fixture_locked, set_locked)` for set fixtures (drag,
  nudge, handles, knobs, Move and S chips; badge; cursor); set Lock commands; the Select bar Lock
  for a selected set fixture (pressed and not pressable while its set is locked) (AC 32).
- [x] R3.6 Profile actions: Differs, Update, Reset on machine fields only; Pick as R2.2; the
  "Fixture sets removed" notice goes (AC 9, 10).
- [x] R3.7 Views: set rows gain `locked`, `can_add` false while locked; set fixture rows gain
  `locked` and `locked_by_set`; the section gains `read_only` with its line and `profile_hint`
  (Generic laser); `can_add_set` false on both; fixtures regenerated
  (`fixtures_view/two_sets_one_off.json`, `locked_pin_grid.json`, new `locked_set.json`,
  `read_only_sets.json`, `generic_laser_hint.json`) (AC 3, 32, 46).
- [x] R3.8 Budgets in `tests/fixture_sets_budget.rs`: (e) and (f) of criterion 42 (thread CPU
  time); (a) to (d) rerun.
- Gate R3: nextest of editor-wasm (chunks as Part C), clippy, `npm run build:wasm`.
- Built 2026-11-16 (implementer, Proposal): fixture and job uids of a set edit are given in the
  edit's own step by `fill_fixture_set_uids` (only `add_fixture_set` takes its uid), so the add
  commands keep their signatures; the session's id source is seeded from the peer and the clock
  and mixes the host's bytes from `seed_record_ids` (R4 calls it once); the edited page itself is
  aligned to the written record in the same step, which also writes its
  `mfg_fixture_sets_modified`; read-only set fixtures keep their place like locked ones but show
  no badge; `machine_record_write_failed(id)` is the host's report for limit (f). The tester
  tests that add "+ Set" on Generic laser or expect the old behaviour are left for R5.1.

### R4. frontend (Sonnet, medium; after the ux-engineer has reviewed UX 4.5; redoes parts of D3 to D5, D8)

- [x] R4.1 Decoders for the R3.7 fields; tests reject a missing and a mistyped field.
- [x] R4.2 `FixtureSetRow.tsx`: Lock toggle, " · locked", add bar hidden while locked; set fixture
  rows: Lock slot (muted, not pressable while the set is locked); `FixtureLockBar.tsx` for set
  fixtures; the section line for read only and Generic laser; content controls hidden in read
  only (AC 3, 32, 46).
- [x] R4.3 `MachineBlock.tsx`: Differs without sets; no "Fixture sets removed" notice (AC 9, 10).
- [x] R4.4 Host: reload the open pages' profile records on window focus (desktop and browser)
  and on a browser `storage` event of a `curvyo.machines/` key, then hand them to the session
  (AC 44).
- Gate R4: `npm run build:wasm && npx tsc -b --noEmit && npm test && npx oxlint`.
- Built 2026-11-16 (implementer, Proposal): no `FixtureLockBar.tsx` split was needed, the existing bar takes set fixtures; `hooks/useMachineRecords.ts` flushes queued record writes after each re-read, shows the failed-write notice and reloads on focus and `storage` (`lib/machineProfilesFile.ts`); the section lines wrap instead of truncating; read only set fixtures keep only the Output toggles of a pin or screw. R4.3 needed no code (Differs and the notice are Rust's).

### R5. tests, docs, verify (Haiku for R5.1 and R5.2, Sonnet for R5.3)

- [ ] R5.1 Superseded pins (each with a "Superseded by `0113` rework" line): tests that compare
  sets in Differs, that read `active` from a record, that expect "Fixture sets removed", that
  add "+ Set" on Generic laser (they pick a saved profile first), that expect set fixtures
  without Lock.
- [ ] R5.2 Part E as before, plus `docs/technical-debt.md` "Record conflicts" names frequent set
  edits (lead) and the ADR index line of 0059 and 0060.
- [ ] R5.3 Part F again with the demo path below; the tester writes `acceptance_0113_*` for 3, 9,
  10, 32, 44 to 47 from the spec before reading the code; architect review of R1 to R3.

Demo path changes (replace steps 1, 2 and 7 above):
1. New project, laser page with Generic laser: Machine sets shows the profile hint, no "+ Set".
   Save as profile "Bench"; "+ Set" appears.
2. As before, plus: Lock "Coaster jig": its screw shows the badge and does not move; its add bar
   goes; unlock. Lock the Holder alone; the trash still removes it; undo (the profile too).
7. Add a second laser page on "Bench": both sets are there, the Holder change of step 2 included,
   with this page's own On this page. Add "Set 3" here: on page 1 it reads "not on this page".
   Open a second project on "Bench" in another window: the sets are the same; rename a set there;
   back in the first window it shows the new name on focus. Ctrl+Z in the second window takes the
   rename back in the profile.
11. Read only: copy the project to a data folder without "Bench" (or delete the record): the sets
    show "From the project file. Bench is not on this computer.", no content controls; On this
    page still works; Save as profile makes them editable.

## Review fixes (2026-11-16, architect and UX review)

- [x] A1 After every undo or redo of a set edit, done or refused, each page on the profile whose
  mirror differs from the loaded record is aligned, whatever the times (criterion 45; ADR 0060 §5a).
- [x] A2 `align_fixture_sets` in `RESERVED_LABELS` (criterion 47).
- [x] A3 Golden `snapping_centres_type_v47.curvyo`, refused as damaged (criterion 30).
- [x] A4 Doc comments of `CURRENT_FORMAT_VERSION` 47 and record format 5.
- [x] A5 Unused `listing_order`, `ListedSet`, `Listed`, `JobState`, `diameter_circles` removed.
- [x] A6 ADR 0059 §6 names `fill_fixture_set_uids` and its two paths.
- [x] A7 ADR index rows 0059, 0060; three debts; crate placement of `library_journal.rs`;
  `short_name` moved there.
- [x] A8 `fixture_look.rs` split (look, `fixture_overlay.rs`, tests); `panel_tabs.rs` tests split
  out; `view_json.rs` at 500.
- [x] U1 Locked fixture fields as text; U2 Lock toggle colours, `aria-disabled`; U3 badge upper
  left (criterion 32 and UX notes 7 amended); U4 a press inside a pin hits it; U5 job fixture
  numbers skip set fixtures' names (criterion 13 amended); U6 Centre mark ink 1 px only while its
  Score look is not drawn; U7 Apply list 244 px.

### Debt (for the PR text)

Files over 500 lines this story grew (all were over 500 on `main` already): `session/mod.rs`
(2,104), `document.rs` (1,325), `session/keys.rs` (1,304), `layer_rows.rs` (917), `glyphs.rs`
(865), `manufacturing.rs` (642, from 570), `page_tree_rows.rs` (580), `layers_panel.rs` (572),
`session/snapping.rs` (549), `history_names.rs` (530), `history/rows.rs` (522), ui-core
`panel_tabs.rs` (516). Splitting them is behaviour-preserving work for a `refactorer` PR after the
merge.

## Validation

- Tests first for A1 to A10, B1 to B9, C1 to C9; goldens read and write back equal, one damaged
  golden per rule; shared Rust/TS fixtures pin producer and decoder; budgets in thread CPU time.
- The tester writes `acceptance_0113_*` from the spec before reading this code.
- AC 43: no network, device I/O, G-code, output file or telemetry; the tester checks the diff and
  that SVG export, print and drawing export carry no fixture, mark or circle.

## Demo path (desktop and browser)

1. New project, laser page with Generic laser: the Fixtures tab (Holder glyph after Operations and
   Materials) shows "Machine sets" with "+ Set" and "Job fixtures" with its add bar; the Document
   tab has no Fixtures block.
2. "+ Set" twice: "Set 1" and "Set 2", unfolded. In Set 1 add a Grid and a Holder; in Set 2 a
   Screw. Rename Set 2 "Coaster jig". Uncheck its On this page: the screw leaves the canvas, the
   row reads "not on this page", the heading "1 of 2 on this page". Uncheck the Holder's own On the
   bed: "not on the bed". Reorder the sets by drag; undo each step.
3. Draw four circles. Job bar: Pin; the pin hangs at the pointer and snaps its centre to a circle
   (red marker); click, click, click, click: four pins, placement still armed; Escape ends it.
4. Unfold Pin 1: Columns 2, Rows 2, Spacing 30; Diameter 2000: "Diameter is 0.5 to 1,000 mm. Set
   to 1,000 mm."; drag the Columns knob: spacing readout; no rotate glyph.
5. Turn Centre mark on (Size 3) and Diameters on: Score look on the canvas; the Operations tab
   lists "Centre mark" and "Diameters" with the same Output state, no trash.
6. Lock Pin 1 (row or Select bar): drag and arrows do nothing, the badge shows, line 2 "locked";
   the trash still removes it; undo.
7. Save as profile "Bench": the Machine block reads "Fixture sets in profile: 2"; on a new page
   with "Bench" both sets are on and no job fixtures came along.
8. On a second laser page "Apply fixtures from page…": pick the first page, preview dashed, "Apply
   4 fixtures": copies at the same positions, Output off, not mirrored.
9. Save, reopen: sets, On this page, grids, Size, Lock, Output and ticks as before.
10. No-legacy check: open a format-46 project with fixtures (`fixtures_pre_sets_v46.curvyo`): its
    fixtures appear under Job fixtures, there is no set, the screw shows no Jobs toggles, the file
    is unchanged on disk. A format-4 profile record in the data folder is not listed and the load
    notice reads "1 profile has an older format. It was left as it is."
