# Pattern fill: fill a closed shape with a generated or repeating pattern as real vector geometry

Status: Draft (a sketch for the customer; not Ready: it needs the clip of `0067` ; core versus extension decided 2026-10-17)
Priority: Could
Origin: Customer (request of 2026-10-17, translated): "Pattern filling I also find cool. I have to see whether it becomes an extension or a core feature."

## User value

As a maker I want to fill a closed shape with hatching, a grid, dots, a honeycomb or my own motif, so that I get an engraved texture or a lightened, perforated cut part, with the pattern as real lines and shapes that the machine can run.

**What other tools do.**
- **Illustrator and Affinity**: pattern swatches fill a shape as a pattern tile (live, rasterised on screen; Expand makes vectors).
- **Inkscape**: the Pattern fill (an SVG pattern; "Object to Pattern", "Pattern to Objects"), and the extensions Render > Grid, Hatch, Voronoi pattern.
- **LightBurn**: Fill Mode engraves a hatch inside a shape with a line interval, angle, crosshatch and a bidirectional scan; Offset Fill; Fill is a machine setting, not geometry. Cricut and Silhouette: pattern fills (print only); Silhouette has "hatch fill" for a pen.
- **Glowforge**: images only.

## Words

- **Pattern:** a rule that generates geometry inside a region: a tile (repeat along two vectors) or a generator (hatch, honeycomb, dots, Voronoi).
- **Region:** the closed shape; the pattern is clipped to it (the idea of `0067`).
- **Result:** ordinary paths, clipped to the region (a compound path or a group), no longer connected to the pattern unless Frage 1 says so.

## Acceptance criteria (as far as clear)

1. Given a closed shape selected and the **Pattern fill tool** (a symbol in the tool rail with a list of generators in its top bar), then the generator and its options (spacing, angle, line width or dot size, margin from the outline) are in the top bar; nothing is added to the right panel.
2. Given the generators, then the first set is: **Hatch** (parallel lines, angle, spacing; Crosshatch is two angles), **Grid**, **Dots** (round, size, spacing, staggered), **Honeycomb** (hexagons, cell size, wall width), **Motif** (the selected object repeated, which is `0069` tiling clipped to the region).
3. Given the preview, then it is live only while adjusting; Enter commits; Escape cancels.
4. Given the region has holes (a compound path), then the pattern respects the holes.
5. Given lines or walls with a width, then the result is closed outlines (for cutting a perforation) or open lines (for engraving), set by a switch in the bar: **Lines** or **Shapes**. Open lines keep the stroke of the region; shapes take the fill.
6. Given the margin, then the pattern keeps a given distance from the outline (an inset by `0038`, then the pattern clips to it); the outline stays.
7. Given a count limit, then a pattern with more than 50,000 pieces is allowed with a quiet note in the bar; above 500,000 it is refused with the reason (permit over refuse; to be measured).
8. Given determinism, then the same inputs always give the same result; a random option (Voronoi, scattered dots) has a seed as in `0069`.
9. Given hatch for engraving, then lines are one connected zig-zag per hatch row only if a switch "Join ends" is on (a plotter optimisation, Proposal); the default is separate lines.
10. Given the result is a group, then it is selected after the commit and is one undo step "Pattern fill".
11. Given the maker wants the pattern to change when the shape changes, then that is a parametric fill (Frage 1); it is not in the first version.

## Core or extension (architect's notes)

- Generators are **pure functions over a region**: (outline, parameters, seed) in, paths out. That is exactly the shape of `0065`'s contribution contract (a parameter schema plus a function). Hatch, grid and dots are small (under 150 lines each); Honeycomb and Voronoi are medium.
- Core would have to carry the options rendering for every generator itself; an extension gets it from the schema. If the contract exists first, this entry is a good second extension; if not, a core Hatch alone is cheap and has value for the laser (a fill of a closed shape is also what the laser workbench needs for engraving, `R-MFG-LASER-001`).
- Hatch for a machine fill is probably a machine setting (LightBurn's Fill Mode), not geometry; so there are two things: **geometry patterns** (this entry, design) and **machine fill** (`0063`, a toolpath). They must not be mixed; this entry makes geometry.
- Clipping the result needs `0067`'s intersection at the kernel tolerance, which already exists in `0016` as a function.

## Out of scope

- A tile library with a store; Penrose, Truchet and other tilings (extensions).
- Pattern as a live fill that stays on the shape (SVG `pattern` model); gradient fills.
- Machine fill / scan lines (`0063`).
- Pattern along a path (`0033`).

## Decided (customer, 2026-10-17)

- **PAT-HOME:** the default. Hatch, Grid and Dots are core; Honeycomb, Voronoi and further generators are extensions once `0065` has a route.

## Open questions (customer; each has a default)

Frage 1 (PAT-LIVE): One-time result or a live pattern?
- A (default): one-time; the result is plain paths. B: a pattern node that re-generates when the shape changes (a new node kind; shares the question of `0069` TILE-LIVE and `0066`).
- Default if unanswered: A.

Frage 2 (PAT-MOTIF): Is "Motif" (a user shape repeated) in the first set?
- A (default): no, it is `0069` + `0067`; the maker does the two steps. B: yes, as a one-step convenience.
- Default if unanswered: A.

## Links

Requirements: R-EDIT-046 (new)
Depends on: `0016-boolean-operations`, `0038-path-offset`, `0067-clipping` (or Intersect), `0069-tiling` (Motif)
Related: `0033-stroke-brushes`, `0065-extensions-sdk`, `0063-manufacturing-workbenches`
ADRs: none while Frage 1 is A
PR: -
