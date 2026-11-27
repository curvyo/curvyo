# ADRs for material test picks

Status: Proposed (architect, 2026-11-26), read against `main` at `11920627`. The file shapes are decided in [ADR 0093](../../docs/adr/0093-material-test-picks-and-round-engrave-cells.md): material record format 5, the link key `pick`, project format next free at merge, and round Engrave cells. ADR 0093 is `needs-customer` (persistence, document model) and is accepted with the story PR. Fragen 312 to 316 are taken at default A. Decisions marked **(Proposal)** are the architect's and vetoable at the demo.

- [ADR 0093](../../docs/adr/0093-material-test-picks-and-round-engrave-cells.md) (new): `PickKind`; record format 5 (test marks `second_best`, `fastest`, `deepest`; `picks` per laser entry beside `sets`); mark and unmark rules; a source naming a deleted test reads as valid; record text limit 640 KiB; `op_settings.material.pick` (absent = Best); which set runs; format 66 (next free at merge); `Shape::Ellipse` cells for Engrave; picks on the canvas as view data.
- [ADR 0058](../../docs/adr/0058-material-records-shared-across-lasers.md) §2, §6: amended by ADR 0093 decisions 2 to 6 (shape, source rule, damage, limit).
- [ADR 0054](../../docs/adr/0054-laser-settings-and-test-grids-in-the-project-file.md) §1, §2: amended by ADR 0093 decisions 7 and 8 (the link's `pick`, `material_run` by pick). Refresh, overrides and multi-row apply are unchanged.
- [ADR 0057](../../docs/adr/0057-test-grid-object-with-derived-operations.md) §1: amended by ADR 0093 decision 10 (an Engrave cell is an ellipse primitive). Regeneration, ids and Lasered are unchanged.
- [ADR 0062](../../docs/adr/0062-synced-per-user-libraries.md) §5: unchanged. Picks ride the record text, newest `modified` wins and the loser is kept as a conflict copy. §6 (as amended by `0116` P2): the synced material text cap follows the 640 KiB limit.
- [ADR 0069](../../docs/adr/0069-raster-rows-output-and-settings.md) §14: Image tests get picks like the other kinds, and Image pick sets need `interval_mm`.
- [ADR 0072](../../docs/adr/0072-image-adjustments-on-the-image-object.md) §6: a picked cell of a picture grid writes power, speed, passes and interval only, for every pick kind.
- [ADR 0064](../../docs/adr/0064-laser-job-builder-and-output-writers.md): its flattening tolerance of 0.0095 mm bounds the circle's useful precision (decision 10).
- `0101` Engrave fill: hatches the ellipse primitive with Crosshatch as it hatches a rect. No output change.

## Feature-local decisions

- 2026-11-26: **Undo keys** (constants in document-core, words in `curvyo-ui-core/src/history_names.rs`, the words are the UX notes'): `judge_test` stays for every pick kind. The spec says "Judge test", but today's word is "Choose best cell", so UX settles one word for all four kinds. New keys are `unmark_pick` ("Unmark pick") and `use_pick` ("Use pick"). The canvas overlay is no step.
- 2026-11-26: **One query for availability:** `Material::pick_set(profile_id, kind, pick) -> Option<&Set>` feeds the dimmed buttons (criterion 15), the live, stored and missing line (criterion 13) and `material_run`. There is no second lookup in the session **(Proposal)**.
- 2026-11-26: **Test detail and Materials tab** read picks through `materials_view`. Each test gains `picks` (kind to cell). Each laser's set gains `other_picks` (kind, numbers, source test id). The view is the checked decoder `materialsView.ts`, with fixtures for a test with four kinds on three cells and a pick whose test was deleted.
- 2026-11-26: **The operation block** reads `pick` and the per-kind availability through the existing operation settings view (checked decoder). Derived grid rows send no pick buttons (criterion 16).
- 2026-11-26: **No history entry** for Second best, Fastest and Deepest. Best appends its `set` entry as today **(Proposal)**.

## Crate placement (no new crate, no trait, no dependency)

- `curvyo-laser-core`:
  - `material.rs`: `PickKind`.
  - `material_record.rs`: `TestRecord` gets the four marks, as one `Marks` struct in place of `best`. `LaserSets` gains `picks` and `pick_set`.
  - `material_record_file.rs` (472 lines today) stays the record's envelope, sets and tests. A new module `material_record_picks.rs` ("the stored form of a record's marks and pick sets") holds the read, write and damage of decisions 2 and 5, so neither file passes ~500 lines (CLAUDE.md §5).
  - `material_record_edits.rs`: `judge` takes a pick kind, plus a new `unmark`.
  - `operation_settings.rs` (462 lines): `read_link` and the writer gain `pick`. If the file passes ~500 lines, the link's read and write move to `operation_link.rs` ("the stored material link of an operation").
  - `operation_settings_rule.rs`: `material_run` by pick.
  - `operation_settings_edit.rs`: the "Use pick" write.
  - `test_cell_fill.rs`: `cell_shape`.
  - `library_merge.rs`: unchanged.
- `curvyo-document-core`:
  - `CURRENT_FORMAT_VERSION` with its doc paragraph;
  - `library_list.rs` `MAX_MATERIAL_TEXT_BYTES` set to 640 KiB;
  - the undo key constants;
  - the goldens of ADR 0093 decision 9.

  It reads no pick itself: `op_settings` stays plain values (ADR 0033 §3).
- `curvyo-ui-core`: history names.
- `curvyo-editor-wasm`:
  - `session/materials/` (judge with kind, unmark, view fields);
  - `session/test_grid/items.rs` (calls `cell_shape`);
  - the test grid view's marks for the canvas overlay;
  - the operation settings block (pick, availability, multi-row apply with its at most 70-character notice).
- `curvyo-app`: `materials.rs` formats its over-limit text from `MAX_RECORD_TEXT`, with no "512 KiB" literal.
- `frontend`: `materialHostWrites.ts` `MAX_RECORD_TEXT` set to 640 KiB, its text from the constant; pick bar, pick buttons in the operation block, folded pick lines, canvas overlay, and the two decoders. No core logic.

## Goldens and budgets

- `curvyo-laser-core/tests/fixtures/materials/` (ADR 0093 decisions 2, 4, 5):
  - `format5_picks.json`: four kinds on three cells, one cell with two kinds, a pick whose test was deleted, an Engrave pick with interval, an Image pick. It reads and writes back byte for byte.
  - `format5_max.json`: the old maximum plus every mark and every pick set. It must read and stay at or under 640 KiB; measured 545,191 bytes by the architect.
  - one damaged file per damage rule of decision 5;
  - `format4_with_picks.json`: picks in format 4, read as damaged;
  - `source_wrong_test.json`.
  - `source_no_test.json` turns into a valid record, and its test is renamed to say so.
  - `newer_format.json` moves to format 6.
- Project and items goldens: ADR 0093 decisions 9 and 10.
- Tolerance tests: the circle stays within 0.03 % of the radius of the exact circle, area within 0.1 %, at Cell sizes 3, 10 and 50 mm. The bounding box equals the square within 1e-9 mm.
- Budgets (spec criterion 24, thread CPU time, `curvyo-editor-wasm/tests/cpu_time/`): placing a 12 x 12 Engrave grid within 50 ms, a bar edit within 30 ms, marking a pick within 30 ms. The existing budget for loading 200 records keeps its 200 ms with format-5 records.

## Spec changes for the product owner

- S1. Criterion 10 and `0112` criterion 25: a set whose test was deleted reads "Test <id>". Today that record reads as **damaged** (ADR 0093 Context). The fix is ADR 0093 decision 4, best shipped ahead as a `fix/` PR for format 4.
- S2. Criterion 12: "the record converges as any material edit does (ADR 0062 §5): the newer `modified` wins, the other version is kept as a conflict copy with its notice". The words "picks of different kinds or lasers merge" go, because the existing mechanism merges whole records. Field-wise merge is ADR 0004 §8 and out of scope.
- S3. Criterion 20: "every point lies within 0.03 % of the radius of the exact circle (at most 0.007 mm at Cell size 50 mm; the job flattens at 0.0095 mm), the area within 0.1 %". Four kappa arcs cannot meet 0.001 mm above Cell size 7.3 mm. The circle is the existing ellipse primitive, not a new path.
- S4. Criterion 3: a pick set is removed, or Best turns Typed, only when the material's set came from this test. A set taken from another test of the same laser and kind stays.
- S5. Criteria 2 and 13: the undo words. "Judge test" (spec) and "Choose best cell" (today) differ, and UX picks one.
- S6. Criterion 13 or the Out of scope section: Engrave + Outline takes the same pick for its outline (Score pick, else Engrave pick), and Image tests have picks. Both are Proposal.
- S7. Criterion 11: material record **format 5**, and the record limit rises to 640 KiB. Criterion 17: `op_settings.material.pick`, absent = Best, project format next free at merge (66 today).

## Questions for the customer

None new. ADR 0093 is `needs-customer` and is accepted with the story PR. If the product owner wants to keep the promise that picks of different kinds merge field by field (S2), that is a sync decision for the customer and a story of its own (ADR 0004 §8).
