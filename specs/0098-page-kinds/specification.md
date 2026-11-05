# Page kinds: Add page asks Drawing, Sketch or Laser, and the tools follow the active page

Status: Ready (2026-11-05; ADR 0042 is Proposed and is accepted by the customer with the PR)
Priority: Must
Origin: Customer (2026-11-05, translated): "No workbench switching. The laser output is a completely normal page; the workbench follows the active page; Add page has a machine switch." "The Sketch workbench moves to the same model: a Sketch is a page kind. The workbench switcher goes. Pages show their type." "A laser page's size is not editable: the laser is as big as it is." Concept A accepted the same day (`specs/0063-manufacturing-workbenches/concept-output-pages.md`, slice 1 of §9). Details marked **(Proposal)** are the product owner's and vetoable at the demo.

## User value

As a maker I want to add a laser page the way I add any page, see on every page what kind it is, and get the right tools just by working on that page, so that I never hunt for a mode switch and my laser's bed sits next to my drawings in one project.

**What other tools do.** Affinity switches personas (Designer, Photo, Publisher) by hand over one document; FreeCAD switches workbenches by hand and the maker loses track of which one is active. LightBurn has no pages and no modes: the canvas is the bed. Inkscape has pages but no machine. We do better: the page carries its kind, the kind is visible wherever the page's name is, and the tools follow the page without any switch.

## Acceptance criteria

### Adding a page

1. Given the Document tab, when the maker presses **Add page** (+), then an inline group opens under the page list (no popup) with a **Kind** switch `Drawing | Sketch | Laser`, preselected with the kind last added on this computer, Drawing the first time (Proposal), and the buttons **Add** (also Enter) and **Cancel** (also Escape). For Laser the group also shows the machine profile list of `0026` with the profile last picked on this computer preselected, else Generic laser. Cancel or Escape closes the group and changes nothing.
2. Given the group with Kind Drawing, when the maker presses Add, then a drawing page named "Page N" exists with the size of the last drawing page in the list (else the default document size), placed to the right of the last page with the gap of `0059`, active, and the action is one undo step "Add page".
3. Given the group with Kind Sketch, when the maker presses Add, then a sketch page named "Sketch N" exists with the size of the last sketch page (else of the last drawing page, else the default document size; Proposal), placed and activated as in criterion 2, showing one empty sketch whose origin is at the page centre (`0066` criterion 4), in one undo step "Add sketch page". The sketch node itself is created on the first edit (lazy creation of ADR 0026, Proposal); the maker sees no difference.
4. Given the group with Kind Laser and a profile picked, when the maker presses Add, then a laser page named "Laser N" exists with exactly the profile's work area (Generic laser: 400 x 300 mm), its machine copy as in `0026` criterion 6, no fixture, no placed content, and the origin marker of `0026` criterion 11; it is placed and activated as in criterion 2, and one undo step "Add laser page" removes the page together with its machine data. A new laser page holds nothing until the maker puts something on it (Proposal; the automatic placement of the active drawing page, `0093` criterion 5, is gone).
5. Given any page, then its kind is fixed: no control, command or shortcut changes it. A maker who wants another kind adds a new page.

### Seeing the kind

6. Given a page of any kind, then its kind is shown as a glyph and a word next to its name in four places: the page list row (on line 1 of the name cell; it moves to line 2 if the name is too short at the demo, Proposal), the panel header line ("Document · Keychain · Drawing"; only on the page-scoped tabs Document, Style, Layers, Snapping, Grid, Sketches and Parameters, not on File, History, Fonts and Share, Proposal), the page label on the canvas, and the right block of the status bar ("Laser · Generic laser · 400.0 × 300.0 mm"; "Drawing · 125.0 × 75.0 mm"). Drawing uses `PenTool`, Sketch `DraftingCompass`, Laser `Zap`. The kind is never shown by colour alone, and a screen reader reads the row as "<name>, <kind> page".

### Tools follow the page; no switcher

7. Given any project, then there is no workbench switcher: the row above the Properties tab strip is gone for every page kind (`0093` UX notes §1, `0066` criterion 18), and no other control chooses a workbench.
8. Given a page, when the maker activates it by a press inside its frame, by its row in the page list, or, in the desktop app only, by `Ctrl+Page Down` / `Ctrl+Page Up` (next and previous page in list order, stopping at the ends; Proposal; in the browser the Pages list and a press in the frame do the same, because the browser takes the chord), then the tool rail and the tabs follow its kind in the same frame: a drawing or laser page shows the vector tool rail (Text included) and the tabs File, Document, Style, Layers, History, Fonts, Snapping, Grid, Share; a sketch page shows the sketch rail and the tabs of the Sketch workbench of `0066` as built (Sketches, Parameters included). Activating a page is not a history step.
9. Given an open tab that the newly active page's kind does not have (for example Sketches when a drawing page becomes active), then the panel shows the tab last used on a page of that kind, else Document (the `0093` rule).

### The laser page is a normal page

10. Given a laser page, then it is listed in the page list in its place, drawn in the plane with its frame, page label and origin marker, and reached like any page; the page list can reorder, rename and select it as any row (`0059` criteria 2, 5).
11. Given a laser page is active, when the maker draws a rectangle with the Rectangle tool and a text with the Text tool on it, then both are created, selected, moved, styled and undone exactly as on a drawing page; they belong to the laser page by position (`0059` criterion 9, Frage 155 A) and, moved off it, to wherever they now lie. Nothing on a laser page runs on a machine in this slice (no operations exist yet).
12. Given a laser page from an older file that carries a placement of a drawing page (`0093`), then the placed page is drawn live on it as today, and its inside stays read-only (`0093` criterion 7 notice); the placement can be selected and deleted (delete removes only the reference; undo restores it), not yet moved (`0100`).
13. Given a laser page is active, then its Document tab shows, in this order: the page list; the document **Unit** (document-wide, as on any page); a **Machine** block (the block of `0026`, moved from the Laser tab) whose work area is read-only text "Work area 400 × 300 mm" with no width or height fields and no format presets; a **Fixtures** block with the engraved grid of `0093` (add, remove, eye); and the document **Background**, last (document-wide, Proposal). There is no Laser tab any more.
14. Given a laser page and another profile with a 600 x 400 mm work area, when the maker picks that profile, then the page takes 600 x 400 mm, the content on it keeps its distance to the origin corner of the newly picked profile (bottom-left by default; Proposal, `0026` criterion 5 kept the top-left corner), that corner keeps its place in the plane and the frame grows away from it, and all of it is one undo step.
15. Given a user profile in the profile list, then its row has a Ruler action "Edit work area of <name>" that opens width and height fields under the row (Proposal: the only place a work area is typed; the range and clamping of `0026` criterion 1). Generic laser has no such action (hidden, not disabled). The edit writes the library record, not the project, and is no history step; every laser page using that profile then shows "Differs from profile", and **Reset from profile** gives the page the new size as in criterion 14 (one undo step). Generic laser is fixed; Duplicate makes an editable copy (`0026` criterion 2).
16. Given a laser page, when the maker deletes it with the page row's trash button (`0059` criterion 5), then the page, its machine copy, fixtures and placements go, objects made on it stay where they lay on the pasteboard (as `0059` criterion 6), the drawing pages are untouched, and one undo restores all of it. Any page may be deleted while one other page remains (`0059` keeps the last one).
17. Given a project with a drawing page and a laser page with a rectangle on it and an engraved grid, when the maker exports SVG (`0024` slice 4) or prints, then the laser page is a page of the export like any other: the rectangle is in it; the fixture, the origin marker and the work-area text are not. A page placed on the laser page (older file, criterion 12) is drawn in the export and print as on the canvas (Proposal; WB-PAGE-VIS A, which skipped output pages, no longer applies). **Lead, 2026-11-05 (Proposal):** the placed-page part of this criterion moves to `0100` `bring-content-in`, where placed pages are created; in 0098 they only come from older files.

### Extensions, files, budgets

18. Given the page kinds come from the workbench registry of `0065`, then (a) a test registers a dummy second output kind through the same entry point and it appears in the Kind switch with its glyph and word and its pages follow criteria 6 to 9, with no `if laser` branch in the shell (as `0093` criterion 11a); (b) with the Laser workbench switched off in the extension list, Laser is absent from the Kind switch, existing laser pages stay listed and drawn with their content, their chip reads "Laser (off)", their Document tab shows "Laser workbench is switched off" instead of the Machine and Fixtures blocks, and the file is saved back with their data unchanged; (c) a page of a kind this build does not know is listed as "Unknown kind" with a neutral glyph, drawn with its content, and saved back unchanged (R-MFG-010), golden `page_kind_unknown_v37.curvyo` (a page of kind "template"); (d) on a "Laser (off)" page and on an "Unknown kind" page the maker gets the vector tool rail and the drawing tabs, can rename, reorder and delete the page, cannot resize it, and no manufacturing data is ever written (Proposal).
19. Given a file of format 36, when it is opened, then every page is a drawing page, except that (a) the output page of each laser manufacturing (`0093`) becomes a laser page placed at the Add page position of criterion 2 (right of the last page, never overlapping), with its machine copy, fixtures and placements; (b) a drawing page that owns a sketch (`0066`) keeps its other content, and the sketch moves to a new sketch page of the same size named "<page name> sketch" placed at the Add page position, with a linked clone of the sketch (own placement on) at its old place, so the drawing looks the same; a drawing page whose only content was its sketch stays as an empty drawing page (Proposal; old files need not load, `0059` criterion 1, but this keeps the customer's files usable). The conversion is part of reading (ADR 0042 §9, label `migrate_format`): it adds no undo step, does not mark the project changed, shows no message, and is written at the next save. Golden `page_kinds_migration_v36.curvyo`; `laser_workbench_v36` and `machine_profile_v36` also open through it.
20. Given a project with two drawing pages, one sketch page with three constrained lines, and two laser pages with different profiles (one with an engraved grid, hidden), when it is saved and reopened, then every page's kind, name, list order, size, position in the plane and content, the profiles, the fixtures (hidden state included) and the sketch are identical, and the file passes the golden-file test `page_kinds_v37.curvyo`. A page whose kind register is damaged (not a text) reads as a drawing page, is shown, and is not rewritten on read (ADR 0033 §3 rule); golden `page_kind_damaged_v37.curvyo`.
21. Given a project with 20 pages of all three kinds and 1,000 objects, when the maker activates pages of alternating kinds 50 times, then each activation takes within 50 ms of thread CPU time in `curvyo-editor-wasm` (budget confirmed by the architect, adrs.md decision 1; test under `curvyo-editor-wasm/tests/cpu_time/`).
22. Given the whole slice, then there is no network access, no G-code, no device I/O and no telemetry (`R-SYS-002`, `R-SYS-007`).

## Changes to accepted specs

This slice changes accepted criteria; the old text is superseded by the criterion named here.

| Spec | Criterion | Was | Now |
|---|---|---|---|
| `0093` | 1, 2 | Laser offered through "Add manufacturing" in the workbench switcher | Add page with Kind Laser (criteria 1, 4); no switcher (7) |
| `0093` | 3 | work area edited in the Laser tab, output page follows | work area read-only on the page (13); typed only in a user profile (15) |
| `0093` | 4, WB-PAGE-VIS A | Laser workbench shows only the output page, no drawing tools; output page not in the page list or canvas | laser page listed, in the plane, all vector tools (8, 10, 11) |
| `0093` | 5, WB-PLACE-DEFAULT A | Add places the active drawing page on the output page | a new laser page is empty (4) |
| `0093` | 7 | the whole output page read-only | only the inside of a placed page read-only (11, 12) |
| `0093` | 11 b, 13 | switcher entries disappear when the extension is off; delete manufacturing by its own trash button | Kind switch and page chip (18 b); Delete page (16) |
| `0026` | 1 | work area width and height fields in the Machine block of the Laser tab | Machine block in the laser page's Document tab, work area read-only text (13) |
| `0026` | 5 b | content keeps its place relative to the top-left corner | relative to the origin corner (14) |
| `0026` | 7 | a work-area edit in the Machine block moves the output page | no work-area edit on the page (13, 15) |
| `0026` | 10 | Add manufacturing: Laser starts with the last picked profile | Add page with Kind Laser, same rule (1, 4) |
| `0066` | 1, 18 | the workbench selector above Properties (Vector / Sketch) | no selector; a sketch page shows the sketch tools (7, 8) |
| `0066` | 19 | every page has an owning sketch edited in the Sketch workbench | only sketch pages hold a sketch (3); older files convert (19 b). The other rules of 19 stay; `0099` does the rest |
| `0059` | 2, 7 | page row and page label show name and size | also the kind (6) |
| `0059` | 3 | Add page adds a page of the last page's size at once (one press) | Add page opens the Kind group, Add confirms (two presses, Proposal); size from the last page of the same kind (2, 3) |
| `0059` | 4 | Width and Height change the active page | only for drawing and sketch pages (13) |

## Out of scope (later slices of `0063`)

- `0099` sketch pages: a sketch page holding nothing but its sketch, the sketch fixed to its page, dragging it off leaving a linked clone, moving the page moving the sketch, what a drawing object over a sketch page is.
- `0100` bring content in: drag onto a laser page making a linked clone (Alt moves, Ctrl copies), page rows dragged in as placed pages, moving and rotating placements, group clones, the edit-context rule (edit tools show nothing outside the active page's context), double-click to activate a context, Paste as linked clone, the output page's own Layers tree. In this slice a drag across pages is a plain move, as in `0059`.
- `0101` operations: the Operations tab, kinds, assignment, Show/Output, looks, summary, the Cuts last switch. Replaces `0027`.
- `0102` fixtures as objects (grid tapes and major lines, holders and slots, clamps, stops, Active per fixture, selectable fixtures, fixture snapping) and `0103` fixture jobs. The engraved grid stays as built in `0093`.
- `0028` material settings, `0029` preview, gate and G-code; `0064` connector and device I/O.
- Output kinds other than Laser (CNC, Plotter, Embroidery); changing a page's kind; a page kind for templates.

## UX notes

See `specs/0098-page-kinds/ux-notes.md` (ux-engineer, 2026-11-05): the kind chip, the Add page group, the page row, header and label, the laser page's Document tab, the Ruler sub-row, the "Laser (off)" and "Unknown kind" states, the converted sketch page. The lead's decisions of 2026-11-05 on its open points are in the criteria above (Proposal, vetoable at the demo).

## Links

Requirements: R-EDIT-038, R-EDIT-042, R-MFG-001, R-MFG-006, R-MFG-010, R-EXT-003 to R-EXT-005, R-SYS-002, R-SYS-007
Parent: `specs/0063-manufacturing-workbenches/` (slice 1; concept `concept-output-pages.md` §2.1, §2.2, §7, §8, §9)
Changes: `specs/0093-laser-workbench-shell/`, `specs/0026-machine-profile/`, `specs/0066-constraint-sketcher/`, `specs/0059-multiple-pages/`
Depends on: `0059`, `0066`, `0093`, `0026`, `0049` (linked clone of a sketch, criterion 19 b), `0065` registry
ADRs: `docs/adr/0042-page-kinds.md` (Proposed, `needs-customer`), `specs/0098-page-kinds/adrs.md`
UX: `specs/0098-page-kinds/ux-notes.md`
Format: `format_version` 37 (next free at merge; main is at 36). Goldens in `curvyo-document-core/tests/fixtures/`: `page_kinds_v37.curvyo`, `page_kind_damaged_v37.curvyo`, `page_kind_unknown_v37.curvyo`, `page_kinds_migration_v36.curvyo`
PR: -
