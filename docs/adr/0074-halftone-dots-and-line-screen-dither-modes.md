# ADR 0074: Two screen dither modes, Halftone dots and Line screen: an integer threshold per dot from a rotated cell of the item

**Status:** Proposed (architect, 2026-11-19). No customer decision needed beyond Frage 277 (open, default A: both modes in slice 1). Spec: [`0135`](../../specs/0135-image-optimisation/specification.md) criteria 36, 37, 42 to 44. Extends [ADR 0068](0068-raster-dot-values-and-dithering.md) §5 (modes) and §6 (determinism); the stored keys are [ADR 0072](0072-image-adjustments-on-the-image-object.md) §6.

**In plain words (for the customer).** The picture is laid under a regular grid of cells, turned by the Angle. In each cell a round dot (or a band) burns, bigger where the picture is darker. Whole numbers only, so it is the same on every computer.

## Context

ADR 0068's seven modes decide each dot from its grey alone (Threshold, Grey), from its grid position (Ordered) or from its neighbours (diffusions). A screen needs the dot's position inside a cell that is rotated and larger than one dot, and must not change with the scan angle or the run direction.

## Options

| Option | Why it won or lost |
|---|---|
| **A. A threshold table per cell, indexed by the dot centre's position in the rotated cell grid (chosen)** | One lookup per dot, like Ordered; independent of the scan angle, the pass and the output format; integer. |
| B. Draw real circles and lines as vector shapes and hatch them | Exact shapes, but a second engine beside the dot grid, a different look in the Toolpath view, and millions of shapes per picture. |
| C. A larger Bayer matrix rotated | Ordered dithering gives a dispersed pattern, not clustered round dots; not the newspaper look. |

## Decision

1. **Module** `curvyo-laser-core/src/job_raster_screen.rs` ("the threshold of a dot in a screen dither"), called by `job_raster_dither.rs` for the two new `Dither` variants `Halftone` and `LineScreen` (keys `halftone`, `line_screen`); the dot grid, sampling, tone table and rows of ADR 0068 §2 to §4 and ADR 0069 are unchanged.
2. **Cell grid.** `N = max(4, round(size / I))` dots per cell side (`size` = `screen_size_mm` in µm, `I` the Interval in µm). The grid is anchored at the item's frame origin (the visible frame's first corner, ADR 0072 §3), so the screen moves with the object, and turned by `screen_angle_deg` from the page's X axis, **not** from the scan angle, so changing the scan angle does not change the dots. A 90-entry table of `round(65536 · cos)` and `round(65536 · sin)` for whole degrees 0 to 89 is written out in the module. For a dot centre `(x, y)` in page µm relative to the anchor, the rotated position is computed in `i64` from the table, divided by `I` with `div_euclid` into whole dots, and taken `rem_euclid N` into the cell index `(s, t)`.
3. **Threshold tables** (built once per item, `N x N` bytes, deterministic).
   - **Halftone dots:** the clustered round spot: every cell position gets the key `(s − c)² + (t − c)²` in quarter-dots from the cell centre `c = (N − 1)/2`; positions are ranked by key, ties by `(t, s)`, and rank `r` gets the threshold `T = (2r + 1) · 255 / (2N²)` (integer, rounded half up). The dot burns iff `255 − g > T`.
   - **Line screen:** the key is `|t − c|` only; the same ranking over the `N x N` positions gives bands that thicken with darkness.
   - White in burns nothing, black in burns every dot (test).
4. **Fields and limits.** `screen_size_mm` 0.20 to 5.00 (two decimals), `screen_angle_deg` 0 to 89 (whole), both modes; shown in the group only for these modes (spec criterion 44). A cell smaller than 4 dots is widened to 4 (the formula above) and the field keeps its value.
5. **Goldens** `halftone_dots_ramp` and `line_screen_ramp` (`curvyo-laser-core/tests/fixtures/raster/`, the dot-map text of ADR 0068 §6) at angles 0 and 45; native and wasm equal.
6. **Budget**: within ADR 0068 §8's Threshold and Ordered budget (one lookup per dot plus the rotation).

## Consequences

- Nine modes; the dither picker gets a third row (UX notes).
- Screens are binary like Ordered, so M3 and M4 both run them; Grey stays the only level mode.
- A screen finer than about four dots per cell looks like Ordered; the widening of decision 4 is silent (Proposal).
