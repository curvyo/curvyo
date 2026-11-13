# Smoother Spiro and better width handles

Status: In progress (go 2026-11-11; criterion 2 amended 2026-11-12)
Priority: Should
Origin: Customer, 2026-11-11, after trying `0110`

## User value

Drawing and dragging a Spiro feels like Inkscape: the point is under the pointer, the curve does not stick, lag or loop. Width points are easy to see, to type and to scroll.

## Acceptance criteria

1. **Spiro curve.** A Spiro is the curvature-continuous curve of Inkscape's Spiro mode (libspiro `c` points): clothoid segments between smooth points, circular arcs at open ends and beside corners; three points give their circle (Proposal). It is solved as Inkscape does: at most ten Newton steps from the control polygon, and the last step is the curve whether or not it converged; a step that would make a segment sweep more than 270° is cut short (Proposal).
2. **Stability (amended 2026-11-12 after the customer tried the PR).** The curve is a pure function of the controls; no stalls. While a Spiro point is placed or dragged it is under the pointer at every event, and the curve shown is the curve stored and the curve a reopen draws. Where the point crosses the line behind a neighbour the curve turns over at once to the other side (SPIRO-WINDING A). Measured on the Pen walks of `spiro_cache/measure.rs` at 0.5 mm steps: no stalls (was up to 70 %), the curve never the control polygon, and a move of more than 4× the pointer's step + 2 mm at no more than 1 % of the events (6 % at 6.7 mm steps).
3. **Budgets** stay: one rebuild of the live Spiro per pointer event (hover and frame) ≤ 2 ms in a release build for 3 to 20 points (`spiro_cpu_budget.rs`), the solves of `spiro_cpu_budget.rs`, the Pen hover target budget, `frame_start_budget`.
4. **Edit width in the tool bar.** The Edit width toggle is the last control of the Select and Node bars when one stroked path is selected, and is the Done of width editing (pressed and reading "Done" while editing). The profile buttons and the point width field stay in the Style panel. Fits at 800 × 600.
5. **Diamonds.** Width points are drawn larger (16 px, was 11) with a white fill, a dark ring and a white casing, the selected one in the accent colour, readable on light and dark strokes and on the canvas; the hit area grows with them (11 px radius).
6. **Typed width.** Double-click on a width point opens an inline field with its width in mm; Enter sets it (one undo step), Escape or a click elsewhere cancels.
7. **Wheel.** The mouse wheel over a width point changes its width by 0.5 mm per notch (Shift 0.1 mm), never below 0; one burst (pauses under 500 ms) is one undo step; the canvas does not zoom while the pointer is over a width point.

## Changes to accepted specs

- `0110` criterion 2: the turn and loop limits, the catch-up rule and the 90° turn rule for moves are replaced by criteria 1 and 2 here; a moved point never stays behind.
- `0110` criterion 5: the Edit width control moves from the Style panel to the tool bar (criterion 4 here).
- `0110` criterion 8 ("old documents are unchanged"): **customer-approved exception (SPIRO-G2 A, 2026-11-12).** Spiro paths saved by the `0110` build (format 42) are drawn as the G2 curve instead of the G4 curve. What changes: the drawing, SVG export, Stroke to path and the machine/cut output of stored Spiros (the centre line is evaluated from the same controls). Since the amendment of criterion 2 (2026-11-12) a run without a G2 curve (a tight zigzag) is drawn as the last Newton step instead of the G4 curve, so such stored Spiros change a second time. What stays: the controls, the stored document and the format version. Width points are arc-length fractions of the centre line, so a stored width profile slides along with the new shape (the widths at the fractions are kept, the points do not stay at the same spots). Documents without a Spiro are unchanged.
- Peers: live peers cannot diverge today, because documents with editable curves are not shared (the share block of ADR 0053, "Compatibility with older sync peers"). The case that exists is a format-42 file opened on a `0110` build: it draws the G4 curve for the same controls and gives no warning. The format does not record the solver version; see the entry in `docs/technical-debt.md`.
- Winding (SPIRO-WINDING A, customer, 2026-11-12): a point circling its neighbour is not given a stored winding; no format change (see Out of scope).

## Out of scope

Keeping a curve's winding when a point circles its neighbour (needs a format change; the customer chose to leave it, SPIRO-WINDING A, 2026-11-12). Pressure input.

## Links

[UX notes](ux-notes.md) · [Plan](plan.md) · [ADRs](adrs.md) · [ADR 0053](../../docs/adr/0053-editable-curves-and-width-profiles.md) · [0110](../0110-pen-modes-variable-stroke/specification.md)
