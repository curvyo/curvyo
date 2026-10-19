# UX review, story 0021 (head 17e74ef8)

Verdict: CHANGES REQUESTED. Driven in Chromium at 1280x800 and 800x600 against a fresh wasm build. Screenshots in `review-ux/`.

## Must fix
1. Curve field clips its value. "100.0" shows as "100." and "-100.0" as "-100" (scrollWidth 103 > clientWidth 94: 48 px left padding, 20 px right padding, text). It is the default value. Widen to about 112 px or cut the left padding to the label width. `27-curve-field-zoom.png`, `30-star-curve-neg.png`, `04-ellipse-800x600.png`.
2. Skew of an ellipse with an arc does nothing. The preview shows "Skew y +16.3°" and a blue arc, the release writes nothing (no History entry, shape stays an arc). Criterion 56: it becomes a path, as a whole ellipse does today (`23-whole-skew.png`: Path). Tried both side skew handles (`22-skew-left.png`, `22-skew-right.png`, `22-history.png`).

## Should fix
3. Bar height. 1280x800: 3 rows (Curve far right on row 1, Start/End on row 2, Object to path alone on row 3). 800x600: 6 rows, 216 px, 36 % of the window; a polygon at 800 sits under it. Put Curve beside Start/End on row 2 (104+4+104+96 fits the 324 px bar). `04`, `05`, `29`.
4. Entry chips (double-click on an arc or curve knob) show the number only: no "°" or "%" suffix as the UX notes require. `13b-curve-chip.png`, `13d-arc-chip.png`.
5. The Curve tooltip line "100 % circle, 0 % diamond, -100 % star." is an operating hint (Proposal 2). Customer rule: remove it until the customer decides.
6. Knob displacement. A knob whose end meets a box corner is moved about 15 px (the 28 px bound did not occur in my cases; acceptable as a bound). The two ends go different ways: start inward on the arc, end outward onto the undrawn outline, where it floats detached from the arc. Prefer inward for both. `08`, `37-half-circle-corner-knobs.png`.
7. Threshold T uses the tight box, so a 30 degree sweep of a 200 px circle has no arc knobs and can only be widened in the bar. Measure T on the ellipse's own 2rx x 2ry. `38-30deg-sweep-no-knobs.png`.
8. With keyboard focus in an invalid Start/End field, the field's tooltip covers the "Enter a number" message (`36-invalid-field.png` shows the mouse-away state). Close the tooltip while the field is invalid.
9. Start and End do not step with the arrow keys, Curve does. Same field pattern, same behaviour.

## Checked and fine
- Ellipse group is always shown for an ellipse; kinds and Whole ellipse greyed on a whole ellipse (`aria-disabled`, Tab stop and tooltip with reason kept, `34`); Curve greyed with an arc. `role="group"` "Arc kind" with `aria-pressed`. Tab order: Pie, Arc, Chord, Whole ellipse, Arc start, Arc end, Curve. Space on a greyed button does nothing; Enter commits and returns focus to the canvas.
- Resting knobs 14 px apart at the 45 degree seam, curve knob 90 degrees away (`02`); distinct from resize squares. No hint chip on hover (`33`).
- Drag: readout "start 252.3° · sweep 152.7°", old black, new blue; Ctrl snaps to 285 (`06`, `11`). Tight box around the drawn part (`07`).
- Mirror swaps and mirrors the angles (300/120 to 60/240), pie fill correct (`15`, `16`); non-uniform stretch keeps angles and ellipse (`18`); rotation keeps it (`20`); undo works.
- Polygon/star Curve knob, snap to 100 %, readout "curve 100.0 %", star at -100 % (`25`, `26`, `30`). Announcements "Arc start 252.3°, end 45.0°, arc" and "Flipped horizontally".
- No hint text, no right-panel section, no Apply button.
