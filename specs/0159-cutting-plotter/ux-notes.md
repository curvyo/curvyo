# 0159 Cutting plotter, slice 1 (MVP): UX notes

Revised 2026-11-28 by `docs/design-drafts/2026-11-machine-families/ux-unify.md` (one output page for all four families): the page is the active mat's cut area, the mat drawn around it (supersedes 4 "Page", 5.3 veil and cut-area line, C4); the Kind switch is one row of 32px symbol buttons (supersedes 2 and C1); the machine fields leave the Machine block for the Machines tab (supersedes 4 "Machine block"); Mat size and Grip are Choice strips (6); Cut and Draw are in the Select bar, the rows and Output in the Operations tab, Toolpath only at the page label, the play bar on the canvas, white laser fields. Withdraws the `plan.md` deviation "one panel in the Document tab".

ux-engineer, 2026-11-27. Slice 1 only (criteria 1 to 18). Written from `specification.md` (Fragen 416 to 420, 427 to 429 answered), `docs/design-drafts/2026-11-machine-families/plan.md` and the three mat photos in that folder, and checked against the built laser page: `specs/0098-page-kinds/ux-notes.md` (Add page, kind chip), `specs/0101-laser-operations/ux-notes.md` (row, kind looks, casing, Cuts last), `specs/0111-laser-operation-tools/ux-notes.md` (Select-bar Operation group, unfolded row), `specs/0029-laser-job-preview-and-output/ux-notes.md` (Toolpath view, play bar, output group, checks, total, Export), `specs/0150-operation-always-has-numbers/ux-notes.md` (Default pair), `specs/0102-laser-fixtures/ux-notes.md` and `specs/0113-fixture-sets-and-pins/ux-notes.md` (Fixtures tab, fixture looks), `docs/design-system.md` ("Interaction conventions"). Customer rules applied: the new families look and work like the laser page; no operating tips or hint labels; no success notice except the export notice; hidden, not disabled; no popups in the panel; kind is shape, colour only says "cut"; nothing moves under the pointer. Items marked (Proposal) are vetoable at the demo; section 14 lists them as C1 to C16. `0160-embroidery/ux-notes.md` and `0161-cnc-output/ux-notes.md` refer to sections 2 and 3 of this file.

Terminology: "ink" is `--toolbar-icon`; "field" is the mat's printed 12 x 12 in (or 12 x 24 in) grid area; "sheet" is the whole plastic mat; "mat origin" is the field's top-left corner as fed (next to the logo).

## 0. What is reused

| Part | Reused from | Change for the plotter |
|---|---|---|
| Add page, kind chip | `AddPageGroup.tsx`, `KindChip` (`0098` 1, 2) | three more kinds, rows of two (section 2) |
| Machine block | `MachineBlock.tsx` (`0026`, `0029` 8, `0150` 2) | plotter fields; laser-only rows hidden (section 4) |
| Fixtures tab | `FixturesTab.tsx` (`0113` 3) | one section "Mat" instead of Machine sets and Job fixtures (section 6) |
| Select-bar Operation group | `SelectToolbar.tsx` (`0111` 3) | two kind buttons, Cut and Draw |
| Operation row, unfolded row, Cuts last | `OperationRow.tsx`, `OperationSettingsBlock.tsx` (`0101` 4, `0111` 6, `0029` 6.1) | plotter numbers; no Material pick in slice 1 |
| Operations view, Toolpath view, play bar | `PageFrames.tsx`, the `0029` 3 to 5 look | Draw look added; Design Space play axis (section 8) |
| Output group, checks, total, Export | `OutputGroup.tsx`, `OutputChecks.tsx` (`0029` 6.3 to 6.6, 7) | no Start pick; plotter texts (section 9) |

## 1. Principle

A plotter page is the maker's mat. The page frame is the mat's printed field, so the rulers, the positions in the panel and the positions in the file all count from the same corner the maker sees on the real mat. The sheet, its printing and its notch are drawn around and over that field exactly as on the photos, in the grip's colour, so the maker recognises his mat at a glance; the printing is reference, never content. Everything else is the laser page: the same Select-bar buttons, rows, toggles, checks and Export, with two kinds instead of five.

## 2. Add page and page identity (all three families)

- **Kind switch** (`0098` 2): six kinds, Drawing, Sketch, Laser, **Plotter**, **Embroidery**, **CNC**. Three to a row does not fit: in the 228px content width a third-width segment is 74px, and "Embroidery" at 13px (about 70px) plus the 14px glyph and 4px gap needs about 90px. The segments therefore wrap **two to a row** (three rows of 114px segments), by the "Bar overflow" rule (whole segments, never a split word) (C1). Keys unchanged: one Tab stop, Left and Right move through all six in reading order.
- **Glyphs** (Lucide, 14px in the switch, 12px in the chip): Plotter `Slice`, Embroidery `Spool`, CNC `Drill` (C2). `Scissors` stays free for the embroidery trim mark (`0160` 2).
- **Machine list**: shown for every output kind (Laser, Plotter, Embroidery, CNC), listing only that family's profiles (criterion 2). Preselected: the last profile picked for that kind, else the family's built-in (Cricut Maker 3, W6 EU7, Generic GRBL router). The remembered profile is kept per kind (C3).
- **Names and steps**: "Plotter N", "Embroidery N", "CNC N"; History "Add plotter page", "Add embroidery page", "Add CNC page".
- **Kind chip** in the page list row, panel header, canvas label and status bar (`0098` 1): "Plotter", "Embroidery", "CNC". Width check: "Embroidery" with its glyph is about 75px at 11px, about the width of "Laser (off)", so the `0098` 3 rule holds (the name keeps about 65px; tooltip and header carry the rest).
- **Row line 2 and status bar**: "<profile> · <page size>" as on a laser page ("Cricut Maker 3 · 12 × 12 in" shown in the document unit: "304.8 × 304.8 mm").
- **Tabs** of an output page of these families: File, Document, Style, Layers, History, Fonts, Snapping, Grid, Share as on a laser page, plus Operations and Fixtures; Materials only on laser pages (CNC has Tools, `0161` 5). Tool rail unchanged (Text included).

## 3. Shared output-page rules (all three families)

- **Laser-only controls are absent, not disabled**: Laser mode, Max power, Motorised Z axis, Air assist, the Start pick, Fixture jobs, Machine sets, Job fixtures, the Test grid tool, Frame and Focus point. They leave the tree on these pages.
- **The Output row** (`0029` 8) shows the profile's format read-only: "Design Space SVG", "HPGL", "DST", "GRBL".
- **Casing**: every operation, stitch and toolpath line is drawn over a 1px casing on each side (`0101` 13.2), so it never depends on the mat, hoop or stock under it.
- **The export notice and the error dialog** are the laser's (`0029` 7): one canvas notice "Exported <file>." (browser "Downloaded <file>."), 3 s; Cancel does nothing.
- **Light theme only**, as every canvas token so far; the new tokens get no dark values.

## 4. The page and the Document tab

- **Page = the active mat's printed field** (C4, mismatch M1): 12 x 12 in (304.8 x 304.8 mm) or 12 x 24 in (304.8 x 609.6 mm), not editable, origin at the mat origin. The ruler zero, the panel positions, the export positions (criterion 11) and the mat's cm numbering share one corner. Switching the mat changes the page size in the same step. A profile without mats (Generic HPGL) keeps page = work area (600 x 1000 mm), origin as its Origin corner.
- **Machine block** of a plotter page, top to bottom: profile button row and state row (built), then:
  - Mat profile: "Cut area" line (plain text, the `0098` 9 Work area look) with the active mat's cut area, "292.1 × 292.1 mm". No Origin corner control (the mat fixes it).
  - Matless profile: "Work area" line, Origin corner (built control, default bottom left).
  - **Max speed** (mm/s), **Default pressure** (%) and **Default speed** (mm/s): the `0150` 2 fields and commit rules with the words changed; History "Edit default pressure", "Edit default speed". On a Design Space SVG profile these three fields are absent (they change nothing there, as the row numbers of C13); their values stay stored in the copy (criterion 8).
  - **Output** row, read-only (section 3).
- Connection group: absent in slice 1 (sending is slice 6).

## 5. The mat on the canvas (criteria 3 to 5)

### 5.1 Measured geometry (12 x 12 in mat)

Method: in each photo the four field corners were picked at 4 to 6 times zoom on a pixel grid and mapped onto an exact 12 x 12 in square by a projective transform; the sheet corners (as the intersection of the straight edges next to each rounded corner), the notch and the centre cross were mapped through it. Three mats: StandardGrip (`cricut-mat-standardgrip.jpg`), LightGrip (`cricut-mat-lightgrip.jpg`), StrongGrip (`cricut-mats-stack.jpg`, top mat). The field size is the input (printed 1 in grid), not a result. Checks: the centre cross maps to (6.00, 6.01), (5.98, 5.98), (6.02, 6.03) in; the cm-15 tick to 5.935 in (5.906 exact, 0.7 mm off).

| Quantity | Measured (min to max of the samples) | Drawn (C5) |
|---|---|---|
| Field (printed grid) | input | 12 x 12 in, 304.8 x 304.8 mm |
| Sheet margin left | 0.43 to 0.57 in | 0.5 in (12.7 mm) |
| Sheet margin right | 0.46 to 0.54 in | 0.5 in (12.7 mm) |
| Sheet margin top (logo side) | 0.95 to 1.06 in | 1.0 in (25.4 mm) |
| Sheet margin bottom | 0.94 to 1.05 in | 1.0 in (25.4 mm) |
| Sheet | from the margins | 13 x 14 in, 330.2 x 355.6 mm, corner radius 3 mm |
| Notch: a hole in the bottom margin, not an edge cut (M2) | flat top 1.14 to 1.15 in wide, centre at x 5.93 to 5.95 in, top 0.33 to 0.37 in below the field, apex 0.71 in below | inverted triangle, top edge 29 mm wide centred at x = 152.4 mm, top 9 mm and apex 18 mm below the field, corners rounded 1.5 mm |
| Grid line width | 2 to 3 px at about 60 px per in, JPEG-blurred: 0.6 to 1.0 mm | 0.6 mm |
| Centre cross | at the field centre; heavier than the grid; arms read 7 to 12 mm | arms 9 mm each side, 1.2 mm wide |
| Cut area (not printed, not measurable from the photos) | none | 11.5 x 11.5 in, centred, 0.25 in (6.35 mm) inside the field (C6, M3) |

**Uncertainty.** About 60 px per inch, corner picks to about 2 px, no lens correction, and the StandardGrip lay over a laptop edge (one grid line near the bottom mapped 0.3 in off there). The sheet values are therefore good to about ±0.1 in (±2.5 mm); the drawn values are the round numbers every sample brackets. None of this reaches the output: the file counts from the field corner, which is exact. Two facts would settle the sheet: the customer measures the sheet's width and height with a ruler (M4). The **12 x 24 in mat** is extrapolated: same margins, sheet 13 x 26 in, notch centred in its bottom margin, cut area 11.5 x 23.5 in centred; to be confirmed by a photo (spec fact 1).

### 5.2 Printing (Mat grid on)

As on the photos, in `--mat-print` (white), at the mat's scale (it thins when zoomed out, like the bed tapes of `0102` 5.1):

- **Grid**: lines every 25.4 mm across the field, the field's border included; 0.6 mm wide, never under 1 device px.
- **Centre cross**: at the field centre (152.4, 152.4 mm on the 12 x 12 mat; 152.4, 304.8 mm on the long mat), arms 9 mm, 1.2 mm wide, over the grid lines.
- **cm rulers** (counted from the mat origin): along the top edge in the top margin and along the left edge in the left margin. A 2.5 mm tick outward from the field edge at every whole cm; the number 1 to 30 (1 to 60 on the left of the long mat) centred on its tick, beyond it, digits 3 mm high, upright as fed.
- **Inch numbers** (counted from the field's bottom-right corner, as printed): along the right edge in the right margin and along the bottom edge in the bottom margin; each inch line runs on as a 2 mm tick past the field edge; the number centred on it, 3 mm high, **turned 180 degrees** as on the mat (C7). Right edge: 1 at the line 1 in above the bottom, up to 12 at the top corner (24 on the long mat); bottom edge: 1 at the line 1 in left of the right corner, up to 12 at the left corner.
- **Grip word**: the grip's name ("StandardGrip") 3 mm high in the top margin at the right end, and turned 180 degrees in the bottom margin at the left end, as printed (C8).
- **No logo.** The Cricut word mark is a trademark and is not drawn; its place in the top-left and bottom-right margins stays empty (C8).
- **Small zoom**: numbers and the grip word are not drawn below 6 device px digit height; ticks below 3 device px apart; grid lines stay at 1 device px down to 4 px apart, then go. The sheet, notch, cut area and veil always draw.
- The printing is not hit, not selectable, not a snap target unless snapping to the grid is on (criterion 5), never exported.

### 5.3 Sheet, cut area, colours

- **Sheet**: filled with the grip colour (section 2 of the tokens below), 1px ink outline, corners rounded 3 mm, drawn under the page; the margins lie on the pasteboard around the page frame. The notch is a hole: it shows the pasteboard.
- **Cut area** (C6): the band between the field edge and the cut area is veiled with `--mat-print` at 45 % (the mat reads lighter where the blade cannot go; veil against mat 1.46:1, informational); the cut area's edge is a 1px solid ink line (5.1:1 on every grip colour). Inside the cut area the mat shows at full colour.
- **Page background**: on a mat page the document background is not drawn inside the frame; the mat is the ground. The page frame line is the field's border (drawn as the printed border when the grid is on, as a 1px ink line when it is off).
- **Draw order**, bottom to top: pasteboard, sheet, veil, printing, cut-area line, items (their looks of section 7.3), origin marker, selection.
- **Grid off** hides grid, cross, rulers, inch numbers and the grip word; sheet, notch, veil and cut-area line stay (criterion 5).

### 5.4 Tokens (light; measured with the WCAG 2.x formula)

| Token | Value | Photo sample (room light) | White print on it | Ink on it | `--cut` on it |
|---|---|---|---|---|---|
| `--mat-lightgrip` | `#8FB3CC` | `#9FA5AB` | 2.21:1 | 5.11:1 | 2.66:1 |
| `--mat-standardgrip` | `#9CB87D` | `#9E9C70` | 2.20:1 | 5.15:1 | 2.68:1 |
| `--mat-stronggrip` | `#B6A7DB` | `#A599AE` | 2.20:1 | 5.14:1 | 2.67:1 |
| `--mat-fabricgrip` | `#D6A1B7` | no photo (C9) | 2.18:1 | 5.19:1 | 2.70:1 |
| `--mat-print` | `#FFFFFF` | white | | 11.3:1 | 5.88:1 |

- The four grips share one luminance (white print about 2.2:1, ink about 5.1:1), so switching the grip changes the hue and nothing else. The hues follow the photos, cleaned of the room's yellow cast.
- The white printing at 2.2:1 is informational, like `--grid-minor`: the app's rulers and the panel carry every position; the printing is for recognition.
- `--cut` on a mat is 2.67:1, under 3:1; the casing rule (section 3) puts every cut line on white, 5.88:1, as `0101` 13.1 argued for the pasteboard.

## 6. Fixtures tab: the Mat section (criteria 3, 5)

- Present on plotter pages whose profile has mats; absent on matless profiles (Generic HPGL). Header "Fixtures" with the Plotter chip. One section, heading "Mat" (the `0113` 3 heading). Machine sets and Job fixtures are absent (section 3).
- Rows, top to bottom, 244px column, 8px apart:
  1. **Size**: `SegmentGroup` `role="radiogroup"` "Mat size", one segment per mat of the profile ("12 × 12 in", "12 × 24 in"; the mat's own unit, whatever the document unit), 28px (36 coarse). One step "Set mat"; the page size changes with it (section 4).
  2. **Grip**: the `OutputPick` look (full width x 28, label "Grip" inside left, the value at the right with a 12px swatch of the grip colour before the word, `ChevronDown`), inline list of the four grips each with its swatch; current with `Check`. One step "Set grip". The swatch is shape-plus-word: the grip word is always beside it.
  3. **Mat grid**: label "Mat grid" left, `PanelSwitch` at the right edge (the `0029` 8 switch row), default on. One step "Mat grid" (criterion 5).
- The grip is the page's (one pick), kept when the size changes (C10).
- Snapping: the cut-area corners are snap targets (fixture points of `0060`); the field corners, the grid and the centre cross snap only while snapping to the grid is on (criterion 5) (C11).

## 7. Operations: Cut and Draw (criteria 7 to 9)

### 7.1 Kind buttons in the Select bar

The `0111` 3 Operation group with **two** buttons: **Cut** (the laser's Cut glyph, `--cut`, same meaning: the blade follows each outline) and **Draw** (custom 16px glyph: an S-curve 1.5px ink with a 3px dot at its start, the pen down). Names and tooltips "Cut", "Draw"; descriptions "Assigns to <name>" / "Creates <Kind> <N>"; press rules, Shift press and group rules of `0111` 3.3 and 4 unchanged.

### 7.2 Operations tab

- **Cuts last** switch on by default for a new plotter page (criterion 9: Draw before Cut), sections "Marking" (Draw) and "Cutting" (Cut) as `0101` 4.1 (C12).
- **Row** of `0029` 6.1: line 1 as built; line 2 run ring, "Cut · 6 items", time at the right; line 3 the numbers.
  - HPGL page: line 3 "40 %, 200 mm/s, 2 ×" for Cut, "40 %, 200 mm/s" for Draw ("typed" never appears: slice 1 has no material).
  - Design Space SVG page (C13, M5): line 2 has no time; line 3 reads "Set in Design Space" (11px muted). The numbers stay stored (criterion 8, so a switch to an HPGL profile shows them) but are not shown and cannot be edited: fields that change nothing are removed (hidden, not disabled). Such a row does not unfold; its press only selects.
- **Unfolded row** (HPGL page; `0111` 6.2 layout, no Material pick in slice 1): Cut: Pressure (112, "%") and Passes (112, "×") in one row, Speed (228, "mm/s"). Draw: Pressure and Speed. No lower part. Commit, validation chip and one step "Edit settings" as `0111` 6.4.

### 7.3 Operations view look (canvas)

| Kind | Look |
|---|---|
| Cut | solid `--cut` line, 1 device px, on each outline (the laser Cut look) |
| Draw | solid ink line, 1.5px, on the centre line of each item |
| Not assigned, Output off, Show off | as `0101` 13.2 |

Kind chips over a selected object (`0101` 13.3) with the Cut and Draw glyphs.

## 8. Toolpath view and play bar (criterion 10)

- The `0029` 3 to 5 toggle, look and bar. Cut moves solid `--cut` 1px, Draw moves solid ink 1.5px, travel `--lane-line` dashed 4 on 3, arrowheads and run rings as built. Passes are drawn once.
- **HPGL page**: slider axis and readout are time, as the laser ("1 min 2 s of 3 min 40 s").
- **Design Space page** (C14): Design Space sets speeds, so there is no time. The slider runs over path length and the readout reads "0.62 of 1.80 m". Play still plays the whole job in 10 s.

## 9. Output group and Export (criteria 6, 11 to 16)

After the list, as `0029` 6.3: heading "Output", **machine line** ("Cricut Maker 3 · Design Space SVG", "Generic HPGL cutter · HPGL"), **checks**, **total**, **Export**. No Start pick.

- **Checks** (`0029` 6.4 look and buttons): block "3 items lie outside the mat's cut area." / "… outside the 600 × 1000 mm work area." (criterion 6; the count is a select button); block "Nothing runs. Assign items to an operation."; warn "The machine is Generic HPGL cutter; its limits may not match yours."; the not-assigned, hidden and Output-off warns of `0029` 10 unchanged.
- **Total**: HPGL "About 3 min 40 s" then "Runs 2 operations, 7 items." and "Cut 1.24 m · draw 0.56 m · travel 0.88 m". Design Space: no time line; the other two lines.
- **Export** (`0029` 6.6): the save dialog with "<project> - <page>.svg", file type "Design Space SVG", or ".plt", "HPGL" (C15, M6). Notice and error as section 3. After a successful export nothing else changes (no fixture jobs on a plotter page).

## 10. Keyboard

As the laser page: kind buttons in the Select bar (Enter, Space, Shift+Enter), digits 1 to 9 assign to operation n, rows and unfolded bodies (`0111` 8), Toolpath view and bar (`0029` 9). Fixtures tab: Tab through Mat size (one stop, Left and Right select), Grip (Enter opens, arrows, Enter picks, Escape closes), Mat grid (Space). No new shortcut.

## 11. Texts (each at most 70 characters)

| Text | Kind |
|---|---|
| "3 items lie outside the mat's cut area." | block |
| "3 items lie outside the 600 × 1000 mm work area." | block |
| "The machine is Generic HPGL cutter; its limits may not match yours." | warn |
| "Set in Design Space" | row line 3 |
| "Pressure is 1 to 100 %." / "Speed is 1 to 800 mm/s." / "Passes is 1 to 10." / "Enter a whole number." | validation chip (the 800 is the copy's Max speed) |
| "Default pressure limited to 100 %." / "Default speed limited to 800 mm/s." | Action notice (`0150` 2) |
| "Exported Card - Plotter 1.svg." / "Downloaded …" | export notice |

## 12. Accessible names

Kind switch segments "Plotter", "Embroidery", "CNC"; Mat size radiogroup "Mat size", segments "12 × 12 in", "12 × 24 in"; Grip pick "Grip" (value the grip); Mat grid switch "Mat grid" (`aria-checked`); kind buttons "Cut", "Draw"; Design Space row name "Cut 1, run 2, set in Design Space"; machine fields "Max speed (mm/s)", "Default pressure (percent)", "Default speed (mm/s)"; Output line "Output: Design Space SVG". The mat drawing is `aria-hidden` (canvas).

## 13. Undo steps and states

- Steps: "Add plotter page", "Set mat", "Set grip", "Mat grid", "Assign to <name>", "Edit settings", "Edit default pressure", "Edit default speed", "Edit max speed", "Export job file" (a step only if the export changes stored state; on a plotter page it changes none, so no step, C16). No step for the toggles, the slider, Play, opening a pick, a cancelled dialog.
- **Damaged mat value** (criterion 17): the page reads the default mat (12 x 12 in StandardGrip, grid on); it is read as default with a notice (criterion 17); nothing on the page marks it.
- **Nothing runs**, **many items**: as `0029` 13; the mat drawing is one cached layer per zoom step (it never redraws on an item edit).

## 14. Decisions for veto (Proposal)

- **C1** Six kinds wrap two to a row in the Kind switch.
- **C2** Glyphs: Plotter `Slice`, Embroidery `Spool`, CNC `Drill`.
- **C3** The last picked profile is remembered per kind.
- **C4** The plotter page is the active mat's printed field (origin = mat origin); a matless profile's page is its work area.
- **C5** Sheet 13 x 14 in with 0.5 in side and 1.0 in top and bottom margins; notch, line and cross sizes of 5.1.
- **C6** Cut area centred, 0.25 in inside the field; veil outside it and a 1px ink edge.
- **C7** Inch numbers turned 180 degrees and counted from the bottom-right corner, as printed.
- **C8** The grip word drawn as printed; no Cricut logo.
- **C9** FabricGrip colour from the published pink, not from a photo.
- **C10** One grip per page, kept across a size change.
- **C11** Cut-area corners are snap targets; field corners, grid and centre cross only with grid snapping on.
- **C12** Cuts last on by default; Draw is Marking, Cut is Cutting.
- **C13** On a Design Space page the numbers are hidden, line 3 reads "Set in Design Space", the row does not unfold.
- **C14** Design Space play axis is path length, readout in metres.
- **C15** File name "<project> - <page>.<ext>" as the laser export.
- **C16** No history step for an export on a plotter page.

## 15. Mismatches for the product owner and the architect

- **M1 (criteria 1, 4, 11):** criterion 1 makes the page the profile's work area; with two mat sizes, a page origin at the mat origin and an SVG at mat scale, the page should be the active mat's printed field (C4). Architect: page size follows the mat (a mat switch resizes the page in one step).
- **M2 (criterion 4 a):** the notch is a hole in the bottom margin (top 9 mm below the field, 7 mm above the sheet edge), not a cut-out of the bottom edge.
- **M3 (criterion 4 c):** "offsets of the cut area measured from the photos" cannot be done: the cut area is not printed. Proposal C6; verified at the demo by cutting an 11.5 in square placed at 0.25, 0.25 in through Design Space (fact 2).
- **M4 (facts):** add two facts: the sheet's width and height measured with a ruler, and a photo of a 12 x 24 in mat.
- **M5 (criterion 8):** "always has numbers" holds in storage; on a Design Space page the operation numbers and the Machine block's Max speed, Default pressure and Default speed are not shown or editable (C13, section 4).
- **M6 (criterion 11):** the file name `<page name>.svg` differs from the laser's "<project> - <page>"; Proposal C15 keeps one rule; the golden files are unaffected.
- **M7 (criterion 5):** the printing also carries the grip word and the inch ticks; the inch numbers run along the right and bottom edges counted from the bottom-right corner, turned 180 degrees (photos); on the long mat the right edge counts to 24 (extrapolated).
- **M8:** the spec's "UX notes" section should point to this file.

## 16. Design system (added now: tokens; rows at acceptance)

- Tokens `--mat-lightgrip`, `--mat-standardgrip`, `--mat-stronggrip`, `--mat-fabricgrip`, `--mat-print` (added to "Color tokens" with this draft).
- Rows at acceptance: "Mat (`0159`)" (geometry, printing, veil, draw order), "Kind switch: six kinds, two to a row", "Kind chip: Plotter, Embroidery, CNC", "Output page of a non-laser family" (the section 3 rules), "Draw look (`0159`)".

## 17. Implementer checklist

- Add page: six kinds two to a row; machine list per family; names, steps, chips.
- Page = active mat field; mat switch resizes; matless profile keeps its work area.
- Mat drawing of 5.1 to 5.3 at mat scale with the zoom rules; tokens of 5.4; no logo; grid off hides the printing only.
- Fixtures tab Mat section: size segments, grip pick with swatches, Mat grid switch; steps.
- Machine block: plotter fields, laser-only rows absent, Output row.
- Select bar Cut and Draw; Cuts last default on; rows with line 3; Design Space rows without numbers, no unfold.
- Operations and Toolpath looks; Design Space play axis in metres.
- Output group without Start; checks, total, Export with file type; notices.
