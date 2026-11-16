# ADRs for fixture sets and pins

Status: Proposed (architect, 2026-11-12). Builds third, after `0111` and `0112` (needs the profile record format 4 of ADR 0056 and the Materials tab before Fixtures). File shapes: [ADR 0059](../../docs/adr/0059-fixture-sets-job-fixtures-and-pins-in-the-project-file.md) (project file, format 47) and [ADR 0060](../../docs/adr/0060-fixture-sets-in-the-machine-profile-record.md) (profile record format 5), both `needs-customer`, accepted with the story PR. Decisions marked **(Proposal)** are the architect's and vetoable at the demo.

- [ADR 0059](../../docs/adr/0059-fixture-sets-job-fixtures-and-pins-in-the-project-file.md): set nodes with `set_on_page`, job fixtures as direct children with `fixture_locked`, the on-the-bed rule, the `pin` kind and pin and screw grids, `job_role` jobs with `.marks` and `.circle`, Apply fixtures from page, no migration (old flat fixtures read as job fixtures), limits.
- [ADR 0060](../../docs/adr/0060-fixture-sets-in-the-machine-profile-record.md): `fixture_sets` in record format 5, pins and roles in entries, formats 1 to 4 skipped as older, copy into a page keeping job fixtures, Differs.
- [ADR 0049](../../docs/adr/0049-laser-fixtures-in-the-project-file.md): fixture node, slots, matrix, damage per field; §1, §2, §7 amended.
- [ADR 0050](../../docs/adr/0050-laser-fixture-set-in-machine-profile.md) and [ADR 0052](../../docs/adr/0052-laser-fixture-jobs-in-the-project-file.md): record, copy, jobs and part keys; amended by ADR 0059 §5 and ADR 0060.
- [ADR 0039](../../docs/adr/0039-machine-profile-copy-in-the-project-file.md) §4, §5: Pick, Reset and Differs now act on sets only.
- [ADR 0042](../../docs/adr/0042-page-kinds.md) §9: `migrate_format` reused.
- [ADR 0032](../../docs/adr/0032-workbench-model-and-registry.md): the skip predicate covers sets, fixtures (depth 2) and jobs (depth 3).
- `0060` snapping (Objects target and its index): reused for centre snapping; no ADR change.

## Feature-local decisions

- 2026-11-12: **Format.** Project `format_version` 47, profile record format 5. Numbers taken at merge.
- ~~2026-11-12: **Old files migrate at open** into one set "Set 1".~~ **2026-11-15, lead (Proposal, vetoable; Curvyo is not deployed): no legacy conversion.** A file of format 46 or lower opens unchanged; its direct fixture children are job fixtures by ADR 0059 §2, no set is made, screws get no role jobs (toggles hidden), nothing is written; profile records of formats 1 to 4 are skipped as "older format" (ADR 0059 §7, ADR 0060 §3, spec criterion 39).
- 2026-11-12: **Lock** is `fixture_locked`, not the drawing `locked`, because a locked fixture stays selectable and listed. Enforced in the session's drag, nudge, handle and chip paths; a pure check `is_locked` in `curvyo-laser-core/src/fixture_sets.rs` so the hit and drag code ask one place.
- 2026-11-12: **Centre mark Size** is the fixture parameter `mark_mm`, since jobs carry no settings (ADR 0052 §1).
- 2026-11-12: **Centre snapping** of job fixtures queries the `0060` Objects target for centres only (circle, ellipse, polygon, box centre), with its capture, release, hysteresis and Alt rules and its marker; each centre of a grid is tried and the nearest match moves the whole grid. Set fixtures do not centre-snap.
- 2026-11-12: **Apply** copies are appended after this page's job fixtures, Output off, ticks remapped, never mirrored, no rename (ADR 0059 §6).
- 2026-11-12: **Undo labels** (document-core constants, names in `curvyo-ui-core/src/history_names.rs`, in `FREE_STEPS` as `0103`): `add_fixture_set` "Add fixture set", `rename_fixture_set`, `remove_fixture_set`, `move_fixture_set`, `set_fixture_set_on_page` / `set_fixture_set_off_page`, `add_pin`, `add_job_fixture` "Add job fixture", `lock_fixture` / `unlock_fixture`, `apply_page_fixtures` "Apply fixtures from page"; the role toggles use `set_fixture_job_output_on` / `_off` of ADR 0052 §8; size, count and spacing commits use `resize_fixture`.

- 2026-11-16, architect review: **Apply counts against the 64 job fixtures** (ADR 0059 §8 amended, Proposal): refused as a whole, nothing copied, the page is not offered in the Apply list. **Order** comes from the document-core snapshot only (ADR 0059 §3 amended); the unused `listing_order`, `ListedSet`, `Listed`, `JobState` and `diameter_circles` of `curvyo-laser-core` go (or are used by the session, not both). **Idle frames**: the fixture layer is rebuilt and the whole frame re-uploaded every animation frame; recorded in `docs/technical-debt.md` by the lead, fixed in its own `fix/` PR (frame generation skip first, then a cached fixture list), not in this story.

- 2026-11-16, architect, after the customer tried the build ("machine sets cannot be locked yet, and they are bound to the page; they should be persistent across all pages and instances"): **sets live in the profile** (ADR 0059 amended second, ADR 0060 §1, §5a to §7, §9). Proposal unless marked:
  - **Home:** the profile record (format 5, per user, synced by ADR 0062) is the only home of a set; the Fixtures tab edits it from any page. A page keeps a **mirror** subtree (the existing set nodes) with the record's ids (`set_uid`, `fixture_uid`, `job_uid`) and `mfg_fixture_sets_modified`; the mirror is the project's snapshot, read-only where the profile is not on this computer (or is Generic laser, damaged, older, newer). Option "opaque snapshot plus live record read" rejected: every fixture command and session path twice.
  - **Page choices** stay on the mirror nodes and never reach the record: set On this page, fixture On the bed (Frage 211 B is per page), eye, job Output. The record loses `active`.
  - **Lock** at the profile: `locked` per set and per fixture in the record, mirrored as `set_locked` / `fixture_locked`; a fixture is locked by its own or its set's lock; behaviour as a job fixture's (no handles, no drag, no nudge, no knobs or chips; trash, eye, check, jobs still work). A locked set's add bar is hidden; rename, order, On this page and its trash still work. Labels `lock_fixture_set` "Lock fixture set", `unlock_fixture_set` "Unlock fixture set"; set fixtures reuse `lock_fixture` / `unlock_fixture`.
  - **Edits and undo:** each content edit is one document step (existing labels) that commits on the mirror, aligns the other pages of the project on that profile, and writes the record; the ADR 0055 §9 journal (machine records added, one 16 MiB budget, limits (a) to (f)) takes the record back on Ctrl+Z. A drag writes the record once, at release.
  - **Alignment:** `align_fixture_sets` (document-core, new module `manufacturing_fixture_align.rs`) matches by uid, keeps node ids and choices, creates new sets **off** on this page (Frage 218 default A) and new fixtures On the bed; runs guarded (record `modified` above the page's and contents differ) at open, library reload and page activation as a reserved non-step commit `align_fixture_sets`, and unguarded inside an edit step for the project's other pages. The host reloads open pages' profile records on window focus and on a browser `storage` event.
  - **Profile actions:** Differs, Update profile and Reset from profile act on the machine fields only; Save as profile writes the mirror with its ids (this is how a carried project becomes editable); Pick builds a fresh mirror, every set on (criterion 9 kept); Generic laser deletes the mirror, no notice, and its Fixtures tab shows "+ Set" hidden with a hint.
  - **Limits** unchanged: 16 sets and 64 set fixtures per profile (ADR 0060 §2), 64 job fixtures per page.

## Crate placement (no new crate)

- `curvyo-laser-core`: `fixture_kind.rs` (pin), `fixture_grid.rs`, `fixture_marks.rs`, `fixture_sets.rs`, `fixture_part.rs` (`.circle`, `.marks`), `machine_profile_sets.rs` (record format 5 set layer; formats 1 to 4 are skipped as older, 2026-11-15), `machine_profile_fixtures.rs` (entries with pins and roles).
- `curvyo-document-core`: set node codec and commands (`manufacturing_fixture_set_commands.rs`), `fixture_locked`, `job_role` in `fixture_job_codec.rs`, Apply (no migration module, 2026-11-15), validator; `CURRENT_FORMAT_VERSION` 47; 2026-11-16: uids, `set_locked`, `mfg_fixture_sets_modified`, `manufacturing_fixture_align.rs` (alignment).
- `curvyo-editor-wasm` 2026-11-16: the record write and the journal entry of a set edit, the read-only gate and the alignment triggers in `session/fixtures/profile_sets.rs` ("keeps a page's sets in step with its profile record"), beside the material journal it shares.
- `curvyo-editor-wasm` 2026-11-16 (architect review): the journal of a library step's record part (material and machine records, one 16 MiB budget) is `session/library_journal.rs` ("the record part of this session's library steps"), shared by `session/materials/` and `session/fixtures/profile_sets.rs`; the notice name helper `short_name` lives there too.
- `curvyo-editor-wasm`: `session/fixtures/` gains `sets.rs`, `placement.rs` (armed placement and centre snapping), `lock.rs`, `apply.rs`.
- `curvyo-ui-core` snapping: one query for centres only, no new target.

## Architecture review findings (2026-11-06) in this slice

- **Finding 1:** armed placement, the snap state of a placement and the Apply preview are one owned struct in `session/fixtures/placement.rs` with methods; Escape, tool change and page activation call one `end_placement`.
- **Finding 2:** the Fixtures tab, the set rows, the add bars and the Apply list are components with their own hooks (`useFixtureSets`); placement pointer events go through the existing Rust tool pipeline, not new handlers in `useEditorSession.ts`.
- **Finding 3:** `fixtures_view` gains sets (name, on page, counts), job fixtures (lock, grid, size, roles), the Apply candidates; `fixture_jobs_view` gains `role` and "not on this page". Decoders `fixturesView.ts` and `fixtureJobsView.ts` check every field. Fixtures in `curvyo-editor-wasm/tests/fixtures/fixtures_view/` and `fixture_jobs_view/`: two sets one off, an empty page, a locked pin grid, a screw grid with Centre mark on, Apply candidates, a damaged set. Rust pins the JSON; TypeScript rejects a missing and a mistyped field.

## Goldens and budgets

- Goldens: ADR 0059 §9 (project), ADR 0060 §9 (profile records).
- Budgets confirmed (criterion 42, thread CPU time, `curvyo-editor-wasm/tests/cpu_time/`): (a) hit test within 2 ms and draw list within 16 ms on 3 sets of 20 fixtures, 64 job fixtures and a 400-centre pin grid with both jobs on; (b) a drag move of a job fixture with centre snapping on a drawing of 1,000 shapes within 16 ms (the snap index of `0060`, no full scan per move; a grid tries its centres against the index); (c) Apply of 64 job fixtures within 100 ms; (d) the Fixtures tab view of 16 sets and 64 job fixtures within 100 ms. (The migration budget is dropped with the migration, 2026-11-15.) Added 2026-11-16 (Proposal): (e) a set-fixture field commit on a project with 4 laser pages on one profile of 3 sets x 20 fixtures, including the record text built for the host and the alignment of the 3 other pages, within 50 ms; (f) the guarded alignment at a library reload of 8 laser pages from a record with one changed fixture within 100 ms, and the no-change check of the same 8 pages within 10 ms. Both thread CPU time in `curvyo-editor-wasm/tests/cpu_time/`.
- Goldens added 2026-11-16: ADR 0059 §9 and ADR 0060 §9 as amended (uids and locks in the v47 goldens; `fixture_sets_align_v47.curvyo` with `format5_align_newer.json`; three new damaged record goldens; every `format5_*.json` regenerated).

## Spec changes for the product owner

- S1. Criterion 39 (superseded 2026-11-15 by the lead's no-legacy decision): an old file opens unchanged with its fixtures as job fixtures; older profile records are skipped.
- S2. Criterion 22: the parameter names of ADR 0059 §4 (`column_spacing_mm`, `row_spacing_mm`, `mark_mm`); the Centre mark Size is a fixture parameter.
- S3. Criterion 27: roles are stored (`job_role`); jobs a screw got under `0103` stay and run.
- S4. Criterion 17: limits 16 sets and 64 job fixtures confirmed; profile limits are those of ADR 0060 §2.

- S5 (2026-11-16, sets in the profile): criteria 3, 7 (covers 4 to 7), 8, 9, 10, 17, 32, 38, 40, 42 amended; new 44 to 47; Out of scope loses "Locking set fixtures" and "a live link from a set to the profile". Done in `specification.md` by the architect at the lead's request; the PO reviews the wording.

## Questions for the customer

None new until 2026-11-16. Both ADRs are `needs-customer` and are accepted with the story PR.

Frage 218 (SET-NEW-ELSEWHERE): A new set on the other pages
A set added on one page appears on every other page and project of the same machine. Is it on there?
- A. Off there ("not on this page"); the maker switches it on where needed; a finished project never changes its bed by itself.
- B. On there, as on the page where it was added.
- C. Ask on the next opening of the other page.
Recommendation A (nothing appears unexpectedly on the bed of a finished project; its jobs run only with Output anyway). Default: A.
(Number 218: 215 to 217 are taken by `0116`, ADR 0062.)
