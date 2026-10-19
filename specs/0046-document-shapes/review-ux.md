# UX review, story 0046 stage 1 (head 8545f8a2)

Verdict: CHANGES REQUESTED (1 must-fix, 4 should-fix). Driven in Chromium against the Vite dev build, 1440x900 and 800x600, light theme (the app has no dark theme yet, so none checked).

## Must-fix
1. First view after a template load clips the template (`review-ux/12-template-loaded.png`). Loading `guide_v15.curvyo` into an empty A4 document zooms to 72 %, but the guides extend above the canvas top edge (the selection frame and the arc at y < 0 are cut off) and sit under the floating Select bar. Criterion 13 says the first view of a loaded template includes the guides. Fix: fit the guides plus the page into the area free of the bars, with the same margin Fit to content uses.

## Should-fix
2. At 800x600 the Select bar grows from two rows to three when something is selected (`20-800x600-empty.png` two rows, `22-800x600-guide-pressed.png` three rows: Linked clone drops to row 3, bar 72 px to 108 px high). The UX notes say the bar keeps its two-row layout. The Guide button adds 28 px to row 2 and probably causes the wrap. Fix: trim the group gaps, or move Guide into row 1 where the width allows, so the bar stays at two rows at 800 px.
3. Dashed hairline reads as dotted, and faint at 100 % on the page (`07-guide-ellipse-unselected.png`). At 1345 % the apex of the arc shows no visible 2/2 gaps (checked at pixel level); at 100 % and 400 % the gaps show. Fix: verify the dash is applied to shallow curves at high zoom, and snap the hairline to a whole device pixel as the spec says (criterion 8).
4. A clone of a guide is drawn as a guide but the Guide button is greyed with the clone selected (`31-clone-of-guide.png`). Not wrong by the spec, but the maker cannot tell why. Decide: leave as is and say so in the UX notes, or show the button pressed and inert.
5. The Guide glyph (`SquareDashed`) is nearly the same as the Select all glyph in the left toolbar. Legible at 16 px, but easy to confuse. Low priority; a different glyph (a dashed line, for example) would avoid it.

## Checked, fine
- Guide button: after the Flip group behind a separator, 28x28, one Tab stop (order Flip h, Flip v, Guide, Object to path, Linked clone), accessible name "Guide", `aria-pressed` false/true, Space and Enter toggle, focus ring clear (`30-focus-ring.png`), focus returns to the canvas after a click. It uses `aria-disabled` rather than `disabled`, so the greyed button keeps its tooltip (`32-disabled-tooltip.png`); fine, opacity 0.4.
- States: empty selection greyed; ellipse and rectangle enabled; mixed selection not pressed, a press marks all, the next press unmarks all; Ctrl+Z and redo flip the state; history lists "Guide" with the notice "Redid: Guide (1 rectangle)." (`06-history.png`). Image selection not driven (no image import in the browser build in reach).
- Tooltip is the name only, after about 400 ms (`03-tooltip.png`). No hints, no label. Select bar has no new section.
- Template button: File tab, outline, 118x28, in the row next to Close, DOM order after Close, label and tooltip "Template" (`10-file-tab.png`, `11-template-tooltip.png`). Load result: new guides selected (4 objects, 80.0 by 80.0 mm), notice "Template loaded." in the status slot, focus back on the canvas, the existing circle untouched.
- Guide green `#166534`: judged good. Contrast on the pasteboard (about `#B7B7BD`) is 3.5:1, on the page `#E8E8EB` 5.8:1. The proposed `#15803D` would give 2.5:1 on the pasteboard and fail the 3:1 rule. Accept it; update the spec text and `docs/design-system.md` to say `#166534`.
