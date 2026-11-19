# 0123 Raster engraving: UX notes

ux-engineer, 2026-11-18; **revised 2026-11-19 after the customer's feedback on PR #394**: pictures get
their own operation kind **Image** (as LightBurn's Image layer, not its window), Engrave is the vector
hatch of `0111` again, and Dither is an always-visible choice strip (the font tag strip's look), not a
closed pick with an inline list ("too restless": the opening list pushed everything down). Written from
`specification.md`, `specs/0124-gradient-fill/ux-notes.md`, the built notes of `0111` (sections 3 and
6), `0112` (section 4), `0101` (section 13), `0029` (Toolpath view, play bar, checks) and
`docs/design-system.md` ("Hidden, not disabled", "No popups in the properties panel", "No operating
tips", "No notice after a success", the new "Choice strip"). Drafts: `docs/design-drafts/2026-11-raster/`
(screens 04 to 06 are current; 01 and 03 B/C are superseded where the README says). Customer rules
applied: no invented colours (pale orange `--engrave-hatch`, red `--cut`, ink, the existing blue of a
pressed control), symbol-only buttons with a tooltip, units inside the fields, plain fields and no
sliders, compact, the operation's own row and no dialog, not LightBurn. **(Proposal)** items are
vetoable at the demo; section 11 lists them. "Ink" is `--toolbar-icon`; "body column" is the unfolded
row's body, 232 px wide (panel 280 less 2 x 12 padding, the row's 2 x 4, the body's 12 indent and 4
right: the column the built Brightness 120 + 4 + Contrast 108 fill).

## 0. What is reused

| Part | Reused from | Change |
|---|---|---|
| Kind button, kind glyph, canvas chip | `OperationKindGroup`, `OperationGlyph`, `0101` 13.3 | a sixth glyph, Image |
| Operation row, lines 1 to 3, ring, time | `OperationRow` (`0101` 4.2, `0029` P25) | line 3 adds the dither name |
| Unfolded row | `OperationBody`, `OperationOptionsPart` (`0111` 6) | the Image body of section 2 |
| Choice strip | `PresetStrip` (the font tag strip, the Paper strip) | Dither, in the body column |
| Fields | the bar number field of `0111` 6.3 (label inside left, unit inside right) | Threshold, Brightness, Contrast, Gamma, Lowest power (as built) |
| Icon toggle | the "Link corners toggle" look | Invert (as built) |
| Canvas looks | `0101` 13.2, `0029` Toolpath view and play bar | dots, the wrong-kind look |
| Checks | `OutputChecks` lines (`0029` 6.4) | texts name the Image operation; two warn lines |
| Test grid bar | `TestGridBar` `Segments` | kind Image and the Dither segments |

## 1. Principle

A picture is an object like any other: it goes into **Image** with the same Select-bar press as a shape
into Engrave, and the Image operation's own numbers (dither and adjustments) sit in its unfolded row,
always visible, every choice one press away. Engrave hatches vectors, Image burns dots; an item in the
kind that cannot run it is drawn as "does not run" and named in one check line. What burns is drawn
where the work is: the Operations view shows the dots, the Toolpath view the rows. No image dialog, no
preview window, no list that opens.

## 2. The Image operation (kind, row, body)

### 2.1 Kind, glyph, names

- Sixth kind **Image**, in the **Marking** section with Engrave, Engrave + Outline and Score (it marks,
  it does not cut; Cuts last orders it before the cutting kinds).
- **Glyph** (custom `OperationGlyph` kind `image`, 16 x 16, ink, filled, no stroke): a dither ramp of
  2 x 2 squares on a 4-unit pitch, four columns falling in density from left to right: 4, 3, 2, 1 dots
  (column 1 rows 1 to 4; column 2 rows 1, 2, 4; column 3 rows 1, 3; column 4 row 2). It reads as
  "dots", apart from Engrave's three lines and from the Test grid's 3 x 3 outlined squares; drawn at
  1x and 2x in draft 05 (P36).
- Names: kind word "Image"; new operations "Image 1", "Image 2" ...; Select-bar tooltip and name
  "Image"; accessible description "Assigns to Image 1" / "Creates Image 1" (`0111` 3.2).
- One-line meaning for the PO: **"Burns pictures and gradient fills as rows of dots."**

### 2.2 Kind button in the Select bar

- In the Operation group of `0111` 3.1, **appended last** (spec criterion 47 a): Cut, Score, Engrave,
  Engrave + Outline, Cut-out, **Image**; no built button moves (the rail's rule: a new button is
  appended). Six 28 x 28 buttons, 4 px apart: the group is 188 px (was 156) and wraps as a whole by
  "Select bar layout" (smallest window: it still starts row 2, Flip follows).
- Symbol only, tooltip "Image" `side="bottom"`, no second line, Shift press creates "Image <n>"; every
  press rule of `0111` 3.3 as built (P36).
- Canvas chip order (`0101` 13.3): Cut, Cut-out, Score, Engrave, Engrave + Outline, Image.

### 2.3 The row (folded)

As every run row (`OperationRow`, 56 px, 64 coarse):

| Line | Content |
|---|---|
| 1 | Image glyph, name "Image 1", Show, Output, trash, gap, Up, Down (as built) |
| 2 | the ring with the digit, "Image · 2 items", the time at the right in ink ("7 min 12 s"); without a run the plain digit as built |
| 3 | the set's numbers as every row, then " · " and the **dither short name** of the strip (2.5): "35 %, 6,000 mm/min, 1 ×, 0.10 mm · Floyd"; Grey adds its range: "· Grey 10–35 %" (Lowest power to Power) |

- Scan angle and the adjustments are not on line 3 (the Engrave row does not show its Scan angle
  either; the row stays one line of numbers). Too long: ellipsis, the line's tooltip carries it (P41).
- Damaged settings: line 3 as built for damage (no numbers), the check line of `0029` criterion 33.

### 2.4 The unfolded body (body column 232 px, 4 px between fields, 8 px between groups)

| Part | Controls |
|---|---|
| Upper (the set) | Material pick; Power and Passes; Speed; Interval (the dot size), exactly as Engrave (`0111` 6.2) |
| hairline | 1 px ink at 25 %, 8 px above and below (`0111`) |
| Image settings | `role="group"` "Image settings of Image 1", **always shown** on an Image row (no "only while a raster item" rule; an empty Image still shows its settings, as an empty Engrave shows Interval); right under the set, as spec criterion 11 places it: on Image the dither is the main choice |
| hairline | as above |
| Lower (motion) | row 1: Bidirectional fill toggle (28 x 28, `ArrowRightLeft`), 4, Scan angle (164, "°"); row 2: Overscanning (128, "%"); with a motorised Z axis, row 3: Z offset (128, "mm"). **No Crosshatch** on Image: it would do nothing (criterion 16), so it is absent, not disabled |

**Image settings rows** (P39, replaces P20):

| Row | Controls | Size |
|---|---|---|
| 1 and 2 | **Dither** choice strip (2.5) | 232 x 59 (two 28 px rows, 1 px between, 1 px border) |
| 3 | **Brightness** "%", **Contrast** "%" | 120 and 108 (P34) |
| 4 | **Gamma** (no unit), **Invert** toggle | 100 and 28 x 28 |
| 5 | **Threshold** "%" (Threshold only) or **Lowest power** "%" (Grey only); absent for the other five | 124 and 132 |

- The mode's own field is the **last** row of the group: switching modes never moves the strip,
  Brightness, Contrast, Gamma or Invert; only the lower part under the group moves by 32 px, away from
  the pointer and the focus (customer: "restless"). Spec criterion 11 has it last too (E1 replaced,
  delta E11 folded 2026-11-19).
- No heading (P19 kept): the strip's cells name themselves; "Image settings" is the accessible name.
- Invert, fields, ranges, steps, validation chips, commit and undo: as built (former 2.2 and 2.4,
  unchanged): Threshold, Brightness, Contrast, Lowest power step 1 (Shift 10), Gamma 0.01 (Shift 0.1);
  out of range the typed text stays, 2 px `--field-invalid`, `aria-invalid`, the chip "Threshold is 0
  to 100 %." (and the four others as built); each commit and the Invert press one step "Edit settings".
- Material pick, Use material, Reset to material and a material edit never change Image settings.

### 2.5 The Dither choice strip

The `PresetStrip` component (the font tag strip's look: one 1 px border of ink at 60 %,
`rounded-[5px]`, cells 1 px apart, 14 px text, 28 px rows, pressed cell `--toolbar-icon-active-bg`
with `--toolbar-icon-active-fg` text, hover `--editor-accent-hover`), `width="w-full"` in the body
column. Always visible; nothing opens (P37, replaces P21 and P22).

| Row | Cells (visible / accessible name; tooltip) |
|---|---|
| 1 | Threshold / Threshold · Ordered / Ordered · **Floyd** / Floyd-Steinberg, tooltip "Floyd-Steinberg" |
| 2 | Jarvis / Jarvis · Atkinson / Atkinson · Stucki / Stucki · Grey / Grey |

- **Fixed rows**, a break after the third cell (not the strip's automatic wrap, so the rows are the same
  on every platform font). Cells grow to fill their row (`flex: 1 1 auto`, the strip's rule, minimum
  32 px, 6 px side padding, text never truncated). Measured at 14 px in the UI font: row 1 needs 182 of
  228 px, row 2 205 of 227 px; at the customer's 256 px the cells only grow. Equal widths in row 2 would
  leave "Atkinson" (53 px) 2 px of padding at 232, so the cells keep the strip's flex rule.
- Order = the spec's order (Threshold, Ordered, Floyd-Steinberg, Jarvis, Atkinson, Stucki, Grey), which
  is also the arrow order. Only Floyd-Steinberg is shortened. **A cell has a tooltip only when its word
  differs from the full name** (corrected 2026-11-19, UX review U3, the customer's "restless": a
  tooltip that repeats the word adds nothing): "Floyd" has "Floyd-Steinberg", the other six none. It
  opens **above** the cell (`side="top"`, 400 ms; `PresetStrip` gains a tooltip side, since "left"
  would cover the neighbouring cell in a row of four). The accessible name is always the full name.
- **Keys** (P38, replaces P23): one Tab stop (the pressed cell); Left / Up and Right / Down move **and
  select** in the order above, without wrap (the strip's arrows; Radix `loop={false}`); Home / End the
  first / last (as built: these keys, the fixed rows and the no-wrap are the Dither strips' own,
  `PresetStrip`'s `fixedRows`; the Document section's strips keep their accepted keys). Each selection is a commit: one step "Edit settings", and the Operations view redraws
  the item's dots (the preview is the result; budget criterion 37 d). A press selects the same way;
  the focus stays on the strip (the body's controls keep it; no return to the canvas). Pointer hover
  previews nothing.
- Default for a new Image: Floyd-Steinberg. Grey on an M3 machine stays selectable (the block line
  says why it cannot run; P33).
- **Damaged** `dither` (criterion 13): no cell pressed, and a 12 px line "Damaged" in `--field-invalid`
  4 px under the strip (the state's word, not a tip), named by the strip's `aria-describedby`; a press on any cell repairs it (one step) (P46).

### 2.6 Engrave and Engrave + Outline

Exactly as `0111` built them: hatch only, no Image settings group, Crosshatch shown. A gradient-filled
shape in Engrave is hatched as a shape (its fill paint never counts, `0101` criterion 24).

## 3. On the canvas

### 3.1 Operations view: an Image item

- A raster item (an image, or a shape, open path or text with a Gradient fill) in Image is drawn as
  **its dots**: burned dots in `--engrave-hatch` on a transparent ground, at most one dot per device
  pixel, clipped to its frame or outline; no outline, no hatch. Always (P25). The Image chip at the
  corner (`0101` 13.3), so the kind is never told by colour alone.
- Grey: each burned dot at an opacity from 20 % (Lowest power) to 100 % (Power) (P26).
- After an edit the previous dots stay until the new ones are ready (criterion 37 d).

### 3.2 An item in a kind that cannot run it (wrong kind)

The `0101` 13.2 **Not assigned look** (dimmed, 1px dashed 4 on 3 `--lane-line` outline, no hatch, no
dots), but the item **keeps its chip, drawn in `--panel-muted-fg`** (the Output-off chip rule of
`0101` 13.2): "assigned, does not run" reads apart from "not assigned" (no chip) and from "runs" (ink
chip). One warn line per operation names it (section 4) (P42).

| Item | In Engrave / Engrave + Outline | In Image | In Cut | In Score | In Cut-out |
|---|---|---|---|---|---|
| Image | its frame rectangle, hatched (Engrave) or hatched and traced (Engrave + Outline) (Frage 271 A, 2026-11-19) | dots | frame line | frame line | frame as the one closed contour |
| Shape with Gradient fill | hatched as a shape (+ outline) | dots | outline | outline | outer contour |
| Shape with Colour, Pattern or no fill, open path, text | hatch (as built) | does not run: wrong-kind look (E9) | as built | as built | as built |

- An image in a vector kind (Frage 271 A, strictly): drawn and run as the rectangle of its frame, in that
  kind's own look (cut line, score line, hatch, hatch plus outline); no wrong-kind look, no chip muted,
  no warn line. The wrong-kind look (E9 text and the muted chip) stays for criterion 48's vector leaves
  in Image only.

### 3.3 Linked clones, clone members, placed pages

- Each placement draws **its own dots**, computed from the item as it stands on the laser page
  (criterion 4): a clone of an Image item shows dots exactly like a plain copy would; a turned or
  mirrored clone shows the picture turned or mirrored, with the dot rows still parallel to the Scan
  angle on the page (not turned with the clone). A clone must never be drawn empty, as its frame only,
  or with its source's dots shifted (the bug of the review build).
- Selected, the `0049` clone marks (violet trace and tag) are drawn over the dots as over any artwork;
  the chips follow the clone's box.
- Cache: per page revision, operation and placement (criterion 38); clones of one source with the same
  transform may share the dots.

### 3.4 Pending

| Where | While the work is not ready |
|---|---|
| Canvas, first build (just assigned, the page opened, an item that had no dots yet) | its frame or outline as a 1 px solid `--engrave-hatch` line and the Image chip, nothing else (no spinner, no text, no blank); the dots replace it when ready (P44) |
| Canvas, after an edit | the previous dots stay until the new ones are ready (as built) |
| Rows and total | P35 as built: the last job's numbers for 300 ms, then the time "…", "About …", "Burn … · travel …" |
| Checks, Export | the last job's checks stay until the job is finished; Export writes the finished job (P35) |
| Strip, fields | show the new value at once (pressed cell, field text); no busy look |

### 3.5 Toolpath view and play bar

As before (former 3.2): runs in `--engrave-hatch`, row steps and gaps over 5 mm as the dashed travel
line, zoomed out the shaded dot area without travel lines (P27), the operation's numbered ring, one
arrowhead at the item's first row, the play bar's full and pale halves (Grey by opacity). Items that do
not run (3.2) draw nothing in the Toolpath view.

### 3.6 Laser page, Operations view off

The image as imported; a Gradient fill grey on a laser page (`0124`); no chips, no wrong-kind look.

## 4. Checks

In the Output group's check list, after the lines of `0029` criterion 5, in this order. The leading name
or count is the text button (underlined, no colour): items select them on the canvas, an operation
selects and unfolds its row and focuses the field named (P28).

| Kind | Symbol | Text (singular / plural) | Button does |
|---|---|---|---|
| block | `OctagonX` | "1 item lies outside the 400 x 300 mm bed." / "<n> items lie outside ..." (as built) | selects them |
| block | `OctagonX` | "**Image 1** needs laser mode M4 for Grey." | unfolds, focus the Dither strip |
| block | `OctagonX` | "Photo 1 would need 120,000,000 dots; the limit is 50,000,000." | selects the item |
| warn | `TriangleAlert` | "1 image is coarser than its Interval." / "<n> images are coarser than their Interval." | selects them |
| warn | `TriangleAlert` | "2 items overlap and burn twice." | selects them |
| warn | `TriangleAlert` | "1 image was reduced and loses detail." / "<n> images were reduced and lose detail." | selects them |
| warn | `TriangleAlert` | "1 image cannot be read and is skipped." / "<n> images cannot be read and are skipped." | selects them |
| warn | `TriangleAlert` | "**Image 1**: Lowest power is above Power." | unfolds, focus Lowest power |
| warn | `TriangleAlert` | spec criterion 48: "1 item in Image 1 is not a picture and does not burn." / "<n> items in Image 1 are not pictures and do not burn." | selects them, unfolds the row |

- One line per operation for the wrong-kind line of criterion 48; the name is the operation's. It
  states the fact; no instruction (customer rule). Order as criterion 48: after the lines of
  criterion 36, before the line of criterion 32 (b). The image-in-Engrave line of the first draft (E9)
  is void: an image never warns (Frage 271 A).

## 5. The test grid: kind Image (criteria 44, 45)

The Engrave grid is `0112` again (vector hatch, Crosshatch; no Dither). The Test grid bar gains a fourth
**Kind** segment **Image** (the Image glyph and the word, at least 68 px like the other three) (P45).
As built and measured (2026-11-19, UX review): Cut, Score and Image 68 px, **Engrave 80 px** (its glyph
and word need more than the minimum), the group 284 px.

| Row 1, Image kind | Control | Size |
|---|---|---|
| Shared values | Power, Speed, Passes, Interval as Engrave; **no Crosshatch** | as built |
| Dither | `radiogroup` "Dither", **six text segments** in the bar's `Segments` look (13 px, the pressed segment `--value-fill` as Kind and the axis parameters): Threshold, Ordered, Floyd, Jarvis, Atkinson, Stucki (no Grey); the tooltip "Floyd-Steinberg" on Floyd only; always shown with the Image kind | 6 x 56 = 336 in one row; 3 x 56 = 168 in two rows |
| Cell size | as built | 120 |

- **No Fill** (spec criterion 44): every Image cell is a black-to-white Gradient rectangle (Angle 0).
  The Fill segments and `TestGridDitherMenu` of the review build go.
- **No menu**: arrows move and select (the bar's `Segments` keys), Home and End the first and the last
  (UX review U9), each a commit: the grid regenerates, one step "Edit test grid".
- Dither is one wrap item; when the bar row has no room for its 336 px (an 800 px window) it stands in
  **two rows of three** (Threshold, Ordered, Floyd over Jarvis, Atkinson, Stucki; 168 px), its halves
  wrapping whole (corrected 2026-11-19, UX review U4: the earlier "goes whole to its own line" did not
  fit an 800 px window).
- The bar's own segment look, not the panel strip's solid blue (P45): in the bar, Kind and the axis
  parameters already use `--value-fill`; a solid-blue Dither would shout louder than Kind.
- Cells: drawn grey on the laser page and as dots in the Operations view; the derived rows are Image
  operations with the chosen Dither.
- The Kind switch between Engrave and Image keeps the shared values and axes (Interval exists in
  both); Dither stays stored on the grid while Engrave is shown (no sign).

## 6. Keyboard routes

| Action | Route |
|---|---|
| Assign to Image | Tab to the Select bar's Image button, Enter or Space (Shift: new Image) |
| Image body | Tab from the row into the body, as built (2026-11-19, `OperationBody.tsx`): the material pick and the settings fields with Interval, then the Image settings group (the Dither strip, one stop; Brightness, Contrast, Gamma, Invert, the mode field Threshold or Lowest power), then Bidirectional fill, Scan angle, Overscanning, (Z offset) |
| Dither strip | Left / Right / Up / Down move and select, Home / End; no wrap |
| Fields | Enter commits, Escape reverts, Up / Down step (Shift 10) |
| Invert | Space or Enter |
| Check lines | Tab to the text button, Enter |
| Test grid Kind, Dither | Tab in the bar order; arrows move and select; on Dither also Home / End |
| Not-a-picture check line | Tab to its button, Enter: selects the items and unfolds the operation's row (UX review U6) |

## 7. Undo steps

"Assign to Image 1" (kind button), "Edit settings" (each strip selection, each field commit, Invert),
"Edit test grid" (Kind, Dither in the bar). No step for focus, hover, the views, zoom or the play
bar.

## 8. Accessible names

| Element | Name | State or description |
|---|---|---|
| Kind button | "Image" | description "Assigns to Image 1" / "Creates Image 1" |
| Row | as built ("Image 1, Image, 2 items") | |
| Group | `role="group"` "Image settings of Image 1" | |
| Dither strip | `radiogroup` "Dither"; radios "Threshold", "Ordered", "Floyd-Steinberg", "Jarvis", "Atkinson", "Stucki", "Grey" | `aria-checked`; damaged: none checked, the "Damaged" line as `aria-describedby` |
| Fields, Invert | as built | |
| Bar Kind | radio "Image" | |
| Bar Dither | `radiogroup` "Dither", radios with the full names | |

## 9. States

- **Empty Image** (no item): the row with "0 items" as any kind; the body with all its settings.
- **Mixed selection** pressed into Image: everything is assigned; vector items take the wrong-kind look
  and the warn line (no refusal, no notice: the canvas and the line show it) (P42).
- **Damaged settings:** the strip's "Damaged" line; the check as built.
- **Large:** a 200 x 200 mm image at 0.1 mm: previous dots stay while rebuilding; zoomed out the shaded
  area (criterion 37 e).
- **Unreadable image:** skipped, its check line; on the canvas the wrong-kind look with the Image chip
  muted (P32).
- **M3 laser with Grey:** the block line; the Toolpath view still draws the rows (P33).

## 10. What is not added

No image dialog, no "Adjust image" window, no preview window, no dither sample tiles, no open list, no
tone curve, no sliders, no per-image settings, no layer table, no Image + Outline kind (criterion 2:
an image and a Score), no new colour token, no new shortcut, no success notice, no tip text.

## 11. Proposals (vetoable at the demo, with defaults)

| # | Proposal | Default |
|---|---|---|
| P19 | No heading over the Image settings group; hairline only | as described |
| P20 | *(replaced by P39)* | |
| P21, P22 | *(replaced by P37: no pick, a strip)* | |
| P23 | *(replaced by P38)* | |
| P24 | *(withdrawn: Crosshatch is absent on Image, Engrave is as built)* | |
| P25 | The Operations view always shows a raster item's dots | always |
| P26 | Grey dots and runs at an opacity from 20 % to 100 % by power | as described |
| P27 | Zoomed out, the item's travel lines are not drawn | as described |
| P28 | Check text buttons: items select, operations unfold and focus the named control | as described |
| P29 | Dot-limit line names the item, no instruction | as described |
| P30 | Overlap line says "items" | as described |
| P31 | *(withdrawn: the Image grid has no Fill, spec criterion 44)* | |
| P32 | An unreadable image shows the wrong-kind look | as described |
| P33 | Grey on M3: rows still drawn in the Toolpath view | as described |
| P34 | Field widths Threshold 124, Lowest power 132, Brightness 120, Contrast 108, Gamma 100 | as built |
| P35 | Pending job: the last job's numbers for 300 ms, then "…" | as built |
| P36 | Kind Image with the dot-ramp glyph (not Lucide `ImageDown`), appended last in the Select bar, Marking section, chip last | dot ramp |
| P37 | Dither as a `PresetStrip` in two fixed rows (3 + 4), "Floyd" the only short name, tooltips full names | as described |
| P38 | Strip arrows move and select; each selection one "Edit settings"; the canvas redraw is the preview | as described |
| P39 | Threshold / Lowest power as the last row of the group | as described |
| P40 | Image body: Engrave's set, the Image settings (always shown), then the lower part without Crosshatch | as described |
| P41 | Row line 3 adds " · <dither short name>" (Grey: its range); no Scan angle | as described |
| P42 | Wrong kind: Not assigned look with the kind's chip muted, plus one warn line per operation; no refusal | as described |
| P43 | Every placement (clone, placed page) draws its own dots, rows on the page's Scan angle | as described |
| P44 | First build pending: a 1 px `--engrave-hatch` frame and the chip | as described |
| P45 | Test grid: kind Image (4th segment) with Dither as six bar segments in the bar's segment look; no Fill, no menu | as described |
| P46 | Damaged dither: no cell pressed, "Damaged" line under the strip | as described |

## 12. Spec deltas for the product owner

E1 to E7 are folded into the specification (2026-11-18); E8 to E13 on 2026-11-19 (PO), with E9 void
and E10 narrowed to criterion 48 by the customer's Frage 271 A (an image is the rectangle of its frame in
every kind but Image) and Frage 270 B (the material's fourth set Image). The PO's revision of 2026-11-19 (commit
`b8290b9c`: Image kind, criteria 1, 2, 11, 44 to 48) and these notes agree on: the sixth kind Image in
the Marking section, appended last in the Select bar; no Crosshatch and no outline on Image; the Image
settings always shown, right under the set; the Engrave set as Image's material set (criterion 2);
Engrave and Engrave + Outline as `0111` built them; the Image test grid with Dither and without Fill;
the wrong-kind warn text of criterion 48; linked clones burning and showing their dots (criterion 46).
Open deltas:

- **E8** Glyph (criterion 47 a): the dot ramp of 2.1, not Lucide `ImageDown` (every kind glyph shows
  what burns, as Engrave's hatch; `ImageDown` reads as "download image" and has no 16 px custom
  stroke rule).
- **E9 (void, Frage 271 A)** An image in Engrave, Engrave + Outline, Score or Cut-out (criterion 1: runs nothing, as
  before) also warns: "1 image in Engrave 1 does not burn." / "<n> images in Engrave 1 do not burn."
  (selects them), the twin of criterion 48, so an image pressed into the wrong kind is never silent.
- **E10** The wrong-kind canvas look (3.2): the Not assigned look with the kind's chip in muted ink,
  for both criterion 48 and E9 items (criterion 20 or 47 e).
- **E11** Criterion 11, the strip: two **fixed** rows (Threshold, Ordered, Floyd / Jarvis, Atkinson,
  Stucki, Grey), not "as many rows as the width needs" (a platform font must not change the rows);
  "Floyd" shown with the tooltip and accessible name "Floyd-Steinberg"; arrows select without wrap,
  each selection one step "Edit settings". The mode's field is the **last** row of the group, not
  under the strip (replaces E1; the customer's "restless").
- **E12** Criterion 47 (d): row line 3 adds " · <dither short name>" (Grey with its range) (P41).
- **E13** Pending first build (3.4, P44) and the damaged strip (P46): add to criterion 20 / 13 if the PO
  wants them testable.

## Links

Spec: `specs/0123-raster-engraving/specification.md`. Drafts: `docs/design-drafts/2026-11-raster/`.
Gradient fill: `specs/0124-gradient-fill/ux-notes.md`. Built notes: `specs/0111-laser-operation-tools/ux-notes.md`,
`specs/0112-test-grid-and-materials/ux-notes.md`, `specs/0101-laser-operations/ux-notes.md`, `specs/0029-laser-job-preview-and-output/ux-notes.md`.
