# ADRs for "Tiling"

Date: 2026-11-01. How a tiling is stored is a document-model and file-format decision:
**[ADR 0028](../../docs/adr/0028-stored-tiling-nodes.md), Accepted by the customer decision
TILE-LIVE B (2026-11-01)**. The behaviour follows TILE-HOME-UI A, TILE-SOURCE A, TILE-SPACING A,
TILE-EXTRAS A and TILE-RESULT-GROUP A. Everything under "Feature-local decisions" is
**(Proposal)**, reversible inside the story. No new crate, trait, generic or dependency.

## Depends on

- [ADR 0028](../../docs/adr/0028-stored-tiling-nodes.md): a tiling is a group with the `tiling`
  recipe and `tiling_sources`; instances keyed by `tiling_cell`, members by `tiling_member`;
  apply in one commit with minimal writes; refusal before the first write; hand edits last until
  the next apply; Release; merge residue repaired by the next apply; format bump; goldens.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) and
  [ADR 0016](../../docs/adr/0016-clone-overrides.md): instances are clones with `own_placement`;
  colour and opacity variance through `own_style`; repair on source delete.
- [ADR 0022](../../docs/adr/0022-texts-on-carriers-and-text-clones.md): a text source gives
  linked text clones. [ADR 0017](../../docs/adr/0017-image-nodes-and-embedded-assets.md): an
  image source gives copies (same asset hash, no new bytes).
- [ADR 0026](../../docs/adr/0026-parametric-sketch-nodes-and-workbench.md): a sketch source gives
  linked clones of the original; no sketch graph is copied, no page claim is touched.
- [ADR 0027](../../docs/adr/0027-linked-dimension-objects.md) §1: dimensions are skipped.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one step per apply;
  [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §2: atomic registers.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §3: `Length`,
  `Angle`, `Point` newtypes in the recipe; explicit tolerances.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md): `document-core` ← `ui-core` ←
  `editor-wasm`; the document never calls the layout, the caller passes placements in.

## Feature-local decisions (2026-11-01)

1. **Where it lives.**
   - `curvyo-document-core`: `tiling_model.rs` (recipe, cell key, the per-cell placement and
     style the caller hands in), `tiling_codec.rs` (the registers, damage and residue rules),
     `tiling_commands.rs` (apply, release, copy remap of sources and members). The copy and
     ungroup writers call into it; no new `ObjectSnapshot` variant.
   - `curvyo-ui-core/src/tiling/`: `pcg32.rs` (generator), `layout.rs`
     (`tiling_layout(source_box, &TilingRecipe) -> Vec<(CellKey, Affine)>`, grid and circle,
     gap or pitch, mirror, half-cell offset), `variance.rs` (random and stepped variance per cell,
     varied style). Pure: no clock, no I/O, `f64` through the `curvyo-document-core` newtypes.
     `source_box` is the box Align and Distribute use (`0061`).
   - `curvyo-editor-wasm/src/session/tiling.rs`: the Tile mode of the Select bar, preview,
     dice, apply, reopen, release. The dice takes its new seed from the frontend
     (`crypto.getRandomValues`); core never reads entropy.
2. **Generator.** Hand-written PCG32 (O'Neill's `pcg32`, XSH RR output, 64-bit state, the
   reference multiplier and the fixed reference increment), about 40 lines with its own
   jump-ahead (`advance(delta)`, O(log delta)). No `rand` dependency. Tests: golden sequences of
   the first 16 outputs for seeds 0, 1, 42 and `u32::MAX`, and `advance(n)` equal to `n` steps.
   A draw becomes a unit value as `x as f64 / 2^32` (exact), a ± range as `v * (2u - 1)`.
3. **Fixed variance order, independent of the count.** The generator of a cell starts at `seed`
   and is advanced by `cell_index(row, column) * 8` draws, `cell_index = (row << 32) | column`
   (u64; the refusal limit keeps both below 2^17, so every cell gets its own stretch). It then draws, always and in this order:
   rotation, scale x, scale y, offset x, offset y, opacity, hue (the eighth slot is spare). A
   channel at zero still draws, so turning one channel on never changes another, and adding rows
   or columns never changes existing cells. Stepped variance is added before the random part and
   draws nothing. The instance affine is `translate(cell + offset) · rotate(angle) · scale`,
   about the source box centre. Golden layouts: fixed recipes and seeds with expected affines.
4. **Platforms.** Generator outputs are integers and identical everywhere. `sin`/`cos` may differ
   in the last bit between native and wasm, so layout goldens compare with 1e-9 mm and 1e-12 rad;
   ADR 0028 §3 writes nothing below 1e-9 mm, so a peer on another platform does not rewrite
   matrices. No `libm` dependency.
5. **Preview without document writes.** While the bar is open the session recomputes the layout
   on each change and draws the source outline mapped by each affine as an overlay (images as
   their frames); on a reopen it hides the managed instances of that tiling from the normal draw.
   Escape drops the preview; nothing is written until Apply or Enter. Test: history length and
   document version unchanged after a preview session.
6. **Budgets (thread CPU time).** In `curvyo-editor-wasm/tests/cpu_time/`: layout plus preview
   build for 10,000 instances; apply that creates 10,000; a seed change on 10,000. Numbers are
   measured at build and written into `plan.md`; the assert runs only in release builds
   (`if !cfg!(debug_assertions)`), as in `multi_object_transform_budgets.rs`.
7. **Refusal before the first write.** The command validates the whole request (sources, count,
   group depth, clone depth) before it opens the commit; a test checks the document version is
   unchanged after each refusal. The count limits (10,000 warn, 100,000 refuse) count created
   instances, not nodes; a multi-source tiling of 100,000 instances can make several times that
   many nodes (question for the PO: count nodes instead?).
8. **History names** in `history_names.rs`: "Tile" (create), "Edit tiling" (reopen and apply),
   "Release tiling".
9. **Recipe fields** (one atomic `tiling` value, documented in `tiling_codec.rs`): grid columns,
   rows, spacing x/y (`Length`), spacing mode `gap` | `pitch` (TILE-SPACING A), mirror rows,
   mirror columns, half-cell offset `none` | `rows` | `columns`; circle count, centre (`Point`),
   radius (`Length`), start angle and sweep (`Angle`), face centre; per channel (rotation, scale
   x, scale y, offset x, offset y, opacity, hue) a random ± and a step per row and per column,
   plus `uniform_scale`; `seed`, `result`, `keep_source`. Absent channel = 0.
10. **Format and goldens.** `CURRENT_FORMAT_VERSION` 32 (provisional, next free at merge; main is
   31). Goldens as ADR 0028 Consequences lists, named `tiling_*_v32.curvyo`.

## For the product owner

- Criterion 9 (reopen) is now the stored tiling of ADR 0028, for copies too; the out-of-scope
  line "Persisting the tiling as a live parametric object" goes.
- The spec must say: hand edits to an instance last until the next apply (deleted cells come
  back); Release keeps them for good. Opacity or hue variance makes instances stop following the
  source's style until the next apply.
- Sketches tile as linked clones, not copies: they are clonable since ADR 0026. Only images are
  copies. (Proposal; the plan's wording predates it.)
- Links line: ADRs → ADR 0028.
