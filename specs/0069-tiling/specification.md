# Tiling: grid and circular arrays of an object, with variance and a seeded randomizer

Status: Ready (2026-11-01; ADR 0028, adrs.md and ux-notes.md written), waiting for the customer's release of the build
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "I need a tool/workflow similar to the tiling in Inkscape or LightBurn: tiled clones with variance control for rotation, scaling etc. with a randomizer. Tiling should also be possible in a circle like in LightBurn." Decisions of the customer (2026-10-17 and 2026-11-01) are listed under "Decided". Everything marked **(Proposal)** is the product owner's or the architect's choice, vetoable at the demo. Where this spec and `docs/adr/0028-stored-tiling-nodes.md` or `adrs.md` differ, the ADR wins.

## User value

As a maker I want to fill a sheet with a shape (50 hearts on a coaster, 12 petals around a centre), let each instance vary a little in rotation, scale, position, opacity or colour, and come back later to change the count, the spacing or the seed, so that a pattern looks hand-made, stays editable, and follows when I change the original once.

**What other tools do, and what we do better.**
- **Inkscape: Create Tiled Clones** (dialog with tabs Symmetry, Shift, Scale, Rotation, Blur, Opacity, Color; per-row/column/random variance in percent; a Randomize button that is not repeatable; no live preview; the result is loose clones that cannot be re-opened). We keep: row/column steps and random variance. We do better: live preview, a seed that makes a random result exactly repeatable on any machine, and a tiling that can be re-opened.
- **LightBurn: Grid Array** (columns, rows, spacing, mirror odd rows/columns) and **Circular Array** (count, centre, radius, angle step, rotate copies); copies only, no variance, no re-opening. We keep both layouts and the mirror switches. We do better: variance, seed, linked clones, re-opening.
- **Illustrator** (Pattern Options, Repeat) and **Affinity** (Repeat) keep the repeat live; we follow that model: the tiling remembers its recipe.

## Words

- **Source:** the selected object(s) the tiling repeats. Their box (the box Align and Distribute use, `0061`) is the **cell box**.
- **Instance:** one repeat of the source, identified by its **cell key** (grid: row and column; circle: index). For a source of several objects, an instance is a group of one clone (or copy) per source object (TILE-SOURCE A).
- **Tiling:** a group named "Tiling" that holds the instances as real children and remembers its recipe (the source objects, all options, the seed) ([ADR 0028](../../docs/adr/0028-stored-tiling-nodes.md)).
- **Grid tiling** and **circle tiling:** the two layouts.
- **Variance:** a per-instance change of rotation, scale, offset, opacity or colour, random within plus or minus a range, stepped per row/column, or both added.
- **Seed:** an integer; with the same seed, source and options the result is exactly the same.
- **Apply:** committing the options; creates the tiling or, on a reopened tiling, re-generates it.
- **Release tiling:** turns the tiling into a plain group for good.

## Decided

Customer, 2026-10-17:
- **TILE-RESULT:** linked clones by default; copies as a switch.
- **TILE-HOME:** core.

Customer, 2026-11-01:
- **TILE-LIVE B:** a tiling is a stored, reopenable object (source + options + seed), reopened by selecting it and pressing Tile or a handle; changing options re-generates the instances; "Release tiling" turns it into its plain group of clones/copies; it is the base for `0071`. (ADR 0028, Accepted.)
- **TILE-HOME-UI A:** a **Tile** button in the Select bar switches the top bar to the tiling options with live preview; Enter or Apply commits, Escape cancels.
- **TILE-SOURCE A:** each instance is a group of clones, one per source object; primitives become paths first; images inside become copies.
- **TILE-SPACING A:** the grid spacing is the gap between boxes (0 = touching); a switch changes it to pitch (centre to centre).
- **TILE-EXTRAS A:** offset every second row/column by half a cell; stepped variance per row/column besides random.
- **TILE-RESULT-GROUP A:** the instances and an in-place source sit in one group "Tiling" in the source's layer.

Lead and architect, 2026-11-01 (**Proposal** until the demo): instances are real nodes that can be edited by hand until the next Apply; sketches tile as linked clones (clonable since ADR 0026), only images are copies; scale variance is uniform only. Enter on a selected tiling reopens it (a double-click enters the group).

## Acceptance criteria

Defaults named here are **(Proposal)** unless stated. "mm" stands for the document's length unit.

### A. Starting, options and committing

1. Given a selection of one or more objects, when the maker presses **Tile** in the Select bar, then the top bar switches to the tiling options (layout Grid or Circle, layout numbers, variance, seed with dice, Result as, Keep source in place, Apply, Cancel) and the canvas shows the live preview of the defaults of criteria 6 and 11. Nothing is written to the document until Apply.
2. Given the tiling options are open, when the maker presses Enter or clicks Apply, then the tiling is committed as one history step "Tile" and the options close; when the maker presses Escape or clicks Cancel, then the document is exactly as before (no history step, no leftover object, document version unchanged) and the options close.
3. Given an empty selection, or a selection that contains a tiling together with other objects, then the Tile button is disabled and its tooltip says why.
4. Given a source, then the following are refused with a notice naming the reason, **before anything is written** (the document version is unchanged after each refusal): no source object left after skipping dimensions (they are never tiled, ADR 0027); an object on a locked layer; an orphan clone or a clone chain over its limit of 8 (`0049`); an instance group nested deeper than the group depth limit; more instances than the refusal limit (criterion 36); a source that contains a tiling (no tiling inside a tiling in this version).
5. Given the source, then the cell box is the box Align and Distribute use (`0061`); a source of several objects keeps its relative arrangement inside every instance; a selected group counts as its leaves (nested groups are flattened inside each instance).

### B. Grid layout

6. Given Grid is chosen, then the options are **Columns** and **Rows** (integers 1 to 100,000, their product at most the limit of criterion 36; defaults 5 and 4), **Spacing** as separate X and Y fields (default gap 5 mm each) and the switches of criteria 8 to 10. Cell (row 0, column 0) is at the source; columns grow to the right, rows grow downward on screen.
7. Given a Spacing mode switch **Gap / Pitch** (TILE-SPACING A), when Gap is chosen (default), then the distance between the right edge of one cell box and the left edge of the next is the X gap (likewise rows with Y), and a gap of 0 makes the boxes touch; when Pitch is chosen, then the distance between box centres is the entered value. Switching the mode converts the entered numbers so that the layout does not move. Negative gaps are allowed (overlap); pitch must be greater than 0.
8. Given the toggles **Mirror every second row** and **Mirror every second column** (independent, default off), then the instances in rows 1, 3, ... are flipped about their own horizontal centre line and those in columns 1, 3, ... about their own vertical centre line; both on flips odd/odd instances both ways.
9. Given the toggles **Offset every second row** and **Offset every second column** (TILE-EXTRAS A; mutually exclusive, default off) and a **Shift** field (percent of the pitch, 0 to 100, default 50), then offset rows shifts rows 1, 3, ... to the right, and offset columns shifts columns 1, 3, ... downward, by Shift of the pitch; row 0 and column 0 stay at the source. With a gap of 0 and Shift 50, offset rows gives a brick pattern.
10. Given mirror and offset together, then the offset moves the cell and the mirror flips the instance about its own (offset) centre.

### C. Circle layout

11. Given Circle is chosen, then the options are **Count** (integer 1 to 100,000; default 12), **Radius** (more than 0; default half the larger side of the cell box plus 10 mm), **Start angle** (degrees, default 0), **Sweep** (degrees, -360 to 360, default 360), **Face centre** (switch, default off) and **Centre** X, Y (document coordinates). Angles run counterclockwise as seen on screen, 0 degrees pointing right; a negative sweep runs clockwise. The default centre is the cell box centre shifted left by the radius, so the source is the instance at the start angle (the 12 petals case works without typing the centre).
12. Given |Sweep| = 360, then instance k sits at start + k x sweep / count; given |Sweep| less than 360, then instance k sits at start + k x sweep / (count - 1) so that the first and last instance lie on the ends of the arc; count 1 places one instance at the start angle.
13. Given **Face centre** on, then each instance is rotated about its own box centre by its angle on the circle, so it keeps its orientation relative to the radius; given it is off, the layout only moves the instances.
14. Given the circle options are open, then a **centre handle** and a **radius handle** on the canvas can be dragged; the fields and the preview follow live, and the handles snap like other handles (`0060`, `0090`, Ctrl suspends). The centre is stored relative to the cell box, so moving the finished tiling moves its centre with it. The grid has no handles in this version.

### D. Variance and seed

15. Given the **Variance** section (behind one button in the bar whose tooltip states which channels are active; defaults all 0), then each of these channels offers **random +/-**, **per row** and **per column** numbers (circle: random +/- and **per step**, added per instance index):
    - **Rotation** in degrees, about the instance's box centre;
    - **Scale** in percent, **uniform only** (**Proposal**; per-axis scale is out of scope for this version, though the stored recipe has room for it), about the instance's box centre;
    - **Offset X** and **Offset Y** in mm;
    - **Opacity** in percentage points of the object's opacity, clamped to 0 to 100 percent;
    - **Colour** as a hue shift in degrees of the fill and the stroke colour of every object of the instance.
16. Given a channel with random +/- r, step s per row and t per column, then for the instance in row i and column j the value is `i x s + j x t + u x r`, where u is a random number in [-1, 1) drawn for that cell and channel; a scale value v means the factor `1 + v / 100`; negative steps are allowed, random +/- must be 0 or more. Stepped variance draws no random number.
17. Given the order of effects, then the instance affine is `translate(cell position + offset) · rotate(angle) · scale` about the cell box centre, with mirror and offset rows/columns as part of the cell position, so the result is the same on every run and platform. An instance that is the kept source (criterion 22) is never varied.
18. Given colour or opacity variance different from zero, then each affected instance receives its own style (clones: the clone's own style override of ADR 0016, the source untouched; copies: written into the copy's style); an object without fill or stroke has nothing to shift for that part. An instance with such an own style **stops following the source's style edits until the next Apply**.
19. Given the **Seed** (integer 0 to 4,294,967,295), then a new Tile starts with a fresh random seed (never 0 by surprise), the dice button next to the field picks a new one and shows it, and the preview updates; the seed is stored with the tiling and is never changed by Apply, reopen or release. The random value for the dice comes from the platform (the frontend), never from the layout code, which reads no clock or entropy.
20. Given the generator, then it is **PCG32** (M. E. O'Neill's `pcg32`, XSH-RR output, 64-bit state, the reference multiplier and the fixed reference increment), written by hand with a jump-ahead `advance(delta)` in O(log delta), without the `rand` crate, and documented in `docs/` with the algorithm and seeding. Its outputs are **pinned by a golden test**: the first 16 outputs for seeds 0, 1, 42 and 4,294,967,295, computed from the reference algorithm and **checked by the implementer against the reference C implementation's output**; a second test proves `advance(n)` equals `n` single steps. A draw becomes a unit value as `x as f64 / 2^32` (exact), and a range as `r x (2u - 1)`.
21. Given the draw order, then the generator for a cell starts at the seed, is advanced by `cell_index x 8` draws (`cell_index = row x 2^32 + column`, circle: row 0, column = index) and draws, always and in this order: rotation, scale X, scale Y, offset X, offset Y, opacity, hue (the eighth slot is spare; uniform scale uses the scale X draw). A channel at zero range still draws. Consequently, turning one channel on or changing its range never changes another channel's values, and adding rows, columns or instances never changes the existing cells. With the same source geometry, options and seed the instance matrices are identical on desktop and wasm within 1e-9 mm and 1e-12 rad (only `sin`/`cos` may differ in the last bit; the generator's integers are identical) and after saving and reopening the document.

### E. Result, source and organisation

22. Given **Keep source in place** (default on), then the selected source objects move into the new group "Tiling" as its lowest children and stand for cell (0, 0) (grid) or index 0 (circle), which is not generated and carries no variance; the other cells are generated. Given it is off, then the source objects stay where they are, untouched and drawn, the "Tiling" group is placed directly above the topmost of them, and all cells including the first are generated.
23. Given **Result as** (TILE-RESULT): **Clones** (default) makes every instance of linked clones of the source objects (`0049`, with their own placement, so moving the source alone does not scatter the instances): paths, compound paths, primitives (converted to paths in the same commit), sketches and clones give linked clones, texts give linked text clones (`0080`); **images are always copies** (they are not clonable; the copy keeps the asset). **Copies** makes every leaf an independent copy of the source leaf mapped by the instance's affine. Dimensions are skipped. Clones and copies look the same on screen.
24. Given a commit, then the document gets one group named "Tiling" in the layer of the source (TILE-RESULT-GROUP A); a source of one leaf gives instances that are that leaf's clone, a source of several leaves gives instances that are groups of one clone or copy per leaf in the sources' z-order.

### F. The stored tiling

25. Given a committed tiling, then the document stores in the "Tiling" group the recipe (layout and all its options, spacing mode, mirror and offset switches, all variance numbers, the seed, Result as, Keep source in place) and the ordered list of source object ids; every instance carries its cell key and every member of a multi-source instance carries its source leaf (ADR 0028, new registers `tiling`, `tiling_sources`, `tiling_cell`, `tiling_member`). Documents without tilings open unchanged; **`format_version` goes up by one to the next free number at merge (32 today, `main` is at 31)**; an older build must refuse a file with a tiling; the damage and tolerated-residue rules of ADR 0028 §10 apply.
26. Given a tiling is selected, then the Select bar's Tile button reads **Edit tiling** and a second button **Release tiling** follows it; pressing Edit tiling, or **Enter** on the selected tiling, opens the tiling options with the stored values (copies tilings too) and the live preview of criteria 33 to 36. Opening writes nothing; the seed is the stored seed.
27. Given a reopened tiling, when the maker changes options and presses Enter/Apply, then the tiling is re-generated in one history step "Edit tiling": instances whose cell key stays are kept and rewritten only where placement, geometry or own style actually differs (beyond 1e-9 mm), missing cells and members are created, cells that left the layout are deleted; an Apply that changes nothing writes nothing and adds no history step; Escape or Cancel leaves everything unchanged. Changing the layout between Grid and Circle keeps variance and seed and each layout's last values.
28. Given edits by hand, then an **instance is an ordinary node**: it can be selected inside the group (Layers panel; a click selects the tiling as a whole, a double-click enters the group and selects one instance), moved, given its own style, unlinked or deleted with the existing commands. The **next Apply overwrites** the placement of every managed instance, restores deleted cells, and rewrites own style only while opacity or hue variance is on; an own style set by hand while that variance is off is kept; an instance the maker unlinked or converted is left alone and its cell counts as taken; objects without a cell key that the maker put into the group are left alone. To keep hand edits for good the maker uses Release tiling.
29. Given the source is edited (shape, nodes, text, style), then all clone instances follow immediately, except the style of instances with an own style (criterion 18); **copy** instances and image copies do not follow and are refreshed by the next Apply; moving or rotating a source on its own does not scatter the instances; if the cell box changes size the layout is recomputed at the next Apply.
30. Given a source object is deleted, then its clones are repaired as for any deleted source (`0049`, ADR 0016); the recipe keeps the dangling id, reopening offers only **Release tiling**, and undo restores everything.
31. Given the whole tiling is deleted, copied, cut, pasted or duplicated, then deleting removes the group with its children, the kept source included, in one history step (undo restores all, with options and seed); copying produces an independent tiling with identical options and seed, also into another document, with its source list and member references remapped to the copy; an instance pasted outside its tiling group loses its cell key and is a plain object. A tiling is not itself clonable (`0049`, groups are not clonable).
32. Given **Release tiling** (Select bar and Object menu; one history step "Release tiling"), then the recipe and every cell key and member key are removed and what remains is a plain group of clones and copies exactly as drawn; Ungroup of a tiling does the same; afterwards nothing remembers the tiling and the selection stays on the group. Given two peers apply concurrently and leave duplicate cells, then reading tolerates it and the next Apply keeps, per cell, the child with the lowest node id and removes the others.
33. Given Export SVG (`0024`), machine output and any other output, then they write the tiling exactly as the same group of clones and copies made by hand (a group of resolved outlines; the file contains no tiling data and opens correctly in Inkscape); the exported geometry equals the on-screen geometry within 0.001 mm. Reading a tiling costs nothing beyond reading a group; opening a document computes no layout.

### G. Preview and counts

34. Given the options are open, then the preview draws the instances (with variance, mirror and offset) as an overlay of the actual outlines, updates on every valid keystroke (debounced 120 ms) and on the dice, and is never selectable, snappable, exported or stored; on a reopened tiling the managed instances are hidden from the normal drawing while the preview is shown. An invalid entry keeps the last valid preview and marks the field.
35. Given the preview and the committed result, then they show the same geometry (one function computes both). Undo and redo are locked while the options are open; switching tool or page, New, Open or closing the document cancel without a message.
36. Given counts are **instance counts**, not node counts (rows x columns, or the circle count; a multi-source tiling creates several times as many nodes; **Proposal**): above 10,000 the bar shows a quiet warning with the number and the preview draws instance bounding boxes only; above 100,000 Apply is disabled and the bar states the limit, so no tiling above 100,000 instances is ever created. The count shown is the number that will be made (the kept source counted as an instance).

### H. History, pages, snapping

37. Given undo and redo, then Tile, Edit tiling and Release tiling are each exactly one step, however many options were changed before Apply; the labels "Tile", "Edit tiling", "Release tiling" are added to `curvyo-ui-core/src/history_names.rs`.
38. Given pages (`0059`) and the grid (`0090`), then instances ignore pages and land where the layout puts them, including outside the page; no page is added or resized; the instances themselves do not snap to the grid; the tiling as a whole snaps like any group when moved; moving, rotating or scaling the whole tiling moves the source and every instance together, and the stored layout numbers do not change.
39. Given a tiling is selected, then Align and Distribute (`0061`) treat it as one object.

### I. Performance and tests

40. Given a source of one path with 50 segments tiled 100 x 100 (10,000 instances) with all variance channels on, then thread-CPU-time tests in `curvyo-editor-wasm/tests/cpu_time/` measure: layout plus preview build after one option change; an Apply that creates 10,000 instances; a seed change on 10,000 existing instances (rewrites only what changed). Initial targets **(Proposal)**: preview at most 50 ms, Apply at most 500 ms; the measured numbers are written into `plan.md`, the asserts run in release builds only, and a missed target is reported and tracked in `docs/technical-debt.md`, not relaxed silently.
41. Given the generator and layout, then golden files in `tests/fixtures/tiling/` hold the PCG32 sequences of criterion 20 and, for each case, the expected affines (tolerance 1e-9): grid 3 x 3 gap 0; grid with mirror odd rows and columns; offset rows; pitch mode; circle default; circle with sweep 180 and face centre on; random variance with seed 42 (all channels); stepped per row and per column; mixed random and stepped; a test proves that changing one channel's range, or adding a column, leaves the other values untouched (criterion 21). The same fixtures run on host and `wasm32`.
42. Given the file goldens `tests/fixtures/tiling_*_v32.curvyo` (version number follows criterion 25): a grid with hue variance, a circle, a multi-source tiling with an image copy, Keep source on and off, a copies tiling, one damage file per rule of ADR 0028 §10 (each is refused), and a residue file (dangling source id, duplicate and missing keys) that opens; a tiling's SVG export equals that of the same group made by hand.
43. Given the acceptance tests, then they cover: the refusals of criterion 4; each option changes the preview as described; Escape leaves the document byte-identical; Tile, Edit tiling, Release tiling, undo and redo are one step each; an unchanged Apply writes nothing; hand edits are overwritten and deleted cells restored; the source edit follows in clones and not in copies; copy and paste of a tiling; the tests listing every writer that copies or ungroups handle the three tiling registers; save, reopen and export SVG reproduce the instances.

## Out of scope

- Inkscape's blur and trace-from-drawing tabs; tiling along an arbitrary path (a later entry, with `0033` brushes); tiling in 3D; non-affine fills (warp, envelope).
- Automatic spacing from the kerf; nesting (`R-MFG-004`); instances avoiding each other.
- Keeping instances inside a shape and pattern fill: `0071`, `0067` (this entry only stores the recipe they can reuse).
- Tilings inside tilings; cloning a tiling.
- A per-instance "detached" flag that survives Apply (Release is the way out).
- Per-axis scale variance (uniform only in this version).
- Hue variance from a palette (Spread colours with named palettes); only a hue shift here.
- Rotating the whole grid layout by an angle (the maker rotates the tiling as a whole); grid canvas handles.
- A keyboard shortcut for Tile.

## UX notes

See [`ux-notes.md`](ux-notes.md) (ux-engineer, 2026-11-01). Where it differs from this spec, this spec and ADR 0028 win; known points to reconcile in `ux-notes.md`: instances are selectable inside the group and the group is expandable (criterion 28), only images become copies (criterion 23), Keep source in place is on by default and means the source stands for cell (0, 0) (criterion 22), and the circle radius, count and face-centre defaults are those of criterion 11.

## Links

Requirements: R-EDIT-044 (new); related R-MFG-004
Depends on: `0049-linked-clone`, `0057-clone-overrides`, `0080` linked text clones, `0061` Select bar, ADR 0026 (sketches clonable)
Related: `0071-pattern-fill`, `0033-stroke-brushes`, `0067-clipping`, `0065-extensions-sdk`, `0024` SVG export, `0059` pages, `0060`/`0090` snapping and grid
Plan notes: `specs/0069-tiling/tiling-plan.md`
ADRs: `docs/adr/0028-stored-tiling-nodes.md` (Accepted, TILE-LIVE B); `specs/0069-tiling/adrs.md`
UX notes: `specs/0069-tiling/ux-notes.md`
PR: -
