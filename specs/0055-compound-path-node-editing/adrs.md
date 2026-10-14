# ADRs for "Compound path node editing"

Architect review, 2026-10-12. The stored form already names every node of a compound path as
`(NodeId, AnchorId)` without an outline index (`subpath_codec.rs`: anchor ids are unique, minted
globally, `anchor_id_minter.rs`), and the first outline (`closed`, `anchors` in the path's meta map)
and each `extra_subpaths` element (`closed`, `anchors` in its own map) use the same two keys. So the
slice is mostly one change: every anchor-level command finds **the outline that holds the anchor**
instead of assuming the first one. **No new ADR, no new crate, no new dependency, no trait, no
generic, no `format_version` bump.**

## Depends on

- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §5, §6, §9: nodes are
  addressed by anchor id; outlines are open or closed runs of cubics; every edit is one command
  and one commit (criterion 10).
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §2, §3: anchors are a movable list
  of maps, `closed` a last-writer-wins register; the node selection is ephemeral. Edits that move
  anchors **between** lists (join of two outlines, an outline moving to another object, the first
  outline replaced, see below) are delete-and-insert, so a peer's concurrent edit of a moved
  anchor is lost. That is today's behaviour of `0006` Join across paths and of `0035` Combine; no
  new merge rule.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md), [`0020` adrs.md](../0020-undo-redo/adrs.md):
  one commit per gesture, existing labels (`move_anchors`, `delete_anchors`, `join_endpoints`,
  `split_at_anchor`, `close_path`, `extend_path`, `connect_paths`).
- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §1, §3, §5: rules in
  `curvyo-ui-core`, overlay drawing in `curvyo-render-core`, the node data over the wasm boundary in
  `curvyo-editor-wasm`; the boundary carries per-outline anchors with their `closed` flag.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: no bump (criterion 12).
- [`0054` adrs.md](../0054-combine-open-paths/adrs.md): words, nonzero fill, depth by contains
  (used nowhere here: edits never re-wind, decision 5).
- [`0002`](../0002-path-node-editing/adrs.md), [`0006`](../0006-path-merge-split-and-node-types/adrs.md),
  [`0031`](../0031-segment-drag-bending/adrs.md), [`0034`](../0034-pen-path-extension/adrs.md):
  the commands this slice extends to outlines.

## Feature-local decisions

- **2026-10-12: one outline resolver in `curvyo-document-core`.** `Document::path_parts(path)`
  returns the meta map and the first anchor list; every anchor command uses it (`paths.rs`
  `move_anchors`, `convert_anchor_kind`, `insert_anchor`, `delete_anchors`, `set_segment_line`;
  `segment_bend.rs`; `path_topology.rs` join and split; `path_extend.rs` extend, connect, close).
  Add `outline_parts(path, anchor) -> (LoroMap, LoroMovableList)`: the map that holds `closed` and
  `anchors` for the outline containing `anchor` (the meta map or the `extra_subpaths` element).
  Commands call it instead of `path_parts`; their bodies stay. A segment is two anchors of the
  **same** outline (adjacent, or last and first when closed); otherwise `NotAnAdjacentSegment`.
  No outline index in any public type and none in `NodeSelection`.
- **2026-10-12: outline removal and promotion (criterion 4).** An outline left with fewer than two
  anchors is removed: an `extra_subpaths` element is deleted from the list; if it is the first
  outline, `extra_subpaths[0]`'s anchors and `closed` are written into the first-outline keys and
  that element deleted (anchor ids kept). One outline left: the `extra_subpaths` key is deleted,
  which is exactly an ordinary path (the reader already treats absent as one outline). None left:
  the node is deleted with `remove_emptied_groups`, as today. One function in `subpath_codec.rs`
  owns this so delete, join and Pen-connect share it.
- **2026-10-12: one commit for a multi-object node edit (criteria 3, 4, 10).** `delete_anchors` and
  the join/close commands commit per call today. The multi-path Delete is `0053` point 4; whichever
  of `0053` and `0055` builds first adds a `delete_anchors_many(&[(NodeId, &[AnchorId])])` with one
  commit, and the other reuses it. Close path already takes a list (`close_paths`).
  - *Superseded 2026-10-14 by `0053` decision 4.*
    - **No `delete_anchors_many`.** `0053` built first and changed `delete_anchors` itself to
      `delete_anchors(&[(NodeId, AnchorId)])` in `paths/anchor_edits.rs`: one commit across
      paths, missing paths and stale ids skipped. `convert_anchor_kind` takes the same pairs and
      refuses on a stale id. There are no twin methods.
    - **What this slice changes.** It swaps `path_parts` for the per-anchor outline resolver in
      both commands.
    - **Removal rule.** It replaces "fewer than two anchors: remove the node" with the outline
      removal and promotion above. Today a Delete that empties the first outline of a compound
      path would remove the whole object.
- **2026-10-12: Join across objects (criterion 5 c, 9).** The survivor's joined outline is written
  where its outline stood; the other object's remaining outlines are appended to `extra_subpaths`
  with their anchor ids (globally unique, so the node selection stays valid); the other object is
  deleted. `join_two_objects` and `connect_paths` gain this append; the count for the notice is
  returned with the result.
- **2026-10-12: Split at a node (criterion 6).** An open outline splits into two outlines of the
  same object: the second part is inserted right after the first (into `extra_subpaths`, at index
  0 when the split outline is the first). A closed outline opens in place (`closed` = false,
  rotated as `0006` criterion 14). Only the duplicated anchor gets a fresh id.
- **2026-10-12: `ui-core` changes.** `node_tool.rs` (drop `compound_only` and the hint), its
  editing set and overlay data per outline; `close_path.rs` candidates become outlines with the
  selected-node rule of criterion 7; `pen_target.rs` enumerates open-outline ends of compound paths
  through the `0050` filtered read (hidden and locked skipped, no tree walker, `0050` decision 4);
  `segment_bend.rs` per outline. `render-core`: the node overlay draws each outline's closing
  segment only when closed. No new module is expected; if `node_tool.rs` passes 500 lines, the
  per-outline overlay data moves to its own module.
- **2026-10-12: performance (criterion 11).** 4004 nodes is within what the overlay draws today for
  ordinary paths; the anchor resolver must not scan all outlines per anchor in a 2000-node delete:
  build one `AnchorId -> (outline, index)` map per path per command (`anchor_positions` in
  `subpath_codec.rs` exists and does this).
- **2026-10-12: format (criterion 12).** No new key, no new value; the shapes written are those
  `0016`, `0035` and `0054` already write. No golden file needed; a save-and-reopen test of an
  edited compound path covers criterion 12.

## Interaction with other slices

- **`0053` node marquee selection:** both change `node_tool.rs`, `selection.rs` and the multi-path
  Delete commit; they must not run in parallel. Recommended order: `0053` first (smaller); its
  "document order" of anchors must iterate `PathSnapshot::all_anchors()` (every outline in order),
  not `anchors`, so it already covers compound paths when `0055` lifts the guard. `(NodeId,
  AnchorId)` pairs need no change for outlines.
- **`0050` layers panel:** the row label follows the kind live, so a compound path that drops to
  one outline reads "Path" after the commit; Pen targets use `0050`'s effective flags.
- **`0049` linked clone:** if `0049` builds first, its rule for clones in the Node tool applies to
  compound origins unchanged; if `0055` builds first, `0049` checks that the origin's outline
  edits propagate (they are ordinary anchor writes).
- **`0054`:** builds first; `0055` needs no geometry from it beyond the fill already painted.

## Flagged

- No customer question: the open choices are the PO's vetoable decisions 2 to 5 in the spec. The
  concurrency loss for anchors that move between lists is the same as today's Join and Combine.
