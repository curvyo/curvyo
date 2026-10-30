# Independent acceptance plan: multiple pages

Status: Prepared from the specification before implementation review; execution
and per-criterion evidence are tracked in [acceptance-report.md](acceptance-report.md).

The customer-decided spatial-frame model, accepted ADR 0019, `adrs.md`,
`ux-notes.md` and the specification's defaults are the oracle. ADR 0019 supersedes
the older draft shape wording: all page frames are rectangular. The customer
explicitly excluded compatibility with old files on 2026-10-29 (unreleased SID);
no legacy migration or old-format fixtures are required. Current single-page UX
behavior remains in scope. This plan does not defer any remaining criterion. Tests will use public APIs in
`curvyo-document-core/tests/acceptance_0059.rs`,
`curvyo-editor-wasm/tests/acceptance_0059.rs`, and frontend tests as appropriate.
Visual and pointer-interaction checks need the running editor; source-text
assertions alone are not evidence of working UI behavior.

## Criterion coverage

| AC | Independent check | Required evidence |
| --- | --- | --- |
| 1 | Create a new current-format document: exactly one rectangular page at `(0, 0)` with the default size. Save a current-format project with multiple differently sized/named/positioned pages and reopen: preserve every page and its order. Opening/querying does not mark the project dirty. | New-document assertions, current-format multi-page golden fixture and round trip, UI dirty check. |
| 2 | Document tab lists each name and size beneath size fields. Add button has tooltip and accessible name. Exactly one active row. Switch to visible page without viewport movement; switch to off-screen page brings it into view. Neither action opens a popup. | Frontend interaction and viewport assertions; visual check. |
| 3 | Add while a non-last page is active: inherit the last listed page's width and height without copying artwork or guides, generate `Page N`, activate it, place using the default gap and wrapping rule. One undo removes it; history label is `Add page`. | Public document/editor tests and UI Add click. |
| 4 | Resize page 2 with distinct-size pages 1 and 3: only page 2 dimensions change; all page origins remain fixed. Wholly contained objects not shared with another page translate by half the size delta; partial/shared objects stay fixed. Preserve the original single-page resize behavior, including its half-delta translation of all objects. Repeat with size text, unit conversion, orientation and format selection. | Editor tests plus frontend fields/formats interaction. |
| 5 | Rename with spaces and Unicode; blur/Enter commits and Escape cancels. Reorder first to last and last to first using the handle: IDs, positions, dimensions and object transforms stay identical. Active identity survives reorder. | Public API order/identity assertions and drag/keyboard UI checks. |
| 6 | Delete an inactive and an active page containing inside/partial objects: only frame disappears. Objects retain IDs, transforms and layer membership. Undo restores frame and order. With one page, deletion is rejected and the row has no delete button. | Document/editor tests and frontend delete checks. |
| 7 | Render three unequal pages, including template guide objects and a negative-origin page: shared background, edges, small names above top-left, stronger active edge, pasteboard between pages. Objects spanning frame edges remain visible. | Render/scene assertions where available; browser visual check at multiple zoom levels. |
| 8 | Switch between origins `(0, 0)`, `(120, 30)`, and `(-50, -20)`: ruler zero follows active top-left, including after pan/zoom. Object global coordinates remain unchanged. | Ruler calculation tests and visual ruler alignment. |
| 9 | Classify wholly inside, partly overlapping, overlapping two pages, and pasteboard objects by bounds. Move frame by edge: wholly contained objects translate once by the frame delta, partial/outside objects stay. Frame-only switch defaults off; when on all objects stay. One undo restores frame and objects together. | Public movement/query tests, nested/clone cases below, pointer drag check. |
| 10 | Original on page 1 and linked clone on page 2: edit original geometry/style, confirm clone updates while retaining its placement and overrides. Move/delete/reorder page frames and undo; original/clone references survive. | Document/editor cross-page clone tests. |
| 11 | One shared layer contains objects on multiple pages. Hide/show, lock/unlock, reorder and rename layer act across pages; switching active page does not create or filter away layers. | Layer public API assertions and Layers tab check. |
| 12 | Last canvas pointer use on active page, then Ctrl+A: selects only overlapping objects in that page context. Last use on pasteboard: selects all eligible objects across pages. Repeat after changing active row, hovering another page (which must not change context), pressing another page, and with a spanning object. Initial/unset and stale page context select the whole drawing; node/text Select all remain unchanged. Existing hidden/locked selection rules remain intact. | Editor input-sequence tests and keyboard check; no test-only direct selection substitute. |
| 13 | Each add/delete/resize/rename/reorder/move produces one named undoable operation. Undo and redo each independently; selection of an active row adds no operation. After undo/delete of active frame, active ID resolves to a surviving frame. Save/reopen does not persist active-page choice. | History and persistence assertions, including a branch after undo. |
| 14 | Build a document of 200 unequal pages with representative objects. Add, repeatedly switch first/last page, and draw. Check correctness and the explicit implementation-plan CPU budget; record timing and environment. | CPU-time tests following existing timing helpers, plus browser responsiveness observation. No wall-clock CI threshold. |

## Edge-case matrix

- Placement: 100 × 100 mm yields a 20 mm gap; 400 × 100 mm yields 40 mm.
  Adding the ninth page exercises the eight-page wrap. For the trailing run
  sharing a top coordinate, wrap when the proposed right edge exceeds 4000 mm
  from the run's leftmost edge; equality does not wrap. Place the next frame at
  that leftmost x and the greatest bottom plus gap. Mixed heights must not overlap
  rows. A frame wider than 4000 mm remains permitted.
  After reorder, inheritance and placement follow the new last listed frame.
- Rectangular frames and templates: Add copies dimensions only, creating no
  artwork or guide nodes. Circle/custom template outlines remain ordinary guide
  objects; membership uses the rectangular frame, including its corners.
- Resize: test positive and negative size deltas against a 10 × 20 mm change
  (expected eligible translation 5 × 10 mm). Partial overlaps and objects shared
  with another page do not move; no object scales and other frame origins stay
  fixed. Undo/redo restores frame size and eligible object positions together.
- Bounds: exact containment on each edge, small positive overlap, zero-area
  objects, negative coordinates, rotated objects, groups and nested groups.
  Edge-only contact expectations must follow the documented geometry policy;
  do not invent epsilon values that hide misclassification.
- Movement: a group plus its descendants must not move twice; an object wholly
  on two overlapping frames moves once when one frame moves. A cross-page clone
  of a moved original must not inherit an accidental additional translation.
  Include locked/hidden wholly contained objects and report the documented policy.
- Identity: delete then add; rename to an existing name; reorder then delete;
  no duplicate IDs and valid active fallback (next row, otherwise previous).
  Page N uses the next unused default number, independent of the active index.
  Whitespace-only names and no-op renames create no history.
- Rejection: invalid IDs, deleting the last page, zero/negative/non-finite sizes,
  invalid moves and invalid reorder requests leave document/history unchanged.
  Follow existing document-size validation rather than introducing new limits.
- Persistence: page IDs, order, names, positions, sizes round-trip;
  active page and frame-only view state do not leak into shared document state.
  Test the current-format multi-page golden file and format-version validation.
- Collaboration: independent page-field edits on two replicas converge without
  losing the unrelated field; order/add/delete conflicts resolve deterministically
  and still leave a usable page list. Compare both merge directions. Include
  simultaneous edits of a new document's first page and concurrent deletion of
  the two remaining pages; fallback must be deterministic and must not write on read.
- Keyboard and cancellation: F2/Enter/Escape name editing, Alt+Up/Down reorder,
  toolbar reorder alternatives and X/Y frame movement work without drag. Escape
  or pointer cancellation during reorder/frame drag leaves history unchanged.
  The frame-only switch resets on New/Open. Row activation preserves object
  selection and zoom; an intersecting page does not cause panning.

## Execution and reporting

Write executable acceptance tests against the finalized public surface before
reading the implementation diff. Then review the diff for additional error paths.
Run affected-crate checks while investigating; run `CLAUDE.md` §7's complete gate
once after integration, with exclusive ownership of the checkout's build/test
slot. Record each command and result, criterion-to-test mapping, browser evidence,
and any unverified criterion. A missing seam or browser capability is a reported
coverage gap, never a passing result. No production changes belong to the tester.
