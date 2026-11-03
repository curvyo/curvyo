# Laser workbench shell: add a laser manufacturing, get an output page with fixtures and your drawing placed on it

Status: Ready (ADRs 0032 to 0034 accepted and all questions answered by the customer, 2026-11-02)
Priority: Must
Origin: Customer (2026-11-02, translated): "create a laser cutter workbench as the first extension. The laser output is for now also just a page, created by the laser cutter manufacturing workbench. The laser cutter can have different fixtures installed, e.g. an engraved grid or holders for specific materials." Details marked Proposal are the product owner's and vetoable at the demo.

## User value

As a maker with a laser I want to add a laser manufacturing to my project and see a page the size of my machine's bed, with the fixtures installed on it and my drawing placed on it, so that I can see what will lie on the bed before any job exists, without my drawing changing.

**What other tools do.** LightBurn: one work area for the chosen machine, no separate drawing view; the bed grid is a display setting, fixtures do not exist. Inkscape: the page is the output, there is no machine bed. Fusion 360: a setup with stock and fixtures over one model. We take the Fusion idea (setup with fixtures, live link to the design) and keep Curvyo's page model: the machine's bed is an ordinary page that a manufacturing owns.

## Acceptance criteria

1. Given the workbench switcher in the Properties panel (Vector and Sketch exist, `0066`), when the app starts, then it also offers **Laser** only through "Add manufacturing" (nothing is added to a project without that), and a project with no manufacturing shows no Laser entry that does anything.
2. Given a project, when the maker uses Add manufacturing and picks Laser (an inline group in the panel, no popup), then a manufacturing named "Laser 1" exists with the placeholder machine profile "Generic laser", work area **400 x 300 mm** (Proposal), the Laser workbench is shown, and the whole action is one undo step that removes the manufacturing and its output page again.
3. Given the new manufacturing, then the project holds one **output page** of exactly the work area size (400 x 300 mm), owned by the manufacturing. Given a work area edited in the Laser Properties tab to 600 x 400 mm, then the output page follows at once and the placed content keeps its place relative to the top-left corner (one undo step per edit).
4. Given the Laser workbench is active, then the canvas area shows the output page, and the toolbox and Properties panel show only Laser entries (the Laser tab with machine and fixtures); the drawing tools and the drawing's pages are not offered. Switching to Vector shows the drawing as before; the output page is not in the Vector page list or on its canvas (WB-PAGE-VIS, decided A).
5. Given a drawing page P exists, when the manufacturing is added, then P is placed on the output page with its top-left corner at the output page's top-left corner, scale 1, rotation 0 (WB-PLACE-DEFAULT, decided A; the exact placement is a Proposal), and the placement is a live reference, never a copy.
6. Given the placement, when the maker edits the drawing in the Vector workbench (draw a circle, move a path, change a stroke colour, undo it) and switches to Laser, then the output page shows exactly the new drawing, and while both are visible in a split view of the canvas area (if the pane host allows it) the change shows without a refresh action. A drawing change never needs a command in the Laser workbench.
7. Given the Laser workbench, when the maker tries to select, move, delete or style a placed object on the output page, then nothing in the drawing changes (the placed content is read-only here); a notice of at most 70 characters says that editing happens in Vector. Moving the placement itself is out of this slice.
8. Given a placed page larger than the work area, then it is drawn in full across the output page edge (permit) and the Laser tab shows a notice "Extends past the work area by 40 x 12 mm" with the actual overhang; nothing is clipped, refused or moved.
9. Given the machine setup, then it lists its installed fixtures. The slice has one fixture kind, **Engraved grid**, with spacing (default 10 mm), line width 0.1 mm and an origin offset (default 0, 0), added and removed with symbol buttons in the Laser tab. A new manufacturing starts with no fixture installed; adding "Engraved grid" draws thin grid lines over the output page area, below the placed drawing, on the output page only.
10. Given a fixture, then it is a reference, not content: it is not part of the drawing, it is not output (no cut, no engrave, not in any export of the drawing), it cannot be selected or hit by marquee or Select All on the output page, and the Laser tab can hide it per fixture with an eye button (view state).
11. Given the Laser workbench is registered through the extension contract of `0065`, then (a) the shell has no `if laser` branch for the switcher entry, toolbox cards or Properties tabs (a test registers a dummy second workbench through the same entry point and it appears in the switcher), and (b) the extension list shows "Laser workbench" with its version and an enable switch; switched off, the Laser entries and "Add manufacturing: Laser" disappear, existing manufacturings are kept in the file and shown disabled with their name.
12. Given a project with a laser manufacturing, 2 fixtures (one hidden), a 600 x 400 mm work area and one placement, when it is saved and reopened, then the manufacturing, machine profile, work area, fixtures (including hidden state), output page and placement are identical, the placement still follows the drawing, and the file passes the golden-file test `laser_workbench_v<next free at merge>.curvyo`. Given a file that holds a manufacturing of an unknown process, then a build opens it, shows the entry disabled and saves it back unchanged (0063 criterion 9).
13. Given a manufacturing is deleted (a trash symbol button that deletes at once, no confirm, because undo restores it; design-system rule, lead decision 2026-11-02, Proposal), then its output page and fixtures go, the drawing and its pages are untouched, and undo restores all of it.
14. Given a project with 1,000 objects on the drawing page, when the Laser workbench is opened, then the output page is shown within 1.0 s of thread CPU time and a drawing edit is reflected on it in the next frame (Proposal for the budget; the architect confirms against `curvyo-editor-wasm/tests/cpu_time/`).
15. Given the whole slice, then there is no network access, no G-code, no device and no telemetry (`R-SYS-002`, `R-SYS-007`).

## Decided (customer, 2026-11-02)

- **MFG-MODEL (Frage 118): A**, the manufacturing node, owned output page and live placements of ADR 0032 and ADR 0033.
- **EXT-LASER-UI (Frage 119): A**, the Laser UI is declarative and drawn by the shell's `ManifestWorkbench` (ADR 0034).
- **WB-NAME A** (Workbench, Manufacturing), **WB-MULTI A** (format allows many, this slice shows one), **WB-PAGE-VIS A** (criterion 4), **WB-FIXTURE-HOME A** (criteria 9, 10), **WB-PLACE-DEFAULT A** (criterion 5); see `specs/0063-manufacturing-workbenches/`.

## Out of scope (follow-up slices, numbers on scheduling)

- Reusable machine profile record, limits, connection data, choice of profile per manufacturing: `0026`. The profile here is a placeholder with a work area only.
- More fixture kinds (material holders as outlines, tapes, pockets), editing a fixture's shape, fixture jobs, measuring tape (`0063` stages 2 and 6). Only the engraved grid exists here; it is not the view grid of `0090` and does not snap.
- Placement UI: dragging, snapping, scaling, rotating, several pages, nesting (`0063` stage 3).
- Roles cut/engrave (`0027`), power and speed, material library and test grid (`0028`), preview, gate, G-code export (`0029`).
- The connector and its protocol (`0064`; the protocol types are the slice after `0029`), device I/O, framing, running a job.
- Output flags per layer or object (R-MFG-005), simulation.
- Other machine families, third-party extensions, loading extensions at run time (`0065`).
- Editing drawing objects inside the Laser workbench.

## UX notes

(to be filled in by ux-engineer before Ready: the switcher entry and the Add manufacturing group, the Laser tab layout (machine, work area, fixtures with eye and delete), the overhang notice, the read-only hint on the canvas, split view with Vector.)

## Links

Requirements: R-MFG-001 (placeholder only), R-MFG-005 (not yet), R-EXT-003 to R-EXT-005, R-SYS-002, R-SYS-007
Depends on: `specs/0059-multiple-pages/`, `specs/0049-linked-clone/`, `specs/0066-constraint-sketcher/` (workbench switcher), `specs/0081-inline-delete-confirm/`
Parent: `specs/0063-manufacturing-workbenches/` (stage 1), `specs/0065-extensions-sdk/` (first extension)
Follow-ups: `0026`, `0027`, `0028`, `0029`, `0064` (protocol), placement and fixtures stages of `0063`
ADRs: 0032, 0033, 0034 (accepted by the customer 2026-11-02); see `adrs.md`
Format: `format_version` bump, next free at merge
PR: -
