# Laser fixtures: the bed's grid, holders, clamps and stops as selectable objects, switched on per fixture, with parts snapping into slots

Status: Ready (2026-11-10, revised 2026-11-10 after the review of PR #313 (handles and chips, screw added) and aligned to `ux-notes.md` (D22 to D27: the thread never above the head, no size fields in the block); adrs.md, ADR 0049, ADR 0050 and ux-notes.md exist; Fragen 171 to 175 answered by the customer 2026-11-10: A, with 173 at a 0.1 mm tolerance and 174 at B and C)
Priority: Should
Origin: Customer (request of 2026-10-17, translated: "In the work area there can be fixtures to lay out or clamp the parts, plus maybe a measuring tape [...] they snap in by fitting size"; concept round 2 of 2026-11-05, feedback 4 and 11: "the engraved grid is good but needs measuring tapes and thicker major lines; fixtures are selectable; several active at once"). Slice 5a of `specs/0063-manufacturing-workbenches/`. Details marked **(Proposal)** are the product owner's and vetoable at the demo.

**Answered by the customer 2026-11-10 (review of PR #313, translated): "The UI is not finished yet. Use the rectangle as the model: take its handles including all its double-click entry boxes. For holder and clamp the input fields in the property bar go away. For the stop, do the analogous thing that fits. The grid can stay as it is. There is also a fixture for a screw/bolt with the dimensions of the thread and of the screw head. Assume they are round."** Applied: holder, clamp and stop are edited on the canvas by the rectangle's handles and their double-click entry boxes (criteria 14, 18, 22, 23); the grid keeps its typed fields (15); a fifth kind, Screw, with a thread and a head diameter, round (criteria 2, 8, 37 to 40). The stop's handle set is a **(Proposal)**, to be confirmed at the demo.

## User value

As a maker with a laser I want the bed to show a grid with measuring tapes, holders with slots, clamps and stops that I can select, switch on and off, and parts that snap into a slot by fitting their size, so that I lay out my parts on the real bed without measuring each one.

**What other tools do.** LightBurn has a bed grid as a display setting and no fixtures; its camera and Print-and-Cut are the workaround for placing parts (`specs/0063-manufacturing-workbenches/concept-output-pages.md` §1). Fusion 360 keeps fixtures as real bodies of a setup, but every change goes through a modal dialog. Inkscape has no bed at all. **We do better:** the fixtures are ordinary objects of the page, selected by their outline, switched on and off by one check box each, and they snap: a placed page or a clone turns and drops into a free slot of its size with no dialog, and the fixture set travels with the machine profile so a new project starts with the bed you own (Frage 175).

## Words (shown in the UI)

- **Fixture:** a part of the bed that is not drawn from the drawing and is never output: a grid, a holder, a clamp or a stop. Part of the machine setup; installed on a laser page.
- **Grid (engraved):** the bed grid of `0093`, now with major lines and measuring tapes. Not the view grid of `0090`, which is the Grid snap target and is shown on every page.
- **Tape:** a measuring scale along an edge of the grid, with ticks and numbers in mm.
- **Major line:** every n-th grid line (default every 5th), drawn thicker.
- **Holder:** a board with slots. The board is the object; each slot is a place a part is put.
- **Slot:** a named rectangle or circle inside a holder.
- **Clamp:** a keep-out area on the bed, drawn hatched.
- **Stop:** an L-shaped fence a board or part is pushed against.
- **Screw:** a round fixture of two concentric circles, the thread and the head, each with its diameter (criteria 37 to 40). Round only: no hex or countersunk shape (customer, 2026-11-10).
- **Active:** the fixture is on the bed now. An inactive fixture is not drawn, not hit and not a snap target; it stays in the page and in the profile.
- **Fit:** a part whose size matches a slot's size (within 0.1 mm, centre to centre, turned by 90 degrees allowed) snaps into that slot and is turned to fit.
- **Fixture set:** the fixtures a machine profile holds (only with Frage 175 A).

## Acceptance criteria

"Laser page" means a page of kind Laser whose extension is on (`0098` criterion 18). Fixtures exist only on laser pages. "Active page" means the active laser page. Every fixture of this slice is a node of the laser manufacturing (ADR 0032 §8, ADR 0033 §2), so every criterion refers to a fixture of the page it lies on.

### The Fixtures block and its rows

1. Given a laser page, when its Document tab is shown, then the Fixtures block (after the Machine block and the Place page button of `0100`) lists every fixture of the page in creation order, one row each: an **Active** check box, the kind's glyph and word, the name, the eye of `0093` (criterion 6) and the armed trash of `0081`. A new laser page with no fixture shows the block with its five add buttons only.
2. Given the Fixtures block, when the maker presses one of five kind buttons, Grid, Holder, Clamp, Stop or Screw (symbol and word each, one press each), then one fixture of that kind is added with the name "<Kind> <N>" (N the lowest number not used by a fixture of that kind on this page), **Active on**, selected, and one undo step "Add <kind>". Proposal sizes, centred on the work area: Holder 100 x 60 mm with no slot; Clamp 40 x 20 mm; Stop two legs of 60 mm and a thickness of 5 mm; Screw a thread of 6 mm and a head of 10 mm diameter (Proposal, an M6 screw). While the page has a grid, the Grid button is hidden (not disabled), so a second grid is never offered and no refusal is shown (Proposal: one grid per page; the grid's row in the block is the reason).
3. Given a fixture row, when the maker double-clicks its name (F2 on the selected row is the same route), then the name is edited in place as in `0101` criterion 3: Enter or leaving commits, Escape cancels, names are up to 40 characters and need not be unique (Proposal); one undo step "Rename fixture".
4. Given a fixture, when the maker presses its armed trash (first press arms, a second press after 400 ms removes, as `0081`), then the fixture and its slots are removed and nothing else changes; one undo step "Remove fixture" restores it exactly, with its slots, Active state and position.
5. Given several fixtures, then any number of them may be Active at once; there is no single-choice behaviour. Given the Active check box is switched off, then the fixture is not drawn, not hit and not a snap target, and its row stays; switching it on restores all of it. Each switch is one undo step ("Set fixture active" or "Set fixture inactive"). A new fixture starts Active (Proposal). A fixture with no stored Active value reads as Active, so files saved by `0093` look as they did.
6. Given a fixture whose eye is off, then it is not drawn and not hit, but it stays Active and remains a snap target (Proposal: the eye hides, Active decides what lies on the bed). The eye is stored on the fixture as `0093` shipped it, and each switch is one undo step ("Show or hide fixture"); it changes no output.
7. Given a fixture on the page, then it lies in the bottom layer: below every placed page and every own item of the page, above the page background. The origin marker and the placement boxes of `0100` draw above it (`0098` adrs, drawing order). A fixture is never output: it is in no SVG export, no print and no export of the drawing, and no operation can be assigned to it (`0101` criterion 10).

### Selecting fixtures

8. Given the laser page is active, when the maker presses on the outline of an Active fixture (hit within 4 device pixels of the outline with a mouse, 12 with a coarse pointer, Proposal), then the fixture is selected alone: it shows its box and its handles (criteria 13 to 15), with the tag "Grid", "Holder", "Clamp", "Stop" or "Screw". A fixture is never part of a selection with other objects: a press on another object replaces it, and a Shift+press on another object adds nothing from the fixture (Proposal).
9. Given a holder or a clamp, when the maker presses inside its area, then that press does not select it, and the press acts on what lies under it (a part on a board stays reachable). A press on a slot's outline selects the holder that holds the slot.
10. Given a marquee or Ctrl+A on the active laser page, then no fixture is selected (Proposal: a select-all must never move the bed). A fixture is selected by a press (criterion 8), by its row in the Fixtures block, or by its row in the Layers tree (criterion 12).
11. Given a fixture of a laser page that is not the active page, then it is not hit: a press on its outline acts as a press on that laser page's empty area, which activates the page as `0098` criterion 8 says, and selects nothing (Proposal; this keeps Frage 164 of `0106` in one rule).
12. Given the laser page's Layers tree (`0106` criterion 17), then the Fixtures row at the bottom lists each fixture as a row with its glyph, its name and its eye (criterion 6); pressing the row selects the fixture on the canvas, and a fixture selected on the canvas marks its row. No lock is shown on fixture rows (Proposal).

### Grid

13. Given a selected grid, then it shows its box without move, rotate or scale handles: its position is its origin offset (criterion 15). It is removed only by its trash in the Fixtures block; the Delete key does nothing to a grid and shows the notice "Remove the grid with its trash in the Fixtures block." (Proposal).
14. Given a holder, a clamp, a stop or a screw, when the maker moves it (drag from its outline, or from inside its box where no handle and no other object is hit; arrow keys of `0044`, 1 mm and 10 mm with Shift; the centre move handle or M, which open the Move chip with "X" and "Y", Relative or Absolute and the nine-point Reference, as the rectangle's Move chip) or turns it (one of the four corner rotate glyphs, 22.5 degree snap as `0008`; a double-click on a glyph or R opens the Angle chip), then it moves or turns as one object, one undo step per move or turn ("Move fixture", "Rotate fixture"). It is never skewed. Its size changes only by its handles and their chips (criteria 18, 22, 23, 38) or by the S chip, never by a field of the Select bar, which is not shown for these four kinds (`0008`, `0043`). A holder moves its slots with it; the slots keep their place on the board. A screw does not turn: it has no rotate glyph and no R (Proposal, D25).
15. Given the grid row, when the maker sets its parameters in place, then these are the rows, each with its range and default: **Spacing** 1 to 100 mm (default 10); **Major every** 2 to 20 lines (default 5); **Tapes on** one switch per edge (default: all four edges on, Frage 174 C); **Origin** X and Y as `0093` (default 0, 0). The origin corner is the configurable origin corner of the 0090 grid (its `grid_origin` register, Frage 174 B), not the profile's corner and not fixed to bottom-left; Origin X and Y are the grid's offset from that corner, and the tapes' numbers count from that origin. A value outside its range is set to the nearest limit with a notice of at most 70 characters (the `0026` pattern; nothing is refused). Each change is one undo step and the canvas shows it in the next frame.
16. Given a grid at spacing 10 mm with major every 5, origin at the bottom-left corner of a 600 x 400 mm work area, then minor lines run every 10 mm from the origin across the whole work area, the lines on the work-area edges included: 61 vertical and 41 horizontal lines, of which 13 vertical and 9 horizontal are major (every 5th line from the origin). Minor lines are 1 device pixel wide, major lines 2 device pixels, in the stronger fixture ink (tokens in `ux-notes.md`). The stored line width of `0093` (0.1 mm) is kept unchanged and is not shown in this slice.
17. Given tapes on an edge, then a band 5 mm wide runs along that edge inside the work area, with a tick every 5 mm (short), a longer tick every 10 mm and a number every 50 mm. The numbers are millimetres counted from the grid origin along the edge. Test: a 600 mm bottom tape with the origin at the corner has 121 ticks at 5 mm, 61 of them long, and 13 numbers (0, 50, ..., 600).

### Holders and slots

18. Given a holder, then it is drawn as a board: solid outline in the fixture ink with a light tint (tokens in `ux-notes.md`). Its Width and Height are 1 to 1,000 mm (default 100 and 60, criterion 2). They are set only by its eight resize squircles (corners and edge midpoints), by their chip (a corner squircle opens Width and Height, a north or south squircle only Height and an east or west squircle only Width, as the rectangle does; the opposite side or corner stays fixed), or by the S chip. A typed value outside the range is set to the limit with the Action notice of `ux-notes.md` section 4.4. Each resize or chip commit is one undo step ("Resize fixture"). The Fixtures block shows no Width or Height field for a holder. A resize that leaves a slot outside its board is allowed and noticed as in criterion 20; a slot keeps its size and its centre offset from the board's corner (Proposal). Its position and angle are those of criterion 14. A holder has no corner knobs and no skew (D22).
19. Given a holder (Frage 172 A: slots are rectangles or circles), when the maker presses **Add slot**, then a slot "Slot <N>" is added: a rectangle of 20 x 20 mm centred on the board (Proposal). Each slot has a Name (up to 40 characters), a Shape (Rectangle or Circle), its centre X and Y in mm from the board's corner, and its Width and Height (rectangle) or Diameter (circle). Each field edit and each add or remove is one undo step. A holder holds up to 50 slots; at 50 the Add slot button is hidden, so a 51st slot is never offered and no notice is shown. Each slot's trash is armed as in criterion 4.
20. Given a slot that lies outside its board or past its edge, then it is allowed and drawn, with the notice "Slot <name> lies outside <holder>." (permit over refuse). A size below 0.5 mm or above 1,000 mm is set to the nearest limit with a notice of at most 70 characters.
21. Given a holder with slots, then each slot is drawn as its outline in the fixture ink with its name as a label. Each slot is a snap point and a fit target (criteria 24 and 25) while the holder is Active.

### Clamps and stops

22. Given a clamp, then it is a rectangle drawn with a diagonal hatch in the fixture ink and a solid outline. Its Width and Height are 1 to 1,000 mm (default 40 and 20, criterion 2) and are set only by its eight resize squircles, their "W" and "H" chip, or the S chip, as in criterion 18; the Fixtures block shows no size field for a clamp. It moves and turns as criterion 14, and it has no corner knobs and no skew. It is a keep-out area only: an operation that crosses it is neither refused nor counted in this slice (Out of scope).
23. Given a stop, then it is an L: leg 1 runs along the stop's width and leg 2 along its height, from the corner. Each leg is 1 to 1,000 mm long; the thickness is 0.5 to 50 mm; drawn with a solid outline. Proposal (D23), the analogous set that fits a rectangle: a stop has no corner or edge resize handles. Its handles, each with its chip: a **Leg 1** squircle at the free end of leg 1, on its centreline (a drag changes the length of leg 1; the "Leg 1" chip has one field); a **Leg 2** squircle at the free end of leg 2, the same for leg 2; a **Thickness** knob on the top edge of leg 1, at its midpoint (a drag moves it perpendicular to leg 1; the thickness is the distance from the bottom edge of leg 1; the "Thickness" chip has one field). The stop also has the four corner rotate glyphs, the centre Move chip, and the S chip with "Leg 1", "Leg 2" and "Thickness" in one chip. Limits (D24): a leg is never shorter than the thickness, and the thickness is never above the shorter leg or 50 mm; a typed value is set to its limit with the Action notice, the other value does not move, and a drag stops at the limit without a notice. The Fixtures block shows no leg or thickness field for a stop. Its outer edges and its corner are snap points (criterion 27).

### Snapping to fixtures

24. Given the **Fixtures** snap target (a switch in the Snapping Properties panel of `0060`, on by default; a document setting that travels with the file as the register `snapping_fixtures`; a file without that register reads as on, and no key is written on open), when a placed page, a clone, a group or an own item of the active laser page is moved, then its box points (centre, corners, edge midpoints) snap to the snap points of the Active fixtures of criterion 27, with the capture, release, hysteresis and Alt rules of `0060`. Nothing else of the snapping rules changes.
25. Given the fit rule (Frage 173 A), when a moved item's box size matches a slot's size within 0.1 mm in its own orientation or turned by 90 degrees, and its centre comes within the capture distance of the slot's centre, then the item's centre snaps to the slot's centre and, when the match needs the turn, the drag ghost shows the item turned by 90 degrees. The slot lights in the accent colour (`ux-notes.md`). Release commits the move and the turn as one undo step "Move to slot". Fit never changes the size of an item. A square item in a square slot is never turned: a turn happens only when the other orientation is the fit (Proposal). A size that differs by more than 0.1 mm in both orientations does not fit, and no fit happens.
26. Given a placed page dragged from the page list by `0100` criterion 13, when an Active holder has a slot that it fits, then criterion 25 applies. When no slot fits or no fixture is Active, it lands where released as built, and no slot cue is shown.
27. Snap points of the Active fixtures of the page, with the fixture switch on: (a) each grid intersection of an Active grid, within the capture distance; (b) for each slot of an Active holder: its centre, its corners and its edge midpoints (the holder's board corners and edge midpoints too); (c) for each Active stop: its outer edges and its corner; (d) for each Active clamp: none (Proposal: a keep-out is avoided, not snapped to). (e) for each Active screw: none (criterion 40). A fixture that is not Active contributes nothing. A fixture with its eye off still contributes (criterion 6).

### Profile and fixture set (only with Frage 175 A)

28. Given Frage 175 A, then a machine profile holds a fixture set: the page-independent list of its fixtures (kind, name, Active, position and angle, and parameters; no selection and no eye). The Machine block of the profile shows "Fixtures in profile: N". **Save as profile** stores the page's fixtures in the profile; **Update profile** writes them to the library (armed as `0026` criterion 7); **Reset from profile** replaces the page's fixtures with the set, in one undo step.
29. Given Frage 175 A, then picking a profile on a laser page (`0026` criterion 5) replaces the page's fixtures with the profile's set in the same undo step as the pick. A new laser page made from a profile starts with the set. The built-in Generic laser holds an empty set, so a new laser page with Generic laser starts with no fixture, as `0093` criterion 9 says. Picking Generic laser on a page that has fixtures removes them, in the same undo step as the pick. "Differs from profile" (`0026` criterion 7) compares the fixture set too, derived on every read.
30. Given Frage 175 A and a profile record of format 1 of `0038` (no set), then it reads as an empty set and is not rewritten on read. Any write of a profile record, including a rename, writes format 2 (ADR 0038 record bump, ADR 0050 §3); reads never write. This is a profile record change, not a project format change. A damaged fixture set makes the whole profile record damaged: the record is skipped with its notice. The per-field defaults of criterion 33 apply only to a page's copy (ADR 0050 §2). A record of a newer record format is skipped with its notice, as `0026` criterion 13 says.

### Files, failures and round trip

31. Given a laser page with one fixture of each of the five kinds (the screw included), two holders with two slots each (one circle), an inactive grid, a hidden clamp, and a holder moved and turned by 30 degrees, when the project is saved and reopened, then every value, Active state, eye state, slot and position is identical, and the golden test `fixtures_v40.curvyo` (`curvyo-document-core/tests/fixtures/`) passes (format 40, see "Format").
32. Given a fixture of a kind this build does not know, then it is kept with all its values, listed in the Fixtures block as "Unknown fixture" with a neutral glyph, draws nothing, is not a snap target, is not output, and can only be renamed or removed (R-MFG-010). Saved back unchanged. Golden `fixtures_unknown_kind_v40.curvyo`.
33. Given a damaged value (text where a number belongs, a non-finite number, a slot without a size or shape), then the field reads as its default (a damaged slot is skipped: not drawn, not a snap target) and one notice of at most 70 characters counts the damaged values of the fixture ("1 value damaged on Holder 1."). The file is never rewritten on read. Golden `fixtures_damaged_v40.curvyo`. A slot whose holder is missing (an orphan child node) is skipped the same way; golden `fixtures_slot_orphan_v40.curvyo`.
34. Given a file saved by `0093` (a grid with spacing and origin, no tapes, no Active value), when it is opened, then the grid is Active, its lines are unchanged, its tapes show on all four edges of criterion 15 (their default), and nothing is written on open. Golden `fixtures_grid_v39.curvyo` (ADR 0049 §2).

### Budgets and no I/O

35. Budgets, thread CPU time in `curvyo-editor-wasm` (tests under `curvyo-editor-wasm/tests/cpu_time/`, never wall time; the figures are confirmed in `adrs.md` §8): (a) on a laser page with 4 holders of 15 slots each and 1,000 own items and placed leaves, each pointer move of a drag takes within 16 ms, including the fit and snap query; (b) a press on a page with 50 fixtures is resolved (hit test) within 2 ms; (c) the Fixtures block with 50 fixtures shows its first rows within 100 ms; (d) building the draw list of a grid at 1 mm spacing over 1,000 x 1,000 mm (2,002 lines and their ticks) takes within 16 ms.
36. Given the whole slice, then there is no network access, no device I/O, no G-code, no output file and no telemetry (`R-SYS-002`, `R-SYS-007`). Fixtures add nothing to an output: `0101` and `0029` see no fixture as a leaf.

### Screw

37. Given a screw (customer, 2026-11-10, translated: "a fixture for a screw/bolt with the dimensions of the thread and of the screw head. Assume they are round."), then it is two concentric circles on one centre: the head and the thread. Each has a diameter of 0.5 to 1,000 mm; defaults 10 mm for the head and 6 mm for the thread (Proposal, an M6 screw). The thread is never larger than the head (lead decision D26): a typed thread above the head is set to the head, and a typed head below the thread is set to the thread, each with the Action notice of `ux-notes.md` section 4.4; a drag stops at the limit without a notice. Both circles have a solid outline in the fixture ink. It is added by the Screw button of criterion 2.
38. Given a selected screw, then it shows its box (the square around the head circle) and two diameter knobs (the Parameter handle look, D25): the **Head** knob on the head circle at 0 degrees (east) and the **Thread** knob on the thread circle at 180 degrees (west), so that they never meet. A drag of a knob changes the diameter of its circle by the pointer's distance from the centre along the knob's axis, and stops at the other circle's diameter. A double-click on a knob opens its one-field chip, "Head diameter" or "Thread diameter", in mm; the S chip holds "Head" and "Thread" in one chip (Tab between them). Each commit is one undo step ("Resize fixture"). The centre move handle and the Move chip work as in criterion 14. A screw has no rotate glyph, no R, no resize squircle and no skew, and the Fixtures block shows no diameter field for it.
39. Given a screw, then its thread diameter and head diameter are stored as the parameters `thread_diameter` and `head_diameter` (mm, see "Format"). Its name, Active state, eye, trash and round trip are those of criteria 3 to 7 and 31, and it is never output (criterion 7).
40. Given a screw, then it is not a snap target and not a fit target in this slice: it has no snap point, not even its centre, and a placed page is not fitted to a screw (Out of scope). Its Active switch still decides whether it lies on the bed (criterion 5).

## Changes to accepted specs

This slice changes accepted text; the old text is superseded by the criterion named here.

| Spec | Criterion | Was | Now |
|---|---|---|---|
| `0093` | 9 | one fixture kind, Engraved grid; a new manufacturing starts with none; spacing, line width, origin | five kinds (1, 2; the screw of 37 to 40); the grid gets major lines and tapes (15 to 17); a new laser page starts with the profile's set, empty for Generic laser (29) |
| `0093` | 10 | a fixture is not selectable or hit, cannot be selected by marquee or Select All | selectable by its outline on the active page only (8 to 12); not part of marquee or Select All (10); still never output (7) |
| `0093` | Out of scope ("does not snap") | the grid is not a snap target | a snap target through the Fixtures switch (27) |
| `0098` | 13 (Fixtures block) | one row per grid, add, remove, eye | Active check box, five kinds, the grid's parameters and slot rows in place (1 to 5, 15, 19); holder, clamp and stop sizes on the canvas, not in the block (18, 22, 23) |
| `0100` | 13 (without fixtures) | a placed page lands where dropped; no slot, no fit snap | with an Active holder, criteria 25 and 26 apply; without one, unchanged |
| `0100` | 13 (ux notes: "no slot cue, no fit-by-size snap") | no slot cue | the slot cue is shown (25) |
| `0106` | 1 (laser context) | its own items and clones are editable | plus its fixtures: edited in the active laser page's context, moved and turned by criterion 14 (8, 13, 14) |
| `0106` | 17 (Fixtures row) | the row is a row; no selection or editing of fixtures until `0102` | the rows select fixtures (12); fixture editing as in this spec |
| `0060` | Snap targets | Objects, Guides, Nodes, Page | plus **Fixtures** (24, 27); stored as the register `snapping_fixtures`, absent reads as on (24) |
| `0026` | Out of scope ("fixtures moving into the profile") and criteria 5, 7 | fixtures stay in the manufacturing; a profile has no fixture set | with Frage 175 A the profile holds a set; picks, Update, Reset and "Differs" include it (28 to 30) |
| `0026` | record of `0038` | record format 1 | record format 2 with the fixture set, only with Frage 175 A (30) |
| ADR 0032 | §8 | fixtures are never hit or selected | hit by outline while their laser page is active (8 to 11); amended on acceptance |
| ADR 0042 | §4 | fixtures are not hit | same; amended on acceptance |
| ADR 0033 | §2 | `fixture_kind` and `fixture_params`; `visible` is the eye | new kind values and parameter keys, an `active` register (absent reads as on) and, if the architect keeps slots in the parameters, a list inside them; additive, see "Format" |
| `0093` | Fixtures block, line width | the grid's line width (0.1 mm) is a visible field | the field is hidden; the stored width is kept, not shown (16; `ux-notes.md` D8) |
| `0093` | Fixtures block, delete | one press removes a fixture or a slot row | armed trash as `0081`: the first press arms, the second removes (4, 19; `ux-notes.md` D9) |
| `0098` | 13 (Fixtures block), row height | 36 px rows | 40 px rows in the block, 28 px in Layers (`ux-notes.md` D5) |
| `0106` | 17 (Fixtures row) | the row has no children and no chevron | the row has its group eye and always lists one row per fixture, with no chevron (12; `ux-notes.md` D14) |
| ADR 0038 | record format | record format 1, no set | record format 2 with the fixture set; any write of the record writes format 2; only with Frage 175 A (30; ADR 0050) |
| ADR 0039 | §4, §5 | the profile copy on a page holds no fixtures | only with Frage 175 A: the page's copy holds the set (28, 29; ADR 0050); the grid's origin corner is the 0090 `grid_origin` register, not the copy (Frage 174 B) |
| `0101` | 10, 17 | fixtures are not targets; the summary counts leaves only | unchanged; the clamp crossing count of concept §6 is not added (Out of scope) |
| `0098` | 13 (Fixtures block), holder, clamp and stop sizes (review of PR #313, 2026-11-10) | Width and Height of a holder and a clamp, and the legs and thickness of a stop, typed in the block | removed from the block; set on the canvas by the handles and their entry boxes (18, 22, 23) |
| `0008`, `0043` (Select bar, review of PR #313, 2026-11-10) | the bar's typed fields for a selected object | X, Y, Width, Height and Angle fields for a holder, a clamp or a stop | no fields for a holder, a clamp, a stop or a screw (14) |

## Out of scope

- Fixture jobs: a fixture carrying operations, the Fixture jobs section, Output switches on fixtures (`0103`). Fixtures take no operations here.
- The engraved grid as output (its tapes and lines engraved onto the bed board is `0103`'s job).
- The "operation crosses Clamp" count in the summary (concept §6). It changes the accepted summary of `0101` criterion 17 and is proposed for `0103` or later; the customer decides there.
- Slots as any closed path drawn with the tools, or node-editing a holder's outline (Frage 172 B; later).
- Skewing fixtures, scaling a stop or a screw by its box (only their parameter handles change them), rotating the grid; any fixture kind beyond the five (pen jig, material holders as outlines, pockets, sheet guides, hex or countersunk screws).
- A screw's centre as a snap point or a fit target for holes, thread pitch, screw length and a drawn thread (criterion 40; the customer decides later).
- Automatic arrangement of parts into slots (nesting, R-MFG-004); the fit only snaps, it never places a part by itself.
- Fixtures on drawing, sketch or machine pages other than laser pages; a fixture moved to another laser page (only through the profile, Frage 175 A).
- Collision and crossing checks, 3D view, physical machine models.
- Fixtures of other machine families (CNC table, plotter, embroidery).
- The stored grid line width of `0093` (0.1 mm) as a visible setting.
- Undo of profile library changes (`0026` out of scope stands).

## Open customer questions

Answered by the customer on 2026-11-10 (the answers are applied in this text, in `adrs.md` and in `ux-notes.md`); the answers are recorded under each Frage. Numbers 171 to 175 are the next free ones (`grep "Frage 17"` in `specs/` finds none, `0101` ends at 170).

**Frage 171 (FIX-KINDS): Which fixture kinds come in this slice?**
**Answered by the customer 2026-11-10: A (for four kinds; the customer added the screw on 2026-11-10, see the note under Origin).**
- A: all four: Grid (with major lines and tapes), Holder with slots, Clamp and Stop. The concept's full set in one slice (criteria 1 to 23). **Revised 2026-11-10 (review of PR #313): the customer added a fifth kind, Screw (criteria 37 to 40), so A now covers five kinds in this slice; the answer itself is unchanged.**
- B: Grid and Holder now; Clamp and Stop in a later slice.
- C: Grid, Holder and Clamp now; Stop later.
- Recommendation: A. The customer named all four in round 2; the parts are small and each is one kind with one shape.
- Default if unanswered: A.

**Frage 172 (SLOT-SHAPE): What is a slot?**
**Answered by the customer 2026-11-10: A.**
- A: a rectangle or a circle, typed in the Fixtures block (name, centre, size). Nothing is drawn; a coaster board is a holder with circle slots.
- B: any closed path, drawn on the laser page with the normal tools and then assigned to the holder as a slot (the engraved coaster outline of the concept). Needs an assignment flow of its own.
- C: rectangles only.
- Recommendation: A now; B later, once the maker has used typed slots and wants freehand outlines.
- Default if unanswered: A.

**Frage 173 (FIT-RULE): How does fitting by size work?**
**Answered by the customer 2026-11-10: A, with a tolerance of 0.1 mm instead of 0.5 mm: a page placed into a slot must fit exactly, within 0.1 mm, centre to centre, turned by 90 degrees as before.**
- A: a part whose size matches a slot within 0.1 mm snaps centre to centre and is turned by 90 degrees when that is the fit; the drag ghost shows the turn before release (criterion 25).
- B: only the same orientation fits; the maker turns the part by hand.
- C: a fit only lights the slot; the maker presses to place the part, nothing is turned for them.
- Recommendation: A (the customer's words are "snap in by fitting size"; the turn is visible before release, so it is not a surprise).
- Default if unanswered: A. The tolerance is 0.1 mm as answered (the customer's figure, not a Proposal).

**Frage 174 (GRID-ORIGIN): Where do the grid's origin and its tapes sit?**
**Answered by the customer 2026-11-10: B and C.** The origin is the configurable origin corner of the 0090 grid (its `grid_origin` register) plus the grid's offset, not the profile's corner and not fixed bottom-left. Tapes sit on all four edges by default, each switched on or off, with numbers counted from that origin.
- A: the origin is the origin corner of the machine profile (plus the grid's origin offset); the tapes run along the two edges that meet at that corner, numbers counted from that corner in mm (criterion 15, 17).
- B: the origin is always the bottom-left corner, and the tapes are always the bottom and left edges, whatever the profile says.
- C: tapes on all four edges, numbers counted from the origin corner; the maker switches each edge on.
- Recommendation (as put to the customer): A. Answered: B and C, see above.

**Frage 175 (FIX-SET-HOME): Does the machine profile hold the fixtures?** (WB-FIXTURE-HOME A is decided in the concept: fixtures live in the machine setup. This asks only how the copy behaves.)
**Answered by the customer 2026-11-10: A.**
- A: yes. The profile holds a fixture set; each laser page holds a copy, as it does for the limits. Picking, Update profile and Reset from profile move the set (criteria 28 to 30).
- B: no. Fixtures stay on each laser page as `0093` built them; the profile has no set.
- C: yes, live. A laser page follows its profile's set without a copy; a change to the profile reaches every page at once.
- Recommendation: A (install a bed once and use it on every project, and a file still shows its bed without the library, `0039`).
- Default if unanswered: A. B would remove criteria 28 to 30 and the record format 2 bump.

## UX notes

Written: `specs/0102-laser-fixtures/ux-notes.md` (ux-engineer, 2026-11-10, with Fragen 171 to 175 at their defaults). It covers the Fixtures block and its add group, the rows and the armed trash, the typed fields of each kind and of the slots, the canvas look of grid, tapes, holder, clamp and stop, the selection box and tag, the slot cue and ghost turn of criterion 25, the snap switch, the Layers row, the profile rows, keyboard routes, accessible names, undo steps and budgets. It uses ink and the existing tokens only. Its section 14 lists the Proposals and the changes it asked of this spec; this revision applies them (criteria 1, 2, 8, 15, 19, 25 and the changes table). Each Proposal is vetoable at the demo. Revision of 2026-11-10 after the review of PR #313: criteria 14, 18, 22, 23 and 37 to 40 replace the typed holder, clamp and stop fields and the four kinds of the notes; `ux-notes.md` (D22 to D27) is the detail behind these criteria and is aligned to them.

## Format

`format_version` **40** (decided in ADR 0049, proposed, accepted with the PR; design in `adrs.md` §2).

- Slots are child nodes of the holder: a new node shape (`fixture_slot`) under the manufacturing node, as ADR 0046 §6 does for operations. An older build would keep or drop these nodes by accident, so the bump is needed. The position is the register `fixture_matrix`; Active is the register `active` (absent reads as on); the snap switch is the root register `snapping_fixtures` (absent reads as on).
- No migration: a format 39 file opens unchanged. Main is at 39 (`0101`).
- Kinds: five values of `fixture_kind`. The screw adds the parameters `thread_diameter` and `head_diameter` (mm, additive). The architect updates `adrs.md` and ADR 0049 §2 for the screw and for the handle-set sizes; the other parameter keys stay as they are there.
- The fixture set of Frage 175 A is a profile record change, record format 2 (ADR 0050, ADR 0038 record bump), not a project format change.
- Goldens in `curvyo-document-core/tests/fixtures/`: `fixtures_v40.curvyo`, `fixtures_unknown_kind_v40.curvyo`, `fixtures_damaged_v40.curvyo`, `fixtures_slot_orphan_v40.curvyo`, `fixtures_grid_v39.curvyo`. The five v40 goldens are regenerated for this revision (`fixtures_v40.curvyo` holds one screw); the existing goldens stay and must open unchanged.

## Links

Requirements: R-MFG-007 (fixtures and the work area; Should, as this spec ships it; `docs/requirements.md` updated with this revision, customer reference slice 5a of `0063`), R-MFG-008 (placement and snapping by fitting size), R-MFG-010 (unknown kinds round-trip), R-EDIT-038 (pages), R-SYS-002, R-SYS-007
Parent: `specs/0063-manufacturing-workbenches/` (slice 5a; concept `specs/0063-manufacturing-workbenches/concept-output-pages.md` §6 "Fixtures as objects", sketch g in `concept-sketches.html`, feedback 4 and 11 of §0)
Depends on: `0093` (the engraved grid, the fixture node, the Fixtures eye), `0098` (the Fixtures block, page kinds), `0100` (placed pages and clones dropped by drag; placed where dropped until this slice), `0101` (fixtures take no operations), `0106` (the laser page's Layers tree and the context rule), `0026` and ADR 0038/0039 (the profile and its copy), `0060` (snapping and its target switches), `0081` (the armed trash), `0044` (arrow nudges), `0008` (22.5 degree rotation snap)
Next: `0103` `fixture-jobs`
Changes: `specs/0093-laser-workbench-shell/`, `specs/0098-page-kinds/`, `specs/0100-bring-content-in/`, `specs/0106-move-versus-edit/`, `specs/0026-machine-profile/`, `specs/0060-snapping/`; ADR 0032 §8, ADR 0042 §4, ADR 0033 §2; ADR 0038 and ADR 0039 §4, §5 (only with Frage 175 A); `adrs.md`, ADR 0049 and ADR 0050 of this slice (proposed, accepted with the PR)
PR: -
