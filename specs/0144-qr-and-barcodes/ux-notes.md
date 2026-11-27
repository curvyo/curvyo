# 0144 QR codes, barcodes and Data Matrix: UX notes

ux-engineer, 2026-11-26, revision 1 (slice 1: QR with styles, logo clear area and colours, criteria 1 to
10 and 13 to 19; a short outline for slices 2 and 3). **Revision 2, 2026-11-27:** section 11 is the full
slice 2 (1D barcodes), written against slice 1 as built (PR #518); section 13 holds its open points; slice 3
stays an outline (11.14). Sections 1 to 10 and 12 are unchanged (slice 1 as proposed; where the build
differs, 11.1 says so). Checked against `specification.md` (Draft of
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

## 11. Slice 2, 1D barcodes (criteria 3, 11, 13 to 16, 19; revision 2)

Revision 2 (2026-11-27) replaces the outline of revision 1 for slice 2. It is written against slice 1 as
built (PR #518), not against sections 1 to 10 as proposed. Everything here is **(Proposal)** unless a
criterion or a decided Frage fixes it. Frage 323 A (the text under the bars, with font and size) and
Frage 406 B (no Background, no Finder colour) apply.

### 11.1 As built in slice 1 (what this section builds on)

- The Generators bar segment reads **"QR code"** (the kind's label; `GeneratorKind::segment`), not "Code"
  as section 1 proposed; the segments sort by their text.
- One kind per symbology (ADR 0091 §2): every 1D symbology is its own kind, so a switch is a kind change.
- The object's box is the symbol without the quiet zone (criterion 2, ADR 0091 §4); Width includes it.
- Colours holds Inverted and the contrast line only (Frage 406 B). Output is Merged or Separate in Advanced.
- Content that cannot be encoded is stored with the last valid paths, drawn in the No solution outline,
  with the lasting line under the field (section 5, ADR 0091 §3). Clamps use the house form in the 3s
  status under the field (`0141` section 6).

### 11.2 The Code segment in the Generators bar (criterion 5)

- **One segment for the whole family**, text **"Code"**, 84px, replacing the built "QR code" segment
  **(Proposal)**.
  Five more segments (one per 1D kind) would push the bar to ten segments and two rows at 1280px; the bar
  picks the kind of object, the panel picks the symbology. Needs a family on the generator kind (ADR 0091
  §2 names `GeneratorKind.family`), section 13 item 1. Order unchanged by the rule: **Box, Box from path,
  Code, Gear, Rounded box**; the bar gets narrower (Code is 84px, "QR code" measures wider).
- **What it places:** the symbology last chosen in this app session (by placing or by a switch in the
  Symbology group); the first of a session is QR code. Not stored in the document. It places that
  symbology **at its defaults** (11.3), as every generator does.
- **Glyph follows what it places** **(Proposal)**: Lucide `QrCode` while it places a QR code, `Barcode` while it places
  a 1D code, 16px. It names the state; no tooltip, no hint. Accessible name "Code".

### 11.3 Ghost, placing and default content

- **Ghost** in the "Drag ghost" look, centred on the pointer: for QR as section 2; for a 1D code the
  box of the bars and, as outlines, only the bars that name the symbology: the start and stop patterns
  of Code 128 and Code 39, the three guard pairs of EAN and UPC (reaching into the text band). Narrow bars
  at 0.33 mm are about 1px at 100 % zoom; all bars would read as a grey block.
- Placing as section 2: one step **"Add EAN-13"** (the label, as "Add QR code"), selected, back to Select.
- **Default content** per symbology **(Proposal)**: visibly placeholders, and for EAN and UPC numbers from the GS1
  **restricted-circulation** ranges (in-store numbers, never a real product). Check digits included and
  computed here; criterion 3's `4006381333931` is a real retail product and stays a test vector only.

| Symbology | Default content | Modules (bars) | Width at the default module size and quiet zone |
|---|---|---|---|
| QR code | `https://example.com` | 25 x 25 | 33.0 mm (Module size 1 mm, quiet zone 4, section 2) |
| Code 128 | `EXAMPLE-01` | 145 (start 11, 10 x 11, check 11, stop 13) | 54.45 mm (quiet zone 10 + 10) |
| Code 39 | `EXAMPLE-01` | 191 at 3 : 1 (12 characters of 15, 11 gaps) | 69.63 mm (10 + 10) |
| EAN-13 | `2001234567893` (prefix 200, check 3) | 95 | 37.29 mm (11 + 7, fixed) |
| EAN-8 | `20123451` (prefix 2, check 1) | 67 | 26.73 mm (7 + 7, fixed) |
| UPC-A | `212345678909` (number system 2, check 9) | 95 | 37.29 mm (9 + 9, fixed) |

  Code 128 and Code 39 share their default, so a switch between them keeps an untouched default (11.4).

### 11.4 The Symbology group and the switch

- **First group, heading "Symbology"** **(Proposal)** (the heading is the label, as Error correction), above Content in
  every code's block, the QR code's included. Revision 1 put it as the first row of Content; a strip
  needs a visible name, and Content's heading names the field.
- **One radio group "Symbology"**, fixed rows (the Dither precedent), one Tab stop, the arrows move and
  choose: **QR code | Code 128 | Code 39** and **EAN-13 | EAN-8 | UPC-A**, six 80px cells, 2px apart
  (244px). No Data Matrix cell before slice 3 (hidden, not disabled); slice 3 re-lays the strip (11.14).
- **A press is a switch**, one step **"Change QR code to EAN-13"**. The object stays the same object
  (Layers row, selection, Fill, Stroke, laser operations, a name the maker typed); a default layer name
  follows the new label. The **centre** stays in place (the bars' centre for a 1D code, 11.11).
- **What crosses a switch:** parameters of equal key and meaning (ADR 0091 §2), with one rule on top,
  **an untouched default follows the symbology** **(Proposal)**: a value still equal to the old symbology's default
  takes the new one's default; a value the maker changed is kept (clamped to the new range, house form).
  So a fresh QR code switched to EAN-13 becomes `2001234567893` with the text on, while typed content is
  never thrown away.
  - Content: always by that rule.
  - Between 1D codes: Module width, Bar height, Human-readable text, Font, Size, Inverted, Bar width
    adjust, Quiet zone (Code 128 and Code 39 only). Wide:narrow is Code 39's own.
  - Between QR and 1D: Content and Inverted only; Module size (2D) and Module width (1D) are different
    keys, so a 1 mm QR does not become a 1 mm-module barcode 160 mm wide.
- **Typed content the new symbology cannot encode** ("Hello" switched to EAN-13): kept, the line of
  11.7 under the field, and the previous paths drawn in the No solution outline (ADR 0091 §3): the shape
  is the old symbology's, the dashed `--destructive` outline says it is not what the panel asks for.
- **Several codes selected:** one symbology, one block (`0141` section 10). Different symbologies are
  different kinds: no Generator block, as `0141` section 10 (section 13 item 3).

### 11.5 The groups (criteria 11, 14)

| Group | Row | Element | Shown when |
|---|---|---|---|
| Symbology | (the heading is the label) | strip of six 80px cells, two rows (11.4) | always |
| Content | Content | one-line text field (11.6), default 11.3 | always |
| Size | Module width | length, default 0.33 mm | 1D |
| | Bar height | length, default 15 mm, the data bars only | 1D |
| | Width | derived length: the outer width with the quiet zones; typing or dragging writes Module width = Width / (modules + both quiet zones); no reset icon (section 3's Width rule) | 1D |
| | Wide:narrow | number, one decimal, suffix ": 1", default 3.0 | Code 39 |
| Text | Human-readable text | switch, on for EAN and UPC, off for Code 128 and Code 39 (criterion 11) | 1D |
| | Font | labelled font row (11.9) | switch on |
| | Size | length, default 3 mm (11.9) | switch on |
| Colours | Inverted | switch, off | always |
| | (contrast line) | state line, section 7 unchanged | as section 7 |
| Advanced | Quiet zone | count, modules, default 10 | group open, Code 128 and Code 39 |
| | Bar width adjust | signed percent, default 0 % (11.8) | group open, 1D |

- **For a 1D code** Modules, Finder, Error correction and Logo are not in the block (not in the kind's
  schema); for a QR code Text is not. Rows below a hidden row move up, as everywhere.
- **No Output row for 1D** **(Proposal**; revision 1 listed one): bars never touch, so Merged and Separate give the
  same paths; a choice that changes nothing is not shown (the one-cell strip precedent of section 3).
- **No Quiet zone row for EAN and UPC** **(Proposal)**: the standard fixes both margins (EAN-13 11 and 7, EAN-8 7 and 7,
  UPC-A 9 and 9), the leading and trailing digits stand in them, and one count cannot say two values
  (section 13 item 5).
- **Height** with the defaults: about 560px for EAN-13 with the text on; Advanced collapsed as in slice 1.

### 11.6 The Content field for 1D codes (one line)

- **Look:** the section 4 field fixed at one line, 244 x 28px; text longer than the field scrolls
  sideways inside it, the caret kept in view. Visible label: the group heading; accessible name
  "EAN-13 content" (the label plus "content", as "QR code content").
- **Keys:** **Enter commits** and returns focus to the canvas (Ctrl+Enter does the same, so the QR habit
  works); Tab commits and moves on; a press elsewhere commits; Escape restores the stored text and its
  geometry and writes nothing. Every key typed stays in the field (section 4).
- **Paste** puts the text in as one line: trailing line breaks are dropped, inner ones become spaces.
- **Live:** every keystroke previews, as section 4; one step per commit, **"EAN-13: Content"**; the content
  is never in a step name, notice or log (criterion 13, 18).
- **Normalised at commit, shown in the field, no notice** **(Proposal**; permit over refuse; the field shows exactly
  what is encoded, so nothing changes behind the maker's back):
  - EAN and UPC: spaces and hyphens are dropped (numbers are often written `4 006381 333931`).
  - Code 39: lower-case letters become capitals (Code 39 has none; the preview shows capitals at once).

### 11.7 Check digits, error lines and the No solution look (criteria 3, 13)

- **EAN-13 (12 or 13 digits), EAN-8 (7 or 8), UPC-A (11 or 12):**
  - One digit short: the check digit is **appended at commit**; the field then shows the full number. No
    notice: nothing the maker typed changed, the new digit is in plain sight.
  - Full length with a wrong last digit: **corrected at commit**, the field shows the right digit and the
    3s status under it, "Check digit corrected." **(Proposal**; criterion 3 asks for "Check digit corrected to 1";
    naming the digit puts content into a notice, criterion 13; section 13 item 4).
  - **Never while typing:** the field keeps exactly what is typed until the commit; the preview already
    draws the code with the right check digit, so the drawing never waits for the 13th key.
- **Code 128:** its mod-103 check symbol is always added, never shown in the field or the text. **Code 39:**
  no check character in slice 2 (optional in the standard; section 13 item 6).
- **Error lines** (section 5's look: `TriangleAlert` 12px, 12px `--field-invalid`, 2px border,
  `aria-invalid`, `aria-describedby`, announced once, lasting, rows below move):
  - EAN-13: "EAN-13 takes 12 or 13 digits." (also when empty; EAN-8 "7 or 8", UPC-A "11 or 12")
  - Code 128: empty "Enter a text to encode."; "Code 128 takes ASCII letters, digits and symbols only."
    (anything outside printable ASCII; GS1-128 and extended characters are not in slice 2)
  - Code 39: empty as Code 128; "Code 39 takes A to Z, 0 to 9, space and - . $ / + % only."
  - Code 128 and Code 39: "Too long for Code 128: 92 characters, at most 80." (section 13 item 6)
  - The sentences name the symbology and the rule, never a character of the content.
- **The status and the line never stand together:** a correction happens only on a commit that encodes;
  should a later edit make the content invalid, the line replaces the status at once.
- **No solution look** exactly as section 5: the last valid paths in the 1.5px dashed `--destructive`
  outline, selected or not, screen only, never exported or cut; the line shows whenever the object is
  selected, also after reopening.

### 11.8 Size rows, tracks and clamps

| Field | Track | Curve | Step (Shift, Ctrl) | Note |
|---|---|---|---|---|
| Module width | 0.2 to 2 mm, to 20 | log | 0.01 (0.1, 0.001) | typed 0.1 to 20 (as Module size); up to three decimals, trimmed |
| Bar height | 5 to 100 mm, to 500 | log | 1 (10, 0.1) | typed 1 to 500 |
| Width | 10 to 300 mm, to what Module width 20 mm gives | log | 1 (10, 0.1) | derived; clamped through Module width |
| Wide:narrow | 2.0 to 3.0 | linear | 0.1 (0.5, 0.01) | criterion 11 |
| Text Size | 1 to 10 mm, to 50 | log | 0.1 (1, 0.01) | typed 0.5 to 50; EAN and UPC clamped (11.9) |
| Quiet zone | 0 to 20 modules | linear | 1 (10, -) | integer; below the standard 10 without a notice |
| Bar width adjust | -30 to +30 % | linear | 1 (10, 0.1) | shown signed: "+5 %", "0 %", "-5 %" |

- Clamps in the house form: "Module width set to 0.1 mm, the minimum.", "Wide:narrow set to 2.0 : 1,
  the minimum.", "Bar width adjust set to +30 %, the maximum."
- **Bar width adjust** widens (plus) or narrows (minus) **what is drawn**, centred on each bar, by the
  percent of Module width, and the gaps by the same amount the other way, so the code keeps its length. On
  an Inverted code the drawn parts are the spaces, so plus widens the spaces: the setting always
  compensates the burn or the kerf of what is engraved or cut (section 13 item 10).

### 11.9 Human-readable text (Frage 323 A)

- **Human-readable text** is a `PanelSwitch` row; Font and Size follow it when on (hidden, value kept).
- **Font row:** a labelled row (label "Font" 60px, control 176 x 28px): an outline button showing the
  face name in 14px ("Source Sans 3 Regular"), truncated with an ellipsis, the full name in its
  tooltip; accessible name "Font: Source Sans 3 Regular". It names a state, so it is not a hint. A press
  (mouse, Enter, Space) does what the Text bar's Font button does: the Fonts tab, the current family
  scrolled into view and its face row focused; the Fonts tab's subject line reads "EAN-13". Choosing a
  face writes Font, one step **"EAN-13: Font"**; focus stays in the Fonts tab, as for a text.
- **Default face: Source Sans 3 Regular** **(Proposal)**, bundled (OFL-1.1), so it exists on every install. OCR-B, the
  face of the GS1 examples, is not bundled (section 13 item 12).
- **Single-line faces** have no area and the code is one filled part: choosing one is refused with the
  existing sentence "Single-line text has no area. Nothing was changed." (`0076`), in the Fonts tab's
  notice place.
- **Missing face** (a file opened on another computer): the Font row shows the Font button's missing look
  (`TriangleAlert` 16px in `--field-invalid` before the name, the name ending ", missing", tooltip
  "Missing on this computer. Shown in Source Sans 3."); the stored paths stay as drawn until an edit
  regenerates the code with the substitute (section 13 item 9).
- **Size row:** a panel Value field 244 x 28px (not the bar's 120px Size field), label "Size", unit mm,
  accessible name "Text size"; the size of the type as in the Text bar.
- **Layout (fixed, no rows):**
  - Code 128 and Code 39: one line centred under the bars, its top 2 modules below them. The text is the
    content as encoded, without the Code 39 `*` and without check characters.
  - EAN and UPC: the GS1 layout: the digits' top sits 1 module below the data bars, the two digit groups
    under the data bars, the guard bars reaching down beside them to the digits' baseline (at least 5
    modules below the data bars); the leading digit of EAN-13 and UPC-A (and UPC-A's last digit) stand in
    the quiet zone, at the same baseline. Digits wider than their 7-module groups would collide, so for
    EAN and UPC Size is limited to what Module width allows: "Size set to 3.1 mm, the most this module
    width allows." (a fixed subject, as `Bound::DependentFor` is built).
  - **The text band** is everything below the data bars: from their bottom to the digits' baseline (the
    guard bars' bottom) for EAN and UPC, to the text's baseline for Code 128 and Code 39. **The box ends at
    the band's bottom** (descenders of Code 128 text included); with the text off there is no band, and
    EAN and UPC guard bars still reach 5 modules down.
- The text is part of the code's one part: same fill, Inverted reverses it with the bars, Convert to
  paths gives it as outlines (criterion 15).

### 11.10 Resize with edge handles (criterion 14; amends `0141` section 8 for 1D codes)

- **A QR code keeps corner handles only** (section 9). **A 1D code shows the eight resize handles of any
  path**: four corners and four edge midpoints, the 8 x 8px "Transform resize handle" look, unchanged; no
  skew handles, no parameter knobs, rotate handles as for any object. Below a 24px box side only the
  corners are drawn (the house rule); a corner reaches both values, so nothing is lost.
- **What a drag writes** (the box is the bars and the text, 11.11):
  - **Left or right edge:** Module width by the width factor; Text Size by the same factor (EAN digits
    must stay under their groups, so the text follows the bars; **(Proposal)**); Bar height kept. With the text on, the
    box's height changes by the text band only.
  - **Top or bottom edge:** Bar height = the new box height minus the text band; Module width and Size
    kept. The opposite edge stays, as for any object.
  - **Corner:** free, as for any path: the horizontal factor writes Module width and Size, the vertical
    one Bar height. **Ctrl** keeps the proportion (all three by one factor), as for any object.
  - Clamped at the field bounds; at a bound the readout appends " min" / " max".
- **Readout chip** **(Proposal)** (the "Live transform readout"): left or right edge "Module width 0.40 mm", top or
  bottom edge "Bar height 22.0 mm", corner "W × H mm". The number a scanner depends on is the module
  width, so the side drag shows it rather than the box width.
- **During the drag:** the "Live preview outline" (blue new over black old, `0141` section 7); Escape
  restores and writes nothing; one step **"Scale"** on release (as built).
- **Size chip** (`S`): W and H independent for 1D; W writes as a side drag, H as a top or bottom drag.
- **A code in the No solution state** (content the symbology cannot encode, for example after a switch)
  refuses every resize, as built (`plan_resize` answers Refused): the handles drag nothing and the box
  stays; the fields remain the route.

### 11.11 On the canvas, convert, files

- **Box:** the bars and the human-readable text, without the quiet zone (ADR 0091 §4), so the EAN-13 box
  reaches left to the leading digit. Inverted brings the quiet zone in, as for QR.
- **Fixed point** **(Proposal)**: the centre of the **bars**, not of the box, so turning the text on adds the band below
  and the bars do not move; a change of Content, Module width, Quiet zone, Wide:narrow or a switch keeps
  that centre; Bar height grows up and down from it (section 13 item 11).
- **Object to path:** one step "Convert EAN-13 to paths"; one compound path, bars and text outlines
  (criterion 15); Break apart (`0035`) gives the bars and the glyphs as paths.
- **Unknown symbology** (criterion 16): as section 9, "1 code uses a symbology this version does not
  have. It is shown as plain outlines."

### 11.12 Names and glyph

- Layers glyph Lucide **`Barcode`** for every 1D kind (`QrCode` stays for QR); names are the labels:
  **"Code 128", "Code 39", "EAN-13", "EAN-8", "UPC-A"**. Not enterable.
- Subject line "EAN-13"; several of one kind "2 EAN-13 codes" ("2 QR codes"); status bar
  "EAN-13 · <page>".
- Steps: "Add EAN-13", "EAN-13: Content", "EAN-13: Module width" (the field label), "EAN-13: Font",
  "Change QR code to EAN-13", "Scale", "Convert EAN-13 to paths".

### 11.13 Accessibility

Tab order is screen order: Symbology, Content, Module width, Bar height, Width, Wide:narrow, Human-
readable text, Font, Size, Inverted, the Advanced header, Quiet zone, Bar width adjust, then the Style
sections. The strip and each switch are one stop; the Font button is one stop. Every control has a visible
label (the strip by its heading). The error line and the check digit status are linked by
`aria-describedby`; the status is `role="status"`, polite. Edge handles are pointer only, as for any
object; the keyboard route is the Size chip and the fields. Tokens only; nothing animates.

### 11.14 Slice 3, Data Matrix (criterion 12, outline)

- Symbology strip re-laid in three fixed rows: **QR code | Data Matrix** (122px), **Code 128 | Code 39 |
  EAN-13** and **EAN-8 | UPC-A** (80px); "Data Matrix" does not fit 80px.
- Groups: Symbology, Content (multi-line, as QR), Size, Modules (as QR: shape, rounding, fill), Colours
  (Inverted), Advanced (Quiet zone, default 1; Output). No Finder group (the L finder and the clock edge
  are always solid), no Error correction (ECC 200), no Logo, no Shape row until rectangular sizes come.
  Corner handles only, as QR.
- Layers glyph Lucide `Grid3x3`, name "Data Matrix"; default content `https://example.com` (it crosses
  from QR untouched).

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

## 13. For the product owner and the architect, slice 2 (revision 2)

Readings and open points (the lead numbers them; default = as written in section 11):

1. **One Code segment** (architect, product owner): a family on the generator kind (ADR 0091 §2's
   `GeneratorKind.family`) so the bar shows one segment "Code" for every symbology, placing the session's
   last one, glyph following it; this renames the built "QR code" segment. Alternative: one segment per
   kind (ten segments, two bar rows at 1280px). Recommendation and default: one segment.
2. **The switch as a host command** (architect): the kind change that keeps the object, its style and
   operations, the parameters of equal key, and the "untouched default follows" rule of 11.4 (a value
   equal to the old kind's default takes the new default). Keys proposed: `module_width`, `bar_height`,
   `text`, `font`, `text_size`, `bar_adjust`, `quiet_zone` shared by the 1D kinds; `module_size` stays
   QR's. Step "Change QR code to EAN-13"; the centre kept.
3. **Mixed symbologies selected** (product owner): no block, as `0141` section 10 (default). Alternative:
   the Symbology group alone, to switch them all at once (Later).
4. **Check digit wording** (product owner): "Check digit corrected." without the digit (default), since
   criterion 3's "Check digit corrected to 1" puts content into a notice, which criterion 13 forbids; the
   field shows the digit. A criterion 3 amendment.
5. **EAN and UPC quiet zones** (product owner): fixed by the standard, no row (default), a reading of
   criterion 11 ("11 and 7 for EAN/UPC as the standard says").
6. **Code 128 and Code 39 limits** (product owner): at most 80 characters (default; scanners commonly
   take 48 to 80); printable ASCII only for Code 128; no optional Code 39 check character in slice 2.
7. **No Output row for 1D** (product owner, architect): bars never touch, so the choice changes nothing;
   criterion 11's "Merged by default" holds trivially.
8. **Edge handles for 1D codes** (product owner, architect): amends `0141` section 8 ("corner handles
   only") for 1D kinds; needs a resize edit with two independent factors (today `Edit::Scale` has one),
   corners free and Ctrl proportional as for any path; criterion 14 already allows it.
9. **Font for a generator** (architect, ADR 0091 §9 revision): a font parameter kind (family and style,
   stored by name), glyph outlines passed into the pure generator by the host, the Fonts tab acting on a
   selected code (subject line, face row press writes Font, single-line refused), and a missing face:
   stored paths kept until an edit regenerates with the substitute (default).
10. **Bar width adjust** (architect, product owner): a signed percent (`ParamKind::Percent` is unsigned
    today); it applies to what is drawn, so on an Inverted code it widens the spaces (default).
11. **Fixed point and box** (architect): the origin at the bars' centre (so the text toggle does not move
    the bars); the box includes the text and the EAN leading digit, without the quiet zone.
12. **Defaults** (product owner): contents from GS1 restricted-circulation ranges (11.3); text face Source
    Sans 3 Regular (bundled); OCR-B not bundled (adding it is a font licence question); text Size 3 mm,
    following Module width in a resize and limited for EAN and UPC by the group width.
13. **Normalisation at commit** (product owner): EAN and UPC drop spaces and hyphens, Code 39 upper-cases,
    pasted line breaks become spaces; all shown in the field, no notice.

**To go into `docs/design-system.md` when slice 2 is Done:** the Code segment (one per family, glyph
following the placed symbology), the Symbology strip, the one-line Content field, the Font row (labelled
outline button with the face name), the 1D edge-handle rule and its readouts, the check digit status.
