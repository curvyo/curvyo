# 0071 Pattern fill: UX notes

Status: Draft for Ready. Owner: ux-engineer. Authority: `specification.md` (customer decisions PAT-LIVE,
PAT-PLACE, PAT-SET, PAT-SLICE). References: `docs/design-system.md` ("Properties panel: Style section",
"Value field", "`ToggleGroup` item", "Mixed-state display", "No popups in the properties panel", "No
operating tips"), `FillSection.tsx`, `PaintRow.tsx`, `ValueField.tsx`, `ColourBlock.tsx`. **(Proposal)**
marks reversible choices. Customer rule 2026-10-16: no operating tips or hint labels. Labels that name a
control or a state stay; there is no explanatory caption anywhere below. Customer goal: nice, consistent,
above all working, so every control here is an existing component except the tile picker.

## 1. Where it lives

Properties > Style > **Fill** section, the same section that holds Colour today. No rail tool, no top bar,
no new tab, no popup, no new shortcut. The section stays one column of 28px rows, 8px apart, content
244px. The Fill title row never moves when the rows below come and go.

## 2. Fill kind

The Fill paint group (`PaintRow`) gets a third item: **None / Solid / Pattern**, three 44px items in the
176px control column. The spec's "Colour" is the shipped "Solid"; the UI keeps "Solid" (consistency, no
rename of accepted UI). Names and tooltips: `No fill`, `Solid fill`, `Pattern fill`. New 16px glyph
`PatternPaintIcon` (1.5px stroke, `currentColor`): a 12px square with three diagonal lines inside it.
`Stroke` keeps two items (the group takes its options as a prop).

- Pattern on a closed shape: the Pattern block (section 3 to 8) replaces the Colour block. On an open
  shape Pattern is also offered (criterion 23) and the block is shown (section 8).
- Choosing Pattern on a shape that has no pattern starts with **Hatch** at its defaults and a fresh stored
  seed, one undo step "Fill: Pattern". Switching to None or Solid keeps the pattern and its parameters
  (lead decision 2026-11-02; criterion 3 is met because nothing is drawn while the fill is not Pattern).
  Choosing Pattern again brings the kept pattern back as it was; no undo is needed for that.
- Pattern is offered for every selected object that can carry a fill outline (path, compound path,
  rectangle, ellipse, polygon, star). Texts, images, groups are skipped like the 0076 fill skip: the rest
  is changed and the existing skip line shows "1 text cannot have a pattern and was not changed." If
  nothing selected can take it, the Pattern item is not in the tree (hidden, not disabled).

## 3. Block order (top to bottom, 8px rows, 16px not used)

| # | Row | Control |
|---|---|---|
| 1 | Fill title row | None / Solid / Pattern |
| 2 | Generator | tile picker, 2 columns x 5 rows (section 4) |
| 3 to 6 | Parameters | one `ValueField` per parameter of the generator (section 5), 244 x 28 |
| 7 | Margin | `ValueField`, all generators |
| 8 | Seed | only for seeded generators (Leaf forest): entry field and re-roll button |
| 9 | Draw as | label column 60px "Draw as", `ToggleGroup` of two 88px text items `Lines`, `Shapes` |
| 9a | Whole pieces only | `PanelSwitch` "Whole pieces only", 28px row right under Draw as; shown only for Dots, Honeycomb, Brick wall, Leaf forest, Sea waves (criterion 29, customer 2026-11-02); Mixed shows off |
| 10 | State line | count, limit and open-shape lines (sections 7, 8); absent when empty |
| 11 | Colour block, picker, Opacity | **Shapes only**: the existing Fill rows unchanged, because the pieces are filled. Lines hides them (the stroke paints the lines) |
| 12 | Convert | full-width text button "Convert to paths" (section 10) |

Parameters come right under the picker so cause and effect sit together; Draw as follows the numbers
because it changes the look the least often. Heights: picker 5 x 40 + 4 x 4 = 216px, so a pattern Fill is
about 216 + 5 x 36 + the rest: it scrolls in the one panel scrollbar like Stroke does at 800 x 640.
Hidden rows leave the tree; a changed generator changes the rows below the picker only.

## 4. Generator picker

- `role="radiogroup"` named `Pattern generator`, ten radios in the spec order (criterion 4): Hatch,
  Crosshatch / Grid, Dots / Honeycomb, Brick wall / Leaf forest, Sea waves / Waves, Interpolation waves.
- Tile: 120 x 40px, `rounded-[5px]`, 4px gaps, a 24 x 24px **static** line glyph at the left (6px
  padding, `aria-hidden`, 1.5px stroke, `currentColor`) and the **visible name** at the right, 12px
  (`--toolbar-icon`), up to two lines, never ellipsised. The glyph shows the generator in miniature, not
  the shape's own pattern (no live thumbnails, no cost per tile). "Interpolation" is the longest word
  (about 70px at 12px): the implementer checks it fits; if not, the glyph drops to 20px, not the name.
- Look: the `ToggleGroup` look. Unpressed: 1px `--toolbar-icon` at 60% border, `--toolbar-icon` glyph
  and name. Hover `--editor-accent-hover`. Pressed (`aria-checked`): `--toolbar-icon-active-bg` ground,
  `--toolbar-icon-active-fg` glyph and name (4.5:1). Focus-visible: 2px `--editor-accent` ring, 1px
  `--toolbar-bg` offset. Mixed (selected shapes use different generators): no tile pressed.
- Keyboard: one Tab stop (the pressed tile, else the first). Arrow Left/Right move and select (wrap to
  the next or previous row), Up/Down move one row, Home/End first/last, as `ToggleGroup` selects with the
  arrows. Every move is a commit: each press of an arrow is one undo step "Pattern: Honeycomb". Pointer
  press returns focus to the canvas, keyboard keeps it on the tile.
- New component `TileGroup` (2D roving radiogroup); `ToggleGroup` is one row and stays as it is.
- Glyphs (24px grid): Hatch 5 diagonal lines; Crosshatch two crossing sets; Grid 3 + 3 orthogonal lines;
  Dots a staggered 3-2-3 dot lattice; Honeycomb three hexagons; Brick wall three offset rows of bricks;
  Leaf forest three small leaf outlines at angles; Sea waves two rows of three nested arcs; Waves three
  sine lines; Interpolation waves four lines fanning from one curve to another.

## 5. Parameters

All lengths are **mm whatever the display unit**, like Stroke width and the Text size field. Every field
is the shipped `ValueField` (label inside left, value and unit right, bar, reset slot): drag, click,
double-click to type, arrows step (Shift x10, Ctrl /10), Ctrl+Backspace resets to the generator default,
Mixed shows "Mixed". No +/- buttons (rule "Value field"). The accessible name is `Pattern <label>`
("Pattern spacing"); `aria-valuetext` says the unit in words. New scales only, no new look:

- **Length** (Spacing, Size, Radius, Width, Height, Diameter, Wavelength, Amplitude, Wall, Joint, Ring
  spacing): log track 0.1 to 50 mm, `v = 0.1 x 500^p`, grid 0.01 (Shift 0.1, Ctrl 0.001), typed 0.05 to 1000.
- **Margin**: the Stroke width curve (`v = 20 (100^p - 1) / 99`, drag 0 to 20, typed 0 to 1000), default 0.
- **Angle**: signed bar with the zero mark (rule "Signed bar"), track -180 to 180, integer, Shift 15,
  Ctrl 0.1, unit `°`, typed any finite number stored modulo 360 and shown in -180 to 180 (as `0072`, `0090`).
- **Percent**: linear 0 to 100, integer. **Count**: linear 1 to 50, typed 1 to 500 (the Count field).
  **Rotation jitter**: linear 0 to 180 `°`.

| Generator | Fields (label: default) |
|---|---|
| Hatch | Spacing 2 mm; Angle 45 |
| Crosshatch | Spacing 2 mm; Angle 45; Second angle 135° (lead decision 2026-11-02; the spec leaves the second line set open) |
| Grid | Spacing 5 mm; Angle 0 |
| Dots | Spacing 3 mm; Diameter 1 mm |
| Honeycomb | Cell size 6 mm; Wall 0.5 mm |
| Brick wall | Width 20 mm; Height 10 mm; Joint 1 mm |
| Leaf forest | Leaf size 6 mm; Size jitter 30 %; Rotation jitter 180 degrees; Density 50 %; Seed |
| Sea waves | Scale radius 5 mm; Rings 3 (Count scale, 1 to 10 dragged); Ring spacing 1 mm |
| Waves | Wavelength 10 mm; Amplitude 2 mm; Spacing 2 mm; Angle 0 |
| Interpolation waves | Lines 12 (Count scale, 2 to 50 dragged, typed 2 to 500) |
| all | Margin 0 mm |

- Defaults are first estimates sized for a 100 mm shape; the implementer may adjust them if a default
  gives an ugly or over-limit result on a 100 mm square, and records it in the PR.
- Cross-field limits: Joint (Brick wall) and Wall (Honeycomb) must stay below the smaller dimension of
  their cell. A drag stops at that bound; typing past it shows the validation chip "Must be smaller than
  the brick height" / "Must be smaller than the cell size". No field is ever disabled.
- Switching generator: Spacing, Angle, Margin, Draw as and Seed carry over where the new generator has
  them; other fields take the new defaults; the old generator's own values are dropped (undo restores).
  **(Proposal)**
- **Seed row** (Leaf forest only; the stored seed of an unseeded generator is kept and not shown): label
  `Seed`, entry field (`EntryField` pattern, whole number 0 to 4294967295, tabular, left aligned, 136px)
  and a 28 x 28 icon button, Lucide `Dices` 16px, 1.5px, the Eyedropper button's look, name `Re-roll
  seed`, no shortcut. Re-roll draws a random seed, commits it (one step "Pattern: new seed") and the
  canvas shows the new scatter. Mixed seeds: the field shows "Mixed"; re-roll gives each shape its own
  new seed.
- **Draw as**: `Lines` / `Shapes` ground and focus as `ToggleGroup`. Lines uses the shape's stroke settings
  (width, colour, opacity, dash) even while the outline's own stroke is off (lead decision 2026-11-02).
  The outline's stroke is not changed, and no state line is shown for it.

## 6. Live preview and undo

- While a field is dragged, stepped by an arrow key or the seed is typed, the canvas shows the
  regenerated pattern, at most once per animation frame, latest value wins (preview per frame, one
  commit, as every `ValueField`). If a regeneration is slower than a frame the previous pattern stays
  until the new one is complete: never blank, never half a pattern. The panel and the drag stay responsive.
- **One undo step per committed change**: drag release, click-set, Enter or Tab in typing, one arrow
  release, a tile press or arrow, Re-roll, Draw as, the fill-kind switch, Convert. An arrow key held down
  previews and commits once on release (the shipped field rule). Escape between press and release, or
  while typing, restores the previous value and writes nothing (criterion 2).
- History names as built (lead decision 2026-11-02): "Fill: Pattern" (kind switch to Pattern), "Pattern:
  <generator>" (e.g. "Pattern: Honeycomb"), "Pattern spacing", "Pattern margin" (field edits named by their
  field), "Pattern: new seed", "Pattern: Lines" / "Pattern: Shapes", "Convert pattern to paths". Switching
  to Solid or None, and the colour edits, stay "Change style". Consecutive steps of the same field are not
  merged. Undo and redo refresh the tiles, fields and state line in the same update.
- Editing the outline on the canvas (node drag, resize, hole) regenerates every frame under the same
  "previous stays until ready" rule; a move only translates the pattern.

## 7. Count limits (criterion 18)

The state line (row 10) is one 12px text row in the panel, reserved only while it has text.
- Under 50,000 pieces: no line.
- From 50,000 to 500,000: quiet, `--panel-muted-fg`, "52,300 pieces". Updates live during a drag.
  Not a live region, not a notice.
- Above 500,000 pieces: "Not generated: 612,000 pieces. The limit is 500,000." in `--field-invalid`
  with the 12px alert glyph, `role="alert"` once when it appears. Past the limit a drag **previews the
  last valid pattern** (lead decision 2026-11-02: the last valid pattern stays visible) and shows the live
  count; on release (or Enter) the field returns to its last valid value, nothing is committed (no undo
  step), and the refusal text stays 8 s, then the quiet line or nothing. A typed value is treated the same.
- A shape edit (resize, node move) that pushes an already placed pattern over the limit keeps the edit
  and draws no pattern for that shape; the red line stays as the shape's state until it is back under
  (lead decision 2026-11-02).
- Wording uses "pieces" because that is the spec's word; the counts use a comma as thousands separator.

## 8. Open shapes (criterion 23)

Pattern stays offered and the block is shown in full (a maker can set it up before closing the path).
Nothing is drawn and no error appears. The state line says it, muted: "Open shapes show no pattern." For
a mixed selection: "1 open shape shows no pattern." / "3 open shapes show no pattern." This is the only
state label for open shapes. Lines with the outline's stroke off gets no state line (lead decision 2026-11-02).
Convert (section 10) is not in the tree when no selected shape has generated geometry.

## 9. Multi-selection with mixed fills

Follows "Mixed-state display on multi-select".
- Fill kinds differ (some None or Solid, some Pattern): no item of the kind group pressed. The Pattern
  block is not shown. Pressing Pattern makes every shape Pattern: a shape with a kept pattern gets it back
  as it was (lead decision 2026-11-02), a shape without one gets the Hatch defaults and its own fresh seed.
  Shapes that are already Pattern are untouched. One step.
- All Pattern, different generators: no tile pressed; the parameter rows are not shown. Margin and Draw
  as are shown (Mixed if they differ). Pressing a tile sets that generator on all (section 5 carry-over
  per shape).
- All Pattern, one generator: that generator's tile pressed, all its fields shown, Mixed where values
  differ (muted "Mixed", no bar). Editing a Mixed field sets every shape to the new value (absolute
  drag, as the shipped rule). The count line shows the **sum**.
- Colour block (Shapes mode): shown when all shapes are Shapes; Mixed Draw as shows neither item pressed
  and no colour rows.
- Skipped objects (section 2) use the existing skip line.

## 10. Convert to paths

- A full-width (244 x 28px) text button **Convert to paths** at the end of the Pattern block, the look of
  the Grid pane's "Reset to page corner" (1px `--toolbar-icon` at 60% outline, 14px label, hover
  `--editor-accent-hover`, 2px focus ring). No glyph, no tooltip beyond its name, no shortcut.
- In the tree only when a selected shape has generated geometry (hidden, not disabled); pattern shapes
  that are open or over the limit are skipped silently because they have nothing to convert.
- One press, all selected pattern shapes, one step named "Convert pattern to paths" (criterion 24). No
  confirmation, no success notice (rule 2026-10-31). The new group(s) become the selection, so the panel
  switches to the group's Style; the shape stays under it with Fill None. Focus: the button leaves the
  tree, so the canvas takes focus after a mouse or keyboard press.
- **Panel button only** (lead decision 2026-11-02). No Object menu and no object context menu exist (the
  window has no menu bar since `0051`; the only context menu is the node menu), so criterion 24's "Object
  menu and context menu" cannot be met literally. The product owner rewords criterion 24. No Select bar
  button.

## 11. On the canvas

The pattern is ordinary vector pieces drawn with the shape's own styling, nothing special:
- **Lines**: open pieces with the shape's stroke colour, width, opacity, dash, cap and join, also while the
  outline's stroke is off (lead decision 2026-11-02). Naturally closed pieces in Lines are stroked, as
  criterion 16.
- **Shapes**: closed pieces with the shape's fill colour and opacity (the Colour rows of section 3) and no
  stroke. Line generators (Hatch, Crosshatch, Grid, Waves, Sea waves, Interpolation waves) become closed
  strips as wide as the shape's stroke width, cut at the region, so Shapes shows what Lines shows
  (lead decision 2026-11-02). Dots, Honeycomb, Brick wall and Leaf forest are closed pieces as they are.
- The shape keeps its own outline stroke, drawn above the pieces; the pattern region has no other fill.
- No overlay, badge, frame or handles for the pattern. The selection box and handles are the shape's.
  The Node tool shows only the outline's nodes. Pieces are not separate hit targets: the shape's interior
  is the hit area (a press between two lines selects the shape, as with a Solid fill).
- Pieces are clipped at the margin edge with no marker. The preview is the final look (no ghost style).
- Copies, duplicates and undo show the same pattern at once; the pattern never looks "pending".

## 12. Accessibility and keyboard

- Tab order is visual order: Fill kind group, tile group (one stop), parameter fields in row order, Seed
  field, Re-roll, Draw as group, Colour block rows (Shapes), Convert. The kind group is `role="radiogroup"`
  `Fill paint` with arrows; tiles likewise; fields are `spinbutton` with `aria-valuemin/max/now/text`.
- Every control has a visible name in the row or tile; no icon-only control except Re-roll, which has the
  accessible name `Re-roll seed`. Colour is never the only state: pressed tiles also change ground and
  glyph. Contrast: `--panel-muted-fg` text 5.6:1 on the panel, `--field-invalid` 4.8:1.
- Targets: tiles 120 x 40, fields and buttons 28px high (as the rest of the panel); the shipped coarse
  pointer enlargements apply unchanged.
- Announcements: tile and kind changes are read by the radio semantics; the refusal line is the only
  `role="alert"`. Nothing animates; reduced motion needs no rule.
- New shortcuts: none. Existing keys apply (Ctrl+Z, Esc, Ctrl+Backspace on a field).

## 13. Tokens and components

- **New tokens: none.** All colours reuse `--toolbar-icon`, `--toolbar-icon-active-bg/fg`,
  `--editor-accent`, `--editor-accent-hover`, `--panel-muted-fg`, `--field-invalid`, `--field-placeholder`.
- New components: `TileGroup` (2D radiogroup), `PatternIcons.tsx` (ten tile glyphs and `PatternPaintIcon`),
  a parameter table in code (field list, scale, default, label per generator) so the panel is one loop
  over it. `PaintRow` takes a third option; `ValueField` takes the new scale kinds above; nothing else new.
- To go into `docs/design-system.md` when the story is Done: the Pattern block rows, the tile picker,
  the state-line rule, and the Fill paint group with three items.

## 14. Questions and flags for the lead

1. **Convert to paths**: decided, panel button only (section 10, lead decision 2026-11-02). Open for the
   product owner: criterion 24 wording.
2. **Crosshatch second angle**: decided, 135° (section 5, lead decision 2026-11-02).
3. **Over-limit after a shape edit**: decided, the edit is kept and no pattern is drawn for that shape
   (section 7, lead decision 2026-11-02).
4. **Stroke width 0 in Shapes**: line strips are as wide as the stroke, so with stroke width 0 a line
   generator in Shapes draws nothing. Proposal: leave it as is, no state line. Lead to confirm.
5. **Seed range** 0 to 4294967295 is a Proposal; the architect may narrow it.

## UI review (2026-11-02)

Reviewer: ux-engineer. Build of `story/0071-pattern-fill` at 120f4f0a, Chromium 1440 x 1000, preview build.
Screenshots: `target/0071-browser/ux/` (own run) and `target/0071-browser/` (story scripts).

**Verdict: APPROVE WITH CHANGES.** Nothing blocks the demo. Items 1 and 2 are required before the PR
is opened; 3 to 5 are required in this PR but small; 6 to 8 may go to `docs/technical-debt.md`.

Scripts: `pattern-fill.cjs` passes (137 paths in the export). `pattern-fill-tester.cjs` 20 passed, 2 failed;
the two failures are the obsolete criterion 24 Object-menu and context-menu checks (Convert is panel-only).

Checked and fine: Fill kind None / Solid / Pattern (44 x 28 items, names `No fill`, `Solid fill`, `Pattern
fill`); ten tiles 120 x 40, spec order, `Interpolation waves` wraps to two lines without clipping, pressed
ground and focus ring visible; keyboard (arrows move and select, Down moves one row, Home/End, wrap, one
Tab stop on the pressed tile, Shift+Tab returns to the kind group); field set per generator matches
section 5; Seed row and Re-roll change the scatter (leaf forest, new seed every press); Lines / Shapes
with the colour rows in Shapes only; margin typed; "Must be smaller than the cell" chip for Wall and
Joint; quiet count (118,996 pieces) and the red refusal; open shape offered, state line "Open shapes show
no pattern.", no Convert; mixed kinds show no pressed item and no block, all-Pattern shows one block with
the shared generator; History names "Fill: Pattern", "Pattern: Dots", "Pattern spacing", "Pattern
margin", "Pattern: Shapes", "Change style", "Convert pattern to paths". All ten generators on a rectangle
with a hole stay out of the hole and are cut at the outline, and Lines with the outline stroke off gives
clean hatch lines (`ux/22-laser-lines-no-outline.png`; the hole shows only as the gap).

### Required changes

1. **Refusal and open-shape lines are out of sight (where: state line, row 10).** The line sits after Draw
   as, below every field. At 1000 px height with Waves, Brick wall, Leaf forest or any open path (the
   Markers rows push it down) it is below the fold: `ux/13-waves-tiny.png` shows a black block and no
   reason, while the text "Not generated: too detailed for this shape..." exists only below the visible
   panel. Criterion 18 says the section shows the reason. Fix: render the state line directly under the
   Fill kind row (above the tiles), same text, same `role="alert"` rule. It is absent when empty, so the
   Fill title row does not move in the normal case; the rows below it shift when a line appears.
2. **After Convert to paths nothing is selected on the canvas (where: Convert, criterion 24, section 10).**
   `ux/23b-after-convert-600ms.png`: no selection frame, the panel shows a Stroke / Fill body without the
   object-type label, and switching to History then leaves the Style tab disabled (`aria-disabled`). The
   spec and section 10 say the new group is selected. Fix: select the new group(s) in the same commit so
   frame, panel header ("Group") and Style tab agree; add one assertion to `pattern-fill.cjs` (selection
   handles present after Convert).
3. **Piece count for line generators is not pieces (where: refusal text, `pattern.rs`).** Interpolation
   waves with 500 lines and Sea waves with 500 rings both say "650,000 pieces" (500 x 1,300 sample
   points). The maker typed 500 and sees 650,000 of something that is not on the canvas. Fix: count the
   paths the output will contain, or use the "too detailed" text for those cases. Do not show a number
   that no field of the panel can explain.
4. **Refusal wording, two texts for one situation (where: `refusal_text`).** "Not generated: 1,125,721
   pieces. The limit is 500,000." (counted) and "Not generated: too detailed for this shape. Use a
   larger spacing, size or fewer lines." (estimated). Both start alike, which is good. Change the second
   so it names no lever the current generator lacks: "Not generated: too detailed for this shape. Make
   the pattern coarser." Interpolation waves has no spacing or size, Dots has no lines. One sentence, no
   instruction list (rule 2026-10-16: no operating tips).
5. **Cross-field chip names the wrong thing for Brick wall (where: `FIELD_MESSAGES.cell`).** Joint over the
   brick height shows "Must be smaller than the cell"; section 5 specified "the brick height" and "the
   cell size". Pass the message per field (Wall: "Must be smaller than the cell size", Joint: "Must be
   smaller than the brick height"). The chip also covers the Margin row while it shows; acceptable.

### May go to technical debt

6. **Stroke "Pattern" label vs Pattern fill.** The dash row in Stroke is labelled "Pattern" ("Solid, e.g.
   6 4"); the Fill kind and the block now use the same word for something else. Rename the dash label to
   "Dash pattern" later (existing accepted UI, so not in this PR without the customer).
7. **Value text collides with the bar end tick** in Brick wall Width / Height and Waves Wavelength at the
   default value ("10 mm" with the tick through the digits). Shipped `ValueField` look, not new.
8. **Leaf forest Seed row and Convert are below the fold at 1000 px** (Seed is cut by the panel edge in
   `gen-leaf_forest.png`). Expected with the single panel scrollbar; no change beyond item 1.

### Not a finding

Typing into a field by double-click first click-sets the value from the click position and commits it, so
each typed edit creates two history steps. This is the shipped `ValueField` behaviour for every field and
is outside this story.
