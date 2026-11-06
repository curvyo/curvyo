# ADRs for sketch pages

Architect, 2026-11-06. Slice 2 of `0063` (concept A). Frage 157 A and 158 A assumed.

- [ADR 0043](../../docs/adr/0043-sketch-pages-content-by-ownership.md) (new, Proposed,
  `needs-customer` for the document model; design by the customer): one kind-aware membership read
  (criteria 1, 2, 15, 17, 18), the sketch not pickable and moved only by its frame (3, 4, 5, 6),
  Convert to paths writing `kind` Sketch to Drawing (15), Delete with the `0049` clone repair (16,
  17), `place_linked_clone` (8, 11), output through the membership read (18), no format bump (19).
  With 157 B or C, or 158 B or C, its decisions 4 and 5 shrink as its Consequences say; the format
  is unchanged in every case.
- [ADR 0042](../../docs/adr/0042-page-kinds.md) §1, §3, §4: kind register and read rule
  unchanged; amended by 0043 for the one second write of `kind` and for a sketch page's content.
  §8 (only sketch pages own a sketch) and §9 (migration) unchanged.
- [ADR 0024](../../docs/adr/0024-spatial-pages.md) §1, §5: position membership for drawing and
  output pages, the frame move as one commit with the clone-aware translation; amended by 0043 for
  sketch pages.
- [ADR 0026](../../docs/adr/0026-parametric-sketch-nodes-and-workbench.md): claim registers, lazy
  creation, conflict objects and the conversion body (same id, claim cleared) unchanged; page
  deletion and original transforms amended by 0043.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md),
  [ADR 0016](../../docs/adr/0016-clone-overrides.md) §2: clones read the root original's outline
  and style (7); `own_placement` on placed clones (8); `Hold::Whole` holds every clone back in the
  frame move (5).
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): Convert, Delete and Place are one
  step each, restored field by field, including the `kind` register (15, 17); activation and the
  row drag before the drop write nothing (8, 9).

## Feature-local decisions

1. 2026-11-06: **History labels.** New: `place_sketch_clone` ("Place sketch clone"),
   `convert_sketch_page` ("Convert sketch to paths", a document-level page operation),
   `delete_sketch_page` ("Delete sketch page"). Kept: `move_page`, `resize_page`, `unlink_clones`.
   The `0020` criterion 41 label guard gets the three new entries.
2. 2026-11-06: **CPU budget** of criterion 20 confirmed at 16 ms thread CPU per pointer move, in
   `curvyo-editor-wasm/tests/cpu_time/`, measuring the session's move call and the canvas view
   built for that frame. Neither drag re-solves the sketch: the frame drag changes only the
   placement (the graph fingerprint matches, the checkpoint is reused), and the ghost outline is
   resolved once when the row drag begins and only translated per move.
3. 2026-11-06: **Press order** for criterion 2: hit-test pickable drawing objects first (the owned
   sketch is not pickable, ADR 0043 §2); only a press that hits nothing activates the page under it
   by the `0098` criterion 8 rule. The same page-under-pointer rule picks the drop target of the
   row drag where frames overlap.
4. 2026-11-06: **Page-row drag plumbing, shared with `0100`.** Built once by whichever slice merges
   first; the other adds one enum variant and its drop command, nothing else:
   - front end: the drag-out of a row lives in `PagesBlock.tsx` next to the reorder drag (a release
     inside the list still reorders); one hook `frontend/src/hooks/usePageRowDrag.ts` forwards
     pointer positions in canvas coordinates to the session and draws the badge from its answer;
   - session: `curvyo-editor-wasm/src/session/row_drag.rs` holds the drag as view state
     (`RowDrag { source: RowDragSource, ghost, at }`), with `row_drag_begin(page)`,
     `row_drag_move(point)` returning the badge kind, the target page and the snapped ghost
     offset, `row_drag_drop()` and `row_drag_cancel()`; `RowDragSource` is an enum (`Sketch` here,
     `DrawingPage` in `0100`), no trait; snapping is the Select-move snapping of `0060` with the
     ghost box as the moving box;
   - document-core: no shared code; each source has its own one-commit drop command
     (`place_linked_clone` here).
   Badge words are front-end strings keyed by the badge kind (UX notes). Nothing is written before
   the drop; Escape or a drop on a sketch page calls `row_drag_cancel`.
5. 2026-11-06: **"Has geometry"** (criteria 8, 10, 11, 15) means the solved sketch has at least one
   non-construction entity, so a clone, a ghost and a conversion draw something. A sketch with only
   parameters or only construction geometry counts as empty for these buttons and badges
   (Proposal; see spec issue 6).
6. 2026-11-06: **Usage count** (criteria 12, 16) counts every clone whose chain's root original is
   the sketch (direct and chained), by `clone_link` read, with its pages from the ADR 0043
   membership read.

## Spec issues (for the product owner)

1. Criterion 18 (a) asks for "closed loops"; `0024` criterion 138 writes a sketch as one element
   per entity, no merging, and its golden `export_sketch` pins that. Proposal: 18 (a) says "as
   `0024` criterion 138"; no export change in this slice.
2. Criterion 18 (c) holds only for **Active page only** (`0024` criterion 130): a whole-plane
   export writes every object at its coordinates (criterion 128), so the circle is in the file
   and Inkscape shows it inside the sketch page's frame. Proposal: (c) and the "page holds
   exactly" of (a) are stated for Active page only with the sketch page active.
3. Criterion 18 (d): a whole-plane export writes a clone as a `<use>` where `0024` criterion 59
   allows it, a path otherwise; only with Active page only on the drawing page is the clone a path.
   Proposal: "as `0024` criteria 59 and 130 (e) write it".
4. "or prints" in criterion 18: there is no print command (`0059` lists printing as out of scope).
   Proposal: drop print from this slice.
5. Criterion 4 forbids selecting the sketch "in no context", but Out of scope keeps "the Layers row
   of a sketch on drawing pages as built", which selects it. Proposal: a click on that Layers row
   activates the sketch page like Select original (criterion 13); nothing is selected.
6. Criteria 8, 10, 11, 15 "with geometry" / "empty": a sketch with parameters but no entity, or only
   construction lines, is neither empty (`0098` criterion 3) nor drawable. Proposal: feature note 5;
   the badge of criterion 10 then also covers it.
7. Criterion 12 with clones on the pasteboard: "on 2 pages" does not count them, and Select clones
   has no page to activate if all lie on the pasteboard. Proposal: the line reads "Used by 3 linked
   clones on 2 pages" counting pasteboard clones only in the first number; Select clones then keeps
   a drawing page active (the last active one) and selects the pasteboard clones.
8. Criterion 4 makes ADR 0026's pending original-scaling question unreachable (no command
   transforms an original). The spec lists it as out of scope; it should say it is closed as "not
   offered" unless the customer reopens it.
9. Criterion 17: a clone a peer creates while the page is deleted elsewhere becomes an orphan and is
   removed (`0049` criterion 35), not turned into a path. Proposal: add one sentence; it cannot
   occur without a shared room.
10. Criterion 1 test wording: "belongs to its sketch page only" also needs the sketch overlapping a
    drawing page to be no member of that drawing page (ADR 0043 §1); suggest the test place the
    sketch across a neighbouring drawing page's edge.
