# ADRs for Layers and objects panel

Architect review, 2026-10-10, revised 2026-10-11 after the customer's decisions (ADR 0015
accepted, output of hidden nodes left open, clone chains in `0049`). The spec needs **one new
ADR, [0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md)**, shared with
`0049` and accepted by the customer on 2026-10-11. It is not an in-place amendment of ADR 0002 §5, for
the reason given in 0015's preamble: 0002 is accepted, so changing it takes a new ADR. Layers
need no other ADR: a layer is the group node of `0023` with one more register. No new crate, no
new dependency, no trait, no generic.

## Depends on

- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §2, §4, §6
  (Accepted): the node registers `name`, `visible`, `locked` and `opacity`, the effective values
  resolved through the ancestors, tolerance of merge residue, and the single read pass.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  - §5: sibling order is z-order, and reparenting is a tree move that keeps the `NodeId`.
  - §8: nodes are assigned to layers. The job assignment of a layer is `0027`.
- [`specs/0023-groups/adrs.md`](../0023-groups/adrs.md): the group node, the two tree functions
  (children of a context, leaf descendants), `remove_emptied_groups` and the depth limit of 32.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md):
  - §2: the active layer, the expansion state, the panel target and the scroll position are
    ephemeral.
  - §3: each register is last writer wins.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) §1 and §11: every action is one
  step, and Delete layer is revived by one injected move (decision 8).
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: format bump.
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md): opacity is multiplied per
  paint. The cost of compositing a group as a whole is in ADR 0015's Consequences.

## Feature-local decisions

1. **2026-10-10: stored shape (format, protect this).**
   - A layer is a `0023` group node (`group: true`) with one more register, `layer: true`, at the
     top level of the objects tree.
   - Any node may carry `name` (string), `visible` (bool), `locked` (bool) and `opacity` (`f64`
     in 0..=1). An absent key reads as no name, visible, unlocked and 1, so files from earlier
     builds open unchanged. The keys do not clash with existing ones (`fill_opacity` and
     `stroke_opacity` are style; `id` and `kind` live in anchor maps).
   - A name is cleaned when it is written: trimmed, cut to 128 characters, control characters
     and bidi controls removed. A name read from a file is cleaned for display only, and the
     file is left as it is (criterion 17).
   - A file is damaged if a register has the wrong type, if `opacity` is not finite or lies
     outside 0..=1, or if `layer` is set on a node that is not a group.
   - A `layer` marker below the top level is read as a plain group (merge residue, ADR 0015 §4).
2. **2026-10-10: `remove_emptied_groups` skips layers.** This amends `0023`'s decision and
   criterion 14: a new layer is empty, and a maker empties a layer on purpose. The test that
   lists the removers gets a layer case.
3. **2026-10-10: the read becomes one pre-order pass of nodes.** The object cache of `0044`
   (`object_cache.rs`, one read per document version) becomes one pre-order list of node reads.
   Each node read holds:
   - its id, parent and depth;
   - its kind (layer, group or leaf with its `ObjectSnapshot`);
   - its own registers;
   - its effective visible, effective locked and effective opacity (for a `0049` clone: the
     **root** original's `opacity` register times the clone's own ancestors', ADR 0015 §3 and
     criterion 28; the ancestors of the root and of middle clones do not count; the root is read
     in the same pass, so this costs nothing extra);
   - the nearest ancestor that hides or locks it, which the inherited-state description of
     criterion 22 needs.

   The walk carries the ancestor values down, so it is O(n) with no per-node ancestor walk.
   Hidden leaves are still resolved, because `0049` clones need hidden origins. The renderer and
   the hit test skip them by flag. A layer toggle is one register write and one re-read. The
   150 ms of criterion 50 is therefore the cost of re-reading 10,000 objects, the same as after
   any other edit today. The slice measures it. If it fails, an incremental cache is a separate
   decision and is not built ahead.
4. **2026-10-10: consumers read the flags; none walks the tree.** Render, hit test, marquee,
   lasso, Alt-click cycle, Ctrl+A, eyedropper, Fit to content, status measures, the Node tool and
   the Pen's continue and connect all read the effective flags from the read. One test lists them
   (criterion 19). `0023`'s "children of a context" function, called for the document top level,
   returns the loose objects and the direct children of every layer in stacking order, without
   hidden or locked nodes. This is the one place that makes layers transparent (criterion 42).
   `ui-core` selection and `hit_test_object.rs` write no walker.
5. **2026-10-10: the selection invariant (criterion 14).** After every change of the document
   version (a local commit, an import or an undo), the session drops from the selection every node
   that is effectively hidden or locked. It does this in the same pass that applies `0023`'s
   context fallback, before the frame is drawn. The cost is O(selection) set lookups in the read.
6. **2026-10-10, changed 2026-10-11: no output rule and no `output_objects()` (criteria 24,
   54).** The customer left open what a hidden or locked node means for output (ADR 0015 §2), so
   this slice writes no rule about it. No exporter or job generator exists; a core function whose
   only callers are tests is the speculative API that `CLAUDE.md` §5 rules out. The read carries
   every leaf with its effective flags, and the first exporter or job generator (`0024`, `0029`)
   decides with the customer how to use them. The test of criterion 54 names the two writers
   that exist (`export_loro_snapshot`, `export_json`) and checks that they carry hidden and
   locked nodes with their registers.
7. **2026-10-10: panel rows are a pure window over a cached flattening (criteria 48 to 52).**
   - `layer_rows(read, collapsed, selection)` in `curvyo-ui-core` flattens the visible rows once
     per document version and collapse change, and the session caches the result.
   - `rows_window(first, count)` slices that cache in O(count). Only the window crosses the wasm
     boundary.
   - Each row carries:
     - id, depth, kind, and the name or default name;
     - own and effective flags, with the ancestor that causes an inherited state;
     - opacity;
     - "contains the selection";
     - `aria-posinset` and `aria-setsize`.
   - The total row count comes from the same cache, so the scrollbar is exact.
8. **2026-10-10: moving and deleting nodes.**
   - **Move rules.** One pure function in `curvyo-ui-core`, `plan_move(read, moved, target) ->
     Result<MovePlan, MoveRefusal>`, serves the drag, the Alt+arrow keys and the canvas z-order
     keys. It refuses:
     - a move into the node itself or one of its descendants;
     - a move into a locked container;
     - a move of a locked node;
     - a layer anywhere but the top level;
     - a depth above 32.
   - **The move command.** `Document::move_nodes(plan)` (label `move_nodes`) is one commit of
     tree `mov` / `mov_after` / `mov_before`, which keeps ids, registers and clone links
     (criterion 31). It checks the same rules again before the first write.
   - **Delete layer** (label `delete_layer`) does three things in one commit:
     - It calls `0049`'s `repair_clones_of_removed` for the layer's subtree, once `0049` exists:
       clones outside the layer that follow something in it are unlinked, or re-pointed to the
       next original that stays when a middle clone of a chain goes with the layer (criterion
       36).
     - It deletes the layer node itself in **one** tree delete. Loro keeps the subtree under the
       deleted node, so the injected move of ADR 0014 §11 revives the layer with every
       descendant and every id (criterion 45).
     - It does **not** delete each descendant on its own. That would turn one undo into N
       revives.
   - **2026-10-12 (amends decisions 8 and 14).** The rules of criterion 30 live once in
     `curvyo-document-core` `Document::check_move`; `move_nodes` runs it before the first write.
     ui-core `reorder_plans` (`layer_move.rs`) only turns a gesture into `MovePlan`s, each asked
     of `check_move`, and `move_refusal_text` maps `MoveRefusal` to the texts of criterion 30. `parent_group`, `ancestor_groups` and
     `child_groups(None)` treat layers as the top level (criterion 42); `parent_node` and
     `children` give the raw tree. Layers count toward the 32-level limit of `0023` criterion 5,
     in the reader and in every writer alike, so a layer holds at most 31 nested groups. Group,
     Ungroup and Delete of objects refuse a layer (`ObjectEditError::IsALayer`); a layer goes
     only by `delete_layer`, which checks its lock.
9. **2026-10-10: Delete layer never confirms (criterion 36), and the Pen gate (criterion
   53).** Delete layer is reversible by Ctrl+Z (decision 8), so it writes at once; there is no
    confirm state in the session. Every panel action that writes or selects passes the
    "Pen unfinished" check that `0020` already has for undo and redo (its notice table, the 2 s
    hint). The check sits at the one place in `curvyo-editor-wasm` where panel actions enter the
    session, not in each handler, and it runs before any write. Scrolling, expanding and
    collapsing do not pass through it. One test per action listed in criterion 53, done as a
    table.
10. **2026-10-10: opacity per paint (criterion 27, Question 5 A).**
   - The render change is in `curvyo-render-core` `node_opacity.rs` (`faded`, `drawn_leaves`,
     and `drawn_leaves_with` for the Opacity preview; corrected 2026-10-12): each paint's alpha is
     multiplied by the effective opacity.
   - The cost of option B is in ADR 0015's Consequences: one more render pass and texture per
     faded container per frame.
11. **2026-10-10: the tree form of `document.json` (decided once for groups and layers).**
    `document.json` is the non-authoritative view of ADR 0004 §1.
    - Each node is written as `{ "id", "kind": "layer" | "group" | <leaf fields as today>,
      "name"?, "visible"?, "locked"?, "opacity"?, "children"? }`.
    - Registers at their defaults are omitted.
    - `0023` did not fix this form, so the first of `0023` and `0050` to merge writes it. `0023`
      leaves out the registers.
    - **Superseded 2026-10-12 (implementer, by the lead's decision):** `0023` merged first and
      wrote groups as `{ "shape": "group", "id", "objects": [...] }`, the key its leaves already
      use (`"shape": "path" | "rect" | ...`). That form stays. A layer is written as
      `{ "shape": "layer", "id", "objects": [...] }`, and every node, layer, group or leaf, may
      carry the optional keys `"name"` (string), `"visible": false`, `"locked": true` and
      `"opacity"` (number below 1), each left out at its default. Reason: renaming `shape` to
      `kind` and `objects` to `children` would change a merged view for no reader's gain, and
      one key that names what a node is (`shape`) reads the same for leaves, groups and layers.
      `document.json` is non-authoritative (ADR 0004 §1), so no format version depends on it.
12. **2026-10-10: duplicate.** `copy_map` copies every key, so a copy keeps its name and flags
    (the product owner's default: the same name). Copied clones are relinked by `0049` decision 8, chains included.
13. **2026-10-10: concurrency.**
    - Each register is last writer wins.
    - If one peer hides a layer while another moves an object into it, the object is hidden.
    - If one peer locks a node that another peer has selected, the other peer's next read drops
      the node from its selection.
    - Two peers who move the same node: one move wins (Loro), and no cycle can form.
    - **Question 10:** default A. B can be done reliably. The log identifies the case: the
      node's last move targets a parent whose delete does not include that move in its version
      vector. Lifting the node out needs the injected move of ADR 0014 §11, because a deleted
      node cannot be moved through the handler API. It is deferred to the sharing story
      (ADR 0015, Consequences), together with merged trees deeper than 32 levels.
14. **2026-10-10: crate placement.**
    - `curvyo-document-core`: the codec and validation, the registers, the read pass with
      effective values, `move_nodes`, `new_layer`, `delete_layer`, the setters
      (`set_node_visible`, `set_node_locked`, `set_node_opacity`, `rename_node`; each takes many
      ids and makes one commit; `set_node_opacity` skips clones, criterion 26), and the format bump.
    - `curvyo-ui-core`: `layer_rows`, `layer_move` (`reorder_plans`, `move_refusal_text`;
      corrected 2026-10-12), `layer_text`, the selection filter and the transparent context.
    - `curvyo-render-core`: alpha.
    - `curvyo-editor-wasm`: the row window, the commands and the key gate (Shift+Ctrl+L, Alt+arrows
      and the Proposal keys of criterion 33).
    - `frontend`: the tab.
15. **2026-10-10: format version.**
    - The next free number at merge: 12 if `0023` takes 11.
    - An older build checks the container's `format_version` per file, so it refuses **every**
      file saved by this build, with or without layers. A file from an earlier build opens
      unchanged and is not rewritten on open.
    - Golden files:
      - `layers_v<N>.curvyo`: a layer holding a group, a hidden node, a locked node, opacities,
        names with markup, and a loose object;
      - one damaged file (opacity 1.5);
      - one file with a `layer` marker inside a group, which reads as a plain group.

## Shared files and order

Built after `0023`, and after `0043` for the tab strip. It runs alone among `0023`, `0049` and
`0050`, because all three touch the read pass, the selection context and the panel files. `0044`'s
Ctrl+A and `0015`'s Fit to content are amended in this slice's PR. Whichever of `0020` and this
spec merges second adds the operation names of criterion 44 to `0020`'s table.

## Flagged for the product owner (defaults taken, no customer question)

Resolved in the spec fixes of 2026-10-10 and 2026-10-11: criteria 24, 26, 27, 28, 36 and 46. One
point remains:

1. **Criterion 47 and `0023` criterion 27.** "Deeper than 32 is damaged" stays for now. Under
   sharing, two legal moves can merge into 33 levels. The sharing story relaxes the reader, which
   needs no change to any file.
