# ADRs for "Fused lettering"

Architect, 2026-11-26, for `specification.md` (Draft; slice 1 Cut-out, slice 2 Stencil).

The bridge geometry is a new module family `fuse/` in `curvyo-geometry-core`, next to Union and
Weld. It runs on the 0016 grid, uses the pinned `i_overlay` and the existing integer offsetter.
The cut line is the 0016 **Union** of pieces and connectors, not Weld (spec issue 1). The plan is
in `curvyo-ui-core`, the write is one new `curvyo-document-core` command, and the button's
presence is one session predicate. **No new ADR, no new crate, no new dependency, no `Cargo.toml`
change, no `format_version` bump.** No trait, no generic.

## Depends on

- [ADR 0003 §1, §3, §7, §8](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md):
  kernel in `curvyo-geometry-core`, explicit `Tolerance`, flattened polygons, `i_overlay` `=9.0.1`
  on the 0.001 mm grid, polyline results, no backend trait. Connectors are kernel polylines.
- [ADR 0002 §5, §9](../../docs/adr/0002-document-model-units-and-svg-round-trip.md): the
  stacking order is the document's; one interaction, one commit.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one step, label `fuse`, which
  `history_names.rs` maps to "Fuse".
- [ADR 0024 §1](../../docs/adr/0024-spatial-pages.md) and
  [ADR 0042 §7](../../docs/adr/0042-page-kinds.md): page membership by bounds. The tools follow
  the active page, with no `if laser` in the shell.
- [ADR 0046 §1, §4](../../docs/adr/0046-laser-operations-record.md): `member.` keys on the
  operation node. An assignment whose object is deleted is dropped silently (Frage 169 A).
- [`specs/0016-boolean-operations/adrs.md`](../0016-boolean-operations/adrs.md): `operand_regions`,
  `normalize`, `run`, `cleanup`, `canonical_millimetres`, `BooleanError` and its offenders,
  `MAX_COORDINATE_MM`, `BOOLEAN_TOLERANCE` 0.01 mm.
- [`specs/0068-laserable-lettering/adrs.md`](../0068-laserable-lettering/adrs.md) decision 6: the
  operand collection of `weld_plan.rs` (leaves at any depth, clones resolved, `texts_refusal`,
  `texts_as_paths`, stacking order, operands without area dropped).
- [`specs/0037-fracture-and-flatten/adrs.md`](../0037-fracture-and-flatten/adrs.md): `GridBox`,
  `normalize_shapes`, and the `performance-budgets` job.

## Feature-local decisions

1. **2026-11-26: crate and modules.** Everything geometric is in `curvyo-geometry-core/src/fuse/`.
   Each file does one thing and stays under 500 lines:

   | Module | Does |
   |---|---|
   | `mod.rs` | the entry points, `Fused`, `FuseError` |
   | `pieces.rs` | the pieces of the union and which of them are dots |
   | `gap.rs` | the narrowest gap between two contours, with the tie rule (decision 4) |
   | `forest.rs` | attaches dots and builds the spanning forest with the obstruction test |
   | `axis.rs` | the connector's axis (straight or one cubic) and its band |
   | `blend.rs` | the local closing that adds the fillets |
   | `islands.rs` | (slice 2) islands, crossings and island chains |

   It needs the crate-private grid helpers (`Paths64`, `run`, `normalize_shapes`, `EdgeIndex`,
   `polyline_offset`'s offsetter, `cleanup`), so it cannot live in `ui-core` (same reason as 0068
   decision 10).

   Rejected options:
   - A new `curvyo-lettering-core` crate. It would need those helpers made public, and §5 asks for
     an ADR for a crate with one caller.
   - Putting it in `weld.rs`. Weld's job is to keep each visible line once, which is a different
     responsibility.

2. **2026-11-26: public API (slice 1).**

   ```rust
   pub struct Fused { pub outline: Vec<Vec<Point>>, pub connectors: Vec<Vec<Vec<Point>>> }
   pub enum FuseError { Operands(BooleanError), NothingToJoin, TooManyBridges }
   pub fn fuse_cut_out(operands: &[&[Outline<'_>]], width: Length, reach: Length,
                       max_bridges: usize, tolerance: Tolerance) -> Result<Fused, FuseError>
   ```

   - `outline` holds the closed outlines in Union's canonical form (0016 criterion 41 winding).
   - `connectors` holds one entry per connector. An entry is usually one outline and may be
     several when an overlay splits it.
   - `FuseError` uses `thiserror`, with `#[from] BooleanError` as `StackError` does.
   - `TooManyBridges` is decided once the forest is known, before any band is built. The limit
     comes from the caller (`FUSE_MAX_BRIDGES = 2000` in `ui-core`), as Fracture's limit does.
   - Slice 2 adds `fuse_stencil(operands, width, bridges_per_island: u8, tolerance)` and
     `FuseError::NoIslands`. It is a second function, not a mode enum, so slice 1 has no
     speculative variant.
   - Blend is not a parameter. Its radius is `width` (spec Words, Proposal), so there is no
     speculative option.

3. **2026-11-26: the Cut-out algorithm.** The steps:
   1. Read the regions with `operand_regions` (0016 checks and refusals). Then `U = normalize` of
      all of them, and pieces are `normalize_shapes(U)`: an outer contour with its holes.
   2. Sort the pieces by the smallest point `(x, y)` of their outer contour. That gives each
      piece its index.
   3. Classify the dots, comparing exact `i128` double areas (spec issue 2).
   4. Find the candidate pairs: piece boxes grown by Reach must meet, then `gap.rs` finds the
      narrowest gap between the **outer** contours. Only outer contours take part, because a
      piece inside a hole is out of scope (spec issue 6).
   5. Rule 7a: attach each dot to its nearest non-dot. Then rule 7b: Kruskal with union-find over
      the remaining pairs whose gap is at most Reach. Edges are ordered by exact `f64` distance,
      then the gap points `(x, y)`, then the piece indices. The obstruction test (criterion 15)
      runs lazily, only on an edge Kruskal would accept. That edge's band minus `A ∪ B` must have
      no area inside `U`. If it does, the edge is skipped and the pair is joined through the third
      piece.
   6. Count the accepted edges. If there are none, the result is `NothingToJoin`. If there are
      more than `max_bridges`, it is `TooManyBridges`.
   7. Build each connector's axis (`axis.rs`):
      - The normal at a gap point is the edge normal, or at a vertex the bisector of the two edge
        normals.
      - The axis is straight when both normals lie within 5° of the joining line. The test is a
        dot product against the literal `cos 5°`, with no trig call.
      - Otherwise the axis is one cubic, `p, p + nA·d/3, q + nB·d/3, q`, where `d = |q − p|`. If
        it has two inflections, both handles are halved, up to 4 times, and then the axis is
        straight.
      - The band is a ribbon of `width` around the axis, flattened at the tolerance. It is
        extended by `width` into each piece along `−n` and normalized. The extension is clipped
        to its own piece (`∩ A`, `∩ B`), so it never pokes out of a thin stroke.
   8. Add the fillets (`blend.rs`), on a local box: the band's box grown by `4·width`.
      - Clip `A ∪ B ∪ band` to the box, then close it: offset by `+r`, then by `−r`, with
        `r = width`. This uses the integer offsetter with round joins at **at most 10° per arc
        step** (`ArcOptions::max_step`), not the tolerance-derived step, which is 16° at r = 1 mm
        (criterion 16c, spec issue 5).
      - The connector is `(closed ∩ N) − U`, where `N` is the band grown by `2r`. Only the
        components that touch the band are kept, so a nearby concavity of a letter is not filled.
      - Third pieces stay out of the closing, so a narrow gap to a neighbour is never closed.
   9. `outline = canonical_millimetres(cleanup(normalize(U ∪ connectors)))`. Each connector is
      `canonical_millimetres(cleanup(connector))`. Connectors are ordered by their first point.

   Rejected options:
   - Analytic fillet arcs, tangent to an arbitrary flattened boundary. They need a tangent-circle
     solve per junction against polylines, and that is more robustness surface than the closing
     on the grid.
   - Weld for the final join. Its inner lines would cut the part apart (0068 correction 4).

4. **2026-11-26: determinism and tolerances.**
   - Lengths are `Length` and the tolerance is `Tolerance`, both passed in by the caller.
   - Grid arithmetic is integer. Distances are `f64` with `+ − * /` and `sqrt` only, so they are
     identical on every target, as 0016 step 2 is.
   - Collections are `BTreeMap`/`Vec`. No `HashMap` iteration is used.
   - **Tie tolerance = the kernel tolerance passed in (0.01 mm).** Gap points whose distance lies
     within it of the minimum form the tied set, because flattening alone moves distances by up
     to that much (spec issue 3). The tied stretch is taken on the lower-indexed piece's outer
     contour. Its middle is measured by arc length, and among several stretches the middle with
     the smallest `(x, y)` wins. The partner point is the nearest point on the other piece.
   - The tie function is `pub(crate)` with its own unit tests: criterion 9's bars and the
     concentric ring of criterion 24.
   - Goldens are compared natively and in wasm at 1e-6 mm (`boolean_golden_wasm` pattern), in
     `curvyo-geometry-core/tests/fixtures/fuse/`: the i, the ü, the bars, and the criterion 16
     cases.
   - Criterion 18's byte-identical run-twice test is in `curvyo-editor-wasm` (names, node order,
     anchor ids from the session minter).

5. **2026-11-26: plan and write.**
   - **Plan:** `curvyo-ui-core/src/fuse_plan.rs`.
     - It reuses Weld's operand collection. A first refactor commit moves it out of
       `weld_plan.rs` (354 lines) into `kernel_operands.rs`, with no behaviour change.
     - It holds the clamps and constants: Width 0.2 to 10 mm (default 1.0), Reach 0.5 to 50 mm
       (default 5.0), and `FUSE_MAX_BRIDGES`.
     - It produces the refusal codes. The sentences belong to the host.
   - **Write:** a new `Document::replace_with_group(removed, place, &NewGroup, label)` in
     `curvyo-document-core/src/replace_group.rs`, in **one commit**. It:
     - deletes the operands;
     - creates the named group at the place of the **top-most** operand (criterion 11);
     - creates the child path "Outline", and above it the child group "Connectors" holding one
       path per connector (named "Connector 1".., in kernel order, Proposal; the UX notes may
       rename them);
     - writes the `member.` keys of the given operation ids for the Outline. The key encoding is
       already document-core's (`operation_codec`).

     Every id and every anchor id is resolved before the first write, as in
     `replace_with_pieces`. Style: Outline and connectors take the bottom-most operand's complete
     style, as Union does (Proposal; the UX notes decide the look).
   - **Operations (Frage 396 A):** the operations are the bottom-most operand's **effective**
     assignment by the nearest rule (`curvyo-laser-core` `operation_nearest`, read in the
     session). Connectors get none (spec issue 9).
   - **Session:** `session/fuse.rs`, with `wasm_fuse.rs` binding it. `apply_fuse(width, reach)`
     runs under `recording`. The texts are converted in the same step (`texts_as_paths`). The
     result is selected.

6. **2026-11-26: format.**
   - The result is an ordinary group, an ordinary path or compound path (format 8), ordinary
     paths, the existing `name` register and existing `member.` keys. That means no new key, no
     new object kind and **no `format_version` change**.
   - A save-and-reopen golden covers criterion 20.
   - The panel values are app preferences, not document data. The frontend keys are
     `curvyo.fuse-width-mm` and `curvyo.fuse-reach-mm`, and slice 2 adds `curvyo.fuse-mode` and
     `curvyo.fuse-bridges`. The values are clamped again by `ui-core` when they are read back.

7. **2026-11-26: laser-page-only gate (criteria 1, 2, 4).**
   - `Session::fuse_available()` is `operations_available()`: an active laser page whose
     extension is on (0101 criterion 28). That is the same predicate `test_grid_tool_available`
     builds on.
   - The wasm side exposes `fuse_available()`. The `vectorRail` in `workbenches/builtin.tsx`
     passes it to `ToolRail` as a prop, and `BooleanCommands` renders Fuse only when it is true.
     The shell never names a process.
   - `apply_fuse` returns `Ignored` when the gate is false, as a second line of defence.
   - Criterion 4 is a session check: every selected leaf must be a member of the active page
     (ADR 0024 §1), and none may be a placement (spec issue 8).

8. **2026-11-26: preview and budget (criteria 3, 19).**
   - The preview runs **the same `fuse_cut_out`**. A cheaper connector-only pass would be a
     second implementation that could disagree with the result.
   - It runs on each committed field value (Enter, Up/Down step, leaving the field), not on each
     character. The session caches it by (selection revision, width, reach). Pressing Fuse reuses
     the cached result when nothing has changed. It is never written to the document.
   - Budget: `curvyo-editor-wasm/tests/fuse_budget.rs` measures `apply_fuse` end to end with the
     `cpu_time` helper (thread CPU time, scaled by `CURVYO_BUDGET_FACTOR`). The input is 60
     pieces and 3,000 nodes, and the limit is 300 ms in release. It joins the
     `performance-budgets` job.
   - If the budget misses, the allowed fix is to cache the pieces and gaps across Width changes,
     since gaps do not depend on Width. If it still misses, the implementer logs it in
     `docs/technical-debt.md`, as 0068 decision 8 did.

## Spec issues (for the product owner; each a Proposal)

1. **Weld vs. Union.** Criteria 6, 11 and 12 and "reusing Weld" read as if the Outline is Weld's
   result. But Weld keeps inner lines where letters overlap, and cutting those separates the
   parts (0068 correction 4).
   - Proposal: operands are read as Weld reads them, and the Outline is the **Union** (0016) of
     pieces and connectors.
   - Criterion 12 then holds by construction and its property test stays.
2. **The dot rule fails on a lone letter.** With "a quarter of the median piece area", a lone ü
   (3 pieces) has the dot's area as its median, so no piece is a dot and criterion 10 cannot hold.
   The rule also leaves the median of an even count undefined.
   - Proposal: a dot piece is a piece whose area is below a quarter of the **largest piece within
     Reach** of it.
3. **The tie tolerance is not stated.** Flattening moves distances by up to 0.01 mm, so ties
   within one grid unit would let flattening noise pick the bridge on curves (criterion 24's
   ring).
   - Proposal: points within the kernel tolerance (0.01 mm) tie (decision 4).
   - Several tied stretches: the middle with the smallest `(x, y)`.
   - Spanning-forest ties: exactly equal distances only.
4. **Criteria 23 and 24 contradict each other.** On the concentric O every crossing ties.
   "Further bridges at least a quarter perimeter away, then top-most" puts the second bridge at
   the left or right, not at (0, 8).
   - Proposal: among the crossings at least a quarter perimeter away, take the narrowest. Among
     ties, take the one **farthest along the boundary** from the earlier bridges, then the
     top-most, then the left-most.
5. **Criterion 16c's 15° cannot hold on the operand's own curves.** A 1 mm dot flattened at
   0.01 mm turns about 16° per node.
   - Proposal: measure 16c on the nodes the connector adds plus its two seam nodes.
   - The fillet arcs are built at no more than 10° per step (decision 3.8).
6. **Pieces inside a hole** (a dot inside a counter, Cut-out). The out-of-scope line says such
   pieces are not bridged, while criterion 7 says every dot is connected.
   - Proposal: they are left apart silently and stay in the result.
7. **A dot with no non-dot within Reach** (a colon standing alone).
   - Proposal: it takes part in rule 7b like any piece.
8. **Placements on the laser page** are not drawing objects.
   - Proposal: a selection that holds one is refused with criterion 4's sentence.
9. **Criterion 14, "operations of the bottom-most operand".**
   - Proposal: its effective assignment by the 0101 nearest rule, so an assignment inherited from
     its group counts.
10. **Criterion 19, "browser or desktop".**
    - Proposal: measured natively in release, as every other budget is. The preview is
      recomputed on committed values, not per character (decision 8).
11. **Criterion 27, "stroke thinner than the Width".** A bridge's width runs along the stroke, so
    a thin stroke does not limit it.
    - Proposal: drop the clamp in criterion 27. A bridge is never wider than a quarter of its
      island's perimeter.

## Implementation notes (implementer)

(Dated notes go here.)
