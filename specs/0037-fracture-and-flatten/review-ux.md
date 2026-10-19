# UX review: 0037 Fracture and Flatten

Reviewer: ux-engineer, 2026-10-19. Branch `story/fracture-and-flatten`, driven live in Chromium
(Playwright, `vite preview` of `frontend/dist`) at 1280 x 800 and 800 x 600. Light theme only.

**Verdict: APPROVE** after the glyph fix below. No MUST-FIX, no SHOULD-FIX, no customer question.

## Checked live

| Area | Criteria / UX notes | Result |
|---|---|---|
| Placement | Placement | Pass. Buttons 6 and 7 of the Boolean card, after Reverse difference; 40 x 40 px, 4 px pitch as the five above. Card 312 px (y 224 to 536), column B 36 to 536 px |
| 800 x 600 | Placement | Pass. Card ends at 536 px, 40 px above the status bar; nothing clipped. The spec's "ends 512 px" was a miscount and is corrected to 536 |
| Dimming | 1, States | Pass. One object, or the Rectangle tool with two selected: `aria-disabled="true"`, glyph at 40 %, focusable, the tooltip opens, a click does nothing |
| Tooltips | 1, Tooltips | Pass. "Fracture" / "Cuts overlapping shapes into pieces." and "Flatten" / "Trims each shape to what shows.", with the shared note ("Select two or more objects.", "... Use the Select tool.", "Replaces the selection."). No extra sentences, no operating tips |
| Fracture | 6, 9 | Pass. Two overlapping squares: "Fracture: 2 objects became 3 pieces.", 3 paths selected, tool stays Select; the notice sits level with Union |
| Flatten | 10 to 14 | Pass. Squares: "Flatten: 2 objects became 2. Strokes follow the trimmed outlines." Small disc under a large one: "Flatten: 2 objects became 1. 1 hidden object was removed." |
| Refusals | 2, 3, 5 | Pass. Touching squares: "Fracture: the selected shapes do not overlap. Nothing was changed.", no outline. A group in the selection: "Flatten does not work on groups. Ungroup first. Nothing was changed.", group's shapes in the red outline. A two-node pen line: "Fracture needs shapes that enclose an area. 1 of 2 selected objects has no area. Nothing was changed.", the line outlined. Refusals use the alert style of the Boolean card |
| Undo | 9, 14 | Pass. Ctrl+Z after each: "Undid: Fracture (2 rectangles)." / "Undid: Flatten (1 ellipse).", selection restored in one step |
| Keyboard | 1 (`0016` 22a) | Pass. One Tab stop, End reaches Flatten, ArrowUp Fracture, Enter applies and keeps focus on the button; a mouse press returns focus to the canvas |
| Live region | `0019` | Pass. Selection sentence follows after its 500 ms settle ("3 objects selected, ...") |
| Customer rules | | Pass. Two commands on an existing card, no panel section, no option, no dialog; no hint text added |

## Fixed in this review

- **Glyph gap** (`frontend/src/components/BooleanGlyphs.tsx`, `GAP`): 1.4 units (1.75 px) raised to
  1.6 units (2 px). The pieces are filled without a stroke, so the geometric gap is the visible
  gap; the 2.8-unit rule of "Command glyphs" applies to stroked neighbours, whose strokes eat
  1.2 units of it. 2 px meets "no visible feature is under 2px", the pieces stay inside the
  viewBox (lowest coordinate 0.15), and on a 4x-enlarged 1x sheet the Fracture gaps and the
  Flatten trim edge read as clean lines; Fracture / Flatten and Exclusion / Fracture read apart.

## Not checked

- Dark theme and WebKitGTK (Tauri): glyphs use `currentColor`, nothing story-specific.
- Large documents: covered by the tester (fixture (b), 2,000 squares).

## Screens

- `ux-screens/01-fracture-tooltip-800x600.png`: card at 800 x 600, Fracture tooltip, two squares selected.
- `ux-screens/02-fracture-notice-800x600.png`: result, notice level with Union.
- `ux-screens/03-flatten-group-refusal-800x600.png`: group refusal with the red outline.
