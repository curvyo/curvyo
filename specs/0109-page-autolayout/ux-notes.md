# UX notes: page auto-layout

Architect, 2026-11-10, written for the lead in place of a ux-engineer pass because the story adds
no control; the ux-engineer reviews the built result. Revised the same day after the customer's
answers (Fragen 146 B, 147 A, 148 A, 149 B, then 150 A and 151 B). Customer rules applied: no popup, no hint labels or
operating tips, no notice after a successful operation, hidden not disabled. Choices marked
(Proposal) are vetoable at the demo.

## 1. Principle

Nothing new to learn: the maker presses Portrait or Landscape (or types a size, picks a format,
fits the page, picks a laser profile), drags or deletes a page exactly as today, and the other
pages keep their distance to it. Turning back puts them back; after a delete the pages behind it
close the hole. There is no switch to turn auto-layout off, no option for the gap and no mode.

## 2. What the maker sees on a size change

- **The changed page** turns or resizes about its centre as today (Frage 147 A). Its drawing does
  not move on screen.
- **Moved pages** appear at their new place in the same frame as the size change, with their
  drawing, placed pages and fixtures on them. On a turn to Landscape the pages left and right
  move apart and the page below (or above) moves closer; on the turn back they return. No
  animation, no flash, no highlight of what moved (Proposal: a jump keeps the frame budget and
  needs no reduced-motion rule).
- **The view** does not pan or zoom; the active page stays active. A moved page may leave the
  visible area; nothing scrolls to it.
- **Page labels** (name and kind chip above the top-left corner) move with their page.
- **Objects that stay** (centre on the pasteboard, a straddling object whose centre is on a page
  that does not move) stay at their place on screen; nothing marks them.
- **The page list** in the Document tab does not change order or content; sizes show the new
  size of the changed page only.
- **Rulers** keep counting from the active page's top-left corner (`0059` criterion 8); the active
  page's corner moves through its own resize, or when it is moved as a neighbour.

## 3. Dragging a page

- **What follows the drag** (Frage 148 A): at press, the objects whose centre the page holds are
  fixed as the moving set; the preview moves the frame and that set by the same delta. An object
  hanging over the edge with its centre inside moves; one whose centre is outside stays, even if
  part of it is on the page. The "move frame alone" switch keeps its look and meaning.
- **Pushing** (Frage 149 B): during the drag no other page moves (Proposal: no live preview of
  the pushes; the drop target stays readable and the preview stays cheap). On release, every page
  the dropped page lands on jumps away to the dropped page's gap, with its objects, in the same
  frame as the drop; a chain of pages follows. A drop that only comes close moves nothing. The
  dropped page lands exactly where it was released.
- **Overlap**: a page can no longer be dropped over another page, and no key allows it (Frage
  150 A, 2026-11-10). A page that already overlaps another (old file, a merge) can still be
  dragged; that overlap does not push.
- Escape, pointer cancellation or a zero delta write nothing, as today.

## 3a. Deleting a page

- The row's trash button keeps its look, tooltip and accessible name ("Delete page: <name>", the
  sketch-page text of `0099` criterion 16); it does not say that pages move (no hint labels).
- On the press (Frage 151 B), the page goes and, in the same frame, the pages behind it in its
  row jump left into the hole with their drawing, placed pages and fixtures; the nearest one
  starts where the deleted page started. A page in a column closes upwards instead. Pages left of
  it or above it, and pages in other rows, stay. No animation, as in section 2.
- The deleted page's own drawing stays on the pasteboard where it lay, as today; a page that moves
  in may now lie over it. Nothing marks it.
- The view does not pan or zoom; the active row moves as today (`0059` ux-notes: the next
  surviving row, or the preceding one at the end).

## 4. Controls

None added, none removed. The Portrait/Landscape buttons, Width and Height fields, the formats
list, Fit page, the Machine block's profile pick, the page edge drag and the row's trash button
keep their look, names and tooltips. No tooltip mentions that pages move (no hint labels).

## 5. Undo and History

- One History row per size change, with today's name ("Resize page", "Fit page to content", the
  profile change), one "Move page" row per drag, pushes included, and one "Delete page" (or
  "Delete sketch page") row per delete, the close included. No new row name, no "Move pages" row.
- Ctrl+Z restores the size, the drag or the deleted page and every moved page and object in one
  press; Ctrl+Shift+Z repeats it.
- The History preview of an older version shows that version's page places (ADR 0024 §5).

## 6. Keyboard

No new keys. The size controls keep their keys; Ctrl+Z and Ctrl+Shift+Z as above. Focus stays on
the control that was used.

## 7. Status region and announcements

- No visible notice, toast or status line text after a size change, a drop or a delete, whether
  pages moved or not (customer rule of 2026-10-31).
- Screen readers (Proposal): the Document section's `sr-only` `role="status"` region
  (`DocumentSection.tsx`) receives "2 pages moved." ("1 page moved.") when a size change, a drop
  or a delete moved other pages, nothing otherwise. Only that region gets the text; its visible notice copy
  stays empty. No new region.
- Failures stay as today (an invalid typed size shows its field message); auto-layout itself has
  no failure the maker sees: a pull that stops short, or a page sent to a new row below (ADR 0051
  §2, §3, §3a), happens without a notice.

## 8. Design-system rows

None change. `docs/design-system.md` "Add page group" already has no position fields after PR #311.
