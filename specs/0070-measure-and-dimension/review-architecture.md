# Architecture review: Measure and dimension

Architect, 2026-11-01. Branch `story/0070-measure-and-dimension`, commits `8af0bb21..c59b582b`
against `origin/main`. Measured against [ADR 0027](../../docs/adr/0027-linked-dimension-objects.md),
[`adrs.md`](adrs.md) and `CLAUDE.md` §3, §5, §6. Read-only review; nothing was built or run.

## Verdict: CHANGES REQUESTED

The structure is right and matches ADR 0027: a `dimension` leaf with slots, resolution on read in
the one pass after the text carriers (`clone_read.rs:383`), the kept value refreshed in
`commit_step` with a no-dimension fast path, a guide by kind through `effective_guide` and
`is_output_node`, `ObjectSnapshot::Dimension` arms everywhere, format 31 with a golden and four
damaged fixtures, the layout rename with no `sketch_` leftovers, one value formatter used by both
workbenches, `DimensionLook { Driving, Reference, Guide }` in `render-core`, no new crate, trait,
generic or dependency, no `ui-core` → `render-core` edge, no clock, thread or I/O in the core
modules, every production `unwrap` behind an `// invariant:` comment. Two behaviours deviate from
ADR 0027 §5, and a few smaller items are cheap to fix before the format is merged.

## Required changes

1. **A linked dimension that is moved or transformed without its referents changes more than its
   place** (ADR 0027 §5: "Moving a linked dimension alone changes only `dimension_place`").
   `curvyo-document-core/src/dimension_codec.rs:339` (`translate_dimension_meta`), `:359`
   (`write_moved`) and `dimension_model.rs:381` (`mapped_by`) move the kept points and the free
   slots of every dimension, linked or not. For a fully linked one the commit refresh then writes the
   kept points back: an arrow nudge, the Move dialog or Align gives a history step with no visible
   change. For a linked one with a `free` slot, the free point moves and the measured value
   changes. Fix: a dimension moves its free slots and kept points only when it is unlinked or
   when every referenced node is in the moved set. Otherwise it writes nothing (or only
   `dimension_place`, if the session wants a nudge to move the line). Add a test next to
   `an_unlinked_dimension_moves_its_kept_points_but_never_its_value`
   (`dimension_commands_tests.rs:329`): nudge a linked dimension alone, then assert that no step is
   written and the value is unchanged; repeat with one free slot.
2. **A copied radius or diameter dimension without its curve stays linked to the original**
   (`dimension_commands.rs:331-345`, marked Proposal). The copy resolves against the original
   circle, so it is drawn exactly on top of the original's dimension, not at the paste offset. That
   gives an invisible duplicate, which ADR 0027 §5 did not intend ("references outside it become
   `free` at their resolved position"). The format is still unmerged, so fix it now:
   **(a, recommended)** a curve slot `free_circle` (centre in mm, radius as `Length`), written for
   a curve whose node was not copied and translated with the copy, with a damage rule and a line in
   the golden; or (b) leave such a dimension out of the copy. I amend ADR 0027 §2/§5 to match the
   option you choose.
3. **The conversion and the hit test read a cache of the last drawn frame.**
   `curvyo-editor-wasm/src/session/dimension_edit.rs:97-103` builds the converted paths from
   `dimension_labels`, which `dimension_layer` (`dimension_view.rs:286`) fills while drawing. A Guide
   press with no frame drawn since an undo, a load or a selection change converts stale geometry or
   nothing (`convert_selected_dimensions` then returns `false` silently). A document write must not
   depend on render order. Fix: lay the records out at the moment of conversion
   (`self.dimension_records(&objects)` exists). Keep the cache for the overlay JSON only, or say in
   `dimension_at` that pointer hits use the last frame, which is fine for hits.
4. **"Looks unchanged" after Convert has two sources of truth** (criterion 10).
   `dimension_edit.rs:104-108` hard-codes the guide green that `curvyo-render-core/src/theme.rs:251`
   (`theme::GUIDE`) already defines. `dimension_edit.rs:133-135` repeats the arrowhead size (8 px
   by 3 px half-width) that `dimension_marks.rs:13-14` keeps private. Fix: export one helper from
   `render-core` (for example `dimension_arrow_outline(tip, back, px) -> [Point; 3]`) and use it
   both in `build_dimension_marks` and in the conversion. Take the colour from `theme::GUIDE`.
   Split `dimension_conversions` (82 lines, §5 ~60) while doing it: one function for the paths, one
   for the text.
5. **Bare `f64` in new public APIs** (§5: units are newtypes; ADR 0002 §3). The crates are
   unpublished, so this is cheap now and becomes expensive once callers multiply:
   `curvyo-ui-core/src/measure_pick.rs:27-31` (`radius: f64`, `arc: (f64, f64)` → `Length`,
   `(Angle, Angle)`), `:308-313` (`(snap, hit): (f64, f64)` → `Length`);
   `curvyo-document-core/src/dimension_read.rs:300` (`arc_circle` radius → `Length`), `:15`
   (`CIRCLE_TOLERANCE_MM` → `Length`); `dimension_model.rs:318` (`differs_from(tolerance_mm: f64)`);
   `curvyo-ui-core/src/dimension_text.rs:27,39` (`length_text(mm: f64)`, `dimension_text(value:
   f64)` → `Length` / `DimensionValue`); `measure_tool.rs:124` (`radius_readout(radius_mm: f64)`).
   The Sketch caller (`session/sketch/annotations.rs:266`) wraps its value at the call site.
6. **The golden does not cover every slot form the codec writes** (§5: golden-file tests for every
   format writer). The test doc says "every kind and every slot form", but
   `curvyo-document-core/tests/dimension_format.rs:62-153` has no `arc` curve slot, no `circle`
   under the `radius` kind, and `shape_point` only as `center` (`corner`, `vertex` and `edge_mid`
   are missing). Add them, plus the slot from item 2 (a), and regenerate `dimension_v31.curvyo`.
7. **`session/measure.rs` has 547 non-test lines** (§5, ~500). Split it, or justify it in the PR.
   Natural cut: the selection-first picks and the angle picks (`selection_picks`, `angle_of`,
   `point_of`, `free`, lines 61-180) are pure. Move them to `curvyo-ui-core/src/measure_tool.rs`,
   where they can be unit-tested without a session.

## Recommendations (not blocking)

- `dimension_read.rs:316`: `det` is twice a triangle area (mm²) but is compared with
  `DEGENERATE_MM`. A nearly straight cubic with a tiny bulge passes as an "arc" with a huge
  radius. Compare the sagitta (distance from the middle point to the chord) against a length
  tolerance instead.
- `dimension_keep.rs` / `history/commit.rs:214`: the no-dimension fast path is only correct if
  every writer that can add a dimension calls `forget_dimension_scan`. Three writers do so today
  (create, copy remap, clone version), and undo is safe because `commit_with_header` clears
  `last`. Add a debug assertion in `commit_step` that rescans when the fast path is taken in debug
  builds. Otherwise a future importer or paste goes stale silently.
- `curvyo-document-core/tests/dimension_budget.rs:16-19`: `MOVE_BUDGET` 60 ms and `SCAN_BUDGET`
  15 ms are absolute numbers that no criterion sets. Make them relative, as the frame test does
  (the commit with dimensions against the same commit without), or print them only.

## The 200-dimension frame budget (Proposal)

`curvyo-editor-wasm/tests/dimension_frame_budget.rs` asserts that the frame with 200 dimensions is
at most `max(16.7 ms, 1.5 × the frame without)`. **Acceptable under `CLAUDE.md` §3.** The customer
set the target in criterion 11 ("200 dimensions follow a drag at the frame rate of a 200-object
move"). The test turns that relative target into a number and does not invent a limit of its
own. The 1.5 factor is a measurement margin. The 16.7 ms floor only loosens the test. It never
blocks a build whose drag already runs at 60 Hz, and the measured 2.1 ms against 1.1 ms leaves a
wide margin. Conditions: keep it marked Proposal in the PR's Review guide, together with the
measured numbers. The assertion runs in release builds only and the gate runs debug, so the tester
runs `cargo test --release -p curvyo-editor-wasm --test dimension_frame_budget` for criterion 11.

## Architect follow-ups (my files, after this review)

- `adrs.md` decision 5: the Convert step is named "Convert to path" (spec criterion 10 wins over
  "Guide"). Decision 2: feature slots are resolved in `measure_pick` and not through the `0060`
  snap targets (accepted). Decision 8: a native/wasm parity test of `dimension_read` is waived.
  Its results are plain IEEE arithmetic, and the 1e-9 mm refresh tolerance absorbs last-ulp
  `atan2` differences, so no `wasm-bindgen-test` dev-dependency is added to
  `curvyo-document-core`.
- ADR 0027 §2/§5: amend for item 2.
- `docs/technical-debt.md`: the new entry lacks a blank line before `## The trace worker…` and has
  a doubled blank line above it.

## Follow-up check (2026-11-01)

Read the diff of `88f77743` (nothing built or run). **All seven required changes are resolved.**

1. Resolved. `DimensionSnapshot::stays_when_moved` (`dimension_model.rs`) and the skip in
   `transform_objects.rs`: a linked dimension moved without all its referents writes nothing.
   Tests `a_linked_dimension_nudged_alone_writes_nothing` and
   `a_linked_dimension_moved_with_its_referents_moves_its_free_slot`.
2. Resolved with option (a). `CurveSlot::FreeCircle` (codec form `["free_circle", x, y, r]`; a
   non-finite value or r <= 0 is a malformed slot), written by `relink_copied_dimensions` at the
   paste offset; test `a_copied_diameter_without_its_circle_measures_a_free_circle_at_the_offset`.
   ADR 0027 §2, §5, §8 and `adrs.md` amended to match.
3. Resolved. `dimension_conversions` lays out from `document_read()`, not `dimension_labels`.
4. Resolved. `curvyo_render_core::dimension_arrow_outline` and `GUIDE` serve both the canvas and
   the conversion; the function is split (`converted_paths`, `converted_text`, `converted_styles`).
5. Resolved. `Length`, `Angle`, `Tolerance` and `DimensionValue` in the listed signatures.
6. Resolved. The golden adds corner, vertex, edge middle, a circle under `radius`, an arc and a
   free circle; `dimension_v31.curvyo` regenerated. A bad `free_circle` falls under the existing
   "malformed slot" rule; no extra damaged fixture needed.
7. Resolved. `session/measure.rs` has 463 non-test lines; the picks moved to
   `curvyo-ui-core/src/measure_tool.rs`.

The three recommendations (sagitta test, debug rescan, relative budgets) stay open, non-blocking.
