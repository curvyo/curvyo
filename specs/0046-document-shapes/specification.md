# Document template and guide paths: mark outlines as guides, load a template from a file

Status: Stage 1 Done (PR #150, merged 2026-10-19): mark paths as guides, load outlines from another Curvyo document. Stage 2 (load from an SVG file) waits for `0024`.
Priority: Should
Origin: Customer (request of 2026-10-10: templates that are not square, key rings, labels, hoops). Redesigned by the customer's answers of 2026-10-17: no shaped page; a template is a set of outlines placed in the document and shown in a marking colour; the same thing serves as guide lines. The folder keeps its number and slug (`document-shapes`); the earlier design (ellipse and rounded-rectangle area, shape in formats) is dropped.

## User value

As a maker who cuts key rings, coasters, labels and discs, or embroiders inside a hoop, I want the outline of my blank on the canvas as paths I can see, move and edit, so that I draw inside the real material. I draw the outline with the tools I know (a circle, two circles for a donut), mark it as a guide, and it stays out of the cut. I can also load the outlines from an SVG or from another Curvyo document. Guide lines for alignment are the same thing.

**Reference tools.** Inkscape and Illustrator let a maker turn any path into a guide (Edit, Objects to Guides; View, Guides, Make Guides) and keep the page a rectangle. LightBurn shows the bed as a rectangle and has no template. Cricut and Silhouette offer mat and material templates. Here a guide is an ordinary object with a flag, so every tool works on it.

## Words used below

- **Guide:** a path, compound path, rectangle, ellipse, polygon or star that carries the flag `guide` (ADR 0019). It is drawn in the **marking style** and is not output.
- **Template:** the guides that came from a file or from another document. It has no identity of its own; after loading it is guides.
- **Marking style:** green, dashed 2 px on / 2 px off, one screen pixel wide at any zoom, no fill (token `--guide`, `#166534`: 3.6:1 on `--pasteboard-bg`, 5.8:1 on `--canvas-bg`, measured at build; the first proposal `#15803D` has only 2.5:1 on the pasteboard).
- **Document area:** the page rectangle (`0015`) plus any guides. Only "zoom to document" uses it; the page itself stays the rectangle.
- **Frame:** the page of a source: a Curvyo document's page rectangle; an SVG's `width`/`height` or `viewBox` (`0024`).

## Dependencies and stages

| Stage | Delivers | Needs first | State |
|---|---|---|---|
| 1 | The Select-bar command, the marking style, guides out of output, loading from another Curvyo document (File tab button **Template**), file format version | `0050` (layers), `0049` (clones), `0051` (File tab); all built | **Ready** |
| 2 | Loading from an **SVG** file through the same button | Stage 1 and the importer of `0024-svg-import-export` (Draft; an API that returns the file's drawing and its frame is agreed in `0024`'s plan) | Not Ready: waits for `0024` Ready |

DXF is not planned (customer, 2026-10-17: SVG first, DXF waits; no requirement exists). Criteria carry "(stage 2)" when they are not stage 1. Numbers of criteria that fell away stay as "Removed" placeholders.

## Acceptance criteria

### Part A: the guide flag

1. Given a document without guides (every file before this slice), then it opens and draws exactly as before, nothing is written by opening it, and saving it writes no `guide` key.
2. Given a node marked as guide, then the node stores the register `guide` = true (ADR 0019) on its node; a path, compound path, rectangle, ellipse, polygon or star can carry it; unmarking removes the key. `format_version` goes to the next free number at merge; an older build refuses the file as "saved by a newer version" (it would otherwise cut the template).
3. Removed (was: the document has a shape).
4. Removed (was: kinds of shape).

### Part B: how a guide is drawn and picked

5. Given a guide, then it is drawn in the marking style at its place in the tree, above the nodes below it and below the ones above, with its geometry only: its own fill and stroke are kept in the node and not drawn. Given its layer is hidden, it is not drawn (`0050`).
6. Removed (was: objects not clipped by a shaped area).
7. Given the pointer, then a guide is selected, moved, deleted, grouped, locked and hidden like any node: a click on its line selects it, the marquee and lasso select it as any object, and Select all includes it. Because it has no drawn fill, a click inside a closed guide does not select it; the line is the hit area. The Select, Node, Shape and Transform tools work on it unchanged (an ellipse guide keeps its radius handles).
8. Given a zoom from 1 % to 6400 %, then the marking style stays one screen pixel wide with dashes of 2 px on and 2 px off, measured on the screen, restarting at the start of each path; it is snapped to whole device pixels like the transform guide (`docs/design-system.md`).

### Part C: the command in the Select tool

9. Given the Select tool, then its bar has one **Guide** button: an icon button (28 × 28 px, 16 px Lucide `SquareDashed`, 1.5 px stroke, no label), always in the bar, directly after the Flip group, with the tooltip "Guide" and the accessible name "Guide". It is disabled when the selection contains no node that can be a guide (nothing selected, or only an image or text). Pressing it marks every such node of the selection (for a group: its paths and shapes) as a guide, in one commit `set_guide`, one undo step named "Guide". The selection stays.
10. Given a selection whose markable nodes are all guides, then the Guide button is pressed (`aria-pressed`); pressing it turns them back into ordinary nodes, in one commit `set_guide`, undo step "Guide". Given a mixed selection, then it is not pressed and a press marks all. A guide turned back keeps its geometry, fill and stroke exactly.
11. Removed (was: Radius field).
12. Removed (was: scaling an outline with the box).
13. Given **Fit to content** (`0015`), then it measures artwork only: guides do not count, and Fit leaves the tree when the document holds guides only (as with no objects). **Zoom to document** and the first view of a loaded template include the guides, so an overhanging template can be seen.
14. Given the Guide button and the File tab button of this slice, then they are in the panel or bar column: no popup, no Apply button, no operating hints or helper text, nothing appears out of nowhere; the tooltip is the name only (customer rules 2026-10-16 and 2026-10-17).

### Part D: formats

15. Removed (was: shape in a format).
16. Removed.
17. Removed.
18. Removed. The document size formats of `0045` are unchanged: rectangular A0 to A6 and slides.

### Part E: loading a template

19. Given the File tab (`0051`), then it has a **Template** outline text button (28 px high, in the row after Save as… and Close, as the five others) that opens the OS file dialog (the only layer, as Open). Stage 1 accepts `.curvyo`; stage 2 also `.svg`. It is enabled whenever Open is. The file is read by a platform crate; the core crates open no file. The tooltip is "Template". It never replaces the project and runs no unsaved-changes check (`0051`).
20. Given a `.curvyo` file, then every node of its tree that is a guide in that document comes in as a guide; if it has no guide, **everything** comes in: every path, compound path, rectangle, ellipse, polygon and star that is visible (`0050`), with its geometry. Nothing else comes in (no images, no text, no fills or strokes drawn, no clones: a clone comes in as its resolved geometry). The nodes are copied with their coordinates: the source's top-left is our top-left. Given the source is the open document's own file, then it loads from the saved copy.
21. (Stage 2) Given an `.svg` file, then the importer of `0024` reads its paths and shapes and the nodes come in with the SVG's frame's top-left at the document's top-left corner (0 mm, 0 mm). Given the SVG has no frame (no `viewBox`, no `width`/`height`), then the top-left of the horizontally aligned (axis-aligned) bounding box of everything is placed at the document's top-left. The scale is the SVG's own (user units at 96 per inch, ADR 0002); nothing is fitted or scaled, an overhang is allowed.
22. Given any source, then **everything goes in**: any number of outlines (four lines, odd shapes, two circles for a donut), open or closed, nested or not, in any size; there is no check of closedness, count, size or fit and no message about them. Limits that protect the program stay: a file over 64 MiB, or one that is not a valid Curvyo document or SVG, is refused with a plain message and nothing changes. (stage 2 for the SVG part)
23. Given a load, then it is one commit (`load_template`) and one undo step named "Template": the new nodes are guides, sit in the active layer at the top (`0050`), keep their names where the source had them, and are selected afterwards, so the maker can move them as a group at once. The source is not stored: no link, no path; moving or deleting the file changes nothing. Loading again adds again.
24. Golden-file tests in `tests/fixtures/` (`CLAUDE.md` §5): stage 1: a `.curvyo` with a key ring (a path with a hole), two concentric circles, four lines, and one with guides and ordinary paths (only the guides come in). Stage 2: an SVG with a frame, one without, an overhanging one, one with groups and a `<use>`, and one that is not an SVG.

### Part F: links to manufacturing (stated, not built)

25. Guides are never part of manufacturing output: the job preview and every machine output (`0029`) leave them out; artwork export (`0024` export, a later slice) leaves them out by default, and an option to include them is that slice's decision; nesting (R-MFG-004) may use guides as the container; an embroidery hoop is a template of the maker's own. Each is a later entry that reads `guide`; none changes this spec's model. Importing an SVG into a project never marks anything as guide. Snapping to guides is `0060` (draft); this slice does not snap.

### Part G: how the guide is stored (ADR 0019)

26. Given two replicas of one document, when one marks a path as guide while the other moves it or edits its nodes, then after merge the path is a guide at its edited place (the flag is its own node register; geometry registers merge as before). Given both mark and unmark the same node, then the last writer wins. Test: merge both ways, same result.
27. Removed (was: point-in-shape test).

### Part H: undo, history, determinism, limits

28. Given `set_guide` or `load_template`, then each is one commit and one undo step (`0020`) named "Guide" and "Template" in the history list (`0041`, `0042`). Ctrl+Z reverses it in one press and redo restores it. Undoing after a concurrent edit by another replica keeps that edit (ADR 0009).
29. Given the History tab (`0042`) with Preview, then the preview of an older version draws the guides that version had, in the marking style; "Go to this version" and "Clone" restore them; a version before this slice has none.
30. Given a document with guides, then saving it twice, or saving, opening and saving again, gives byte-identical project files; the `guide` key is written only on nodes that are guides. Golden: `guide_v<N>.curvyo` (a path, a circle, a compound path with a hole, a clone of a guide).
31. Given damaged data, then opening reports the file as damaged with the key name and does not draw: a `guide` value that is not a bool. A version number above the build's is refused first as "saved by a newer version". One golden fixture per rule.
32. Removed (was: Radius field limits).
33. Given a template of 5,000 nodes or one 100 000 mm wide, then it loads and draws without NaN or empty output, and the marking style adds at most one stroke run per guide to the draw list (stage 1 test: counts; the frame budget of the canvas is not touched).
34. Given any other feature, then none changes because a guide exists beyond what this spec states: Select all, Duplicate, Group, Boolean, Align and Transform treat a guide as the node it is; Duplicate and a copy-paste keep the flag. **A new path made from inputs by Boolean, Combine or Merge is a guide only when every input was a guide**, otherwise a normal path. A clone (`0049`, `0057`) of a guide is drawn as a guide and is not output; the origin's flag is read at draw time.
35. Removed.
36. Removed.
37. Given keyboard use, then the Guide button is one Tab stop in the Select bar, operable by Space and Enter, with `aria-pressed` and the accessible name "Guide"; the Template button is a separate stop in the File tab. Focus returns to the canvas after a mouse press.
38. Removed.
39. Removed.
40. Removed.

### Part I: guides out of output (new)

41. Given a document with guides, then the machine outputs of the slices that exist now and later (`0029`, G-code, HPGL, embroidery) and the artwork export of `0024` do not contain them, and the Layers tab (`0050`) lists them as ordinary rows. Test: the same drawing with and without a guide gives the same output bytes (the output function filters on `guide`).

## Out of scope

- A shaped page (circle, rounded rectangle, outline as the document area), shape formats, scaling or fitting an outline: dropped by the customer (2026-10-17).
- Checks of any kind on a template: closedness, count, size, overhang (customer: "we do no checking").
- Drawing the circle and the rounded rectangle with a dedicated Template tool, or a Circle/Diameter shape for the template: "later, out of scope for this slice" (customer). The existing Ellipse and Rectangle tools make them now.
- DXF, and any importer beyond `0024`'s SVG.
- Snapping to guides, magnetic guides, ruler-dragged guide lines: `0060` (draft) and later. Multi-page (`0059`, draft): a guide is an object at coordinates and belongs to the page whose rectangle holds it, as any object does; nothing here changes when pages exist.
- Per-guide colour or style, guide layers, locking all guides at once, a "show guides" switch.
- Including guides in an export; a "Template" in the formats library.

## Decided by the product owner (2026-10-17)

1. One concept: template paths and guide lines are the same thing, the `guide` flag. No second register, no separate "template" object.
2. The flag applies to paths, compound paths and the parametric shapes (a circle is an ellipse node, so it must), and it is kept when the shape is edited.
3. Fit to content ignores guides; zoom to document includes them. The page stays the rectangle.
4. Loading from a Curvyo document takes the source's guides, and everything visible when it has none ("load directly from the document"); there is no picker (permit over refuse).
5. Pressing Guide on a mixed selection marks all; the button is pressed only when everything markable is already a guide (a toggle with a clear state).
6. A path made from inputs by an operation is a guide only when all inputs were.
7. The Template button sits in the File tab beside Open: it is an action on the file, a text button like its neighbours.
8. Not asked of the customer: his answers decide SHAPE-ADR (yes), SHAPE-LIST, SHAPE-MULTI, SHAPE-CIRCLE and SHAPE-SCALE; no question is open.
9. The guide green is `#166534` (2026-10-19, measured at build): 3.6:1 on `--pasteboard-bg` and 5.8:1 on `--canvas-bg`; the proposed `#15803D` has 2.5:1 on the pasteboard, below the 3:1 this spec asks for.

## Open questions

None for the customer. Defaults taken: the green is `#166534` (measured at build, see decision 9); the Template button is in the File tab (alternative: a button in the Select bar, not chosen because it is a file action).

## UX notes

Revised for the new design on 2026-10-17 against `docs/design-system.md` ("Select bar layout", "Flip buttons", File tab) and the customer rules: the right panel never grows a section for an operation; nothing appears out of nowhere; operations are tools or actions with a visible trigger; symbols only; permit over refuse; no hint labels; no Apply button.

- **Guide button.** In the Select bar, after the Flip group, one 28 × 28 px icon button, Lucide `SquareDashed` 16 px, 1.5 px stroke, `currentColor` (`--toolbar-icon`), `rounded-md`, hover as the Flip buttons. Always present; disabled (greyed) only when the selection holds nothing that can be a guide. Pressed (`aria-pressed`, `--editor-accent` ground as other toggles) when every markable node of the selection is a guide. Tooltip "Guide" (Radix, after 400 ms), accessible name "Guide". No label, no caption, no tip. It does not move: the Select bar keeps its two-row layout.
- **Marking style on the canvas.** `--guide` stroke, one device pixel, dashes 2 / 2 device-aligned as the transform guide; no fill; no casing; selection overlay, handles and node glyphs as for any node. Hover and selected looks are the normal ones, drawn on top. The colour and the dash are the same everywhere a guide shows (canvas, history preview).
- **Panel.** The Document section and the Style tab are not touched. The Layers tab (`0050`) shows a guide as an ordinary row; no new glyph, no new column. Style shows the guide's own fill and stroke as stored.
- **Template button.** In the File tab, an outline text button like New and Open… (28 px high, 118 px where it pairs, otherwise full width in its own row after Close), label "Template", tooltip "Template". It opens the OS file dialog and nothing else. After the load the canvas notice slot (3 s, `role="status"`) reads "Template loaded." and the new guides are selected.
- **No inline form, no dialog, no preview.** The result is the preview; Undo is the way back.
- **Design system.** The implementer adds a row "Guide (`0046`)" to `docs/design-system.md` (the token `--guide`, the button, the dash) in the PR; the earlier rows for the shape glyphs and the shape formats are removed with this spec.

Criteria changes applied on 2026-10-17: **1, 2, 5, 7, 8, 9, 10, 13, 14, 19 to 25, 26, 28 to 31, 33, 34, 37** rewritten for the new design; **41** added; **3, 4, 6, 11, 12, 15 to 18, 27, 32, 35, 36, 38 to 40** removed; staging reduced to two stages.

## Links

Requirements: R-EDIT-028
Depends on: `specs/0050-layers-and-objects-panel/`, `specs/0051-sidebar-and-no-menu/` (File tab), `specs/0049-linked-clone/`, `specs/0042-history-branches/`, `specs/0015-document-size-and-rulers/`; stage 2 `specs/0024-svg-import-export/` (Draft)
Related: `specs/0045-document-formats-library/` (formats unchanged), `specs/0016-boolean-operations/`, `0057`, `0058` (unaffected), `0059` and `0060` (drafts: pages, snapping), ADR 0002 (units, SVG at 96 dpi), ADR 0004 §9 (format versioning)
ADRs: `adrs.md`; [ADR 0019](../../docs/adr/0019-document-template-and-guide-paths.md) (Accepted 2026-10-17)
PR: https://github.com/curvyo/curvyo/pull/150 (stage 1)

**SVG export (2026-11-01):** Guides are written in one hidden, locked layer "Guides" and come back as guides (criteria 143, 144); see `specs/0024-svg-import-export`, "Slice 4: export".
