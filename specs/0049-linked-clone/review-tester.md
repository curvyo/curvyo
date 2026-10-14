# Tester review of 0049 linked clone (on 0050, format 13)

Branch `test/linked-clone` (from `scratch/linked-clone-on-0050`, tip `c070568`). Date 2026-10-14.

**Verdict: FAIL.** One blocking finding (F1): a CI release test fails, and it points at a real merge defect.
Everything else passes. The full local gate is green except that one release test.

## Method

- Acceptance tests were written from `specification.md` (and the clone parts of `0050` criteria
  7, 24, 25, 26, 28, 36, 38, 45) before the implementation was read. Only the names of the public
  seams were looked up. Hostile files are forged through Loro against the stored form
  `clone_link = {origin, matrix}`.
- Files: `curvyo-document-core/tests/acceptance_0049_tester.rs` (36 tests),
  `curvyo-editor-wasm/tests/acceptance_0049_tester.rs` (44 tests). 80 tests, all pass.
- "Independent" below means my own test. "Implementer only" means I did not test it myself and rely on
  the implementer's tests, which I did not review.

## Per criterion

| AC | Result | Where |
|---|---|---|
| 1 | PASS | doc `ac1_*`, session `ac1_ac5_*` (order, identity matrix, selection, one commit `linked_clone`) |
| 2 | PASS | session `ac2_*` (silent conversion, same id, one commit), `ac2_a_rounded_rectangle_*` (8 nodes). Ellipse, polygon, star not tested |
| 3 | PASS | doc `ac3_*`, session `ac3_*` |
| 4 | PASS | session `ac4_*` (group, none, Node tool). Mixed path plus group not asserted separately |
| 5 | PASS (text only) | notices exact, no "No undo yet". `role="status"` and the 3 s timer are frontend: not tested |
| 6 | PASS | session `ac6_*` sweeps 37 keys times 8 modifier sets on a path and on a clone: no key makes or unlinks a clone |
| 7 | PASS | session `ac7_*`, doc `ac7_*` (square 10, M = scale 2, translate 30: 50..70 x 20..40; handles use L only; open stays open). Compound hole: implementer fixture only |
| 8 | PASS | doc `ac8_*`, session `ac8_*` (root fill edit shows on clone and chain clone) |
| 9 | PASS (style level) | doc `ac9_*`: clone style equals root's under scale 4 and 2. Drawn stroke width on canvas not measured |
| 10 | PASS (move) | doc/session `ac10_and_12_*`. Node/handle drag of an original not tested |
| 11 | PASS (partial) | session `ac11_*`: nothing written during the drag, clone follows on release. The blue preview outline and the 500 cap are not tested |
| 12 | PASS | doc and session `ac12_*` (rotated M: original 5 mm right moves clone 5 mm down; translate-only moves right) |
| 13 | PASS | doc `ac13_*` (merge of original edit and clone transform; same-clone conflict gives one whole link) |
| 14 | PARTIAL | clone moved by gesture (`ac15_*`), group box with clone selected works. Typed M/R/S/K entries on clones tested only for S (`ac16a`); skew/rotate/stretch not tested |
| 15 | PASS (move) | session `ac15_*` (only `clone_link` changes, original untouched, same label as a path move). Note: label is `translate_objects` as for every move, not `transform_objects`; consistent with paths, not a defect |
| 16 | PASS (partial) | doc/session `ac16_*` (move with original keeps relation), `ac16a_*` (typed size, scale about box centre: clone keeps relative place). Spec cases (b) chain and (c), and the singular-matrix unlink: not tested |
| 17 | PASS | session `ac17_*` (O 1 mm, A and B scale 1000, move of O by 5000 refused as a whole, notice names "Clone of ..."; a small move works) |
| 18 | PASS | session `ac18_*` |
| 19 | PASS | doc/session `ac19_*` (copy is a sibling with moved matrix, above its source; copy of a path original is independent; copy of a middle clone is a clone of O) |
| 20 | PASS | session `ac7_*` (click, marquee). Lasso and Alt-click cycle not tested |
| 21 | PASS | session `ac21_*`: "Clone of Path xxxxxx", "Clone of Clone of ...", "2 clones", "3 objects" (read from `style_scope().subject`). The 120/80 px ellipsis is frontend: not tested |
| 22 | PASS (data) | session `ac22_ac56_*` (`clone_panel().block`, depth, root). The buttons and wording are frontend: only the existing frontend test |
| 23 | PASS (data) | session `ac23_*` (followers counts direct clones only) |
| 24 | PASS | session `ac24_*` (edit goes to the others, Mixed ignores clones, clones-only writes nothing) |
| 25 | PASS | session `ac25_*` (one step up, root, clones; no commit; each target once; refusal text exact for one clone; partial text exact). For several clones the refusal reads "The originals are hidden or locked. Show or unlock them in the Layers tab. Nothing was changed." (plural; spec gives only the singular; accepted as reasonable, see F4). Hover box (Proposal) not tested |
| 26 | PASS | session `ac26_*` (Node tool: `clone_only`, no change from drags/insert/join/split; double-click keeps the tool). The 3 s hint chip and Pen continue/connect not tested |
| 27 | PASS | session `ac27_*` |
| 28 | PASS | doc `ac28_*`, session `ac28_*` (same id/place/selection, outline within 1e-9, fresh unique anchor ids, root style, rotation 0, label `unlink_clone`, notices "Unlinked 1 clone." / "Unlinked 3 clones.") |
| 29 | PASS | doc `ac28_*`, `ac55_*` |
| 30 | PASS | doc `ac30_*` |
| 31 | PASS | doc/session `ac31_*` (P with C1, C2; P and C1; chain middle, root, A+O). Notice pieces exact: "3 clones became paths.", "1 clone now follows its next original.". Boolean/Combine/Split etc. removals: implementer guard test only |
| 32 | NOT INDEPENDENTLY TESTED | implementer only |
| 33 | PASS | session `ac33_*` (Union of a clone and a path equals Union of the unlinked clone and the path). Other commands: implementer only |
| 34 | PASS (partial) | doc `ac34_*` (link holds across Group). Group transform cases not tested |
| 35 | PASS | doc `ac35_*` (nothing/primitive/group origin, self-loop and rings 1/2/3/7, chain behind an orphan, 12-deep chain not orphan, merge delete-vs-clone, same result on both peers), session `ac35_*` (Open removes in one step `remove_orphan_clones`, notice plural, project dirty), golden orphan+ring fixture opens |
| 36 | PASS (with F1) | session `ac36_*`, `ac31_*` (undo of Delete restores original and clones with matrices, undo of middle-clone delete, undo/redo of Linked clone and Unlink). Operation names in the history guard: implementer only. Case (a) "target no longer exists" not tested. F1 is a merge-with-undo failure |
| 37 | PASS | doc `ac37_*` (bit-exact matrices, order, chain), newer version refused (14 refused, 13 and 12 open), session `ac37_*` (open does not dirty). Golden `clone_v13.curvyo` exists (implementer) |
| 38 | PASS | doc `ac38_*`: 12 damaged forms refused with `Damaged`, ±1e7 boundary opens |
| 39 | PASS | doc `ac39_*`: ring of 10,000 (all orphans, removed), chain of 10,000 (resolves, depth 10,000), 30,000 clones of one original. 100,000 clones: implementer release test `clone_hostile_sizes` passes |
| 40 | PASS | doc `ac40_*` |
| 41 | PASS | doc `ac41_*` (read of a clone equals read after unlink, style equal) |
| 42 | n/a | carried by `0024` |
| 43 | PASS | release `clone_budget`: read 10,000 direct 50 ms, 10,000 in chains of 8 41 ms (budget 300); first frame 226 ms (1 s) |
| 44 | PASS | frame after the original of 10,000 clones moved 62 ms (150 ms) |
| 45 | PASS | Linked clone of 10,000 599 ms (5 s); Unlink of 10,000 passes in the same test |
| 46 | measured | 29 bytes per clone |
| 47 | PASS | doc `ac47_*` (copied set relinks to the copy; outside clone stays on source; independent edits) |
| 48 | PASS | doc `ac48_*` (copied chain; clone copied without original keeps source original; origin copied without clone) |
| 49 | PASS | session `c0050_28_*`: alpha 102 / 51 / 102 for root, clone in a 50 % layer, chain clone; after root opacity to 100 %: 204 / 102 / 204 |
| 50 | PASS | doc `ac50_*` (A 30..40, B 60..80 x 0..20, composed (2,0,0,2,60,0)) |
| 51 | PASS | doc `ac15_and_51_*` ((b) and (c); (a) and (d) via `ac8`, `ac10`) |
| 52 | PASS | doc `ac52_*` (eighth level ok, ninth refused all-or-nothing, root can be cloned). Refusal notice text and outline: implementer only |
| 53 | PASS (mine) / FAIL (implementer's) | my 1000 seeded sequences (no merges) hold. The implementer's own `clone_history ac53` with two replicas and merges FAILS in release: F1 |
| 54 | PASS | doc `ac54_*` (B follows O with (2,0,0,2,60,0), outline unchanged, ids/order kept), session `ac31_*` notice and undo |
| 55 | PASS | doc `ac55_*` (a), (b), (c), (d) |
| 56 | PASS (data) | session `ac22_ac56_*` |
| 57 | PASS | session `ac57_58_*` (one trace for one selected clone; none after deselect) |
| 58 | PASS | roles Clone/Original/Root for B -> A -> O with tags "Clone", "Original", "Root"; "Clone + Original" for a clone that is also an original; hidden original not traced and tag "Clone (original hidden)" (see F2) |
| 59 | PASS | Select and Node show marks, Pen does not, marquee drag hides them. Preview-outline case and the faint Proposal half: not tested |
| 60 | NOT TESTED | tag placement (200 generated selections): implementer only |
| 61 | PASS (partial) | session `ac61_*`: Ctrl+A over 300 clones: at most 200 clone traces, at most 12 tags. 10,000 case: release `ac61_ctrl_a_*` passes (144 ms vs 331 ms for paths). Rings under 6 px not tested |
| 62 | NOT TESTED | contrast and announcer text are frontend / visual |
| 63 | PASS | release doc `clone_budget` (50 ms and 41 ms against 300 ms). The one-transform-per-clone counter: implementer only |

0050 clone parts: 7 PASS (`c0050_7_38_*`: "Clone of ...", "Clone of Clone of ...", row action `select-original`), 24 and 25 PASS (`c0050_24_25_*`: hidden original not drawn, its clone drawn from its outline and style, hidden clone not drawn), 26 PASS (`c0050_26_*`: caption "Linked clones show the opacity of their original.", Opacity field writes nothing), 28 PASS (`c0050_28_*` alpha chain, row readout "50%"), 36 and 45 PASS (`c0050_36_45_*`: Delete layer unlinks the outside clone in the same step, undo restores it as a clone with its matrix), 38 (rowAction only; roving focus is frontend): not tested.

## Findings

**F1 (blocking). Two replicas with an undo in the middle can reach a state their peer refuses to merge.**
`cargo test --release -p curvyo-document-core --test clone_history ac53` fails deterministically
(`import_updates` returns `SyncError::Malformed`, `clone_history.rs:305`). The debug run uses 60 sequences and
passes; the release run uses 1,000 and fails, and the CI release job runs it (`ci.yml` line 372), so CI would be red.
Diagnosis (probe copy of the test, since removed): in the merged tree one node (`134@1`) has only the key `shape`
(a clone tag) and neither a `clone_link` nor `anchors`; `validate_clone_node` rejects it. Replica B's last
steps were `unlink_clone`, `undo`, `unlink_clone`, `linked_clone` ... `undo`; replica A concurrently ran `delete_objects`, `delete_layer`,
`linked_clone`. So an undo that puts a clone back (shape and link) merges key by key with a concurrent
write on A that removes the link or the shape, and the result is neither a path nor a clone. Spec
AC 13/53 say the link is one atomic register and merges never mix; AC 38 says such a file is damaged, so a
replica that receives it refuses the merge and the two replicas diverge for good. Reproduction:
`CARGO_BUILD_JOBS=4 cargo test --release -p curvyo-document-core --test clone_history ac53`.

**F2 (minor). The long tag "Clone (original hidden)" is dropped for small objects.** At the default zoom a 30 mm
clone (113 px) is wide enough for the 24 px rule but the 177 px tag meets a handle glyph in all six candidate
places, so no tag is drawn (the trace stays; AC 60(c) allows dropping). A 60 mm clone shows it. Reproduction: hide the
original of a 30 mm square clone, select the clone, read `Session::clone_marks().tags` (empty).

**F3 (minor). Selection is not visible through `style_scope().ids` for clones.** Clones-only selections give an empty
Style scope by design (AC 22); tests and any host that reads the scope as "the selection" must use the Layers rows. No
action needed; noted because it is easy to trip over.

**F4 (minor, wording). Refusal notice of Select original is plural for several clones** ("The originals are hidden or locked ...").
The spec shows only the singular. Probably intended; the spec could list both.

**F5 (coverage). Not independently verified:** AC 32, 60, 62; typed rotate/skew/stretch on clones (14); chain and singular-matrix cases of 16;
node-drag preview of 11; the 500-clone preview cap; lasso and Alt-click of 20; Pen continue/connect refusal (26).

## Changed earlier tests (judgement)

About 86 test files changed, +2,925 / -43 lines. All 43 removed lines were read.

- `Clone(_)` match arms (about 80 sites): added to existing matches next to `Primitive(_)`; panicking arms stay
  panicking (`panic!("no clone here")`), read arms read `resolved.style`. No assertion weakened. One label helper
  (`kind()` in `acceptance_0019_stretch_tester.rs`) maps a clone to the string `"primitive"`; it never sees one.
- Format version pins to 13: `acceptance_0047_part_a_tester` pins 13 exactly. `acceptance_0050_tester` ac46 and `layers_format.rs`
  changed from `== 12` to "manifest equals `CURRENT_FORMAT_VERSION` and `>= 12`" and "newer = CURRENT + 1". That is looser than an
  exact pin, but it is the pattern `0023`'s test already used when `0050` took 12, the exact value 13 is pinned in
  `acceptance_0047_part_a_tester`, in my `ac37_a_newer_format_*` and in the golden file, and the newer-file refusal got
  stronger (it now tests the real next version). Accepted, no weakening of what the test is about.
- 0017 fixture skip lists gained `clone_matrix_five_v13.curvyo` (damaged, correctly refused) and `clone_orphan_and_ring_v13.curvyo` (orphans have no
  style until repaired). Justified: the 0017 test reads styles of every fixture that opens.
- Three tests skip the orphan fixture (`acceptance_0051_tester` c43, `acceptance_0020_tester_more` old files,
  `project_dirty_0051` every fixture opens clean). AC 35 says Open marks the project changed and repairs in one step, so "clean after open"
  and "empty undo stack" cannot hold for exactly that file. The skip is by exact file name and the orphan behaviour is covered by my
  `ac35_*` session tests (dirty, one step `remove_orphan_clones`, notice). Accepted. The only cost is that these three no longer prove the orphan file opens without a panic; my
  `ac35_the_golden_orphan_and_ring_fixture_opens_without_hanging` does.
- Verdict: no assertion was weakened to make a test pass.

## Gate (run locally in this worktree)

| Step | Result |
|---|---|
| `cargo fmt --all --check` | ok |
| `cargo clippy --workspace --all-targets -- -D warnings` | ok (after frontend `npm run build`, which `curvyo-app` needs) |
| `cargo nextest run --workspace -j 4` | 6270 passed, 64 skipped |
| `cargo build` and `clippy --target wasm32-unknown-unknown -p` document-core, render-core, ui-core, geometry-core, editor-wasm | ok |
| `cargo deny check` | ok |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | ok |
| frontend: `tsc -b --noEmit`, `oxlint` (warnings only), `npm test` (310 pass), `npm run build` | ok |
| release: `clone_hostile_sizes`, document-core `clone_budget`, editor-wasm `clone_budget` (4 tests, `--test-threads=1`) | ok |
| release: `clone_history ac53` | **FAIL (F1)** |

## Re-check 2026-10-14

Verdict: PASS. Branch `test/linked-clone` on `origin/story/linked-clone` at d1e1919.

| Item | Test | Result |
|---|---|---|
| AC 53 / 13 / 36 (release `clone_history`, 3 runs, 15 tests each) | `clone_history.rs` | PASS x3 |
| AC 38 no link, no outline: orphan, repair deletes | `acceptance_0049_recheck_tester.rs` (core) `ac38_a_clone_with_no_link_and_no_outline...` | PASS |
| AC 38 no link, valid outline: same id, outline, style, place; clone behind it resolves | `ac38_a_clone_with_no_link_but_a_valid_outline...` | PASS |
| AC 38 no link, unreadable outline: deleted | `ac38_..._unreadable_outline_is_deleted` | PASS |
| AC 35 repair converges on every peer | `ac35_repair_of_the_same_state...` | PASS |
| AC 38 incomplete or empty link map still damaged | `ac38_a_present_but_incomplete_link...`, `ac38_an_empty_link_map...` | PASS |
| AC 35/38 notices on Open (both sentences, conversion only) | session `ac35_38_*` | PASS |
| 0050 55-57: Linked clone button applies by the Select bar's rule, never creates an Initial Layer, stays in its layer | session `toolbar_linked_clone_*` | PASS |
| Duplicate layer re-points clones to the copied originals; clone-only layer follows the same original; Duplicate of clone rows | session `duplicate_*` | PASS |
| 0038 Offset: New object takes the root's style, Change original turns the clone into a path (follower follows it, undo restores), AC 17 refusal | session `offset_*` | PASS |
| AC 14 typed M/R/S/K on a clone | `ac14_typed_move_rotate_size_and_skew...` | PASS |
| AC 16 b and c | `ac16b_and_c_chain_selected...` | PASS |
| AC 20 click, lasso, Alt-click cycle | `ac20_...` | PASS |
| AC 26 Pen does not continue or connect a clone | `ac26_the_pen_...` | PASS |
| AC 11 cap of 500 (500: previews drawn; 501: none, jump on release) | `ac11_more_than_500...` | PASS |

Not covered: AC 20 "holes of a compound origin are not hit" (needs a compound fixture).

Edits to 0050 tests: `curvyo-editor-wasm/tests/acceptance_0050_tester.rs` only adds a match arm for `ObjectSnapshot::Clone`, and the frontend 0050 tests add the `linkedClone` field. Two more edits are not only that: `curvyo-document-core/tests/acceptance_0050_tester.rs` and `layers_format.rs` stop pinning format version 12 and compare with `CURRENT_FORMAT_VERSION` (0049 took 13). The weaker check is acceptable only because `acceptance_0049_tester.rs` pins 13 and the newer-file refusal.

Gate (local): fmt, clippy, nextest (6586 passed), wasm32 build and clippy for the four core crates and `curvyo-editor-wasm`, deny, doc, all 12 release jobs of ci.yml, frontend tsc, lint (0 errors, warnings only), 330 node tests, build: green. The frontend `src/wasm-bindings` is git-ignored; run `npm run build:wasm` first or tsc reports missing offset methods.
