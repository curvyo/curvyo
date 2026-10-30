# Architecture: multiple pages

Date: 2026-10-29. Spatial membership is customer-decided; all implementation
choices marked **Proposal** are reversible in this story.
Customer clarification (2026-10-29): old files are irrelevant while unreleased;
backward compatibility and old-file migration are not required.

## Decisions used

- [ADR 0024](../../docs/adr/0024-spatial-pages.md): ordered page frames in one
  global plane; per-field registers from initialization; format 26.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  millimetre newtypes and the shared object tree remain the document model.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md): page fields
  merge independently; active page and pointer context are ephemeral.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one labelled
  transaction per page command; conditional fieldwise undo retains peer edits.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md)
  and [0016](../../docs/adr/0016-clone-overrides.md): cross-page clones retain
  their references; use resolved bounds and existing transform semantics.
- [ADR 0019](../../docs/adr/0019-document-template-and-guide-paths.md): frames
  stay rectangular. Criterion 3's older shape reference means no new shape
  model; templates/guides stay ordinary objects.
- Rejected [ADR 0012](../../docs/adr/0012-pages-in-the-document-model.md) is
  historical background, not an instruction to create page root nodes.

## Implementation review points (Proposal)

- `curvyo-document-core/src/document.rs` writes format 25 today; reserve 26.
  `container.rs` already rejects a newer manifest; do not bump the independent
  Loro snapshot version. Initialize a real first page for new documents;
  avoid a second legacy storage path. Old formats have no guarantee.
- `history/touched.rs` recognizes the root map, `history/checkout.rs` reads it,
  and `history/merge.rs::restore_map` restores independent fields. Flat root
  page registers reuse this boundary without another tree restore engine.
  Extend `history/rows.rs::is_document_level_label` and UI operation labels.
  Extend preview data and rendering so held versions keep their own frames.
- The initial page may alias its width and height directly to the existing
  root size registers: these remain its sole authoritative fields, with no
  mirrored copy. Other page dimensions use their page-ID namespace. Legacy
  size APIs must consistently address the initial live page, or the first
  surviving page after its deletion, for both reads and writes.
- Fractional numeric order needs a deterministic renumbering fallback in the
  same reorder transaction when equal concurrent ranks or floating-point
  precision leave no strictly intermediate value. Stable ID tie-breaking
  ensures convergence but cannot alone implement every requested move.
- Current `Document::resize` in `document_size.rs` shifts every object by half
  the size delta. Preserve that single-page contract. Active-page commands
  must keep the active frame origin fixed and shift wholly contained objects
  by half the size delta. Other frame origins stay fixed; partially contained
  objects and objects shared with another page stay in place. Preserve current
  resize behaviour for a document that still has just its original page.
- Determine fully contained movable leaves from their resolved bounds before
  a drag. Use existing clone-aware bulk transforms, in the same transaction as
  the frame move. Frame-only mode writes no object transforms.
- Add, delete, rename, reorder, resize and move need core tests, including
  no-op edits, undo/redo and save/open. Include concurrent first page edits and
  concurrent deletion down to the invariant of at least one visible frame.
- UI tests cover active fallback after undo/deletion/import, active ruler
  origin, last-pointer Ctrl+A context, shared formats and labels, and the
  200-page scenario. Existing single-page tests remain regression evidence.
- Apply the spec's unanswered defaults: shared layers; rightward gap of
  `max(20 mm, 10% of the larger side)` with deterministic wrapping at the
  specified approximate row limits. The plan fixes the exact wrapping rule.

No new crate, trait, dependency, page-local coordinate system or membership
register is required. Export per page remains outside this story.
