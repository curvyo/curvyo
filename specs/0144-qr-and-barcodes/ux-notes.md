# 0144 QR codes, barcodes and Data Matrix: UX notes

ux-engineer, 2026-11-26, revision 1 (slice 1: QR with styles, logo clear area and colours, criteria 1 to
10 and 13 to 19; a short outline for slices 2 and 3). Checked against `specification.md` (Draft of
2026-11-20, Fragen 319 to 323 at their defaults: 319 A, 320 A, 321 B, 322 A, 323 A),
`specs/0141-parametric-generators/ux-notes.md` (sections 1 to 15; a Code is a further generator of the
same contract, so everything not named here is as there), `specs/0147-rounded-box/ux-notes.md`,
`specs/0155-generator-joint-options/ux-notes.md` (the Advanced group), `specs/0124-gradient-fill/` and
`docs/design-system.md` (rows "Value field", "Text field", "Validation chip", "Choice strip", "Color row",
"Picker", "Gradient (`0124`)", "No solution outline", "Drag ghost", "Live preview outline", "Vector
dimension look", "Action notice", "Hidden, not disabled").

Customer rules applied: no popups in the panel, no operating tips or hint labels, no notice after a
success, hidden not disabled. No new colour token. One new component: the multi-line text field (section
4, added to `docs/design-system.md`). Everything not fixed by the spec or a decided Frage is
**(Proposal)**, vetoable at the demo.

## 1. The entry in the Generators bar (criterion 5)

- **A further segment** of the Generators bar's radio group "Generator" (`0141` section 1): text
  **"Code"**, Lucide `QrCode` 16px **(Proposal)**. One segment for every symbology: the bar picks the
  kind of object, the panel picks the symbology (slice 2). The rail button and the tool are unchanged.
- **Order** by the registry rule (built-ins by segment text): **Box, Box from path, Code, Gear, Rounded
  box**. Code is 84px; the five segments are about 520px, so the bar is one row from a window of about
  900px and wraps whole segments below that by the bar overflow rule (implementer measures).

## 2. Ghost and placing (criterion 5)

- **Ghost.** The default code (content "https://example.com", 19 bytes: version 2 at M, 25 x 25 modules,
  33 x 33 mm with Module size 1 mm and Quiet zone 4) in the "Drag ghost" look, centred on the pointer:
  the square of the symbol (without the quiet zone) and the outer squares of the three finder patterns,
  no data modules **(Proposal)**: hundreds of module outlines at a low zoom read as grey noise; the
  finders say "QR" at once.
- Click or press-drag-release places it centred there; one step **"Add code"** (criterion 5; section 12
  item 1 recommends "Add QR code");
  selected; back to Select; the Style tab shows; focus is not moved (`0141` section 2). Enter and Escape
  as `0141`.
- **Default content "https://example.com"** (spec Proposal, kept): visibly a placeholder, scans to a
  harmless page, and is the most common kind of content, so the maker sees the right shape before typing.
- **Names.** Layers glyph `QrCode`, name "QR code"; subject line "QR code", "2 QR codes"; status bar
  "QR code · <page>". Not enterable (one object, as the Box).

## 3. The Generator block (criteria 5 to 10)

The `0141` block (sections 3 and 15): Style tab, first block, a heading per group, numbers as Value
fields 244 x 28px, choices as choice strips, on/off as a `PanelSwitch` row, nothing after the last group.
Groups **(Proposal)**: Content, Size, Modules, Finder, Error correction, Logo, Colours, Advanced
(collapsed). The lead's "Style" group is split into **Modules** and **Finder** and not named "Style",
because a "Style" heading would sit right above the Style tab's own Stroke and Fill sections; **Size** is
its own group because Module size and Width belong neither to the content nor to the look.

| Group | Row | Element | Shown when |
|---|---|---|---|
| Content | Content | multi-line text field (section 4), default "https://example.com" | always |
| Size | Module size | length, default 1 mm | always |
| | Width | derived length (the Pitch diameter rule of `0141` section 5): the outer width including the quiet zone, the same number as the selection box and the W readout; typing or dragging writes Module size = Width / (modules + 2 x quiet zone); no reset icon | always |
| Modules | Module shape | choice strip **Square / Rounded square / Dot / Diamond**, two rows of two 122px cells, one radio group, one Tab stop | always |
| | Rounding | percent, default 30 % | Module shape Rounded square |
| | Module fill | percent, default 100 % | always |
| Finder | Outer | labelled choice row (label 60px, strip 176px: **Square / Rounded / Circle**) | always |
| | Inner | labelled choice row, the same three | always |
| Error correction | (the heading is the label) | choice strip **L 7 % / M 15 % / Q 25 % / H 30 %**, four 61px cells, default M | always |
| Logo | Clear area | switch, off | always |
| | Shape | choice strip **Square / Rounded / Circle**, 80, 80, 84px; default Circle | Clear area on |
| | Size | percent of the symbol width (without the quiet zone), default 20 % | Clear area on |
| | Margin | count, modules, default 1 | Clear area on |
| Colours | Inverted | switch, off | always |
| | Finder colour | switch, off; when on a Color row follows | always |
| | Background | switch, off; when on a Color row follows, default white | always |
| | (contrast line) | state line (section 7) | on a drawing page, while the contrast is below 3 : 1 |
| Advanced | Quiet zone | count, modules, default 4 | group open |
| | Output | choice strip **Merged / Separate**, 122px cells | group open, Module shape Square or Rounded square and Module fill 100 % |

- **Symbology is not drawn in slice 1**: with QR the only choice a one-cell strip says nothing (the
  `0147` Top shape precedent). It comes as the first row of Content in slice 2.
- **Choice strips with four options** wrap into two rows of two (the Dither fixed-row precedent), so
  "Rounded square" keeps its full name; at 61px it would not fit. Four short cells (Error correction)
  stay one row.
- **Labelled choice row** (Finder Outer and Inner): two strips with the same three cells need a visible
  name each, so they take the Style panel's row grid (label column 60px, control column 176px, as Join
  and Cap). Cells about 58px; if "Rounded" does not fit at 14px with the cell padding, the cell reads
  "Round" (implementer measures).
- **Error correction cells** carry the recovery share ("M 15 %"): it names the state, the letter alone
  does not. Accessible names "L, 7 percent recovery" and so on; the radio group is named "Error
  correction". The group sits **directly above Logo** **(Proposal)**, because turning the clear area on
  changes it (section 6) and the maker sees the change right above the switch. It is not in Advanced:
  both the content limit (section 5) and the clear-area limit depend on it.
- **Fill is not in the block.** The modules' colour is the object's ordinary fill (criterion 9, `0124`):
  the Style tab's Fill section below the block, with Solid and Gradient as for any object. The Colours
  group holds only what a plain fill cannot say. Default fill black, no stroke.
- **Inverted in Colours** **(Proposal)**, not in Advanced: it decides which modules take the fill, and an
  engraving on dark material needs it often.
- **Output** (criterion 7): shown only where both make sense, Square or Rounded square at Module fill
  100 %; elsewhere the modules are Separate and the row is hidden (the stored choice is kept and comes
  back at 100 %). A new code is Merged. The row comes and goes while the Module fill slider passes
  100 %; it is in the collapsed Advanced group, so this rarely shows (section 12 item 7).
- **Colour rows** in Colours: the panel's Color row (swatch 28, eyedropper 28, hex) unchanged, eyedropper
  names "Pick finder colour from the drawing", "Pick background colour from the drawing". **One picker**
  (the `0017` area and hue) under the active colour row, the 3px `--editor-accent` bar marking that row
  (the `0124` stop precedent); the active row is the one last switched on or focused. No popup.
  Finder colour starts as the fill's colour (the first stop of a gradient).
- **Conditional rows** as `0141` section 3: hidden, the value kept, only the rows below move.
- **Height.** About 740px with the defaults, about 1000px with the clear area and one colour (and its
  picker) on: the tab scrolls in its one scrollbar, as the gear and box do.

## 4. The Content field (criteria 5, 13; the new component)

- **Look.** The panel "Text field" (white ground, 1px `--toolbar-icon` at 60 % border, `rounded-[5px]`,
  14px text, focus border `--editor-accent`), 244px wide, multi-line: one line high (28px) for one line,
  growing with the text to five lines (108px), then it scrolls inside. Long words break anywhere (URLs have
  no spaces). Visible label: the group heading "Content"; accessible name "QR code content".
- **Keys.** Enter inserts a line break (a multi-line field); **Ctrl+Enter** commits and returns focus to
  the canvas; Tab commits and moves on; a press elsewhere commits; Escape restores the stored text and its
  geometry and writes nothing. Every key typed in the field stays in it (tool letters, Delete, Backspace
  and Ctrl+Z do not reach the canvas). What is typed is encoded exactly, a trailing line break included.
  Paste keeps line breaks.
- **Live.** Every keystroke regenerates the code in place (the latest finished regeneration is drawn,
  never blank, `0141` section 7). One undo step per commit, **"QR code: Content"**; the step name never
  holds the text (criterion 18), nor does any log.
- **Several codes selected:** "Mixed" placeholder when the contents differ; a committed text writes all
  (`0141` section 10).
- **Focus.** A click places the caret; Tab in selects all (the field does this; a textarea does not by
  default).

## 5. Invalid or empty content (criterion 13)

- **Reason in place.** Under the field, in the flow (rows below move down; it is a lasting state, so not
  the validation chip, which clears on the next keystroke): `TriangleAlert` 12px and 12px
  `--field-invalid` text, the field's border 2px `--field-invalid`, `aria-invalid="true"` and
  `aria-describedby` to the line, announced politely once when it appears. Texts **(Proposal)**:
  - empty: "Enter a text to encode."
  - too long: "Too long for a QR code at error correction M: 2412 bytes, at most 2331." (the unit the
    encoder used: digits, characters or bytes; at L the limit of version 40, criterion 2)
- **Geometry kept.** While typing, the canvas keeps the last geometry that was valid. A commit of an
  invalid or empty text **is stored** **(Proposal)**, so lowering the error correction afterwards can make
  the same text fit without retyping it; one step as any commit. While stored content does not match the
  drawn code, the object keeps the **"No solution outline"** (1.5px dashed `--destructive`, selected or
  not, screen only, never exported or cut: the design system's one look for "not as asked"), so nobody
  engraves a code that says something else than its field. The line shows whenever the object is
  selected.
- Escape before a commit returns to the stored text and its geometry; the line goes.

## 6. Error correction and the clear area (criteria 5, 8)

- **Turning Clear area on** raises Error correction to **H** in the same step when it is lower; if the
  content does not fit at H, to the highest level it fits. No notice: the strip right above changes in
  the same frame. **Turning it off** leaves the level as it is (nothing changes behind the maker's back).
  **(Proposal**, option a; option b, an "automatic" level that follows the switch until the maker picks
  one, needs an automatic marker on a choice in the schema, for the architect.)
- **The budget** limits the clear area (criterion 8). Clamps as `0141` section 6 (permit, the value set,
  a 3s status under the edited field); texts:
  - "Clear area limited by error correction M" (criterion 8's chip text, for Size typed or dragged past
    the budget; section 12 item 2 recommends "Logo size set to 24 %, the most error correction M
    allows.")
  - "Logo margin set to 1 module, the most error correction M allows."
  - knock-on, under the Error correction strip when a lower level shrinks the budget: the clear area sentence.
  - "Error correction set to Q, the most this content allows." (a higher level pressed that the content
    does not fit; the level is set as high as it fits)
  - a content commit whose new version shrinks the budget: the clear area sentence under the Content field.
- The stored Size is the clamped value (the numbers stay true): raising the level later does not grow the
  area by itself.
- **Clear area outline on the canvas** (spec UX placeholder "while the section is open"; groups do not
  collapse, so read as **while the code is selected**, alone or with other objects, so it guides the logo
  while both are selected for Align): the requested shape at Size, centred on the symbol, 1px
  `--dim-guide` dashed 4px on 3px off with the white casing, screen space, never exported, cut or
  snapped to; with the Select tool only. The hole itself (the removed modules, Margin included) is the
  code's geometry. The logo centres on the object's centre with Align (the quiet zone is symmetric).

## 7. Colours and contrast (criterion 9)

- **Contrast line** (criterion 9's chip): the last line of the Colours group, in the flow, `TriangleAlert`
  12px and "Low contrast, may not scan" in 12px `--toolbar-icon` (a warning to act on, not a refusal: no
  `aria-invalid`), `role="status"`, announced politely once. Shown while any dark colour (each fill stop,
  the Finder colour) is below 3 : 1 against the light one (the Background when on, otherwise the document
  background, `0040`). Inverted swaps nothing in the check (the ratio is symmetric); a light code on a
  dark Background passes. **(Proposal**, for the product owner: the worst stop of a gradient and the
  document background as the light side.)
- **On a laser page** the line is not shown: colours there are operations, not ink (`0101`).
- The Background is one rectangle behind the code covering the quiet zone (criterion 9), part of the
  object's geometry only while on.

## 8. Tracks (as `0141` section 15.3)

| Field | Track | Curve | Step (Shift, Ctrl) | Note |
|---|---|---|---|---|
| Module size | 0.2 to 5 mm, to 20 | log | 0.05 (0.5, 0.005) | typed 0.1 to 20 (criterion 5) |
| Width | 5 to 300 mm, to the most Module size 20 mm gives | log | 1 (10, 0.1) | derived; clamped through Module size |
| Rounding | 0 to 50 % | linear | 1 (10, 0.1) | 50 % is a circle |
| Module fill | 50 to 100 % | linear | 1 (10, 0.1) | |
| Logo size | 5 to 35 % | linear | 1 (10, 0.1) | the budget by clamp |
| Logo margin | 0 to 2 modules | linear | 1 (1, -) | integer; the budget by clamp |
| Quiet zone | 0 to 10 modules, to 20 | linear | 1 (10, -) | integer; below the standard 4 is allowed without a notice |

Clamps at the field bounds as `0141`: "Module size set to 0.1 mm, the minimum.", "Width set to 3.3 mm,
the least this code allows."

## 9. On the canvas, resize, convert, files (criteria 14 to 16)

- **Fixed point.** A change in the panel that changes the code's size (content to a new version, Module
  size, Quiet zone) keeps the **centre** in place **(Proposal)**, so a code placed on a label stays
  centred on it.
- **Resize** (criterion 14): proportional only, corner handles, as the `0141` section 8 generators; the
  factor writes Module size; readout "W × H mm"; at 0.1 or 20 mm "min" / "max". Move, rotate, mirror and
  align as any object. No canvas knobs in slice 1 **(Proposal)**: no distance here is worth aiming at.
- **Object to path** (Select bar, `0141` section 15.5): one step "Convert QR code to paths"; a Merged code
  becomes one compound path, a Separate one a group of module paths; with Finder colour or Background on,
  a group of those parts; selected after. No button in the block.
- **Node tool** pill as `0141` section 8. Operations, booleans and linked clones as `0141` section 8.
- **Unknown symbology** (criterion 16): as `0141` section 12, text "1 code uses a symbology this version
  does not have. It is shown as plain outlines." / "3 codes use ...".

## 10. Accessibility

Tab order is screen order: Content, Module size, Width, Module shape, Rounding, Module fill, Outer,
Inner, Error correction, Clear area and its rows, Inverted, Finder colour and its Color row, Background
and its Color row, the picker, the Advanced header, Quiet zone, Output, then the Style sections. Each
strip and switch is one stop; Space toggles a switch. Every control has a visible label (a strip by its
cells or its row label, Error correction by its heading). The error and contrast lines are linked by
`aria-describedby`. Tokens only; nothing animates.

## 11. Slices 2 and 3 (outline)

**Slice 2, 1D barcodes (criterion 11).**
- **Symbology** becomes the first row of Content: one radio group "Symbology" of fixed rows, **QR code /
  Data Matrix** (122px cells, Data Matrix from slice 3), then **Code 128 / Code 39 / EAN-13** and **EAN-8 /
  UPC-A** (80px cells). The bar's Code segment places the last symbology of the session (first: QR).
  A switch keeps Content; content the new symbology cannot encode shows section 5's line and keeps the
  previous geometry in the No solution outline.
- Content is one line for 1D (Enter commits). Check digit: the field shows the corrected digits and a 3s
  status under it, "Check digit corrected to 1." (criterion 3). Texts such as "EAN-13 takes 12 or 13
  digits."
- Groups: Content, Size (Module width, Bar height, Width derived, Wide:narrow for Code 39), Text
  (Human-readable text switch, Font button and Size as the Text bar's, criterion 11 and Frage 323 A),
  Colours (Inverted, Background; no Finder colour), Advanced (Quiet zone, Bar width adjust, Output).
  Modules, Finder, Error correction and Logo are hidden.
- **Resize breaks the generators' proportional-only rule** (criterion 14): a 1D code shows edge handles
  too; a horizontal drag writes Module width, a vertical one Bar height. For the product owner and the
  architect, since `0141` section 8 says "corner handles only".
- Layers glyph Lucide `Barcode`, names "Code 128", "EAN-13", steps "EAN-13: Content".

**Slice 3, Data Matrix (criterion 12).**
- Symbology cell Data Matrix. Groups: Content, Size, Modules (as QR: shape, rounding, fill), Colours
  (Inverted, Background; no Finder colour), Advanced (Quiet zone, default 1; Output). No Finder group (the
  L finder and the clock edge are always solid), no Error correction (ECC 200), no Logo, no Shape row
  until rectangular sizes come.
- Layers glyph Lucide `Grid3x3` **(Proposal)**, name "Data Matrix".

## 12. For the product owner and the architect

Readings and open points (the lead numbers them; options, recommendation, default):

1. **Step name on placing** (product owner): the spec's "Add code" (default, built as criterion 5) or
   "Add QR code", matching Layers, the subject line and "QR code: Content". Recommendation: "Add QR code"
   (a criterion 5 amendment).
2. **Clamp wording** (product owner): the spec's "Clear area limited by error correction M" (default,
   criterion 8) or the house form "Logo size set to 24 %, the most error correction M allows."
   Recommendation: the house form, one sentence shape for every clamp (a criterion 8 amendment).
3. **Invalid content is stored** (architect): the record keeps the typed content and the last valid
   paths, and a reopen draws those paths in the No solution outline (section 5; criterion 16 already
   keeps stored paths for an unknown symbology). Alternative: refuse the commit and keep only a draft in
   the field, which loses the text on a selection change. Recommendation and default: stored.
4. **Error correction with the clear area** (architect, product owner): option a of section 6 (switch
   on raises to H, off leaves it) or option b (an automatic level, a schema extension). Default a.
5. **Centre alignment pattern** (product owner): from version 7 a QR has an alignment pattern at the
   centre. Criterion 8 keeps function patterns, so it stays inside the clear area and shows through or
   around the logo. Either accept it, or offset the clear area (no longer centred) for those versions.
   Default: accept, centred.
6. **Logo size reference** (product owner): percent of the symbol width without the quiet zone (section
   3); the spec says "of the code width".
7. **Output below 100 % fill** (product owner): as criterion 7, Separate and the row hidden (default,
   section 3). Alternative: keep the row for Square and Rounded square at every fill (no row jump; Merged
   below 100 % is one compound path of untouching squares), a criterion 7 amendment. Recommendation: the
   default, since the row sits in collapsed Advanced.
8. **Contrast measure** (product owner): the worst gradient stop and, without Background, the document
   background (section 7). Hidden on laser pages.
9. **Schema elements for ADR 0080 §2** (architect): a **Text** element (multi-line, with an error
   report the shell shows under it), a **Colour** element (the Color row with a switch to turn it on), a
   **labelled choice** (visible row label) and the two-row layout for a choice of four, a **percent**
   unit, and a **derived** length whose commit writes another parameter (exists for Pitch diameter).
   Finder colour and Background make the output parts with their own fills; the Style sections' Fill
   writes the modules' part only.
10. **Selection box with the quiet zone** (architect): the object's box includes the quiet zone even
    without a Background, so Width, the W readout and criterion 2's 29 mm agree.
11. **Export or send a code in the No solution outline** (product owner): an Output check that says
    the code does not match its content? Default: none, the outline only, as for a sketch clone.

**To go into `docs/design-system.md` when Done:** the Code segment, the Code block's groups, the labelled
choice row and the two-row four-choice strip in the Generator block, the clear area outline, the content
and contrast lines.
