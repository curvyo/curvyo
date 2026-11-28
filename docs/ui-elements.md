# UI elements

The catalogue of Curvyo's interactive elements as they **should** be: one entry per element
kind, with its anatomy, sizes, tokens, states, icon rule, keyboard model, when to use it and the
one component that implements it. Started by `specs/0159-ui-consistency` (2026-11-27, customer:
"buttons uniform, sliders uniform, toggle buttons shown by pressed state and colour instead of the
sliding switch, all buttons with symbols"); the code as found is in
[`specs/0159-ui-consistency/inventory.md`](../specs/0159-ui-consistency/inventory.md) (findings
`I1` ...).

This file does not repeat [`design-system.md`](design-system.md). Colour values and contrast
figures are in its "Color tokens" table, screen sizes of canvas glyphs and handles in "Spacing and
sizing", behaviour patterns (Inline confirm, Armed action, List row, Canvas notice slot, Value
field, No popups in the panel, Bar overflow) in "Interaction conventions". Where an entry here
and a feature section there disagree, this file is the rule from 0159 on and the feature section
gets a pointer here when it is next touched.

Choices marked **(Proposal)** are the UX engineer's and can be vetoed at the demo. The customer
questions of 0159 are Frage 416 to 420 and 430 to 432 (`specs/0159-ui-consistency/fix-plan.md`
§3); the customer answered them on 2026-11-27 (416 A, 417 A, 418 A, 419 A, 420 A, 430 A, 431 A,
432 A; `specs/0159-ui-consistency/specification.md`, "Answers"). Entries built by 0159 carry a
"Built" line; the build plan is `specs/0159-ui-consistency/plan.md`.

## 0. Shared rules

### Surfaces

| Surface | Ground | Where |
|---|---|---|
| Panel | `--panel-bg` | Properties panel body and strip |
| Bar | `--toolbar-bg`, `--panel-elevation-shadow` | contextual bars and the rail cards over the canvas |
| Canvas chip | `--toolbar-bg`, shadow when it holds a field | readouts, entry chips, notices, tags over the canvas |
| Field | white (`--tile-bg`) | every field a value is typed into |
| Inline form box | the panel ground, 1 px `--toolbar-icon` at 25 % border, 8 px padding | Add page, Add format, Add source, New material: a group of fields and buttons that opens in place |

Panel and bar share one colour, so a control on them is never told apart by its ground alone: it
has a border, a fill or a glyph (I3).

### Sizes

- **Control height:** 28 px in the panel and the bars, 24 px only inside a list row, 36 px under
  `pointer: coarse` for every control a finger can reach (`pointer-coarse:h-9`). The rail keeps
  its 40 px buttons ("Toolbox card").
- **Panel grid:** 244 px content width, label column 60 px, control column 176 px, 8 px between
  rows (design-system "Properties panel: Document section", "Rows").
- **Gaps:** 4 px between buttons of one group, 8 px between buttons of an inline confirm or form,
  12 px between groups in a bar (with the 1 px 20 px divider at 25 %).

### Radius (Proposal)

Three values, written as arbitrary values so the shadcn radius theme (`--radius` 0.625rem, which
makes `rounded-md` 8 px and `rounded-lg` 10 px) cannot shift them:

| Radius | For |
|---|---|
| 5 px (`rounded-[5px]`) | every control: buttons, icon buttons, toggle buttons, segmented strips, fields, the symbol pill's inner cells |
| 8 px (`rounded-[8px]`) | every surface: bars, rail cards, entry and readout chips, notices, tooltips, inline form boxes |
| full (`rounded-full`) | discs and badges: participant discs, modifier badges, the canvas symbol pill |

`rounded-md`, `rounded-lg`, `rounded`, `rounded-sm`, `rounded-[4px]` and `rounded-[7px]` go
(I36). The 28 px bar icon buttons move from 8 to 5 px: they read as buttons rather than as dots.
**Built (0159, token pass):** no `rounded-md`, `-lg`, `-sm`, bare `rounded`, `-[4px]`, `-[7px]` left in
the components or `index.css`; list rows and fields at 5 px, floating chips, drag ghosts and
popovers at 8 px, the 3 px selection bars, progress bars and skeleton lines `rounded-full`. Kept:
3 px on the checkbox, the 16 px sketch marks, the 12 px photo-crop handle and the font skeleton
bars (too small for 5 px); 6 px on the colour picker's hue strip (its own control, §9); 2 px on the
text link's focus ring (it hugs inline text).

### Type (Proposal)

14 px (`text-sm`) for control labels, field values and row names; 12 px (`text-xs`) for secondary
lines, chips, notices, tooltips and section headings (semibold); 11 px only for tabular meta lines
inside a row (sizes, counts, "Material 60"). 13 px and 10 px go (I37). Numbers are always
`tabular-nums`. Sentence case everywhere. **Built (0159, token pass):** 13 px and 12 px arbitrary sizes are
`text-sm` and `text-xs`, 10 px meta lines 11 px or 12 px. Kept: the 13 px value of the 84 px hex
field (`NumberField compact`, a nine-character hex must fit), 8 to 10 px numerals inside the 14 to
17 px discs and badges (pick set, ring labels, operation disc, sketch marks), the 28 px specimen of
the font info view (a sample, not UI text).

### Icon rule

- **Every button has a symbol.** A button with a visible label shows the symbol at its left, 4 px
  gap ("Symbol + label"). Icon-only is for the rail, the bars' established icon groups (Align,
  Flip, Style, Writing mode, Operation kinds), list-row actions and panel header actions; each
  icon-only button has an `aria-label` and a tooltip with the same name. (Frage 417 A, customer
  2026-11-27: Cancel, OK and Add included.) `Button` takes the symbol as a required `icon` prop,
  so a labelled button without one does not type-check.
- **Glyph:** Lucide, 16 px in controls (20 px in the rail), `strokeWidth={1.5}` with
  `absoluteStrokeWidth`, `currentColor`, `aria-hidden`. Own glyph sets (`BooleanGlyphs`,
  `PathGlyphs`, `OperationGlyphs`, ...) follow "Command glyphs". No text characters as glyphs ("×",
  "+"; I10). Disclosure chevrons are 12 px everywhere (I38). **Built (0159, token pass):** `GLYPH`, `SMALL_GLYPH` and
  `RAIL_GLYPH` in `ui/looks.ts` (16, 12, 20 px, 1.5 px absolute stroke) on every Lucide glyph that
  had a size (14, 15 and 18 px controls become 16; 14 px chevrons 12). Kept: the 8 px glyphs of the
  modifier badges and the pressed tab tick; glyphs sized by a class (`size-3`) or by their parent.
- **Common symbols:** one symbol per act, so the same word always shows the same glyph.

| Act | Lucide | Act | Lucide |
|---|---|---|---|
| Cancel, Close | `X` | Add, New ... | `Plus` |
| OK, Done, Apply | `Check` | Save | `Save` |
| Delete, Remove | `Trash2` | Discard, Wipe | `Eraser` |
| Undo move, Restore | `Undo2` | Reset placement | `RotateCcw` |
| Edit | `Pencil` | Copy, Copy report | `Copy` |
| Import ... | `FileInput` | Export ... | `FileOutput` |
| Download | `Download` | Try again | `RefreshCw` |
| Join, Connect, Pair | `LogIn`, `Plug`, `KeyRound` | Link, Unlink | `Link2`, `Unlink` |
| Inspect, Show ... | `Eye` | Search | `Search` |
| Convert to paths, Object to path | `Spline` | Open outside the app | `ExternalLink` |

  A feature-specific act keeps its own glyph from its UX notes. Chosen in 0159 where none existed
  **(Proposal)**: Fit to content `Scan`, Trace `Scan`, Analyse `ScanSearch`, Recognise text
  `ScanText`, Bridge and Stroke to path `Spline`, Box from path `Box`, Remove rounding `Square`,
  Linked clone `Link2`, Template `LayoutTemplate`, Quit `Power`, Pen probe `Activity`, Set file
  aside `Archive`, Spread colours `Palette`, Make editable `Pencil`, Select original(s) and
  Select clones `MousePointerClick`, AI run `Sparkles`, Automatic `WandSparkles`, Show sets
  `Layers`, Host your own library `BookOpen`.

### States

One rule set for every control; an entry below only names what it adds.

| State | Look |
|---|---|
| Rest | as the entry says |
| Hover | `--editor-accent-hover` ground on controls; `--row-hover` on list rows (I35). Never a border change alone |
| Pressed (a toggle that is on, the chosen segment) | `--toolbar-icon-active-bg` ground and border, `--toolbar-icon-active-fg` glyph and label; hover does not change it. The only "on" colour (I14). Built (0159 S2): `PRESSED` in `ui/looks.ts`; the Into slot caption, the fixture shape and tape toggles and the Test grid kind strip now use it |
| Active press (mouse down) | no extra look (no 1 px shift, no darkening) |
| Focus | 2 px `--editor-accent` ring, 1 px offset in the surface colour (`ring-offset-[var(--toolbar-bg)]` in bars, `--panel-bg` in the panel); **inset** (`ring-inset`) inside a scrolling list or strip, where an outer ring would be cut. `:focus-visible` only. No `outline-2`, no underline-only focus (I33) |
| Disabled | **Panel:** the control is removed, not disabled (design-system "Properties panel"). **Bars and rail:** `aria-disabled="true"`, 40 % opacity, the press does nothing, stays a Tab stop, the tooltip gives the reason. Never the `disabled` attribute on a button with a reason; never 50 % (I34) |
| Invalid | 1 px `--field-invalid` border plus 1 px inset shadow (2 px look), `aria-invalid`, and the validation chip; never colour alone |
| Armed | design-system "Armed action": `--destructive` fill (or `--armed-accent`), white glyph |
| Busy | the symbol becomes `LoaderCircle` spinning (`motion-safe`), label and width unchanged, `aria-busy` |

### Copy

Labels name the act in one or two words, sentence case. "…" (one character) after a label that
opens a file dialog or another view. The same act has the same word everywhere (OK, not Done; I39). Built (0159 S1): "Pen
probe…", "Add folder…", "Add source…"; the Share tab's Done is OK.
**No verbose surface** (customer 2026-11-27, Frage 418 A and 431 A, superseding the earlier
reading of design-system "No operating tips by default"): how to operate a control may be said in
its tooltip (or an info panel), and the canvas hint chips and the slider help line stay; what goes
is status and result chatter after an act: counts of what came out, "Done" lines, repeated words.
A label that names a control or a state stays.

## 1. Primary button

- **Anatomy:** symbol + label, centred. One per inline confirm, form or step: the act that keeps
  or makes something (Add, Save, Join, Connect, Trace, Apply).
- **Sizes:** 28 px high (36 coarse), padding 12 px, or a fixed width when it shares a row (76 px
  in a three-button confirm, 118 px in a pair, 244 px alone).
- **Tokens:** `--toolbar-icon-active-bg` ground and border, `--toolbar-icon-active-fg` symbol and
  label, 5 px radius, 14 px.
- **States:** hover `brightness-110`; focus ring with 1 px offset; disabled per "States" (bars),
  removed in the panel; busy.
- **Keyboard:** Enter and Space. In an inline form, Enter in a field presses it.
- **When:** the main act of a form or confirm; never two in one group. Not in the tool bars except
  a bar's finishing act (Trace, Apply of Tiling and Centre lines).
- **Component:** `components/ui/button.tsx` `Button variant="primary"`; shadcn's variants are
  gone. The class strings live in `components/ui/looks.ts` (`buttonClass`, `BUTTON_LOOK`) for the
  few elements that are not a `Button` (a Radix trigger, an armed confirm). Replaces I7.
- **Built:** 0159 S1 (Add page Add, Save of the File tab check, Tiling and Centre lines Apply,
  Trace and Recognise finish, Analyse, the Share tab's Join, Pair, Connect, the library Add and
  Save, New material Add, Crop, Take photo).

## 2. Secondary button

- **Anatomy:** symbol + label. Every other act: Cancel, Fit to content, Convert to paths, Export
  SVG..., the bar commands (Object to path, Linked clone, Bridge, Box from path).
- **Sizes:** as the primary button. In a bar: 28 px, 8 px padding.
- **Tokens:** transparent ground, 1 px `--field-border` (`--toolbar-icon` at 60 %), `--toolbar-icon`
  symbol and label, 5 px radius, 14 px. The design system's "Fit to content" look, now with a
  symbol.
- **States:** hover `--editor-accent-hover`; focus; disabled (bars); busy.
- **Keyboard:** Enter and Space; Cancel also on Escape in its group.
- **When:** every button that is not primary, destructive, a toggle or icon-only. A flat text
  button (no border) is not used (I3, I4).
- **Component:** `Button variant="secondary"` (the default). Replaces the 45 definitions of I5,
  the library CSS rules (I6) and `sketchButton` (I22).
- **Built:** 0159 S1. The bar commands (Object to path, Linked clone, Remove rounding, Stroke to
  path, Box from path, Recognise text, Bridge, Curve to Bézier, Trace, Pen probe…) are secondary
  bar buttons; the panel, Library, Share, Sketch and workbench copies use `Button` or
  `buttonClass`; the `index.css` library button rules are gone. A fixed width of a shared row is
  a minimum width, so a label with its symbol never overflows. `sketchButton` stays for the
  Sketch panel's list rows and pressed toggles (S2, S6).

## 3. Destructive button

- **Anatomy:** symbol (`Trash2`, `Eraser`) + label: Delete, Discard, Wipe.
- **Tokens:** white ground, 1 px `--destructive` border and label; focus or press fills it
  `--destructive` with a white label. Before any confirm it is a secondary button with a red label
  and the full-strength border (not 25 %, I8).
- **Behaviour:** design-system "Inline confirm" (400 ms guard, Cancel focused) or "Armed action".
- **Component:** `Button variant="destructive"`.
- **Built:** 0159 S1 (Discard, Wipe and Wipe history, Delete workspace, the Sketch Delete, the
  library token Remove).

## 4. Text link

- **Anatomy:** label only, `--editor-accent`, underline on hover and focus, 12 or 14 px as the
  text around it.
- **When:** only to open something outside the current view that is read, not done: a licence
  text, the library guide, an external page (`ExternalLink` after the label for an external one).
  "Clear all", "Select", "Show all" are acts and become small secondary buttons (I11).
- **Component:** `TextLink` (`components/ui/text-link.tsx`); `TEXT_LINK_CLASS` for a `<button>`
  that opens a page through the host (Licences, Example repository).
- **Built:** 0159 S1: Clear all (three places), Select, Show all / Show less are secondary
  buttons. Words inside a sentence that select ("3 runs" in the Output total) stay inline.

## 5. Icon button

- **Anatomy:** one 16 px glyph, centred, no label; tooltip and `aria-label` with the name.
- **Sizes:** 28 x 28 in bars and panel headers; 24 x 24 as a list-row action; 40 x 40 in the rail
  (with 20 px glyph). 36 x 36 coarse for the 28 and 24 px kinds (a row action grows with its row).
- **Looks:** **plain** (transparent, no border: bars' icon groups, list-row actions) and
  **outlined** (the secondary button's border: panel header actions such as Add format, Add page,
  Import formats..., the "Add font..." look). A panel header uses the outlined kind, so its buttons
  read as buttons next to the heading text (the plain "Add page" `Plus` of `PagesBlock.tsx:90`
  becomes outlined).
- **Tokens and states:** `--toolbar-icon` glyph, 5 px radius; hover, focus, disabled as in
  "States"; list-row actions may show on row hover and focus only (Inline delete confirm).
- **Keyboard:** Enter and Space; in a list, reached by Left and Right (design-system "List row").
- **Component:** `IconButton` (`components/ui/icon-button.tsx`, prop `look="plain" |
  "outlined"` and `size={24 | 28 | 40}`; `iconButtonClass` in `ui/looks.ts`). Replaces the
  ICON_BUTTON constants of I9 and `CommandButton`'s button part.
- **Built:** 0159 S1: the bar and panel icon-button constants are at 5 px; Add page is outlined,
  the Pages up and down are plain 28 px; the Fonts tag `+` is a 24 px outlined button with
  `Plus`; the library browser header and detail bar actions are outlined. Folding every constant
  into `IconButton` is left to S6 (radius and size pass).

## 6. Toggle button (replaces the switch)

The customer's rule: an on/off setting is a button that stays pressed, shown by the pressed state
and the colour, never a sliding switch (I12).

- **Anatomy:** symbol + label in the panel (a 244 x 28 row-wide button, symbol at the left, label
  left-aligned) (Frage 416 A, customer 2026-11-27); symbol + label in a bar where there is room, otherwise
  icon-only with the name as tooltip (Scale stroke width, Scale corner radius, Simulate pressure).
  The label is the name of the setting and does not change with the state ("Cuts last", not
  "Cuts last: on").
- **Sizes:** 28 px high (36 coarse); icon-only 28 x 28.
- **Tokens:** off = the secondary button (outlined in the panel) or the plain icon button (in an
  icon group of a bar); on = "Pressed" in "States": `--toolbar-icon-active-bg` ground and border,
  white symbol and label. The symbol may change with the state where the glyph pair already exists
  (`Link2` / `Unlink`, `Eye` / `EyeOff`); never the only cue.
- **States:** hover on the off state only; focus; disabled (bars); armed (the "Sync to my devices"
  off press keeps the Armed action: the pressed ground turns `--destructive`).
- **ARIA and keyboard:** `<button aria-pressed>`, Enter and Space toggle (a switch toggled on Space
  only; a toggle button takes both). After a mouse press the focus returns to the canvas, as the
  switches did (`onReturnFocus`). One Tab stop each.
- **When:** every boolean setting: the panel's former switches (Snapping, Grid, Cuts last, Skip
  inner shapes, Whole pieces only, Show, Add to quick selection, Sync to my devices, Expose, Laser
  mode, Motorised Z axis, Air assist, Focus pulse, Customizable, the export options), the bars'
  former switches (Scale stroke width, Scale corner radius, Face centre, Keep source, Join all,
  Simulate pressure, Join tips, Keep inner areas, extensions), Move frame only, and the option
  checks of the entry chips (Frage 420).
- **Not for:** two named states (Relative / Absolute, Rectangle / Circle): that is a segmented
  control (I13).
- **Component:** `ToggleButton` (`components/ui/toggle-button.tsx`, on Radix `Toggle`, props
  `look="panel" | "bar" | "icon"`, `tooltip`, `armed`, `extra`); `toggleClass` and `PRESSED` in
  `ui/looks.ts` for a list-row toggle or an entry-chip check that keeps its own keys.
  `ui/switch.tsx` and `ToolbarSwitch.tsx` are deleted; `CentreSwitch` wraps `ToggleButton`.
- **Built:** 0159 S2. Panel: Snapping and its targets and axes, Grid, Cuts last, Skip inner
  shapes, Whole pieces only, Show (format group), Add to quick selection, Sync to my devices
  (armed), Expose, Motorised Z axis, Air assist, Focus pulse, Customizable, the generator
  switches, the four export options, Fixed origin and Reference (Sketch). Bars: Scale stroke width
  (`Scaling`), Scale corner radius (`SquareRoundCorner`), Face centre, Keep source, Join all,
  Simulate pressure, Join tips, Keep inner areas, Laser mode, the extensions (icon-only), Motion
  compression. Rows: Move frame only (icon-only, `Frame`). Entry chips: Copy, Snap angle, From
  centre, Reference, Expose (Frage 420 A). Library into slot: Placed page, Real size. **Keyboard
  (Proposal):** in an entry chip Enter keeps committing the chip (accepted behaviour of `0073`),
  Space toggles the check; everywhere else Enter and Space toggle. Relative / Absolute of the Move
  chip is a two-item choice and moves to the segmented control in S5.

## 7. Segmented control and choice strip

- **Segmented control** (design-system "Segmented icon control (`ToggleGroup`)"): one bordered
  strip, items 1 px apart, the chosen item in "Pressed". Items show a symbol, or text where no
  symbol reads ("Spaced"), or both. 28 px high (32 for Fill type and the Pen bar, as specified),
  item width per group. `role="radiogroup"`, one Tab stop, arrows move and select, Home and End.
  In a bar the `bar` look (one ring, no lines between items).
  **Component:** `components/ui/toggle-group.tsx`. Folds `PanelRadioGroup`, `ModeSwitch`, the Add
  page Kind segments, `.library-kind`, the ShapeToolbar mode, the FixtureBody shape and the
  MoveEntryChip switch (I13, I16).
  **Built (0159 S5):** one look, `SEGMENT_STRIP` and `SEGMENT_ITEM` in `ui/looks.ts` (1 px bordered
  strip, items 1 px apart, chosen item "Pressed", inset focus ring), used by `PanelRadioGroup`
  (Grid Mode and Scale, Snapping "Grid snaps with"; Home and End added), `ModeSwitch`, the Add page
  Kind, `.library-kind`, the Polygon/Star mode, the fixture slot shape, the Test grid strips and the
  Move chip's Relative / Absolute (a radio group now, no `role="switch"`; arrows move and choose,
  F20). `ToggleGroup`'s bar look uses `--field-border` and can wrap (`wrap`, the Generators bar).
  **Built (0159, last pass):** the Origin corner is one 2 x 2 strip with its label beside the top
  row (F21); the Stroke Width profile is a `ToggleGroup` radio group (arrows move and choose, the
  path's preset checked, F19); the Tile bar's strips use the look (F45). Kept: the Fonts tag strip
  wraps into rows, as a choice strip of an open set of tags does (F19, no change).
- **Choice strip** (design-system "Choice strip (`PresetStrip`)"): up to about eight named choices
  in fixed rows, the font tag strip's look. **Component:** `panel/PresetStrip.tsx` (`DitherStrip`
  uses it).

## 8. Checkbox

- **Anatomy:** a 16 x 16 box, 1 px `--toolbar-icon` border, 3 px radius (the one exception to
  5 px: the box is too small), `Check` 12 px white on `--toolbar-icon-active-bg` when ticked, a
  2 px bar for "some".
- **When:** only to tick items of a list for a later act (which objects an operation holds, which
  parts a fixture job takes): the tick is part of the row, tri-state, and a row can be ticked
  without pressing it. An on/off setting is a toggle button.
- **ARIA and keyboard:** `role="checkbox"`, `aria-checked` true / false / mixed, Space ticks; in a
  list it is a cell of the row (Left / Right).
- **Component:** `Checkbox` (`components/ui/checkbox.tsx`; `checkboxClass` in `ui/looks.ts`).
  Folds the five custom ones (`OperationRow.tsx:80`, `FixtureJobParts.tsx:97`,
  `FixtureSetRow.tsx:154`, `FixtureRowItem.tsx:127`); the native checkboxes go (I23).
- **Built:** 0159 S2: the operation row tick, the fixture job parts, the fixture set "On this
  page" and fixture "On the bed" ticks and the Share tab's "Workspaces to sync" list use the one
  look (`--toolbar-icon-active-bg` fill, white `Check` or `Minus`). The option checks of the entry
  chips and the Sketch panel became toggle buttons (Frage 420 A).

## 9. Slider

- **Anatomy:** a 4 px track (`--toolbar-icon` at 25 %), the filled part `--editor-accent`, a
  14 px thumb (white, 2 px `--editor-accent` ring), the value field or readout beside it.
- **Sizes:** 28 px row; the hit target is the full row height.
- **States:** hover thumb ground `--editor-accent-hover`; focus = the 2 px ring with offset around
  the thumb (PlayBar's look); the value field beside it carries invalid.
- **ARIA and keyboard:** `role="slider"`, `aria-valuemin/max/now/text`; arrows step, Shift ten
  times coarser, Home / End.
- **When:** outside the panel (the bars' Ratio, the font preview's Size, the Play bar), and in the
  panel only for a picture preset's Strength (design-system "Plain fields, not sliders, in the
  panel"). The colour picker's area and hue are their own control.
- **Component:** `Slider` (Proposal, `components/ui/slider.tsx`). Folds `RatioSlider`,
  `SizeSlider`, PlayBar's slider and `AdjustStrength`'s slider (I17).
- **Built (0159 S5):** `ui/slider.tsx` `SliderMarks` draws the one track, fill and thumb (2 px ring
  everywhere, focus ring around the thumb, hover tint) and holds `SLIDER_THUMB_PX` and
  `SLIDER_INSET_PX`; the four sliders keep their own pointer and key code (Proposal: folding the
  behaviour too is a later refactor).

## 10. Number field

- **Value field** (panel, drag to set): design-system "Value field". **Component:**
  `panel/ValueField.tsx`.
- **Number field** (bars, entry chips, inline forms, machine settings): 28 px, white ground, 1 px
  `--field-border`, 5 px radius, label inside the left edge (12 px muted) where the group has no
  label of its own, value right-aligned 14 px tabular, unit inside the right edge, the "limited"
  tag before the unit. Focus: `--editor-accent` border plus 1 px inset. Invalid per "States". Enter
  commits, Escape restores, a press elsewhere restores (the entry-chip rule); arrows step, Shift
  coarse, Ctrl fine. No native `type="number"`, no steppers (I19).
  **Component:** `NumberField` (Proposal, `components/ui/number-field.tsx`). Folds both
  `EntryField`s, `BarNumberField` and the inline fields of I18.
  **Built (0159 S6, partly):** no browser steppers anywhere (`index.css`); `BarNumberField` pads
  its value by the label's measured width plus 8 px, so "Randomized" and its value never touch
  (W19). **Built (0159, fields pass):** `ui/number-field.tsx` `NumberField` draws the one look
  (`fieldClass` in `ui/looks.ts`: 1 px `--field-border`, focus border plus 1 px inset, invalid border
  plus 1 px inset), the measured label inside the left edge (`lib/fieldLabelWidth.ts`), the unit and
  the "limited" tag inside the right edge; both `EntryField`s, `BarNumberField`, the Align, Radius,
  Points, Bridge, Text size, Offset, Text on path, Transform, Gradient, Tiling and Width point fields
  use it and keep their own commit rules (Proposal, as the slider: folding the behaviour is a later
  refactor). A field whose label sits beside it in the bar (Radius, Points, Gap) keeps that label;
  the label goes inside where the field has none of its own. The machine settings' label-inside rows
  use `fieldFrameClass` / `FIELD_FRAME` (the same look on the frame). The Polygon/star Points and
  Bridge fields keep a native number input for their accepted arrow-key step (`numeric`, no
  steppers drawn); the Machine profile list's work-area fields are text fields now.

## 11. Text, search and rename field

- **Text field:** 28 px, white ground, 1 px `--field-border`, 5 px radius, 6 px padding, 14 px
  (`font-mono` 12 px for codes and keys). Focus and invalid as the number field.
- **Search field:** the text field with `Search` 16 px inside the left edge and a clear `X` icon
  button inside the right edge while it holds text. No browser cancel button. **Never**
  `ring-border` (I20).
- **Rename field:** replaces a row's name in place, the row's height minus 4 px (24 px in a 28 px
  row), 1 px `--editor-accent` border, text selected, Enter writes, Escape cancels, blur commits a
  valid name (design-system "Document formats library", "Rename").
- **Components:** `TextField`, `SearchField`, `RenameField` (Proposal, `components/ui/`). Fold
  I20, I21, `sketchInput` (I22), `.library-search`, `.library-source-form input`.
- **Built (0159 S6, partly):** every search and tag field has the 1 px `--field-border` (no
  `ring-border` left, F27); `.library-search` and the library source form fields are white with
  the field border and the focus border; the Sketch panel's field is white, 14 px (F29); the
  library source form is an inline form box. **Built (0159, fields pass):** `TextField`,
  `SearchField` (font search, glyph search and sample, library tab, library browser; the browser
  cancel button and the `.library-search` CSS are gone) and `RenameField` (operation, material,
  fixture job, format, layer, page, workspace, font tag and fixture slot rows: 24 px, 1 px accent
  border, 14 px) in `components/ui/`; the Share tab and Sketch panel fields use `FIELD_WIDE`, the
  pick-list filters `fieldClass`. Kept: `.library-source-form input` (CSS, already the field look).

## 12. Pick list and menu

- **Pick list** (design-system "Choice strip", last sentence): a closed row that opens an inline
  list in the panel's column, pushing what is under it; rows 28 px (44 with a second line), the
  chosen row in the List row "selected" look (`--value-fill` and the 3 px bar), never the pressed
  fill (I26). `role="listbox"`, arrows, Enter chooses, Escape closes. **Component:**
  `InlinePickList.tsx` (`MaterialPick`, `OutputPick`, the Add page machine list and
  `MachineProfileList` use it). No native `<select>` anywhere (I24). Built (0159 S5): the Print and
  cut Sheet is an `InlinePickList` (with an optional symbol left of its label, also used by Place
  on…); the Add page machine list's chosen-row look is unchanged (F23 partly).
- **Menu (Proposal):** a popup list is allowed only outside the panel, for a bar's preset or
  material choice (Trace presets, Test grid material), Radix `DropdownMenu` look: the tooltip's
  surface, 28 px items, `menuitemradio` with `Check`. Recorded here because the two exist (I25);
  no new one without the customer.

## 13. Bar, canvas chip and canvas symbol pill

- **Bar** (design-system "Switch (`ScaleStrokeSwitch`)", "Select bar", "Bar overflow"): 36 px
  rows (`min-h-9`), 8 px side padding, 8 px radius, `--toolbar-bg`, `--panel-elevation-shadow`,
  groups 12 px apart with the 20 px divider, row 2 by the chevron button. Every bar, including
  Crop (44 px today) and Play (40 px), uses it; `NodeToolbar`'s 12 px padding goes (I27).
  **Component:** `Bar` (Proposal, `components/ui/bar.tsx`, the pill and row 2).
  **Built (0159 S4, Frage 430 A: bars wrap, never past the canvas, no "More" menu):** pills at 8 px
  radius and 36 px rows (Crop and Play bar included); a group whose component draws nothing takes
  no room (no empty row, no divider after the last group; `BarGroup` in `TextBar.tsx`); groups
  shrink to the bar's width and wrap inside (ellipse fields); the Pencil bar uses the shared groups
  and row starts; the Generators bar's choice wraps inside its pill; **compact Select bar
  (Proposal):** Scale stroke width, Scale corner radius, Remove rounding, Object to path, Stroke to
  path, Linked clone, Box from path, Recognise text, Trace and the Tile buttons are icon-only with
  their name (and what they do) in the tooltip (`Button iconOnly tooltip`, `ToggleButton
  look="icon"`); Place on… shows `MapPin` left of its label; the Node bar's Sharp and Smooth show
  `Triangle` and `Circle`; canvas clone tags sit below the bars (z 15); the rail's scrolling cards
  take the wheel. **Built (0159, last pass, Proposal):** one anchor for every bar, left, right of
  the tool rail (F43); the bar area publishes its bars' live box (`hooks/useBarBox.ts`): clone
  edge chips stay 12 px below it (F42) and a page label whose place a bar covers moves 4 px below
  the bar, inside its frame (`pageLabelTop`); a new view keeps the page top 72 px down, so the
  label clears a one-row bar (`curvyo-ui-core` `DOCUMENT_TOP_CLEAR_PX`, F50); the Tile bar's
  strips, icon toggles and Variance button use the shared looks and tooltip (F45); the Test grid
  code carries the muted caption "Test", Passes fields show no "×" (F47); the Sketch rail is three
  cards (Select, shapes, Fillet), tooltips "<Tool> tool" (F49); Distribute "Gap" and "Grid gap"
  (F46). **Built (0159, Frage 433 B, customer 2026-11-28):** a kind group (Radius, Points, Ratio,
  the ellipse kind and arc fields, Curve, Randomized, the Text groups, Object to path, Box from
  path, Recognise text) shows only while every selected object is of its kind, so a mixed
  selection shows only what its kinds share (`lib/selectBarGroups.ts` `kindGroupShown`, over the
  session's `selection_kinds`); one Gap field between Distribute and Grid serves both; Distribute is
  its four buttons, Grid its button and Rows and Columns, labels inside the fields; Align is two
  groups of three (horizontal, vertical; Proposal) so the bar wraps between the axes. **Narrow
  class** (Proposal): the bar's usable width below 690 px (`NARROW_SELECT_BAR_PX`, the width at
  which Scale, Flip and Guide, Tile, Align and Distribute with its Gap fit in one row; an 800 px
  window with the panel open, 324 px, is in it): Align, Distribute and Gap are not shown with
  fewer than two selected objects.
- **Canvas chip** (readouts, entry chips, tags, hint chips): `--toolbar-bg`, `--toolbar-icon`,
  12 px text, 8 px radius, 6 px side padding (8 px with fields), shadow when it holds fields;
  `pointer-events: none` unless it holds fields (I28). Built (0159 S3): the hint chips, nudge
  readout, colour pick chip, drop badge, missing-font tags and fixture labels at 8 px; the
  fixture labels' 16 px tag is a disc; the Measure readout holds its mode ("Aligned") as its last
  line instead of a second chip. Badges (Copy, Lock, pick set) are 16 px
  discs ("Modifier badge").
- **Canvas symbol pill** (laser page, View: Operations; drawn by
  `curvyo-render-core/src/operation_symbol.rs`): **one pill per object** holding the symbols of
  every kind the object is in (Frage 419, recommended A): 22 px high, radius 11 (full), 3 px
  padding, symbols 16 px **centred** in 16 x 16 cells 4 px apart (so a pill of n symbols is
  `6 + 16n + 4(n − 1)` px wide), `--toolbar-bg` ground, 1 px `--lane-line` edge; the symbol's own
  1.5 px outline is drawn inside its cell (inset 0.75 px), so it does not shift the visual centre
  (I1). **Anchor (Frage 432 A, customer 2026-11-27):** centred over the object, its bottom edge
  6 px above the object's top edge, moved inside the viewport when it would leave it (Proposal);
  **while the object shows its own transform handles** (it is the one selected object) the pill
  takes the corner anchor (right of the top-right corner, mirrored to the top-left at the viewport
  edge). **A pill never lies over a transform handle** (lead decision, Proposal): it moves past
  every handle glyph it would touch, 4 px clear of it (`HANDLE_GAP_PX`, glyph sizes from
  `theme.rs`), so the corner pill sits right of the corner rotate handle; the members of a
  multi-selection keep their centred pills and move past the group box's handles; a pill with no
  place left on screen is not drawn. Collision skip, the 500 cap and the muted ink stay as in 13.3 and
  13.4; a pill counts as one item and collides as a whole.
  **Built:** 0159 S3 (`chips_list`, `pill_width`; `ChipItem.handles` set by
  `curvyo-editor-wasm` `look_overlay.rs` for the selected items).

## 14. Tab

- **Panel strip** (design-system "Properties panel: tabs"): 28 x 28 icon tabs in the 32 px strip,
  the selected tab in "Pressed", focus ring inset, `role="tablist"`, arrows move, the tab
  shortcuts. **Component:** `panel/PanelStrip.tsx` on `ui/tabs.tsx`. No second tab component;
  a sub-view switch inside a tab is a segmented control (the font preview's mode).

## 15. Tooltip

- **Look:** the canvas chip surface (`--toolbar-bg`, `--toolbar-icon`, 12 px, 8 px radius, 8 px
  by 4 px padding, shadow), max 260 px, an optional muted second line (state, shortcut). Text
  only, no controls, no focus (Proposal: one ground; today panel tooltips are white popovers and
  bar tooltips grey, I30).
- **Behaviour:** Radix `Tooltip`, 400 ms, opens on hover and on keyboard focus, `side` left in the
  panel, bottom in the bars, right past the rail; z above the overlay row. **Deliberate
  exceptions in the panel (0159, Proposal):** tooltips open above in the dense, side-by-side
  control groups where a left tooltip would cover the neighbouring cell: JogPad, MachineControls
  (with its Frame row and Send to machine), OutputGroup, JobList, PrintAndCut, and the image
  AdjustmentsGroup (with its preset, dither and adjust-step strips); the Layers tab row (LayerRow)
  opens below so it does not hide the neighbouring rows' names.
- **Content:** the control's name (and for an icon-only control exactly its `aria-label`), the
  reason when it is dimmed, the shortcut in parentheses. No operating hints.
- **Component:** `components/ui/tooltip.tsx` only. Native `title` attributes on controls and the 18
  own Radix tooltips go (I30); `title` stays only on truncated, non-interactive text.
- **Built (0159 S7, partly; Frage 431 A: a tooltip may say how to operate the control):**
  `TOOLTIP_CLASS` in `ui/tooltip.tsx` is the one look (canvas chip surface, 8 px, max 260 px); the
  shared `Tooltip`, the rail, the command buttons, the Import card and the bars' own Radix tooltips
  use it. **Built (0159, tooltip pass):** the native `title` attributes on controls, rows, chips
  and bars (about 90) are `Tooltip`s (left in the panel and its workbench tabs, below elsewhere;
  multi-line text keeps its line breaks); a `Button` takes `tooltip`. Kept: `title` on 25 truncated,
  non-interactive text lines (names, paths, meta lines), as this entry allows; `title` as a prop
  name of own components (`ErrorDialog`, `PaintRow`, `BlockHeader`, the Pen probe's `Rows`) is not
  the attribute.

## 16. Notice

Three kinds, all text only, no controls (except Back to live), never focused
(design-system "Canvas notice slot", "Action notice", "No notice after a success"):

| Kind | Where | Look |
|---|---|---|
| Canvas notice | the slot, centred at the bottom of the viewport | canvas chip surface, 12 px, max 480 px, `role="status"`, 3 s |
| Action notice | 4 px under the control that caused it (panel, Node bar), or beside the rail card | the same surface, max 360 px |
| Alert line | under the row it belongs to, in the panel's flow | 12 px `--field-invalid`, `TriangleAlert` 14 px, `role="alert"` |

A refusal uses the validation chip look and `role="alert"`, 8 s. **Built (0159 S7, partly):** every
notice surface at 8 px (canvas slot, action notices, Boolean notice on the chip surface, slot
replace notice); result notices carry no counts of what came out (customer 2026-11-27; the list is
in `specs/0159-ui-consistency/plan.md` S7). **Built (0159, notice pass):** `ui/notice.tsx`
`ActionNotice` (status on the canvas chip surface; `refusal` in the validation chip look with
`CircleAlert`) for the rail's Boolean notice, the Fonts tab notices, the Tiling bar's field,
instance and Apply notices, the Text bar's size notice, the Centre lines notice, the Layers skip
notice and the panel's field refusal chips (`EntryField`, Document size, format rename and form,
font tag rename); `AlertLine` (`TriangleAlert` 12 px, as the icon rule's 12 px line) for the
red lines in the panel's flow (Share tab and My devices, workspace, file, adjustments, pattern,
gradient, job list, library refusals; the `.library-refusal` CSS is gone). Kept: the canvas slot
(`CanvasNotice`), the slot replace notice (it holds its Replace button), the bar field messages
that shift to stay on screen (`useNoticeShift`: Arc, Radius, Align fields), the sketch chips'
inline refusals and the admin-change status line (a status, not an alert). **Component:** `CanvasNotice.tsx`
(slot), `ActionNotice` (Proposal: one component for the action notices of `BooleanNotice`,
`FontTabNotice`, the Tiling chip, the Centre lines notice, the Layers hint), `AlertLine`
(Proposal). Folds I31.

## 17. Panel section and panel row

- **Section heading:** 12 px semibold `--toolbar-icon`, 28 px row, heading actions as outlined icon
  buttons at the right (Add, Import, Export), 4 px apart.
- **Disclosure row:** 244 x 28, chevron 12 px, label 14 px, muted count at the right,
  `aria-expanded`.
- **Built (0159 S6, partly):** the Sketch Parameters tab has one "Parameters" heading (the panel
  header's), F36; the laser Document tab's read-only Work area sits on the fields' grid (label and
  value where Max feed and Max power hold theirs). **Built (0159, token pass):** one heading style
  (`SECTION_HEADING`, 12 px semibold: Extensions, Machine, Connection, Output, Fixtures, Machine
  controls, Snap targets, Snap axes, Parts, My devices, Recovery, New material); one label column
  (`PANEL_ROW`, `PANEL_LABEL`, `PANEL_CONTROL_PX` in `ui/looks.ts`: `StyleRow`, the Grid and Snapping
  fields, Pattern, Format form, Generator rows, the Print and cut readout). Kept: the machine
  settings keep the label inside the field (§10), the list captions "Runs first" and the lane
  headings of Operations stay 11 px muted (list meta), the Connector readouts keep 80 px (their
  words are longer than 60 px).
- **Panel row:** label column 60 px (14 px `--toolbar-icon`), control column 176 px, 28 px, 8 px
  apart; a full-width control (a toggle button, a secondary button) spans 244 px. Sections are
  separated by a 16 px gap and the 1 px `--toolbar-icon` at 25 % rule.
- **Inline form box:** "Surfaces" above; its last row holds Cancel (left, secondary, `X`) and the
  primary act (right), equal width, 8 px apart, Cancel focused when it is a confirm, the first
  field focused when it is a form. Built: 0159 S1 for Add page (`FORM_BOX` in
  `panel/AddPageGroup.tsx`, 8 px radius as a surface) and the library Add source and token forms.
- **Panel header subject:** the page-kind chip of the panel header is muted plain text with its
  glyph (no border), like the other tabs' subject (0159 S1, F11).

## 18. List row

design-system "List row": fixed height per list, hover `--row-hover`, selected `--value-fill` and
the 3 px `--accent` bar, focus ring inset, muted ink plus a word for off or undone, one Tab stop
with roving focus, row actions as 24 px plain icon buttons reached by Left / Right. Add
`--row-hover` (`--accent` at 8 %) to the design system's colour table (I35). The Library rows'
CSS (`index.css` `.library-row`) follows the same rule.

## 19. Dialog

There are no dialogs in the editing flow: a destructive or data-removing act asks with the Inline
confirm or the Armed action, in place. The one modal is `ErrorDialog.tsx` (a failed open or save the maker
must acknowledge, `specs/0001-project-file-foundation`); its look moves to the app's tokens: 8 px radius surface, title
14 px semibold, message 12 px, one primary `Button` "OK" with `Check` (I32). Built: 0159 S1. The
Pen probe (`specs/0158-pen-display`, a diagnostic sheet, not in the editing flow) takes the same
look: an 8 px surface with the panel elevation, 16 px inside the window over the dialog's dim,
title 14 px semibold, column headings in the section heading style, `Button`s with symbols (0159
F58).

## 20. Canvas handles and marks (Rust)

Node and handle glyphs, transform handles, parameter knobs, guides, marquee and the symbol pill
are drawn in `curvyo-render-core` in constant screen pixels; sizes and colours are the rows of
design-system "Spacing and sizing" and "Color tokens", values in
`curvyo-render-core/src/theme.rs`. A colour constant there that copies a DOM token names the
token in its doc comment (I2; built for the pill's ground and edge in 0159 S3). Hit targets are never smaller than the drawn glyph plus 4 px.
