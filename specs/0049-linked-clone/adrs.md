# ADRs for Linked clone

Architect review 2026-10-10, revised 2026-10-11 after the customer's decisions (ADR 0015
accepted, chains allowed, clone marks on the canvas, output of hidden nodes left open). The spec
needs **one new ADR, [0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md)**,
shared with `0050` and accepted by the customer on 2026-10-11. No new crate, no new dependency, no
trait, no generic.

## Depends on

- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §3 to §6
  (Accepted): the atomic clone link, chains and their root, no cycle by construction, residue
  tolerance, repair on removal, the orphan repair and the single read pass. It supersedes the "no
  inheritance" sentence of ADR 0002 §5.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  - §5: ids, and reparenting as a tree move. The link is by id, so moves never break it.
  - §6: "object to path" is explicit; criterion 2 calls the existing conversion.
  - §9: one commit per command, with the labels `linked_clone`, `unlink_clone` and
    `remove_orphan_clones`.
  - §10: SVG writes a clone as a path (Question 7, A).
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §3: last writer wins per
  register. The link is one register.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) §1 and §11: undo restores a kind
  change field by field and revives a deleted node with its id (decision 9).
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: format bump, and a
  newer file is refused.
- [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md): no change. The
  kernel receives the resolved path.

## Feature-local decisions

1. **Stored shape (format, protect this).** *2026-10-10, changed 2026-10-11: one link register
   instead of two.* A clone is a leaf of the objects tree. Its meta map holds:
   - `shape = "clone"`;
   - `clone_link`: one `LoroValue::Map` **value** `{ "origin": <NodeId text form of 0041>,
     "matrix": [a, b, c, d, e, f] }`, written with `insert` as a whole and never with
     `insert_container`.

   Why one register: the matrix means something only relative to its origin. With two registers,
   a re-point (origin O, matrix M_B·M_A) and a concurrent or object-undone transform (matrix
   G·M_B) can merge into "origin O, matrix G·M_B", which draws B in a wrong place and looks valid,
   so no repair can find it. With one value the merge keeps one coherent pair (ADR 0015 §3).
   Every writer that changes a clone's matrix writes the whole link with the origin it read.

   A clone has no anchors, `closed`, `extra_subpaths`, style keys, `opacity` or `rotation`. The
   key does not clash with `0041`'s `clone_of`, a history key. Validation runs only when
   `shape == "clone"`: the file is damaged if `clone_link` is missing, is not a map with an
   `origin` id string and a `matrix` of six finite numbers within ±1e7, or if the clone node has
   children. Other keys on a clone, and `clone_link` on any other node, are ignored residue
   (ADR 0015 §4); `set_node_opacity` of `0050` skips clones.
2. **The read is a new variant (2026-10-10, extended 2026-10-11).**
   `ObjectSnapshot::Clone(CloneSnapshot { id, origin, root, depth, matrix, composed, resolved })`.
   `composed` is the matrix from the root's coordinates to the clone's (M_B·M_A for B → A → O).
   `resolved: PathSnapshot` holds the clone's id, the root's anchors under `composed` (point
   `p → C·p`, handle `v → L·v`) keeping the root's anchor ids, the root's `closed` and extra
   outlines, the root's `Style`, and `rotation` 0. The `0050` node read gives it the effective
   opacity of ADR 0015 §3 (root's `opacity` times the clone's own ancestors'). Readers that draw,
   hit, measure or feed the kernel reach `resolved` through their path arm.

   Rejected: a `Path` with `clone: Option<…>`. Every writer that takes a path snapshot back and
   writes anchors (`transform_objects`, `resize_path`, `rotate_object`, node edits, Join, Split)
   would silently write anchors into a clone. A new variant makes the compiler list every match
   site. The writers get `Planned::Clone`, which writes only `clone_link`. Node edits, Join, Split,
   segment bending, the Pen's continue and connect, and Object to path refuse a clone with the
   refusal pattern of `0016` and `0023`.
3. **Resolving chains, and the cost (criteria 43, 44, 50, 63).** *2026-10-11.* In the one read
   pass the object cache of `0044` makes per document version:
   - The tree walk reads every node once. Roots (paths, compound paths) are read whatever their
     flags, because a hidden root still feeds its clones (`0050` criterion 25). A clone records
     its link only.
   - Then each clone is resolved with an explicit stack and a state map (`Pending`, `OnStack`,
     `Resolved { root, depth, composed }`, `Orphan`): push clones while following `origin` until
     a node is resolved, orphaned, a root, missing, of another kind, or `OnStack` (a ring: every
     clone on the stack from that point is an orphan). Unwinding computes `composed = M ·
     composed(origin)` once per clone. A non-finite composed matrix makes the clone and everything
     behind it orphans. No recursion and no depth bound (ADR 0015 §3), so a ring or a chain of
     10,000 costs O(10,000).
   - Each resolved clone transforms the root's snapshot once with `composed`; a counter in the
     test build pins one transformation per clone (criterion 63). Chains add one 2x3 product per
     clone, so 1,250 chains of depth 8 cost what 10,000 direct clones cost.
   - The same pass builds `clones_of: NodeId → Vec<NodeId>` (direct clones in stacking order).
     Select clones, the origin line, the Layers rows, the preview count of criterion 11 (walk
     `clones_of` transitively, stop at 501) and the marks (decision 12) read it.
   - There is no cross-version cache: an edit of a root is a new version and a full re-read, as
     any edit today. A second read of the same version is the cached object list (criterion 63's
     5 ms). If criterion 43 or 44 fails, the next step is sharing the root outline and transforming
     at tessellation; measured first, not built ahead. `Document::object(id)` on one clone walks
     its chain alone.
4. **One matrix type and one rule for transforming a set (criteria 15, 16, 34).** *2026-10-10,
   generalised 2026-10-11.* `curvyo-document-core` `units.rs` gets `Affine` (compose, `inverse()
   -> Option`, `None` when |det| < 1e-9, apply to point and vector). For a command that maps
   selected objects by G, every object's outline changes by a document map T: G if selected,
   the identity if nothing in its chain moves, and for an unselected clone Y with original Z,
   `T_Y = M_Y · T_Z · M_Y⁻¹`, computed root first. A selected clone X with original Y gets
   `M_X' = G · M_X · T_Y⁻¹` (G·M_X·G⁻¹ when Y is selected, G·M_X when nothing below moves).
   Unselected clones are not written. When G, or an `M_Y` that the rule must invert, is singular,
   the selected clones of that case are unlinked first (criterion 16). One pure function,
   `clone_matrices_after(read, selected, g) -> Result<Vec<(NodeId, Affine)>, _>`, serves
   `curvyo-ui-core` (preview and result) and every `curvyo-document-core` writer that moves a set:
   `translate_objects`, `transform_objects`, `Document::resize`, `fit_to_content`, and the group
   transform of `0023`. Check of criterion 16 (c): O and B selected, A not, gives
   M_B' = translate (30, 30). The limit check of criterion 17 runs on every composed matrix and
   resolved outline downstream before the first write.
5. **Unlink and repair on removal (criteria 28 to 31, 54, 55).** *2026-10-10, changed 2026-10-11
   for chains.* Module `clones.rs` in `curvyo-document-core`.
   - `unlink_clones(ids, minted_anchor_ids)` (label `unlink_clone`) writes under the same id and
     tree position the resolved anchors (composed matrix on the root) with fresh ids, `closed`,
     `extra_subpaths`, a copy of the root's style and of the root's `opacity` register, and removes
     `shape` and `clone_link`. Clones that follow an unlinked clone are not touched: they now
     follow a path (criterion 55).
   - `repair_clones_of_removed(removed_roots)` replaces `unlink_clones_of_removed`. It runs before
     the removal, in the same commit, and reads the state before the command. For each live clone
     outside the removed set whose original is in it, it walks down the chain while the node is a
     removed clone, composing the skipped matrices: a surviving node gets a re-point (one
     `clone_link` write, `M_X · M_skipped…`); a removed root gets an unlink with the outline
     before removal. Removed sets include all descendants of a removed group or layer. Callers:
     every command that calls `0023`'s `remove_emptied_groups` (delete, last-node delete,
     `replace_with_path`, Break apart, Split, Fracture, Flatten), `0050`'s Delete layer, and
     `0020`'s restore when undo, redo, object undo or go to version removes a node (decision 9).
     One test lists the callers of both helpers.
6. **The orphan repair (criteria 35, 38, 39).** *2026-10-10, changed 2026-10-11.*
   `Document::remove_orphan_clones() -> usize` (label `remove_orphan_clones`) deletes every clone
   the read of decision 3 marks `Orphan`: a missing or wrong-kind node in its chain, a ring, or a
   non-finite composed matrix. Whether a clone is an orphan depends only on the graph, not on the
   order of the walk, so two peers that repair the same converged state delete the same set, and
   deleting a deleted node does nothing. Peers that repair different versions delete different
   sets; after their merge the clones behind those are orphans too, and the next repair takes
   them, so the rounds converge. The session calls it after Open and after `import_updates`; it
   is an ordinary history step; the notice of criterion 35 shows when the count is above 0. It
   deletes and never unlinks (criterion 35). With chains, a deleted clone's tombstone can still
   give a way to the root; using it is left to the sharing story (ADR 0015, Consequences).
7. **Merge cases (criteria 13, 53).** *2026-10-10, extended 2026-10-11.* Tests in the
   `acceptance_0005_peers.rs` style:
   - *P deletes the root O and unlinks clone C, while Q transforms C.* C is a path with P's
     outline: only P wrote `shape`. Q's link survives as ignored residue or loses the race.
   - *P deletes middle clone A of B → A → O (re-points B to O with M_B·M_A), while Q transforms B
     (link A, G·M_B).* One link wins whole. P's: B draws where it did, Q's move is lost. Q's: B
     follows the deleted A and is an orphan, which today's repair deletes (sharing story, ADR
     0015).
   - *P deletes B and Q deletes A in C → B → A → O.* C's link names the deleted A: an orphan, as
     above. Both peers deleting the same B write the same link value: no conflict.
   - *P edits root O, while Q transforms A or B.* Both apply; B follows both (criterion 13).
   - *P makes a clone of A while Q deletes A.* The new clone is an orphan (criterion 35).
   - *Two peers transform the same clone; two peers unlink the same clone.* One whole link; one
     `anchors` container with the same geometry. Replicas converge.
   - *Property test (criterion 53):* random sequences on two replicas with merges; after each
     merge and repair, no ring exists and every live clone resolves.
8. **Copying sets (criteria 19, 47, 48).** *2026-10-10, extended 2026-10-11.* `copy_map` copies
   every key, so a copied clone starts with its source's link. `relink_copied_clones(copies, g)`
   gets the map from source id to copy id that the copy command builds. A copied clone whose
   original is also copied gets the original's copy and matrix G·M·G⁻¹ (M for Duplicate); any
   other copied clone keeps its original and gets G·M. A copied chain is therefore a chain of
   copies with the same depths. The copy command creates the copies root first (sorted by depth
   in the source), so each original's copy is created before the copies that follow it, which
   keeps the no-cycle argument of ADR 0015 §3. Callers: `duplicate_objects`, `0023`'s deep
   duplicate, later duplicate layer and paste within a project. Duplicate of a clone of depth 8
   gives depth 8, so no copy exceeds the creation limit.
9. **Undo (criterion 36).** *2026-10-10, extended 2026-10-11.* ADR 0014 §1 treats an absent key
   as a value, so undo of Unlink or of a removal that unlinked clones turns them back field by
   field (the `0047` conversion is the same case). Two rules for `0020`'s restore code, which owns
   the fix if it lacks them: (a) a `clone_link` is restored only when its `origin` is live after
   the restore (else skipped and reported; the clone keeps its current original); (b) a restore
   that removes nodes calls `repair_clones_of_removed`. Undo may deepen a chain past 8; the reader
   accepts it. Tests: undo of Unlink; undo of Delete of a root with two clones; undo of Delete of
   middle clone A gives B its link (A, M_B) back; object undo of A's delete after B was moved
   keeps B on O.
10. **Crate placement.** *2026-10-10, extended 2026-10-11.*
    - `curvyo-document-core`: codec (`shape_codec` gets `clone`), validation, `clones.rs`,
      `Affine`, the read variant and the chain resolver, `clones_of`, `Planned::Clone`, the
      refusals, the `document.json` entry (`"shape": "clone"`, `"origin"`, `"matrix"`), the
      format bump.
    - `curvyo-ui-core`: availability and the depth check of criterion 52, the gesture result, the
      panel lines, Select original, root original and clones as view state, and `clone_marks`.
    - `curvyo-render-core`: the variant's arm, and the traces of decision 12.
    - `curvyo-editor-wasm`: the repair call after Open and import, the commands.
    - `frontend`: buttons, notices, tags and edge chips.
11. **Format version.** The next free number at merge (13 if `0023` takes 11 and `0050` 12). An
    older build refuses every file saved by this build. A file from an older build opens
    unchanged. Golden files: `clone_v<N>.curvyo` (a root, two clones, a chain of three, a
    compound-path root, a rectangle converted by making its clone); one damaged file (a matrix of
    five entries); one file with an orphan and a ring. The 100,000-clone, 10,000-ring and
    10,000-chain cases are built in the tests.
12. **Data for the canvas marks (criteria 57 to 62).** *2026-10-11.* No document data is added:
    the marks are view state. `clone_marks(read, selection, viewport) -> CloneMarks` in
    `curvyo-ui-core` is pure and returns, within the caps of criterion 61, trace items `(NodeId,
    role: Clone | Original | Root | FaintClone, form: Trace | Ring)`, tag rectangles placed by
    criterion 60 against the handle and reference-mark rectangles the frame already has, and edge
    chips. It needs from the read, per selected clone, `origin`, `root`, `depth` and the effective
    visible of the original (for "Clone (original hidden)"), and for the Proposal of criterion 59
    `clones_of`. The renderer draws a trace from the outline already in the read by `NodeId`; no
    outline is copied. It is recomputed on a selection, version or viewport change, not per frame,
    and is empty during a canvas drag (criterion 59).

## Shared files and order

Build after `0023` and `0050`. All three touch the object read (`object_cache.rs` and the read
pass); this spec and `0050` share the panel files, this spec and `0023` the selection context. It
runs alone among those three. `repair_clones_of_removed` is called wherever `0023`'s
`remove_emptied_groups` is.

## Flagged for the product owner

The two points of 2026-10-10 (opacity on unlink, criterion 37 wording) are fixed in the spec.
New, from the chain review of 2026-10-11 (defaults taken in the decisions above; no customer
question, the depth limit stays a creation rule):

1. **"Orphan" (Words), criteria 35, 39, 53: no depth bound when reading.** "Longer than 8 is an
   orphan" deletes clones that undo or a merge legally pushed past 8: undo of the Unlink of a
   middle clone (Ctrl+U of `0041` with a chain built behind it), undo of a re-point, a peer's
   undo concurrent with Linked clone. Criterion 53's "a chain can never grow beyond the limit by
   a merge" is therefore not true. Default: the limit 8 holds for Linked clone only (criterion
   52); a deeper chain resolves; criterion 39's chain of 10,000 resolves fully.
2. **"Orphan", the ±1e7 composed-matrix clause.** Two peers that each scale one link of a chain
   within limits can merge into a composed matrix over ±1e7, and object undo can do the same;
   deleting the clone then loses work. Default: only a non-finite composed matrix orphans;
   criterion 17's limits keep applying to edits, and a command taking an over-range clone as an
   operand applies its own limits.
3. **Criterion 36 misses two undo cases.** Undo of a Delete whose removed clone cannot be revived
   keeps the re-pointed clones on their new original (decision 9 a); an undo, redo or object
   undo that removes a clone or root repairs the clones behind it as criterion 31 says.
4. **Criterion 13**: "the matrix is one atomic value" should read "the original and the matrix
   are one atomic value" (decision 1).
5. **Key names.** The Links line of `specification.md` and `specs/README.md` (two table rows) name
   `clone_origin` and `clone_matrix`; the key is now `clone_link` (decision 1). The "Open for the
   architect" sentence of the Links line is closed by this revision.
