# Plan for Node marquee selection

Branch `story/node-marquee-selection`, one PR for the whole slice at the end (`CLAUDE.md` §4).
Inputs: `specification.md` (criteria 1 to 28), `adrs.md` (decisions 1 to 9). Base: `main` at
c124832 (`0050` and `0051` merged). No new ADR, crate or dependency; no `format_version` change.

Note on paths: `adrs.md` names `session/*.rs` without a crate; those modules live in
`curvyo-editor-wasm/src/session/`.

## Affected crates/modules

- `curvyo-document-core`
  - `paths.rs` (or, if it grows, a new `paths/anchor_edits.rs`): `delete_anchors` and
    `convert_anchor_kind` take `&[(NodeId, AnchorId)]` across paths, one commit each
    (decision 4). Removed paths' parents go through `remove_emptied_groups` once.
  - `path_model.rs`: the order guarantee of `PathSnapshot::all_anchors` is stated in its doc
    comment and pinned by a test (the anchor order `0055` relies on).
  - call sites in `src/` tests and `tests/*.rs`: mechanical signature migration only.
- `curvyo-ui-core`
  - new `node_marquee.rs` ("Which anchors a node marquee selects."):
    `editing_set_anchors(paths)` (document order, the one order rule), `anchors_in_box`,
    `NodeMarquee`.
  - `selection.rs`: `NodeSelection::apply(SelectionCombine, &[(NodeId, AnchorId)])`; module doc
    updated (the single-path-only remark is gone).
  - `node_tool.rs`: `delete_selected` / `convert_selected` across paths; `select_all(paths)`
    replaces `select_all_anchors`; `Drag::Marquee`; `pointer_down(.., Modifiers)`,
    `pointer_up(document, paths, point, Modifiers)`; `join_check` / `JoinBlock`;
    `NodeToolbarState` gains `join_block`, `selected_nodes`, `total_nodes`, `segment_selected`.
- `curvyo-render-core`: `DecorationInput::ringed_nodes`, drawn with the `--hover-box` tone ring
  under the box (decision 6).
- `curvyo-editor-wasm`
  - `session/mod.rs`: Node press/move/release pass modifiers and paths; the marquee's inside
    list is refreshed at each move and modifier change.
  - `session/select_gesture.rs`: Node arm of `marquee_overlay`, `node_marquee_legend`, cursor.
  - `session/node.rs`: decorations (rings, no hover during the drag), toolbar state.
  - `session/key_table.rs`, `session/keys.rs`, `session/nudge.rs`: Ctrl+A in the Node tool.
  - `wasm_node_tool.rs`: toolbar fields, legend, announcement count.
- `frontend`: legend text, minus badge, crosshair cursor, status-bar count (three-column grid),
  polite announcement, Join reason line (`aria-disabled` Join button).
- `docs/technical-debt.md`: "Node selection holds one path" resolved; `specs/0044` criterion 3
  amended note.

## Milestones (commits on this branch, each green and pushed)

1. **M1: core rules.** Document-core API change (multi-path Delete and Convert, `all_anchors`
   order guarantee), the ui-core order function, `anchors_in_box`, `NodeSelection::apply`,
   `NodeTool::select_all`, the cross-path Delete/Convert fix in the Node tool. **Stop here for the
   architect's review of the document-core API.**
2. **M2: the marquee in the session.** Node tool drag state, press/move/release, Escape, Ctrl+A,
   box, rings, legend, badge, cursor, `JoinBlock`, toolbar state counts.
3. **M3: the frontend.** Legend, status-bar count, live region, Join reason line, wasm bindings.
4. **M4: performance and docs.** `#[ignore]` release benchmarks (decision 8), debt and spec notes.

## Tasks

### M1
- [x] 1. `Document::delete_anchors(&[(NodeId, AnchorId)])`: one commit across paths, per-path rule
  of `0002` criterion 13, removed paths' parents emptied once, stale anchors ignored, a missing
  path refuses the whole call before any write (AC 18).
- [x] 2. `Document::convert_anchor_kind(&[(NodeId, AnchorId)], kind)`: one commit across paths,
  refuses the whole call on a stale id before the first write (AC 20).
- [x] 3. `PathSnapshot::all_anchors` order guarantee documented and tested (AC 9; `0055`).
- [x] 4. `node_marquee::editing_set_anchors` (document order: slice order, then
  `all_anchors` order) and `anchors_in_box` (inclusive within tolerance, clamp at ±1e7 mm, zero
  width box, non-finite corner ignored by the caller) (AC 3, 4, 9, 16).
- [x] 5. `NodeSelection::apply` Replace / Add / Remove with linear membership (AC 5 to 8, 25).
- [x] 6. `NodeTool::select_all(paths)` replaces `select_all_anchors` (AC 22).
- [x] 7. `NodeTool::delete_selected` / `convert_selected` across paths, one commit (AC 18, 20).

### M2
- [x] 8. `Drag::Marquee`: press on nothing starts it unarmed; arms past `drag_threshold`; release
  applies `from_modifiers(shift, ctrl)`; an unarmed release is a click that clears unless Shift
  or Ctrl (AC 1, 5, 12, 13).
- [x] 9. Escape cancels the marquee without touching the selection (AC 11, 21).
- [x] 10. Session: modifiers and paths to `pointer_down` / `pointer_up`; inside list cached per
  move and modifier change; zoom and pan keep the press corner in document space (AC 14, 15, 16).
- [x] 11. Ctrl+A in the Node tool through the key gate, consumed when there are no nodes (AC 22).
- [x] 12. Box overlay (always red), rings in decorations, no hover during the drag, legend text
  and badge, crosshair cursor (AC 2, 10).
- [x] 13. `join_check` / `JoinBlock`; `NodeToolbarState` counts (AC 19, 23).
- [x] 14. No commit for marquee, click or Ctrl+A (AC 17, 28).

### M3
- [x] 15. wasm bindings for the new toolbar fields and the legend (AC 10, 19, 23).
- [x] 16. Status-bar count in a three-column grid; 6-digit fit (AC 23, 26).
- [x] 17. Polite announcement after a marquee release or Ctrl+A (AC 24).
- [x] 18. Join reason line, `aria-disabled` Join (AC 19).

### M4
- [x] 19. Benchmarks: `anchors_in_box` < 1 ms, `apply` 5,000 on 5,000 < 5 ms, marquee frame
  ≤ 1.1 × plain Node frame (AC 25).
- [x] 20. `docs/technical-debt.md`, shortcut table, `0044` criterion 3 note; full gate (AC 27).

## Deviations (for the review)

- `NodeTool::pointer_down` keeps a bare `shift` (decision 2 asked for `Modifiers`): the clear
  moved to the release, which reads Shift and Ctrl, so Ctrl has no meaning at the press.
- Benchmark "marquee frame at most 1.1 times the plain Node frame" (decision 8): measured 3.3
  times (10.3 ms against 3.1 ms, release, 5,000 rings). The rings are about eight times the
  geometry of the node glyphs, so the ratio is not reachable with the ring of the UX notes.
  `node_marquee_interactivity.rs` asserts criterion 25's absolute 16.7 ms and prints the ratio.

## Validation

- Unit tests first for every core rule, from the spec examples (criteria 3, 9, 16, 18).
- Session tests in `curvyo-editor-wasm` for the press/move/release cascade, Escape, Ctrl+A and
  the "no commit" rule (history length and unsaved marker unchanged).
- Release benchmarks of decision 8 run with `cargo test --release -- --ignored`.
- Manual check in the real window: box, rings, legend, badge, status bar at 800 x 600.
- Full gate of `CLAUDE.md` §7 before the PR.
