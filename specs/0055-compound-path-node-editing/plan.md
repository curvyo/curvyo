# Plan for Compound path node editing (0055)

One branch `story/compound-path-node-editing`, stacked on `story/combine-open-paths` (`0054`, PR
#154), one PR, opened by the lead when the maker can edit the nodes of a compound path in the app
and the full local gate (`CLAUDE.md` §7, the frontend checks, the budget tests) is green. Milestones
are commits on the branch; none is a PR. **No `format_version` change** (criterion 12). No new
crate, no new dependency, no trait, no generic.

## Affected crates/modules

- `curvyo-document-core`
  - `subpath_codec.rs`: the outline resolver. `AnchorPositions` also keeps each outline's map
    (the one that holds `closed` and `anchors`: the meta map for the first outline, the
    `extra_subpaths` element for the others) and its slot; new `remove_outlines` (removal and
    promotion of `adrs.md`), `insert_outline` (Split) and `append_outlines` (Join across objects).
  - `paths.rs`: `Document::outline_parts(path, anchor)`; `move_anchors`, `set_handle`,
    `insert_anchor`, `set_segment_line`, `set_segment_curve` resolve the outline of the anchor.
  - `paths/anchor_edits.rs`: `convert_anchor_kind` and `delete_anchors` per outline; Delete removes
    an outline left with fewer than 2 nodes, promotes, and turns a compound path of one outline into
    an ordinary path.
  - `segment_bend.rs`: `bend_segment` per outline.
  - `path_topology.rs`: Join (a) one outline, (b) two outlines of one object, (c) two objects with
    the other object's remaining outlines appended; Split at a node of an outline of a compound path
    keeps one object.
  - `path_extend.rs`: `PathGrowth` names the grown outline by its end node (`at`) and the absorbed
    outline by its end node (third element of `absorb`); compound survivors and absorbed compound
    paths are allowed; the absorbed object's other outlines are appended.
  - `path_model.rs`: `PathSnapshot::anchor(id)` and `PathSnapshot::outline_of(id)` read helpers.
- `curvyo-ui-core`: `node_tool.rs` (anchor lookups over every outline, Join/Split notice counts),
  `hit_test.rs` (nodes, handles, segments of every outline), `segment_bend.rs` (outline of the
  segment), `close_path.rs` (outline candidates, selected-node rule, path/outline counts),
  `pen_target.rs` (open outline ends of compound paths, the outline of a continuation),
  `pen_tool.rs` and `pen_tool/commit.rs` (`at` and the absorbed outline; moved-outline count).
- `curvyo-render-core`: `decorations.rs` draws nodes, handles, hover band and selected segment of
  every outline.
- `curvyo-editor-wasm`: `Session::paths` keeps compound paths; `draw.rs` decorates them; the
  `compound_only` flag and the `"compound_path"` double-click hint are removed; Join, Split, Close
  path and Pen outcomes carry the counts for the notices.
- `frontend`: Node bar without the compound pill, Join and Split notices, Close path texts with
  outline counts, the Pen join notice in the Edit hint chip surface.
- `.github/workflows/ci.yml`: the release budget test of criterion 11.

## Tasks

Milestone 1: document model commands work per outline (commit
`feat(curvyo-document-core): node commands act on the outline that holds the anchor`).

- [ ] 1. Outline resolver in `subpath_codec.rs`, `outline_parts` in `paths.rs`; move, handle,
  insert, make line/curve, bend and convert per outline; a segment of two outlines is
  `NotAnAdjacentSegment` (AC 3).
- [ ] 2. Delete per outline with removal, promotion and the drop to an ordinary path (AC 4, 10).
- [ ] 3. Join (a), (b), (c) and Split at a node on compound paths (AC 5, 6, 10).
- [ ] 4. `PathGrowth.at` and the absorbed outline; extend, connect and close per outline (AC 7, 8,
  9, 10).
- [ ] 5. Save and reopen of an edited compound path keeps outlines, states, directions, ids; no
  format change (AC 12).

Milestone 2: the Node tool, Close path and the Pen on compound paths (commit
`feat(curvyo-ui-core): the Node tool, Close path and the Pen edit compound paths`).

- [ ] 6. Editing set keeps compound paths; overlay, hit test, drag, handles, insert, bend per
  outline; `compound_only` and the compound double-click hint removed (AC 1, 2, 3).
- [ ] 7. Join and Split through the Node tool with the moved-outline and split counts (AC 5, 6).
- [ ] 8. Close path: outline candidates, selected-node rule, counts of paths and outlines apart
  (AC 7).
- [ ] 9. Pen: open outline ends are continue, close and connect targets; moved-outline count for
  the notice (AC 8, 9).
- [ ] 10. Undo restores each command (AC 10).
- [ ] 11. Release budget test on the 4004-node plate, added to `ci.yml` (AC 11).

Milestone 3: the frontend (commit `feat(frontend): node editing of compound paths in the Node bar
and the Pen`).

- [ ] 12. Node bar without the compound pill and hint chip; Join tooltip words "open paths or
  outlines"; Join and Split notices; Close path notes and notice with outline counts; the Pen's
  Join notice in the Edit hint chip surface (AC 1, 5, 6, 7, 9).

## Decisions taken while planning (safe defaults, reported to the lead)

- **`PathGrowth` names outlines by their end node.** A compound path has several open outlines, so
  `path` + `end` no longer say which one grows. `PathGrowth` gains `at: AnchorId` (the end node of
  `path` at `end`) and `absorb` becomes `(NodeId, PathEnd, AnchorId)`. An anchor id, not an outline
  index, so `adrs.md` "no outline index in any public type" holds. A public-API change in a core
  crate, additive in meaning; the callers are `curvyo-ui-core` and the crate's own tests.
- **The notice counts are computed by the caller before the command.** `join_endpoints` and
  `connect_paths` keep their return types (many tester tests call them): the moved-outline count is
  the other object's outline count minus one, read from the snapshots `curvyo-ui-core` already has.
- **Coincident nodes:** the hit test keeps today's tie rule (selected first, then the first in
  document order). The UX notes say "the one drawn last". Question NODE-TIE.
- **Close path hover preview** (UX notes, not a criterion): not built. Question CLOSE-PREVIEW.
- **Pen chip line "2 more outlines move into the result."** (UX notes, Pen hint chip): not built.
  The notice after the commit (criterion 9) is built. Question PEN-CHIP-LINE.
- **Marker sentence after a Join** ("Markers are not drawn on compound paths.", UX notes): not
  built. Question JOIN-MARKERS.
- **Tests of the withdrawn `0016` criteria 38 and 38a** (`acceptance_0016_pr2_tester.rs`
  `ac38a_...`, `compound_path.rs` `join_and_split_refuse_a_compound_path`, the editor-wasm tests that
  check the compound pill and the compound double-click hint) assert behaviour this spec withdraws.
  They are rewritten to the amended criteria (Join and Split now act), not deleted. Reported.

## Validation

- Document-core unit and integration tests for every command on compound paths, including undo and
  save-and-reopen (AC 3 to 10, 12).
- `curvyo-ui-core` tests for the node tool, Close path counts and the Pen targets (AC 1 to 9).
- `curvyo-editor-wasm` session tests end to end through pointer events (AC 1 to 10).
- Release budget test `compound_node_editing_budget` on a closed rectangle with 2000 open lines of 2
  nodes (4004 nodes): overlay within 500 ms, one drag frame within 33 ms, Delete of 2000 nodes within
  1 s (AC 11).
- Frontend `node --test` for the new texts.
