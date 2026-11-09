# ADR 0045: Clones of groups, texts and images, and movable placed pages

**Status:** Proposed 2026-11-09, `needs-customer` (document model and file format; accepted with
the `0100` PR, as ADR 0042 and 0043 were). Amends, by notes on their Status lines:
[ADR 0015](0015-node-references-and-ancestor-resolved-state.md) §1, §3, §5, §6 (wider origins,
no cycle through containment, repair as a copy, the read of a drawn subtree);
[ADR 0016](0016-clone-overrides.md) §1 to §4 (own placement for every root, own style only for
outline roots); [ADR 0017](0017-image-nodes-and-embedded-assets.md) "Clones" (an image may be an
original); [ADR 0032](0032-workbench-model-and-registry.md) §6, §9 and
[ADR 0042](0042-page-kinds.md) Consequences (a placed page moves, is copied and stays on its own
laser page). Spec: `specs/0100-bring-content-in` (criteria 1 to 21). Feature notes:
[`0100` adrs.md](../../specs/0100-bring-content-in/adrs.md).

## Context

The maker drags a design onto a laser page and gets a live clone with its own place (Frage 150 A):
a path, a primitive, a text, an image, a whole group, or a whole drawing page. Today a
`clone_link` origin must be a path, compound path, sketch or clone (ADR 0015 §3); anything else
reads as an orphan and the repair after Open deletes it. A clone's read result is one
`PathSnapshot` (`CloneSnapshot::resolved`). A placement (ADR 0032 §6) already stores a full
matrix in `placement_link`, but only Delete writes it.

What must stay cheap to change later: the stored shape of `clone_link` (`{ origin, matrix }`,
unchanged here) and the rule for which origins are legal (widened here, never narrowed again
without losing files).

### Options: what a clone of a group is

- **A. One clone node whose origin is the group; the read draws the group's drawn subtree live
  under the clone's matrix (chosen).** One write per drag; a path drawn into the group or a member
  moved out shows at once (`0100` criterion 7); delete and undo work on one node.
- **B. A new group holding one clone per member.** Reuses path clones, but a member added to the
  original later is missing and a member moved out stays (fails criterion 7 d, e), and a drag of
  a 200-path group writes 201 nodes. Rejected.
- **C. A copy refreshed on every edit of the group.** Copy-on-write, rejected by ADR 0015 for
  the same reasons (write amplification, peers disagree after a merge).

### Options: what the read hands consumers

- **A. A clone of a group, text or image carries its drawn leaves as ordinary snapshots, already
  mapped (chosen).** Render, hit test, bounds and export reuse their per-kind code; no consumer
  walks the group (ADR 0015 §6).
- **B. The clone carries `(root group, matrix)` and each consumer expands it.** Five consumers
  would each walk the subtree and follow inner clones: the "one read" rule broken five times.
  Rejected.
- **C. Flatten group clones into synthetic leaves in the object list.** Synthetic ids leak into
  selection, hit results and history. Rejected.

### Options: a clone inside its own original (criterion 12)

- **A. Refuse the command that would build it; a file or merge that holds one makes the clones
  on the cycle orphans (chosen).** Local edits never produce one; the reader stays total.
- **B. Allow it and draw the nested copy to a fixed depth.** Pictures that depend on a cut-off,
  exponential cost, and a laser job that engraves a fractal by accident. Rejected.

### Options: placed page off its laser page (criterion 18)

- **A. A placement belongs to its manufacturing by ownership; a transform that leaves no overlap
  with its laser page's frame is refused (chosen).** Matches ADR 0042's ownership rule; no
  re-parenting between manufacturings.
- **B. Move the placement to the laser page under the pointer.** A reparent across
  manufacturings with page-local coordinates converted, for a case the spec puts out of scope.
  Rejected for now; a later story can add it without a format change.

## Decision

1. **Legal origins** (amends ADR 0015 §3, ADR 0017). A `clone_link` origin is a path, compound
   path, primitive, sketch, text (also a text on a carrier or a `text_source` text clone, read
   resolved), image, group, or clone. A layer, a dimension, a manufacturing, a placement or a
   fixture is "a node of another kind" (orphan, as before). The **root** is the first non-clone
   on the chain; **outline roots** are path, compound path, primitive and sketch; **picture
   roots** are group, text and image. Depth counts clone links only (unchanged); nesting through
   groups adds no depth. This is still the one `clone_link` way of ADR 0015 §1, not a new kind
   of reference. `clone_link`'s stored value is unchanged.
2. **Drawn subtree of a group G** (the "what it draws" of criterion 9). The leaves under G in
   stacking order, minus: a leaf hidden by its own flag or by an ancestor strictly below G; a
   guide (own flag, or a clone's root's, ADR 0019); a dimension; an orphan. G's own `visible`
   and everything above G do not count (ADR 0015 §3: a hidden original feeds its clones). A clone
   inside G contributes its own drawn result. Each drawn leaf keeps its own style; its opacity is
   its effective opacity counted from the leaf up to and including G, times the opacity of the
   clone's own ancestors. A text or image root draws itself (one leaf), with the opacity rule of
   ADR 0015 §3.
3. **Read result** (amends ADR 0015 §6; `0049` adrs decision 3). `CloneSnapshot` replaces
   `resolved: PathSnapshot` with `drawing: CloneDrawing`, an enum: `Outline(PathSnapshot)` for
   outline roots (today's value, the shared root tessellation unchanged) and
   `Picture(Arc<[ObjectSnapshot]>)` for picture roots, the drawn leaves already mapped by the
   composed matrix: a path or primitive member as a mapped `PathSnapshot` (a primitive is never
   written, only drawn as its outline), a text or image member with its own affine composed. The
   one pass memoises each group's drawn list once per version (DFS order makes a group's leaves
   one contiguous range of `leaves_under`) and maps it once per clone: cost O(nodes + clones +
   drawn leaves of clones), one outline transform per (clone, drawn path leaf), each member's
   tessellation shared across clones as the root's is today. `Document::object(id)` on a clone of
   a group reads that group's range and the chains of the clones in it, and still equals the full
   read's entry.
4. **No cycle through containment** (amends ADR 0015 §3 "No cycle by construction").
   Dependency edges: clone → origin, group → each child. ADR 0015's argument still holds for
   links alone, but a move can now put a clone inside what it draws.
   - *Reader.* A clone on a cycle of these edges (a non-trivial strongly connected component,
     found iteratively) is an orphan, and so is every clone whose chain reaches an orphan. A group
     with an orphan inside still resolves, without it. This replaces "meets a node twice" and is
     a pure function of the state, so peers repair the same nodes (§5 of ADR 0015 unchanged).
   - *Writers.* One check in `curvyo-document-core`, `would_cycle(moved, into)`, is true when
     `into` or one of its ancestors is reachable over the edges from a node in a moved subtree.
     Every command that reparents nodes into a group (the Layers drag, any later "move into
     group") and tiling apply, which writes instances into its group, call it and return
     `CloneError::InsideOwnOriginal`, writing nothing. Moving into a layer, Group, Ungroup,
     Duplicate, the Ctrl copy and creating a clone beside its original can never close a cycle
     (the new parent is a layer, a new group, or the parent of an acyclic node) and are not
     checked.
   - Undo, object undo and merges can still build one; it reads as orphans, and the repair after
     Open and merge deletes them (technical debt: a concurrent pair of legal moves loses those
     clones in a shared room).
5. **Unlink and removal repair as a copy** (amends ADR 0015 §5, ADR 0016 §4). For a picture root
   or a primitive root, Unlink, and the repair that ends at a removed root, run the existing
   copy of the root's drawn subtree (the `copy_map` rule of ADR 0015 §3: a copied clone whose
   original was copied too follows the copy, else it follows the same original) and then the
   existing transform writer with the composed matrix, in the clone's place, in the same commit.
   The transform writer's own rules apply (a skewed text or primitive is converted as `0047`
   does). The repair copies the subtree as it was before the removal. Outline roots other than
   primitives keep today's Unlink into a path. No change to the restore engine.
6. **Own placement and own style** (amends ADR 0016 §1 to §3). `own_placement` works for every
   root. For a group original, the held-back function of ADR 0016 §2 counts G as transformed by
   T when the command maps every node of G's subtree by the same T (a transform of G or of a
   container above it); a transform of some members is an edit and flows into the clones
   (criterion 7). `own_style` is offered and read only for outline roots; on a clone with a
   picture root, the marker and style keys are residue and ignored (ADR 0016 §5), and its style
   source is the members' own styles.
7. **Placed pages move** (amends ADR 0032 §6, §9, ADR 0042 Consequences). A placement is a
   selectable object of its laser page with the box "source frame under its matrix". Every
   transform writer of `curvyo-document-core` accepts a placement id and writes its
   `placement_link` whole: the global T conjugated into page-local coordinates, no clone holding.
   A result whose box has no overlap with its own page's frame refuses the whole command
   (`ManufacturingError::OffPage`). Duplicate and the Ctrl copy write a new placement node under
   the same manufacturing with the same link (then the move). `place_page(output page, source
   page, matrix)` is the one-commit drop and Place page command. No cycle: placements still point
   only at non-output pages and nothing references a placement.
8. **Export** (extends `0024` criteria 59 and 130 and ADR 0043 §7). A clone with a picture root
   is written as one `<g>` (its id, name and state) holding its drawn leaves, each written as the
   export writes that kind with the composed matrix as the extra transform the leaf writers
   already take (paths mapped, strokes unscaled; texts and images with `transform`). It is never
   a `<use>`. A placement is one `<g>` holding its page's members under the placement's global
   matrix; fixtures, the origin marker, the placement box and the work-area text are never
   written. Active page only takes a laser page's placements and own items from the ADR 0043 §1
   membership read. Goldens in `curvyo-svg-core/tests/fixtures/export/` (criterion 20 a, b, d).
9. **Format 38** (ADR 0004 §9; next free at merge, main is at 37). No register is added; the
   change is the set of legal origins. A format-37 build would read every clone of a group,
   text, image or primitive as a wrong-kind orphan and its repair after Open would delete them,
   and the deletion would be saved: silent loss. So a format-38 file is refused there as "saved
   by a newer version". A format-37 file opens unchanged and is not rewritten (a format-37 file
   cannot hold a live picture-root clone: its repair deleted any). Golden
   `curvyo-document-core/tests/fixtures/bring_content_in_v38.curvyo` (criterion 21); the v37
   goldens stay.
10. **Crates.** No new crate, no new dependency. `document-core`: origins, `CloneDrawing`, the
    SCC orphan rule, `would_cycle`, Unlink and repair as a copy, `place_linked_clones`,
    `place_page`, placement transforms and copies. `ui-core`: export, bounds and hit of picture
    clones and placements. `render-core`: draws `Picture` leaves through its existing per-kind
    paths. `editor-wasm`: the drags (`0100` adrs.md notes 3 and 4).

## Consequences

- Every `match` on `CloneSnapshot::resolved` changes (about 120 places matched
  `ObjectSnapshot::Clone` when ADR 0017 counted them); the compiler finds them. Commands that need
  one outline treat a picture-root clone as they treat its root's kind (`0100` adrs.md note 5).
- Nested group clones multiply: a clone of a group holding clones of a group draws the product.
  That is what the maker built; the cost is linear in what is drawn, never exponential beyond it.
- Moving the members of a group now writes no clone of the group, but transforming the group
  writes one `clone_link` per own-placement clone of it, as for a path original.
- A clone may now land inside a group only after the cycle check; the check walks the moved
  nodes' dependencies, O(dependencies), once per command.
- Not decided here: own style for picture roots, clones of layers, placed pages on drawing pages,
  moving a placement to another laser page, slots (`0102`), what edit tools do outside the active
  page's context (`0106`).

## Verification

Tests in `curvyo-document-core`: a group clone draws its members under the matrix within 1e-9 mm
and follows criterion 7 (a) to (f); hidden members, guides and dimensions are left out, G's own
flag is not; the two-group cycle and the self-containment both orphan and repair identically on
two replicas; `would_cycle` refuses both, accepts a clone of a member placed inside G; Unlink and
delete of G give copies drawing the same outlines within 0.01 mm, an inner clone following the
copied member; held-back clone of G under move, rotate, scale, skew and flip of G and of its
layer; a member transform flows; 100 clones of a 100-path group read within 300 ms with one
transform per (clone, leaf); placements move, refuse off-page, copy; the golden of §9; a
format-38 file refused by the format-37 check.
