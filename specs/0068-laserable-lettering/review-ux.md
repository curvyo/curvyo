# UX review: 0068 Weld

ux-engineer, 2026-10-20, branch `story/weld` (3 commits on `story/text-objects`). Driven live in
Chromium (production build, `vite preview`) at 1280 x 800 and 800 x 600, panel open, rulers on.
Checked against the UX notes in `specification.md` and `docs/design-system.md`.

**Verdict: CHANGES REQUESTED** (one MUST-FIX in the Gap field).

## Findings

1. **MUST-FIX. Escape in the Gap field commits the typed value.**
   `frontend/src/components/WeldGroup.tsx:158-169`. Escape calls `onReturnFocus()` while
   `draft` still holds the typed text; the blur that follows runs `commitDraft()` with it. Typed
   "0.7", Escape: the field and `curvyo.weld-gap-mm` show 0.7 (expected: the old 0.2 stays).
   Fix: keep the draft in a ref, or set a `skipBlurCommit` ref in the Escape branch (and after a
   commit by Enter) and read it in `onBlur`. Add a test: type, Escape, value unchanged.
2. **SHOULD-FIX. A typed unit value is stored unrounded.** `frontend/src/hooks/useWeld.ts:318-324`.
   "0.004 in" shows "0.1" but stores and welds with 0.1016 mm. Round to 0.01 mm (the shown
   precision and the step) before storing, so the field shows what Weld uses.
3. **SHOULD-FIX. A press on the dimmed button closes its tooltip.** UX notes §2 say it stays open.
   Radix closes a tooltip on pointer-down; Flip and Guide behave the same, so fix all three at once
   (`onPointerDown` with `preventDefault()` on the trigger while `aria-disabled`), in
   `WeldGroup.tsx:218`, `FlipGroup.tsx:85`, `GuideButton.tsx:38`.

## Fixed in this review (visual only)

- **Glyph:** the three-disc `WeldGlyph` read as a capsule (a link icon) at 1x and 2x: the
  notches between discs 4 units apart are 0.7 px deep. Switched to the fallback of the UX
  notes, two discs of radius 4 on (5, 8) and (11, 8), one silhouette. It now reads as two joined
  circles and apart from Union (two squares) and Combine. Design-system row "Weld" updated.

## Checked and passing

- **Place:** Select bar, after Flip and Guide behind a divider (12 + 1 + 12 px), one group
  `role="group"` "Weld" of 136 px (28 + 4 + 104). Absent with Pen, Node, Rectangle, Ellipse and
  Text tools.
- **Button:** 28 x 28, no label; tooltip "Weld" / "Joins overlapping shapes into one outline."
  (one line), plus "Select an object first." only while dimmed; shown on keyboard focus.
  Dimmed: `aria-disabled`, 40 %, default cursor, no hover ground, press does nothing.
- **Busy:** during the press the group and the root are `aria-busy`, the cursor is `wait`, the
  button keeps its pressed ground; all three clear on return.
- **Focus:** a mouse press returns focus to the canvas; Enter on the focused button keeps it.
- **Gap field:** 104 x 28, "Gap" and "mm" inside, "0.05" fits without clipping. Enter and blur
  commit; Up/Down 0.01, Shift 0.1, stop at 1 without a notice; "5" and "-1" give the clamp
  notices 4 px under the field; "abc" gives the invalid ring and "Enter a number", the stored
  value is kept. Kept after a reload.
- **Notices:** success and refusal 4 px under the button, left-aligned, max 360 px, refusal with
  alert glyph and ring, `role="alert"`; the success text is in the polite region.
- **Welding:** "Annabelle" in Great Vibes 20 mm: "Weld: 1 text became 1 compound path, 2
  pieces." at Gap 0 and 0.1 (the letters overlap, no point contacts in this word); "Annie
  Jill": 5 pieces, i-dots kept loose. Ring with an island and a bar: "Weld: 3 objects became 1
  compound path, 2 pieces.", island kept. Open pen line with a rectangle: one path with the
  stroke band. Dashed line: "... Dashes were welded as solid lines.". Open line without stroke
  and fill: refusal "Weld needs shapes that enclose an area. 1 of 1 selected object has no
  area. Nothing was changed." with the red Refusal outline.
- **Result and undo:** one compound path selected, Layers row "Annabelle" with the compound
  glyph, History "Weld"; one Ctrl+Z gives back the text, and a double-click edits it again.
- **Customer rules:** nothing opens, no panel section, no preview; no hint labels (the dimmed
  line names a state).

## Select bar rows, measured

| Selection | 800 x 600 (bar 324 px) | 1280 x 800 |
|---|---|---|
| Nothing selected | 2 rows, 72 px | 1 row, 557 px |
| One path | 3 rows, 108 px (was 2) | 1 row, 676 px |
| One text | 4 rows, 144 px | 2 rows, 804 px |
| One rectangle | 5 rows, 172 px | 2 rows, 804 px |
| One ellipse | **5 rows, 180 px (was 4; the notes predicted 4)** | 2 rows, 804 px |

The ellipse's arc kind group (145 px) shared row 2 with Flip and Guide before; behind Weld only
39 px are left, so it moves to row 3. Accepted like the path case: the bar overlays the
pasteboard, not the panel. Recorded in the design-system rows.

## Not checked live

- Missing-face refusal (needs a project with a missing font) and point-contact squares at Gap 0
  against 0.1 (needs exact placement): covered by the acceptance tests of criteria 7 and 27.

Screens: `ux-screens/01-annabelle-welded-1280x800.png`, `02-no-area-refusal-800x600.png`,
`03-path-selected-tooltip-800x600.png`.
