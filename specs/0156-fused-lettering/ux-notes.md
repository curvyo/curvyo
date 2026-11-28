# 0156 Bridged lettering: UX notes

ux-engineer, 2026-11-26; revised by the product owner 2026-11-26 after the customer's answers (Frage 395
B-like: no entry chip, no canvas preview; Bridge applies at once; its values are fields in the Select
bar; Frage 398 C: Join all switch) and the build (`plan.md` deviations). Written on top of
`docs/design-system.md` (rows "Boolean toolbox", "Tool rail architecture", "Rail keyboard model", "Rail
tooltip and notice placement", "Command glyphs", "Boolean glyphs", "Boolean tooltip", "Action notice",
"Primary button"), `specs/0101-laser-operations/ux-notes.md` (sections 6, 7, 13) and the built Weld
button. No new token, no new colour. **(Proposal)** items are vetoable at the demo.

No operating hint or tip text anywhere (customer rule of 2026-10-16): the Select bar group and the card
carry labels that name a control or a state, nothing else.

## 1. Principle

Bridge is a Boolean-card command with a few numbers. The numbers sit in the Select bar as ordinary
fields while the Select tool is active on a laser page; one press on the card button or the bar button
writes the result at once. Afterwards the result is ordinary objects: the maker finds the bridges in the
Layers tree and on the canvas and assigns them like any item.

## 2. The button (criteria 1, 2)

- **Place.** The ninth button of the Boolean card, after Weld, **laser pages only** (a laser page with
  the extension on, `0098`). On every other kind the button is **absent**: no slot, no dimmed button,
  the card is 356px high as today; on a laser page it is 400px (44 x 9 + 4). Because it is the last
  button, its absence moves nothing (the row rule "a new button is appended"). Switching the active page
  changes the card in the same render (row "Page activation"); a focused Bridge button that disappears
  hands focus to the canvas.
- **Rail height.** Column B on a laser page ends 600px below the viewport top (12 + 180 + 8 + 400),
  18px past the 582px the 800 x 640 window leaves. **Nothing is built for it** (Frage 403, customer
  2026-11-26: the rail wraps by itself): column B does not scroll, no card is clipped or reordered, the
  rail is as before apart from the Bridge button. Criterion 2's "4 px" bound is therefore not
  met in the smallest window; the rails rework (`docs/technical-debt.md`, "Rails scroll and a single
  column on small screens") remains the place for it.
- **Look and rules.** Those of the row "Boolean toolbox": a command, never the active tool, no
  `aria-pressed`, 40 x 40px, glyph 20px, `--toolbar-icon`, hover `--editor-accent-hover`, the 2px
  focus ring. **Enabled** with the Select tool active and one or more objects selected (Weld's rule, one
  function for both, so the two never dim differently); **dimmed** otherwise (`aria-disabled`, 40%,
  focusable, a press does nothing). No shortcut, no letter, no menu entry.
- **Select bar twin:** the same command is also a **Bridge** button in the Bridge group of the Select bar (section 4); both run the same function.
- **Glyph** (FU1; art finalised on the glyph sheet at 1x, 1.25x and 2x, greyscale, as every command
  glyph): the dot of an i held to its stem. Solid pieces without a stroke (as the Fracture pieces, so
  the geometric gap is the visible gap), 16 x 16 viewBox at 20px: the **stem** a rounded rectangle
  x 5 to 11, y 8 to 15, radius 1; the **dot** a disc centred (8, 3.25), radius 2.25; the **neck** x 7
  to 9 from the dot into the stem, with concave fillets of radius 1 where it meets both. Air beside the
  neck is 2.5 units (3.1px), the neck 2 units (2.5px): both above the 2px minimum. Fallback if it reads
  as a letter or as Info at 1x: two solid 5.5 x 7 rounded blocks side by side, 3.5 units apart, joined
  at mid-height by the same filleted neck. Pairs that must read apart at 1x (added to "Command glyphs"):
  Bridge / Weld (solid pieces with a neck against a hollow outline with a trim line), Bridge / Fracture
  (joined against gapped), Bridge / Split (a neck against a hole), Bridge / Union, Bridge / the Text tool's
  `Type`.

## 3. Tooltip (criterion 1)

The "Boolean tooltip" surface, three lines, opening past the rail's right edge, also on keyboard focus.

- **Name:** "Bridge" (also the accessible name; issue S1 on the name).
- **Rule** (at most 45 characters, never wraps): **"Bridges loose pieces so they hold together."**
  (43). It names what is new against Weld's "Keeps each visible line once, in one object.": Weld removes
  lines, Bridge adds material.
- **Note**, first match wins: dimmed, "Select one or more objects." (with another tool followed by "Use
  the Select tool."); else "Replaces the selection." No sentence about text conversion (criterion 6; the History step and Ctrl+Z are the record).

## 4. The Bridge group in the Select bar (criteria 3, 5, 21; Frage 395, 398 C)

- **Where:** the Select tool's context bar, only while a laser page is active and Bridge is available
  for the selection (Weld's rule, one function). Absent otherwise, not dimmed.
- **Slice 1 (Lettering):** fields "Width" and "Reach" (mm, like the bar's other number fields), a switch
  "Join all" (default off; on, Reach is not used), then the **Bridge** button.
- **Slice 2:** a "Mode" two-segment control "Lettering | Stencil" (S7); in Stencil the fields "Width" and
  "Bridges" (integer, accessible name "Bridges per island", 1 to 4). Reach and Join all are hidden, not
  disabled.
- **Bridge options (`0164` Frage 461 A, 2026-11-28):** the group starts with the **Bridge** button, then
  an icon-only toggle "Bridge options" (shared `ToggleButton`, Lucide `SlidersHorizontal`, tooltip
  "Bridge options"). Mode, Width and the mode's fields follow only while it is on; off by default, kept
  for the session (not across restarts). The button and the toggle never move when the fields appear.
- **Values:** an app preference; first use Width 1.0, Reach 5.0, Join all off, Mode Lettering, Bridges 2.
  A typed value outside the range is clamped on commit and the field shows the limit; the notice "Width
  set to 10 mm, the maximum." shows under the group for 3 s in the status look, never `aria-invalid`. A
  field with no number is the one error ("Enter a number", `aria-invalid`, 2px `--field-invalid`
  border); the buttons stay dimmed until it holds a number.
- **Applying:** both buttons run the command with the current values; the rail's busy rules apply.
  Success: the result is selected, no success notice (customer rule of 2026-10-31). Refusal: the usual
  rail notice in the "Action notice" refusal look, offenders in the "Refusal outline" where `0016` draws
  them. Texts are the spec's (criteria 4, 14, 17, 22).
- No chip, no preview, no state line, no second surface (customer answer 2026-11-26).

## 5. The result in the document (criteria 11, 13, 14, 26)
- **Names** : the group "Bridged lettering" (Stencil: "Bridged stencil") with
  two members in this order, top to bottom in the Layers tree: **Outline** on top, then **Bridges** below it. Each
  bridge path is named "Bridge 1", "Bridge 2", ... in the deterministic order of criterion 18
  (FU6), so the names are stable and the golden files can hold them.
- **Kind glyphs** (row "Kind glyphs", no new glyph): the group and Bridges `Group`, the Outline
  `Spline` or the compound path glyph, each connector `Spline`. Row names: "Bridged lettering, group",
  "Outline, compound path", "Bridges, group", "Bridge 1, path".
- **Layers after Bridge** (FU7): the "Bridged lettering" row is selected and **expanded one level**, so the
  Outline and Bridges rows are visible at once; Bridges stays collapsed (up to 2000 rows behind
  it); the list scrolls the group row into view. This is the one-press route to the bridges: press
  "Bridges" to select every bridge, expand it to pick one.
- **On the canvas:** the ordinary group rules of `0023`. A press selects the whole result; a
  double-click enters it and a press then selects Outline or Bridges; a double-click on Bridges
  enters it and a press selects one bridge. No new gesture.
- **Style in View: Drawn** (issue S10): the Bridges take the Outline's style, so the drawing looks
  as one piece and only the short seams where a bridge meets a stroke show with a stroked style.

## 6. "Not assigned" and assigning the bridges (criteria 13, 14, 20)

- No new UI. The bridges start **Not assigned** (Frage 396 A) and show it where every item does:
  - **View: Operations:** the Not assigned look of `0101` section 13.2 (dimmed, 1px dashed `--lane-line`
    outline). Where a bridge's outer edge coincides with the Outline's Cut line, the operation line is
    drawn on top of the Not assigned outline (FU8), so the maker reads red where the laser cuts and
    grey dashes only on the seams. Implementer check: draw Not assigned items before assigned ones.
  - **Checks** of the Output group (`0029` criterion 5): "4 items are not assigned." counts them with
    every other item; its count button selects them on the canvas and in Layers (`0101` section 7), the
    second route to the bridges.
  - **Layers:** no marker. Own items show no operation chips (`0101` section 6.4); Bridge does not change
    that.
- **Assigning:** select Bridges (or some bridges) by either route, then the Operations tab's ticks
  or Add button, one step "Assign to Engrave 1" (`0101` section 6). The member's assignment beats the
  group's (nearest rule).
- **Inherited assignments** (Frage 404 A, customer 2026-11-26): when the place of the result sits under
  a group or layer that carries an operation, the bridges inherit it by the nearest rule (`0101`
  criterion 12) and draw in that look at once; Bridge is never refused for it, and no notice mentions it.

## 7. Undo and History (criteria 6, 11)

- One step **"Bridge"** (History row, Edit menu "Undo Bridge" / "Redo Bridge"), the text conversion of
  criterion 6 inside it. Ctrl+Z restores the operands, their selection and, for a text, the editable
  text. Editing the fields writes no step; the remembered values are not undone.

## 8. Large documents and empty states

- 2000 bridges: Layers is virtualised and Bridges is collapsed after Bridge (section 5), so nothing
  renders 2000 rows unasked.
- Nothing to join, no islands, too many bridges: the rail notice, no placeholder on the canvas.

## 9. Design-system rows to add at Done

- "Boolean toolbox": nine buttons on a laser page (400px), Bridge after Weld, absent on other kinds; the
  enabled rule shared with Weld.
- "Tool rail architecture": column B 13 buttons on a laser page, 600px, 18px past the 582px of the
  800 x 640 window; column B does not scroll (Frage 403: the rail wraps by itself, nothing built).
- "Command glyphs": the Bridge brief and its pairs; "Boolean glyphs": Bridge is the second exception
  (solid pieces, no squares). "Boolean tooltip": the Bridge rule and note.
- Select bar: the Bridge group (section 4). The `ux-notes` rows for the entry chip and the canvas preview
  of the first draft are not added; the UX engineer adds the rows above with the review.

## 10. Proposals (vetoable at the demo)

- **FU1** The i-dot glyph, with the side-by-side fallback. **FU3** Clamp on commit with the `0141` limit
  notice under the group. **FU6** Bridge paths named "Bridge n" in the order of criterion 18. **FU7** The
  result's Layers row expanded one level, Bridges collapsed. **FU8** Operation lines drawn over the Not
  assigned outline where they coincide. **FU14**
  Mode words "Lettering | Stencil".
- Dropped with the chip (Frage 395): FU2 state line, FU4, FU5, FU9 to FU12 (chip anchor, refusal in the
  chip, Enter applies, blue preview).

## 11. Decisions

Spec issues S1 (name Bridge), S2 (members "Bridges", paths "Bridge n"), S7 (Lettering | Stencil), S9,
S10 are taken into `specification.md`. S4, S5 and S8 fell away with the chip. S3 (column B scrolls) is
withdrawn: Frage 403 was answered by the customer on 2026-11-26 as "the rail wraps by itself", nothing
built. S6 is settled by Frage 404 A (Bridges inherit the enclosing operation, no refusal).

## Links

Spec: `specification.md`. ADRs: `adrs.md`. Build and deviations: `plan.md`. Patterns: `docs/design-system.md`
rows named in the header; `specs/0101-laser-operations/ux-notes.md`; `specs/0068-laserable-lettering/`
(Weld). Debt: `docs/technical-debt.md`, "Rails scroll and a single column on small screens".
