# ADR 0036: Variable-width strokes

**Status:** Proposed. `needs-customer` (document model: a new path register; file formats: the
next `format_version` and `curvyo:` SVG profile v2). Questions 135 to 139 below. Extends ADR 0002
§5 (path schema), ADR 0003 §5 (medial axis), ADR 0018 §9 (SVG profile); feeds specs `0033`
(brushes), `0085` (stroke to path). Requirements: R-EDIT-050, prerequisite of R-VEC-006,
R-MFG-EMB-004 and the width input of R-MFG-CNC-002.

## Context

A path's stroke has one `stroke_width`. Three planned features need a width that changes along
the path: drawn variable strokes (R-EDIT-050), a centre line whose width follows the local
thickness of a region (R-VEC-006, a medial axis) and a satin column from one path (R-MFG-EMB-004).
V-carving (R-MFG-CNC-002) needs the same medial axis with depth instead of width.

What exists: anchors live in a movable list with a stable `AnchorId` each (ADR 0002 §5,
`path_codec.rs`); dashes are stored in multiples of the stroke width (`0007`); stroke to path
(`0085`) builds exact pieces with `i_overlay` `=9.0.1` deciding topology; `0033` (Ready, not built)
plans width-profile brushes with a normalised profile times `stroke_width`, and `0032` plans
`variable_outline.rs` on `i_overlay`'s variable stroke for pressure strokes (stored as filled
outlines, no bump). The document is at `format_version` 35.

### Options considered: representation

- **A. Width stops anchored to a segment (chosen).** A stop is (`AnchorId` of the segment's start
  anchor, `u` = arc-length fraction in that segment in `[0, 1)`, width). Between stops the width
  is linear in arc length; before the first and after the last it is constant.
- **B. A width per anchor.** Simplest storage (one more key per anchor), but a width change needs
  a node there: a taper in the middle of a curve adds a node and changes the shape's editing.
  Medial-axis output would fit, user editing would not. Lost on R-EDIT-050's "stops on the path".
- **C. Stops at a global `t` in `[0, 1]` of the whole path, or a separate profile object.** `t`
  shifts every stop whenever any node moves or is added (a node edit on the left end slides the
  taper on the right end); a separate profile object (Inkscape PowerStroke as an effect) adds a
  node kind, a reference and a clone story for one list. Lost on stability and on size.

### Options considered: relation to brushes

- **A. One width function, two sources (chosen).** `geometry-core` gets one `WidthAlong` value
  (sorted arc-length positions with widths) and one outliner. Stops produce it; with no stops it
  is the constant `stroke_width`. A `0033` brush takes `WidthAlong` instead of a scalar width, so
  a width-profile brush (Taper) multiplies the stops and stamp and pattern brushes scale by the
  local width.
- **B. Either stops or a brush on one path.** Two outliners, and a rule the maker must learn.
- **C. Width-profile brushes become presets that write stops.** Loses Taper's live parameter
  (the ramp no longer follows a length change) and rewrites a Ready spec.

## Decision

1. **Storage (Proposal, Q135, Q136).** A path and a compound path get one new register map
   `width_stops` next to `anchors` in the node's meta map: key = stop id (minted like an
   `AnchorId`), value = one atomic LWW register `[anchor_hex, u, factor]`. `factor` is a
   `WidthFactor` newtype in `[0, 16]`, a multiple of `stroke_width` (as dashes), so the panel's
   width field scales the whole profile and a scaled object scales it with the stroke. Absent or
   empty map = uniform stroke. Per-stop registers keep two concurrent stop edits apart (ADR 0009
   §3). The stops are geometry, not style: they are not in the style schema, not copied by paste
   style, not an `own_style` key; a linked clone reads them with the original's outline (ADR
   0015). A primitive gets stops only through the silent conversion to a path (`0047`). Widths
   are symmetric about the centre line (Q139).
2. **Edits keep stops by rule, in the same commit as the edit** (`document-core`, one function per
   rule, golden tests):
   - move a node or a handle: unchanged; the stop follows its segment by arc-length fraction.
   - insert a node at `u0` in a segment: stops with `u < u0` get `u / u0`, the others move to the
     new anchor with `(u - u0) / (1 - u0)`; the inserted node gets a stop at the interpolated
     width only if the segment had none on both sides (shape unchanged).
   - delete a node: stops of both segments are re-keyed to the merged segment by their arc-length
     position before the delete; delete an end node of an open path drops its segment's stops.
   - reverse: a stop (A, `u`) on segment A to B becomes (B, `1 - u`).
   - split, cut at crossings, break apart: each piece keeps the stops of its segments; a cut
     segment splits as an insert. Join and combine keep both sets; the joining segment gets none.
   - booleans, offset, fracture, flatten, weld: results are new outlines with new anchors and no
     stops (uniform `stroke_width`); the command's existing notice says "width stops removed"
     when a subject had any.
   - a stop whose anchor is gone after a merge is ignored on read and dropped by the next write of
     that path (no repair commit).
3. **Rendering and stroke to path.** `geometry-core::variable_outline` (the module `0032` and
   `0033` already name) flattens the centre line at manufacturing tolerance, inserts a vertex at
   every stop, gives each vertex its width and calls `i_overlay`'s variable stroke, then cleans
   up with the existing `offset_rebuild`. Joins and caps are round for a stroke with stops (what
   the engine offers; Proposal, the panel shows Join and Cap as inactive). Dashes cut the centre
   line first, as in `0085`. `editor-wasm` composes as in `0033` decision 1; `render-core` does
   not depend on `geometry-core` (ADR 0011 §3 unchanged). Stroke to path of a path with stops
   writes that outline: a polyline on the 0.001 mm grid, not exact pieces (there is no exact
   Bézier offset at a varying width); curve refit waits for R-VEC-003. Hit-testing uses the same
   outline. A job (laser, plotter) reads the outline, never the centre line.
4. **SVG export (Q137).** A path with stops is written as a filled `<path>` of its outline in the
   stroke paint (correct in every SVG reader and for every laser tool), carrying
   `curvyo:centre` (the centre line as path data) and `curvyo:width-stops`
   (`anchor-index u factor` triples, anchor index in document order). This is `curvyo:format="2"`:
   the v2 reader still reads v1 files; it keeps the centre line and stops only when their outline
   matches `d` within 0.01 mm, otherwise the filled outline wins. A v1-only reader (older builds)
   sees a filled shape. Import of Inkscape PowerStroke is not part of this ADR.
5. **Format.** `width_stops` takes the next free `format_version` at merge (36 if nothing merges
   first). An older build would draw a uniform stroke and a job would cut the wrong outline, a
   silent wrong output, so the bump is required (same reason as `0033`); per the policy in
   `specs/README.md` an older build refuses the file ("saved by a newer version"). A new build
   reads every older file as uniform strokes. Golden files `width_stops_v<N>.curvyo` plus damaged
   ones (factor out of range, `u` outside `[0, 1)`, unknown anchor).
6. **Medial axis: one implementation in `geometry-core::medial_axis`.** Input: closed outlines
   (with holes) and a `Tolerance`; output: a graph of polylines with a clearance radius per
   vertex. It follows ADR 0003 §5 (constrained Delaunay triangulation of the flattened boundary
   with `spade`, MIT OR Apache-2.0, pure Rust, robust predicates; added to `geometry-core` with
   this ADR's first story). Three callers, no trait: centre line with width (R-VEC-006: prune,
   chain, fit with `kurbo`, write `stroke_width` = largest diameter and stops = diameter /
   `stroke_width`), V-carve depth (radius / `tan(half angle)`, clamped) and later auto-satin.
   Trace does not get its own: R-VEC-006 from a trace runs on the traced outlines, so
   `vectorize-core` needs nothing new (its edge to `geometry-core` exists, ADR 0011 §3).
7. **Embroidery satin** reads the centre line and `WidthAlong` directly: rails at plus and minus
   half the width along the normal, stitches spaced by arc length, pull compensation added to the
   width. No second rail path is stored (Ink/Stitch uses two rails; one path plus widths is what
   R-MFG-EMB-004 asks).

## Consequences

- One width function and one outliner serve stops, brushes, pressure strokes and satin. `0033`'s
  `generate` takes `WidthAlong` instead of a scalar width (a feature-local note in its `adrs.md`
  when it is built; it is not built yet, so nothing is rewritten). `0032` could later store a
  pressure stroke as an editable centre line with stops instead of a filled outline; that is its
  own decision.
- `stroke_width` keeps its meaning as the reference width, so the panel, scaling and dashes need
  no second rule; a profile at factor 0 tapers to a point.
- Stroke-to-path output of a variable stroke has many nodes until curve refit exists (already in
  `docs/technical-debt.md` for booleans).
- Joins and caps other than round are not available with stops while the outliner is
  `i_overlay`'s; a miter variable stroke needs our own outliner on exact pieces, a later story.
- The SVG profile moves to format 2; every later profile change is a further number.
- **Open risk: medial-axis numerics.** CDT quality depends on boundary densification (spurious
  branches at fine flattening, jagged axes at sharp corners), on pruning (which branch is noise)
  and on degenerate input (touching holes, slivers thinner than the grid, collinear runs). Input
  size grows with densification: a full traced image can mean 10^5 to 10^6 boundary points in
  wasm. Before the R-VEC-006 story: a throwaway `spike/medial-axis` measures axis error against
  analytic shapes (rectangle, ring, circle, stroked letters), pruning on real trace output,
  native versus wasm32 determinism and time for 10^5 points; result recorded here. Escalation
  stays as in ADR 0003 §5: `boostvoronoi` (segment Voronoi, BSL-1.0, known bugs). For the lead:
  record this risk in `docs/technical-debt.md` with a link to this ADR.
- Satin from very wide stops (over about 8 mm) or a width larger than twice the local curvature
  radius folds its rails; the satin generator handles or refuses that (embroidery spec), the
  stored model does not change.

## Questions for the customer

**Frage 135 (VW-REPR): Where do width stops sit?**
A: On a segment (anchor plus fraction of the segment length); moving or inserting nodes keeps
them in place. B: One width per node; every width change needs a node. C: Fraction of the whole
path length; every node edit shifts all stops. Recommendation A. Default A.

**Frage 136 (VW-UNIT): Relative or absolute width?**
A: A factor of the stroke width (as dashes); the width field scales the whole profile.
B: Absolute mm per stop; the width field then only shows the maximum.
Recommendation A. Default A.

**Frage 137 (VW-SVG): SVG export of a variable stroke?**
A: Filled outline only; Curvyo reads it back as a plain shape. B: Filled outline plus centre line
and stops in `curvyo:` profile 2; other apps see the right shape, Curvyo gets the editable stroke
back. C: Centre line with a uniform stroke plus an attribute; other apps and laser software see
the wrong width. Recommendation B. Default B.

**Frage 138 (VW-BRUSH): Width stops and brushes (`0033`)?**
A: One width function: stops are the base width, a brush such as Taper acts on it. B: Either stops
or a brush on one path. C: Width-profile brushes write stops when applied (no live parameter).
Recommendation A. Default A.

**Frage 139 (VW-SIDE): Symmetric or separate left and right widths?**
A: Symmetric about the centre line (enough for satin, centre line and V-carve). B: Separate left
and right (as Illustrator's width tool): twice the data and handles, a satin special case.
Recommendation A. Default A; B later would be a further `format_version` step.
