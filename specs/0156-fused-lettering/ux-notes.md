# 0156 Fused lettering: UX notes

ux-engineer, 2026-11-26. Written from `specification.md` (Draft, criteria 1 to 28, Fragen 395 to 398
with their defaults) on top of `docs/design-system.md` (rows "Boolean toolbox", "Tool rail
architecture", "Rail keyboard model", "Rail tooltip and notice placement", "Command glyphs", "Boolean
glyphs", "Boolean tooltip", "Boolean preview", "Action notice", "Offset entry chip", "Primary button",
section "Properties panel: Layers tab"), `specs/0101-laser-operations/ux-notes.md` (sections 6, 7, 13),
`specs/0141-parametric-generators/ux-notes.md` (sections 1, 6, 14) and the built Weld button
(`frontend/src/components/BooleanCommands.tsx`, `BooleanGlyphs.tsx`, `lib/weldText.ts`). No new token,
no new colour. One new surface, the **Fuse entry chip**, built from the Offset entry chip's parts.
**(Proposal)** items are vetoable at the demo; section 11 lists them as FU1 to FU14, section 12 the
spec issues for the product owner and the architect.

No operating hint or tip text anywhere (customer rule of 2026-10-16): the chip and the card carry
labels that name a control or a state, nothing else.

## 1. Principle

Fuse is a Boolean-card command with a few numbers. The numbers live in a small chip next to the
button for as long as the maker is choosing them, the canvas shows the bridges those numbers would
make, and one press writes them. Afterwards the result is ordinary objects: the maker finds the bridges
in the Layers tree and on the canvas and assigns them like any item. Nothing about the result is new
to the panel.

## 2. The button (criteria 1, 2)

- **Place.** The ninth button of the Boolean card, after Weld, **laser pages only** (a laser page with
  the extension on, `0098`). On every other kind the button is **absent**: no slot, no dimmed button,
  the card is 356px high as today; on a laser page it is 400px (44 x 9 + 4). Because it is the last
  button, its absence moves nothing (the row rule "a new button is appended"). Switching the active page
  changes the card in the same render (row "Page activation"); a focused Fuse button that disappears
  hands focus to the canvas.
- **Rail height.** Column B on a laser page ends 600px below the viewport top (12 + 180 + 8 + 400),
  18px past the 582px the 800 x 640 window leaves; at 800 x 600 in the browser it is further past. Column
  B gets the "Too short" rule column A already has: the Path card stays, the Boolean card scrolls inside
  its column (`overflow-y: auto`, thin scrollbar, a focused button is scrolled into view). This is the
  trigger the customer named for the rails rework (`docs/technical-debt.md`, "Rails scroll and a single
  column on small screens"): Frage 403, section 13. Criterion 2's "4 px in an 800 x 600 window" cannot
  hold (issue S3).
- **Look and rules.** Those of the row "Boolean toolbox": a command, never the active tool, no
  `aria-pressed`, 40 x 40px, glyph 20px, `--toolbar-icon`, hover `--editor-accent-hover`, the 2px
  focus ring. **Enabled** with the Select tool active and one or more objects selected (Weld's rule, one
  function for both, so the two never dim differently); **dimmed** otherwise (`aria-disabled`, 40%,
  focusable, a press does nothing). No shortcut, no letter, no menu entry.
- **While the chip is open** the button keeps the `--editor-accent-hover` ground (the busy-press look
  of the card, which ties a surface that is a column away to its button), `aria-expanded="true"`,
  `aria-haspopup="dialog"`, `aria-controls` the chip. A second press on the button closes the chip and
  changes nothing.
- **Glyph** (FU1; art finalised on the glyph sheet at 1x, 1.25x and 2x, greyscale, as every command
  glyph): the dot of an i held to its stem. Solid pieces without a stroke (as the Fracture pieces, so
  the geometric gap is the visible gap), 16 x 16 viewBox at 20px: the **stem** a rounded rectangle
  x 5 to 11, y 8 to 15, radius 1; the **dot** a disc centred (8, 3.25), radius 2.25; the **neck** x 7
  to 9 from the dot into the stem, with concave fillets of radius 1 where it meets both. Air beside the
  neck is 2.5 units (3.1px), the neck 2 units (2.5px): both above the 2px minimum. Fallback if it reads
  as a letter or as Info at 1x: two solid 5.5 x 7 rounded blocks side by side, 3.5 units apart, joined
  at mid-height by the same filleted neck. Pairs that must read apart at 1x (added to "Command glyphs"):
  Fuse / Weld (solid pieces with a neck against a hollow outline with a trim line), Fuse / Fracture
  (joined against gapped), Fuse / Split (a neck against a hole), Fuse / Union, Fuse / the Text tool's
  `Type`.

## 3. Tooltip (criterion 1)

The "Boolean tooltip" surface, three lines, opening past the rail's right edge, also on keyboard focus.

- **Name:** "Fuse" (also the accessible name; issue S1 on the name).
- **Rule** (at most 45 characters, never wraps): **"Bridges loose pieces so they hold together."**
  (43). It names what is new against Weld's "Keeps each visible line once, in one object.": Weld removes
  lines, Fuse adds material.
- **Note**, first match wins: dimmed, "Select one or more objects." (with another tool followed by "Use
  the Select tool."); else "Replaces the selection." No preview line (the chip has the preview), no
  sentence about text conversion (criterion 6; the History step and Ctrl+Z are the record).

## 4. The Fuse entry chip (criteria 3, 5, 21; Frage 395 A)

This is the first surface that opens from a rail button. The row "Rail tooltip and notice placement"
says "No popup, flyout or menu opens from a rail button"; that rule was written against flyouts of
*commands* (a second click to reach a command, covering the operands). The chip holds no commands, only
the values of the one command that opened it, and Frage 395 A asks the customer for exactly this. The
row gets an exception for it at Done (section 10).

### 4.1 Surface and placement

- The "Transform entry chip" surface (`--toolbar-bg`, `--panel-elevation-shadow`, 8px radius, 6px
  padding, rows 6px apart), DOM overlay, `role="dialog"`, `aria-modal="false"`, named "Fuse".
- **Left edge** `--rail-right` + 12 (128px, where the bars and notices start). **Bottom edge** level with
  the bottom edge of the Fuse button as it is on screen (after any column scroll), so the chip grows
  upwards from the button's row and never covers the card; clamped 4px inside the viewport. Fixed while
  values change; recomputed on resize and column scroll, not on pan or zoom (it belongs to the button,
  not to the document).
- The Fuse tooltip does not show while the chip is open.

### 4.2 Rows, slice 1 (168 x 108px)

| Row | Content |
|---|---|
| 1 | Label "Width" (12px, 52px box), 4px, the Offset chip's 100 x 28px field, fixed `mm` suffix, accessible name "Width, millimetres" |
| 2 | Label "Reach", the same field, "Reach, millimetres" |
| 3 | Left: the **state line**, 12px `--panel-muted-fg`, tabular, `role="status"` (polite). Right: the **Fuse** button, a 56 x 28px "Primary button" (`--toolbar-icon-active-bg` ground, `--toolbar-icon-active-fg` label) |

State line texts (they name a state, they do not instruct, FU2): "1 bridge", "3 bridges", "No bridges",
"More than 2000 bridges", "Too large to preview", and for half a second or more of computing "…"
(nothing while the result is fresh). Values are mm only (criterion 5), whatever the display unit.

### 4.3 Rows, slice 2 (168 x 142px)

- **Row 0, above Width:** "Mode", a 156 x 28px two-segment pill **"Lettering | Stencil"** (the Offset
  Direction pill look: 1.5px `--toolbar-icon` ring, active segment `--toolbar-icon-active-bg` / `-fg`,
  12px text), `role="radiogroup"` named "Mode", one Tab stop, arrows move and choose (wrapping), Home and
  End. Issue S7 on the word "Cut-out".
- **Row 2 in Stencil:** Reach is removed from the tree (hidden, not disabled, criterion 21) and its row
  holds "Bridges" (visible label, 52px box) with a 100 x 28px integer field, no suffix, accessible name
  "Bridges per island", 1 to 4. The chip keeps its height in both modes, so nothing under the pointer
  moves when the mode changes.

### 4.4 Values

- **Opening values:** the remembered values (app preference per OS user, not in the file); first use
  Width 1.0, Reach 5.0, Mode Lettering, Bridges 2 (the spec's defaults). Shown with up to two decimals.
- **Steps:** Up and Down in a number field step 0.1 mm, Shift 1 mm, Ctrl 0.01 mm (the Offset chip);
  Bridges steps by 1, modifiers ignored. A step at a limit stays at the limit.
- **Clamping** (criterion 5, the `0141` section 6 pattern, FU3): a typed value outside the range is
  applied clamped when the field commits (Enter, Tab, an arrow step); the field shows the limit and the
  chip's message line shows, in the "Action notice" status look, "Width set to 10 mm, the maximum." /
  "Reach set to 0.5 mm, the minimum." for 3s. Never `aria-invalid`. A field that holds no number ("",
  "abc") is the one error: "Enter a number" in the message line, `aria-invalid`, 2px `--field-invalid`
  border, the Fuse button dimmed until it holds a number.
- **Remembered** when the chip closes by any route, with its valid values (the Offset chip's rule, FU4).

### 4.5 Focus and keys

- **Open:** a press on the enabled Fuse button, or Space or Enter on it. Focus goes to Width with its
  text selected, in the frame the chip opens.
- **Tab loop** inside the chip: (Mode,) Width, Reach or Bridges, Fuse button, and around; Shift+Tab
  backwards. The chip is not modal for the pointer, but Tab does not leave it (the Offset chip).
- **Enter** in any field or on the Mode group applies, also with untouched text (the remembered values
  are the fast route: Tab to the card, End, Enter, Enter). Enter on the Fuse button applies.
- **Escape** closes and changes nothing.
- **Also closes without writing:** a press outside the chip (not swallowed: a press on the canvas
  selects as usual), a second press on the Fuse button, a tool change, a selection change, a change of
  the active page, window blur, Ctrl+Z or Ctrl+Y from the canvas.
- **Focus return** (the rail rule): opened by a key, focus goes back to the Fuse button after Escape,
  after the result and after a refusal that closes the chip; opened by the mouse, it goes to the canvas.
- Tool letters and canvas keys are ignored while focus is in the chip (the "focused control" rule).

### 4.6 Applying

- The chip's Fuse button and Enter run the command with the chip's values. While it runs: the rail's
  busy rules (`aria-busy` on the Boolean toolbar, cursor `wait`, other activations ignored, the busy
  look painted two frames before the call), the chip's button in its pressed look.
- **Success:** the chip closes, the result is selected, **no success notice** (customer rule of
  2026-10-31). The selection box on the new group is the feedback.
- **Refusal:** the chip **stays open** (issues S5, S8) and the refusal shows in its message line in the
  "Action notice" refusal look (`--popover` ground, 1px `--field-invalid` ring, `--field-invalid` text,
  12px alert glyph, `role="alert"`), the offenders in the "Refusal outline" where `0016` outlines them.
  The maker can raise Reach and press again, or press Escape. Texts are the spec's: "Fuse works on one
  laser page. Nothing was changed.", "Fuse: nothing to join. Nothing was changed.", "Fuse would make
  more than 2000 bridges. Nothing was changed.", "Fuse: no islands. Nothing was changed.", and the
  `0016` no-area and out-of-range sentences with "Fuse". The message line grows the chip upwards and
  wraps at the chip's width (168px; the chip widens to 244px while a refusal is longer than two lines).
- **Known before the press:** when the preview (section 5) already found a refusal, the message line
  shows it while the chip is open and the Fuse button is dimmed (`aria-disabled`, focusable); a change
  of a value clears it when the next preview is clean. A press on the dimmed button does nothing.

## 5. Canvas preview (criterion 3, Proposal of the spec)

- **Look** (issue S4): each bridge the chip's values would make as the "Boolean preview" line, a hollow
  1.5px `--preview-new` outline with the white casing, constant screen width, no fill, below the
  selection box and handles, over the operands, which stay as committed. Blue is the app's "this is what
  a press would write" in every view; it is drawn the same in View: Drawn and View: Operations. Nothing
  else is previewed: the Outline is the operands' silhouette plus the bridges, which the maker already
  sees.
- **When:** at once when the chip opens (remembered values), after each arrow step, after a field
  commits (Tab, Enter is the apply), and after a Mode or Bridges change. Typing alone does not preview
  until the field commits (the Offset chip's rule; the design-system rule "typed values do not
  preview"). The computation runs after the frame; a stale result is dropped, never drawn; the old
  preview stays until a new one replaces it, and the state line shows "…" after 500ms.
- **Limit:** above the preview budget the architect sets in `adrs.md` (criterion 19 and the open point
  "preview pass"), no preview is drawn and the state line reads "Too large to preview"; the Fuse button
  stays enabled and a refusal, if any, comes on the press.
- Pieces left apart (beyond Reach) get no mark (FU5): the absence of a blue bridge is the state.

## 6. The result in the document (criteria 11, 13, 14, 26)

- **Names** (issue S2 proposes "Bridges"): the group "Fused lettering" (Stencil: "Fused stencil") with
  two members in this order, top to bottom in the Layers tree: **Outline**, then **Connectors**. Each
  connector path is named "Connector 1", "Connector 2", ... in the deterministic order of criterion 18
  (FU6), so the names are stable and the golden files can hold them.
- **Kind glyphs** (row "Kind glyphs", no new glyph): the group and Connectors `Group`, the Outline
  `Spline` or the compound path glyph, each connector `Spline`. Row names: "Fused lettering, group",
  "Outline, compound path", "Connectors, group", "Connector 1, path".
- **Layers after Fuse** (FU7): the "Fused lettering" row is selected and **expanded one level**, so the
  Outline and Connectors rows are visible at once; Connectors stays collapsed (up to 2000 rows behind
  it); the list scrolls the group row into view. This is the one-press route to the bridges: press
  "Connectors" to select every bridge, expand it to pick one.
- **On the canvas:** the ordinary group rules of `0023`. A press selects the whole result; a
  double-click enters it and a press then selects Outline or Connectors; a double-click on Connectors
  enters it and a press selects one bridge. No new gesture.
- **Style in View: Drawn** (issue S10): the Connectors take the Outline's style, so the drawing looks
  as one piece and only the short seams where a bridge meets a stroke show with a stroked style.

## 7. "Not assigned" and assigning the bridges (criteria 13, 14, 20)

- No new UI. The bridges start **Not assigned** (Frage 396 A) and show it where every item does:
  - **View: Operations:** the Not assigned look of `0101` section 13.2 (dimmed, 1px dashed `--lane-line`
    outline). Where a bridge's outer edge coincides with the Outline's Cut line, the operation line is
    drawn on top of the Not assigned outline (FU8), so the maker reads red where the laser cuts and
    grey dashes only on the seams. Implementer check: draw Not assigned items before assigned ones.
  - **Checks** of the Output group (`0029` criterion 5): "4 items are not assigned." counts them with
    every other item; its count button selects them on the canvas and in Layers (`0101` section 7), the
    second route to the bridges.
  - **Layers:** no marker. Own items show no operation chips (`0101` section 6.4); Fuse does not change
    that.
- **Assigning:** select Connectors (or some bridges) by either route, then the Operations tab's ticks
  or Add button, one step "Assign to Engrave 1" (`0101` section 6). The member's assignment beats the
  group's (nearest rule).
- **Inherited assignments** (issue S6): when the place of the result sits under a group or layer that
  carries an operation, the bridges inherit it by the nearest rule and draw in that look at once; the
  spec has to say whether that is intended.

## 8. Undo and History (criteria 6, 11)

- One step **"Fuse"** (History row, Edit menu "Undo Fuse" / "Redo Fuse"), the text conversion of
  criterion 6 inside it. Ctrl+Z restores the operands, their selection and, for a text, the editable
  text. Opening, editing and closing the chip write no step; the remembered values are not undone.
- With the chip open, Ctrl+Z in a field is the field's own text undo; from the canvas it closes the chip
  first (section 4.5).

## 9. Large documents and empty states

- 2000 bridges: Layers is virtualised and Connectors is collapsed after Fuse (section 6), so nothing
  renders 2000 rows unasked. The preview draws up to the budget of section 5.
- Nothing to join, no islands, too many bridges: the chip's message line (section 4.6), no empty
  placeholder on the canvas.
- A selection of one piece: the state line "No bridges", the message "Fuse: nothing to join. Nothing was
  changed.", the button dimmed.

## 10. Design-system rows to add at Done

- "Boolean toolbox": nine buttons on a laser page (400px), Fuse after Weld, absent on other kinds; the
  enabled rule shared with Weld; `aria-expanded` and the held hover ground while its chip is open.
- "Tool rail architecture": column B 13 buttons on a laser page, 600px; the Boolean card scrolls in the
  smallest window (or the outcome of Frage 403).
- "Rail tooltip and notice placement": the exception "except the Fuse entry chip (`0156`, Frage 395 A),
  which holds the values of its own command and no command".
- "Command glyphs": the Fuse brief and its pairs; "Boolean glyphs": Fuse is the second exception (solid
  pieces, no squares).
- "Boolean tooltip": the Fuse rule and note.
- New row "Fuse entry chip": sections 4 and 5 of these notes.

## 11. Proposals (vetoable at the demo)

- **FU1** The i-dot glyph, with the side-by-side fallback.
- **FU2** A state line in the chip that names the bridge count or why there is none.
- **FU3** Clamp on commit with the `0141` limit notice in the chip; "Enter a number" is the only error.
- **FU4** Values remembered on every close, Escape included (the Offset chip).
- **FU5** No mark on pieces left apart.
- **FU6** Connector paths named "Connector n" in the order of criterion 18.
- **FU7** The result's Layers row expanded one level, Connectors collapsed.
- **FU8** Operation lines drawn over the Not assigned outline where they coincide.
- **FU9** The chip anchored at the button's bottom edge, growing upwards, fixed on pan and zoom.
- **FU10** A refusal keeps the chip open; one the preview already knows dims the chip's Fuse button.
- **FU11** Enter applies with untouched values (Tab, End, Enter, Enter is the keyboard route).
- **FU12** Blue preview in both views.
- **FU13** Column B scrolls like column A (Frage 403 default A).
- **FU14** Mode words "Lettering | Stencil" (slice 2).

## 12. Spec issues (for the product owner and the architect)

1. **S1 Name: Fuse and Weld are near synonyms** (to fuse is to weld; Cricut and Silhouette describe
   their Weld as fusing). Both are four letters on the same card. Glyph and rule line tell them apart,
   but the name does not. Proposal: name the command **Bridge** (group "Bridged lettering" / "Bridged
   stencil", undo "Bridge", tooltip rule unchanged); it says what is new and matches "bridges" in the
   refusals and "Bridges per island". If the PO keeps Fuse, these notes stand as written.
2. **S2 One word for one thing.** The UI says "bridges" (criteria 17, 21, the state line) and the member
   is "Connectors". Proposal: members **"Bridges"** and "Bridge 1", "Bridge 2", ...; "Connector" stays the
   spec's term in Words. Independent of S1.
3. **S3 Criterion 2's height clause cannot hold.** "At least 4 px under it in an 800 x 600 window": with
   Weld the column already ends at 556px, past what 800 x 600 gives; with Fuse it ends at 600px, 18px
   past the 800 x 640 desktop minimum. Proposal: replace it with "Column B scrolls inside the smallest
   window (the Path card stays, the Boolean card scrolls, a focused button is scrolled into view)", per
   Frage 403.
4. **S4 Criterion 3 "preview in the operation look".** The bridges are Not assigned (Frage 396 A), so
   their operation look is the dimmed grey dashes, and there is no operation look in View: Drawn.
   Proposal: the "Boolean preview" look (section 5).
5. **S5 Criterion 3 "pressing Fuse closes it and applies"** needs the refusal case: Proposal, a refusal
   keeps the chip open with the message in it (section 4.6), so the maker can change Reach without
   reopening.
6. **S6 Criterion 14 and inherited operations.** If the top-most operand sits in a group or layer that
   carries an operation (allowed by `0101` criterion 12), the result lands there and the bridges inherit
   it; for a Cut they would be cut along the seams, which cuts the bridge off. Proposal: the PO states
   the rule in criterion 14 (recommended: the nearest rule applies unchanged, the Operations view shows
   it at once, the tester covers the case); the architect confirms where the result is placed.
7. **S7 Slice 2 Mode "Cut-out"** is also the name of a `0101` operation kind, on the same laser page, in
   the Operations tab next door; a maker may read it as "assign Cut-out". Proposal: **"Lettering |
   Stencil"**, the words of the group names.
8. **S8 Refusals vs. criterion 17 placement.** The spec's refusals are rail notices by default; with the
   chip they appear in the chip (section 4.6). Proposal: say so in criterion 17.
9. **S9 Criterion 5 "clamped"**: Proposal, clamped on commit with a limit notice in the chip (section
   4.4), as `0141`.
10. **S10 Style of the result is not stated.** Proposal: Outline as Weld's result style (`0068`), the
    Connectors the same style (section 6); architect confirms with criterion 20.

## 13. Customer question

Frage 403 (FUSE-RAIL): Fuse makes the Boolean card taller; how does the rail fit small windows?
- A: Fuse is simply appended; in the smallest window the Boolean card scrolls inside its column by
  18px, as column A already does since the Generators tool; the rails rework stays in the debt list.
  **Recommended**: the same answer as Frage 327, and Fuse exists on laser pages only.
- B: Do the rails rework now (rails scroll to fit any height, one column on small screens) as part of
  this story, before Fuse is built. Your trigger for it ("the next card that grows") is reached.
- C: Raise the desktop minimum window height from 640 to 680px, so nothing scrolls on the desktop.
- Default if unanswered: A.

## Links

Spec: `specification.md`. ADRs: `adrs.md` (architect, in progress). Patterns: `docs/design-system.md`
rows named in the header; `specs/0101-laser-operations/ux-notes.md`; `specs/0141-parametric-generators/
ux-notes.md`; `specs/0068-laserable-lettering/` (Weld). Debt: `docs/technical-debt.md`, "Rails scroll and
a single column on small screens".
