# 0089 Trace quality and colour: UX notes

Status: Draft for Ready. Owner: ux-engineer. Builds on `specs/0025-raster-trace/ux-notes.md` (§2 to §17, the
bar rules: pill, 36 px rows, 28 px controls, whole-group wrap, first row never moves, grows only downward) and
the customer's decisions of 2026-10-29 for the current bar (i button with About tab, Preset menu, View pill,
Threshold %, Smooth, Ignore below, progress pill in row 1, Advanced expander at the far right with the Invert
and Ignore background icon toggles first, no Create button: Enter creates, counts only in Advanced). Source:
section 6 of the customer's document (`tracing-roadmap-de.md`). Built on the defaults of open Frage 80 to 88.
Everything marked **(Proposal)** is mine and vetoable at the demo. No hint texts: every string below names a
control or states a state; none tells the maker what to do. Pixel widths are arithmetic from the 0025 rows
(inner width 344 at 800 x 600 panel open, 616 closed); the PR measures and reports the differences.

## 1. Conflicts between the document and the customer's decisions

| Document (section 6) | Customer decision | Resolution |
|---|---|---|
| "Abschlussaktion" as 4th group of the bar | No Create button, Enter creates (2026-10-29), Frage 84 default A | No button. Enter creates; the progress pill and status text are the only things in the last position. |
| Profile/Automatik as first control, profiles by image type | Four job presets (0025 Frage 64 A), Frage 86 default A | Presets stay and mean the job. Analysis is its own line (§3) and sets only image-dependent values. |
| "Node-Anzeige optional" | Counts only in Advanced | Counts stay in Advanced. Nothing new in the main bar shows nodes. |
| Advanced: noise, hole and gap options, lighting correction, technical limits | Honest controls: a control the tracer cannot honour is not drawn (0025 criterion 41) | Only Denoise, Upscale, Corners, Full-size preview, Invert and the border-shape toggle exist. Gap closing and lighting correction are out of scope in the spec; no placeholders. |
| View order Original, Mask, Result, Overlay | View pill is Overlay, Trace, Image, Overlay is the default | Mask is inserted after Overlay (§4). |
| Explanations of settings ("einsehbar") | No tips (2026-10-16) | "Inspectable" means visible values and a reason line that states what was found (§3), not instructions. |
| Auto applies settings (document 5.2) | Frage 83 default A: only proposes, one click applies | Designed for A with the state "Suggested"; B only removes that state (§3). |

## 2. Bar order and widths

One flow of whole groups, same pill, same wrap rule. Stable order **(Proposal, from spec criterion 22 plus M)**:

| # | Group | Width | Shown |
|---|---|---|---|
| 1 | State label "Trace", **i** button, **Preset** button, **View** pill (4 segments) | 48 + 28 + 132 + 100 + 24 = 332 | always |
| 2 | **Analysis line** and its one button (§3) | line flexible, button 88 | always |
| 3 | **Method** menu button | 160 | always |
| 4 | **Threshold** (Brightness) or **Tolerance** (the other two): the same 168 px field, same position | 168 | always |
| 5 | **Colour group** of the method (§5) | Background 172; Pick colours up to 8 x 28 + gaps + pipette = 288 | colour methods only |
| 6 | **Output** pill: Single colour, Multicolour; with Multicolour the **Colours** field follows | 208; + 8 + 120 | always |
| 7 | **Smooth** field with the muted readout | 168 + 8 + 56 | always |
| 8 | **Ignore below** with its pixel readout, **Advanced** button | 212 + 8 + 96 = 316 | always |
| 9 | **Palette** (strip, detail row, §7) | strip 1 to 2 rows, detail 1 row | Multicolour only |
| 10 | Status line (state words, §10) | flexible | when there is something to say |
| 11 | **Advanced block** (§8) | as 0025 §12 | when open |

Rules: rows 1 to 4 never move. A method change changes only group 5 (the method's own parameters) and the
label of group 4. Groups 9 and 11 appear below everything else, so choosing Multicolour moves nothing above
the palette. A colour group that appears or disappears with the method moves groups 6 to 8 by its row; that is
the method's own parameter and the only shift (accepted; the alternative is an empty reserved row for the
default method). **(Proposal)**

Row arithmetic at 800 x 600, **panel open (inner 344)**: R1 group 1 (332; the progress pill keeps its place
at the row end; it does not fit beside group 1 with 12 px free, so the build measures it; if it wraps, it
takes row 2's right end of the status text, not a row of its own); R2 analysis; R3 Method + Threshold (336);
R4 colour group (Background 172, or Pick up to 288); R5 Output (208, Colours wraps to R6 only when it does not
fit: 208 + 8 + 120 = 336 fits); R6 Smooth (232); R7 Ignore below + Advanced (316); then palette and status.
Brightness, Single colour: 6 rows, 216 px, one row more than today (the analysis line). Remove background: 7.
Multicolour adds 2 to 3 (strip, detail). Worst case (Pick colours, Multicolour, 16-colour palette, detail
row, Advanced closed): about 10 rows, 360 px plus the status, inside the 560 px minimum height; with Advanced
open 504 px, which still fits; the PR measures and reports it.
**Panel closed (inner 616):** R1; R2; R3 Method + Threshold/Tolerance + Output (160 + 168 + 208 + 16 = 552);
R4 colour group; R5 Smooth + Ignore below + Advanced (232 + 212 + 96 + 16 = 556); palette strip fits one row
(16 x 32 = 512). Brightness, Single colour: 4 rows.

## 3. Analysis line, Auto and Custom (group 2)

**Layout.** One row, 36 px: the line (12 px, `--toolbar-icon`, up to two lines, never truncated with an
ellipsis, `role="status"`, polite, announced once per change) on the left, the **action button** (text,
28 px, 88 px) on the right. The row exists from the first frame so nothing moves when the result arrives.

**States** (the line is the whole explanation; no tooltip repeats it):

| State | Line | Button |
|---|---|---|
| Analysing (shown after 400 ms, like the trace) | "Analysing..." | none (the slot stays empty) |
| **Suggested** (Frage 83 A only, the default) | "Suggested: Line drawing · paper background evened out" | **Apply** |
| **Auto** (applied) | "Line drawing · paper background evened out" | **Re-analyse**, `aria-disabled`, dimmed: nothing was edited, the result would be the same (deterministic, spec criterion 34) |
| **Custom** (the maker changed an analysed value) | "Custom · Line drawing" | **Re-analyse** |
| **Low confidence** (spec criterion 32) | "Type unclear · Logo / Line art with automatic Threshold" | **Re-analyse**, disabled as in Auto |
| Analysis failed | "Analysis failed · Logo / Line art settings used" | **Re-analyse** |

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

## 4. View pill with Mask (group 1)

- Four 25 x 28 px segments, `role="radiogroup"` "View", arrows move and select, one Tab stop (0025 §13):
  **Overlay, Mask, Trace, Image**. Mask glyph: a 16 px outlined square with the lower-left half filled
  (Lucide `Contrast`). Tooltip names the segment ("Mask"); the Image tooltip keeps its key ("Image (\)").
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

## 5. Method and its context fields (groups 3 to 5)

**Method button.** The Preset button's look and menu mechanics (0025 §11): text button 28 px, 160 px,
chevron because it names a menu, `aria-haspopup="menu"`, accessible name "Method"; menu 232 px with three
`menuitemradio` rows, **Brightness, Remove background, Pick colours**, each with the 0025 second line in the
muted style (describing the result: "Dark and light pixels by a cut-off.", "Everything unlike the
background.", "Pixels like the colours you picked."). Keys as the Preset menu. A pick is not a history step,
changes the method's parameters only, never Smooth, Ignore below, Output or the palette, and marks the
analysed values as edited (Custom).

**Threshold / Tolerance (group 4).** One field, same slot and size for all methods: label inside "Threshold"
(0 to 100 %, as decided 2026-10-29) for Brightness, "Tolerance" (0 to 100) for Remove background and Pick
colours; keys as the 0025 fields (arrows 1, Shift 10, Home, End, type and Enter). Each method remembers its
own value (app setting). The tolerance is one value for all swatches (per-swatch tolerance is not offered).
**(Proposal)**

**Brightness.** No colour group. Invert and the border-shape toggle sit in Advanced as today.

**Remove background (group 5, 172 px).** `[chip 28] [#RRGGBB 56, tabular 12 px] [Auto 44] [pipette 28]`.
- The chip is display only: the background colour over the checkerboard if it has alpha, `--swatch-border`.
  The hex is always printed (colour is never the only cue).
- **Auto** is a text toggle (`aria-pressed`): pressed = the colour is detected from the image border (spec
  criterion 16), the chip and hex show the detected value. A pipette pick fixes the colour and releases Auto;
  pressing Auto again returns to detection. Auto is pressed on entering the method.
- A transparent border detects "transparent": chip shows the checkerboard, the hex reads "none", Tolerance is
  disabled (`aria-disabled`, value kept) because transparency needs none.

**Pick colours (group 5, up to 288 px).** The swatch strip of §7 in a reduced form plus the pipette:
`[swatch x n, 28 px each, 4 apart, up to 8] [pipette 28]`. Empty: no swatches, the pipette is armed on
entering the method and the status says "No colour picked." Swatches and strip behave exactly as the palette
(§7: selection, hex and name, roving focus), with one action in the detail row: **Remove**. The ninth pick
replaces nothing and says "8 colours picked" in the status (permit over refuse: the pipette stays armed, the
click does nothing else). **(Proposal)**

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
  the pick adds a fixed palette colour the same way (§7).
- Armed ends by Esc, the button, choosing another method, or leaving the tool. Esc order: a running trace
  stops first (0025 §5), then the armed pipette disarms, then the tool is left. Focus stays where it was;
  the pipette is a mouse tool and has no key (the button is reachable by Tab, Space toggles it).
- A pick is one settings change: the preview re-traces after the 150 ms debounce, not a history step.

## 7. Output switch and palette (groups 6 and 9)

**Output pill** (group 6): `role="radiogroup"` "Output", two text segments 104 px, **Single colour** and
**Multicolour**; arrows move and select, one Tab stop. Changed only by the maker (never by analysis).
Remembered as an app setting. With **Multicolour** the **Colours** field (120 px) follows in the same group:
label inside, value right; the value is `Auto` with the resolved count muted ("Auto  6"), or a number 2 to
16. Typing a number fixes it; Down at 2, or Delete and Enter on an empty field, returns to Auto. In
Multicolour with Pick colours the palette is exactly the picked swatches (Frage 90), the field shows the
count and is `aria-disabled`.

**Palette** (group 9), below the Ignore below row, `role="listbox"` "Palette", `aria-orientation="horizontal"`:
- **Strip.** Swatches 28 x 28 hit area, 20 px colour (alpha over the checkerboard), `--swatch-border`, 4 px
  apart, up to 10 per row at 344 px, 2 rows maximum (16 colours), one row at 616 px. Order: area descending
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
- **Pipette** for the palette: the pipette of §6 lives at the end of the Output group while Multicolour is on
  (Brightness and Remove background), and in group 5 for Pick colours; both are the same button state.
  A pick adds a fixed colour; a colour within the merge distance of an existing one selects that swatch and
  says "Same as #3366CC" instead of adding a twin.
- No colour perception needed: every swatch is a button with hex and name as its accessible name, the state
  words (picked, excluded, merged into, isolated) are text in the name and the detail row, the marks
  (corner square, `Ban`, dot) are shapes. Hit targets 28 px. Contrast of mark and ring on both themes is
  checked in the review.
- Empty and edge states: no colour found: strip is empty and the status says "No colours found."; one
  colour: one swatch, Merge into is `aria-disabled` (nothing to merge into); more than 16 asked: the field
  clamps and shows 16.

## 8. Geometry controls and Advanced (groups 7 and 11)

**Smooth** (Frage 85 A): the same 0 to 100 field and default. It now drives the input smoothing and the
deviation tolerance together, monotonically; it never moves a protected corner and never rounds one (spec
criterion 8). Old stored values map by the architect's migration (a Logo / Line art trace looks the same or
better). **Readout (Proposal, Frage 91):** a muted 56 px readout right of the field, "0.6 px", the deviation
tolerance in source pixels that Smooth currently means (the number the harness measures); a value like Ignore
below's pixel hint, not an explanation. If Frage 85 becomes B this slot holds the Tolerance field instead.
**Corners** stays in Advanced, renamed in its tooltip only ("Corner protection"; the visible label stays
"Corners"): 0 to 100, higher marks more bends as corners, 0 none. There is no node budget and no node
display outside Advanced.

**Advanced block** keeps 0025 §12 (two 168 px columns, 36 px rows, open state remembered, never dimmed). New
order **(Proposal)**: R1 the two icon toggles **Invert** and **Ignore background** renamed **Drop border
shapes** (Frage 88 A; the accessible name and tooltip use the new name; Invert works in every method; in
Remove background "Invert" keeps the background); R2 Corners and Denoise; R3 Upscale and Full-size preview;
R4 the counts (paths, nodes, ignored specks; the only place). Node limit is gone (0025 Frage 74 A). In Pick
colours and Remove background Denoise is available as in Brightness; analysis may set it.

## 9. Computing, stale and failure

Unchanged from 0025 §14, with: the palette strip and the analysis line keep their last value while a job runs;
the stale preview is 50 % in Overlay, Trace and Mask. A palette that is being recomputed shows the strip at
50 % with the word "Updating..." in the status. Multicolour over the 4 s budget is not UX: the existing
Stop and "Trace again" apply. The failure card is the 0025 card; an analysis failure is the line in §3, not a
card.

## 10. Status line

Only state words, one line plus the muted second line of 0025: "Tracing...", "Stopped", "Preview is out of
date", "Preview from a reduced copy", "0 paths: no colour picked.", "0 paths at this Tolerance: too low." (or
too high, as the 0025 zero-path rule), "No colours found.", "Isolating #FFF2A8", "Merging #FFF2A8 into...",
"2 colours excluded", "8 colours picked", "Same as #3366CC", "Over 100,000 nodes" as before. The success
notice after Enter: "Traced: 7 paths, 412 nodes." (Single) or "Traced: 3 colours, 7 paths, 412 nodes."
(Multicolour; the result is one group "Trace of <name>", one compound path per colour).

## 11. Keyboard and focus

Tab order follows the visual order: Select bar Trace, **i**, Preset, View (one stop), analysis button (when
enabled), Method, Threshold/Tolerance, colour group (Auto toggle, pipette; the strip is one stop with arrows,
Delete removes in Pick colours), Output (one stop), Colours, Smooth, Ignore below, Advanced, then the palette
strip (one stop), its detail buttons, the Advanced controls when open, Retry when shown. There is no
Create button in the order: **Enter** with focus on the canvas or on any bar button creates (Ctrl+Enter in
a field), exactly as today. Enter on a swatch or on the analysis button belongs to that control (Enter on
Apply applies; it does not also create). Esc order in §6. Fields keep the 0025 keys. Focus ring 2 px
`--editor-accent`; a mouse press on a bar button returns focus to the canvas except in fields, the strip and
the picker. Every state change not visible on the canvas is text in the line or the status, announced once.
Nothing animates.

## 12. Strings (all to be in code exactly)

Labels: Trace, Preset, View (Overlay, Mask, Trace, Image), Method (Brightness, Remove background, Pick
colours), Threshold, Tolerance, Auto, Output (Single colour, Multicolour), Colours, Smooth, Ignore below,
Advanced, Invert, Drop border shapes, Corners, Denoise, Upscale, Full-size preview; palette actions Isolate,
Recolour, Reset, Merge into, Exclude, Remove; analysis buttons Apply, Re-analyse. Lines and states: §3, §10.
No other explanatory text. The About tab (i) gets the sections for the methods, the Mask view, the
analysis line and Multicolour in the same pass (spec criterion 24); it is the only place that explains.

## 13. Questions for the customer

Frage 89 (TRQ-PICK): The pipette after a pick
A: In Pick colours and Multicolour it stays armed (Shift+click adds more), Esc ends it; in Remove background it ends after the pick.
B: Always one pick, then it disarms; every further colour needs the button again.
Recommendation: A. Default: A.

Frage 90 (TRQ-MULTIPICK): Multicolour together with Pick colours
A: The palette is exactly the picked colours; the Colours count is fixed and greyed out.
B: Picked colours stay fixed and an automatic count adds more inside the tolerance of the picks.
Recommendation: A (B adds colours the selection has just excluded). Default: A.

Frage 91 (TRQ-SMOOTHREADOUT): Number beside Smooth
A: Show the deviation tolerance as a small value, "0.6 px", beside the field (no sentence).
B: Smooth alone, the number only in Advanced.
Recommendation: A (the only number the quality is measured in). Default: B, because of the no-hints rule until you say yes.

Frage 92 (TRQ-OUTPUTPOS): Where Single colour / Multicolour lives
A: In the main bar after the colour fields (+36 px at panel open; Multicolour is a headline function).
B: In Advanced (the bar stays as small as today; most traces are single colour).
Recommendation: A. Default: A.
