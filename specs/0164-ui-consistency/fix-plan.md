# 0164 UI consistency: fix plan

Status: Draft (UX, 2026-11-27). Merges [`inventory.md`](inventory.md) (code scan, `I1` to `I39`)
and [`walkthrough.md`](walkthrough.md) (browser pass, `W1` to `W52`, screens in `screens/`) into
one findings list, cuts it into slices (one PR each) and collects the customer questions. The
target of every finding is an entry of [`docs/ui-elements.md`](../../docs/ui-elements.md)
(`§n` = its section). Criteria per slice are in [`specification.md`](specification.md).

Choices marked **(Proposal)** are the UX engineer's and can be vetoed at the demo.

## 1. Findings

Severity: **high** = looks broken, unreachable, not recognisable as a control, or below an
accessibility minimum; **medium** = a visible inconsistency; **low** = polish or code
duplication. A finding takes the higher severity of its sources. Screens are `screens/NN-*.png`;
"-" means none was exported (the walkthrough or the code names the place). F numbers follow the
lead's grouping a to g; the slices S1 to S7 follow the build order of section 2.

| F | Finding | From | Screens | Sev. | Target | Slice |
|---|---|---|---|---|---|---|
| F1 | Add page form: Add and Cancel are flat text, Add is not primary, the form has no edge, Kind is three borderless choices | I3, W30 | 15 | high | §1, §2, §7, §17 Inline form box | S1 |
| F2 | Bar commands are borderless text and read as labels (Object to path, Remove rounding, Linked clone, Stroke to path, Box from path, Recognise text, Place on..., Bridge, Add parameter; Node bar "Close path" is a group label per design-system "Close path group" and becomes the muted group-label look) | I4, W2 | 03, 04, 05, 06, 07, 34, 36 | high | §2 (bar size) | S1 |
| F3 | No single button look: 45 outline definitions, six heights, six radii, five border strengths; `sketchButton` flat | I5, W1, I22 (buttons) | 01, 03, 07, 18 | high | §2 | S1 |
| F4 | Library tab CSS buttons: no hover, no focus ring (1.3:1), no primary, Remove token borderless | I6 | - | high | §1, §2 | S1 |
| F5 | Primary button in 11 versions; Tiling Apply drawn in the pressed-toggle look | I7 | 14, 29 | high | §1 | S1 |
| F6 | Destructive button in four copies plus a 25 % border look; Wipe object history pinned far from its list | I8, W41 | 19 | medium | §3 | S1 |
| F7 | Icon buttons in three sizes and four radii; Add page `+` and the Pages up/down are bare glyphs; page-label toggles 24 px at 7 px radius; Fonts tab 20 px tag `+`; library header expand/close bare | I9, W21, W29, W38 (glyphs), W39 | 16, 17, 23, 32, 37 | high | §5 | S1 |
| F8 | About 115 action buttons are text only; "×" and "+" characters as glyphs; OK versus Done, "..." versus "…" | I10, I39, W3 | 13, 16, 18, 19, 24, 25 | high | §0 Icon rule, Copy | S1 |
| F9 | Text links act as buttons in five looks (Clear all, Select, Show all) | I11 | 37 | low | §4 | S1 |
| F10 | Three inline-confirm layouts (File New/Close, Add page, New material) | W52 | 15, 19, 29 | medium | §17 Inline form box | S1 |
| F11 | Panel header page-kind chip is a bordered 28 px static span that looks like a button; other tabs show plain text | W28 | 16, 38 | medium | §17 Section heading | S1 |
| F12 | The one modal (ErrorDialog) uses shadcn's near-black button and radii | I32 | - | medium | §19 | S1 |
| F13 | Disabled in three conventions (40 %, 50 %, filled grey block, pale blue fill); full-width buttons 244 versus 256 px; "Join..." full width beside content-wide "Add device" | I34, W35, W36, W37 | 26, 28 | medium | §0 States, §1, §2 sizes | S1 |
| F14 | Sliding switch in five implementations and three sizes (33+ on screen); switch column moves within the laser Document tab; "All formats" pairs a switch with "Off" | I12, W4, W5 | 01, 11, 14, 17, 18, 24, 25, 27, 28, 34 | high | §6 | S2 |
| F15 | Two "selected" colours: pale blue with dark text equals hover (Test grid, Pencil profile, Template after a press); Into slot caption label at 2.5:1; Fixture shape pressed in ink | I14, W6, W45 | 11, 18, 33 | high | §0 States Pressed, §6 | S2 |
| F16 | Snapping block mixes a switch with pressed text toggles without symbols | I15 | 24 | medium | §6 | S2 |
| F17 | Option checks (Copy in the move, transform and placement chips; Reference, Fix origin, Expose in the Sketch panel) are native checkboxes | I23 (options) | 36 | medium | §6 (Frage 456) | S2 |
| F18 | List-tick checkboxes in five custom versions; Operations row: checkbox, chevron at x 284 versus 256, 24 px actions | I23 (ticks), W34 | 28 | medium | §8, §18 | S2 |
| F19 | Five constructions of segmented control in 11 implementations; Stroke Profile shows no active segment; Fonts filter two-row grid; library Items/Sets | I16, W7, W38 (Items/Sets), W43 | 09, 14, 15, 16, 20, 23, 24, 25, 34, 37 | medium | §7 | S5 |
| F20 | Relative / Absolute is a `role="switch"` drawn as segments | I13 | - (move chip) | medium | §7 | S5 |
| F21 | Origin corner: four loose 28 px icons, no group ring, label centred against two rows | W33 | 25, 27 | medium | §7 (radio grid) | S5 |
| F22 | Four slider copies and four slider looks; value field's value 30 px from the right edge | I17, W8, W38 (Size) | 11, 13, 20, 37 | high | §9 | S5 |
| F23 | Native `<select>` in the panel (Print and cut Sheet); popup menus in two bars undocumented; pick-list selection in three looks | I24, I25, I26 | 15 (machine list) | high | §12 | S5 |
| F24 | Two components named `EntryField` and about 15 inline bar-field copies; three invalid marks | I18 | 03, 04, 10, 27, 33, 34 | medium | §10 | S6 |
| F25 | Native number inputs with steppers; Polygon/star "New: Points" field has no field ground | I19, W20 | 09 | medium | §10 | S6 |
| F26 | Four label placements in number fields; label touches value ("Randomized0.00"); unit outside the field | W9, W19 | 03, 04, 05, 10, 27, 33, 34 | medium | §10 | S6 |
| F27 | Search and tag fields use `ring-border` (about 1.2:1), below 3:1 | I20 | 23, 37 | high | §11 | S6 |
| F28 | Rename fields in seven looks | I21 | 21 | medium | §11 | S6 |
| F29 | Sketch panel field on the panel ground | I22 (field) | 36 | low | §11 | S6 |
| F30 | Six focus-ring variants; the Pages list and the panel body take focus without a ring | I33, W48 | 16 | medium | §0 States Focus | S6 |
| F31 | Nine radii in use; `rounded-md`/`rounded-lg` are 8/10 px under the shadcn theme | I36 | all | low | §0 Radius | S6 |
| F32 | Text sizes 10, 11, 13 px beside 12 and 14 | I37 | all | low | §0 Type | S6 |
| F33 | Lucide default 2 px strokes beside 1.5 px; chevrons at 12, 14, 16 px | I38 | all | low | §0 Icon rule | S6 |
| F34 | Two row-hover grounds, `--row-hover` undocumented | I35 | 21, 28 | low | §18 | S6 |
| F35 | Three section heading styles; label column 68 px, inside-label and right-aligned grids per tab | W31, W32 | 16, 18, 24, 26, 27 | medium | §17 | S6 |
| F36 | Generator card runs into Stroke without heading or hairline; Sketches tab blank without an empty line; Parameters heading twice | W44, W42 | 13, 35, 36 | low | §17 | S6 |
| F37 | Layers: selected ground stops 22 px short, row tooltip over the tab strip (default names with ids: not this story) | W40 | 21 | low | §18 | S6 |
| F38 | Select bar with nothing selected is 72 px: an empty group wraps to row 2 | W13 | 01, 03 | high | §13 Bar | S4 |
| F39 | A mixed selection makes the Select bar four rows (five at 800) and covers the top handles | W14 | 06, 40 | high | §13 Bar (Frage 457) | S4 |
| F40 | At 800 px the ellipse fields and the Generators bar run past the canvas under the panel | W15 | 40, 42 | high | §13 Bar | S4 |
| F41 | Dangling dividers after the last group and at the start of wrapped rows | W16 | 03, 06, 44 | medium | §13 Bar | S4 |
| F42 | The "Original" clone tag is drawn over the Select bar | W17 | 43 | high | §13 (z-order) | S4 |
| F43 | Two bar anchors: Select bar left and canvas-wide, the others centred and content-wide | W18 | 03, 07, 10, 12 | medium | §13 Bar | S4 |
| F44 | Bar pill 36, 40, 44 px high, 8 or 12 px padding; ShapeToolbar ring segments | I27 | 07, 09 | medium | §13 Bar | S4 |
| F45 | Tile bar mixes six looks and becomes a nine-row column at 800 | W22 | 14, 41 | medium | §6, §7, §13 | S4 |
| F46 | Two fields called "Gap" in one bar | W10 | 06 | medium | §0 Copy | S4 |
| F47 | Test grid bar: unlabelled "40BYO" code, "Passes 1 x" suffix reads as a clear button | W23 | 33 | low | §10 | S4 |
| F48 | Tool rail cut off at 640 px height: Generators, Import (laser: Test grid) unreachable | W24 | 45 | high | Toolbox card | S4 |
| F49 | Sketch rail is one long card; sketch tool names lack the "tool (key)" pattern | W25, W26 | 35 | low | Toolbox card, §15 | S4 |
| F50 | Page label "Page 1 Drawing" half under the Select bar in the default view | W12 | 01 | medium | §13 | S4 |
| F51 | Laser operation chips: square (no radius), symbol 1 to 1.5 px off centre, one chip per kind, row floats 10 px right of the corner; ground and edge colours copy DOM tokens without naming them | I1, I2, W11 | 30, 31 | high | §13 Canvas symbol pill, §20 | S3 |
| F52 | Canvas chips in four radii | I28 | 30, 43 | low | §13 Canvas chip | S3 |
| F53 | Measure readout and its "Aligned" state are two chips | W50 | - | low | §13 Canvas chip | S3 |
| F54 | 19 tooltip implementations, two grounds, three max widths; 139 native `title` attributes | I30 | all | medium | §15 | S7 |
| F55 | Tool and slider tooltips carry operating instructions | W46 | 20, rail | medium | §15 (Frage 458) | S7 |
| F56 | Four canvas hint chips and the slider help line predate the no-hint rule | I29 | - | medium | §0 Copy (Frage 454) | S7 |
| F57 | Nine notice implementations; paste/export notice radius matches nothing | I31, W49 | - | medium | §16 | S7 |
| F58 | Pen probe is a raw full-window sheet with text-only buttons | W47 | - | low | §19 | S7 |

Coverage: every `I1` to `I39` and every `W1` to `W52` (W27 and W51 were dropped by the
walkthrough) maps to one row; I22, I23 and W38 are split across rows by part.

## 2. Slices

Each slice is one PR the customer can try end to end, on its own branch from `main`
(`story/ui-consistency-<part>`; slice S1 carries this folder and `docs/ui-elements.md`). Order
by value: what the customer named first, then what is broken or unreachable. **(Proposal)** The
lead's sketch order a to g is kept except that the canvas pill (f) moves to third and the bars
(e) to fourth: the pill is small, isolated in Rust and named by the customer; the bars hold five
high findings (empty row, overflow, clone tag, rail cut off). No slice changes `format_version`,
the document model or a public Rust API; no new dependency (Radix `Toggle` and `ToggleGroup` are
in already).

**Pattern for every TS slice:** commit 1 adds the shared component under
`frontend/src/components/ui/` with unit tests for ARIA, keyboard and states (no call site
changes); then the call sites move. Once the component exists, the swaps whose target is fully
given by the component and the mapping tables of `docs/ui-elements.md` (symbol per act, size per
surface) are mechanical and can go to a **refactorer**; swaps that need a layout or copy decision
(a form box, a row grid, which glyph for a feature act) stay with the implementer. The last
commit deletes the folded constants and CSS rules, so a grep for them is the completeness check.

**Verify (every slice):** the full gate; then one browser agent, foreground, at **1440 x 900 and
800 x 640**, re-shoots the screens named for the slice and compares them with the old ones in
`screens/`; plus the UX review against `docs/ui-elements.md`. Keyboard: Tab through every changed
control, `:focus-visible` ring present (the walkthrough's focus script).

### S1 Buttons and symbols (lead's a) - 13 findings: F1 to F13

- **Customer sees:** every button is outlined or filled and carries a symbol; Add page has a
  framed form with Cancel (`X`) and a blue Add (`Plus`); bar commands look like buttons; one
  primary, one destructive look.
- **Components:** `ui/button.tsx` rewritten (`primary`, `secondary`, `destructive`; sizes bar
  and panel; `busy`); `ui/icon-button.tsx` (`plain` / `outlined`, 24 / 28 / 40); `ui/text-link.tsx`.
- **Touches (TS only):** `panel/AddPageGroup.tsx`, `SelectToolbar.tsx`, `NodeToolbar.tsx`
  (Close path label), the 14 bar files of I4, the 45 sites of I5, the library CSS rules in
  `index.css` (`.library-*` buttons) and their users, `ErrorDialog.tsx` / `ui/alert-dialog.tsx`,
  `panel/PagesBlock.tsx`, `PageFrames.tsx` (label toggles), the panel header chip, the four
  destructive sites, `panel/HistoryTab.tsx`.
- **Risk:** largest diff of the story (about 300 sites). Bar widths grow by the symbols (16 + 4
  px per button), so the Select bar wraps sooner at 800 px until S5; check `screens/40`, `44`.
  Disabled moves from `disabled` to `aria-disabled` in the bars: a press must still do nothing
  (test). Escape-cancels in inline groups stays.
- **Screens:** 01, 03 to 07, 13, 15 to 19, 23 to 26, 28, 29, 32, 34, 36, 37, 38.
- **Design-system row changes:** yes (see §4).

### S2 Toggle buttons instead of switches (lead's b) - 5 findings: F14 to F18

- **Customer sees:** no sliding switch anywhere; every on/off setting is a button that turns
  blue with a white symbol and label when on; one "on" colour across the app; list ticks one
  checkbox.
- **Components:** `ui/toggle-button.tsx` (on Radix `Toggle`, `aria-pressed`, Enter and Space,
  `onReturnFocus`, armed variant for Sync to my devices); `ui/checkbox.tsx` (tri-state).
- **Touches (TS only):** `ui/switch.tsx`, `ToolbarSwitch.tsx`, `CentreLinesButton.tsx`
  (`CentreSwitch`), `panel/ExportSvgSection.tsx`, `panel/PagesBlock.tsx` (Move frame only) and
  their 28 call sites; the 15 pressed-toggle class sets of I14 (`index.css:488`,
  `workbench/FixtureBody.tsx:189` fixed first); `panel/SnappingSection.tsx`; entry-chip and Sketch
  option checks (if Frage 456 A); `OperationRow.tsx`, `FixtureJobParts.tsx`, `FixtureSetRow.tsx`,
  `FixtureRowItem.tsx` checkboxes.
- **Risk:** tests that query `role="switch"` break and move to `aria-pressed` (acceptance tests of
  earlier stories: the tester updates the selectors, the behaviour criteria stay). Space-only
  toggling becomes Enter and Space: Enter inside a bar must not also commit a field. A row-wide
  panel toggle (Frage 452 A) changes panel heights; check the laser Document tab at 640 px.
- **Screens:** 01, 11, 14, 17, 18, 24, 25, 27, 28, 33, 34, 36.
- **Design-system row changes:** yes ("Switch (`ScaleStrokeSwitch`)" is replaced).

### S3 Canvas symbol pill (lead's f) - 3 findings: F51 to F53

- **Customer sees:** on a laser page in View: Operations each object has one rounded pill with
  all its symbols, centred, above the object.
- **Touches:** **Rust**: `curvyo-render-core/src/operation_symbol.rs` (`chips_list` and `chip`
  become a pill per item: height 22, radius 11, 3 px padding, 16 px cells 4 px apart, the symbol
  outline inset 0.75 px; the collision rect is the whole pill, the cap counts pills),
  `glyphs::rounded_rect` for ground and a rounded 1 px edge (a rounded outline primitive may be
  needed), the token names on the colour constants in `theme.rs` (I2);
  `curvyo-editor-wasm/src/session/operations/look_overlay.rs` only if the anchor needs the handle
  boxes. **TS**: canvas chip radii (F52, `rounded-[8px]`), the Measure readout (F53).
- **Tests:** golden draw-list tests in `curvyo-render-core`: pill width `6 + 16n + 4(n - 1)`,
  each symbol's visual centre on its cell centre within 0.1 px, mirror at the viewport edge,
  collision skip and cap per pill; the existing laser-look CPU-time test under
  `curvyo-editor-wasm/tests/cpu_time/` stays green.
- **Risk:** the anchor of Frage 459 A (centred, 6 px above the top edge) sits where the top edge
  resize handle, the skew handle (16 px) and the Shift rotate handle (32 px, hit radius 16) of a
  selected object are; under the existing "a chip never covers a handle" rule the pill of the
  selected object would vanish. **(Proposal)** While the object shows transform handles, its
  pill keeps today's corner anchor (10 px right of the top-right corner, 6 px above). Needs an
  architect look only if `look_overlay.rs` must pass handle boxes into render-core.
- **Screens:** 30, 31 (new shots with one, two and five kinds; selected and not), 43.
- **Design-system row changes:** yes ("Spacing and sizing": a Canvas symbol pill row;
  `specs/0101-laser-operations/ux-notes.md` 13.3 gets a pointer).

### S4 Bars, rail and canvas overlap (lead's e) - 13 findings: F38 to F50

- **Customer sees:** the Select bar is one row when nothing is selected; no bar runs under the
  panel or covers the selection beyond two rows; no tag over a bar; the tool rail scrolls at
  640 px height; all bars sit the same way.
- **Components:** `ui/bar.tsx` (pill, row lines, dividers only between two groups on one row,
  empty groups not rendered, `max-width` of the canvas column).
- **Touches (TS, maybe Rust):** `SelectToolbar.tsx`, `NodeToolbar.tsx`, `PencilBar.tsx`,
  `TextBar.tsx`, `GeneratorsBar.tsx`, `TilingBar.tsx`, `ShapeToolbar.tsx`, `CropBar.tsx`,
  `PlayBar.tsx`, `TestGridBar.tsx`, `TraceBar.tsx`, `PenModeBar.tsx`; `CloneTags.tsx` (z below the
  overlay row, now `z-30`); `ToolRail.tsx` (column A scrolls, `overflow-y-auto` as the panel
  already does); the default view's top inset for F50 (where the fit lives; Rust if it is in
  `curvyo-editor-wasm`).
- **Order inside the slice:** first everything that holds under every answer to Frage 457 (F38,
  F40 to F44, F46 to F50); the two-row limit and the "More" part (F39, F45) last, in their own
  commit, so a late answer changes one commit.
- **Risk:** Frage 457 B and C reverse the customer rule "Bar overflow: expandable second row,
  never a dropdown icon" (2026-10-24, `0084`); the Text bar's row-2 chevron must keep working.
  Changing the Select bar's anchor (F43) moves every Select-bar control; check the move and
  transform chips that sit below it.
- **Screens:** 01, 03, 05, 06, 07, 09, 10, 12, 14, 33, 35, 40 to 45.
- **Design-system row changes:** yes ("Select bar layout", "Bar overflow" if 457 B or C,
  "Toolbox card").

### S5 Segmented controls, sliders, pick lists (lead's c) - 5 findings: F19 to F23

- **Customer sees:** every "pick one" looks the same (one strip, chosen item blue); one slider
  look; no system dropdown in the panel; the plain stroke shows Uniform as chosen.
- **Components:** `ui/toggle-group.tsx` (exists; gets the radio-grid layout for Origin corner);
  `ui/slider.tsx` (new); `InlinePickList.tsx` (exists).
- **Touches (TS only):** `panel/PanelRadioGroup.tsx`, `fontPreview/ModeSwitch.tsx`, the Add page
  Kind (done in S1, verify), `.library-kind`, `ShapeToolbar.tsx` mode, `workbench/FixtureBody.tsx`,
  `MoveEntryChip.tsx:330`, `TestGridBar.tsx:41`, the Stroke Profile strip; `RatioSlider.tsx`,
  `fontPreview/SizeSlider.tsx`, `PlayBar.tsx`, `panel/AdjustStrength.tsx`;
  `panel/PrintAndCutSection.tsx:163`; `TracePresetMenu.tsx`, `TestGridMaterialMenu.tsx` (look
  only); `panel/AddPageGroup.tsx:154`, `index.css:492`.
- **Risk:** arrow keys in a radiogroup select at once: a segmented control that triggers a heavy
  recompute (Dither, Trace) must not recompute per arrow step more than today. Stroke Profile
  "Uniform" active is a state read, check it shows Mixed for a mixed selection.
- **Screens:** 09, 11, 13, 14, 15, 16, 20, 23, 24, 25, 27, 34, 37.
- **Design-system row changes:** yes ("Segmented icon control", "Slider", "Choice strip" pointer).

### S6 Fields, focus, contrast, panel grid (lead's d) - 14 findings: F24 to F37

- **Customer sees:** every number field, search field and rename field looks the same; search
  fields have a visible edge; every focus stop shows the same ring; panel sections share one
  heading and one label column.
- **Components:** `ui/number-field.tsx`, `ui/text-field.tsx`, `ui/search-field.tsx`,
  `ui/rename-field.tsx`; the radius, type and icon clean-up as a token pass.
- **Touches (TS and CSS only):** both `EntryField.tsx`, `ArcCurveGroups.tsx` (`BarNumberField`),
  the inline fields of I18, `ShapeToolbar.tsx:86`, `SelectToolbar.tsx:451`, `BridgeGroup.tsx:74`,
  the I20 and I21 sites, `sketch/SketchField.tsx`, `index.css` (`.library-search`,
  `.library-source-form input`, `--row-hover`), the panel sections of W31 and W32, `LayersTab.tsx`,
  `PagesBlock.tsx` and the panel body (focus ring).
- **Risk:** the radius and type pass touches nearly every file (about 600 class tokens): do it
  as a separate commit by the refactorer from a fixed mapping (`rounded-md` → `rounded-[5px]` on
  controls, `rounded-[8px]` on surfaces), and re-shoot all screens. Field commit rules (Enter,
  Escape, blur) must not change: existing entry-chip tests are the net. Replacing native
  `type="number"` removes the browser's arrow-key step: `NumberField` provides arrows, Shift,
  Ctrl.
- **Screens:** all, both sizes (token pass); focus script on the 13 tab states.
- **Design-system row changes:** yes (colour table `--row-hover`, "List row" hover, "Bar number
  field", "Properties panel" row grid).

### S7 Tooltips, notices, hints (lead's g) - 5 findings: F54 to F58

- **Customer sees:** one tooltip look that also opens on keyboard focus; tooltips name the control
  and its shortcut only (Frage 458); the hint chips and the slider help line gone (Frage 454 B);
  one notice look.
- **Components:** `ui/tooltip.tsx` (look moves to the canvas chip surface); `ActionNotice`,
  `AlertLine` (new, `components/ui/`).
- **Touches (TS only):** the 18 own Radix tooltips of I30, the `title` attributes on controls in
  72 files, `lib/sliderNames.ts:52-54`, `KeyHintChip.tsx`, `HandleHintChip.tsx`, `EditHintChip.tsx`,
  `PenHintChip.tsx`, `Canvas.tsx:410-441`, the notice sites of I31, `PenProbe.tsx`.
- **Order inside the slice:** tooltip component and look first; removing the hint chips and the
  instruction texts last, one commit each, so an answer A to 418 or 431 drops a commit.
- **Risk:** removing hints changes accepted behaviour of earlier stories (their acceptance tests
  that assert the chips or the help text are rewritten, and the specs get a note). Tooltips on
  focus must not open on the focus return to the canvas after a press. If 431 is C, the
  instruction texts go into the user documentation in the same PR.
- **Screens:** 20, rail tooltips, export and paste notices, canvas hint chips (before/after).
- **Design-system row changes:** yes ("No operating tips by default" keeps; the hint-chip rows
  and tooltip texts of the tool rows go).

| Slice | Lead's | Findings | High | Rust | PR shows |
|---|---|---|---|---|---|
| S1 Buttons and symbols | a | 13 | 7 | no | every screen with a button |
| S2 Toggle buttons | b | 5 | 2 | no | panel and bar settings |
| S3 Canvas symbol pill | f | 3 | 1 | yes (render-core) | laser page, View: Operations |
| S4 Bars and rail | e | 13 | 5 | maybe (default view) | bars at 800 x 640, rail |
| S5 Segmented, sliders, pick lists | c | 5 | 2 | no | strips, sliders, Print and cut |
| S6 Fields, focus, contrast | d | 14 | 1 | no | fields, focus, panel grid |
| S7 Tooltips, notices, hints | g | 5 | 0 | no | tooltips, notices, canvas |

## 3. Customer questions

The Fragen as asked in `inventory.md` and `walkthrough.md`, with the lead's defaults. Each blocks
nothing: the default is built and the answer can change it at the demo.

Frage 452 (SWITCH-LOOK): How an on/off setting in the panel looks once the switch is gone
- A: a full-width 244 x 28 button with symbol and label; on = blue ground, white symbol and label.
- B: the label at the left, a 28 x 28 symbol toggle at the right.
- C: keep the switch in the panel, toggle buttons only in the bars.
Recommendation: A. Default: A. Slice S2.

Frage 453 (BTN-SYMBOL): "All buttons with symbols": which buttons
- A: every button with a label gets a symbol, Cancel (`X`), OK (`Check`), Add (`Plus`) included.
- B: action buttons get one; Cancel, OK and Done stay text only (outlined).
- C: bars icon-only with tooltips, panel symbol + label.
Recommendation: A. Default: A. Slice S1.

Frage 454 (HINT-KEEP): The four canvas hint chips and the slider help line predate the no-hint rule
- A: keep them as they are.
- B: remove them all.
- C: remove the slider help line, keep the canvas chips.
Recommendation (lead): B, per the no-hint rule. Default: B. Slice S7, last commit.

Frage 455 (OP-PILL): The operation symbols on the laser canvas
- A: one rounded pill per object holding all its symbols, centred.
- B: one chip per kind as now, but rounded (4 px) and centred.
- C: A plus a count when more than three kinds.
Recommendation: A. Default: A. Slice S3.

Frage 456 (CHECK-TOGGLE): The option checks in the entry chips and the Sketch panel
- A: become toggle buttons like the former switches.
- B: stay checkboxes, in the one checkbox look.
Recommendation: A. Default: A. Slice S2.

Frage 457 (BAR-NARROW): What a context bar does when its groups do not fit in one row
- A: wrap whole groups into more rows, never past the canvas; the bar may cover handles.
- B: one row; groups that do not fit go into a "More" menu at the bar's end.
- C: A up to two rows, then B.
Recommendation (lead): C. Default: C. Slice S4, last commit. B and C replace your rule of
2026-10-24 "expandable second row, never a dropdown icon".

Frage 458 (TIP-TOOLTIPS): Tool and slider tooltips with operating instructions
- A: keep them.
- B: tooltips show name and shortcut only; the instructions go.
- C: B, and the instructions move into the documentation.
Recommendation (lead): C. Default: C. Slice S7.

Frage 459 (PILL-ANCHOR): Where the operation pill sits on the laser canvas
- A: centred above the object, 6 px over its top edge.
- B: at the top-right corner as now, only the look changes.
- C: centred inside the object when it fits, else A.
Recommendation: A. Default: A, with the corner anchor while the object shows its handles
(Proposal, slice S3 risk). Slice S3.

**Dependencies:**
- 419 and 432: 432 places what 419 decides; with 455 B "centred" means the chip row is centred.
  C of 432 puts the pill over the artwork, against 13.3 "chips never lie over the object's hatch".
- 416 and 420: 456 A uses the look 416 picks; with 452 C, 456 A means toggle buttons in the
  chips but switches in the panel (two "on" looks again), so 452 C pairs with 456 B.
- 417 and 416: 453 C (bars icon-only) also makes the bar toggles icon-only.
- 418 and 431: one family; the slider help line is both a hint (418) and a tooltip text (431,
  Opacity). 454 B with 458 A would keep the text in the tooltip: the lead's pair B + C is
  consistent. Both remove accepted behaviour, so both are the customer's call; the removals are
  the last commits of S7.
- 430 and S1: symbols widen the bars, so the bars wrap more until S4 lands; 430 decides how far.

## 4. Changes to `docs/ui-elements.md` and `docs/design-system.md`

`docs/ui-elements.md` is the rule from 0164 on; each slice marks its entries as built (a line
"Built: PR #n" under the entry), turns its **(Proposal)** into the rule once the customer accepts
the PR, and records the Frage answers. `docs/design-system.md` sections that a slice supersedes
get a one-line pointer to the `ui-elements.md` entry instead of their own spec (no copy kept).

| Slice | `docs/ui-elements.md` | `docs/design-system.md` |
|---|---|---|
| S1 | §0 Icon rule (417 answer, symbol table final), Copy (OK not Done); §1 to §5 component paths final and "Built"; §17 Inline form box and confirm layout; §19 Dialog; header paragraph: questions 416 to 420 and 430 to 432, link to this plan | "Fit to content", File tab "Buttons", "Close path group", "Stroke to path", "Flip buttons", "Guide", "Link corners toggle" and the Add page group row: pointer to §1, §2, §5; "Properties panel" disabled rule unchanged |
| S2 | §6 Toggle button (416 answer, list of former switches checked off); §8 Checkbox (420 answer); §0 States "Pressed" | "Switch (`ScaleStrokeSwitch`)" replaced by a pointer to §6; the Select bar and laser tab rows naming a switch; Library facets "Customizable" row (`ui/switch`) |
| S3 | §13 Canvas symbol pill (419 and 432 answers, anchor while selected); §13 Canvas chip radius; §20 token names | "Spacing and sizing": new "Canvas symbol pill" row; "Transform entry chip" and chip radii to 8 px; `specs/0101-laser-operations/ux-notes.md` 13.3 pointer |
| S4 | §13 Bar (`ui/bar.tsx`, one anchor, empty groups, never past the canvas, 430 answer) | "Select bar layout", "Bar overflow" (only if 457 B or C), "Toolbox card" (column scrolls), Text bar row "measured" values re-measured |
| S5 | §7 Segmented control (radio grid), §9 Slider (`ui/slider.tsx`), §12 Pick list and Menu (I25 decided) | "Segmented icon control (`ToggleGroup`)", "Choice strip" last sentence, the Slider row, "No popups in the panel" unchanged |
| S6 | §0 Radius and Type (Proposal to rule), §0 States Focus, §10, §11, §17 Panel row grid, §18 List row | Colour table: add `--row-hover`; "List row" hover to `--row-hover`; "Bar number field"; "Properties panel: Document section" label column 60 px |
| S7 | §15 Tooltip (431 answer), §16 Notice components, §0 Copy (418 answer) | "No operating tips by default" gains the 418 and 431 outcome; the hint-chip rows and the instruction texts in tool rows go; "Canvas notice slot" pointer to §16 |
