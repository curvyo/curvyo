# ADRs for "SVG import and export"

Architect review, 2026-10-14, of the Draft specification and its six questions; reworked 2026-10-16 for
the customer's decisions (see the specification, "Decided (customer, 2026-10-16)"): first slice is the
import, export is a later slice, no loss report, no passthrough bag, `<use>` becomes a linked clone where
it can; reviewed again on 2026-10-26 against the code of that day (dated note below). The slice adds the
first importer (and, later, the first exporter). **One new ADR
([0018](../../docs/adr/0018-svg-import-crate-parser-and-limits.md), accepted 2026-10-26), one new crate
(`curvyo-svg-core`), two new dependencies (`svgtypes`, `simplecss`), no `format_version` bump**
(criterion 77 holds: no register, no node kind, no schema change; a linked clone uses the register of
`0049`). The one `needs-customer` item of the draft, the passthrough bag, is decided (BAG A, below).

## Depends on

- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  - §2, §4: mm is the document unit, Y grows down like SVG; px, pt, pc, in become mm once, at the
    import boundary (criteria 15 to 20). No unit conversion anywhere else.
  - §3: units are newtypes, every comparison has a tolerance; criteria 25, 27, 50 and 57 use
    the named tolerances (1e-6 mm, `TOUCH_DISTANCE_MM` 0.001).
  - §5: a style is resolved per leaf (so the importer resolves the CSS cascade itself; there is
    no inheritance in the model, ADR 0015 notwithstanding for ancestor-resolved state).
  - §10: SVG strategy: own subset, golden files. Two customer amendments of 2026-10-16 (ADR 0018 §4):
    the **named listed loss is dropped** (no report) and the **passthrough bag is not built** (BAG A).
    Gradient amendment of 2026-10-08 is what criterion 38 implements.
  - §11 with [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: no
    `format_version` change (criterion 77). The importer writes through the existing document
    API, so the project file is whatever the model writes (format 25 since `0058`, 2026-10-27).
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §5: the browser boundary
  carries strings and scalars. The importer is called from `curvyo-editor-wasm` with the file
  bytes; the 50 MiB limit is checked before the call. There is no report to return.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §2: its table gives
  `curvyo-document-core` "SVG import/export" in one sentence. That sentence is superseded by
  ADR 0018 (reason below).
- [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md) and
  [`0054` adrs.md](../0054-combine-open-paths/adrs.md): `touching_outlines`, `outline_nesting`
  and `TOUCH_DISTANCE_MM` of `curvyo-geometry-core` decide the even-odd re-winding (criteria
  50 and 51, EVENODD A); `kurbo` (already a workspace dependency) converts arcs.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md),
  [ADR 0016](../../docs/adr/0016-clone-overrides.md), `0049`, `0056` and `0057` adrs: **USE-CLONE B.**
  A `<use>` becomes a clone and (later) a clone is written as `<use>` where the relation to its
  original is a move, rotation or flip only (criteria 47 and 59). Checked against the model:
  a clone's `clone_link = { origin, matrix }` holds a relative matrix of any affine kind (ADR 0015
  §3); the matrix that the importer computes is `C_use · translate(x, y) · A_target⁻¹` (the target's
  own transform is baked into its anchors and cancels), so ADR 0016's rule that a transform of an
  original is baked and the clone stays relative is met without a new register. A clone's stroke
  and dashes are not scaled by M (`0049` criterion 9) whereas SVG scales a `use`'s stroke, hence only
  isometries (orthogonal linear part within 1e-6; the determinant may be -1, `0056`) are linked, in
  both directions. `own_placement` and `own_style` (ADR 0016 §1) are never set by an import and make
  a clone a `<path>` on export (`0057` criterion 40). Primitives are not clone origins (`0049`
  criterion 2 would convert them silently), so a `use` of a primitive is a copy. Chain depth 8
  (`0049` criterion 52) applies at import.
- `0050` adrs (layers are top-level groups with a marker) and `0023` (depth 32): criteria 44 and 45.
- `0046` stage 2 calls the same importer (criterion 73): it depends on `curvyo-svg-core`, not
  on a second parser.

## ADR 0018 "SVG import and export crate, parser and limits" (written 2026-10-26)

[ADR 0018](../../docs/adr/0018-svg-import-crate-parser-and-limits.md) records the proposal of
2026-10-16 as changed by the dated note of 2026-10-26 below. What it decides for this feature:

1. `curvyo-svg-core` reads bytes into `SvgDrawing` (mm, `kurbo` paths, styles resolved per leaf,
   clone leaves by tree index); it depends on no Curvyo crate.
2. `ui-core` module `svg_import` maps `SvgDrawing` to the document (primitives, EVENODD A
   re-winding with `geometry-core`, USE-CLONE B); `document-core` gets `insert_drawing`, one commit.
3. The hostile-XML guard and `path_data` move from `text-core` to `svg-core`; the import uses the
   DOCTYPE rule of criterion 68, fonts keep `0076` criterion 23.
4. Dependencies: `roxmltree` =0.20.0 (present), `svgtypes` =0.16.1 (attribute grammars, not path
   data), `simplecss` =0.2.2 (criterion 36).
5. BAG A and no report: amendments to ADR 0002 §10 (customer, 2026-10-16).
6. Limits of criteria 6, 67 to 69; determinism of criterion 72 (`f64` formatting, `libm` if
   targets ever differ).
7. `<text>` and `<image>` stay ignored (criteria 13, 49); the ADR says how a later story maps them.

## Feature-local decisions (dated notes)

- **2026-10-14: no new `format_version`, no new register.** Imported content is created with the
  existing constructors (`create_path`, `create_rect`, `create_ellipse`, `create_polygon`,
  `create_star`, groups, layers, styles). Shadow data lives in the SVG, not in the project.
  A linked clone is created with the `0049` command, not a new one.
- **2026-10-14: even-odd (EVENODD A) agrees with `0054`'s `adrs.md` "Flagged" and its
  decision 3 (nonzero stays the one fill rule; `fill-rule` must not enter a bag).** Differences to
  `0054` criterion 27, which `0024` criterion 76 amends in the building PR: re-winding by
  depth parity instead of "unchanged", and the listing only for non-reproducible paths. The
  PO text was tightened here: no re-wind and no entry when the fill is off; any
  `touching_outlines` pair counts as not reproducible (the predicate already exists, so
  there is one definition of touch); a path mixing an open outline with area and another
  outline is not reproducible either (even-odd XORs the chord-closed open outline, and
  reversing an open outline changes its cutting direction). The depth rule is parity of the
  number of containing outlines (`outline_nesting`), which keeps every winding sum at 0 or 1.
- **2026-10-14: DOCTYPE with literal entities is accepted (criterion 68, changed).** Older
  Adobe Illustrator exports declare `ns_ai`, `ns_extend` and others in an internal subset and use
  them in `xmlns:*="&ns_ai;"`. The draft refused every `<!ENTITY`, which refuses those files
  and makes criterion 19(a) unreachable for them. The bounded literal form cannot nest, so
  expansion is at most 100,000 x 256 bytes.
- **2026-10-14: `<polygon>` is a path (criterion 27, changed).** The regular-polygon and star
  guess contradicted criterion 29 and D4, depended on a tolerance nobody could pin, and the
  exporter never writes `<polygon>`, so no round trip needed it.
- **2026-10-14: namespaces are matched by URI, never by prefix.** `inkscape:`, `sodipodi:`,
  `curvyo:` and Adobe namespaces are recognised through `roxmltree`'s resolved names; the
  Illustrator test of criterion 19 reads the resolved URI.
- **2026-10-14: `use` expansion counts toward the command budget.** 100,000 copies of a
  400,000-node path would be 40 billion nodes; copies are counted in path commands (criteria 47
  and 69), and `limit-reached` ends the expansion.
- **2026-10-14: the apply step is one commit.** `document-core` gets one insert API for a drawing
  (`ObjectRead`-shaped tree) so Import is one history step (criterion 3) and Open builds a new
  document in the same way. Not a trait, not generic.
- **2026-10-26: review against the code of the day; ADR 0018 written and accepted.** Changes to the
  proposal of 2026-10-16: (a) `svg-core` no longer depends on `document-core` and
  `geometry-core`; the mapping to the document moves to `ui-core`, because `text-core` (which may
  not depend on `document-core`, ADR 0011 note 2026-10-20) must share the importer's XML guard and
  path parser, and `0046` stage 2 then reads outlines without building a document. (b) `0076`
  already brought `roxmltree` =0.20.0, a prolog/DOCTYPE scan that skips decoys, a depth scan and a
  complete path data parser on the workspace `kurbo`; these move to `svg-core` instead of being
  written twice, so `svgtypes` is used for transform, colour, length, paint and number lists only.
  (c) The entity rules differ on purpose: fonts refuse any internal subset (`0076` criterion 23),
  the import accepts bounded literal entities (criterion 68); one guard, the rule as an argument.
  (d) Checklist of 2026-10-16 run: `svgtypes` 0.16.1 (2026-01-09) and `simplecss` 0.2.2
  (2025-01-06) are Apache-2.0 OR MIT, linebender-maintained, without `unsafe`; `svgtypes` uses
  `kurbo` 0.13 (one version in the lock file) and `siphasher`, `simplecss` uses `log`, both already
  locked; a scratch wasm32 build passes and adds about 89 KB (40 KB gzip). `roxmltree` reports the
  error position without its `positions` feature, so criterion 6's line and column need no
  feature change. (e) `0049` is merged, so criterion 47 (a) is not deferred. (f) Text now exists
  (`0074`, `0079`, `0080`), but criteria 13 and 49 still ignore `<text>`; importing it as a text
  node is a scope question for the PO (ADR 0018 point 8 says how it would map).
- **2026-10-14: bytes cross the Tauri boundary as the project open already does** (reuse that
  path; do not send 50 MiB as a JSON number array). The implementer states the mechanism in
  `plan.md`.

- **2026-10-26 (implementer): budget 71 measured.** Native release build, one thread: 20,000
  paths of 20 cubic nodes (17 MB) from bytes to a finished document in 2.3 s, 5,000 groups of 4
  in 2.2 s, a 120 KB file in 20 ms. Reading and mapping take 0.13 s; Loro takes about 2.0 s for
  the two million anchor registers alone (measured with the plain Loro API; writing detached
  containers saves about 10 %). As the risk below foresaw, the two large budgets are restated
  from the measurement to 3.0 s and 3.5 s (`curvyo-ui-core/tests/svg_import_budget.rs`) until
  the PO confirms; the 100 ms budget holds.
- **2026-10-26 (implementer, after the tester): budget 71 in thread CPU time.** The budget test
  moved to `curvyo-editor-wasm/tests/svg_import_budget.rs` and times `Session::open_file` by
  thread CPU time (`tests/cpu_time/`). Open of 20,000 paths: 2.32 s, 5,000 groups of 4: 2.27 s,
  122 KB: 20 ms (one test per process). Profile of Open: read and map 0.11 s, session setup and
  id minting under 1 ms, the Loro write 1.85 s with a warm heap; the remaining 0.4 s of a cold
  run is kernel time for about 300,000 page faults while the heap first grows to 1.2 GB, not work
  in our code, so nothing outside Loro can be trimmed. The restated 3.0 s and 3.5 s stay until
  the customer confirms them.
- **2026-10-27: slice 2, text and images (criteria 78 to 110, Frage 48 default A).** Checked
  against the code of the day.
  1. **Where it runs.** `curvyo-svg-core` stays font-free (ADR 0018): it reads `<text>` into a
     font-free run list (characters after criterion 79, resolved style per character including
     `font-family` list, weight and italic state, `x`/`y`/`dx`/`dy`/`rotate` lists, line breaks,
     anchor, writing mode, the `textPath` target's element id) and decodes `data:` hrefs to
     bytes. Font matching (criteria 80 to 82), shaping, Size (83), kerns (86 to 90) and the
     split into texts (92, Frage 48 A) run in `curvyo-ui-core::svg_import` over the session's
     face listing (`font_match.rs`, `ListedFace`) and `curvyo-text-core` (`FaceStyle` rules of
     `0078`). No new crate, no new dependency edge: ui-core already depends on both.
  2. **Missing font: no `format_version` bump.** Today's `text_font` requires a 64-hex
     `face_hash` (`text_codec::read_font` returns `None` otherwise, `check_font` refuses), so an
     absent hash would drop the font in a format-25 reader and need format 26. Instead the import
     writes the reserved hash `MISSING_FACE_HASH` = 64 zeros (a named constant in
     `text_model.rs`, never a real SHA-256): it matches no loaded face, so `match_face` falls
     through to family-and-style (case b) and then `Missing`, which is exactly criteria 82 and 30.
     Rejected: `Option` hash plus format 26 (every older build refuses projects with any import
     text, for no behaviour gain). Criterion 109 holds; the dependency line above stays "no change".
  3. **Face metrics: the import waits, no fallback ratio.** Kerns (86, 87) and the split (92) need
     the real advances, not only the H ratio, and a later re-size would break determinism (109)
     and make a second history step. Conversion is two-phase and pure: phase 1 matches every run
     and returns the `NeedsLoad` faces; the host loads them (desktop and browser alike, the
     existing load path) and calls phase 2, which converts with all faces loaded; `Missing` uses
     the included substitute, which is always loaded. The busy notice of slice 1 covers the wait.
  4. **Images in the one commit.** `DrawingNode` gets `Text`, `TextOnPath` and `Image` variants;
     `insert_drawing` takes the asset bytes beside the nodes and writes them through the
     `0058` asset store (content-addressed, the code behind `Document::add_images`) inside its
     single `batch`, so criterion 104's one step "Import SVG" holds. Identification and decoding
     stay in `curvyo-image-core` (by content, criterion 100); svg-core only base64/percent-decodes.
  5. **Kerns and rotation** are ADR 0021 per-character marks written in the same commit; values
     outside `0079`'s range start a new text (92 b), never clamp. **textPath** is an ADR 0022
     carrier: the carrier's node is created first in the commit (as originals before clones), the
     text references it; a `textPath` to an element not drawn as a carrier is skipped.
  6. **Criterion 99 is a partial gap, not today's Illustrator bug.** `reader.rs` (`"switch"`)
     already skips a child with a non-empty `requiredExtensions`, and Illustrator's
     `&ns_ai;` resolves by criterion 68, so the Illustrator layout imports today. Missing: the
     `systemLanguage` and `requiredFeatures` tests and skipping a bare `<foreignObject>` (taken
     today, yields nothing, so the switch is lost). The fixture of 107 must pin both layouts.

- **2026-10-27 (slice 3, Inkscape shapes, criteria 111 to 127).** No new crate, no new
  dependency, no document-model or format change (criterion 126). `curvyo-svg-core` stays a
  reader and only exposes the tool numbers: a `LeafKind::Arc` (centre, radii, optional start and
  end, `arc-type`, `open`, the drawn `d` and the same `d` with arcs split within 1e-5 mm for the
  check), `StarData` gains `arg2` (and `r2` becomes optional: a polygon ignores it), and
  `NodeState::inkscape` names a spiral, a 3D box and a box face. Every decision (which primitive,
  the transform rule of criterion 114, names) is made in `curvyo-ui-core::svg_import`. The
  0.001 mm check of criterion 111 is a private function of `curvyo-ui-core`
  (`svg_import_match.rs`, both outlines sampled at 16 points per segment, nearest point on the
  other within a bounding-box pre-filter); no core crate gains a public function. Arcs are mapped
  with `0021`'s existing `ellipse_after_linear_map` and `arc_after_linear_map`.

## Risks for the plan (not decided here)

- **Budget 71 is unproven.** A path anchor is a Loro map with five registers
  (`path_codec.rs`), so 400,000 anchors are about 2 million registers in one commit. 2.0 s
  from bytes to document may not hold. The first plan task is a benchmark of
  `create_path` over generated input; if it misses, the budget is restated from the
  measurement (the PO agrees), not hidden.
- **Silent partial import.** Without a report, a file over the limits of criterion 69 imports
  as far as it goes and says nothing. Accepted by the customer; the limits are far above real files
  (500,000 elements, 5,000,000 path commands).
- **`use` needs two passes.** A target may stand after its `use` and a clone's original must exist
  first; the importer builds the tree first, then `document-core` creates originals before clones
  in one commit. (`0049` is merged; the deferral once noted here no longer applies, 2026-10-26.)
- **Re-wound outlines change direction**, which matters for dash start and for cutting
  direction. Accepted (EVENODD A); the fixture `even_odd.svg` pins the stored directions.
- **`svgtypes` and `simplecss` drift.** Both are pinned in `Cargo.toml` with `=` like
  `i_overlay`, because the golden files depend on their error behaviour.

## Questions decided (customer, 2026-10-16)

- EVENODD A: sound, with the tightening above.
- USE-CLONE B: structurally sound, see the dependency note on ADR 0015/0016 (matrix formula,
  isometries only, overrides give a path on export). Primitive originals and the `symbol` case stay copies.
- REPORT-PLACE: no report; ADR 0018 point 4(b).
- BAG A: ADR 0018 point 4(a).
- CLIP A: ignored; no kernel use in the importer.
- PIXEL-SIZE A: detection reads resolved namespaces and comments only.
- SVG-EXPORT-LATER A: export is a later slice; the crate and ADR 0018 cover both halves so the
  crate name and boundary do not change. The export slice adds its own `plan.md` and a review of
  criterion 59 against `0049`/`0057` as built then.

## Export (slice 4, architect 2026-11-01)

Source: `export-plan.md` (accepted 2026-11-01: SVG-PAGES A with "Active page only", SVG-TEXT A,
SVG-MARKERS A, SVG-GUIDES B, SVG-OPTIONS A, one PR), the "Later: export" criteria 4, 52 to 66,
71 and 72, ADR 0018 and the code of the day. **No new crate, no new dependency, no
`format_version` change** (confirmed: the export writes no project data; the import additions
below use registers that exist since `0018`, `0046`, `0059`). Checked against the code: clone
overrides are `CloneChain::own_placement` / `own_style` (`0057` as built), so criterion 59 (i)
reads them directly; nothing in criterion 59 needs to change.

**The SVG profile is a file format we write.** `curvyo:format="1"` and the `curvyo:` attribute
list of criterion 57 (plus `curvyo:origin`, `curvyo:role`, `curvyo:font` below) are frozen once
merged; a change is `curvyo:format="2"`. The reader ignores every `curvyo:` attribute when
`curvyo:format` is missing or not `1` (Proposal). ADR 0018 gets a point 9 "Export and the
`curvyo:` profile v1" with this section's decisions 1, 2, 5 and 9 before the PR (architect).

### 1. Layers of the export (where each part lives)

| Step | Crate / module | Input → output |
|---|---|---|
| Writer | `curvyo-svg-core` `export.rs` (types), `write.rs`, `write_style.rs`, `write_text.rs`, `write_number.rs`, `base64.rs` | `&SvgExport` → `String` |
| Mapping | `curvyo-ui-core` `svg_export.rs`, `svg_export_style.rs`, `svg_export_text.rs` | document + session's laid-out objects → `SvgExport` |
| Session glue | `curvyo-editor-wasm` `session/svg_export.rs`, `wasm_svg_export.rs` | options → `String` |
| Desktop file | `curvyo-app` `svg_export.rs` (two commands), `curvyo-storage-io::write_atomic` | text → file |
| Browser file | `frontend/src/platform/host.ts` (existing download helper) | text → Blob download |

Why the mapping is in `ui-core` (rejected: `document-core`, `editor-wasm`): text outlines,
"Text as paths" (`text_convert::text_path`), textPath layout and "Strokes as paths" (the
geometry `stroke_to_path_plan` already calls) are `ui-core` code; `document-core` has no faces
and must stay without text layout (ADR 0020). `editor-wasm` would make the mapping untestable
without a session and break the symmetry with `ui-core::svg_import`. The session only supplies
its laid-out `ObjectRead` (texts with `outline` set by its `fonts`), the selection and the
active page.

### 2. Writer input `SvgExport` (svg-core, no Curvyo dependency)

Mirrors `SvgDrawing`; reuses the reader's `Rgb`, `Paint`, `Cap`, `Join`, `StarData`, `ArcData`,
`DisplayUnitName`, `PageBackground`, `TextAnchor` (one definition per SVG concept).

```rust
pub struct SvgExport {
    pub pages: Vec<ExportPage>,          // ≥ 1; [0] is the root viewport; mm, relative to `origin`
    pub origin: (f64, f64),              // global mm of pages[0]'s corner → curvyo:origin
    pub display_unit: DisplayUnitName,
    pub background: PageBackground,
    pub nodes: Vec<ExportNode>,          // top level, z-order, first at the bottom
    pub guides: Vec<ExportNode>,         // leaves of the "Guides" layer; empty: no layer
}
pub struct ExportPage { pub name: String, pub rect: kurbo::Rect }
pub struct ExportState { pub name: Option<String>, pub hidden: bool, pub locked: bool, pub opacity: f64 }
pub enum ExportNode {
    Group { state: ExportState, layer: bool, children: Vec<ExportNode> },
    Leaf  { state: ExportState, leaf: ExportLeaf },       // takes a leaf number
    Text  { state: ExportState, text: Box<ExportText> },  // takes a leaf number (textPath, use)
    Image { state: ExportState, image: Box<ExportImage> },
}
pub struct ExportLeaf { pub kind: ExportKind, pub style: ExportStyle, pub shadow: Shadow }
pub enum ExportKind {
    Path    { path: BezPath, node_types: String, id_kind: PathIdKind }, // path|compound|polygon|star|rectangle|sketch → id prefix
    Rect    { rect: Rect, rx: f64, ry: f64, rotation_deg: f64 },
    Ellipse { center: Point, rx: f64, ry: f64, rotation_deg: f64 },    // <circle> when rx == ry
    Star    { star: StarData, path: BezPath, node_types: String },
    Arc     { arc: ArcData, path: BezPath, node_types: String },
    Use     { original: usize, matrix: Affine },                       // leaf number of the target
}
pub struct ExportStyle { pub fill: Option<Paint>, pub stroke: Option<ExportStroke> }
pub struct ExportStroke { pub paint: Paint, pub width_mm: f64, pub dash_mm: Vec<f64>,
    pub cap: Cap, pub join: Join, pub start: Option<MarkerShape>, pub mid: Option<MarkerShape>,
    pub end: Option<MarkerShape> }                                     // only markers SVG can draw
pub struct Shadow { pub kind: Option<ShadowKind>, pub frame: Option<ShadowFrame>,
    pub radii: Option<[f64; 4]>, pub dash: Option<Vec<f64>>, pub stroke_width: Option<f64>,
    pub markers: Option<ShadowMarkers>, pub outlines: Option<Vec<bool>> }
pub struct ExportImage { pub mime: &'static str, pub bytes: Arc<[u8]>, pub placement: Affine }
pub fn write_svg(drawing: &SvgExport) -> String;
pub fn is_isometry(m: Affine) -> bool;   // moved out of uses.rs, see 4
```

`ExportText` carries the transform, family list, weight, italic, size (mm), anchor, fill/stroke,
`face_hash`, lines (start `x`/`y`, characters, per-character `dx` and `rotate` from ADR 0021
marks) and `on_path: Option<{ carrier: usize, start_offset_mm: f64 }>`; field for field what
`SvgText` reads, so slice 2's reader is the round-trip oracle (Proposal; the implementer may
rename fields, not add concepts). **`Shadow` is shared with the reader:** `SvgLeaf` gains
`shadow: Shadow` (raw typed values; svg-core does not judge them), and `ui-core::svg_import`
applies criterion 57's agreement tolerances. All numbers in `SvgExport` are finite by
construction of the mapping; the writer writes a non-finite number as `0` behind
`debug_assert!` (no panic, no `Result`, Proposal).

### 3. Numbers, escaping, ids, order (criteria 53, 54, 57, 60, 72)

- `write_number.rs`: `fn coord(v, out)` = `{:.6}`, trailing zeros and a bare `.` trimmed, `-0` →
  `0` (coordinates, lengths, `rect`/`ellipse` numbers, page rects, dash in mm, stroke width).
  `fn exact(v, out)` = Rust `{}` (shortest round trip, never an exponent for `f64`), `-0` → `0`
  (all `curvyo:` numbers, opacities, angles, ratios, the `use` matrix, text and image transforms).
  Both are pure Rust formatting: no locale, no libc, same bytes on every target (ADR 0018 §6).
  `{:.6}` re-reads to the same text, so criterion 63 holds after the first trip.
- `escape(text, out)`: `& < > " '` as entities; names (criterion 53) also drop C0/C1 controls
  and bidi controls U+200E, U+200F, U+202A to U+202E, U+2066 to U+2069; text content drops only
  characters XML 1.0 forbids.
- Ids are assigned by the writer in one pre-pass in document order (guides layer last), counters
  per prefix `layer g path compound rect ellipse circle polygon star arc text image use marker`
  (criterion 53 plus `arc`, `text`, `image`, `marker`); `Use`/textPath targets are leaf numbers,
  resolved to ids in the second pass, so a `use` may precede its target. No `HashMap` iteration
  anywhere in writer or mapping; marker defs in order of first use (a `Vec`).
- Element order: prolog, `<svg>` (namespaces `svg`, `xlink`, `inkscape`, `sodipodi`, `curvyo`,
  always declared), `sodipodi:namedview` with `inkscape:page` children, `<defs>` only when a
  marker is used, content, guides layer. Attribute order is fixed per element in `write*.rs`.

### 4. Model mapping (ui-core)

```rust
pub struct SvgExportOptions { pub text_as_paths: bool, pub strokes_as_paths: bool,
                              pub selection_only: bool, pub active_page_only: bool }
pub struct ExportScope<'a> { pub selection: &'a [NodeId], pub active_page: usize }
pub struct SvgExportOutcome { pub svg: SvgExport, pub objects: usize, pub texts_kept_live: usize }
pub fn svg_export(document: &Document, laid: &ObjectRead, scope: &ExportScope<'_>,
                  options: SvgExportOptions) -> SvgExportOutcome;
```

The four flags are the customer's (SVG-OPTIONS A), not speculative. The walk is
`document.read_nodes()` for the tree and registers; each leaf's snapshot is taken from `laid`
by id (texts with outlines). Per kind:
- **Paths, primitives, arcs, curved sides, sketches:** criteria 54, 55; arcs as
  `sodipodi:type="arc"` (slice 3 reads them back); curved-side polygon/star and rectangles with
  unequal radii as `Path` + `curvyo:kind`/`curvyo:frame`; a sketch as one `Path` of
  `SketchSnapshot::resolved`, constraints dropped (Proposal; imports as a path).
- **Clones (criterion 59, the import's test):** `svg_core::is_isometry` (moved from `uses.rs`,
  `ORTHOGONAL_TOLERANCE`, `MAX_COORDINATE_MM`) is the one test in both directions. `Use` when
  `!own_placement && !own_style && is_isometry(chain.matrix)`, the origin's own `visible`
  register is on, the origin is written (filters below) and the clone is in `laid.objects`
  (orphans are not). Otherwise `Path` of `resolved` with its effective style.
- **Origin shift.** Every absolute placement is pre-multiplied by `T = translate(-origin)`; a
  clone's relative matrix is written as `T · M · T⁻¹` (same linear part, so the isometry test is
  unaffected). Open with a valid `curvyo:origin` applies the inverse, so global coordinates and
  pages come back exactly; Import SVG ignores it (it centres on the view, criterion 3).
- **Text (SVG-TEXT A):** live `<text>` with `transform`, `font-family` (stored family),
  `font-weight`, `font-style`, `font-size` (mm), one `<tspan x y>` per line, `dx`/`rotate` only
  where ADR 0021 marks exist, plus `curvyo:font="<face_hash>"` so slice 2's `match_face` case (a)
  finds the same face; `MISSING_FACE_HASH` is never written. Text on path: `<textPath
  xlink:href="#carrier" startOffset>`; the carrier is always written (even outside a filter) and,
  when it is a primitive, as a `Path` with shadow data, because SVG 1.1 and Inkscape take a path
  only. "Text as paths" and single-line fonts (always): `text_convert::text_path`, one compound
  `Path` per text (single-line: open outlines, stroked, no fill). A text drawn with a substitute
  or missing face stays live and is counted in `texts_kept_live` for the notice (Proposal). A
  text clone (`0022`) is written as an independent text with its resolved values; the link does
  not survive the round trip (Proposal, listed as an exception in criterion 62).
- **Images:** `<image x="0" y="0" width="1" height="1" preserveAspectRatio="none"
  transform="matrix(T·placement)" xlink:href="data:<mime>;base64,…">` with the stored bytes
  (`Document::asset`), mime from `curvyo-image-core`'s content identification. Base64 encoder
  hand-written in svg-core (30 lines; rejected: the `base64` crate for one function).
- **Markers (SVG-MARKERS A):** one `<marker>` per (shape, start|mid|end, stroke colour,
  opacity), `markerUnits="strokeWidth"`, `orient="auto"`, start shapes drawn reversed (no SVG 2
  `auto-start-reverse`), geometry from the renderer's marker constants. `marker-start`/`-end`
  only on paths whose single open outline the model marks; `marker-mid` only for At nodes.
  Spaced middle markers and marker cases SVG cannot express are carried by `curvyo:markers`
  alone (default; question to the PO: other programs then draw no spaced markers). The importer
  takes `curvyo:markers` only when every standard end it names still has its `marker-*`
  property (criterion 57's rule).
- **Guides (SVG-GUIDES B):** every node with `NodeRead::guide` leaves its place and goes, in
  document order and flat, into `guides`; a group left empty is not written. The writer emits
  `<g id="layerN" inkscape:groupmode="layer" inkscape:label="Guides" curvyo:role="guides"
  style="display:none" sodipodi:insensitive="true">` on top. **Import recognition:** a top-level
  layer with `curvyo:role="guides"` (namespace by URI); its hidden and locked flags are ignored,
  its leaves come in as top-level objects at its z position with `guide = true` (paths and
  primitives; ADR 0019 §2). The label is not used, so a renamed or translated layer still works.
  Round trip compares guides as an ordered set of guide leaves, not by tree position (Proposal).
- **Strokes as paths:** a stroked leaf becomes its stroke outline (filled with the stroke paint)
  plus, when filled, its fill path, as a group with the leaf's name (Proposal); `Use` is kept.
- **Filters.** Selection only: the selected nodes, their descendants and their ancestors as
  containers. Active page only: leaves whose resolved bounds overlap the page
  (`Page::overlaps`, the membership rule of ADR 0024), written whole, not clipped; that page is
  `pages[0]` and the only page. Both together: intersection. A filtered-out clone origin makes
  the clone a `Path`.
- **Pages (SVG-PAGES A):** root `width`/`height`/`viewBox="0 0 w h"` = `pages[0]`; with two or
  more pages each is `<inkscape:page id="pageN" x y width height inkscape:label>` in user units
  (mm), positions relative to `pages[0]`. The reader gains `SvgDrawing::pages: Vec<SvgPage>` (all
  `inkscape:page`, not only the first) and `origin`; `DrawingPage` gains the page list so
  `Document::from_drawing` creates the pages on Open (one doc-core API change, no format change).

### 5. Desktop and browser (criterion 4: whole or nothing)

`wasm_svg_export.rs`: `export_svg(&self, text_as_paths: bool, strokes_as_paths: bool,
selection_only: bool, active_page_only: bool) -> String` (scalars and a string, ADR 0001 §5),
plus `export_svg_notice_counts()` for `objects` and `texts_kept_live`. The SVG is built whole in
memory before anything is written. **Desktop:** `pick_svg_export_path(suggested_name) ->
Option<String>` (native save dialog, filter `.svg`; the path stays in `AppState`, the page gets
the file name), then the busy state and `export_svg`, then `write_svg_export(text: String) ->
Result<String, String>` = `curvyo_storage_io::write_atomic` (temp file + rename, so a failure
leaves the old file or nothing) to the stored path. A JSON string, not a number array as
`save_project_bytes` uses (25 MB). **Browser:** `export_svg`, then one `Blob` and the existing
download helper. Options are remembered per user in the frontend (`localStorage`, try/catch, the
UX notes' place); they never enter the document.

### 6. Tests

- **Writer goldens (svg-core):** `curvyo-svg-core/tests/write.rs` builds `SvgExport` by hand:
  criterion 52's skeleton byte for byte, number edge cases (`-0`, 1e-7, 0.1+0.2, ±1e7), escaping,
  ids, markers, pages; goldens in `curvyo-svg-core/tests/fixtures/export/`.
- **Exported goldens:** `name.exported.svg` beside each import fixture under
  `curvyo-svg-core/tests/fixtures/svg_real/` and `inkscape_text/`, written by
  `curvyo-editor-wasm/tests/svg_export_golden.rs` (Open through the session, so text faces are
  real), which also checks criteria 63 and 64 (b), (c). `curvyo_all.svg` from the generated
  document of criterion 62 in the same test. `CURVYO_UPDATE_GOLDEN=1` regenerates (project
  pattern).
- **Round-trip property test (criteria 62, 63):** `curvyo-ui-core/tests/svg_round_trip.rs`,
  `proptest` (already a workspace dev-dependency) with 500 cases: generated `Vec<DrawingNode>`
  and pages → `Document::from_drawing` → `svg_export` → `write_svg` → `read_svg` → `svg_import`
  → `from_drawing`, compared on `read_nodes()` with ids dropped, geometry within 1e-6 mm, the
  rest exact, then the second export byte-compared. Without text (no faces in `ui-core`); text
  round trips are fixtures in the golden test. Criterion 72's locale run: the same test under
  `LC_ALL=de_DE.UTF-8` in one case (Rust formatting ignores it; the test proves it).
- **Criterion 61:** strip the three namespaces from each exported golden and compare outlines
  with an independent flattener in the test (expands `use`, 0.01 mm).
- **Budget 71 (export part):** `curvyo-editor-wasm/tests/svg_export_budget.rs`, the 20,000-path
  document of `svg_import_budget.rs`, `Session::export_svg` timed by thread CPU
  (`tests/cpu_time/`), asserts < 1.5 s and < 25 MB only when `!cfg!(debug_assertions)`, prints
  otherwise; one line added to CI's `performance-budgets` job. This replaces the criterion's
  `#[ignore]` benchmark (project pattern; PO amends the wording).
- Hand check 65 with the current Inkscape by the tester, version in the PR.

### Risks

- **Reader work is part of the export.** Shadow data, all `inkscape:page`s, `curvyo:origin`, the
  guides layer and `curvyo:font` are new import code; without them criteria 62 and 63 cannot
  pass. Plan them first, test-first, against the writer goldens.
- **Live text fidelity in other programs** depends on their shaping; positions are per line, so
  Inkscape may set a line a little wider or narrower. Accepted by SVG-TEXT A.
- **Inkscape page coordinates.** `first_inkscape_page` scales page attributes by the user-unit
  factor; with `viewBox` in mm the factor is 1. The `inkscape_oracle.py` run confirms that
  Inkscape 1.2+ places the pages where we wrote them before the goldens are blessed.

### 2026-11-01 (architect review of the build): accepted deviations

The build follows the customer-accepted specification where it is more specific than this
section; review in `review-export.md`. The frozen profile v1 is ADR 0018 point 9's list, which
**replaces** the names above: `curvyo:guides="1"` (not `curvyo:role`), `curvyo:face="family|style|hash"`
plus `curvyo:size` (not `curvyo:font`), and **no `curvyo:origin`** (the export origin is the first
page's, the pages' union's or the active page's corner; Open puts it at 0,0 and keeps relative page
positions). Also accepted: `xmlns:xlink` only when needed; exported goldens in
`curvyo-svg-core/tests/fixtures/export/real/`; writer references by key instead of leaf number;
extra `curvyo:frame` on turned `<rect>`/`<ellipse>`, arcs and Inkscape stars for byte stability;
text-on-path carriers keep their element kind (criterion 134); module names `write_leaf.rs`,
`svg_export_parts.rs`, `svg_export_notices.rs`.

### 2026-11-01 (implementer, review and acceptance fixes; Proposal)

- `rotate` and `line-height` are written as the shortest round-trip text, each chosen as the
  value next to the model's that the import gives back unchanged (degrees through the canvas's
  radians; the factor through the 6-decimal `font-size`); arc angles likewise settle through the
  document's degree register. A uniform radius that `rx` cannot say exactly adds `curvyo:frame`
  and `curvyo:radii` to the `<rect>`; the import keeps those radii only when all four agree with
  `rx` within 0.001 mm.
- The text line anchor is SVG's: the chunk's whole advance (trailing letter spacing included)
  and the import's upright lead; a turned character's `dx` is measured against the import's own
  pens (a turned character shaped alone). Three import fixes this needed, in `svg_text_align`:
  the alignment shift moves the origin by minus the residual (the placement is
  `translate(origin)`; a centred text with letter spacing was off by half of it); a turned first
  glyph is compared at its turned origin; a column's origin stays on the file's pen across the
  column. A text matrix whose scale is 1 within 1e-12 counts as exactly 1.
- New modules: `svg_export_shapes.rs` (primitives, one function per kind), `svg_export_sketch.rs`
  (one object per drawn sketch entity, a circle or arc as the primitive its import gives),
  `svg_export_pens.rs`, `svg_import_pages.rs`; `limits::MAX_PAGES = 1000`.
