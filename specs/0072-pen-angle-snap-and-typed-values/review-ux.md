# UX review: 0072 pen angle snap and typed values

Reviewer: ux-engineer, 2026-10-19. Branch `story/pen-angle-snap-and-typed-values`, driven live
in Chromium 154 (Playwright, `vite preview` of `frontend/dist`) at 1440 x 900 and 800 x 600.
Light theme only: the app did not follow `prefers-color-scheme: dark` in this run, so dark was
not checked.

**Verdict: APPROVE** after the width fix below, with two SHOULD-FIX items for the lead and one
question for the customer. No MUST-FIX.

## UX notes

The PO draft in `specification.md` ("UX notes") is confirmed. It now gives the field widths
the review measured (154 px and 150 px instead of 132 px; see "Fixed in this review").

## Checked live

| Area | Criteria | Result |
|---|---|---|
| Ctrl lock, rubber band | 1, 2, 7 | Pass. (400,300) to (520,309): Ctrl makes the band exactly horizontal; a Ctrl-click places the node on L's y |
| Ctrl press and release at rest | 4 | Pass in Chromium: the band snaps on key-down and returns on key-up with no pointer move. **WebKitGTK untested** (key-up delivery, the known `0014` case) |
| Handle snap and drag readout | 3, 8, 35 | Pass. Ctrl drag: "21.0 mm, 15°" (raw 11.6° to the 15° stop); without Ctrl "21.3 mm, 12.2°". Sits 12 px up-right of the pointer, units on both numbers, gone on release; the node stays where it was pressed |
| Tab opens the chip | 12, 13, 34 | Pass. Group "Next node", focus in "Length, millimetres" with its text selected, live region says "Next node." With no path, Tab is not consumed and moves focus in the browser order |
| Field order and Tab cycle | 18 | Pass. Length, Angle, wrap to Length; Shift+Tab goes back; the received field's text is selected; Tab does not freeze |
| Follow and freeze | 15, 16, 17 | Pass. Live values follow the pointer; the first edit freezes values and position; the band runs to the typed point; after Ctrl+wheel zoom the frozen chip moves with L |
| Enter, Escape, keys in the chip | 19, 22, 23 | Pass. "Node added, 40 millimetres at 30 degrees."; focus back on the canvas; Escape closes; "r" and Ctrl+Z stay in the field |
| Notices | 24 to 29 | Pass. "Not a number. Used 49.0 mm.", "Length must be more than 0." (chip stays, Length selected), "Negative length: drawn in the opposite direction.", "Too far away. Length limited to the document limit."; "1in" with 450° adds 25.4 mm at 90° with no notice. Canvas notice slot, bottom centre, text only, no key hints |
| Right press finishes | 19a | Pass. Two nodes plus chip (unfrozen or frozen): right press finishes with 2 nodes and adds no typed node, no menu. One node: nothing happens, same as Enter (`PenTool::finish` needs 2 nodes) |
| Right press, no path | 19a | Unchanged from before the story: it **starts a path** at the press (see question C1) |
| 800 x 600 | 33 | Pass after the fix: chip 320 x 40 px, inside the canvas at all four corners and along the right edge, never over L, 9 digits not clipped |
| Cursor | UX notes | Pen nib on the canvas with the chip frozen; text cursor in the fields |
| Design system, customer rules | 13, 36 | Placement chip surface and `EntryField`, no hint row, no panel section, no bar, no button, no toggle; the rail tooltip is unchanged; three shortcut-table rows added. Matches "operations are tools" and "no hint labels" |

## Fixed in this review (folded into the frontend commit)

1. `frontend/src/components/PenEntryChip.tsx`: both fields had 46 px of text room (132 px
   minus a 52 px label padding, the 32 px suffix padding and the border); 9 digits at 14 px need
   70 px, so "123456789" scrolled inside Length (criterion 33 failed). The fields are now 154 px
   (Length, 48 px label padding) and 150 px (Angle, 44 px as in the placement chip): 72 px of
   text each, chip 320 px, still inside the 336 px right of the tool rail at 800 x 600. The row
   in `docs/design-system.md` and the UX notes say so.

## SHOULD-FIX

1. `frontend/src/components/PenEntryChip.tsx:93-100`. After the freeze the chip stays at the
   placement it had for the pointer, but the typed point can lie under it: L = (500,300), pointer
   (600,250), typed 40 mm at -30° ends at (631,224), under the chip (screenshot 03). The maker
   cannot see where the node will go. Fix: while frozen, if the chip's rectangle intersects the
   segment from L to the typed point, use the rule's existing fallback (the other side of L on
   the vertical axis, then clamp), checked on each edit; no transition. Criterion 16 already
   says the chip never covers the rubber band.
2. `frontend/src/components/PlacementChip.tsx:9-12` (story `0052`, not this branch). The same
   arithmetic: Angle 132 px with a 44 px label leaves 54 px, so the design-system claim "widths
   hold 9 characters" is false there too (the r/⌀ size field: 124 - 28 - 34 = 62 px; W and H
   with 20 px keep 70 px and are fine). A `fix/` PR that widens Angle and r to 72 px of text.

## Question for the customer

- **C1 (PEN-RIGHT-IDLE).** With no path in progress, a right press in the Pen starts a path at
  the press (it was so before this story; 19a keeps it). A second right press then does nothing,
  because a one-node path cannot be finished, so the maker is left with a started path until
  Escape. Options: A) a right press with no path does nothing, as in the shape tools (`0052`
  criterion 7); B) keep it. Recommendation A. Default if unanswered: B (keep), as 19a says.

## Screenshots

- `ux-screens/01-ctrl-lock-at-rest.png`: Ctrl held at rest, the band horizontal from L.
- `ux-screens/02-drag-readout-ctrl.png`: Ctrl drag, handle on the 15° stop, readout "21.0 mm, 15°".
- `ux-screens/03-entry-chip-typed.png`: the frozen chip, 40 mm at -30°; the typed point under
  the chip (SHOULD-FIX 1).
- `ux-screens/04-chip-800x600-near-edge.png`: 800 x 600, L near the bottom-right edge, 9 digits.
