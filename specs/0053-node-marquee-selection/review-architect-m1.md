# Architect review of milestone M1 (0053 node marquee selection)

Date: 2026-10-14. Diff: `origin/main...story/node-marquee-selection` (5d40992 plan, 68440c7 M1).
Checked: `curvyo-document-core/src/paths/anchor_edits.rs`, `paths.rs`, `path_model.rs`;
`curvyo-ui-core/src/node_marquee.rs`, `selection.rs`, `node_tool.rs`;
`curvyo-editor-wasm/src/session/select_bar.rs`. Clippy (`-D warnings`) and nextest for
`curvyo-document-core`, `curvyo-ui-core` and `curvyo-editor-wasm` are green (2844 passed), and the
wasm32 build of both core crates passes.

## Verdict: CHANGES REQUESTED

There is one small must-fix. Everything else is approved as built.

## Findings

1. **Must fix. `curvyo-document-core/src/paths/anchor_edits.rs` `delete_anchors`: a missing path
   refuses the whole Delete.** This breaks the rule the function itself documents for stale ids
   (ADR 0009 §2: a stale selection entry stops resolving and drops out).
   - Scenario: nodes on P and Q are selected, and a peer, or an undo, removes Q. Delete then
     returns `NoSuchPath` and P keeps its nodes. `NodeTool::delete_selected` ignores the error
     (`let _ =`) and clears the selection, so the key appears to do nothing and the selection is
     lost. A marquee builds selections across many paths, so this case gets much more likely.
   - Fix: treat a pair whose path no longer exists exactly like a stale anchor id and skip it.
     The nodes of a removed path are already gone, so the intent of the Delete is met.
   - With that, `delete_anchors` has no error case left. Return `()`, update the doc comment
     (drop `# Errors`), and change the test
     `a_missing_path_refuses_the_whole_delete_before_any_write` to "a missing path is skipped and
     the live paths are still deleted in one commit".
   - The call-site churn is mechanical (tests only, plus `node_tool.rs`). Dated note in
     `adrs.md` decision 4.
2. **The asymmetry between Delete and Convert (question 1) is right for anchors once finding 1
   is fixed.**
   - Removal is idempotent: a node that is already gone satisfies "delete these nodes".
   - Convert writes values to named nodes, the same way `move_anchors` does, and `move_anchors`
     also refuses the whole call on a stale id. Keeping Convert strict keeps one rule for all
     value writes.
   - The resolve-everything-first loop guarantees no partial write. Every other refusal rule is
     confirmed:
     - pairs grouped by path in first-appearance order;
     - duplicates deduplicated;
     - a path left with fewer than two anchors is removed;
     - the parents are collected before the deletes, and `remove_emptied_groups` runs once in
       the same commit;
     - nothing is committed for an empty call, an all-stale call, or an all-same-kind convert;
     - one label per call.
   - Follow-up for M2, not blocking: Convert, Move and Join still fail silently on a
     peer-deleted node. If the tester finds that visible, prune `NodeSelection` against the
     editing set when the session refreshes `paths()`, not in the document API.
3. **Approved: `PathSnapshot::all_anchors` order guarantee** (`path_model.rs`).
   - The doc comment states the order: first outline, then `extra_subpaths` in storage order,
     no repeated first anchor, independent of positions.
   - `all_anchors_walks_the_outlines_in_storage_order` pins it.
   - This is the contract 0055 relies on. Changing it later is a breaking change to Join's
     "first selected".
4. **Approved: `editing_set_anchors(paths)`** (`node_marquee.rs`).
   - It yields the slice order, then `all_anchors` order. The module doc states the rule once,
     and `anchors_in_box` and `select_all` both go through it, so there is a single order rule.
   - That the session passes the slice bottom to top is not tested in M1: `Session::paths`
     filters `pickable_objects()` and does not reorder. In M2, add a session test of criterion
     9's example (P1 lower, P2 upper) through `Session` so that the order cannot drift.
5. **Approved: `anchors_in_box`.**
   - A NaN corner is checked before the clamp, because `f64::clamp(NaN)` is NaN.
   - ±infinity clamps to ±`MAX_COORDINATE_MM` (1e7).
   - A negative or NaN tolerance becomes 0 through `max(0.0)`.
   - The edge is inclusive after growing by the tolerance once.
   - A zero-width box works.
   - Only anchor positions are tested, and the result is in document order.
   - The private `Bounds` holds bare `f64`, which is acceptable because it is local, derived
     from `Point`, and never leaves the module (§5 "units are types" covers the public API).
6. **Approved: `NodeSelection::apply`** (`selection.rs`).
   - Replace, Add and Remove match decision 3, including the segment-selection cases.
   - Membership uses one `HashSet` per call, so it is linear.
   - Duplicates in `result` are taken once.
7. **Approved: `NodeTool::select_all(paths)`.** It replaces `select_all_anchors`, has no second
   loop, an empty slice clears the selection, and the "object to path" caller uses
   `std::slice::from_ref`.
8. **Question 4, 0055's `delete_anchors_many`: reconciled in favour of 0053 decision 4.**
   - There is one write path, `delete_anchors(&[(NodeId, AnchorId)])`. Flat pairs win over
     0055's suggested `&[(NodeId, &[AnchorId])]` because `NodeSelection` stores flat pairs, and
     `group_by_path` regroups them once.
   - Dated note added to `specs/0055-compound-path-node-editing/adrs.md`.
9. **Question 5, Delete and Convert only search a path's first outline: acceptable as the
   intermediate state.**
   - Today no compound path reaches these commands, because `Session::paths` filters
     `is_compound()` out of the editing set. Behaviour on `main` was the same.
   - Trap to record, not to fix now: when 0055 lifts the guard, the "fewer than two anchors:
     remove the node" rule must become 0055's outline removal and promotion. Otherwise a Delete
     that empties the first outline removes the whole compound path with all its other
     outlines.
   - Add one sentence to the `delete_anchors` and `convert_anchor_kind` doc comments: "Only the
     first outline is searched until `specs/0055-compound-path-node-editing/`." This goes in the
     same commit as finding 1.
10. **§5 and core purity: pass.**
    - No new crate, dependency, trait or generic.
    - The `unwrap`s carry `// invariant:` comments and a scoped `#[allow]`.
    - `anchor_edits.rs` is 373 lines, and its two public functions are under 45 lines each.
    - `paths.rs` shrank from 1,443 to 1,304 lines, as decision 4 asked.
    - `node_tool.rs` was already over 500 lines before M1 and gained no new production logic
      beyond the two simplified methods. The marquee code went to `node_marquee.rs`, as
      decision 2 asked.
    - Both core crates stay free of I/O, clock and threads, and they build for wasm32.
    - The geometric comparison uses an explicit `Tolerance`.

## Addendum 2026-10-14: frame budget and the M2 to M4 skim

- **Finding 1 is fixed.** Delete skips missing paths and returns `()`.
- **Decision 8's frame ratio of 1.1 is withdrawn.** The gate is the absolute 16.7 ms. The ratio
  is printed only. See the dated note in `adrs.md` decision 8: with 5,000 rings the frame is
  10.6 ms against 3.2 ms.
  - The cheap optimisation is already done (`glyphs::rings`).
  - The next one, instanced ring drawing, is a renderer change and goes under the debt item
    "Canvas performance on Linux/WebKitGTK".
- **Optional, not required:** `DecorationInput::ringed_nodes` could carry each node's position,
  which the marquee already has from `anchors_in_box`. `push_marquee_rings` would then not
  build a map of all 10,000 anchors per frame.
- **The M2 to M4 skim of ui-core, editor-wasm and render-core found no structural issue.**
  - No new crate, trait or generic, and the dependency direction is unchanged.
  - The marquee logic lives in `curvyo-ui-core/src/node_marquee.rs` and the session part in
    `session/node_marquee.rs`.
  - `JoinBlock` and `join_check` follow decision 7.
- **Non-blocking:** `node_tool.rs` now has 931 production lines, up from 822, which is far over
  §5's 500. The PR must state the reason. The first split to make later is the join rules
  (`join_check`, `JoinBlock`, `is_open_end`, `join_selected`) into their own module. Track it
  in `docs/technical-debt.md` if it is not split in this slice.
