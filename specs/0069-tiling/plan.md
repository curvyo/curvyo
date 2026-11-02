# Plan for Tiling (0069)

Implementer, 2026-11-02. Binding: ADR 0028, `adrs.md`, `ux-notes.md`, `specification.md`
(43 criteria). One PR for the whole feature (customer released the build 2026-11-02).

## Affected crates/modules

- `curvyo-document-core`
  - `tiling_model.rs` (new): `TilingRecipe` and its parts (`GridOptions`, `CircleOptions`,
    `Variance`, `Channel`), `CellKey`, `TilingCell` (the per-cell affine and style shift the
    caller hands in), `TilingError`, limits, history labels, `hue_shifted`.
  - `tiling_codec.rs` (new): the four registers `tiling`, `tiling_sources`, `tiling_cell`,
    `tiling_member`; read, write, open-file validation (damage rules of ADR 0028 §10).
  - `tiling_commands.rs` (new): `Document::apply_tiling` (create and regenerate, minimal
    writes, refusal before the first write), `Document::release_tiling`, `Document::tiling`;
    the strip used by Ungroup and by copy/paste outside a tiling.
  - `tiling_write.rs` (new): creating and rewriting one instance (clone, text clone, path
    copy, text copy, image copy) and one member.
  - `group_tree.rs` (validation hook), `groups.rs` (Ungroup strips), `objects.rs`
    (duplicate remaps sources and members, strips keys of instances copied out),
    `document.rs` (`CURRENT_FORMAT_VERSION` 32), `lib.rs`.
- `curvyo-ui-core`
  - `tiling/pcg32.rs`, `tiling/layout.rs`, `tiling/variance.rs`, `tiling/mod.rs` (new):
    the generator, the grid and circle placements and the variance per cell; pure.
  - `tiling/plan.rs` (new): Tile offered or not, the source box, the defaults, the request
    for the document (conversions, cells), the preview outlines.
  - `history_names.rs`: "Tile", "Edit tiling", "Release tiling".
- `curvyo-editor-wasm`
  - `session/tiling.rs` (new): the Tile mode (enter, set recipe, preview, apply, cancel,
    reopen, release, circle handles), the preview overlay, the locks.
  - `wasm_tiling.rs` (new): strings and scalars to the frontend (recipe as JSON).
  - `tests/tiling_budget.rs`: thread-CPU budgets (criterion 40).
- `frontend`: `hooks/useTiling.ts`, `components/TilingBar.tsx`, `lib/tilingText.ts`, the Tile,
  Edit tiling and Release tiling buttons in `SelectToolbar.tsx`, mounting in `App.tsx`.
- Fixtures: `tests/fixtures/tiling/` (PCG32 sequences, layout goldens),
  `curvyo-document-core/tests/fixtures/tiling_*_v32.curvyo`.
- Docs: `docs/guides/pcg32.md` (algorithm and seeding), `docs/requirements.md` (R-EDIT-044),
  `specs/README.md` (row, format table), `docs/technical-debt.md` if a budget is missed.

## Tasks

- [x] 1. PCG32 with `advance`, golden sequences computed with the reference C code
  (`tools/pcg32-reference/pcg32_reference.c`), fixture `tests/fixtures/tiling/pcg32.json` (AC 20, 41).
- [x] 2. Recipe model and `tiling_layout`: grid (gap/pitch, mirror, offset, shift), circle
  (count, radius, start, sweep, face centre, relative centre), variance random and stepped in
  the fixed draw order with per-cell jump-ahead; layout goldens (AC 6-13, 15-17, 21, 41).
- [x] 3. Codec and validation of the four registers, format 32 (AC 25).
- [x] 4. `apply_tiling`: refusals before the first write; create (keep source on/off, clones,
  text clones, image copies, copies, multi-source instance groups, own style for colour and
  opacity variance) in one commit "Tile"; regenerate with minimal writes, recreate deleted
  cells, delete left cells, leave hand-converted and unkeyed children, duplicate repair,
  unchanged apply writes nothing, "Edit tiling" (AC 4, 5, 18, 22-24, 27, 28, 32).
- [x] 5. Release and Ungroup strip; copy/duplicate remap; an instance copied out loses its
  key; delete; dangling source (AC 30-32).
- [x] 6. File goldens `tiling_*_v32.curvyo`, damage files, residue file; SVG export equals
  the hand-made group (AC 33, 42).
- [x] 7. ui-core plan: Tile offered, defaults, request building, preview outlines; history
  names (AC 1, 3, 5, 11, 19, 36, 37).
- [x] 8. Session Tile mode: enter, recipe updates, preview overlay (hidden managed instances,
  simplified above 10,000), apply/cancel, reopen by button or Enter, release, locks (undo,
  selection), circle handles with snapping (AC 1, 2, 14, 26, 34, 35, 38, 39).
- [x] 9. Frontend: Tile / Edit tiling / Release tiling in the Select bar; the tiling bar
  (layout, fields, variance block, seed and dice with `crypto.getRandomValues`, result, keep
  source, count status, notices); Enter applies, Escape cancels (AC 1-3, 15, 19, 26, 36).
- [x] 10. CPU-time budgets in release builds; numbers below (AC 40).
- [x] 11. Docs: PCG32 guide, requirements, specs README, spec status (AC 20).
- [x] 12. Review follow-up (`review.md`, `acceptance-report.md`): Ungroup and Release strip only
  the tiling they dissolve; a re-applied text copy keeps visible, locked and guide; a cell key
  outside the layout is refused; a layer as source gets its own refusal; `update` split per
  member kind, `plan_tiling` and `regenerate` split, dead code and the `RefCell` removed; public
  API narrowed; the radius handle snaps; a reopen hides only keyed instances; the inactive
  layout is cut to the count limit; angles and centre sent back unchanged keep their stored
  value (AC 4, 14, 27, 28, 32, 34).
- [x] 13. UX items: ghost fill with the instance's hue and opacity, dashed circle path, accent
  handles, readout chip of a handle drag, 120 ms debounce, refused-field marking, polite live
  region, cancel on a page switch; Align and Distribute move a tiling as one object
  (AC 14, 34, 35, 39).
- [x] 14. Customer amendment 2026-11-02 (`ux-notes.md` §12, criteria 1, 2, 11, 14, 19, 34, 36,
  40): `curvyo-ui-core::tiling::handles` (four handle positions, radius at the arc middle, sweep
  knob 14 px before the start at a full turn, pick with the tie order, start and sweep drags with
  Ctrl stops, unwrapping, clamp and the 360 snap, start normalised to -180..180);
  `curvyo-render-core::tiling_overlay` (dashed arc, dotted rest, centre lines, Parameter-handle
  knobs); session `tiling_handles.rs` (drags, readouts, cursors, entry box open/keep/restore,
  double-click, keys C/R/A/S); frontend: no Cancel, seed, dice or count text, Apply at the right,
  Grid/Circle icon segments, circle fields reduced to Count and Face centre, notice slot, compact
  Variance with a container query, `TilingEntryChip.tsx` (AC 1, 2, 11, 14, 19, 34, 36).
  Criterion 40 needed no change: the Apply measurements were already reported, not asserted.
- [x] 15. Customer 2026-11-02, live source edits (AC 29): `clone_previews` in
  `curvyo-document-core/src/clone_preview.rs` applies the commit's held-back rule
  (`clone_matrices_after_maps` on `outline_map::object_map`, shared with `hold_for_results`), so
  a Select drag previews each tiling instance with the matrix the release writes; tiling
  instances are not counted against the 500 cap; the session caches `own_placement_clones` per
  document read (`clone_view.rs`), draws instances as themselves in Node drags without a ghost
  (`drag_swap.rs`), and a style preview reaches clones whose style source is previewed
  (`curvyo-ui-core/src/style_edit.rs`). Tests: `session/tiling_live_tests.rs`; frame costs in
  `tests/tiling_budget.rs` (reported, below).

## Validation

- Golden PCG32 sequences checked against the reference C implementation's output (task 1).
- Layout goldens with 1e-9 mm / 1e-12 rad tolerance, run on host and as a wasm32 build.
- File goldens and damage files open or are refused as ADR 0028 §10 lists.
- Document-version checks after every refusal, Cancel and unchanged Apply.
- Budgets measured with thread CPU time, asserted in release builds only.

## Measured budgets

`cargo test --release -p curvyo-editor-wasm --test tiling_budget -- --nocapture`, thread CPU
time, one path of 50 segments tiled 100 x 100 with every variance channel on (2026-11-02):

| Cost | Measured | Target (Proposal) |
|---|---|---|
| Layout and preview build after one option change | 5.0 ms | 50 ms |
| Apply creating 10,000 instances | 196 ms | 500 ms |
| Seed change on 10,000 existing instances | 266 ms | 500 ms |

The preview draws only the instances meeting the viewport; the layout of all 10,000 is in the
number. No target missed, nothing added to `docs/technical-debt.md`.

A frame in the middle of a drag of the source of a stored tiling (keep source on, no variance;
pointer move plus draw list; target 16 ms, Proposal, reported only, 2026-11-02):

| Frame | 10 x 10 | 100 x 100 |
|---|---|---|
| Scale with the Select handle | 1.0 ms | 87 ms |
| Node drag with the Node tool | 0.6 ms | 80 ms |

Before, the 100 x 100 frame cost about 40 ms with the instances held still (no live preview);
the rest is the resolve and the artwork of 10,000 changed instances. Tracked in
`docs/technical-debt.md`.

## Deviations and open points (for the lead)

- **(Proposal)** Copies of primitives, clones and sketches are written as paths (their outline
  mapped by the cell); an image copy keeps its asset; a text copy and a text clone are placed
  straight (an on-path source text loses its carrier in the instances).
- **(Proposal)** Face centre turns instance k by its angle minus the start angle, so instance 0
  equals the unturned source (keep source on and off agree).
- **(Proposal)** Rotation variance is counterclockwise on screen, as the circle angles; a scale
  factor is clamped to at least 0.01.
- **(Proposal)** Turning the opacity and hue variance off clears the own styles a previous Apply
  generated; a hand-set own style without style variance is kept (criterion 28).
- Criterion 14 says "Ctrl suspends"; the circle handles use the suspend rule of every other
  Select-tool handle (Ctrl+Alt, `snapping.rs`), which the 0090 rework set. The spec line needs
  the lead's word.
- `recipe_value` in `tiling_codec.rs` is 78 lines: one map literal of the stored recipe, kept
  whole so the format reads in one place (justified, not split).
- Align and Distribute (`curvyo-ui-core/src/align_commit.rs`) now settle the matrices of linked
  clones by the one clone rule per delta: before, a clone with own placement whose original
  moved in the same commit moved twice, so a tiling was scattered (AC 39). A clone and its
  original in units with different deltas still get the old approximation.
- The PCG32 fixture has no upstream pcg-c expected-output vector yet: pcg-c's published demo
  output is for the two-stream `pcg32` (seed 42, sequence 54), not the one-stream generator
  used here.
- Criterion 31: copy, cut and paste wait for an object clipboard (criterion reworded by the
  lead); Duplicate remaps sources and members, and an instance copied out of its tiling loses
  its keys. Criterion 43's
  writer-listing test is covered by tests of Ungroup, Release and Duplicate, not by a scan.
- Version pins of earlier testers' tests were moved from 31 to 32 or `>=` (`acceptance_0070`,
  `acceptance_0024_slice2`, `drawing_insert`), as at every format bump; the 0024 export
  document snapshots record format 32.
