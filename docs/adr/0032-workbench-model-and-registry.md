# ADR 0032: Workbench model and registry

**Status:** Accepted (customer, 2026-11-02: Frage 118 MFG-MODEL A; spec `0063` WB-NAME, WB-MULTI, WB-PAGE-VIS, WB-FIXTURE-HOME, WB-PLACE-DEFAULT all A). `needs-customer` (document model: the manufacturing record, the
output page and placements). Extends ADR 0024 (pages), ADR 0015/0016 (clone resolution) and
ADR 0026 (Sketch workbench state). Specs: `0063` (epic), `0093` (first slice), `0065`.
Persistence details: [ADR 0033](0033-manufacturing-data-in-the-project-file.md). Extension
contract: [ADR 0034](0034-extension-contract-compiled-in.md).
Note 2026-11-05: §1 to §5 and §9 amended by [ADR 0042](0042-page-kinds.md) (proposed; customer feedback of 2026-11-05 reversed WB-PAGE-VIS A and WB-PLACE-DEFAULT A): the output page is listed and in the plane, the workbench follows the active page, no switcher, no Add manufacturing.

## Context

Two workbenches exist today: Vector (the drawing) and Sketch (ADR 0026). Both are a session enum
`Workbench { Vector, Sketch }` in `curvyo-editor-wasm`, and the front end branches on it with
ternaries (`App.tsx`, `PropertiesPanel.tsx`, `Canvas.tsx`). ADR 0026 refused a framework while
Sketch was the only addition. The customer now asks for a Laser workbench as the first extension
(2026-11-02): "the laser output is for now also just a page, created by the laser cutter
manufacturing workbench", with fixtures installed on the machine. A third workbench, delivered by
an extension crate the shell must not name in its branches (`0093` criterion 11), is the second
implementation that `CLAUDE.md` §5 asks for before a registry.

Three questions: what a workbench is in code; where a manufacturing lives (document node or a
record beside the document); how the output page shows the drawing live.

### Options: where the manufacturing lives

- **A. A top-level node in the shared object tree (chosen).** Undo (ADR 0014), merge (ADR 0009),
  sync (ADR 0004) and copy work unchanged; placements and fixtures are its children.
- **B. A separate container member (`manufacturing/<id>.json`).** Keeps the CRDT untouched, but
  gets no history, no merge and no live collaboration, and a placement would hold node and page
  ids across two sources of truth. Rejected.
- **C. Registers in the root map only (as pages, ADR 0024 §3).** Fine for flat settings, but
  placements and fixtures are ordered, individually undoable objects. Rejected for the content; the
  page itself stays in the root map (below).

### Options: the output page and the live link

- **A. An ADR 0024 page owned by the manufacturing, content by ownership (chosen).** Reuses the
  page frame, its size registers, rulers, the per-page grid and the canvas. Content is the
  manufacturing's subtree in page-local coordinates, not spatial membership.
- **B. A plain spatial page in the drawing plane.** Drawing objects dragged over it would become its
  content; the drawing's page list would carry machine beds. Rejected (spec `0063` Frage 3).
- **C. A new canvas kind with its own frame model.** Duplicates pages. Rejected.

For the link: a **placement** node referencing a drawing page with a matrix, resolved on read like
a linked clone (chosen); copying the page's objects (rejected: the customer requires live updates);
a clone per drawing object (rejected: a new object on the page would need a write in every
manufacturing).

## Decision

1. **A workbench is a view mode of the session, never document state.** The session holds one
   `WorkbenchId`: `Vector`, `Sketch`, or `Manufacturing(NodeId)`. A manufacturing's process
   (ADR 0033 §1) names the extension whose workbench contribution supplies its tabs and toolbox
   (ADR 0034). Switching writes nothing to the document (as ADR 0026). The session's per-workbench
   behaviour stays a `match` with three arms; **no Rust `Workbench` trait now** (the Manufacturing
   arm delegates to the extension dispatch of ADR 0034 §3, so a further process adds no arm).
2. **Registry, Rust side.** `curvyo-editor-wasm` exposes one list: the built-in Vector and Sketch
   entries, one entry per manufacturing in the document (name, process, enabled or disabled), and
   the "Add manufacturing" choices taken from the enabled extensions' manifests. Properties tabs
   per workbench: Vector and Sketch keep today's `PanelTab` sets; a manufacturing shows the tabs its
   manifest declares. File and History are project-wide and appear in every workbench (Proposal,
   UX notes decide).
3. **Registry, front end.** `frontend/src/workbenches/registry.ts` maps a workbench kind to a
   `WorkbenchEntry` (tool rail, Properties tab bodies, canvas overlays). Three implementations:
   Vector and Sketch (today's components moved behind the interface, the ternaries go) and one
   generic `ManifestWorkbench` that draws any extension workbench from its declared contributions
   (ADR 0034 §4). The Laser workbench has no laser-specific TSX. A test registers a dummy second
   manifest and finds it in the switcher (`0093` criterion 11).
4. **The manufacturing is a top-level node** with the marker `manufacturing` (like `layer`), its
   own `name`, and the registers of ADR 0033. Its children are placements and fixtures only. It is
   excluded from the Vector workbench: not in Layers, not hit, not drawn, not in drawing exports.
5. **The output page is an ADR 0024 page owned by the manufacturing.** The page record gains one
   register, `page.<id>.owner` = the manufacturing's `NodeId`. An owned page is not in the drawing
   page list, not on the Vector canvas (spec `0063` WB-PAGE-VIS, decided A) and has no spatial
   membership: its content is exactly its manufacturing's subtree. **Its size is the machine's work
   area** (one source, no copy): editing the work area resizes the page. Content coordinates are
   **page-local** (mm, origin at the page's top-left corner, the machine origin corner later from
   `0026`), so the page's position in the plane never matters. Exception to ADR 0024 §1 ("pages
   never own objects"), limited to owned pages.
6. **Placement = a live reference to a drawing page.** A placement node (`shape = "placement"`)
   carries one atomic register `placement_link = { page, matrix }` (written whole, as
   `clone_link`, ADR 0015 §3). It draws the objects that are members of that page under ADR 0024
   §1 (resolved bounds overlap the frame), each in full, unclipped, through the matrix composed with
   the source page's origin offset. Resolution runs in the existing one-read pass after clones
   (ADR 0015 §6): one composed matrix per placement, the resolved outlines reused, never a copy.
   Guides (ADR 0019) and hidden objects (ADR 0015 §2) are not drawn; output flags (R-MFG-005)
   come later. **No cycle by construction:** a placement points at a page, owned pages are not
   valid targets, and manufacturing subtrees are never members of a drawing page.
7. **Missing page.** A placement whose page is deleted draws nothing and is kept (no orphan
   repair, unlike ADR 0015 §5): undo of the page deletion restores the view; the workbench shows a
   notice. A matrix that is not finite is damaged (ADR 0015 §4 rule).
8. **Fixtures are part of the machine setup, drawn on the output page.** A fixture node
   (`shape = "fixture"`) is a child of the manufacturing with a kind owned by the extension and
   its parameter values (ADR 0033 §3). Its outlines are computed by the extension from the
   parameters and the work area (laser shell: Engraved grid), drawn below placements, never
   output, never hit or selected, hidden with the ordinary `visible` register (saved, as `0093`
   criterion 12 asks). When `0026` brings reusable machine profiles, installed fixtures move with
   the setup into the profile and the manufacturing keeps a snapshot (spec `0063` WB-FIXTURE-HOME, decided
   A); that move is that story's ADR.
9. **Placed content is read-only in a manufacturing workbench.** Editing the drawing means
   switching to Vector (spec `0063` criterion 4). Moving a placement is stage 3 and writes only
   `placement_link`.

## Consequences

- Undo, merge, sync, copy and history work for manufacturings with no new mechanism; the cost is
  that every consumer that walks the tree (Layers, hit test, exports, page membership, "Select
  all", repair) must skip manufacturing subtrees. That skip is one predicate, tested per consumer.
- An object overlapping two drawing pages appears in both placements: a direct consequence of
  ADR 0024's overlap rule, visible and accepted.
- A deleted drawing page leaves an empty placement rather than a silent deletion; the maker sees
  it and can undo.
- The existing Sketch and Vector branches in the front end move behind the registry in the first
  slice: a refactor without behaviour change, paid once.
- Not decided here: placement UI, nesting, simulation, output flags, machine profiles (`0026`),
  roles (`0027`). A Rust `Workbench` trait is extracted only when a workbench needs behaviour the
  manifest cannot declare (first canvas tool of an extension) and two such workbenches exist.
