# UX review of 0054 combine-open-paths

Branch `story/combine-open-paths`, head `bcfd71ee`. Reviewer: ux-engineer, 2026-10-19.
Checked against `specification.md` (criteria 1 to 24, UX notes) and `docs/design-system.md` (rows
"Action notice", "Refusal outline", "Compound path in the UI", "Path toolbox", "Path tooltips and
notices", "Boolean tooltip").

**Verdict: CHANGES REQUESTED.** What the maker sees works well. A plate, a score line, a U and a
circle become one object with one press. The notice names holes and open outlines in one sentence.
No overlay draws a closing line. Fill paints the plate and leaves the score line a plain stroke. The
long notice stays 8 s and wraps inside the canvas. Four must-fix items are left, all small. A
re-check of these four is enough.

## How this was tested

WebKitGTK `MiniBrowser` under Xvfb, viewport 800 x 600 CSS px, real pointer and key input from
`xdotool`, the `npm run build` output of `bcfd71ee` served by `vite preview`. Test documents, drawn
with the Rectangle, Ellipse and Pen tools: (A) a plate 40 x 40 mm, a straight open score line across
its edges, an open U inside, a circle inside; (B) two overlapping squares and an open diagonal line
with an End marker. **Not verified:** dark theme, 2x density, other engines, keyboard (no code of the
Path or Boolean cards' focus handling changed), 2000 score lines (the tester's release budgets cover
criterion 32), Outset (OFFSET-OPEN A keeps today's Offset).

## Results per check

| Check | Result |
|---|---|
| Combine notice, A (crit. 12) | "Combine: 4 objects became 1 compound path with 1 hole and 2 open outlines." Level with Combine, left edge `--rail-right` + 12, 5 s (75 characters): PASS (screen 01) |
| Combine notice, B (crit. 12, 14) | "Combine: 3 objects became 1 compound path with 1 open outline. 2 closed outlines touch or cross. It uses the style of the lowest object. Markers are not drawn on compound paths." 179 characters, 4 lines, clamped to the canvas (ends 12 px before the panel), on screen at 7 s, gone at 9 s: PASS (screen 05) |
| Split one piece (crit. 17) | With a hole: the `0048` sentence, alert, 8 s: PASS. Without a hole: the new sentence in `pathText.ts` matches the criterion (not driven in the browser) |
| Break apart (crit. 18) | "Break apart: 1 compound path became 4 objects. Holes became filled shapes." and "... became 3 objects.": PASS |
| Boolean notice (crit. 21) | Difference of plate, U and circle: "Difference: 3 objects became 1 compound path. 1 open outline was closed with a straight line.", level with Union: PASS |
| Boolean availability (crit. 23) | Enabled with two or more objects whatever their outlines; dimmed with one: PASS. Notes: findings 2 and 3 |
| Combine availability (crit. 13) | Enabled with open paths selected; note "Replaces the selection."; dimmed note "Select two or more paths or shapes.": PASS |
| No closing line in overlays | Cut refusal outline of the compound path (screen 03), the blue drag preview (screen 04) and the hover box: the U is drawn open, the score line is outlined as a line: PASS |
| Fill and strokes (crit. 2, 3, 7c) | Plate filled, circle a hole, score line stroke only, U stroked open and painted as an unfilled "D" (its stored direction against the plate's; documented, decision 2). Crossing squares paint their overlap, every edge stroked: PASS (screens 02, 05) |
| Markers on compound paths (crit. 5, 14) | No marker drawn on the result: PASS. **The Markers block is still in the panel** (finding 1) |
| Subject line (crit. 5) | Reads "Compound p…" at 800 px (finding 4) |
| Customer rules | No panel section grows; nothing is refused that the maker could expect to work, apart from the question below; no new operating hint |

## Findings

1. **Markers block shown for a compound path** (screen 01, right panel). Criterion 5 and `0016`
   criterion 38b: not shown. The notice of the same press says "Markers are not drawn on compound
   paths." while the panel offers three marker rows that do nothing. Fix in
   `curvyo-ui-core/src/style_panel.rs` `markers_panel`: return `None` when a path in the scope has
   more than one outline (a compound path), with a session test in `marker_panel_session.rs`. Not a
   regression of this slice (same code on `main`), but this slice makes it frequent and its notice
   contradicts the panel.
2. **Boolean dimmed note "Select two or more closed objects."** (screen 07) is false since crit. 21.
   Keep the line, drop the word: "Select two or more objects." (`frontend/src/lib/booleanText.ts`
   `tooltipNote`, `booleanTextSpec.test.ts`). Why keep it: it names why the button is dimmed (a
   state, which the 2026-10-16 rule keeps), and Combine, Split, Break apart and Cut carry the same
   kind of line; dropping it on one card only would break the consistency that replaces
   explanation.
3. **"Closes 2 open outlines with a straight line." counts a straight outline that encloses
   nothing** (screen 06): with plate, U, circle and a straight score line selected, the note says
   2, the press then refuses ("Union needs shapes that enclose an area. 1 of 4 selected objects has
   no area."); inside a compound path the same line would be "removed", not "closed". The UX notes
   promise that note and notice say the same thing. Fix in `curvyo-ui-core/src/boolean.rs`
   `boolean_availability`: count only the open outlines that the plan counts as `closed_open`
   (closed form encloses an area), by the same predicate; adjust the ui-core test.
4. **Subject line cut to "Compound p…"** (screen 08). `PanelSubject` in
   `frontend/src/components/panel/PropertiesPanel.tsx` caps it at `max-w-[80px]`; the design-system
   row "Properties panel: tabs" says it takes the rest of the header after an 8 px gap. Replace the
   cap with `flex-1 min-w-0 text-right truncate`; the header has 140 px free at 800 px.

Low, no change requested: the fill colour field tooltip "Click to set, double-click to type the hex
value" is an operating hint from before 2026-10-16; worth a sweep in a `chore/` together with "Use
the Select tool.".

## Question for the customer (via the lead)

**Boolean with a loose straight line in the selection.** Criterion 22 refuses the whole operation
when one operand is a lone straight line (screen 06), but removes the same line without a refusal
when it sits inside a compound path. Options: (A) keep the refusal as specified; (B) remove the
operand with the sentence "1 open outline enclosed no area and was removed." and refuse only when
no operand encloses an area. Recommendation B (prefer permitting; one rule for the line inside and
outside a compound path). Default without an answer: A, as built; B is a follow-up story.

## Design-system rows written

`docs/design-system.md`: "Action notice" (8 s above 140 characters; Boolean open-outline
sentences), "Refusal outline" (no closing line in any overlay; Combine outlines no geometric
offenders), "Compound path in the UI" (open, touching, crossing outlines; subject without counts;
fill of open outlines), "Path toolbox" (open and touching shapes no longer refused), "Path tooltips
and notices" (Combine note, sentence order, single-node and Split sentences, removed refusals),
"Boolean tooltip" (dimmed note without "closed", the "Closes ..." note counting only outlines that
enclose an area: findings 2 and 3 bring the code to these rows).

## Screens (`ux-screens/`)

01 Combine notice and the Markers block · 02 Fill with open outlines · 03 Cut refusal outline, U
open · 04 drag preview, U open · 05 the 179-character notice · 06 Union note "Closes 2" and the
refusal that follows · 07 Boolean dimmed note · 08 subject line cut.
