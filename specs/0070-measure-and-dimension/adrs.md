# ADRs for "Measure and dimension"

Date: 2026-11-01. The storage of a linked dimension is a document-model and file-format decision:
**[ADR 0027](../../docs/adr/0027-linked-dimension-objects.md), Accepted (customer, 2026-11-01:
DIM-STORE A; MEAS-RAIL A and MEAS-KEEP A = Shift decided the same day)**
(recommended default A). The customer chose the behaviour (DIM-LINK A, DIM-LOOK A, DIM-TEXT A);
ADR 0027 is how it is stored. Everything under "Feature-local decisions" is **Proposal**,
reversible inside the story. No new crate, trait, generic or dependency.

## Depends on

- [ADR 0027](../../docs/adr/0027-linked-dimension-objects.md): the `dimension` leaf kind, its
  slots, resolution on read, the kept last number refreshed in `commit_step`, convert, copy remap,
  output exclusion, format bump. Amended 2026-11-01 (review item 2 (a)): a copied radius or
  diameter whose curve was not copied gets a `free_circle` curve slot at the paste offset; a
  linked dimension moved without all its referents writes nothing.
- [ADR 0019](../../docs/adr/0019-document-template-and-guide-paths.md) §2-3: a dimension is a
  guide by kind; the guide filters exclude it from output.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §1, §2, §6 and
  [ADR 0022](../../docs/adr/0022-texts-on-carriers-and-text-clones.md) §3, §5: references
  resolved in the one read pass; ADR 0027 adds the fifth kind; copy remap follows
  `relink_copied_texts`.
- [ADR 0026](../../docs/adr/0026-parametric-sketch-nodes-and-workbench.md): Sketch dimensions stay
  constraints in the sketch graph; Vector dimensions do not reference sketch entities.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §3: `Length`, `Point`,
  `Angle` newtypes, mm and radians; explicit tolerances.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: older builds refuse the
  file; [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §2: atomic registers;
  [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): one step per gesture.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md): `ui-core` → `render-core` is not
  allowed; layout in `ui-core`, drawing in `render-core`, wiring in `editor-wasm`.

## Feature-local decisions (2026-11-01)

1. **Where it lives.**
   - `curvyo-document-core`: `dimension_model.rs` (kind, slots, place, kept), `dimension_codec.rs`
     (registers, damage rules), `dimension_read.rs` (pure resolve and measure against snapshots:
     distance, projected distance, angle, circle from a primitive, an arc segment's circumcircle
     within 0.01 mm, a rounded corner), `dimension_commands.rs` (create, place, move kept points,
     convert, copy remap), `dimension_keep.rs` (the `commit_step` refresh). New variant
     `ObjectSnapshot::Dimension`.
   - `curvyo-ui-core`: rename `sketch_dimension_layout*` / `_text` / `_projected` to
     `dimension_layout*` (behaviour-preserving; it already needs only `document-core` types, no
     `sketch-core`). `measure_tool.rs`: the gesture, kinds, Alt cycling, selection-first, readout.
     The number text ("12.5 mm", "R 25 mm", "⌀ 50 mm", "45°", 3 decimals trimmed) moves from
     `editor-wasm/src/session/sketch/annotations.rs` to one shared formatter in `ui-core`, used by
     both workbenches (DIM-TEXT A).
   - `curvyo-render-core`: `sketch_annotation_marks.rs` becomes the shared dimension marks
     drawing; one look enum `Driving | Reference | Guide` replaces the `reference` bool; `Guide` is
     solid, `theme` guide green.
   - `curvyo-editor-wasm`: session wiring, hit test of marks and text, text labels handed to the
     frontend overlay exactly as Sketch dimensions are.
2. **Slots from snapping.** Point targets of `0060` (nodes, centres, midpoints) gain an optional
   source (`node id` + anchor or shape feature) so the tool gets the slot without a second
   lookup. Grid, page and guide-line targets have none and give `free` points.
3. **Live during drags.** The editor resolves dimensions against the same preview snapshots it
   draws (`dimension_read`), every frame; nothing is written until the gesture's commit.
4. **Measure mode stores nothing.** The readout is session state in `ui-core`; it never touches
   the document (criterion 1 test: history length unchanged).
5. **History names.** Create: "Dimension". Text or offset drag: "Move dimension". Convert:
   "Guide" (the existing button's name). Moving kept points: "Edit nodes".
6. **Convert details.** Lines and arrowheads take the guide green as stroke/fill and the line
   width of 1 screen pixel at the current zoom, in mm. The text becomes a text object (ADR 0020)
   in the Text tool's default font, cap height = the 12 px label height at the current zoom. Not
   outlines: a text object is editable and Convert to path of a text already exists.
7. **Format and goldens.** `CURRENT_FORMAT_VERSION` 31 (provisional; the grid on this branch took
   30). Goldens in `curvyo-document-core/tests/fixtures/`: `dimension_v31.curvyo` (each kind;
   linked to a path anchor, a segment midpoint, every primitive feature, a clone, a circle under
   both curve kinds, an arc and a rounded corner; one unlinked with a deleted node; one with a
   `free` slot; one with a `free_circle` slot) and damaged files `dimension_kind_unknown_v31`,
   `dimension_slots_count_v31`, `dimension_slot_malformed_v31`, `dimension_value_nan_v31`.
   Export goldens: SVG and G-code of a document with dimensions equal those without.
8. **Tests the plan must name.** Kept value after each removing command (Delete, Cut, Boolean,
   Node delete, join, Break apart, Convert to path of a shape, clone unlink); undo relinks; copy
   with and without referents; non-uniform scale of a measured circle unlinks; commit refresh
   cost by thread CPU time; native/wasm parity of `dimension_read`.

## For the product owner

ADR 0027 changes two spec sentences: criterion 4 "a group with the guide flag" → "a dimension
object (a guide by kind)"; criterion 16 (3) and "A reference dimension of `0066` and a dimension
of this entry are the same object" → "share kinds, layout, look and text; no driving flag until a
dimension drives plain geometry (new ADR)". The Links line "no new ADR" → ADR 0027.
