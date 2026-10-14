# ADRs for "Mirror and negative scale: two flip buttons, and scaling over to the other side"

A flip is the existing scale map with a negative factor. The group gesture of 0019
(`GroupMap::Scale { pivot, sx, sy }`) already carries signed `f64` factors, and
`PathSnapshot::scaled_along` already maps anchors and multiplies handle vectors per
component, so a path needs no new code. The work is: remove the zero clamps in the resize
math, decide stretch on magnitudes, mirror the primitives, add two buttons and one typed-entry
rule. **No new ADR, no new crate, no new dependency, no trait, no generic, no
`format_version` bump** (no stored length is negative, no matrix is written for an existing
kind; the file stays readable by the previous version).

## Depends on

- [ADR 0002 §3, §5](../../docs/adr/0002-document-model-units-and-svg-round-trip.md): lengths
  stay positive newtypes; the per-node affine transform of §5 stays unused for existing kinds
  (the app bakes transforms into geometry since 0019, and a mirror does the same). Document Y
  grows downward, so "clockwise" in a note below is as seen on the screen.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): a flip and a resize through
  zero are one commit, one undo step; the step name comes from `history_names.rs`.
- [`specs/0019-multi-object-transform/adrs.md`](../0019-multi-object-transform/adrs.md): group
  box, `GroupMap`, the shared scale rules and the all-or-nothing limit of 1e7 mm. This spec
  changes its decision D5 (clamp at 0) and criterion 20's stretch test.
- [`specs/0047-transform-polish/adrs.md`](../0047-transform-polish/adrs.md): `is_stretch`,
  conversion of polygon, star and off-axis primitives, the true bounding box.
- [`specs/0049-linked-clone/adrs.md`](../0049-linked-clone/adrs.md): the clone's matrix G·M
  and G·M·G⁻¹ (criteria 15 and 16); a reflection is a valid matrix with negative determinant.
  Applies only when 0049 is built; 0056 does not wait for it.
- [`specs/0050-layers-and-objects-panel/adrs.md`](../0050-layers-and-objects-panel/adrs.md):
  the selection never holds a hidden or locked object; a group carries locked descendants.

## Feature-local decisions

- **2026-10-14: signed factors end to end; no wall at 0, no collapse (customer: ZERO-RECOVER C).**
  In `curvyo-ui-core`: `resize_local_box` drops the three `.max(0.0)` (in `resized_extent` and
  the Ctrl branch) and returns signed `sx`, `sy` with `min <= max` normalised, so a caller that
  draws the box never sees an inverted one. The Ctrl branch takes the magnitude from the
  dominant axis and the signs from the free factors (criterion 8). Zero recovery (criterion 15)
  is one function next to `safe_factor`: a resolved extent with magnitude under 0.0005 mm
  becomes `0.01 mm * sign`, where the sign is that of the last preview frame whose extent was
  not recovered-from-zero (positive if none; positive for a typed 0). The "last sign" is one
  `f64` field of the drag state, reset at the press, set only by a frame with non-zero extent;
  preview and commit both call the function with it, so they agree. Under Ctrl it is applied
  to the dominant axis and the partner follows by the common factor (the partner is not
  rewritten); under Shift to the symmetric extent. It is not applied to a flip, to an axis
  whose start extent is under 0.0005 mm, or to the Node tool. An axis therefore never has
  factor 0 after a gesture, and "only undo restores it" no longer holds.
  *Checked against the code:* `safe_factor` returns `new/start` for any start above
  `f64::EPSILON`, so a 0.01 mm start gives a finite factor (20.01/0.01 = 2001 for the
  pull-apart in test 28i); its 1.0 fallback only applies to a start of 0, which the rule never
  produces. Precision: a factor of 1e3 to 1e4 multiplies the f64 rounding error of the
  coordinates; at ±1e5 mm the ulp is 1.5e-11 mm, so the error after a pull-apart is about
  1e-7 mm, below every tolerance used; at the 1e7 mm limit the ulp is 1.9e-9 mm, which is
  about 2e-7 relative to 0.01 mm and still harmless; far beyond that the 1e7 limit refuses
  first. A 0.01 mm object at 1e7 mm is the worst case and stays exact to ~1e-6 of its width.
  The 0.001 mm "grid" is not stored anywhere (it is the merge tolerance of the pen tool and
  the display rounding), so the threshold 0.0005 mm is a named constant of this rule, not a
  grid property; the spec wording was corrected.
- **2026-10-14: stretch on magnitudes.** `is_stretch` becomes
  `(sx.abs() - sy.abs()).abs() > 1e-9 * max(|sx|, |sy|)`. `stroke_or_radius_factor` becomes
  `sqrt(|sx*sy|)` (its clamp to 0 would turn a half turn into a zero-width stroke). Both are
  `pub(crate)`/`pub` in `curvyo-ui-core`; every caller (single object, group map, size chip,
  group entry) goes through them, so the three paths in criterion 28(e) share one rule.
- **2026-10-14: a mirror is "magnitudes first, then reflect".** `scale_primitive` /
  `extent_factors` in `group_transform.rs` apply `(|sx|, |sy|)` by the existing rules and
  then a reflection per negative factor, in one function with a table test per kind:
  rectangle (bounds, `rotation`, four radii), ellipse (rx, ry, `rotation`), polygon and star
  (frame, first-vertex angle). Reflection of an aligned or rotated rectangle is: centre
  mapped, `rotation` negated for a document-axes flip, radii permuted in the rectangle's own
  frame so each stays at its physical corner (the spec's test is the physical corner). A path
  needs no change except the `rotation` register (negated for a document-axes flip). The
  half turn keeps `rotation` and permutes radii diagonally (polygon and star: first-vertex
  angle plus pi).
- **2026-10-14: the flip is a `GroupMap::Scale` about the TBB centre, no new map kind.**
  New `curvyo-ui-core/src/flip.rs` (one sentence: the signed scale map and the no-op test of a
  flip). It builds `Scale { pivot: tbb.centre, sx: -1|1, sy: 1|-1 }`, runs the same `apply`
  as a drag, and compares the result to the start within the `0009` criterion 12 tolerances;
  equal means no write and the notice. A flip writes through the existing
  `transform_commit.rs` path with the label "Flip horizontally" / "Flip vertically".
- **2026-10-14: size chip.** `target_of` in `transform_entry.rs` and the matching code in
  `group_entry.rs` drop `NotPositive`; `InvalidReason::NotPositive` is deleted with its
  message. The typed value divided by the current extent is the signed factor about the
  chip's `pivot`. Millimetres only (customer: PERCENT A): no "%" parsing, `parse_entry_number` is
  unchanged and "%" stays "Enter a number". A typed 0 becomes +0.01 mm with the notice
  (ZERO-RECOVER C).
- **2026-10-14: frontend and wasm.** One new wasm export for the flip (`wasm_select_bar.rs`
  next to the kind groups); the Flip group is a component in the Select bar; the readout
  gets the U+2212 sign. No new crate. The key handler for H and V (Proposal) goes through the
  gate of `0010` in `wasm_keys.rs`.
- **2026-10-14: clones.** The flip passes the signed map G to the clone code of 0049. That
  code must not assume `det(M) > 0`; its determinant check (criterion 16, below 1e-9) is the
  only refusal. If 0049 merges first, its tests get one reflection case; if 0056 merges
  first, nothing is needed there.
- **2026-10-14: budget (criterion 27).** Same generator and `#[ignore]` benchmark pattern as
  0019 criteria 48 and 49; the signed map adds one multiply per coordinate and no allocation,
  so the commit cost is that of a move plus the no-op comparison (one extra pass over the
  result, which is why the 1.1 factor is a bound and not a guess).

## Review of the product owner's decisions (structural points only)

- **D1, D2, D3, D4, D6, D7, D8, D10: accepted.** D3 is the point that protects the document
  model (no stored sign, no matrix); D4 is cheap to change later because nothing reads the
  path direction yet.
- **D5 accepted with one caution.** Signs that follow the pointer while the magnitude comes
  from the dominant axis make the non-dominant axis flip the whole object at once when the
  pointer crosses its line. That is the stated intent; the ux-engineer should check it on
  a real drag.
- **Corrected in the specification:** criterion 20 said that for a rectangle turned by ±90° the
  radius permutation follows the document axes. With `rotation` negated (as the same
  criterion states) it is always the local swap; the invariant is the physical corner, and
  the text now says so. Criterion 25 omitted G·M·G⁻¹ for a clone selected with its original
  (0049 criterion 16); added. The "Depends on" line now answers the parallelism question.
  All claims about `resize_local_box`, `is_stretch`, `stroke_or_radius_factor`,
  `NotPositive` and `parse_entry_number` (accepts U+2212) were checked against the code and
  are right.
- **Risk, not a defect:** criterion 3's no-op test needs a tolerance compare of every
  object; for a 5,000-object selection it is one extra pass and sits inside the 1.1 factor of
  criterion 27.

## Customer decisions (2026-10-14)

- **PERCENT: A**, millimetres only. **ZERO-TYPED** superseded by **ZERO-RECOVER: C**
  (an axis on exactly 0 becomes 0.01 mm, sign of the previous non-zero frame, positive for a
  typed 0, with a notice). Reading checked: "exactly 0" as under 0.0005 mm, "last step" as
  the previous non-zero preview frame, and the dominant axis under Ctrl are structurally
  sound; three gaps were closed in criterion 15 (start extents under 0.0005 mm, the Ctrl
  partner, the flip). Neither decision needs an ADR; no open questions remain.
