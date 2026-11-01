# ADR 0028: Stored tiling nodes

**Status:** Accepted 2026-11-01 by the customer decision TILE-LIVE B (document model, file
format; `needs-customer` answered). Items marked **(Proposal)** are the architect's, vetoable at the demo. Asked by `specs/0069-tiling` (TILE-SOURCE A,
TILE-RESULT-GROUP A, TILE-EXTRAS A). Builds on [ADR 0015](0015-node-references-and-ancestor-resolved-state.md)
and [ADR 0016](0016-clone-overrides.md); adds no new kind of reference to 0015 §1 (the source
list is read only by the tiling command, never by the read pass).

**In plain words (for the customer).** A tiling is a group named "Tiling" that remembers its
recipe: which objects it repeats, the options and the seed. The instances inside are ordinary
clones (copies for images), so drawing, export and every tool treat them as today. Reopening the
tiling and changing a number recomputes the recipe and moves, adds or removes only the instances
that change, in one undo step. Edits by hand to an instance last until the tiling is applied
again; "Release tiling" turns it into a plain group for good. Files with tilings need the new
version of Curvyo.

## Context

The customer chose a reopenable tiling (TILE-LIVE B), each instance a group of clones with one
clone per source object (TILE-SOURCE A), and the result in one group (TILE-RESULT-GROUP A).
What is hard to change later: where the recipe is stored, how an instance keeps its identity
across a regeneration, and what a peer's concurrent edit does to it.

| Option | Why it won or lost |
|---|---|
| **A. A group with a recipe register; the instances are its generated children (chosen)** | The result already is a group (TILE-RESULT-GROUP A). Every reader, tool and exporter sees a group of clones and needs no new arm. Per-instance own style (colour variance) and per-instance edits work because instances are real nodes. Only the tiling command reads the recipe. |
| B. A leaf kind `shape = "tiling"` that derives its instances on read (like the dimension of ADR 0027) | No writes on regeneration, but up to 100,000 instances derived in every read pass; every consumer (hit test, snapping, Boolean, export, job, Objects panel) needs a new arm; an instance cannot be selected, coloured or edited, which criterion 6 needs. Lost. Pattern fill (`0071`) may still want derive-on-read inside a boundary; that is its own ADR and can reuse the recipe and the layout. |
| C. Recipes in a separate root map keyed by group id | Copy, delete, undo and merge would have to keep two places in step; a deleted group leaves an orphan recipe. Lost. |
| D. A, plus a list of generated instance ids in the recipe | Two authorities for one fact; a merge of two applies leaves them disagreeing. Lost: the children and their keys are the list. |

## Decision

1. **The tiling node.** An ordinary group (`group: true`, `name = "Tiling"`, in the source's
   layer) that also carries two registers, each one atomic value (a Loro map or list *value*,
   written whole, never a container; merged per register, ADR 0009 §2):
   - `tiling`: the recipe: `layout` (`grid` | `circle`) with its options in `Length`, `Angle`
     and `Point` (field list in `tiling_codec.rs`), the variance per channel (random ± and step
     per row and column), `seed` (u32), `result` (`clones` | `copies`), `keep_source` (bool).
     One value, because the instances realise one whole option set: half of one peer's options
     with half of another's is a set neither generated.
   - `tiling_sources`: the ordered list of the selected node ids (top-level, as selected; a
     selected group stays one id and is flattened to its leaves at each apply).
2. **Instances.** Each generated child carries `tiling_cell = [row, column]` (u32 pair; a circle
   uses `[0, i]`). The key, not a position in a list, is the instance's identity. When the
   sources flatten to one leaf the instance is that leaf's clone; otherwise it is a group of one
   clone per leaf in the sources' z-order, nested groups of the source flattened (Proposal), and
   each member carries `tiling_member = <source leaf id>`.
   - **Clones** (`result = clones`): paths, compound paths, primitives (converted to paths in the
     same commit, as `0047`), clones and sketches give linked clones (sketches are clonable since
     ADR 0026; the plan's "sketches as copies" predates that, Proposal); texts give linked text
     clones (ADR 0022). Every generated clone has `own_placement = true` (ADR 0016 §2), so moving
     or rotating a source does not scatter rotated instances; node and style edits of the source
     still flow. Its `clone_link` matrix is the instance's affine `A_cell`.
   - **Own style** is written only when the opacity or hue variance of the recipe is not zero:
     the source's style with the varied opacity or hue, ADR 0016 §1. Such an instance no longer
     follows style edits of its source until the next apply (Proposal; the only per-clone
     opacity ADR 0016 allows).
   - **Copies**: images always (not clonable), every leaf when `result = copies`: the source
     leaf mapped by `A_cell` (an image keeps its asset). Dimensions are skipped (ADR 0027 §1).
   - **The source** with `keep_source`: the selected nodes move into the tiling group as its
     lowest children and stand for cell `[0, 0]`, which is not generated. Without it they stay
     where they are and the tiling group goes directly above the topmost of them.
3. **Apply (create and regenerate).** The caller computes the placements; the document writes
   them. One `Document` command takes the recipe, the sources and, per cell key, `A_cell` and
   the varied style, and in **one commit** (history step "Tile" on create, "Edit tiling" on a
   reopen):
   - keeps every child whose key is still in the layout and is still a generated instance
     (a clone or text clone of the expected source with `own_placement`, or a copy of the
     expected kind); writes its matrix, affine, geometry (copies) or own style only where it
     differs beyond 1e-9 mm (or the style differs at all);
   - creates the missing keys and the missing members, deletes children whose key left the
     layout, and members whose source leaf is gone;
   - leaves alone every child without `tiling_cell` (the source, objects the maker put into the
     group) and every keyed child the maker turned into something else (unlinked, converted):
     its cell counts as taken;
   - replaces an instance whose shape changed (bare clone ↔ group of members, clones ↔ copies)
     with new ids;
   - writes the recipe last, only if it changed.
   Variance for a cell depends only on seed, key and recipe, never on the cell count, so adding a
   column leaves every other instance where it was. Unchanged instances write nothing, so undo
   steps and merges stay small, and a peer whose floats differ in the last bit rewrites nothing.
4. **Refusal before the first write.** No sources left after skipping; more instances than the
   refusal limit; an instance group deeper than the group depth limit; a clone chain over its
   limit (`0049`). Nothing is written then.
5. **Edits of instances.** An instance is an ordinary node: it can be selected inside the group,
   moved, given its own style, unlinked or deleted with the existing commands. Apply owns the
   placement of every managed instance and its own style while opacity or hue variance is on:
   the next apply rewrites them and recreates deleted cells. An own style set by hand with no
   style variance is kept. To keep hand edits for good the maker releases the tiling (§7).
6. **Delete, undo, copy, merge.**
   - *Delete* of the tiling group deletes it with its children, the kept source included.
     Delete of a source repairs its clones as `0049` and ADR 0016 §4 do; the recipe keeps the
     dangling id (never cleared, as ADR 0027 §3), reopen offers only Release, Undo restores.
   - *Undo* restores registers and tree moves field by field (ADR 0014), engine unchanged.
   - *Copy, duplicate, paste* of the tiling group remap `tiling_sources` and `tiling_member`
     through the node id map when the sources are in the copied set (the kept source is), else
     keep them, as a copied clone keeps its original (`0049`). A copied instance pasted outside
     its tiling group loses `tiling_cell` and `tiling_member` (Proposal).
   - *Merge* (last writer wins per register, tree operations as Loro merges them): two
     concurrent applies keep one recipe whole and the union of their creations, so a key can
     appear twice and matrices can belong to either recipe; a concurrent delete of an instance
     wins over writes to it; a concurrent Release and apply leave a plain group with extra
     clones; a concurrent delete of the group removes children created under it. Readers
     tolerate all of it; reads never repair. The next apply repairs: per key it keeps the child
     with the lowest node id, deletes the other copies of that key and rewrites placements.
7. **Release** removes `tiling`, `tiling_sources`, every `tiling_cell` and `tiling_member` in one
   commit, leaving a plain group of clones and copies. Ungroup of a tiling group strips the same
   registers in its commit.
8. **Reading costs nothing new.** The read pass reads a tiling as a group and its instances as
   clones (ADR 0015 §6 and the shared root tessellation of `0049`); it never decodes the recipe
   or computes a layout. Opening writes nothing. Only reopening computes the layout.
9. **Export and output.** A tiling exports and outputs exactly as the same group of clones and
   copies made by hand: SVG `<g>` of resolved outlines, G-code and job preview from the resolved
   instances. The recipe is never exported (no passthrough, ADR 0018).
10. **Format.** `format_version` goes up by one, to the next free number at merge (**32** today:
    main is at 31 since `0070`). An older build would edit a tiling as a plain group and lose
    the link between recipe and instances silently, so it must refuse the file. Damage: `tiling`
    on a node that is not a group; an unknown `layout` or `result`; a count of zero or over the
    refusal limit; a non-finite number; a malformed `tiling_sources`, `tiling_cell` or
    `tiling_member`. Tolerated residue: dangling source ids, duplicate keys, missing keys, keyed
    children of a group without a recipe, unkeyed children of a tiling.

## Consequences

- A tiling is cheap to read and expensive only to apply: 10,000 instances cost 10,000 clone
  nodes in the document, as hand-made clones would. Apply and its preview get thread-CPU-time
  budgets (see `specs/0069-tiling/adrs.md`).
- Hand edits of instances are overwritten by the next apply; the spec must say so, and Release
  is the way out. Accepted for simplicity: no per-instance "detached" flag until a story needs it.
- Moving or rotating a source on its own writes one held-back `clone_link` per instance
  (ADR 0016 §2 cost, bounded by the instance count); moving the whole tiling moves every
  instance as any selected clone moves.
- Opacity or hue variance gives instances their own style, so source style edits stop flowing
  to them until the next apply.
- Every writer that copies or ungroups must handle the three tiling registers; a test lists them.
- No new node kind in `ObjectSnapshot`, no new kind of reference in the read pass, no new crate,
  trait, generic or dependency. The layout and generator are pure code in `curvyo-ui-core`.
- `0071` pattern fill can reuse the recipe value and the layout; a boundary or clipping is a new
  register and its own ADR.
- Goldens: `tiling_v32.curvyo` (grid with hue variance, circle, multi-source instances with an
  image copy, `keep_source` on and off, a copies tiling), one damage file per rule, a residue
  file that opens; a tiling's SVG export equals the hand-made group's.
