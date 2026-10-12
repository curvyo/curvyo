# ADRs for "Node marquee selection: drag a box in the Node tool to select many nodes"

First list of points by the product owner (2026-10-12), for the architect to confirm or replace with decisions. **New ADR expected: none.** No new crate, no new dependency, no stored field and no `format_version` change: the node selection is view state, and Delete and Join use the commits they use today.

## Depends on

- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §1, §3: the marquee rule (which anchors are inside, the combine table, the order) lives in Rust (`curvyo-ui-core`); `render-core` draws the box and the rings with the Select marquee's draw code; the legend, the badge and the count slot are DOM.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md) §3: "inside or on the edge" is tested with the document's tolerance (criteria 3, 16). §5: nodes are identified by anchor id, so the order of criterion 9 is a list of (path id, anchor id), as `NodeSelection` holds today.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §2: the node selection, the box and the rings are ephemeral (criterion 17). A selected anchor that a peer deletes drops out of the selection as today.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) and [`0020` adrs.md](../0020-undo-redo/adrs.md): selection changes are not steps; a multi-path Delete is one step "Delete nodes" (`delete_anchors`, criterion 18).
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: no bump (criterion 28).
- [`0006` adrs.md](../0006-path-merge-split-and-node-types/adrs.md): the multi-path node selection and the ordered list that Join reads; the marquee appends to it.
- [`0014` adrs.md](../0014-advanced-selection/adrs.md): the marquee's dead zone, the combine rule and the legend; reused, not copied.
- [`0010` adrs.md](../0010-edit-interaction-polish/adrs.md): the Escape cascade and the key gate (Ctrl+A in the Node tool passes the same gate).

## Points for the architect (to decide)

1. **Where the clear moves.** `NodeTool::pointer_down` clears the selection at a press on nothing; the marquee needs it at the release of a click (criteria 11, 12). Proposed: a `Drag::Marquee { down_at, prior }` variant that keeps the prior selection for Escape, and `pointer_up` decides click or marquee by the 3 px rule (`HitTolerances::drag_threshold` already exists).
2. **`NodeSelection` operations.** Add many in document order, and remove many, without disturbing the order of the nodes already there (criteria 7 to 9). Whether "document order" is computed in `ui-core` from the editing set's snapshots or passed in by the session is yours.
3. **The pre-selection count per frame.** Criterion 25 asks for 10,000 nodes inside a 16.7 ms frame. A linear point-in-rectangle scan over the snapshots is a few microseconds per thousand nodes; no spatial index is expected. Name the reference desktop.
4. **One commit for a multi-path Delete.** Criterion 18 requires one commit and one step when nodes of several paths go. Check that `delete_selected` writes one `delete_anchors` commit for all paths (and removes an emptied group in it, `0023` criterion 14); if it writes one per path, that is the change.
5. **The key gate.** Ctrl+A in the Node tool (criterion 22) goes through the gate of `0010` criterion 55 in `session/keys.rs`; it replaces `0044`'s "ignored in the Node tool".
6. **Draw code sharing.** The Select marquee box in `render-core` takes a colour; the node marquee calls it with `--marquee-contain`. The rings reuse the node hover ring. No new draw primitive is expected.
7. **Join tooltip reason** (criterion 19, Proposal): the reason comes from the same function that computes `NodeToolbarState::can_join`, so the text and the enabled state cannot disagree.

## Feature-local decisions

- 2026-10-12 (product owner, defaults of the spec's open questions until the customer answers): the box reaches every path of the editing set; it is always red; Ctrl removes strictly and wins over Shift; Alt has no meaning in the Node tool.

## Flagged to the PO

Nothing yet; the architect adds here.
