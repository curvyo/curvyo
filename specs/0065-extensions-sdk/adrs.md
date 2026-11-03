# ADRs for the extensions SDK

Architect, 2026-11-02. Customer decisions of 2026-10-17 (EXT-ROUTE: compiled in, front-end
registry; EXT-LICENCE: AGPL-3.0-or-later) and 2026-11-02 (the laser workbench is the first
extension).

- [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md) (new, Accepted by the customer
  2026-11-02): extension crate naming (`curvyo-<name>-core`, first `curvyo-laser-core`),
  the contract crate `curvyo-extension-core`, the single registration module
  `curvyo-ui-core/src/extensions.rs`, manifest and parameter schema as Rust values, the
  `extensions.toml` disable list (criterion 10, R-EXT-002), the licence rule, and the list of what
  waits. Covers criteria 1, 2, 4, 5, 8 (trusted-code mark), 9, 10.
- [ADR 0032](../../docs/adr/0032-workbench-model-and-registry.md) §2, §3: the workbench registry on
  both sides; `ManifestWorkbench` draws any extension workbench without TSX.
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) §3, §4: where an
  extension's data lives in the file and what a build without it does.
- [ADR 0005](../../docs/adr/0005-extension-and-plugin-model.md): wasm components stay the later
  route (§1, §2 unchanged); §9 amended by ADR 0034 for compiled-in crates. Reader's note added.
- [ADR 0006](../../docs/adr/0006-license.md): extensions are part of the AGPL work; reader's note
  added pointing at ADR 0034 §7.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §3: dated note with the new crates
  and edges.

## Feature-local decisions

- 2026-11-02: Criterion 1's "manifest file" is, for the compiled-in route, a typed Rust value
  returned by `manifest()` (ADR 0034 option D). A file arrives with the wasm route.
- 2026-11-02: Criterion 3 (toolbox ordering when two extensions add to one card) and criterion 6
  (vectorizer contribution) are not built until a second contributor exists; the rule "installation
  order, then name" becomes "registration list order, then name" for compiled-in crates.
- 2026-11-02: The first third-party-style proof (a generator such as Honeycomb, `0071`) is the
  natural second extension and the trigger for the `Extension` trait.
