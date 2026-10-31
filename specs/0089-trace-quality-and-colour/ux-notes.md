# 0089 Trace quality and colour: UX notes

Status: Ready, revised 2026-10-29 against `specs/0025-raster-trace/ux-notes.md` §19 to §21 and the merged
#216 bar (`TraceBar.tsx`, `TraceAdvanced.tsx`; where this file and §21 differ on the bar's rules, §21 wins). Owner:
ux-engineer. Builds on 0025 (§2 to §17, the bar rules: pill, 36 px rows, 28 px controls, one flow wrapping by
whole items, first line never reorders) and the customer's decisions of 2026-10-29: "i" button next to the "Trace"
label that selects the About Trace tab, Preset menu, View pill, Threshold in %, Smooth, Ignore below, progress
pill in the flow after Ignore below (running with x, Stopped with Retry, Failed with Retry), Advanced expander
(double chevron) at the far right, Invert and Ignore background icon toggles first in Advanced, counts only in
Advanced, shared `ValueField` fields, no Create button (Enter creates). Source: section 6 of the customer's
document (`tracing-roadmap-de.md`). Built on the defaults of open Frage 80 to 88. Everything marked
**(Proposal)** is mine and vetoable at the demo. No hint texts: every string below names a control or states a
state; none tells the maker what to do. The "i" About Trace tab is the one granted exception (Frage 77 A) and only
gains entries (§12). Pixel widths are arithmetic from the merged bar (inner **308** at 800 x 600 with the panel
open, 616 closed; the PR measures and reports the differences).

**Customer answers of 2026-10-29 that override the text below (all decided, no open questions; where a section
below still says otherwise, this block wins and the implementer follows it):**
- Frage 84: no "Trace" label at the start of line one (the "i" stays); the progress pill is the finish button (idle
  it creates, same as Enter), at the far right, right of the expander (items 12 and 13 swap: expander, then pill;
  it is always present, so §2's "no width when empty" is void; §9 and §11 Tab order follow).
- Frage 85 B: Smooth and Tolerance (px, unit shown) are two bar fields; no readout beside Smooth (Frage 91 is
  superseded). To avoid two fields named Tolerance, the method's 0 to 100 field is named **Colour tolerance**
  **(Proposal)** in Remove background and Pick colours; Brightness keeps Threshold.
- Frage 83 B: no "Suggested"/Apply state; a new image opens in Auto and the line says what was set; the button is
  **Analyse again** (replaces Re-analyse).
- Frage 92 B: Output (Single colour | Multicolour), the Colours field and the palette pipette live in Advanced,
  not in the main bar (items 8 and 9 move; §2 row counts below are for the new flow). The palette (item 14) stays
  below the flow and appears while Multicolour is on.
- Frage 87 A, 89 A, 90 A, 86 A, 88 A: as designed. Frage 80/81/82/93 do not touch the UX.
- Row counts in §2 are arithmetic before these answers where marked "old"; the PR re-measures them.
- **Customer 2026-10-30 (feedback on #223), overrides §2 item 12, §9 and every "pill" wording of the finish action:**
  the finish button is **always blue** (`--editor-accent` fill, white 14 px text) and keeps its shape (75 x 28 px
  at rest and while running); its label is **"Trace"** (not "Create paths"); while a job runs its percentage and the
  x (Stop (Esc)) show inside the same blue button; the progress shows only after the job has run **400 ms** and then
  stays at least **300 ms**, so a short job never flashes percent and x; Stopped and Failed keep Retry inside the
  blue shape (wider by their content); Enter still creates.
- Slice C as built (2026-10-30, measured at 800 x 600, Advanced closed, no picks): the Method button is **176 px**
  (160 cut "Remove background"); Colour tolerance is the shared 168 px field in the Threshold's slot; bar heights
  with the panel open (inner 308): Brightness 252 px, Remove background 288 px, Pick colours 288 px; panel
  closed (inner 588): 108, 108, 144 px. Pick colours' detail row is reduced to one action: a click on a swatch
  selects it and shows its **Remove** (x) button after the strip; Delete removes the focused swatch.

## 1. Conflicts between the document and the customer's decisions

| Document (section 6) | Customer decision | Resolution |
|---|---|---|
| "Abschlussaktion" as 4th group of the bar | No Create button, Enter creates (2026-10-29), Frage 84 default A | No button. Enter creates; the progress slot (§2) and the notice row are all that is left at the end of the flow. |
| Profile/Automatik as first control, profiles by image type | Four job presets (0025 Frage 64 A), Frage 86 default A | Presets stay and mean the job. Analysis is its own line (§3) and sets only image-dependent values. |
| "Node-Anzeige optional" | Counts only in Advanced | Counts stay in Advanced (Frage 78 B). Nothing new in the main bar shows nodes. |
| Advanced: noise, hole and gap options, lighting correction, technical limits | Honest controls: a control the tracer cannot honour is not drawn (0025 criterion 41); Node limit and Full-size preview are gone (0025 Frage 74 A) | Only Denoise, Upscale, Corners, Invert and the border-shape toggle exist in Advanced. Gap closing and lighting correction are out of scope in the spec; no placeholders. |
| View order Original, Mask, Result, Overlay | View pill is Overlay, Trace, Image, Overlay is the default | Mask is inserted after Overlay (§4); the pill grows to four segments, which pushes View to the second line with the panel open (§2). |
| Explanations of settings ("einsehbar") | No tips (2026-10-16) | "Inspectable" means visible values and a reason line that states what was found (§3), not instructions. |
| Auto applies settings (document 5.2) | Frage 83 default A: only proposes, one click applies | Designed for A with the state "Suggested"; B only removes that state (§3). |

## 2. Bar order and widths

The merged bar is one flow of whole items (0025 §21.2: wraps by whole items, 8 px between them, 6 px inside
label, Preset and View, top-aligned, the expander at `margin-left: auto`). 0089 inserts items into that flow and
adds two rows around it. Order **(Proposal)**:

| # | Item | Width | Shown |
|---|---|---|---|
| 1 | Label "Trace" + **i** (About Trace) | 44 + 6 + 28 = 78 | always |
| 2 | **Preset** button | 132 | always |
| 3 | **View** pill, 4 segments (Overlay, Mask, Trace, Image) | 4 x 26 = 104 | always |
| 4 | **Method** menu button | 160 | always |
| 5 | **Analysis row** (§3): the line and one button; a full-width row of its own, 36 px | line flexible + 88 | always |
| 6 | **Threshold** (Brightness) or **Tolerance** (the other two): the same `ValueField`, same slot | 168 | always |
| 7 | **Colour group** of the method (§5): Background 172; Pick colours up to 288; with Brightness and Multicolour only the pipette, appended to item 9 | 172 / 288 | colour methods only |
| 8 | **Output** pill: Single colour, Multicolour | 208 | always |
| 9 | **Colours** field (+ pipette 4 + 28 with Brightness) | 120 (152) | Multicolour only |
| 10 | **Smooth** field, with the muted readout if Frage 91 is A | 168 (232) | always |
| 11 | **Ignore below** with its pixel readout | 212 | always |
| 12 | **Progress slot** (0025 §21.2: pill with x / "Stopped" + Retry / "Failed" + Retry; no width when empty) | 75 / 64 / 100 | while it has something |
| 13 | **Expander** (double chevron "More trace options"), far right of the last line | 28 | always |
| 14 | **Palette** (strip, detail row, §7), a full-width row | strip 1 to 2 rows, detail 1 row | Multicolour only |
| 15 | **Notice row** (0025 §21.3, §10) | full width, no height when empty | when there is something to say |
| 16 | **Advanced block** (§8) | as 0025 §20.5 | when open |

Rules: items 1 to 4 and the analysis row keep their order whatever the method. A method change changes only
item 7 and the label of item 6. Items 14 to 16 come last, so choosing Multicolour moves nothing above the
palette. The slot sits after Ignore below, the expander after the slot, as the customer decided; because the new
items push Ignore below down, "row 1" of 0025 §21.2 is now **the flow's last line**, not the first. The 4 px gap
between Threshold and Smooth (0025 §21.2) no longer applies, since Method-dependent items sit between them; all
gaps are 8 px. A colour group that appears or disappears with the method moves the items after it by its row;
that is the method's own parameter and the only shift (accepted; the alternative is an empty reserved row for
the default method).

**Mask and the first line.** Label group 78 + 6 + Preset 132 + 6 + View 104 = **326**. At 616 that fits with
Method (494). With the panel open (308) it does not: **View drops to line 2 and takes Method with it** (104 + 8 +
160 = 272); line 1 is the label, "i" and Preset (216). The four-segment View therefore costs one row with the
panel open. Making the segments 22 px wide would fit 322 and still not 308, so it is not done (28 px hit targets
are kept at 26 + the pill's own padding as in the merged bar).

**Row counts** after Frage 84, 85 B and 92 B (36 px rows, Advanced closed, no notice; the label is gone, so line
1 is "i" 28 + Preset 132 + View 104 + gaps = 280, which fits at 308; Method 160 is then its own line; the pill, 75,
is always present and last; Smooth and Colour tolerance/Threshold are 168 each; the new Tolerance (px) field is
168 **(Proposal)**; the PR measures them):

| Method | Panel open (308) | Panel closed (616) |
|---|---|---|
| Brightness | 8 (288 px): [i, Preset, View] [Method] [analysis] [Threshold] [Smooth] [Tolerance] [Ignore below, expander 248] [pill, right-aligned] | 4 (144 px): [i, Preset, View, Method 448] [analysis] [Threshold, Smooth, Tolerance 520] [Ignore below, expander, pill 331] |
| Remove background | 9: adds the Background group (172) as its own line | 4: [i ... Method] [analysis] [Colour tolerance, Background, Smooth 524] [Tolerance, Ignore below, expander, pill 507] |
| Pick colours | 9: swatches + pipette (up to 288) own line | 5: [i ... Method] [analysis] [Colour tolerance, Pick 464] [Smooth, Tolerance, Ignore below 564] [expander, pill] |

Multicolour adds no row to the flow (Output and Colours are in Advanced, one more control row there); the palette
adds 1 to 2 strip rows and 1 detail row below the flow. Compared with the old table, Frage 92 B saves the Output
row, Frage 85 B adds the Tolerance row, and the idle pill adds one wrapped row with the panel open (it is always
there; closed it shares Ignore below's line).

Worst case with the panel open (Pick colours, Multicolour, 16 colours, detail row, Advanced closed): 9 + 2 + 1 =
12 rows, 432 px, plus a notice (36); with Advanced open about 150 px more: **it exceeds the 560 px minimum canvas
height** and covers most of the canvas. The PR reports the measured heights; the architect may later propose a
scrolling palette. **(Proposal; flagged, not solved here.)**

## 3. Analysis line, Auto and Custom (item 5)

**Layout.** One row, 36 px: the line (12 px, `--toolbar-icon`, one line; text that does not fit ends in an ellipsis with the
full text as tooltip and accessible name, lead 2026-10-31; `role="status"`, polite, announced once per change) on the left, the **action button** (text,
28 px, 88 px) on the right. The row exists from the first frame so nothing moves when the result arrives. It is not the notice row (that one
holds no controls, 0025 §21.3); the two stay separate, the analysis row above, the notice row below the palette.

**States** (the line is the whole explanation; no tooltip repeats it):

| State | Line | Button |
|---|---|---|
| Analysing (shown after 400 ms, like the trace) | "Analysing..." | none (the slot stays empty) |
| **Auto** (applied; a new image opens here, Frage 83 B; there is no Suggested state) | "Line drawing · paper background evened out" | **Analyse again**, `aria-disabled`, dimmed: nothing was edited, the result would be the same (deterministic, spec criterion 34) |
| **Custom** (the maker changed an analysed value) | "Custom · Line drawing" | **Analyse again** |
| **Low confidence** (spec criterion 32) | "Type unclear · Brightness with automatic Threshold" (lead 2026-10-31: the analysis never sets the preset) | **Analyse again**, disabled as in Auto |
| Analysis failed (also a dying worker; the image keeps the automatic Threshold) | "Analysis failed · automatic Threshold used" (lead 2026-10-31) | **Analyse again** |

The vocabulary of the line is fixed by the analysis: image type (Graphic, Logo, Line drawing, Sketch), a
middle dot, then the findings that changed settings ("paper background evened out", "pale colours kept",
"transparent background"). Capture conditions (scan, paper photo) are findings, not types. The line names what
was found; it never says what to do.

**Semantics (Proposal).**
- *Analysed values* are Method, its parameters (Threshold or Tolerance, background colour), Denoise and
  Upscale. Preset (job), Smooth, Corners, Ignore below, Invert, border-shape toggle, Output and picked
  colours are **never** touched by analysis (picked colours and Output are the maker's decisions: spec
  criteria 25 and 33).
- *Auto* means no analysed value was edited since the analysis was applied. Editing any analysed value moves
  the line to *Custom* at once (the same stored-state rule as the Preset button's Custom: a flag, no value
  comparison). Putting the value back by hand stays Custom; only Re-analyse (or Apply) returns to Auto.
- Lead 2026-10-31: any manual setting change (not only an analysed value) after the image was chosen and before the answer arrives drops the answer for that image; the line reads Custom and Analyse again applies it on request. The line is one line in the 36 px row; when it does not fit (800 x 600 with the panel open) it ends in an ellipsis with the whole text as its tooltip, and the live region reads it whole. Analyse again is 88 x 28 px.
- Analysis runs on a new image and on Re-analyse, nothing else. A late answer after a manual edit is
  dropped (the edit wins); during manual work no value changes by itself.
- Re-analyse replaces only the analysed values, in one go, with no confirmation: they are tool settings, not
  document data, the maker saw them change, and one press recovers the proposal. It is not a history step.
- **Apply** (Suggested) does the same for the first suggestion. The preview shows the current settings, not
  the suggestion, until Apply. With Frage 83 B the state Suggested does not exist: a new image opens in Auto.
- Several images selected: the analysis belongs to the previewed image; the settings apply to all (label
  "Tracing 1 of 3 images" says so). The line is not repeated per image.
- The Preset button keeps meaning the job. Picking a preset does not change the line; it sets only the preset
  values of 0025 criterion 60, and the analysed values stay.

## 4. View pill with Mask (item 3)

- Four 26 x 28 px segments (the merged pill is 3 x 26 = 78; 104 with Mask; width consequence in §2), `role="radiogroup"` "View", arrows move and select, one Tab stop (0025 §13):
  **Overlay, Mask, Trace, Image**. Mask glyph: Lucide `Binary` (16 px, 1.5 px; two states, black and white). Not `Contrast`:
  the Invert toggle in Advanced already uses it (0025 §19) and the two are never told apart by glyph alone. Tooltip names the segment ("Mask"); the Image tooltip keeps its key ("Image (\)").
- **Mask** draws exactly the binary image handed to the tracer (spec criterion 21, pixel-equal): content
  `#000`, the rest `#FFF`, at the image's place and size, at 100 %, no wash, no outline, nearest-neighbour
  when zoomed in (pixels stay pixels). Fully transparent source pixels are shown as not content (white), as
  the mask says. On a dark document ground the 1 px `--selection-casing` edge bounds the rectangle, as the
  Trace view does for black shapes. **(Proposal)**
- With Multicolour the Mask shows the union of all foreground (before the palette splits it); Excluded
  colours are white in it. Mask is the answer to "was my content captured", the Trace view answers "what do
  the shapes look like".
- Peek: `\` still shows Image while held. Shift+`\` cycles the four in pill order. No key for Mask alone (a
  second held key would be a tip to explain). The chosen view is still not remembered; entering starts on
  Overlay.
- Over 4 Mpx the Mask is the reduced copy's mask, with the existing muted line "Preview from a reduced copy".
- A stale mask is shown at 50 % with the existing "Preview is out of date" words.

## 5. Method and its context fields (items 4, 6, 7)

**Method button.** The Preset button's look and menu mechanics (0025 §11): text button 28 px, 160 px,
chevron because it names a menu, `aria-haspopup="menu"`, accessible name "Method"; menu 232 px with three
`menuitemradio` rows, **Brightness, Remove background, Pick colours**, each with the 0025 second line in the
muted style (describing the result: "Dark and light pixels by a cut-off.", "Everything unlike the
background.", "Pixels like the colours you picked."). Keys as the Preset menu. A pick is not a history step,
changes the method's parameters only, never Smooth, Ignore below, Output or the palette, and marks the
analysed values as edited (Custom).

**Threshold / Tolerance (item 6).** The shared `ValueField` (0025 §20.1), 168 px, same slot and size for all methods:
label "Threshold" (0 to 100 %, §20.2 incl. the automatic start) for Brightness, "Tolerance" (0 to 100 %, whole
numbers) for Remove background and Pick colours; behaviour exactly as the app's value field: click sets, drag, double-click, Enter or F2 opens the entry, arrows 1, Shift 10, Home/End, Ctrl+Enter creates, no reset slot,
the existing slider tooltip. `aria-valuetext` carries "percent". Each method remembers its
own value (app setting). The tolerance is one value for all swatches (per-swatch tolerance is not offered).
**(Proposal)** Pick colours starts at **19** (2026-10-30): with the noise filter (3 x 3 median and, in flat areas, 5 x 5
mean) it selects 99.2 % of a picked shape with +-6 noise (the former 15 only 12 %). Above 19 a picked pale grey
pulls in anti-aliased rims; lower the tolerance for near-grey artwork. Remove background starts at 40.

**Brightness.** No colour group. Invert and the border-shape toggle sit in Advanced as today.

**Remove background (item 7, 172 px).** `[chip 28] [#RRGGBB 56, tabular 12 px] [Auto 44] [pipette 28]`.
- The chip is display only: the background colour over the checkerboard if it has alpha, `--swatch-border`.
  The hex is always printed (colour is never the only cue).
- **Auto** is a text toggle (`aria-pressed`): pressed = the colour is detected from the image border (spec
  criterion 16), the chip and hex show the detected value. A pipette pick fixes the colour and releases Auto;
  pressing Auto again returns to detection. Auto is pressed on entering the method.
- A transparent border detects "transparent": chip shows the checkerboard, the hex reads "none", Tolerance is
  disabled (`aria-disabled`, value kept) because transparency needs none.

**Pick colours (item 7, up to 288 px).** The swatch strip of §7 in a reduced form plus the pipette:
`[swatch x n, 28 px each, 4 apart, up to 8] [pipette 28]`. Empty: no swatches, the pipette is armed on
entering the method and the status says "No colour picked." Swatches and strip behave exactly as the palette
(§7: selection, hex and name, roving focus), with one action in the detail row: **Remove**. The ninth pick
replaces nothing; "8 colours picked" shows in the status from the eighth swatch on (permit over refuse: the
pipette stays armed, the click does nothing else). **(Proposal)**

## 6. Eyedropper on the canvas

Reuses the Style eyedropper (design system, "Eyedropper button", colour chip, cursor):
- **Button** 28 x 28, `Pipette` 16 px, `aria-pressed` while armed, name "Pick colour from the image"
  (Remove background: "Pick background colour from the image"), tooltip text "Pick a colour from the
  image (Esc cancels)", the same words the Style button uses. Armed: `--toolbar-icon-active-bg`.
- **Armed** means a canvas click samples. Cursor: the 24 px eyedropper, no object hover. Pan, zoom and the
  wheel work. The **colour chip** follows the pointer (16 px swatch, `#RRGGBBAA`); outside the image (pasteboard,
  other objects) it reads "No colour here" and a click does nothing.
- The sample is taken from the **working image** (what the tracer sees: the stored pixels, not the screen
  with wash, overlay or Mask), the median of the 3 x 3 pixels around the pointer so a click on an
  anti-aliased edge does not pick a blend. **(Proposal; the architect confirms the sampling rule.)** It works
  in every View, including Mask, where the chip is the only colour information.
- **Click** in Remove background: sets the background colour, releases Auto, disarms. In Pick colours: replaces
  the picked set with this one swatch, stays armed. **Shift+click** adds a swatch, stays armed. In Multicolour
  with Brightness the pick adds a fixed palette colour the same way (§7).
- Armed ends by Esc, the button, choosing another method, or leaving the tool. Esc order: a running trace
  stops first (0025 §21.2), then the armed pipette disarms, then the tool is left. Focus stays where it was;
  the pipette is a mouse tool and has no key (the button is reachable by Tab, Space toggles it).
- A pick is one settings change: the preview re-traces after the 150 ms debounce, not a history step.

## 7. Output switch and palette (items 8, 9, 14)

**Output pill** (item 8, **in the Advanced block** per Frage 92 B, first row after the Invert and Drop border
shapes toggles; the Colours field and the pipette follow it there): `role="radiogroup"` "Output", two text segments 104 px, **Single colour** and
**Multicolour**; arrows move and select, one Tab stop. Changed only by the maker (never by analysis).
Remembered as an app setting. With **Multicolour** the **Colours** field (item 9, 120 px, the shared `ValueField`) follows as its own item:
label inside, value right; range 2 to 16, whole numbers, the lowest stop reads "Auto" (position, not a value); the value is `Auto` with the resolved count muted ("Auto  6"), or a number 2 to
16. Typing a number fixes it; Home, or Enter on an empty entry, returns to Auto. In
Multicolour with Pick colours the palette is exactly the picked swatches (Frage 90), the field shows the
count and is `aria-disabled`.

**Palette** (item 14), below the flow and above the notice row, `role="listbox"` "Palette", `aria-orientation="horizontal"`:
- **Strip.** Swatches 28 x 28 hit area, 20 px colour (alpha over the checkerboard), `--swatch-border`, 4 px
  apart, up to 9 per row at 308 px, 2 rows maximum (16 colours), one row at 616 px. Order: area descending
  (the stable order of spec criterion 29, ties by colour value). Picked (fixed) colours carry a 6 px square
  corner mark and sit first. One Tab stop, arrows move (roving tabindex), the focused swatch is the
  selected one: 2 px `--editor-accent` ring with a 1 px casing.
- **Detail row** for the selected swatch (36 px, appears below the strip when a swatch is selected; Esc or a
  click outside the strip clears the selection and the row goes): "#FFF2A8 · Pale yellow" (12 px tabular,
  colour name from a fixed hue/lightness table, the accessible name of the swatch is the same text plus the
  state word) and five 28 px icon buttons, each with a tooltip that names the action and `aria-pressed` where
  it toggles:
  1. **Isolate** (`Eye`): the others are drawn dimmed, not hidden (35 % wash in Overlay; 35 % opacity in Trace
     and Mask). View only; Enter still creates every colour. One swatch at a time; pressing another
     swatch's Isolate moves it. The pressed state and the status "Isolating #FFF2A8" tell the truth.
  2. **Recolour** (`Paintbrush`): opens the inline colour picker of the Style section under the detail row
     (hex field plus area and hue; 244 px fits); changes the output fill only, no refit (spec criterion 29).
     The swatch shows the new colour, the detail row "#FFE000 (was #FFF2A8)", a text button **Reset** sits in
     the picker's hex row. Esc closes the picker, keeps the colour.
  3. **Merge into** (`Merge`): pressed = choosing mode; the status reads "Merging #FFF2A8 into...", the
     other swatches get a dashed `--editor-accent` ring, a click or Enter on one is the target, Esc cancels.
     The merged swatch stays in the strip at 50 % with the target's colour as a 6 px dot at its lower right and
     the detail row "Merged into #3366CC". Its pixels are reassigned to the target and traced together as one
     compound path. Pressing Merge into again on a merged swatch undoes it.
  4. **Exclude** (`Ban`): the colour becomes background (not traced, its area is not given to a neighbour).
     The swatch stays at 40 % with the `Ban` glyph (not a hue change), the detail row says "Excluded".
     Pressing again restores it. This is the only one of the four that deletes shapes from the result; the
     result row says so ("2 colours excluded") and nothing else (document: background versus reassign is
     decided as Exclude = background, Merge into = reassign, matching spec criterion 29).
  5. **Remove** (`X`): only for picked colours; takes the colour out of the palette (the automatic count then
     fills the space). No confirmation (tool state, re-pickable in one click); an automatic colour has no
     Remove (its count belongs to the Colours field).
- **Pipette** for the palette: the pipette of §6 follows the Colours field (item 9) with Brightness + Multicolour,
  and is in the colour group (item 7) for Pick colours; same button state. Remove background + Multicolour has
  only the background pipette: fixed palette colours exist for Pick colours and Brightness only.
  A pick adds a fixed colour; a colour within the merge distance of an existing one selects that swatch and
  says "Same as #3366CC" instead of adding a twin.
- No colour perception needed: every swatch is a button with hex and name as its accessible name, the state
  words (picked, excluded, merged into, isolated) are text in the name and the detail row, the marks
  (corner square, `Ban`, dot) are shapes. Hit targets 28 px. Contrast of mark and ring on both themes is
  checked in the review.
- Empty and edge states: no colour found: strip is empty and the status says "No colours found."; one
  colour: one swatch, Merge into is `aria-disabled` (nothing to merge into); more than 16 asked: the field
  clamps and shows 16.

## 8. Geometry controls and Advanced (items 10, 11, 16)

**Smooth and Tolerance** (Frage 85 B, decided): two `ValueField`s in the bar. **Smooth** (0 to 100, calmness)
drives input smoothing only; **Tolerance** is the deviation tolerance in source pixels with its unit ("px") shown,
right of Smooth in the flow. Neither moves a protected corner or rounds one (spec criterion 8); changing Tolerance
leaves protected corners where they are. Old stored Smooth values migrate by the architect's rule (a Logo / Line
art trace looks the same or better). There is no readout beside Smooth (Frage 91 superseded).
**Corners** stays in Advanced, renamed in its tooltip only ("Corner protection"; the visible label stays
"Corners"): 0 to 100, higher marks more bends as corners, 0 none. There is no node budget and no node
display outside Advanced.

**Advanced block** is the merged one (0025 §20.5), unchanged in structure: row 1 the icon toggles **Invert**
(`Contrast`) and **Ignore background** (`ImageMinus`) at the very front, then Denoise (segmented Off | Low |
High); Upscale (segmented); Corners (`ValueField`); the counts row last and alone. 0089 changes only: the second
toggle's name and tooltip become **Drop border shapes** (Frage 88 A; the glyph stays; the name "Ignore background"
would be read as the Remove background method); in Remove background "Invert" keeps the background; the Corners
tooltip reads "Corner protection" (the visible label stays "Corners"). Rows at 308 as in §20.5: 4 rows, 144 px;
at 616: 3. There is no Node limit and no Full-size preview. In Pick colours and Remove background Denoise is
available as in Brightness; analysis may set it. The expander's accent dot and "Changed: ..." line list the
Advanced values that differ from their defaults, with the new toggle name.

## 9. Computing, stale and failure

Unchanged from 0025 §21.2 and §21.3: Running, Stopped + Retry and Failed + Retry live in the progress slot (item 12),
the timing is 400 ms, the failure card is gone, and an analysis failure is the line in §3, not a state of the slot.
The palette strip and the analysis line keep their last value while a job runs; the stale preview is 50 % in
Overlay, Trace and Mask. A palette that is being recomputed shows the strip at 50 %; there is no "Updating..."
text (the slot's pill already says a job runs). Multicolour over the 4 s budget is not UX: the slot's x and
Retry apply.

## 10. Notice row

Result messages only (0025 §21.3: muted, `role="status"`, no height when empty, no controls). The transient states
"Tracing...", "Stopped", "Preview is out of date" and "Failed" are **not** here; they are the slot. The row holds:
"Tracing 1 of 3 images" (as merged), "Preview from a reduced copy", "0 paths: no colour picked.", "0 paths at this
Tolerance: too low." (or too high, as the 0025 zero-path rule), "No colours found.", "Isolating #FFF2A8", "Merging
#FFF2A8 into...", "2 colours excluded", "8 colours picked", "Same as #3366CC", "Over 100,000 nodes" as before. The
counts ("7 paths, 412 nodes", "3 colours") stay in Advanced only. The success notice after Enter: "Traced: 7
paths, 412 nodes." (Single) or "Traced: 3 colours, 7 paths, 412 nodes." (Multicolour; the result is one group
"Trace of <name>", one compound path per colour).

## 11. Keyboard and focus

Tab order follows the visual order (0025 §21.2): Select bar Trace, **i** About Trace, Preset, View (one stop),
Method, analysis button (when enabled), Threshold/Tolerance, colour group (Auto toggle, pipette; the strip is one
stop with arrows, Delete removes in Pick colours), Output (one stop), Colours (and its pipette), Smooth, Ignore
below, the slot's control (x, or Retry), the expander, then the palette strip (one stop), its detail buttons, and
the Advanced controls when open (Invert, Drop border shapes, Denoise, Upscale, Corners). There is no Create
button: **Enter** with focus on the canvas creates, Ctrl+Enter in a field or entry, a double-click on the image
too; Enter on a focused bar button presses that button (0025 §19), as everywhere. Enter on a swatch or on the
analysis button belongs to that control (Enter on Apply applies; it does not also create). Esc order: a running
trace stops first (slot x), then an armed pipette disarms, then the tool is left. Fields are `ValueField`s with
the keys of 0025 §20.1. Focus ring 2 px `--editor-accent`; a mouse press on a bar button returns focus to the
canvas except in fields, the strip and the picker. Every state change not visible on the canvas is text in the
analysis line or the notice row, announced once. Nothing animates; the slot's fill steps with the number.

## 12. Strings (all to be in code exactly)

Labels: Trace, About Trace, Preset, View (Overlay, Mask, Trace, Image), Method (Brightness, Remove background, Pick
colours), Threshold, Tolerance, Auto, Output (Single colour, Multicolour), Colours, Smooth, Ignore below, More
trace options / Fewer trace options, Invert, Drop border shapes, Corners, Denoise, Upscale; palette actions
Isolate, Recolour, Reset, Merge into, Exclude, Remove; analysis buttons Apply, Re-analyse; the slot's Stop (Esc),
Stopped, Failed, Retry as merged. The word "Advanced" is not drawn (the block's `aria-label` keeps it). Lines and
states: §3, §10. No other explanatory text.

**About Trace tab (the only place that explains; must be extended in the same pass, spec criterion 24).** The page
(0025 §20.3 and §21.1) lists every control in the bar's order, so it needs entries, in the same plain
definition style, for: Method (and one line per method), Tolerance, Background and Auto, the pipette and Pick
colours, Output and Colours, the palette and its five actions, Mask in the View entry, the analysis line with
Apply and Re-analyse, and the renamed Drop border shapes; the Smooth, Preset and Threshold entries are
re-checked against the new meaning (Smooth now drives deviation tolerance too, 0.6 px readout if Frage 91 A).
No other help text is added. The PO's criterion 24 and the implementer's string list carry these entries; the
text is a Proposal for the customer to read.

## 13. Questions for the customer

None open. Decided 2026-10-29 (see the spec): Frage 89 A (pipette stays armed in Pick colours and Multicolour,
Shift+click adds, Esc ends; ends after one pick in Remove background), Frage 90 A (Multicolour with Pick colours:
the palette is exactly the picks, Colours fixed and greyed out), Frage 91 superseded by Frage 85 B (Tolerance in px
is its own bar field), Frage 92 B (Output in Advanced).

## 22. Tidy-up of 2026-10-31 (customer feedback on the merged Trace bar; overrides §2 items 5, 7, 14, 15, §3 layout, §7 placement of the palette, §10, §11 and §12 where they differ)

Customer: the function is good, the bar is fiddly. **Principle: the bar holds what a straightforward trace needs
(choose, set the one main value, trace); everything that explains, analyses, edits or reports sits in the
expandable Advanced block.** Every item marked **(Proposal)** is mine and vetoable at the demo. Widths are
arithmetic from the merged bar (inner 308 at 800 x 600 with the panel open, 588 closed; 36 px rows); the PR
re-measures. No hint texts: every string below names a control or states a state.

Note for the implementer: the analysis row component is not in the checkout this was written against
(`story/text-recognition`); build from §3 and `TraceAdvanced.tsx`, and apply this section to wherever it lives.

### 22.1 The new main bar

One flow of whole items, 8 px apart, wrapping as before. Order:

| # | Item | Width | Shown |
|---|---|---|---|
| 1 | **i** About Trace | 28 | always |
| 2 | **Preset** | 132 | always |
| 3 | **View** pill (4 segments) | 104 | always |
| 4 | **Method** menu button | 176 | always |
| 5 | The method's **one main field**: Threshold (Brightness) or Colour tolerance (Remove background, Pick colours) | 168 | always |
| 6 | **Method group**, only where picking is the method's own job: Remove background keeps chip, hex, Auto, pipette (unchanged, 172); **Pick colours: the pipette plus a muted count label "3 picked" / "None picked" (Proposal)**, about 96. No swatches, no Remove button | 172 / 96 | those two methods |
| 7 | **Smooth** | 168 | always |
| 8 | **Tolerance** (px) | 168 | always |
| 9 | **Ignore below** with its "~ 2 px" readout | 212 | always |
| 10 | **Expander**, then the blue **Trace** button, both at the far right (`margin-left: auto`) | 28 + 8 + 75 | always |

Gone from the bar: the analysis row (item 5 of §2), the swatch row of Pick colours, the palette strip and its detail
row, the whole result line. Brightness + Multicolour adds nothing to the bar (its pipette is in Advanced, 22.3).
The count label is a state label of the pipette ("3 picked" is the count of picks, the state in which Trace can run;
"None picked" is the state in which it cannot), 12 px `--toolbar-icon` at 70 %, not a Tab stop; it replaces the
notice "0 paths: no colour picked." and "8 colours picked" in the bar.

**Rows at 800 x 600** (Advanced closed, no notice):

| Method | Panel open (inner 308) | Panel closed (inner 588) |
|---|---|---|
| Brightness | 7 rows, 252 px: [i, Preset, View] [Method] [Threshold] [Smooth] [Tolerance] [Ignore below] [expander, Trace] | 3 rows, 108 px: [i .. Method 464] [Threshold, Smooth, Tolerance 520] [Ignore below, expander, Trace 331] |
| Remove background | 8 rows, 288 px: as above, the group (172) on its own line after Colour tolerance (168 + 8 + 172 = 348 does not fit) | 3 rows, 108 px |
| Pick colours | **7 rows, 252 px** (was 8): Colour tolerance and the pipette group share a line (168 + 8 + 96 = 272 fits) | **3 rows, 108 px** (was 4) |

The analysis row no longer costs a bar row, so the bar is back to the height slice C measured for Brightness and
one row shorter for Pick colours.

### 22.2 Messages: where each goes

Rule **(Proposal)**: *the bar keeps a notice row only for what the maker must see with Advanced closed and fix with
a bar control, or what changes how the preview must be read. Every other line lives in the Advanced status line.
A result is stated once, in one place.*

| Message | Home | Why |
|---|---|---|
| Failed / Stopped, Loading, Tracing | the Trace button (as built) | unchanged |
| "Tracing 1 of 3 images", "Preview from a reduced copy", "Over 100,000 nodes. Slow to edit." | bar notice row (unchanged; no height when empty) | change how the preview reads or how Trace behaves |
| "0 paths at this Threshold/Colour tolerance: too low/high.", "1 path covering the whole image.", "Too much detail: raise Ignore below." | bar notice row | the fix is a bar control; Trace is disabled for 0 paths and too much detail, while "1 path covering the whole image" can still be created (0025 c15 wins, lead 2026-10-31). Counts are not also printed in Advanced then (the counts line is hidden for an empty or unusable result, so "0 paths" appears once) |
| "0 paths: no colour picked." | **Advanced status line only**; in the bar the pipette label reads "None picked" and the Trace button is `aria-disabled` | customer asked for it to move; the state stays visible where the fix is |
| Result counts "7 paths, 412 nodes, 0.4 s" ("3 colours, ..."), muted "38 specks ignored", "2 regions joined" | **Advanced status line only** (one line, was both notice row and counts line). The success text and the time move into it | removes the double count |
| "No colours found.", "Isolating #FFF2A8", "Merging #FFF2A8 into...", "2 colours excluded" | Advanced status line (they come from Advanced controls) | |
| "Same as #3366CC" (a pick matching an existing colour) | bar notice row, 3 s, then gone | caused by the bar's pipette |
| Analysis line ("Line drawing · pale colours kept", "Custom · ...", "Analysing...", "Analysis failed ...") | Advanced, first row | customer item 1 |

**A disabled Trace button explains itself**: `aria-disabled` with the reason as its tooltip and `aria-describedby`
(the same text as the status line). With Advanced closed, a **visually hidden `role="status"` region in the bar**
announces the Advanced status line and the analysis line once per change, so a screen-reader user loses nothing.
Errors in the Trace button ("Failed", alert) and in the analysis line ("Analysis failed") are unchanged; the
latter is only seen when Advanced is open, which is acceptable because Logo / Line art settings are used and the
trace still runs.

### 22.3 The Advanced block, top to bottom

Divider as built, then rows (36 px unless noted), each present only when it has content:

1. **Analysis row**: the line (12 px, up to two lines, never truncated, polite status) left; the blue
   **Analyse** button right: text button 28 px high, 75 px wide, `--editor-accent` fill, white 14 px text, the
   Trace button's look and focus ring (not a second, dimmer style). It is **always enabled** (permit over
   refuse; in Auto it recomputes the same result, which is deterministic and harmless), so the dimmed
   "Analyse again" state is gone. Pressing it replaces the analysed values as in §3, not a history step; its
   mouse press returns focus to the canvas. States of the line as §3, button unchanged in all of them.
2. **Status line** (`role="status"`, polite, tabular): the result counts, the muted second line, and the
   palette states of 22.2; 50 % while stale; no height when empty. Replaces the old counts row at the bottom.
3. **Output row**: Output pill; with Multicolour the Colours field (120) and, with Brightness, the palette
   pipette (Pick colours has its pipette in the bar, so no second one here; one pipette per method, **Proposal**).
4. **Colours strip and its detail row**, the one and only place swatches are shown (Multicolour: the palette
   strip of §7; Pick colours in either Output: the picked colours as the same strip, so there is no second
   swatch row): swatches, the detail row for the selected one with Isolate, Recolour, Merge into, Exclude, Remove
   as §7 (Pick colours with Single colour offers Remove only). Esc clears the selection. Empty strip: no row.
5. **Options**: the Invert and Drop border shapes toggles first (60 px, as 0025 §20.5), then Denoise,
   Upscale, Corners (168 each). Unchanged in content and order.

**Rows at 800 x 600:**

| Advanced, added | Panel open (308) | Panel closed (588) |
|---|---|---|
| Single colour, Brightness | 6 rows, about 222 px: analysis, status, Output, [toggles + Denoise], Upscale, Corners | 4 rows, about 150 px: analysis, status, Output, options (60 + 3 x 168 + 24 = 588, exactly full; if it does not fit in the build, Corners wraps and this is 5) |
| Multicolour, Brightness, 6 colours | 9 rows, about 330 px (Colours + pipette 252 get their own line; strip 1, detail 1) | 6 rows, about 222 px (Output + Colours + pipette 368 on one line) |
| Multicolour, 16 colours | one row more in the strip | same |

Worst case with the panel open (Brightness, Multicolour, 16 colours, detail row, Advanced open): bar 252 + block
about 366 = 618 px, more than the 560 px canvas height at 800 x 600. This was already flagged in §2 and is not
made worse (the old block had the same rows plus the analysis row and a bar swatch row). **(Proposal, optional,
the implementer's call):** limit the Advanced block to `min(60vh, 360px)` with its own vertical scroll
(`overflow-y: auto`, the divider and the first rows stay reachable by Tab, which scrolls the focus into view).

### 22.4 Keyboard order

Tab: Select-bar Trace button (when it is how the tool was entered), **i**, Preset, View (one stop), Method, the
main field, the method group (Auto, pipette), Smooth, Tolerance, Ignore below, expander, **Trace**, then, when
Advanced is open: **Analyse**, Output (one stop), Colours, the palette pipette (Brightness), the strip (one
stop, arrows move, Delete removes a picked colour), its detail buttons, Invert, Drop border shapes, Denoise,
Upscale, Corners. Enter on Analyse or on a swatch belongs to that control and does not create; Ctrl+Enter in any
field creates; Escape in an Advanced control returns focus to the canvas, as built. Esc order unchanged (stop a
running trace, disarm the pipette, leave the tool). The expander still moves nothing above it. Focus never moves
by itself when Advanced opens or closes; closing it with focus inside puts focus on the expander.

### 22.5 Strings

New or changed: button **Analyse** (was "Analyse again"; the earlier "Re-analyse" and "Apply" are gone);
pipette labels "3 picked" (count, singular "1 picked") and "None picked"; the Trace button's tooltip carries the
reason when `aria-disabled`. Unchanged names: Output, Colours, Palette, Picked colours. "Analyse again" and the
notice "8 colours picked" no longer exist in code, About Trace or docs. About Trace entries are reordered to the
new order (bar items first, then the Advanced block: Analyse and the analysis line, status line, Output, Colours,
palette actions, Invert, Drop border shapes, Denoise, Upscale, Corners); the Pick colours entry says the
picked colours are shown and edited in Advanced. A test compares the strings with `docs/tracing-images.md`.

### 22.6 Changes to the spec and the docs

- `0089` criterion **22** (bar order): the order is now Preset and View, Method with its one main field and
  pipette group, Smooth, Tolerance, Ignore below, expander, Trace; nothing about an analysis row or swatches in
  the bar. **22a** unchanged.
- Criterion **17**: the pipette stays in the bar; each swatch can be removed in the Advanced strip (Delete on the
  focused swatch, the Remove action), not in the bar.
- Criterion **25**: unchanged (Output in Advanced); the palette pipette is Advanced for Brightness, in the bar for
  Pick colours.
- Criterion **29**: the palette is usable without colour perception and by keyboard **inside Advanced**; it is
  not visible with Advanced closed.
- Criterion **31**: the analysis line is shown "in the Advanced block", not "in the bar".
- Criterion **33**: the button is **Analyse** (not "Analyse again"), always enabled, in Advanced; the rest
  (no re-run after a manual change, never switches Output) stands.
- `0089` UX notes: §2 item 5 and items 7 and 14 (analysis row, colour group swatches, palette), §3 layout and
  the disabled-button rows, §7 placement ("below the flow" becomes inside Advanced), §10 list of notices,
  §11 Tab order, §12 strings, row counts in §2.
- `0025` criteria **11, 46, 71** (counts and notice lines: result counts only in Advanced), **44** (unchanged
  content), **57** (Tab order, 22.4); UX notes §21.3 (the notice row is narrowed by 22.2).
- `docs/design-system.md`: rows "Trace bar", "Trace Output and palette" (palette placement, strip rule),
  "Trace notices", "Trace Advanced button and block" (order of 22.3), and the "Trace info tab" row (entry
  order). Add one sentence to the Trace bar row: the bar holds the straightforward path, Advanced everything else.
  No new tokens.

### 22.7 Questions

None open; every choice above has the stated default and can be vetoed at the demo. The one I would look at first
is the "3 picked" / "None picked" label (22.1): without it a maker who has not opened Advanced cannot see that
a pick landed; veto it and the bar shows the canvas result only.

**Lead decisions after the tester (2026-10-31, Proposal):** a duplicate pick in Pick colours is ignored without a notice; Output may share its line with Invert and Drop border shapes when it fits.
