# Stroke brushes: a stroke that is drawn by a brush, with brushes the maker can add

Status: Ready (2026-10-17) for **stage 1 only** (built-in brushes). Stage 2 (the maker's own brush files) and stage 3 (plugin brushes) are not Ready; see "Stages". Nothing in stage 2 or 3 is built from this spec until it is split into its own Ready spec or section.
Priority: Could (stage 1 can be built now: it needs no plugin host)
Origin: Customer (brushes for the stroke, customizable, added by the customer's own code or extension, 2026-10-09). The kinds, the built-in set, the file format and every default below are my proposals until the customer has read them (the customer has answered the questions, see "Decided (customer, 2026-10-17)"; the "Decided by the product owner" list stays open to veto).

## User value

As a maker I want to give a path a brush instead of a plain line (a tapered stroke, a calligraphic pen, a row of dots, a perforation, a scalloped edge), and later to add brushes of my own without waiting for a release, so that decorative lines, borders and cut patterns are one stroke setting on a path and not a hundred hand-placed shapes.

The customer's words (translated): brushes for the stroke, customizable, either through an extension or through code, and addable by customers.

## Stages

| Stage | What | Needs | State |
|---|---|---|---|
| **1** | Built-in brushes of the three kinds (width profile, stamp, pattern along path), the picker and generated parameter rows in the Stroke section, a seed, limits, Expand brush, the stored brush reference, the rule for jobs. A brush is a validated data value (`BrushDef`, TOML) that ships in the program; the loader and its tests are the ones stage 2 reuses. | `0017`, `0018` (both merged), `0049`, `0057` (merged or in review, no code dependency on their branches), the `toml` crate already used by `0030` | **Ready** |
| **2** | Brush files of the maker: a folder, an Import action, the list shows them without a restart, the file embeds the definitions it uses, the folder syncs as a library. Criteria 11, 13 (second half), 14, 15 (second half). | Stage 1; ADR 0004 §6 and §11 (libraries); a platform crate for the folder (`curvyo-storage-io`) | Not Ready: waits for the customer's answer to Frage 4 and for a spec section with its own criteria numbers |
| **3** | Plugin brushes: a WebAssembly component that takes the centre line, the width and declared parameters and returns paths. Criterion 12. | The plugin host (ADR 0005, R-ET-001; not built) and an ADR for the `brush` interface (`needs-customer`) | **Waits for the plugin host** |

Everything below is stage 1 unless a criterion says "Stage 2" or "Stage 3". Criteria 1 to 15 keep the numbers of the sketch (11 to 15 are listed after 26 because they belong to stages 2 and 3, and 15 is split); 16 and up are new.

## What a brush is

A brush is a rule that turns the **centre line** of a path and its stroke width into the closed shapes that are drawn. The path stays an ordinary editable path; the brush is a stroke setting like the dash pattern. Three kinds:

1. **Width profile** (tapered, calligraphic): the line's width varies along its length. Two nibs: a **round** nib (the width at a point is the diameter of a disc; used by Taper) and a **flat** nib at a fixed angle (used by Calligraphic). Result: one closed outline.
2. **Stamp** (dots, perforation): copies of a small closed shape placed along the path by arc length, with a size, an optional rotation to follow the path, and optional seeded jitter. Result: many small closed shapes.
3. **Pattern along path** (art brush): one closed shape stretched and bent along the whole path, repeated a whole number of times. Result: one closed shape per repeat.

A fourth kind (repeated tiles with corner pieces) and start and end pieces for patterns are not in stage 1 (Out of scope).

**Words used below.**
- **Centre line:** the path's outline after the node's own placement and, for a clone, after its matrix (`0049`), flattened to within 0.01 mm. Brush geometry is computed from what is on the page, so a clone and the same path after Unlink clone give the same brush geometry (`0057` criterion 40).
- **w:** the object's stroke Width in mm (`0017`), the brush's master size (criterion 3).
- **L:** the length of the centre line in mm.
- **Piece:** one closed shape of a brush result.
- **Brushable object:** a path (open or closed). Not brushable in stage 1: primitives (rectangle, ellipse, polygon, star, box), compound paths, groups, and clones without an own style (they show their original's brush, `0057`).

## Reference tools

- **Illustrator:** Calligraphic, Scatter, Art, Pattern and Bristle brushes in a Brushes panel; scatter and pattern brushes cannot use artwork with gradients or placed images; the brush is applied to existing paths and is editable until expanded ("Expand Appearance").
- **Affinity Designer:** vector brushes with pressure; stroke width profiles.
- **Inkscape:** no brush panel. The same results come from path effects (Pattern Along Path, PowerStroke, Bend) that live in a dialog, deform the pattern, and are slow and hard to find; this is the "extensions are slow and bolted-on" complaint behind ADR 0005. The Calligraphy tool draws a fixed outline and is not applied to an existing path.
- **LightBurn:** none.
- What we can do better: brushes are a stroke setting (one place), the list is in the panel with no dropdown, brush parameters are rows in the same style as every other field, what is cut or engraved is exactly what is shown, and a brush result is reproducible (same file, same geometry, on every machine).

## Acceptance criteria

### A. The picker and the parameters

1. **Picker in the Stroke section.** Given the Stroke section with Paint Solid and a width above 0, then it holds a row **Brush** directly under Width and above Dash. The label "Brush" is in the label column; the control column holds a grid of tiles, 40 x 28 px, 4 px apart, 4 per row, in the order: **None**, then the built-in brushes (criterion 4), each a `role="radio"` item in one `role="radiogroup"` named "Brush"; the pressed tile is the brush of the selected objects. A tile shows a 24 x 16 px drawn sample of the brush along a short curve in `--toolbar-icon` (None: a plain line); its accessible name and its tooltip (400 ms, `side="left"`) are the brush's name ("None", "Taper"). No dropdown, no popup, no hover preview on canvas (`0017` panel rule). Stage 1 has 7 tiles and 2 rows (60 px high); the grid has no inner scroll area (the question of more tiles is stage 2). The row is **always in the tree** while the Stroke section shows its rows (criterion 5 lists when it is dimmed).
2. **Brush parameters are generated.** Given a pressed brush other than None, then the rows its definition declares follow directly under the grid, in the order of the definition, as standard rows of `0017`: a **number** is a value field in the control column with a fixed unit suffix inside the right edge ("%", "°", "x width"), range and scale from the definition (linear, typed range = declared range, drag range = declared range), reset value = the declared default; a **choice** is a text `ToggleGroup`; a **switch** is not used by a built-in brush. A parameter label is at most 8 characters (a definition with a longer one fails to load, criterion 15). Given a brush that uses randomness, then a row **Seed** (criterion 7) is the last parameter row. The parameter rows are a **fixed reserved block of three rows** directly under the grid (Decided, customer, 2026-10-17): the three rows are always in the tree. Given brush None, or a brush with fewer than three parameters, then the unused rows are dimmed (`aria-disabled`, not focusable, empty label, no value), so the layout never moves when a brush is pressed (customer rule "nothing appears out of nowhere"). A brush cannot draw its own controls (ADR 0005 decision 8). Preview while dragging a field and commit on release follow `0017` ("Behaviour": one commit on pointer-up, Escape reverts, typed values do not preview).
3. **Width is the master size.** The stroke Width is the brush's nominal size: Taper's and Calligraphic's full width, Dots' diameter, Perforation's slit thickness, Scallop's height, Zigzag's peak-to-peak height. All lengths a parameter gives are in multiples of w, so changing the width, or scaling the object with "Scale stroke width" on (`0005`), rescales the whole brush result by the same factor (test: w 1 to 2 gives the same pieces scaled by 2 about the centre line, within 0.002 mm). Colour and opacity are the stroke's; every brush paints in one colour. The fill of the object is independent and is drawn under the brush.
4. **The built-in set.** Six brushes, ids in brackets (the id never changes after release, criterion 16). Lengths are in stroke widths (w); percentages are of the stated length.
   - **Taper** (`taper.1`, width profile, round nib): width 0 at both ends, rising linearly to w over the **Taper** length and falling to 0 over the same length; flat at w between. Parameter **Taper** (number, "%", 5 to 50, default 25: the share of L each ramp takes). Round ends are implied by the disc.
   - **Calligraphic** (`calligraphic.1`, width profile, flat nib): the nib is a rectangle w long and 0.15 w thick, turned by **Angle** (number, "°", 0 to 179, default 45; 0 is horizontal, counter-clockwise on screen). The nib angle is absolute (it does not turn with the object). The outline is the union of the nib swept along the centre line. Test: a straight horizontal line, Angle 0: a band 0.15 w high; Angle 90: w high.
   - **Dots** (`dots.1`, stamp, circle): circles of diameter w. Parameters **Spacing** (centre to centre, "x width", 0.5 to 20, default 2) and **Jitter** ("%", 0 to 100, default 0), then **Seed**.
   - **Perforation** (`perforation.1`, stamp, rectangle, follows the path): slits w thick and **Slit** long, **Gap** apart. Parameters **Slit** ("x width", 1 to 100, default 10) and **Gap** ("x width", 1 to 100, default 10). No jitter, no seed.
   - **Scallop** (`scallop.1`, pattern): a row of half-ellipse arcs, each **Repeat** wide (base) and w high, drawn as a band **Line** thick, bulging to the left of the direction of travel as seen on screen (a left-to-right line has the arcs above it). Parameters **Repeat** ("x width", 1 to 40, default 4), **Line** ("% of width", 2 to 50, default 10), **Side** (Left, Right; default Left).
   - **Zigzag** (`zigzag.1`, pattern): a triangle wave w peak to peak centred on the centre line, drawn as a band **Line** thick, one period **Repeat** long. Parameters **Repeat** (default 2, otherwise as Scallop) and **Line** (default 10).
   Each is a definition file in `curvyo-document-core/data/brushes/<name>.toml` (criterion 15); no brush shape or number is written in Rust source outside tests.
5. **When the Brush row is dimmed, and what a brush does to Dash, Join, Cap and Markers.**
   - Given Paint None or width 0, then the whole Stroke section is as `0017` criterion 5 says (the rows are not in the tree) and the brush setting is kept and returns with the stroke.
   - Given a selection with no brushable object (only primitives, compound paths, or clones without own style), then the Brush row stays in the tree, **dimmed and `aria-disabled`, not focusable**, no tile pressed. The tooltip of the dimmed row (a Proposal, vetoable): "Brushes work on paths." Customer rule "nothing appears out of nowhere" and the precedent of `0057` criterion 17.
   - Given a brush other than None on any selected object, then **Dash, Join and Cap stay in the tree, dimmed and `aria-disabled`, not focusable, showing their kept values**; the layout does not move. They return to normal with brush None. (The sketch said "hidden"; changed 2026-10-17 to the customer's rule, recorded as a third exception to "Hidden, not disabled" in `docs/design-system.md`, with the Style tab dimming of `0043` and the Spread colours button of `0057`.)
   - Given a brush other than None, then **Markers** stay available and are drawn over the brush result at the centre line's start, end and nodes (`0018`); they are not part of the brush result and are not expanded into it.
   - Given a brush that is over its limit (criterion 8), then Dash, Join and Cap are still dimmed.

### B. The geometry

6. **Brush shapes follow the path.** Placement is by arc length of the flattened centre line, curves included.
   - **Dots, open path:** p = Spacing x w; n = floor(L / p) + 1; the centres are at s0 + i p for i = 0..n-1 with s0 = (L - (n-1) p) / 2 (the run is centred). L = 10, w = 1, Spacing 2: n = 6, centres at 0, 2, ..., 10. L = 11: n = 6, s0 = 0.5. L shorter than p: n = 1, at L/2.
   - **Dots, closed path:** n = max(1, round(L / p)); centres at i L / n from the start node (the pitch stretches to a whole count, so the seam is seamless). L = 20, p = 2: n = 10.
   - **Perforation, open path:** pitch q = (Slit + Gap) w; n = floor((L + Gap w) / q) with at least 1; the run n Slit w + (n-1) Gap w is centred. L = 100, w = 1, Slit 10, Gap 10: n = 5, run 90, slits at [5, 15], [25, 35], ..., [85, 95]. A slit follows the path's direction (it is bent along the curve like a pattern tile of its length, so no slit is straight across a bend). **Closed:** n = max(1, round(L / q)), the gap is stretched so n slits fit exactly.
   - **Pattern (Scallop, Zigzag):** the tile length is Repeat x w; n = max(1, round(L / (Repeat w))); the tile is stretched to L / n so whole tiles fill the line, open or closed. L = 100, w = 1, Repeat 4: n = 25. The tile's x axis is the arc length, its y axis is the distance to the left of the line. Every vertex of a bent tile is within 0.01 mm of the exactly bent shape (tiles are subdivided before bending).
   - **Taper and Calligraphic:** the centre line is flattened within 0.01 mm; the outline is within 0.01 mm of the exact sweep (round arcs within 0.01 mm of the circle), on the 0.001 mm grid, in the canonical form of the boolean kernel.
   - **Closed paths and width profile:** the profile is applied from the start node around the loop to the start node. Taper therefore pinches at the start node of a closed path; this is accepted (permit over refuse) and is not an error. Calligraphic has no seam.
   - **Sharp corners:** a pattern or stamp follows the path's tangent continuously; at a corner node the bent tile may overlap itself on the inside of the turn and open on the outside. No special corner handling in stage 1.
7. **Reproducible.** Randomness exists only in Dots (Jitter). The generator is SplitMix64 written in the repository (no `rand` dependency), seeded from the **Seed** field: an integer 0 to 999,999, default 1, stored with the stroke. For stamp i (in path order) two numbers u1, u2 in [0, 1) are drawn in this order; the diameter is w x (1 - Jitter/100 x u1) and the centre is moved sideways (to the left of the direction of travel) by (u2 - 0.5) x Jitter/100 x w. Jitter 0 draws nothing from the generator. The same document therefore gives the same pieces on every machine and every peer; a job run twice gives the same pattern. Test: golden numbers for Seed 1 and Seed 2 on a 100 mm line, plus equality between the native and the wasm32 build (counts and sequence exactly; every vertex within 0.002 mm, because trigonometric functions may differ in the last bit before the 0.001 mm rounding). There is no "New random seed" action: changing the number is the way (value field, drag or type; decision 8).
8. **Limits.** A brush result has at most 1,000,000 vertices and 100,000 stamps (whichever is reached first). Given a set that would exceed it (pressing a tile, committing a parameter, Width or Seed), then nothing is written and the canvas notice slot says why, for example "Brush not applied: this would need more than 100,000 dots. Increase the spacing or use a shorter path." The notice names the limit that was hit. Given a path edit (a node drag, a transform, a clone's source changing) that pushes an object over the limit, then the edit is written as usual, the object is drawn **as the plain stroke** it would have with brush None (its own Dash, Join and Cap values) until it fits again, one notice per command, and a job refuses the object (criterion 10). The check is made before any generation: the count follows from L and the parameters, so an over-limit set costs under 5 ms.
9. **Expand brush.** Given a command **Expand brush**, then for each selected object with a brush other than None that is within its limits, the object is replaced **in place** (same parent, same z-position, same layer, same name) by an ordinary path with the brush result as its outline: one closed path for a width profile, one compound path (`0016`) for a result of several pieces. The new object has the stroke switched off and the fill on in the stroke colour, stroke opacity and node opacity of the original; the original's own fill is lost (it was under the stroke; a maker who needs both Duplicates first). Markers are removed with the stroke. Given more than 20,000 pieces, then the command is refused with a notice and nothing is written. Given no selected object with a brush, then the button is dimmed (U notes 6). One commit, `expand_brush`, label "Expand brush", for all selected objects at once. Undo (`0020`) restores the original with all its settings; so does the object history (`0041`). Customer-decided default 2026-10-17 pending (Decided, customer, 2026-10-17): replace in place; the sketch kept the original.
10. **Manufacturing.** A brush stroke is **not design-only**. For any job (laser, plotter, vinyl cutter, embroidery, CNC) an object with a brush contributes the **union of its brush pieces** (the boolean kernel's union at the manufacturing tolerance of ADR 0003, the same pieces the canvas shows), as closed paths, and **not** its centre line. What is shown is what is cut or engraved; overlapping stamps are cut once. A job refuses an object whose brush is over its limit (criterion 8) and an object with a brush id this build does not know (criterion 16), with the object named. Consequence the maker should know and the panel does not say: Scallop and Zigzag are thin bands, so a laser cuts both edges of the band; the maker who wants one cut line along a scallop uses a closed Scallop shape drawn by hand or brush None. Roles (`manufacturing-roles`) and the job preview treat each piece like any path. Stage 1 builds no job; it builds the pure function `brush_job_geometry(object) -> Vec<Outline>` in `curvyo-geometry-core` (the union above) with its tests, and `manufacturing-roles` calls it. Default A of Frage 2.

### C. The stored form

16. **Brush in the file.** Given a path, then the style may carry (each its own register, last-writer-wins per control, ADR 0009 §3, as the markers of `0018`): `stroke_brush` (string, a brush id such as `dots.1`; absent = None), `stroke_brush_seed` (integer 0 to 999,999; absent = 1), and one register per parameter, `stroke_brush_p.<brush name>.<parameter id>` (number, integer or string by the declared type; absent = the declared default), for example `stroke_brush_p.dots.spacing`. The parameters of a brush that is not pressed are kept (switching to Dots and back to Taper keeps Dots' Spacing). These keys belong to the **style schema** (`style_codec`), so everything that copies a style copies the brush: a clone with own style (`0057`), Unlink clone, the eyedropper (`0017`), Duplicate, copy and paste. Built-in ids are immutable and versioned (`taper.1`); a changed shape would get a new id (`taper.2`) and the old stays in the program. The reader tolerates: a value outside its declared range (clamped for use, the stored value left alone); a missing register (default); brush registers on a node that is not a path (ignored). The reader refuses as damaged: a brush id that is not a string, or a parameter whose type does not match its declaration. A brush id this build does not know (a file from a newer build with more built-in brushes) is **kept and written back unchanged**, the object is drawn as its plain stroke, the Brush row shows no tile pressed, one notice on open ("This file uses a brush this version does not have: N objects drawn as plain strokes"), and a job refuses the object. Adding a built-in brush never changes `format_version`.
17. **Format version.** `format_version` goes to the **next free number at merge** (`specs/README.md`, "`format_version` plan"; an older build would read a brush path as a plain stroke with Dash, Join and Cap, and a job would cut the centre line instead of the brush, a silent wrong output, the reason `0018` bumped). Every file saved by this build is refused by older builds, with or without brushes, as for `0057` criterion 37. Golden files: `brush_v<N>.curvyo` (a path with each of the six brushes, a clone with own style that has a brush, a closed path) and the damaged `brush_param_type_damaged_v<N>.curvyo`, `brush_unknown_id_v<N>.curvyo`. The `document.json` view (ADR 0004 §1) lists the brush id and parameters; it never lists generated geometry. **Generated geometry is not stored**: it is derived on every load, so a file holds the brush reference only (stage 1).

### D. The commands and the rest of the editor

18. **Mixed selections.** Given a selection of several objects, then a tile press applies to every brushable object and the notice names the others: "Brush: 2 objects skipped (not paths)." Given brushable objects with different brushes, then no tile is pressed and no parameter rows are shown (the row of a mixed state is empty, not a "Mixed" tile); pressing a tile gives all of them that brush with their own kept parameters. Given the same brush with different parameter values, then the parameter row shows the mixed state of a value field as `0017` does and an edit sets the value on all. Given only clones without own style, then criterion 5 (dimmed) applies.
19. **Clones, layers, history.**
    - A clone with own style (`0057`) carries its own brush registers (criterion 16); one without shows its effective original's brush computed on its own resolved centre line. Given a mirrored clone (negative determinant, `0056`), then its brush follows the resolved path exactly as a flipped path would, including the side of Scallop; there is no special mirror rule (the **Side** parameter is the way to turn it).
    - A hidden node (own flag or an ancestor's, `0050`) is not generated, drawn or sent to a job. A locked node refuses a brush change as for every style edit (the selection filter of `0050` keeps it out of the selection). Layer and group opacity compose after the brush as for fills.
    - Pressing a tile, committing a parameter or Seed, and Expand brush are one history step each (`0020`), commit names `set_brush`, `set_brush_param` (also for Seed), `expand_brush`; labels in the History tab "Brush: Dots", "Brush parameter", "Expand brush" (`0042` shows them as rows; branching from before a brush works like any commit). The object preview of `0041` and `0042` draws the brush the version had (it renders the version's registers through the same function), not the current one.
    - The plain-stroke fallback of criterion 8 is a drawing rule, not a document edit.
20. **Edge cases.**
    - A path with fewer than 2 anchors, L below 0.01 mm, or width 0 draws no brush and writes nothing (`0017` criterion 5 hides the section at width 0).
    - An open path whose end points coincide is still open.
    - Taper with L shorter than two ramps: the ramps shrink to L / 2 (profile is scaled to fit).
    - A self-crossing path: the pieces overlap; display uses the non-zero rule in one fill, so alpha does not double (`0018`: never darker where it overlaps).
    - A very large coordinate (beyond the kernel range of ADR 0003 §2): the object is drawn as its plain stroke, not generated, and refused by a job.
    - Escape during a drag of a brush field reverts the preview and writes nothing; a second tile press on the pressed tile writes nothing.
    - A node edit or transform of the object while the Brush row is mid-drag: not possible (one pointer).
21. **Canvas behaviour.** The object's selection box is the centre line's box as for a plain stroke; its hit test uses the centre line with a tolerance of max(w, 4 screen px) (so Dots can be picked between two dots; Scallop's arcs are within w). Node tool: nodes and handles belong to the centre line; the brush result is drawn under them, updated on every frame of a node drag within the budgets below. Brush pieces never become selectable objects before Expand brush.
22. **SVG and other formats.** Import (`0024`): an SVG has no brushes; nothing to do. Export (`0024`, "Later: export", not built): a brush stroke is exported as its pieces, one `<path>` per object with `fill` = the stroke colour, `fill-opacity` = the stroke opacity, no stroke (the same result as Expand brush), because SVG cannot draw a brush; re-import gives plain closed paths and does not restore the brush. This is a one-line addition to `0024` "Later: export" (listed under "Changes to other specs"). Copying to the system clipboard as SVG text follows the same rule.
23. **Machine-neutral drawing.** Given the render path, then a brush object draws all its pieces as one non-zero fill in the stroke colour and stroke opacity; the same pieces tessellated at the display tolerance (0.1 screen px as the other shapes) give the pixels. Test: a Dots stroke of 200 dots overlapping at Spacing 0.5 draws one flat colour value where the dots overlap, with opacity 50 %.

### E. Performance

Measured on the CI runner, native release build; the browser build may take up to twice as long. The numbers are proposals and are re-measured at the build.

24. Given a brush on a path of 100 nodes (about 5,000 flattened vertices), then generating the pieces takes at most: Taper 30 ms, Calligraphic 60 ms, Dots with 1,000 dots 30 ms, Perforation with 500 slits 30 ms, Scallop with 100 tiles 50 ms. Dots with 10,000 dots: 300 ms. These are what a node drag, a transform and a parameter drag cost per frame.
25. Given an unchanged object, then its result is cached by (centre line, w, brush id, parameters, seed); a frame that changes nothing in it regenerates nothing. Given 1,000 brush objects, none changed, then the next frame stays within 150 ms (the budget of `0049` criterion 44). The cache holds at most 64 MB of pieces in total (oldest unused first); an evicted result is regenerated. Opening a project with 1,000 brush objects costs the read only (no geometry is stored, criterion 17); the first frame generates what is visible.
26. Given the Brush row, then pressing a tile on an object that is within limits updates the canvas in the next frame and writes in under 100 ms for the cases of criterion 24.

## Stage 2 and 3 criteria (not Ready; numbers kept from the sketch)

11. **Stage 2. Custom brushes: data files.** A brush is described in a plain text file in the format below, loaded by the pure function of stage 1 (text in, validated brush or list of problems out). The maker adds a file by placing it in the app's brush folder or by an Import action using the operating system's file dialog; the brush shows in the picker without a restart. The folder and file reading are in a platform crate, the loader in a core crate.
12. **Stage 3. Custom brushes: code.** A brush that needs real computation (a procedural pattern, text along a path) is a plugin: a WebAssembly component built with the Rust SDK (ADR 0005) that takes the centre line, the width and its declared parameters and returns paths ("paths in, paths out", ADR 0005 decision 4). The plugin host is not part of the laser MVP (R-ET-001); plugin brushes wait for it. Stage 1 keeps the interface honest: every built-in brush is generated by a pure function with the shape "centre line, width, parameters in; closed outlines out".
13. **Stage 2 (and 3). Brush in the file, with its definition.** For a brush that is not built in, the document stores, besides the reference of criterion 16, a copy of the definition (a `brushes` table keyed by id and content hash), so a file draws the same on a machine without the brush. A plugin brush also stores its last generated geometry as a cache; without the plugin the cache is shown and the brush cannot be regenerated. Format change: `format_version` next free at merge, after stage 1's. Stage 1 stores no definitions because built-in ids are immutable.
14. **Stage 2. The library syncs like other libraries.** The maker's brush folder is a library in the sense of ADR 0004 §6 and §11, offline-first, synced with the maker's own computers. Brushes that arrive through an asset-library connector (R-ASSET) are a separate, later story.
15. **Validation.** *Stage 1:* every shipped brush file in `curvyo-document-core/data/brushes/` is loaded by a test in the quality gate; a file that does not load, has a key the format does not define, a parameter label longer than 8 characters, a default outside its range, a shape outside the unit box, a profile that is not increasing, or fewer than the minimum points, fails the gate with line and reason. *Stage 2:* a maker's file that fails to load is listed with each problem (line, reason) in the brush list, the other brushes still load, and nothing crashes.

### The brush file format (stage 1 uses it for the built-ins; stage 2 opens it to makers)

TOML, as the presets of `0030`, parsed by the `toml` crate that `curvyo-document-core` already uses. Shapes are SVG path data in a unit box (1 = the stroke width, x along the line, y to the left of the line); parameters are typed and ranged; nothing in the file can run code. Example (the shape of Dots and a width profile; real files carry all keys of their kind):

```toml
format = 1

[[brush]]
id = "taper.1"
name = "Taper"
kind = "width-profile"
nib = "round"
# width factor along the line (0 = start, 1 = end), linear between points
profile = [[0.0, 0.0], [0.25, 1.0], [0.75, 1.0], [1.0, 0.0]]

  [[brush.param]]
  id = "taper"
  name = "Taper"
  type = "number"
  unit = "percent"
  min = 5
  max = 50
  default = 25

[[brush]]
id = "dots.1"
name = "Dots"
kind = "stamp"
shape = "M -0.5 0 A 0.5 0.5 0 1 0 0.5 0 A 0.5 0.5 0 1 0 -0.5 0 Z"
follow_path = false
uses_seed = true
```

In stage 1 the profile of Taper is built from its Taper parameter (the points above are the default); the file format for how parameters drive a profile is decided in `adrs.md` (decision 3).

## Changes to other specs

| Where | What |
|---|---|
| `0017` criterion 5 and `docs/design-system.md` "Hidden, not disabled" | A third recorded exception: Dash, Join and Cap are dimmed, not hidden, while a brush is set (criterion 5 here). The ux-engineer records it in the design system at build. |
| `0018` | None. Markers stay on the centre line over the brush (criterion 5). |
| `0024` "Later: export" | One line: a brush stroke is exported as its pieces (criterion 22). Not in this PR; the owner of `0024` adds it. |
| `0057` | The brush registers are style keys and are covered by "own style" because they are in the `style_codec` schema (criterion 16); no text change. |
| `0035` Path card (design system "Path toolbox") | One more button, Expand brush (U notes), 4 to 5 buttons. |
| `0032` | None. Width model: see "Interplay". |

## Interplay with other features

- **Pressure (`0032-pen-tablet-input`, parked on `story/pen-tablet-input`, Draft).** `0032` writes a closed outline with pressure widths; it does not store a width model on the path. `0033` stage 1 does not depend on `0032` and `0032` does not depend on `0033`. They share one function: `variable_outline.rs` in `curvyo-geometry-core` (centre line flattened to 0.01 mm, a width at each vertex, round joins and ends via `i_overlay`'s integer variable-width stroke, result in the kernel's canonical form). Whichever story merges first brings the module; the other uses it unchanged. Taper builds its widths from the profile at each vertex's fraction of L and calls it; Calligraphic needs a flat nib, which that function does not do, and uses a new module (`nib_sweep.rs`, union of the hulls of consecutive nib positions). If `0032` later stores recorded pressure on a path (Question 2 B of `0032`), that would be a width profile of the path itself, a separate decision for both specs; it is not assumed here.
- **Variable-width offset (`0038`).** The `i_overlay` offsetter of `0038` and the variable-width stroke of `0032` are the same library family (pinned version of `0038`); no new dependency.
- **Booleans, offset, fracture, flatten, combine, cut (`0016`, `0035` to `0038`).** They work on shapes, not on strokes. A brush stroke is not an operand; they refuse it with a message that says so ("Expand the brush first.") or, where the command already treats strokes as non-operands, ignore its stroke as before. The refusal text is the only change they need.
- **Clone overrides (`0057`) and layers (`0050`):** criterion 19.
- **Paint None or width 0:** no row, no brush (`0017` criterion 5).

## Decided by the product owner

Change any of these if you disagree.

1. **Stages.** Stage 1 is built-in brushes with the brush as a validated data value; maker files and plugins come after. It lets the customer try brushes next to the laser MVP without the plugin host.
2. **Pattern along path is in stage 1,** with the three milestones in this order on the one branch: width profile (Taper, Calligraphic), stamp (Dots, Perforation), pattern (Scallop, Zigzag). If the pattern milestone is not done in time, the slice is demoed without the two pattern brushes and they follow in a second slice; the criteria stay as they are (Decided, customer, 2026-10-17).
3. **The profile of Taper is built from its one parameter** (the ramp share), not from free points; free profile points are a stage 2 file feature.
4. **No start or end pieces for patterns, no tile corner pieces** in stage 1.
5. **Brushes only on paths.** Primitives get a brush after Convert to path; compound paths do not get one (their brush would need a rule per sub-path).
6. **Pieces are closed, filled shapes only.** An open stroke as a brush output is out; Scallop and Zigzag are thin bands (Line, default 10 % of w).
7. **Dimmed, not hidden,** for Dash, Join, Cap and the whole Brush row (customer rule: nothing appears out of nowhere).
8. **Seed is a number the maker can type or drag,** not a "New random seed" action: it needs no clock or OS random number in the core, and it is the same control as every other field.
9. **No operating tips or hint labels** in the row or under the grid (YAGNI, 2026-10-16). Tooltips are names only.
10. **Expand replaces in place** (Decided, customer, 2026-10-17).
11. **Jobs cut the union of the pieces, not the centre line** (Decided, customer, 2026-10-17).
12. **No new shortcut** for any brush command. Proposal for the overall shortcut review: Expand brush gets none.
13. **Parameter rows are a fixed block of three** (Decided, customer, 2026-10-17, replaces the rows that appeared under the pressed brush).

## Out of scope

- Raster, bristle, watercolour, pressure-dynamic simulation brushes.
- Multi-colour brushes, gradients inside brushes (`0017` removed gradients).
- Brushes for fill (hatches, patterns as a fill); this is a stroke feature.
- Brushes for text outlines or for primitives' strokes (convert to path first).
- A brush editor on the canvas, a brush marketplace, brushes delivered by an asset connector.
- Embroidery stitch brushes (running stitch, satin): that is the stitch generator of R-MFG-EMB-001, which may later be offered through the same picker.
- Animating or stacking several brushes on one stroke.
- Repeated tiles with corner pieces; start and end pieces for patterns.
- "New brush from selection" (Illustrator): brushes are files (Decided, customer, 2026-10-17).
- Pressure stored on the path as a width profile (see `0032`).
- Saving the generated geometry in the file (stage 1); only the reference is stored.
- A per-brush preview in the picker beyond the static sample; a live preview on hover.

## Decided (customer, 2026-10-17)

The customer answered the five questions of the earlier draft. On brushes: "I do not know much about brushes yet, we must try it out, so go with the defaults."

- **STAGE-PATTERN: yes.** Scallop and Zigzag are in stage 1 (milestone order as in "Decided by the product owner" 2).
- **JOB-OUTPUT: pieces.** A laser or plotter job sees the union of the brush pieces as closed paths, not the centre line.
- **EXPAND-ORIGINAL: replace.** Expand brush replaces the object in place; Ctrl+Z and the object history (`0041`) bring the original back.
- **OWN-BRUSHES: next stage.** Stage 2 (the maker's own brush files) is next after stage 1, with its own Ready section; it does not need the plugin host. "New brush from the selection": no.
- **PARAM-ROWS: fixed reserved block.** Three parameter rows are always present under the grid, dimmed until a brush with that parameter is pressed. Reason: the customer rule "nothing appears out of nowhere".

Brush details beyond these are to be tried and adjusted after the demo.

## U notes

Rules applied: `docs/design-system.md` "Properties panel: Style section" (rows 28 px, 8 px apart, label column 60 px, control column 176 px), the "Hidden, not disabled" row and its exceptions, "Value field", "ToggleGroup item", "Toolbox card" and "Path toolbox"; customer rules: the right panel never grows a section for an operation, nothing appears out of nowhere, symbols-only toolbars with tooltips, permit over refuse, no operating tips or hint labels by default (2026-10-16), previews only while dragging or adjusting, new shortcuts only as Proposal.

1. **Brush is a property of the stroke, so it lives in the Stroke section** as the row **Brush** under Width and above Dash. It is **always in the tree** while the Stroke section shows rows; it is dimmed (`aria-disabled`, not focusable) when no selected object can have a brush. It is not a new section and not a popup.
2. **The grid.** Label "Brush" (14 px `--toolbar-icon`) in the label column, 28 px high on the first tile row. 40 x 28 px tiles, 4 px apart, 4 per row, 2 rows for 7 tiles (60 px), `rounded-[5px]`; the pressed tile has the pressed look of the Join and Cap items (`--toolbar-icon-active-bg`); focus ring `--accent`. Tiles are `role="radio"` in a `radiogroup` with roving focus (arrows move and press, Home and End), one Tab stop; no tile pressed for a mixed selection (the group is then not "checked"; Tab lands on the first tile). Samples are 24 x 16 px Lucide-style drawings at 1.5 px in `--toolbar-icon`: None a line; Taper a spindle; Calligraphic an angled band; Dots five dots; Perforation five slits; Scallop three arcs; Zigzag a zigzag. No text in tiles; the name is the accessible name and the tooltip.
3. **Parameter rows are a fixed block of three** (Decided, customer, 2026-10-17): always in the tree under the grid, 8 px gaps; a row without a parameter for the pressed brush (or with brush None) is dimmed, `aria-disabled`, not focusable, with no label text and no value. Rows are the standard value fields and text groups, directly under the grid, 8 px gaps. Value field: 176 px in the control column with the unit suffix inside ("%", "°", "x width"); label 14 px, at most 8 characters. Choice (Side): a two-item text group, 88 px items. Seed: a value field, integer, no suffix, linear scale. Mixed values show as in `0017`.
4. **Heights (to be re-measured at the build).** The Stroke section grows by 68 px (grid, 60 + 8) plus 3 x 36 px (28 + 8) for the fixed block, 176 px in total, whatever brush is pressed; Dots, Scallop have 3 parameter rows, Zigzag 2, Perforation 2, Calligraphic 1, Taper 1, the rest dimmed. The panel does not change height when a brush is pressed. Whole panel with Solid stroke and no markers: about 808 px; the body scrolls (existing rule: `overflow-y: auto`, one scrollbar). The ux-engineer checks the 800 x 600 minimum window.
5. **Dimmed Dash, Join, Cap.** The three keep their position and kept values, at the dimmed look of `0043`'s tab and `0057`'s Spread colours (`opacity` per the design system's disabled value; not 40 % text under 4.5:1 for the label: the label stays `--toolbar-icon`, only the controls dim, as the disabled switches of `0057`). Announced as "dimmed" via `aria-disabled`. The Markers block is unchanged.
6. **Expand brush** is a command, not a property: a fifth button in the **Path operations** card (`0035`), icon-only, 40 x 40 px, glyph 20 px (a dotted line with an arrow to a filled shape, drawn by the ux-engineer), tooltip "Expand brush", accessible name "Expand brush", `aria-describedby` for the dimmed state (as the other buttons, `0016` criteria 1, 1a, 2, 3: always rendered, dimmed when no selected object has a brush, keyboard behaviour of the rail). The card grows from 180 to 224 px. No shortcut (decision 12); no second panel, chip or dialog. The notice of criterion 18 and the limit notices use the canvas notice slot.
7. **Previews.** Only while a field or the Seed is being dragged or a key is held on it (ephemeral `StyleOverride`, `0017`); a tile press commits on click and is not previewed on hover; Escape during a drag reverts. No preview is shown for Expand brush (it is one click and one Undo).
8. **Notices** (the canvas notice slot, no dialog): the limit text of criterion 8, "Brush: N objects skipped (not paths)." (criterion 18), the unknown-brush text of criterion 16, the refusal of Expand brush over 20,000 pieces ("Expand brush: more than 20,000 pieces. Increase the spacing or use a shorter path.").
9. **Colour and contrast.** Tile samples and the pressed look reuse existing tokens; no new token. The brush geometry on the canvas uses the stroke colour.
10. **Accessibility.** The grid has the accessible name "Brush"; each tile's name is the brush name; the pressed state is `aria-checked`; parameter rows have the labels as accessible names with the unit in `aria-valuetext` ("2 widths"). A hover tooltip is never the only carrier of a name (the tooltip repeats the accessible name).
11. **Nothing else changes:** no new top bar, tool, panel tab or section, no hint line in the row.

## Links

Requirements: R-EDIT-021 (`docs/requirements.md`); related R-ET-001, R-EXT-002, R-EDIT-016, R-INP-001
Depends on: `specs/0017-style-panel-rework/`, `specs/0018-stroke-markers/` (both merged); stage 3 on the plugin host (ADR 0005, post-MVP)
Related: `specs/0030-document-size-presets/` (data-file loader, `toml`), `specs/0032-pen-tablet-input/` (variable-width outline), `specs/0035-combine-and-break-apart/` (Path card), `specs/0049-linked-clone/`, `specs/0057-clone-overrides/`, `specs/0050-layers-and-objects-panel/`, `specs/0041-object-history/`, `specs/0042-history-branches/`, `specs/0024-svg-import-export/`, `specs/0022-color-management/` (tile grid in the panel, stage 2)
ADRs: [`adrs.md`](adrs.md) (no new ADR for stage 1; ADR for the stage 2/3 interfaces named there)
PR: TBD
