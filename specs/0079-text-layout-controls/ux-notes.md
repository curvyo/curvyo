# 0079 Text layout controls: UX notes

Owner: ux-engineer, 2026-10-24. For `specification.md` in this folder. Rows of
`docs/design-system.md` are named in quotes. Slice 1 is specified in full
(§1 to §9); slices 2 to 4 are sketched in §10 and get their full notes before
each goes Ready. The design-system amendments of §11 go into the build PR of
slice 1.

## 0. Proposed changes to the specification

None blocks slice 1. The product owner may copy these into the criteria:

1. **Criterion 7, drag grid.** The spacing tracks are relative to Size, so a
   fixed grid is too coarse for 3 mm text and too fine for 200 mm text. A drag
   rounds to the smallest 1-2-5 step (0.01, 0.02, 0.05, 0.1, ... mm) that is at
   least the track's mm span divided by the field width, so dragged values land
   on round numbers. Arrow keys step 0.1 mm, Shift 1 mm, Ctrl 0.01 mm.
   Character rotation: drag and arrows 1°, Shift 15°, Ctrl 0.1°.
2. **Criterion 7, rotation by drag.** The track is −180° to 180° and a drag
   stops at the ends; only typed values wrap (270 becomes −90).
3. **Criterion 10, Auto.** While Line spacing is Auto, the value and unit are
   drawn in `--panel-muted-fg` (4.7:1 on the fill bar, 6.9:1 on white), and the
   value text reads "12.4 millimetres, automatic". The muted number names the
   state, so no "Auto" word is drawn.
4. **Criterion 13, different Sizes.** With subject texts of different Size,
   the track and the overshoot use the **smallest** Size, so a drag never
   clamps a text. A typed value larger than that is set on each text up to
   its own limit; when any text was limited: "Letter spacing limited on 1 of 3
   texts, to fit its size." (status, 3 s, under the field; at most 70
   characters with the longest field name).
5. **Criterion 6a, numbers.** The block is 140px in slice 1 and 208px with
   slice 4; the font list keeps 256px and 188px at 800 x 600 (§4).
6. **Reset.** The lead's brief asked for reset by double-click. The spec
   (criterion 7) and the "Value field" row make double-click open typing in
   every value field, and the reset slot with Ctrl+Backspace resets. These
   notes keep that: one gesture, one meaning, app-wide. Changing it would be a
   change to every value field and is not proposed.

## 1. Alignment group (criterion 1)

- `role="radiogroup"`, accessible name "Alignment". Four 28 x 28 buttons, 4px
  apart, 124px. Order and names: Align left, Align centre, Align right,
  Justify.
- **Glyphs**: Lucide `TextAlignStart`, `TextAlignCenter`, `TextAlignEnd`,
  `TextAlignJustify` (lucide-react 1.50 keeps `AlignLeft` and the others only
  as deprecated aliases of these; import the new names). 16px, 1.5px stroke,
  `currentColor`, `aria-hidden`.
- **States** exactly as the weight buttons of the "Style group" row: idle
  `--toolbar-icon`; hover `--editor-accent-hover`; pressed
  `--toolbar-icon-active-bg` / `-fg`, `aria-checked="true"`; mixed: none
  pressed. Never greyed: every alignment applies to every text. No transition.
- **Keys**: one Tab stop on the pressed button (none pressed: Align left);
  arrows, Home and End move focus only, no wrap; Enter or Space presses
  (manual activation, as the weight control: an arrow must not write a
  "Change text alignment" step). Escape returns focus to the canvas or the
  edit sink.
- **Pointer**: acts on pointer up; a mouse press returns focus to the canvas
  or, in edit mode, to the edit sink with the caret where it was. A press on
  the pressed button writes nothing.
- **Tooltip** (Radix, 400 ms, bottom, the bar tooltip surface): the name only
  ("Align centre"); second line "Mixed" when the subject texts differ and
  none is pressed. No shortcut (out of scope), no hint line.
- With no text selected and no edit mode the press sets the next text's
  alignment (criterion 33): the pressed state moves, nothing else happens.

## 2. Bar fit (criteria 1, 32)

- **Text bar**: Font and Size (316) | Style (196) | Alignment (124), each
  `|` a 1px 20px divider at 25 % with 12px gaps: 686px on one row.
- **1280 x 800** (overlay row about 804px): one row, 686px.
- **800 x 600, panel open** (overlay row 324px): three rows, 108px: Font and
  Size; Style; Alignment. Style and Alignment together need 345px and do not
  share a row. Groups wrap whole, left-aligned, no divider at a row start; the
  pill stays centred.
- **Select bar Text group**: Alignment is a group of its own after Style and
  before Object to path, under the same condition (the selection holds a
  text). At 800 x 600 Alignment and Object to path share the fifth row if they
  fit (124 + 25 + the button), so the bar stays 5 rows, 180px; else 6 rows,
  216px. Measure at build and record both bars in "Text bar" and "Select bar
  Text group".
- Fixed widths: no button changes size with its state.

## 3. Text layout block (criteria 6, 7)

Top to bottom inside the Fonts tab:

| Part | Size |
|---|---|
| Header row (unchanged; the subject line names the block's subject) | 24 + 8 |
| **Spacing**: Letter spacing, 4, Word spacing, 4, Line spacing | 92 |
| 8 | 8 |
| **Turn**: Character rotation | 28 |
| Space below the block | 12 |
| Tag strip, search row, list (`0075`) | as today |

- Slice 1 block: 92 + 8 + 28 + 12 = **140px** including the space below
  it. Each field is the "Value field" (244 x 28) with its label
  inside: "Letter spacing", "Word spacing", "Line spacing", "Character
  rotation". The longest label (about 122px at 14px) and the longest value
  ("−179.5°", "−12.25 mm", about 70px) leave a 20px gap before the reset
  slot; the labels are not shortened.
- The two groups (lengths along the line; the glyph's own turn) are told
  apart by the 8px gap, not by headings. The 12px below the block, wider
  than the 8px inside it, separates the selected text's settings from the
  font library without a rule; it goes with the block.
- No section title, no fold, no "Layout" caption: the four labels name
  everything.
- **Tab order** in the tab: Letter spacing, Word spacing, Line spacing,
  Character rotation, then the tag strip, search field, list.

## 4. Fit of the Fonts tab (criterion 6a)

Panel viewport at 800 x 600 about 532px; `0075` §A9 leaves the list 396px with
a 2-row tag strip, 332px with the 4-row cap.

| State | List at 800 x 600, 2-row strip | 4-row strip | 1280 x 800 |
|---|---|---|---|
| No text (no block) | 396px, 7 family rows | 332px | 596px |
| Slice 1 block (140) | 256px, a block header and 4 rows | 192px, 3 rows | 456px |
| Slices 1 and 4 block (208, §10.3) | 188px, 3 rows | **124px**, 2 rows | 388px |

The 120px minimum holds in every state; with the 4-row strip and slice 4 it
holds by 4px. The block never scrolls on its own; only the list scrolls.

## 5. Appearing and going (criteria 6, 6a)

- In the tree when at least one text is selected or a text is in edit mode;
  else not in the tree. In the Text tool before the first click no text
  exists, so no block; the block appears when the new text enters edit mode.
- Instant: no height animation, no fade, no reserved space (the rule of
  "Hidden, not disabled" for rows that come and go).
- **Anchoring**: the list keeps its first fully visible family row at its
  screen position in the same frame (`0075` UX §3); the scroll offset is
  corrected, never the rows.
- **Focus**: never moved by the block appearing. If focus is in the block
  when it goes (Ctrl+Z removes the text, or a canvas click deselects), focus
  goes to the canvas, never to `body`.
- A field mid-drag or mid-typing when the subject changes: the drag is
  cancelled as by Escape (nothing written), typing is discarded.

## 6. The four fields (criteria 7 to 13)

All the "Value field" behaviour of `0047`: drag inside and beyond the field,
click sets, double-click, Enter, F2 or a digit types, Escape restores,
Ctrl+Backspace resets, reset slot, two-line tooltip, "coarse" and "fine"
during a drag.

| Field | Display | Track at Size `h` | Mapping | Reset slot shown |
|---|---|---|---|---|
| Letter spacing | mm, up to 2 decimals trimmed, U+2212 minus | −0.25 h to 1 h | linear | value ≠ 0 or Mixed |
| Word spacing | mm, as above | −0.5 h to 2 h | linear | value ≠ 0 or Mixed |
| Line spacing | mm; muted while Auto (§0.3) | 0 to 3 h | linear | set (not Auto) or Mixed |
| Character rotation | degrees, 1 decimal trimmed, "°" with no space | −180° to 180° | linear | value ≠ 0 or Mixed |

- **Signed fill bar** (new, §11.3). For a track that crosses 0 the bar is
  drawn from the zero position to the value, the 2px `--value-edge` line at
  the value's end; a 1px zero mark (`--toolbar-icon` at 40 %, full field
  height less 6px) shows where 0 is, so a click near it is a deliberate one.
  Line spacing does not cross 0 and draws the normal bar.
- **Overshoot**: spacing fields continue linearly past either end up to the
  typed limit (the bar stays full to that end); rotation does not overshoot.
- **Typed entry**: units of `0052` for lengths ("2", "2 mm", "0.1 in");
  "Enter a number" for anything else. Out of range: set to the limit with the
  spec's notice, under the field (status, 3 s).
- **Semantics**: `aria-label` "Letter spacing" etc.; `aria-valuemin` and
  `-max` the track ends in mm (degrees); `aria-valuetext` "1.5 millimetres",
  "−12 degrees", "Mixed". Names in `lib/sliderNames.ts`.
- **Tooltip line 1**: "Letter spacing: 1.5 mm", "Line spacing: 12.4 mm
  (Auto)", "Character rotation: −12°", "Mixed". Line 2 is the existing gesture
  line of every value field, unchanged.
- **Mixed** (criterion 13): "Mixed", no bar, the zero mark stays (it is
  part of the scale), reset icon shown, drag absolute.
- **Live preview**: the canvas follows the latest value once per animation
  frame; pending values are replaced, never queued, so a slow frame drops
  intermediate values instead of lagging. Release writes one step (criterion
  12); Escape during the drag restores the canvas from the stored values.
- **Edit mode** (criterion 14): a mouse change returns focus to the edit sink
  with the caret where it was; a key change keeps focus in the field; Escape
  in the field returns to the edit sink. The caret and the edit selection are
  redrawn with the preview, so they never sit on stale positions.

## 7. Empty and error states

- No subject: no block (§5); the Alignment group sets the next text.
- Missing face (criterion 36): the fields work and show the stored values;
  the Font button already marks the face missing, nothing more here.
- A text whose layout fails to compute keeps its last drawn state; no field
  shows an error (the layout is total for every value in range).

## 8. Undo and large documents

- One step per drag, click, typed value, reset or alignment press, whatever
  the number of texts; Undo restores every text and the fields show the
  restored values at once.
- Mixed, pressed and track values come from one pass over the subject texts
  per selection or document change, not per render or hover.
- The 50 ms frame budget of criterion 12 applies to the edited texts only;
  with many selected texts the preview relays out only the subjects.

## 9. Visual check at build

Screenshots for the review, light and dark: Text bar at 800 x 600 and 1280 x
800 with a text selected; Fonts tab at 800 x 600 with one text (values at
default), with letter spacing −1.2 mm (signed bar left of zero) and with three
texts Mixed; Line spacing Auto and set.

## 10. Sketch of slices 2 to 4

### 10.1 Slice 2: Taper (criterion 16)

- "Bar number field" like the Select bar Curve field: 99px, "Taper" inside
  left, "%" inside right, one decimal trimmed, Mixed placeholder; a group of
  its own after Alignment with the usual divider. No glyph.
- Text bar 810px: at 1280 x 800 (about 804px) it wraps to two rows unless the
  measured overlay row is wider; the wrap is accepted (whole groups), the
  controls are not narrowed. At 800 x 600 Taper shares the Alignment row
  (124 + 25 + 99 = 248px): still 3 rows.

### 10.2 Slice 3: Writing mode and Glyph orientation (criteria 20, 21)

- Two `radiogroup`s of three 28 x 28 buttons (92px each), the Alignment
  group's look, keys and tooltips. Greyed orientation buttons as the Style
  group (`aria-disabled`, 40 %, tooltip line 2 "Only for vertical text").
- **Custom glyphs** (16px, 1.5px stroke, Lucide grid and caps): Horizontal:
  three horizontal lines, the last shorter, an arrowhead at the right end of
  the first; Vertical, right to left: three vertical lines, the leftmost
  shorter, an arrowhead at the bottom of the rightmost; Vertical, left to
  right: its mirror. Upright: a Source Sans 3 cap "A" upright over a second
  one, with a down arrow beside; Sideways: the same "A"s turned 90° clockwise;
  Auto: a turned "A" over an upright CJK box glyph. Drawn as SVG paths at
  build, reviewed at 1x.
- Text bar about 1044px: two rows at 1280 x 800, four rows (144px) at 800 x
  600 (writing mode and orientation share a row: 209px).

### 10.3 Slice 4: Kerning (criteria 26 to 29)

- A third group in the block after Turn: 8px, Horizontal kerning, 4,
  Vertical kerning: +68px, block 208px (§4). Signed bars.
- In edit mode the fields follow the caret; outside edit mode they show the
  common kern and the reset clears all kerns ("Clear kerning").
- Alt+arrows: the keyboard nudge readout look ("Kern +0.3, 0.0 mm", x then y,
  16px right of and below the caret, gone 800 ms after the last key).
  Proposal for criterion 29; no canvas mark on kerned characters.

## 11. Design-system amendments (build PR of slice 1)

1. "Hidden, not disabled": record the text layout block (`0079`, customer
   decision LAYOUT-PLACE B, 2026-10-23) as the exception of a different kind:
   a panel section that comes and goes, limited to this block, with the
   customer's reason (the sliders are not there when there is no text).
2. "Properties panel: Fonts tab" Layout: replace "no section that comes and
   goes" by a reference to that exception, and add the block of §3 above the tag strip, with the fit table of §4.
3. "Value field": add the signed bar and zero mark (§6) for tracks crossing
   0, as the one exception to "no tick marks"; add the 1-2-5 drag grid for
   tracks relative to Size and the muted Auto value.
4. New row "Alignment group" (§1); "Text bar" and "Select bar Text group":
   the third group and the measured rows (§2).
