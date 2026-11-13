# ADRs for fixture sets and pins

Status: Proposed (architect, 2026-11-12). Builds third, after `0111` and `0112` (needs the profile record format 4 of ADR 0056 and the Materials tab before Fixtures). File shapes: [ADR 0059](../../docs/adr/0059-fixture-sets-job-fixtures-and-pins-in-the-project-file.md) (project file, format 47) and [ADR 0060](../../docs/adr/0060-fixture-sets-in-the-machine-profile-record.md) (profile record format 5), both `needs-customer`, accepted with the story PR. Decisions marked **(Proposal)** are the architect's and vetoable at the demo.

- [ADR 0059](../../docs/adr/0059-fixture-sets-job-fixtures-and-pins-in-the-project-file.md): set nodes with `set_on_page`, job fixtures as direct children with `fixture_locked`, the on-the-bed rule, the `pin` kind and pin and screw grids, `job_role` jobs with `.marks` and `.circle`, Apply fixtures from page, migration into "Set 1", limits.
- [ADR 0060](../../docs/adr/0060-fixture-sets-in-the-machine-profile-record.md): `fixture_sets` in record format 5, pins and roles in entries, reading of formats 1 to 4, copy into a page keeping job fixtures, Differs.
- [ADR 0049](../../docs/adr/0049-laser-fixtures-in-the-project-file.md): fixture node, slots, matrix, damage per field; §1, §2, §7 amended.
- [ADR 0050](../../docs/adr/0050-laser-fixture-set-in-machine-profile.md) and [ADR 0052](../../docs/adr/0052-laser-fixture-jobs-in-the-project-file.md): record, copy, jobs and part keys; amended by ADR 0059 §5 and ADR 0060.
- [ADR 0039](../../docs/adr/0039-machine-profile-copy-in-the-project-file.md) §4, §5: Pick, Reset and Differs now act on sets only.
- [ADR 0042](../../docs/adr/0042-page-kinds.md) §9: `migrate_format` reused.
- [ADR 0032](../../docs/adr/0032-workbench-model-and-registry.md): the skip predicate covers sets, fixtures (depth 2) and jobs (depth 3).
- `0060` snapping (Objects target and its index): reused for centre snapping; no ADR change.

## Feature-local decisions

- 2026-11-12: **Format.** Project `format_version` 47, profile record format 5. Numbers taken at merge.
- 2026-11-12: **Old files migrate at open** into one set "Set 1" (one `migrate_format` commit, not an undo step), and every screw gets its two role jobs; this changes the spec's "nothing written on open" (S1). A virtual legacy set was rejected (ADR 0059 options).
- 2026-11-12: **Lock** is `fixture_locked`, not the drawing `locked`, because a locked fixture stays selectable and listed. Enforced in the session's drag, nudge, handle and chip paths; a pure check `is_locked` in `curvyo-laser-core/src/fixture_sets.rs` so the hit and drag code ask one place.
- 2026-11-12: **Centre mark Size** is the fixture parameter `mark_mm`, since jobs carry no settings (ADR 0052 §1).
- 2026-11-12: **Centre snapping** of job fixtures queries the `0060` Objects target for centres only (circle, ellipse, polygon, box centre), with its capture, release, hysteresis and Alt rules and its marker; each centre of a grid is tried and the nearest match moves the whole grid. Set fixtures do not centre-snap.
- 2026-11-12: **Apply** copies are appended after this page's job fixtures, Output off, ticks remapped, never mirrored, no rename (ADR 0059 §6).
- 2026-11-12: **Undo labels** (document-core constants, names in `curvyo-ui-core/src/history_names.rs`, in `FREE_STEPS` as `0103`): `add_fixture_set` "Add fixture set", `rename_fixture_set`, `remove_fixture_set`, `move_fixture_set`, `set_fixture_set_on_page` / `set_fixture_set_off_page`, `add_pin`, `add_job_fixture` "Add job fixture", `lock_fixture` / `unlock_fixture`, `apply_page_fixtures` "Apply fixtures from page"; the role toggles use `set_fixture_job_output_on` / `_off` of ADR 0052 §8; size, count and spacing commits use `resize_fixture`.

## Crate placement (no new crate)

- `curvyo-laser-core`: `fixture_kind.rs` (pin), `fixture_grid.rs`, `fixture_marks.rs`, `fixture_sets.rs`, `fixture_part.rs` (`.circle`, `.marks`), `machine_profile_sets.rs` (record format 5 set layer and the format-2-to-4 read path), `machine_profile_fixtures.rs` (entries with pins and roles).
- `curvyo-document-core`: set node codec and commands (`manufacturing_fixture_set_commands.rs`), `fixture_locked`, `job_role` in `fixture_job_codec.rs`, the migration (`fixture_set_migration.rs`), Apply, validator; `CURRENT_FORMAT_VERSION` 47.
- `curvyo-editor-wasm`: `session/fixtures/` gains `sets.rs`, `placement.rs` (armed placement and centre snapping), `lock.rs`, `apply.rs`.
- `curvyo-ui-core` snapping: one query for centres only, no new target.

## Architecture review findings (2026-11-06) in this slice

- **Finding 1:** armed placement, the snap state of a placement and the Apply preview are one owned struct in `session/fixtures/placement.rs` with methods; Escape, tool change and page activation call one `end_placement`.
- **Finding 2:** the Fixtures tab, the set rows, the add bars and the Apply list are components with their own hooks (`useFixtureSets`); placement pointer events go through the existing Rust tool pipeline, not new handlers in `useEditorSession.ts`.
- **Finding 3:** `fixtures_view` gains sets (name, on page, counts), job fixtures (lock, grid, size, roles), the Apply candidates; `fixture_jobs_view` gains `role` and "not on this page". Decoders `fixturesView.ts` and `fixtureJobsView.ts` check every field. Fixtures in `curvyo-editor-wasm/tests/fixtures/fixtures_view/` and `fixture_jobs_view/`: two sets one off, an empty page, a locked pin grid, a screw grid with Centre mark on, Apply candidates, a damaged set. Rust pins the JSON; TypeScript rejects a missing and a mistyped field.

## Goldens and budgets

- Goldens: ADR 0059 §9 (project), ADR 0060 §9 (profile records).
- Budgets confirmed (criterion 42, thread CPU time, `curvyo-editor-wasm/tests/cpu_time/`): (a) hit test within 2 ms and draw list within 16 ms on 3 sets of 20 fixtures, 64 job fixtures and a 400-centre pin grid with both jobs on; (b) a drag move of a job fixture with centre snapping on a drawing of 1,000 shapes within 16 ms (the snap index of `0060`, no full scan per move; a grid tries its centres against the index); (c) Apply of 64 job fixtures within 100 ms; (d) the Fixtures tab view of 16 sets and 64 job fixtures within 100 ms. Added: opening a format-45 file with 64 fixtures through the migration within 50 ms.

## Spec changes for the product owner

- S1. Criterion 39: an old file is migrated at open into "Set 1" by one commit that is not an undo step; the file on disk is unchanged until saved; screws gain their two jobs with Output off.
- S2. Criterion 22: the parameter names of ADR 0059 §4 (`column_spacing_mm`, `row_spacing_mm`, `mark_mm`); the Centre mark Size is a fixture parameter.
- S3. Criterion 27: roles are stored (`job_role`); jobs a screw got under `0103` stay and run.
- S4. Criterion 17: limits 16 sets and 64 job fixtures confirmed; profile limits are those of ADR 0060 §2.

## Questions for the customer

None new. Both ADRs are `needs-customer` and are accepted with the story PR.
