# Laser operations: an ordered operation list per laser page, assigned to any item, with Show, Output and Cuts last

Status: Done (PR #305; ADR 0046 accepted with the PR; Fragen 167 B, 168 A, 169 A, 170 A)
Priority: Must
Origin: Customer (concept A, `specs/0063-manufacturing-workbenches/concept-output-pages.md`, accepted 2026-11-05: Frage 140 A, 141 A, 142 B, 147 A, 148 A, 149 A, 153 A, 154 B, 155 A; feedback 2 to 4 and 9 of the same day). Slice 4 of `0063`, replaces the reserved `0027`. Details marked **(Proposal)** are the product owner's and vetoable at the demo.

**Answered by the customer 2026-11-10** (review of PR #305, translated): (1) "I can't find where you got the colours from, so away with the colours." No colour per operation, no colour picker, no operation colour tokens. Each object with an operation gets a symbol, one per kind; Cut is drawn red; where a kind fills or engraves an area, the area is hatched in pale orange. The look has two colours only (criterion 18). (2) No name field: a double-click on the name label renames it in place (criterion 3). (3) Delete is a trash icon next to the Output icon, an armed button as the fonts' trash of `0081` (criterion 4). (4) Sorting has up and down arrows and drag and drop (criterion 5). (5) No cut or engrave settings in this step (no power, speed or passes). The Cut-out margin of criterion 23 is a geometry setting of Cut-out, not a machine setting, and stays.

## User value

As a maker preparing a laser job I want to say, per object, group or placed page, what is cut, scored, engraved or cut out, and to see one ordered list of what the machine will do and in what order, so that I know what runs before anything is sent, nothing runs by accident, and my drawing is never changed by the preparation.

**What other tools do.** LightBurn makes an object's colour its machine instruction and copies everything onto the bed; its ordered layer list with Show and Output is the overview we want. xTool sets the process per object with no list, so nothing shows the whole job or its order. Glowforge has an ordered list of Cut, Score, Engrave and Ignore steps, but groups by colour. Fusion 360 selects geometry explicitly per operation and keeps the order per setup, but every operation is a modal dialog. **We do better:** the explicit selection of Fusion and the overview of LightBurn without colour as instruction, no dialog, and one visible switch **Cuts last** that keeps the cutting steps at the end (the rule the customer knows from PewPew) instead of an optimiser the maker has to fight.

## Words (shown in the UI)

- **Operation:** one step of the laser job on a laser page: a kind, a name, Show and Output. The list order is the run order.
- **Kind:** Cut, Score, Engrave, Engrave + Outline, Cut-out.
- **Symbol:** the mark drawn on an object that has an operation, one per kind the object is in (criterion 18). A symbol belongs to the kind, not to an operation; there is no colour per operation.
- **Look:** the two colours of the output page: red for lines to be cut (Cut, Cut-out), pale orange for hatched areas (Engrave, Engrave + Outline). Score and the outline of Engrave + Outline take their line style from `ux-notes.md`.
- **Assignment:** the membership of an item in one or more operations. An item may be in any number of them (153 A).
- **Item:** a node that can hold an assignment: an own item, a clone (of any kind), a placed page, or a member row inside a placed page or a clone (criterion 11). The tick, the drag and the selection act on items. The counts of criterion 17 and the budgets of criterion 29 count **output-page leaves** instead: each path, text, primitive or image of the output page, own, inside a placed page or inside a clone, drawn or hidden by its own eye (a hidden leaf still runs, 142 B), counted once. Guides, dimensions and fixtures are not counted (criterion 10).
- **Not assigned:** an item with no assignment at any level above or inside it. It is drawn dimmed and never runs (141 A).
- **Show:** view only. It hides the item on the output page's canvas. It never changes what runs.
- **Output:** what runs. An operation with Output off runs nothing.
- **Cuts last:** the switch on the Operations tab that keeps Cutting after Marking (154 B).
- **Marking section:** Engrave, Engrave + Outline, Score. **Cutting section:** Cut, Cut-out.

## Acceptance criteria

"Laser page" means a page of kind Laser whose extension is on (`0098` criterion 18). A "Laser (off)" page is covered by criterion 28.

### The operation list

1. Given a laser page, then its Operations tab exists on that page only, placed after Document (`0063` concept §7). It shows the toolbar with the five kind buttons (Cut, Score, Engrave, Engrave + Outline, Cut-out), the Cuts last switch, the list and the summary line of criterion 17. A new laser page has an empty list. Drawing and sketch pages have no Operations tab, and assignments are made only on a laser page.
2. Given the Operations tab and no item selected, when the maker presses a kind button, then one operation of that kind is added with the name "<Kind> <N>", N the lowest number not yet used by a name of that kind on this page (the first Cut is "Cut 1"), Show on, Output on, placed in its section (criterion 5). An operation has no colour. One undo step "Add operation".
3. Given an operation, when the maker double-clicks its name label, then the label becomes an editable text in place; Enter or leaving it commits, Escape cancels, and the label returns. There is no name field in the row. F2 on the selected row opens the same edit (extra keyboard route). The list, the Layers rows and the summary show a new name at once; an empty name reverts; names need not be unique (Proposal). One undo step "Rename operation".
4. Given an operation, when the maker deletes it, then it is removed with every membership it holds; items that no other operation holds become Not assigned; no drawing object is deleted. One undo step "Delete operation" restores the operation and its memberships exactly. The delete is the trash icon on the row, next to the Output icon. It is an armed button as the fonts' trash of `0081`: the first press arms it (the row shows the armed look, its accessible name says "Press again to delete <name>"); a second press at least 400 ms after arming deletes; a press within 400 ms is ignored; it disarms by itself after a few seconds, as `0081` says; only one armed trash exists in the app, and arming another disarms the first (`0081` criteria 4 and 6). No dialog, no confirm text.
5. Given Cuts last on (default for a new laser page; stored per laser page, Proposal), then the list has two sections, Marking (criterion 1 order) above Cutting, and Cutting always runs after Marking. A new operation lands at the end of its section. The maker reorders an operation with the up and down arrow buttons on its row, or by drag and drop; Alt+Up and Alt+Down are extra keyboard routes to the same arrows. Reorder only inside a section. The up arrow is disabled at the top of its section and the down arrow at its bottom, so a move never crosses the section line; Alt+Up stops at the line too (Proposal). A Cut or Cut-out dragged above the section line is refused: no insertion line is shown and the row returns to where it was.
6. Given Cuts last off, then the section line is gone, any operation can be dragged to any place, and a new operation lands at the end of the list. Switching it off moves nothing (one undo step "Cuts last off", Proposal). Switching it on again moves the cutting operations below the marking ones, each group keeping its relative order, in one undo step "Cuts last on".
7. Given the list, then the list order is the run order, read top to bottom. An item in several operations runs in each of them in list order (153 A). No other order is written in this slice; the order inside one cutting operation (inner shapes before outlines, short travel) belongs to the output of `0029`.

### Assignment

8. Given a selection of items on the output page, when the maker presses a kind button in the toolbar, then one operation of that kind is created as in criterion 2 and the selection is assigned to it, in one undo step "Assign to <name>". When the maker drags the selection onto an operation row, the selection is added to that operation in one step. When the maker ticks an operation in its row of the Operations tab (ticks show in the rows while items are selected; no separate block), all selected items are added to it; unticking removes them; the tick shows mixed when only some are in it. Each is one undo step "Assign to <name>" or "Remove from <name>" (Proposal: the number keys 1 to 9 toggle membership in operation n of the list, as in concept §5 flow 5).
9. Given an item in one or more operations, when the maker removes it from one of them, then only that membership goes; its other memberships stay. An item is never in an operation of another laser page.
10. Given a selection that holds a guide, a dimension of the drawing or the engraved grid (a fixture), then these are not targets: they get no tick and no operation, and the rest of the selection is assigned. A notice names the guides, at most 70 characters: "N guides not assigned. Guides never run." (40 characters for two guides; Proposal, 2026-11-09: fixtures cannot be selected on the output page, so the text names guides only; dimensions are skipped without a notice; the text of `0100` criterion 6 is the precedent). A selection of only such objects adds nothing and writes no history step; with guides in it, the refusal shows "Guides never run.".
11. Given a placed page on the output page, when the maker expands it in the Layers tree (`0106` criterion 18), then its source members appear as rows in the source's own layer and group order (Proposal: the source's layers are rows as well, so one layer can be assigned, as concept §2.5 names layers), and every row can take an assignment as an own item does. The assignment of an inner row is keyed by the source object's id and applies to that object in every placed page of the same source on this output page (Proposal: two placements of one board run the same inner operations). The placed page's own assignment is a separate record. A clone's members are expandable as read-only rows in the clone's own layer and group order, as a placed page's are (`0106` criterion 18, changed), and each member row takes an assignment (Frage 167 B). The assignment of a clone member is keyed by the path from the clone to the member (the clone, then the member), so it applies to that member inside that clone only, never to the original's member or to another clone of the same source. A clone itself also takes an assignment as a whole.
12. Nearest rule. Given a leaf (a drawn path, text, primitive or image) of the output page, its own, inside a placed page or inside a clone, then it runs with the operations of the nearest node above it (itself included) that has any assignment, and with none of the operations of the nodes further up. Example: group G on the output page is in a Cut-out operation and a text T inside G is in an Engrave operation: T runs Engrave only; the rest of G runs Cut-out. A leaf with no assignment at any level is Not assigned. (Concept §2.5, read as the accepted rule of 147 A.) For a clone member the chain is its path: the member, the groups between it and the clone, then the clone (criterion 11).
13. Given a placed page assigned to an operation with Output on, when a member of its source is hidden on its drawing page, then that member is not drawn on the output page and does not run through the placed page's assignment; it runs only with an assignment of its own, inner row (criterion 11) (Frage 149 A, hidden drawing content). With that assignment it runs and is counted as "runs while hidden" in the summary. **The rule, stated for both cases:** a hidden leaf under an assigned placed page does not run through that page; a hidden leaf under an assigned group on the output page still runs (142 B). The difference is intended: the placed page shows what is visible on its drawing page, and the group on the output page is the maker's own choice.
14. Given an own item, a clone or a placed page with no assignment at any level, then it is drawn dimmed with a dashed outline and no symbol (criterion 18), it never runs, and the summary counts it as not assigned. A new object (made with a tool, or dropped on the page by `0100`) starts Not assigned. A copy (Ctrl copy, Duplicate, Paste) keeps the memberships of its original (Proposal). An item moved off the laser page (`0106`: it becomes a drawing object again) loses its memberships in the same step, and one undo restores them.

### Show, Output and the summary

15. Given an operation with Show off, then its items are not drawn on the output page's canvas. An item is drawn when its own Layers eye is on and either it has no assignment or at least one of its operations has Show on. Show is view only: it never changes what runs (142 B).
16. Given an operation with Output off, then its items do not run, whatever their Show state; Output on with Show off runs (142 B). Each operation has its Show and Output switches on its row; both are stored with the page (criterion 25). Output is the only switch that changes what runs.
17. Given the Operations tab, then below the list one summary line reads "Runs <N> operations, <M> items.", where M counts the output-page leaves (Words: item) that hold an assignment and run, drawn or hidden by their own eye (142 B), each counted once. Only when above zero, further lines read "<K> items not assigned.", "<J> items run while hidden." and "<H> items sit in operations with Output off." Each count is a button that selects those items on the canvas and in the Layers tree. With no operation the line reads "Nothing runs. Assign items to an operation." The summary is updated in the same frame as any change.
18. Given the output page, then the view switch "View: Operations" (default) draws every leaf by its operation's look. The look has two colours only: **red** for lines to be cut and **pale orange** for hatched areas. Cut: a red line one device pixel wide on each outline. Cut-out: a dashed red 2 px line on the outer contour of each item only. Engrave: the closed areas of the leaf hatched in pale orange. Engrave + Outline: the same hatch, and the outline as a line in the style `ux-notes.md` sets. Score: a dotted line in the ink colour (red is for lines to be cut only; the style is in `ux-notes.md`). Each object with an operation carries a symbol on it, one per kind it is in (Proposal: two operations of one kind give one symbol); the symbol shapes are in `ux-notes.md`. Kinds differ by symbol and line shape, never by colour alone. Several operations on one leaf are drawn in list order. Not assigned: dimmed with a dashed outline, no symbol and no hatch. "View: Drawn" shows the drawing's own style of every item and no symbols. The switch is session state, not saved. Red and pale orange are each at least 3:1 against the page ground (values in `ux-notes.md`, checked by the ux-engineer).

### What each kind runs

19. Cut: every outline of every assigned leaf runs: a closed outline as a closed line, an open path as an open line, in the leaf's own path order. Fill and stroke never count. An image in Cut runs its frame as a line; its pixels do not run (`0063` concept §3, "frame only, not output").
20. Score: every outline, closed or open, runs as a line, in the Marking section. An image runs nothing. Score runs with the surface power of `0028`; in this slice it differs from Cut only by its kind and place in the list.
21. Engrave: a leaf with at least one closed outline runs as one filled area, with the leaf's own fill rule (the fill rule of the object, 140 A), whatever its fill switch; its open outlines are closed by a straight line (`0054`). A leaf whose outlines are all open runs as a line (140 A). So a compound path with one closed and one open outline runs as one filled area, and one with only open outlines as lines. An image runs as a raster engrave; its pixels are output by `0029`, and the summary counts it.
22. Engrave + Outline: as Engrave, plus every closed outline traced as a line, in the same operation and once per leaf. An image is raster engraved and its frame traced.
23. Cut-out: for each item of the operation (one contour per item, Frage 168 default A), the outer contour of the union of all its output-page leaves runs as one closed line; holes and inner detail do not run. An open leaf is closed by a straight line (`0054`). Margin (Proposal): a typed margin in mm, default 0, grows the contour by the outset rule of `0038` (Round corners); zero or a positive value only. Test: a group of two clones and one connecting bar in Cut-out gives one closed line and no line inside it.
24. Style never changes what runs: colour, stroke width, dash and the drawing's fill switch are ignored by every kind (the rule of the withdrawn `0027` criterion 7, carried over). Guides, dimensions and fixtures never run (criterion 10).

### Files, failures, budgets

25. Given a laser page with operations of all five kinds, names, Show and Output states, Cuts last on or off, memberships on own items, clones, clone members, placed pages and inner rows, when it is saved and reopened, then the list order, names, switches, memberships and Cuts last are identical, and the golden test `operations_v39.curvyo` (`curvyo-document-core/tests/fixtures/`) passes. A build of format 38 opens such a file only through the newer-version message. A format 38 file opens unchanged and is not rewritten on open (`format_version` 39 expected, see the Links).
26. Given an operation of a kind this build does not know, or an operation record that cannot be read, then it is kept with its memberships and saved back unchanged (R-MFG-010); it is listed as "Unknown operation" with a neutral glyph, runs nothing, is counted in no summary line and can only be renamed or deleted. Golden `operations_unknown_kind_v39.curvyo`.
27. Given an object that holds assignments (an own object, a group, a clone or a clone member, a placed page or a member row of a placed page) is deleted, then its assignments, and the assignments of every path through it, are removed in the same step, and no operation shows them (Frage 169 A: dropped silently; no row, no count, no notice). Undo of the deletion brings the object and its assignments back exactly. Deleting a clone's original, or a node on the path of a clone member, removes those assignments the same way; a clone that stops being a clone (repaired as a copy, ADR 0045) loses its clone member assignments in the same step. An assignment left by a merge whose object is gone is ignored: it runs nothing, is shown nowhere and is counted nowhere.
28. Given a laser page whose extension is off (`0098` criterion 18 b), then it has no Operations tab, its operations and memberships are kept and saved back unchanged, and no summary is shown. Deleting such a page or a laser page deletes its operations with it (`0098` criterion 16); one undo restores them.
29. Budgets, thread CPU time in `curvyo-editor-wasm` (tests under `curvyo-editor-wasm/tests/cpu_time/`; confirmed in `adrs.md`): (a) one press assigning a selection of 1,000 items takes within 16 ms; (b) the summary recomputed after any change, on a page with 1,000 items and 50 operations, takes within 16 ms; (c) the Operations tab with 50 operations and 1,000 items shows its first rows within 100 ms; (d) each pointer move of a drag that reorders an operation among 50 operations takes within 16 ms.
30. Given the whole slice, then there is no network access, no device I/O, no G-code, no output file and no telemetry (`R-SYS-002`, `R-SYS-007`). The demo shows what would run; `0029` makes the output.

## Changes to accepted specs

This slice changes accepted text; the old text is superseded by the criterion named here.

| Spec | Criterion | Was | Now |
|---|---|---|---|
| `0106` | 18 (placed page rows) | inner rows show the members of the source; no operation chips | placed page rows show the operation chips and the Show eye (view only); the inner rows also show the source's layers (11) |
| `0106` | 19 | the placed page's eye and lock are not shown in this slice; "the placed page's Show comes with `0101`" | the placed page's Show eye is shown (criterion 15); the lock stays out |
| `0106` | 18 (clone rows) | a clone row is not expandable | a clone row expands like a placed page row into read-only member rows; each member row takes assignments by path, and the clone row takes assignments as a whole (11, Frage 167 B) |
| `0106` | 5 (press on an inner row, lead decision 2026-11-09) | a press on an inner row selects the placed page | a press on an inner row selects the inner object for assignment; it stays read-only for edits, and only assignment is allowed (11) |
| `0098` | 11 | "Nothing on a laser page runs on a machine in this slice (no operations exist yet)" | own items are Not assigned by default and run only once assigned (14); the output still waits for `0029` (30) |
| `0050` | hidden objects in a machine output | "not decided" (R-MFG-005, 2026-10-11) | decided for laser output: hidden drawing content runs only with an assignment of its own (13); Show and Output are separate (15, 16) |
| `0026` | ac5 test (content keeps its place on a laser page, read in its own colour) | content on a laser page is drawn in its own colour | under "View: Operations" an unassigned item on a laser page is in the Not assigned look (14, 18); the test reads the drawing under "View: Drawn", which keeps its own look (18) |
| `0098` | 12 (ac12 tests that read the placed drawing in its own red: 6, 8, 9, 13) | a placed drawing on a laser page is drawn in its own colour | under "View: Operations" an unassigned placed member is in the Not assigned look (14, 18); these tests read "View: Drawn" |
| `0027` | whole spec | the reserved roles story: roles by drawing layer, "hidden is not output" | superseded (PR #269 closed unmerged; 152 A). Its geometry rules (17 to 24), look tokens, notice counts and budgets move here; "hidden is not output" becomes 142 B (criterion 16) |

## Out of scope

- Fixtures as objects: the grid with tapes and major lines, holders and slots, clamps, stops, Active per fixture, fixture snapping (`0102`). Fixtures take no operations here (criterion 10).
- Fixture jobs and the Fixture jobs section (`0103`).
- Power, speed, passes and material per operation, the material library, the Score surface power (`0028`). No cut or engrave setting field is in this slice (customer 2026-11-10). The Cut-out margin (criterion 23) is a geometry setting and stays.
- Preview, time estimate, the gate, G-code, and the order inside one cutting operation (holes before outlines, short travel) (`0029`).
- Image optimisation (dithering, resolution, contrast) and Offset fill: noted for later (148 C); they join the Marking section when their story comes.
- The kinds Dot mode, Image as a kind of its own, and Ignore (148 A: Output off does that job).
- Operations on other machine kinds (CNC, plotter, embroidery); a new kind is one step of the same list later.
- Editing inside a placed page or a clone, beyond the assignment of criterion 11 (`0106` out of scope; double-click question, Frage 170).
- Paste as linked clone and any object clipboard.
- Changing a page's kind (`0104`), nesting (R-MFG-004), saved operation sets or templates.
- Output switches on drawing layers: the drawing's own layers keep their view eye and lock only (`0050`).
- Per-object process settings without the list (refused in concept §1).

## Open customer questions

Defaults apply until the customer answers; none blocks the build.

**Frage 167 (CLONE-DEPTH): Can a member of a clone of a group get its own operation?** (Concept §2.5 names "any depth" and "a clone"; `0106` criterion 18 makes clone rows not expandable.)
- A: no in this slice. A clone takes assignments as a whole.
- B: yes. The clone's members expand in the Layers tree as read-only rows, as a placed page's do, and take assignments (criterion 11 applied to clones).
- C: no for now. To assign a member, the maker unlinks the clone first (`0049` Unlink clone).
- Recommendation: B (a composition that is engraved in one place and cut in another needs it).
- Answered B by the customer 2026-11-09: a member of a clone takes its own assignment; the member rows expand as read-only rows and are addressed by path through the clone (criterion 11).

**Frage 168 (CUTOUT-SET): One Cut-out operation with several items: one contour each, or one for all?** (Criterion 23.)
- A: each assigned item gets its own contour; a composition is cut in one line by grouping its parts first (concept §3: "group, one assignment").
- B: all items of one Cut-out operation are joined into one contour, grouped or not.
- C: one item per Cut-out operation; a second item is refused with a notice.
- Recommendation: A (it keeps Cut-out's meaning, one item's outer contour, and grouping is already the way to make a composition).
- Answered A by the customer 2026-11-09: one Cut-out contour per item (criterion 23).

**Frage 169 (GONE-ITEM): An assignment whose source object is deleted on its drawing page: what does the laser page show?** (Criterion 27.)
- A: the assignment is dropped on the next read, silently.
- B: it stays as a "Missing item" row in the Operations tab, runs nothing, and the maker removes it there.
- C: the assignment is dropped, and the laser page shows a notice once when it is next opened: "An assignment was removed: its object is gone."
- Recommendation: B (what runs stays visible, and nothing is lost silently).
- Answered A by the customer 2026-11-09: an assignment whose object is deleted is dropped silently, in the same step; no Missing item row (criterion 27).

**Frage 170 (DEEP-PRESS): How does the maker assign something inside a placed page?** (Criterion 11. The concept's flow 5 says "double-click into it as into a group"; `0106` criterion 15 says a double-click activates the source page.)
- A: in the Layers tree only (the inner rows of criterion 11). The double-click keeps `0106` criterion 15.
- B: also a double-click on the placed page on the output page enters it for assignment, as for a group. This changes `0106` criterion 15 for placed pages on laser pages.
- Recommendation: A (`0106` is accepted, the inner rows already exist, and one place is enough to learn).
- Answered A by the customer 2026-11-09: assignment inside a placed page in the Layers tree only (criterion 11).

## UX notes

Written by the ux-engineer (2026-11-09): `specs/0101-laser-operations/ux-notes.md`, covering the Operations tab, the operation rows and details, Show and Output, the add group, assignment (tick, drag, Layers, digits), the Cuts last switch, the clone member rows (Frage 167 B), keyboard routes, empty states, accessible names and the look (the symbols, red and pale orange, the Score and Cut-out line styles, the hatch). Open points for the lead are its section 16 (inner row selection needs a `0106` amendment; Selection block scroll; criterion 10 text is 72 characters and is proposed at 67). Proposals in the spec and the notes are vetoable at the demo.

## Links

Requirements: R-MFG-002 (roles become operations), R-MFG-005 (decided for laser output by criteria 13, 15, 16; `docs/requirements.md` to be updated at acceptance), R-MFG-006, R-MFG-010, R-EDIT-038, R-SYS-002, R-SYS-007
Parent: `specs/0063-manufacturing-workbenches/` (slice 4; concept `concept-output-pages.md` §2.4 to §2.6, §3, §5 flows 5 to 7, §7, §8, §9)
Replaces: `specs/0027-manufacturing-roles/` (superseded; PR #269 closed unmerged, 152 A). Its text is not in `main`; the rules and budgets in this spec come from the epic and the concept, and the lead checks them against the closed PR's text before Ready.
Depends on: `0098` (page kinds, Done), `0099` (sketch pages, Done), `0100` (bring content in, Done), `0106` (move versus edit, Done), `0049`, `0057`, `0050`, `0054`, `0038`, `0060`
Next: `0102` `laser-fixtures`; `0103` `fixture-jobs`; `0028`; `0029`
Changes: `specs/0106-move-versus-edit/`, `specs/0098-page-kinds/`, `specs/0050-layers-and-objects-panel/`
ADRs: [ADR 0046](../../docs/adr/0046-laser-operations-record.md) (Proposed, accepted with the PR; operation nodes under the manufacturing, `member.` keys by node id or by path from a clone (Frage 167 B), format 39, amends ADR 0033 on acceptance) and `adrs.md` (architect: the nearest rule beside the membership read of ADR 0043 §1, the two caches, the view JSON contract, the budgets of criterion 29). ADR 0046 replaces nothing. [ADR 0041](../../docs/adr/0041-membership-lifecycle.md) (membership lifecycle of `0097`) is not replaced.
Format: `format_version` 39 (main is at 38; the number is taken at merge). Operations are child nodes of the laser manufacturing node (`shape` `operation`, ADR 0046 §1), not of the output page's record. Each membership is a `member.<key>` register on its operation node: the node id, or for a clone or a clone member the path of node ids from the outermost clone (ADR 0046 §2). The Cuts last switch is the `x.curvyo.laser.cuts_last` register on the manufacturing node (ADR 0046 §3).
Goldens (`curvyo-document-core/tests/fixtures/`): `operations_v39.curvyo` (with clone member keys), `operations_unknown_kind_v39.curvyo`, `operations_dangling_v39.curvyo`, `operations_unreadable_v39.curvyo`; SVG export is unchanged (operations are not drawn in an export).
PR: -
