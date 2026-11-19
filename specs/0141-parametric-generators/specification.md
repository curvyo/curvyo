# Parametric generators: laser boxes with finger joints and gears, as live objects with typed parameters

Status: Draft (a sketch; decisions of 2026-11-19 applied; not Ready: `adrs.md` (the extension contract's second extension, the shared instance mechanism with `0099`) and UX notes are missing)
Priority: Could
Origin: Customer (2026-11-19, translated): generators for laser boxes (finger joints etc.) and gears (module, tooth count, diameter, solid, spokes, cut-outs, centre bore, keyway ...). Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As a maker I want to type "box 120 x 80 x 60 mm, 3 mm plywood, finger joints" or "gear module 2, 24 teeth, 6 mm bore, keyway" and get cut-ready outlines that I can change later by changing the numbers, so that I do not draw involute teeth or finger joints by hand.

**What other tools do.** MakerCase and BoxesPy (web generators, one-shot SVG download); Inkscape: Extensions > Render > Gear and "Tabbed Box Maker" (a one-shot dialog, outlines are plain paths afterwards); LightBurn: no gear or box generator. We do better: the generator result is a live object kept in the document, editable in the Properties panel, with Convert to paths when the maker wants plain outlines.

## Relation to existing and neighbouring specs

- **`0065` extensions SDK:** its decisions name "a small generator" (Honeycomb, Voronoi) as the likely second extension and the real test of the contract. **(Frage 277 A, decided):** box and gear are the first built-in generators **through the extension contract** (parameter schema, declarative blocks, no TSX: Frage 119 A of `0065`), written core-side as pure functions (parameters, seed) in, paths out (the architect notes of `0071`); no generator code in the shell.
- **`0071` pattern fill:** the same pure-function shape (outline, parameters, seed) to paths, as a live fill kind. A generator is the same without an outline: parameters to paths.
- **Sketch instances (`0099`, revision of 2026-11-19, criterion 23 ff.):** a parametric sketch placed on a page with handles. A generator is the **second kind of parametric instance**: same placement, same parameter fields, same handles where a parameter is a distance, same Convert to paths; the first slice shares the one instance mechanism the architect designs (Frage 278 A).
- **`0028`/`0112` materials:** a box's material thickness is a typed value in this slice; filling it from a material record is a Proposal for later.

## Acceptance criteria (as far as clear)

1. Given the Generators entry in the toolbox card of the create tools (Proposal: `More tools`), then Box and Gear are offered; pressing one places a generated object at the pointer with its defaults, selected, with its parameters in the Properties panel (not a dialog), one undo step "Add gear".
2. Given a **gear**, then its parameters are: Module (mm), Teeth (integer, 6 to 200), Pressure angle (degrees, default 20), Pitch diameter (shown, derived from module and teeth; typing it changes the module), Centre bore (diameter, 0 = none), Keyway (width and depth, off), Spokes (count 0 to 12, rim width, hub diameter), Cut-outs between spokes as rounded windows, and Solid (no spokes). The outline is an involute tooth profile within 0.01 mm of the exact involute, closed; the output is one compound path with holes (`0035`).
3. Given a **box**, then its parameters are: Inner width, Depth, Height (mm), Material thickness (mm), Joint (finger, or none), Finger width (mm, default 2 times the thickness **(Proposal)**), Kerf compensation (mm, default 0; `0028` left kerf out, so it is only a number here), Lid (none, flat, with fingers), Layout (six flat plates side by side, with a gap). The plates are separate closed outlines in one group; the fingers of two plates that meet fit with the joint clearance (mm, default 0): a test mates two plates at the given thickness and asserts the engaged edges lie on one line within the kerf value.
4. Given any generated object, then changing a parameter regenerates its paths within 100 ms of thread CPU time for a gear of 200 teeth and a box with 100 fingers per edge; invalid values (negative thickness, teeth below 6) are clamped with a short notice (permit over refuse), never an error dialog.
5. Given a generated object, then it moves, rotates, scales and is aligned like any object; scaling changes its size parameters, not a hidden transform **(Proposal)**, so the numbers stay true; operations of `0101` assign to it like to any item.
6. Given Convert to paths, then the object becomes ordinary paths in one undo step and its parameters are gone.
7. Given save and reopen, then the generator kind and its parameters survive and the paths are recomputed; a file with a generator kind this build does not know opens with the last paths as plain paths and a notice **(Proposal)**; `format_version` change by the architect.
8. Given the first build, then at least the box and the gear of criteria 2 and 3 exist; other generators (hinges, living hinges, spirals, hole patterns, honeycomb) are later entries through the same contract.

## Out of scope

- A user-written generator, scripting, a generator marketplace (`0065` no marketplace).
- Assembly drawings or 3D preview of the box; sprocket and rack generators; helical gears; gear train layouts with meshing check.
- Material databases for joint clearance, automatic kerf from the laser.

## Decisions

Customer, 2026-11-19:

- **Frage 277 (GEN-HOME): A.** Built-in extensions through the `0065` contract (the second real extension); the shell stays free of generator code.
- **Frage 278 (GEN-OBJECT): A.** A live parametric object (kept parameters, Convert to paths) sharing one instance mechanism with sketch instances (`0099` revision, criteria 23 to 31).

## Still open before Ready

- `adrs.md` (architect: the shared instance mechanism, the generator contract, `format_version`).
- UX notes (parameters in the Properties panel, the Generators entry).

## UX notes

(ux-engineer, before Ready.)

## Links

Requirements: R-EDIT-055 (new), R-EXT-004, R-EXT-005
Depends on: `0065` (contract), `0099` revision (the instance mechanism, if Frage 278 A)
Related: `0066`, `0071`, `0035`, `0101`
PR: none yet
