# 0160 Embroidery, slice 1 (MVP): UX notes

ux-engineer, 2026-11-27. Slice 1 only (criteria 1 to 20). Written from `specification.md` (Fragen 420 to 424, 427, 428 answered), `docs/design-drafts/2026-11-machine-families/plan.md`, and checked against the laser page notes listed in `specs/0159-cutting-plotter/ux-notes.md` (header), whose sections 2 (Add page, kind chip) and 3 (shared output-page rules) apply here unchanged. Customer rules: the family looks and works like the laser page; no operating tips or hint labels; no success notice except the export notice; hidden, not disabled; no popups in the panel; kind is shape; **no font controls in the embroidery view** (Fragen 422 C, 423). Items marked (Proposal) are vetoable; section 13 lists them as E1 to E18.

Terminology: "block" is one item of one operation (criterion 10); "thread colour" is an operation's colour; "ink" is `--toolbar-icon`.

## 0. What is reused

| Part | Reused from | Change for embroidery |
|---|---|---|
| Add page, chip, Machine block, Output row | `0159` 2, 3; `0098`; `0029` 8 | profile W6 EU7, output DST |
| Fixtures tab | `0113` 3 | one section "Hoop" |
| Select-bar Operation group, rows, unfolded row | `0111` 3, 6; `0029` 6.1 | three kinds; a thread swatch on the row |
| Colour editing | the inline `ColourPicker` (`ColourPicker.tsx`, `0017` criteria 17 to 21) | opened inside the unfolded row |
| Toolpath view, play bar | `0029` 3 to 5 | stitch look; stitch axis; speed segments |
| Output group, checks, total, Export | `0029` 6.3 to 6.6, 7 | embroidery texts; no Start pick |

## 1. Principle

The page is the hoop. The maker assigns lines and areas to Running stitch, Tatami fill or Satin with the same three-symbol press as on a laser page; each operation is one thread colour and the list order is the sewing order. The Toolpath view sews the design stitch by stitch from the same list that writes the DST, so what plays is what the machine will do. Nothing about fonts is added: text is drawn with the normal Text tool and Fonts tab and simply assigned like any other item.

## 2. Tokens and symbols

No new colour token. Thread colours are the maker's data, not tokens; they get a visibility rule instead:

- **Thread casing** (E1): every stitch line, satin zigzag and fill hatch in a thread colour is drawn over a 1px casing on each side in **white** (`--selection-casing`) or **ink**, whichever contrasts more with that thread. Measured worst case over the whole sRGB cube (5-step grid): 3.36:1 (a mid olive `#7D9614`), so every thread line reads at 3:1 or better against its casing on any ground. Examples: white thread 11.3:1 on its ink casing; black 21:1 on white; `#808080` 3.95:1 on white.
- Jumps `--lane-line` dashed 4 on 3 (the laser's travel); trims and the head ink; run rings as `0029` 4.

| Control | Symbol (16px custom on the 16 grid, ink, unless Lucide is named) |
|---|---|
| Running stitch | three 3px dashes on a horizontal line, 1.5px |
| Tatami fill | a square of four rows of short dashes, alternate rows offset by half a dash (brick) |
| Satin | a 6px-high zigzag band across the glyph, 1.25px |
| Hoop (Fixtures tab glyph on embroidery pages) | a rounded rectangle with a smaller one inside |
| Trim mark | Lucide `Scissors` 10px |
| Speed | text segments "1×", "4×", "16×" |

## 3. The page, the hoop and the Document tab (criteria 1 to 4)

- **Page**: the profile's largest hoop's sewing field plus 20 mm on each side (190 x 290 mm for the EU7), not editable (criterion 1). The hoop is centred on it.
- **Hoop drawing** (E2), at the drawing's scale, the `0102` holder look (no new colour):
  - **Frame**: a band 12 mm wide, 8 mm outside the sewing field, outer and inner corners rounded (14 and 10 mm), opaque `--toolbar-bg` ground, 1.5px ink outline on both edges with casing. A mounting tab 40 x 16 mm on the left side, centred, same look (where the hoop attaches to the arm; to confirm at the demo).
  - **Sewing field**: a 1px solid ink rectangle with casing; the hoop template marks: a 6 mm ink tick inward at the middle of each side and a 10 mm cross at the centre, 1px ink. Everything inside the field is the page background (the fabric).
  - Orientation: 150 mm wide, 250 mm high (long side front to back) (E3).
  - Not hit, not selectable; the field's corners, side middles and centre are snap targets (fixture points of `0060`).
- **Machine block**: profile row and state row (built), "Hoop" line (plain text, the Work area look) "150 × 250 mm", **Speed** field ("stitches/min", 100 to 1,500, default 600; the `0150` field look; History "Edit speed"), "Max stitch" plain line "12.1 mm" (the DST limit, not editable in slice 1), **Output** row "DST". Laser-only rows absent (`0159` 3).

## 4. Fixtures tab: the Hoop section (criterion 3)

- Header "Fixtures", Embroidery chip. One section, heading "Hoop". One row: `SegmentGroup` `role="radiogroup"` "Hoop", one segment per hoop of the profile ("150 × 250 mm", "100 × 100 mm"); more than three hoops (a copied profile) use the `OutputPick` inline list instead. One step "Set hoop". The page size does not change (it is the largest hoop's); the frame and field redraw centred.
- Nothing else in the tab in slice 1.

## 5. Operations (criteria 5, 6, 9, 10)

### 5.1 Kind buttons

The `0111` 3 Operation group with **three** buttons: **Running stitch**, **Tatami fill**, **Satin**, glyphs of section 2, names and tooltips the kind words. Press rules as `0111` 3.3. An item a kind cannot take (an open path in Tatami fill) is still assigned and named by a check (section 7) (E4, permit over refuse).

### 5.2 Rows

- **No Cuts last switch** on embroidery pages (absent): the list order is the sewing order.
- **Line 1**: glyph, then a **thread swatch** 12 x 12px (thread colour, 1px `--swatch-border`), 4px, the name (16px narrower than on a laser row), Show, Output, trash, Up, Down (E5).
- **Line 2**: run ring (filled with the thread colour when this operation starts a new colour, section 6.1), "Satin · 2 items", the time at the right (stitches at the profile speed).
- **Line 3**: Running "2.5 mm, 1 ×"; Tatami "0°, 0.25 mm, 3 mm"; Satin "0.4 mm". "typed" never appears (no fabric presets in slice 1).
- Accessible name adds the colour name: "Satin 1, red, run 3, 1 min 4 s" (the colour word of `colourName` in `frontend/src/lib/tracePaletteText.ts`, the Trace palette's naming).

### 5.3 Unfolded row (`0111` 6.2 layout, 228px)

1. **Thread**: the `OutputPick` look (full width x 28, label "Thread" inside left, at the right the 12px swatch and the hex "#C8102E"). A press unfolds the inline `ColourPicker` of `0017` directly under it, inside the row body (no popup); a pick or a hex entry commits on release or Enter, one step "Set thread colour"; Escape closes it.
2. Kind numbers (criterion 6), the bar number field look, label inside left, unit inside right:
   - Running stitch: **Stitch length** (112, "mm", 1 to 10, one decimal) and **Repeats** as a `SegmentGroup` "1×", "3×", "5×" (three values only, so a pick, not a field) (E6).
   - Tatami fill: **Angle** (112, "°", 0 to 179, whole degrees) and **Row spacing** (112, "mm", 0.10 to 2.00), **Max stitch** (112, "mm", 1 to 10, one decimal).
   - Satin: **Zigzag spacing** (112, "mm", 0.20 to 2.00). The width is the item's stroke (criterion 6); no field, no explanation.
3. Commit, validation chip and one step "Edit settings" per commit as `0111` 6.4.

### 5.4 Operations view look (canvas)

| Kind | Look (thread colour, with the thread casing) |
|---|---|
| Running stitch | the item's line dashed 6 on 2 screen px, 1.5px |
| Tatami fill | the closed areas hatched at the operation's Angle, 1px lines 5 screen px apart, no outline |
| Satin | a zigzag along the line, its amplitude half the stroke width at the drawing's scale, 6 screen px per zag, 1px |
| Not assigned, Output off, Show off | as `0101` 13.2 |

The look is abstract and cheap (it does not generate stitches); the stitches are the Toolpath view's. Kind chips over a selected object (`0101` 13.3) use the three glyphs.

## 6. Simulation: Toolpath view and play bar (criteria 12, 13)

### 6.1 Look

- The `0029` 3 toggle ("Toolpath view", `Route`) on the page label.
- **Stitches**: a straight line from needle point to needle point in the thread colour, 1 device px (2 px from 400 % zoom), with the thread casing; needle points as 2px dots only where a stitch is at least 12 screen px long.
- **Jumps**: `--lane-line` dashed 4 on 3 (the laser's travel look). **Trims**: the `Scissors` mark 10px ink in a 14px white circle at the trim point (E7). **Colour change**: the run ring of the operation that starts a new colour is filled with the new thread colour, its digit in white or ink by the casing rule (E8); a ring without a colour change stays white. Tie-ins and tie-offs are ordinary stitches.
- **Rings and arrowheads** as `0029` 4.
- **After the head: not drawn** (E9). On a laser page the rest shows pale; stitches overlap (a later fill covers an earlier run), so pale future stitches would hide the sewn ones. Before the head everything is drawn at full weight; the head is the `0029` ring.

### 6.2 Play bar

The `0029` 5 bar, left to right: **Play** (28px), **slider**, **readout**, **Speed**.

- **Axis**: stitch number (E10). Keys: Left and Right one stitch, Page Up and Page Down 10 %, Home, End (E11; the laser's 1 % arrow step is replaced because a stitch is the unit worth stepping).
- **Readout** 12px tabular: "1,204 of 8,960 · ■ 2 of 3" (stitch number, then a 10px swatch of that stitch's thread and the colour block's number; "of" muted) (E12). `aria-valuetext`: "Stitch 1,204 of 8,960, colour 2 of 3, red".
- **Speed**: `SegmentGroup` "Speed", "1×", "4×", "16×", 28px, segments 32px. **1× is the machine's speed** (the profile's 600 stitches per minute, 10 per second), so 16× sews 8,960 stitches in 56 s (E13). Default 16×. Session state, not a step.
- **Width**: the bar's minimum width is 440px on an embroidery page (the laser's 360px does not hold the speed segments) (E14).
- Start position, Play, Pause, edit-pauses, reduced motion as `0029` 5.

## 7. Output group and Export (criteria 4, 12, 14 to 18)

After the list, as `0029` 6.3: heading "Output", **machine line** "W6 EU7 · DST", **checks**, **total**, **Export**. No Start pick.

- **Checks** (`0029` 6.4 look; the name or count is the select or go-to button):
  - block "Tatami fill 1: 2 items lie outside the hoop." (criterion 4)
  - block "Nothing runs. Assign items to an operation."
  - warn "Satin 1: 1 item is too narrow and runs as running stitch." (criterion 9; a check, not a canvas notice, so it stays while true, E15)
  - warn "Tatami fill 1: 1 item has no closed area." (E4)
  - the not-assigned, hidden and Output-off warns of `0029` 10.
- **Total** (criterion 12): "About 14 min 56 s" (13px semibold), then "Runs 3 operations, 5 items." and "8,960 stitches · 2 colours · 4 trims · thread 41.2 m" (12px muted, tabular).
- **Export**: save dialog "<project> - <page>.dst", file type "Tajima DST" (M3); notice and errors as `0159` 3. No step (nothing stored changes).

## 8. No font controls (Fragen 422 C, 423)

Nothing is added for lettering: no font pick in the Select bar group, the rows, the unfolded body, the Fixtures tab or the output group. The Text tool and the Fonts tab stay exactly as on every page (removing them would change accepted behaviour); a text object is assigned like any other item and stitched by its glyph outlines or strokes (criterion 5).

## 9. Keyboard

As `0159` 10, plus: Hoop segments (one Tab stop, Left and Right); Thread pick (Enter opens the picker, its built keys, Escape closes); Repeats segments; play-bar slider keys of 6.2; Speed segments (one Tab stop, Left and Right). No new shortcut.

## 10. Texts (each at most 70 characters)

| Text | Kind |
|---|---|
| "Tatami fill 1: 2 items lie outside the hoop." | block |
| "Satin 1: 1 item is too narrow and runs as running stitch." | warn |
| "Tatami fill 1: 1 item has no closed area." | warn |
| "Stitch length is 1.0 to 10.0 mm." / "Angle is 0 to 179°." / "Row spacing is 0.10 to 2.00 mm." / "Max stitch is 1.0 to 10.0 mm." / "Zigzag spacing is 0.20 to 2.00 mm." / "Speed is 100 to 1,500 stitches/min." | validation chip |
| "8,960 stitches · 2 colours · 4 trims · thread 41.2 m" | total |
| "Exported Badge - Embroidery 1.dst." | export notice |

## 11. Accessible names

Hoop radiogroup "Hoop"; kind buttons "Running stitch", "Tatami fill", "Satin"; thread pick "Thread" (value "red, #C8102E"); Repeats radiogroup "Repeats"; fields "Stitch length, millimetres", "Angle, degrees", "Row spacing, millimetres", "Max stitch, millimetres", "Zigzag spacing, millimetres"; slider "Run position" (valuetext of 6.2); Speed radiogroup "Speed"; machine field "Speed (stitches per minute)".

## 12. Undo steps and states

- Steps: "Add embroidery page", "Set hoop", "Assign to <name>", "Set thread colour", "Edit settings", "Edit speed". None for the toggles, slider, Play, Speed, opening a pick, export.
- **Damaged hoop or numbers** (criterion 20): read as default with a notice; nothing marks the page.
- **Large**: 50,000 stitches: the Toolpath view draws the stitches up to the head from a cached path per colour block; scrubbing redraws only the block under the head; needle dots only where they fit (6.1).

## 13. Decisions for veto (Proposal)

- **E1** Thread casing: white or ink, whichever contrasts more (worst case 3.36:1).
- **E2** Hoop drawn as a 12 mm frame band 8 mm outside the sewing field with a left mounting tab and template marks.
- **E3** 150 wide x 250 high.
- **E4** An item a kind cannot stitch is assigned and named by a warn check.
- **E5** Thread swatch on line 1 after the glyph.
- **E6** Repeats as three segments 1×, 3×, 5×.
- **E7** Trims marked by a small scissors in a white circle.
- **E8** A colour change fills the operation's run ring with the new thread.
- **E9** Stitches after the head are not drawn (unlike the laser's pale rest).
- **E10** The slider's axis is the stitch number.
- **E11** Arrows step one stitch.
- **E12** Readout "1,204 of 8,960 · ■ 2 of 3".
- **E13** 1× is the profile's machine speed; default 16×.
- **E14** Embroidery play bar at least 440px wide.
- **E15** "Too narrow for satin" is a warn check, not a transient notice.
- **E16** No Cuts last switch; list order is sewing order.
- **E17** The Operations view look is abstract (dash, hatch, zigzag); stitches only in the Toolpath view.
- **E18** Page and hoop are not hit or selectable; the field's marks are snap targets.

## 14. Mismatches for the product owner and the architect

- **M1 (criterion 13):** "1x, 4x, 16x" has no base; Proposal E13 (1× = machine speed). The readout carries stitch number and colour (spec), unlike the laser's time-only readout.
- **M2 (criterion 9):** the notice "Too narrow for satin" becomes a warn check with the operation and count (E15).
- **M3 (criterion 14):** the file name `<page name>.dst` differs from the laser's "<project> - <page>" (as `0159` M6).
- **M4 (criterion 12, architect):** DST positions count from the design's start, which the W6 places at the hoop centre; the page and its rulers count from the page's top-left. The export must translate by the hoop centre (page centre); the readouts show no coordinates, so nothing on screen depends on it.
- **M5 (criterion 1):** page = largest sewing field + 20 mm; the drawn frame (8 mm gap, 12 mm band) fills that margin exactly.
- **M6:** the spec's "UX notes" section should point to this file.

## 15. Design system

No token. Rows at acceptance: "Hoop (`0160`)", "Thread casing (`0160`)", "Stitch look (`0160`)", "Play bar: stitch axis and Speed (`0160`)", "Thread swatch on operation rows (`0160`)".

## 16. Implementer checklist

- Add page Embroidery (`0159` 2); page size; Machine block with Hoop line, Speed, Max stitch, Output.
- Hoop drawing of section 3; Fixtures tab Hoop segments; step "Set hoop".
- Three kind buttons; rows with swatch, ring, time, line 3; unfolded Thread pick with the inline picker; numbers and chips.
- Operations view looks with the thread casing.
- Toolpath view: stitches, jumps, trims, colour-change rings, nothing after the head; play bar with stitch axis, readout, Speed, 440px minimum.
- Output group checks, total, Export "Tajima DST"; nothing font-related added anywhere.
