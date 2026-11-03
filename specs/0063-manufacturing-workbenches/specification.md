# Manufacturing workbenches (epic): a view per machine process, with work area, fixtures, placed pages and a simulation

Status: Draft (an epic sketch with a stage plan, reworked 2026-11-02 after the customer made the laser workbench the first extension; not Ready: ADRs are being written, stories are split off, the first is `0093`, Done (#255))
Priority: Should (the laser part is the Must MVP, slices 0026 to 0029; the rest follows)
Origin: Customer (request of 2026-10-17, translated): "Start with light refinement; not an easy feature. Workflow: you decide to add a manufacturing (or whatever we call it) for a laser cutter. The machine is specific: defined work area, controls and possibilities. In the work area there can be fixtures to lay out or clamp the parts, plus maybe a measuring tape. There is a fixture-preparing step to engrave or cut out tapes, layout outlines, insertion pockets, etc. Then you position the pages of the documents on the work area; they snap in by fitting size, can be dragged there, etc. You can still scale and rotate there, without changing the original drawing. Changes in the original drawing carry over live into the manufacturing. CNC milling is very similar, but e.g. an embroidery machine is entirely different: strokes may have to become satin stitches and a lot must be set. A simulation is needed to look at what a CNC mill, laser cutter or embroidery machine will do. For that we need different views or 'workbenches'. A Curvyo project can be used for several manufacturing processes, not rarely CNC milling and engraving together, or vinyl cutting and embroidery. In Affinity you can switch between vector, pixel and layout. If a switcher is needed, it sits above the property panel on the right and is always visible."

## User value

As a maker I want to prepare one drawing for several machines and to see, for each, the machine's bed, my parts placed on it, the fixtures that hold them, and a simulation of what the machine will do, without that preparation changing my drawing, so that I design once and produce on a laser, a mill or an embroidery machine from the same project.

**What other tools do.** LightBurn: one window, a work area for the selected machine, a Preview window that simulates the job; layers carry the cut settings; no separate drawing view. Affinity: personas (Vector, Pixel, Layout) switched in the toolbar; each persona has its own tools and panels over the same document. Fusion 360: workspaces (Design, Manufacture, Simulation) over one model; toolpaths live in Manufacture and follow the design. Ink/Stitch: converts in place and simulates in a window. The customer wants the Affinity way (a switcher) with the Fusion idea (a live link from the design).

## Words used below (names decided, WB-NAME A)

- **Drawing workbench:** what exists today. Pages (`0059`), objects, styles, layers.
- **Manufacturing:** a named record in the project for one process on one machine: machine profile, work area, fixtures, **placements** of pages, per-process settings, and the output flags (R-MFG-005). A project can hold several (CNC milling and engraving; vinyl and embroidery).
- **Workbench:** the view and tool set for a manufacturing of one process family (laser, CNC, vinyl, embroidery), with its own toolbox cards, Properties tabs and simulation.
- **Placement:** a page (or a part of one) put on the work area with a matrix (move, rotate, scale). It is a live reference to the page, like a linked clone (`0049`), never a copy.
- **Fixture:** a shape on the work area that lays out or clamps parts (outlines, pockets, tapes, a measuring tape). A fixture is made by a **fixture job**: its own manufacturing step (cut or engrave the outlines, tapes and pockets first, then produce the parts into them).

- **Output page:** the page a manufacturing creates in the project. It is sized to the machine's work area, belongs to the manufacturing (not to the drawing), and its content is the manufacturing's placements of drawing pages (live, never copies). The laser output is "just a page" (customer, 2026-11-02): later stages (roles, preview, G-code) read this page. The slice story is `0093`.
- **Machine setup:** the machine profile plus the **fixtures installed** on that machine (an engraved grid in the bed, holders for specific materials). Fixtures belong to the setup, not to the drawing.

## Stage plan (reworked 2026-11-02: laser is the first extension)

The Laser workbench is the first user of the extension contract (`0065`): it is registered through the front-end registry like any later extension, but compiled in (route A). Stage 0 and 1 of the old plan are merged into the first slice, which is small and demo-able end to end.

| Stage | Delivers | Needs first |
|---|---|---|
| 0 | ADRs (workbench model, output page and live link, per-process data in the file, extension registry), written by the architect now | the customer input of 2026-11-02 |
| 1 | **`0093` Laser workbench shell** (first slice, one PR): Laser is a switcher entry registered through the extension contract; Add manufacturing creates a laser manufacturing with a placeholder machine profile (work area w x h) and an **output page** of that size; the Laser workbench shows the output page with the active drawing page placed on it (live); the installed fixtures are drawn on it (an engraved grid at least); the file round-trips | stage 0; `0059` pages, `0049` clones |
| 2 | **Machine setup:** `0026` machine profile as a reusable record (work area, limits), fixtures as setup data (engraved grid editor, material holders as outlines), the choice of setup per manufacturing | stage 1 |
| 3 | **Placement:** several pages placed on the output page, dragged, snapped by fitting size (`0060`), scaled and rotated in the manufacturing only; R-MFG-005 output flags; nesting later (R-MFG-004) | stage 1 |
| 4 | **Laser job content:** `0027` roles (cut, engrave) read from the output page, `0028` material test library, `0029` preview, gate, GRBL G-code | stage 2 |
| 5 | **Connector protocol:** the message types between workbench and connector (`0064`, in a core crate), then device I/O (GRBL over serial, frame, run) | stage 4 for the G-code job; the types can start with stage 2 |
| 6 | **Fixture jobs:** engrave or cut outlines, tapes and pockets as a manufacturing step of their own, a measuring tape tool | stage 3 |
| 7 | **CNC workbench** (2.5D, V-carve): the second machine family; only here is a shared machine abstraction allowed to appear (`CLAUDE.md` §6) | stage 4; second-family ADR |
| 8 | **Embroidery and vinyl workbenches:** stroke to satin, stitch parameters, stitch simulation; registration marks | stage 7; a family ADR each |
| 9 | Workbenches contributed by third parties (`0065`) | the extension contract proven by Laser |

Every stage is a slice the maker can try end to end (one PR per slice). The customer's goal after stage 1 is to engrave with Curvyo: stages 2, 4 and 5 are the road to it, in that order unless the customer says otherwise.

## Acceptance criteria (as far as clear; the stages carry the rest)

1. Given the Properties panel, then the **workbench switcher** is a row of symbol buttons (tooltips, accessible names) at its top, above the tabs and always visible, whatever tab is open; the active workbench is pressed. It never scrolls away and never grows a section for an operation.
2. Given a project with no manufacturing, then the switcher shows only Drawing and an **Add manufacturing** symbol button (permit; nothing else appears out of nowhere). Add manufacturing asks for the process by an inline group in the panel, not a popup, then shows the new workbench.
3. Given a project with several manufacturings, then each is a switcher entry with its process symbol and a name; the Drawing workbench is always first. Deleting a manufacturing never touches the drawing; undo restores it.
4. Given the Laser workbench, then the canvas area shows the output page (sized to the work area of the machine profile, `0026`) with the installed fixtures, the placed pages and the simulation; the toolbox shows the laser cards only; the drawing tools are not offered there, and editing the drawing means switching back (the drawing is not editable inside a manufacturing).
5. Given a change in the drawing (an edit, a clone updating, an import), then every manufacturing that places that page shows it at once, live, without a refresh button. A placement's own matrix is kept.
6. Given a placement, then scale and rotate change only the placement; the page and its objects stay as drawn (criterion 5 of the customer text).
7. Given "visible" and "output" (R-MFG-005), then the drawing's hidden and locked flags (`0050`) are inputs and the manufacturing has its own output switch per layer or object; the default is permit: whatever is visible is output.
8. Given the simulation, then each workbench has a view of what its machine will do (laser: toolpath playback and time; CNC: tool path and removed material; embroidery: stitches), drawn from the same display-list pipeline as the canvas, with play, pause and a position slider as symbol controls.
9. Given the project file, then per-process data is stored under the manufacturing's id and unknown processes round-trip untouched (an older build keeps the data and shows the entry disabled), so a CNC record survives a build that only knows lasers. A `format_version` bump is expected at stage 1.

## Fit with the reserved MVP slices

| Slice | In the workbench model |
|---|---|
| `0026` machine-profile | A machine profile is data reused by many manufacturings (R-MFG-001). The Laser workbench shows its work area. The profile is global (per user and synced), the choice of profile is per manufacturing. |
| `0027` manufacturing-roles | Roles by layer (R-MFG-002) are stored in the manufacturing, not on the layer, so the same layer can be Cut in the laser job and a pocket in the CNC job. For the MVP they may be stored by layer id in the one manufacturing. |
| `0028` material-test-library | A tool of the Laser workbench; its material records are global (R-MAT). |
| `0029` job preview and output | The simulation view and the gate of the Laser workbench. |
| R-MFG-005 | Lives in the manufacturing (criterion 7). `0050`'s visible and locked flags stay drawing-side. |
| `0059` pages | Placements reference pages; with one page placement is implicit. |

The MVP slices keep their numbers and scope. Their specs, when written, are written **as the Laser workbench**: the only cost today is that stage 1 comes first.

## Out of scope

- Any machine family other than the laser before stage 5; a generic "machine" trait before the second family exists.
- Nesting algorithms (R-MFG-004), a CAM kernel, tool libraries beyond the first CNC story.
- Device I/O (`0064`; its protocol types are stage 5 here), third-party workbenches (`0065`; Laser is the first, compiled in, and the model must not forbid more).
- 3D simulation, collision checking, physical machine models.

## Architect notes (2026-10-17)

Three ADRs, all `needs-customer`: (1) the **workbench model** (what a workbench is in code: a view over the document plus a toolbox, tabs and a simulation; how the canvas area switches; whether the manufacturing is a document node or a separate record); (2) the **live link** (the placement as a reference with a matrix, reusing the clone mechanism of ADR 0015 and 0016, resolved on read; cycles impossible because a placement points at pages only); (3) **per-process data in the file** (the container section for manufacturings, round-trip of unknown processes, its relation to the sync of ADR 0004 and 0009). The first two shape the document model, the third the file format.

## Decided (customer, 2026-10-17)

- **WB-ORDER: A.** A small workbench shell (switcher with Drawing and Laser, one manufacturing record) is built before `0026` to `0029`; the laser workbench is the MVP.

## Decided (customer, 2026-11-02, translated from German)

Input: "I want you to tackle 0065 + 0063 and create a laser cutter workbench as the first extension. The laser output is for now also just a page, created by the laser cutter manufacturing workbench. The laser cutter can have different fixtures installed, e.g. an engraved grid or holders for specific materials. 0064 also becomes relevant, at least the protocol between connector and workbench. That will be your next big task: I want to laser engrave with Curvyo."

- **WB-LASER-FIRST:** the laser workbench is the first extension of `0065` (compiled in, registered through the front-end registry).
- **WB-OUTPUT-PAGE:** the output of a manufacturing is a page in the project, created by the workbench when the manufacturing is added; its content is a live placement of drawing pages. For now this is the whole output; no separate job object.
- **WB-FIXTURES:** fixtures are part of the machine setup (per machine, not per drawing): an engraved grid in the bed, holders for specific materials. They are shown on the output page.
- **WB-CONNECTOR:** `0064` is relevant next, at least the protocol between connector and workbench (message types in a core crate); device I/O follows.
- **Priority:** this is the next big task (goal: engrave with Curvyo). The stage plan above is reordered accordingly.

## Decided (customer, 2026-11-02, answers to the open questions and the ADRs)

- **MFG-MODEL (Frage 118): A.** A manufacturing is a top-level node in the shared object tree; its output page is a page it owns; placements are live page references (ADR 0032, ADR 0033, both Accepted).
- **WB-NAME: A.** **Workbench** for the view, **Manufacturing** for the record.
- **WB-MULTI: A.** The file format and the switcher allow several manufacturings from the start; the stage 1 UI may show one.
- **WB-PAGE-VIS: A.** The output page appears only in its manufacturing's workbench; the drawing's page list, canvas, Export and Print skip it.
- **WB-FIXTURE-HOME: A.** Fixtures live in the machine setup, for stage 1 inside the manufacturing record; `0026` moves the setup to a reusable per-user record. Fixtures never enter the drawing and are not output.
- **WB-PLACE-DEFAULT: A.** Add manufacturing places the active drawing page at the work area's top-left corner, scale 1, no rotation (Proposal for the exact placement).
- **EXT-LASER-UI (Frage 119): A.** The laser workbench's UI is declarative (schema, blocks, display list) drawn by the shell; no extension TSX (ADR 0034, Accepted).
- **Connector (Frage 120, 121):** see `specs/0064-machine-connector/` (TLS with key pinning or an EST certificate; browser through the relay and through the web app served by the connector).

## Links

Requirements: R-MFG-001 to R-MFG-005 (existing), R-MFG-006 to R-MFG-010 (new)
Depends on: `specs/0059-multiple-pages/`, `specs/0049-linked-clone/`, `specs/0050-layers-and-objects-panel/`, `specs/0051-sidebar-and-no-menu/`
First slice: `specs/0093-laser-workbench-shell/`
Related: `specs/0026` to `0029` (reserved MVP slices, no folders), `specs/0060-snapping/` (fitting size), `specs/0062-fonts/` (pane host), `specs/0064-machine-connector/`, `specs/0065-extensions-sdk/`, `specs/0046-document-shapes/` (blanks and hoops); ADR 0004, ADR 0009
ADRs: 0032, 0033, 0034, 0035 (accepted by the customer 2026-11-02)
PR: -
