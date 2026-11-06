# ADRs for masked and composite trace

Architect, 2026-11-04. Feature-local decisions for slice 1 (one mask region per trace, Inside or
Outside, picked by selecting the image plus paths), with the hooks slice 2 (parts and Join) needs.
Assumes the spec's defaults Frage 130 A to 134 A. Every choice below is a **(Proposal)**,
vetoable at the demo.

**No new ADR, no new crate, no new dependency, no trait, no generic, no `format_version` change,
nothing new in the document.** The mask is tool state; the result is the group Create paths
already writes.

## Depends on

- [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md) §3, §7: the
  boolean kernel (`curvyo-geometry-core::boolean`, integer grid of 0.001 mm, explicit
  `Tolerance`, polylines out). Reused as is; no new kernel function.
- [ADR 0023](../../docs/adr/0023-raster-trace-crate-pipeline-and-worker.md) §4, §5: `trace()` and
  `TraceParams` stay unchanged; the worker job gains one input (the region). A dated note on §5 is
  added when the slice is built ("a job may carry a mask region in document mm").
- [`0091` adrs.md](../0091-trace-text-recognition/adrs.md) decisions 1 to 3: OCR once per image,
  the text pass and in-painting are reused; the mask only filters their output.
- [`0088` adrs.md](../0088-trace-recognition/adrs.md) Part B: the shape pass is reused, run on
  the clipped outlines.
- [`0016` adrs.md](../0016-boolean-operations/adrs.md): how operands are collected (groups
  expanded, clones resolved, primitives and open paths as outlines, document z-order) and the
  0.01 mm checking tolerance.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md): no new crate edge
  (`curvyo-ui-core` already depends on `curvyo-geometry-core`; `curvyo-vectorize-core` stays free
  of it).

## Feature-local decisions

### 1. Where the clip happens: geometric clip of the traced shapes with the kernel

Options:
- **(a) Clip the traced shapes against the mask region with the boolean kernel.** Exact stencil
  edge (Frage 134 A); one trace serves Inside and Outside, so criterion 4 holds by construction.
  Cost: shapes that cross the stencil edge come back as polylines.
- (b) Rasterise the mask and trace only the pixels inside. Rejected: the edge follows the pixels
  (rough, Frage 134 B), and Inside plus Outside leave a seam of up to one pixel, which fails
  criterion 4 and slice 2's seam (criterion 22).
- (a') A curve-preserving clip: cut the traced contours and the mask at their crossings with
  `cut_parameters` (0036), choose pieces by an interior test, chain them into loops. Rejected for
  now: new kernel code with coincident-edge cases (a mask edge on the picture border, as R1 in M1)
  and no second user yet. It is the natural fix of the existing debt "Boolean results are
  polylines, not curves" (`docs/technical-debt.md`) and would replace (a) without changing the
  job or the wire.

**Decision: (a).**

**Space and units.** The trace already returns outlines in document mm through the image's
`pixel_map` (placement after `1/w, 1/h`). The clip therefore runs in document mm. Rotated, scaled
or skewed images (criterion 8) cost nothing extra, and the reduced preview copy and the full-size
picture use the same region. The inverse `pixel_map` is used only on the raster side (decision 3,
region-only analysis, the OCR centre test).

**The region, in the session (`curvyo-editor-wasm`), once per mask change.**
- Each mask path becomes one kernel operand of world-space outlines. The collector is the one the
  Boolean commands use: groups expanded, clones resolved, rectangles, ellipses, polygons and stars
  as outlines, the world transform applied. Operands come in document z-order, never selection
  order (determinism).
- `boolean(Union, operands, 0.005 mm)` gives the region M:
  - a compound path keeps its holes and a self-intersecting path paints its crossing (kernel
    step 4);
  - an open path is closed by its straight closing line (kernel step 2; criterion 6);
  - overlapping paths are one union (criterion 5).
- **Tolerance 0.005 mm:** the kernel keeps 0.002 mm for the grid and flattens the mask's curves
  at 0.003 mm. That leaves half of criterion 2's 0.01 mm for the test, instead of none at a
  kernel tolerance of 0.01 mm.
- An operand that encloses no area (`EmptyOperands`, a straight open path) is dropped and the
  union is retried without it, so there is no refusal. If none is left, the mask counts as "outside
  the image".
- The image quad Q is **not** intersected into the clip region. Traced shapes touching the picture
  border lie exactly on Q's edge, so each of them would count as crossing and turn into a
  polyline. Q is used only for:
  - the test "M ∩ Q is empty" behind the message "0 paths: the mask is outside the image."
    (criterion 7), which is checked in the session and dims the button without starting a job;
  - the Overlay wash (decision 3);
  - the raster mask.
- M is recomputed when the set of mask paths changes or one of them is edited or deleted
  (criterion 27 in slice 2). It is not recomputed when a slider moves.

**Hand-off.** The session passes M to the worker with every job as flat numbers in document mm
(outline count, then per outline its vertex count and coordinates), next to `map`. With no mask
there is no region and **no clip runs**, so criterion 1's regression is bit-identical.

**The clip, in the worker, in `curvyo-ui-core`.** One new module, e.g. `trace_region` ("Clips
traced outlines to a mask region"):
- It is called inside `trace_job_with` / `trace_colour_job_with` (and the text variants) on the
  `TraceResult` / `ColourTrace`, **before** the wire is written. Layer outline counts change, so
  it cannot work on the wire.
- Outlines are grouped into islands (an outer outline with its holes) with `outline_nesting`.
- An island whose control-point box crosses no edge of M lies wholly inside or outside. One
  point-in-polygon test of its first anchor against M decides whether it is kept, curves intact,
  or dropped. Only `+ - * /` are used; there is no curve root solving, see decision 4.
- Only crossing islands go through `boolean`, one operand per island (holes included):
  Intersection with M for Inside, Difference by M for Outside. An `EmptyResult` drops the island.
- Counts (paths, nodes, foreground) are taken after the clip; specks removed stay the trace's.
- The worker, not the session, runs it: it is off the UI thread, cancelled by terminate (ADR 0023
  §5), and preview and Create paths share it.

**Consequence.** A traced shape that crosses the stencil edge is a polyline at the 0.001 mm grid,
over its whole outline and not only along the edge. Everything else keeps its curves. This is
visible in node counts and goes under the existing debt entry; the curve-preserving clip (a') is
the fix.

### 2. Text and shape recognition with a mask

- **OCR is unchanged:** once per image on the full working image, cached by asset hash (`0091`
  decision 1). There is no OCR crop by the mask, so changing the mask never re-runs OCR.
- **Text pass:** every accepted line is in-painted as today, then the worker keeps in the text
  section only the lines whose box centre lies in the region. The test maps the centre px to mm by
  `pixel_map` and runs point-in-polygon against M, or outside M for Outside. A line whose centre
  is outside is neither text nor traced, because its pixels were in-painted (criterion 11).
  In-painting before filtering is what makes this hold: filtering first would let the trace pick
  up the inside half of a line whose centre lies outside.
- A kept text object is set whole even where it crosses the stencil edge, because a text is not
  clipped. **For the PO:** criterion 2's "no node lies outside R1" must exempt text objects, or
  the rule becomes "only lines wholly inside the region". Default: exempt (M1 holds no text, so
  the criterion's fixture is unaffected).
- **Shape pass:** it runs after the clip on the clipped outlines, so a primitive can never cross
  the stencil. A disc cut in half is fitted as whatever the half fits, or stays a traced path.
  This replaces the spec's "centre" rule for shapes in slice 1 and gives the same result (a shape
  outside is gone before recognition). In slice 2 the centre rule decides which part's switch
  applies.

### 3. Mask view, Overlay and region-only analysis

- **One raster of the region**, a pure scanline rasteriser in `curvyo-ui-core`:
  - M is mapped by the inverse `pixel_map` into the mask's pixel grid (twice the picture after
    Upscale);
  - pixel centres are tested with the nonzero rule, crossings found row by row;
  - the result is a `Vec<bool>`, worked out in the worker per job;
  - a per-pixel point-in-polygon is ruled out: 4 Mpx times 500 edges is too slow.
- **Mask view:** `trace_mask_job` (and the text and colour variants) whiten pixels outside the
  raster, or inside it for Outside (criterion 10).
- **Analysis** (Remove background, Pick colours, Analyse, auto Threshold): the same functions over
  only the pixels in the raster (criterion 9). With no mask they read every pixel, as today.
- **Overlay:** the preview outlines are already clipped, so the outline ends on the mask path
  without extra work. The 12 % wash of the UX notes is the session's `Difference(Q, M)` for Inside
  or `Intersection(Q, M)` for Outside. It is computed with M and tessellated with the preview
  mesh.
- **Trace view:** unchanged, since it shows the clipped result.

### 4. Performance and determinism

- The kernel is integer arithmetic plus `sqrt`, the island test uses `+ - * /`, and M arrives in
  the kernel's canonical order. The clip therefore adds no difference between native and wasm, or
  between one worker and two, beyond ADR 0023's known `acos`. Criterion 15 runs twice in one test.
- Cost is one union per mask change in the session, then per job one box test per island and
  `boolean` only on the islands that cross. Criterion 16 (2000 x 2000 px, 500-node mask, at most
  0.1 s extra) is measured in thread CPU time with the existing harness
  (`curvyo-editor-wasm/tests/cpu_time/`). It is reported and does not block; a miss goes to
  `docs/technical-debt.md`.
- Slice 1 always traces the whole image. A crop to the region would change the outlines near the
  crop border and break criterion 4.

### 5. Hooks for slice 2 that slice 1 must keep open

- The clip is a pure function of (outlines, region, Inside or Outside) and knows nothing about
  selection or "one mask", so a part is just another region.
- Stacking order (Frage 132 A, last on top): each part's effective region is its region minus the
  regions of the later parts. `visible_regions` already computes "region minus everything above"
  in the same kernel. Part 1 with no mask means region Q, used for this subtraction only.
- Join: each part's wire is cached as today's, so criterion 25 retraces only the changed part.
  The clipped pieces of all parts are snapped to the same 0.001 mm grid, so a per-colour
  `boolean(Union)` closes the seams (criteria 22 and 23).
- Criterion 31 (crop per part): the job may later carry a px crop rectangle with a margin. Slice 1
  adds no parameter for it (§5, no speculative options).

### 6. Risks

- **Self-intersecting masks:** the kernel's nonzero self-union paints the crossing, as Boolean
  operations do. A compound path drawn with the even-odd rule and same-wound holes reads as 0016
  reads it. This is consistent with the Boolean commands; a test with the ring of criterion 5
  covers it.
- **Masks outside the picture:** the session's M ∩ Q test (criterion 7). A mask partly outside
  needs nothing: traced shapes never leave the picture.
- **Rotated or skewed images:** handled by working in document mm (decision 1). The raster side
  uses the inverse map, which exists for every placement that is not degenerate. A degenerate
  placement already fails the trace.
- **Open paths:** closed by their straight line. A straight open path has no area and is dropped
  (decision 1).
- **Node growth:** crossing shapes become polylines (decision 1, Consequence). The customer sees
  this at the demo. If he objects, (a') is the follow-up, with no change to the job or the wire.
- **Shapes with an edge on the stencil:** an island whose edge coincides with M's edge goes
  through the kernel. The kernel handles shared edges on the grid; the M1 fixture (the bar on the
  picture border under R1's edge at x = 0) covers it.

## Note of 2026-11-04 (as built, architect review)
- **Hand-off.** The region travels inside the settings JSON as a string, not as numbers next to `map`. Rust writes it, the frontend splices it in unchanged, so nothing is rounded on the way and two workers read the same string. The settings JSON is never persisted and unknown keys are ignored, so there is no versioning risk. A 500-node mask makes the string tens of KB, and each job parses it three times; accepted. See the dated note in ADR 0023 §5.
- **Clip.** All crossing outlines go to `boolean` as one operand, not one per island. This is equivalent under the kernel's per-operand nonzero union, provided the tracer winds holes opposite to their outer outline. A test with a ring crossing the mask edge, Inside and Outside, covers it.
- **Failures.** A refused clip keeps the crossing shapes whole when the mask keeps their first point (no exact edge for them). A region that cannot be combined shows its own row, "0 paths: the mask paths could not be combined.".


## Note of 2026-11-05 (slice 2 as built, implementer; Proposal, vetoable at the demo)
- **Clips.** Each part's kept area goes to `visible_regions` bottom first (Inside: its region's
  kernel polygons; Outside: the image's box grown by 1 mm minus the region; the whole image: that
  box). The box keeps every clip edge off the picture border, so decision 1's reason for leaving Q
  out of the clip holds. A part nothing later overlaps keeps its own region (its slice 1 clip, curves
  intact); a trimmed part is clipped Inside to its visible region; a hidden one gets no job.
  Muted parts and parts whose mask misses the image are left out of the stack.
  (`curvyo-ui-core::trace_parts::trace_part_clips`.)
- **Join.** A pure function of the parts' results and clips in stacking order
  (`join_trace_parts`). An outline whose control box meets no edge of its part's clip stays as it
  is; the others are united per fill colour with one operand per part on the same 0.001 mm grid
  (tolerance 0.005 mm), so a shape across a seam is one path and the seam leaves no line. Pieces of
  a single part are not united. Layers merge by exact RGB, largest area first; a single-colour part
  counts as #000000 next to a multicolour one. Shape regions follow their outlines' new places and
  are dropped when an outline went into a union. Text lines are concatenated. Any part TooDetailed
  or Invalid fails the join. The palette is the active part's (its strip edits its values).
- **Where.** The clip stays in the worker: each part is an ordinary job whose settings carry its
  clip as slice 1's `"mask"`. The session computes the clips (cached per set of areas) and joins the
  parts' wires into the preview and the result; Create paths is unchanged. With one part nothing of
  this runs (criterion 33).
- **State.** Parts are Trace tool state in the session (stable id, mask path ids, mode, the active
  part; the active part's mask is the selection) and the parts' settings live in the frontend under
  the stable id. Leaving the tool drops both (criterion 29); only part 1's values are remembered.
- **Caching.** Each part's wire is kept in the page under its settings (clip included) and size, so
  a setting change traces one part (criterion 25); a later part's mask change retraces the parts
  below it, whose clips change.
- **Region-only analysis per part (tester and coordinator 2026-11-06).** With two or more parts each
  part reads its own automatic Threshold, background and analysis from its clip, once per clip, and
  applies them as slice 1 does for a new mask (`partAutoValues`, `applyRegionAnalysis`): what the
  maker set for that part stays. Add part copies what the maker set with the settings.
- **Crop (criterion 31).** A part is traced from its clip's pixel box plus a margin, with the map
  moved, so that its result is the one of the whole picture: the margin is 16 px (Denoise, the
  smoothing and the fit stay outside the clip) or the pixel count of Ignore below's area when that
  is more (a shape reaching into the clip is not taken for a speck). The whole picture is sent
  instead for Recognise text (the OCR lines are in the picture's pixels), for Drop border shapes
  (the crop's edge is no border), for Denoise (tester 2026-11-06: a thin line across the cut came
  back 0.83 % off; margins of 32 and 64 px did not help, so the fit of a shape that crosses the cut
  is not local), with Upscale Auto only when Auto would decide differently for the crop than for the
  whole picture (the upscaled map's offset was fixed in PR #282), and for an Outside clip or a box that reaches the
  picture. The same non-locality could in principle reach a long shape across the cut without
  Denoise; no case is known. Multicolour
  with Colours Auto counts the colours of the crop, which suits the region-only analysis
  (Proposal). Measured: four quarter parts of a 2000 x 2000 px picture cost 0.98 x one whole
  trace in thread CPU time (release, `acceptance_0094_cpu`); a cropped part matches its whole-picture
  trace within 0.1 % (session test).
- **Text.** The parts with Recognise text trace with the read lines (whole picture, no crop); the
  fit runs once over the joined result (the preview's full trace and Create paths), as for several
  images, so the preview's texts appear after the join.
