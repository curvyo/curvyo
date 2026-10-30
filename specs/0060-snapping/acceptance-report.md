# Snapping acceptance report

## PR #225 customer feedback — current verification

Status: **full gate PASS**, independent acceptance **31/31 passed**
(document4, engine14, session13), including all original22. Final feedback browser PASS
on regenerated WASM; UX screenshot review pending. This section supersedes the historical result below.
Independent feedback plan was written before implementation reads.

| AC | Current executed evidence |
| --- | --- |
| 1 | Original defaults/persistence plus axis peer merge/local undo; root format suite tests all four axis roundtrips, exported JSON and fieldwise no-op commits. |
| 2 | Prior rail browser remains applicable; lead feedback browser checks stored enabled state during temporary inversion. |
| 3–4 | Existing raw screen-distance hysteresis tests retained; new mask/held-state tests ensure disabled components cannot persist. |
| 5–6 | Existing target/atomic-point/finite-curve tests retained; independent one-axis nodes must be reachable in 2D, finite curves intersect the allowed line. |
| 7 | Existing creation/Node/Pen/move tests plus actual Ctrl-aspect resize: disabled coordinate prevents coupled snap, both-enabled reaches exact target. Existing tool constraints remain under broad regression. |
| 8 | Public rendered reference vertices establish idle marker appearance/cleanup; copy readout/badge regression and existing renderer screen-width tests. |
| 9 | Independent same-pointer Alt on/off XOR under both stored states and Ctrl combinations; no settings/history writes, Ctrl+Alt no object copy or Copy feedback, Ctrl-only restores feedback; Alt-at-press lasso/remove badge preserved. |
| 10 | Existing fields/target controls plus lead four-axis browser controls/undo and format normalization validation. |
| 11 | Existing object/dense-guide/mixed-composition CPU tests retained; new production public Session idle-hover benchmark measures 10k objects without per-frame rebuild. Architecture reviews cache reuse separately. |
| 12 | Original one-step commit/undo/cancel and clone tests retained; new typed placement/follow click/Pen one-step creation and exact undo/redo. |
| 13 | Original independent clone drag to original/sibling, overrides/link retained, remains unchanged. |
| 14 | Two new explicitly named malformed x/y fixtures assert Damaged in the old sweep; all seven damage fixtures and current-format absent defaults covered. |
| 15 | Original actual inactive-page origin/resize and exact fraction tests retained. All-visible default unchanged; no hidden-page API. |
| 16 | Independent all4 masks with nodes/lines, finite-curve intersections, normalized-direction scale invariance, actual aspect resize, axis peer merge/undo, save/open and root field tests. |
| 17 | No-button hover for Rectangle/Ellipse/Polygon/Star/Pen: marker, no drag/history/artwork; leave/Escape/tool/open/disable/both-off cleanup; Alt does not invert idle hover. |
| 18 | All4 shape typed placements ×4masks×hover/no-hover, press after stale different hover, exact geometry versus unsnapped reference; follow+click placement, first/subsequent Pen nodes without accidental handles; lead real-browser Enter/click confirmation. |

### New defects and oracle review

Public acceptance reproduced the UX copy-feedback defect before production fix:
Ctrl+Alt during a Select move still showed a plus badge. Regression now asserts
no Copy readout or badge, restoration with Ctrl-only, and unchanged initial
Ctrl+Alt lasso remove badge. Implementer fixed only effective-copy use in
move_indicators; tester made no production edits.

Architect's earlier direction-scale finding is supplementary white-box-derived
coverage: proportional direction vectors at 1e-8/1/1e8 cannot introduce movement
on a disabled coordinate. It is identified as post-plan review evidence.

Reviewed existing test changes: original acceptance initialization only gained
x/y true; no original behavior assertion removed. Format suite additions test
nine registers and seven malformed fixtures. Exact historical export expectations
now separately include x/y true; damaged-fixture inventory explicitly asserts
the two added files are Damaged. Shape test setup initially omitted a required
second typed dimension; then used the wrong Enter dispatch (frontend calls
place_placement, not generic key_down). Corrected test setup, preserved geometry
oracle. Resize setup initially used Shift centre resize; corrected to existing
Ctrl aspect constraint, then the strict geometry assertions passed.

### Commands, limits and gate status

Focused command: `cargo test -q -p curvyo-document-core -p curvyo-ui-core -p
curvyo-editor-wasm --test acceptance_0060`:4+14+13 passed. Full gate uses the same
nine-package command list recorded below, with `env -u
WEBKIT_DISABLE_DMABUF_RENDERER` and escalated process/cache access for established
nextest sandbox restrictions. Logs `/tmp/snapping-feedback-gate-*.log`.
Initial clippy stopped on four test-style lints (empty assertions/redundant str
conversion); equivalent assertions corrected before broad tests started.

Scope limits: not every tool×modifier×handle combination is independently
exhausted; existing regression suites supplement representative new public tests.
No independent cache-read counter was added; hover CPU plus architecture cache
review supply distinct evidence. Document replacement is tested by opening a
fresh session from saved data, not an unavailable in-place replacement method.
The fixed default labels and all-visible-page policy remain pending optional
customer choices. No unrelated interaction scope is inferred.

### Final browser evidence

Tester reran `frontend/tests/browser/snapping-feedback.cjs` on final regenerated
WASM using the optional runtime variables in the browser README: PASS, no JS
page errors. Added explicit assertions for a present live move readout without
Copy and zero copy badges during Alt holds. All four mask export/geometry tests,
axis undo/redo, hover, second-canvas-click and Enter placement, both Alt inversion
directions and Ctrl+Alt no-copy/stored-state checks passed. Screenshot
`target/browser-check/alt-false-true-held.png` now reflects corrected feedback;
axis and other Alt images were refreshed in the same run. The unchanged original
`snapping.cjs` was run by the lead before this handoff, not repeated by tester.

### Final feedback gate results

- `cargo fmt --all --check` and workspace all-target clippy with `-D warnings`: PASS.
- `env -u WEBKIT_DISABLE_DMABUF_RENDERER cargo nextest run --workspace --status-level fail --no-fail-fast`: **9519 passed, 0 failed, 117 existing skips**, 433.755s,496 binaries. All31 independent tests,19 format tests and3 performance tests included.
- Nine-package `wasm32-unknown-unknown` build and clippy `-D warnings`: PASS.
- `cargo deny check` and `RUSTDOCFLAGS="-D warnings" cargo doc -q --workspace --no-deps`: PASS.
- Frontend `npm run build:wasm`, `npx tsc -b --noEmit`: PASS on final source.
- Frontend `npm test`: **633 passed,0 failed,56 existing skips**; `npx oxlint`: exit0 with existing warnings.
- Current gate logs: `/tmp/snapping-feedback-gate-results.txt` and individual check logs. This feedback gate has a single all-green broad nextest invocation; historical bounded reruns below do not describe this final revision.

The three performance suites cover10k objects, dense curved guides and actual
idle Session hover, with <=2ms thread-CPU assertions. The independent mixed
10k guide+10k edge acceptance test also passed. Numeric idle-hover measurement
0.0183ms/frame is implementer handoff evidence; the tester independently ran its
budget assertion in the full gate, not a fresh printed timing sample. Cache
read/rebuild structure remains architecture-reviewed rather than independently
instrumented. No wall-clock responsiveness threshold substituted for CPU.

## Historical acceptance before customer feedback

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
