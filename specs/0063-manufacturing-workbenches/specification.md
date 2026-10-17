# Manufacturing workbenches (epic): a view per machine process, with work area, fixtures, placed pages and a simulation

Status: Draft (an epic sketch with a stage plan; not Ready: it needs three customer-level ADRs and it is split into stories later; shell-first order decided 2026-10-17)
Priority: Should (the laser part is the Must MVP, slices 0026 to 0029; the rest follows)
Origin: Customer (request of 2026-10-17, translated): "Start with light refinement; not an easy feature. Workflow: you decide to add a manufacturing (or whatever we call it) for a laser cutter. The machine is specific: defined work area, controls and possibilities. In the work area there can be fixtures to lay out or clamp the parts, plus maybe a measuring tape. There is a fixture-preparing step to engrave or cut out tapes, layout outlines, insertion pockets, etc. Then you position the pages of the documents on the work area; they snap in by fitting size, can be dragged there, etc. You can still scale and rotate there, without changing the original drawing. Changes in the original drawing carry over live into the manufacturing. CNC milling is very similar, but e.g. an embroidery machine is entirely different: strokes may have to become satin stitches and a lot must be set. A simulation is needed to look at what a CNC mill, laser cutter or embroidery machine will do. For that we need different views or 'workbenches'. A Curvyo project can be used for several manufacturing processes, not rarely CNC milling and engraving together, or vinyl cutting and embroidery. In Affinity you can switch between vector, pixel and layout. If a switcher is needed, it sits above the property panel on the right and is always visible."

## User value

As a maker I want to prepare one drawing for several machines and to see, for each, the machine's bed, my parts placed on it, the fixtures that hold them, and a simulation of what the machine will do, without that preparation changing my drawing, so that I design once and produce on a laser, a mill or an embroidery machine from the same project.

**What other tools do.** LightBurn: one window, a work area for the selected machine, a Preview window that simulates the job; layers carry the cut settings; no separate drawing view. Affinity: personas (Vector, Pixel, Layout) switched in the toolbar; each persona has its own tools and panels over the same document. Fusion 360: workspaces (Design, Manufacture, Simulation) over one model; toolpaths live in Manufacture and follow the design. Ink/Stitch: converts in place and simulates in a window. The customer wants the Affinity way (a switcher) with the Fusion idea (a live link from the design).

## Words used below (names are open, Question 1)

- **Drawing workbench:** what exists today. Pages (`0059`), objects, styles, layers.
- **Manufacturing:** a named record in the project for one process on one machine: machine profile, work area, fixtures, **placements** of pages, per-process settings, and the output flags (R-MFG-005). A project can hold several (CNC milling and engraving; vinyl and embroidery).
- **Workbench:** the view and tool set for a manufacturing of one process family (laser, CNC, vinyl, embroidery), with its own toolbox cards, Properties tabs and simulation.
- **Placement:** a page (or a part of one) put on the work area with a matrix (move, rotate, scale). It is a live reference to the page, like a linked clone (`0049`), never a copy.
- **Fixture:** a shape on the work area that lays out or clamps parts (outlines, pockets, tapes, a measuring tape). A fixture is made by a **fixture job**: its own manufacturing step (cut or engrave the outlines, tapes and pockets first, then produce the parts into them).

## Stage plan

| Stage | Delivers | Needs first |
|---|---|---|
| 0 | Three ADRs (workbench model, live link from drawing to manufacturing, per-process data in the project file), `needs-customer` | this sketch answered |
| 1 | **Workbench shell:** the switcher above the Properties panel (Drawing, Laser), the canvas area able to show another view, a Manufacturing record in the file with a placeholder machine; no placement yet | stage 0; the pane host also serves the font preview (`0062`) and supports a split view and a full view of the canvas area |
| 2 | **Laser workbench = the MVP slices:** `0026` machine profile (the work area shown in the laser view), `0027` roles per manufacturing (cut, engrave), `0028` material test library, `0029` preview, gate, GRBL G-code. With one page the placement is implicit: page one at the origin, no UI for it | stage 1 |
| 3 | **Placement:** pages placed on the work area, dragged, snapped by fitting size (`0060`), scaled and rotated in the manufacturing only; live updates from the drawing; R-MFG-005 output flags; nesting later (R-MFG-004) | `0059`, `0049`, stage 2 |
| 4 | **Fixtures:** shapes on the work area, a fixture job (engrave or cut outlines, tapes, pockets), a measuring tape tool | stage 3 |
| 5 | **CNC workbench** (2.5D, V-carve): the second machine family; only here is a shared machine abstraction allowed to appear (`CLAUDE.md` §6) | stage 2; second-family ADR |
| 6 | **Embroidery and vinyl workbenches:** stroke to satin, stitch parameters, stitch simulation; registration marks | stage 5; a family ADR each |
| 7 | Workbenches contributed by extensions (`0065`) | the extension contract |

Every stage is a slice the maker can try end to end (one PR per slice); stages 0 and 1 together are the first, because a switcher with nothing behind it shows nothing.

## Acceptance criteria (as far as clear; the stages carry the rest)

1. Given the Properties panel, then the **workbench switcher** is a row of symbol buttons (tooltips, accessible names) at its top, above the tabs and always visible, whatever tab is open; the active workbench is pressed. It never scrolls away and never grows a section for an operation.
2. Given a project with no manufacturing, then the switcher shows only Drawing and an **Add manufacturing** symbol button (permit; nothing else appears out of nowhere). Add manufacturing asks for the process by an inline group in the panel, not a popup, then shows the new workbench.
3. Given a project with several manufacturings, then each is a switcher entry with its process symbol and a name; the Drawing workbench is always first. Deleting a manufacturing never touches the drawing; undo restores it.
4. Given the Laser workbench, then the canvas area shows the work area of the machine profile (`0026`), the placed pages and the simulation; the toolbox shows the laser cards only; the drawing tools are not offered there, and editing the drawing means switching back (the drawing is not editable inside a manufacturing).
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
- Device communication (`0064`), extension-provided workbenches (`0065`; the model must not forbid them).
- 3D simulation, collision checking, physical machine models.

## Architect notes (2026-10-17)

Three ADRs, all `needs-customer`: (1) the **workbench model** (what a workbench is in code: a view over the document plus a toolbox, tabs and a simulation; how the canvas area switches; whether the manufacturing is a document node or a separate record); (2) the **live link** (the placement as a reference with a matrix, reusing the clone mechanism of ADR 0015 and 0016, resolved on read; cycles impossible because a placement points at pages only); (3) **per-process data in the file** (the container section for manufacturings, round-trip of unknown processes, its relation to the sync of ADR 0004 and 0009). The first two shape the document model, the third the file format.

## Decided (customer, 2026-10-17)

- **WB-ORDER: A.** A small workbench shell (switcher with Drawing and Laser, one manufacturing record) is built before `0026` to `0029`; the laser workbench is the MVP.

## Open questions (customer; each has a default)

Frage 1 (WB-NAME): What do we call it?
- A (default): **Workbench** for the view and **Manufacturing** for the record, as in this sketch.
- B: **Mode** (view) and **Job setup** (record).
- C: **Studio** per process (Laser studio, Embroidery studio).
- Default if unanswered: A. UI strings only; no model depends on it.

Frage 2 (WB-MULTI): Does a project hold several manufacturings from the first version?
- A (default, recommended): the file format and the switcher allow many from the start; the UI of stage 1 may show one.
- B: one manufacturing per project for now.
- Default if unanswered: A, because the customer said "not rarely" two processes.

## Links

Requirements: R-MFG-001 to R-MFG-005 (existing), R-MFG-006 to R-MFG-010 (new)
Depends on: `specs/0059-multiple-pages/`, `specs/0049-linked-clone/`, `specs/0050-layers-and-objects-panel/`, `specs/0051-sidebar-and-no-menu/`
Related: `specs/0026` to `0029` (reserved MVP slices, no folders), `specs/0060-snapping/` (fitting size), `specs/0062-fonts/` (pane host), `specs/0064-machine-connector/`, `specs/0065-extensions-sdk/`, `specs/0046-document-shapes/` (blanks and hoops); ADR 0004, ADR 0009
ADRs: three new ADRs, `needs-customer`, written at stage 0
PR: -
