# Fixture sets and pins: a Fixtures tab with named machine sets a page switches on, job fixtures of one page, the pin kind, centre marks and diameters as jobs, centre snapping, lock and "Apply fixtures from page"

Status: Draft (Ready after the customer merges this PR)
Priority: Should
Origin: Customer (laser redesign after the demo of PR #333, decisions of 2026-11-12: Frage 207 B, 208 A, 209 A, 210, 211 B; screens 06 to 09 of `docs/design-drafts/2026-11-laser-redesign/` accepted: "die screens sehen gut aus"). Third of the three stories of the redesign, after `0111` `laser-operation-tools` and `0112` `test-grid-and-materials`. Details marked **(Proposal)** are the product owner's and vetoable at the demo.

## User value

As a maker who holds work on a bed board, a coaster jig or loose pins, I want my machine's fixtures in named sets that each project switches on or off, plus fixtures that belong to one job only, round pins and screws I can lay out as a grid and lock, and centre marks and diameters I can laser onto the board, so that the same bed serves every project and a one-off jig is set up once, snapped to my drawing and copied to the back of the piece.

**What other tools do.** LightBurn has no fixtures; its jigs are drawn as ordinary shapes on a layer that is switched off by hand, and a flipped job needs a mirrored copy. Fusion 360 keeps fixtures as bodies of a setup. **We do better:** the machine's fixtures are sets a page turns on by one check box, the job's own pins are placed by snapping their centre to a hole of the drawing and locked against a nudge, the centre marks and circles run as a Score job that is off until the maker turns it on, and the pins of the front page are copied to the back page in one press.

## Words (shown in the UI)

- **Fixtures tab:** the tab of a laser page that holds all fixtures. Strip position after Operations and, where the Materials tab of `0112` exists, after it (`0111` draft P6). The Fixtures block leaves the Document tab (see "Changes to accepted specs").
- **Machine set** (shown as "Set"): a named group of fixtures that belongs to the page's machine. It is a name and its fixtures, nothing more. A machine can have any number of sets.
- **On this page:** the check box of a set. A set that is on is drawn, hit, snapped to and run on this page; a set that is off is none of these. The choice belongs to the page, not to the machine.
- **Job fixture:** a fixture of this page only (pins, a screw, a holder for this job). It is not in a set and not in the machine profile.
- **Pin:** a round dowel fixture with a diameter, single or gridded.
- **Grid (of pins or screws):** Columns and Rows of the same pin or screw, evenly spaced from the first one. Not the engraved bed grid of `0102`, which stays the fixture kind "Grid".
- **Centre mark:** a small cross inside a circle at the centre of a pin or screw, with a settable Size.
- **Diameters:** the circle of a pin, or the head and thread circles of a screw, as engraved lines.
- **Lock:** a toggle of a job fixture that fixes its place.
- **Apply fixtures from page:** copies the job fixtures of another laser page to this page.

## Acceptance criteria

"Laser page", "active page", "Active" and "fixture" are used as in `0102` and `0103`. Anything not named here keeps the accepted behaviour of those specs.

### The Fixtures tab and machine sets

1. Given a laser page, when the maker opens the Fixtures tab (`Holder` symbol, after Operations and, where it exists, Materials), then it shows two sections in this order: **Machine sets** and **Job fixtures**. A page of another kind has no Fixtures tab. The Document tab no longer shows a Fixtures block.
2. Given the Machine sets section, then each set is one folded row: a chevron, the set's name, a second line with the kinds it holds and their count ("Holder, Screw · 2 fixtures"), an **On this page** check box, an armed trash and up and down arrows. The section heading reads "<n> of <m> on this page". A set that is off shows its name muted and "not on this page" in line 2, and its fixtures are not drawn, not hit, not snap targets, not selectable on the canvas and run no job (criterion 6).
3. Given a "+ Set" button (symbol and word) in the section, when it is pressed, then a set named "Set <N>" (N the lowest number not used) is added, empty, **On this page on**, unfolded, with one undo step "Add fixture set". A machine with no set (Generic laser) shows only "+ Set".
4. Given a set row, when the maker double-clicks its name (F2 on the selected row is the same), then the name is edited in place as `0102` criterion 3 says (Enter or leaving commits, Escape cancels, up to 40 characters, need not be unique); one undo step "Rename fixture set". The controls of the row leave while the field is open.
5. Given a set row, when the maker presses its armed trash (first press arms, a second press after 400 ms removes, as `0081`), then the set and all its fixtures and their jobs are removed and nothing else changes; one undo step "Remove fixture set" restores them exactly.
6. Given the arrows of a set, or a drag of its row (an insertion line shows the drop place), when the order changes, then the sets and their fixtures keep their contents and the new order is the order of criterion 3 of `0103` (jobs run: sets in this order, then job fixtures); one undo step "Move fixture set".
7. Given a set row is unfolded (chevron), then it shows, one level in, its **add bar** and its fixture rows. The add bar holds symbols only (the tooltip and the accessible name are the kind's word): Grid, Holder, Clamp, Stop, Screw, Pin; Grid is hidden while this set has one (`0102` criterion 2). A fixture is added with the proposal sizes of `0102` criterion 2, centred on the work area, **Active on**, selected, one undo step "Add fixture". Fixture rows, their rename, armed trash, Active check box, eye and the on-canvas editing are those of `0102` criteria 3 to 6, 8 to 14 and 18 to 23; the Fixtures block fields of `0102` that stay (the grid's Spacing, Major every, Tapes, Origin, the slot rows) are shown in the row of the fixture in this tab. Press to unfold or fold is not an undo step and is kept for the session only.
8. Given a fixture inside a set (Frage 211 B), then it keeps its own **On the bed** check box (`0102` criterion 5) besides the set's On this page: it lies on the bed only when its set is on this page **and** its own check is on. Both switches are undo steps ("Set fixture active", "Set fixture inactive", "Set fixture set on page", "Set fixture set off page"). A new fixture starts Active.
9. Given a machine profile (`0026`), when a new laser page is made from it or the maker picks it on a page, then every set of the profile is copied onto the page, **all sets On this page** (Proposal: a new page starts with the whole bed, as before this slice), with fresh node ids and in the profile's order, in the same undo step as the pick (ADR 0050 §4). The page keeps its own On this page choices when the maker later presses Update profile or Reset from profile (Reset from profile restores the sets' contents and order; On this page is turned on for every set).
10. Given the Machine block of the profile (`0026`), then its row reads "Fixture sets in profile: N" (N sets). **Save as profile** and **Update profile** (armed) write the page's sets (name, order, fixtures and their jobs, no On this page, no job fixtures) to the library; **Differs from profile** compares the sets in order by name and fixtures (kind, name, Active, parameters, matrix, slots, jobs as `0103` criterion 23 says); On this page and job fixtures are not compared (Proposal).
11. Given a laser page that has no set and no job fixture, then the Fixtures tab shows both sections with their add controls and nothing else; the "Apply fixtures from page" button is hidden while no other laser page has job fixtures (criterion 33).

### Job fixtures

12. Given the Job fixtures section, then its add bar holds symbols only (tooltip and accessible name = the kind's word): Pin, Screw, Holder, Clamp, Stop (no Grid; `0102` allows one engraved grid per set only). Each job fixture is a row of the same shape as a set's fixture row, plus a **Lock** toggle (criterion 26), and its second line shows kind, count, size, "locked" and, last, the jobs that are on ("Pin · 2 × 2 · ⌀ 6 mm · locked · Centre mark").
13. Given a kind pressed in the Job fixtures add bar, then placement is armed (Proposal, `0111` P17): the button stays pressed, the fixture hangs at the pointer by its centre with a dashed outline, a click drops it as one undo step "Add job fixture" (named "<Kind> <N>", N the lowest number not used among the page's job fixtures of that kind; selected; **Active on**), and Placement stays armed for the next fixture after a drop; Escape, a right click, a second press on the pressed button or another tool ends it without adding. While placement is armed, a click on the canvas does not select anything. With the pointer outside the canvas the fixture is not shown.
14. Given a job fixture, then it lies on the page only: it is in the project file, in no set, in no machine profile, and Save as profile, Update profile and Reset from profile neither write nor remove it. Switching a set off does not affect it; picking another profile keeps it (Pick profile replaces the sets only, in the same undo step as `0102` criterion 29).
15. Given a job fixture, then it is a fixture in every other way: selected by its outline on the active page (`0102` criterion 8), hidden by its eye, switched by its own On the bed check, edited by the handles and chips of its kind (`0102` criteria 14, 18, 22, 23, 38), never output as a drawing (`0102` criterion 7) and listed in the Layers tree's Fixtures row. It may carry fixture jobs of `0103`. A job fixture is below the placed pages and the own items, as a set fixture is.
16. Given the order of everything that lists fixtures (the Layers row, the group order of `0103` criterion 3, the summary), then it is: the sets in their order, each with its fixtures in creation order, then the job fixtures in creation order.
17. Given a page that holds more than 64 job fixtures or more than 16 sets, then the add control of that section is hidden (not disabled) and no notice is shown (limits 16 sets and 64 job fixtures confirmed in `adrs.md`; the limits of a profile record's sets are those of ADR 0060 §2).

### Pin

18. Given a pin, then it is a circle with a **Diameter** of 0.5 to 1,000 mm, default 6 mm (Proposal), solid outline in the fixture ink. A typed value outside the range is set to the nearest limit with an Action notice of at most 70 characters (the pattern of `0102` criterion 20).
19. Given a pin or a screw, then it may be single or a grid: **Columns** and **Rows**, each an integer of 1 to 50 with a **Spacing** of 1 to 1,000 mm (centre to centre), default count 1 and spacing 20 mm (Proposal). The centres lie evenly spaced from the first pin: columns run to the right and rows run down on the screen. The product of the counts is at most 400 (Proposal); a typed count that would exceed it is set to the largest allowed count with a notice. The pins of a grid have one diameter; the grid is one fixture and has one row, one trash and one name.
20. Given a pin or screw row, then Columns and Rows are shown as two units, each count and its spacing in one row that wraps as a whole; Spacing is hidden while its count is 1 (`0111` draft 07). Each field commit is one undo step "Resize fixture". A pin shows its Diameter field in the row; a screw shows no Head or Thread field (Frage 211: the accepted knob presentation of `0102` criterion 38 stays, nothing is added).
21. Given a selected pin or screw grid on the canvas, then it is edited like a rectangle: its box, the centre move handle (M opens the Move chip as `0102` criterion 14), plus (Proposal, `0111` draft 07C) a **Diameter** knob on the first pin (pin only; the screw keeps the Head and Thread knobs of `0102` criterion 38 on the first screw), a **Columns** knob at the centre of the last pin of row 1 and a **Rows** knob at the centre of the last pin of column 1; a knob is hidden while its count is 1 (the count is then typed in the row or set by the chip of the other knob). A drag of a Columns or Rows knob changes the spacing; a double-click on it opens its chip with the count and the spacing. Each commit is one undo step "Resize fixture". A pin or a screw grid does not turn: no rotate glyph and no R (`0102` criterion 38), no skew.
22. Given a pin, then it is stored as a fixture of the kind `pin` with the parameters `diameter`, `columns`, `rows` and the two spacings `column_spacing_mm` and `row_spacing_mm`, the Centre mark Size as the fixture parameter `mark_mm` (jobs carry no settings, ADR 0052 §1), and a screw gains columns, rows, the two spacings and `mark_mm` beside its `thread_diameter` and `head_diameter`. An unknown kind keeps round-tripping as `0102` criterion 32 says.
23. Given a pin or a screw, then it is not a snap target and not a fit target (as `0102` criterion 40 says for the screw). It is shown, selected and moved like the other fixtures.
24. Given a pin, a screw and a grid of either, then no back-side aid exists: the pins are not mirrored, flipped or paired with the back (Out of scope).

### Centre mark and diameters

25. Given a pin or a screw, then its unfolded body shows a **Jobs** pair of toggles (the 40 px row has no room; line 2 of the row names the jobs that are on), each with a symbol and its word: **Centre mark** and **Diameters**. Both start off. Centre mark has a **Size** field (0.5 to 50 mm, default 4 mm, Proposal) shown only while the toggle is on. Each toggle is the **Output** switch of a fixture job (criterion 27), so its state and its undo step ("Output on" or "Output off") are those of `0103` criterion 4. A Centre mark is drawn at the centre of every pin or screw of the grid: a short cross inside a circle of the Size as diameter, the cross lines spanning that circle. Diameters draw the circle of the pin at its diameter, or the head and the thread circle of the screw at theirs, at every centre. Both are shown on the canvas in the Score look of `0101` while their toggle is on and the fixture and its set are on the bed, and in the fixture ink otherwise (the look rule of `0103` criterion 18 applies).
26. Given the two toggles, then both run as **Score** (the kind of `0101`): a line along the circle and the cross lines at the pin's or the screw's centres, once for each centre. A centre mark larger than the pin's diameter is allowed without a notice (permit over refuse).
27. Given a pin or a screw is added, then it is created with the two fixture jobs of `0103` named "Centre mark" and "Diameters", kind Score, **Output off**, with fixed parts: Centre mark takes the part key suffix `.marks`, Diameters takes `.circle` on a pin and `.head` and `.thread` on a screw (Proposal; `0103` part keys). Their Parts lists show those ticks as read-only; the maker cannot tick other parts of a pin or a screw, cannot delete these two jobs by their trash and cannot rename them (they live and die with the fixture). The two jobs are stored with a role (`job_role`: centre mark, diameters), so a toggle finds its job by role; jobs a screw already got under `0103` stay and run. The Fixture jobs section of the Operations tab lists them like any other job, with the same Output switch as the toggle (one state in two places). Further fixture jobs on a pin or a screw are not offered (Proposal).
28. Given the jobs of a pin or screw, then `0103` criteria 13 to 15 apply with one addition: a job runs only when its fixture's set is on this page, besides the fixture being Active; a job whose fixture is on a set that is off shows "not on this page" (priority in the row: not on this page, then not on the bed, then Output off, then the part count). A job fixture has no set and no such condition.
29. Given the page is output (`0029`), then the rule of `0103` criterion 6 turns these Output switches off like any other fixture job's. This is `0029`'s acceptance, not this slice's; nothing here outputs a page.

### Placing by centre and Lock

30. Given a job fixture that is placed (criterion 13) or dragged, then its centre (for a grid, each centre of the grid) snaps to the centres that the Objects target of `0060` offers on the page's drawing: the centre of a circle, an ellipse, a polygon, and of the box of any other shape, with the capture, release, hysteresis and Alt rules of `0060` and its marker (red lines along the matched axes, a cross at the point, no text). The snap is on only while the Objects target is on. Set fixtures do not centre-snap (Proposal: the bed is placed by the typed or dragged rules of `0102`).
31. Given a job fixture, then it has a **Lock** toggle (lock symbol) in its row, and the Select bar of a selected job fixture holds the Lock toggle (symbol and word) as its only control (Proposal; the transform and operation controls are hidden for fixtures). Each press is one undo step "Lock fixture" or "Unlock fixture"; the row and the bar show the same state.
32. Given a locked job fixture, then it keeps its place and size: no drag from its outline or from inside its box, no arrow-key nudge, no handles, no knobs, no Move chip, no S chip; it stays selectable, shown, listed, switched by its eye and its Active check, and it still runs its jobs. A drag on it does nothing and starts no marquee. It shows a lock badge at its upper right on the canvas and "locked" in line 2 of its row. The trash still works. Set fixtures have no Lock (Proposal: the bed is not nudged by accident; a set fixture is changed in its set).

### Apply fixtures from page

33. Given a laser page, when the maker presses "Apply fixtures from page…" in the Job fixtures section, then, in place (no popup), the other laser pages that have at least one job fixture are listed, each with its count ("Box lid front · 4 job fixtures"); the page itself and pages without job fixtures are not listed. The button is hidden while no page qualifies. Cancel or Escape folds the list; no undo step.
34. Given a page picked in the list, then (Proposal) its job fixtures are previewed on this page in the dashed blue live preview, and the button reads "Apply <n> fixtures".
35. Given "Apply <n> fixtures", then all job fixtures of the picked page are copied to this page in one undo step "Apply fixtures from page": at the same work-area positions, with the same names, kinds, sizes, grids, Lock states and eye, each as an independent fixture with fresh node ids (a later change of the copy changes nothing on the source and the reverse). Their fixture jobs are copied with **Output off** (Proposal: never output by accident, as `0103` criterion 5); the part ticks follow. The copies are added after this page's own job fixtures; nothing is replaced and nothing is renamed to avoid a duplicate name. Applying the same page twice gives two copies.
36. Given a copy lies partly outside this page's work area, then it is kept at its position (permit over refuse) with the notice "<n> fixtures lie outside the work area." of at most 70 characters.
37. Given Apply, then the fixtures are never mirrored (customer, Frage 210): the maker flips the design with the existing tools (`0056`); only the pins are copied as they are.

### Files, failures and round trip

38. Given a laser page with two sets (one off for this page), in them a grid, a holder with two slots, a screw as a 4 × 1 grid with Centre mark on and a 3 mm Size, and a fixture with its own check off; two job fixtures, a locked 2 × 2 pin grid with Diameters on and a single pin; and a fixture job carrying a hand-set tick, when the project is saved and reopened, then every set (name, order, On this page), every fixture, grid, Size, Lock, Output and tick is identical, and the golden `fixture_sets_v<N>.curvyo` (`curvyo-document-core/tests/fixtures/`, `<N>` from "Format") passes.
39. Given a file saved before this slice (page-level fixtures, no set), when it is opened, then those fixtures are converted when the file opens, in one step that is not an undo step and is not saved until the maker saves (the file on disk stays unchanged until then): they become one set named "Set 1", **On this page on**, and every screw gets its two role jobs (Centre mark, Diameters) with Output off, its other jobs unchanged (ADR 0059; golden `fixtures_pre_sets.curvyo`). Given a profile record of a format before this slice, then its flat `fixtures` array reads as one set "Set 1"; the record is not rewritten on read, and any write of a record writes the new record format with named sets (ADR 0050 §3).
40. Given a fixture kind or a set value that is damaged (text where a number belongs, a non-finite number, a missing set name, a pin with no diameter, a count of 0, a product above 400), then the field reads as its default, a damaged fixture is skipped, and one notice of at most 70 characters counts the damaged values of the set or of the job fixtures ("2 values damaged in Set 1."), as `0102` criterion 33; the file is never rewritten on read. Goldens `fixture_sets_damaged_v<N>.curvyo`. A damaged value inside a profile record's set makes the whole record damaged and skipped with its notice (`0102` criterion 30).
41. Given a profile record of a newer format than this build knows, then it is skipped with its notice, as `0026` criterion 13 says.

### Budgets and no I/O

42. Budgets, thread CPU time in `curvyo-editor-wasm` (tests under `curvyo-editor-wasm/tests/cpu_time/`, never wall time; figures confirmed in `adrs.md`): (a) on a laser page with 3 sets of 20 fixtures, 64 job fixtures and one pin grid of 400 centres with Centre mark and Diameters on, a press is resolved (hit test) within 2 ms and the draw list is built within 16 ms; (b) each pointer move of a drag of a job fixture with centre snapping, on a drawing with 1,000 shapes, takes within 16 ms; (c) Apply of 64 job fixtures commits within 100 ms; (d) the Fixtures tab with 16 sets and 64 job fixtures shows its first rows within 100 ms.
43. Given the whole slice, then there is no network access, no device I/O, no G-code, no output file and no telemetry (`R-SYS-002`, `R-SYS-007`). The sets, job fixtures, marks and circles are not in any SVG export, print or drawing export. A job fixture is never part of a library record except as a set fixture.

## Changes to accepted specs

This slice changes accepted text; the old text is superseded by the criterion named here. The format numbers are the architect's; this table names the change.

| Spec | Criterion | Was | Now |
|---|---|---|---|
| `0102` | 1 (Fixtures block in the Document tab) | one block listing the page's fixtures | its own Fixtures tab with Machine sets and Job fixtures (1 to 7, 12, 15); the block leaves the Document tab |
| `0102` | 2 (five kind buttons, text and symbol) | Grid, Holder, Clamp, Stop, Screw, each added to the page | symbol-only add bars; a sixth kind, Pin; the bar of a set offers Grid (hidden when it has one), the bar of Job fixtures does not (7, 12, 13) |
| `0102` | 5 (Active, any number at once) | a check box per fixture | kept as the fixture's own "On the bed" check and joined by the set's On this page (Frage 211 B): drawn only when both are on (2, 8) |
| `0102` | 12, 16 (Layers Fixtures row; listing in creation order) | one flat list | the sets in order then job fixtures, one row per fixture (16) |
| `0102` | 14, 38 (screw and the rotate rule) | a screw has no rotate glyph | a pin and a grid of pins or screws do not turn either (21); no other change to the accepted screw |
| `0102` | 37 to 40 (screw: head and thread knobs, no snap point) | one screw, Head and Thread by knobs | a screw may be a grid of Columns and Rows (19 to 21) with centre mark and diameter jobs (25 to 27); the Head and Thread presentation and no-snap rule are unchanged (Frage 211, 23) |
| `0102` | 28 to 30 (profile holds one flat fixture set; page holds a copy) | one unnamed list of fixtures | named sets that the page may switch on or off, plus job fixtures kept in the page only (9, 10, 14, 39) |
| `0102` | Out of scope ("a fixture moved to another laser page") | out | job fixtures are copied to another page by Apply (33 to 37); set fixtures still travel only through the profile |
| `0103` | 1, 2, 3 (Fixture jobs section, grouping) | jobs grouped by fixture in the Fixtures block order | the order is sets then job fixtures (16); a pin or a screw owns two fixed jobs listed read-only (27) |
| `0103` | 8 (Parts of a fixture) | grid, holder, clamp, stop, screw parts | plus pin part `.circle` and the part `.marks` of a pin or a screw; a screw's `.head` and `.thread` also serve the Diameters job (27) |
| `0103` | 13, 15 (a job runs when its fixture is Active) | Active fixture, Output on, known kind, one tick | plus: its set is on this page; a job of a set that is off reads "not on this page" (28) |
| `0103` | 23, 24 (profile record format 3, jobs inside the fixture entries) | `fixtures` array with `jobs` | the entries sit inside named sets of a new record format; a flat array of an older record reads as one set (10, 39) |
| ADR 0049 | §2 (fixture node), §3 (children), kinds | five kinds, slot and job children, fixtures under the manufacturing | a sixth kind `pin`; count and spacing parameters for pin and screw; a set node holds its fixtures; job fixtures sit beside the sets, with a `locked` register; amended on acceptance by a new ADR |
| ADR 0050 | §1 to §5 (record, copy into a page) | `fixtures` array, copy replaces the page's fixtures | a `fixture_sets` array of named sets; the copy replaces the page's sets only and keeps job fixtures; On this page is a page register (9, 14) |
| ADR 0052 | §2, §7 (part keys, profile tokens) | keys and tokens of five kinds | pin tokens, `marks` and `circle`; page to page copy of jobs with Output off (27, 35) |
| `0026` | Machine block row, criterion 7 | "Fixtures in profile: N" | "Fixture sets in profile: N"; Differs compares the sets (10) |
| ADR 0039 | §4, §5 (the page's copy of the profile) | the copy holds the flat fixture set | the copy holds the sets and the page's On this page choices (9) |
| `0098` | 13 (Fixtures block) | in the Document tab | in the Fixtures tab (1) |
| `0106` | 17 (Fixtures row in Layers) | one row per fixture | unchanged except the order and that fixtures of a set that is off are listed muted and not hit (2, 16) |
| `0060` | Objects snap target | snaps moved items | also offers its centres to a job fixture being placed or dragged (30) |
| `0111` | draft README, P14 to P18, screens 06 to 09 | proposals | accepted as the basis; P16 is reduced: the screw keeps its current Head and Thread presentation (20) |

## Out of scope

- Mirroring or flipping job fixtures, a back-side aid or alignment helper, any pairing of front and back pins; the design is flipped with the existing tools (Frage 210).
- Locking set fixtures, grid or slot fixtures; locking pages or drawing items.
- Centre snapping for set fixtures, and snapping of other objects or pages to a pin or screw centre (criterion 23).
- Hex or other pin shapes, pin height, thread pitch, a gridded holder or clamp, a rotated grid of pins, and a grid of the engraved bed grid kind.
- Other jobs on a pin or a screw than Centre mark and Diameters, and Cut or Engrave settings of them (`0103` stands: no settings in a job row).
- The page output and the turn-off of Output after it (`0029`).
- A focus test and a motorised Z axis (`0111`), the test grid, materials and sets of settings (`0112`).
- Sharing a set between machines, importing or exporting a set, and a live link from a set to the profile (ADR 0050 option C stays rejected).
- Undo of profile library changes (`0026` out of scope stands).
- Fixtures of machine families other than the laser.

## Open customer questions

None. Every point of the slice was decided on 2026-11-12 (Fragen 207 B, 208 A, 209 A, 210, 211 B); the Proposals above have safe defaults and are vetoable at the demo. Numbers 212 and up stay free.

## UX notes

Draft screens: `docs/design-drafts/2026-11-laser-redesign/06-fixtures-tab` to `09-apply-fixtures-from-page` (HTML and PNG), real tokens, 280 px body. Two things in the screens are superseded here: screen 07B shows Head and Thread fields in a screw row (not built, criterion 20), and screen 06 names Frage 211 as open (decided B, criterion 8). `ux-notes.md` is complete.

## Format

`format_version` 46 (ADR 0059; `<N>` above is 46; main is at 43 and the numbers follow the merge order of `0111` 44, `0112` 45), profile record format 5 (ADR 0060): a set node, a `pin` kind and a `locked` register change the node shapes, so an older build must refuse the file by version rather than read it as damage (ADR 0033 §7). Migration: a file before the bump is converted when it opens (criterion 39): page-level fixtures become "Set 1", screws gain their role jobs; one step, not an undo step, not saved until the maker saves. The machine profile record gets a new record format (`fixture_sets`, ADR 0038 and 0050 pattern); older records still read. Goldens: the project goldens named in criteria 38 to 40 in `curvyo-document-core/tests/fixtures/`; profile goldens (a record with two sets, an old flat record, a damaged set, a newer record) in `curvyo-laser-core/tests/fixtures/machine_profiles/`; the earlier goldens stay and open unchanged.

## Links

Requirements: R-MFG-007 (fixtures and the work area), R-MFG-008 (placement by fitting size, unchanged), R-MFG-010 (unknown kinds round-trip), R-SYS-002, R-SYS-007
Parent: `docs/design-drafts/2026-11-laser-redesign/` (P14 to P18, screens 06 to 09); story 3 of 3 of the laser redesign
Depends on: `0102` (fixtures), `0103` (fixture jobs), `0026` and ADR 0038, 0039, 0050 (the profile and its page copy), `0060` (snapping, Objects target and its marker), `0081` (armed trash), `0101` (Score kind, look, Operations tab), `0111` `laser-operation-tools` (tab strip, view toggle) and `0112` `test-grid-and-materials` (Materials tab before Fixtures)
ADRs: `adrs.md`, ADR 0059, ADR 0060. UX: `ux-notes.md`.
Changes: `specs/0102-laser-fixtures/`, `specs/0103-fixture-jobs/`, `specs/0026-machine-profile/`, `specs/0098-page-kinds/`, `specs/0106-move-versus-edit/`, `specs/0060-snapping/`; ADR 0049, 0050, 0052, 0039 §4 and §5, 0038 record format; `docs/requirements.md` (R-MFG-007 after the demo)
PR: -
