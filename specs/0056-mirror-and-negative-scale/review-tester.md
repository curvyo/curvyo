# Tester review: 0056 mirror and negative scale

Branch `test/mirror-scale` on `origin/story/mirror-and-negative-scale` (10a9443). Tests:
`curvyo-editor-wasm/tests/acceptance_0056_tester.rs` (44 tests, written from the specification
before the implementation was read; one is `#[ignore]` for the release budget).

**Verdict: FAIL on criterion 27 only** (the "at most 1.1 times a plain move" clause). Everything else passes.

## Gate

| Step | Result |
|---|---|
| `cargo fmt --all --check` | green |
| `cargo clippy --workspace --all-targets -- -D warnings` | green (needs `frontend/dist`, built first) |
| `cargo nextest run --workspace` | 6971 passed, 70 skipped |
| wasm32 build and clippy, five core crates | green |
| `cargo deny check`, `cargo doc -D warnings` | green |
| ci.yml release job (`performance-budgets`) | all green |
| `npm run build:wasm`, `tsc -b`, `npm run lint`, `npm test` (362), `npm run build` | green (lint: warnings only) |
| `mirror_budgets` (ui-core, ignored, release) | passes its own gates; flip 1.39x and release beyond zero 1.41x a move |
| `ac27_budgets_5000_objects` (session level, ignored, release) | FAIL, see below |

## Criteria

| AC | Tests | Result |
|---|---|---|
| 1 | ac01 | pass (layout, 28 px, tooltips: frontend, not covered here) |
| 2, 29 | ac02, ac05, ac29 | pass |
| 3 | ac03 | pass (state and no write); the notice text is produced by the frontend, not the session |
| 4 | ac04 x2 | pass |
| 5 | ac05 | pass |
| 6, 7, 8, 9, 11, 12, 13 | ac06..ac13 | pass |
| 10 | ac10 | pass, deviation below |
| 14, 15, 17 | ac14, ac15 x2, ac17, edge_* | pass |
| 18..22 | ac18, ac19, ac20 x2, ac21_ac22 x2 | pass |
| 23 | ac23 | pass |
| 24, 25 | ac24, ac25 x2 | pass |
| 26 | ac26 | pass |
| 27 | ac27 | FAIL: flip 1.34 to 1.39 x a move, release beyond zero 1.41 x (ui-core) and 2.8 x (session level, 1156 ms against 417 ms); absolute budgets (2.5 s) met; group box 1.8 ms and preview step 4.2 ms met |
| 28 a..j | ac28 x4, ac06_ac18, ac14_ac17 | pass (at +-1e5 mm: edge_far_coordinates) |
| 30 | not tested (docs row) | - |

## Findings

1. **AC27, 1.1x clause.** Flip and release beyond zero share `Document::transform_objects`, which
   costs 1.2 x `translate_objects` even for a positive uniform scale (implementer's own note in
   plan.md). Spec says "at most 1.1 times the commit of a plain move". Needs either a lighter writer
   or a customer decision to loosen the clause. Reproduce: `cargo test --release -p curvyo-ui-core
   --test mirror_budgets -- --ignored --nocapture`.
2. **AC10 readout text.** The readout is "-12.0 x 10.0 mm" (existing format with U+2212), the spec's
   example is "-12.5 mm x 8.0 mm" (unit after each number). Sign and second line ("Mirrored left to
   right") are right. Cosmetic; align the spec or the code.
3. **UX note "Announcement".** "Resized. Mirrored left to right." after a release: nothing in the session
   (`selection_announcement` is empty after a resize through zero). Not an acceptance criterion; check in the UX review.
4. The zero-size notices (three texts) and the chip note lines match the spec exactly.

## Superseded tests (PR #139): verdicts

| File | Verdict |
|---|---|
| ui-core `acceptance_0005.rs` | Correct rewrite. The `sx >= 0` property became "extent = |sx*w|"; the pinned-edge property accepts min or max edge (the normalised box), slightly looser but right. The stroke factor test is exact (`-1, 4` gives 2). |
| ui-core `acceptance_object_transform_refinements.rs` | Correct: NotPositive cases removed; "not a number" kept. |
| ui-core `multi_object_transform.rs` | Correct and exact (positions computed from the factor). Typed 0 / negative now checked through `resolve` and `zero_recovery`, not through a commit; the commit is covered in `negative_scale_entry.rs`. |
| wasm `acceptance_0005.rs` | Correct. One case of "stroke never zero when collapsing" changed from (-30,-30) to (-10,-5) because the old one now grows the stroke; acceptable. |
| wasm `acceptance_0005_stroke_switch.rs` | Correct (drag onto the edge: 0.01 mm, stroke floor). |
| wasm `acceptance_0019_stretch_tester.rs` | **Partly weakened.** The typed refusals were rightly dropped, the circle test is exact. But `ac22_a_negative_factor...` now only checks "x is on the other side, within 30 mm" instead of exact positions, and drops the handle assertions. My ac18/ac06 tests cover exact positions for a group; no exact check for converted paths under a stretch through zero. |
| wasm `acceptance_0019_tester.rs` | Correct (exact positions for the two squares). |
| wasm `acceptance_0047_part_a_tester.rs` | Strengthened (adds the leftmost = min - 50 check). |
| wasm `acceptance_corner_radii_tester.rs` | Correct; `mirrored()` follows AC20. The ratio check `r1/r0 == 2` is skipped for flipped corners instead of checking 0.5: small loss. |
| wasm `acceptance_otr_tester.rs` | Correct (four NotPositive cases removed; the array shrank 9 to 5). |
| wasm `acceptance_unified_editing.rs` | Acceptable: a tiny finite pointer is now a real drag that converts the star, so the inner-handle check moved to a fresh star. |
| wasm `multi_object_transform_gestures.rs` | Correct (width 50 at x -30 for sx = -0.5). |
