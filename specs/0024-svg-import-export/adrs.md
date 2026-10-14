# ADRs for "SVG import and export"

Architect review, 2026-10-14, of the Draft specification (77 criteria) and Fragen 1 to 6. The
slice adds the first importer and exporter. **One new ADR (0018, proposed, not `needs-customer`
for the crate and dependencies; the one decision that is, Frage 4, is recorded in it as a
separate section), one new crate (`curvyo-svg-core`), three new dependencies, no
`format_version` bump** (criterion 77 holds: no register, no node kind, no schema change).

## Depends on

- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  - §2, §4: mm is the document unit, Y grows down like SVG; px, pt, pc, in become mm once, at the
    import boundary (criteria 15 to 20). No unit conversion anywhere else.
  - §3: units are newtypes, every comparison has a tolerance; criteria 25, 27, 50 and 57 use
    the named tolerances (1e-6 mm, `TOUCH_DISTANCE_MM` 0.001).
  - §5: a style is resolved per leaf (so the importer resolves the CSS cascade itself; there is
    no inheritance in the model, ADR 0015 notwithstanding for ancestor-resolved state).
  - §10: SVG strategy: own subset, named listed loss, golden files. Its **passthrough bag is not
    built here** (Frage 4, ADR 0018 §4). Gradient amendment of 2026-10-08 is what criterion 38
    implements.
  - §11 with [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: no
    `format_version` change (criterion 77). The importer writes through the existing document
    API, so the project file is whatever the model writes (current format 13).
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §5: the browser boundary
  carries strings and scalars. The importer is called from `curvyo-editor-wasm` with the file
  bytes and returns the report as one JSON text (like the History rows); the 50 MiB limit is
  checked before the call.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §2: its table gives
  `curvyo-document-core` "SVG import/export" in one sentence. That sentence is superseded by
  ADR 0018 (reason below).
- [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md) and
  [`0054` adrs.md](../0054-combine-open-paths/adrs.md): `touching_outlines`, `outline_nesting`
  and `TOUCH_DISTANCE_MM` of `curvyo-geometry-core` decide the even-odd re-winding (criteria
  50 and 51); `kurbo` (already a workspace dependency) converts arcs.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md),
  [ADR 0016](../../docs/adr/0016-clone-overrides.md), `0049` and `0057` adrs: a clone is read as
  its resolved outline on export (`clone-as-path`); import creates no clone (Frage 2 A).
- `0050` adrs (layers are top-level groups with a marker) and `0023` (depth 32): criteria 44 and 45.
- `0046` stage 2 calls the same importer (criterion 73): it depends on `curvyo-svg-core`, not
  on a second parser.

## New ADR needed: 0018 "SVG import and export crate, parser and limits"

Needed because `CLAUDE.md` §5 says no new crate without an ADR and ADR 0011 is accepted, so its
row cannot be edited. It records the following, which are the architect's proposals here.

1. **Crate `curvyo-svg-core`** (pure, `forbid(unsafe_code)`, builds for wasm32, no I/O; bytes in,
   `SvgImport { page, display unit, background, tree, report }` out, and
   `export(&ObjectRead snapshot, page) -> (String, Report)`). Depends on `curvyo-document-core`
   and `curvyo-geometry-core`. *Why not a module in `curvyo-document-core` (ADR 0011's row):*
   the re-winding needs `touching_outlines` and `outline_nesting`, which live in
   `geometry-core`, and `geometry-core` depends on `document-core`, so the module cannot exist
   there without moving the geometry down or duplicating it; and every crate (render, ui,
   wasm) would compile an XML parser and CSS reader for no reason. *Why not in `ui-core`:* it
   is not editor state, and `0046` and later machine specs need it without the editor.
   Applying a finished import to the open document in one commit and one history step is a
   `document-core` API (`insert_drawing`, a tree of the existing snapshot types), so the crate
   needs no Loro knowledge.
2. **Dependencies** (all permissive, in the `cargo deny` allow list; to be confirmed by the
   ADR's spike on `spike/svg-parser`, checklist below): `roxmltree` (MIT or Apache-2.0, no
   dependencies, `forbid(unsafe_code)`, takes `&str`, resolves namespaces by URI, gives line and
   column for the report), `svgtypes` (MIT or Apache-2.0: path data, transform, colour, length
   and paint grammar; stops at the first path error, as criterion 23 needs), `simplecss`
   (MIT or Apache-2.0: the `<style>` subset of criterion 36; selectors with combinators are
   parsed, then skipped by us). *Rejected:* `usvg` (it normalises away what the report needs:
   line numbers, `sodipodi:*`, `inkscape:label`, `use` as clone, per-element counts; pulls font
   and image crates; resolves text), `quick-xml` (streaming, no tree: we would rebuild a DOM,
   find entity declarations ourselves and compute line numbers from byte offsets),
   `xml-rs` (slow, no wasm story needed), and writing all grammars ourselves (about 800 lines
   of already solved, fuzz-tested code; reconsider only if the checklist fails).
   *Checklist before 0018 is Accepted:* licence in `cargo deny`; `forbid(unsafe_code)` or no
   `unsafe` in the used code (`CLAUDE.md` §5); wasm32 build; one `kurbo` version in the lock
   file (if `svgtypes` pins another, use its `PathParser` without arc simplification and convert
   arcs with the workspace `kurbo`); `roxmltree` entity and depth limits behave as in point 3
   (these were written from memory of the crates, not run).
3. **Limits and DOCTYPE** (criteria 6, 67 to 69). Checks that run before parsing: size 50 MiB,
   UTF-8, gzip magic, and a byte scan of the DOCTYPE that applies criterion 68 (literal
   internal entities only; at most 64, 256 bytes each, 100,000 references). Only then
   `roxmltree` with `allow_dtd` on and a `nodes_limit` of 500,000. Element depth 256 is
   enforced by our own walk; every tree walk is iterative or bounded by that depth.
4. **Frage 4, passthrough bag: `needs-customer` (document model).** ADR 0002 §10 is
   customer-signed. Answer A (default) defers the bag: the ADR gets an amendment line like
   the gradient one of 2026-10-08, and `foreign-data` is the visible cost. Answer B would
   need its own ADR; one more risk for B that the spec does not name: attributes kept
   verbatim and written back are untrusted input in the export (`onclick`, `href`, script),
   so a bag needs an allowlist of namespaces, and merge semantics in the CRDT (ADR 0009 §3).
5. **Floating point** (criteria 54, 57, 62, 63, 72). All text is produced with Rust's own
   `f64` formatting (`{:.6}` for lengths, `{}` shortest round trip for `curvyo:` data, opacities,
   ratios and radians); both are exact and independent of locale. Rounding to 6 decimals is
   idempotent, which is why export(import(export(d))) is byte-identical. The
   code uses `+ - * /` and `sqrt` (exact in IEEE 754; Rust never fuses multiply-add) and
   `sin`, `cos`, `atan2`, `tan` (library-defined; native and WASM may differ in the last bit).
   Hence the contract of criterion 72: bytes identical for the same input on the same
   target; across targets documents agree within 1e-9 mm and goldens compare with that
   tolerance; exported goldens are compared byte for byte on the Linux gate and nightly on
   Windows and macOS. If the nightly job ever shows a one-digit difference, the fix is the
   `libm` crate for those four functions in this crate, not a looser test. Maps are
   `BTreeMap` or `Vec` in order; nothing iterates a `HashMap`.

## Feature-local decisions (dated notes)

- **2026-10-14: no new `format_version`, no new register.** Imported content is created with the
  existing constructors (`create_path`, `create_rect`, `create_ellipse`, `create_polygon`,
  `create_star`, groups, layers, styles). Shadow data lives in the SVG, not in the project.
  Report and notes are session state in the editor facade, never in the Loro document.
- **2026-10-14: even-odd (Frage 1, A) agrees with `0054`'s `adrs.md` "Flagged" and its
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
- **2026-10-14: bytes cross the Tauri boundary as the project open already does** (reuse that
  path; do not send 50 MiB as a JSON number array). The implementer states the mechanism in
  `plan.md`.

## Risks for the plan (not decided here)

- **Budget 71 is unproven.** A path anchor is a Loro map with five registers
  (`path_codec.rs`), so 400,000 anchors are about 2 million registers in one commit. 2.0 s
  from bytes to document may not hold. The first plan task is a benchmark of
  `create_path` over generated input; if it misses, the budget is restated from the
  measurement (the PO agrees), not hidden.
- **Report counts versus noise.** Figma and Illustrator wrap whole drawings in a clip
  rectangle, so `clip-path` fires on nearly every such file. Proposal for Frage 5 A: a clip
  whose region contains the bounds of everything it clips goes to the silent list of criterion
  13 (it changes nothing the maker sees or cuts). Default if unanswered: take it.
- **Re-wound outlines change direction**, which matters for dash start and for cutting
  direction. Accepted by Frage 1 A; the fixture `even_odd.svg` pins the stored directions.
- **`svgtypes` and `simplecss` drift.** Both are pinned in `Cargo.toml` with `=` like
  `i_overlay`, because the golden files depend on their error behaviour.

## Fragen reviewed

- Frage 1 (EVENODD): sound, with the tightening above. Default A stays.
- Frage 2 (USE-CLONE): sound. Default A. B is also fine structurally (original's id is
  exported) but is a second feature.
- Frage 3 (REPORT-PLACE): UI only, no structural issue. Default A.
- Frage 4 (BAG): `needs-customer`, see ADR 0018 point 4.
- Frage 5 (CLIP): sound. A, with the silent-list proposal. B needs the kernel, which the crate has.
- Frage 6 (PIXEL-SIZE): sound. A. Detection reads resolved namespaces and comments only.
