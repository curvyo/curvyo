# ADR 0015: Node references and ancestor-resolved node state

**Status:** Accepted (customer, 2026-10-11; see "Customer decisions 2026-10-11"). Proposed
2026-10-10, revised before acceptance on 2026-10-11 for clone chains and for leaving output open.
It supersedes one sentence of [ADR 0002](0002-document-model-units-and-svg-round-trip.md) §5
("Each node carries its own affine transform and its own fully resolved style: no CSS cascade, no
inheritance"), and 0002's Status line points here. Asked by `specs/0049-linked-clone` and
`specs/0050-layers-and-objects-panel`.

## Context

ADR 0002 §5 rules out inheritance, so that concurrent edits never contend on a shared parent
register. Groups (`0023`) carry no style or transform, which fits.

Two specs now need a node whose meaning depends on registers that another node owns:

- `0050`: layers and groups get `visible`, `locked` and `opacity`. A hidden layer hides
  everything in it on the canvas.
- `0049`: a linked clone shows another path's outline and style through its own matrix, and a
  clone may follow another clone (a chain).

0002 is accepted, so the change is a new ADR (precedent: ADR 0014 and 0004 §2).

### Options: flags and opacity of layers and groups

- **A. Resolve through the ancestors when the document is read (chosen).** Each container has
  one register per flag, and a node's effective state combines its own register with every
  ancestor's.
- **B. Copy down.** Rejected. Hiding a layer of 10,000 writes 10,000 registers (`0050` criterion
  50 asks for one), and an object a peer moves in meanwhile stays visible.
- **C. Per-user view state.** Rejected. Every peer must see the same picture (`0050` decision 7),
  and whatever the manufacturing view later decides about output will read shared state.

### Options: linked clone

- **A. Reference (chosen).** The clone stores its original's id and a matrix, as one value. The
  outline and the style are read from the root of the chain when the document is read.
- **B. Copy on write.** Every edit of the original rewrites its clones. Rejected: one node drag
  under 10,000 clones of 100 nodes writes a million anchors, and a peer's edit reaches only the
  clones it knows, so clones silently disagree after a merge.
- **C. SVG `<use>` with per-property overrides (Inkscape).** Rejected: a cascade, which §5
  excludes, and `0049` needs no overrides.

## Decision

1. **The superseded sentence now reads:** "Each node carries its own fully resolved style; there
   is no style cascade. A node's meaning may depend on another node in exactly the two ways of
   ADR 0015 §2 and §3." A third way needs a new ADR. Group and layer nodes carry no style.
2. **Ancestor-resolved node state.** Any node may carry four node registers, which are not style:
   `name` (string, absent = none), `visible` (bool, absent = true), `locked` (bool, absent =
   false) and `opacity` (number in 0..=1, absent = 1). Effective values: visible if the node's
   own flag and every ancestor's are on; locked if the node's own flag or any ancestor's is on;
   opacity is the product of the node's value and its ancestors' values (for a clone, §3). `name`
   is never inherited. Nothing is copied into children, so showing an ancestor again restores
   the child exactly as it was. **This ADR does not decide what a hidden or locked node means for
   output.** The first exporter or job generator decides it with the customer (`0050` criteria
   24 and 54).
3. **Clone reference and chains.** A clone node (`shape = "clone"`) stores one register,
   `clone_link`: one value `{ origin, matrix }`, where `origin` is the original's `NodeId` and
   `matrix` six `f64`. It is written whole and never as a container, so a merge or an undo keeps
   an origin together with the matrix that was computed for it and never pairs one peer's origin
   with another peer's matrix.
   - The original is a path, a compound path or another clone. The **root** is the path or
     compound path at the end of the chain; the **depth** is the number of clones from the clone
     to the root, itself included.
   - A clone resolves when following `origin` reaches a live root without meeting a missing
     node, a node of another kind or a node twice, and the composed matrix (its own matrix times
     its original's composed matrix) is finite. Otherwise it is an **orphan**. The reader has no
     depth bound: the depth limit of `0049` (8) is checked only where a clone is created.
   - A clone reads exactly three things, all from the root, live: the outline(s), the `Style` and
     the `opacity` register. Its effective opacity is the root's `opacity` times the `opacity` of
     the clone's own ancestors; the ancestors of the root and of the clones in between do not
     count. It has no `opacity` of its own. `visible`, `locked` and `name` are its own: a hidden
     or locked original or middle clone leaves the clones behind it unchanged.
   - **No cycle by construction.** Every `origin` ever written names a node whose tree create op
     precedes the clone's own create op in (Lamport, peer) order: a new clone points to an
     existing node; a re-point (§5) points further down the clone's own chain; a copy creates the
     copy of an original before the copies of the clones that follow it. The property belongs to
     each written value, not to a state, so it holds after any merge and any undo, and no union
     of such links can form a ring. A ring can only come from a foreign or hand-edited file.
4. **Readers tolerate merge residue.** None of these counts as damaged: an orphan (not drawn, not
   hit, repaired by §5); `clone_link` on a node whose `shape` is not `clone` (ignored); other keys
   on a clone node (ignored); a `layer` marker below the top level (read as a plain group); a
   chain deeper than the creation limit (resolved). "Damaged" stays for values no writer of ours
   produces: a link that is not `{ id string, six finite numbers within ±1e7 }`, an opacity outside
   0..=1, a flag that is not a boolean.
5. **Removal, repair and orphans.** Every command that removes nodes repairs the live clones whose
   original it removes, in the same commit: it walks from the clone's original down the chain
   while the node is removed and is a clone. If the walk reaches a node that stays, the clone is
   re-pointed to it with the composed matrix of the skipped links (one `clone_link` write, the
   drawn outline unchanged). If it ends at a removed root, the clone is unlinked into a path. This
   includes the removals that undo, redo, object undo and go to version make (ADR 0014 §1). After
   Open and after every merge, one repair commit deletes all orphans. The set of orphans is a
   pure function of the converged state, independent of walk order, so peers that repair at once
   delete the same nodes; deleting an already deleted node does nothing.
6. **One read.** One pass per document version resolves the effective state and every clone, and
   consumers (render, hit test, selection, panels, marks, later output) use its values; none walks
   ancestors or follows a link itself. Each clone's composed matrix is computed once, from its
   original's (memoised, iterative, no recursion), and the root outline is transformed once per
   clone, never once per link. The pass also yields each clone's original, root and depth, and
   the direct clones of every object, which the panels and the canvas marks of `0049` read.
7. **Format.** Each of the two specs bumps `format_version` at merge (ADR 0004 §9). An older build
   refuses every file saved by the newer build.

## Consequences

- Every register has one owner, so concurrent edits combine without conflict: a peer hides a layer
  while another moves an object into it, the object ends up hidden; a peer edits a root while
  another transforms a clone, both apply. Two writes of one clone's link (two transforms, or a
  re-point and a transform) keep one of them whole.
- The read costs O(nodes + clones + clone anchors) per document version, the cost of the object
  cache of `0044`; chains add one matrix product per clone. A layer toggle is one write and one
  re-read.
- Undo, object undo and merges can deepen a chain past 8 (undo of an unlink of a middle clone,
  undo of a re-point). It still draws; only Linked clone on it is refused.
- Undo of a step that re-pointed or unlinked clones restores the link field by field (ADR 0014
  §1). The engine treats an absent key as a value and restores a `clone_link` only when its
  `origin` is live after the restore; otherwise the field is skipped and reported, and the clone
  keeps the original it follows now.
- Opacity is multiplied per paint (`0050` Question 5, A). Group compositing would cost a render
  pass per faded container per frame (ADR 0001); a separate decision.
- **Left for the sharing story.** None of these can happen before two peers edit one document, and
  each fix only relaxes a reader or adds a repair, so no file written today changes:
  - A node moved into a container a peer deletes goes with it (as for groups in `0023`); ADR 0014
    §11 can lift it out.
  - A link can end on a node a peer deleted: a clone made of a node a peer deletes, two peers that
    delete different middle clones of one chain, or a transform of a clone whose link wins over a
    concurrent re-point. Today the repair deletes such a clone. The deleted node's tombstone still
    holds its outline or its own `clone_link` (ADR 0014 §11 relies on Loro keeping it), so the
    repair can instead re-point through it or unlink from it. One atomic `clone_link` per clone is
    what keeps that repair possible.
  - Two legal moves can merge into a tree deeper than 32 levels, which the `0023` reader refuses;
    the walkers are iterative, so the reader can accept any depth.
- Named styles, style overrides or a relink command need a new ADR.

## Verification

Tests in `curvyo-document-core`:

- effective state over a three-level container chain;
- a clone of depth 1 and one of depth 3 resolve to the same path as after Unlink, within 1e-9 mm,
  with the root's style and the opacity of §3;
- a ring of 10,000 and a chain of 10,000 read without recursion; the ring is orphaned, the chain
  resolves;
- two replicas repair the same orphans concurrently, merge, and have equal state;
- a re-point and a concurrent transform of the same clone merge into one whole link;
- every residue of §4 reads without an error;
- 10,000 clones of a 100-node path, and 1,250 chains of depth 8, read within `0049` criterion 43,
  with one outline transformation per clone.

## Customer decisions 2026-10-11

1. Accepted (option A): a clone reads style and opacity from its original; visibility, lock and
   opacity resolve through the ancestors.
2. A clone of a clone is allowed; §3, §5 and §6 were revised for chains before acceptance. The
   depth limit 8 is a product rule of `0049`.
3. Output of hidden and locked nodes is not decided here (§2).
