# Grid: a document grid on the canvas and a grid target for snapping

Status: In review (PR #238, 2026-11-01; amendment of 2026-11-01 in progress)
Amendment: Customer feedback on PR #238 (2026-11-01, translated): "Origin X and Y and Reset to page corner are not needed, but you could choose one of the four corners. For a laser cutter the origin is bottom left. Can the ruler follow it, also per page, so that per page 0/0 can be bottom left? Name the entry Grid and Scale/Ruler, whatever is best English." Criteria 16 to 19 below; they replace the origin offset of criteria 1 and 3.
Priority: Should
Origin: Customer (request of 2026-10-31, translated): "We need a grid: a new element that is operated like snapping. Shift+click switches it on and off. You can set how the grid behaves: the spacing, whether it zooms and finer cells appear, or whether it is rigid. The grid can be rotated and switched to logarithmic. The default is a 5 mm (at 100 %) linear grid that zooms along, so at 500 % it has 1 mm cells. Whatever makes sense. Snapping gets an extra switch for the grid, where you then state which part snaps to the grid. Default is the centre point."

## User value

As a maker I want a visible grid behind my drawing that I can switch on and off in one click, so that I can lay out parts by eye in regular steps, and I want moved or drawn things to land on that grid, so that parts sit on round measures without typing numbers. Zoomed out I see coarse cells, zoomed in fine ones, so the grid stays readable at every zoom; for work on one fixed raster I can freeze it. The grid belongs to a page, and the output of a job is a page: a grid that a laser cutter or cutting plotter has engraved into its own bed can be repeated on the page, so I place parts on the page exactly where they will land on the machine. A rotated grid helps with parts laid out at an angle, a logarithmic one with scales and charts.

## Terms

- **Base spacing**: the cell size in document length at 100 % zoom (96 px per inch, as everywhere in Curvyo). Default 5 mm.
- **Subdivision**: the factor by which cells split when zooming in (and merge when zooming out). Default 5.
- **Adaptive** grid: the shown cell size follows the zoom. **Fixed** grid: the shown cell size is always the base spacing.
- **Major line**: every `subdivision`-th line, drawn stronger than the minor lines.
- **Grid frame**: the grid's origin (the page's origin corner, criterion 16) and angle; grid coordinates are measured in it.
- **Page grid**: the grid settings of one page; every page has its own.

## Acceptance criteria

### Document settings

1. Given a page (`0059`), then the **grid is a property of that page** (customer, GRID-HOME and GRID-ORIGIN, 2026-10-31): shown on/off, mode (adaptive/fixed), scale (linear/logarithmic), base spacing, subdivision, angle and origin offset are stored with the page and travel with the file. The origin offset is measured from the page's top-left corner, so the grid moves with its page when the page moves, and a page resize keeps the grid where it is relative to that corner. A new document's page has: grid hidden, adaptive, linear, base spacing 5 mm, subdivision 5, angle 0°, offset 0, 0. A page added with Add page takes the grid settings of the page it copies its size from (customer, GRID-NEWPAGE, 2026-10-31). Opening a file whose pages have no grid registers uses these defaults and does not rewrite the file; a malformed register is refused as `OpenError::Damaged`. `format_version` goes up by one.
2. Given any tool and selection, then the right Properties rail has a dedicated **Grid** entry (icon, tooltip, accessible name), next to Snapping and behaving the same: a normal click opens the Grid panel without changing the shown switch; **Shift+click toggles the grid shown/hidden** as one history step labelled like the other document settings; the entry is subtly blue while the grid is shown. Entry, Shift+click and panel act on the **active page's** grid (customer, GRID-SCOPE, 2026-10-31); the panel names that page in its header. The panel has its own keyboard-accessible shown switch.
3. Given the Grid panel, then it holds: shown switch; mode (Adaptive / Fixed); scale (Linear / Logarithmic); base spacing (display unit, `0015`); subdivision (integer, at least 2); angle (degrees); origin offset X and Y from the page's top-left corner (display unit) with a Reset to page corner action. Every control works with the grid hidden. A non-finite value, a spacing of zero or less, or a subdivision below 2 is refused with a notice of at most 70 characters; the angle is taken modulo 360 (permit over refuse). Each change is one history step.

### Drawing the grid

4. Given a page with its grid shown, then the grid is drawn over that page's background and under every object, **inside that page's rectangle only** (customer, GRID-CLIP, 2026-10-31), each page with its own settings, in the design system's grid colour, thin and at constant screen width, major lines stronger than minor lines. It is a view aid only: never printed, exported, sent to a machine or picked by the eyedropper, and never hit by selection.
5. Given an **adaptive linear** grid at zoom `z` (1.0 = 100 %), then the shown minor spacing is `base × subdivision^(−k)` with `k = floor(log_subdivision(z))` (customer, GRID-ZOOM, 2026-10-31): with the defaults 25 mm at 20 %, 5 mm at 100 %, 1 mm at 500 %, 0.2 mm at 2500 %. Between two levels the next finer lines fade in as their screen spacing grows (Proposal: from 0 to full opacity between 4 px and 12 px screen spacing), so a zoom never makes the grid jump. A line of a coarser level stays on the same document coordinate at every zoom (levels nest).
6. Given a **fixed linear** grid, then the minor spacing is the base spacing at every zoom. As its screen spacing falls from 12 px to 4 px the minor lines fade out and below 4 px they are left out; the major lines do the same (Proposal, no moiré, no popping, bounded cost).
7. Given an angle `a`, then the grid's lines are rotated by `a` (clockwise, 0° = axis aligned, as in `0072`) around the page grid's origin; spacing, levels and fading are unchanged.
8. Given a **logarithmic** grid, then (customer, GRID-LOG, 2026-10-31) each axis is laid out like log paper: one decade is `base × subdivision` long, starting at the page grid's origin; inside each decade lines sit at `log10(n)` of the decade for n = 1…10, the decade line (n = 1) is a major line. Decades repeat in both directions. In adaptive mode, zooming in so that a cell exceeds 48 px screen size adds the next log subdivision (n = 1.1, 1.2 … inside that cell); in fixed mode the lines stay. Decades repeat (not mirror) on both sides of the origin. In both modes, log lines closer than 4 px on screen to a drawn neighbour are left out (Proposal, bounded cost). Rotation applies as in criterion 7.
9. Given 10,000 objects and any zoom, then drawing the grid costs at most 1 ms of thread CPU per frame on the reference machine; the number of drawn lines is bounded by the screen size, never by the document size, the number of pages or the zoom.

### Snapping to the grid

10. Given the Snapping panel (`0060`), then it gets a fifth target switch **Grid** next to Objects, Guides, Nodes and Page, stored like the other target switches. A new document has it **on** (Proposal, GRID-SNAP-DEFAULT); it pulls only where a grid is shown (customer, GRID-SNAP-HIDDEN, 2026-10-31).
11. Given the Grid target, then a **grid snap point** setting chooses which part of the moving thing snaps to the grid (one choice, stored in the document with the other snapping settings; customer, GRID-SNAP-POINT, 2026-10-31): **Centre** (default: the centre of the moving selection's box), **Box corners and edges** (the nine reference points of the box, nearest wins), **Nodes** (the moving anchors of a path; for a moved selection its nodes; an object without nodes, such as a text or image, snaps by its box centre, Proposal). For a resize the dragged handle's reference point snaps; for the Node tool the dragged anchor snaps; for Pen and shape creation the placed point snaps, whatever the choice (Proposal: creation and resize always snap the point under the pointer).
12. Given grid snapping and a moving point, then the targets are the **intersections** and the **lines** of the finest level that is drawn at full opacity at the current zoom (what you see is what you snap; lines still fading in do not pull; Proposal, GRID-SNAP-LEVEL), of the shown page grid whose page contains the moving point; where pages overlap, the page drawn on top decides; on the pasteboard and on pages with a hidden grid there is no grid target (customer, GRID-CLIP, 2026-10-31). Capture and release zones, hysteresis, Alt inversion, priority by distance and the marker are those of `0060`. On an exact tie, every other target kind wins over the grid.
13. Given a rotated grid, then the snap is resolved in grid coordinates: an intersection snaps both grid axes, a line snaps the coordinate across it. The X and Y switches of `0060` refer to the grid's own axes for grid targets (Proposal, GRID-SNAP-AXES); for an angle of 0° they are the document axes.
14. Given a logarithmic grid, then its intersections and lines are the targets exactly as drawn.
15. Given a snap to the grid, then the result lies exactly on the grid coordinate (to the kernel tolerance) and is one history step as before; grid targets are computed from the settings and the view, never stored, never indexed per object, and add no document read per frame.

### Origin corner and rulers (amendment of 2026-11-01)

16. Given a page, then it has an **origin corner**: Top left (default), Top right, Bottom left or Bottom right, stored with the page (one register, replacing the grid's origin offset X/Y and Reset to page corner, which are removed). Add page copies it with the grid (GRID-NEWPAGE). A malformed value is refused as `OpenError::Damaged`; a missing one is Top left.
17. Given an origin corner, then the page's **grid** starts at that corner (its first lines run along the two page edges meeting there; rotation and logarithmic decades start there), and the **rulers**, while that page is active, show 0 at that corner and count **positive into the page** (customer, ORIGIN-AXES, 2026-11-01): Bottom left counts Y upwards, the right corners count X to the left. Switching the active page switches the rulers to that page's corner in the same frame.
18. Given the rail entry and panel, then they are named **Grid & Rulers** (tooltip "Grid & Rulers — on/off", accessible name "Grid and rulers") (Proposal, wording). The panel shows the origin corner as four corner buttons (a small page pictogram with the four corners, one selected) under the page name; the corner works whether or not the grid is shown. Shift+click on the entry still only shows or hides the grid. Each corner change is one history step ("Change grid").
19. Given the origin corner, then the **rulers, the grid and the typed positions** follow it (customer, ORIGIN-FIELDS, 2026-11-01): the Absolute X and Y of the typed move (`0047`, for every reference point of the reference box, typed or picked with a mark click) show and take the chosen point's position measured from the active page's origin corner, positive into the page, as the rulers count. Relative X and Y keep the document directions (right and down positive). The `0072` Pen entry shows no absolute position (its length is relative and its angle keeps the `0072` convention), so it is unchanged. The status bar's cursor readout measures the same way as the rulers (Proposal).

## Out of scope

- Separate spacings for X and Y, non-square cells, isometric/triangular/polar grids (own entry if wanted).
- A grid per layer, a grid on the pasteboard; a grid colour or opacity setting (design-system colour).
- Reading a machine's bed grid from the machine or a profile (`0063`, `0064`); here the maker types spacing and angle and picks the origin corner.
- Showing the grid in exports, prints or machine jobs.
- Keyboard nudge by grid steps (`0044` keeps 1 mm / 10 mm).
- Snapping rotation to the grid angle.
- Old-file compatibility (customer, 2026-10-30): missing registers resolve to the defaults.

## Decisions (customer, 2026-10-31)

- GRID-HOME and GRID-ORIGIN: neither document nor user: the grid hangs on the page (B for the origin). Reason: matching a grid the machine has engraved into its bed; the job output is a page.
- GRID-ZOOM: A (refine by the subdivision factor, finer lines fade in).
- GRID-LOG: A (log paper on both axes, repeating decades).
- GRID-SNAP-HIDDEN: A (only a shown grid pulls).
- GRID-SNAP-POINT: A (one choice: Centre / Box corners and edges / Nodes).
- GRID-CLIP: A (drawn and snapped to only inside its page; no grid on the pasteboard).
- GRID-SCOPE: A (entry, Shift+click and panel change the active page's grid only).
- GRID-NEWPAGE: A (a new page takes the grid of the page it copies its size from).

## Decisions (customer, 2026-11-01)

- ORIGIN-AXES: A (the rulers count positive into the page: Y up from a bottom corner, X to the left from a right corner).
- ORIGIN-FIELDS: B (the typed positions follow the origin corner in this PR, criterion 19).

## Links

Requirements: R-EDIT-049 (new), R-EDIT-039 (snapping)
Depends on: `specs/0059-multiple-pages/` (the grid is stored with the page), `specs/0060-snapping/` (rail entry, panel, resolver, marker), `specs/0015-document-size-and-rulers/` (display unit)
Related: `specs/0063-manufacturing-workbenches/` (job output is a page), `specs/0004-canvas-navigation-and-selection/` (zoom), `specs/0072-pen-angle-snap-and-typed-values/` (angle convention)
Plan: `plan.md`
