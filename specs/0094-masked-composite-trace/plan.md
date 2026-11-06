# Plan for Masked and composite trace, slice 1 (one mask region per trace)

Implementer, 2026-11-04, on `story/masked-trace` (from `main` `d1275c72`). Built against
`specification.md` slice 1 (criteria 1 to 17), `adrs.md` decisions 1 to 6, ADR 0023 §5, ADR 0003,
`ux-notes.md` §1, §2, §7 and §10. Everything marked **(Proposal)** is mine and vetoable at the demo.

## Affected crates/modules

- **`curvyo-ui-core`**
  - `trace_region.rs` (new): the mask region (kernel union of the mask paths at 0.005 mm) and the
    clip of traced outlines to it (box test, point test, `boolean` only for crossing outlines).
  - `trace_region_raster.rs` (new): the region as a pixel mask (scanline, nonzero), the Mask view's
    whitening and the region-only analysis (Threshold, background, analysis report).
  - `trace_transfer.rs`, `trace_text_wire.rs`, `trace_text_colour.rs`: `*_in` variants of the jobs
    that take an optional region; the existing functions delegate with `None` (bit-identical).
  - `trace_text.rs`: `TextRegion::centre` (the line's box centre in mm) for the centre rule.
  - `trace_region.rs` also exposes `mask_operand`, the Boolean commands' collector, for the
    session.
- **`curvyo-editor-wasm`**
  - `session/trace.rs`: the selection is one image plus mask paths; a refusal for text, sketch or
    a second image with mask paths; the region cached per mask; Inside / Outside as tool state.
  - `session/trace_region.rs` (new): the region of the selected mask paths, its JSON for the
    worker, the outside-the-image test and the Overlay's 12 % wash; tests in
    `session/trace_region_tests.rs`.
  - `trace_glue.rs`, `wasm_trace.rs`: the region read from the settings JSON (`"mask"`), the
    region-only defaults (`trace_region_defaults`), the view's mask fields.
- **`frontend`**: Mask item after the View pill, the mode in the session, the region in the
  settings and in the defaults request, the muted rows, the status segment.
- Docs: `docs/adr/0023-...` §5 dated note, `docs/design-system.md` Trace rows,
  `docs/tracing-images.md` "Mask", `docs/technical-debt.md`, spec criterion 2 (lead decision).

## Decisions (Proposal)

- The region crosses the wasm boundary inside the settings JSON (`"mask": {"outside", "region"}`,
  flat numbers in document mm), not as a new slice parameter: no wasm function changes its arity
  (frontend tests call them directly), and the result and mask caches key on the settings string,
  so a new mask or mode invalidates them by construction. Without a mask the settings string is
  unchanged.
- Region-only analysis: the auto Threshold reads the histogram of the region's pixels only; the
  background reads the region's border pixels (pixels of the region next to a pixel outside it or
  the picture's edge), so a mask over the right half of M2 sees the paper; the analysis report
  reads the region's box with the pixels outside made transparent.
- Text objects are exempt from criterion 2 (lead decision): a recognised line is kept whole when
  its box centre lies in the kept area, else it is dropped (its pixels were in-painted).
- A crossing traced shape comes back as a polyline over its whole outline (kernel flattening at
  0.005 mm); tracked under "Boolean results are polylines" in `docs/technical-debt.md`.
- Architect review (2026-11-04): a kernel refusal of the union is its own failure ("0 paths: the
  mask paths could not be combined."), not "outside the image"; a kernel refusal while clipping
  keeps the crossing outlines whole when the region keeps their first point (no exact edge for
  them) instead of dropping them. A linked clone is a mask path only when it resolves to outlines.
- Criterion 11 is checked on a synthetic picture with synthetic OCR lines (the poster tests need
  the OCR weights and are `#[ignore]`d).

## Tasks

1. [x] ui-core tests first: fixtures M1, M2 and the clip cases (criteria 2 to 8, 15).
2. [x] ui-core: `trace_region.rs` clip and `*_in` job variants; no-mask path delegates
   (criteria 1, 2 to 8, 15).
3. [x] ui-core: raster, Mask view whitening, region-only defaults (criteria 9, 10).
4. [x] ui-core: text and shape passes with a region (criterion 11).
5. [x] session: selection with mask paths, refusal row, region, outside-image test, wash, Create
   paths leaves the mask paths (criteria 1, 5, 6, 7, 12, 13, 14, 17).
6. [x] wasm surface and worker: settings `"mask"`, `trace_region_defaults`, view fields
   (criteria 1, 7, 9, 10).
7. [x] frontend: Mask item, mode, muted rows, status segment, keys (criteria 1, 7, 13, 14, 17).
8. [x] criterion 16: CPU time of the clip on 2000 x 2000 px with a 500-node mask (reported).
9. [x] docs: ADR 0023 §5 note, design-system row, tracing-images, technical debt, criterion 2.
10. [ ] full gate; browser files of 0091, 0088 Part B, 0025, 0089 tidy-up.

## Validation

- `cargo nextest run -p curvyo-ui-core -p curvyo-editor-wasm` during work; the existing golden
  JSONs (`curvyo-editor-wasm/tests/fixtures/*_native.json`) stay unchanged.
- New tests: `curvyo-ui-core/tests/acceptance_0094_mask.rs` (M1, M2, clip, raster, text),
  session tests in `curvyo-editor-wasm/src/session/trace_region_tests.rs`, the criterion 16
  harness `curvyo-editor-wasm/tests/acceptance_0094_cpu.rs` (ignored, run in release).
- One existing assertion changed by the spec: `session/trace_tests.rs` expected an image plus a
  rectangle to be "not an image"; with 0094 that is a masked trace, so the case now selects the
  rectangle alone.
- Determinism: the same job twice gives the same numbers (criterion 15).

## Slice 1b, pick mask paths (criteria 34 to 43)

Implementer, 2026-11-05, on `story/mask-pick-button` (from `main` after PR #271). Built against
`specification.md` slice 1b (criteria 34 to 43, Frage 130 C) and `ux-notes.md` §11.

### Affected crates/modules

- **`curvyo-editor-wasm`**
  - `session/trace_mask_pick.rs` (new): the pick mode as Trace tool state (armed, the refused
    flag), the armed press (accept, refuse or ignore), Clear mask, the hover candidate. `trace.rs`
    is already above 500 lines, so the mode gets its own module.
  - `session/trace.rs`: the Shift+click toggle (`selection.toggle_all(units.leaves_with(hit))`)
    becomes `trace_select_hit` and is the one add/remove route for Shift+click and the pick; the
    pick accepts a hit whose unit (the object, or its group's leaves) is all path-like by
    `trace_region::is_mask_path`, the rule `trace_images` applies to the Select route.
  - `session/mod.rs` (press, hover), `select_cursor.rs` (`"pick-mask"`), `select_view.rs` (the
    candidate's `--hover-box` while armed), `trace_pick.rs` (the pipette and the pick exclude
    each other).
  - `wasm_trace.rs`: `set_trace_mask_pick`, `trace_mask_pick`, `trace_mask_pick_refused`,
    `clear_trace_mask`.
  - Tests: `session/trace_mask_pick_tests.rs`.
- **`frontend`**: `TraceBar.tsx` (the Mask group: pick button, Inside / Outside, Clear mask),
  `useTrace.ts` (the mode, Escape order, refused row), `traceApi.ts`, `traceText.ts` (strings),
  `cursors.ts` (`"pick-mask"`); unit tests `tests/traceMaskPick.test.ts`; Chromium
  `tests/acceptance_0094_pick_browser.test.ts`.
- Docs: `docs/design-system.md` (the §11 row; the Mask row loses the `SquareDashed` glyph),
  `docs/tracing-images.md` (Mask: Pick mask paths, Clear mask).

### Decisions (Proposal)

- The pipette (Remove background, Pick colours) and the pick mode exclude each other: arming one
  disarms the other, so a press never has two meanings and the cursor tells which one is armed.
- A group is accepted only when every leaf is path-like (the Select route's rule); a group with a
  text in it is refused with the row like a text.
- The hover box while armed is drawn only over a candidate the pick would accept.
- An armed press while the selection holds no traceable image does what the tool does unarmed.

### Tasks

1. [x] Session tests first: arm/disarm and cursor (35), pick adds/removes and stays armed (36),
   each kind gives the Select route's region (37), refusals and no-ops, double-click (38),
   Escape-independent disarm, leaving the tool and Create paths disarm (39), Clear mask keeps the
   paths, no history step (40), Shift+click and the pick give the same mask (41).
2. [x] Session: `trace_mask_pick.rs`, the shared `trace_select_hit` and `is_mask_path`, cursor,
   hover, double-click, pipette exclusion (35 to 41).
3. [x] wasm surface (34 to 40).
4. [x] Frontend unit tests first: strings, cursor (35, 43).
5. [x] Frontend: the Mask group in the bar, Escape order (pick first, then the running trace, then
   the tool), refused row, polite on/off status, Clear mask (34 to 41, 43).
6. [x] Chromium file: arm, pick two paths, Inside / Outside appears, pick again removes, Clear
   mask, Escape disarms, leaving the tool disarms, images and texts refused with the row, Tab
   order, 800 x 600 wrap (34 to 41, 43).
7. [x] Docs: design-system row, tracing-images (34 to 43).
8. [ ] Full gate; Chromium files of 0094 tester, trace_text_browser, 0091 tester, 0088 Part B,
   0025 x3, 0089 tidy-up.

Criterion 42 needs slice 2 (parts); with one part "the active part" is the only one, so it holds
trivially and is checked with slice 2.

### Validation

- `cargo nextest run -p curvyo-editor-wasm` during work; goldens unchanged.
- Expected tester-test updates (reported, not edited): `acceptance_0094_tester` c1 (`MASKED_BAR`,
  `BRIGHTNESS_BAR` gain "Pick mask paths", masked also "Clear mask"), `0089_tidyup` c57 Tab order
  and the bar arrays.

## Slice 2, parts and Join (criteria 18 to 33, 42, 43)

Implementer, 2026-11-05, on `story/trace-parts` (from `main` after slice 1b). Built against
`specification.md` slice 2 (criteria 18 to 33, 42, 43; Frage 141 A, customer 2026-11-05),
`adrs.md` decision 5 ("Slice 2 hooks") and the note of 2026-11-04, `ux-notes.md` §3 to §8.

### Affected crates/modules

- **`curvyo-ui-core`**
  - `trace_parts.rs` (new): what each part keeps after the later parts are taken out (the
    kernel's `visible_regions` over the parts' kept areas, bottom first) and the join of the
    parts' traced results into one `TraceWire` (seam pieces united per colour, the rest kept
    with their curves). `trace_region.rs` lends its box test and its rewinding.
- **`curvyo-editor-wasm`**
  - `session/trace_parts.rs` (new): the parts as Trace tool state (stable id, mask path ids,
    Inside/Outside, the active part; the active part's mask is the selection), add, remove,
    activate, the chips' view, each part's clip JSON for the worker, the join of the parts'
    wires into the preview and the result, the crop of a part's pixels.
  - `wasm_trace_parts.rs` (new): the wasm surface of the above (`wasm_trace.rs` is already
    above 800 lines).
- **`frontend`**
  - `lib/traceParts.ts` (new, no React): the parts' jobs one by one with a cache per part key,
    one progress over the parts that are traced, the joined result.
  - `hooks/useTraceParts.ts` (new): each part's settings, chip pick, Add part, Remove part.
  - `components/TraceParts.tsx` (new): the Parts row in Advanced; `TraceBar.tsx` the
    "Part 2 of 3" label and the expander dot; `TraceAdvanced.tsx` the row; `useTrace.ts` the
    branch to the parts' jobs with two or more parts.
- Docs: `adrs.md` dated note, `docs/adr/0023` §5 note, `docs/design-system.md` row status,
  `docs/tracing-images.md` "Parts".

### Decisions (Proposal)

- With one part every path is slice 1/1b's (the same settings string, the same region, no join),
  so goldens, determinism and the 0091 and 0088 tests stay as they are (criterion 33).
- A part's kept area: Inside its region, Outside the image's box grown by 1 mm minus the region,
  the whole image that box; the box keeps every region edge off the picture border (decision 1).
  A part nothing later overlaps keeps its own region (curves intact); a trimmed one is clipped to
  its visible region; a hidden one is not traced.
- Join: outlines whose box meets no edge of their part's clip stay as they are; the others are
  united per colour (one operand per part) and come back as polylines. Layers merge by exact RGB;
  a single-colour part counts as #000000 next to a multicolour one. Any part TooDetailed or Invalid
  fails the join. The palette strip shows the active part's palette; layers come from all parts.
- A setting change retraces only its part (criterion 25); a later part's mask change retraces the
  earlier ones too, because their clips change.
- With two or more parts a part's values are its own: a new mask does not re-apply the analysis;
  only part 1's values are remembered (criterion 29).
- Text with parts: the parts with Recognise text on trace with the read lines; the fit runs once
  over the joined result (the preview's full trace and Create paths), as for several images.
- Crop (criterion 31): a part without Recognise text is traced from the crop of its clip's pixel
  box plus a margin; with one part nothing is cropped.

### Tasks

1. [x] ui-core tests first: M1 and M2 with 2 and 3 parts, overlapping and not, the seam, the
   multicolour layers, shapes (criteria 20 to 23, 21).
2. [x] ui-core: `trace_parts.rs` clips and join (criteria 20 to 23).
3. [x] Session tests first: chips (add with copy, cap 8, remove, renumber, activate selects its
   paths), mask path deleted, leaving the tool, Clear mask and picks on the active part, the
   join into preview and result, Create paths one group one step, the crop (criteria 18, 19, 24,
   27, 28, 29, 31, 42); text in part 1 only is the core test of task 1 (criterion 30).
4. [x] Session: `trace_parts.rs`; wasm `wasm_trace_parts.rs` (criteria 18, 19, 24, 27 to 30, 42).
5. [x] Frontend unit tests first: the parts' jobs with a fake runner (one trace call after one
   change, one rising progress, cancel), chip names (criteria 25, 26, 32).
6. [x] Frontend: `traceParts.ts`, `useTraceParts.ts`, `TraceParts.tsx`, bar label and dot,
   `useTrace.ts` branch (criteria 18, 19, 25, 26, 28, 32, 43).
7. [x] Criterion 31: CPU time of 4 quarter parts against one trace on 2000 x 2000 px (reported).
8. [x] Docs: adrs.md note, ADR 0023 §5, design-system, tracing-images.
9. [ ] Full gate; Chromium files.
