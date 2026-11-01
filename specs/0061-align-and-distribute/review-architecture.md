# Architecture review: 0061 align and distribute

Date: 2026-11-01. Reviewer: architect. Range: `story/constraint-sketcher..42fe7ea8`
(12 commits, branch `story/0061-align-and-distribute`). Read-only review; nothing
was built or run.

## Verdict

**CHANGES REQUESTED**

What holds: no new crate, no new dependency, no `Cargo.toml`/`Cargo.lock`
change. **No format impact**: no `format_version` change, no serde type added
or changed; the key object and the page cue are session view state, never in
the document, never undoable. Dependency direction is right: pure plans in
`curvyo-ui-core`, one additive public field in `curvyo-render-core`
(`SelectDecorationInput::key`), glue in `curvyo-editor-wasm`, strings and
scalars at the wasm boundary (ADR 0001 §5). No `unwrap`/`expect`/`unsafe` in
new non-test code; the `unwrap_or` fallbacks on integer conversions are fine.
One commit per press through `Session::recording` and the existing move writer
(`commit_group`) is the right reuse.

## Required changes

1. **`curvyo-ui-core/src/align.rs` (549 lines) — §5 "modules over ~500 lines are
   split".** Split along the line already in the file:
   - `align_plan.rs` (lines 1-317): `MOVE_TOLERANCE`, `AlignBox`, `AlignAxis`,
     `AlignCommand`, `DistributeMode`, `align_translations`,
     `distribute_translations`, `distribute_on_frame_translations`, `order_on`,
     `translation`. Doc: "The pure translations of an align or distribute
     press." (~320 lines, no `Document`.)
   - `align_commit.rs` (lines 319-549): `AlignOutcome`, `AlignInput`,
     `unit_boxes`, `page_object_ids`, `translated_results`, `commit_plan`,
     `align_selection`, `align_page`, `distribute_selection`,
     `distribute_page`. Doc: "Writes an align or distribute plan as one
     commit." (~230 lines.)
   `grid_arrange.rs` imports from both; the `lib.rs` re-exports stay as they are.

2. **`curvyo-ui-core/src/align.rs:349-369` — index misalignment between units and
   boxes.** `unit_boxes` uses `filter_map`, so a unit with no leaf in `objects`
   is dropped and `boxes` gets shorter than `units`. Callers then index and zip
   by position: `align.rs:477-481` (`boxes[at]` with `at` from `selected`),
   `align.rs:445` / `401-411` (`units.zip(plan)`), and the same in
   `distribute_*` and `grid_arrange.rs:119-124`. One missing unit shifts every
   later delta onto the wrong object, silently. Fix: return
   `Vec<(SelectedUnit, AlignBox)>` (or filter the units in the same pass) and
   plan/commit over that list; add a test with a selected unit whose leaves
   are not in `objects`.

3. **`curvyo-editor-wasm/src/session/align.rs:21-47, 240` — history labels escape
   the name guard.** The 17 labels live in a `const fn` match and a literal in
   the wasm crate and reach `document.batch(label, ..)` as a variable, so
   `curvyo-ui-core/tests/history_names_guard.rs` (which collects
   `commit_with_label("..")`, `batch("..")`, `*_COMMIT_LABEL`) never sees them;
   only the hand-written unit test in `history_names.rs` covers 9 of 17.
   Fix: move the mapping into `curvyo-ui-core` as `*_COMMIT_LABEL` constants
   plus `AlignCommand::commit_label(on_page)`, `distribute_commit_label(axis,
   on_page)` and `ARRANGE_GRID_COMMIT_LABEL`, and drop the `label: &'static str`
   parameter from `align_selection`, `align_page`, `distribute_selection`,
   `distribute_page`, `arrange_selection_in_grid` (`align.rs:467, 499, 519, 541`,
   `grid_arrange.rs:116`).

4. **`curvyo-editor-wasm/src/session/align.rs:49-115` vs
   `curvyo-ui-core/src/align.rs:474-485` — the reference rule is split across
   crates (§6: domain logic in `*-core`).** `AlignReference` and the count rule
   (0 → page objects, 1 → page, 1|2 → page for distribute) live in the wasm
   session, while `align_selection` keeps its own `len() == 1 → page` branch
   that the session never reaches (it routes one unit to `align_page`). Fix:
   move `AlignReference` with `align_reference(count)` and
   `distribute_reference(count)` into `curvyo-ui-core` (tested there), delete
   the `Ordering::Equal` branch and the `page` parameter of `align_selection`
   (or let it take the resolved reference); the session only calls.

5. **Units and tolerance newtypes — §5 "units are newtypes, never bare `f64`;
   every comparison has an explicit tolerance" (ADR 0002 §2, §3).**
   - `gap_mm: Option<f64>` / `f64` in the new public core API
     (`align.rs:198, 259, 518, 540`, `grid_arrange.rs:23, 115`): take
     `curvyo_document_core::Length`; the wasm boundary keeps `f64` and converts
     in `session/align.rs`.
   - `MOVE_TOLERANCE_MM: f64` (`align.rs:18`) and its use at `align.rs:310`:
     make it a `Tolerance::from_mm(1e-6)` constant and compare with
     `as_mm()`.

6. **`curvyo-editor-wasm/src/wasm_select_bar.rs:229, 243` — an unknown code runs a
   command.** `align("lefft")` bottom-aligns and `distribute("x")` runs a
   vertical centres distribute, each writing a history step. Fix: unknown code
   returns `"nothing"` without calling the session.

7. **`curvyo-editor-wasm/src/session/align_tests.rs` (585 lines) — §5 module size.**
   Split by topic (`align_tests.rs`, `distribute_tests.rs`, `grid_tests.rs`) or
   justify the size in the PR description.

## Notes (no change required)

- `align.rs:449-453`: a failed `commit_group` reports `NothingSelected`; after
  change 2 a dedicated outcome or `Unchanged` would read more honestly.
- `grid_translations` accepts a negative gap; the frontend refuses it today.
  Fine while the session is the only caller.
- The "not enough cells" rule exists in `frontend/src/lib/gridText.ts` and in
  core (`TooFewCells`); the core check is authoritative, the TS one is field
  feedback. Acceptable.
- `adrs.md` "no render-core change" refers to the page cue only; the key mark
  itself added `SelectDecorationInput::key` (`select_box.rs:56`). Clarify the
  sentence when `adrs.md` is next touched.
