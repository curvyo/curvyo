# 0124 Gradient fill: UX notes

ux-engineer, 2026-11-18. Written from `specification.md` (criteria 1 to 26, Fragen 232 to 236 as decided),
the pattern `specs/0071-pattern-fill/ux-notes.md` (sections 1, 2, 3 and the "kept when switched away"
rule), `docs/design-system.md` ("Properties panel: Style section": rows, Text field, Swatch, Eyedropper,
Colour picker, `ToggleGroup` item; "Transform entry chip", "Parameter handle", "Position handle entry
box", "Casing over artwork", "Hidden, not disabled", "No popups in the properties panel", "No operating
tips") and the code `FillSection.tsx`, `PaintRow.tsx`, `ColourBlock.tsx`, `ColourPicker.tsx`,
`EntryField.tsx`, `styleOptions.tsx`. Drafts: `docs/design-drafts/2026-11-gradient/` (three screens,
HTML and PNG). Customer rules applied: no invented colours (the swatches and handles show the stop
colours and nothing else), symbol-only buttons with a tooltip, units inside the fields, plain fields and
no sliders (the 0017 colour picker is the one existing slider pair and stays), editing in place on the
shape, no new tool, no hint labels. **(Proposal)** items are vetoable at the demo; section 13 lists
them as P1 to P18 with their defaults. "Ink" is `--toolbar-icon`.

## 0. What is reused

| Part | Reused from | Change |
|---|---|---|
| Fill paint switch | `ToggleGroup` with `FILL_KIND_OPTIONS` (`styleOptions.tsx`) | a fourth item, Gradient |
| Colour row | `ColourBlock` (swatch, eyedropper, hex `TextField`) | one per stop, narrower hex, Position and Remove after it |
| Colour picker | `ColourPicker` (area 244 x 96, hue 244 x 12) | shown under the active stop row; inside the entry box at 232px |
| Number fields | the bar number field of `0111` (`EntryField` look: label inside left, unit inside right, no bar) | Position, Angle, Radius |
| Entry box | "Position handle entry box" surface (the Transform entry chip) | two variants (section 7) |
| Handles | "Parameter handle" metrics and the casing rule | colour-filled dots on the gradient line (section 6) |
| Skip line | the `0076` / `0071` style skip notice 4px under the Fill title | the image and group wording |
| Readout | the "Live transform readout" chip at the pointer | angle, radius, position during a drag |

## 1. Where it lives

Properties > Style > **Fill**, the section that holds Solid and Pattern today. No rail tool, no top bar
control, no new tab, no popup, no shortcut (criterion 1). The Fill title row is the anchor: it never
moves when the rows under it come and go (0071 section 1).

## 2. The Fill paint switch (criteria 1, 2)

- Four 44px items in the 176px control column: **None / Solid / Pattern / Gradient**. Names and tooltips
  `No fill`, `Solid fill`, `Pattern fill`, `Gradient fill`. The UI keeps "Solid" as built (0071 section 2).
- New glyph `GradientPaintIcon` (16px grid, `currentColor`): the 11px square of the other paint glyphs
  (1.5px outline at 2.5 / 2.5) filled with a diagonal fade from `currentColor` at the top left to
  transparent at the bottom right (an SVG `linearGradient` whose stops use `currentColor`, so the pressed
  white glyph fades too). Hairline-free, no second colour.
- Offered for every selected rectangle, ellipse, polygon, star, path, compound path, linked clone, open
  path and multi-line text. Not for images, groups and single-line texts (those have no fill, `0076`).
  For a text alone the switch shows None / Solid / Gradient (Pattern is not offered for text, `0071`),
  so the Gradient item sits in the third place there: hidden, not disabled.
- Choosing Gradient on a shape that has none: Linear, Start = the shape's Solid colour (alpha kept), End
  `#FFFFFFFF`, Angle 0 across the box; one step "Fill: Gradient" (criterion 2). Switching away keeps the
  gradient; choosing Gradient again brings it back as it was, no step needed beyond the switch's own.
- Mixed selection with images or groups: the others change; the skip line under the Fill title says
  "1 image cannot have a gradient and was not changed." / "2 images cannot have a gradient and were not
  changed." / "1 group cannot have a gradient and was not changed."; images and groups together:
  "3 objects cannot have a gradient and were not changed." (P16).

## 3. The Gradient block: order and sizes (criterion 3)

One column, content 244px. Rows 28px. Between rows 8px, except between stop rows 4px (a list, P3).

| # | Row | Control | Size |
|---|---|---|---|
| 1 | Fill title row | "Fill" (12px semibold, label column 60px) and the paint switch | 60 + 8 + 176 |
| 2 | Kind row | `ToggleGroup` "Gradient kind", items `Linear`, `Radial` (symbol only); right-aligned the **Angle** field (Linear) or the **Radius** field (Radial) | 2 x 44 = 88; field 148 |
| 3..n | Stop rows | one per stop, first to last (section 4) | 244 x 28, 4px apart |
| (picker) | Colour picker | `ColourPicker` under the **active** stop row, 8px above and below | 244 x 116 |
| n+1 | Stop actions | **Add stop** (`Plus`) and **Reverse** (`ArrowLeftRight`), 28 x 28 icon buttons, 4px apart, left-aligned | 60 |
| n+2 | State line | only with a mixed selection: "Mixed gradients" (12px `--panel-muted-fg`) in place of rows 3 to n and the picker | 244 x 16 |

- Angle and Radius sit in the Kind row (P1) because they are the geometry of the kind: switching Kind
  swaps the one field in place and nothing else moves. The spec's Tab order (criterion 7) puts them
  last; with this layout the Tab order is the visual order: paint switch, Kind, Angle or Radius, stop
  rows (each: eyedropper, hex, Position, Remove), the picker area and hue, Add stop, Reverse (spec delta D1).
- There is **no Opacity field** in the Gradient block: alpha is per stop, in the 8-digit hex (P4).
- Heights: two stops 28 + 8 + 28 + 8 + (28 + 4 + 28) + 8 + 116 + 8 + 28 = **292px**; each further stop
  adds 32px. The Solid block is 224px, the Pattern block taller; all scroll in the one panel scrollbar.
  The picker moves with the active stop but is always shown, so the block's height does not change when
  the active stop changes, only when stops are added or removed (P2). No inner scroll area, also at 256
  stops: the rows are windowed in code, the panel scrolls (criterion 24 c and e).

### Kind row

- `Linear` glyph `LinearGradientIcon`: the 11px paint square filled with a left-to-right fade of
  `currentColor`. `Radial` glyph `RadialGradientIcon`: an 11px circle (1.5px outline) filled with a fade
  from `currentColor` at the centre to transparent at the rim. Tooltips and names "Linear", "Radial".
  `ToggleGroup` look and keys (one Tab stop, arrows move and select). Mixed kinds: nothing pressed.
- A kind press keeps the stops and sets the default geometry of the new kind (criterion 3): one step
  "Gradient: Linear" / "Gradient: Radial" (P5, named like "Pattern: Honeycomb").
- **Angle**: label "Angle", unit "°" (P6; the spec says "deg"; every angle field of the product shows
  °, spec delta D2), one decimal, trailing ".0" kept ("30.0 °") so the column does not jump. Up / Down 1°,
  Shift 15°, committed on key-up. Typed values outside -180 to 180 are wrapped into it (as the Pattern
  Angle and `0072`; P7). Invalid text: "Enter a number." in the validation chip.
- **Radius**: label "Radius", unit "mm" whatever the display unit (as Stroke width and the Pattern
  lengths), the distance from the centre handle to the radius handle on the page, up to 3 decimals,
  trailing zeros trimmed. Up / Down 0.1 mm, Shift 1 mm. Range above 0 to 10,000: "Enter a number above 0."
- Both: the bar number field, 148 x 28, plain (no bar, no drag-to-change, no reset slot); Enter or Tab
  commits, Escape restores, as `0111` 6.4. Mixed: placeholder "Mixed".

## 4. Stop rows (criteria 3, 5)

`[swatch 28] 4 [eyedropper 28] 4 [hex 84] 4 [Position 64] 4 [Remove 24]` = 244px.

- **Swatch**: the 0017 swatch (stop colour at its alpha over the checkerboard). A pointer press on it
  makes the row active; it stays `aria-hidden` and is not a Tab stop (0017 rule).
- **Eyedropper**: the 0017 button; picks into this stop; name "Pick colour of stop 2 from the drawing".
- **Hex**: the hex `TextField` at 84px (`#RRGGBBAA` at 14px tabular is about 72px plus 6px padding
  each side; the draft renders `#DDDDDDDD`, the widest real value, without clipping; if a platform
  font clips, the text drops to 13px, not the Position field). Accessible name "Stop 2 colour".
  Typing previews nothing (0017 rule); Enter or Tab commits "Edit gradient".
- **Position**: bar number field 64px, no label inside (the column is the position; the accessible
  name says it: "Stop 2 position, percent"), value right, "%" inside. One decimal, trailing ".0"
  trimmed ("50 %", "33.3 %"). Clamped between its neighbours on commit, equal allowed, without a
  message (criterion 3: stops never reorder by typing). Outside 0 to 100: "Enter a number from 0 to 100."
- **First and last row**: Position is static text "0 %" / "100 %" right-aligned in the same 64px slot,
  `--panel-muted-fg`, no Tab stop; **no Remove** button, the 24px slot stays empty so the columns align
  (P8: only inner stops are removed, as on the canvas; spec delta D3). So a two-stop gradient shows no
  Remove at all, which is the spec's "hidden when only two stops are left".
- **Remove**: 24 x 28 icon button, `Trash2` 16px, tooltip "Remove stop", name "Remove stop 2". One step
  "Remove gradient stop"; the next stop becomes active (the previous one if it was the last inner one).
  No arming: the step is undoable and one stop is small (P9; the armed trash of `0081` is for objects
  and operations).
- **Active stop** (P2): one stop of the panel's gradient is active, session state, never saved, default
  the first stop when the block appears. It becomes active by: focus entering its row, a press on its row
  or swatch, a click on its handle on the canvas (section 6), being added. Marked by a 3px `--accent`
  bar in the panel padding at the row's left (x = -8px, the list row bar of `0101`) and by the halo of
  its handle on the canvas; colour is never the only cue (the bar is a shape). The picker sits under it
  and edits it: a drag previews on the canvas each frame, release commits "Edit gradient".
- **Add stop**: `Plus` 16px, tooltip and name "Add stop". Adds a stop in the middle of the widest gap
  (the first of equal gaps), in the shade's colour there, one step "Add gradient stop"; the new stop is
  active. A pointer press returns focus to the canvas, a keyboard press keeps it on the button, so Enter
  Enter Enter adds three (P10).
- **Reverse**: `ArrowLeftRight` 16px, tooltip and name "Reverse". Order and positions mirror, the
  handles stay where they are (so the shade flips), one step "Reverse gradient"; the active stop stays
  the same stop, now in its mirrored row.
- No Add or Remove limit: 256 stops is 256 rows (criterion 24 e).

## 5. Multi-selection (criterion 8)

"Mixed-state display on multi-select" as 0071 section 9.
- Paint kinds differ: no paint item pressed; no block. Pressing Gradient makes all Gradient (kept
  gradients come back, others start at the default of criterion 2); one step.
- All Gradient, same kind and same number of stops: the full block, each field "Mixed" where values
  differ (the swatch shows the 0017 mixed hatch). Typing sets every shape.
- All Gradient, kinds or stop counts differ: Kind row (nothing pressed if kinds differ; the Angle or
  Radius field only when all share the kind), then the state line "Mixed gradients", then Reverse alone
  (Add stop needs a common list; it is not in the tree, P11). Reverse acts on each shape's own gradient.
- The handles are drawn for the first selected Gradient shape only (criterion 8).

## 6. Handles on the shape (criteria 6, 7)

### 6.1 When they are drawn

Select tool; at least one selected shape with Fill Gradient; drawn for the first such shape. Not in the
Node tool, not while a text is being edited, not during a move, resize, rotate or skew drag of the
shape (they return on release), not on a locked object. Not drawn when the gradient line is shorter
than 24px on screen (Linear) or the radius under 12px (Radial): the panel is the route then (P12).

### 6.2 Look (no new colour)

| Element | Look |
|---|---|
| Gradient line | 1px `--accent` solid over the 1px-each-side white casing (`--selection-casing`), from Start to End (Linear) or centre to radius handle (Radial) |
| Radial circle | the circle (or ellipse) of the radius, 1px `--accent` dashed 4 / 3 over the casing (the selection box rhythm) |
| End handle (Linear End, Radial radius) | 12px circle filled with the stop colour (over a 4px checkerboard where alpha < 1), 1.5px `--accent` ring, 1px white casing outside the ring |
| Start handle (Linear) and centre (Radial) | the same, but an 11px square with a 2px corner radius, so the direction reads without colour (criterion 6) |
| Inner stop | 8px circle, colour-filled, 1.5px `--accent` ring, white casing |
| Hover | a 2px halo 2px outside the ring in `--hover-box` |
| Active stop | the halo in full `--accent` (also during its drag and while its entry box is open) |
| Canvas-selected stop (section 6.4) | the active halo; nothing else |

Handles are screen-space and not rotated. On a laser page the handles keep the real stop colours, as
the panel does: the handle is the stop, not the burn (P13). Colours shown are the stop colours only.

### 6.3 Priority against the shape's own handles

At the defaults the Linear ends sit exactly on the left and right edge resize handles, and the Radial
centre on the centre move glyph and its radius handle on the right edge handle. With the spec's
Proposal (the gradient handle loses within 8px) a fresh gradient could not be edited on the canvas at
all. So (P14, spec delta D4): **gradient handles rank first**, like the parameter handles (rank 0 in
ties, drawn above every other handle). A shape handle whose centre lies within 8px of a drawn gradient
handle is **not drawn and not hit** while the gradient handles are shown; the other corners and edges
still resize, and S types a size. Hit radii: end, Start and centre 12px (24px target); inner stops
10px; the line 4px each side (for the double-click only; a press on the line is a press on the body).

### 6.4 Gestures

| Gesture | Result | Undo step |
|---|---|---|
| Press on a handle | that stop becomes active (panel row marked and scrolled into view if the Style tab is shown; the tab is never switched, keyboard focus stays on the canvas: P15, spec delta D5) and **canvas-selected** | none |
| Drag an end handle (Linear) | that end moves; Angle and length follow; readout "30.0°" at the pointer | "Edit gradient" |
| Drag the radius handle | radius and direction follow; readout "r 24.5 mm" | "Edit gradient" |
| Drag the centre (Radial) | the whole circle moves (centre with its radius handle), as Inkscape | "Edit gradient" |
| Drag an inner stop | its position along the line, clamped between its neighbours; readout "42 %" | "Edit gradient" |
| Shift during an end or radius drag | the angle locks to the angle stops of `0072` (every 15° and 22.5°), about the other end | (same) |
| Ctrl during an end drag | the whole line moves, both ends (P17, spec delta D6: the spec's Alt is taken by Linux window managers for window moves, the design system's rule in the keyboard map) | (same) |
| Escape during a drag | the previous state returns, nothing is written | none |
| Double-click on the line (not on a dot) | a stop at that place in the shade's colour there; it is active | "Add gradient stop" |
| Double-click on an end handle (Linear) or the radius handle | the end entry box (section 7) | on commit |
| Double-click on an inner stop or the Radial centre | the stop entry box (section 7) | on commit |
| Delete or Backspace with an inner stop canvas-selected | removes it (down to two) | "Remove gradient stop" |
| Delete or Backspace with an end or centre canvas-selected | nothing; the shape is **not** deleted while a gradient handle is canvas-selected (P18) | none |
| Escape with a stop canvas-selected | clears the stop selection; a second Escape clears the object selection as before | none |

The stop selection also ends with a press elsewhere, a selection change or a tool change. The first
press of a double-click is a press (it activates); the drag threshold is the canvas's 3px (8px touch).
The readout is the "Live transform readout" chip: values only, no hint line; there is **no hint chip**
on gradient handles (customer rule: no operating tips).

## 7. Entry boxes on the canvas (criterion 7, Frage 235 A plus)

The "Position handle entry box" surface: `--toolbar-bg`, `--panel-elevation-shadow`, 8px radius, 6px
padding, its top left 16px right of and below the handle, flipped and clamped inside the canvas the
same way; upright; follows the handle on zoom and pan; the handle keeps the active halo while it is open.

- **End box** (Linear Start or End): one row, `Angle` field (112px, "°") 4px, the stop's **swatch
  button** (28 x 28) 4px, its **hex** field (92px). Width 248px. Radial radius handle: the same with
  `Radius` (112px, "mm") in place of Angle.
- **Stop box** (inner stop): swatch button, hex (92px), `Position` (88px, "%"). Radial centre: swatch
  button and hex only (it is the 0 % stop).
- **Swatch button**: the swatch drawn as a button (1px ink at 60 % outer border, hover
  `--editor-accent-hover`), name "Edit colour", `aria-expanded`. A press opens the **0017 colour
  picker inside the same box**, under the row: area 236 x 80, 8px, hue 236 x 12 (the box grows down,
  or up when clamped). This is the "colour picker popover" of criterion 7 and Frage 235: 0017 has no
  popover any more (the design system removed it), so the box hosts the inline picker (spec delta D7).
  A second press closes it.
- Focus on open: the first field with its text selected (Angle, Radius, or the hex of a stop). Tab and
  Shift+Tab cycle inside the box. Picker drags preview on the canvas. **Enter commits the whole box as
  one step "Edit gradient"**; Escape, a press outside or a tool change closes it and writes nothing.
  Invalid text: the field's invalid look and the 12px message row under the fields ("Enter a number.",
  "Enter a number above 0.", "Enter 3, 4, 6 or 8 hex digits") as the Transform entry chip; the box stays open.
- Group name of the box: "Gradient end" / "Gradient stop"; field names "Gradient angle, degrees",
  "Gradient radius, millimetres", "Stop 2 colour", "Stop 2 position, percent".

## 8. Open paths and text (criteria 20, 21)

- **Open path**: Gradient is offered and drawn. The fill closes the path with the straight line from the
  last to the first node, exactly as its Solid fill; no extra mark for that closing line, no state line
  (unlike Pattern's "Open shapes show no pattern."). The handles live on the path's box.
- **Text** (multi-line, `0074`): one shade across the text box, clipped to the glyphs. Handles on the
  text box in the Select tool; none while the text is being edited. A text change re-fits the shade
  (criterion 15), so the handles move with the box.

## 9. Laser page grey (criterion 10)

- On a laser page the canvas draws every Gradient fill as its grey (the luminance of criterion 10, over
  white). The page thumbnails of the Pages list do the same. The Style tab, the entry boxes and the
  handles show the real colours (P13). No notice, no label, no setting (spec Proposal).
- On a drawing page the colours. PNG export always colour (criterion 11).
- In the Operations view of a laser page the gradient is replaced by the kind look as every fill
  (`0101`); once `0123` is built, an Engrave gradient shows its dots (`0123` UX notes section 3).

## 10. SVG import notice (criterion 14)

Today SVG import leaves out what it does not read without a message (`docs/svg-import.md`). Criterion
14 asks for counts, so this story adds **one canvas notice after an Open or Import** that lost gradient
features (spec delta D8: the first partial-loss notice of SVG import; the architect carries the counts
in the import outcome). Canvas notice slot, the notice look, `role="status"`, the slot's times (3 s,
5 s above 70 characters), one sentence per line, only the lines that apply, in this order:

| Case | Singular | Plural |
|---|---|---|
| `spreadMethod` reflect or repeat | "1 gradient repeat was shown as a plain shade." | "3 gradient repeats were shown as plain shades." |
| `fx` / `fy` focal point | "1 focal point was read as the centre." | "2 focal points were read as the centre." |
| mesh gradient (no fill) | "1 mesh gradient was left out." | "2 mesh gradients were left out." |

No notice when nothing was lost; no success notice. Each line is under 70 characters.

## 11. History names (criterion 18)

"Fill: Gradient" (paint switch), "Gradient: Linear" / "Gradient: Radial" (kind, P5), "Edit gradient"
(any colour, Position, Angle, Radius, handle drag, entry box commit, eyedropper pick), "Add gradient
stop", "Remove gradient stop", "Reverse gradient". Switching to None, Solid or Pattern stays as built
("Change style" / "Fill: Pattern"). Undo and redo refresh the rows, the picker and the handles in the
same update; the active stop is kept if it still exists, else the first.

## 12. Keyboard and accessibility

- Tab order (visual): paint switch, Kind, Angle or Radius, per stop row the eyedropper, hex, Position,
  Remove, then the picker's area and hue (they edit the active stop), Add stop, Reverse. Focus entering
  a row makes it active (the picker moves under it before the area receives focus).
- `role="group"` "Stop 2 of 4" per row; the active row has `aria-current="true"`.
- Canvas handles are pointer controls; every handle action has a panel route (fields, Add, Remove,
  Reverse). Canvas keys: Delete / Backspace and Escape as section 6.4; no new letter shortcut.
- Contrast: ink 8.3:1, muted 5.6:1 on the panel; the handle ring and casing follow the casing rule
  (at least 2.1:1 for the better of the two against any fill); the active halo is full `--accent`.
- Targets: 28px rows; handles 24px targets (inner stops 20px); coarse pointer enlargements unchanged.
- Nothing animates.

## 13. Proposals (vetoable at the demo, with defaults)

| # | Proposal | Default |
|---|---|---|
| P1 | Angle or Radius in the Kind row, right of Linear / Radial | as described; Tab order follows |
| P2 | One active stop; the picker always shown under it | first stop active when the block appears |
| P3 | Stop rows 4px apart (a list), other rows 8px | as described |
| P4 | No Opacity field in the Gradient block; alpha per stop in the hex | as described |
| P5 | History name of the kind switch | "Gradient: Linear" / "Gradient: Radial" |
| P6 | Angle unit shown as "°" | "°" (not "deg") |
| P7 | Angle typed outside -180..180 wraps | wraps, as Pattern Angle |
| P8 | Only inner stops have Remove (panel and canvas) | ends never removed |
| P9 | Remove stop without arming | one press, undoable |
| P10 | Add stop: pointer returns focus to the canvas, keyboard keeps it | as described |
| P11 | Mixed stop lists: Add stop not in the tree, Reverse shown | as described |
| P12 | No handles below 24px line length / 12px radius on screen | panel only then |
| P13 | Handles, panel and entry boxes show real colours on a laser page | real colours |
| P14 | Gradient handles win; covered shape handles are not drawn or hit | as described (reverses the spec's 8px Proposal) |
| P15 | A handle press activates the row without moving focus or switching the tab | as described |
| P16 | Skip wording for images and groups together | "3 objects cannot have a gradient and were not changed." |
| P17 | Ctrl, not Alt, moves the whole line | Ctrl |
| P18 | Delete with an end handle canvas-selected does nothing (never deletes the shape) | as described |

## 14. Spec deltas for the product owner

- **D1** Criterion 7, Tab order: Kind, Angle or Radius, stop rows, Add stop, Reverse (P1).
- **D2** Criterion 3: Angle unit "°" instead of "deg" (P6).
- **D3** Criterion 3: Remove only on inner stops; the end rows have none (P8).
- **D4** Criterion 6: gradient handles win within 8px and the covered shape handle is hidden (P14);
  the spec's Proposal would make the default handles unreachable.
- **D5** Criterion 7: a handle click activates and scrolls the row, it does not move keyboard focus to
  the panel (else Delete on the canvas could not remove the stop) and does not switch the tab (P15).
- **D6** Criterion 6: Ctrl moves the whole line instead of Alt (P17); the Radial centre drag moves the
  whole circle without a modifier.
- **D7** Criterion 7 and Frage 235: "the colour picker popover of 0017" does not exist; the entry box
  hosts the 0017 inline picker behind its swatch button.
- **D8** Criterion 14 and `docs/svg-import.md`: the import notice is new (SVG import is silent about
  losses today); wording in section 10. The architect adds the counts to the import outcome.
- **D9** Criterion 3 has no Opacity for gradients; say so ("alpha per stop") so the tester does not
  look for one (P4).

## 15. Implementer checklist

- `FILL_KIND_OPTIONS` gains `gradient` with `GradientPaintIcon`; `LinearGradientIcon`,
  `RadialGradientIcon` in `PatternIcons.tsx` or a new `GradientIcons.tsx` (16px, `currentColor` fades).
- `GradientBlock`: Kind row with the field, stop rows (window in code above about 40), the picker under
  the active row, Add and Reverse, the Mixed line; one loop over the stops from Rust's view.
- Canvas: the handle overlay (line, circle, square Start or centre, dots, halo), the 8px hide rule for
  shape handles, gestures and readouts of 6.4, stop selection and Delete routing, the two entry boxes.
- Laser page grey in the canvas and the thumbnails; panel and handles in colour.
- Import notice texts; History names; no new colour token, no new shortcut, no popup in the panel.
- `docs/design-system.md` row "Gradient fill (`0124`)" is in the Style section (added with these notes).

## Links

Spec: `specs/0124-gradient-fill/specification.md`. Drafts: `docs/design-drafts/2026-11-gradient/`.
Pattern followed: `specs/0071-pattern-fill/ux-notes.md`. Next: `specs/0123-raster-engraving/ux-notes.md`.
