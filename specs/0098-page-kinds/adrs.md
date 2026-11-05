# ADRs for page kinds

Architect, 2026-11-05. Slice 1 of `0063` (concept A).

- [ADR 0042](../../docs/adr/0042-page-kinds.md) (new, Proposed, `needs-customer` for document
  model and format; design by the customer): the `page.<id>.kind` register and its read rule
  (criteria 5, 18 c, 20), output pages listed and in the plane (10, 11, 12, 17), the page owning
  the manufacturing's lifetime (4, 16), size from the profile about the origin corner (13, 14,
  15), the workbench derived from the active page and the kind list (7, 8, 9, 18 a, b), only
  sketch pages own a sketch (3), the format-36 migration (19), format 37 and goldens (20).
- [ADR 0024](../../docs/adr/0024-spatial-pages.md) §1, §3, §5: frames, membership by position
  (own items of a laser page, 155 A), the frame move; amended by 0042 for the kind.
- [ADR 0032](../../docs/adr/0032-workbench-model-and-registry.md): manufacturing node, placement,
  fixtures and the `WorkbenchEntry` registry survive; §1 to §5, §9 amended by 0042.
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md): registers and the
  unknown-process rule unchanged; §1, §5 amended by 0042 (lifetime, node `name`).
- [ADR 0038](../../docs/adr/0038-machine-profile-records.md): the record already holds
  `work_area_mm`; criterion 15 edits it, no record format change.
- [ADR 0039](../../docs/adr/0039-machine-profile-copy-in-the-project-file.md): `mfg_profile`
  unchanged; §4 amended by 0042 (corner-anchored resize).
- [ADR 0026](../../docs/adr/0026-parametric-sketch-nodes-and-workbench.md): claim registers and
  lazy sketch creation unchanged; ownership limited to sketch pages by 0042.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md),
  [ADR 0016](../../docs/adr/0016-clone-overrides.md) §2: the linked clone the migration leaves
  and the held-back move of the sketch (criterion 19 b).
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): reserved label `migrate_format`,
  not an undo step; activation is no step (criterion 8).
- [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md): manifests supply the Laser
  entry of the kind list and its glyph and word; the disable list gives "Laser (off)".

## Feature-local decisions

1. 2026-11-05: **CPU budget** of criterion 21 confirmed at 50 ms thread CPU per activation in
   `curvyo-editor-wasm/tests/cpu_time/`. Activation is view state: it reads the page list and one
   meta map per output page and must not re-resolve geometry or walk the object tree.
2. 2026-11-05: **Output page drawing order**: page backgrounds, then fixtures, then placements,
   then the drawing tree, then frames and labels (ADR 0024 consequences, ADR 0032 §8 order kept).
3. 2026-11-05: **Kind glyph and word** come from the manifest for output kinds and from the shell
   for Drawing and Sketch; no `if laser` in the shell (criterion 18 a).

## Spec issues (for the product owner)

1. Criterion 17 does not say whether a placed page is in the SVG export and print of a laser
   page. Proposal: yes, drawn as on the canvas (it is visible content of the page); fixtures, the
   origin marker and the work-area text no.
2. Tools on an "Unknown kind" page and on a "Laser (off)" page are not stated. Proposal (ADR 0042
   §7): the vector rail and drawing tabs; the manufacturing data is never written.
3. Criterion 19 b says the new sketch page is "beside it"; ADR 0042 uses the Add page position of
   criterion 2 (right of the last page), which never overlaps. Proposal, same for 19 a.
4. Criterion 14 "the origin corner of the profile": read as the newly picked copy's corner, and
   the corner keeps its place in the plane (the frame grows away from it), so own items are not
   moved. Proposal.
5. Criterion 3 "holding one empty sketch": built as ADR 0026's lazy creation (no node until the
   first edit); the maker sees the same. Proposal.
6. Criterion 19 b on a drawing page whose only content is its sketch leaves an empty drawing page
   beside the new sketch page. Acceptable; converting such a page in place would be a second
   path for little gain.
