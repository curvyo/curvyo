# ADRs for "Document template and guide paths"

Refined 2026-10-17 after the customer's answers. A document-model and file-format change:
**`needs-customer`**, decided by him (design in his answers of 2026-10-17). The decision is
[ADR 0019](../../docs/adr/0019-document-template-and-guide-paths.md) (Accepted). No new crate, no
trait, no generic, no new dependency.

## Depends on

- [ADR 0019](../../docs/adr/0019-document-template-and-guide-paths.md): the node register `guide`,
  the page stays a rectangle, one version bump, templates are loaded as guides.
- [ADR 0002 §2, §9](../../docs/adr/0002-document-model-units-and-svg-round-trip.md): mm only; one
  commit per edit; SVG user units at 96 per inch (stage 2).
- [ADR 0004 §9](../../docs/adr/0004-persistence-and-cross-machine-sync.md): an older build
  refuses the file instead of cutting the template.
- [ADR 0009 §2](../../docs/adr/0009-concurrent-editing-semantics.md): `guide` is a last-writer-wins
  node register.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) and `0042`: `set_guide` and
  `load_template` are one commit, one history step; the preview draws the guides of that version.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) and
  [ADR 0016](../../docs/adr/0016-clone-overrides.md): a clone reads the origin's `guide` at draw
  and output time.
- `0050` (layers): a guide is an ordinary node in the tree; `0051`: the File tab hosts the
  Template button; `0049`, `0057`, `0058`: unaffected.
- Stage 2 only: [`0024` adrs.md](../0024-svg-import-export/adrs.md) and ADR 0018 (the SVG reader
  crate). The loader needs from it the drawing's nodes and its frame (page size); agreed in
  `0024`'s plan.

## Feature-local decisions (2026-10-17)

1. **Stages.** Stage 1 needs nothing the repo lacks (flag, marking style, output filter, load from
   a Curvyo document). Stage 2 adds the SVG source after `0024`.
2. **Where the flag lives.** `document-core`: a `guide` accessor and `set_guide` command beside
   `visible` and `locked`. Output and export consumers filter on it in one function
   (`is_output_node`) so the rule exists once. Format version: next free at merge. Golden
   fixtures: `guide_v<N>.curvyo`, one damaged file (non-bool).
3. **Marking style.** `render-core` draws a node with `guide = true` as a stroke only: token
   `--guide`, device-pixel line, 2 / 2 dashes, fill and own stroke ignored. No lyon change; the
   dash is generated like the transform guide's.
4. **Loader.** `document-core` gets `copy_nodes_as_guides(source: &Document, target)`: pure,
   no I/O; the platform crate reads the bytes. A `.curvyo` source needs no new parser (the
   project reader exists). Placement for a Curvyo source is identity; for an SVG (stage 2) a
   translation by the frame's or the bounding box's top-left. Stage 2's function takes the
   SVG importer's drawing; no second parser.
5. **Boolean and combine.** The result node is a guide only when every input is one; the rule is
   one line in the shared result builder of `0016` and `0054`.
6. **Rejected here, recorded:** a per-guide colour or style (YAGNI), a `Template` node kind that
   remembers its source (a link that breaks; ADR 0019 option E), scaling at load (customer: no).
7. **Superseded:** the first version of this feature (shape registers, `contains_in_shape`,
   shape in formats, `0045` decision 9's "0046 hook") is not built. `0045` formats stay
   rectangles; its schema numbers 3 and 2 are not taken.
