# Pen modes and variable-width paths

Status: Revised 2026-11-10 after the first demo, waiting for go (customer approved the concept and ADR 0053 on 2026-11-07)
Priority: Should
Origin: Customer, 2026-11-07 (renumbered from 0106 and ADR 0045: both were taken on `main`)

## User value

Draw with Bézier, Spiro, B-spline, straight-line and freehand modes, and shape a path's width along its centre line, in preparation for tablet drawing and satin embroidery.

## Demo findings (2026-11-10)

Lines works. Spiro and B-spline paths are not visible while drawing or afterwards. Width editing half works but is awkward. The mode buttons carry text instead of symbols. The branch must be rebased onto `main`.

## State of the art

- **Inkscape** (Bézier, Spiro, B-spline, paraxial): the mode is an icon row beside the tool; the smooth curve is visible while placing points; nodes stay editable afterwards.
- **Illustrator Width tool, Inkscape Power Stroke, Affinity Stroke Width tool**: drag out from the centre line to widen both sides equally; width points are small diamonds on the line; a numeric field and a few ready profiles (taper start, taper end, both ends) make the common cases one click; Reset returns to uniform.

## Acceptance criteria

1. The Pen bar shows one icon button per mode (Bézier, Spiro, B-spline, Lines, Freehand) with a tooltip and an accessible name; no text labels. The active mode is highlighted; Bézier is the default. Freehand alone shows a Smoothing slider.
2. Spiro and B-spline show their curve live while points are placed, and the finished path is drawn, hit-tested, selected and exported as that curve. Their control points show as nodes and stay editable after save and reopen (CURVES A). Preview and result agree. A Spiro never shows a "cannot be solved" notice and never falls back to straight lines: a point that has no solution leaves the curve at its last solvable result. The solver starts from the previous solution; a segment turns by at most 90° and the sweep changes by at most 270° when a point is added or removed (Proposals). A Spiro from an older build that has no stored solution is drawn as straight lines and refuses operations that need the curve with "This Spiro curve comes from an older build…".
3. Straight lines, Freehand and the Bézier behaviour of the first demo keep working. Pen abort keys: Esc discards the unfinished path (a second Esc returns to Select); right click finishes the path, and with nothing in progress does nothing.
4. Width entry: the selected path shows its width points as diamonds on the centre line. Click the line to add a point, drag a diamond along the line to move it, drag sideways to change the width live in document units, Delete removes it, Escape or a visible Done leaves the mode. One drag is one undo step.
5. The Style panel offers width profiles as buttons: Uniform, Taper start, Taper end, Taper both, plus an Edit width control and a numeric width field for the selected point. Choosing a profile is one undo step and keeps the centre line.
6. Rendering, hit testing and SVG export of a variable stroke follow the visible outline; Stroke to path makes the closed outline. Machine output keeps the centre line (OUTPUT A). Operations that cannot keep curve data or a profile refuse with a message and change nothing.
7. The Document panel stays populated while drawing and the bottom bar is gone (STATUS A); both work at 800 × 600.
8. Old documents and ordinary strokes are unchanged; the format version is bumped so older builds cannot silently drop the new data.
9. Profile interpolation, B-spline rules, merge behaviour and the profile's behaviour under transforms follow ADR 0053; each rule there has a test.

## Out of scope

Pressure capture, tablet UX, satin stitches, a brush library (`0032`, `0033`). The profile stays editable so pressure can feed it later.

## Customer decisions

2026-11-07 (unchanged): CURVES A, WIDTH A, OUTPUT A, STATUS A; one branch, one PR.

2026-11-11: no "cannot be solved" notice and no straight-line fallback for Spiro; it stays at its last solvable result (warm-started solver, per-segment turn ≤ 90°, sweep ≤ 270° after adding or removing a point; the limits are Proposals). Esc discards the unfinished Pen path, a second Esc returns to Select; right click finishes the path, nothing in progress does nothing. A Spiro from an older build without a solution is drawn as straight lines with the refusal "This Spiro curve comes from an older build…".

## Links

[UX notes](ux-notes.md) · [Plan](plan.md) · [ADRs](adrs.md) · [ADR 0053](../../docs/adr/0053-editable-curves-and-width-profiles.md) · [Inkscape pen modes](https://wiki.inkscape.org/wiki/ReleaseNotes047)
