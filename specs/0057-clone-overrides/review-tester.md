# Tester review: 0057 clone overrides

Verdict: **FAIL** (one finding, criteria 3 and 7). Everything else in scope passes.

Tests: `curvyo-document-core/tests/acceptance_0057_tester.rs` (41), `curvyo-editor-wasm/tests/acceptance_0057_tester.rs` (8),
`curvyo-ui-core/tests/acceptance_0057_tester.rs` (5, one a proptest over 64 random maps and thin shapes).
Written from the specification and `plan.md` before the diff was read.

## Finding

F1, criteria 3 and 7: a clone with own placement is not held back when a clone between it and the
original has a singular matrix. Test: `edge_a_held_clone_behind_a_singular_following_clone_is_still_held_or_unlinked`.
Reproduce: O (square 0..10), A follows O with M_A = (0, 0, 0, 1, 5, 0), C follows A with own placement and
M_C = translate (30, 0). Move O by (5, 3). C's drawn outline moves from y = 0..10 to y = 3..13 and C stays a clone.
Cause (read afterwards): `clone_matrices_after_maps` returns `None` for T_A (it needs M_A inverse) and the held-back
loop does `Some(Identity) | None => {}`, so C is neither written nor listed in `held_unlinked`. Criterion 7 wants C
unlinked first. A user can reach M_A singular by scaling a clone alone to zero width (0049 criterion 15 allows it).

## Smaller deviations

- Criterion 7 notice: the spec says the notice adds "2 clones became paths."; the session says
  "Zero size. 2 clones with own placement became paths." (`take_clone_hold_notice`). Wording only.

## Edits to earlier tests (judged against their specs)

All seven moves from format 13 to 14 (15 as "newer") keep every assertion and only change the number and the comment: OK.
`acceptance_0049_tester.rs` ac37 now refuses 15 and still checks 14 in the manifest: OK.
`clone_commands.rs` passes `d.own_placement_clones()` (empty in those tests), so the 0049 behaviour is tested unchanged: OK.
`acceptance_0017_block1_tester.rs` skips the new damaged-flag fixture, which is damaged by design: OK.
`acceptance_0050_tester.rs` skips `clone_overrides*`: acceptable (the golden stores opacities, which that scan asserts
absent in older builds), but the scan no longer looks at the new fixtures; low risk.

## Gate

fmt, clippy (host and wasm32, all core crates plus editor-wasm), nextest (7018 pass, 1 fail = F1), cargo deny, doc with
`-D warnings`, wasm dependency ban list, all release jobs of `ci.yml`: green except F1.
Budgets (release, this VM): own style on 10,000 clones 133 ms, own placement 105 ms (budget 250), read of 10,000 own-style
clones 73 ms (300), hold-back write of 10,000 clones 158 ms (target 3 s).
Frontend: build:wasm, tsc, oxlint (warnings only), build; `npm test` 3 runs: 360/360 each, no flakes.
Note: `npm test` fails if run under `ulimit -v 8000000` (node needs the address space); run it without.

## Not covered by tests of mine

Criterion 34 (two-peer merges), 33 object undo and Preview of an earlier version, 27/28/29 in the React components
(only the Rust models and the session were tested), 9 beyond the "nothing written during a transform drag" check,
Flip through the session (key not looked up; the writer is covered at document level).
