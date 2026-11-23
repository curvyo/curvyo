# ADRs for "Shape loops: a Rounded handle on polygon, star, rectangle and ellipse"

One new optional register on existing primitives and one pure evaluation beside `0021`'s
Curve. **No new crate, no new dependency, no new trait, no numbered ADR**: like
`specs/0013-rectangle-corner-radii/adrs.md` and `specs/0021-ellipse-arcs-and-shaping/adrs.md`
it extends ADR 0002 §5 by a register of the same kind, so it is not `needs-customer`
(`CLAUDE.md` §3). Reference state: branch `chore/0145-ready`, `CURRENT_FORMAT_VERSION` 53,
read on 2026-11-23.

## Depends on

- [ADR 0002 §3, §5, §9](../../docs/adr/0002-document-model-units-and-svg-round-trip.md): units
  are types; a primitive's parameters are keys in its meta map; one interaction, one commit.
- [ADR 0004 §9](../../docs/adr/0004-persistence-and-cross-machine-sync.md): a newer file is refused.
- [ADR 0009 §3](../../docs/adr/0009-concurrent-editing-semantics.md): per-register LWW.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one commit, one step ("Change rounding").
- `0003`, `0009`, `0013`, `0021` `adrs.md`: stored raw and limited where evaluated; Object to path
  strips every primitive key; `apply_param` for drag, entry and bar; absent key means default.

## Feature-local decisions

- **1. 2026-11-23: the Rounded formula (criterion 4).** Inkscape's `sp_star_get_curvepoint`
  (`src/object/sp-star.cpp`), *recalled, not read* (no Inkscape source or binary here):
  for a ring vertex `o` with ring neighbours `p` (previous) and `n` (next) (a star's neighbours are
  the other kind of vertex, a polygon's the same kind), `mid = (p + n)/2`,
  `biss = mid + 1e5 · R(unit(n − mid))`, `rot = R(unit(biss − o))` with `R(x, y) = (y, −x)`;
  the handle toward `p` is `o + q·|p − o|·rot`, the one toward `n` is `o − q·|n − o|·rot`
  (the sign flip). Each side `i → i+1` is the cubic `(V_i, H⁺_i, H⁻_{i+1}, V_{i+1})`; `|q| < 1e-4`
  draws straight lines (Inkscape's `not_rounded`, recalled). **Closed form we build** (derived here,
  exact for every ring we evaluate, where `|p − o| = |n − o|` and `o` lies on the perpendicular
  bisector of `p n`, so the `1e5` point only fixes a side): with `s` the side length and `t_i` the
  unit tangent perpendicular to the radius at `V_i`, pointing toward increasing index,
  `H⁺_i = V_i + q·s·t_i`, `H⁻_i = V_i − q·s·t_i`. Polygon `s = 2R·sin(π/N)`; star
  `s = |V_outer − V_inner|`; both kinds of star vertex get the same `q·s`. The form does not depend
  on the ring's direction, so a mirror keeps it (`0056` D3). Nodes are `AnchorKind::Symmetric`.
  *Certain*: one cubic per side, handles on the tangent of `prev → next`, length proportional to
  the neighbour distance, the sign flip, `q = 0` straight. *Recalled*: the factor is exactly
  `q` (no 1/3), the `1e-4` threshold, the `1e5` bisector point. A reading as "perpendicular to the
  angle bisector of `p o n`" gives the same handles on all our rings. Inkscape wins on any
  difference (spec Terms); the golden fixtures of decision 7 settle it.
- **2. 2026-11-23: where it lives.** New module `curvyo-document-core/src/ring_rounded.rs` ("The
  outline of a ring of vertices rounded by Inkscape's tangent rule."), reusing `Ring` of
  `ring_curve.rs` (already 1034 lines, so nothing more goes there). It holds the newtype
  `Rounded(f64)` (`RANGE = 10.0`; `new` refuses a non-finite value; `effective()` limits to
  ±10, criterion 11's "limited" tag) and `rounded_outline(ring, q) -> Vec<OutlineAnchor>`.
  `Shape::Polygon` and `Shape::Star` gain `rounded: Rounded` (slice 2: `Rect`, `Ellipse`).
  `outline_with_pieces` gets one arm ahead of the Curve arms; `outline_of` and
  `drawing_outline_of` agree (no arcs). Every place that keeps a polygon or star a primitive by
  assuming straight sides treats `rounded` as it treats `curve`: `is_straight`,
  `outline_with_pieces`, `shape_frame_bounds`, `offset_primitive.rs`, `transform_primitive.rs`
  (`polygon_star_stretch`), `svg_export_shapes.rs` (`ring_kind`), `svg_import_shadow.rs`,
  `svg_import_shapes.rs` (`star_geometry`); the implementer greps every `curve` use on these
  variants.
- **3. 2026-11-23: Rounded and Curve (criterion 5, Frage 309 A).** One writer writes both
  registers in one commit, so one is always 0. An LWW merge can still yield both non-zero; that
  state is **valid** on open and in a merge (never `Damaged`, a merged file must reopen), and
  evaluation lets **Rounded win** and ignores Curve until Rounded is 0. Rectangle radii (slice 2)
  are kept and ignored the same way.
- **4. 2026-11-23: format.** Register key `rounded` (`f64`) next to `KEY_CURVE` in
  `shape_arc_codec.rs`; absent is 0; returning to 0 **deletes** the key (`0021` decision 1), so an
  untouched shape stays byte for byte. Non-finite on open is `Damaged`; out of ±10 is limited on
  use, nothing written back. `format_version`: one bump **per slice**, the next free number at
  merge (an older build would cut a rounded shape straight); slice 2 bumps again because a
  slice-1 build ignores `rounded` on a rectangle or ellipse; slice 3 shares a bump only if merged
  together. A generated golden project fixture for the new version (`0013` habit).
- **5. 2026-11-23: Convert to paths, bounds, flattening, determinism.** Object to path takes
  `outline_of_rotated` as today and adds `rounded` to the stripped keys; loops are kept.
  **Bounds** (criterion 7): for `q ≠ 0` the frame box is the **union** of today's circumscribed
  box and the exact box of the cubics (`kurbo::CubicBez::bounding_box`, extrema) of the ring
  **as drawn**: `Ring::of(shape, true)`, `frame.angle` included, the object's `rotation` excluded
  (`ring_bounds`' "vertex 0 at angle 0" would be wrong here); `q = 0` keeps today's box (criterion 4). The union keeps the drag
  continuous: the box only grows once loops leave the circle. Hit test and marquee read the same.
  **Flattening**: the renderer's existing tolerance on screen, `0029`'s job tolerance for jobs;
  no new constant. **wasm**: only `+ − × ÷ sqrt` (IEEE exact) beyond the `sin`/`cos` the vertices
  already use; no FMA, no iteration; criterion 13 holds within 0.001 mm. **Budget**: evaluation is
  O(vertices); criterion 12 includes flattening 2048 cubics with handles ten sides long at q 10,
  so the `cpu_time` test is written and measured first, before the drag is wired.
- **6. 2026-11-23: SVG (criterion 18).** Import: `star_data()` in `curvyo-svg-core/src/shapes.rs`
  accepts a finite `inkscape:rounded` (still refuses `inkscape:randomized ≠ 0`); `StarData` gains
  `rounded`; `star_geometry` builds the shape with it and keeps its existing match check
  (`MATCH_MM` plus `rounding_reach` of the written digits), so a wrong formula fails safe into a
  path. Export: a polygon or star with Curve 0 and Randomized 0 writes
  `inkscape:rounded` with `attr_exact(q)` and our `d`. The shadow (`svg_import_shadow.rs` matches
  kind and exact length) **appends** `q`: polygon `[cx, cy, r, n, angle, turn, k, q]` (8), star
  `[…, k, ratio, q]` (9); the old 7 and 8 number forms still read, as `q = 0` (one test each).
- **7. 2026-11-23: golden fixtures (criterion 4).** The customer makes them in Inkscape 1.4: open
  `curvyo-svg-core/tests/fixtures/svg_real/inkscape/inkscape_mm_a4.svg` (mm user units, it has the
  5-point star `s1` and hexagon `s2`); Preferences, Input/Output, SVG output: path data
  **absolute**, numeric precision **16**; duplicate each, set `inkscape:rounded` in the XML editor to
  -1, -0.3, 0.3, 1, 3 for the star with `sodipodi:r2 = r1/2` (Ratio 0.5) and the hexagon; label them
  `star-q<value>`, `hex-q<value>`; save as Inkscape SVG to
  `curvyo-ui-core/tests/fixtures/svg/inkscape_1_4_rounded.svg` (document-core cannot read SVG). The
  test compares nodes and handles within 0.001 mm **up to a cyclic shift** of the start node.
  Until it arrives, document-core tests use hand-computed values of decision 1 (e.g. hexagon
  `R = 10`, `q = 0.3`: every handle 3 mm along `t_i`), each citing the issue the lead opens for the
  fixture (no `TODO` without an issue link); the PR's Review guide says criterion 4 is covered
  provisionally.
- **8. 2026-11-23: Randomized (slice 3, criterion 17).** Registers `randomized` (`f64`, ±10) and
  `seed` (32-bit). PRNG: the existing `splitmix64` of `pattern_model.rs` (reference-tested,
  portable integer arithmetic); seed creation and New seed reuse `PatternSeed`'s fresh/re-roll,
  renamed to one shared seed type in a refactor commit, never a twin. Draw `k` of vertex `i`:
  `splitmix64(seed ^ splitmix64(4i + k))`, top 53 bits to `[−0.5, 0.5)`. Inkscape's structure,
  our numbers: vertex shift `r·2R_max·(d1, d2)`, handle (from the unshifted ring) turned by
  `r·π·d3` and scaled by `1 + r·d4`. Inkscape's own generator (LCG 69069 seeded by a hash of the
  point's coordinates) cannot be reproduced from a stored seed, so randomized Inkscape stars
  import as paths and Curvyo exports randomized shapes as paths with the shadow, never as
  `inkscape:randomized ≠ 0` (Inkscape would redraw them with its own numbers).
- **9. 2026-11-23: slice 2 (rectangle, ellipse).** Both evaluate on a **unit 4-ring, then the
  affine scale** (as `Ring.scale` does for Curve). Ellipse ring E, S, W, N: `s = √2`, so
  `q_c = KAPPA/√2 = 0.390524…` gives handles exactly `KAPPA`, i.e. **exactly today's
  `ellipse_outline`** (radial error 0.027 % < 0.05 %); the snap writes `q_c` exactly, a typed
  0.39 stays within 0.05 % too. Rectangle ring = its corners (unit square `(±1, ±1)`, scaled by
  `w/2, h/2`): the tangent at a corner is perpendicular to the diagonal, so every `q > 0` bulges
  the sides **out of the box**, and `q_c` is the ellipse **through the four corners** (√2 larger
  than the box), not a soft square. An ellipse with an arc and a Rounded value: the arc wins and
  Rounded is ignored until the arc is whole (`0021` decision 2). Slice 2 is not Ready until
  finding B is answered and finding C applied.

## Findings for the product owner (spec text, not decisions)

- **A. "Normal range 0 to 1, never a loop" is not true.** Probe of decision 1 (flattened
  self-intersection): hexagon first loops near q 1.2 (a polygon side loops iff `q·cos(π/N) > 1`),
  star N 5 Ratio 0.5 near 1.85, Ratio 0.3 near **0.95**, Ratio 0.9 near 1.2; every `q < 0` loops
  at once. Proposal: keep the marker at q = 1 as "end of the normal range", drop "never a loop".
- **B. Criterion 15's soft square does not exist on the corner ring** (decision 9). A Frage for
  slice 2: A rectangle ring on the corners as specified (pillow, then circumscribed ellipse);
  B Rounded only on ellipse; C a squircle on the rectangle as its own parameter. Default B.
- **C. Criterion 7** must read "for `q ≠ 0`, touches the extreme points wherever the outline leaves
  today's box" (decision 5); a tight box at `q = 0` would change accepted behaviour.
- **D. Stale references**: the spec calls itself "new spec 0144" (0144 is QR and barcodes) and
  cites `format_version` 51 (53 now).
