# Architect review: 0069 tiling

Date: 2026-11-02. Branch `story/0069-tiling` at `be3e4102`, reviewed read-only against
[ADR 0028](../../docs/adr/0028-stored-tiling-nodes.md), [`adrs.md`](adrs.md) and `CLAUDE.md`
§5/§6 (`git diff origin/main...HEAD`; no build, no tests run).

**Verdict: CHANGES REQUESTED.** Two blocking findings. Both are small, local fixes in
`curvyo-document-core` and need no format or ADR change. Everything else holds or is
non-blocking.

## Blocking

1. **Ungroup of any group releases every tiling nested inside it.**
   `curvyo-document-core/src/tiling_copy.rs:13-22` (`strip_tiling`) walks every descendant
   group. `groups.rs:219` calls it for *every* ungrouped group. So ungrouping a plain group that
   holds a tiling (for example a tiling grouped with a frame) silently strips that tiling's
   recipe. Release of a tiling that holds a tiling the maker put into it also strips the inner
   one.
   Rule: ADR 0028 §7 ("Ungroup **of a tiling group** strips the same registers"). Releasing a
   tiling as a side effect changes the stored model without the maker asking.
   Fix: strip only `group`'s own registers (`tiling`, `tiling_sources`, `tiling_cell`), the
   `tiling_cell` of its direct children, and the `tiling_member` of those children's direct
   children. Do not recurse further. Add a test: group a tiling with a path, ungroup the outer
   group, and check that `Document::tiling(inner)` is still `Some`.

2. **Re-applying a text copy erases the instance's node registers.**
   `curvyo-document-core/src/tiling_write.rs:333-339` deletes every meta key except the tiling
   keys and `name`. That includes `visible`, `locked` and `guide`. A maker who hid or locked a
   text-copy instance loses that on the next Apply whenever the text differs.
   Rule: ADR 0028 §5. Apply owns placement and the variance's own style, not visibility or
   lock. The path-copy and image-copy routes keep these registers.
   Fix: remove only the text, style, shape and opacity keys, as `clone_write::clear_clone_keys`
   does for paths. Add a test: lock a text-copy instance, edit the source text, re-apply, and
   check the instance is still locked.

## Non-blocking (one follow-up commit on this branch, before the PR)

3. `tiling_write.rs:266-357` `update` is about 91 lines. `tiling_regenerate.rs:252-319`
   `regenerate` (67 lines) and `tiling_commands.rs:208-270` `plan_tiling` (62 lines) are just
   over the limit. Rule: §5, about 60 lines. Fix: split `update` into one function per
   `MemberKind` arm (it already matches on them), and justify the other two in the PR.
4. Dead code at `tiling_regenerate.rs:158,168`: `let (node, meta) = ...; let _ = node;`. Fix:
   use `let (_, meta)`. The `RefCell<Vec<TreeID>>` for `doomed` (`:21`, `:43`) is interior
   mutability without a need. Fix: pass `&mut Vec<TreeID>`. §5 asks for the simplest form.
5. The public API is wider than its users. `hue_shifted`, `TILING_WRITE_TOLERANCE_MM`,
   `TILING_GROUP_NAME` (`document-core/src/lib.rs:266-271`) and `FULL_TURN_TOLERANCE_RAD`,
   `DRAWS_PER_CELL`, `MIN_SCALE_FACTOR`, `circle_angle`, `cell_keys` and `CellVariance`
   (`ui-core/src/tiling/mod.rs`) have no caller outside their own crate. Some have no caller
   outside their own file. Fix: make them `pub(crate)`. Keep `Pcg32` and `cell_variance`
   public, because the integration tests use them.
6. The radius handle does not snap. `session/tiling_view.rs:190` uses `point`, not the
   `snapped` it computed one line earlier. Criterion 14 says both handles snap. Fix: use
   `snapped`.
7. A reopen hides *every* leaf of the tiling except the sources
   (`session/tiling.rs:160-176`). That includes unkeyed children and hand-converted instances,
   which Apply leaves alone and which stay visible after Apply. Criterion 34 says "managed
   instances". Fix: hide only the leaves under children that have a `tiling_cell`. This needs a
   small `Document` query, for example `tiling_instances(group)`.
8. `check_stored` (`tiling_model.rs:254-267`) counts the *inactive* layout too. A 1000 × 1000
   grid left behind after switching to Circle refuses the Apply with "too many instances". It is
   also damage on open. That matches ADR §10 as written. Fix in the bar: clamp or flag the
   inactive layout's count, so the refusal does not surprise the maker.
9. The PCG32 oracle is not independent. `tools/pcg32-reference/pcg32_reference.c` is a
   transcription of pcg-c by the same agent that wrote `pcg32.rs`. It is allowed in the repo:
   no build step, no CI job, no Cargo or deny impact, and an algorithm reimplementation is not
   a copy, so the AGPL-3.0 repository is unaffected. It lacks a header naming the upstream
   (pcg-c, Apache-2.0 OR MIT) and the fact that nothing builds it. `docs/guides/pcg32.md:37`
   builds into `/tmp`, which breaks §4 "nothing outside the project directory". `plan.md`
   names the wrong path (`tools/pcg32_reference.c`). Fix: add the header and build into
   `target/`. Preferably also add one upstream expected-output vector (pcg-c `test-high`
   oneseq-64 XSH-RR, seed 42) to `pcg32.json`.
10. The commit `be3e4102` (repair of clones of removed instances) has no test of its own. Fix:
    clone an instance by hand, reduce the columns, and check that the clone was repaired as in
    `0049` (it becomes a copy, with no dangling link).
11. The merge repair is tested by editing a file
    (`tests/tiling_format.rs:423`), not by two real peers. The test also does not assert *which*
    duplicate survived. Fix (tester): two documents apply concurrently and merge; the next
    Apply keeps the lowest node id of each key.
12. The bar sends the whole recipe back as JSON in degrees (`frontend/src/hooks/useTiling.ts:94`).
    So every angle goes rad → deg → rad on each field change and can drift by one ulp. An
    "edit, change back, Apply" then writes a recipe-only "Edit tiling" step. Instances are not
    rewritten, because the tolerance holds. This is cosmetic. Record it in
    `docs/technical-debt.md`, or compare recipes in `write_recipe` with a 1e-12 tolerance.
13. `TilingError::ContainsTiling` is returned for a layer as a source
    (`tiling_commands.rs:236`), so the maker reads "a tiling cannot be tiled". Fix: use
    `Locked` or a separate variant. Minor.

## Checked and holds

- **Storage (ADR §1, §2, §10):** one atomic map value `tiling` and one list value
  `tiling_sources`, written whole and only when changed (`tiling_codec.rs:178-194`).
  `tiling_cell = [row, column]` as I64 pair and `tiling_member` as an encoded node id. Damage
  on open is wired into `validate_tree` (`group_tree.rs:171`). There is one fixture per damage
  rule, plus a residue file that opens and keeps the dangling id. `CURRENT_FORMAT_VERSION` is
  32 with its doc paragraph (`document.rs:313-323`).
- **Cell keys and jump-ahead (decision 3):** `CellKey::index = row << 32 | column`, and the
  stream is advanced by `index · 8` in O(log n) (`variance.rs:68-71`). Seven draws always run,
  in the fixed order. Tests show that one channel or an added column leaves the other values
  unchanged.
- **Minimal-write regeneration (§3):** matrices and copy geometry are compared with 1e-9. Own
  style is written only when it differs. The recipe and sources are written last, only if
  changed. An unchanged apply makes no commit (`TilingOutcome::written`, tested). Deletes run
  after every write, with the clone repair first. `Fit::{Fits, Replace, Foreign}` matches the
  keep, replace and leave-alone rules.
- **Refusal before the first write (§4):** recipe, sources, locks, depth, cells and
  conversions are all checked in `plan_tiling` before `convert_to_paths_uncommitted`. One test
  checks the document version after each refusal.
- **Release, duplicate, delete (§6, §7):** Release is one commit. Duplicate remaps sources and
  members, and an instance copied out loses its keys (`tiling_copy.rs:28-63`, wired at
  `objects.rs:443`). A deleted source gives `SourceMissing` and only Release.
- **Core purity (§6):** the tiling code in `document-core` and `ui-core` has no clock, I/O,
  threads or entropy. The seed comes from `crypto.getRandomValues` in the frontend. The
  document never calls the layout; `ui-core` computes the cells.
- **Units and tolerances:** `Length`, `Angle`, `Vec2` and the new `Percent` newtype appear in
  the recipe. Named tolerances: `TILING_WRITE_TOLERANCE_MM` and `FULL_TURN_TOLERANCE_RAD`.
  Golden comparisons use 1e-9 mm and 1e-12.
- **unwrap rules:** every non-test `unwrap` has an `// invariant:` line before it. Index
  accesses (`sources[0]`, `objects[0]`) are guarded by the preceding checks.
- **Module sizes:** all are under 500 lines, the largest being `tiling_model.rs` at 480. Each
  has a one-sentence doc comment.
- **Determinism, native and wasm:** the generator uses integers only. `mul_add` is correctly
  rounded on both targets, and only `sin_cos` may differ. CI runs `tiling_golden` on
  `wasm32-unknown-unknown` in Node (`ci.yml:536-538`). The budget runs in the release job.
- **Cargo:** `wasm-bindgen-test = "0.3"` is a wasm32-only dev-dependency, the same declaration
  as `curvyo-geometry-core` (MIT OR Apache-2.0). `proptest` moved to native-only
  dev-dependencies. There is no new runtime dependency, crate, trait or generic.
  `keyed<K>` has two concrete key types (`CellKey`, `NodeId`).
- **Budgets:** `tests/tiling_budget.rs` measures thread CPU time and asserts only in release
  builds. The measured numbers are in `plan.md`.
- **Earlier testers' tests:** the edits are mechanical (format pins, fixture skip lists, the
  damage-file list). `acceptance_0070` changed from `== 31` to `>= 31`, which is acceptable
  and matches the pattern of `tiling_format.rs`.

## Implementer deviations

| Deviation | Verdict |
|---|---|
| Text copies and text clones are placed straight; a source on a path loses its carrier in the instances | **Accepted (Proposal) for this story.** The alternative, cloning the carrier as an extra member and laying the text clone on it, needs text-on-path clone support that ADR 0022 does not have. PO: add a line to criterion 23 and to Out of scope. Lead: add an entry to `docs/technical-debt.md`, and show it at the demo. |
| Face centre turns instance k by θ<sub>k</sub> − start, so instance 0 is unturned | **Accepted.** It keeps "keep source on" and "off" consistent, and equals criterion 13 at the default start of 0°. PO: reword criterion 13. |
| Rotation variance runs counterclockwise on screen | **Accepted** for consistency with the circle angles of criterion 11. Note for the demo: the app's rotate readout is clockwise-positive (`design-system.md`). PO: state the direction in criterion 15. |
| Scale factor clamped to ≥ 0.01 | **Accepted.** Without it a stepped scale gives singular matrices, which `check_cells` would refuse. PO: add it to criterion 16. |
| A clone instance without `own_placement` still counts as fitting and gets the flag back (ADR §3 says "with own_placement") | **Accepted.** It follows from §5 (Apply owns placement). The architect will reword ADR 0028 §3 after the merge decision. |
| Circle centre stored as `Vec2` relative to the cell box centre (the ADR said `Point`) | **Accepted.** It is the correct type for an offset (criterion 14). |

## Resolution (implementer, 2026-11-02)

| # | Done |
|---|---|
| 1 | `strip_tiling` strips the dissolved group's own registers, its children's cell keys (only when it is a tiling) and their members' keys; a nested tiling keeps its recipe. Tests: `ungrouping_a_group_around_a_tiling_keeps_the_tiling`, `releasing_a_tiling_keeps_a_tiling_put_into_it`. |
| 2 | The text-copy rewrite keeps `name`, `visible`, `locked`, `guide` and `layer`. Test: `a_re_applied_text_copy_keeps_its_lock`. |
| 3 | `update` split into one function per member kind; `plan_tiling` split (`check_sources`, `check_conversions`); `regenerate` split (`apply_cell`). `recipe_value` (78 lines, one map literal) justified in `plan.md`. |
| 4 | `let (_, meta)`; `doomed` is a `&mut Vec<TreeID>`. |
| 5 | `TILING_WRITE_TOLERANCE_MM`, `FULL_TURN_TOLERANCE_RAD`, `DRAWS_PER_CELL`, `MIN_SCALE_FACTOR`, `circle_angle`, `cell_keys` are private to their crate. Kept public: `TILING_GROUP_NAME` (integration tests), `CellVariance` (the return type of the public `cell_variance`), `hue_shifted` (now used by the editor's preview ghost). |
| 6 | The radius handle uses the snapped point. Test: `both_circle_handles_snap_to_the_grid_and_the_modifier_suspends`. |
| 7 | New `Document::tiling_instances`; a reopen hides only the leaves under keyed children. Test: `a_reopen_hides_only_the_managed_instances`. |
| 8 | The bar cuts the inactive layout to the count limit before it takes a recipe. Test: `a_grid_left_over_the_limit_does_not_block_the_circle`. |
| 9 | Header names pcg-c (Apache-2.0 OR MIT) and that nothing builds the file; it builds into `target/`; `plan.md` path fixed. No upstream vector (see `plan.md`). |
| 10 | `a_clone_of_a_removed_instance_is_repointed_to_the_source` (re-pointed with the composed matrix, `0049` criterion 30). |
| 11 | Left to the tester (the wasm `c32_*` test already merges two peers). |
| 12 | Angles and the centre the bar sends back unchanged keep their stored value (`keep_untouched` in `tiling_json.rs`); test `a_field_sent_back_unchanged_keeps_its_stored_value`. |
| 13 | New `TilingError::IsALayer`. Test: `a_layer_cannot_be_tiled`. |
