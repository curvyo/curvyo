# Plan for image optimisation, slice 1: Presets and Strength (0135)

**Status:** Done (PR #435, 2026-11-20).

Slice 1 = criteria 1, 2, 4, 5 (Denoise, Levels, Local contrast, Contours, Sharpen), 8, 9, 11 to 13,
15 to 46, 51 to 62 (spec "Acceptance criteria" intro). Not in this build: slice 2 (Crop, Resize,
Channel mix, Curves, reorder rules beyond the bands, Copy and Paste adjustments; criteria 3, 6, 7,
10, 14) and slice 3 (AI steps; 47 to 50, 63 to 66). Design: ADRs 0072 (registers, clone override,
format), 0073 (pipeline, presets, cache), 0074 (screen dithers); UX: `ux-notes.md`, drafts
`docs/design-drafts/2026-11-image-optimisation/`. Branch `story/image-optimisation`.

**Format:** `format_version` 51 (next free after `main`'s 50, `specs/README.md` table row added by
the lead with the PR). **Material record format 4**: yes, criterion 51 (Strength axis of the Image
test grid) is in slice 1's list. No new crate. No new third-party dependency; one new edge
`curvyo-image-core -> libm` (workspace pin, already locked; ADR 0073 §1).

## Affected crates/modules

- `curvyo-image-core` (Part A): `adjust_plane.rs` (grey plane; moves `GreyPlane` from laser-core),
  `adjust_blur.rs`, `adjust_tone.rs` (Levels table), `adjust_detail.rs` (Sharpen, Contours),
  `adjust_clahe.rs`, `adjust_denoise.rs`, `adjust_analysis.rs`, `adjust_run.rs` (`PixelStep`,
  `Adjusted`). Slice 2 adds Curves, Channel mix, Crop, Resize (`adjust_geometry.rs`); slice 3
  `adjust_mask.rs`.
- `curvyo-laser-core` (Part A): `image_adjust_fields.rs` (typed stored steps, ranges, defaults,
  keys), `image_preset.rs` (seven presets, Strength mapping, derivation), `image_adjust_resolve.rs`
  (effective pixel steps at one Interval, mm -> 1/16 px), `job_raster_screen.rs` (ADR 0074), the
  `Dither` variants `Halftone`, `LineScreen` and the screen fields in `operation_image_options.rs`,
  `job_raster_dither.rs`, `job_raster_sample.rs` (`GreyPyramid` of an adjusted plane).
- `curvyo-document-core` (Part B): `image_adjust_codec.rs`, image snapshot, clone source A(c),
  `CURRENT_FORMAT_VERSION` 51, goldens.
- `curvyo-ui-core` (Part B): commands, history names, Restore original, Make editable, Follow
  original, preset pick writing the operations' Dither.
- `curvyo-editor-wasm` (Part C): Style-tab group view, Before/after data, adjusted-image cache,
  job input, bindings, contract fixtures, budgets.
- `frontend` (Part D): Adjustments group, Before/after pane, clone override row, Dither strip.
- Docs (Part E): `docs/design-system.md`, `docs/requirements.md`, `docs/technical-debt.md`, spec
  status, README format row.

## Tasks

### Part A: pixel pipeline, analysis, presets, screen dithers (Opus)

- [x] A1. `GreyPlane` in image-core (`adjust_plane.rs`): over-white luminance identical to
      `over_white_grey`, edge-clamped read, 2 x 2 box halving, mip of at most 1 Mpx; laser-core
      re-exports it, `GreyPyramid::of_plane` (AC 17, 19, 39).
- [x] A2. Three-pass integer box blur from a sigma in 1/16 px (`SubPx`) (AC 17; ADR 0073 §4).
- [x] A3. Levels table: Black, White, Gamma (the `0123` convention) with `libm::pow` (AC 5, 17).
- [x] A4. Sharpen (unsharp mask, threshold) and Contours (DoG) (AC 5, 45).
- [x] A5. Local contrast (CLAHE: tiles of 2 x radius, clip from Amount, bilinear 1/256) (AC 5).
- [x] A6. Denoise (5 x 5 bilateral, integer spatial table, `libm::exp` range table), per channel
      before the first grey step (AC 5, 7 as far as Denoise).
- [x] A7. `adjust_run`: `PixelStep` list in zone order, grey conversion before the first non-Denoise
      tone step, `Adjusted::{Unchanged, Rgba, Grey}`; empty list is `Unchanged` (AC 1, 17, 18, 56).
- [x] A8. Analysis: 256-bin histogram, percentile, median, Otsu (u128 cross-multiplication, ties
      lower), Immerkær noise estimate in tenths (AC 39; tests: ramp < 1.0, ramp +-8 > 3.0, two
      greys).
- [x] A9. Typed stored steps in laser-core with ranges, defaults, integer keys (`_pct`, `_centi`,
      `_um`) and a reader from `(key, integer)` pairs that reports damage (AC 5, 8, 23).
- [x] A10. Presets as data: seven presets, Dither and its fields per preset, Strength mapping
      v0 + round-half-up((v100 - v0) S / 100) on the field step, "k x Interval" fields, clip to
      percentiles, Gamma for a median or Otsu target via `libm::log`, quantised to stored units
      (AC 25, 27, 28, 31 to 38, 40, 41).
- [x] A11. Resolve: Expert = enabled manual steps; preset = derived tone steps instead of the manual
      tone steps (geometry and mask zones kept for slices 2, 3); mm -> 1/16 px by the pixel pitch;
      one resolution per Interval (AC 28 to 30, 38, 40, 60, 61).
- [x] A12. Halftone dots and Line screen: `Dither::Halftone`, `Dither::LineScreen` (eighth and
      ninth), `screen_size_mm` / `screen_angle_deg` stored keys, cos/sin table, threshold tables,
      anchor at the item's first ring corner, independent of the scan angle; the `"halftone"`
      damage cases move to `"halftone_x"` (AC 42 to 44).
- [x] A13. Goldens: `curvyo-image-core/tests/fixtures/adjust/` (`levels_stretch`, `levels_gamma`,
      `sharpen_unsharp`, `local_contrast_clahe`, `denoise_bilateral`, `contours_dog` PNG);
      `curvyo-laser-core/tests/fixtures/adjust/` (three golden pictures, per preset the derived
      stack JSON and the dot map at Strength 0, 50, 100); `curvyo-laser-core/tests/fixtures/raster/`
      (`halftone_dots_ramp`, `line_screen_ramp` at 0 and 45 degrees, `ramp_adjusted_stack`); the
      `0123` dot maps unchanged with an empty stack (AC 17, 42, 43, 54).
- [x] A14. Budgets, thread CPU, release, 2,000 x 2,000: Levels 40 ms, Sharpen 120 ms, Local
      contrast 400 ms, Denoise 800 ms, the Photo stack 1.2 s, analysis 150 ms
      (`curvyo-image-core/tests/adjust_budgets.rs`, `--ignored`) (AC 55 for the steps).

### Part B: document model, commands, format 51 (Opus)

- [x] B1. Codec `image_adjust_codec.rs`: `image_preset`, `image_strength`, `image_adjustments`
      (checked: kind, enabled, integer fields; unknown kinds and damaged fields kept, not run,
      counted), never repaired on read (AC 4, 8, 23).
- [x] B2. `own_adjustments` marker, adjustment source A(c) on the clone chain, all-or-nothing
      copy on the first clone edit, Follow original, Unlink and repair with the effective
      registers, Duplicate copies (AC 9, 46, 61).
- [x] B3. Commands in ui-core: preset pick (writes Dither and its fields in every assigned Image
      operation; notice for n > 1; *superseded by R12, Frage 283 B*), Strength (one step per drag), step add (pipeline place),
      edit, on/off, remove, move within band, Make editable (byte-exact), Restore original
      (clone: own, empty); undo names "Edit adjustments", "Restore original", "Follow original"
      (AC 2, 24, 25 to 29, 59).
- [~] B4. Test grid recipe `picture` and `strength` axis; material record format 4 (done); cells as
      image nodes with the cell's Strength moved to slice 2 with C5 (AC 51, 52).
- [x] B5. Check lines "<n> steps in the Adjustments of <image> could not run." (AC 23; the line in
      C1).
- [x] B6. `CURRENT_FORMAT_VERSION` 51; goldens `image_adjust_v51.curvyo`,
      `image_adjust_damaged_v51.curvyo` (preset picture, manual stack, clone with own adjustments,
      clone chain, two-operation picture; each damage case); `clone_own_adjustments`,
      `restore_original` (AC 8, 9, 54, 59).

### Part C: editor session (Opus)

- [x] C1. Adjusted-image cache by content (`AdjustKey`), LRU past 256 MiB; grey pyramid keyed by
      it; job fingerprint gains it; the job reads the adjusted image per operation Interval (AC 18,
      19, 20, 21, 60, 61).
- [x] C2. Style-tab Adjustments group view JSON: header (Restore original enabled state, Before and
      after), preset strip cells with tooltips, Strength, derived rows with "From preset <name>,
      Strength <n> %", manual rows with "Preset in use", add toolbar (slice-1 steps), "Not
      engraved yet", clone override row (AC 1, 25, 26, 28, 30, 57, 59, 60).
- [x] C3. Before and after pane data: operation pick, subject line, After = the job's sweeps of the
      item; while the job is worked a live picture of the current stack at a reduced level
      (`output/pane_picture.rs`, `reduced: true`) (AC 11 to 13, 15).
- [x] C4. Bindings and contract fixtures (`tests/fixtures/adjustments_view/`,
      `tests/fixtures/before_after_view/`, decoders `frontend/src/lib/adjustmentsView.ts`,
      `beforeAfterView.ts`; the Image settings decoder knows the nine modes and the screen fields);
      budgets in `src/session/image_adjust/budget_tests.rs` (preset pick frame 50 ms, with the
      reduced Before and after 100 ms, worst tick 50 ms, 1,000 pictures: group 50 ms, pane 35 ms)
      (AC 13, 55).
- [x] C6. Resumable adjustment: `AdjustRun` in image-core works a stack a band of rows per call,
      byte-identical to the one-shot run (stepped == one-shot test, goldens unchanged); a budgeted
      job step works `ADJUST_WORK` units; worst tick of a 4 Mpx stack 15 to 31 ms (AC 13, 55).
- [x] C7. Canvas look of an adjusted picture (`image_adjust/look.rs`) (AC 15).
- [ ] C5. **Moved to slice 2** (see Deviations): the test grid's Picture pick and Strength axis with
      cells as image nodes (AC 51, 52). Material record format 4 stays as built (the `picture`
      recipe and the `strength` axis fields are read and written, never used).

### Part D: frontend (Sonnet)

- [x] D1. Style tab Adjustments group: Preset two-row strip, Strength slider (the one slider) and
      percent field, Make editable, Restore original, step list with the slice-1 bodies, add
      toolbar (AC 1, 2, 5, 25, 26, 28, 29, 57, 59).
- [x] D2. Before and after side pane (Split default, Side by side, operation pick, locked zoom and
      pan) (AC 11 to 13).
- [x] D3. Clone override row "Adjustments: Original's | Own"; canvas tag and Layers tooltip (AC 9).
- [~] D4. Nine-mode Dither strip (third row Halftone | Lines) with Cell size / Line pitch and Angle
      (AC 42 to 44); test grid Picture pick and Strength axis (AC 51).

### Part E: gate, docs, PR (lead)

- [x] E1. `docs/design-system.md` (Adjustments group, the Strength slider exception),
      `docs/requirements.md` (next R-MFG-LASER), `docs/technical-debt.md`, README format row 51,
      spec `Status: Done (PR #435)` (AC 57, 58).
- [x] E2. Full gate (`CLAUDE.md` §7), PR with the review guide.

## Validation

Golden PNGs and dot maps (`CURVYO_UPDATE_GOLDEN=1` rewrites; diffs reviewed), the `0123` dot maps
unchanged by an empty stack, property-style unit tests (white never burns, black always burns,
monotone in darkness, Make editable byte-exact), integer arithmetic only (`libm` for the 256-entry
tables) so native and wasm agree (image-core and laser-core build for `wasm32-unknown-unknown`),
thread-CPU budgets in release one test binary at a time, the full gate before the PR, the tester's
acceptance tests from the spec, UX review of Part D, architect review of the codec and format.

## Deviations (Proposal)

- Preset goldens (derived stack JSON and dot maps) live in `curvyo-laser-core/tests/fixtures/adjust/`
  instead of image-core's folder: the presets are laser-core's (ADR 0073 §1); image-core keeps the
  per-step PNGs.
- Per-step budgets run in `curvyo-image-core/tests/adjust_budgets.rs` (the crate's existing budget
  harness); the session budgets stay in `curvyo-editor-wasm/tests/cpu_time/` (Part C).
- Contours gain: `d · (100 + 15 · sensitivity) / 100` (1x to 16x) instead of ADR 0073 §4's
  `(16 + 15 · sensitivity / 100) / 16` (1x to 2x), which leaves a pen line at about grey 186, too
  light for Line art's 50 % threshold. For the architect to confirm.
- The screen anchor is the first corner of the item's first ring (the image frame's first corner;
  for a gradient leaf its outline's first point).
- Screen keys are written only when not at their defaults, so `0123` option maps stay unchanged.
- Part B: the document reads the three registers by shape only (kind text, enabled bool, integer
  fields); kinds, ranges and the unknown preset are the laser extension's (`AdjustRegisters::read`,
  `image_adjust_stored.rs`), so document-core does not depend on laser-core. Damage never refuses
  the file; `document.json` lists `adjustments` on a node that stores any register and
  `own_adjustments` on a clone only when set.
- Part B: a new step goes after the last row of the same or an earlier kind (ux-notes §3.5: a
  second step of a kind goes after the first); a row this build does not know neither moves nor
  is passed by a move; Make editable keeps such rows after the resolved steps.
- Part B: Restore original on an image writes `image_preset: "expert"` and an empty list (not the
  absent keys), as on a clone, so the two read the same.
- B4 in Part B is the recipe and the record only (`PictureSource`, `Param::Strength` as an axis that
  is never shared, record format 4, golden `format4_picture_test.json`); the grid cells as image
  nodes with the cell's Strength move to Part C with the grid's place pipeline.
- B5 in Part B is the count only (`AdjustRegisters::broken_steps` with the damaged registers); the
  check line needs the registers on the job's image leaf and moves to C1.
- The goldens `clone_own_adjustments` and `restore_original` are asserted on the registers the
  job reads (`curvyo-laser-core/tests/image_adjust_goldens.rs`); their dot maps follow with C1.
- Radius fields are the Gaussian sigma (Sharpen, Local contrast's tile is 2 x radius, Contours' first
  sigma is width / 2).
- Part C: the pyramid cache lives in laser-core (`image_adjust_cache.rs`, shared with the session
  through `PlanCache::pictures`) instead of beside the session's working-image cache, so the
  job's ticks adjust a picture: a budgeted step with at least `ADJUST_DOTS` (200,000) dots works
  `ADJUST_WORK` (12 million units, about 15 to 30 ms natively) of one adjustment and ends; an edit
  frame (`FIRST_READ_DOTS`) never adjusts. The steps are resumable by row bands (`AdjustRun`:
  box passes, Levels, Denoise rows, CLAHE tile rows then blend rows, Sharpen and Contours
  combine); at most two half-done runs are kept (a Strength drag leaves one per value). The
  first tick of a run also reads the picture (the grey conversion of 4 Mpx, a few ms).
- Part C: `BlockNumbers::picture_interval` carries the operation's Interval, so the Operations
  view's coarse preview derives the preset at the real Interval and runs it on the reduced level
  its dot step fits (`reduced_level`, `WorkingImage::reduced_over_white`).
- Part C: the check line is "<n> steps in the Adjustments of <image> could not run." (criterion
  23's text), one per picture node, `Cause::BrokenAdjustments`.
- Part C: a held Strength (`hold_adjust_strength`) is shown on the picture's own leaf in the job
  and the views; clones that follow it update on release.
- Part C, criterion 13: the Before and after pane's After is the job's dots while the job is
  current; while it is worked, the item alone is dithered on the smallest power-of-two multiple
  of the Interval with at most 100,000 dots (Proposal), its picture adjusted at the reduced level
  that grid fits, `reduced: true` (the field replaces `stale`). The coarse grid covers the whole
  item, not only its visible part in the pane (criterion 13's wording), since the pane's zoom is
  the frontend's (Part D).
- Part C, criterion 15: an image with non-empty adjustments draws its look: the adjusted picture
  at the first halving with at most 1 Mpx (Proposal), opaque over white (transparency is not
  kept, as the job burns it), resolved at the Interval of its first Image operation on the active
  laser page (0.10 mm without one) with a held Strength; made in the host's ticks (the
  `job_pending` / `advance_job` bindings now also cover looks: `Session::ticks_pending`,
  `advance_tick`), named by its content key and kept while drawn; until then the image shows its
  last look, else the picture as imported. The look has its own pyramid cache, so a laser page's
  plans never drop a look half made.
- Part C, budgets: the 1,000-picture pane budget is 35 ms (20 ms measured); the lib tests check
  every cached drawing read against a fresh read (about 6 ms each on that page), so the pane and
  the settled job now read the drawing and the page input once (the held or pending job's input
  while it is current).
- Part C: the contract fixtures are written by an in-crate test
  (`src/session/image_adjust/contract_tests.rs`, `CURVYO_UPDATE_GOLDEN=1`) because the states need
  the session's selection and document; the fixtures live in `tests/fixtures/` as the others.
- Part C: the `0123` frontend tests that pinned seven Dither modes and refused `"halftone"` now
  expect the nine modes of criteria 42 to 44 (an unknown mode is still refused).
- **C5 moved to slice 2** (criteria 51, 52: the Image test grid's Picture pick and Strength axis,
  cells as image nodes with the cell's Strength). It needs a new grid-cell kind in
  `curvyo-document-core`, a public API and format change, so the architect decides first:
  1. what a cell is: an image node of its own (the picked picture's asset by hash, the cell's
     placement, its own registers with the cell's Strength), or a linked clone of the picked
     picture with own adjustments (ADR 0072's clone override, only Strength differing);
  2. where the cell's Strength lives: in the cell's own `image_strength` register, or only in the
     recipe's `Param::Strength` axis, derived when the job reads the cell (no register per cell);
  3. what the Picture pick stores: a reference to the image node (follows its edits, needs a
     repair when it is deleted) or a copy of its asset hash and registers at the pick;
  4. the grid-part API (`GridPart` gains an image part or a new part kind), its codec, the
     `format_version` bump and goldens, and how the place pipeline (ui-core, editor-wasm) creates
     the cells in one undo step;
  5. how the job reads the cells (raster leaves of the grid's Image block at the cell's Interval
     and Strength) and how the material record's axis fields (format 4, already read and written)
     map back when a cell is picked as the result.
  Material record format 4 stays as built; its `picture` recipe and `strength` axis are unused
  until then.
  **Decided by the architect (2026-11-20, ADR 0072 §6, ADR 0073 "Slice 2 decisions"):** (a) a
  cell is its own image node tagged `grid_cell`, not a clone; (b) Strength is the recipe axis,
  regenerated into each cell's `image_strength` as derived data; (c) the Picture pick copies the
  asset and the registers into the cells and the recipe keeps `picture: "selected"`; (d) the
  slice-2 format bump is shared with Crop, no new `GridPart`, and the job reads the cells through
  the per-operation Interval path; (e) a picked cell writes power, speed, passes and interval
  only, and a preset pick on the source picture does not change the grid's Dither. Slice 1 keeps
  the recipe keys `picture` and `strength` and material record format 4.

## Follow-up after the tester's report (2026-11-20)

- [x] F1. Local contrast: the clip and the excess spread in exact fractions (a flat picture keeps
      its grey on small tiles); ADR 0073 §4 amended, goldens regenerated (AC 5, 54).
- [x] F2. The canvas look asks `is_adjusted()`: Restore original draws the original (AC 15, 59).
- [x] F3. Preset and Dither strips (fixed rows): the arrows, Home and End move and select on the
      key itself, not on Radix's later focus (AC 25).
- [x] F4. A material edit in the same whole second as the last one is a step (marker moves on).
- [x] F5. Levels: the histogram of the rows above (256 bars, a mip of at most 256 Kpx, kept by
      content) and Stretch (0.5 % and 99.5 %, `stretch_points`, one step) (AC 5, 16).
- [x] F6. Memory (AC 56): `step_bytes` / `peak_bytes` (2 x in colour, 1.5 x grey), a run within
      `RUN_BYTES` (160 MiB) skips the step that does not fit and those after it; the group's alert
      line names the last step that ran (P17).
- [x] F7. A drag of a step row is one step (`move_adjust_step_to`) (AC 2).
- [x] F8. Docs: R-MFG-LASER-008, the format 51 row, the spec status, debt lines (AC 58). The
      drag ghost stays the name chip (debt line).

## Follow-up after the architect and UX reviews (2026-11-20)

- [x] R1 (A1). The adjust goldens (image-core steps, laser-core presets and screen dot maps) are
      embedded with `include_bytes!` and run on wasm32 in the CI golden job (AC 17, 54).
- [x] R2 (A2, A3, A4). One byte limit for pictures and half-done runs (288 MiB job cache, 32 MiB
      look cache, 320 MiB in all); the look queue keeps the looks drawn last, newest first; a look
      keeps the picture's alpha (AC 13, 15, 56).
- [x] R3 (A5, A7). ADR 0072/0073 amended, slice-2 decisions a to e (C5 above), notes on ADR 0016,
      0017, 0045, 0057, 0058, 0068, 0069; index rows 0054 and 0073; spec criteria 51, 52 slice 2.
- [x] R4 (A6). The first tick of a preset picture reads the analysis within 50 ms (25 ms measured,
      `budget_tests::c13_the_first_tick_...`); no split needed.
- [x] R5 (nits). An unreadable entry with no plain form is kept by the next edit; test hooks
      `#[doc(hidden)]`; `reduced_level(Length, Length)`; the screen pick names its Dither; Home or
      End on the chosen cell does not press it.
- [x] R6 (U1 to U6, U8, U10, U11). Editor accent; Own copies the original's adjustments whole;
      Before and after keeps its size, scales smoothly below 100 %, 100 % is the document's,
      1.25 a notch, a message for a picture in no Image operation; tooltips wait 400 ms again;
      Strength field 112 px; step rows one Tab stop with a focus ring and the focus over a move;
      tag and Layers line "own adjustments"; fixed strip rows never wrap; the clone's Adjustments
      row stands with Placement; no lone hairline.
- [x] R7 (U7). Line art: Contours 5 x Interval, Sensitivity 60 to 100 %, Local contrast 0 to 30 %;
      Black and white without Local contrast (spec tables amended, `tests/preset_quality.rs`).
- [x] R8 (U9). The Levels graph is one Tab stop: Left and Right pick a handle, Up and Down move it,
      a drag writes once at release (AC 5). The fields do not follow the drag live (debt below).
- [x] R9 (question 12, Proposal). "Dither set for N pictures." when a preset pick changes the
      Dither of an operation that runs other pictures. *Superseded by R12 (Frage 283 B): no notice.*

## Rework: Dither on the picture (Frage 283 B, 2026-11-20)

Design: ADR 0072 "Amendment: the Dither on the picture" (A1 to A9), notes on ADR 0069, 0073, 0074;
spec criteria 4, 8, 22, 27, 59, 67. Format 51 and material record 4 change in place (no bump, no
migration). Lowest power follows Frage 284 (default A: on the picture; with B drop
`lowest_power_permille` and the `Sweep` field of R13). Numbered from R10 because R1 to R9 above are
taken. Order R10 to R15 on one branch, one agent building at a time; R16 with each task.

Customer answer 2026-11-20, Frage 284 B: Lowest power stays on the operation (`lowest_power_pct`
next to Power). `image_dither` is `{mode, threshold_pct, screen_size_um, screen_angle_deg}`; a
picture in Grey reads its operation's Lowest power; the Grey checks keep targeting the operation;
a preset that picks Grey writes no Lowest power; no `Sweep` field (R13). R11 was built before R10
because laser-core reads document-core's `StoredDither`.

- [x] R10 (laser-core, Opus). New `image_dither.rs` ("the dither a picture burns with"):
      `DitherSettings` (mode, threshold %, lowest power in tenths, screen size in 10 µm steps, angle),
      `DEFAULT` (Floyd-Steinberg, 50, 0, 800 µm, 45), read from the stored shape with each A5 damage
      case, the stored map written whole, `with(ImageEdit)` for the five dither edits only (the
      others refused); `ImageSettings::with_dither` / `dither_settings`; `AdjustRegisters::dither`
      read with the other registers, damage counted once (AC 8, 23, 67).
- [x] R11 (document-core, Opus). Codec register `image_dither` (shape read, `AdjustDamage::dither`,
      written whole), `ADJUST_KEYS` four, `ImageAdjustments::dither`, `AdjustmentsWrite::dither`,
      Restore original removes it on an image and a clone, the enabled test `is_restorable()`
      (`is_adjusted()` unchanged), `edit_image_adjustments(target, &write)` without the operation
      writes (public API), `document.json` lists it. Tests: clone first edit, Follow original,
      Unlink, repair and Duplicate move all four; two peers (dither vs stack, pick vs dither).
      Goldens regenerated: `image_adjust_v51.curvyo` (the preset picture holds `jarvis`, the
      two-operation picture holds the halftone 1.60 mm / 30 the coarse operation held, the own
      clone its own dither, the coarse operation keeps its keys over one gradient rectangle);
      `image_adjust_damaged_v51.curvyo` (+ not a map, unknown mode `halftone_x`, a missing
      field, `screen_size_um` 6000, 805) (AC 8, 9, 24, 54, 59, 67).
- [x] R12 (ui-core, Opus). `pick_preset(preset, current: DitherSettings, interval_um) ->
      AdjustmentsWrite` (Preset and dither, no `OptionsWrite`); `set_dither(current, ImageEdit)`;
      `dither_notice` and `AssignedOperation::options` removed; Make editable untouched
      (AC 27, 29, 67).
- [x] R13 (laser-core job, Opus). `raster_settings(numbers, leaf)` per leaf (picture: the
      operation's tone fields with its own dither; gradient: the operation's), `RasterSettings`
      owns its `ImageSettings`; `raster_dot_map*` follow the job's rule (goldens put the mode on the
      picture leaf, bytes unchanged); `Sweep::lowest_tenths` read by the GRBL writer (Frage 284 A);
      `GreyNeedsM4` and `LowestPowerAbovePower` per leaf with targets (operation or pictures); the
      numbers line names one dither or the "several" text. Tests: two pictures with two presets in
      one operation give each the dot map it gives alone; a picture in two operations burns one
      dither; a dither change re-dithers without a new adjusted image (cache count); a gradient
      leaf keeps the operation's dither (AC 19, 42 to 44, 60, 67).
- [x] R14 (editor-wasm, Opus). Commands `set_picture_dither(mode)`,
      `edit_picture_dither_field(field, value)` on the selected picture (a following clone's first
      edit copies the four registers); preset pick without operation writes or notice; one
      `DitherView` (mode, modes, mode field, screen fields) shared by the Adjustments view
      (`dither`) and the Image row (`dither: null` unless the operation holds a gradient leaf);
      Before and after subject names the picture's dither; check lines select the pictures;
      bindings; contract fixtures `adjustments_view/*`, `before_after_view/open_two_operations.json`,
      `operations_view/options_image.json` and a new `options_image_gradient.json`; the dither
      pick on the picture within the 50 ms edit frame (`budget_tests.rs`) (AC 1, 11, 22, 27, 55, 67).
- [x] R15 (frontend, Sonnet, after the UX notes). The Dither strip moved into the Adjustments group
      under the Preset strip (the component out of `ImageSettingsGroup.tsx`, reused);
      `ImageSettingsGroup` draws its strip only when `dither` is present; decoders
      `adjustmentsView.ts`, `operationSettingsView.ts`; `useAdjustments.ts` reads no notice after a
      preset pick (Make editable keeps its notice) (AC 1, 22, 44, 67).
- [x] R16 (tests that pin the operation's dither on a picture; each task's agent fixes its own
      crate's, the tester rewrites the acceptance files from the amended spec).
      Preset pick writes the operation (invert to "writes the picture, no operation, no notice"):
      `curvyo-ui-core/src/image_adjust_edit.rs` `a_preset_pick_writes_the_dither_of_every_operation`
      and its `dither_notice` asserts; `curvyo-document-core/tests/image_adjust_format.rs`
      `a_preset_pick_writes_the_operations_dither_in_the_same_step` and `golden()`;
      `curvyo-laser-core/tests/image_adjust_goldens.rs`
      `a_picture_in_two_operations_derives_its_steps_at_each_interval`;
      `curvyo-editor-wasm/src/session/image_adjust/tests.rs`
      `c27_a_preset_pick_is_one_step_and_the_job_burns_the_adjusted_picture`,
      `c59_restore_original_is_the_imported_picture`, `c29_make_editable_burns_the_same_dots`;
      `curvyo-editor-wasm/tests/acceptance_0135_tester.rs`
      `c27_a_pick_writes_the_dither_in_every_operation_in_one_step_and_says_so`,
      `c1_c54_c59_restore_original_gives_the_0123_dots_byte_for_byte`,
      `c59_restore_original_keeps_strength_dither_and_the_tone_fields` (now: dither back to
      Floyd-Steinberg, operation untouched), `c29_make_editable_keeps_the_dots_byte_for_byte_for_every_preset`.
      A picture burning the operation's dither (move the mode to the picture, or test the operation's
      dither on a gradient leaf): `acceptance_0135_tester.rs` `screen_case` (`c42_*`, `c43_*`),
      `c42_c43_white_burns_nothing_black_burns_every_dot_at_any_angle_and_scan_angle_is_ignored`,
      `c44_the_dither_picker_lists_nine_modes_halftone_eighth_lines_ninth`;
      `acceptance_0135_tester_budget.rs` `c55_screen_modes_cost_about_an_ordered_job`;
      `acceptance_0123_tester.rs` `c11_settings_stay_stored_while_the_group_is_hidden`,
      `c11_the_group_shows_only_with_a_raster_item_in_the_documented_order`,
      `c12_every_image_edit_is_one_undo_step_edit_settings_and_undo_restores`, `c33_grey_on_m3_blocks`,
      `c41_the_format_50_golden_opens_engraves_and_round_trips`; `acceptance_0123_tester_budget.rs`
      `c12_c37d_a_dither_pick_and_its_undo_in_the_app_frame_take_50_ms`,
      `c37b_the_writer_of_a_worst_case_dither_takes_250_ms`;
      `src/session/output/tests/raster_tests.rs` `criterion_11_...`, `criterion_32_lowest_power_above_power_warns_and_focuses_lowest_power`;
      `tests/output_view_contract.rs` `state_raster_checks`; `tests/operations_view_fixtures.rs`
      `state_options_image`. New tests from criterion 67: two presets in one operation, a following
      clone changes with its original's dither, Restore original gives the `0123` dots with a
      default operation.
- [x] R17 (lead, done by the UX review 2026-11-20). Spec status, README format-51 row note
      ("`image_dither`", already in the row), design-system lines for the picture's Dither strip, the
      Image body (Lowest power while Grey) and the Strength row; debt: none new.

## Debt (justified, architect review A8)

Functions over ~60 lines kept whole in this slice, each one reading or building one value in
one pass, where a split would pass most locals along:

- `curvyo-editor-wasm/src/session/image_adjust/before_after.rs` `before_after_view` (90): the
  pane's view in one read of the job and the picture.
- `curvyo-editor-wasm/src/session/image_adjust/view.rs` `adjustments_view` (77): the group's view
  from one read of the picture, its operations and its resolved steps.
- `curvyo-laser-core/src/image_preset.rs` `derive` (69): the preset tables as data, one arm per
  preset.
- `curvyo-laser-core/src/image_adjust_cache.rs` `advance` (72): one run started, resumed or
  finished under one lock discipline.
- `curvyo-document-core/src/clone_picture.rs` `picture_clone` (73) and `clone_read.rs` `walk`
  (66): the chain walk of ADR 0015 §6 with the adjustment source beside the style source.
- `curvyo-document-core/src/lib.rs` is 504 lines of re-exports only.
- The Levels graph writes at a drag's release; the fields follow live only once the session has
  a held Levels edit like the held Strength (slice 2, with Curves).
