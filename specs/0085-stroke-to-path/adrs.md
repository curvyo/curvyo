# ADRs for "Stroke to path"

The stroke outline is built from exact pieces with the offset machinery of
`0038` (parallel lines, `kurbo` offset cubics, true arcs, miter and bevel
lines), and `i_overlay` decides the topology on the 0.001 mm grid. **No new
ADR, no new crate, no new dependency, no `Cargo.toml` change, no trait, no
generic, no `format_version` bump** (results are ordinary paths and compound
paths, criterion 23). Nothing in this slice is `needs-customer`: the engine is
an application of the ADR 0003 §4 amendment of 2026-10-12, which the customer
approved. Customer answers of 2026-10-26: Frage 45 A, 46 A, 47 A.

## Depends on

- [ADR 0003 §1, §3, §4 (amendment 2026-10-12), §7](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md):
  the kernel lives in `curvyo-geometry-core`, takes an explicit `Tolerance`
  (0.01 mm), results are curves rebuilt from exact pieces, `i_overlay`
  (`=9.0.1`, integer engine) decides topology, the polyline is the
  per-outline fallback. The display tolerance of `curvyo-render-core` is not
  reused by the kernel.
- [ADR 0002 §5, §9](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  sibling order is z-order (outline directly above its fill part); one
  interaction, one commit, every id resolved before the first write.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md):
  a linked clone is stroked from the outline and style it resolves through
  `clone_link`; `visible` and `locked` resolve through ancestors (hidden and
  locked objects are never subjects, criterion 18).
- [ADR 0020](../../docs/adr/0020-text-nodes-text-engine-and-font-sources.md):
  texts are converted by the existing Object to path route, uncommitted, in
  the same commit (criterion 15).
- [ADR 0011 §3, §6](../../docs/adr/0011-workspace-and-crate-layout.md): crate
  edges unchanged. `render-core` and `geometry-core` stay unconnected; see
  decision 4 for what that costs.
- [`specs/0016-boolean-operations/adrs.md`](../0016-boolean-operations/adrs.md):
  grid, `MAX_COORDINATE_MM`, refusal codes with offenders and the refusal
  outline layer (criterion 26).
- [`specs/0038-path-offset/adrs.md`](../0038-path-offset/adrs.md): entries of
  2026-10-12 and 2026-10-13 (exact pieces, tagged flattening, Asymmetric
  smooth nodes, golden test native and wasm32 at 1e-6 mm).

## Feature-local decisions

### 1. 2026-10-26: engine, exact pieces (option A) over `i_overlay`'s stroke builder (option B)

| | A: `offset_pieces` two-sided + caps + union | B: `i_overlay` `StrokeOffset` |
|---|---|---|
| Result | lines, offset cubics, true arcs; a circle stroke is 2 x 4 smooth arcs (criterion 7) | polylines; a 1 mm round cap needs dozens of chords for 0.01 mm |
| "Clean curves" rule, criteria 4, 7, 12 | met | missed (criterion 7 and the 2-cubic round cap are impossible) |
| Self-intersection | `i_overlay` overlay of the raw loops, same as B internally | built in |
| Dashes, markers | neither has them; we split and add loops ourselves in both | same |
| Miter limit 4 | `miter_within_limit`, same rule as lyon's `MITER_LIMIT` (tip at most 2 w from the vertex) | `Miter(2 asin(1/4))`, same |
| wasm size | a few new modules; `i_overlay` and `kurbo` already linked | same (stroke builder already linked by the 0038 fallback) |
| Speed | 0038 measured ~1 ms for a 200-node gear | similar, more vertices downstream |

A wins on the only axis that differs: the result must be clean curves. B stays
what it is in 0038: the polyline fallback for a ring that cannot be mapped
within 0.01 mm (criterion 12, last sentence).

### 2. 2026-10-26: the construction

- **Raw loops per subject**, all in one overlay:
  - an open outline or a dash interval: one loop = right-side offset forward
    with joins, end cap, left-side offset backward with joins, start cap;
  - a closed undashed outline: two loops, the right-side offset of the
    outline and of its reverse (outer and hole for a square, criterion 5);
  - every marker: one closed loop of its exact shape (arrow 3 lines, dot
    4 quarter arcs), placed as decision 4 says.
- Caps: Butt one line, Square three lines, Round one half circle as two
  quarter arcs (`offset_pieces::arc`, criterion 4). Joins: `offset_pieces`'
  corner code with the stroke's join; miter falls back to bevel by
  `miter_within_limit` (criterion 6).
- **Union: `i_overlay` integer overlay with the NonZero fill rule** (the
  spec's "nonzero union", and what Skia and lyon fill a stroke with), through
  `offset_rebuild::rebuild` with a NonZero `keep` closure. The inner
  swallowtails of tight curves lie inside the swept area and are kept.
- Output form: `offset_finish::canonical` (criterion 12; smooth nodes are
  `AnchorKind::Asymmetric`, as ruled in 0038). An empty result on the grid is
  "no stroke", not a refusal (criterion 27).

### 3. 2026-10-26: where the code lives

- `curvyo-geometry-core`: `stroke_outline.rs`, one pure public function
  `stroke_outline(outlines: &[Outline], stroke: &StrokeShape, tolerance: Tolerance)
  -> Result<OffsetOutlines, StrokeOutlineError>`; `StrokeShape` holds width
  (`Length`), join, cap, dash pattern and phase, marker settings, all as
  document-core types. Helpers in their own modules, one responsibility each:
  `stroke_loops.rs` (decision 2's loops), `stroke_dash.rs` (dash intervals on
  `PathMeasure`), `stroke_markers.rs` (placements and shapes). The needed
  `offset_pieces` functions become `pub(crate)`; nothing else changes there.
- `curvyo-ui-core`: `stroke_to_path_plan.rs`: subjects (groups at any depth,
  clones as shown, texts, skips and counts for criteria 18, 22, 27), the
  refusals of criteria 25 and 26, the kernel call per subject, fresh anchor
  ids. `history_names.rs` maps `stroke_to_path` to "Stroke to path".
- `curvyo-document-core`: `stroke_to_path_write.rs`, one command that in **one
  commit** `stroke_to_path` unlinks clones of converted originals, converts
  texts and primitives as needed (the existing `*_uncommitted` routes), sets
  Stroke off on fill parts and inserts each outline directly above
  (criteria 19, 21). A subject with Fill off is replaced by its outline and
  keeps its id, as `convert_to_paths` keeps ids. A primitive fill part stays
  the same primitive (criterion 14). Guides are filtered in the plan.
- `curvyo-editor-wasm`: `session/stroke_to_path.rs` and the Select bar binding
  in `wasm_select_bar.rs`, in the pattern of `session/offset.rs`.

### 4. 2026-10-26: matching the canvas (criterion 11), tolerance stated

The canvas draws with `lyon` at display tolerance in `render-core`; the kernel
works on exact geometry. Adding a `render-core → geometry-core` edge to share
the dash and marker walks was rejected (ADR 0011 change for two small
functions). Instead the kernel ports the semantics of `dash_walk.rs`
(odd pattern doubled, phase, zero-length dots) and `marker_place.rs` (ends and
nodes from the control polygon, spaced markers by arc length) onto exact arc
length. **Tolerance:** stroke edges within 0.01 mm of the exact stroke
(criterion 12); dash ends and marker positions within 0.05 mm of where
`render-core` puts them, checked by a parity test in `curvyo-editor-wasm`
(the one crate that sees both) on the criterion 11 shapes and the 29 (d) path.
If the parity test fails on long curves, `render-core` measures dashed paths
at a tighter tolerance; the kernel is the reference. The duplicated walk is
logged by the architect in `docs/technical-debt.md` ("Dash and marker walk
exist twice").

### 5. 2026-10-26: determinism and budgets

Golden files in `tests/fixtures/stroke_to_path/` join the `offset_golden`
cross-target run (native and Node wasm32, 1e-6 mm, segment counts;
criterion 28). The criterion 29 budgets run in the `boolean-budgets` CI job,
release build. Per subject one overlay; a text's glyphs are separate subjects,
so 500 Hershey characters are 500 small overlays, not one large one.
