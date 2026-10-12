# Plan for pen tablet input

One PR on `story/pen-tablet-input`, built as five milestones (customer rule of
2026-10-09: no PR until the customer can try the slice end to end). Each milestone is
one or more commits that leave the gate green, and the branch is pushed after each one.
The PR is opened once, after milestone E.

State found on `main` (639e067): there is no freehand tool. The rail already has Group
under Tools in column A and Boolean in column B, so the Pencil is appended to the Tools
card and only the Group card moves down 44 px (row "Tool rail architecture", exception 2).
`i_overlay` 9.0.1 has the integer variable stroke (`mesh/int/variable_stroke`), and
`kurbo` 0.13.1 has `simplify_bezpath`. No new dependency, no new crate, no format change.

## Affected crates/modules

- `curvyo-geometry-core`: new `freehand_fit.rs` (samples to a fitted Bézier centre line,
  0.08 mm), new `variable_outline.rs` (centre line plus widths by arc length to a closed
  outline through `i_overlay`'s integer variable stroke, then `cleanup` and
  `canonical_millimetres`). Two `mod` lines in `lib.rs`.
- `curvyo-ui-core`: new `pointer_sample.rs` (`PointerSample`, `PointerKind`), new
  `pencil_pressure.rs` (smoothing, Soft/Linear/Firm, width mapping, Min/Max clamp), new
  `pencil_tool.rs` (stroke state machine, "pressure present", touch ignored, hover,
  minimum length, cancel). One row in `history_names.rs` (`pencil_stroke` → "Pencil
  stroke"). Module lines in `lib.rs`.
- `curvyo-render-core`: the blue in-flight preview: a 1.5 px line or a hollow outline,
  white casing. One additive field in the decoration input.
- `curvyo-editor-wasm`: `Tool::Pencil` (one variant), new `session/pencil.rs` (session
  settings, preview, the commit through the existing path-creation command, selection of
  the new object), new `wasm_pencil.rs` (`pencil_samples` taking one flat `Float64Array`
  of 6 values per sample, kind and buttons; settings and readout getters), one row in
  `session/key_table.rs` (`P`), one step in the Escape cascade.
- `frontend`: `Canvas.tsx` pointer wiring (`getCoalescedEvents()` when present, pen hover
  through the existing move path), `ToolRail.tsx` (seventh tool button, Lucide
  `PencilLine`), new `PencilToolbar.tsx` (readout, Min, Max, Curve), crosshair cursor in
  `lib/cursors.ts`, new `lib/penInput.ts` (event to sample batch, pure, unit-tested).
- `docs`: `design-system.md` rows only if the build deviates; `requirements.md` R-INP-001
  status; spec status line at the end.

**Shared files with parallel work** (navi: `0050` layers panel in document-core,
render-core, ui-core, editor-wasm, frontend; naru: `0038` offset in geometry-core
`offset.rs`, ui-core `offset_entry.rs`, document-core `replace.rs`): edits to `lib.rs`
module lists, `Tool`, `key_table.rs`, `history_names.rs`, `ToolRail.tsx`, the toolbar
switch and the render decoration input stay one-line, additive and appended at the end
of their list, so rebases resolve trivially. No change to `curvyo-document-core`. No
refactoring of shared code; anything worth refactoring is noted for the lead.

## Tasks

### Milestone A: geometry (AC 7, 8, 10)

- [ ] A1. Test first, `freehand_fit.rs`: 100 samples on a line give 2 nodes; 200 samples
  on a half circle r = 30 mm give fewer than 10 nodes; every sample within 0.1 mm of the
  result, measured with `nearest_point_on_segment`, not trusted from the accuracy
  argument; fewer than 2 samples or length below 0.1 mm give no path. (AC 7, 12)
- [ ] A2. Implement the fit with `kurbo::simplify_bezpath` at 0.08 mm on the sample
  polyline (duplicate points dropped within the 0.001 mm grid). (AC 7)
- [ ] A3. Test first, `variable_outline.rs`: constant width 2 mm on 50 mm gives a capsule
  of 103.1 mm² ± 0.3; linear 0.2 → 2 mm is 0.2 mm wide at the start and 1.1 mm at x = 25
  ± 0.05; the outline lies within 0.02 mm of the exact variable-width outline (sampled
  check against the analytic offset); a self-crossing stroke gives an outline with a hole
  as a compound result. (AC 8)
- [ ] A4. Implement: flatten within 0.01 mm, widths interpolated by arc length, snap to
  grid, integer `variable_stroke` with round joins and ends and the 0.01 mm arc step of
  `0038`, `cleanup`, `canonical_millimetres`. (AC 8)
- [ ] A5. Budget tests (release, `boolean-budgets` job, `#[ignore]` in debug as the
  existing budget tests do): 2000 samples on a 150 mm S-curve give fewer than 600 nodes
  and fit plus outline under 200 ms; outline of the raw 2000-sample polyline (the
  preview path) under one frame (16 ms). Decimate to 0.05 mm spacing only if it misses.
  (AC 10, 11)

### Milestone B: input record and Pencil state machine in ui-core (AC 1, 3, 4, 5, 9, 12, 14)

- [ ] B1. Test first, `pointer_sample.rs`: `PointerSample` carries position, kind, pressure,
  tilt x/y, eraser (`buttons & 32`), time, unchanged from the input. (AC 1)
- [ ] B2. Test first, `pencil_pressure.rs`: smoothing 0.2, 0.2, 1.0, 0.2, 0.2 → 0.2, 0.2,
  0.467, 0.467, 0.467 (± 0.001); 0.25 → 1.1 / 0.65 / 0.3125 mm for Soft / Linear / Firm
  with 0.2 and 2 mm; Min range 0.05 to 20, Max 0.1 to 50, Max below Min raised to Min.
  (AC 9, 14)
- [ ] B3. Test first, `pencil_tool.rs`: pressure present for pen 0.1 to 0.9, absent for pen
  constant 0.5, pen constant 0 with contact, and mouse; touch starts nothing; pen move
  with `buttons` 0 starts nothing, pen down with `buttons` 1 starts a stroke; Escape
  cancels; fewer than 2 samples or length below 0.1 mm finish with nothing; the
  pressure-present flag switches mid-stroke on the first usable sample. (AC 3, 4, 5, 12)
- [ ] B4. `history_names.rs`: `pencil_stroke` → "Pencil stroke"; the guard test there
  passes. (AC 13)

### Milestone C: session, preview, commit in editor-wasm and render-core (AC 1, 2, 6, 7, 8, 11, 12, 13, 14, 16)

- [ ] C1. Test first, `session/pencil.rs` through the session API with synthetic sample
  batches: a mouse stroke gives one open path with the new-Pen-path style; a pressure
  stroke gives one closed path, fill = new-path stroke colour, stroke None; exactly one
  commit labelled `pencil_stroke`; the new object is selected; the tool stays Pencil;
  Escape mid-stroke writes nothing. (AC 7, 8, 12, 13)
- [ ] C2. `wasm_pencil.rs`: `pencil_samples(kind, buttons, Float64Array)` with 6 values per
  sample; one batch of 5 samples gives 5 samples in the stroke. No pointer id or device
  name in the signature. (AC 1, 2, 16)
- [ ] C3. `Tool::Pencil`, key `P` in `key_table.rs` (gated like the other tool letters),
  Escape cascade first step, crosshair cursor id. Every existing mouse test passes
  unedited. (AC 1, 6, 12)
- [ ] C4. Session settings Min, Max, Curve with getters/setters; "pen seen" and "pressure
  seen" flags for the bar; readout values (pressure, tilt). Settings are not in the
  document, the log or any file: a test saves the document after a pressure stroke and
  checks the bytes for none of the settings or sample data. (AC 14, 15, 16)
- [ ] C5. Render-core preview: line or hollow outline in blue with white casing; on
  release the preview is gone and the object lies within 0.15 mm of the last preview
  (test compares the last preview outline with the committed outline). (AC 11)

### Milestone D: frontend wiring, rail button, Pencil bar (AC 1, 2, 4, 5, 6, 14, 15)

- [ ] D1. Test first, `lib/penInput.ts` (node test): a move event with 5 coalesced events
  gives 5 samples, one without `getCoalescedEvents` gives 1; touch is filtered for the
  Pencil; pen hover goes the mouse-move path. (AC 2, 4, 5)
- [ ] D2. `Canvas.tsx`: with the Pencil active, down/move/up send one `pencil_samples` call
  per DOM event; other tools unchanged. (AC 1, 2, 6)
- [ ] D3. `ToolRail.tsx`: seventh button, `PencilLine`, "Pencil tool (P)" / "(Esc, P)", Group
  card moves down 44 px by layout. (AC 6)
- [ ] D4. `PencilToolbar.tsx`: hidden until a pen is seen; readout "Pen: pressure 0.62, tilt
  12° / -3°" in a fixed 232 px muted box; Min, Max, Curve after the first pressure stroke
  (value fields of `0017`, `role="radiogroup"`); not in the tree for a mouse-only
  session. (AC 14, 15)
- [ ] D5. Docs: design-system rows touched only where the build differs; R-INP-001 status.

### Milestone E: verification and PR

- [ ] E1. Playwright run of the built frontend with synthetic `PointerEvent`s
  (`pointerType: "pen"`, `pressure`, `tiltX`, `tiltY`, `buttons`) dispatched on the
  canvas: a pressure stroke creates a filled outline, the bar and readout appear, a
  mouse-only session never shows them, a touch stroke creates nothing. Chromium from
  `/opt/pw-browsers` or the system browser; never `playwright install`. (AC 1 to 6, 11, 14, 15)
- [ ] E2. Full gate of `CLAUDE.md` §7 locally, release budget tests, wasm32 builds of the
  core crates.
- [ ] E3. Open the PR ("What changes in the demo", Review guide). It states that no real
  pen hardware was available: pressure was verified only with synthetic Pointer Events
  in Playwright and with sample records in Rust tests; the platform questions are for
  the demo.

## Validation approach

- Core logic test-first in `geometry-core` and `ui-core` with the exact numbers of the
  criteria; session behaviour in `editor-wasm` tests with synthetic sample batches;
  frontend pure helpers with `node --test`; end to end in Playwright with synthetic pen
  events.
- Budgets (AC 10, 11) as release-build tests in the `boolean-budgets` CI job.
- Every build and test command runs with `ulimit -v 8000000; CARGO_BUILD_JOBS=4` and
  `cargo nextest run -j 4`.
- Git: `git add <paths>` only (never `-A`), no bare stash, no push to `main`, no
  force-push. Conventional Commits, scope = crate name.

## Not done here (for the lead)

- Native pen axes on Linux (GDK), eraser and tilt behaviour, editable width profiles:
  out of scope per specification and `adrs.md`.
