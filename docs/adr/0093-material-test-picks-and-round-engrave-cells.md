# ADR 0093: Material test picks: four pick kinds on the test and in the material, the pick kind in the operation's material link, round Engrave cells; material record format 5, project format next free

**Status:** Proposed 2026-11-26 by the architect, `needs-customer` (persistence: material record format 5; document model: a new key in the operation's material link and a project `format_version`), accepted with the story PR as ADR 0057 and 0058 were. Built on the spec's defaults Frage 312 A (four kinds, at most one cell per kind per test), 313 A (each linked operation says which pick it follows, default Best, live), 314 A (a hatched circle of the cell's diameter, old grids keep squares until edited), 315 A (Cut and Score stay squares), 316 A (view-only frames on the selected grid). Amends [ADR 0058](0058-material-records-shared-across-lasers.md) §2 and §6 (record shape, set source rule, record text limit), [ADR 0054](0054-laser-settings-and-test-grids-in-the-project-file.md) §1 and §2 (the link gains `pick`, which set runs), [ADR 0057](0057-test-grid-object-with-derived-operations.md) §1 (the Engrave cell's shape) and [ADR 0062](0062-synced-per-user-libraries.md) decision 6 as amended by `0116` P2 (the synced material text cap); notes go on each. [ADR 0062](0062-synced-per-user-libraries.md) decision 5 (merge) is unchanged. Spec: [`0146`](../../specs/0146-material-test-picks/specification.md). Feature notes: [`0146` adrs.md](../../specs/0146-material-test-picks/adrs.md). Choices marked (Proposal) are the architect's, vetoable at the demo.

**In plain words (for the customer).** A test can carry four marks: Best, Second best, Fastest, Deepest. Each mark copies its cell's numbers into the material for that laser, as Best does today, and the numbers stay when the test is deleted. An operation linked to a material remembers which of the four it follows; an operation that says nothing follows Best, so every project you have opens and runs as before. Engrave test cells become circles the next time you change such a grid. Two devices that mark picks offline merge as any material edit does today: the newer one wins and the other is kept as a copy.

## Context

Today a test holds one `best` cell, and judging writes the laser's set of the test's kind (ADR 0058 §2). An operation's link (`op_settings.material`, ADR 0054 §1) is `{ id, name, outline_from?, modified }` and always takes that set. Reading the code at `11920627` shows four facts the spec does not state:

- **A set whose test is deleted reads as damaged.** `material_record_file.rs` refuses a `{ "test": id }` source that names no fitting test of the record, and `delete_test` leaves the set's source as it is. So after "Delete test" on a judged test, the next read skips the whole material as damaged. `0112` criterion 25 and `0146` criterion 10 both assume the set survives and reads "Test <id>".
- **Sync merges whole records.** `library_merge` decides the winner of a whole record by `modified` and keeps the loser as a conflict copy (ADR 0062 §5). It has no per-field merge, so "picks of different kinds merge" (spec criterion 12) is not what the existing mechanism does.
- **The maximal record no longer fits.** `format2_max.json` is 460,087 bytes against the 512 KiB limit. With three more marks on each of its 200 tests and three pick sets for each of the four kinds of its 16 lasers it is 545,191 bytes (measured with the canonical two-space form).
- **Four kappa arcs miss criterion 20's tolerance.** The cubic quarter circle with kappa 0.5522847498 lies up to 2.7253e-4 x r off the circle: 0.0068 mm at Cell size 50 mm. 0.001 mm holds only up to Cell size 7.3 mm. The area is 0.028 % too large.

## Options

**Where the pick sets live in the material**

- **A. A `picks` map per laser entry beside `sets`, keyed by kind, then by pick kind; Best stays the `sets` entry (chosen).** `sets` keeps its shape, so a Best set, a typed set and every reader of `set_for` are unchanged. A pick set has the set's shape.
- **B. Every kind in `sets` becomes a map of four pick kinds.** One shape for all four, but every existing set moves one level down. The typed set, whose source is "typed", has no pick kind. Rejected.
- **C. Only the marks on the test; pick numbers computed from the recipe when needed.** No new numbers, but a pick dies with its test, which breaks criterion 10. Rejected.

**Where the pick kind lives in the project**

- **A. A key `pick` inside the atomic `material` map of `op_settings` (chosen).** A merge can never pair one peer's material with the other peer's pick. With `pick` absent meaning Best, an operation that follows Best stays byte-identical to format 65.
- **B. Its own register on the operation node.** Two peers changing the material and the pick at once could keep one's material with the other's pick, for example a Deepest pick on a material that has none. Rejected.

**Sync of picks marked on two devices (spec criterion 12)**

- **A. The record text rides ADR 0062 unchanged: newest `modified` wins, the loser is a conflict copy with its notice (chosen).** No new mechanism, as the spec asks, and no edit is lost.
- **B. Merge records field by field (picks of different kinds or lasers both kept).** This is the union merge of ADR 0004 §8, which ADR 0062 §5 names as future work. It is a persistence and sync decision for the customer and is larger than this story. Rejected here.

**Format numbers**

- **A. Material record format 5 and project format next free at merge, 66 on today's main (chosen).** Criterion 11 asks that an older build leaves a record with picks untouched as newer, which only a bump gives. The `material` map refuses unknown keys, so a build of 65 would read every pick-linked operation as Damaged settings. Version 61 took a bump for exactly that reason. In-place changes so far happened only inside the story that had introduced the number (ADR 0069 record format 3, ADR 0072 format 51 and record format 4), and 4 and 64 belong to merged stories.
- **B. Material format 4 and project format 64 in place.** An older build would call a record with picks damaged, not newer, and a format-64 build of last week would show Damaged settings without a version notice. Rejected.

**The round cell**

- **A. The existing `Shape::Ellipse` primitive with rx = ry = Cell size / 2 (chosen).** The ellipse outline, the Engrave fill and Crosshatch (`0101`), the job builder and SVG export already handle it. No new geometry or output code.
- **B. A path of four or eight Bezier arcs written by the grid code.** A second circle builder beside `primitive_outline::ellipse_outline`. Rejected.

## Decision

1. **Pick kinds.** One enum `PickKind { Best, SecondBest, Fastest, Deepest }` in `curvyo-laser-core/src/material.rs`, stored names `best`, `second_best`, `fastest`, `deepest`, in that order. The order is also the order for display and the frame colour (spec criterion 6). Picks exist for every test kind: Cut, Score, Engrave and Image (Image has its set since ADR 0069 §14) (Proposal).
2. **Record format 5** (amends ADR 0058 §2). Two additions, everything else as format 4:
   - **A test** gains the optional keys `second_best`, `fastest`, `deepest` after `best`, each `{ "column", "row" }` like `best`. They are absent until marked, and one cell may carry several.
   - **A laser entry** gains `picks` after `sets`, absent when empty: `{ "<kind>": { "<pick>": <set> } }`, where kinds are written in the order `cut`, `score`, `engrave`, `image` and picks in the order `second_best`, `fastest`, `deepest`. Best is never a key here, because Best is the entry in `sets`. A pick set has the shape of a set (`power_pct`, `speed_mm_min`, `passes`, `interval_mm` required on `engrave` and `image` and refused on the others). Its `source` is always `{ "test": "<id>" }`; a pick set is never typed and never from history (spec criterion 9).
   - An entry with neither sets nor picks goes, as an entry without sets does today. History: marking a pick other than Best appends no entry, and Best appends its `set` entry as today (Proposal; the history stays the log of the sets operations take by default, and its 500 entries are not spent on picks).
   ```json
   "lasers": [ { "machine": { "id": "6f1c…", "name": "Diode 20 W" },
     "sets": { "cut": { "power_pct": 60.0, "speed_mm_min": 1950, "passes": 1, "source": { "test": "7K2QX" } } },
     "picks": { "cut": { "second_best": { "power_pct": 50.0, "speed_mm_min": 1800, "passes": 1, "source": { "test": "7K2QX" } },
                         "fastest": { "power_pct": 80.0, "speed_mm_min": 3000, "passes": 1, "source": { "test": "7K2QX" } } } } } ],
   "tests": [ { "id": "7K2QX", …, "best": { "column": 2, "row": 1 }, "second_best": { "column": 1, "row": 1 },
                "fastest": { "column": 3, "row": 3 }, "note": "", … } ]
   ```
3. **Marking and unmarking** (`material_record_edits.rs`; spec criteria 2, 3):
   - `judge(test, cell, pick, stamp)`: the mark goes on the test. For Best the laser's `sets` entry of the test's kind is written as today. For the others it is the `picks` entry: the two axis values over `fixed`, power, speed, passes and interval only, source the test. Core answers `Unchanged` for the same cell; the session sends a tap on a cell that already carries the pressed kind to `unmark` (spec criterion 3).
   - `unmark(test, pick, stamp)` removes the test's mark. A pick set is removed, and Best's set turns to source `"typed"` with its numbers kept, **only when that set's source names this test**. A set taken from another test of the same laser and kind stays as it is (Proposal).
   - Marks are per test, sets are per laser and kind, and the newest mark writes the set, as Best does today: marking Fastest on test B moves the Fastest set to B and leaves test A's mark in place.
   - `put_test` drops every mark outside the new recipe, as it drops `best` today. Copying a grid (new id) copies no marks.
4. **The set source rule** (amends ADR 0058 §6, for formats 2 to 5 alike):
   - A `{ "test": id }` source that names **no test of this record** is a deleted or moved test. It reads as valid and is shown "Test <id>" (spec criterion 10, `0112` criterion 25).
   - A source that names a **present** test of another machine or kind is damage, as today.
   - Loosening a reader writes nothing, so it applies to the existing formats without a bump. The golden `source_no_test.json` becomes a valid record, and a new `source_wrong_test.json` (a present test of another kind) carries the damage case.
5. **Reading format 5 and older** (spec criteria 11, 23). Formats 2 to 4 read as today, with no marks besides `best` and no picks. They are never converted and never written on read. In a record below format 5, `picks` or one of the three test keys is damage (as an Image set in format 2, ADR 0069 §14). In format 5 these are damage, each with a golden:
   - a mark outside the recipe's counts;
   - a pick key other than the three, and `best` inside `picks`;
   - a kind key that is not `cut`, `score`, `engrave` or `image`;
   - a pick set whose present source test has another kind or machine;
   - a missing `interval_mm` on an Engrave or Image pick, or an extra one on Cut or Score;
   - a source that is not a test;
   - an empty `picks` map or an empty kind map.

   A record above format 5 is skipped as newer and never written.
6. **Record text limit** 512 KiB to **640 KiB** (Proposal; the next 128 KiB step above the measured 545,191 bytes, with 110 KB to spare). Three constants rise together: `MAX_RECORD_TEXT` in `material_record_file.rs`, its copy `MAX_MATERIAL_TEXT_BYTES` in `curvyo-document-core/src/library_list.rs` (`0116` P2), and `MAX_RECORD_TEXT` in `frontend/src/lib/materialHostWrites.ts`. The refusal text "it is over 512 KiB" in `curvyo-app/src/materials.rs` and `materialHostWrites.ts` is then formatted from the constant, not written as a literal.
7. **The operation's link** (amends ADR 0054 §1). `op_settings.material` may hold `pick`: `"second_best"`, `"fastest"` or `"deepest"`. When it is absent the link means Best. `"best"` is never written, and any other value, `"best"` included, is a bad link (Damaged settings, saved back unchanged, never repaired). `MaterialLink` gains `pick: PickKind`.
8. **Which numbers run** (amends ADR 0054 §2). `material_run` takes the main set from `Material::pick_set(profile, kind, pick)`. Best is `set_for`, and the others are the `picks` entry. A missing pick set is the "set missing" state, with the kind's word in the line ("Material has no Fastest set for this laser, using stored numbers."). The outline of Engrave + Outline follows the same pick: the Score set of that pick, else the Engrave set of that pick (the main numbers), which is today's fallback applied per pick (Proposal). The refresh guard, the overrides and the multi-row apply are unchanged:
   - Choosing a pick kind, also the one already set, writes the link with that kind, the numbers as last seen and no overrides in one step "Use pick" (spec criteria 13, 14).
   - The grid's labels operation and the derived cell operations keep their links and numbers. Their links never carry `pick` (spec criterion 16).
9. **Project `format_version` next free at merge, 66 on today's main.** A build of 65 reads a link with `pick` as Damaged settings, so this build's files are refused there by version. A file of 65 or lower opens unchanged, and every link in it follows Best. No migration. Goldens in `curvyo-document-core/tests/fixtures/`:
   - `material_pick_v66.curvyo`: four operations linked to Best, Second best, Fastest and Deepest, one with an override, plus a test grid whose labels row carries no pick;
   - `material_pick_damaged_v66.curvyo`: `pick` `"best"`, `"cleanest"` and `3`. Each reads Damaged settings and saves back unchanged.
10. **Round Engrave cells** (amends ADR 0057 §1; spec criteria 18 to 22). `curvyo-laser-core/src/test_cell_fill.rs` becomes "what a test grid's cell looks like". Beside `cell_style` it gains `cell_shape(test, min, side) -> Shape`:
    - an Engrave recipe gets `Shape::Ellipse` centred on the cell's square with rx = ry = side / 2, no arc;
    - every other kind gets the `Rect` of today.

    `session/test_grid/items.rs` calls it in place of the literal `Rect`. The circle's bounding box is the square, so `grid_origin`, which reads the first cell's bounds, places a regenerated grid where it was. Regeneration already deletes and recreates the cell items (`replace_grid_items`), so a format-46-to-64 grid keeps its squares until its next committed bar change. Nothing changes on open, and nothing changes the test id.
    - The circle is the ellipse outline of `primitive_outline.rs`: four smooth anchors with KAPPA. Its tolerance, written as a test, is at most 0.03 % of the radius off the exact circle (at most 0.007 mm at Cell size 50 mm, below the job's flattening of 0.0095 mm, ADR 0064) and an area within 0.1 % (measured +0.028 %).
    - No project format change: an ellipse primitive under a `grid_cell` tag is already valid in every format since 46.
    - Golden `curvyo-editor-wasm/tests/fixtures/test_grid_items/engrave_3x3_10mm.json`: the nine cell shapes (centres and radii), the unchanged label origins and texts.
11. **Picks on the canvas** (spec criterion 8) are view data only. The session's `test_grid_view` gains the selected grid's marks (cell to kinds) read from its test in the loaded store. The frontend draws them in the canvas overlay layer. Nothing is written to the document, the job or an export.

## Consequences

- A material with picks opens in an older build as "newer" and is never touched there. In a device group, the older device skips the incoming record (ADR 0062 `SkipIncoming`), which is the upgrade-together rule of `0116` P3.
- Two devices marking picks offline produce one winner and one conflict copy, as any concurrent material edit does today. The spec's criterion 12 is reworded to say so (feature notes, S2).
- After this story, deleting a judged test no longer makes its material unreadable. Format 4 has the same defect today (Context), and it gets its own `fix/` PR with decision 4 alone, ahead of this story (Proposal to the lead).
- Published keys: test `second_best`, `fastest`, `deepest`; laser entry `picks`; link `material.pick`. Renaming one is a format change.
- Budgets of spec criterion 24 hold by construction: an ellipse primitive costs what a rect costs in the item writer, and marking is one record edit. They are measured with thread CPU time in `curvyo-editor-wasm/tests/cpu_time/`.
- Rejected and why: pick sets nested inside `sets` (B, moves every existing set and has no pick for a typed set), marks only on the test (C, a pick dies with its test), the pick in its own register (B, a merge pairs a material with a foreign pick), field-wise record merge (B, ADR 0004 §8, the customer's sync decision and a story of its own), in-place formats (B, wrong signal in older builds), a hand-built Bezier circle (B, a second circle builder).
