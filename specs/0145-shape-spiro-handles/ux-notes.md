# UX notes for shape loops (the Rounded handle)

Status: complete for slice 1 (polygon, star); slice 2 (rectangle, ellipse) is provisional until the customer has tried slice 1 (criterion 15). `ux-engineer`, 2026-11-23.

Everything here reuses the parameter-handle vocabulary of `0009` (knob, tier `T` = 72 px, 12 px hit radius, 4 px clearance), the knob displacement and Curve rules of `0021` and the design-system rows "Parameter handle", "Parameter handle guide", "Knob displacement", "Drag ghost", "Live transform readout", "Bar number field", "Limited tag" and "Select bar layout". No new colour token, no new glyph family, no new font size. Distances are screen pixels. No hint chip, no tooltip line beyond a control's name, no tip text anywhere (customer rule 2026-10-16; the hint chips and the "zoom in" tooltip of `0009` §2, §4 and §5 are superseded by that rule and are not copied here).

## 1. Reference geometry: the straight shape, frozen at the press

All placement below is measured on the shape **at `q = 0`** (its ring of vertices and its straight box), never on the grown box of a looped outline (criterion 7). So:

- the tier `T` (shorter side `s` of the box, `0009` §1) is measured on the straight shape's box, for every parameter knob of that shape (Rounded, Curve, inner radius, corner radius, arc), so a drag into loops never makes knobs appear or vanish; this matches the `0021` build note "the 72 px threshold is measured on the whole ellipse";
- the knob's ray, its unit length `U` and the tick positions are taken once at the press and held for the whole drag (the transform glyphs and the selection box do follow the live Bezier bounds, criterion 7).

## 2. Placement of the Rounded knob (criterion 1)

The standard 10 px knob of the "Parameter handle" row (idle, hover, dragging looks; drawn above the transform handles).

- **Ray:** from vertex 0 (polygon: the vertex the creation drag put at B; star: the first outer vertex, as in `0021`) towards the ring's centre `C`, continued past `C` to the opposite side and beyond. `û` is the unit direction from vertex 0 to `C`; `D` is the screen distance from vertex 0 to `C` (the outer radius on screen).
- **Position:** `P(q) = V0 + (15 + q·U)·û` with **`U = D − 15`**. So `q = 0` is 15 px inside vertex 0 (the `0013` placement), **`q = 1` is exactly at the centre** (the end of the normal range: "pull the corner to the middle and it is soft; past the middle come the loops"; some stars loop a little before 1 and every `q < 0` loops, `adrs.md` finding A, so the tick marks the range end, not the first loop), `q > 1` lies beyond the centre towards the opposite side, `q < 0` towards and past vertex 0, outward.
- **Worked numbers** (a regular polygon or star with outer radius `D` on screen; a pentagon at `s` = 72 has `D` of about 40):

| `D` (px) | `U` (px per 1.0) | `q = 1` | `q = 2` | `q = 10` reached at | `q = −10` reached at |
|---|---|---|---|---|---|
| 40 | 25 | centre | 25 px past the centre | 265 px from V0, inward | 235 px outside V0 |
| 60 | 45 | centre | 45 px past | 465 px | 435 px outside |
| 120 | 105 | centre | 105 px past | 1065 px | 1035 px outside |
| 300 | 285 | centre | 285 px past | 2865 px | 2835 px outside |

  On a large shape on screen the outer values are out of reach of a drag inside the window; the maker zooms out or types a value (bar field, entry chip). This is the honest price of a scale-invariant map (the same pull gives the same form at every zoom), and the range the screenshot shows (about -1 to 3) is in reach at every size from `T` up to `D` of about 300 px.
- **Clearance at rest and on hover** (criterion 1, the "Knob displacement" row): the Rounded knob keeps `P(q)` unless a transform glyph is closer than 15 px or an already placed knob closer than 14 px. Placement order: the star's inner-radius knob keeps its place (as today), then the **Rounded knob**, then the Curve knob. The Rounded knob is displaced **along its own ray** by the shortest distance that clears, at most 28 px, towards the centre if that does not clear; the Curve knob clears the Rounded knob along its side's bisector by the existing rule. Known cases: a 4-sided polygon whose vertex 0 is a box corner at `q` near −15/U (the knob over the corner resize square); a star at ratio near 0.99 with many points (the inner-radius knob on almost the same ray); a star at `q = 0` whose Curve knob sits about 7 px away (5 points, ratio 0.5): the Curve knob moves.
- **During the drag** the dragged knob is never displaced: it is drawn at `P(q)` and may pass over other knobs and glyphs, which keep the drawn positions they had at the press (no knob dances around the moving one). Nothing else can be hit during a drag, so the 4 px rule is a rule for rest and hover.
- Not drawn for a selection of two or more objects, nor during a move, resize, rotate or skew drag of the same object (`0009` §1).

## 3. Drag: through the centre, snapping, cancel (criterion 2)

- **Relative to the press:** `q = q_press + (pointer displacement projected on û) / U`. A knob that was drawn displaced at rest therefore does not change the value at the press; from the first counted movement it is drawn at its true `P(q)` (a one-time glyph shift of at most 28 px, no value jump). On a knob that was not displaced, relative and absolute are the same, so the knob stays under the pointer.
- **No stop at the centre:** the projection is a full line, so the drag runs from `−10` (outside vertex 0) through `0`, `1` (the centre) and on past the opposite side to `10`. At a limit the value is clamped: the knob stops on the ray, the readout appends "max" or "min", and moving back changes the value again only once the pointer's projection is back at the limit point (the handle stays under the pointer until a limit, `0009` §1).
- **Snap without a modifier** to 0 and 1, and on an ellipse also to `q_c` (section 9). The window is **the larger of 0.015 and 3 px / U** (section 11, change 2): at `U` = 25 that is 0.12 (3 px), at `U` = 285 it is 0.015 (4.3 px).
- **Ctrl:** steps of 0.25 (Ctrl replaces the plain snaps; the limits are on the step grid). Shift and Alt have no meaning on this knob.
- **Escape** cancels: the shape, the Curve value, the knobs and the box return to the committed state in that frame, nothing is written. Less than 3 px of movement writes nothing. Release writes one commit "Change rounding"; a release at the press value writes nothing.
- **Cursor:** the built-in `pointer` on hover and during the drag, never changed by Ctrl (the "Parameter handle" row).

## 4. Guide, the normal-range ticks and snap feedback

- **Guide:** the 1 px dashed `--shape-handle-guide` line of the "Parameter handle guide" row, drawn **while the Rounded knob is hovered or dragged**, never at rest. It runs along the ray from `P(min(q, 0))` to `P(max(q, 1))`: it always covers the whole normal range and reaches the knob, through the centre when `q > 1`, past vertex 0 when `q < 0`. Clipped at the canvas edge.
- **Ticks** (the end-of-normal-range marker of criterion 2, plus its start): one tick at `q = 0` and one at `q = 1` (the centre), each a line **perpendicular** to the guide, 10 px long, 1.5 px, solid `--accent` with a 1 px white casing each side (the casing of the "Live preview outline" row, so it reads over a dark fill). Solid accent and not the 60 % guide colour, because the 60 % colour measures about 2.4:1 on white and the ticks carry meaning (non-text contrast 3:1; `--accent` on white is 4.5:1). A tick is not a knob (a line, not a dotted circle), has no hit area and no label.
- **Snapped:** while the value is held at a snap target the tick under the knob is drawn 16 px long (3 px sticking out on each side of the knob), so the snap reads by shape, not by colour; the readout shows the exact value ("rounded 1.00").
- **Ctrl held during the drag:** minor ticks, 6 px, same style, at every 0.25 step along the drawn guide, only where they are at least 6 px apart (`U` ≥ 24); the step the value sits on is drawn 16 px like a snap.
- **The centre handle** (`0009` §1) is not drawn while the Rounded knob is hovered or dragged, because the guide and the `q = 1` tick pass through the middle of the shape; it already yields during every parameter drag and whenever a knob is within 20 px of it (a knob at `q = 1` rests at the centre).

## 5. Live loops while dragging and the readout

- **The shape is drawn as itself** with the outline a release would commit, by the **"Drag ghost"** row (customer 2026-10-27), not by the blue hollow outline that `0009` and `0021` still describe: fill (nonzero, overlapping loops solid, criterion 8), stroke and stacking place as committed; the committed outline stays behind as the ghost; Escape and release remove the ghost in that frame. The outline is recomputed and flattened within the 2 ms of criterion 12 per pointer event.
- The selection box and the transform glyphs follow the live Bezier bounds (criterion 7), drawn above the shape; the parameter knobs stay placed from the straight shape (section 1).
- **Readout** (the "Live transform readout" chip, 12 px up and right of the pointer, its flip and clamp rules): "rounded 0.35", two decimals, real minus sign ("rounded −1.25"), "rounded 10.00 max" / "rounded −10.00 min" at a limit. Lowercase like "curve -35.0 %" and "ratio 0.45".

## 6. Rounded and Curve: mutual exclusion, shown without words (criterion 5)

- **During a Rounded drag**, from the first counted movement the preview has Curve 0: the shape loses its arcs and the **Curve knob jumps to its 0 position** (the middle of the straight side, which then lies off the looped outline). That jump is the visible statement that the two exclude each other. If the drag returns to the press value before release, the preview shows the old Curve again (the preview is a function of the current `q` only); Escape restores both.
- **During a Curve drag** of a shape with `q ≠ 0`, the preview has `q = 0` and the Rounded knob jumps to its rest point 15 px inside vertex 0. Same rule both ways.
- **In the bar** after the commit, the other field simply shows 0 ("0.0" for Curve, "0.00" for Rounded). Neither field is ever greyed, struck through, linked by an icon or explained by a line; both stay editable; typing in one resets the other in the same commit, one Ctrl+Z restores both.
- A shape with a Curve that is neither 0 nor the shape's "looks like q" value changes visibly at the first pixel of a Rounded drag (arcs gone). That is the decided rule (Frage 309 A); the ghost shows what it was.

## 7. Bar field, entry chip, Mixed (criterion 3)

- **Place:** the Rounded field sits **immediately after the Curve field wherever Curve is rendered**, 4 px apart, so the two exclusive parameters are side by side: in the Curve group for polygons and stars (after the Points and Ratio group), inside the arc-fields group after Start, End, Curve when an ellipse is selected (slice 2, as `SelectToolbar.tsx` renders Curve today). Where Curve is not rendered but Rounded is (slice 2: rectangles only), Rounded is a group of its own directly after the rectangle group. Shown by the one rule of `0009` §4: while the selection contains a kind Rounded acts on (slice 1: polygon or star), acting on those only.
- **Field:** the "Bar number field" pattern of the Curve field: 28 px high, label "Rounded" inside the left edge (12 px `--toolbar-icon` at 70 %), **no suffix** (a factor, like Ratio), right-aligned tabular value with two decimals, -10 to 10. Width (Proposal, measured at the build like Curve's 99 px): about 104 px for the label and "-10.00"; about 153 px while it shows the "limited" tag. Tooltip: the name only, "Rounded".
- **Keys:** Enter commits a typed value, limited to the nearest bound and that bound written (criterion 3, no error message for out of range); Escape or a press elsewhere restores and writes nothing; empty or non-numeric: the 2 px `--field-invalid` border and "Enter a number" under the field. Arrow keys step 0.05, Shift 0.25 (Proposal), live preview, one commit on key-up (the Curve field rule).
- **Mixed:** shapes with different values show the empty field with the muted placeholder "Mixed"; a typed value goes to all of them in one commit and sets their Curve to 0 (the Curve field then reads 0.0, not Mixed).
- **Limited** (criterion 11, a stored value outside -10 to 10): the field shows the bound with the 12 px "limited" tag before the field end; tooltip second line "Stored 12.40, limited to 10.00." (a state, not a tip).
- **Double-click on the Rounded knob** opens the existing entry chip (`0021` "Entry chips") with one field, accessible name "Rounded", prefilled with `q` to two decimals, no suffix, no Apply button; Enter commits by the field rules, Escape closes; the knob keeps the dragging look while the chip is open.
- **Focus order:** Points, Ratio, Curve, Rounded (slice 2 with an ellipse: Start, End, Curve, Rounded).

## 8. Below the 72 px threshold

No Rounded knob, no guide, no ticks, no ghost glyph and no message: the canvas shows exactly what a small polygon or star shows today. The bar field and the double-click entry are not reachable without the knob, so the **bar field is the route** (criterion 1); its tooltip stays the name only. The shape itself draws its loops at every size.

## 9. Slice 2: ellipse (Ready 2026-11-24); rectangle not built, Frage 326 answered B

- **Ellipse:** vertex 0 is E (right-hand cross node, own rotated frame); the knob rests 15 px inside E on the ray to the centre, `U = rx on screen − 15`, `q = 1` at the centre. Clearance against the arc knobs (seam at 45°) and the Curve knob (315° at 100 %) holds by distance at `T` (about 26 px at `rx` 36). **Rest at `q_c` while the ellipse is untouched (Proposal):** an ellipse with `q = 0` and Curve 100 % draws its knob at `P(q_c)`, where its look already is, so the first pixel of a drag continues from the circle instead of jumping to a diamond (`q = 0` with Curve 0). A third tick marks `q_c`, which is a snap target (criterion 15, window as in section 3). The field shows the stored "0.00" meanwhile. In that case `q_press` in section 3 is the drawn `q_c` (0.3905, `adrs.md` decision 9), not the stored 0, and a release at `q_c` writes nothing. **Exception to section 1:** the arc knobs exist only at Curve 100 % ("Arc handle layout"), so they vanish while a Rounded drag holds Curve at 0 and come back on Escape or a return to `q_c`.
- **Rectangle (not built, Frage 326 answered B 2026-11-24; kept as the placement a later spec would start from):** vertex 0 is the top-left corner. The knob lies on the **ray from the top-left corner to the centre** (the box diagonal, which differs from the 45° corner-radius diagonal on a non-square box) with its **`q = 0` point at 29 px** from the corner (one pitch beyond the radius knob's 15 px rest, so the two are 14 px apart when both rest), `U = |corner to centre| − 29` (22 px on a 72 px square), `q = 1` at the centre. The top-left radius knob keeps its place (drawn position after the diagonal cap of `0013` UX note 2) and the Rounded knob clears it along its ray by the displacement rule (on a square the radius knob passes the Rounded rest point at about `ρ` 0.5). While `q ≠ 0` the radius knobs rest at their 0 positions (the effective radius is 0, criterion 14) and the Radius field shows the effective 0; a radius drag sets `q` to 0 in the same commit and the Rounded knob jumps to its rest point (section 6). No `q_c` tick on the rectangle: on the corner ring `q_c` is the ellipse through the four corners, not a soft square (`adrs.md` decision 9, finding B; the rectangle half of slice 2 waits for that Frage, default B "Rounded only on the ellipse"). The placement above holds if A is chosen.
- **Bar:** section 7; with an ellipse the arc-fields row gets wider than 308 px and wraps by the existing rule; row counts measured at the build.

## 10. Accessibility

- Accessible names: bar field "Rounded", entry chip field "Rounded"; the "limited" tag and the invalid message are referenced by `aria-describedby` as in the Curve field; Mixed is the placeholder.
- The canvas knob, its guide, its ticks and the readout are not in the accessibility tree (the known gap of `0009`); the bar field is the keyboard and screen-reader route at every size.
- Contrast: knob and field reuse tokens that already meet 3:1 and 4.5:1; the ticks are solid `--accent` with a white casing (section 4); snapped state is shown by tick length, not colour alone.

## 11. Changes the criteria need (for the PO)

1. Criterion 1: the 4 px rule holds at rest and on hover; the dragged knob may pass over other glyphs mid-drag. `T` is measured on the box of the shape at `q = 0`.
2. Criterion 2: name the unit (`q = 1` at the ring's centre, `U = D − 15`), and the snap window "within 0.015 or 3 screen px, whichever is larger" (0.015 alone is 0.3 px on a shape at `T`, no snap a maker can feel). The drag is relative to the press (a knob displaced at rest does not jump the value).
3. Criterion 14: the rectangle's Rounded knob is on the corner-to-centre ray with its zero point 29 px from the corner, not 15 px (that point is the top-left radius knob's rest).
4. Open for the PO, with Proposals: Rounded on an ellipse with an arc (Proposal: greyed with "Not available while the ellipse has an arc" and no knob, as Curve); whether "Remove rounding" on a rectangle also sets `q` to 0 (Proposal: yes, and it is enabled while `q ≠ 0`); the rectangle half of slice 2 follows `adrs.md` finding B. "Remove rounding" touches an accepted `0013` control, so that Proposal is vetoable at the demo or a customer question.

## 12. Design-system rows the implementer adds in the build PR

"Rounded handle" (sections 2, 3), "Rounded guide ticks" (section 4), "Select bar Rounded field" (section 7); the "Knob displacement" row gains the placement order inner radius, Rounded, Curve; the "Live transform readout" row gains "rounded 0.35"; the centre-handle yield of the "Parameter handle layout" row gains "and while the Rounded knob is hovered or dragged".
