# Multiple pages acceptance report

Status: PASS — technical acceptance and full quality gate (2026-10-30).
Customer demo acceptance remains the customer's decision.

## Scope and method

The independent acceptance plan was prepared from the specification before
implementation review. Nine additional public-API acceptance tests live in
`curvyo-document-core/tests/acceptance_0059.rs` and
`curvyo-editor-wasm/tests/acceptance_0059.rs`. They supplement the implementer's
`tests/pages.rs` suites and the lead's actual Chromium interactions. The customer
explicitly excluded compatibility with old files; current-format persistence and
existing single-page editing behavior remain required.

## Criterion evidence

| AC | Executable evidence | Result |
| --- | --- | --- |
| 1 | `ac1_current_document_starts_with_one_default_frame_and_roundtrips_without_reads_writing`; `golden_multiple_pages_v26_roundtrips` | PASS |
| 2 | `ac2_ac3_ac6_active_row_changes_do_not_enter_history_and_add_uses_last_page`; `activating_offscreen_page_reveals_it_without_repeated_drift`; browser `pages.cjs` | PASS |
| 3 | `ac3_gap_wrap_and_reordered_last_frame_supply_dimensions`; active/last-page session test; browser Add | PASS |
| 4 | `ac4_overlapping_frame_resize_centers_exclusive_content_but_keeps_shared_and_partial`; browser active-only width and coordinate units | PASS |
| 5 | `ac5_ac6_ac13_reorder_rename_delete_keep_objects_and_have_individual_undo`; browser rename, F2/Escape, reorder, cancelled grip | PASS |
| 6 | Same core test; session active fallback test; browser delete/last-page protection | PASS |
| 7 | `frame_draw_list` in the 200-page CPU test; browser frame count, frame visibility and screenshots; UX review | PASS |
| 8 | `active_resize_moves_only_exclusive_content_and_rulers_follow_origin` | PASS |
| 9 | `ac9_ac13_frame_only_state_does_not_persist_and_content_moves_once_after_reopen`; `frame_drag_carries_contained_objects_is_cancelable_and_undoes_once`; browser frame-only/X/Y | PASS |
| 10 | `ac10_ac11_cross_page_clones_share_a_layer_and_still_follow_source_style_after_frame_move`; `page_move_keeps_cross_page_clones_stationary_and_shared_layers_intact` | PASS |
| 11 | Same cross-page clone/shared-layer tests | PASS |
| 12 | `ac12_unset_and_stale_context_select_all_but_hover_does_not_replace_pressed_context`; existing page/pasteboard selection test | PASS |
| 13 | Core no-op/refusal/history tests, session active-state tests, existing page history preview | PASS |
| 14 | `two_hundred_pages_cpu_budget`; browser `many-pages.cjs` | PASS |

## Regression investigation

- A real damaged-size regression was found among initially classified oracle
  updates: a persisted width of `1.0 - 1e-12` bypassed the strict read boundary.
  The implementer repaired the page reader. The existing unchanged damaged-size
  assertions now pass, including the no-write guarantee.
- Existing readable-export comparisons now assert the additive `pages` member
  explicitly, then compare all prior members. The unnamed-node flag check is
  scoped to objects because page frames necessarily have names.
- Nine existing golden suites retain their original fixture bytes. Their expected
  newly allocated node IDs compensate for exactly four initial page-register
  operations through `tests/support/pre_pages_ids.rs`; anchor IDs, geometry,
  styles, references, flags and ordering remain asserted. This avoids rebuilding
  old fixtures or weakening content assertions.
- Drawing-import and SVG-import version expectations now include format 26.
  The focused regression run passed 95/95 tests.
- The first workspace Nextest attempt stopped in the existing desktop fresh-process
  test because this host exports `WEBKIT_DISABLE_DMABUF_RENDERER=1`. The complete
  suite was restarted with that variable unset, preserving the assertion.
- That sandboxed run reached 7,324 passed tests before the existing SVG-font
  forked property test aborted inside `wait-timeout`: writing its signal pipe
  returned `EPERM`. The identical complete suite then passed outside the sandbox:
  9,400 tests passed, 114 existing ignored tests skipped, 315.012 s. No test was
  skipped or weakened to address the failure.
- Initial new-test compilation found an API return-type mismatch, corrected in
  the test. Full-gate Clippy found two empty-vector assertion style violations;
  those test-only corrections passed the subsequent identical Clippy command.

## Gate commands and results

Environment: Linux 7.2.11-arch1-1, Rust 1.99.0, Node v26.12.0.

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --message-format=short -- -D warnings` | PASS after test-style correction |
| `env -u WEBKIT_DISABLE_DMABUF_RENDERER cargo nextest run --workspace --status-level fail` (outside sandbox) | PASS: 9,400 tests; 114 existing skipped |
| `cargo build -q --target wasm32-unknown-unknown <packages>` | PASS |
| `cargo clippy --target wasm32-unknown-unknown <packages> -- -D warnings` | PASS |
| `cargo deny check` | PASS: advisories, bans, licenses, sources |
| `RUSTDOCFLAGS="-D warnings" cargo doc -q --workspace --no-deps` | PASS |
| `cd frontend && npm run build:wasm` | PASS |
| `cd frontend && npx tsc -b --noEmit` | PASS |
| `cd frontend && npm test` | PASS: 633 passed, 56 existing skipped |
| `cd frontend && npx oxlint` | PASS: exit 0, existing warnings |

`<packages>` was derived from `cargo metadata --no-deps --format-version 1`:
`-p curvyo-document-core -p curvyo-editor-wasm -p curvyo-geometry-core
-p curvyo-image-core -p curvyo-render-core -p curvyo-svg-core
-p curvyo-text-core -p curvyo-ui-core -p curvyo-vectorize-core`.

Local detailed logs: `/tmp/pages-acceptance.log`, `/tmp/pages-oracles.log`,
`/tmp/pages-gate-*.log`, `/tmp/pages-gate-results.txt`.

Final `cargo fmt --all --check` and `git diff --check`: PASS.
The remaining build/documentation/frontend commands ran outside the sandbox
with access to local caches and advisory metadata; no checks were omitted.

## Browser evidence and performance

The lead initially drove Chromium against the built editor. Both browser
scripts passed without JavaScript page errors. After the final frontend build,
the tester exclusively reran `pages.cjs`, adding focused-row history coverage:
Ctrl+Z / Ctrl+Shift+Z undo/redo rename, and Ctrl+Z / Ctrl+Y undo/redo reorder.
This regression passed with no JavaScript page errors after the implementer
fixed the page row swallowing unrelated key events. Permanent copies and optional
runner instructions are in `frontend/tests/browser/`; no new package dependency
or CI requirement was introduced.

The CRUD check covers active row, visible-frame navigation, rename commit and
Escape/F2 focus, active-only resize, reorder, cancelled grip movement, frame-only
switch, coordinate unit conversion, delete and final-page protection.
The 200-page check adds 199 frames, verifies 200 rows and canvas frame labels,
activates first/last and performs a rectangle drawing interaction. Its approximately
37.5 s wall duration measures automation overhead and is not the CPU budget.
Screenshots: `target/browser-check/pages.png` and `many-pages.png`.

The Rust performance assertion uses thread CPU time: page addition below 100 ms,
switching and frame drawing below 20 ms average. These assertions passed in the
final workspace test run. Development measurements recorded in `plan.md` were
22.7 ms add, 5.05 ms average switch and 4.83 ms average draw (thread CPU, debug).

Final browser command (repository root):

```sh
NODE_PATH="$PWD/target/browser-check/node_modules" \
PLAYWRIGHT_BROWSERS_PATH="$PWD/target/browser-check/browsers" \
LD_LIBRARY_PATH="$PWD/target/browser-check/runtime/usr/lib" \
node frontend/tests/browser/pages.cjs
```

The final tester run is logged in `/tmp/pages-browser-final.log`. Browser
performance timing remains observational; all enforceable CPU budget assertions
passed unchanged in the complete Rust suite. No new tests were ignored.
