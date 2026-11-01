# ADR 0027: Linked dimension objects

**Status:** Accepted 2026-11-01 by the customer (DIM-STORE A; document model, file format). Asked by
`specs/0070-measure-and-dimension` (DIM-LINK A, DIM-LOOK A, DIM-TEXT A). Extends
[ADR 0015](0015-node-references-and-ancestor-resolved-state.md) §1 (as amended by
[ADR 0022](0022-texts-on-carriers-and-text-clones.md) §5) with a fifth kind of reference, and
[ADR 0019](0019-document-template-and-guide-paths.md) §2 with a node kind that is always a guide.

**In plain words (for the customer).** A dimension is stored as its own small object: "what kind,
which points of which shapes, where the text sits, and the last number". It does not store its
lines; they are drawn from the shapes each time, so the number is always right while the shapes
exist, during a drag too, with nothing written. When a measured point disappears the dimension
falls back to its last number and stays where it was; Undo brings the link back. Files with
dimensions need the new version of Curvyo. A Sketch dimension (`0066`) stays inside its sketch;
both kinds share the look, the tool names and the text, not the storage (see §6).

## Context

The customer chose linked dimensions (DIM-LINK A): a Vector dimension follows the nodes it
measured through moves, scales, rotations and Node-tool edits, live, and keeps its last number when
a measured node is deleted. What is hard to change later: how a dimension names "this point of that
shape" in the file, and that an older build must not cut it.

## Options

| Option | Why it won or lost |
|---|---|
| **A. A dimension leaf kind with references, drawn from the referenced geometry (chosen)** | Stores intent, not marks. Linked values are derived on read, so a drag updates without writes, merges cannot disagree with the geometry, and undo is ordinary register restore. Marks are screen-constant (DIM-LOOK), so they cannot be stored in millimetres anyway. |
| B. A guide group of paths and a text, plus anchor references (the spec's first wording) | Every geometry edit would have to rewrite the group's paths and text in the same commit (write amplification, merge conflicts between the derived paths and the geometry), and screen-constant arrows have no millimetre size to store. Every path tool would also edit the derived paths. Lost. |
| C. Put Vector dimensions into a sketch graph (one object with `0066`) | Sketch constraints reference sketch entity IDs in local coordinates (ADR 0026); plain paths and shapes are not sketch entities and no solver moves them. Would need Vector geometry inside the sketch or the solver over the whole document. Lost. |
| D. Derive on read, but write the measured state into the dimension on every geometry edit | Correct values, but every edit to a measured shape writes the dimension too, and a missed command leaves it wrong. Lost to §4 (one commit funnel, only the "last number" record). |

## Decision

1. **Node kind.** A leaf in the objects tree with `shape = "dimension"`, on a layer like any
   object; visible, locked and opacity resolve through ancestors (ADR 0015 §2). The kind is a
   guide by definition: no `guide` register is written, every guide filter treats it as one. It
   is never a clone source and Make Clone skips it.
2. **Registers**, each one atomic value (ADR 0009 §2, merged per register):
   - `dimension_kind`: `aligned`, `horizontal`, `vertical`, `angle`, `radius`, `diameter`.
     Horizontal and vertical are along the document axes.
   - `dimension_ref_a`, `dimension_ref_b`, `dimension_ref_c` (two or three, by kind; angle uses
     vertex, first and second): a **point slot** is `anchor` (node id, `AnchorId`), `segment_mid`
     (node id, `AnchorId` of the segment start), `shape_point` (node id, `center`, `corner i`,
     `vertex i` or `edge_mid i` of a primitive) or `free` (a point in mm). Radius and diameter
     use one **curve slot**: `circle` (ellipse primitive), `arc` (node id, `AnchorId` of the
     segment start), `round_corner` (rectangle node id, corner `i`) or `free_circle` (centre in
     mm, radius as `Length` above zero; refers to no node, always resolves, written only by a
     copy, see §5). A clone is referenced by its own node id and its root's anchor ids.
     Sketch, text and image targets give `free` points: no references into a sketch graph.
   - `dimension_place`: signed offset of the dimension line from the measured points (mm) and the
     text position along it (fraction, default 0.5); for radius/diameter the leader angle.
   - `dimension_kept`: the measured points (mm) and the value (`Length` in mm or `Angle` in
     radians) as last measured. Written at creation.
3. **Resolution on read.** A pure function in `curvyo-document-core` resolves a dimension against
   a list of object snapshots, in the one read pass (ADR 0015 §6) after every other object, and
   against the editor's preview snapshots during a drag. A slot resolves when its node exists,
   is of the right kind and the feature exists; a curve slot also needs a circle within
   0.01 mm (a non-uniform scale unlinks it). All slots resolved: **linked**, value from the
   resolved document-coordinate geometry. Any slot not resolved: **unlinked**, everything from
   `dimension_kept`. References are never cleared, so Undo of a removal links again.
4. **The last number is kept in one funnel.** `commit_step` (the one place every editing step
   commits, ADR 0014) refreshes `dimension_kept` of each linked dimension whose resolved value or
   points changed beyond 1e-9 mm, in the same commit, only when the document has dimensions.
   A deletion, Boolean, conversion, Node-tool delete or join thus always leaves the value of the
   step before. Undo and redo restore it fieldwise; opening writes nothing.
5. **Edits.** Moving a linked dimension alone changes only `dimension_place`; moving it with its
   shapes writes nothing on it. An unlinked dimension's kept points move and transform, its value
   never changes (criterion 13). Copy/duplicate/paste remap references to the copies when the
   referenced node is in the copied set (node id map, positional `AnchorId` map, as
   `relink_copied_texts`); a point slot outside it becomes a `free` point, and a curve slot
   outside it a `free_circle`, at its resolved position moved by the paste offset (amended
   2026-11-01, review item 2 (a)). Free points and free circles of the copy move by the offset.
   **Moves.** A linked dimension moved or transformed while some node it refers to stays put
   writes nothing; an unlinked one, or one moved together with all its referents, moves its free
   slots and kept points.
   **Convert** (Guide button off) replaces the dimension, in place and in one commit, with a group
   of plain paths (lines; arrowheads as filled closed paths) and a text object, sized from the
   view at that moment so it looks unchanged; the caller passes the laid-out marks.
6. **Relation to `0066`.** A Sketch dimension stays a constraint in its sketch graph
   (ADR 0026). Shared: the kind vocabulary (one-to-one with the sketch relations), the layout
   (`DimensionGeometry`, `layout_dimensions` in `curvyo-ui-core`, which needs only
   `curvyo-document-core` types), the marks drawing and the number text. No `driving` register
   now (`CLAUDE.md` §5): a dimension that drives plain paths needs a solver over them and is a new
   ADR; the record can gain a register then with a format bump.
7. **Output.** Never in SVG export, G-code, job preview or PNG; no "include guides" switch.
8. **Format.** `format_version` goes up by one, next free at merge (31 on this branch, after the
   grid's 30). An older build does not know `shape = "dimension"`. Damage: an unknown kind, a
   slot count that does not fit the kind, a malformed slot (including a `free_circle` with a
   non-finite coordinate or a radius not above zero), a non-finite value.

## Consequences

- ADR 0015 §1 now reads "five ways": ancestor state, `clone_link`, `text_carrier`, `text_source`
  and a dimension's slots. A sixth needs a new ADR.
- Commands that replace geometry (Boolean, Convert to path of a shape, Break apart, clone unlink,
  which mints new anchor ids) unlink dimensions on it; their numbers stay. Accepted, not repaired.
- A commit in a document with dimensions does one resolve pass; its cost is measured with a
  thread-CPU-time test.
- Merge residue: after a peer's change is merged, `dimension_kept` may be older than the
  geometry until the next local step. Visible only if the merged change also unlinked it.
- Every exhaustive match on `ObjectSnapshot` gains a `Dimension` arm; outline readers skip it.
- No new crate, trait, generic or dependency; everything is in crates already built for wasm32.
- Spec `0070` wording to follow: "a group with the guide flag" (criterion 4) becomes "a dimension
  object"; criterion 16 (3) and "the same object" become "same kinds, look and text".
