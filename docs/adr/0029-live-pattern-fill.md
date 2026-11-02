# ADR 0029: Live pattern fill

**Status:** Accepted (customer, 2026-11-02: PAT-STORE A; the generators as specified, PAT-NEW A). Document model, file format. The behaviour was chosen by
the customer (PAT-LIVE B, PAT-SET, PAT-PLACE, 2026-11-01/02); this ADR is how it is stored and
computed. Option A decided (Frage PAT-STORE in `specs/0071-pattern-fill/adrs.md`). Asked by
`specs/0071-pattern-fill`. Extends the style schema of `specs/0007-stroke-and-fill-styling` and
`specs/0017-style-panel-rework` (decision 1) with a second fill paint; no accepted ADR text changes.

**In plain words (for the customer).** A shape with a pattern stores only "which pattern, its
settings, the margin, Lines or Shapes, and a seed". The hatch lines, dots or leaves themselves are
never saved; Curvyo computes them from the shape whenever it draws or exports, so they always fit
the current shape. Computing is remembered per shape, so moving or rotating a patterned shape
does not recompute. Convert to paths turns the computed pieces into ordinary paths. Files with
patterns need the new version of Curvyo.

## Context

PAT-LIVE B makes the pattern a fill kind that is regenerated when the shape or its parameters
change, and every output carries real, clipped vector geometry. What is hard to change later: how
a pattern is written into the file, and that the same file gives the same geometry on every
machine (criterion 17), because the geometry is never stored and is recomputed on each peer.

## Options

| Option | Why it won or lost |
|---|---|
| **A. Store the intent (generator, typed parameters, margin, mode, seed) as a style register; derive the pieces on read, memoised outside the document (chosen)** | Same pattern as dimensions (ADR 0027): edits and merges touch one register, undo is ordinary register restore, the pieces cannot disagree with the outline, the file stays small. Cost: generation must be deterministic across machines (§5). |
| B. Store the generated pieces as hidden child paths, rewritten on every edit of the shape | Every node drag rewrites up to 500,000 pieces in the log (write amplification, history size, merge conflicts between pieces and outline); every path tool would have to skip them. Lost. |
| C. A new node kind "pattern" referencing the shape (a fifth reference, like a dimension) | The spec makes it a fill kind of the shape, not an object of its own; a reference adds copy remap, dangling-reference rules and a Layers row for nothing. Lost. |
| D. SVG `<pattern>` tile as the stored and exported form | Not clipped geometry, not machine-runnable, excluded by the spec (criterion 26, out of scope). Lost. |

Where the code lives:

| Option | Why it won or lost |
|---|---|
| **A. Model and schema in `curvyo-document-core`, generation and clipping in a `pattern` module group of `curvyo-geometry-core` (chosen)** | The model is part of the format, so it belongs with the style schema. Generation needs the kernel's offset (margin), boolean intersection and string clip (`i_overlay`, already used by `weld.rs`); geometry-core already builds for wasm32 and has the golden and CPU-time harness. No new crate (`CLAUDE.md` §5). |
| B. A new crate `curvyo-pattern-core` | One consumer, the same dependencies as geometry-core, and the extension seam of ADR 0005 is a WIT interface, not a crate boundary. Would earn its place only if generators grew a dependency geometry-core must not carry. Lost. |
| C. A `PatternGenerator` trait with ten implementations | Allowed by §5 (ten implementations), but an enum and one `match` is shorter, exhaustively checked, and the parameter schema the panel needs is data either way. ADR 0005 §9: no capability trait before the plugin host. Lost. |

## Decision

1. **Storage: two new style keys** in an object's meta map (the schema of `style_codec.rs`), both
   in `ALL_STYLE_KEYS`, so copy, Object to path, clone `own_style` (ADR 0016) and Unlink carry them
   like every other style key:
   - `fill_paint`: `"colour"` | `"pattern"`, absent = `"colour"`. The fill paints a pattern iff
     `fill_enabled` and `fill_paint = "pattern"`. The name avoids the legacy `fill_kind` key that
     `legacy_fill.rs` still reads past; writing either key runs `legacy_fill::forget` as today.
   - `fill_pattern`: one atomic value (ADR 0009 §2, last writer wins on the whole pattern), a map
     `{ generator: string id, margin: mm, mode: "lines" | "shapes", seed: u32, params: { name:
     number } }`. Kept while the paint is Colour or the fill is off, like the colour is kept, so
     switching back restores it. Only the current generator's parameters are stored.
2. **Typed model.** `PatternFill { generator: PatternGenerator, margin: Length, mode: PatternMode,
   seed: PatternSeed }` in `style_model.rs`; `PatternGenerator` is an enum with one variant per
   generator holding newtypes (`Length`, `Angle`, a count, a fraction in [0, 1]), never bare `f64`.
   The parameter schema (wire name, kind, frozen default, minimum) is a static table per generator
   in document-core; the codec and the panel both read it, and a test round-trips every variant
   through it. An absent parameter reads as its frozen default. Damage (file refused): unknown
   generator id or `fill_paint` value, a parameter of the wrong type, non-finite or below its
   minimum, a seed that is not an integer in u32.
3. **Generation is one pure function** in `curvyo-geometry-core::pattern`:
   `pattern_pieces(region: &[Outline], frame: PatternFrame, fill: &PatternFill, tolerance,
   limits) -> Result<PatternPieces, PatternError>`. Steps: inset the region by the margin with the
   kernel offset (round join, so reflex corners keep the distance; holes grow); generate the
   lattice or scatter over the inset region's bounding box in the frame; keep pieces wholly
   inside unchanged (exact arcs and circles); clip open pieces with the string clip and closed
   pieces with boolean intersection at the kernel tolerance. Open regions give no pieces.
4. **Frame (Proposal).** The pattern's origin is the centre of the outline's bounding box in the
   object's frame, its base angle the object's `rotation` register; the generator's own angle adds
   to it. Moving or rotating the shape moves and rotates the pattern with it; scaling the shape
   does not scale spacings, which stay in mm.
5. **Determinism.** Same document, same pieces, byte-equal in the SVG export on every machine and
   in native and wasm. Generators use only `+ - * / sqrt` and `libm` (pure Rust, MIT, already in
   the lock file; pinned as a direct dependency) for trigonometry, never `f64::sin` (ADR 0023 §4:
   platform `libm` differs). Randomness is an in-crate SplitMix64 seeded by `PatternSeed`; its
   algorithm is part of the format and frozen by goldens. A fresh seed is derived from the node id
   (no clock, no RNG source in core); copies keep the stored seed.
6. **Derived on read, memoised outside the document.** The session in `curvyo-editor-wasm` keeps
   one cache entry per node: key = the frame-local outline, `PatternFill` and tolerance, compared
   in full (no hash-only hits); value = the frame-local pieces. A move or rotation is a cache hit
   and only maps the result. Entries of nodes absent from the current read are dropped. Outputs
   (SVG export in `curvyo-ui-core`, Convert to paths) call `pattern_pieces` directly; the cache
   only memoises, it never changes the result.
7. **Canvas.** `curvyo-render-core` does not change and gets no geometry-core dependency (ADR
   0011). The session draws a patterned leaf as the leaf with its fill off plus one synthetic,
   non-pickable path snapshot of the pieces directly beneath its stroke: Lines with the shape's
   stroke (colour, width, opacity, dash, join, cap; no markers), Shapes with the fill colour and
   opacity. Interior hit-testing treats a pattern fill as painting (`Fill::paints`).
8. **Output.** SVG export writes the region path with `fill="none"` and its stroke, then a `<g>`
   with one `<path>` per piece in the paint of §7: no `<pattern>`, no generator reference. Every
   later output (PNG, laser and plotter jobs) reads the same pieces from `pattern_pieces`; there
   is no second expansion.
9. **Convert to paths.** One commit: a group placed directly above the shape holding one path
   object per piece with the paint of §7, and the shape's `fill_enabled` set to false. Geometry is
   the exact `pattern_pieces` result, so it equals the export (criterion 25).
10. **Limits.** `pattern_pieces` estimates the piece count from the lattice before generating and
    refuses above the hard limit without work (`PatternError::TooMany { estimate }`); above the
    soft limit it generates and reports the count. Both numbers are constants in the module, first
    set to the spec's 50,000 and 500,000, and moved to the measured values by the CPU-time tests
    (thread CPU, `rustix`, as in `stroke_to_path_golden.rs`). Generation is synchronous; a worker
    is added only if the measurement says a frame is lost.
11. **Undo.** A parameter drag is a style preview (`style_edit.rs`), no write; release, a typed
    value, a paint or generator switch and New seed are each one commit. Escape drops the preview.
12. **Format.** `CURRENT_FORMAT_VERSION` 31 → 32: a version-31 reader would paint a patterned
    shape as a solid colour, so it must refuse the file. Migration from 31: none.

## Consequences

- Patterns are core-only. ADR 0005 §5 runs a plugin only on the invoking peer and does not require
  determinism, so a live pattern cannot come from an extension under that ADR: an extension
  generator would need its result stored or a determinism contract. That is a new ADR when the
  first extension generator is asked for; the stored shape (string id plus named numbers) does not
  block it.
- Goldens: `pattern_v32.curvyo` (every generator, a compound region with holes and margin, both
  modes); per-generator piece goldens in `curvyo-geometry-core/tests/fixtures/`, compared in the
  native and the wasm golden runs; SVG export goldens of the same document.
- geometry-core grows by one module group (one generator per file, each under 500 lines).
- Every new output must call `pattern_pieces`; tracked in `docs/technical-debt.md` until PNG export
  and machine jobs exist.
