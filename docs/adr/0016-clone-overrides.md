# ADR 0016: Clone overrides

**Status:** Accepted 2026-10-14 by the customer (proposed the same day; document model and file format). Extends
[ADR 0015](0015-node-references-and-ancestor-resolved-state.md) §3 (what a clone reads), §4
(residue) and §5 (repair); ADR 0015 said overrides need a new ADR. Asked by
`specs/0057-clone-overrides`.

## Context

ADR 0015 lets a clone read outline, style and opacity live from the root of its chain, and
`0049` keeps only `clone_link = { origin, matrix }` on the node. The customer (2026-10-14) wants
two things a clone cannot do: stay where it is when its original is transformed, and carry a
complete style of its own, opacity included (one thread colour per embroidery tab). A third wish,
a rotation pivot of its own, was dropped by the customer the same day (a miscommunication); no
pivot exists in this ADR.

Constraint that shapes everything: a transform of an original is **baked into its anchors**
(`0049` decision 6). The original does not remember how it was moved, so a clone cannot be
"absolute" and no stored point in the original's frame survives a transform of the original.

### Options: own placement

- **A. A flag; the transform writer holds the clone back (chosen).** The matrix stays relative.
  When a command maps the original's outline by T, a flagged clone gets `M' = M·T⁻¹` (its drawn
  outline is unchanged) and its followers see no change.
- **B. An absolute matrix instead of a relative one.** Rejected: the original's anchors already
  carry the move, so an absolute matrix would draw the clone at the wrong place.
- **C. Keep the original's transform history.** Rejected: contradicts baking, and a merge of two
  peers' histories has no defined order.

### Options: own style

- **A. The ordinary style keys and the existing `opacity` register on the clone node, plus a
  marker `own_style` (chosen).** The reader and writer of paths are reused; edits merge key by
  key like a path's.
- **B. One atomic `own_style` value.** Rejected: two peers editing fill and stroke of one clone
  lose one edit, and every style writer (panel, eyedropper) needs a second code path.
- **C. Per-property overrides.** Rejected by ADR 0015 (a cascade) and by the customer ("always
  the whole style").

### Options: what an "Original's" clone in a chain reads

- **A. The nearest clone on the chain that has own style, else the root (chosen, product-owner
  decision 5).** B → A → O with A blue: B is blue. A new clone of a coloured clone looks like it.
- **B. Always the root's style.** Simpler repair (no copy on removal), but Linked clone of a blue
  tab gives a tab in the root's colour. Offered to the customer as Frage 5.

## Decision

1. **Stored form (protect this).** Two optional registers on a clone node's meta map, both absent
   by default, written as plain booleans, never as containers:
   - `own_placement`: bool, absent = false.
   - `own_style`: bool, absent = false. When true, the ordinary style keys of the node (the schema
     of `style_codec`, absent key = default, so an own style is complete by construction) and the
     node's `opacity` register (ADR 0015 §2, number in 0..=1, absent = 1) are the clone's style
     and opacity. Switching to Own writes the marker, all style keys and `opacity` (only when it
     is not 1) in one commit; switching back removes marker, keys and `opacity` in one commit.

   They are **not** part of `clone_link`. That value stays `{ origin, matrix }`, because only an
   origin and the matrix computed for it need to be coherent (ADR 0015 §3). A flag has no
   meaning relative to a matrix, and a toggle must not contend with the maker's move of the same
   clone, which writes the link whole.
2. **Held-back rule.** One pure function in `curvyo-document-core` (the `clone_matrices_after` of
   `0049` adrs decision 4) decides every write to the clones downstream of a transformed set.
   For an unselected clone Y with original Z: if `own_placement`, then `M_Y' = M_Y·T_Z⁻¹` and
   `T_Y = identity`, else `T_Y = M_Y·T_Z·M_Y⁻¹` as before (a following clone is not written).
   Selected clones keep the `0049` rule whatever the flag. The write is the whole `clone_link`
   with the origin read. When `|det T_Z| < 1e-9` the held-back clone is unlinked first, in the
   same commit, with the outline it had (it keeps its place, which is what the flag promised);
   when `M_Y'` or the composed matrix would pass ±1e7, nothing is written (`0049` criterion 17).
   Node edits, Join and Split never go through this function, so they flow to flagged clones.
   Document resize and fit-to-content transform every object, so every clone is in the set and
   the flag changes nothing there. Cost: a transform of an original writes one `clone_link` per
   downstream clone **with the flag**, in the same commit; other clones are still not written.
3. **Style and opacity resolution (extends ADR 0015 §3 and §2).** A clone still reads its
   outline(s) from the root. Define its **style source** S(c): the nearest node on its chain,
   c itself first, whose `own_style` is true, else the root. The clone reads its `Style` from S(c)
   and its object opacity from S(c)'s `opacity` register (absent = 1). Its **effective opacity**
   is that register times the `opacity` of the clone's own ancestors (groups and layers above c).
   This is the rule of ADR 0015 §3 with "root" replaced by "S(c)": the ancestors of S(c) and of
   the clones in between never count. Consequences:
   - A clone with its own style: its own register times its own ancestors. The original's
     opacity, and later edits of it, do not reach it. Layer opacity above the clone applies as
     for any node.
   - Chain B → A → O, A own with 0.5, B without: B is 0.5 times B's own ancestors; the layer
     opacity above A does not reach B. B own: B's register only.
   - Paint alpha stays fill or stroke paint opacity times effective opacity (`0050` criterion 27).
   - An `opacity` register on a clone without `own_style` is ignored (merge residue, §5). The
     `0050` Opacity field writes only to clones with `own_style`.
   - The one read pass of ADR 0015 §6 memoises S(c) per clone next to `composed`; no consumer
     walks the chain. For the shared root tessellation of `0049` adrs decision 3, the sharing
     test compares only the style fields that change geometry (width, cap, join, miter, dash,
     markers) and writes colours and opacity into the mapped vertices; otherwise a row of
     differently coloured clones would lose the sharing and the frame budget of `0049`
     criterion 44.
4. **Repair and Unlink (extends ADR 0015 §5).** When re-pointing past removed clones takes away
   the style source of a re-pointed clone that has no `own_style` (S(c) was removed), that clone
   gets a copy of the style it showed (marker, style keys and `opacity` of S(c)), in the same
   commit, so its look does not change. If S(c) stays, nothing is copied. A re-pointed clone keeps
   `own_placement` and its own style as they are, because its drawn outline is unchanged by a
   re-point. Unlink writes the path's style keys and `opacity` from S(c) (for a clone with own
   style, its own), `opacity` being the register value and not multiplied by ancestors: the path
   takes the clone's place under the same ancestors, so the picture does not change (a change to
   `0049` criterion 28, which copied the root's). It removes `own_placement` and `own_style`.
5. **Merge (ADR 0009 §3, last writer wins per register) and residue (extends ADR 0015 §4).**
   - Switch to Own with a concurrent edit of the original's style or opacity: the clone has the
     copy taken at the switch.
   - Back to Original's with a concurrent edit of one own-style key or of `opacity`: the marker
     is gone, the stray values are ignored.
   - Own placement toggled with a concurrent transform of the original: the clone may move with
     it once; nothing is lost or orphaned.
   - Unlink with a concurrent override edit: the node is a path; `own_placement` and `own_style`
     on it are ignored. If both peers write style keys or `opacity`, each key keeps one writer's
     value; a complete style either way.
   - Two peers hold back one clone for different transforms: one matrix wins whole, as for any
     contested link (ADR 0015 consequences).
   - Readers also tolerate: override registers on a node that is not a clone; style keys and
     `opacity` on a clone without `own_style`; `own_style` true with style keys missing
     (defaults). Damaged is only what no writer of ours produces: a flag that is not a bool, an
     `opacity` outside 0..=1 on any node (ADR 0015 §4), a style value that would be damaged on a
     path (checked only when `own_style` is true).
6. **Format and older builds.** `format_version` goes to the next free number at merge (14 after
   `0049` has taken 13). Every file the new build writes carries it, with or without overrides
   (as for every bump), and ADR 0015 §7 applies: an older build refuses it as "saved by a newer
   version". The residue rules would let it open one with overrides, but it would draw an own
   style in the original's colours (wrong thread colours in an embroidery job) and would edit a
   flagged clone without holding it back. Files of the previous version open unchanged and are
   not rewritten. Rejected: no bump because the keys are additive; and a per-file minimum version
   written only when an override exists, which is a new policy for ADR 0004 §9 with one user.
7. **History.** The registers are ordinary fields. Undo, object undo and Restore bring them back
   field by field (ADR 0014 §1); `clone_link` keeps its live-origin check, the new registers need
   none. No change to the restore engine.
8. **Still needs its own ADR:** named styles, per-property overrides, a relink command, any
   pivot or origin of a clone.

## Consequences

- ADR 0015 §3 ("a clone has no `opacity` of its own") and the matching text of `0049`
  criterion 49 and `0050` criteria 24, 26 and 28 are amended as listed in `0057`.
- Every transform writer must use the one function of §2; a writer that skips it breaks
  the flag silently. A test lists the writers (move, resize, rotate, skew, flip, group transform,
  document resize, fit-to-content) and runs each against a flagged clone.
- The rule "a transform writes only the clones it moves" now has one exception, bounded by the
  clones that carry the flag: held-back matrices.
- A clone with own style and a chain behind it makes the look of the followers depend on a middle
  node; deleting that node costs a style copy (§4).
- Node edits flow through own placement, including a Node tool drag of every node. The flag
  holds against transforms of the object, not against edits of its shape.
- Object Restore of an original to an older version flows through as a shape edit.
- Held-back matrices accumulate float error per transform; the criterion tolerance is 1e-6 mm.

## Verification

Tests in `curvyo-document-core` (and `ui-core` for the gesture):

- held-back clone: outline unchanged after move, rotate, scale, skew and flip of the original;
  chain cases (a) to (c) of `0057` criterion 5 with their matrices;
- singular T unlinks the flagged clone with its outline, style and opacity; ±1e7 refuses all;
- S(c) over B → A → O; effective opacity for the cases of §3 (own, chain with a layer above A,
  layer above the clone, `0057` criterion 16); delete A copies style and opacity onto B; Unlink
  writes S(c)'s values; chain of 10,000 without recursion;
- two replicas: switch Own versus edit of the original; Back versus key or opacity edit; Unlink versus
  override; equal state after merge;
- every residue of §5 reads without error, every damage case is refused;
- golden file `clone_overrides_v<N>.curvyo`; a file of the previous version opens unchanged;
- 10,000 clones with own style: read at most 300 ms, frame at most 150 ms with colour-only
  differences.

## Questions for the customer

Accept or change this ADR (document model, file format). One behaviour question, **Frage 5
(CHAIN-STYLE)**: A (default, recommended) a clone without own style shows the style of the nearest
coloured clone on its chain; B it always shows the root's. Default if unanswered: A.
