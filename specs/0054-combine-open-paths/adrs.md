# ADRs for "Combine open paths: compound paths with open, touching and crossing outlines"

Architect review, 2026-10-12, re-checked the same day after the customer decisions (Booleans
close open outlines, closed outlines may touch and cross, node editing moved to `0055`). Most of
this slice exists below the commands: the stored form has an open or closed state per outline
since `0016` (format 8), and the renderer, the point-in-fill test, the hit test, the bounds and
Break apart honour it. What is new: Combine writes `closed: false` and touching or crossing
outlines, depth becomes "contains" instead of a first-node test, the kernel's flattening closes an
open outline with a straight line, and the Boolean kernel accepts open outlines. **No new ADR, no
new crate, no new dependency, no trait, no generic, and no `format_version` bump.**

## Depends on

- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  - §3: every geometric test here uses the 0.001 mm touch distance (`TOUCH_DISTANCE_MM`), the
    definition of "contains" included.
  - §5, §9: Combine, Split, Break apart and the Booleans replace tree nodes in one commit, as in
    `0035`, `0048` and `0016`.
  - §6: an outline is a run of cubic segments with an explicit open or closed state. Outlines
    that touch or cross are not excluded by the ADR; the "closed, non-touching" rule was
    `0016` criterion 41 and `0035`, which this spec amends.
  - §10: SVG round trip, see "SVG" below.
  - §11 with [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: a bump
    only when a reader of the current version would misread the file (decision 1).
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §4: display tessellation
  stays in `curvyo-render-core` with lyon, `FillRule::NonZero` (`fill.rs`); `build_fill_path`
  already closes an open outline with a straight chord and strokes it open. Nonzero is the one
  fill rule of the document (`0016` adrs.md: no `fill_rule` register); this slice keeps it, so
  ADR 0001 is untouched. Renderer: tests only (criteria 2, 3).
- [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md) §1 to §3: the
  touch, contains and point-in-region tests and the Boolean input stay in
  `curvyo-geometry-core`, integer grid and `i_overlay` nonzero unchanged.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §3: each outline's `closed`
  stays a last-writer-wins register; no new merge rule.
- [`0016` adrs.md](../0016-boolean-operations/adrs.md), [`0035` adrs.md](../0035-combine-and-break-apart/adrs.md),
  [`0048` adrs.md](../0048-split-compound-path/adrs.md): `extra_subpaths`, `outline_touch`,
  `outline_nesting`, `replace_with_pieces`, `plan_split`, `plan_break_apart`, the generated
  performance inputs and the `boolean-budgets` CI job.

## Feature-local decisions

- **2026-10-12: decision 1, no `format_version` bump (criterion 29; customer default A).** The
  per-outline `closed` exists since format 8 and windings are stored, not derived, so a file
  with open, touching or crossing outlines means the same to an earlier build. Such a build
  draws, hit-tests and breaks it apart correctly; only its Split groups differently (criterion 29,
  corrected). Nothing is lost or misread. Since `0050` bumps to 12 first, "earlier build" means a
  build between `0050` and this slice. *Rejected:* a bump; it locks out peers for no data safety.
- **2026-10-12: validation (criterion 30).** `validate_extra_subpaths` requires every element's
  `closed` to be a boolean. The first outline's `closed` keeps today's reading (absent = open)
  unless every golden file from format 1 to the current one holds it; then the same check
  applies. A single-anchor outline stays valid. Golden file `compound_open.curvyo` as criterion
  29 lists it.
- **2026-10-12: one flattening that respects `closed` (fixes a latent bug).** `flatten_closed`
  closes every outline with the cubic through the last anchor's out-handle and the first
  anchor's in-handle. For an open outline those handles are not part of the shape, so the Boolean
  region, the touch test and the depth test would disagree with the fill (`interior.rs` and
  `render-core` use the straight chord). It becomes `flatten(anchors, closed, tolerance)`:
  closed as today; open gives the polyline of its segments only, and `Flat` carries `closed` so
  `chord()` and `self_touches` do not wrap. The Boolean grid polygon of an open outline is then
  closed by the straight chord implicitly. Same function, one flag that already exists on
  `Outline`.
- **2026-10-12: "contains" for depth (criterion 7, definition).** `outline_nesting` keeps its
  shape and callers (Combine, Split). For a pair whose boxes nest and that **does not touch**
  (`touching_outlines`), the first-node test stays: it is exact there. For a **touching or
  crossing** pair, B is in A when every vertex of B's flattening farther than the touch
  distance from A is inside A, at least one such vertex exists, and every vertex of A farther
  than the touch distance from B is outside B (this second half catches a concave A whose edge
  dips into B between B's vertices). Coincident outlines have no qualifying vertex and do not
  contain each other. The inside test for this branch is the crossing count on the flattened
  polygon, `+ - * /` only, so criterion 33 holds on every target. Cost: only touching pairs from
  the existing box sweep, linear in their vertex counts. The implementer pins: touch from
  inside (7b), crossing (7c), coincident (7d), concave A with B across its notch.
- **2026-10-12: nonzero with crossing outlines is well defined, with one consequence to know.**
  The painted area is the nonzero winding of the stored outlines, whatever their depth; depth
  only chooses the windings Combine writes. Two holes that cross inside one shape paint their
  overlap (1 − 1 − 1 = −1), and no choice of windings avoids it under any winding rule (even-odd
  paints it too); only a Union of the two holes first gives one hole. Added to the spec's
  edge-case table; the PO adds a pinned test (7g) and the ux-engineer decides whether the touch
  sentence of criterion 12 should say "Union them first" (customer principle: a result plus a
  notice).
- **2026-10-12: Combine (`curvyo-ui-core/src/combine.rs`).** The `OpenPaths`, touch,
  self-crossing and no-area refusals go, with their enum variants and tooltip rows. Each
  operand keeps its own `closed` flag (`Outline::new(t, closed)`). Single-node outlines are
  dropped before planning and counted. Open outlines skip all geometry and are copied with fresh
  ids, direction kept. Closed outlines go through `touching_outlines` (now a count for the
  notice, criterion 12) and `outline_nesting` (depth by contains); windings follow depth; an
  outline with no area (`outline_has_area`, unsigned, so a cancelling figure eight is not
  fooled) keeps its direction.
- **2026-10-12: Split (criterion 15, `split.rs`).** Parts form from closed outlines as today,
  with the new depth. Open outlines join the same union-find: (a) touch pairs of an open outline
  with any outline, flattened without its chord; (b) else the part whose closed outlines paint
  its first node (`contains_point_in_outlines` over that part's closed outlines; spec wording
  corrected for parts with several outer outlines); (c) else its own set. Pairs already in one set
  skip the exact test, so a hatch stays near-linear.
- **2026-10-12: Booleans (criteria 21, 22, `geometry-core/src/boolean.rs`, `ui-core/src/boolean.rs`).**
  The kernel drops step 1's open refusal and the `BooleanError::OpenOperands` variant; with the
  flattening fix each open outline enters the nonzero normalisation closed by its chord, which is
  exactly the fill area. Nothing else in the kernel changes: a straight or zero-length open
  outline is a degenerate polygon that `normalize` already drops, and an operand whose region is
  empty is still refused (`EmptyOperands`, `0016` criterion 16). The counts for the notice are
  `ui-core`'s: open outlines with 2+ nodes, split by `outline_has_area`. `is_open` stays for the
  tooltip count and for Outset (`0038`). Fracture, Flatten and Offset are not built; they take
  criterion 24 into their specs.
- **2026-10-12: budget (criterion 32).** Same generator and job as `0035`/`0048`. Combine (b) runs
  no geometry; (d) runs the touch and contains tests on 999 neighbour pairs. Split (c) tests 2000
  lines against the rectangle by box sweep, then (b) once per line. Split of a crossing grid is
  not budgeted; the skip-same-set rule keeps the exact test linear, the box sweep remains.

## SVG round trip (criteria 26, 27, requirements for `0024`)

Well defined: SVG fills an open subpath closed by a straight line, never strokes that line, and
`fill-rule="nonzero"` with the stored direction gives our picture exactly, crossing outlines
included; depth is not stored, windings are, so export and re-import change nothing. Two notes
for `0024`'s architect: (1) ADR 0002 §10 puts parsed-but-unmodelled attributes in the
passthrough bag; `fill-rule` must **not** go there, or export would write `evenodd` while the
canvas paints nonzero. (2) Customer question below on even-odd input.

## Interaction with other slices

- **`0050`** (building, format 12): shared crates only (`document-core`, panel rows); order kept.
- **`0055`** (next): reuses the definitions here; this slice changes no node-editing code.
- **`0049`** (format 13 after `0050`): a clone copies `closed` and `extra_subpaths` verbatim;
  overlap only in the commands that replace paths (`replace.rs`).

## Flagged

- **Customer, for `0024` (default A):** an SVG `<path fill-rule="evenodd">`: A, re-wind its
  closed outlines by depth on import, so the picture stays the same, and list it in the loss
  report only when closed outlines cross; B, as criterion 27 says now: keep the windings, draw
  nonzero and always list it (an Inkscape Combine of two same-direction circles then imports as a
  filled disc instead of a ring). Recommendation A; decided in `0024`, not here.
- No new ADR: ADR 0002 §6 already allows open and closed outlines per path; the restriction to
  closed, non-touching outlines lived in specs, which this one amends.
