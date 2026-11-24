# Parametric generators: laser boxes with finger joints and gears, as live objects with typed parameters

Status: Done (PR #455; revision 2 Draft 2026-11-24 after the customer's try, see Decisions, corner wording per ADR 0082; Ready 2026-11-23; Fragen 277, 278, 326 and 327 answered; `adrs.md`, ADR 0080 (Accepted) and `ux-notes.md` exist)
Priority: Could
Origin: Customer (2026-11-19, translated): generators for laser boxes (finger joints etc.) and gears (module, tooth count, diameter, solid, spokes, cut-outs, centre bore, keyway ...). Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As a maker I want to type "box 120 x 80 x 60 mm, 3 mm plywood, finger joints" or "gear module 2, 24 teeth, 6 mm bore, keyway" and get cut-ready outlines that I can change later by changing the numbers, so that I do not draw involute teeth or finger joints by hand.

**What other tools do.** MakerCase and BoxesPy (web generators, one-shot SVG download); Inkscape: Extensions > Render > Gear and "Tabbed Box Maker" (a one-shot dialog, outlines are plain paths afterwards); LightBurn: no gear or box generator. We do better: the generator result is a live object kept in the document, editable in the Properties panel, with Object to path (Select bar) when the maker wants plain outlines.

## Relation to existing and neighbouring specs

- **`0065` extensions SDK:** its decisions name "a small generator" (Honeycomb, Voronoi) as the likely second extension and the real test of the contract. **(Frage 277 A, decided):** box and gear are the first built-in generators **through the extension contract** (parameter schema, declarative blocks, no TSX: Frage 119 A of `0065`), written core-side as pure functions (parameters, seed) in, paths out (the architect notes of `0071`); no generator code in the shell.
- **`0071` pattern fill:** the same pure-function shape (outline, parameters, seed) to paths, as a live fill kind. A generator is the same without an outline: parameters to paths.
- **Sketch instances (`0099`, revision of 2026-11-19, criterion 23 ff.):** a parametric sketch placed on a page with handles. A generator is the **second kind of parametric instance**: same placement, same parameter fields, same handles where a parameter is a distance, same Convert to paths; the first slice shares the one instance mechanism the architect designs (Frage 278 A).
- **`0028`/`0112` materials:** a box's material thickness is a typed value in this slice; filling it from a material record is a Proposal for later.

## Acceptance criteria (as far as clear)

1. Given the Generators tool in the More tools card (with Offset, Measure and Grid; Frage 327 A), then Box and Gear are offered as segments of the tool's bar; pressing one places a generated object at the pointer with its defaults, selected, with its parameters in the Properties panel (not a dialog), one undo step "Add gear".
2. Given a **gear**, then its parameters are: Module (mm), Teeth (integer, 6 to 200), Pressure angle (degrees, default 20), Pitch diameter (shown, derived from module and teeth; typing it changes the module; revision 2: a handle resize changes Teeth, see criterion 5), Centre bore (diameter, 0 = none), Keyway (width and depth, off), a Web choice, Solid or Spokes (Spokes: count 2 to 12, rim width, hub diameter; the windows between the spokes are always rounded, no field). The outline is an involute tooth profile within 0.01 mm of the exact involute, closed; the output is one compound path with holes (`0035`).
3. Given a **box**, then its parameters are: Inner width, Depth, Height (mm), Material thickness (mm), Finger width (mm, default 2 times the thickness **(Proposal)**), Kerf compensation (mm, default 0; `0028` left kerf out, so it is only a number here), Lid (none, flat, with fingers), Layout (the plates flat side by side, with a Gap field; five plates without a lid). Joint is always finger (revision 2: the Joint choice is gone, customer-decided 2026-11-24). **Revision 2, corners:** with a floor plate the corner fingers must not drop off: **every corner where plates meet has exactly one owner** (ADR 0082 §1): the bottom and the lid own the corner cubes together with the finger next to them, the walls' vertical joints keep front and back as owners of the end pieces, so a plate is never joined through a single point and the plates stay joined at the corner **without raising Kerf compensation**; tests (ADR 0082 §9): every plate outline of the five-plate box at Kerf 0 and 0.2 is a simple polygon (no repeated vertex, no self-intersection) for every lid and piece counts 1 to 9, along each of the 12 edges every piece and corner cube belongs to exactly one plate, and mated corners overlap, not leave a gap. The live box is **one object** (one leaf with one closed outline per plate, not enterable); Convert to paths (criterion 6) gives the **group** of plates; the fingers of two plates that meet fit with the joint clearance (mm, default 0): a test mates two plates at the given thickness and asserts the engaged edges lie on one line within the kerf value.
4. Given any generated object, then changing a parameter regenerates its paths within 100 ms of thread CPU time for a gear of 200 teeth and a box with 100 fingers per edge; invalid values (negative thickness, teeth below 6) and dependent values (bore below the root circle, hub inside the rim, keyway inside the hub) are clamped to the nearest valid value with a short status notice that names the field and the limit (permit over refuse), never an error dialog.
5. Given a generated object, then it moves, rotates and is aligned like any object; it resizes **proportionally only** (corner handles, no skew), and a resize changes its size parameters, not a hidden transform, so the numbers stay true: **gear (revision 2, customer-decided 2026-11-24): the handle changes Teeth in whole numbers (clamped 6 to 200) and never the Module, the tooth size stays; the pitch diameter follows from Teeth; Bore, Keyway and Rim width, Hub diameter keep their values;** box Inner width, depth and height; Pressure angle, Spokes, Thickness, Kerf, Finger width, Clearance and Gap stay, so a box after a resize is near, not exactly, the dragged size; operations of `0101` assign to it like to any item.
6. Given Object to path in the Select bar (revision 2: the Convert to paths button in the Generator block is gone, customer-decided 2026-11-24), then the generated object becomes ordinary paths (a box: the group of plates) in one undo step and its parameters are gone.
7. Given save and reopen (Frage 326 A), then the generator kind, its parameters and its outline survive and the file **opens exactly as stored**; a test asserts that regenerating from the stored numbers gives the same outline, and the outline is rebuilt on the next parameter change. A file with a generator kind this build does not know (or disabled) opens with the stored outline drawn, cut, moved, rotated, copied, deleted and converted to paths, but not resized or changed by number; it is **not** converted on open, a save keeps its record unchanged, and one status notice tells the maker once; `format_version`: next free at merge (ADR 0080).
8. Given the first build, then at least the box and the gear of criteria 2 and 3 exist; other generators (hinges, living hinges, spirals, hole patterns, honeycomb) are later entries through the same contract.

9. **(Revision 2, 2026-11-24)** Given the Generator block in the Style tab, then every numeric field is the usual slider row like Opacity or Width (value field plus slider, min and max from the parameter schema, typing beyond the slider range still allowed up to the schema limit); no plain number-only input remains in the block. **(Proposal)** The same applies to the other schema-drawn numeric fields in the app (extensions of `0065`); the shell renders a schema number with a range as a slider row everywhere.
10. **(Revision 2)** Given the box with a lid-less or lidded layout, then no Joint control exists in the block and old files with `joint: none` or `finger` open as stored (finger).

## Out of scope

- A user-written generator, scripting, a generator marketplace (`0065` no marketplace).
- Assembly drawings or 3D preview of the box; sprocket and rack generators; helical gears; gear train layouts with meshing check.
- Material databases for joint clearance, automatic kerf from the laser.

## Decisions

Customer, 2026-11-24 (after trying 0141; decisions, translated): the gear handle changes the tooth count, not the module, in whole numbers; the Convert to paths button leaves the Generator block (Object to path does it); the box corner with a floor plate must not drop off (push in or no gap at the start) without the maker having to raise kerf; the Joint choice goes (always finger); all Generator block inputs are sliders. A second box type is `0147-rounded-box`.

Customer, 2026-11-19:

- **Frage 277 (GEN-HOME): A.** Built-in extensions through the `0065` contract (the second real extension); the shell stays free of generator code.
- **Frage 278 (GEN-OBJECT): A.** A live parametric object (kept parameters, Convert to paths) sharing one instance mechanism with sketch instances (`0099` revision, criteria 23 to 31).

- **Frage 326 (GEN-STORED): A.** (2026-11-23) A file opens exactly as stored (kept outline, no regeneration on open); ADR 0080 is Accepted by this answer.
- **Frage 327 (GEN-RAIL):** (2026-11-23) The Generators tool goes into the More tools card with Offset, Measure and Grid, simply there for now. The rails must later be reworked to scroll so they fit the screen size; on a small screen everything could be one rail, below each other (`docs/technical-debt.md`).

## Still open before Ready

Nothing. `adrs.md`, ADR 0080 (Accepted 2026-11-23) and `ux-notes.md` exist; the architect's spec issues of `adrs.md` and the UX flags are resolved above. Slice and PR are the implementer's `plan.md`; `format_version` takes the next free number at merge.

## UX notes

See `specs/0141-parametric-generators/ux-notes.md` (the Generators tool and bar in the More tools card, placing at the pointer, the Generator block of the Style tab drawn from the schema, Pitch diameter, clamping with a status notice, instance handles shared with the `0099` revision, Convert to paths, the unknown-kind notice on open, and the questions for the lead; revision 2 in section 15: Value fields with tracks per parameter, no Convert button, no Joint, the gear resize writing Teeth).

## Links

Requirements: R-EDIT-055 (new), R-EXT-004, R-EXT-005
Depends on: `0065` (contract), `0099` revision (the instance mechanism, if Frage 278 A)
Related: `0066`, `0071`, `0035`, `0101`, `0147` (rounded box; the group-like unit with Detach part of criterion 15 there is the pattern for per-part operations of generated objects), `0149` (box from any path)
ADRs: `docs/adr/0080-generated-objects-extension-trait-and-parametric-instances.md` (Accepted)
PR: none yet
