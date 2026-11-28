# 0164 UI consistency: visual walkthrough

Status: Draft (UX, 2026-11-27). Browser companion to [`inventory.md`](inventory.md) (code scan);
targets refer to [`docs/ui-elements.md`](../../docs/ui-elements.md) (`§n` = its section).

## Method

- Branch `story/ui-consistency` at `e26524df`, wasm built with `npm run build:wasm`, Vite dev
  server, Chromium 1.65 (playwright-core) headless, deviceScaleFactor 1, 1440 x 900 and 800 x 640.
- Visited: the opening screen (there is no separate start screen; the app opens on an Untitled
  document, Document tab), drawing, laser and sketch pages, every tool of the drawing rail and its
  bar, the Select bar for rectangle, ellipse, star, path, text and a mixed selection, Node, Pen,
  Polygon/star, Text, Pencil, Offset, Measure, Test grid, Generators (Box), Tile, the Boolean card
  tooltip, every right panel tab of all three page kinds (File, Document, Style, Operations,
  Materials, Fixtures, Layers, History, Fonts, Library and library browser, Snapping, Grid and
  rulers, Sketches, Parameters, Share), the inline flows (Add page, New and Close confirm, New
  material), the Pen probe, the paste and export notices, canvas chrome (selection box, handles,
  move readout, measure readout, laser operation chips, page labels, rulers, clone tags).
- Not reached: machine controls (no machine connected, no tab appears), Trace and image
  adjustments (need an imported bitmap), Open (native file dialog).
- **Dark theme: does not exist** (`docs/design-system.md`: one light theme; `index.css` `.dark`
  holds only unused shadcn defaults). Every finding below is light theme.
- Focus: a script focused each visible control after a key press and read `:focus-visible` and
  the computed outline/box-shadow on 13 tab states (about 1 000 controls). Every control showed a
  ring except the two focusable containers in W48.
- Measured button variants over all visited states (height, radius, border, content): heights
  16, 20, 24, 28, 32, 40 px; radii 0, 4, 5, 6, 7, 8 px; 167 borderless text-only buttons in
  bars and panels. No shared button component: `ui/button.tsx` is imported by one file, the
  inventory counts 437 raw `<button>` sites.
- Screenshots: `screens/NN-*.png` (crops, 1x). Severity: high = looks broken, unreachable or
  not recognisable as a control; medium = visible inconsistency a user notices; low = polish.

## Summary

| Severity | Count |
|---|---|
| High | 14 |
| Medium | 18 |
| Low | 18 |
| Total | 50 |

IDs are stable; W27 and W51 were dropped during the pass (not reproducible).

## Cross-cutting

**W1 (high) No single button look.** `01`, `03`, `15`, `18`, `34`. Six heights and six radii
(see Method); in one bar `07` (Node) "Close path" is borderless text, "Sharp"/"Smooth" are
outlined text, "Find centre lines..." and "Edit width" are outlined with symbol. Target: §1 to §5
only; every button is one of Primary, Secondary, Destructive, Icon, Toggle.

**W2 (high) Borderless text actions read as labels.** `03`, `04`, `05`, `06`, `07`, `15`, `34`,
`36`. Select bar "Remove rounding", "Object to path", "Stroke to path", "Linked clone", "Box from
path", "Recognise text", "Place on...", "Bridge"; Node bar "Close path"; Add page "Cancel" and
"Add"; Sketch "Add parameter". Target: §2 Secondary (28 px, 1 px ring, symbol + label), the
confirming one §1 Primary.

**W3 (high) Text-only buttons without symbol in the panel.** `18`, `16`, `13`, `19`, `24`, `25`.
File tab New, Open..., Save, Close, Template, Export SVG..., Pen probe...; Document "Fit to
content", "Add registration marks"; generator "Convert to paths"; File confirm Cancel, Discard,
Save; Snapping targets and Grid Mode/Scale choices; Pen probe Clear, Copy report, Close. Target:
§0 Icon rule, symbol before every label (Frage 453).

**W4 (high) Sliding switches everywhere, in three sizes.** `01`, `11`, `14`, `17`, `18`, `24`,
`25`, `27`, `28`, `34`. 32 x 18 (Select bar Scale stroke and Scale corners, Extensions, Cuts last,
Laser mode, Focus pulse, Motorised Z axis, Air assist, Snapping, Grid, Simulate pressure, Keep
source, Join all), about 24 x 14 (SVG export options, All formats "Show"), and the unlabelled
switch in every Pages row ("Move frame only", visible name none). "All formats" pairs the switch
with a redundant "Off" text. Target: §6 Toggle button, pressed = accent ground (Frage 452).

**W5 (medium) Switch position varies in one tab.** `27`. Laser mode: label indented 4 px, switch
at the right edge; Focus pulse: switch directly after the label; Motorised Z axis and Air assist:
right edge again. Target: §17 panel row, one control column.

**W6 (high) Two "selected" colours.** `33`, `11`, `18`. Solid accent with white text (Paper,
Unit, Orientation, Snapping, Grid Mode, Text align, Tile Gap/Clones, Pen modes) versus pale blue
with dark text (Test grid kind strip and Power/Speed/Passes, Pencil Soft/Linear/Firm, the File
tab Template button after a press). The pale one also equals the hover colour, so "selected" and
"hovered" look the same there. Target: §7 one active look (accent ground, white content).

**W7 (medium) Five constructions of segmented control.** `16`, `24`, `25`, `15`, `14`, `34`,
`09`, `23`. Joined with grey inactive ground (Paper A0 to A6), joined transparent (Unit,
Join, Cap, History Document/Object), separate buttons with gaps (Snapping targets, Grid
Mode/Scale), borderless with only the active pill (Add page Kind, Tile Gap/Pitch and
Clones/Copies, Polygon/Star), a small 12 px variant with a 1.5 px ring (Bridge Lettering/Stencil).
Fonts filter uses a two-row joined grid of unequal cells. Target: §7.

**W8 (high) Four slider looks.** `20`, `11`, `13`, `37`. Value field with fill bar (Opacity,
Width, Smoothing, generator sizes; the value sits 30 px from the right edge while number fields
are flush right), colour hue track with ring thumb, the library browser "Size" thin track with a
solid blue knob and a detached "96 px" readout, and the picker area. Target: §9, one slider
(value field look) plus the colour picker as its own kind.

**W9 (medium) Four label placements in number fields.** `03`, `04`, `10`, `27`, `33`, `34`.
Outside left (Radius, Points, Columns, Rows, Gap, Width/Height in Document), inside left at 14 px
(Text Size, Max feed, Inner width), inside top-left as an 11 px tag (Start, End, Curve, Rounded,
Min, Max, Count, Cell size, Passes), unit outside the field (Bridge "Width [1] mm", "Reach [5]
mm"). Target: §10, one placement per surface (bar, panel).

**W10 (medium) Two fields called "Gap" in one bar.** `06`. Distribute gap (row 1) and Arrange in
a grid gap (row 2) on a mixed selection. Target: name by group ("Spacing", "Grid gap") or one
group label.

**W46 (medium) Tooltips carry operating instructions.** Select tool: "Drag a box. Shift: add.
Ctrl: remove. Alt: invert, lasso, cycle / Arrows nudge 1 mm ...", Opacity: "Click to set,
double-click to type, drag to change. Shift: coarse. Ctrl: fine. Ctrl+Backspace: reset." This
conflicts with the no-hint rule (Frage 458). Target: §15 name + shortcut only.

## Canvas chrome

**W11 (high) Laser operation chips: square, off centre, detached.** `30`, `31`. Each kind is its
own 22 x 22 px square with a 1 px `--lane-line` edge and **no corner radius** (the 0101 UX notes
specify 4 px). Pixel read: the Score dots start 1 px from the left edge and end 2 px from the
right, and sit 1 px above the chip's centre line. The chip row floats 10 px right of the bounding
box's top-right corner, so on a rectangle it reads as a stray glyph, not as a label of the object.
Target: §13 Canvas symbol pill: one pill per object (radius 11, symbols centred on both axes),
anchor per Frage 455 and Frage 459.

**W12 (medium) Page label hidden under the Select bar.** `01`. On the default view the label
"Page 1 Drawing" sits at y 35, the bar covers y 36 to 108; only the top half of the letters shows.
Target: default view keeps the page top 12 px below the bar, or the label moves inside the
frame's top-left.

**W13 (high) Select bar has an empty second row.** `01`, `03`. With nothing selected the bar is
72 px high: the last group is 0 px wide and wraps to row 2 (measured: 5 groups at y 36, one empty
group at y 72). Target: empty groups are not rendered; one 36 px row.

**W14 (high) Select bar covers the selection.** `06`, `40`. A mixed selection makes four rows at
1440 (bar bottom y 180) and five at 800; the top resize and rotate handles are under the bar.
Target: Frage 457.

**W15 (high) Bars overflow the canvas at 800 px.** `40`, `42`. The ellipse field row sticks out
past the bar's right edge ("Rounded 0.00" over the panel edge); the Generators bar runs under the
panel and "QR code" / "Rounded box" print over "Cuts last". Target: §13 a bar never extends past
the canvas; wrap whole groups or overflow into a menu (Frage 457).

**W16 (medium) Dangling dividers.** `03`, `44`, `06`. A divider after the last group ("Box from
path |"), and at 800 every wrapped Pencil row starts with a divider. Target: dividers only
between two groups on the same row.

**W17 (high) Clone tag drawn over the Select bar.** `43`. The "Original" clone tag covers the
Distribute buttons at 800. Target: canvas tags and chips below every bar in z-order.

**W18 (medium) Two bar anchors.** `03`, `07`, `10`, `12`. The Select bar is left-anchored and as
wide as the canvas; the Node, Pen, Text, Polygon/star, Pencil, Generators and Tile bars are
content-wide and centred over the page. Target: §13 one anchor (Proposal: centred, content-wide).

**W19 (medium) Label touches value.** `05`. "Randomized0.00" (no gap), "Rounded 0.00" with 4 px.
Target: field width from label + 8 px + value + unit.

**W20 (low) Polygon/star bar field has no field ground.** `09`. "New: Points 6": the field is
grey on grey, no white ground or ring, unlike every other bar field; "New:" is a hint-like
prefix. Target: §10 bar number field.

**W22 (medium) Tile bar mixes six looks.** `14`, `41`. Grid/radial segmented with ring,
borderless Gap/Pitch, ungrouped row and column direction icons, "Variance" text + symbol,
borderless Clones/Copies, Keep source switch, Primary "Apply". At 800 it becomes a nine-row
column over the canvas. Target: §6, §7, §13.

**W23 (low) Test grid bar.** `33`. "40BYO" monospace code without a label; "Passes 1 x" where
the "x" suffix reads as a clear button. Target: name the code or drop it; suffix "passes".

**W21 (low) Page label toggles.** `32`. Operations view and Toolpath view next to the laser page
label are 24 px with a 7 px radius, the only buttons of that size and radius. Target: §5 icon
button 24 px, radius 6.

## Tool rail

**W24 (high) Rail cut off at 640 px height.** `45`. Column A ends below the window: Generators and
Import (laser: Test grid too) cannot be reached and do not scroll. Target: rail scrolls inside its
column, or column A reflows into column B below 720 px (Proposal, safe default: scroll).

**W25 (low) Sketch rail is one long card.** `35`. Constraints sit in one card with internal
dividers (button pitch jumps 44 to 49 px at each divider); the drawing rail uses separate cards
8 px apart. Target: separate cards, as the drawing rail.

**W26 (low) Sketch tool names lack shortcuts.** `35`. "Rectangle", "Coincident" versus
"Rectangle tool (R)" on the drawing rail. Target: same name pattern; shortcut where one exists.

## Right panel

**W28 (medium) Panel header page-kind chip looks like a button.** `38`, `16`. On Document,
Style, Layers, Snapping, Grid, Materials, Fixtures, Sketches, Parameters the header shows a
bordered 28 px "Drawing"/"Laser"/"Sketch" chip; it is a static `span role=img` (no press, cursor
auto). File, History, Fonts, Library and Share show plain muted text there instead. Target:
plain text with the kind symbol, no ring, on every tab.

**W29 (high) "Add page" is a bare glyph.** `16`, `17`. A 28 px "+" with no ground and no ring,
next to equally bare up/down arrows; "All formats" has the same "+" with a ring; Materials uses
"+ New material" outlined with label. Target: §2 Secondary "Add page" with Plus, or §5 icon
button with ring; up/down as §5.

**W30 (high) Add page inline form.** `15`. Kind: three borderless choices, only the active one
has a ground; "Cancel" and "Add" plain text, "Add" not primary, both right-aligned with no
ring. The New material form (`29`) already does it right (outlined Cancel, filled Add). Target:
Kind as §7 segmented with symbols; Cancel §2 with X; Add §1 with Plus.

**W52 (medium) Three inline-confirm layouts.** `15`, `19`, `29`. File New/Close: Cancel,
Discard, Save content-wide with unequal widths; Add page: text buttons right-aligned; New
material: two half-width buttons. Target: §19 one inline confirm layout.

**W31 (medium) Three section heading styles.** `18`, `24`, `26`, `27`. 12 px semibold ("Paper",
"Pages", "Background", "SVG export", "Marking", "Output"), 14 px regular ("Extensions",
"Machine", "Connection", "Snap targets", "Grid snaps with"), 14 px semibold ("My devices").
Target: §17 one heading style.

**W32 (medium) Row grid differs per tab.** `16`, `24`, `27`. Document: label column 68 px, fields
from x 112; laser Machine: full-width fields with the label inside; Snapping Capture/Release:
label left, short field right-aligned. Target: §17 one label column per panel.

**W33 (medium) Origin corner as loose icons.** `27`, `25`. Four 28 px icons in a 2 x 2 block at the
right edge, the label vertically centred against both rows; no ring around the group. Target:
§7 radio grid with a group ring, in the control column.

**W34 (medium) Operations list rows.** `28`. A checkbox square at the row start; the fold chevron
is at x 284 on the Score row and x 256 on the Engrave row; action icons 24 px while other panel
icon buttons are 28 px. Target: §18 list row with a fixed trailing column; §8 checkbox (Frage
420).

**W35 (low) Operations tab details.** `28`. "Cutting" heading with only "Fixture jobs (0)" under
it; disabled "Export job file" is a grey filled block, while other disabled buttons fade. Target:
§0 States, one disabled look.

**W37 (medium) Share tab disabled looks.** `26`. "Add device" disabled = pale filled blue;
"Invite person" and "Restore with passkey" disabled = faded outline; "Join..." full width,
"Add device" content-wide. Target: §0 States; one width rule per button kind.

**W43 (medium) Stroke Profile has no active segment.** `20`. Profile shows four segments none of
which is pressed for a plain stroke; every other strip shows its value. Target: "Uniform" active.

**W44 (low) Generator card crowds the Style tab.** `13`. Generator fields sit above Stroke with no
heading or hairline before Stroke; "Convert to paths" is a text-only outline button 4 px above
"Stroke". Target: §17 section with heading and hairline.

**W38 (low) Library browser header.** `37`. Items/Sets segmented, "Filters" outlined text +
symbol, Size slider (W8), expand and close as bare glyphs. Target: §5, §7, §9.

**W39 (low) Fonts tab controls.** `23`. Tag chips 20 px with an "x", the add-tag "+" 20 px with a
ring, star toggles without ring, search field 28 px beside two 28 px ringed icon buttons. Target:
§5 sizes; tag chip one height.

**W40 (low) Layers tab.** `21`. Default names carry random ids ("Path o6bf5k"); the selected row
ground stops 22 px before the panel edge; the row tooltip opens over the tab strip. Target: §18.

**W41 (low) History tab.** Not exported. "Wipe object history" is a
full-width destructive outline pinned to the panel bottom, far from the list it acts on. Target:
§3 destructive button under the list.

**W42 (low) Sketches tab empty.** `35`/`36`. "Sketches" shows a blank panel with no empty-state
line; Parameters shows the heading twice ("Parameters" title and "Parameters" sub-heading).
Target: §17 empty state line; one heading.

**W45 (low) Template button state.** `18`. After a press "Template" turns pale blue with no
other change; it is unclear whether it is armed, pressed or hovered. Target: §6 if it is a
toggle, else no persistent tint.

**W36 (low) Narrow "New material" button.** Not exported (Materials tab before a press). 244 px wide in a 256 px
column where other full-width panel buttons are 256 px. Target: §2 full width in the panel.

## Overlays, dialogs, notices

**W47 (low) Pen probe is a raw full-window sheet.** Not exported (developer tool). Text-only
28 px Clear, Copy report, Close; no dialog surface or title bar. Target: §19 dialog.

**W48 (low) Focus without ring on two containers.** The Pages list and the panel body take focus
(`:focus-visible` true) with no outline. Target: §0 States, ring on every focus stop.

**W49 (low) Notices.** Paste and export notices are centred at the canvas bottom, 12 px text on a
grey chip; their radius matches neither the bars (8 px) nor the operation chips. Target: §16.

**W50 (low) Measure readout splits into two chips.** Measure drag shows the readout chip and,
8 px below, a separate "Aligned" chip of a different width. Target: one readout chip with the
state as a third line.

## Questions for the customer

Frage 457 (BAR-NARROW): What a context bar does when its groups do not fit in one row
- A: wrap whole groups into more rows (as now), but never past the canvas; the bar may cover
  handles.
- B: one row; groups that do not fit go into a "More" menu at the bar's end.
- C: A up to two rows, then B.
Recommendation: C (keeps the common case visible, never covers more than 72 px). Default: C.

Frage 458 (TIP-TOOLTIPS): Tool and slider tooltips with operating instructions (W46)
- A: keep them (accepted behaviour).
- B: tooltips show name and shortcut only; the instructions go.
- C: B, and the instructions move into the documentation.
Recommendation: C (fits the no-hint rule, nothing is lost). Default: A until answered. Same
family as Frage 454.

Frage 459 (PILL-ANCHOR): Where the operation pill sits on the laser canvas (W11)
- A: centred above the object, 6 px over its top edge.
- B: at the top-right corner as now, only the look changes.
- C: centred inside the object when it fits, else A.
Recommendation: A ("centred" as asked, never over the artwork). Default: A.
