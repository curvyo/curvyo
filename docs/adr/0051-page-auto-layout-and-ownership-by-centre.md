# ADR 0051: Page auto-layout on a size change, a drop or a delete, and object ownership by centre

**Status:** Proposed (2026-11-10). The behaviour is the customer's (requests 7 and 8 of
2026-11-06, definitions confirmed 2026-11-10, Fragen 144 A and 145 A; answered 2026-11-10:
146 B PAGE-SHRINK, 147 A PAGE-TURN-ANCHOR, 148 A PAGE-DRAG-CARRY, 149 B PAGE-DRAG-PUSH,
150 A PAGE-DROP-OVER, 151 B PAGE-DELETE-CLOSE). The rule, the placement of the code and the
history shape below are the architect's **Proposal**. 148 A, 149 B and 151 B change accepted
behaviour of `0059` (criterion 9, the free drop, and criterion 6 where no other page moved); the
customer decided it. No document-model or format change, so not `needs-customer`; no question of
`specs/0109-page-autolayout` is open. Amends, by notes on their Status lines:
[ADR 0024](0024-spatial-pages.md) §1, §5 (a size change, a drop or a delete also moves other
frames and their content) and
[ADR 0043](0043-sketch-pages-content-by-ownership.md) §1 (the membership read gains the centre
owner). Spec: `specs/0109-page-autolayout`.

## Context

Since PR #311 a page has no typed X/Y; positions are automatic. A page whose size changes
(Portrait/Landscape, a typed side, a format, Fit page, a laser profile pick) is written by
`resize_page`, `fit_page_to_content` or `set_manufacturing_profile`: the frame alone, about its
centre (ADR 0031 9a) or its origin corner (ADR 0042 §6). Nothing else moves, so "Page 1" turned
to Landscape overlaps "Laser 1" beside it. The customer wants the neighbours to keep their
distance to the changed page in both directions (Portrait, Landscape, Portrait returns every page
to where it started), each object to travel with the page whose area holds its centre, a dragged
page to carry its objects by the same rule, a dropped page to push away the pages it lands on
(with no key to lay it over another on purpose, 150 A), and a deleted page's hole to close: the
pages behind it move in by its width or height plus their distance to it, like a page that
shrinks to nothing, so the distances stay as they were (151 B). Today a frame drag carries the
leaves wholly inside the frame (`0059` criterion 9, `Reach::Inside`, its only product caller is
`contained_page_objects` in the editor session), and neither a drop nor a delete moves another
page. `delete_page` removes the frame and keeps every drawing object; a sketch page takes its
sketch (`delete_sketch_page`, own label, ADR 0043 §5), a laser page its manufacturing node in the
same commit (ADR 0042 §5), and a drawing page's placements on laser pages go with it (`0100`
criterion 24).

Fixed facts from the code: frames are one register each (`page.<id>.frame`); placements and
fixtures are page-local under the manufacturing node and follow a frame write by construction
(ADR 0042 §4); a sketch moves only with its frame, clones held back (ADR 0043 §3);
`translate_page_objects` translates leaves with every downstream clone held back; membership
reads live in `curvyo-ui-core/src/page_membership.rs` and nowhere else (ADR 0043 §1);
`next_page_origin` gives a gap of `max(20 mm, 10 % of the last page's larger side)`. Pages have no
hidden or locked state. Clipping (`0067`) is not built.

### Options: what moves on a size change

- **A. Push only.** A page moves only when it would overlap or crowd a changed or moved page; a
  smaller page pulls nothing back. Rejected by the customer (146 B): a turn and its inverse do not
  return the pages.
- **B. Edge shift (chosen).** The pages beyond each edge of the changed page, across that edge,
  and the pages behind them, move by that edge's change; every distance among them and to the
  changed page stays what it was. A smaller page pulls them back.
- **C. Re-lay out every page from the list order.** Throws away every page the maker dragged.
  Rejected.

### Options: which pages lie "across an edge"

The test needs the changed page's extent across the edge (its y range for the right edge), and the
frame changes.

- **A. The union of the before and after extents.** Safe for growth, but in a two-row Add page
  grid [A B / D E] with A turned to Landscape, E (under B) counts as under A because A got wider:
  E is pulled up into B, a cap fires and the commonest layout loses its round trip. Rejected.
- **B. The extent of the before frame.** Not symmetric: the inverse change picks other pages.
  Rejected.
- **C. The extent of whichever frame, before or after, reaches farther out on that side
  (chosen).** Symmetric under the inverse change, so the inverse picks the same pages. In the grid
  above, A's bottom edge moves up, the portrait frame reaches farther down, its width picks D
  only; E stays, and the inverse restores everything. A page outside that extent cannot be
  overlapped by the frame that reaches farther, so growth needs no gap test.

### Options: how a delete closes the hole

- **A. The reflow (2) with the deleted frame shrunk to nothing about its centre.** The pages on
  both sides move in by half its width and a double distance stays between them; the pages before
  it move too. Rejected: 151 B moves what is behind the page, by its width plus their distance.
- **B. Close along both axes.** The pages to the right move left and the pages below move up,
  each into the hole; in a grid both chains land in the same place. Rejected.
- **C. One-sided close from behind, along the row, else along the column (chosen).** The chain
  beside the right edge moves left by the width plus its distance, so its nearest page starts
  where the deleted page started; a page in no row closes its column the same way upwards. The
  pages before it never move; the chain keeps its inner distances (the reflow's chains, cap and
  fallback).

### Options: where the rule and ownership live

- **A. Layout as pure functions over frames in `curvyo-document-core`; ownership as the centre
  owner in `curvyo-ui-core::page_membership`, read by the session before the command; one
  document-core command writes everything (chosen).** Same split as `move_page`, whose caller
  passes the carried leaves today.
- **B. Both in document-core.** Document-core has no resolved bounds of picture clones and texts
  (ui-core does) and would hold a second membership test, against ADR 0043 §1. Rejected.
- **C. In the editor session.** Not testable without wasm glue, not reusable by a later server or
  CLI. Rejected.

### Options: stored or computed ownership

- **A. Computed on demand from the state before the command (chosen).** No register, no format
  change, nothing to go stale.
- **B. A stored `page` register per object.** Stale after every move, merges badly, needs a format
  bump; rejected for the reasons of ADR 0043 option C.

## Decision

1. **Triggers.** `resize_page` (typed side, format, orientation swap), `fit_page_to_content` and
   `set_manufacturing_profile` run the reflow (2) in their own commit. The changed frame is
   whatever the command computes today (centre, ADR 0031 9a; origin corner, ADR 0042 §6); the
   reflow never moves the changed page. `move_page` (a drag) runs the drop push (3).
   `delete_page`, on every path (a sketch page included), runs the close (3a). Add page, rename
   and reorder run none of them. A single-page `resize` is unchanged.
2. **Reflow on a size change** (`curvyo-document-core/src/page_layout.rs`, pure,
   `reflow(frames, changed id, new frame) -> Vec<(page id, Vec2)>`, frames in list order; all
   tests on the frames before the step, within 1e-9 mm, `pages::EPS`).
   - *Edge deltas:* `dR = x1' - x1`, `dL = x0' - x0`, `dB = y1' - y1`, `dT = y0' - y0`. An edge
     with a delta under 1e-9 mm moves nothing.
   - *Extent of an edge:* the changed page's range on the other axis, taken from whichever frame,
     before or after, reaches farther out on that side (option C above).
   - *Direct members:* a page beside the right edge lies wholly right of the changed page's
     before frame (`x0 >= x1 - EPS`, touching counts) and its y range overlaps the right edge's
     extent by more than 1e-9 mm. Likewise for the other three edges.
   - *Followers:* a page joins a side's chain when it lies wholly beyond a chain page on that side
     and overlaps that chain page's own range across it, transitively.
   - *Shift:* every page of the right chain moves by `dR` on x, of the left chain by `dL`, of the
     bottom by `dB` on y, of the top by `dT`. All distances inside a chain, and every chain page's
     distance to the changed page, stay exactly what they were: the Add page gap, a dragged
     distance, 0 (touching) or far. No page is in two chains on one axis (a right-chain page
     starts at or after the changed page's right edge, a left-chain page ends at or before its
     left edge, and a frame has positive width). A page in an x chain and a y chain (beyond a
     corner, across both extents) moves on both axes and keeps both distances.
   - *Overlap before:* a page that shared area with the changed page before the step is in no
     chain. Pairs that shared area before the step are exempt from the cap and the fallback.
   - *Cap (pulls only):* a chain moving towards the changed page moves by the largest amount, at
     most its delta, at which none of its pages shares area with a page outside it that it did not
     share area with before. Chains are capped in the order right, left, bottom, top, each against
     the changed page's new frame and the others as already shifted. Growth is never capped. No
     "closer than the gap" test: it would contradict distance preservation.
   - *Fallback:* any pair that shares area after the shifts and did not before: the page that is
     not the changed page (the later in list order if neither is) goes to the start of a new row
     below the lowest page (the `next_page_origin` wrap rule), in list order.
   - *Bounded and deterministic:* chains are one closure each (every page added once), a cap is one
     number per chain, the fallback one pass: O(pages²), no iteration, a pure function of its input.
   - *Round trip:* an orientation turn followed by the turn back on the same page, with nothing
     between, restores every frame exactly when the first step capped no chain, sent no page to the
     fallback, and no two pages shared area before it. (Corrected 2026-11-10 after the property test of
     0109: a resize that grows or shrinks both sides, or one that moves a page beyond a corner, does not
     always undo exactly; only turns are guaranteed and tested.) The extent and the beyond-edge test are the
     same both ways and chains keep their inner distances, so the inverse picks the same chains
     with opposite deltas; they return to places that were clear, so it cannot cap. Add page rows
     and grids of one format always qualify. Objects return with their pages, except an object whose
     centre a moved page came to hold in the first step (on the pasteboard in a gap a pushed page
     now covers, or of the changed page in the strip a pulled-in neighbour now covers), which
     leaves with that page in the second.
3. **Drop push** (`page_layout.rs`, pure, `push_apart(frames, dropped id, new frame, frames at
   press) -> Vec<(page id, Vec2)>`). A worklist starting with the dropped page; each page taken
   checks the others in list order. Page Q conflicts with page P when they share area now and did
   not at press (pairs overlapping at press are exempt, so a laser page lying over a drawing page
   can be nudged). Q moves in the one of the four directions (right, left, down, up, in that order
   on a tie) that needs the shortest move to end exactly P's gap from P (`max(20 mm, 0.1 × P's
   larger side)`), and is re-entered. A page is pushed again only in the direction of its first
   push; a push in another direction, or `pages²` moves, stops the pushing and every page still
   in conflict goes to the fallback row, in list order. The dropped page never moves; nothing is
   pulled into the place it left; a drop that only comes close (no shared area) moves nothing
   (trigger and gap **Proposal**). Pushes are computed on release, not during the drag. There is
   no modifier that drops a page over another without pushing (150 A).
3a. **Close on delete** (`page_layout.rs`, pure, `close_gap(frames, deleted id) ->
   Vec<(page id, Vec2)>`, frames before the delete in list order; tests as in (2)).
   - *Axis:* the row when any page lies beside the deleted frame's left or right edge (wholly left
     or right of it, touching counts, its y range overlapping the deleted frame's by more than
     1e-9 mm), else the column. Only the row closes in a grid; the pages below never move up into
     a row.
   - *Chain:* the pages beside the deleted frame's right edge (row) or bottom edge (column) and
     their followers, as in (2), with the deleted frame's own range as the extent. The pages beside
     its left or top edge never move.
   - *Shift:* `-(w + d)` on x for a row (`w` the deleted frame's width, `d` the smallest distance
     from it to a direct member of the chain), `-(h + d)` on y for a column. The nearest page then
     starts where the deleted page started; every distance inside the chain stays. A deleted page
     that ends a row (or a column with no row) has an empty chain: nothing moves. One that starts a
     row moves the rest of the row to its left edge.
   - *Overlap before, cap, fallback:* as in (2): a page that shared area with the deleted frame is
     in no chain; the chain is a pull, capped where it would first share area with a page outside
     it that it did not share area with before; a pair still sharing new area goes to the fallback
     row, the later page in list order of the pair. The deleted frame takes no part in the tests
     after the step.
   - *Content:* the chain pages carry the objects whose centre they hold (4, 5). The deleted page's
     own content is handled exactly as today and none of it moves: its drawing objects stay where
     they lie (ownership is read before the delete, and their owner does not move); a sketch page's
     sketch goes and its clones become paths; a laser page's manufacturing node goes; a drawing
     page's placements on laser pages go. A chain page that comes to lie over the objects left
     behind owns them from then on (no special case, as an object near a shrinking edge in (2)).
4. **Ownership by centre** (amends ADR 0043 §1; `PageMembership::owner_by_centre`). The unit is
   the top-level object: the outermost non-layer node (a path, primitive, text, image, clone or a
   whole group; a clip group when `0067` lands). Its centre is the centre of the union of its
   drawn leaves' outline boxes (`object_outline_bounds`, stroke excluded). It is owned by the page
   whose closed frame (`Page::contains`) holds the centre, among drawing, unknown-kind and output
   pages (own items); sketch frames do not exist for it (ADR 0043 §1). Centre in several such
   frames: an output page wins over a drawing page (the `0106` `home_of` precedence); still
   several, no owner. No owner: it stays. All leaves of an owned unit move, hidden, locked, guides
   and dimensions included; clones elsewhere are held back (`translate_page_objects`). The same
   owner decides what a drag carries (148 A): the session passes `owner_by_centre`'s contents to
   `move_page` and `Reach::Inside` loses its only product caller and goes.
5. **Members by ownership.** A moved sketch page carries exactly its sketch, held back as ADR
   0043 §3. A moved output page carries its placements and fixtures by the frame write, never by
   the translation. A drawing object over a sketch page has no owner and stays. The per-user
   "frame alone" switch applies to the dragged page only; a reflowed or pushed page always carries.
6. **One history step.** One `batch` holds the changed, dropped or deleted frame, every moved
   frame and every moved page's translation, under the triggering command's existing label
   (`resize_page`, `fit_document_to_content`, the profile label, `Move page`, `delete_page`,
   `DELETE_SKETCH_PAGE_COMMIT_LABEL`); no new label. On a delete the close joins the commit that
   already removes the sketch, the manufacturing node or the placements. Undo restores all of it
   field by field (ADR 0014). The ownership read is taken from the state before the batch (at
   press for a drag).
7. **Add page** keeps `next_page_origin`; if that spot shares area with a page Q, or lies closer
   to Q than Q's gap, the new page goes to the start of a new row below the lowest page. Existing
   pages never move on Add.
8. **Sync.** Ordinary frame and transform writes; no wire, register or label change. The layout
   runs only inside a local command, never on read, open or merge. Two peers moving pages
   concurrently can merge into an overlap, which stays until the maker drags it. Recorded in
   `docs/technical-debt.md`.
9. **Format.** No `format_version` change; main stays at 39. No golden changes.
10. **Crates.** No new crate, trait or dependency. `document-core`: `page_layout.rs` (`reflow`,
    `push_apart`, `close_gap`, the shared fallback row and gap helper, which `next_page_origin`
    also calls), the five commands (the four of (1) and `delete_page`) taking the caller's
    `PageContents` (page id → owned leaves). `ui-core`: `owner_by_centre`, top-level unit bounds.
    `editor-wasm`: computes the contents before each size command and delete and at drag press.

## Consequences

- Turning a page about its centre moves the pages on both sides: a Landscape turn pushes the
  pages left and right by half the width gain and pulls the page above or below by half the
  height loss, so rows lose their common top edge. Accepted with 147 A and 146 B.
- A neighbour pulled in towards a page that shrinks can land over drawing of that page near the
  edge (content never moves with the changed page); that drawing then belongs to the neighbour.
  Visible after the change and undoable; it follows from 146 B, 147 A and request 8.
- A neighbour far away beside an edge moves too (146 B); a page just outside an edge's extent
  does not, and may end closer than the gap (never overlapping) after a growth.
- A page can no longer be dropped over another to overlap it, and there is no key for it (150 A);
  overlaps from before (old files, merges) stay and can be dragged.
- Deleting a page moves the pages behind it in its row (or column); deleting the first page of a
  row shifts the whole rest of the row. The page that moves in can land over the drawing the
  deleted page left on the pasteboard and owns it from then on. A delete and its undo restore every
  frame; a delete followed by Add page does not.
- A capped pull or a fallback breaks the exact round trip; both are visible and undoable.
- A moved drawing page that is the source of a placed page keeps the placed picture unless a
  straddling object stays behind; then the placed picture changes. Visible and undoable.
- Three membership reads remain (reach, `home_of`, centre owner); `Reach::Inside` goes.
- Cost: O(pages²) per size change and per delete, at most `pages²` pushes on a drop; 200 pages
  stay well under a frame.
- Not decided here: re-layout on merge, a "tidy pages" command, ownership for the edit context.

## Verification

`page_layout` unit tests for `reflow`: each edge growing and shrinking moves the page beside it by
the delta and keeps its distance (gap, dragged, touching, far); N pages in a row (2, 5, 8) turned
at the first, a middle and the last page; Coaster, Key ring, Board in a row stay in the row; the
two-row grid [A B / D E] with A turned to Landscape pulls D up, leaves E, and the inverse
restores all; the grid [A B C / D E F] with B turned moves A left, C right and E up, D and F stay,
the inverse restores all; a corner page across both extents moves on both axes; a page that shared
area with the changed page does not move; a constructed pull capped by a page outside the chain;
a constructed fallback; a property test over random non-overlapping frames: the changed page never
moves, no pair that was apart shares area afterwards, the same input gives the same output, and
when the first step reports no cap and no fallback its inverse restores every frame. For
`push_apart`: a drop on one page, on a row (chain), between two pages, a drop that only comes
close, a pair overlapping at press, the fallback. For `close_gap`: three A4 portrait pages by Add
page, the middle deleted (the last moves 239.7 mm left, 29.7 mm from the first), the first and the
last deleted; a dragged distance kept inside the chain; the grid [A B C / D E F] with B, C and E
deleted; a column; a page overlapping the deleted one; a constructed cap and fallback. Ownership
tests in `page_membership.rs`: centre
inside, outside, on an edge, in two drawing pages, in a laser and a drawing page, over a sketch
page, a straddling group. Document-core tests: Landscape on "Page 1" beside "Laser 1" and a drop
that pushes are one commit each; one undo restores every frame, the moved own items and the
placements byte-identically. Deleting a middle drawing page, laser page and sketch page with
clones is one commit each under its existing label; the deleted page's drawing objects keep their
transforms, the moved pages' objects move, and one undo restores everything byte-identically.
