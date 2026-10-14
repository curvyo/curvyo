# ADRs for "Node marquee selection: drag a box in the Node tool to select many nodes"

Architect decisions of 2026-10-12, checked against `main` at 6841fb6. They replace the product owner's
first list of points.

**Summary.**

- No new ADR. No new crate. No new external dependency.
- No stored field and no `format_version` change. The node selection, the box and the rings are view
  state.
- One public API change in `curvyo-document-core`. `Document::delete_anchors` and
  `Document::convert_anchor_kind` take `(path, anchor)` pairs across paths and still make one commit
  (decision 4). Today both work on one path only. Under `CLAUDE.md` §4 the architect reviews that
  milestone on the branch diff.
- Nothing here is `needs-customer`.

## Depends on

- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md):
  - §1: the marquee rule lives in `curvyo-ui-core`: which anchors are inside, how the result combines,
    and the order.
  - §4: `render-core` draws the box with the Select marquee's code and the rings with the node hover
    ring.
  - §5: the legend, the count slot and the live region are DOM. They read a small state per
    interaction, never the anchors.
- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md):
  - §3: "inside or on the edge" takes an explicit `Tolerance` (criteria 3, 16).
  - §5: a path is a `NodeId`.
  - §9: a multi-path Delete or Convert is one commit (criterion 18).
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md):
  - §2: the node selection, the box and the rings are ephemeral (criterion 17). A selected anchor that
    a peer deletes stops resolving and drops out.
  - §3: anchors are a movable list of maps with stable ids.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md) and
  [`0020` adrs.md](../0020-undo-redo/adrs.md):
  - a selection change is not a step;
  - a multi-path Delete is one step, "Delete nodes" (`delete_anchors`);
  - undo restores the node selection a step recorded, by anchor id.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9: no format bump (criterion
  28).
- [`0002` adrs.md](../0002-path-node-editing/adrs.md): the anchor schema and `AnchorId`, minted once
  and globally unique. Delete's rule: adjacent nodes are rejoined, and a path with fewer than two
  nodes is removed.
- [`0006` adrs.md](../0006-path-merge-split-and-node-types/adrs.md), "one Node-tool session over several
  paths":
  - the ordered `(NodeId, AnchorId)` list that Join reads as first and second;
  - "a multi-path node drag is one commit", which decision 4 extends to Delete and Convert.
- [`0014` adrs.md](../0014-advanced-selection/adrs.md):
  - `SelectionCombine { Replace, Add, Remove }` and `from_modifiers`, in `object_selection.rs`: one
    combine rule for objects and nodes;
  - the 3 px dead zone;
  - the legend and the minus badge.
- [`0010` adrs.md](../0010-edit-interaction-polish/adrs.md):
  - decision 4: the key gate (`session/key_table.rs`);
  - decision 6: the Escape cascade (`session/keys.rs`). The marquee is step 2, "a drag in flight".
- [`0023` adrs.md](../0023-groups/adrs.md): `remove_emptied_groups` in the same commit as the delete
  (criterion 18).
- [`0044` adrs.md](../0044-editing-quick-wins/adrs.md): Ctrl+A. This spec amends its criterion 3 for
  the Node tool.

## Feature-local decisions

### 1. The editing set (2026-10-12; criteria 1, 4, 22, 23)

- **What the code does.** The Node tool shows, hit-tests and drags the nodes of `Session::paths()`.
  That is every non-compound path of the current context: the entered group, or the whole document at
  the root. It is not only the object selection at the moment the tool was entered. Two places do
  this:
  - `session/mod.rs`: `paths()`, which `pointer_down` uses for the hit test;
  - `session/draw.rs`: `editable`, the paths the decorations get.
- **Decision.** The editing set is exactly the slice that the hit test and the decorations receive.
  The marquee, Ctrl+A and the count read the same slice, so the box selects exactly the nodes the maker
  sees. When `0050` filters hidden and locked nodes out of the context read (its decision 4), and
  `0049` filters clones, the marquee inherits that filter with no code of its own.
- The spec's "What exists today" row is corrected under "Flagged to the PO", item 1.

### 2. Where the clear moves; the marquee state (2026-10-12; criteria 1, 5, 11, 12, 13)

- `node_tool.rs` has 831 lines before its tests. The new code therefore goes into a new module,
  `curvyo-ui-core/src/node_marquee.rs`, with the doc sentence "Which anchors a node marquee selects."
  It holds:
  - `NodeMarquee { down_at, current, armed }`;
  - `pub fn anchors_in_box(paths: &[PathSnapshot], from: Point, to: Point, tolerance: Tolerance)
    -> Vec<(NodeId, AnchorId)>`.
- `anchors_in_box` gives anchor positions only:
  - it never returns handles or segments;
  - the box is inclusive within `tolerance`;
  - corners beyond ±1e7 mm are clamped to `MAX_COORDINATE_MM` (criterion 16);
  - the order is the slice order, then anchor index (criterion 9).
- `NodeTool`'s `Drag` gets a variant `Marquee(NodeMarquee)`.
  - **Press.** The `None` arm of `pointer_down` no longer clears the selection. It starts
    `Drag::Marquee` with `armed = false`.
  - **Move.** `pointer_moved` arms the marquee once the pointer is `tolerances.drag_threshold` away
    from the press (3 px at the current zoom; the field exists).
  - **Release.** The release decides:
    - not armed: a click. It clears the node and segment selection unless Shift or Ctrl is held
      (criterion 12);
    - armed: `selection.apply(SelectionCombine::from_modifiers(shift, ctrl), &anchors_in_box(...))`.
- **Escape.** The node selection changes only at the release. So a cancel just drops the drag, and the
  selection, with its order, is untouched. No copy of the prior selection is kept (criterion 11).
  `cancel_drag()` and `drag_in_flight()` cover the new variant unchanged.
- **Signatures.**
  - `NodeTool::pointer_up(&mut self, document, paths, point, modifiers)` gets the paths and the release
    modifiers. Today it takes only the document and the point.
  - `pointer_down` takes `Modifiers` instead of a bare `shift`, so a Ctrl press on nothing no longer
    clears (`0014` flag 1).
  - `Session::pointer_up` already has `shift` and `constrain` (Ctrl) to pass on. Alt is ignored
    (criterion 13).
- **Tests.**
  - `node_tool.rs::clicking_empty_space_clears_the_selection` is rewritten to "clears at the release
    of a click". The test is kept.
  - New tests in `node_marquee.rs` use the examples of criteria 3, 9, 16 and 18.

### 3. `NodeSelection` operations and order (2026-10-12; criteria 6 to 9, 22)

- `selection.rs` gets `pub fn apply(&mut self, combine: SelectionCombine, result: &[(NodeId, AnchorId)])`.
  It reuses `SelectionCombine` from `object_selection.rs`; no second enum.
  - **Replace** sets the list.
  - **Add** keeps the existing pairs in their order and appends the new ones from `result` in
    `result`'s order. A segment selection is replaced only by a non-empty result.
  - **Remove** removes the pairs, and a segment selection is left as it is.
  - Membership is tested with a `HashSet` built once per call, so 10,000 against 10,000 stays linear
    (criterion 25).
- The document order is computed in `curvyo-ui-core` from the slice the session passes. The session
  does not pass it in a separate argument. `Session::objects()` is in stacking order, bottom to top
  (`0023`), and `anchors_in_box` keeps slice order and anchor index.
- `NodeTool::select_all(paths)` is `apply(Replace, every anchor in that order)`. The existing
  `select_all_anchors(path)`, used after "object to path", becomes `select_all(&[path])`; there is no
  second loop.

### 4. One commit for a multi-path Delete, and for Convert (2026-10-12; criteria 18, 20)

- **Today's bug.** `NodeTool::delete_selected` and `convert_selected` start with
  `let Some(path) = self.selection.path() else { return };`. `path()` is `None` when the selection
  spans two paths. So today Delete, Backspace, the Delete node button and Make
  corner/symmetric/asymmetric do nothing on a Shift+click selection across paths, while their buttons
  are enabled (`can_delete = has_nodes`). A marquee makes such selections common, so this slice fixes
  it.
- **Decision.** `Document::delete_anchors(&self, anchors: &[(NodeId, AnchorId)])` and
  `Document::convert_anchor_kind(&self, anchors: &[(NodeId, AnchorId)], kind)` take pairs, as
  `move_anchors` already does. Each makes one commit with its existing label.
  - **Delete per path:** today's rule. Stale ids are ignored, as today.
  - **Removed paths:** every path left with fewer than two nodes is removed. Their parents are
    collected, and `remove_emptied_groups` runs once in the same commit. After `0050` that function
    skips layers (`0050` decision 2).
  - **Convert:** refuses the whole call on a stale id, as today, before the first write.
- **Option rejected:** keep the single-path methods and add `_across` twins. That gives two write paths
  for one rule, and the single-path ones would be left with test callers only.
- **Call sites:** about 34, almost all tests in `curvyo-document-core`. The change is mechanical.
  `paths.rs` already has 1,443 lines. If the two methods grow it, move them with their helpers into
  `paths/anchor_edits.rs` in the same milestone.
- **Undo.** One commit revives the removed paths through the injected move of ADR 0014, as a
  single-path Delete does today.
- The debt item "Node selection holds one path" in `docs/technical-debt.md` is updated. This slice
  resolves it.
- **2026-10-14, M1 review (`review-architect-m1.md` finding 1).**
  - **Delete skips stale pairs.** A pair whose path no longer exists is skipped, the same way as
    a stale anchor id. Its nodes are already gone. This replaces "a missing path refuses the
    whole call".
  - **`delete_anchors` returns `()`.** It has no error case left.
  - **Why Delete and Convert differ:** removal is idempotent. Convert writes values to named
    nodes and stays strict, as `move_anchors` does.
  - **First outline only.** Both commands search only a path's first outline until `0055`.
- **2026-10-14, architect review of M1.** `delete_anchors` skips a pair whose path no longer
  exists, the same way it skips a stale anchor id (ADR 0009 §2), so it has no error case and
  returns `()`. Convert stays strict, like `move_anchors`. Both commands search only a path's
  first outline until `specs/0055-compound-path-node-editing/`.

### 5. The key gate and Ctrl+A (2026-10-12; criteria 21, 22)

- In `session/key_table.rs`, `KeyAction::SelectAll` acts for `Tool::Select | Tool::Node` behind the same
  `gate_open`.
- `Session::select_all` (`session/nudge.rs`) dispatches on the tool. In the Node tool it calls
  `node.select_all(&paths)` and returns a non-`Ignored` outcome even when there are no nodes, so the key
  is consumed.
- A running marquee is `node.drag_in_flight()`, which counts as `operation_in_flight`. So every other
  key is gated with no new code.
- The table test in `keys.rs` gets a Node row for Ctrl+A. `0044` criterion 3 is amended in the PR.

### 6. Drawing (2026-10-12; criteria 2, 10)

- **Box.** `Session::marquee_overlay()` (`session/select_gesture.rs`) gets a Node-tool arm. It returns
  `MarqueeOverlay::Box { from, to, contain: true }`, and `build_marquee_overlay` draws it as for the
  Select tool, always red. There is no new primitive.
- **Rings.** `DecorationInput` (`render-core/src/decorations.rs`) gets `ringed_nodes: Vec<(NodeId,
  AnchorId)>`. They are drawn under the box with the ring in the 65 % `--hover-box` tone from the UX notes (amended 2026-10-12; the plain hover ring is near invisible under the red box fill). The hover state is
  empty during the drag.
- **When the inside is computed.** The session stores the "inside" list in the drag at each pointer
  move and each modifier change. The draw, the legend count and the release read it; it is not
  recomputed per draw call.
- **Legend.** A new `Session::node_marquee_legend() -> Option<(usize, SelectionCombine)>` feeds the
  legend. The legend component and the minus badge come from `0014` and are reused.

### 7. Join reason and the count (2026-10-12; criteria 19, 23)

- **The join check.** `NodeTool::can_join` becomes
  `join_check(&self, document) -> Result<(), JoinBlock>`, and `can_join` is `join_check().is_ok()`.
  So the text and the enabled state come from one function.
- **`JoinBlock`.** It has these values:
  - `Count(usize)`: not exactly two nodes;
  - `NotEndNodes { not_end: usize, of: usize }`: computed from the snapshots, an end node being the
    first or last anchor of an open path;
  - `Other`: `document.check_join` refused for another reason, such as the two ends of a two-node
    path. It shows no reason line.
- `NodeToolbarState` gets `join_block`, `selected_nodes`, `total_nodes` and `segment_selected`.
  `toolbar_state` therefore takes the paths slice as well. The texts are frontend strings.

### 8. Performance (2026-10-12; criterion 25)

- **Reference desktop:** the customer's Linux machine, under Tauri, WebKitGTK and WebGL2. It gets one
  frame-rate check in the real window.
- **Gates:** `#[ignore]` benchmarks in a native release build. The test data is 10 paths of 1,000
  nodes, with the box over half of them:
  - `anchors_in_box` takes under 1 ms;
  - `NodeSelection::apply` for 5,000 nodes against 5,000 takes under 5 ms;
  - the marquee frame of `Session::draw_list()` takes at most 1.1 times the plain Node-tool frame with
    the same paths.
- **2026-10-14, the frame ratio is replaced.** The marquee frame of `Session::draw_list()` must
  take under 16.7 ms. The ratio to the plain Node-tool frame is printed and not asserted.
  - Why: the 1.1 ratio left out that the rings add geometry in proportion to the ringed nodes.
    5,000 rings, each a casing and a ring of 16 segments, are about 320,000 triangles, many times
    the plain node glyphs. Measured: 10.6 ms against 3.2 ms (3.3 times).
  - The ring code already computes the unit circle once per frame.
  - Fewer segments or no casing would break the UX notes.
  - The remaining fix belongs to the renderer: one ring mesh drawn as GPU instances. It goes
    under the debt item named below and is not built in this slice.
  - The PO amends criterion 25's wording to match.
- There is no spatial index. A linear scan is enough at this size.
- A miss of the absolute 16.7 ms in the real window is recorded under "Canvas performance on
  Linux/WebKitGTK: measured once, renderer not cached". It does not block this slice.

### 9. Interactions with `0050`, `0051` and `0052` (2026-10-12)

- **`0050` (layers, being built).**
  - Hidden and locked paths: decision 1 inherits the filter. Its consumer-list test (`0050` criterion
    19) gets the rows "Node marquee" and "Node Ctrl+A", added by whichever slice merges second.
  - `0050` decision 5 drops hidden and locked objects from the object selection. Whoever merges second
    adds the same pass for `NodeSelection` pairs whose path became hidden or locked.
  - `0050`'s `remove_emptied_groups` skips layers, and decision 4 relies on that.
  - Shared files:
    - `session/key_table.rs` and `session/nudge.rs` (`0050` amends Ctrl+A for layers);
    - `session/draw.rs` (marquee and decorations);
    - `curvyo-document-core/src/paths.rs` and `groups.rs`.

    These are merge conflicts, not design conflicts.
- **`0051` (merged in #104).**
  - At 464 px the Node bar has about 24 px to spare (`0051` UX measurement), so the node count goes
    into the status bar's right segment, not the Node bar (UX notes and criterion 23, amended 2026-10-12).
  - File chords are already gated during a drag, which includes the marquee.
- **`0052`.** Shared files: `key_table.rs` and `session/keys.rs`.
- **Order.** `CLAUDE.md` §4 runs two implementers in parallel only on features that touch different
  crates. `0050`, `0052` and `0053` all touch `curvyo-document-core`, `curvyo-ui-core`,
  `curvyo-editor-wasm` and the frontend. This slice starts after `0050` merges and is not built in
  parallel with `0052`. Of the two, build it first: it is smaller, and it fixes the cross-path Delete
  bug of decision 4.

### Product owner defaults (2026-10-12)

Until the customer answers the spec's open questions:

- The box reaches every path of the editing set.
- The box is always red.
- Ctrl removes strictly and wins over Shift.
- Alt has no meaning in the Node tool.

## Flagged to the PO

Each item has a default. None is a new customer question.

1. **"What exists today", the row "Which paths show nodes", is wrong.** In the code, the Node tool
   shows and edits the nodes of every non-compound path in the current group, or in the whole document
   at the root, not only of the selected paths (decision 1).
   - Correct the row.
   - Reword Question 1, option A, to "every path whose nodes the Node tool shows: every path of the
     current group, or of the document at the top level". The customer then answers what the code
     really does.
   - Criterion 22's Ctrl+A at the root therefore selects every node in the document.

   *Resolved 2026-10-12: row corrected, Question 1 option A reworded, criterion 22 and "Changes to
   accepted behaviour" name the root case.*
2. **"What exists today", the Delete row, and "Changes to accepted behaviour".** Today Delete,
   Backspace, the Delete node button and Make corner/symmetric/asymmetric do nothing on a selection
   across paths, while the buttons are enabled (decision 4). Add a row to "Changes to accepted
   behaviour": "a cross-path node selection: Delete and Convert did nothing; now they act on every
   selected node in one step." *Resolved 2026-10-12: Delete row corrected, row added.*
3. **Criterion 19.** When `check_join` refuses two end nodes for a reason the two texts do not cover
   (the two ends of a two-node path), no reason line is shown. Default: keep it that way, or add a
   third text if the ux-engineer wants one. *Resolved 2026-10-12: no third text (UX notes); criterion
   19 says so.*
4. **Criterion 25.** Name the gates of decision 8 in the criterion. The absolute frame budget is the
   real-window check, and a miss goes to the renderer-cache debt item. *Resolved 2026-10-12: criterion
   25 names the gates.*
