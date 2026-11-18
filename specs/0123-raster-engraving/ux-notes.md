# 0123 Raster engraving: UX notes

ux-engineer, 2026-11-18. Written from `specification.md` (criteria 1 to 45, Fragen 227 to 231 as
decided), `specs/0124-gradient-fill/ux-notes.md` (the Gradient fill itself), the built notes
`specs/0111-laser-operation-tools/ux-notes.md` (section 6, the unfolded row), `specs/0112-test-grid-and-materials/ux-notes.md`
(section 4, the Test grid bar), `specs/0101-laser-operations/ux-notes.md` (section 13, the canvas
look), `specs/0029-laser-job-preview-and-output/ux-notes.md` (Toolpath view, play bar, checks) and
`docs/design-system.md` ("Hidden, not disabled", "No popups in the properties panel", "No operating
tips", "No notice after a success"). Drafts: `docs/design-drafts/2026-11-raster/` (three screens, HTML
and PNG). Customer rules applied: no invented colours (pale orange `--engrave-hatch`, red, ink only),
symbol-only buttons with a tooltip, units inside the fields, plain fields and no sliders, the
operation's own row and no dialog (nothing like LightBurn's Image mode window or "Adjust image"), no
machine settings in the operation. **(Proposal)** items are vetoable at the demo; section 11 lists them
as P19 to P34 (P1 to P18 are `0124`'s). "Ink" is `--toolbar-icon`.

## 0. What is reused

| Part | Reused from | Change |
|---|---|---|
| Unfolded row | `OperationBody`, `OperationOptionsPart` (`0111` 6.3) | the Image settings group after the lower part |
| Closed pick | `MaterialPick` / `InlinePickList` (inline list under the field, no popup; the Start pick of `0029`) | Dither |
| Fields | the bar number field of `0111` 6.3 (label inside left, unit inside right) | Threshold, Brightness, Contrast, Gamma, Lowest power |
| Icon toggle | the "Link corners toggle" look (Crosshatch, Bidirectional fill) | Invert |
| Canvas looks | `0101` 13.2 Engrave look, `0029` Toolpath view and play bar | raster items as dots |
| Checks | `OutputChecks` lines (`0029` 6.4) | five new texts |
| Test grid bar | `TestGridBar`, `TestGridMaterialMenu` (the Trace Preset menu) | Fill segments and the Dither menu |

## 1. Principle

A picture is an object like any other: it goes into Engrave with the same five buttons, and its
dither and its adjustments are the operation's own numbers, in the same unfolded row as Interval and
Scan angle. What the laser burns is drawn where the work is: the Operations view shows the dots, the
Toolpath view the rows. There is no image dialog, no preview window and no second list.

## 2. The Image settings group (criteria 11 to 13)

### 2.1 Place

In the unfolded Engrave or Engrave + Outline row (`0111` 6.1), after the lower part (Crosshatch,
Bidirectional fill, Scan angle; Overscanning; Z offset): a 1px hairline (`--toolbar-icon` at 25 %, the
`0111` hairline) 8px under it, the group 8px under the hairline. **No heading** (P19): the Dither pick
is the first control and names the group; the spec's "Image settings" is the code name and the
accessible name. The group is in the tree only while the operation holds at least one raster item
(an image, or a shape, open path or text with a Gradient fill); otherwise the row is exactly `0111`'s.
Its values stay stored while it is hidden, without a sign.

### 2.2 Rows (228px column, 4px between fields, 8px between rows)

| Row | Controls | Sizes |
|---|---|---|
| 1 | **Dither** pick: label "Dither" inside left, the mode at the right, `ChevronDown` | 228 x 28 |
| 2 | **Threshold** (with Threshold only) or **Lowest power** (with Grey only); absent for the other five modes | Threshold 124, Lowest power 132 |
| 3 | **Brightness** "%", **Contrast** "%" | 124 and 100 |
| 4 | **Gamma** (no unit), **Invert** toggle | 100 and 28 x 28 |

- Lowest power sits in row 2 with Threshold, not last as criterion 11 lists it (P20, spec delta E1):
  both belong to one mode, so the mode's own number is always right under the pick and rows 3 and 4
  never move between Threshold and Grey.
- Widths are set by the longest content at the 12px label and 13px value ("Brightness -100 %" needs
  about 119px; "Lowest power 100 %" about 129px), the `0111` UX review lesson (no clipped labels).
- **Invert**: `Contrast` 16px (the half-filled circle), tooltip and name "Invert", `aria-pressed`,
  default off. Pressed: active fill, white glyph.

### 2.3 The Dither pick

- Closed: as the Start pick of `0029` (`MaterialPick` look, no glyph at the left, P21: the pick needs
  no symbol, the label names it). Enter, Space or a press opens the **inline list under it** (rows below
  move down; no popup in the panel): Threshold, Ordered, Floyd-Steinberg, Jarvis, Atkinson, Stucki,
  Grey, 28px rows, the current one with `Check` and the selected ground. Plain names, no sample glyphs
  (P22: the canvas shows the real result, section 3).
- Keys: Up / Down move without wrap, Home / End, Enter or a press picks and closes (one step "Edit
  settings"), Escape closes and keeps the old mode. While the list is open and the Operations view is
  on, the item under the keyboard focus is **previewed on the canvas** (section 3.1); pointer hover
  does not preview (P23: no flicker while the pointer crosses the list).
- Default for a new operation: Floyd-Steinberg (spec Proposal). Damaged value: the pick shows
  "Damaged" in `--field-invalid`, as the Start pick (the operation does not run, criterion 13).

### 2.4 Fields

- Plain bar number fields, Enter or leaving commits, Escape reverts, Up / Down step and commit on
  key-up (`0111` 6.4). Steps: Threshold, Brightness, Contrast, Lowest power 1 (Shift 10); Gamma 0.01
  (Shift 0.1). Values with more decimals than the field holds are rounded to its step (criterion 12).
- Out of range: the typed text stays, 2px `--field-invalid`, `aria-invalid`, the validation chip 4px
  under the field: "Threshold is 0 to 100 %.", "Brightness is -100 to 100 %.", "Contrast is -100 to
  100 %.", "Gamma is 0.10 to 4.00.", "Lowest power is 0 to 100 %." ("Enter a number." for text);
  nothing is written.
- Each commit and the Invert press: one step "Edit settings"; the Operations view, the Toolpath view and
  the checks follow in the same frame where the budget allows (criterion 37 d).
- A material pick, Use material, Reset to material and a material edit never change them (criterion 13).
- Crosshatch keeps its place even when the operation holds only raster items, where it does nothing
  (criterion 11 keeps the lower part as built; no state line, P24).

## 3. On the canvas (criteria 20, 25)

### 3.1 Operations view

- A raster item in Engrave or Engrave + Outline is drawn as **its dots**: burned dots in
  `--engrave-hatch` on a transparent ground, at most one dot per device pixel (the full grid when the
  zoom allows), clipped to the item's frame or outline. Engrave + Outline adds the 1px ink outline.
  No 45 degree hatch on a raster item (it would hide the picture). The Engrave glyph chip at the corner
  as `0101` 13.3, so the kind is never told by colour alone.
- **Always**, not only while an Image setting is edited (P25, spec delta E2): the dots are what burns,
  and the hatch says nothing about a picture. Built lazily, cached per page revision and operation
  (criterion 38); after an edit the previous dots stay until the new ones are ready (never blank, never
  half), within 50 ms (criterion 37 d).
- Grey mode: each burned dot drawn at an opacity that follows its power, from 20 % (Lowest power) to
  100 % (the set's power), so the shade reads with one colour (P26).
- Images in Cut, Score or Cut-out: their frame line in that kind's look, as built. Not assigned:
  dimmed and the dashed outline of `0101`. A gradient shape in Cut or Score: its outline only.
- Operations view off: the drawing's own look; on a laser page a Gradient fill is grey (`0124` 9).

### 3.2 Toolpath view

- Burn runs: lines along each row in `--engrave-hatch`, at least one device pixel wide, the Engrave
  look of the job (`0029`). Row steps and gaps over 5 mm: the thin dashed travel line of `0029`.
  Leading and trailing unburned stretches: nothing.
- Zoomed out until rows are closer than one device pixel: the item is drawn as its **shaded dot area**
  (the burned dots rendered at the screen's resolution, same data), and the travel lines of that item
  are not drawn at that zoom (P27: a dashed line per pixel row reads as grey noise, not as travel).
- The operation's start keeps its numbered ring; the item gets **one arrowhead** at the start of its
  first row in the row direction (spec Proposal), not one per row.
- Play bar: the rows burned up to the head in full colour, the rest pale, the head ring on the row it
  is on (`0029` play bar rule). Grey runs use the opacity rule of 3.1.

### 3.3 Laser page look of the source

The image itself is drawn as imported (its own colours) when the Operations view is off. A Gradient
fill is grey on a laser page (`0124` criterion 10), so what the maker sees is the lightness that burns.

## 4. Checks (criteria 31 to 36)

In the Output group's check list (`0029`), after the lines of `0029` criterion 5, in this order. The
leading name or count is the check's text button (underlined, no colour): items select them on the
canvas, an operation selects and unfolds its row and focuses the field named (P28).

| Kind | Symbol | Text (singular / plural) | Button does |
|---|---|---|---|
| block | `OctagonX` | "1 item lies outside the 400 x 300 mm bed." / "<n> items lie outside the 400 x 300 mm bed." (as built; raster items counted like any item) | selects them |
| block | `OctagonX` | "Engrave 1 needs laser mode M4 for Grey." | unfolds, focus Dither |
| block | `OctagonX` | "Photo 1 would need 120,000,000 dots; the limit is 50,000,000." | selects the item |
| warn | `TriangleAlert` | "1 image is coarser than its Interval." / "<n> images are coarser than their Interval." | selects them |
| warn | `TriangleAlert` | "2 items overlap and burn twice." | selects them |
| warn | `TriangleAlert` | "1 image was reduced and loses detail." / "<n> images were reduced and lose detail." | selects them |
| warn | `TriangleAlert` | "1 image cannot be read and is skipped." / "<n> images cannot be read and are skipped." | selects them |
| warn | `TriangleAlert` | "Engrave 1: Lowest power is above Power." (Grey only; criterion 32 b) | unfolds, focus Lowest power |

- The dot-limit line names the item by its name, also for a shape ("Ellipse 3 would need ..."), and
  drops "Raise the Interval." (P29, spec delta E3): the button selects the item and the line under it
  is the operation's row, so no instruction is needed (customer rule: no operating tips).
- The overlap line counts raster items, images and gradient shapes alike, so it says "items"
  (P30, spec delta E4: the spec says "images").
- Singular forms for every count (spec delta E5: the spec gives plurals only).

## 5. The Engrave test grid: Fill and Dither (criteria 44, 45, Frage 231 A)

In the Test grid bar (`0112` 4.1), row 1, Engrave kind only, a new group **Fill** after the Shared
values group (with its Crosshatch toggle) and before Cell size:

| Control | Look | Size |
|---|---|---|
| Fill | `role="radiogroup"` "Fill", two symbol-only segments of the `0112` segment look: `SolidPaintIcon` (tooltip and name "Solid"), `GradientPaintIcon` (tooltip and name "Gradient"); default Solid | 2 x 36 |
| Dither | shown only with Gradient: the Material button look of the bar (Trace Preset button), muted "Dither" then the mode and a 10px chevron; opens the bar's menu (`role="menu"`, `menuitemradio` rows of 32px, `Check` at the picked one): Threshold, Ordered, Floyd-Steinberg, Jarvis, Atkinson, Stucki (no Grey); default Floyd-Steinberg | 176 |

- A bar may open a menu; the panel may not (`0112` 4.4). The menu keys are those of the material menu.
- Fill and Dither are one wrap item. A change is a commit: the grid regenerates, one step "Edit test
  grid". Dither is never an axis segment (criterion 44).
- With Gradient each cell is a black-to-white Gradient rectangle (Angle 0), drawn grey on the laser
  page and as dots in the Operations view (section 3.1); the generated rows' unfolded bodies show the
  Image settings group with the chosen Dither (hand-editable, as every generated row).
- Fill uses symbols (P31) where the spec says "a closed pick": two values are a segment in a bar, and
  the glyphs are the Style tab's paint glyphs (spec delta E6).

## 6. Keyboard routes

| Action | Route |
|---|---|
| Image settings | Tab from the unfolded row's lower part into the group (Dither, the mode field, Brightness, Contrast, Gamma, Invert) |
| Dither | Enter, Space or Down opens; Up / Down, Home / End; Enter picks; Escape closes |
| Fields | Enter commits, Escape reverts, Up / Down step (Shift 10) |
| Invert | Space or Enter |
| Check lines | Tab to the text button, Enter |
| Test grid Fill and Dither | Tab in the bar order; arrows in Fill; Enter or Down opens the Dither menu |

## 7. Undo steps

"Edit settings" (every Image settings commit, the Dither pick, Invert), "Edit test grid" (Fill,
Dither in the bar). No step for opening the pick, the list preview, the Operations or Toolpath view,
zoom or the play bar.

## 8. Accessible names

| Element | Name | State or description |
|---|---|---|
| Group | `role="group"` "Image settings of Engrave 1" | |
| Dither pick | "Dither" | value the mode; the list a `radiogroup` "Dither" |
| Fields | "Threshold, percent", "Brightness, percent", "Contrast, percent", "Gamma", "Lowest power, percent" | `aria-invalid` with the chip |
| Invert | "Invert" | `aria-pressed` |
| Bar Fill | radiogroup "Fill", radios "Solid", "Gradient" | |
| Bar Dither | "Dither: Floyd-Steinberg" (button), `aria-haspopup="menu"` | |

## 9. States

- **No raster item in the operation:** no group; the row as `0111` built it.
- **Mixed items** (raster and vector in one Engrave): the group is shown; Crosshatch and Interval apply
  to the vector hatch as before.
- **Damaged settings:** the pick shows "Damaged"; the check "<n> operations have damaged settings and do
  not run." as built.
- **Large:** a 200 x 200 mm image at 0.1 mm: the Operations view keeps the previous dots while it
  rebuilds; the Toolpath view zoomed out draws the shaded area (criterion 37 e).
- **Unreadable image:** skipped, its check line; in the Operations view its frame only, dimmed as Not
  assigned (P32).
- **M3 laser with Grey:** the block line; the Toolpath view still draws the rows, so the maker sees what
  would run (P33).

## 10. What is not added

No image dialog, no "Adjust image" window, no preview window, no dither sample tiles, no tone curve,
no sliders, no per-image settings, no layer table, no new colour token, no new shortcut, no success
notice.

## 11. Proposals (vetoable at the demo, with defaults)

| # | Proposal | Default |
|---|---|---|
| P19 | No heading over the Image settings group; hairline only | as described |
| P20 | Lowest power in the Threshold row (row 2) | as described |
| P21 | Dither pick without a leading symbol | as described |
| P22 | Dither list with names only | as described |
| P23 | Keyboard focus in the open list previews on the canvas, pointer hover does not | as described |
| P24 | Crosshatch stays shown with only raster items | as described |
| P25 | Operations view always shows a raster item's dots | always |
| P26 | Grey mode dots and runs drawn at an opacity from 20 % to 100 % by power | as described |
| P27 | Zoomed out, the item's travel lines are not drawn | as described |
| P28 | Check text buttons: items select, operations unfold and focus the named field | as described |
| P29 | Dot-limit line names the item and drops "Raise the Interval." | "Photo 1 would need 120,000,000 dots; the limit is 50,000,000." |
| P30 | Overlap line says "items" | "2 items overlap and burn twice." |
| P31 | Test grid Fill as two symbol segments | Solid |
| P32 | An unreadable image shows its frame only, dimmed | as described |
| P33 | Grey on M3: rows still drawn in the Toolpath view | as described |
| P34 | Widths Threshold 124, Lowest power 132, Brightness 124, Contrast 100, Gamma 100 | as described |

## 12. Spec deltas for the product owner

All seven are folded into the specification (2026-11-18): E1 in criterion 11, E2 in 20, E3 in 34, E4 and E5 in 35 and 36, E6 in 44, E7 in the new criterion 32 b. The "no interval" check was dropped (ADR 0069 §6; this note's table lost its row).

- **E1** Criterion 11: Lowest power in the place of Threshold (row 2), not last (P20).
- **E2** Criterion 20: the Operations view shows a raster item's dots always, not only while an Image
  setting is edited (P25).
- **E3** Criterion 34: name the item, not "Image 1", and drop "Raise the Interval." (P29).
- **E4** Criterion 35: "<n> items overlap and burn twice." (raster items include gradient shapes) (P30).
- **E5** Criteria 31 to 36: add the singular forms of section 4.
- **E6** Criterion 44: Fill is a two-segment radiogroup with the paint glyphs, Dither a bar menu (P31).
- **E7** (decided: allowed, warn line, as proposed) Lowest power above the set's Power is not defined (the Power can also change later by a
  material pick, so a refusal at typing time is not enough). Proposal for the PO: allowed, and every
  burned dot then runs at Power (the ramp collapses), with a warn line "Engrave 1: Lowest power is above
  Power." (`TriangleAlert`, unfolds the row). The PO decides; the UX needs only that line.

## Links

Spec: `specs/0123-raster-engraving/specification.md`. Drafts: `docs/design-drafts/2026-11-raster/`.
Gradient fill: `specs/0124-gradient-fill/ux-notes.md`. Built notes: `specs/0111-laser-operation-tools/ux-notes.md`,
`specs/0112-test-grid-and-materials/ux-notes.md`, `specs/0101-laser-operations/ux-notes.md`, `specs/0029-laser-job-preview-and-output/ux-notes.md`.
