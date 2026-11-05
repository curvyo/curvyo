# Manufacturing workbenches (epic): output pages with operations and fixtures, the tools following the page

Status: Draft (epic; reworked 2026-11-05 for concept A, accepted by the customer. Slices `0098` to `0103`, then `0028` and `0029`; the first slice is `0098`)
Priority: Should (the laser part is the Must MVP; 0098, 0100, 0101, 0028 and 0029 are on the road to "engrave with Curvyo")
Origin: Customer (request of 2026-10-17, translated): "Start with light refinement; not an easy feature. Workflow: you decide to add a manufacturing (or whatever we call it) for a laser cutter. The machine is specific: defined work area, controls and possibilities. In the work area there can be fixtures to lay out or clamp the parts, plus maybe a measuring tape. There is a fixture-preparing step to engrave or cut out tapes, layout outlines, insertion pockets, etc. Then you position the pages of the documents on the work area; they snap in by fitting size, can be dragged there, etc. You can still scale and rotate there, without changing the original drawing. Changes in the original drawing carry over live into the manufacturing. CNC milling is very similar, but e.g. an embroidery machine is entirely different [...] A Curvyo project can be used for several manufacturing processes." Redirected by the customer's feedback of 2026-11-05 (concept rounds 1 and 2, `concept-output-pages.md` §0): "No workbench switching. The laser output is a completely normal page; the workbench follows the active page; Add page has a machine switch."

## User value

As a maker with a laser (later a mill, a plotter, an embroidery machine) I want the machine's bed to be an ordinary page of my project, where I bring in my designs as live clones or whole pages, add connecting parts and text with the normal tools, say per object or group what is cut, scored or engraved, and lay everything into the fixtures on my bed, so that I design once, see exactly what will run in which order, and never have my drawing changed by preparing a job.

**What other tools do, and what we do better** (full table: `concept-output-pages.md` §1). LightBurn makes an object's colour its machine instruction and copies everything in; we keep the drawing clean and link it live. PewPew Studio and node tools wire sources to operations in a graph; we keep the canvas and one ordered list. xTool sets the process per object with no overview; we keep the one-press assignment but in a list you can reorder and switch off. Fusion 360 selects geometry per operation and has real fixtures, behind modal dialogs and a manual workspace switch; we take the explicit selection and the fixtures, with no dialog and no switch: the tools follow the page you work on.

## Words (shown in the UI)

- **Page kind:** every page is a Drawing, a Sketch or an output kind for a machine (Laser first). Chosen once in Add page, never changed, shown as glyph and word next to every page name.
- **Output page:** a page of a machine kind. Its size is the machine's work area. Its content: placed items (linked clones, placed pages; live), own items (made there with the normal tools) and fixtures.
- **Operation:** one step of the machine job on an output page: a kind (Cut, Score, Engrave, Engrave + Outline, Cut-out), a name, a colour, settings, Show and Output. The list order is the run order.
- **Assignment:** which items an operation applies to, at any depth; a leaf runs with the operations of the nearest node above it (itself included) that has any.
- **Fixture:** a holder, clamp, stop or the engraved grid on the bed; part of the machine setup; selectable; a snap target.
- **Fixture job:** an operation of a fixture (engrave the board outlines once), Output off by default.

The word "workbench" stays in code and specs for the tool set of a page kind; the UI never asks the maker to choose one.

## Slices, in order

Every slice is one story and one PR that the customer can try end to end.

| # | Spec | Slice | Meaning in one line | Priority |
|---|---|---|---|---|
| 1 | [`0098`](../0098-page-kinds/) `page-kinds` | Page kinds, no switcher | Add page asks Drawing, Sketch or Laser; the tools and tabs follow the active page; the workbench switcher is gone; the laser page is a normal listed page. | Must |
| 2 | `0099` `sketch-pages` | Sketch pages | A sketch page holds its sketch and nothing else; the sketch is fixed to its page and is used elsewhere only as a linked clone. | Should |
| 3 | `0100` `bring-content-in` | Bring content in, move versus edit | Dragging a design onto the laser page makes a live linked clone (Alt moves, Ctrl copies), a dragged page row a placed page; objects move anywhere but are edited only in their page's context. | Must |
| 4 | `0101` `laser-operations` | Operations | An ordered list of operations per laser page, assigned to any object, group, clone or placed page, with Show, Output and the Cuts last switch. Replaces `0027`. | Must |
| 5a | `0102` `laser-fixtures` | Fixtures as objects | Grid with tapes and major lines, holders with slots, clamps and stops as selectable objects of the bed, each Active or not; parts and pages snap into slots by fitting size. | Should |
| 5b | `0103` `fixture-jobs` | Fixture jobs | A fixture carries its own operations (engrave the board outlines once), listed below the work with Output off by default. | Should |
| 6 | `0028`, `0029` | Settings and output | Power and speed per operation from the material library; preview, gate and G-code that read the operation list in order, holes before outlines inside a cut. | Must |

Slice 5 of the concept (`concept-output-pages.md` §9) is split in two (Proposal): fixtures are useful without fixture jobs, and fixture jobs need the operations of slice 4. Slices 2 and 3 can run in parallel. Later machine families (CNC, plotter, embroidery) add their kinds to the same list, assignment, Show/Output and fixtures; only the kinds and their settings are per machine (concept §3). Device I/O and the connector stay with `0064`.

## Acceptance criteria (epic level; the slices carry the detail)

1. Given any project, then there is no workbench switcher anywhere in the app; the tool rail and the Properties tabs follow the kind of the active page and nothing else (customer, 2026-11-05).
2. Given any place that shows a page's name (page list row, panel header, canvas page label, status bar), then the page's kind is shown next to it as a glyph and a word; never by colour alone.
3. Given an output page, then it is a full editing context (all vector tools, Text, Layers, Snapping, Grid, Share) and its size is the machine's work area, not editable on the page.
4. Given drawing content brought onto an output page, then it is a live reference (linked clone or placed page) whose shape follows the drawing and whose position, rotation and scale belong to the output page; preparing a job never writes the drawing.
5. Given an output page, then what runs is exactly what its operation list says, in list order: an item with no operation never runs, a fixture runs only through an enabled fixture job, and with Cuts last on (the default) no cutting operation runs before a marking operation.
6. Given a project file, then page kinds, output pages, operations, assignments and fixtures round-trip, and a page or process of a kind this build does not know is kept and saved back unchanged (R-MFG-010).

## What changes in built and planned specs

| Spec | What changes | By slice |
|---|---|---|
| `0093` laser-workbench-shell (Done) | The workbench switcher entry and **Add manufacturing** (criteria 1, 2) are replaced by Add page with Kind Laser; the read-only output page (criteria 4, 7) becomes a full editing context, only the inside of a placed item stays read-only; the output page **is listed** and drawn in the plane (WB-PAGE-VIS A reversed); the Laser tab goes, its Machine and Fixtures blocks move to the Document tab; deleting the manufacturing is Delete page. The manufacturing node, owned page, placement link, unknown-process round-trip and extension registry survive (concept §8). | 0098; placement by drag 0100 |
| `0026` machine-profile (Done) | The Machine block moves to the laser page's Document tab; the work area is read-only text there (criterion 1 fields, criterion 7 work-area edit); a changed profile keeps content relative to the origin corner instead of the top-left corner (criterion 5 b); the profile later gains a fixture set. | 0098; fixture set 0102 |
| `0027` manufacturing-roles (reserved, PR #269 closed unmerged, 152 A) | **Superseded by `0101` laser-operations.** Roles keyed by drawing layer (the withdrawn laser-roles ADR of the closed PR #269) are dropped; the geometry rules (style ignored, 140 A open/closed rule, images only engrave), the look tokens, the notice counts, the CPU budgets and the tests move into 0101; "hidden is not output" is replaced by 142 B. | 0101 |
| `0066` constraint-sketcher (Done) | The Vector / Sketch selector (criterion 18, criterion 1's "workbench selector") goes; the Sketch workbench is the tool set of **Sketch pages**; a drawing page has no sketch (criterion 19's page sketch lives on a Sketch page). | 0098; the rest of the sketch-page rules 0099 |
| `0059` multiple-pages (Done) | Add page asks for a kind (criterion 3); the size fields belong to drawing and sketch pages only (criterion 4); the page list and canvas label show the kind (criteria 2, 7). The position rule for membership (criterion 9) stays and decides what an object over a laser page is (155 A). | 0098 |
| `0049` linked-clone (Done) | Clones of groups become possible (criterion 4 forbids them today); on an output page own placement is on by default. | 0100 |

## Decided

### Customer, 2026-10-17 and 2026-11-02 (still valid)

- **WB-ORDER A**, the laser comes first and is the MVP; **WB-LASER-FIRST**, the laser is the first extension of `0065` (compiled in, registered through the front-end registry); **MFG-MODEL (Frage 118) A**, the manufacturing node with an owned output page and live placements (ADR 0032, 0033); **EXT-LASER-UI (Frage 119) A**, declarative laser UI (ADR 0034); **WB-MULTI A**, several output pages per project are allowed by the format; **WB-FIXTURE-HOME A**, fixtures live in the machine setup; **WB-CONNECTOR**, `0064` protocol next.
- No longer valid: **WB-PAGE-VIS A** (output page hidden from the page list), reversed 2026-11-05; the switcher above the Properties panel (request of 2026-10-17, criteria 1 to 3 of the old epic), removed 2026-11-05; **WB-PLACE-DEFAULT A** (the active drawing page placed automatically), replaced by bringing content in by drag (0100).

### Customer, 2026-11-05 (concept A, `concept-output-pages.md` §10)

- **147 OP-ATTACH: A.** An ordered operation list per output page with explicit assignment at any depth; nearest assignment wins.
- **148 OP-KINDS: A.** Cut, Score, Engrave, Engrave + Outline, Cut-out. Noted for later (option C): image optimisation (dithering, resolution, contrast) and Offset fill; both join the Marking section when their story comes.
- **149 HIDDEN-INHERIT: A.** Hidden drawing content runs only with an assignment of its own.
- **150 DROP-LINK: A** (default, not answered). A drag onto the laser page makes a linked clone with its own position.
- **151 SAME-PLANE: A.** All page kinds in one plane and one page list.
- **152 PR-269: A.** PR #269 is closed unmerged; its rules, tokens and tests move into the operations slice.
- **153 MULTI-OP: A.** One item may be in any number of operations; they run in list order.
- **154 RUN-ORDER: B.** The Marking/Cutting rule as a switch **Cuts last** in the Operations tab, on by default; off allows any order (concept §2.6).
- **155 DROP-ON-LASER: A.** A drawing object moved onto a laser page is that page's own item by position, Not assigned; dragged off, a drawing object again.
- **156 PASTEBOARD-EDIT: A.** Pasteboard objects are editable while a drawing page is active; with a laser or sketch page active they only move; a double-click switches to the last drawing page.
- Earlier the same day: **140 A** (Engrave fills closed areas, open paths as a line), **141 A** (nothing runs until assigned), **142 B** (Show and Output separate; a hidden item in an operation with Output on still runs).
- Round 2 feedback: no workbench switcher anywhere; Sketch is a page kind; a laser page's size is the machine's and not editable; objects move anywhere, editing only in their context; fixtures selectable, several active at once, with tapes and major lines on the grid and jobs of their own.

## Out of scope (of the epic)

- Machine families other than the laser before slice 6 is done; a shared machine abstraction before the second family (`CLAUDE.md` §6).
- Nesting (R-MFG-004), a CAM kernel, tool libraries.
- Device I/O and the connector (`0064`), third-party extensions and run-time loading (`0065`).
- A node graph, colour-as-instruction, per-object process settings without a list, modal operation dialogs (refused, concept §1).
- Offset fill, image optimisation, Dot mode (noted for later, 148 C).
- 3D simulation, collision checking, physical machine models.

## Architect work implied (not decided here)

An amendment of the `0059` page ADR (a page `kind` register; for Sketch, the owned sketch) with a `format_version` bump (0098); an amendment of ADR 0032 §5 (owned page listed and in the plane) (0098); group clones, amending ADR 0015 (0100); an ADR for operations and assignments in the file, keyed by output-page node and inside a placed item by source node id, replacing ADR 0041 (0101); the fixture set and fixture jobs in the profile record, an ADR 0038 bump (0102, 0103). The run-order sections are derived from the kinds and the Cuts last switch, not stored per row.

## Links

Concept: `concept-output-pages.md`, `concept-sketches.html` (this folder)
Requirements: R-MFG-001 to R-MFG-010; R-EDIT-038, R-EDIT-042
Slices: `specs/0098-page-kinds/`, `0099` to `0103` (reserved), `0028`, `0029`
Changed by this epic: `specs/0093-laser-workbench-shell/`, `specs/0026-machine-profile/`, `specs/0066-constraint-sketcher/`, `specs/0059-multiple-pages/`, `specs/0049-linked-clone/`; superseded: `0027` (PR #269 closed unmerged)
Related: `specs/0057-clone-overrides/`, `specs/0060-snapping/`, `specs/0090-grid/`, `specs/0046-document-shapes/`, `specs/0064-machine-connector/`, `specs/0065-extensions-sdk/`
ADRs: 0032, 0033, 0034, 0035, 0038, 0039 (accepted); the withdrawn laser-roles ADR of the closed PR #269 (to be replaced)
PR: -
