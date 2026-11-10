# Page auto-layout: a page that changes size or is deleted moves its neighbours, and objects go with their page

Status: Ready (2026-11-10; Fragen 146 to 151 answered 2026-11-10; builds after PR #311 `story/formats-panel-restyle` is merged)
Priority: Should
Origin: Customer requests 7 and 8 of 2026-11-06; definitions confirmed 2026-11-10 (Frage 144 A, Frage 145 A). Request 7 (translated): when a page changes its orientation, the neighbouring pages move automatically so that no pages overlap, at the gap used when pages are added; today "Page 1 Drawing" turned to Landscape overlaps "Laser 1 Laser". Request 8: an object belongs to the page whose area contains the object's centre and moves with that page; objects whose centre lies in no page stay. Only the page is turned or resized, never the content. PR #311 removed the page X/Y fields from the Pages block; positions are automatic from now on, pages stay arrangeable by drag on the canvas and by reordering as today. Answers of 2026-11-10: a smaller page pulls its neighbours back, so a turn and its inverse return every page (146 B); pages turn about their centre (147 A); a dragged page carries the objects whose centre it holds (148 A, changes `0059` criterion 9); a dropped page pushes away the pages it lands on (149 B); there is no key to lay a page over another on purpose, old overlaps stay (150 A); deleting a page closes the hole: the pages beside it move in like a page that shrinks to nothing, by the same reflow rule, so the distances stay as they were, Undo restores, one History step (151 B, changes `0059` criterion 6).

Choices marked **(Proposal)** are the architect's and vetoable at the demo.

## User value

As a maker with a drawing page and a laser page side by side, I want to turn or resize one page and have the others keep their distance to it, so that I never have to untangle overlapping pages by hand, turning back puts everything where it was, and the drawing on each page stays on its page.

**What other tools do.** Inkscape (1.2 and later) and Affinity let pages overlap after a resize and leave the tidying to the user (Inkscape offers a separate re-arrange). Illustrator's artboards overlap too until "Rearrange All Artboards" lays them out again from scratch. **We do better:** the arrangement follows the change at once, keeps every distance the maker chose, and turning back restores it.

## Words used below

- **Gap:** the distance Add page leaves after a page today (`0059` Frage 2 A), `max(20 mm, 10 % of that page's larger side)`.
- **Size change:** any command that changes a page frame's width or height: Portrait/Landscape, a typed Width or Height, a format, Fit page, a laser profile pick.
- **Beside an edge:** a page lies beside the right edge of the changed page when it lies wholly to the right of it (touching counts) and overlaps, top to bottom, the changed page's height. That height is taken from the frame, before or after the change, that reaches farther to the right. Likewise for the other three edges.
- **Behind:** a page lies behind a moving page when it lies beside that page's far edge in the same way (the moving page's own height or width counts).
- **Distance:** the gap between two page rectangles, 0 when they touch.
- **Close:** what a delete does to the hole it leaves (criteria 35 to 41).
- **Owner:** the page that holds an object's centre (criteria 14 to 20). The object is the top-level object as the maker selects it: a path, shape, text, image, clone or a whole group.

## Acceptance criteria

### Pages keep their distance on a size change

1. Given two pages "Page 1" (A4 portrait, drawing) and "Laser 1" placed by Add page to its right, when Page 1 is switched to Landscape, then Page 1 turns about its centre as today (ADR 0031 9a), its right edge moves 43.5 mm to the right, Laser 1 moves 43.5 mm to the right and is still exactly the gap away, and no two pages overlap. Within 1e-6 mm.
2. Given a page P and a neighbour beside its right, left, top or bottom edge, when that edge of P moves out or in by d (eight tests: four sides, growing and shrinking), then the neighbour moves by d along that axis only and keeps its distance to P exactly, whatever it was: the gap, a distance the maker dragged, 0 (touching), or far away.
3. Given a row of N pages (N = 2, 5, 8) laid out by Add page, when the first, a middle or the last page turns to Landscape, then every page right of it moves right and every page left of it moves left by half the width gain, every page behind them follows, and every distance in the row stays what it was. Also given a row "Coaster" (100 x 100 mm), "Key ring" (50 x 30 mm), "Board" (300 x 200 mm), when Coaster becomes A4 by a format, then Key ring and Board both move right by 55 mm in the same row, each keeping its distance; neither goes to a new row.
4. Given a page that gets smaller (Landscape back to Portrait, a smaller format), then the pages beside its edges that moved in, and the pages behind them, move towards it by the same amount and keep their distance to it (Frage 146 B).
5. Given rows and grids laid out by Add page with pages of one format (1 x 2, 1 x 8, 2 x 8), when any page turns Portrait, Landscape, Portrait (and Landscape, Portrait, Landscape), then after the second turn every page frame is where it started, within 1e-6 mm, and no two pages overlap after either turn. Every object is where it started too, except an object whose centre a moved page came to cover in the first turn: one on the pasteboard in a gap a pushed page now covers (criterion 15), or one of the changed page in the strip a pulled-in neighbour now covers (criterion 17); it belongs to that page from then on and leaves with it in the second turn. Named case: in the grid [A B / D E], when A turns to Landscape, then B moves right, D (under A) moves up and E does not move; when A turns back, everything is where it started.
6. Given a grid [A B C / D E F] laid out by Add page, when B turns to Landscape, then A moves left and C moves right by half the width gain, E (under B) moves up by half the height loss and keeps its distance to B, D and F do not move, and the second row no longer has a common top edge (follows from Fragen 146 B and 147 A); when B turns back, everything is where it started.
7. Given a page that is not beside any edge that moves and not behind a page that moves (diagonal to the changed page, or in another row and not under it), when the size change runs, then it does not move. A page beside an edge moves also when it is far away and was never in the way (Frage 146 B).
8. Given a page beyond a corner of the changed page that, after a growth on both axes (Fit page, a larger format), lies beside both edges, then it moves on both axes and keeps both distances.
9. Given a page that overlaps the changed page before the change (an old file, a merge), when the size change runs, then it does not move. Two other pages that overlapped before are not treated as a new overlap by criteria 10 and 11; each moves by its own chain or stays.
10. Given a page that moves back towards the changed page (criterion 4) and would come to overlap a page that does not move with it, then it and every page moving with it stop where the two just touch; no new overlap (Proposal). Turning back then does not restore that part exactly; one Ctrl+Z does.
11. Given a constructed case in which the moves would still leave two pages overlapping that were apart, then the page of the pair that is not the changed page (the later one in the page list if neither is) goes to the start of a new row below the lowest page. No two pages that were apart overlap after any size change.
12. Given the same project and the same change on two devices, then the result is identical (the layout is a pure function of the page frames in list order).
13. Given no page beside an edge that moves, when the size change runs, then nothing but the changed frame is written (as today).

### Objects go with their page

14. Given an object whose centre lies inside a page that moves (criteria 1 to 11, 21 to 23), then it moves by exactly the same distance in the same step; its shape, size and rotation do not change.
15. Given an object whose centre lies in no page (the pasteboard), then it never moves when pages move, even if part of it overlaps a page that moves.
16. Given an object that straddles two pages, then it moves with the page that holds its centre and only with that one. A group moves as a whole by the centre of the whole group; its members are never torn apart.
17. Given the page that changes size itself, then nothing on it moves (only the page is turned or resized; content is never rotated or scaled: rotating content with the page is **not part of this story**). Given an object of that page near an edge that moves in, when a neighbour is pulled in so that it now holds the object's centre, then the object stays where it is, belongs to that neighbour from then on and moves with it at its next move (no special case).
18. Given an object whose centre lies in a drawing page and a laser page that overlap, then it belongs to the laser page (as `0106` criterion 1); centre in two drawing pages or two laser pages, it belongs to neither and stays (Proposal). A centre exactly on a frame edge counts as inside (1e-9 mm).
19. Given a sketch page that moves, then its sketch moves with it and every linked clone of the sketch elsewhere stays (ADR 0043 §3); a drawing object lying over a sketch page belongs to no page and stays. Given a laser page that moves, then its placed pages, clones, own items and fixtures move with it. Given a linked clone on a page that moves whose original is on a page that stays, then the clone moves and the original stays; the reverse holds too.
20. Given hidden or locked objects, guides and dimensions on a page that moves, then they move with it like any other object (Proposal: moving a page is not an edit of its objects).

### Dragging a page

21. Given a page with objects on it and over its edge, when it is dragged on the canvas by its edge, then every object the page owns (criteria 14 to 20) moves with it, also one hanging over its edge; an object whose centre lies outside the page stays, even when part of it lies on the page. This replaces "only what lies wholly on it" of `0059` criterion 9 (customer, Frage 148 A, 2026-11-10). The drag preview shows that set moving; it is fixed when the drag starts.
22. Given a page dropped so that it overlaps a page it did not overlap when the drag started, then that page moves away, in the one of the four directions (right, left, down, up, in that order on a tie) that needs the shortest move, to exactly the dropped page's gap, carrying its objects; a page it is pushed onto moves on in the same way, in a chain; the dropped page stays exactly where it was dropped (Frage 149 B; direction and gap Proposal). If pushing cannot settle it, the pages still overlapping go to the start of a new row below the lowest page.
23. Given a page dragged close to another page, when it is dropped without overlapping it, then no other page moves (Proposal). No page moves into the place the dragged page left.
24. Given a page that already overlapped another page when the drag started (for example a laser page laid over a drawing page in an old file), when it is dropped, then the drop never pushes that page because of that overlap (Proposal); it pushes other pages as in criterion 22. A page cannot be laid over another on purpose; there is no key or mode for it (Frage 150 A, 2026-11-10).
25. Given the per-user "move frame alone" switch on a page, when that page is dragged, then the dragged frame moves alone as today; pages it pushes, and pages moved by a size change, always carry their objects (Proposal).

### One step

26. Given any size change that moves pages, then the size change, every moved page and every moved object are one history step with the size change's existing name ("Resize page", "Fit page to content", the profile change); one Ctrl+Z restores every frame and every object exactly, one Ctrl+Shift+Z repeats it.
27. Given a drag that pushes pages, then the drag, every pushed page and every moved object are one "Move page" step; one Ctrl+Z restores everything.
28. Given the moves, then nothing else is written: no new field in the file, `format_version` stays 39, a file saved after an auto-layout opens in today's build unchanged.

### Other page commands

29. Given Add page, then the new page goes where Add page puts it today; if that place is closer than the gap to a page (one the maker dragged there), the new page goes to the start of a new row below the lowest page instead. Existing pages never move on Add page (Proposal). The same holds for Add laser page.
30. Given Delete page, then the hole it leaves closes by criteria 35 to 41 (Frage 151 B, 2026-11-10; changes accepted behaviour of `0059` criterion 6, where no other page moved); what happens to the deleted page's own content is unchanged (criterion 36).
31. Given reordering the page list, then no page moves (`0059` criterion 5).
32. Given a typed Width or Height, a format from the formats list, Fit page or a laser profile pick, then the same auto-layout runs as for an orientation change (Proposal: one rule for every size change). A laser profile pick keeps the profile's origin corner (ADR 0042 §6; amended 2026-11-10 to the code), so with a right or bottom origin the pages beside the left or top edge move too.
33. Given a page the maker dragged earlier, then a later size change of another page moves it by the same rule as every other page (criteria 2 to 11): it keeps its own distance to the changed page.
34. Given a project with 200 pages, then one orientation change, one drop that pushes, or one delete that closes a hole does not stall: the test only guards against a regression (500 ms thread CPU in a debug build, amended 2026-11-11 after the CI runner failed the first limit of 100 ms; the measured 38 to 66 ms are tracked in `docs/technical-debt.md`, there is no hard limit of the customer).

### Deleting a page closes the hole

35. Given three pages A, B, C (A4 portrait, 210 x 297 mm) laid out by Add page in a row, when B is deleted, then C moves 239.7 mm to the left (B's width 210 mm plus C's distance 29.7 mm to B) and ends exactly 29.7 mm right of A, the distance A had to B; A does not move. Within 1e-6 mm (Frage 151 B).
36. Given a deleted page, then its own content is handled exactly as today and none of it moves with the closing pages: a drawing page's objects, also those whose centre it held, stay where they lie on the pasteboard with their ids, transforms and layers (`0059` criterion 6), and its placed pages on laser pages are removed in the same step (`0100` criterion 24); a laser page's machine copy, fixtures and placements go and the objects made on it stay on the pasteboard (`0098` criterion 16); a sketch page's sketch goes, each linked clone of it elsewhere becomes paths in its place, and drawing objects over it stay (`0099` criterion 17). Given a closing page that comes to lie over such left-behind objects, then they stay where they are, belong to that page from then on and move with it at its next move (no special case, as criterion 17).
37. Given a page that moves to close the hole, then it carries every object it owns by criteria 14 to 20, read before the delete: its drawing, also an object hanging over its edge; a sketch page its sketch; a laser page its placed pages, clones, own items and fixtures. An object on the pasteboard, or one whose centre lies on a page that does not move, stays.
38. Given a deleted page with a page beside its left or right edge (it is in a row), then the pages beside its right edge and the pages behind them move left by its width plus the distance of the nearest of them to it, so the nearest one starts where the deleted page started; every distance among the moving pages stays what it was (the Add page gap, a dragged distance, 0). Given a deleted page with no page beside its left or right edge (it is in a column), then the pages beside its bottom edge and behind them move up the same way by its height plus that distance. Pages beside its left or top edge never move, nor, for a page in a row, the pages above or below it (Proposal: the hole closes from behind, along the row first).
39. Given a grid [A B C / D E F] laid out by Add page, when B is deleted, then C moves left into B's place and D, E, F do not move; when C is deleted, then no page moves (the last page of a row); when E is deleted, then F moves left into E's place; when A is deleted, then B and C move left so that B starts where A started (the first page of a row; follows from Frage 151 B, Proposal). Given a page with nothing beside its right edge (row) or bottom edge (column), then nothing but the delete is written (as today).
40. Given a page that overlapped the deleted page (an old file, a merge), then it does not move. Given a page that moves in and would come to overlap a page that does not move with it, then it and every page moving with it stop where the two just touch (as criterion 10); a pair still overlapping goes by criterion 11 (the later page of the pair in the page list goes to the new row). No two pages that were apart overlap after a delete, and the result is the same on every device (criterion 12).
41. Given a delete that moves pages, then the delete, every moved page and every moved object are one history step with today's name ("Delete page", or "Delete sketch page" for a sketch page); one Ctrl+Z restores the page with its content as criterion 36 says, every moved frame and every moved object exactly; one Ctrl+Shift+Z repeats it. Nothing new is written (criterion 28).

## Out of scope

- Rotating or scaling content with the page on an orientation change (request 8: only the page turns).
- A "tidy all pages" command, and re-layout after a merge of concurrent edits from another device (see ADR 0051 §8 and `docs/technical-debt.md`).
- A way to lay a page over another on purpose (Frage 150 A, 2026-11-10: not wanted).
- Deleting the objects of a deleted page (they stay as today, criterion 36), and moving the pages before it or in other rows into the hole (criterion 38).
- Using the centre owner for the edit context of `0106` (still "member" by reach).
- Clip groups (`0067` not built): when they land, a clip group is one object.

## Answered (customer, 2026-11-10)

- Frage 146 (PAGE-SHRINK): **B.** A page that gets smaller pulls its neighbours back; they keep their distance to it; Portrait, Landscape, Portrait returns every page to where it started.
- Frage 147 (PAGE-TURN-ANCHOR): **A.** A page turns about its centre, as resizing does today.
- Frage 148 (PAGE-DRAG-CARRY): **A.** Dragging a page carries the objects whose centre it holds; changes accepted behaviour of `0059` criterion 9.
- Frage 149 (PAGE-DRAG-PUSH): **B.** A dropped page pushes away the pages it lands on, by the same ownership rule, in one step with the drag.
- Frage 150 (PAGE-DROP-OVER): **A.** A drop always pushes; there is no key to lay a page over another on purpose; overlaps from before stay.
- Frage 151 (PAGE-DELETE-CLOSE): **B.** Deleting a page closes the hole: the pages beside it move in like a page that shrinks to nothing, by the same reflow rule as a size change, so the distances stay as they were; Undo restores; one History step. Changes accepted behaviour of `0059` criterion 6.

## Open questions

None.

## Links

Requirements: R-EDIT-051, R-EDIT-052 (new); R-EDIT-038
Depends on: `specs/0059-multiple-pages/`, `specs/0098-page-kinds/`, `specs/0099-sketch-pages/`, `specs/0100-bring-content-in/`, `specs/0106-move-versus-edit/`; PR #311 (`specs/0045-document-formats-library/ux-notes.md` section 6, on that branch until it merges)
Changes: `specs/0059-multiple-pages/` criterion 9 (what a drag carries), its free drop and criterion 6 (a delete closes the hole) (dated notes there)
ADRs: [ADR 0051](../../docs/adr/0051-page-auto-layout-and-ownership-by-centre.md); see [adrs.md](adrs.md)
UX: [ux-notes.md](ux-notes.md)
PR: -
