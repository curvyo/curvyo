# Pattern fill: a live fill kind that fills a closed shape with generated vector geometry

Status: Done (PR #248)
Priority: Could
Origin: Customer (request of 2026-10-17, translated): "Pattern filling I also find cool. I have to see whether it becomes an extension or a core feature." Scope decisions 2026-11-01/02.

## User value

As a maker I want to give a closed shape a pattern fill (hatching, grid, dots, honeycomb, brick wall, leaves, waves) that stays live while I edit the shape, so that I get an engraved texture or a lightened, perforated cut part, and the machine runs the pattern as real lines and shapes.

**What other tools do.**
- **Illustrator and Affinity**: pattern swatches fill a shape as a tile (live, rasterised on screen; Expand makes vectors).
- **Inkscape**: the SVG Pattern fill (live, but a raster-like tile; "Pattern to Objects" expands), extensions Render > Grid, Hatch, Voronoi, and Interpolate (blend between two paths).
- **LightBurn**: Fill Mode engraves a hatch with line interval, angle, crosshatch; it is a machine setting, not geometry.
- **Glowforge**: images only.

**What we do better.** The pattern is a live fill kind with real vector geometry on every output (SVG, laser and plotter jobs, PNG), clipped to the region with holes and margin respected, and "Convert to paths" makes it ordinary paths in one undo step.

## Words

- **Pattern fill:** a fill kind of a closed shape, next to None and Colour. It holds a generator, its parameters and a seed.
- **Generator:** a pure function (region outline, parameters, seed) to paths.
- **Region:** the closed shape (with holes, inset by the margin) the pattern is clipped to.
- **Generated geometry:** derived from the shape and the parameters; not stored as user paths; regenerated when either changes.

## Acceptance criteria

Placement
1. Given a closed shape selected, when the maker opens Properties > Style > Stroke & Fill, then the fill kind has the choices None, Colour, Pattern. Choosing Pattern shows a generator picker and that generator's parameters in the same section. There is no rail tool and no top bar control for patterns.
2. Given Pattern is chosen, when the maker changes a parameter by dragging or typing, then the canvas shows the regenerated pattern live while adjusting; releasing the drag or committing the typed value is one undo step; Escape while dragging restores the previous value.
3. Given a shape with a pattern, when the maker switches the fill kind to None or Colour, then the pattern disappears; undo brings it back with its parameters.

Generators (first set; interpretations confirmed by the customer, PAT-NEW A, 2026-11-02)
4. Given the generator picker, then it lists exactly: Hatch, Crosshatch, Grid, Dots, Honeycomb, Brick wall, Leaf forest, Sea waves, Waves, Interpolation waves.
5. Hatch: parallel lines with spacing and angle. Crosshatch: Hatch at two angles. Grid: two perpendicular line sets with spacing and angle.
6. Dots: round dots of a given diameter on a staggered (every second row offset by half the spacing) lattice, with spacing.
7. Honeycomb: regular hexagons with cell size and wall width.
8. Brick wall (running bond): brick width, brick height, joint width, each row offset by half a brick. Given width 20 mm, height 10 mm, joint 1 mm, then row n+1 is shifted by 10 mm against row n.
9. Leaf forest: small leaf shapes scattered with parameters leaf size, size jitter, rotation jitter, density and seed; no two leaves overlap (tolerance: geometry kernel tolerance).
10. Sea waves (Seigaiha): concentric half-circle arcs in rows, each row offset by half a scale width; parameters scale radius, ring count, ring spacing.
11. Waves: parallel sine lines with wavelength, amplitude, line spacing and angle. Amplitude 0 gives straight parallel lines equal to Hatch with the same spacing and angle.
12. Interpolation waves: lines that blend from one curve to another, with a line count. Default curves are the region's top and bottom boundary (its bounding-box extreme edges of the outline: the topmost and bottommost boundary run between the leftmost and rightmost points); the first line follows the top curve, the last the bottom curve, the lines in between interpolate point by point. Choosing other curves is out of scope (Proposal: later slice).

Clipping, margin, holes
13. Given a region with holes (a compound path), then no generated geometry lies inside a hole.
14. Given a margin m, then no generated geometry lies closer than m to the outline or to a hole edge, within the kernel tolerance; the outline itself stays.
15. Given any generator, then all generated geometry lies within the region (minus margin); lines and shapes that cross the boundary are cut at it, not drawn over it.

Lines vs Shapes
16. Given a switch Lines / Shapes in the Pattern section, then Lines gives open paths with the shape's stroke (for engraving or plotting) and Shapes gives closed outlines with the fill (for cutting a perforation). Generators that are naturally closed (Dots, Honeycomb cells, Leaf forest, Brick wall) produce closed outlines in both modes; in Lines mode they are stroked, in Shapes mode they are filled. Each straight hatch or grid line is its own straight path; lines that cross are not joined into a zig-zag. (Proposal) In Shapes mode, overlapping strips are merged so a crossing is cut once.

Determinism
17. Given the same shape, parameters and seed, then the generated geometry is identical across runs, sessions and machines (byte-equal in the SVG export). Changing the seed changes seeded generators (Leaf forest) and has no effect on unseeded ones. A new pattern fill gets a fresh seed that is stored, so saving and reopening does not reshuffle.

Count limits (permit over refuse; limits to be measured, Proposal)
18. Given a pattern that would produce more than 50,000 pieces, then it is generated and a quiet note shows the count in the Pattern section. Above 500,000 pieces it is not generated, the shape keeps its previous state, and the section shows the reason and the count. Both limits are measured, not estimated: generation CPU time (thread CPU time, not wall time) on a 100 mm square stays far below one second up to 500,000 pieces (plan.md, measurement table: for example 451,248 dots in 88 ms), so 50,000 (note) and 500,000 (refusal) stand. The refusal also uses a measured cost estimate computed before generating, so no allowed setting freezes the UI; measured values: see plan.md.

Live model
19. Given a shape with a pattern fill, when the maker edits the shape (moves a node, resizes, adds a hole), then the generated geometry is regenerated and matches the new outline without any extra step.
20. Given a shape with a pattern fill, when the maker copies and pastes (or duplicates) it, then the copy has the same pattern fill and parameters and the same seed; editing the copy does not change the original.
21. Given a clone of a shape with a pattern fill (once clones exist, see `0029`), then the clone shows the pattern of its source and follows source edits. If clones do not exist yet, this criterion is not tested and tracked as a debt.
22. Given any edit to a pattern fill or its shape, when the maker presses undo, then the previous state returns exactly (parameters and geometry); redo reapplies it.
23. Given an open shape, then the fill kind Pattern is offered, but nothing is generated (same as Colour on an open path); no error.

Convert to paths
24. Given a shape with a pattern fill, when the maker presses the Convert to paths button in the Pattern section of Stroke & Fill (the only entry point; Curvyo has no Object menu and no object context menu), then the generated geometry becomes ordinary paths in a group placed with the shape (the shape stays with its fill kind set to None), selected after the command; one undo step named "Convert pattern to paths"; undo restores the live pattern.
25. Given the converted paths, then they are identical in geometry to what the live pattern exported before conversion.

Output
26. Given a shape with a pattern fill, then the SVG export, the PNG export and laser and plotter jobs contain the generated geometry as real vector paths (PNG: rendered from them); a reader opening the exported SVG in Inkscape sees paths, no `pattern` element and no reference back to the generator.
27. Given the generated geometry in a laser job, then it takes the operation assigned to the shape's layer or colour like any other path (Lines goes to a line or fill operation by the stroke colour; Shapes by the fill colour); no machine-specific fill is invented here.

Whole pieces only (customer, 2026-11-02)
29. Given a shape-based generator (Dots, Honeycomb, Brick wall, Leaf forest, Sea waves), when the maker turns on the switch "Whole pieces only" (default off) in the Pattern section, then every piece the region boundary, the margin inset or a hole would cut (or that touches them, or holds a hole or island inside it) is left out instead of clipped; a Sea waves scale is kept or left out with all its rings. The pieces that stay are unchanged. The switch is stored with the pattern, works in Lines and Shapes mode, is deterministic and within the same work limits (criterion 18), is one undo step "Pattern: whole pieces", and the SVG export and Convert to paths use the same pieces. It is hidden for the line generators (Hatch, Crosshatch, Grid, Waves, Interpolation waves): their pieces are lines across the region, and Shapes mode turns them into strips, not closed cells (Proposal).

Slice
28. Given the demo, then all of the above works end to end in one PR: the fill kind, the ten generators, Lines/Shapes, margin, seed, limits, Convert to paths, and the SVG export.

## Core or extension (architect's notes)

- Generators are pure functions over a region: (outline, parameters, seed) in, paths out. This is the contribution contract of `0065`; generators are written core-side first with a parameter schema so they can move to extensions later (Proposal; the architect decides in the ADR). `PAT-HOME` of 2026-10-17 (Honeycomb and more as extensions) is superseded by the customer's PAT-SET: all ten are in the first set.
- Hatch for a machine fill is a machine setting (LightBurn Fill Mode, `0063`), not geometry. This entry makes geometry. They must not be mixed.
- Clipping needs the intersection of `0016`/`0067` at the kernel tolerance.
- The live model is a new fill kind on the document model; the ADR and the `format_version` change are the architect's.

## Out of scope

- Motif (a user shape repeated): `0069` tiling clipped to the region; the maker does the two steps (PAT-MOTIF A). Tiling `0069` is being built separately.
- A tile library or store; Penrose, Truchet and other tilings.
- Choosing other curves for Interpolation waves than the top and bottom boundary.
- Gradient fills; SVG `pattern` element output.
- Machine fill / scan lines (`0063`).
- Pattern along a path (`0033`).
- A rail tool or top bar for patterns.
- Voronoi, "Join ends" zig-zag for plotters (Proposal for a later slice).
- Editing individual generated pieces without Convert to paths.

## Decided (customer, 2026-10-17, 2026-11-01/02)

- **PAT-LIVE: B.** Live pattern: Pattern is a fill kind of a closed shape, regenerated when shape or parameters change; outputs get real vector geometry clipped to the region (holes, margin); "Convert to paths" in one undo step. Architect writes the ADR / format change.
- **PAT-PLACE:** right Properties panel, Style tab, Stroke & Fill: fill kind Pattern, generator picker, parameters, live preview while adjusting; no rail tool, no top bar.
- **PAT-SET: A plus new generators.** Hatch, Crosshatch, Grid, Dots, Honeycomb, Brick wall, Leaf forest, Sea waves, Waves, Interpolation waves.
- **PAT-MOTIF: A.** Motif is not in this slice.
- **PAT-SLICE: B.** Everything in one PR.
- **PAT-HOME (2026-10-17), superseded by PAT-SET** for the generators in this slice.

## Decided questions

- **PAT-NEW: A (customer, 2026-11-02).** The interpretations of criteria 8 to 12 stand as written.
  Interpolation waves behave like Inkscape's Interpolate extension: intermediate curves blending
  step by step between two curves (here the region's top and bottom boundary).
- **PAT-STORE: A (customer, 2026-11-02).** See `adrs.md` and ADR 0029.

## Links

Requirements: R-EDIT-046 (new)
Depends on: `0016-boolean-operations`, `0038-path-offset`, `0067-clipping` (or Intersect)
Related: `0033-stroke-brushes`, `0065-extensions-sdk`, `0063-manufacturing-workbenches`, `0069-tiling`, `0029` (clones)
ADRs: to come (architect: live fill kind, format_version change)
PR: -
