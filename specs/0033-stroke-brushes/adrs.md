# ADRs for "Stroke brushes" (stage 1: built-in brushes)

Stage 1 adds style registers on path nodes (one format bump), a brush
definition type with a TOML loader, generators in `geometry-core`, a picker row
and a command. **No new crate, no new dependency edge outside the ones below,
no trait, no generic, no ADR amendment, no new ADR.** Stages 2 and 3 (maker
files, plugin brushes) need ADRs that are named at the end and are not part of
this Ready.

## Depends on

- [ADR 0002 §5, §9](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  per-node resolved style; the brush is a style value, generated geometry is
  derived and never stored; edits are commits in the journal.
- [ADR 0003 §1, §3, §7](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md):
  generators are `curvyo-geometry-core` functions on the 0.001 mm grid with an
  explicit tolerance; `i_overlay` integer engine for union and variable-width
  stroke (pinned by `0038`).
- [ADR 0004 §1, §9](../../docs/adr/0004-persistence-and-cross-machine-sync.md):
  an older reader would draw a brush path as a plain stroke and a job would cut
  the centre line, a silent wrong output, so the format version goes up.
  Libraries (§6, §11) matter from stage 2.
- [ADR 0005 §4, §8](../../docs/adr/0005-extension-and-plugin-model.md):
  "paths in, paths out" with declared parameters and generated controls. Stage
  1 builds the built-ins in exactly that shape so stage 3 only adds a caller.
- [ADR 0009 §3](../../docs/adr/0009-concurrent-editing-semantics.md): one
  last-writer-wins register per control.
- [ADR 0011 §3](../../docs/adr/0011-workspace-and-crate-layout.md):
  `render-core -> document-core` only.
- [ADR 0016](../../docs/adr/0016-clone-overrides.md): own style is "the
  ordinary style keys of the node", so brush keys in the style schema are
  covered with no ADR change.
- [`0018` adrs.md](../0018-stroke-markers/adrs.md) (register split, format bump
  reason), [`0030` adrs.md](../0030-document-size-presets/adrs.md) (TOML data
  file and the `toml` crate), [`0038` adrs.md](../0038-path-offset/adrs.md)
  (`i_overlay` integer API).

## Feature-local decisions

- **2026-10-17: (1) where the code lives.**
  - `curvyo-document-core`: `brush_def.rs` (the `BrushDef` value, its TOML
    loader, validation with line and reason) and `data/brushes/*.toml`; the
    style registers in `style_model.rs` and `style_codec`. Reason: the document
    needs the declarations to clamp and validate parameters on read; the
    presets of `0030` are already data files here and the `toml` crate is
    already a dependency of this crate.
  - `curvyo-geometry-core`: `brush/` with `width_profile.rs` (calls
    `variable_outline`), `nib_sweep.rs` (union of nib hulls), `stamp.rs`,
    `pattern.rs` (bend along arc length), `prng.rs` (SplitMix64, about 15
    lines, no `rand`) and `job.rs` (`brush_job_geometry`: union of pieces).
    One function, `generate(def, params, seed, centre_line, width) ->
    Result<BrushPieces, BrushError>`, with a `match` on the three kinds. No
    `Brush` trait: there is one implementation set and no second caller until
    stage 3, and ADR 0005's seam is "a pure function", not a Rust trait
    (`CLAUDE.md` §5).
  - `curvyo-editor-wasm` composes: it asks `geometry-core` for the pieces of
    each visible brush object and hands them to `render-core` as a flat fill
    layer with the stroke paint. **`render-core` does not depend on
    `geometry-core`** (ADR 0011 §3 stays). Rejected: adding the edge
    `render-core -> geometry-core` (an ADR 0011 amendment for one feature, and
    it would let `lyon` and the kernel drift together); placing the generators
    in `render-core` (markers do that because they need only `lyon`; brushes
    need `i_overlay`).
  - All new modules are under 500 lines by construction; `generate` dispatches
    to small functions per kind.
- **2026-10-17: (2) data model and format.** New keys on a **path** node, each
  its own LWW register (as `0018` decision 1), in the style schema:

  ```text
  stroke_brush                      : string  absent = none, e.g. "dots.1"
  stroke_brush_seed                 : integer 0..=999999  absent = 1
  stroke_brush_p.<name>.<param id>  : F64 | I64 | string by declaration, absent = default
  ```

  `<name>` is the brush id without its version suffix (`dots`), so the value
  survives `dots.1` to `dots.2` if the parameter keeps its meaning (the
  definition of the new version decides). Parameters of unpressed brushes are
  kept. Rejected: one grouped `brush` map (concurrent edits of two parameters
  would overwrite each other, ADR 0009 §3); one register per brush with a
  JSON string value (same, and opaque to `document.json`).
  Readers: clamp out-of-range values for use only; refuse a wrong type as
  damaged; keep and write back an unknown brush id (`0033` criterion 16). A
  primitive or compound path never gets these keys written.
  `format_version` = next free at merge (`specs/README.md`).
- **2026-10-17: (3) the Taper profile.** A `width-profile` definition gives its
  profile as points; Taper's points depend on its one parameter. Decision:
  the definition may mark a parameter as `drives = "ramp"` and the profile is
  then built as `[[0,0],[r,1],[1-r,1],[1,0]]` with r = parameter/100, capped
  at 0.5. No expression language and no general parameter binding (YAGNI);
  stage 2 can add more `drives` values when a real file needs them. Rejected:
  a small expression evaluator in the file (a second language to specify,
  test and secure).
- **2026-10-17: (4) determinism across targets.** Counts, placement rules and
  the SplitMix64 sequence are integer or exactly specified. Vertices come from
  `f64` trigonometry, which may differ in the last bit between `std` on
  native and on wasm32; every result is rounded to the 0.001 mm grid, and the
  cross-target test allows 0.002 mm (spec criterion 7). A `libm` dependency
  for bit-identical results was rejected: not needed for the stated goal
  (same pieces within the grid, same counts, same jobs).
- **2026-10-17: (5) limits are checked before generation.** The vertex and
  stamp counts follow from L and the parameters in O(1) (stamps, patterns) or
  from the flattened vertex count (profiles), so an over-limit set returns
  `BrushError::TooLarge { limit }` without building anything (spec criterion
  8). Constants live next to the generator; they are not settings.
- **2026-10-17: (6) job geometry.** `brush_job_geometry` is the union of the
  pieces by the boolean kernel at manufacturing tolerance (ADR 0003 §2). Stage
  1 builds and tests it; calling it is `manufacturing-roles`' job. A brush
  object is never a boolean operand before Expand brush (`0016` refusal text).
- **2026-10-17: (7) cache.** Results are cached in `editor-wasm` per object by
  (centre-line hash, width, brush id, parameters, seed), at most 64 MB,
  least-recently-used evicted. Session state, never document state (ADR 0009
  §2).
- **2026-10-17: (8) `variable_outline.rs` and `0032`.** `0032` (parked on
  `story/pen-tablet-input`) owns `variable_outline.rs`. `0033` needs the same
  function (centre line plus a width per vertex, round joins and ends).
  Whoever merges first brings the module; the other uses it unchanged, and a
  needed change goes in as a small shared commit, not a copy. Calligraphic's
  flat nib is not that function (the swept shape is not a disc) and gets
  `nib_sweep.rs`. Pressure stored on a path is not assumed (a document-model
  decision for both specs, `needs-customer`, only if `0032` Question 2 B is
  chosen).
- **2026-10-17: (9) licence.** The project is AGPL-3.0-or-later (ADR 0006).
  The six shipped brush files and their shapes are written for this project and
  carry the project licence; no third-party brush set is copied. A brush idea
  ported from GPL code (Inkscape's Pattern Along Path, PowerStroke) is a
  re-implementation from the behaviour, not a copy; code under
  GPL-2.0-or-later or GPL-3.0 would be compatible, code under GPL-2.0-only
  would not. Maker files (stage 2) stay the maker's.

## Deferred, with the ADR they need

- **Stage 2 (maker files):** definitions embedded in the document (a
  `brushes` table keyed by id and content hash, a second format bump), the
  folder as a library in ADR 0004's sense, a crate-level home for the folder
  reader (`curvyo-storage-io` or a new `-io` module). Needs a short ADR or an
  amendment to ADR 0004 §6 and §11 on the library entry, and a decision on the
  file format as a thing makers write (version policy, compatibility promise).
  Marked **needs-customer** when written (persistence and sync).
- **Stage 3 (plugin brushes):** a `brush` world in the WIT interface of ADR
  0005 (input: centre line, width, parameter values; output: closed paths;
  declared parameters as in `BrushDef`), a cache of the last generated
  geometry in the file when the plugin is absent, and the capability and
  licence terms for a plugin that links the SDK. Needs an ADR
  (**needs-customer**, plugin model) and the plugin host (R-EXT-001, not
  built). Until then nothing in the stage 1 code names plugins.

## Flagged to the lead

- The one structural point to check on the branch diff: the format change
  (decision 2) and the crate placement (decision 1). Review the milestone that
  adds the registers; the rest is generators and UI.
- Order of milestones on the branch: format and registers with golden files;
  width profile; stamp; pattern; panel row and Expand brush; job function.
- The build needs `0032`'s `variable_outline.rs`: if `0032` has not merged,
  cherry-pick that one file (decision 8) rather than writing a second one.
