# Architecture review: Pattern fill

Architect, 2026-11-02, branch `story/0071-pattern-fill` at `608cf210` (`git log origin/main..HEAD`,
12 commits), against [ADR 0029](../../docs/adr/0029-live-pattern-fill.md) and `adrs.md`.
Read-only review; nothing was built or run.

## Verdict

**CHANGES REQUESTED.** The model, format 32, kernel placement and determinism are sound. Five
fixes are needed before the PR: two concern the cache and the panel, one the SVG output order, one
a Leaf forest worst case that must be settled **before merge** (after merge its geometry is
frozen by saved files), and one is a §5 size rule.

## Required changes (implementer)

1. **Cache entries are evicted between the two draw calls.**
   `curvyo-editor-wasm/src/session/draw.rs:47-53` calls `drawn` (and `PatternCache::expand`) twice
   when the trace fill splits the list. `curvyo-ui-core/src/pattern_cache.rs:118-133`
   `retain`s only the ids of the current call, so the second call drops every entry of the first
   part and each frame regenerates those patterns (ADR 0029 §6: a move is a hit; entries are
   dropped only for nodes absent from the read). Rule: ADR 0029 §6. Fix: expand once over all
   `objects` and split the expanded list at the mapped index, or move the eviction into its own
   `PatternCache::retain_only(ids)` called once per draw. Add a session test with a trace fill
   in place that shows a second draw is a hit.

2. **Convert availability generates the whole pattern on every panel read, uncached.**
   `curvyo-ui-core/src/pattern_output.rs:127-134`, called from
   `curvyo-editor-wasm/src/session/pattern.rs:215`. Up to the hard limit this is ~90 ms per
   read on top of the draw. Rule: ADR 0029 §6 (the memo serves the session's reads). Fix: decide
   availability from the `PatternState` that `pattern_state_lines` already reads from the cache
   (a selected path or primitive with `Pieces(n)`, `n > 0`); drop `pattern_convert_available`'s
   own generation.

3. **SVG writes the pieces above the outline; the canvas draws them below it.**
   `curvyo-ui-core/src/svg_export_parts.rs:176` (`vec![region, pieces]`). ADR 0029 §7 puts the
   pieces beneath the stroke, and on a laser or plotter document order is job order: the
   engraving must come before the outline is cut, or the part drops out first. Rule: ADR 0029
   §7 and criterion 25 (export equals what is drawn). Fix: `vec![pieces, region]`; regenerate
   `curvyo-ui-core/tests/fixtures/pattern/pattern_export.svg` and review the diff. (ADR §8's
   wording is corrected by the architect, see below.)

4. **Leaf forest's worst case is unmeasured and unbounded by the limit.**
   `curvyo-geometry-core/src/pattern/leaves.rs:70` throws up to `30 × wanted` candidates. At
   Density 100 % random circles saturate near half the area, so every remaining throw fails:
   near the hard limit that is ~13 million throws with nine hash lookups each, synchronously on
   the UI thread. The estimate gates `wanted`, not the work. Rule: ADR 0029 §10 (limits set by
   measurement). Fix: add a case to `tests/pattern_cpu_time.rs` with Density 100 % and a box
   just under `PATTERN_MAX_PIECES`; if it exceeds the 400 ms note-limit budget, end the
   throwing after a fixed number of consecutive rejections (a constant, frozen by the goldens).
   Settle this **before merge**: any later change to the throwing changes the geometry of saved
   files (ADR 0029 §5).

5. **`pattern_model.rs` exceeds the module and function size rule.**
   `curvyo-document-core/src/pattern_model.rs` has 605 non-test lines;
   `PatternGenerator::from_values` (`:472`) is 75 lines. Rule: `CLAUDE.md` §5 (~500 lines per
   module, ~60 per function, split or justify). Fix: move the value-table conversions
   (`values`, `get`, `with`, `from_values`, `switched_to`, `degrees`, `radians`) into
   `pattern_values.rs` and build each variant in a small per-kind helper. Also split
   `curvyo-ui-core/src/pattern_cache.rs` (its doc says "memo ... and the canvas drawing"):
   `expand`, `any_pattern` and `pieces_path` go to `pattern_draw.rs` (§5, one responsibility).

## Judged and accepted

- **Cache in `curvyo-ui-core`** instead of `curvyo-editor-wasm`: correct. ADR 0011 makes
  editor-wasm a facade "with no logic of its own" and gives it no geometry-core edge; ui-core
  already depends on geometry-core, builds for wasm32, and the memo has no clock, I/O or
  threads. ADR 0029 §6 was wrong on the location.
- **`pattern_pieces(outlines, fill, line_width, tolerance)`** plus `pattern_pieces_of` and
  `PatternFrame` instead of the ADR's `(region, frame, fill, tolerance, limits)`: simpler; the
  limits as module constants are enough while no caller varies them; `line_width` is needed by
  Shapes strips. The cache and the outputs run the same `to_local → pattern_pieces →
  to_document`, so a hit and a recompute give the same bits.
- **Determinism**: generators use `+ - * /`, `sqrt`, `floor/ceil/round` and `libm::sin/cos`;
  clipping and inset are `i_overlay` integer mode on the 0.001 mm grid; SplitMix64 matches the
  reference (test against `splitmix64(0)`); the Leaf forest `HashMap` is lookup only, never
  iterated. Goldens run natively and on wasm32 in CI (`ci.yml`). Accepted.
- **`libm` direct dependency**: justified, MIT OR Apache-2.0, already in the lock. Note:
  `default-features = false` does not hold in the build, `i_float` turns on `libm/arch`
  (`cargo tree -e features -i libm`); harmless, `arch` only swaps correctly rounded operations
  and the wasm golden shows equal output. No change needed.
- **Format 32**: bump with rationale, damage rules, lenient read, `pattern_v32.curvyo` golden,
  keys in `ALL_STYLE_KEYS`, tester pins moved by version only. Accepted.
- **Margin and clipping** through `offset_polyline::inset_region` and `i_overlay`, Shapes strips
  through the integer stroke, inside pieces kept exact by a conservative cell map. Accepted.
- **Convert to paths**: one commit, group above the shape, fill off, geometry from
  `pattern_pieces_of`; linked clones skipped (Proposal). Accepted.
- **History names**: labels are written into the history and are frozen once shipped; the set is
  complete for every `StyleEdit` pattern edit. Accepted.
- **render-core** reads only `Fill::paints_solid`; no new dependency. Accepted.
- **Plan deviations** (pattern kept under Solid/None, Shapes strips, Lines without stroke, the
  generator interpretations, over-limit behaviour): inside the ADR and the customer's decisions.
  The debts recorded in `docs/technical-debt.md` (refusal regeneration, previews, scaled clone
  `<use>`, Strokes as paths) are accepted as debts.
- Not required: `lines.rs:105` compares the Amplitude parameter to exact 0 (a parameter value,
  criterion 11, not a geometric comparison); the synthetic piece path reuses the shape's id
  (only guide styling reads it).

## Architect follow-ups (separate docs commit)

- ADR 0029 §3: the actual signature and the `line_width` input; §6: the memo lives in
  `curvyo-ui-core`; §8: pieces are written before the region path.
- `adrs.md` decision 1: the cache location.
