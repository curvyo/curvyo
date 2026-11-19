# 0135 Image optimisation: UX notes

ux-engineer, 2026-11-19. Written from `specification.md` (criteria 1 to 66; Fragen 272 A, 273 C, 274 B,
275 A, 276 B answered; 277 to 279 at their default A), `specs/0123-raster-engraving/ux-notes.md` (Image
row, Dither strip, test grid), the clone rules of `0049` and `0057` (override rows, tags), the font
preview pane of `0077`, the Trace bar of `0025` (a bar that replaces the Select bar) and
`docs/design-system.md` ("Hidden, not disabled", "No popups in the properties panel", "No operating
tips", "No notice after a success", "Choice strip", "Inline confirm", "Inline delete confirm", "List
row", "Canvas notice slot"). Drafts: `docs/design-drafts/2026-11-image-optimisation/` (01 to 05, PNG
beside each HTML). Customer rules applied: no invented colours (ink, muted ink, the existing blue of a
pressed or selected control, the existing red of a refusal, pale orange `--engrave-hatch` for dots,
the clone violet), symbol-only buttons with tooltip and accessible name, no tips, edits in place,
units inside the fields, plain fields and no sliders **except the one Strength slider**, the
tool-specific bar for tool settings (Crop), armed trash, arrows plus drag for order, compact (256 px
column), not LightBurn (no image window). **(Proposal)** items are vetoable at the demo; section 17
lists them as P1 to P27, section 18 the spec deltas E1 to E14 for the product owner. "Ink" is
`--toolbar-icon`; "muted" is `--panel-muted-fg`; "column" is the Style tab's 256 px content column;
"body column" is an unfolded step's 228 px (the column less the row's 2 x 4 padding and a 20 px
indent under the glyph).

## 0. What is reused

| Part | Reused from | Change |
|---|---|---|
| Place of the group | the Style tab (the object's own settings; stroke and fill for a shape) | an image gets the Adjustments group there |
| Preset pick | `PresetStrip` (the Dither strip of `0123`) | seven cells in two fixed rows |
| Strength slider | the Ratio slider look and `useSliderPointer` | 144 px wide in the panel, with a field |
| Step rows | `OperationRow` anatomy (glyph, name, power, armed trash, arrows), the reorder drag of `0101` 4.6 | no eye, no rename, line 2 holds the numbers |
| Add toolbar | the Operations tab's add group (`0101` 1) | step glyphs, two rows |
| Fields | the bar number field (label inside left, unit inside right) | none |
| Crop bar | the Trace bar (replaces the Select bar in place), entry chips (`EntryField`), the rectangle's handles | new bar `Crop` |
| Before and after | the canvas side pane of the font preview (`CanvasArea`, `SplitDivider`, Full view, Close) | new pane |
| Consent | the Inline confirm block (sentence, two buttons, 400 ms guard) | the Apply look in place of the destructive one |
| Clone override | the `0057` override row (two-segment pill) | a third row, Adjustments |
| Dither strip, Image row, test grid bar | `0123` sections 2 and 5 | two modes, the Picture pick and the Strength axis |
| Checks | `OutputChecks` warn lines | two lines |

## 1. Principle

The picture carries its own improvements, as a shape carries its own fill: select it, and its Style tab
shows what is done to it. The one main path is two presses: pick what kind of picture it is, set how
hard to work. Fine-tuning is one more press (Make editable) and then ordinary steps that look and move
like operation rows. What will burn is shown at the real dot size beside the original, in the canvas
area, never in a window. Nothing about the picture leaves the machine; the one download asks first, in
place. The Image operation row is untouched apart from two dither modes.

## 2. Where the adjustments live

- **Style tab of the selected image** (P1). Exactly one image object (or one image clone) selected,
  Select tool: the Style tab shows the Adjustments group (section 3), after the clone block of a clone
  (section 9). The Style tab is no longer dimmed for an image (E1). Several images selected: no
  Adjustments group (batch is out of scope; Copy and Paste adjustments carry a set from one to the
  other) (P2). An image inside a selection with paths: the group is absent, the paths' Style shows.
- **Selecting an image does not switch tabs** (P3). On a laser page the Operations tab stays when it is
  active; the Style tab is one press on the strip (`Shift+Ctrl+F`), as for any object's style.
- **Operations tab**: the Image row of `0123` as built, plus Halftone and Lines (section 10). No Preset,
  no Strength, no stack there (criterion 22).
- **Select bar**: nothing new. While Crop is edited on the canvas the **Crop bar** replaces it in place
  (section 3.9), as the Trace bar does.
- **Canvas**: Select view and Operations view as built; they show the adjusted picture (section 12).
  Before and after is a side pane of the canvas area (section 7).

## 3. The Adjustments group (drafts 01, 02)

### 3.1 Order and sizes (column 256 px, 4 px between rows, 8 px around hairlines)

| # | Part | Size | When |
|---|---|---|---|
| 1 | Header: "Adjustments" 14 px semibold; then Before and after, Copy adjustments, Paste adjustments, Restore original, 28 x 28 symbol buttons 4 px apart | 28 | always |
| 2 | State line, 12 px muted: "Not engraved yet" (no Image operation, criterion 60) or "Numbers for Image 1, 0.10 mm" (two or more Image operations) | 16 | only then |
| 3 | Preset strip | 58 | always |
| 4 | Strength: slider and field, then the end labels | 44 | Preset not Expert |
| 5 | Hairline | 9 | |
| 6 | Derived block: "From preset Photo, Strength 50 %" (12 px muted), read-only step rows, **Make editable** (256 x 28 outline text button) | 16 + 42 n + 28 | Preset not Expert |
| 7 | Hairline | 9 | Preset not Expert |
| 8 | Add toolbar | 28 or 60 | always |
| 9 | Step list (manual stack) | 42 per folded row | one or more steps |
| 10 | Hairline and the budget line "Stored results 120 MB of 512 MB" | 25 | one or more stored AI results in the document |

Copy and Paste adjustments are slice 2: before it the header holds two buttons. Restore original is
greyed (`aria-disabled`, still focusable, tooltip "Restore original") while there is nothing to restore
(Preset Expert, no step, no stored result): the header never changes its length (P4).

### 3.2 Preset strip (criterion 25)

`PresetStrip`, `role="radiogroup"` "Preset", two fixed rows (a break after Black and white), cells grow
to fill (draft 01). Arrow order is reading order (Photo to Expert), no wrap, each arrow a commit;
Home and End. Not a list that opens (P5, the `0123` Dither decision: "a list that opens reads as
restless", and the panel has no popups).

| Cell (visible) | Accessible name | Tooltip (above, 400 ms) |
|---|---|---|
| Photo | Photo (detailed) | Photo (detailed) · Dither: Jarvis |
| Graphics | Graphics (flat areas) | Graphics (flat areas) · Dither: Ordered |
| Black and white | Black and white (hard edges) | Black and white (hard edges) · Dither: Threshold 50 % |
| Line art | Line art (outlines and handwriting) | Line art (outlines and handwriting) · Dither: Threshold 50 % |
| Dots | Dot raster (round dots) | Dot raster (round dots) · Dither: Halftone |
| Newspaper | Newspaper (retro screen) | Newspaper (retro screen) · Dither: Lines |
| Expert | Expert | Expert · Dither as set in the operation |

Every cell has a tooltip here (the strip rule gives one only to shortened cells; the customer asked
for the dither in it). The one-line meanings of the spec are not shown (no tips, E2). Measured at
14 px: row 1 needs about 231 of 256 px, row 2 about 247 (verify at build; at 256 nothing truncates).

### 3.3 Strength (criterion 26), the one slider

- Row 1 (28 px): the slider, 144 x 24 px target (4 px track at ink 25 %, the filled part `--accent`,
  14 px white thumb with a 1.5 px `--accent` ring; the Ratio slider look), 8 px, the field "Strength
  50 %" 104 px. Row 2 (16 px): "Gentle" under the slider's left end, "Strong" under its right end,
  12 px muted. The ends are labels, not buttons.
- `role="slider"`, name "Strength", `aria-valuetext` "50 %"; the field is a spinbutton with the same
  value. Keys of criterion 26; pointer: press sets the value at the pointer, drag changes it (the
  Slider row's behaviour); no tooltip on the slider (the field shows the value, P6). Out of range in
  the field: refused, "Between 0 and 100 %" under the field.
- Live while dragging (reduced level, criterion 13); one drag or one key run is one step.
- Hidden with Expert, value kept (criterion 26).

### 3.4 Derived rows and Make editable (criteria 28, 29)

Read-only step rows: glyph, name in ink, line 2 the resolved numbers (12 px muted, tabular, ellipsis,
whole text as tooltip of line 2). No power, trash or arrows, not a Tab stop each: the block is one
`role="list"` "Steps from preset" in the Tab order only through Make editable. **Make editable** is a
full-width outline text button (no tooltip: a text button names itself, E3). After the press focus goes
to the first manual row.

### 3.5 Add toolbar (criterion 1)

`role="toolbar"` "Add adjustment", one Tab stop, arrows move, Enter or Space presses; 28 x 28 buttons
with the step glyphs, 4 px apart. Row 1, tone and detail in pipeline order: Channel mix, Denoise,
Levels, Curves, Local contrast, Contours, Sharpen (220 px). Row 2: Crop, Resize, 8 px, Upscale,
AI denoise, Remove background. Slice 1 has Denoise, Levels, Local contrast, Contours, Sharpen only.
Tooltip and name "Add Levels". It replaces the spec's single "Add adjustment" button that opens a list
(the panel has no popups; the Operations tab adds the same way) (P7, E4).

- While the Preset is not Expert, **row 1 is hidden** (P8): a tone step would not run (criterion 30).
  Row 2 stays: Crop, Resize and the AI steps always run.
- A press inserts the step **at its pipeline place**: after the last row whose kind comes earlier in
  the order of criterion 3 (a second step of the same kind goes after the first). The new row is
  unfolded and focus goes to its first field; one step "Edit adjustments". Adding Crop also opens the
  Crop bar (3.9).

### 3.6 Step rows (criterion 2)

| Part | Value |
|---|---|
| Line 1 (26 px) | glyph 16 px, 4, name 13 px (a button: press folds or unfolds; `aria-expanded`; ellipsis, whole name as tooltip), power 24 px (`Power` / `PowerOff`, `aria-pressed`, name and tooltip "Use Levels"), trash 24 px (armed delete), 4 px gap, up and down arrows 24 px (`ChevronUp` / `ChevronDown`, "Move Levels up") |
| Line 2 (16 px) | folded: the numbers, 11 px muted tabular, ellipsis ("Black 37 %, White 72 %, Gamma 0.88"); off: "Off · " before them and the name muted; Preset in use: "Preset in use · " and the name muted (criterion 30) |
| Body | unfolded: replaces line 2; 2 px above, 8 px below, body column 228 px; several rows may be open |
| Armed | the Inline delete confirm: row `--destructive-soft`, trash a filled `--destructive` square with a white glyph; second press removes, 400 ms guard, 4 s, Escape |
| Unknown kind | `unknown` glyph, "Unknown step", line 2 "Not run · from a newer Curvyo", trash only (no power, no arrows, no body) |
| Running AI step | line 2 (folded) "Running · 62 %" or "Downloading 21 of 64 MB" |

"Remove background" is cut to "Remove backgro..." in a 256 px row with all controls (the operation
row's rule: ellipsis and tooltip; accessible name whole).

### 3.7 Bands and reorder (criterion 3)

Three bands in this order, a 1 px ink-25 % hairline between bands that both have rows: **A** Crop,
Resize, Upscale, AI denoise; **B** Channel mix, Denoise, Levels, Curves, Local contrast, Contours,
Sharpen; **C** Remove background. A step moves only inside its band: the arrows are absent at the
band's ends (the slot stays, as in operation rows), Alt+Up and Alt+Down do nothing there. Drag: the
`0101` reorder drag (starts after 4 px on glyph or name, ghost at 70 %, the source row keeps an 8 %
ink ground, 2 px `--accent` insertion line), landing only inside the source band; outside it no line and
`not-allowed`, a release there changes nothing (no step, no notice). Each move is one step "Edit
adjustments". The hairline makes the rule visible without a word (P9).

### 3.8 Step bodies (draft 02; fields 28 px, label inside left, unit inside right, 4 px apart)

| Step | Body |
|---|---|
| Levels | graph 228 x 64 (white field ground, 1 px field border, 256 hairline bars in muted ink, baseline ink 25 %; under it three 10 px triangles: Black filled `--accent`, Gamma and White white with a 1.25 px `--accent` ring); Black 112, White 112; Gamma 112 and **Stretch** (`MoveHorizontal`, tooltip "Stretch to the picture's range") |
| Curves | graph 120 x 120 (quarter grid ink 18 %, the identity dashed muted, the curve 1.5 px ink, points 8 px squares: white with `--accent` ring, the selected one filled); beside it 104 px: In, Out (the selected point), "Point 2 of 4" 12 px muted, **Straight curve** (custom glyph, tooltip "Straight curve") |
| Local contrast | Radius 112 (mm, one decimal), Amount 112 (%) |
| Sharpen | Radius 112 (mm, two decimals), Amount 112; Threshold 112 |
| Denoise | Strength 112, Detail kept 112 |
| Contours | Line width 112 (mm), Sensitivity 112 |
| Resize | Width 96 (px), the Keep ratio toggle 28 (`Link`, the Link corners look, pressed by default), Height 96 (px; derived ink `--field-derived-fg` while the lock is on); state line "Working image 2,400 × 1,800 px" (12 px muted) |
| Channel mix | strip "Channel preset": Luminance, Red, Green, Blue (accessible names "Red filter" and so on; none pressed when the fields match none); Red 74, Green 74, Blue 72 |
| Crop | Left, Top (112 each); Width, Height (112 each), all %; **Crop on the canvas** (`Crop`, symbol, `aria-pressed` while the Crop bar is open) |
| Remove background, Upscale, AI denoise | section 8 |

Graphs: a press on a handle and a drag change the field values live; one drag is one step. Each graph is
one Tab stop (`role="group"`, its handles reachable by Left and Right, moved by Up and Down 1 % or Shift
10 %); the fields are the exact route. Curves: a click on the curve adds a point (at most 8), Delete
on a focused point removes it (reversible, no arm), the end points move only vertically. Refusals as
`0123` criterion 12: the field border `--field-invalid` and a `role="alert"` line under it, "Between 0
and 94 %" (the range as it stands, here White minus 2).

### 3.9 Crop bar and crop handles (draft 03 A)

- Opened by Crop on the canvas or by adding Crop. Replaces the Select bar in place, `role="toolbar"`
  "Crop": the Crop glyph and the word "Crop" (14 px, a name, not a control, P10), the bar number fields
  Left 96, Top 96, Width 104, Height 112 (%), then **Done** (`Check`, tooltip "Done (Enter)").
- Canvas: the whole working image is drawn, the part outside the crop washed with white at 65 % (the
  Trace preview wash); the crop frame 1 px `--accent` with the white casing; eight handles in the
  Parameter handle look. Corner drag: two sides, Shift keeps the ratio; side drag: one side; drag inside
  the frame moves it. The frame follows the object's rotation and skew. A double-click on a handle opens
  its entry chip (`EntryField`) with the fields it moves (a side one, a corner two); Enter writes,
  Escape closes. Every release or field commit is one step "Edit adjustments" (no Apply and Cancel:
  undo covers it, P11).
- Leaves on Done, Enter on the canvas, Escape (a cascade step: first a chip, then the bar), a change of
  selection or tool. While it is open the object's own handles are hidden and it cannot be moved.

## 4. A preset pick (criterion 27)

One press writes the Preset and, in every Image operation of the picture, the Dither mode and its
fields; one step "Edit adjustments". When more than one operation changed: the notice "Dither set in 2
operations." (canvas notice slot, status, 3 s; the one notice of the story: a consequence the maker
cannot see in the tab he is on, kept like "2 guides were not cloned", P12). Expert writes the Preset
only; manual tone steps run again (their "Preset in use" lines go). The Strength row appears or
disappears in the same frame, without animation.

## 5. Restore original, Make editable, Copy and Paste

- **Restore original** (criterion 59): header symbol button `RotateCcw` (the reset glyph of the value
  field), tooltip and name "Restore original"; one press, one step "Restore original", no question,
  focus stays (P13; the spec's text button and its tooltip sentence, E5). On a clone: own adjustments
  emptied (section 9).
- **Make editable** with manual tone steps present ("Preset in use"): the derived steps replace them;
  Crop, Resize and AI steps stay (the dots stay identical, criterion 29) (P14, E6). With the picture in
  two operations the notice "Numbers fixed for Image 1, 0.10 mm." (status, 3 s) (P15).
- **Copy adjustments / Paste adjustments** (slice 2, criterion 10): header symbol buttons `Copy` and
  `ClipboardPaste`. Copy holds Preset, Strength and stack in the app (not the system clipboard; kept for
  the session, P16); Paste is greyed until something was copied; one step "Paste adjustments". No
  notice after either.

## 6. States and refusals

| State | Look |
|---|---|
| Step off | `PowerOff`, name muted, "Off · " on line 2 |
| Preset in use | name muted, "Preset in use · " on line 2, power as the maker set it |
| Field out of range | `--field-invalid` border, alert line with the range, value not written |
| Unknown step, damaged fields | row of 3.6; check line (section 10) |
| Image bytes missing | the group as it stands, steps kept; the `0123` check |
| Not in an Image operation | state line "Not engraved yet"; presets still pickable (they store Preset only) |
| Stack too large for memory (criterion 56) | an alert line under the group header: "The adjustments of this picture need more memory than is free. The steps after Levels were not run." (P17) |

## 7. Before and after (criteria 11 to 13; draft 03 B, C)

- Opened by the header's **Before and after** (`Columns2`, `aria-pressed` while open). A side pane of
  the canvas area (`role="region"` "Before and after") as the font preview's: right of the drawing,
  **50 %** of the area (P18; the font preview's 35 % gives two halves too small), the `SplitDivider`,
  Full view (`Maximize2` / `Minimize2`) and Close (`X`). One side pane at a time: opening it closes the
  font preview and the other way round (P19).
- Header (28 px, wraps under 560 px as the font preview's): **Split | Side by side** (preset strip
  look, `radiogroup` "View"; default Split, P20), the operation pick (strip `radiogroup` "Operation",
  the Image operations of the picture in Operations-list order; present only with two or more), the
  subject line "photo.jpg · Image 1 · Jarvis · 0.10 mm" (12 px muted, ellipsis), the zoom readout
  "200 %" (12 px tabular), Full view, Close.
- Body on `--canvas-bg`: **Before** is the original as imported (no steps, no mask) cut to the crop, so
  both line up; **After** is the dots in pale orange `--engrave-hatch`, the same dots as the job
  (criterion 12). Split: one picture, a vertical divider (7 px hit, 1 px ink 60 %, 3 px `--editor-accent`
  on hover, drag and focus; Left and Right move it 1 %, Shift 10 %); Side by side: two halves split by a
  1 px ink-25 % line. Tags "Before" and "After" (20 px, bar ground, 1 px `--lane-line` border, 12 px)
  in the top corners. Zoom by Ctrl+wheel or pinch, pan by drag or wheel, both halves together; opens
  fitted; 100 % is one dot Interval mm at the document's 100 %.
- Follows the selection: another image shows its own; nothing or no image: "No picture selected." in
  the body (14 px muted, the font preview's empty look). Escape with focus in the pane closes it.

## 8. AI steps (slice 3; criteria 47 to 50, 63 to 66; draft 04)

| Part | Value |
|---|---|
| Body | the fields (Remove background: Edge softness 124, Threshold 100; Upscale: Factor strip 2× \| 4×, 120 px; AI denoise: Strength 112), then **Run** (the Apply look, 72 x 28 text button), then one state line |
| Not downloaded | "Model not downloaded yet (38 MB)." muted |
| Consent | first Run of a model not on the device: the Run row is replaced in place by a box (1 px field border, 8 px padding): the sentence of criterion 63 (12 px) and **Cancel** \| **Download** (equal width, 8 px apart; Download in the Apply look, 400 ms guard); focus on Cancel; Escape, Cancel, a change of selection or tab cancel. Once per model per device |
| Progress | 4 px bar (`--accent` on ink 20 %), "Downloading 21 of 64 MB" or "Running · 62 %" (12 px muted), **Cancel** outline button; in a worker, the panel stays live |
| Host not reachable | "Model host not reachable. Everything else keeps working and stored results still burn." muted; Run reads **Try again** |
| Checksum | "The downloaded model did not match its checksum and was removed." `role="alert"` in `--field-invalid`; Run reads Try again |
| Stored | "Stored result 2.1 MB" muted; **Remove stored result** (outline, beside Run, wraps to its own row when it does not fit) whenever a result exists (P21) |
| Out of date | `TriangleAlert` 16 px ink and "Out of date. The stored result still burns."; folded line 2 begins with the 12 px triangle and "Out of date · " |
| Upscale | state line "1,200 × 900 px to 2,400 × 1,800 px"; over the working limit refused before running: "4× would be 9,600 × 7,200 px, over the working limit of 16 megapixels." |
| Limits | "This result would be 70 MB; one stored result can be at most 64 MB." / "... the stored results of this document can be at most 512 MB." (alert lines); budget line from 75 %: `TriangleAlert` and the line in ink |
| Mask on the canvas | Select view: the masked-out area is the transparency checkerboard (`--checker-a`, `--checker-b`: "burns nothing"); Operations view and After: no dots there |

No notice after a download or a run (the state line changes). A run is one step "Run Remove background"
(criterion 24).

## 9. Linked clone of an image (criterion 9; draft 05 A, B)

- The clone block of `0049` as built for image roots (no Style row) gains the third override row
  **Adjustments**: 12 px muted label, the two-segment pill **Original's** | **Own** (`radiogroup`
  "Adjustments"; tooltips "Shows the original's adjustments." / "Has its own adjustments; the
  original's stay as they are."). It replaces the spec's line "Own adjustments" and the text button
  Follow original (P22, E7): one pattern for every override.
- With Original's the group below shows the original's adjustments, **live and editable**: the first
  edit makes them Own in the same step (the copy and the edit together, one undo). Pressing Original's
  is **Follow original**: one step "Follow original", no question. Restore original on a clone sets Own
  with nothing in it. Several clones with different states: no segment pressed, hidden "Mixed" (0057).
- Canvas tag "Clone, own adjustments" (and the combinations "Clone, own placement and adjustments");
  Layers tooltip line "Own adjustments" (`0057` criteria 28 and 29).

## 10. Image row, checks (draft 05 C, D)

- Dither strip of the Image row: a third fixed row **Halftone | Lines** (break after Grey); tooltips
  "Halftone dots" and "Line screen", accessible names the same; arrow order the reading order. The
  mode's fields stay the last row: Halftone: Cell size 124 (mm, two decimals) and Angle 104 (°); Lines:
  Line pitch 124 and Angle 104. Line 3: "Halftone 0.80 mm", "Lines 1.20 mm".
- The test grid bar's Dither segments keep their six modes (no Grey, no Halftone, no Lines: a grid cell
  is too small for a screen, P23).
- Checks: the two warn lines of criterion 23 with a **Select** link (selects the image, shows its Style
  tab).

## 11. Test grid (criterion 51; draft 05 E)

- Row 1 of the Image grid's bar gains **Picture**: bar segments Ramp | Selected picture (after the
  material). Selected picture is present while one picture is selected or the grid already holds one
  (P24).
- With Selected picture and a Preset on it, the axis parameters of Columns and Rows gain **Strength**;
  Min 0 %, Max 100 %, Count 5 by default, editable as the other axes (P25, E8). With Ramp or Expert the
  segment is absent (hidden, not the spec's hint "Pick a Preset on the picture first", E9).

## 12. Canvas looks

The Select view shows the adjusted picture as the Select view shows an image; the Operations view its
dots. No symbol on the object for a stack with steps off (P26, E10): the Style tab says it. While the
Crop bar is open: section 3.9. Nothing is drawn while an AI step runs.

## 13. Keyboard routes

| Do | Keys |
|---|---|
| Reach the group | `Shift+Ctrl+F` (Style tab), then Tab: header buttons, Preset strip (one stop), Strength slider, Strength field, Make editable, add toolbar (one stop), step list (one stop), each open body's fields |
| Pick a preset | arrows in the strip, each a commit; Home, End |
| Strength | Left/Down -1, Right/Up +1, Page Up/Down ±10, Home 0, End 100; or type in the field |
| Add a step | arrows in the add toolbar, Enter or Space |
| Steps | Up, Down, Home, End between rows; Enter or Space folds; Tab into the open body; Alt+Up and Alt+Down move inside the band; Delete arms, Delete or Enter removes; Space on the power toggle |
| Crop | Crop on the canvas (Space), Tab through the bar; on the canvas a double-click on a handle; Enter or Escape leaves |
| Before and after | the header button; in the pane Tab: View, Operation, Full view, Close, divider; Escape closes |

No new letter or chord (P27).

## 14. Undo steps

"Edit adjustments" (preset pick, Strength drag or key run, Make editable, add, remove, move, power, a
field or graph edit, a crop edit, the first edit on a clone), "Restore original", "Follow original",
"Paste adjustments", "Run Remove background" / "Run Upscale" / "Run AI denoise". No step for folding,
the Before and after pane, the Crop bar opening, a download or Cancel.

## 15. Accessible names

Group `role="group"` "Adjustments"; buttons "Before and after", "Copy adjustments", "Paste
adjustments", "Restore original"; strip "Preset" with the full preset names; slider "Strength"; list
`role="list"` "Adjustment steps", rows named "Levels, Black 37 %, White 72 %, Gamma 0.88" (with "off" or
"preset in use"); toolbar "Add adjustment"; pane "Before and after"; bar "Crop". A polite status region
says an armed trash's text and "Running Remove background, 62 %" at 25 % steps.

## 16. What is not added

No image window, dialog, popup or menu; no new tab and no Select-bar button; no new colour or token;
no operating tips (the spec's one-line meanings, the hint of criterion 51, the Make editable tooltip);
no success notices; no symbol on the canvas object; no automatic preset guess.

## 17. Proposals (vetoable at the demo, with defaults)

- **P1** The group lives in the Style tab of the selected image.
- **P2** Only for exactly one image (or image clone) selected.
- **P3** Selecting an image does not switch tabs.
- **P4** Restore original greyed, not hidden, when nothing is to restore (the header keeps its length).
- **P5** Preset as a fixed two-row choice strip, not a list; short cells "Dots" for Dot raster.
- **P6** No tooltip on the Strength slider; the field shows the value.
- **P7** An always-visible add toolbar of step glyphs instead of one Add button that opens a list.
- **P8** With a preset active the tone and detail add row is hidden.
- **P9** Bands split by hairlines; arrows and drag stay inside a band.
- **P10** The Crop bar starts with the glyph and the word "Crop".
- **P11** Crop edits are live steps; Done only leaves (no Apply, no Cancel).
- **P12** The notice "Dither set in 2 operations." (the story's only notice).
- **P13** Restore original as a header symbol button, no question.
- **P14** Make editable replaces manual tone steps that were "Preset in use".
- **P15** Notice "Numbers fixed for Image 1, 0.10 mm." when the picture is in two operations.
- **P16** Copy adjustments uses an in-app clipboard for the session.
- **P17** The out-of-memory line names the last step that ran.
- **P18** Before and after takes 50 % of the canvas area.
- **P19** One side pane at a time (Before and after or font preview).
- **P20** Split is the default view; Side by side is one press away.
- **P21** Remove stored result is offered whenever a result exists.
- **P22** The clone uses a third override row Adjustments (Original's | Own).
- **P23** No Halftone or Lines on the test grid's Dither segments.
- **P24** Selected picture in the grid bar only while a picture is selected or held.
- **P25** The Strength axis has Min, Max, Count like the others (default 0, 100, 5).
- **P26** No canvas symbol for steps that are off.
- **P27** No new keyboard shortcut.

## 18. Spec deltas for the product owner

- **E1** Criterion 1: the "Properties panel" is the Style tab of the image; the Style tab is no longer
  dimmed for an image selection.
- **E2** Criterion 25 and "Words": the preset is a choice strip, not a drop-down listbox; the one-line
  meanings are not shown (no tips); the tooltip is "<full name> · Dither: <mode>".
- **E3** Criteria 29 and 59: Make editable and Restore original have no tooltip sentence (a text button
  names itself; Restore original is a symbol button whose tooltip is its name).
- **E4** Criterion 1: "a symbol-only add button opening the step list" becomes the add toolbar of
  section 3.5 (no list opens).
- **E5** Criterion 59: Restore original is a header symbol button, not a text button.
- **E6** Criterion 29: say what happens to manual tone steps "Preset in use" (they are replaced).
- **E7** Criterion 9: the line "Own adjustments" and the text button Follow original become the third
  override row of `0057` (Original's | Own); pressing Original's is Follow original.
- **E8** Criterion 51: the Strength axis has Min, Max and Count (default 0, 100, 5), not five fixed values.
- **E9** Criterion 51: no hint "Pick a Preset on the picture first"; the segment is absent.
- **E10** Criterion 15: no symbol on the object.
- **E11** Criterion 11: Before and after takes 50 % of the canvas area and closes the font preview.
- **E12** Criterion 1: Restore original greyed while there is nothing to restore (criterion 59 says
  "enabled" when not empty; say greyed, not hidden, otherwise).
- **E13** Criterion 3: insertion rule "after the last row whose kind comes earlier in the pipeline";
  a second step of a kind is allowed.
- **E14** Criterion 47: Remove stored result is offered whenever a stored result exists (criterion 66
  names it only after a refusal).

## Links

Drafts `docs/design-drafts/2026-11-image-optimisation/README.md`; design system "Image adjustments
(`0135`)"; `specs/0123-raster-engraving/ux-notes.md`; `specs/0057-clone-overrides/specification.md`
(UX notes section); `specs/0077-font-preview/`.
