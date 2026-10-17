# Tiling: grid and circular arrays of an object, with variance and a seeded randomizer

Status: Draft (a sketch for the customer; not Ready: it depends on linked clones and on the Select bar rework)
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "I need a tool/workflow similar to the tiling in Inkscape or LightBurn: tiled clones with variance control for rotation, scaling etc. with a randomizer. Tiling should also be possible in a circle like in LightBurn."

## User value

As a maker I want to fill a sheet with a shape (50 hearts on a coaster, 12 petals around a centre), and let each copy vary a little in rotation, scale or position, so that a pattern looks hand-made, and so that I can change the original once and have all copies follow.

**What other tools do.**
- **Inkscape: Create Tiled Clones** (a dialog with tabs Symmetry, Shift, Scale, Rotation, Blur, Opacity, Color; rows x columns, per-row/column/random variance in percent, a Randomize button that is not repeatable, and the Trace tab; produces clones; problems: a big dialog, no live preview, random values cannot be recovered).
- **LightBurn: Grid Array** (columns, rows, spacing, mirror odd rows or columns, auto-nesting) and **Circular Array** (count, centre, radius, angle step, rotation of each copy); creates **copies** not clones; a preview while adjusting; random variance is not offered.
- **Illustrator**: Pattern Options, Repeat on a path with live editing; Affinity has Repeat on arrays.

## Words

- **Source:** the selected object(s) the tiling repeats; unchanged.
- **Grid tiling** and **circle tiling:** the two layouts. A circle tiling places N instances at equal angles on a circle (or arc) about a centre.
- **Variance:** a range around the layout value for each instance (rotation plus or minus a number of degrees, scale plus or minus a percent, position plus or minus a length, optionally opacity and style colour of `0057`).
- **Seed:** an integer; with the same seed, source and options the result is exactly the same.

## Acceptance criteria (as far as clear)

1. Given a selection, then a **Tiling tool** (symbol in the tool rail, a card of its own with `Grid` and `Circle` as two modes) shows all options in the top bar: columns, rows, column and row spacing (or total size), mirror every second row/column (switches), for circles count, radius, start angle, sweep, and whether each instance turns to face the centre.
2. Given the options, then the result is drawn live while the maker adjusts a number or drags a handle on the canvas; the live preview exists only while adjusting. A preview is never stored; releasing or pressing Enter commits.
3. Given variance, then each of rotation, scale (uniform or per axis), offset x/y and opacity has a "random +/-" number field in the top bar (collapsed behind a single symbol button in the bar to keep the bar short; the button stays visible and states its state in the tooltip); its default is 0.
4. Given the **Seed**, then a number field and a dice button: the dice picks a new seed and shows it; the same seed gives the same layout on every platform and every run (a documented portable generator, for example PCG with a fixed algorithm, not the platform's `rand`).
5. Given **Result as**, then a switch in the bar: **clones** (default; linked to the source by `0049`, each clone placement relative to the source) or **copies** (independent paths). Variance with clones uses the clone's own matrix (own placement, `0057`); a source change follows all clones.
6. Given the colour variance and clones, then each clone may receive its own fill from the palette (random or stepped) with the mechanism of `0057` "Spread colours"; with copies the style is written to each path.
7. Given the source is part of the grid, then the first cell is the source itself by default; a switch "Keep source in place" turns the tiling into all new instances.
8. Given a count limit, then a tiling over 10,000 instances is permitted with a quiet warning in the bar, and refused only above 100,000 (permit over refuse; the limit is a Proposal and is measured at build).
9. Given a committed tiling of clones, then the group of clones is one object with a handle to re-open the tiling options (Proposal); with copies there is no re-opening.
10. Given undo, then the commit is one history step "Tile".
11. Given the maker wants instances to avoid each other or stay inside a shape, then that is not this entry: nesting is `R-MFG-004`; staying inside a shape is `0071` and `0067`.

## Out of scope

- Inkscape's colour, blur and trace-from-drawing tabs; tiling along an arbitrary path (a later entry, with `0033` brushes); tiling in 3D.
- Fill by a non-affine map (warp, envelope).
- Automatic spacing from the kerf; nesting.
- Persisting the tiling as a live parametric object (criterion 9 is a Proposal; without it, clones remain editable one by one).

## Decided (customer, 2026-10-17)

- **TILE-RESULT: linked clones** (criterion 5 default stays; copies remain the switch).
- **TILE-HOME: core.**

## Open questions (customer; each has a default)

Frage 1 (TILE-LIVE): Is a tiling re-openable after commit?
- A (default): no in the first version; a new tiling replaces by undo and redo. B: yes, as a "tiling" node (a new node kind, a `format_version` bump; also the base for 0071 pattern fill).
- Default if unanswered: A.

## Links

Requirements: R-EDIT-044 (new)
Depends on: `0049-linked-clone`, `0057-clone-overrides`
Related: `0071-pattern-fill`, `0033-stroke-brushes`, `0067-clipping`, `0065-extensions-sdk`
ADRs: none while Frage 1 is A; a node kind would need one (`needs-customer`)
PR: -
