# Plan for multiple pages

## Affected crates/modules
- document-core: page frames, independent root registers, history, persistence.
- ui-core: spatial bounds/context, page view state, rulers.
- editor-wasm: session commands, rendering and preview integration, public bridge.
- frontend: Document rows, accessible actions, frame dragging and rulers.

## Implementation choices (Proposal)
- Flat root keys `page.<id>.<field>` for name, x, y, width, height, order,
  deleted; eager initial ID `first`; generated IDs use the Loro peer/counter.
  Initial frame width/height alias existing root `width_mm`/`height_mm`, with
  no duplicated size authority. Accepted by architecture review.
- Numeric order registers with ID tie-breaking; reorder writes only order.
  Equal or precision-exhausted ranks are renumbered in the same transaction.
  A frame exposed after concurrent all-page deletion is revived by its next edit.
- Wrap when the current row already contains eight frames or adding a frame
  would cross 4000 mm from its row start. The next row starts at the first
  frame's x, below the greatest bottom edge in that row plus the last frame's
  gap. Gap is max(20 mm, 10% of last frame's larger side).
- Preserve original single-page resize centering; multi-page resize centers
  exclusively contained content in the active frame, following ADR 0024.
- Use resolved bounds for page membership; edge contact follows existing
  bounding-box intersection tolerance. Active identity is view state.
- Performance budget: 200-page enumeration/render preparation and active-page
  switching each average under 20 ms thread CPU time on the development host;
  adding a page under 100 ms. These are measured targets, not page limits.

## Tasks
- [x] 1. Tests first: initialized first page, fieldwise edits, ordering, validation,
  concurrency, undo/redo, save/open, format 26 golden (AC 1, 3–6, 13).
- [x] 2. Core frame model and commands; contained-object movement, clones,
  shared layers and context membership (AC 3–6, 9–11, 13).
- [x] 3. Session active state, viewport/ruler handling, selection context,
  preview frames, background/edge/label rendering (AC 2, 4, 7–8, 12–13).
- [x] 4. Accessible Document rows/actions and canvas frame drag with cancel,
  existing formats and sizes applied to active page (AC 2–6, 9, 13).
- [x] 5. Focused regression, integration and CPU measurements; independent
  tester full gate plus architecture/UX review (AC 1–14).

## Validation
Run affected Rust crate tests and frontend type/tests during development.
Core tests precede implementation. Include current-format round trips and a format-26 golden file,
independent replica merges, history preview and cross-page clone regressions.
The independent tester owns acceptance tests and the single full gate after
integration. Record checks and measurements here as each milestone passes.

## Development evidence
- Core tests written before page APIs: expected missing-method compilation failures.
- `cargo test -p curvyo-document-core --test pages`: 8 passed, including
  independent field merges, concurrent deletion and exhausted reorder ranks.
- `cargo test -p curvyo-editor-wasm --test pages -- --nocapture`: 5 passed
  before adding the offscreen-activation regression; 201 frames: add 22.7 ms,
  switching 5.05 ms average, drawing 4.83 ms average (Linux thread CPU, debug).
- Affected library clippy passes; frontend tsc passes; npm test: 104 passed;
  oxlint exits 0 with existing repository warnings.
- Lead browser smoke found and verified fixes for the canvas slot child count
  and offscreen activation pan direction. Final browser review pending.
- Affected-crate nextest completed (see regression findings). Two setup-counter assertions updated
  from 3 to 7 operations for the eager page registers; no behavior test dropped.
- Customer steering 2026-10-29: old-file compatibility and migration excluded.
- Frame membership uses resolved global leaf bounds (1e-9 mm inclusive edge
  tolerance). Frame movement includes hidden/locked leaves, just like document
  resizing; it changes their geometry without changing layer visibility/locks.
  Groups are expanded once to leaves, so no ancestor/descendant double shift.

- Page-edge gestures use outside-only open side bands; exact boundaries and
  corners remain available for existing marquees. Entered-group editing makes frame gestures
  inert. Object/selection handles, placement and eyedropper retain priority.

## Regression findings resolved during independent verification

The first full affected-crate run executed 8,343 tests: 8,299 passed, 44 failed,
94 remained ignored by their existing definitions. Real interaction regressions
were fixed in production (corner marquees, entered groups, exact eyedropper
edges); the targeted rerun covers every affected editor behavior test.

The independent tester reviewed the following pre-page assumptions without
removing behavioral assertions (see `acceptance-report.md`):
- `acceptance_0015_pr1_whitebox_tester` (additive JSON page snapshot),
  document `acceptance_0040_tester::ac4_every_object_of_an_older_file_is_where_it_was`,
  `acceptance_0050_tester::a_document_without_flags_exports_no_flag_keys_at_all`:
  additive `pages` export and current version 26.
- `drawing_insert::ac2_ac77_a_document_from_a_drawing_has_no_step`: eager
  the allowed current-format range now includes page format 26.
- `arc_curve_format`, `clone_overrides_format`, `guides_format`, `image_format`,
  `layers_format`, `text_clones`, `text_format`, `text_single_line_format`,
  `text_on_path` golden tests: expected IDs are rebuilt with today's constructor
  instead of read from the fixture; initial page registers shift new IDs by four.
  A test-only expected-ID adjustment preserves the original fixture bytes and
  all geometry/style/reference assertions.
- editor `acceptance_0024_slice2_tester::c109_deterministic_and_format_unchanged`:
  fixed expected version 25 is now 26.

## Final implementation verification
- Focused nextest: 17/17 passed (9 document pages, 7 session pages, cut regression).
- Session pages rerun after adding partial-overlap clone assertion: 7/7 passed.
- Prior targeted regression: 43/45 passed; both failures fixed and verified above.
  All 29 original editor behavior regressions now pass; expected format/ID/export
  oracles remain for the independent tester as listed above.
- Page transforms compensate every unselected clone via the existing hold plan,
  retaining clone links and own-placement flags. Ordinary object transforms keep
  their existing semantics. Outside/partial clones remain stationary, including
  hidden and locked objects on shared layers.
- Architecture review approved. UX source fixes include keyboard Escape/focus,
  unit changes, roving row controls, theme tokens and full-contrast page labels.
- Final affected Rust `clippy --all-targets -- -D warnings`: passed.
- Final WebAssembly release build + generated bindings: passed (43.31 s).
- Final frontend `tsc -b` and Vite production build: passed (existing chunk-size warning).
- Final frontend unit suite: 104/104 passed; oxlint exit 0 (existing warnings,
  none in the new page components). `git diff --check`: passed.
- Implementation handed off for independent acceptance-oracle updates and full
  gate; final browser review is owned by the lead.

## Independent final acceptance (2026-10-30)

- Nine independent public-API tests supplement the 16 document/session page tests;
  criterion-by-criterion evidence is recorded in `acceptance-report.md`.
- Focused regression: 95/95 passed. One real persisted-size boundary regression
  was fixed in production; its original damaged-size assertion was retained.
- Complete workspace Nextest: 9,400/9,400 passed; 114 existing ignored tests.
  The host WebKit environment variable was unset for the existing fresh-process
  test, and the complete run used sandbox escalation for a fork/signal-pipe
  restriction. No assertions were changed or tests skipped for those issues.
- Full gate passed: fmt; workspace/all-target Clippy; nine-package wasm build
  and Clippy; cargo deny; warning-denied workspace rustdoc; final frontend wasm
  build; TypeScript; npm tests (633 passed, 56 existing skipped); oxlint exit 0
  with existing warnings. Final whitespace check passed.
- Chromium: lead's 200-page rows/frames/navigation/drawing interaction passed;
  tester's final page CRUD/unit/cancellation/reveal script passed, including
  focused-row undo/redo of rename and reorder after the late key-propagation fix.
  Optional permanent scripts and runner instructions are in `frontend/tests/browser/`.
- Architecture review approved; lead's visual/keyboard review findings resolved
  and covered by final browser checks. Customer demo acceptance remains pending.
