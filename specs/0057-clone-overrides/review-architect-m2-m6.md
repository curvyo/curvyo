# Architect review: 0057 clone overrides, milestones 2 to 6

Date: 2026-10-15. Branch `story/clone-overrides` at `d7394ac` (three commits on `origin/main`
`2f2859b`), diff `origin/main...HEAD`, Rust only. Read: `clone_transform.rs`, `clone_hold.rs`,
`outline_map.rs`, `clone_overrides.rs`, `clone_preview.rs`, the hold call sites in `objects.rs`,
`paths.rs`, `transform_objects.rs`, the diffs of `clone_write.rs`, `clone_repair.rs`,
`render-core/src/shared_outline.rs`, `ui-core/src/clone_colours.rs`, both `lib.rs`, and
`tests/clone_held_back.rs`, `tests/clone_overrides.rs`, both `clone_budget.rs`. Ran `cargo nextest`
for document-core, ui-core and render-core (3600 passed), the wasm32 build of the three core crates
(passes) and the document-core budget test in release.

## Verdict

**APPROVE**, with three small fixes on the branch before the PR (F1 to F3). No new crate,
dependency, trait or generic parameter; no I/O, clock or thread in a core crate; the three new
modules (`clone_hold`, `clone_overrides`, `outline_map`) each have a one-sentence responsibility
and stay below 200 lines.

## The held-back rule

- **One rule, one place: kept.** `clone_matrices_after(links, held, selected, g)` is a thin wrapper
  over `clone_matrices_after_maps(links, held, moved)`; the gesture preview (`ui-core`
  `clone_gesture`, `group_transform`), the four document writers and the document fit all go
  through it. An unselected held clone is `Moved::Identity` inside the chain walk, so its
  followers stay too (criteria 5a to 5c), and a singular T or one below `SINGULAR_DETERMINANT`
  unlinks it (criterion 7) with the same threshold the selected-clone path uses. The limit check
  refuses the whole command before any write (`plan_hold` returns `Err` first).
- **`outline_map` is numerically sound.** It fits on centred coordinates (large absolute positions
  cost nothing), uses a determinant threshold relative to `trace²` (scale invariant), and accepts
  the fit only if every anchor and handle end lands within `1e-6 · max(1 mm, extent)`. With exact
  affine input the normal-equation error stays far below that tolerance even for an outline of
  aspect 1e-6. Degenerate cases are handled deliberately: one point gives a translation; a
  collinear outline is extended conformally across its line, which leaves the drawn outline the
  same whatever the true map was; a collapse to a line gives a singular T and so the unlink of
  criterion 7; a changed anchor count or a non-affine change gives `None`, which the writers treat
  as an edit (criterion 4: the held clone follows the shape). Primitives are never origins
  (`clone_read::walk` marks them `NotOrigin`), so `drawn` returning `None` for them is correct.
- **Guard test.** `every_writer()` lists translate, rotate, resize and `transform_objects`; I
  found no other caller of `write_matrix` that should hold back (`document_size` is the fit, which
  moves every clone by design and has its own test). The list is by hand, so a new transform
  writer must add itself; the module doc of `clone_hold.rs` should say so (part of F3).

## L1 to L4

- **L1 done.** `keys_replaced_by_unlink` now lists both flags and `opacity`; `unlink_meta` writes
  the style source's register (`source_opacity`). Covered by `ac30_unlink_keeps_the_effective_style_...`
  and `ac31c_...`.
- **L2 done.** Copies take every meta key (flags and style keys come along; `ac32` checks
  duplicate with both states and an independent own style); the repair of a lost style source
  writes an own style (`style_copies`, criterion 31 (b)); object undo/redo restores the states
  field by field (`ac33`); a held unlink takes the source register.
- **L3 done.** `same_geometry` compares the whole `Style` after copying the four colour and
  opacity fields from the root, so every shape field and `fill.enabled` / `stroke.enabled` must
  match; the render-core test checks one tessellation for twelve coloured clones, equality with
  the unshared draw, and the fallback when a paint is switched on.
- **L4 done.** Both switches take the clones from one `read_objects()` (`live_clone_metas`).
  Release run here: own style 75 ms, own placement 59 ms for 10,000 clones (budget 250 ms; the
  implementer's 87 ms is the same order), read 50 ms, move holding back 10,000 clones 101 ms.

## New API

- `Document::own_placement_clones() -> HashSet<NodeId>`: public because ui-core needs `held` for
  the gesture preview; reads only meta maps. Accepted.
- `Document::take_clone_hold() -> CloneHold`: same take-and-reset pattern as `take_clone_repair`
  and `take_clone_refusal`. Accepted.
- `Document::recolour_clones(&[CloneColours])` and `CloneColours { clone, fill, stroke: Option }`:
  one commit, validated before the first write, fill always and stroke only where on, as
  `spread_colours` produces. Accepted; see F2 for a duplicated read.
- `clone_previews(objects, changed, cap, transform: bool)`: the flag decides drag kind (Select drag
  holds back, node drag does not). Two callers, two values; a bool is enough. Accepted.

## Fix on the branch before the PR

**F1. No test for the degenerate fits outside the happy path.** Rule: every geometric comparison
uses an explicit tolerance, and the outline-to-map fit is what decides between hold, unlink and
edit. `outline_map.rs` tests a square, a node edit, a collapse and a line at the origin. Fix: add
three unit tests: a one-anchor path (gives a translation), a square 1e6 mm from the origin mapped
by a rotation (fits, max error below 1e-6), and a 100 mm by 1e-4 mm rectangle mapped by a shear
(fits, and `plan_hold` stays within the limit).

**F2. The style source's opacity is read twice by hand.** Rule: §5 one responsibility per module;
`clone_write::source_opacity` exists for exactly this. `clone_overrides.rs` lines 91 and 143
repeat `tree.get_meta(tree_id_of(clone.chain.style_source)).map_or(1.0, ...)`. Fix: call
`crate::clone_write::source_opacity(&tree, clone)` in both and drop the `read_registers` import.

**F3. Held-back code lives in `objects.rs`, and the rule runs twice per translate.** Rule: §5
modules over ~500 lines need a reason; `objects.rs` was 798 lines on main and grows to 851.
`moved_clone_matrices` calls `clone_matrices_after` and then `plan_hold`, which calls
`clone_matrices_after_maps` again on the same input and ignores the first result's `held_back`.
Fix: move `hold_for_results` into `clone_hold.rs`, give `plan_hold` the already decided
`CloneMatrices` (or let `moved_clone_matrices` take `held_back` / `held_unlinked` from its own
call), and add to the `clone_hold.rs` doc that every transform writer calls it and appears in
`tests/clone_held_back.rs::every_writer`.

## Frage 4 (SKIP-NOTICE-PLACE)

Agree with the default: the panel's notice slot. It is where the held-back and unlink notices of
criteria 7 and 8 already show, so one place serves every clone notice, and it needs no anchor API
from the style sections. The choice is UI only and reversible in one PR; nothing in the core or
the format depends on it. Leave the anchor to the UX review.

Addendum 2026-10-15 (at `55c37e4`): F1 done and sound. `fit` now sums in the principal axes of the outline (atan2 of the covariance; any frame is correct when the spread is isotropic, so the angle needs no stability guard), the determinant has no cancellation, the full-fit cutoff `1e3·ε·trace²` does not depend on rotation, and the conformal line fallback runs only when the full fit misses the tolerance (`suu ≥ trace/2 > 0` there); the thin rectangle along the axes and turned 45° both fit.
