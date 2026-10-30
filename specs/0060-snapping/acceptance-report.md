# Snapping acceptance report

2026-10-30. **PASS with bounded rerun evidence**, as detailed below; no unresolved test failure. Independent plan preceded
production reads. Tester added 22 public-API acceptance tests: document 3,
ui-core 12, editor session 7; all passed the focused run. No test is ignored.

## Criterion evidence

| AC | Executed evidence / scope |
| --- | --- |
| 1 | Independent defaults, save/open, no-write reads and peer-field merge/undo; snapping_format fixtures cover each register. |
| 2 | Lead browser snapping.cjs: plain/child Shift clicks, collapsed rail, Shift+Enter repeat and Shift+Space, enabled vs selected state, sticky drawing pane; UX screenshot APPROVE. |
| 3 | Independent capture/release boundaries at several screen scales, stored normalization; browser zero/fractional/large zones. |
| 4 | Independent raw-position hold/release/recapture sequences. |
| 5 | Independent true cubic vs chord, exact page fractions, hidden/locked groups, moving descendants, anchors and dependent clones; production collection regressions include rotated drawn bounds. |
| 6 | Independent nearest/ties/order reversal, atomic nodes and ghost rejection, mixed finite guide/edge both orientations and hold, finite endpoint and closer-node competition, held edge finding farther compatible guide. |
| 7 | Independent Rectangle/Ellipse/Polygon/Star, Pen first/subsequent clicks, Shift move and Node anchor gestures. Production session tests add moving-corner resize, node-only handle targets, mirrored and rotated aspect resize. Full existing tool regression checks retained constraints. |
| 8 | Independent actual guide/axis marker identity and suppression; production renderer verifies 1px line/3px casing at scales 0.1/1/25. UX source/screenshot APPROVE; production session cancellation tests. Node two-axis lines follow explicit UX contract. |
| 9 | Independent Ctrl+Alt suppression for four creation modes with/without Shift, engine held-state clearing; production session checks press-during-move suppression without copying. |
| 10 | Independent invalid negative/NaN/infinite edits create no writes/history; lead browser all controls, normalized display, invalid/Escape, native input undo and document undo/redo. |
| 11 | Independent late-order target and 10k finite guides + 10k box edges exact composition, 1000 fresh captures, <2ms thread CPU. Production 10k document-object and dense-cubic benchmarks; architecture source review confirms indexed local queries/cache lifecycle. |
| 12 | Independent Shift move exact commit, one undo step, undo/redo, cancellation without writes; Node one-step movement. Production live-preview/release resize and move equality. |
| 13 | Independent dependent-clone and moving-anchor exclusions plus public Session clone drags to stationary original and sibling: exact own-box alignment, link/own-style/own-placement preserved, references unchanged, one-step undo/redo. |
| 14 | Independent current-format defaults/no-write reads; snapping_format missing/malformed register and valid-edge golden tests. Format 27; old migration excluded by customer. |
| 15 | Independent all six exact fractions with nonzero origins and inactive resized/moved actual page through session drawing; production target-revision refresh tests. All page frames currently visible; no hidden-page API exists to exercise. |

## Independent regression finding

AC6 initially failed: raw `(99,99)`, box edge `x=100`, finite horizontal guide
`y=100` produced `(99,100)` instead of `(100,100)`. Implementer fixed finite
curve/axis composition and searching beyond incompatible finite candidates.
The independent red test now passes, including mirrored axes and held state.
Numerical endpoint projection uses 1e-8 tolerance; target identities stay asserted.
A move test initially clicked a nearby parameter handle; using the centre move
handle establishes the intended gesture without changing its geometry oracle.

## Commands and reproducibility

Initial focused: `cargo test -q -p curvyo-editor-wasm -p curvyo-ui-core -p curvyo-document-core --test acceptance_0060` (21 passed). Additional public clone gesture test then passed with all 7 editor acceptance tests (22 total).

Full gate is sequential under `env -u WEBKIT_DISABLE_DMABUF_RENDERER` with
escalated process/cache access: prior workspace validation established that the
sandbox blocks nextest signal pipes and the inherited WebKit variable changes
environment-test behavior. No product assertions are disabled. Exact commands:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --message-format=short -- -D warnings
cargo nextest run --workspace --status-level fail --no-fail-fast
PACKAGES="-p curvyo-document-core -p curvyo-editor-wasm -p curvyo-geometry-core -p curvyo-image-core -p curvyo-render-core -p curvyo-svg-core -p curvyo-text-core -p curvyo-ui-core -p curvyo-vectorize-core"
cargo build -q --target wasm32-unknown-unknown $PACKAGES
cargo clippy --target wasm32-unknown-unknown $PACKAGES -- -D warnings
cargo deny check
RUSTDOCFLAGS="-D warnings" cargo doc -q --workspace --no-deps
cd frontend
npm run build:wasm
npx tsc -b --noEmit
npm test
npx oxlint
```

Full-gate logs: `/tmp/snapping-gate-*.log` and `-results.txt` (local artifacts).
First workspace clippy found a redundant closure in the regenerated format
fixture test; replaced with the same method reference, then restarted before
any broad test run. No production edit by tester.

Lead browser command: `node frontend/tests/browser/snapping.cjs` with optional
Playwright runtime environment documented in `frontend/tests/browser/README.md`.
Lead reported PASS with no JS errors; after final WASM getter cleanup tester reran
the same script and independently confirmed PASS, no page errors. Screenshot:
`target/browser-check/snapping-verified.png`.

## Performance and remaining matrix limits

Host: Linux x86_64, Intel Xeon SierraForest, 6 visible cores; debug test profile.
Independent mixed scene measured 0.1008ms/frame incremental thread
CPU over 1000 fresh captures, subtracting raw probe baseline. Index construction
is outside frame measurement; every measured frame asserts exact intersection
and both markers. No explicit warmup; this includes first-query cost. Production
object benchmark separately reports index construction. This is engine/search
cost, not browser frame rendering time; structural per-frame cache behavior is
also reviewed by the architect, not instrumented by an independent read counter.

The original matrix is intentionally broader than the new tests: no exhaustive
cross-product of all eight resize handles × every modifier × every tool state,
and no
pixel automation of every marker cancellation path. Existing tool/session tests
and architecture/UX review supply complementary evidence, not an invented claim
that every proposed matrix row was independently executed. Hidden page handling
cannot be exercised because the current model exposes all frames. All-visible
scope remains the documented default pending the optional customer answer;
PR224 integration/approval is a release coordination issue, not a test defect.

## Broad regression oracle corrections

First fail-fast run: 715 passed, one failure, 117 existing skips, 8782 not run.
`acceptance_0015_pr1_whitebox_tester::older_files_export_exactly_what_the_previous_build_exported`
now separately asserts the exact additive snapping defaults; all prior fields
remain equal. Complete inventory then ran 9498: 9481 passed, 17 failed, 117
existing skips, 425.288s. Failures below are expected current-schema/rail changes,
not removed tests or relaxed geometry assertions. Final run includes the added
clone test; its exact result and focused correction are recorded below.

| Failing test (suite::name) | Exact correction |
| --- | --- |
| `curvyo-document-core::acceptance_0017_block1_tester every_older_fixture_still_opens_with_the_styles_it_stored` | Classify five explicitly named malformed snapping fixtures and assert Damaged; retain every valid-fixture assertion. |
| `curvyo-document-core::acceptance_0040_tester ac4_every_object_of_an_older_file_is_where_it_was` | Assert exact additive snapping-default member separately; retain all old object/size/style equality. |
| `curvyo-document-core::acceptance_0046_tester c1_no_guides_means_no_guide_key_and_open_writes_nothing` | Parse JSON and inspect exact guide keys, distinct from new guides setting. |
| `curvyo-document-core::acceptance_0046_tester c2_every_kind_can_carry_the_flag_and_unmarking_removes_the_key` | Parse JSON and inspect exact guide keys, distinct from new guides setting. |
| `curvyo-document-core::drawing_insert ac2_ac77_a_document_from_a_drawing_has_no_step` | Current format 27 (snapping) instead of 26; retain deterministic/no-step assertions. |
| `curvyo-editor-wasm::acceptance_0024_slice2_tester c109_deterministic_and_format_unchanged` | Current format 27 (snapping) instead of 26; retain deterministic/no-step assertions. |
| `curvyo-editor-wasm::acceptance_0043_tester c10_a_selection_then_pen_then_unfinished_path_keeps_the_rule_pure` | Include seventh Snapping tab in exact ordered names/labels/count; preserve selected/enabled/tooltips and history adjacency checks. |
| `curvyo-editor-wasm::acceptance_0043_tester c10_an_unfinished_pen_path_empties_the_body_and_keeps_the_strip` | Include seventh Snapping tab in exact ordered names/labels/count; preserve selected/enabled/tooltips and history adjacency checks. |
| `curvyo-editor-wasm::acceptance_0043_tester c23_the_node_tool_with_a_selection_it_can_style_shows_style_and_with_none_document` | Include seventh Snapping tab in exact ordered names/labels/count; preserve selected/enabled/tooltips and history adjacency checks. |
| `curvyo-editor-wasm::acceptance_0043_tester c23_the_node_tool_with_a_selection_it_cannot_style_shows_document` | Include seventh Snapping tab in exact ordered names/labels/count; preserve selected/enabled/tooltips and history adjacency checks. |
| `curvyo-editor-wasm::acceptance_0043_tester rapid_random_sequences_keep_the_strip_consistent_and_never_panic` | Include seventh Snapping tab in exact ordered names/labels/count; preserve selected/enabled/tooltips and history adjacency checks. |
| `curvyo-editor-wasm::acceptance_0050_tester ac1_the_strip_is_file_document_style_layers_history_and_layers_is_never_dimmed` | Include seventh Snapping tab in exact ordered names/labels/count; preserve selected/enabled/tooltips and history adjacency checks. |
| `curvyo-editor-wasm::acceptance_0051_tester c2_tab_order_is_file_document_style_history_and_labels_and_tooltips` | Include seventh Snapping tab in exact ordered names/labels/count; preserve selected/enabled/tooltips and history adjacency checks. |
| `curvyo-editor-wasm::history_tab_session the_history_tab_exists_and_the_shortcut_selects_it` | Include seventh Snapping tab in exact ordered names/labels/count; preserve selected/enabled/tooltips and history adjacency checks. |
| `curvyo-ui-core::acceptance_0043_tester c1_the_tabs_in_order_with_names_and_tooltips` | Include seventh Snapping tab in exact ordered names/labels/count; preserve selected/enabled/tooltips and history adjacency checks. |
| `curvyo-ui-core::acceptance_0050_tester the_strip_has_layers_fourth_and_never_dimmed` | Include seventh Snapping tab in exact ordered names/labels/count; preserve selected/enabled/tooltips and history adjacency checks. |
| `curvyo-ui-core::acceptance_0051_tester c45_tabs_found_by_name_order_file_document_style_history` | Include seventh Snapping tab in exact ordered names/labels/count; preserve selected/enabled/tooltips and history adjacency checks. |

## Final Rust execution outcome

Final broad command ran **9499 tests: 9498 passed, 1 failed, 117 existing skips**
in 427.841s. The remaining failure was the second, label-list assertion in
`acceptance_0051_tester::c2_tab_order_is_file_document_style_history_and_labels_and_tooltips`:
its name list already included Snapping; its expected labels still stopped at
Fonts. Adding the exact Snapping label changes no production or behavior oracle.
Afterward `cargo test -q -p curvyo-editor-wasm --test acceptance_0051_tester`
passed **38/38**, and `cargo clippy -p curvyo-editor-wasm --all-targets
--message-format=short -- -D warnings` passed. Final test evidence is the union
of unchanged broad-run passes and that corrected full suite; there was no
single all-green broad command after the label-only edit. Lead explicitly
accepted this bounded verification instead of repeating unaffected 7-minute tests.
All 22 new acceptance tests and existing 2 performance tests passed in the final
broad run. The 117 skips are the pre-existing nextest selection, not new ignores.

## Completed gate and final bridge verification

| Required check | Final evidence |
| --- | --- |
| fmt | `cargo fmt --all --check`: exit 0 after final edits; `git diff --check`: clean. |
| Native all-target clippy | Workspace exit 0; affected editor all-target clippy exit 0 after final test-oracle correction. |
| Native tests | 9498 unchanged broad passes plus affected suite 38/38 after exact label correction; 22 independent acceptance tests all passed. Full-run details above, not a claim of one all-green invocation. |
| WASM build / clippy | All 9 core/editor packages: both exit 0 on final bridge source. |
| Dependencies / docs | `cargo deny check` exit 0; warning-denied workspace rustdoc exit 0. |
| Frontend | Final regenerated `npm run build:wasm` and `npx tsc -b --noEmit`: exit 0. `npm test`: 633 passed, 56 existing skips, 0 failed. `npx oxlint`: exit 0 with existing warnings. |
| Browser | Tester final `snapping.cjs`: PASS on regenerated WASM, all listed interactions and no JS page errors. |
| Review | Architecture APPROVE and UX APPROVE; see review.md / ux-notes.md. |

The first wasm-clippy attempt exposed 18 WASM-only bridge lints: 16 unnecessary
`getter_with_clone` attributes on Copy fields and the primitive settings setter's
argument/bool count. Implementer removed the Copy getter attributes (default
getters preserve the API) and documented narrow ABI-specific allowances on the
setter. Tester reran all-nine WASM build/clippy, regenerated frontend WASM,
typechecked the frontend and executed the browser script. No native engine or
session behavior changed; the completed native tests remain applicable. Native
rustdoc does not compile those WASM-only attributes. No failure was hidden by
loosening workspace lint policy or test assertions.
