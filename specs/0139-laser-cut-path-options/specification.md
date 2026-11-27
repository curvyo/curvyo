# Laser cut path options: tabs that hold a part in, start and end of the cut, lead-in, lead-out and overcut

Status: Slice 1 tabs Done (PR #519); slice 2 path options Ready. Fragen 414 and 415 stay open at their defaults and are applied, vetoable at the demo.
Priority: Should
Origin: Customer (2026-11-19, translated): (8) tabs or bridges that interrupt a cut so the part does not fall out, width adjustable; (10) advanced laser operation options (Z offset, start point, end point, lead-in, lead-out, overcut), assignable per operation or item. Everything marked **(Proposal)** is the product owner's and vetoable at the demo.
Revision 2026-11-26: read against the code and specs as built (`0029` Done PR #371, `0101`, `0111`, `0123`, `0103`, `0118`, `0156`); criteria renumbered, tolerances in mm added, golden G-code fixtures listed, one new question (Frage 414). The decisions of 2026-11-19 (Fragen 279 A, 280 A, 281 A) are unchanged.
Revision 2026-11-26 (b): reconciled with `adrs.md` (spec issues 1 to 8) and `ux-notes.md` (C1 to C14, Frage 415); see "Reconciliation". Format 66, two slices, damaged item records, Automatic marker rules, per-operation item records.
Depends on: `0101` (nearest rule, operations), `0111` (unfolded row, Operations view), `0029` (the job and its order, done). Related: `0057` (all-or-nothing override rule), `0118` (Frame reads the job's burn moves), `0156` (Bridge, see below), `0123` (raster kinds, untouched).

## User value

As a maker cutting a part out of a sheet I want to leave small tabs so the part stays in the sheet until I free it, and to say where each cut starts and in which direction, how the beam enters and leaves, and how far it runs on to close the cut, so that parts do not shift or fall onto the burning sheet and the start mark does not ruin a visible edge.

**What other tools do.** LightBurn: Tabs/Bridges per shape (count, size, skip inner shapes) and, in the Cut Settings editor, Lead-in/Lead-out, Overcut and a start point per shape (exact field names not re-checked). Fusion 360 CAM: tabs on 2D contours, lead-in and lead-out arcs. Inkscape has none (extensions only). We do better: tabs are plain handles on the path with a width in mm, set on the item or by one operation default, and the Operations view and the Toolpath view show exactly where the gaps and the leads are, from the same moves the file writes.

## Words

- **Tab:** a short stretch of a cut path that is **not** cut. The path stays whole and closed in the drawing and in the document; only the **output** leaves the gap.
- **Tabs versus Bridge (`0156`):** Bridge makes new **drawn geometry** (a second object, real connecting shapes, visible in every export, assignable to an operation of its own). Tabs are **output gaps** on an unchanged path: Object to path, boolean operations, SVG export and Layers never see them. Bridge joins separate pieces into one outline; a tab cannot. Neither replaces the other.
- **Piece:** the burnt run between two tabs (or the whole path when it has none).
- **Start point:** where the cut of a closed path begins, a position anywhere on the path (not only a node). **Not** the page's Start pick of `0029` criterion 38 / `0118` (machine origin, head position, user origin: where the *job* starts on the bed).
- **Direction:** which end an open path is cut from.
- **Automatic (start, direction):** what the output does today (`0029` criterion 22: the next path is the one whose nearest node or end is closest to the head; a closed path may start at any of its nodes, an open path at either end). A path stays Automatic until the maker sets a start or direction on it (Frage 414 A).
- **Waste side:** the side of a closed path away from the part. Decided by nesting among the closed paths of the same operation (the nesting of `0029` criterion 22, `outline_nesting`): an outline at even depth (0, 2, ...) is a part edge and its waste side is outside; at odd depth (a hole) it is inside the hole. A path with no parent is depth 0.
- **Lead-in, lead-out:** a short extra burnt stretch before the first burnt point and after the last that lies on the waste side, so the entry and exit marks do not land on the edge.
- **Overcut:** the beam keeps burning past the point where a closed path meets its start, along the path, by a typed distance, so the cut closes cleanly.
- **Z offset** is `0111` criterion 21. Not repeated here.

## What changes in the output, the preview and the views

- **Output (`0029`):** a tab is a laser-off rapid across the gap (a `G0`, as every travel move, no dwell); lead-in, lead-out and overcut are burn moves at the operation's numbers. One builder writes both the file and the Toolpath view (`0029` criterion 9), so they cannot differ.
- **Toolpath view:** shows exactly those moves. A gap is a thin dashed travel line between two pieces; lead-in, lead-out and overcut are burn lines in the operation's kind look (Cut red); every piece starts with the arrowhead of `0029` criterion 8 (a path with 4 tabs shows 4 arrowheads); the numbered ring stays at the operation's start. Nothing new is drawn and no handle or marker is shown, because the view is read-only (`0029` criterion 8). The time, the burn length and the travel length of the total and of each row follow the moves (`Timeline`).
- **Operations view (`0111` criterion 15, `0101` criterion 18):** a tab is a short gap in the red cut line with a small bar across it; the start point is a small marker with an arrowhead; lead-in and lead-out are thin dashed segments, overcut a dashed stretch along the path; two colours only. In the normal drawing view none of it is drawn, except that a selected item's tab handles and start marker are shown (handles, only while the item is selected and not in the Toolpath view).
- **Frame and work-area check (`0118` criteria 22 to 24, `0029` criterion 4 b):** both read the job's burn moves, so lead-ins, lead-outs and overcuts count and gaps do not. A lead that leaves the work area blocks like any burn move (no change to the block rule).
- **Not touched:** Score, Engrave, Engrave + Outline, Image (`0123`) and every raster move; fixture jobs (`0103`), test grids (`0112`) and Print and cut registration marks; no block of this spec appears on those rows.

## What changes accepted behaviour

- **Format:** `format_version` 67 (the next free number at merge, after 66 of `0146`; main wrote 65 on 2026-11-26). A build of 65 or lower would misread the new `op_options` keys and ignore item records, which would cut held parts loose, so older builds refuse the file with the newer-version message. A file of 65 or lower opens unchanged with the defaults; nothing is written on open (ADR L6).
- **Amendments made in the story PR (the implementer adds them, text from `adrs.md` L14):** `0029` criterion 22 ("a closed path with a set start point and an open path with a fixed direction have one entry; a closed path with tabs is entered at a burnt vertex or a piece start") and `0101` criterion 19 ("the direction of an open path may be fixed per item; tabs leave gaps in the output, the outline itself is unchanged"); Status notes on ADRs 0046, 0056 and 0064. Each amendment goes into the PR of the slice that builds it.
- **Nothing else changes for a maker who does not use it:** with the defaults the job is byte-identical (criterion 24).

## Slices

Each slice is one PR, testable end to end on the laser (draw a part, set the options, read the Toolpath view and the G-code).

- **Slice 1: tabs** (criteria 1 to 12, 24, 25 a, the tab goldens, the format bump). **Built first (Proposal):** it is the smaller, more common need, and criterion 15 (tabs win over lead and overcut) needs it.
- **Slice 2: path options** (criteria 13 to 23, 24, 25, the path goldens). Starts when slice 1 is merged. If slice 1 is already merged under format 66, slice 2 takes the next free number (Proposal; the architect assumed one bump for the whole story).

## Acceptance criteria

Slice 1: tabs (customer idea 8). Tolerances: gap length and tab position 0.005 mm and 0.01 mm as stated per criterion; the written path is the flattened path of `0029` criterion 17 (at most 0.01 mm from the exact curve), arc length is measured on it.

1. Given a Cut or Cut-out operation row (`0111`), then it shows a **Tabs** block: Count (whole number 0 to 20, default 0), Width (0.1 to 20 mm, two decimals, default 0.5 mm **(Proposal)**) and a switch Skip inner shapes (off). A value outside its range is refused with the range in the field message and nothing is written (`0111` criterion 24); a change is one undo step "Edit settings". Given a Score, Engrave, Engrave + Outline or Image row, then the block is absent (hidden, not disabled). Test: the block exists on exactly Cut and Cut-out; Count 21 and Width 0.09 and 20.01 are refused.
2. Given Count n above 0 and a path (closed or open) of length L, then k = min(n, floor(L / (4 x Width)), floor(L / (Width + g))) tabs are placed, with g = 1 mm on a closed path and g = 2 mm on an open path (the last limit keeps every burnt piece at least 1 mm, criterion 5; an open path has two end pieces next to its tabs, so both keep at least 1 mm), centred at the fractions (i + 0.5) / k of L, i = 0 to k-1, measured from the path's first stored point along its drawn direction (closed paths wrap; the placement does not depend on the start point or the order). With k below n for some path the block shows a muted line "Placed: 3 per path" ("2 to 4 per path" when paths differ, "none" when none; read-only); with k = 0 the path is cut whole. Test: a 40 x 40 mm square drawn from its bottom-left corner, Count 4, Width 2 mm: four gaps centred on the four side midpoints within 0.01 mm, each 2 mm +/- 0.005 mm along the path; a 100 mm open line, Count 1: one gap centred at 50 mm; a circle of 10 mm diameter (circumference 31.4 mm), Width 2.5 mm, Count 20 places 3 (floor(31.4 / 10)).
3. Given Skip inner shapes on, then a path that has a parent among the closed paths of the same operation (the nesting of `0029` criterion 22, any depth, open paths included by their nodes) gets no automatic tabs; its own tabs (criterion 5) are kept. Test: a plate with a round hole, Count 4: four gaps on the outline and none on the hole; switch off: four on each.
4. Given an item in the operation's output page, then it follows the operation's tabs, per leaf (the nearest rule of `0101` criterion 12): a group gives every leaf its own placement by criterion 2; a clone is an item of its own and may carry own tabs; an image in a Cut or Score operation is its frame as a closed line (`0101` criterion 19) and gets tabs like any closed path; a Cut-out has one contour per item (`0101` criterion 23) and the tabs sit on that contour.
5. Given a selected item that runs in the operation, then each of its tabs shows a handle on the path. Dragging slides it along the path (snapping to a node within 8 screen px, Alt disables); a click on the path with the Tabs tool of the Select bar's operation group adds one; Delete on a handle removes it. From the first change the item has **own tabs**: all or nothing (the rule of `0057`; the operation's tabs no longer reach it, and later changes of Count or Width on the operation do not touch it), shown in the item part as the pill Tabs "Operation's | Own" with "Own tabs (n)"; pressing Own copies what the item shows now (output unchanged, step "Own tabs"), pressing Operation's returns to the operation's (step "Back to the operation's"), and a handle edit makes the tabs own inside its own step. Own tabs are per item and operation (ADR L3). Two tabs never overlap and never leave a burnt piece shorter than 1 mm **(Proposal)**: a drag stops at that limit, and an add that finds no room changes nothing. A tab stores its position as a fraction of the path's length. Test: a drag to 0.4 mm from its neighbour stops at 1 mm; Back to the operation's restores criterion 2's gaps byte for byte in the output. Keyboard routes, the 48 px and 50-leaf handle limits and the readouts are in `ux-notes.md` sections 5 and 7; handles are editable on locked items and on the selected member of an entered laser group.
6. Given a tab, then its width is the operation's Width unless it has its own, typed in the entry chip of `0052` opened by a double-click on its handle (0.1 to 20 mm, mm only). Own widths only exist on own tabs. A typed width that would leave a piece under 1 mm is refused with "No room for this width.".
7. Given the Operations view, then a tab is drawn as in "What changes" above, 1 to 2 device px bar, at the exact gap; in the normal view only the selected item's handles. Tabs never change geometry: Object to path, booleans, SVG export, Layers and the document's paths ignore them. Test: an SVG export of a page with tabs equals the export without.
8. Given the output (`0029`), then every tab is a gap: the piece before it ends, a laser-off `G0` goes to the piece after it, no beam in the gap (GRBL M4: the gap is a `G0`, which never fires the laser under M4, so no `S0` word is required), `M5` before the travel and `M3 S<n>` after it with M3 (`0029` criterion 25), no dwell. The gap measured along the written path is the tab's width +/- 0.005 mm (on a curve by arc length) and its centre is at its stored fraction +/- 0.01 mm. Test: the tester's G-code parser sums the lengths of the burn pieces and the gap travel of a straight 100 mm line, Count 3, Width 2 mm: burn 94.000 +/- 0.015 mm, gaps 6.000 +/- 0.015 mm; `Timeline.burn_mm` and `travel_mm` change by the same amounts.
9. Given the order (`0029` criteria 21 and 22), then tabs change neither which path runs when nor the passes: the path's place in the order comes from its whole shape (inner before outer, nearest entry); a closed path with tabs is entered at the nearest uncut corner (burnt vertex) that is not a piece start, else mid-piece, never at the far end of a tab, and its pieces run one after the other along its direction, wrapping once; with Passes above 1 every pass writes the same gaps. Test: a plate with a hole and tabs on both cuts the hole's pieces before the plate's; two passes show 2 x the gaps at the same coordinates.
10. Given own tabs that cover a whole path (no burnt piece is left), then the path cuts nothing and the Checks of `0029` show one warn line "<n> paths are covered by tabs and cut nothing." ("1 path is covered by tabs and cuts nothing." for one; at most 70 characters; the count selects the items); the job still runs when anything else burns, and when nothing burns `0029` criterion 4 d (nothing runs) blocks as before.
11. Given save and reopen, then the operation's Count, Width and Skip, and each item's own tabs (fractions, own widths) survive; a path edit moves its tabs with it by fraction; own tabs are per item and operation; removing an item from the operation or deleting it deletes its own tabs of that operation in the same step (`0101` criterion 27), a copy (Copy, Duplicate, Paste) keeps no tab, `path_own` or `entry` record and comes out without options and without assignment, a new linked clone starts with none; a damaged own-tabs record (not a map, fraction outside 0 to 1, width out of range, overlapping tabs) makes that item run nothing in that operation, counts in the damaged-settings warning and is saved back unchanged; a file without tabs opens unchanged and a build that does not know the new format refuses the file through the newer-version message. `format_version` 66, or the next free number at merge (main wrote 65 on 2026-11-26).
12. Given any tab change, then it is one undo step named "Add tab", "Move tab", "Delete tab", "Tab width" or "Edit settings".

Slice 2: path options (customer idea 10).

13. Given a Cut or Cut-out row, then it also shows **Lead-in** and **Lead-out** (each None, Line or Arc; Length 0.1 to 20 mm, two decimals, default None and 1 mm **(Proposal)**) and **Overcut** (0 to 10 mm, two decimals, default 0). They apply to closed paths only and are absent on the other kinds (as criterion 1). Out-of-range typing is refused as in criterion 1.
14. Given a closed path with Lead-in Line, then the lead is a straight segment of Length that ends at the first burnt point, perpendicular to the path direction there, on the waste side (as in Words). Given Arc, then it is a quarter circle of radius Length that ends at that point tangent to the path direction, bending to the waste side (written as straight segments within 0.01 mm of the circle). Lead-out is the same at the last burnt point, leaving in the path direction and bending away on the waste side. The direction at a corner is the direction of the segment that leaves (lead-in) or arrives (lead-out). Test: a 40 x 40 mm square (depth 0) with Line 2 mm starting at a bottom-side midpoint: the lead starts 2.000 +/- 0.005 mm below the side; a 10 mm round hole (depth 1) with Line 1 mm starts inside the hole 1.000 +/- 0.005 mm from the edge.
15. Given Overcut d above 0, then after the closed path reaches its start the beam keeps burning along the path by d (+/- 0.005 mm), never beyond one full path length, and stops at the first tab it meets; lead-out starts where the overcut ends. Given a tab that covers the closing point, then there is no overcut and the lead-out sits at the last burnt point (the end of the final piece). Tabs win: no lead, overcut or piece burns inside a tab. Test: a circle of 60 mm circumference, Overcut 5 mm: the burn length is 65 +/- 0.01 mm; with one 2 mm tab starting 1 mm after the start point, the overcut stops at the tab (1 mm), so the burn length is 60 - 2 + 1 = 59 +/- 0.01 mm.
16. Given Passes above 1, then every pass writes its own lead-in, overcut and lead-out. Lead and overcut run at the operation's numbers (power, speed) like the path.
17. Given the numbers of lead and overcut, then their burn moves count in the Timeline, the work-area check (block rule 4 b, tolerance 0.01 mm) and the Frame (`0118`); a lead that leaves the work area blocks like any burn move. Test: a lead-in Line 2 mm adds 2.0 +/- 0.01 mm to `Timeline.burn_mm`; a part at the bed edge with a lead outside the area blocks Export.
18. Given a lead-in, lead-out or overcut segment that comes within 0.01 mm of any other burn path of the same operation (its own path excepted at the junction point), then the Checks show one warn line "<n> leads cross or touch a cut." ("1 lead crosses or touches a cut." for one; at most 70 characters; n counts such segments; the count selects the items) and the job still runs (permit over refuse); no other mark.
19. Given a closed path, then its **start point** is a marker the maker drags along the path (any position, snapping to nodes within 8 px, Alt disables), per closed outline of the selected item (one marker each for a compound path or a group's leaves). Until the maker sets it the path is Automatic (output as today, Frage 414 A); its marker rests where the job really enters the outline and is shown only while the Tabs tool is on (Frage 415 C); a set marker is shown whenever the item is selected. A set start is written exactly: the burn begins at that position +/- 0.005 mm, goes around in the drawn direction and returns to it; the order picks the path as before but with that one entry. An **Automatic** button on the item returns it, one undo step. Start point is per item and operation (stored with the operation's records, ADR L3; the item part shows the values of the operation of its row); an operation cannot place one point for many shapes. The marker cannot be dropped inside a tab (it stops at the tab's edge); if a later tab covers it, the burn begins at that tab's end. Delete on a current set marker returns the outline to Automatic.
20. Given an open path, then its **Direction** is Automatic (today: either end, nearest first), As drawn or Reversed (a three-value pick on the item, which sets the end the cut starts at; the direction arrowhead of the Operations view shows it; when `R-MFG-EMB-005`'s direction display exists this reuses it). A fixed direction writes that single entry. **End point (Frage 281 A, revisitable at the demo):** for an open path the end is the other end; for a closed path the end is where the overcut ends (start point plus Overcut along the path); there is no separate end marker and no early-stop gap (that is a tab).
21. Given a selected item that runs in the operation, then it may carry **own path options**: Lead-in, Lead-out and Overcut as one block (all or nothing as criterion 5, Frage 280 A), shown in the item part as the pill "Path options" "Operation's | Own" (Own copies the operation's values, step "Own path options"; Operation's is "Back to the operation's"); start point and direction are always per item and are not part of this block. The key of an own record is the assigned item (the nearest rule, `0101` criterion 12) and, for start point, its closed outline.
22. Given save and reopen, then the operation's lead and overcut values, the items' own blocks, start points and directions survive; a path edit keeps a start point by its fraction of the path length; a file without them opens unchanged; deleting an item or removing it from the operation deletes its records of that operation in the same step, copies (Copy, Duplicate, Paste) keep none of them (they come out without options and without assignment), damaged item records behave as in criterion 11. These values belong to the operation only: none of them is a material number or a machine profile field (`0111` criterion 25 rule). Same `format_version` as criterion 11 when both slices ship together; otherwise the next free number (see Slices).
23. Given any change of this slice, then it is one undo step: "Move start point", "Automatic start", "Direction", "Own path options", "Back to the operation's", "Edit settings".
24. Given the defaults (Count 0, Lead None, Overcut 0, every start and direction Automatic), then the job is byte-identical to today: every golden file in `curvyo-laser-core/tests/fixtures/output/grbl/` still passes unchanged, and the Toolpath view and the timeline of an existing project are equal. This is the guard that the story changes nothing for a maker who does not use it.
25. Budgets (thread CPU time; tests in `curvyo-editor-wasm/tests/job_builder_budget.rs` and `operation_tools_budget.rs`; limits confirmed by the architect, ADR L13): the job of 1,000 Cut items of 20 nodes each with Count 4, Lead-in Arc and Overcut builds within 150 ms (`0029` criterion 36 a allows 100 ms without them); the item handles and markers of a selected item redraw within 16 ms per pointer move.

## Golden fixtures

G-code, `curvyo-laser-core/tests/fixtures/output/grbl/`, each read back by the tester's own GRBL parser (extent, sum of burn and travel lengths, order, the gaps and the lead lengths) independently of the generator (`0029` criterion 31):

- Tabs: `tabs_square_even.gcode`, `tabs_open_line.gcode`, `tabs_circle_arc_length.gcode`, `tabs_skip_inner.gcode` (plate with hole, Skip on), `tabs_own_item.gcode` (own tabs and an own width), `tabs_passes.gcode` (two passes), `tabs_cutout.gcode`, `tabs_m3_constant_power.gcode` (`M5`/`M3` around gaps), `tabs_cover_path.gcode` (own tabs cover a path, nothing burnt).
- Path options: `lead_in_line_outer.gcode`, `lead_in_arc_outer.gcode`, `lead_in_hole.gcode` (hole inside a plate, order inner first), `lead_out_arc.gcode`, `overcut_closed.gcode`, `overcut_stops_at_tab.gcode`, `path_options_passes.gcode`, `path_options_own_item.gcode`, `start_point_set.gcode`, `open_direction_reversed.gcode`, `tabs_leads_overcut.gcode` (all together on a Cut-out).
- Document: `curvyo-document-core/tests/fixtures/laser_path_options_v<N>.curvyo` (operation values, own tabs, own blocks, start points, directions, a clone and a placed-page row), `laser_path_options_damaged_v<N>.curvyo` (values out of range, a tab fraction outside 0 to 1, not a map: each reads as damaged settings and is saved back unchanged), N the format of criterion 11.
- Unchanged and re-run: all existing `output/grbl/*.gcode` (criterion 24), including `inner_first`, `passes`, `cutout_margin` and `print_and_cut_cut_line`.

## Out of scope

- Z offset (`0111`), CNC lead-in with ramps (`R-MFG-CNC-*`), tabs on engraved or filled areas or raster, automatic "tabs only where the part is heavy", corner power and kerf compensation, lead-in on open paths.
- Drawn bridges (`0156` Bridge, `0068`): geometry in the design, not output gaps.
- Reversing the direction of a closed path (stays as drawn); a free end marker; early-stop gaps other than tabs.
- Tabs and lead on fixture jobs, test grids and registration marks.
- Scripted or imported tab positions from SVG; other output formats than GRBL (the model is format-neutral, `0029` criterion 39).
- Machine control (`0118`) changes: the Frame already follows the job.
- Follow-up, not scheduled, needs the customer's decision: copies keep an item's operation membership and options (today a copy has no membership, `0101` gap).

## Decisions

Customer, 2026-11-19: **Frage 279 (TABS-WHERE) A**: an operation default (count and width) plus handles per item. **Frage 280 (PATHOPT-LEVEL) A**: lead-in, lead-out and overcut on the operation, overridable per item. **Frage 281 (ENDPOINT-MEANING) A**: customer unsure, **revisitable at the demo**; open path: choice of which end the cut starts at; closed path: the end is where the overcut ends.

Decided by the product owner **(Proposal)**: tabs win over lead, overcut and pieces; lead only on closed paths; waste side by nesting within the operation (a hole that is cut alone, with its plate not in the operation, is depth 0 and gets its lead outside the hole); minimum burnt piece 1 mm; default tab width 0.5 mm and lead length 1 mm.

Open questions, applied at their defaults, vetoable at the demo:

**Frage 414 (START-DEFAULT): What happens to a path whose start the maker never sets?**
Today every closed path starts at the node nearest the head and every open path at its nearer end (`0029` criterion 22).
- A: **Automatic until the maker sets it** (today's behaviour; setting a start point or direction fixes that one path). No existing job changes.
- B: Always as drawn (first node, drawn direction) unless set. Predictable, but every existing job changes its order and travel.
- C: A switch per operation, Automatic or As drawn, default Automatic.
Recommendation: A. Default if unanswered: A (applied).

**Frage 415 (START-MARKER-SHOW): When does the canvas show the start marker of a path the maker never set?**
- A: Always, for every selected closed path.
- B: Never; only a marker the maker has set is shown.
- C: Only while the Tabs tool is on; a set marker is always shown while its item is selected.
Recommendation: C (fewer marks in the way when the maker is only selecting). Default if unanswered: C (applied).

## Reconciliation (2026-11-26)

`adrs.md` and `ux-notes.md` exist; no customer answer is contradicted (Fragen 279 A, 280 A, 281 A hold). Decided by the lead (Proposal), vetoable at the demo:

- ADR issue 1: an Automatic start marker shows where the job really enters, not at the first stored point (criterion 19).
- ADR issue 2: a closed path with tabs is entered at a burnt vertex or a piece start (criterion 9).
- ADR issue 3: a start marker cannot be dropped inside a tab; a later tab over it moves the burn to that tab's end (criterion 19). This overrides `ux-notes.md` 5.2 ("a start may sit inside a tab"); the UX notes follow.
- ADR issue 4: a damaged item record makes that item run nothing in that operation, counted in the damaged-settings warning, saved back unchanged (criteria 11, 22).
- ADR issue 5: k also limited by floor(L / (Width + 1 mm)); the test values do not change (criterion 2).
- ADR issue 6: removing or deleting an item drops its records of that operation; copies keep none (criteria 11, 22).
- ADR issue 7: budget tests live in `curvyo-editor-wasm/tests/` (criterion 25).
- Copies (Copy, Duplicate, Paste), after slices 1 and 2 were built, Decided by the lead (Proposal): a copy keeps no tab, `path_own` or `entry` record, because copies keep no operation membership in the code today (`0101` gap); it comes out without options and without assignment (criteria 11, 22).
- Tester round 1 (0139 slice 1), Decided by the lead (Proposal): (F2) criterion 8: "no S above 0 in the gap" means no beam; the gap is a `G0`, which never fires the laser under M4, so no `S0` word is required. (F3) criterion 2: on open paths the automatic count is capped at floor(L / (Width + 2 mm)) so both end pieces keep at least 1 mm; closed paths keep floor(L / (Width + 1 mm)). (F1, built) criterion 9: a closed path with tabs is not entered at the far end of a tab; it enters at an uncut corner that is not a piece start, else mid-piece.
- ADR issue 8: criterion 4 d stays; the job runs when anything else burns (criterion 10).
- UX C1: placed line as a range, only when some path gets fewer (criterion 2). C2: item records are per item and operation, also start and direction; this overrides `ux-notes.md` 3.1 ("the same values in every operation's body"), the item part shows the values of its operation. C3: pill "Operation's | Own" with step "Own tabs" (criteria 5, 21). C4: Frage 415. C5: handle limits (criterion 5). C6: width refused without room (criterion 6). C7: as ADR issue 1. C8: Delete on a set marker is Automatic (criterion 19). C9, C10: keys on handles, handles on locked items and entered groups (criterion 5). C11 to C14: look and tool placement as in `ux-notes.md` (no criterion).
- `ux-notes.md` has no numbered issue list; C1 to C14 were treated as the issues.

## UX notes

See `ux-notes.md` in this folder (ux-engineer, 2026-11-26).

## Links

ADRs: `adrs.md`  UX: `ux-notes.md`
Requirements: R-MFG-LASER-004 (tabs), R-MFG-LASER-005 (path options)
Depends on: `0101`, `0111`, `0029`
Related: `0112`, `0057` (override rule), `0068`, `0103`, `0118`, `0123`, `0156`
PR: none yet
