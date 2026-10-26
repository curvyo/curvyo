# ADRs for "SVG import" (export later)

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
    API, so the project file is whatever the model writes (current format 24, 2026-10-26).
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
