# UX review: 0074 text objects

ux-engineer, 2026-10-20. Branch `story/text-objects` (4 commits), browser build (`vite preview`,
Included fonts only), headless Chromium at 1280 x 800 and 800 x 600. Checked against the UX notes
in `specification.md` and `docs/design-system.md`.

**Verdict: CHANGES REQUESTED** (no blocker for the demo; two SHOULD-FIX items are behavioural, the
third affects only the desktop font list).

## Checked and fine

- Text tool: slot 7 of the Tools card, Lucide `Type`, tooltip "Text tool (T)", `T` selects it; the
  I-beam cursor over empty canvas and over objects.
- Edit mode: entry by click, by double-click with Select (switches to the Text tool), by Enter on a
  selected text (caret at the end). Exit by Escape, click outside (starts the next text), tool
  switch. One history step per session ("Add text", "Edit text"); an empty session writes nothing;
  Undo takes a session back in one step. Second Escape: the Select tool with the text selected.
- Caret: 1.5 px accent line with white casing; selection band in accent at 30 %, caret hidden while
  a range is selected; a line break inside the selection shows its band.
- Space types a space; Tab moves to the Font button, then the Size field; Escape in the bar returns
  focus to the edit sink with edit mode kept.
- Text bar: 316 x 36 px, Font button 168 x 28, Size 120 x 28 in mm, one row at 800 x 600. Select
  bar: the Text group after Guide and before Object to path (4 rows at 800 x 600, as planned).
- Fonts tab: sixth tab, `CaseSensitive`; Font button opens it with focus on the current face row;
  rows set in their own face; search filters, "3 of 120" style counts, empty state
  "No font matches "zzz"."; Up, Down, Left, Right, Home, End, Page Down move focus only; Enter and
  click pick; a printable key goes to the search field; a mouse pick returns focus to the sink.
- Size clamp: "Size set to 2000 mm, the largest size." under the field.
- Missing font (golden `text_v17.curvyo`): canvas tag "Font missing", Font button with the alert
  glyph and "missing", tooltip second line, "Missing" block first with the current mark. Object to
  path refused with "Font Missing Font Bold is missing. Nothing was changed." and the refusal
  outline. Picking Source Serif 4 replaces the face, the tag goes, Object to path gives a group of
  one path per letter.
- No hint labels, no popups, nothing appears from nowhere; Layers row "A long house name Street…".

## Fixed in this review (folded into the feat commit)

1. `frontend/src/components/panel/FontsTab.tsx`: the current-face bar and the collapsed-family
   diamond used `var(--accent)`, which in the code is the shadcn near-white token (`index.css:62`),
   so the 3 px bar was invisible. Now `var(--editor-accent)`, as the Layers row bar.

## SHOULD-FIX

1. **First keys after a click can go to the tool letters.** `frontend/src/hooks/useText.ts:251`
   focuses the sink in a `requestAnimationFrame` after the sync. Keys typed within about 30 ms of
   the release land on the canvas container: with "erp" typed at once, 3 of 5 starts switched to
   the Ellipse or Rectangle tool (swiftshader, slow frames; a large document makes frames slower).
   Fix: focus the sink synchronously in the pointer-up path when the session enters edit mode, or
   keep focus on the sink by `preventDefault` on the canvas `pointerdown` while in the Text tool.
2. **No hover box in the Text tool.** UX notes §1 "Hover": a text under the pointer shows
   `--hover-box`. `text.rs:276` sets `hovered_object`, but
   `curvyo-editor-wasm/src/session/select_view.rs:131` returns no hover for any tool but Select.
   Fix: let `Tool::Text` through for the hovered box (no selected boxes, no handles).
3. **Block headers are not sticky.** UX notes §4: the header stays at the top while its rows
   scroll. `FontsTab.tsx` (header row near line 403) is a plain virtualised row. Invisible in the
   browser (one block, 3 families), it matters on the desktop with "This computer". Fix: draw the
   header of the block at the scroll top as an overlay row above the list.
4. **Object to path refusal posts in the canvas notice slot** (`useText.ts:540`,
   `showNotice(..., "canvas", ...)`), not 4 px under the Object to path button (UX notes §4, §6).
   Fix: post it as the Select bar's anchored notice, as the Linked clone notice does.

## Note (no change)

- "Source Sans 3 Regular", the default face, is always cut at 168 px ("Source Sans 3 Re…"). The
  tooltip and the Fonts tab header carry the full name; widening the button breaks the one-row fit
  at 800 x 600. Kept as specified.

## Screens

- `ux-screens/01-edit-mode-selection-1280x800.png`: edit mode, Text bar, selection band.
- `ux-screens/02-text-bar-800x600.png`: Text bar on one row at 800 x 600.
- `ux-screens/03-fonts-tab-select-bar-800x600.png`: Select bar Text group, Fonts tab, current mark.
- `ux-screens/04-missing-font-refusal-1280x800.png`: tag, Missing block, refusal notice.
