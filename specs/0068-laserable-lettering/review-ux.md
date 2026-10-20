# UX review: 0068 Weld

ux-engineer, 2026-10-20. Branch `story/weld-v2` (head 1cb57ab before the fixups), browser build
(`npm run build` with the wasm bindings, `vite preview`), headless Chromium at 1280 x 800 and at the
new minimum of 800 x 640, 1x and 2x. Checked against the UX notes in `specification.md` and
`docs/design-system.md`.

**Verdict: APPROVE** (one visual change made in this review, no open MUST-FIX or SHOULD-FIX).

## Checked and fine

- **Button 8:** eighth button of the Boolean card, after Flatten; card 48 x 356 px, buttons
  40 x 40 px. No Weld control elsewhere (no Select-bar group, no Gap field).
- **Column B at 800 x 640:** the card ends at y 580; the status bar starts at about 616, so the
  column ends with more than the planned 26 px clear. Nothing scrolls. The Select bar with a text
  selected is 4 rows, 144 px, ending at y 180.
- **States:** with one object selected, Weld is the only bright button (`aria-disabled="false"`,
  the other seven dimmed); with nothing selected all eight are dimmed. With two or more selected,
  all are bright.
- **Tooltip:** "Weld | Keeps each visible line once, in one object. | Replaces the selection.",
  245 px wide, one line for the rule; "Closes 1 open outline with a straight line." with an open
  3-node path in the selection. No preview line, no operating tip.
- **Results (all line art):** Fill none, Stroke on, one selection box, Properties panel subject
  "Compound path" (squares, ring, group, text) or "Path" (a disc hiding a disc). Hit test on lines
  only: clicks inside the squares' L, in the ring band and on the island select nothing; a click on
  a line selects the result.
  - Two offset 20 mm squares: the silhouette plus the upper square's two edges, once.
    "Weld: 2 objects became 1 compound path with 140.0 mm of line."
  - Ring (Difference of r 20 and r 10) with a disc r 5 in the hole: three circles, hole and island
    empty, 220.0 mm.
  - "Annabelle" in Great Vibes at 20 mm: "Weld: 1 text became 1 compound path with 658.3 mm of
    line."; overlapping strokes inside a letter are gone.
  - A nested group (a group of two squares plus a rectangle): "Weld: 1 object became 1 compound
    path with 180.1 mm of line.", one box.
- **Follow-on sentences**, in the specified order: "Weld: 2 objects became 1 path with 125.7 mm of
  line. 1 hidden shape was removed. It uses the style of the lowest object. Fill was switched off.
  Stroke was switched on." (171 characters, three lines at 360 px, 8 s); "1 open outline was closed
  with a straight line." after the count. The notice sits level with Union, 12 px right of the rail,
  as the row "Action notice" says.
- **Refusal inside a group:** a group of two open 2-node lines gives "Weld needs shapes that
  enclose an area. The selected object has no area. Nothing was changed." in the refusal chip, and
  both leaves inside the group get the red Refusal outline. Nothing changed.
- **Undo:** one Ctrl+Z after welding a text brings back the text, selected and editable (Enter
  then typing adds to it); after the squares, both rectangles come back selected. Notices read
  "Undid: Weld (1 text)." and "Undid: Weld (2 rectangles).".
- **Keyboard:** the card is one Tab stop; End lands on Weld; Space welds and keeps focus on the
  button; a mouse press returns focus to the canvas.
- No hint labels, nothing opens on a press, no new tokens or colours.

## Fixed in this review (folded into the feat commit and the docs commit)

1. `frontend/src/components/BooleanGlyphs.tsx`: the line-art glyph read as Lucide `Copy`, which is
   the Duplicate glyph of the Layers toolbar (`LayersTab.tsx:574`), at 1x and 2x. A sharp trim
   corner did not help. The glyph now has a solid dot of radius 1.6 on each of the two joints where
   the trim line meets the silhouette. At 1x it reads apart from Copy, from Intersection (two small
   dots against a solid overlap), from Union and from Flatten. `docs/design-system.md` (row
   "Command glyphs", including the pair Weld / Duplicate) and the UX notes ("Glyph") describe the
   dots.

## Observations, no change

- With a hidden lowest operand, the result takes that hidden shape's style. This is the stated rule
  (style of the lowest object), and the notice says so.
- A group of 2-node open lines shows "Replaces the selection." in the tooltip, then the no-area
  refusal: a 2-node line encloses no area when closed, so it is not counted as closable (`0054`).

## Screenshots

- `ux-screens/01-weld-only-bright-tooltip-800x640.png`: one rectangle selected, Weld the only
  bright button, its tooltip, column B at the minimum window.
- `ux-screens/02-two-squares-welded-notice-1280x800.png`: the result of criterion 4 and its notice.
- `ux-screens/03-no-area-refusal-in-group-1280x800.png`: the refusal chip and the red outlines on
  the leaves inside a group.
