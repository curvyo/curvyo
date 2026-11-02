# ADRs for "Pattern fill"

Date: 2026-11-02. Storage and computation of the live pattern are a document-model and
file-format decision: **[ADR 0029](../../docs/adr/0029-live-pattern-fill.md), Accepted — PAT-STORE A decided by the
customer, 2026-11-02**. The customer chose the
behaviour (PAT-LIVE B, PAT-PLACE, PAT-SET, PAT-SLICE B). No new crate, no trait, no generic;
one direct dependency made explicit (`libm`, already in the lock file). Everything under
"Feature-local decisions" is **Proposal**, reversible inside the story.

## Depends on

- [ADR 0029](../../docs/adr/0029-live-pattern-fill.md): keys `fill_paint` and `fill_pattern`, the
  typed model and schema, `pattern_pieces`, frame, determinism, cache, canvas, SVG output,
  Convert, limits, undo, format 32.
- [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md) §1, §3, §4: the
  margin is the kernel offset, clipping is the `i_overlay` intersection and string clip at an
  explicit `Tolerance`. `0067-clipping` is not needed: the kernel already clips.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §3, §9: newtypes for
  every parameter; one interaction, one commit.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §2: `fill_pattern` is one atomic
  register; [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one step per gesture.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §3 and
  [ADR 0016](../../docs/adr/0016-clone-overrides.md): a clone reads the pattern keys with the rest
  of its style and generates from its own resolved outline, so criterion 21 is testable now
  (linked clones exist, `0049`).
- [ADR 0005](../../docs/adr/0005-extension-and-plugin-model.md) §4, §8, §9: generators are pure
  "paths in, paths out" functions with a declared parameter schema; no plugin trait. A live
  extension generator needs a new ADR (ADR 0029, Consequences).
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md): `render-core` stays without
  geometry-core; generation in geometry-core, expansion for the canvas in `editor-wasm`, export
  in `ui-core`.
- [ADR 0018](../../docs/adr/0018-svg-import-crate-parser-and-limits.md) and `0024-svg-import-export`:
  the export writes pieces as paths; import knows nothing of patterns.

## Feature-local decisions (2026-11-02)

1. **Where it lives.**
   - `curvyo-document-core`: `pattern_model.rs` (`PatternFill`, `PatternGenerator`,
     `PatternMode`, `PatternSeed`), `pattern_schema.rs` (wire names, kinds, defaults, minimums),
     `pattern_codec.rs` (the two keys, damage rules); `StyleEdit` gains the pattern edits.
   - `curvyo-geometry-core/src/pattern/`: `mod.rs` (`pattern_pieces`, frame, limits, clipping),
     one file per generator, `seed.rs` (SplitMix64).
   - `curvyo-ui-core`: panel content from the schema, Convert command plan, SVG export of pieces.
   - `curvyo-editor-wasm`: `session/pattern_cache.rs` and the synthetic draw snapshot.
2. **Generators.** Leaf forest: seeded dart throwing over a cell grid; a leaf is two cubic arcs
   inside a bounding circle; candidates whose circles meet an accepted one are rejected, so no two
   leaves overlap with room to spare; density is the target fraction of covered area. Interpolation
   waves: top and bottom boundary of the largest outer outline, each resampled by arc length to the
   same point count, blended linearly; clipping removes what falls into holes or the margin.
   Waves with amplitude 0 take the Hatch code path (criterion 11 exact).
3. **Lines mode draws the pieces with the shape's stroke settings even when the outline's own
   stroke is off**, so a pattern can be engraved without the outline being cut. UX may veto.
4. **Convert to paths** makes one path object per piece. If the measured commit for 50,000 pieces
   is slow, the implementer records it as a debt; Combine remains the user's way to one path.
5. **Fresh seed**: SplitMix64 of the node id's peer and counter; New seed: SplitMix64 of the old
   seed. Choosing Pattern again on the same shape gives the same seed (still stored, still stable).
6. **CPU budgets.** Thread-CPU tests on a 100 mm square at default parameters for each generator,
   in `curvyo-geometry-core/tests/pattern_cpu_time.rs`; the measured numbers replace the
   provisional limits in the spec and the module before Done. No limit is invented here.
7. **Outputs that do not exist yet** (PNG export, laser and plotter jobs; criteria 26 and 27 for
   those): not testable in this slice; tracked in `docs/technical-debt.md`.

## Questions for the customer

Frage PAT-STORE: Is a pattern stored as its settings and recomputed on every open (ADR 0029)?
- A (default, recommended): yes, settings only; the pieces are recomputed, files stay small, and
  files with patterns need the new Curvyo version.
- B: also store the computed pieces in the file (larger files, every shape edit rewrites them).
- Default if unanswered: A.
- **Decided: A (customer, 2026-11-02).**

Frage PAT-NEW (in `specification.md`): **Decided: A (customer, 2026-11-02)**; the generator
interpretations stand, Interpolation waves behave like Inkscape's Interpolate.
