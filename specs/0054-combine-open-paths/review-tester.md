# 0054 combine-open-paths: tester review

Verdict: PASS. Branch `story/combine-open-paths` at 179010ec, tests on `test/combine-open-paths`.
Acceptance tests: `acceptance_0054_tester.rs` in `curvyo-editor-wasm` (45 tests), `curvyo-render-core`
(3) and `curvyo-document-core` (6). Written from the specification before the implementation was read.
The fill oracle (nonzero winding, open outlines closed by their closing line) is written in the test.

| AC | Test (file: name prefix) | Result |
|---|---|---|
| 1, 4 | wasm `c4_click_hits...`, `c7_open_outlines_keep_direction...` | PASS |
| 2, 3 | render `c2_...`, `c3_...` (3 tests, both directions) | PASS |
| 5 | not covered by a dedicated test (transforms, style edits of a compound path are 0016's tests) | gap |
| 6, 8, 9, 10, 11 | wasm `c6_`, `c8_`, `c9_`, `c10_`, `c11_` | PASS |
| 7a-g | wasm `c7a_` to `c7g_`, `c7c_..._every_direction_and_order` | PASS |
| 12, 13, 14 | wasm `c12_*`, `c13_`, `c14_` (counts through the outcome; texts are frontend tests) | PASS |
| 15, 16, 17 | wasm `c15_*` (4), `c16_`, `c17_` | PASS |
| 18, 19 | wasm `c18_*`, `c19_*` | PASS |
| 20 | wasm `c20_` | PASS |
| 21, 22, 23 | wasm `c21_*`, `c22_*` (4), `c23_` | PASS |
| 24 | Offset ignores open outlines (decided, OFFSET-OPEN A); Fracture and Flatten (0037) not built | expected gap |
| 25 | Node tool unchanged: no new test, no diff in the Node tool | expected gap |
| 26, 27 | owned by 0024 | expected gap |
| 28, 29, 30 | doc-core `c28_c29_`, `c29_golden_`, `c29_earlier_golden_`, `c29_format_version_`, `c30_*` (4) | PASS |
| 31 | wasm `c12_combine_is_one_undo_step`, `c31_*` (2) | PASS |
| 32 | wasm `c32_budget_a/b/d` (release assert < 2 s; debug prints), golden counts | PASS |
| 33 | wasm `c33_` | PASS |

## Rewrites of earlier tester tests
All seven judged against the replacing 0054 criteria: correct, none weakened.
- `geometry-core/acceptance_0016.rs`: correct. The first `ac15_withdrawn` test compares open and
  closed results without asserting `Ok`; two equal `Err` would pass. Minor, the second test asserts `Ok` with 2 outlines.
- `editor-wasm/acceptance_0016_pr3_tester.rs`: correct (criterion 21, 23; counts 2/0 asserted per op,
  `Refused(Empty)` allowed only where the result is legitimately empty).
- `editor-wasm/acceptance_0035_tester.rs`: correct (criteria 6-8, 12, 13; 7b gives (2,1,0,2)).
- `editor-wasm/acceptance_0048_tester.rs`: correct (`with_holes` flags match the inputs; single-node drop per criterion 18).
- `frontend/acceptance_0048_0036_tester.test.ts`, `booleanTextSpec.test.ts`, `pathToolsSpecText.test.ts`: correct.
  The new sentences are pinned in `pathText.test.ts` and `booleanText.test.ts`.

## Gate
fmt, clippy (host and wasm32 for the four core crates and editor-wasm), nextest (7361 passed), cargo deny,
cargo doc, tsc, oxlint, npm test (372), npm build, and all `performance-budgets` jobs of ci.yml in release: green.
`curve_intersections::recall::clear_crossings_are_found` (#152) passed in both runs here; not seen.
Not run: `boolean-wasm-golden` (needs wasm-bindgen runner), Windows/macOS (nightly).

## Observations
- `ci.yml` `performance-budgets` does not list `acceptance_0054_tester`; its budget tests assert only in release. Add it if wanted.
- Boolean tooltip dim note still reads "Select two or more closed objects." (booleanTextSpec); the spec does not change it, but it is no longer true.
- No test of AC 5 (move, duplicate, style edits on open compound paths) and of the overlay without closing line (UX notes); needs a seam or the render overlay API.
