# Align and distribute: a row of symbol buttons in the Select bar

Status: In progress (implementation requested 2026-10-30; the defaults below apply)
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "We definitely need it too, similar to Inkscape but user-friendly. Inkscape's features are good but not easy to operate, and it takes long to understand them."

## User value

As a maker I want to line up several objects on an edge or centre and space them evenly with one click each, without first learning which of seven "relative to" settings is active, so that a row of coasters or a column of labels is tidy in seconds and I can see what a button will do before I press it.

**What other tools do.** Inkscape's Align and Distribute dialog: a "Relative to" dropdown (last selected, first selected, biggest, smallest, page, drawing, selection), six align buttons, a dozen distribute buttons (edges, centres, equal gaps, baselines), then Rearrange, Remove overlaps, Unclump, Randomise, Exchange positions, "Move/align selection as group", "Treat selection as group". All powerful, and the result of a click depends on settings elsewhere in the dialog. Illustrator: align to selection, key object or artboard, distribute by objects or by a typed spacing. Affinity: the same, with the key object marked by a thicker outline. Our version keeps the common ones, decides the reference by what is selected, and shows it.

## Acceptance criteria (as far as clear)

1. Given the Select tool and two or more objects selected, then the Select bar shows **Align** and **Distribute** as groups of symbol-only buttons (16 px glyphs, tooltip and accessible name each; no text, no popup, no dialog). With one object selected the Align group stays visible and acts on the active page (criterion 4); the Distribute group is not shown (nothing to distribute). **Amendment 2026-10-30 (customer):** with **nothing selected**, both groups stay in the bar and act on the **active page's objects**: align puts every object of the page onto the page (criterion 4's reference for all of them), distribute spreads them evenly across the page (criterion 6's modes, the page as the outer frame).
2. Given Align, then the buttons are: left edges, horizontal centres, right edges, top edges, vertical centres, bottom edges. Align left moves every selected object so its left edge lies on the **reference's** left edge; likewise for the others. Objects are moved whole (a group as one object; a compound path as one object).
3. Given two or more objects, then the reference is the **key object** if one is set (criterion 5), otherwise the **selection box**: the box around all selected objects, as the group box of `0019`. An object that already lies on the reference does not move, and an alignment that changes nothing records no history step.
4. Given one object, then the reference is the **active page** (`0059`; the document area while there is one page). So "centre this on the page" needs no setting.
5. Given a **key object**: with the Select tool, a click with Ctrl on an already selected object (Proposal) marks it as key; the mark is a thicker outline on the canvas and a second Ctrl click or Escape removes it. Alignment then leaves the key object where it is and moves the others to it. Whether to have this at all is Question 1.
6. Given Distribute, then the buttons are, per axis: equal gaps between objects, and equal distances between centres. Horizontal: objects are ordered by their position, the first and last stay, the others are spaced evenly between them. Vertical: the same. The glyphs show the pattern (bars with equal spaces).
7. Given a distribute with fewer than three objects, then it changes nothing and records nothing; the buttons stay (permit; no refusal message needed for a no-op). **Amendment 2026-10-30 (customer, "distribute funktioniert noch gar nicht"):** with a **typed gap** two objects are enough — the first stays and the second is spaced from it (criterion 8 needs no ends to define the spacing); without a typed gap the ends define it and three or more are still required.
8. Given **Space by a typed distance** (Question 2), then a value field next to the Distribute buttons takes a gap in mm and "equal gaps" uses it instead of the first and last positions. Empty means "keep the ends".
9. Given the boxes used, then they are the true bounding box of each object as shown in the selection box (`0056`), stroke width not included unless the true box includes it, so the buttons agree with what the maker sees.
10. Given a locked object in the selection, then it counts for the reference and for ordering but does not move (Proposal; the Layers tab owns the lock, `0050`). Hidden objects are never selected.
11. Given a linked clone (`0049`, `0057`), then it moves through its own placement matrix; an original is not moved by aligning its clone to it.
12. Given any align or distribute, then it is one history step with the operation name ("Align left", "Distribute horizontally", `0020`), undone as one; the selection stays.
13. Given the keyboard, then every button is reachable with Tab and activated by Space or Enter; shortcuts are a Proposal for the key table (R-SYS-009) and not assumed.

## Implementation interpretation (2026-10-30)

- The customer requested this story; unanswered ALIGN-KEY, ALIGN-GAP and ALIGN-GRID use their stated defaults A, A and A. Behaviour marked Proposal remains vetoable at the demo.
- [UX notes](ux-notes.md) define reversible interaction details and exact control behaviour.
- [Architecture notes](adrs.md) keep the operation in existing selection, transform and history boundaries. No format change is needed.

## Out of scope

- Rearrange on a grid or circle, Unclump, Randomise, Remove overlaps, Exchange positions (each is a possible later tool with its own trigger; **Arrange in a grid** is the likely first, because of the coaster-sheet use case; it also belongs with nesting, R-MFG-004).
- Align text baselines, align nodes in the Node tool (the Node tool has its own rules, `0053`, `0055`).
- Align to the drawing's box or to the biggest or smallest object as a separate reference (the selection box and the key object cover them).
- Keeping spacing while objects change size (constraints).

## Architect notes (2026-10-17)

- No format change and no new crate. The command is a pure function `(boxes, reference, axis, mode) -> translations` in `ui-core` or `document-core`, applied as one commit through the existing move writer, so linked clones and compound paths need no new rule.
- The key object is view state (per user, never undoable), like the selection.

## Open questions (customer; each has a default)

Frage 1 (ALIGN-KEY): Do we offer a key object (Ctrl-click, thicker outline), as Illustrator and Affinity do?
- A (default, recommended): yes; it removes the "which one stays" question without a setting.
- B: no; the reference is always the selection box (simplest, nothing new on the canvas).
- C: yes, but the key object is simply the last clicked object, with no extra gesture.
- Default if unanswered: A.

Frage 2 (ALIGN-GAP): Do we add the typed gap field next to Distribute (criterion 8)?
- A (default): yes, one value field in mm.
- B: no; equal gaps between the ends only.
- Default if unanswered: A.

Frage 3 (ALIGN-GRID): Is "Arrange in a grid" (rows, columns, gap) part of this slice?
- A (default): no; a follow-up entry, to be built with page and nesting work.
- B: yes, as a third group of buttons plus two value fields; a bigger slice.
- Default if unanswered: A.

## Links

Requirements: R-EDIT-040 (new)
Depends on: `specs/0019-multi-object-transform/` (the group box), `specs/0056-mirror-and-negative-scale/` (buttons in the Select bar, true box)
Related: `specs/0059-multiple-pages/`, `specs/0060-snapping/`, `specs/0049-linked-clone/`, `specs/0050-layers-and-objects-panel/`, `specs/0020-undo-redo/`, `docs/design-system.md`
ADRs: none expected
PR: -
