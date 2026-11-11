# UX notes — revised 2026-11-11

## Pen bar

Icon buttons only, in one row, no "Mode" label: Bézier (pen with handles), Spiro (smooth loop through points), B-spline (curve with control polygon), Lines (zigzag), Freehand (scribble). Tooltip and `aria-label` carry the name. Active button uses the accent colour. Smoothing slider appears right of the row for Freehand only. A mode change while a path is unfinished is refused with one short line beside the bar.

## Drawing

Spiro and B-spline: each click adds a node and the curve updates under the pointer. Nodes are squares (corner) or circles (smooth); B-spline also shows the control polygon as a thin line. Shift+click makes a corner. Enter or double-click finishes. Right click finishes too (nothing in progress: nothing happens). Esc discards the unfinished path; a second Esc returns to Select. A Spiro never shows a notice while you place points: a point with no solution leaves the curve where it was. A Spiro from an older build is drawn as straight lines and refuses operations with "This Spiro curve comes from an older build…".

## Width

Selecting a stroked path shows **Width** in the Style panel with the profile buttons and an Edit toggle. In Edit the diamonds appear on the centre line, the cursor over the line shows a ghost diamond (click adds), dragging sideways shows a live readout in mm and a faint outline preview. A **Done** button in the Pen/Select bar leaves the mode; Escape does too. Profile buttons act at once, with or without Edit.

Check at 800 × 600 and at desktop size. Icons come from the existing icon set; new symbols follow `docs/design-system.md`.
