# ADRs for the laser workbench shell

Architect, 2026-11-02. First slice of `0063` (stages 0 and 1) and first extension of `0065`.

- [ADR 0032](../../docs/adr/0032-workbench-model-and-registry.md) (new, Accepted by the customer
  2026-11-02): `WorkbenchId::Manufacturing(NodeId)`, the registry on both sides, the
  manufacturing node, the owned output page sized to the work area (criteria 2, 3, 4), the
  placement and its resolution (5, 6, 8), read-only placed content (7), fixture nodes (9, 10).
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) (new, Accepted by the customer
  2026-11-02): registers, `fixture_kind` / `fixture_params`, unknown-process round-trip,
  goldens and the `format_version` bump (12, 13).
- [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md) (new, Accepted by the customer
  2026-11-02): crates `curvyo-extension-core` and `curvyo-laser-core`, the registration
  module, manifest and block kinds `Params`, `FixtureList`, `Notice`, the disable list (11).
- [ADR 0024](../../docs/adr/0024-spatial-pages.md), [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md),
  [ADR 0026](../../docs/adr/0026-parametric-sketch-nodes-and-workbench.md),
  [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): as listed in the `0063` adrs.md.

## Feature-local decisions

1. 2026-11-02: **Affected crates.** New `curvyo-extension-core`, `curvyo-laser-core`;
   `document-core` (manufacturing, placement, fixture nodes, `page.<id>.owner`, placement
   resolution in the one-read pass, skip rule for manufacturing subtrees, format); `ui-core`
   (`extensions.rs`, `extensions.toml` parsing, workbench tabs); `editor-wasm` (`WorkbenchId`,
   workbench list and manifest JSON, laser view drawing); `frontend` (`workbenches/registry.ts`,
   `ManifestWorkbench`, Vector and Sketch moved behind it). No new external dependency.
2. 2026-11-02: **The skip rule is one predicate** in `document-core`
   (`is_in_manufacturing(node)`), used by Layers rows, hit test, marquee, Select all, page
   membership, exports and every repair. Each consumer gets one test with a manufacturing present.
3. 2026-11-02: **Engraved grid outline** is computed by `curvyo-laser-core` from spacing, line
   width and origin offset over the work area, as open line segments in page-local mm; drawn as a
   thin stroke below placements. It is not the view grid of `0090` and does not snap (spec out of
   scope).
4. 2026-11-02: **Criterion 10 and 12 together:** hiding a fixture writes the ordinary `visible`
   register (saved, undoable), not session view state; the spec's word "view state" means "no
   effect on output", which holds.
5. 2026-11-02: **Criterion 14 budget** (1.0 s thread CPU for 1,000 objects): accepted as a
   Proposal; measured with a test in `curvyo-editor-wasm/tests/cpu_time/`. The placement reuses
   resolved outlines and composes one matrix, so the cost is close to drawing the Vector page.
6. 2026-11-02: **Placeholder setup** "Generic laser" 400 x 300 mm is `mfg_machine_name` plus the
   owned page's size; no profile record until `0026`.
