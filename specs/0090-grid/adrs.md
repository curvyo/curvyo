# ADRs for Grid

Date: 2026-10-31. Customer-decided: the grid is a property of each page (GRID-HOME,
GRID-ORIGIN), drawn and snapped to only inside its page (GRID-CLIP), only a shown grid
pulls (GRID-SNAP-HIDDEN), one grid snap point choice (GRID-SNAP-POINT); old-file
compatibility waived (2026-10-30). Everything below is **Proposal**, reversible inside
the story. No new ADR, crate, dependency, trait or generic: the per-page registers
extend ADR 0024's per-field page registers, the snapping registers extend `0060`.

## Existing decisions

- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md): millimetres,
  `Length`/`Point`/`Vec2`/`Angle` (radians) newtypes, explicit tolerances.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md): container
  `format_version`; Loro stays inside `curvyo-document-core`.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md): independent registers
  merge fieldwise; view state (zoom, levels, held snaps) is ephemeral.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md): layout and snapping
  logic in `curvyo-ui-core`; `curvyo-render-core` cannot depend on `curvyo-ui-core`.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one labelled operation per
  settings change, fieldwise restore.
- [ADR 0024](../../docs/adr/0024-spatial-pages.md): pages as `page.<id>.<field>` root
  registers; the grid adds fields to the same scheme.
- [`0060` adrs.md](../0060-snapping/adrs.md): resolver, raw hysteresis, axis mask,
  marker, Alt inversion, frame budget method; reused unchanged where not stated here.

## Feature-local decisions

### 1. Page grid storage (`curvyo-document-core/src/page_grid.rs`)

- 2026-10-31: New module `page_grid.rs`, one responsibility: the grid registers of one
  page. Fields through the existing `pages::key(id, field)`, so the first page uses
  `page.first.grid_*`:

  | Key field | Loro value | Default | Valid on open |
  |---|---|---|---|
  | `grid_shown` | Bool | `false` | Bool |
  | `grid_mode` | String `"adaptive"` / `"fixed"` | adaptive | one of the two |
  | `grid_scale` | String `"linear"` / `"log"` | linear | one of the two |
  | `grid_spacing` | Double, mm | 5.0 | finite, > 0 |
  | `grid_subdivision` | I64 | 5 | 2 ..= `u32::MAX` |
  | `grid_angle` | Double, radians | 0.0 | finite |
  | `grid_offset_x`, `grid_offset_y` | Double, mm | 0.0 | finite |

  Lengths carry no unit suffix, like the page's own `x`, `width`; the angle is radians
  like every other angle register (ADR 0002 §3). Strings, not numbers, for the two
  enums so a file stays readable and a third value is a refusal, not a silent alias.
  No upper limit on spacing or offset (`CLAUDE.md` §3); the `u32` bound is the type's.
- 2026-10-31: Rust value `PageGrid { shown: bool, mode: GridMode, scale: GridScale,
  spacing: Length, subdivision: u32, angle: Angle, offset: Vec2 }` with
  `PageGrid::DEFAULT`; `GridMode { Adaptive, Fixed }`, `GridScale { Linear, Logarithmic }`.
  `Page` gets a field `grid: PageGrid`, filled by `pages_of_root` in the same read, so
  every consumer that already reads pages (frame, snapping gather, page panel) gets the
  grid with no extra document read. `Serialize` lower-case, so `document.json` carries it.
- 2026-10-31: Normalisation on write: angle `rem_euclid(TAU)`, and a result within
  `ANGLE_EPS = 1e-12` rad of `TAU` becomes 0, so 360° and -0° store 0. A stored finite
  angle outside `[0, TAU)` reads normalised and is not damage (permit over refuse).
  Refused with `PageGridError` (thiserror), writing nothing: non-finite spacing, angle
  or offset; spacing ≤ 0; subdivision < 2. The degree conversion and the 70-character
  notices live in `curvyo-editor-wasm`/frontend, never in the register.
- 2026-10-31: Reads are lenient (absent or malformed field reads as its default, the
  read writes nothing, a merged replica never panics), each field independently, as
  `snapping_of_root`. `page_grid::validate(&LoroDoc)` is the strict check, called next to
  `snapping::validate` in `Document::from_loro_snapshot`: every root key
  `page.<id>.grid_<field>` with a known field must hold the type and range above or the
  open fails with `OpenError::Damaged`, deleted pages included. A container value is
  damage. Fields are independently valid, so no cross-field merge repair is needed.
- 2026-10-31: Writer `Document::set_page_grid(id, PageGrid) -> Result<bool, PageGridError>`
  writes only the fields that differ, as one commit labelled `set_page_grid` ("Change
  grid" in `curvyo-ui-core/src/history_names.rs`); the Shift+click toggle and Reset to
  page corner go through the same writer (one label, like `set_document_snapping`).
  Unknown page is `PageGridError::NotFound`.
- 2026-10-31: New document writes no grid register (absent = default, nothing to
  rewrite). `add_page` copies the resolved grid of `last` (the page it takes its size
  from), writing only fields that differ from `PageGrid::DEFAULT`, inside its existing
  `add_page` commit. Move, resize and reorder never touch grid registers: the offset is
  relative to `page.origin` (top-left corner, which `resize_page` keeps), so the grid's
  global origin is always `page.origin + offset`. Delete keeps them under the tombstone.

### 2. Snapping registers and version

- 2026-10-31: `curvyo-document-core/src/snapping.rs` gets `snapping_grid` (Bool, default
  `true`) and `snapping_grid_point` (String `"centre"` / `"box"` / `"nodes"`, default
  centre). `DocumentSnapping` gains `grid: bool` and `grid_point: GridSnapPoint { Centre,
  Box, Nodes }`; read, `validate`, `SnappingJson` and `set_snapping` follow the existing
  pattern. The grid point is a document setting, not per page.
- 2026-10-31, renumbered 2026-11-01: `CURRENT_FORMAT_VERSION` 29 → **30** (main took 29
  for `0066` revision 4 first), with a doc paragraph like the 27/28/29 ones. Goldens in
  `curvyo-document-core/tests/fixtures/`: `grid_v30.curvyo` (two pages with different
  non-default grids, one rotated, logarithmic and with a bottom-left origin corner,
  `snapping_grid = false`, `snapping_grid_point = "nodes"`), and one malformed fixture per
  register type (`grid_spacing_zero_v30`, `grid_subdivision_one_v30`, `grid_mode_unknown_v30`,
  `grid_angle_nan_v30`, `grid_shown_type_v30`, `grid_origin_unknown_v30`,
  `snapping_grid_point_unknown_v30`). Older fixtures stay as they are; no migration.

### 3. Units and tolerances

- 2026-10-31: Document lengths are `Length`/`Vec2`/`Point` (mm), the angle is `Angle`.
  Screen quantities in the new code are a `ScreenPx(f64)` newtype in
  `curvyo-ui-core/src/viewport.rs` beside `Zoom` and `PX_PER_MM_AT_100`, with
  `to_length(scale)`. The thresholds are named constants: `FADE_START = ScreenPx(4.0)`,
  `FADE_FULL = ScreenPx(12.0)`, `LOG_REFINE = ScreenPx(48.0)`. Snapping's existing bare
  `f64` pixel zones are not migrated in this story (debt, see report).
- 2026-10-31: Tolerances, each explicit at its use: `LEVEL_EPS = 1e-9` added to
  `log_s(z)` before `floor` so 500 % and 2500 % land on their level exactly;
  `ANGLE_EPS = 1e-12` rad for "axis aligned" (decision 5); page containment uses
  `Page::contains` (1e-9 mm); a snapped coordinate equals the grid coordinate within the
  resolver's `EPS_MM = 1e-9` mm (criterion 15).

### 4. Grid layout (`curvyo-ui-core/src/grid.rs` + `grid/`)

- 2026-10-31: New module beside `ruler` and `snapping`, pure arithmetic, no document
  read: `grid/frame.rs` (`GridFrame`: global origin, cos/sin; `to_grid`, `from_grid`),
  `grid/level.rs` (adaptive levels, fade, fixed thinning), `grid/log.rs` (decade table),
  `grid/layout.rs` (clipping, line list), `grid/snap.rs` (decision 5), `grid/tests.rs`.
- 2026-10-31: Entry point
  `pub fn grid_layout(page: &Page, view: ViewTransform, visible: (Point, Point)) -> GridLayout`
  where `visible` is the canvas rectangle in document mm and `GridLayout { lines:
  Vec<GridLine> }`, `GridLine { start: Point, end: Point, emphasis: f32, alpha: f32 }`.
  Each line is already clipped (Liang-Barsky) to page rectangle ∩ visible rectangle, so
  the renderer clips nothing. Hidden grid → empty layout.
- 2026-10-31: ui-core owns every formula; render-core only multiplies `alpha` into the
  colour and blends minor/major colour by `emphasis`. With `s(j)` the screen spacing of
  level `j` (`base · subdivision^-j · scale`): `alpha(j) = ramp(s(j); 4, 12)`,
  `emphasis(j) = ramp(s(j+1); 4, 12)`. A line is emitted once, at its coarsest level
  (levels nest, criterion 5). This gives the spec's 25/5/1/0.2 mm at 20/100/500/2500 %
  and no jump of either opacity or major weight at a level change. Fixed: minor lines
  at alpha 1 while `s ≥ 4 px`, majors (`subdivision`-th) while `≥ 4 px`, else nothing.
- 2026-10-31: Log: decade length `L = base · subdivision` from the grid origin, decades
  repeat by translation in both directions (same orientation). Positions `log10(n)·L`;
  an inner line is emitted only while its smaller gap to a neighbour is ≥ 4 px, in both
  modes (no moiré, bounded count; not in the spec, see report). Adaptive refinement: a
  cell wider than 48 px gets tenths, recursively per cell. Decade lines have emphasis 1.
- 2026-10-31: Bound by construction: every emitted level has screen spacing ≥ 4 px, so
  lines per axis ≤ visible extent / 4 px, summed over a geometric series of levels.
  Iterate only the index range that intersects page ∩ visible, never the page or the
  document. No object, page count or zoom term in the cost.

### 5. Grid snapping

- 2026-10-31: Computed, not gathered: grid targets never enter `SnapTargets` or its
  index. `grid/snap.rs` exposes `pub fn snap_to_grid(input: &GridSnapInput, settings:
  &SnapSettings, held: &mut Option<HeldGrid>) -> Option<GridCandidate>` and a direction
  variant for `resolve_probe_along` callers (nearest grid-line crossing along the
  permitted direction). It uses `grid/level.rs` so the targets are exactly the lines the
  layout draws with alpha 1 and their intersections (WYSIWYS); fading lines are not
  targets. Per probe point: transform to grid coordinates, nearest line per grid axis
  (linear: `round`; log: binary search in the level's decade table), the intersection of
  both; drop anything outside the page rectangle. Cost O(1) per probe point.
- 2026-10-31: Page: for each grid probe point the page whose `contains` is true, the last
  in list order (drawn on top) if pages overlap; no page, hidden grid,
  `settings.grid == false` or snapping effectively off (Alt XOR stored) → no candidate.
- 2026-10-31: Axes: the `0060` X/Y mask is applied in grid axes (u, v): X off forbids
  corrections along u. Capture and release in mm from the current scale, measured on raw
  geometry; `HeldGrid { probe, page, feature: Intersection(i, j) | U(i) | V(j), level }`
  in `SnapState` releases on distance, level change, or document version change.
- 2026-10-31: Combination lives in `curvyo-ui-core/src/snapping/grid.rs`:
  `resolve_with_grid` wraps `resolve` and is what `session/snapping.rs` calls. With an
  axis-aligned grid (`angle ≤ ANGLE_EPS`), a grid line competes per axis with the indexed
  result and a grid intersection competes as an atomic point, the `0060` point-versus-line
  rule; with a rotated grid the grid candidate is atomic against the whole indexed
  correction. Smaller correction wins; an exact tie goes to the indexed target (grid
  after page fractions); inside the grid an intersection beats a line, then probe order.
  Marker: `MarkerLine::Vertical/Horizontal` axis aligned, otherwise `MarkerLine::Curve`
  with zero handles along the rotated line clipped to the page; cross at the point.
- 2026-10-31: Grid probe is separate from the indexed probe (`SnapProbe` for boxes stays
  as is). The session builds `grid_points` per tool: Select move from
  `snapping_grid_point` (Centre: box centre; Box: the nine reference points; Nodes: the
  same node set `SnapTargets::add_path_nodes` uses, an object without anchors gives its
  box centre); resize: the dragged handle's reference point; Node tool: the dragged
  anchor; Pen and shapes: the placed point.
- 2026-10-31: `session/snapping.rs`: `SnappingDrag` keeps the `Vec<Page>` it already reads
  at `begin_drag` and on a version change; the level is recomputed from `self.view()` on
  each resolve. No document read per frame. `SnapSettings::from_document` carries `grid`.

### 6. Rendering

- 2026-10-31: `curvyo-render-core/src/grid_lines.rs`: `GridStroke { start, end, emphasis,
  alpha }` and `build_grid_lines(&[GridStroke], view) -> DrawList`, constant screen
  width, colours from new `theme` grid tokens (values from `docs/design-system.md`, UX).
  Its own input type because render-core cannot see ui-core; the session maps one-to-one.
- 2026-10-31: Only `Session::frame_draw_list` (`session/frame.rs`) draws the grid: each
  shown page's grid right after that page's area layer and before the next page's area
  and the artwork, from `shown_pages()` (so it follows a page drag). Not in the history
  version preview (0042 criterion 19 "plain"), never in `draw_list`, export, print,
  machine output, eyedropper (`colour_pick.rs` reads document geometry) or hit testing.
- 2026-10-31: Budget: a thread-CPU test in `curvyo-editor-wasm/tests/cpu_time/`: frame
  with grid shown minus hidden, 10,000 objects, 3840 × 2160 canvas, worst cases (fade
  band, rotated, log adaptive), ≤ 1 ms; plus a grid-snap case with the Nodes choice.

### 7. Editor and host

- 2026-10-31: `curvyo-editor-wasm/src/session/grid.rs`: active page's grid record (with
  page name and degrees), `set_active_page_grid`, `toggle_active_page_grid`; refusals map
  `PageGridError` to the notices. UI reflects the document after undo, redo and merges.

## Accepted deviations (architect review, 2026-11-01)

From `review.md`; each overrides the decision text above where they differ.

- Decision 1, history: one label "Change grid" for every grid edit, as decided.
- Decision 1, `document.json`: a default grid and default grid-snapping values are left out
  of `document.json` (`pages.rs`, `snapping.rs`). `document.json` is a derived export, the
  `.curvyo` Loro registers are the source of truth, and older goldens' `document.json` stay
  byte-identical. "so `document.json` carries it" holds only for non-default values.
- Decision 1, equality: two normalised angles within `ANGLE_EPS` around the circle are equal,
  so 390° over a stored 30° writes nothing and adds no history step (tester finding).
- Decision 3: snapping pixel zones stay bare `f64`; debt entry in `docs/technical-debt.md`.
- Decision 4: Fixed mode fades between 12 and 4 px (`grid/level.rs`), following spec
  criterion 6, instead of "alpha 1 while `s ≥ 4 px`".
- Decision 5: `HeldGrid` has no `level` field. `retain` checks each frame that the held line
  is still drawn at full opacity; the release behaviour is the same and nothing goes stale.
- Decision 5: 90°, 180° and 270° count as rotated (atomic competition, curve marker); the
  spec asks for document axes only at 0°.
- Decision 5, API: the direction variant `snap_to_grid_along` and `ScreenPx` are crate
  private; `snap_to_grid`, `GridSnapInput`, `GridCandidate`, `GridFeature` and `HeldGrid`
  stay public because the 0090 acceptance tests call `snap_to_grid` directly.
- Decision 5, Node tool: the grid probe is the pressed anchor, kept by the node drag.

## Amendment: origin corner and rulers (2026-11-01)

Customer feedback on PR #238 and the answers ORIGIN-AXES A, ORIGIN-FIELDS B (spec criteria 16
to 19). Each point overrides the decision text above where they differ.

- Decision 1, register: the origin offset (`grid_offset_x`, `grid_offset_y`) and Reset to page
  corner are removed. `PageGrid.origin: OriginCorner { TopLeft, TopRight, BottomLeft,
  BottomRight }` is one String register `page.<id>.grid_origin` (`"top_left"`, `"top_right"`,
  `"bottom_left"`, `"bottom_right"`), absent = `top_left`, anything else is damage on open.
  It lives in `PageGrid` so Add page copies it with the grid and it goes through the same
  writer and label ("Change grid"). Still format 30: the offset never shipped.
- Decision 1, geometry: `Page::zero_point()` is the page's origin corner in document
  coordinates; `Page::measured(point)` and `Page::placed(position)` convert to and from a
  position measured from it, positive into the page (`OriginCorner::signs`). Move and resize
  need no register write: the corner is relative to the page rectangle.
- Decision 4, grid frame: `GridFrame` starts at `page.zero_point()` and flips its axes with
  the corner's signs, so the first lines run along the two page edges at the corner and log
  decades grow into the page; rotation stays clockwise on screen.
- Rulers: `curvyo-ui-core/src/ruler.rs` gains `RulerDirection { Forward, Reversed }` and
  `ruler_layout_in` (`ruler_layout` keeps its signature and counts forward). The session's
  ruler layout puts 0 at the active page's zero point and reverses x for a right corner and y
  for a bottom corner. The host repaints the rulers on `ruler_origin_key()` (zero point and
  corner), which replaces `active_page_x`/`active_page_y`.
- Typed positions: `MoveEntry` (`curvyo-ui-core/src/move_entry.rs`) measures Absolute from a
  zero point and signs set by `measure_from(&Page)`; the session sets the active page right
  before a commit and shows the prefills with `absolute_prefill_from`. Relative offsets keep
  the document directions. This also makes Absolute page-relative on a page other than the
  first (it measured from document 0 before), which matches the rulers.
- Frame cost (criterion 9, CI finding): `pages_of_root` gathers the `grid_*` registers in the
  single pass over the root map it already makes, so a page without grid registers costs no
  keyed read per frame; before, every page read eight registers on every `pages()` call.
