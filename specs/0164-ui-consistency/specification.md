# UI consistency

Status: Done (PR #529); slices S1 to S7 in one PR (customer, 2026-11-27). Frage 461 (LASER-BAR-FOLD) answered A.
Priority: Should
Origin: Customer (2026-11-27)

## User value

As a maker who spends hours in Curvyo I want every button, switch, slider, field and canvas
label to look and behave the same way wherever it appears, so that I recognise a control at a
glance, see what is on, and never mistake a button for a label or lose a control off the screen.

Customer request (2026-11-27, translated): "Review the complete UI and straighten it out: buttons
uniform, sliders uniform, toggle buttons always shown by pressed state + colour instead of the
sliding switch, all buttons with symbols, ... 'Add page' and 'Cancel' are not recognisable as
buttons. On the laser page the canvas display of the operations is not properly in a pill and not
centred."

## Acceptance criteria

One per slice of [`fix-plan.md`](fix-plan.md); each holds at 1440 x 900 and at 800 x 640, and
every changed control is reachable by Tab and shows the 2 px focus ring.

1. **S1 Buttons.** Given any tab of the right panel, any context bar and the Add page form, when
   the maker looks at a control that performs an act, then it is either outlined (secondary),
   filled blue (primary) or red-outlined (destructive), carries a symbol left of its label (Frage
   417), and no borderless text acts as a button; in Add page, Cancel shows `X` and Add is the blue
   primary with `Plus` inside a framed form.
2. **S2 Toggle buttons.** Given any on/off setting in the panel, a bar or an entry chip, when the
   maker presses it, then it is a button that turns blue with a white symbol and label while on
   and back to outlined when off (Frage 452, 420), Space toggles it (Enter too, except inside an
   entry chip, where Enter keeps committing the chip as every chip does; lead decision, Proposal),
   and no sliding switch remains anywhere in the app.
3. **S3 Canvas symbol pill.** Given an object on a laser page in View: Operations that is in one
   or more operations, when the canvas is drawn, then the object shows one rounded pill holding one
   symbol per kind, each symbol centred in its cell, the pill centred above the object (Frage 455,
   432) and never covering a transform handle.
4. **S4 Bars and rail.** Given the Select tool with nothing selected, then its bar is one 36 px
   row; given any bar at 800 x 640, then no part of it extends past the canvas or under the panel,
   no divider starts or ends a row, no canvas tag is drawn over it (Frage 457); and given a window
   640 px high, every tool of the rail can be reached by scrolling the rail.
5. **S5 Choices and sliders.** Given any "pick one of a few" control, then it is one bordered strip
   whose chosen item is blue with white content and arrow keys move the choice; given any slider,
   then it has the one track, fill and thumb look; and the panel opens no system dropdown (Print
   and cut Sheet included).
6. **S6 Fields and focus.** Given any number, text, search or rename field, then it has a white
   ground and a visible 1 px border of at least 3:1 against its surface, one label placement per
   surface, no browser steppers; and every focus stop, including the Pages list and the panel body,
   shows the same ring.
7. **S7 Tooltips and notices.** Given any control with a tooltip, when it is hovered or focused by
   keyboard, then one tooltip look opens with the control's name, shortcut and, where it has one,
   how to operate it (Frage 458 A); the canvas hint chips and the slider help line stay (Frage 454 A);
   every canvas notice uses one look, and result notices that only report counts or chatter after an
   action are removed or shortened (customer 2026-11-27: the UI surface is not verbose).

## Out of scope

- A dark theme (none exists; `docs/design-system.md` has one light theme).
- New controls, features or shortcuts; default object names in the Layers tab (W40).
- Machine controls, Trace and image adjustments beyond the shared components (not reached in the
  walkthrough; checked at the UX review of each slice).

## UX notes

The target look of every element is [`docs/ui-elements.md`](../../docs/ui-elements.md). Findings:
[`inventory.md`](inventory.md) (code, `I1` to `I39`), [`walkthrough.md`](walkthrough.md) (browser,
`W1` to `W52`, `screens/`), merged in [`fix-plan.md`](fix-plan.md) (`F1` to `F58`, slices S1 to S7,
Fragen 452 to 420 and 430 to 432 with defaults).

## Links

Requirements: none new (UI quality across all features). ADRs: none expected (no crate, format or
dependency change; S3 asks the architect only if the pill needs handle boxes from
`curvyo-editor-wasm`). PR: -

## Answers (customer, 2026-11-27)

- Frage 452 SWITCH-LOOK: **A**. Frage 453 BTN-SYMBOL: **A**. Frage 455 OP-PILL: **A**. Frage 456 CHECK-TOGGLE: **A**.
- Frage 457 BAR-NARROW: **A** (bars keep wrapping into more rows, never past the canvas; no "More" menu).
- Frage 458 TIP-TOOLTIPS: **A** (tooltips keep their instructions).
- Frage 459 PILL-ANCHOR: **A** (centred above the object, 6 px over its top edge; at the corner while handles are shown, Proposal).
- Frage 454 HINT-KEEP: **A** (2026-11-27): the handle, edit, Pen and key hint chips and the slider help line stay. The no-hint rule is against a verbose UI surface, not against explaining: how to operate something may stay in tooltips (and an info panel); what goes is status and result chatter after an act ("how many paths came out", counts). S7 (Proposal) lists such verbose result notices as findings to remove or shorten.
- All seven slices are built one after another on this branch and land in **one PR** (customer: fewer PRs, test it properly once).
- Frage 460 BAR-FOLD (customer, 2026-11-28): **B**. In a mixed selection the Select bar shows only the controls every selected kind shares (kind-specific groups such as corner radius, points or font only when one kind is selected), with one shared "Gap" field; in addition, below 2 selected objects Align and Distribute are not shown in narrow windows (800 px class). This changes accepted behaviour of `0009` and `0061` (their criteria get an amendment line in this PR).

## Open question

Frage 461 (LASER-BAR-FOLD): the laser groups in the Select bar at 800 px (operation kinds, Bridge with Mode, Width, Reach, Join all) make one rectangle 7 rows and two rectangles 9.
- A: show the Bridge values only while Bridge is needed; the Bridge button stays.
- B: the laser groups as symbols with tooltips, values in a chip after a press.
- C: leave it.
Recommendation A. Default C (this PR does not wait; A would be a commit in the same PR).

Answer (customer, 2026-11-28): Frage 461 **A**. Built as (lead decision, Proposal): the Bridge button stays in the Select bar and applies at once as before (`0156` Frage 395); next to it an icon toggle "Bridge options" shows Mode, Width, Reach, Join all and Bridges per island in the bar while it is on (off by default, remembered for the session); the values stay remembered as before.
