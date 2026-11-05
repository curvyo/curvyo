# ADRs for manufacturing workbenches (epic)

Architect, 2026-11-02. Stage 0 of the epic. Customer input of 2026-11-02: the laser workbench is
the first extension, its output is a page created by the workbench, fixtures are part of the
machine setup, the connector protocol follows.

- [ADR 0032](../../docs/adr/0032-workbench-model-and-registry.md) (new, Accepted by the customer
  2026-11-02): a workbench is session view state; a manufacturing is a top-level node; its
  output page is an owned ADR 0024 page sized to the work area with page-local content; placements
  are live page references resolved like clones; fixtures are machine-setup nodes drawn on the
  output page and never output; the front-end registry with three `WorkbenchEntry` users.
  Covers criteria 1 to 6 and the model half of 9.
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) (new, Accepted by the customer
  2026-11-02): registers, `x.<process id>.*` process data, unknown processes kept untouched,
  `format_version` next free at merge. Covers criterion 9.
- [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md) (new, Accepted by the customer
  2026-11-02): the laser workbench is `curvyo-laser-core`, registered in
  `ui-core/src/extensions.rs`; stage 7 (extension workbenches) is therefore the same path as the
  laser's.
- [ADR 0035](../../docs/adr/0035-machine-connector-protocol.md) (new, Accepted by the customer
  2026-11-02): the connector protocol the Laser workbench will speak after `0029`.
- [ADR 0024](../../docs/adr/0024-spatial-pages.md): page records and membership by overlap, which
  a placement reads; owned pages are the exception of ADR 0032 §5.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §3, §6 and
  [ADR 0016](../../docs/adr/0016-clone-overrides.md): the one-read resolution pass a placement
  joins; `placement_link` is written whole like `clone_link`.
- [ADR 0026](../../docs/adr/0026-parametric-sketch-nodes-and-workbench.md): the Sketch workbench
  and its session state; its "no framework yet" sentence is overtaken by ADR 0032 now that a third
  workbench exists.
- [ADR 0019](../../docs/adr/0019-document-template-and-guide-paths.md): guides are not drawn in a
  placement and never output.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md), [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md):
  manufacturings use ordinary history steps and per-register merge.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §1, §6: no new container
  member; reusable machine profiles (`0026`) are library records.

## Feature-local decisions

- 2026-11-02: Stage 2 (laser MVP slices `0026` to `0029`) is built inside `curvyo-laser-core`
  and the Laser workbench of ADR 0032, not as editor features. Machine profile records with
  installed fixtures move to the library in `0026` with their own ADR (ADR 0032 §8).
- 2026-11-02: Stage 5 (CNC) is the trigger for a shared machine abstraction (`CLAUDE.md` §6) and,
  with the second extension crate, for the `Extension` trait (ADR 0034 §3).
- 2026-11-02: WB-PAGE-VIS A and WB-FIXTURE-HOME A (customer, 2026-11-02) are what ADR 0032 §5
  and §8 assume; no change to the ADRs. WB-NAME A, WB-MULTI A, WB-PLACE-DEFAULT A need none either.
- 2026-11-05: Concept A (customer). Slice `0098` is covered by [ADR 0042](../../docs/adr/0042-page-kinds.md)
  (page kinds, output page listed and in the plane, workbench of the active page; amends 0024,
  0026, 0032, 0033, 0039). WB-PAGE-VIS A and WB-PLACE-DEFAULT A no longer hold. The withdrawn laser-roles ADR of the
  closed PR #269 never reached `main`; its replacement is the operations ADR of
  `0101`.
