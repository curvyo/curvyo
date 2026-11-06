# Sketch pages: a sketch page holds only its sketch, and the sketch is used elsewhere as a linked clone

Status: Done (PR #287; Frage 157 A and 158 A accepted with the PR)
Priority: Should
Origin: Customer (2026-11-05, translated): "The Sketch workbench moves to the same model: a Sketch is a page kind. [...] Sketches are positioned only via linked clone, they are fixed on their page." "Objects can be moved anywhere: pasteboard, over a laser page, over a sketch page. Moving always works; editing only in the context of an active vector page or a matching manufacturing page." Slice 2 of `specs/0063-manufacturing-workbenches/` (concept A, `concept-output-pages.md` §2.1, §2.7, §5 flow 9, §9; sketches f and i). Builds on the merged `0098` `page-kinds` (PR #279), which made Sketch a page kind, put the sketch tools on sketch pages and converts old files. Details marked **(Proposal)** are the product owner's and vetoable at the demo.

## User value

As a maker who constructs a part with constraints (a bracket, a box side, a hinge) I want the part to live on its own sketch page and to put it onto my drawings and my laser bed as live clones, so that I can change one dimension once and every copy follows, while nothing else ever gets mixed into the sketch.

**What other tools do.** FreeCAD keeps a sketch in a body and reuses it through a separate placement and "Clone"/"Shape binder" steps; it is powerful but the maker manages three objects for one shape. SolveSpace has no 2D composition at all: one sketch per file, exported as DXF. Inkscape and LightBurn have no constraints, so a changed dimension means redrawing. We do better: the sketch sits on a page of its own (one glyph in the page list says so), and one drag of that page's row onto any drawing or laser page places a live clone that can be turned and scaled there without touching the sketch.

## What 0098 already built, and what this slice adds

Built by `0098`: the Sketch page kind, Add page with Kind Sketch, the sketch rail and tabs on a sketch page, no workbench switcher, the kind chip, sketch size editable like a drawing page's, the format-36 conversion (a sketch on a drawing page moves to a new sketch page and leaves a linked clone).

Added here: what lies on a sketch page besides its sketch, the sketch fixed to its page (frame move, resize), the two ways to place a linked clone of the sketch (page-row drag and a button), reaching the sketch from a clone, what Convert to paths does now, deleting a sketch page with clones elsewhere, sketch pages in SVG export and print, and the sketch state in the page row.

**Split with `0100` `bring-content-in`** (both slices may run side by side; see "Links"):

| Gesture | Slice |
|---|---|
| Drag a **sketch** page row onto a drawing page, a laser page or the pasteboard: linked clone of the sketch | 0099 (criteria 8 to 10) |
| Drag a **drawing** page row onto a laser page: placed page | 0100 |
| Drag an object on the canvas from a drawing page onto a laser page: linked clone, Alt moves, Ctrl copies, cursor badges | 0100 |
| A canvas drag of the whole sketch off its page | 0100, if at all: on a sketch page a press edits sketch geometry, so 0099 gives the page row and a button instead (concept §2.1 "dragging the sketch off its page" is met by the row drag) |
| Move-only selection of foreign objects while a laser or sketch page is active, double-click to switch context for any object | 0100 |
| Double-click on a **clone of a sketch** opens its sketch page | 0099 (criterion 13; it restates `0066` criterion 9 without the switcher) |

The page-row drag out of the list onto the canvas (drag source, ghost, drop on a page) is built once: by whichever of 0099 and 0100 merges first; the other adds its row kind to it.

## Acceptance criteria

### The sketch page holds only its sketch

1. Given a sketch page, then its only content is its own sketch, by ownership and not by position: the sketch belongs to its sketch page even where its outline crosses the frame, and a drawing object (path, shape, text, image, group, dimension or any linked clone, a clone of this very sketch included) lying wholly or partly over the sketch page's frame is not part of that page. For page membership (`0059` criterion 9) such an object counts as lying on the pasteboard, unless it also overlaps a drawing or laser page, to which it belongs as today. It is drawn as drawn, with no notice (Proposal; sketch i). A test places a rectangle wholly inside a sketch page's frame and asserts it belongs to no page, and a sketch whose outline crosses its frame and also the edge of a neighbouring drawing page, and asserts it is a member of its sketch page only and of that drawing page not at all. The membership is one read shared by the frame move, Fit page, export scope and press-to-activate (ADR 0043 §1).
2. Given a drawing page is active and a drawing object lies over a sketch page, when the maker presses on the object with Select (also Shift+press and a marquee started on a drawing page or the pasteboard), then the object is selected and the drawing page stays active; it moves, is styled and is node-edited as a pasteboard object (Frage 156 A). A press inside the sketch page's frame that hits no drawing object activates the sketch page as in `0098` criterion 8.
3. Given a sketch page is active, then the sketch tools never pick, select, move, delete or change a drawing object lying over it: a press, a marquee, Ctrl+A and Delete in the Sketch Select act on sketch geometry only, and a press on such an object reaches the sketch geometry or empty canvas beneath. (Move-only selection of these objects while the sketch page is active is `0100`.)

### The sketch is fixed to its page

4. Given a sketch page with a sketch, then in no context is the sketch a whole object to transform: it has no selection box and no handles, a marquee or Select all on a drawing or laser page never includes it, and move, rotate, scale, skew, flip, align, distribute, arrow keys and the X, Y, W, H and angle fields never apply to it; it cannot be selected from the Layers tab either, and a click on the sketch's Layers row on a drawing page activates its sketch page like Select original (criterion 13) and selects nothing. The Document tab's X and Y of the sketch page move the frame and the sketch together (criterion 5). Inside the sketch page, moving sketch geometry and the origin works as built (`0066` criteria 4, 29). No command transforms an original sketch; ADR 0026's pending question on scaling the original is closed as "not offered" unless the customer reopens it.
5. Given a sketch page, when the maker moves its frame in the plane (the frame-edge drag of `0059` criterion 9), then the sketch moves with the frame by the same offset in the same step, also where its outline lies outside the frame; drawing objects lying over the page stay where they are; no linked clone of the sketch moves (every clone is held back, also one without own placement); and the "move frame alone" switch of `0059` criterion 9 is not shown on a sketch page's row (Proposal). One undo step "Move page".
6. Given a sketch page, when the maker changes its size (Width or Height, a format preset, portrait or landscape), then the sketch keeps its place in the plane and is not re-centred (`0066` criterion 4), no clone moves, and the change is one undo step.

### Using the sketch elsewhere: linked clones

7. Given any linked clone of a sketch, then it follows every edit of the sketch in the same frame under the rules of `0049` and `0057`. A test changes a driving dimension from 80 mm to 100 mm on the sketch page and asserts that a clone on a drawing page and a clone on a laser page (rotated by 12 degrees and scaled by 50 %) change by 20 mm times their scale along the clone's rotated axis, within 0.01 mm.
8. Given a sketch page whose sketch has geometry, when the maker drags its row out of the page list and releases it over a drawing page, a laser page or the pasteboard, then a linked clone of the sketch is created with the centre of its box under the pointer (snapping of `0060` applies as for a Select move, Proposal), with own placement on (`0057`, so moving the sketch page never pushes it), on top of the active layer, and selected; the page under the pointer becomes active (on the pasteboard the active page stays); the sketch page and the sketch are unchanged; the whole is one undo step "Place sketch clone" (Ctrl+Z removes the clone and clears the selection; activation is not a history step). Focus is on the canvas afterwards. Where the active page is a sketch page and the clone lands on the pasteboard, the clone shows a selection box without handles until `0100` adds move-only selection. During the drag the grip of the row is the drag source, the row stays in the list, a ghost of the sketch's solved outline follows the pointer, the page under it gets a frame cue and the badge reads "Linked clone" (sketch i; ghost, badge and cue per UX notes §2). A drag that is released inside the page list reorders as today (`0059` criterion 5); moving in and out of the list switches between reorder and place per pointer move. Nothing is written before the drop. On a laser page the clone is an own item by position, Not assigned (`0098` criterion 11, Frage 155 A).
9. Given that row drag, when it is released over a sketch page (its own or another), then nothing is created and the ghost returns to the row; while the pointer is over a sketch page the badge reads "Not on a sketch page" (Proposal). Escape during the drag cancels in the same way. Neither adds a history step.
10. Given a sketch page whose sketch has nothing to draw, when the maker drags its row onto the canvas, then nothing is created, no ghost is shown and the badge reads "Bracket has no sketch yet" (with the page's name, truncated at 24 characters; Proposal); a drag inside the list still reorders. "Has geometry" in criteria 8, 10, 11 and 15 means the solved sketch has at least one non-construction entity; a sketch with only parameters or only construction geometry counts as nothing to draw for these buttons and badges (Proposal). The row state "Empty" of criterion 14 still follows `0098` criterion 3 (no geometry and no parameter).
11. Given the Sketches tab of a sketch page with geometry, then in its "Use" group it has a button **Place clone on** (Proposal: the keyboard route for criterion 8) that opens an inline list under it (no popup) of every drawing and laser page with their kind chips, in list order, sketch pages absent; choosing a page (click, or Enter or Space) places the clone at that page's centre with the rules of criterion 8, activates that page and puts focus on the canvas; Escape closes the list, changes nothing and returns focus to the button. The button is hidden (not disabled) with an empty sketch or when the project has no drawing or laser page.
12. Given the Sketches tab of a sketch page, then its "Use" group has one line that says how the sketch is used: "Not used on other pages", "Used by 1 linked clone on 1 page" or "Used by 3 linked clones on 2 pages" (Proposal). The count covers every clone whose chain's root original is the sketch (direct and chained); clones on the pasteboard count in the first number only. The line is shown also for an empty sketch. The same count appears as "used by 3 clones" in the tooltip of line 2 of the sketch's page row. With at least one clone, a **Select clones** button (`0049`; "Select clone" for one) activates the page of the first clone in page-list order and selects every clone of the sketch on that page, and puts focus on the canvas; clones on later pages are not selected (accepted limit; they are reached from the clone's page by Select original). If all clones lie on the pasteboard, the last active drawing or laser page stays active and the pasteboard clones are selected. The `0049` refusals for hidden or locked clones apply unchanged and then no page is activated.
13. Given a linked clone of a sketch on any page, when the maker double-clicks it with Select or presses **Select original** (`0049`), then its sketch page becomes active with the sketch tools, the view scrolls to that page only if no part of it is visible (never zooms), no sketch geometry is selected, the clone's own selection is cleared, and focus is on the canvas (restates `0066` criterion 9 without the switcher). Of a selection of clones of two sketches, the first sketch page in list order is activated. **Unlink clone** (`0049`) on a clone of a sketch turns that one clone into ordinary paths in place (curves within 0.01 mm, closed where the sketch's loops close, `0066` criterion 8); the sketch and its other clones are unchanged.
14. Given a sketch page, then its row in the page list shows the sketch's state at the start of line 2, before the size ("Fully constrained · 120 × 80 mm"): "Empty", "Fully constrained", "3 DOF", "Conflict" or "Solver failed" (Proposal; sketch f); shown as glyph and text, never by colour alone ("Conflict" and "Solver failed" at full opacity), and its accessible name spells the state out after the kind ("Bracket, Sketch page, fully constrained", "Bracket, Sketch page, 3 degrees of freedom"). The row updates in the frame the solver reports and announces nothing when it changes.

### Convert to paths (Frage 157, default A)

15. Given a sketch page with a sketch, when the maker presses **Convert to paths** in the Sketches tab, then in one undo step "Convert sketch to paths": the sketch becomes ordinary paths as in `0066` criterion 8 (Béziers within 0.01 mm, closed where loops close; construction geometry, dimensions, constraint symbols, the origin and the axes are dropped); the page becomes a **drawing page** with the same name, size, place in the plane and place in the list, its chip reads Drawing in all four places of `0098` criterion 6, the tool rail and tabs change to the drawing ones in the same frame, and the new paths are selected, the open tab becomes the one last used on a drawing page (else Document) and focus is on the canvas; drawing objects that lay over the page now belong to it by position (`0059` criterion 9); every linked clone of the sketch stays a linked clone, now of those paths, with its outline unchanged within 0.01 mm. Undo gives back the sketch page with the sketch, its constraints, dimensions and parameters, and the sketch tools, with an empty selection; redo converts again. The button is full width, last in the Sketches tab, and hidden (not disabled) while the sketch has no geometry. It converts a sketch in conflict or with a failed solve from the last valid solve. Converting an ownership-conflict object (`0026`) keeps its behaviour and does not change a kind.

### Deleting a sketch page (Frage 158, default A)

16. Given a sketch page whose sketch has three linked clones on two other pages, then the trash button of its row has the tooltip and accessible name "Delete Bracket and its sketch; 3 clones become paths" (no dialog, Proposal; one clone: "1 clone becomes a path"; without clones "Delete Bracket and its sketch"; the count is live and covers pasteboard clones). Other page kinds keep "Delete page: <name>".
17. Given that page, when the maker presses the trash button, then in one undo step "Delete sketch page": the page and its sketch are deleted; each clone becomes ordinary paths in its place with its outline unchanged within 0.01 mm and the style it showed (the `0049` rule for a deleted original); drawing objects lying over the page stay where they are, on the pasteboard (`0059` criterion 6). Undo restores the page, the sketch with all its constraints and parameters, and the three clones as linked clones of it. The last page of a project still cannot be deleted (`0059` criterion 6). A clone that a peer creates while the page is deleted elsewhere becomes an orphan and is removed (`0049` criterion 35), not turned into a path; this cannot occur without a shared room.

### Export

18. Given a project with a drawing page holding a linked clone of the sketch and a sketch page whose sketch has a closed outline with two holes, a construction line, three dimensions and constraint symbols, and a drawing circle lying over the sketch page, when the maker exports SVG (`0024` slice 4), then (a) with **Active page only** and the sketch page active, the export holds exactly the sketch, written as `0024` criterion 138 writes a sketch (one element per entity, curves within 0.01 mm) in the sketch's own style, the style a clone without own style shows; (b) construction geometry, dimensions, constraint symbols, the origin, the axes and the state line are in no export; (c) with Active page only on the sketch page the circle is not in the file (it lies on the pasteboard, criterion 1); a whole-plane export writes every object at its coordinates (`0024` criterion 128), so the circle is in the file there and shows inside the sketch page's frame; (d) with Active page only on the drawing page the clone is written as a path, in a whole-plane export as `0024` criteria 59 and 130 (e) write it. No export change beyond the membership read of criterion 1. Golden files in `tests/fixtures/` for the SVG of (a) to (d). Printing does not exist yet (`0059` out of scope) and is not part of this slice.

### Files, budgets, no I/O

19. Given a project with two drawing pages, a laser page, a sketch page with three constrained lines and a circle, a linked clone of that sketch on a drawing page and one on the laser page (rotated by 12 degrees), a drawing rectangle lying over the sketch page, and a second sketch page converted to a drawing page by criterion 15, when it is saved and reopened, then every page's kind, name, list order, size and place, the sketch, the clones with their placements, the rectangle and the converted paths are identical, and the file passes the golden-file test `sketch_pages_v37.curvyo` (`curvyo-document-core/tests/fixtures/`; no `format_version` bump, the format stays 37, see "Format").
20. Given the 100-entity sketch workload of `0066` criterion 6 with 20 linked clones on three other pages, then (a) each pointer move while dragging the sketch page's frame (criterion 5) and (b) each pointer move of the row-drag ghost (criterion 8) takes within 16 ms of thread CPU time in `curvyo-editor-wasm`, measuring the session's move call and the canvas view built for that frame (confirmed in `adrs.md` note 2; test under `curvyo-editor-wasm/tests/cpu_time/`). Neither drag re-solves the sketch.
21. Given the whole slice, then there is no network access, no device I/O, no G-code and no telemetry (`R-SYS-002`, `R-SYS-007`).
22. Given Place clone on, Select clones, Select original onto a sketch page and Convert to paths, when each is used with the mouse or the keyboard, then focus ends on the canvas, because the panel control belongs to a tab the new page does not have. This overrides the `0049` key rule (first control of the new body) for commands that change the page only; commands that stay on the page keep theirs (Unlink clone keeps its notice and focus on the first Style control).

## Changes to accepted specs

This slice changes accepted criteria; the old text is superseded by the criterion named here.

| Spec | Criterion | Was | Now |
|---|---|---|---|
| `0059` | 6 | Delete page: the objects that lay on it stay on the pasteboard | for a sketch page, its sketch is deleted with it and its clones become paths (16, 17; Frage 158); foreign objects over it still stay |
| `0059` | 9 | membership by position for every page; a frame move carries the objects wholly on it; a "frame alone" switch | a sketch page's content is its sketch by ownership, objects over it count as pasteboard (1); its frame move carries the sketch only, holds clones back, no "frame alone" switch (5) |
| `0066` | 8 | Convert to paths replaces the sketch with paths on its page | the page also becomes a drawing page (15; Frage 157) |
| `0066` | 9 | entering a clone's Sketch editing context opens the source page | double-click or Select original activates the sketch page (13) |
| `0098` | 5 | a page's kind is fixed; no command changes it | fixed, except that Convert to paths turns a sketch page into a drawing page (15; Frage 157 A). A general kind change is `0104` |
| `0098` | 8 | a press inside a page's frame activates it | except a press on a drawing object lying over a sketch page while a drawing page is active (2) |

## Out of scope

- A general change of a page's kind (Drawing, Sketch, Laser in any direction): `0104` `page-kind-change` (customer request of 2026-11-05).
- Everything of `0100` in the split table above: drawing page rows as placed pages, canvas drags with Alt and Ctrl, Paste as linked clone, group clones, the move-only selection of foreign objects while a laser or sketch page is active, double-click to switch context for any object, the laser page's own Layers tree.
- Scaling the original sketch (the pending question of ADR 0026): closed as "not offered", since criterion 4 leaves no command that transforms an original; the customer may reopen it. Clones scale freely as today.
- Printing (no print command exists, `0059`).
- Auto-pan during the row drag, selecting clones on later pages in one press, a notice after Place, Convert or Delete.
- Duplicate a sketch page (with an independent copy of the sketch, a part variant); a "duplicate page as linked clones" command (`0059` out of scope).
- Several sketches on one sketch page, a sketch picker, sketches of other sketches (a clone of a sketch on a sketch page).
- Importing paths or SVG into a sketch as sketch geometry; turning drawing objects over a sketch page into sketch entities.
- Fit page to content on a sketch page (works as built; no sketch-specific rule).
- Copy and paste between sketch pages and drawing pages beyond what is built; the Layers row of a sketch on drawing pages stays as built.
- Operations on sketch clones (`0101`), fixtures (`0102`), machine output (`0029`).

## Open questions for the customer

Frage 157 (SKETCH-CONVERT): What "Convert to paths" does on a sketch page
- A. The sketch becomes ordinary paths and the page becomes a drawing page in the same place (same name, size, position), with the drawing tools at once; clones follow the paths; one undo step gives the sketch back. Needs a one-way kind change Sketch to Drawing (ADR 0042 §3).
- B. The sketch becomes paths on a new drawing page at the Add page position (right of the last page), which becomes active; the sketch page stays, with an empty sketch.
- C. No Convert on the sketch page; a sketch becomes paths only per clone, by Unlink clone on the drawing or laser page; the sketch always stays editable.
- Recommendation: A. It is what the maker means ("this part is done, make it a drawing"), nothing jumps to another place, and paths never lie on a sketch page where no tool can edit them (the state the `0098` build leaves today).
- Default if unanswered: A.

Frage 158 (SKETCH-DELETE): Deleting a sketch page whose sketch has linked clones on other pages
- A. Page and sketch go; every clone becomes ordinary paths in its place, so the drawings look the same (the `0049` rule for a deleted original); the trash tooltip says how many clones become paths; one undo step restores all.
- B. Refused while clones exist; the trash button says "Used by 3 clones", and Select clones finds them.
- C. The page goes; the sketch becomes paths on the pasteboard where it lay, and the clones stay linked to those paths.
- Recommendation: A. It is the clone rule the maker already knows, it leaves nothing uneditable behind, and undo is one press away.
- Default if unanswered: A.

## UX notes

See `specs/0099-sketch-pages/ux-notes.md` (row drag, ghost and badges; Place clone on; the Use group; the state line; press table for objects over a sketch page; Convert; Delete; focus rule). Lead decisions of 2026-11-06 (Proposal, vetoable at the demo), applied in the criteria above: the visible state reads "3 DOF" (criterion 14, accessible name spelled out); a clone dropped on the pasteboard while the sketch page stays active shows a selection box without handles until `0100` (criterion 8); Select original and double-click clear the clone's selection (criterion 13); sketch rows get the delete-consequence tooltip, Convert to paths is full width and hidden for an empty sketch (criteria 15, 16); Select clones reaches the first page with clones only (criterion 12); "used by N clones" in the Sketches tab and the row tooltip (criterion 12); commands that change the page leave focus on the canvas (criterion 22).

## Links

Requirements: R-EDIT-042, R-EDIT-038, R-EDIT-030, R-SYS-002, R-SYS-007
Parent: `specs/0063-manufacturing-workbenches/` (slice 2; concept `concept-output-pages.md` §2.1, §2.7, §5 flow 9, §9; sketches f, i)
Depends on: `0098` (merged, #279), `0066`, `0059`, `0049`, `0057`, `0024` slice 4
Runs beside: `0100` `bring-content-in` (split table above; the page-row drag is built by whichever merges first, so the lead should not run both in one checkout at once)
Changes: `specs/0059-multiple-pages/`, `specs/0066-constraint-sketcher/`, `specs/0098-page-kinds/`
ADRs: `specs/0099-sketch-pages/adrs.md` and `docs/adr/0043-sketch-pages-content-by-ownership.md` (Proposed, `needs-customer` for the document model, accepted by the customer with the PR); it amends ADR 0042 §1, §3, §4, ADR 0024 §1, §5 and ADR 0026 (original transforms, page deletion). Frage 157 A and 158 A assumed; with B or C decisions 4 and 5 shrink as the ADR says. No new crate, no new dependency.
UX: `specs/0099-sketch-pages/ux-notes.md`.
Format: no `format_version` bump; it stays 37 (every write uses existing registers: `kind`, the sketch claim, `clone_link`, `own_placement`, `deleted`; membership is read, not stored). Golden: `curvyo-document-core/tests/fixtures/sketch_pages_v37.curvyo` and the SVG goldens of criterion 18.
PR: -
