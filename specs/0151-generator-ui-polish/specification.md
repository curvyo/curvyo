# Generator UI polish: nothing armed by itself, arm and disarm by hand, then a better look

Status: **Slice 1 Done (PR #TBD, 2026-11-27)**; later slices Draft (customer reviews the UX preview first)
Priority: Should
Origin: Customer (2026-11-25, after PR #481: "functional now, the UI needs another pass"; 2026-11-27, translated: "nothing in the Generators tool is armed automatically"). Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As a maker I want the Generators tool to do nothing until I choose a generator, and to let me choose, change my mind and leave again with one click or Escape, so that I never place a box or gear I did not ask for.

## Relation to neighbouring specs

- **`0141` parametric generators** (Done, PR #455, revision 2): the Generators tool, its bar and the placing of a generated object. **Slice 1 revises criterion 1 of `0141`** (see Revised criteria). `0147` criterion 1 (Rounded box segment), `0149` criterion 1 (Box from path segment) and `0144` criterion 5 (Code segment) say "pressing it places ... at the pointer"; they read as **arm the segment, then place** from this slice on; their own text is not rewritten.
- **`0052` click to create shapes:** the same pattern (arm, click or press Enter on the canvas, one object); a generator behaves like a shape tool.
- **`0144`, `0147`, `0149`:** their segments are the segments of this bar; nothing here changes their parameters.

## Acceptance criteria (slice 1)

1. Given the maker presses the Generators tool (More tools card), then **no segment is armed**: the bar shows all segments (Box, Gear, Rounded box, Box from path, Code) unpressed, the pointer is the ordinary arrow, a click or Enter on the canvas places nothing, and no object is created or selected by entering the tool. Today the tool arms its first segment on entry; that is what changes.
2. Given no segment is armed and the maker presses one segment, then that segment is **armed** (pressed look, the pointer shows the placing cursor of `0052`); the bar's hint line names what the next click does ("Click the canvas to place a box.", **Proposal**); a click on the canvas (or Enter, `0141` UX notes section 2: at the pointer if over the canvas, else the centre of the visible canvas) then places one object with its defaults, selected, parameters in the Properties panel, one undo step "Add box" and so on as before.
3. Given a segment is armed, when the maker presses **the same segment again**, then it is **disarmed**: unpressed, ordinary pointer, nothing placed, the tool stays active; given another segment is armed, pressing a different segment **moves** the arming to it (exactly one armed at a time).
4. Given a segment is armed or not, when the maker presses **Escape**, then an armed segment is disarmed first (tool stays); a second Escape leaves the tool for Select, as the Escape cascade of `0141` UX notes section 2 says today (Proposal: the cascade gets one more step in front, nothing else changes).
5. Given a segment was placed once, then it **stays armed** (lead decision 2026-11-27 **(Proposal)**, replacing "disarmed after one placement, Shift keeps it armed"): the tool stays active, the new object is selected so the maker edits it at once, and the next click or Enter places another one; the maker releases the segment by pressing it again or with Escape. **Box from path** (`0149`) is armed only while a single closed object is selected; with another selection it is disabled with the hint "Select one closed shape first." and arming is refused (unchanged rule, now shown in the hint).
6. Given the bar, then every segment has a **tooltip** with its name, what it makes in one line and its shortcut if any ("Box: a laser box with finger joints"), and the hint line of criterion 2 is the single place the armed state is also written in words (not colour alone); each segment is reachable and operable by keyboard (Tab, Space or Enter to arm, Escape to disarm) and the pressed state is announced (`aria-pressed`).
7. Given save and reopen, then nothing about arming is stored (it is tool state, not document state); given the tool is left and entered again, then it starts unarmed again (criterion 1).
8. Given the gate, then acceptance tests cover criteria 1 to 5 (entry arms nothing; arm, place, undo; toggle off; switch; Escape cascade; stays armed after a placement; Box from path disabled without a closed selection) and a keyboard-only run of criterion 6.

## Built in slice 1 and left out (Proposal, 2026-11-27)

Built: criteria 1, 2 (arm), 3, 4, 5 (stays armed), 7 and the `aria-pressed` toggle buttons with Left, Right, Home and End moving
the focus without arming. **Not built:** the hint line of criteria 2 and 6 (the UX preview section 1.1 item 9 says "no hint
label, no notice"), the per-segment tooltips (only the Loading tooltip of the Code segment), the "Select one closed shape
first." hint and the keyboard-only acceptance run of criterion 6. A **Box from path** pick (`0149`) still ends in Select as
before; only the pointer-placed generators stay armed.

## Revised criteria (accepted behaviour that changes, customer-decided 2026-11-27)

- **`0141` criterion 1** ("pressing one places a generated object at the pointer") becomes: pressing a segment **arms** it; the **next click or Enter on the canvas** places the object (criteria 2 and 5 above). Entering the tool arms nothing.
- `0141` UX notes section 2 (Keyboard, Escape): the Escape cascade gets the step "disarm" first; the architect or UX engineer updates the notes text, not this spec.

## Later slices (Draft, not decided)

The UX engineer's preview `specs/0151-generator-ui-polish/ux-preview.md` (in progress, not yet reviewed by the customer) shows the concepts for the rest of the pass: the look of the Generators bar and the Generator block, Box from path pick mode, and the canvas marks of the rounded box and the path box. **No criterion for them is written until the customer has reviewed the preview and chosen**; the product owner then writes slices 2 and following from the decisions.

## Out of scope

- Any change to what a generator makes, its parameters, the file format (arming is tool state only).
- New generators; arming for other tools (Offset, Measure, Grid keep their behaviour).
- The later look-and-feel slices (above).

## Questions for the customer

None for slice 1. "Stays armed after a placement" (criterion 5) and the placing by click, not at press, are Proposals; veto at the demo.

## UX notes

Slice 1 needs none beyond the hints and tooltips of criteria 2 and 6; the UX engineer's preview (`ux-preview.md`) covers the later slices.

## Links

Requirements: R-EDIT-055
Related: `0141`, `0144`, `0147`, `0149`, `0052`
PR: same PR as `0144` slice 4 (customer 2026-11-27: all in one PR), #TBD
