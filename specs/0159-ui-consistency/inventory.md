# 0159 UI consistency: code-level inventory

Status: Draft (UX, 2026-11-27). Input for `specification.md` and `plan.md` of this folder; the
target look of every element kind is in [`docs/ui-elements.md`](../../docs/ui-elements.md).

Customer request (2026-11-27, translated): "Review the complete UI and straighten it out: buttons
uniform, sliders uniform, toggle buttons always shown by pressed state + colour instead of the
sliding switch, all buttons with symbols, ... 'Add page' and 'Cancel' are not recognisable as
buttons. On the laser page the canvas display of the operations is not properly in a pill and not
centred. Watch for things like that."

## Method

- A TypeScript-AST scan of every `.tsx` under `frontend/src` (286 component files, tests and
  `wasm-bindings` excluded) listed each `<button>`, `<input>`, `<select>`, Radix `Root`/`Item`/
  `Trigger` and every element with an interactive `role`, with its class string, `aria-label`,
  `aria-pressed`, disabled state and children (icon component, literal text, expression). Then
  every shared class constant and `index.css` class rule was read, and the Rust canvas chrome in
  `curvyo-render-core/src/theme.rs` and `operation_symbol.rs`.
- Code only, no browser (another agent drives it). Pixel claims below are from class names and
  constants; the browser check of the plan confirms them.
- Paths are relative to `frontend/src/components/` unless they start with `frontend/`,
  `curvyo-` or `docs/`. Line numbers are the opening tag or the constant.

## Counts

| What | Count |
|---|---|
| `<button>` sites in JSX | 437 (128 icon only, 49 icon + text, 131 **text only**, 127 content from an expression) |
| Text-only buttons that are actions (links and row-name buttons excluded) | about 115, every one needs a symbol |
| Separate class definitions of the outline (secondary) button | 45 (31 Tailwind constants or inline strings, 14 `index.css` rules) |
| Separate definitions of the primary (Apply) button | 11, three of them not in the Apply look |
| Flat text buttons (no border, no ground) that read as labels | 26 sites in 14 files, among them **Add page: Add and Cancel** |
| Switch implementations / call sites | 5 implementations plus 1 switch styled as segments / 28 call sites, at least 33 switches on screen over all views; **every one becomes a pressed toggle button** |
| Pressed-toggle implementations already in the code | 15 (4 Radix `Toggle`, 11 `aria-pressed` class sets); 2 with a wrong pressed look |
| Segmented-control implementations | 11 |
| Slider implementations | 5 (4 copies of one pointer slider, plus the colour picker) |
| Number-field implementations | 14 |
| Text, search and rename field implementations | 19 |
| Checkbox implementations | 9 native, 5 custom `role="checkbox"` |
| Tooltip implementations | `ui/tooltip.tsx` plus 18 files with their own Radix tooltip; 139 native `title=` attributes in 72 files |
| Pick lists and menus | 8 inline lists, 2 popup menus, 1 native `<select>` |
| Notice implementations | at least 9 |
| Focus-ring variants | 6 |
| Corner radii in use | 9 (`rounded-sm`, `rounded`, `rounded-[4px]`, `rounded-[5px]`, `rounded-[7px]`, `rounded-md`, `rounded-[8px]`, `rounded-lg`, `rounded-full`) |
| Text sizes in use | 6 (10, 11, 12, 13, 14 px, plus 16 for one heading) |

## Findings

Severity: **high** = the customer sees it or it fails an accessibility minimum; **medium** = a
visible inconsistency between two places; **low** = code duplication or a detail. "Target" names
the entry of `docs/ui-elements.md`.

### Canvas chrome (Rust)

**I1 (high). The operation symbols on a laser page are square chips with the symbol off centre, one chip per kind instead of one pill.**
`curvyo-render-core/src/operation_symbol.rs:12` (`CHIP_PX` 22), `:121` (`chip`), `:128`
(`let inset = 2.0`). The chip is a 22 x 22 px square with no corner radius; the 16 px symbol box
is placed at a 2 px inset, so it ends 4 px from the right and bottom edges (2 px left and top):
1 px off centre, and the 1.5 px outline of Cut and Cut-out (`square_outline`, drawn on the box
edge) reaches 0.75 px past the box, so the visible symbol sits about 1.5 px up and left. The UX
notes asked for "4 px radius, the 16 px symbol inside at 2 px inset, centred"
(`specs/0101-laser-operations/ux-notes.md` 13.3); 2 px inset and centred cannot both hold in
22 px. One object in two operations shows two separate squares 4 px apart, not one shape.
Target: "Canvas symbol pill": one pill per object, height 22, radius 11, 3 px padding, symbols
16 px centred at 3 px, 4 px apart, `OP_CHIP_GROUND` ground and a 1 px `OP_LANE_LINE` edge;
anchor and collision rules unchanged (Frage 419).

**I2 (low). The chip ground and edge colours are copies of DOM tokens.** `curvyo-render-core/src/theme.rs:263-273`
(`OP_MUTED_INK`, `OP_INK`, `OP_CHIP_GROUND`, `OP_LANE_LINE`) repeat `--panel-muted-fg`,
`--toolbar-icon`, `--toolbar-bg`, `--lane-line`. Expected (the canvas is WebGL, the design
system says so); a dark theme must change both. Target: a comment that names the token on each
constant (most already do); no change in behaviour.

The other Rust canvas chrome (node 14, handle 12, transform resize 8, rotate 12, move 16, skew
18 x 12, knob 10, `theme.rs:73-417`) matches its rows in `docs/design-system.md` "Spacing and
sizing"; no finding.

### Buttons

**I3 (high). Add page: the Add and Cancel buttons are flat text on a ground of the same colour.**
`panel/AddPageGroup.tsx:28` (`TEXT_BUTTON`: no border, no ground), `:180` Cancel, `:183` Add; the
group itself is `bg-[var(--toolbar-bg)]` (`:104`) inside the panel whose ground is
`--panel-bg` = `--toolbar-bg`, so the group has no visible edge either. Add is the primary action
and has no Apply look. Target: "Secondary button" (Cancel, symbol `X`) and "Primary button" (Add,
symbol `Plus`); the group gets the inline form box of "Add and Edit form" (1 px border at 25 %,
5 px radius, 8 px padding).

**I4 (high). Text buttons in the bars have no border and no symbol, so they read as labels.**
`SelectToolbar.tsx:235` (`BUTTON_CLASS`, used by Linked clone `:588`, Remove rounding `:731`,
Object to path `:857`), `BoxFromPathButton.tsx:6`, `BridgeGroup.tsx:20` (Bridge `:243`),
`StrokeToPathButton.tsx:9`, `RecogniseBar.tsx:23`, `TraceBar.tsx:54`, `TileGroup.tsx:5`,
`TilingBar.tsx:30`, `CentreLinesButton.tsx:32` (Cancel `:324`), `TraceColourGroup.tsx:11`,
`CurveToBezierButton.tsx:11` (12 px, the others 14 px), `PencilBar.tsx:147`, `OpenGroups.tsx:17`.
The Node bar's Sharp and Smooth already have the outline ("Close path group" row), so two looks
exist in the bars. Target: "Secondary button", bar size, symbol + label.

**I5 (high). 45 definitions of the outline button, five border strengths, four radii, three text sizes.**
Canonical in the design system: "Fit to content" (`panel/DocumentSection.tsx:188`) and
`panel/FormatForm.tsx:32` (`OUTLINE_BUTTON`): 28 px, 1 px `--toolbar-icon` at 60 %, 5 px, 14 px.
Deviations:
- border at 40 %: `workbench/FixtureSetsSection.tsx:12`, `workbench/ApplyFixturesList.tsx:102`,
  `workbench/ManifestTab.tsx:97`, `workbench/FixtureBody.tsx:184`, `sketch/SketchesPanel.tsx:41`;
- border at 25 %: `panel/HistoryTab.tsx:650` (Wipe history), `:690` (Cancel), `HistoryPreview.tsx:43`
  (Back to live);
- `--lane-line` border, 12 px text, 4 px radius: `panel/MaterialBody.tsx:14`, `panel/LaserSets.tsx:27`,
  `panel/TestDetail.tsx:14`, `panel/TestPhoto.tsx:12`, `panel/OperationSettingsBlock.tsx:49`,
  `panel/NewMaterialForm.tsx:115`, `panel/PhotoCrop.tsx:146`;
- full-strength border: `panel/OperationPathPart.tsx:222` (Automatic);
- `ring-1` instead of a border: `libraryBrowser/BrowserGrid.tsx:281`, `BrowserDetailBar.tsx:24`,
  `SetsGrid.tsx:37`, `GuideView.tsx:57`, `fontPreview/GlyphDetailBar.tsx:59`;
- 13 px text or `rounded-md` (8 px): `panel/AiStepBody.tsx:9`, `FixtureSetsSection.tsx:12`,
  `ApplyFixturesList.tsx:102`;
- 24 px high, 12 px: `panel/WorkspaceDeleted.tsx:9`, `GeneratorBlock.tsx:315`, `index.css:439`
  (`.library-online-add`), `:487`, `:498`;
- hover by border only: `sketch/ExposeControls.tsx:78`;
- copies of the canonical string: `panel/devicesUi.tsx:8`, `ExportSvgSection.tsx:13`,
  `FileTab.tsx:18`, `WorkspaceSection.tsx:23`, `PrintAndCutSection.tsx:30`, `CloneBlock.tsx:17`,
  `AdjustmentsGroup.tsx:28`, `GeneratorBlock.tsx:386`, `:429`, `PatternBlock.tsx:190`,
  `PropertiesPanel.tsx:615`, `RegistrationMarksBlock.tsx:68`, `WorkspaceTree.tsx:437`,
  `PenProbe.tsx:11`, `RestartConfirm.tsx:7`, `workbench/FixtureBody.tsx:106`, `index.css:409`,
  `:422`, `:437`, `:462`, `:480`.
Target: one `Button` component (variant `secondary`), "Secondary button".

**I6 (high). The Library tab's CSS buttons have no hover, no focus ring and no primary.**
`frontend/src/index.css:462` (`.library-source-buttons button`: Cancel and Add/Save look alike),
`:409` (`.library-add`), `:422`, `:437`, `:480`, `:487`, `:456` (`.library-token-remove`: no
border at all); used by `panel/AddSourceForm.tsx:182`, `panel/TokenForm.tsx:125`,
`LibraryPreview.tsx:555` (Import page), `:558`, `IntoSlotList.tsx:178`. The only focus cue is the
base `outline-ring/50` (grey at 50 %, about 1.3:1 on the panel), below 3:1. Target: the same `Button`;
remove the CSS button rules.

**I7 (high). Primary buttons in 11 versions; three are not the Apply look.**
Apply look as specified: `panel/FormatForm.tsx:34`, `panel/devicesUi.tsx:10`, `panel/FileTab.tsx:287`.
Variants: `panel/NewMaterialForm.tsx:122` (12 px, 4 px radius, no hover), `panel/AiStepBody.tsx:11`
(`--editor-accent`, hover by opacity), `TraceBar.tsx:311` (`FINISH_CLASS`, no border; also Centre
lines Apply `CentreLinesButton.tsx:195`), `TraceAdvanced.tsx:56`. Not the Apply look:
`TilingBar.tsx:493` (Apply drawn with `SELECTED_CLASS`, the pressed look of a toggle, so it reads
as "on"), `ErrorDialog.tsx:49` (shadcn `AlertDialogAction`, near-black `--primary`), the library
Add/Save (I6) and Add page Add (I3) with no primary at all. Target: `Button` variant `primary`.

**I8 (medium). Destructive buttons in four copies.** `panel/FileTab.tsx:275` (Discard),
`panel/HistoryTab.tsx:698` (Wipe), `panel/WorkspaceSection.tsx:388` (Delete),
`panel/devicesUi.tsx:12` (`DESTRUCTIVE`); same look, four strings. `HistoryTab.tsx:650` shows the
red label on a 25 % border before arming, a fifth look. Target: `Button` variant `destructive`.

**I9 (medium). Icon buttons: three sizes and four radii for the same job.**
28 px `rounded-md`: the bars (`AlignGroup.tsx:18`, `FlipGroup.tsx:8`, `TextBar.tsx:30`, ...);
28 px `rounded-[5px]`: `panel/PanelStrip.tsx:106`, `panel/RecoveryBlock.tsx:9`,
`libraryBrowser/BrowserHeader.tsx:13`, `fontPreview/PreviewHeader.tsx:34`, `panel/FormatList.tsx:28`;
28 px `rounded` (4 px): `panel/PagesBlock.tsx:34`; 24 px `rounded-[4px]` / `rounded-md` / `rounded-[5px]`:
`panel/LayerRow.tsx:131`, `panel/WorkspaceTree.tsx:58`, `workbench/rowKeys.ts:23`,
`panel/HistoryRow.tsx:194`, `workbench/FixtureLockButton.tsx:7`; 24 x 28: `workbench/MachineProfileList.tsx:12`,
`panel/FormatList.tsx:25`. Only some grow to 36 px under `pointer-coarse`. The "Add font..." look
(28 px outline icon button, `FormatList.tsx:28`) and the plain one sit side by side in the panel
headers without a rule. Target: "Icon button" (28 bar and panel header, 24 in a list row, 40 rail).

**I10 (medium). About 115 action buttons are text only.** The full list is the AST scan; the
groups: inline confirms and forms (Cancel 24 times, OK, Done, Add, Save, Join, Pair, Connect,
Try again, Not now, Restore, Delete, Discard, Wipe), panel actions (Fit to content, Convert to
paths 3 times, Replace outline, Add registration marks, Add mark, Spread colours, Make editable,
Set file aside, Reset placement, Edit material, Automatic, Export SVG..., New document, Make
workspace, Undo move, Add slot), bar commands (I4), Library (Import page, Import elements, Placed
page, Real size, Clear all 3 times, Show sets, Copy, Replace, Insert), Sketch panel (Inspect, Edit,
Add parameter, Construction, Delete, Show free geometry, Unlink), `sketch/SketchesPanel.tsx:31`
(a "×" character as Remove) and `panel/FontTagLine.tsx:253` (a "+" character with a count).
Target: "Icon rule" in `docs/ui-elements.md`: symbol + label; the mapping table there (Frage 417).

**I11 (low). Text links act as buttons in five looks.** `libraryBrowser/BrowserFacets.tsx:79`
(Licences, hover underline), `:251` (Clear all, accent, no underline), `GuideView.tsx:10` (always
underlined), `index.css:380` (`.library-guide-link`), `index.css:443` and `:468` (ink, underlined),
`panel/FixtureJobParts.tsx:127` (Select, ink, underline on hover), `fontPreview/InfoValue.tsx:41`
(Show all). Target: "Text link": only for opening a document or a page outside the app; actions
become buttons (Clear all, Select, Show all become "Secondary button", small).

### Toggles and switches

**I12 (high). The sliding switch exists in five implementations; the customer wants it gone.**
- `ui/switch.tsx` `PanelSwitch` (32 x 18, border 1 px at 60 %): `panel/SnappingSection.tsx:27`,
  `GridSection.tsx:42`, `PatternBlock.tsx:180`, `FormatForm.tsx:247`, `FormatList.tsx:275`,
  `OperationsSection.tsx:401` (Cuts last), `OperationTabsPart.tsx:38`, `WorkspaceShareBlock.tsx:200`,
  `GeneratorBlock.tsx:403`, `sketch/ExposeControls.tsx:40`, `libraryBrowser/BrowserFacets.tsx:320`,
  `workbench/MachineBlock.tsx:314`, `:323`, `:375`;
- `ToolbarSwitch.tsx:45` (32 x 18, border 1.5 px at 100 %, own tooltip): `SelectToolbar.tsx:982`,
  `:989`, `TilingBar.tsx:306`, `:487`, `BridgeGroup.tsx:235`, `PencilBar.tsx:94`, `PenProbe.tsx:109`,
  `panel/ExtensionsGroup.tsx:24`, `workbench/MachineBlock.tsx:291`;
- `CentreLinesButton.tsx:39` (`CentreSwitch`, a third copy): Join tips, Keep inner areas;
- `panel/ExportSvgSection.tsx:77` (24 x 14 track drawn by hand): four export options;
- `panel/PagesBlock.tsx:345` (20 x 12 track, `--editor-accent` fill, dark thumb): Move frame only.
`ui/switch.tsx` and `ToolbarSwitch.tsx` differ in the off border (60 % 1 px versus 100 % 1.5 px;
the design system's "Switch" row says 1.5 px `--toolbar-icon`). Target: "Toggle button" (Frage 416).

**I13 (medium). A two-state choice is built as a switch with segments.** `MoveEntryChip.tsx:330`
(Relative / Absolute, `role="switch"` drawn as two segments). Two named states are a choice, not
on/off. Target: "Segmented control", two items.

**I14 (medium). The existing pressed toggles have two wrong looks.**
`index.css:488` (`.into-slot-caption button[aria-pressed="true"]` sets the blue ground but not the
white label: `--toolbar-icon` on `--toolbar-icon-active-bg` is about 2.5:1, below 4.5:1);
`workbench/FixtureBody.tsx:189` (Rectangle / Circle pressed = `--toolbar-icon` ground, white
label: a second "on" colour). The other 13 (`SelectToolbar.tsx:513`, `OperationKindGroup.tsx:63`,
`panel/OperationOptionsPart.tsx:47`, `panel/ImageSettingsGroup.tsx:46`, `GuideButton.tsx:6`,
`LibrarySyncToggle.tsx:48`, `panel/SnappingSection.tsx:16`, `panel/AdjustmentsGroup.tsx:26`,
`sketch/SketchField.tsx:3`, `sketch/SketchTools.tsx:11`, `WidthEditToggle.tsx:31`,
`StyleGroup.tsx:33`, `panel/JobList.tsx:39`) use `--toolbar-icon-active-bg` / `-fg`; good, but as
13 class strings. Target: one `ToggleButton` component.

**I15 (medium). Snapping mixes both patterns in one block.** `panel/SnappingSection.tsx:27` is a
switch ("Snapping"), `:39`, `:46`, `:56` are pressed text toggles (Grid, Centres, Fixtures) without
symbols. Target: four toggle buttons with symbols.

### Segmented controls and choice strips

**I16 (medium). 11 implementations of "pick one of a few".** `ui/toggle-group.tsx` (the
specified one), `panel/PanelRadioGroup.tsx:3` (separate bordered items, 4 px radius, outline
focus), `panel/PresetStrip.tsx`, `panel/DitherStrip.tsx`, `fontPreview/ModeSwitch.tsx:30`,
`panel/AddPageGroup.tsx:26` (borderless filled items), `index.css:459` (`.library-kind`, CSS),
`ShapeToolbar.tsx:51` (`ring-border`, `px-2 py-1`, `aria-pressed` buttons, not a radio group),
`workbench/FixtureBody.tsx:184` (I14), `TestGridBar.tsx:41` (outline group), `MoveEntryChip.tsx:330`
(I13). Target: "Segmented control" = `ToggleGroup`; `PresetStrip` stays the choice strip.

### Sliders

**I17 (medium). Four copies of the pointer slider.** `RatioSlider.tsx:19`,
`fontPreview/SizeSlider.tsx:14`, `PlayBar.tsx:11`, `panel/AdjustStrength.tsx:17` each define
`THUMB = 14`, `INSET`, track, fill and thumb. Differences: the thumb ring is 2 px in three and
1.5 px in `AdjustStrength.tsx:136`; the track colour is set inline in each; the focus cue is a
thumb ring in `PlayBar.tsx:140` and a ring on the wrapper elsewhere. `panel/ColourPicker.tsx:19`
(area 14, hue 16) is its own kind and stays. Target: one `Slider` component, "Slider".

### Number and text fields

**I18 (medium). Two components named `EntryField`.** `EntryField.tsx` (entry chips, invalid =
1 px inset) and `panel/EntryField.tsx` (panel, invalid = 2 px inset). Plus inline copies of the
bar number field: `AlignGroup.tsx:242`, `ArcCurveGroups.tsx:149` (`BarNumberField`),
`SelectToolbar.tsx:304`, `OffsetEntryChip.tsx:255`, `TextPathEntryChip.tsx:108`,
`TransformEntryChip.tsx:300`, `GradientEntryBox.tsx:177`, `TilingBar.tsx:88` (no border),
`WidthPointEntry.tsx:98` (24 px, 4 px radius), `TraceField.tsx`, `panel/SettingsNumberFields.tsx`,
machine fields inside a wrapper (`workbench/MachineBlock.tsx:485`, `:544`, `:577`,
`workbench/FixtureFields.tsx:113`, `workbench/MachineProfileList.tsx:330`). Invalid marks: 1 px
inset, 2 px inset, `border-2` (`BridgeGroup.tsx:74`). Target: "Number field": one `NumberField`
(bar and entry chip) beside `ValueField` (panel).

**I19 (medium). Native number inputs with browser steppers.** `ShapeToolbar.tsx:86` (with the old
shadcn `border-border`, "New:" prefix), `SelectToolbar.tsx:451`, `BridgeGroup.tsx:74`. The design
system says "no +/- buttons, no steppers". Target: "Number field".

**I20 (high). Search and tag fields have a border that cannot be seen.** `fontPreview/PreviewHeader.tsx:51`,
`libraryBrowser/BrowserHeader.tsx:101`, `panel/FontSearchRow.tsx:44`, `panel/FontTagStrip.tsx:58`,
`panel/FontTagLine.tsx:47` use `ring-1 ring-border`; `--border` is shadcn's `oklch(0.922 0 0)`,
about 1.2:1 on white, below the 3:1 for a field boundary. `index.css:384` (`.library-search`)
uses 40 %. Target: "Text field" (1 px `--field-border`).

**I21 (medium). Rename fields in seven looks.** `workbench/FixtureFields.tsx:27` (24 px, accent
border, 4 px), `panel/OperationRow.tsx:336`, `panel/MaterialRow.tsx:202`,
`panel/FixtureJobRow.tsx:206` (no height), `panel/FormatList.tsx:70` (24 px, 5 px),
`panel/LayerRow.tsx:389` (28 px, field border), `panel/WorkspaceNameField.tsx:54`
(`--background` ground), `panel/PagesBlock.tsx:290` (`w-full text-xs`, no border, no height).
Target: "Rename field", one component.

**I22 (low). The Sketch panel's field and buttons sit on the panel ground.** `sketch/SketchField.tsx:2`
(`bg-[var(--toolbar-bg)]`: a field without the white ground), `:3` (`sketchButton`: flat text
buttons, `outline-2`). Target: "Text field", "Secondary button".

### Checkboxes

**I23 (medium). Native checkboxes beside custom ones.** Native (`accent-color` only, 14 px):
`MoveEntryChip.tsx:407`, `PlacementChip.tsx:219`, `TransformEntryChip.tsx:364`,
`sketch/SketchCanvasChips.tsx:186`, `:190`, `panel/DeviceChoices.tsx:129`,
`sketch/SketchesPanel.tsx:36`, `:164`. Custom 16 px `role="checkbox"`: `panel/OperationRow.tsx:80`,
`panel/FixtureJobParts.tsx:97`, `:112`, `workbench/FixtureSetRow.tsx:154`,
`workbench/FixtureRowItem.tsx:127`. The design system has no checkbox entry. Target: "Checkbox"
(list ticks only); option checks become toggle buttons (Frage 420).

### Pick lists, menus, selects

**I24 (high). A native `<select>` in the panel.** `panel/PrintAndCutSection.tsx:163` (Sheet) opens
the system dropdown, against "No popups in the properties panel". Target: "Pick list"
(`InlinePickList`) or a segmented control for two or three sheets.

**I25 (medium). Popup menus in two bars, not in the design system.** `TracePresetMenu.tsx:152`,
`TestGridMaterialMenu.tsx:135` (`aria-haspopup="menu"`). Target: "Menu" entry records them as the
only popups outside the panel (Proposal) or they become inline lists; decide in the plan.

**I26 (medium). The selected item of a pick list in three looks.** `panel/AddPageGroup.tsx:154`
(machine list: selected = blue fill, 44 px rows), the List row rule (selected = `--value-fill` and
3 px bar: `panel/MaterialPick.tsx`, `workbench/MachineProfileList.tsx`), `index.css:492`
(`.into-slot-row.current`: `--value-fill` without the bar). Target: "Pick list".

### Bars, pills, chips

**I27 (medium). The bar pill has four heights and two paddings.** 36 px (`h-9`/`min-h-9`, most
bars), 40 px `PlayBar.tsx:94`, 44 px `CropBar.tsx:51`, `NodeToolbar.tsx:213` with `px-3`
(the others `px-2`). `ShapeToolbar.tsx:45` is the oldest bar: `ring-border` segments, native
stepper, "New:" captions. Target: "Bar".

**I28 (low). Canvas chips in four radii.** `rounded-md` (8 px: hint chips, `NudgeReadout`,
`ColourPickChip`), `rounded-[8px]` (entry chips), `rounded-lg` (10 px: `DropBadgePill`,
`FontMissingTags`, `FixtureLabels`), `rounded-[4px]` (`FixtureLabels` small tag), `rounded-full`
(badges). The design system writes "8px radius" everywhere; `rounded-lg` is 10 px in this
Tailwind theme (`--radius` 0.625rem, `index.css` `@theme`). Target: "Canvas chip", radius 8.

**I29 (medium). Operating hint texts that predate the no-hint rule.** `KeyHintChip.tsx`,
`HandleHintChip.tsx`, `EditHintChip.tsx`, `PenHintChip.tsx` (rendered from `Canvas.tsx:410-441`)
and the slider help lines `frontend/src/lib/sliderNames.ts:52-54` ("Click to set, double-click
to type, ..."). Accepted features from before 2026-10-16; removing them changes accepted
behaviour (Frage 418). No new one is added by this story.

### Tooltips, notices, dialogs

**I30 (medium). 19 tooltip implementations and 139 native `title` attributes.** `ui/tooltip.tsx:50`
(white `bg-popover`, `z-[60]`, max 256 px) and own Radix tooltips in `ToolRail.tsx`,
`ToolbarSwitch.tsx`, `CommandButton.tsx`, `FlipGroup.tsx`, `AlignGroup.tsx`, `AlignmentGroup.tsx`,
`WritingGroups.tsx`, `ImportCard.tsx`, `StrokeToPathButton.tsx`, `RecogniseBar.tsx`,
`StyleGroup.tsx`, `ArcCurveGroups.tsx`, `GuideButton.tsx`, `TilingBar.tsx`, `TileGroup.tsx`,
`CentreLinesButton.tsx`, `TraceBar.tsx`, `SelectToolbar.tsx` with `z-50`, max 260/280/320 px, and
two grounds (white popover with ring, or `--toolbar-bg` set inline). Native `title=` (72 files, e.g.
`IntoSlotList.tsx:178`, `MoveEntryChip.tsx:336`, `panel/JobList.tsx`) gives the browser's tooltip:
another look, another delay, not shown on keyboard focus. Target: "Tooltip": `ui/tooltip.tsx` only.

**I31 (medium). At least nine notice implementations.** `CanvasNotice.tsx` (the slot),
`BooleanNotice.tsx`, `panel/FontTabNotice.tsx`, `panel/MembershipNotices.tsx`,
`SlotReplaceNotice.tsx` with `index.css:497` (6 px radius, red text on a ring), `TilingBar.tsx:38`
(`CHIP_CLASS`), the TextBar notice, `CentreLinesButton.tsx:398`, the Layers tab hint
(`panel/LayersTab.tsx:211`, 2 s). Target: "Notice": canvas slot, action notice, alert line.

**I32 (medium). The one modal dialog is shadcn's default look.** `ErrorDialog.tsx:43` via
`ui/alert-dialog.tsx`: `--primary` near-black button, shadcn radii; the only use of
`ui/button.tsx`. Target: "Dialog": keep the modal for fatal errors only, its button the
`Button` primary.

### States, tokens, sizes

**I33 (medium). Six focus-ring variants.** `ring-2` + 1 px `--toolbar-bg` offset (38 uses, the
specified one for bars), `ring-2` inset (38, lists and strips), `ring-2` without offset (most of
the panel), `outline-2` (11: `PagesBlock.tsx:35`, `PanelRadioGroup.tsx:3`, `SnappingSection.tsx:16`,
`sketch/*`), `has-[:focus-visible]` on a label (`ToolbarSwitch.tsx:41`), `focus:ring-2` on every
focus (`SelectToolbar.tsx:451`), underline only (`FixtureJobParts.tsx:127`). Target: "Focus".

**I34 (medium). Disabled in three conventions.** `opacity-40` class with `aria-disabled` (57
uses), `opacity-50` (13: `CentreLinesButton.tsx:195`, `CloneBlock.tsx:270`, shadcn
`disabled:opacity-50`), and the `disabled` attribute versus `aria-disabled` (23 versus 11 class
hooks; `disabled` drops the control from the Tab order and hides its tooltip reason). Some dimmed
bar buttons explain themselves with a native `title` (`SelectToolbar.tsx:731`), others with a
Radix tooltip. Target: "Disabled".

**I35 (low). Two hover grounds for rows, one undocumented.** `--row-hover` (8 %, `index.css`)
in `panel/HistoryRow.tsx`, `panel/LayerRow.tsx`; `--editor-accent-hover` (20 %) in every other
list (`OperationRow`, `MaterialRow`, `FixtureJobRow`, `TestRow`, `FontTreeRows`, `FormatList`,
`WorkspaceTree`, `MachineProfileList`, `JobList`, `index.css:390`). `--row-hover` is not in the
design system's colour table. Target: "List row": `--row-hover` for rows (it keeps muted text at
4.5:1), `--editor-accent-hover` for controls; add the token to the table.

**I36 (low). The radius scale is not written down.** 9 radii in use (counts: `rounded-md` 170,
`rounded-[5px]` 161, `rounded` 100, `rounded-lg` 41, `rounded-full` 43, `rounded-[8px]` 22,
`rounded-[4px]` 21, `rounded-sm` 12, `rounded-[7px]` 4). The design system writes "rounded-md"
and "rounded-lg (8px)" meaning Tailwind's defaults (6 and 8 px), but the shadcn theme makes them
8 and 10 px. Target: "Radius" in `docs/ui-elements.md` (Proposal: 5 px controls, 8 px surfaces,
full for discs).

**I37 (low). Text sizes 10, 11 and 13 px beside 12 and 14.** `text-[13px]` 57 uses (Add page
segments, AI step, fixture buttons, rename fields), `text-[11px]` 51, `text-[10px]` 8, besides
`text-xs` 466 and `text-sm` 237. Target: "Type": 14 control labels and values, 12 secondary and
chips, 11 only for tabular meta lines (Proposal).

**I38 (low). Icons drawn at Lucide's default 2 px stroke beside 1.5 px.** 271 icons at 16 px,
230 with `strokeWidth={1.5}`; the rest of the Lucide icons in controls take the default 2 px.
Chevrons at 12, 14 and 16 px for the same disclosure (`FixtureJobsSection.tsx:394`,
`GeneratorBlock.tsx:371`: 12; `InlinePickList.tsx:41`: 14). Target: "Icon rule".

**I39 (low). Small copy inconsistencies.** "Pen probe..." (`PencilBar.tsx:147`) versus
"Pen probe…" (`PenProbe.tsx:138`); "OK" and "Done" for the same act (`MyDevicesBlock.tsx:279`,
`ShareInviteJoin.tsx:97`). Target: the copy rule in "Button".

## Duplicated one-off components to fold

| Becomes | Folds |
|---|---|
| `Button` (primary, secondary, destructive; bar and panel size) | the 45 outline and 11 primary definitions of I5 to I8, `ui/button.tsx` (rewritten), the library CSS button rules |
| `IconButton` | about 40 `ICON_BUTTON` / `BUTTON_CLASS` constants of I9 |
| `ToggleButton` | `PanelSwitch`, `ToolbarSwitch`, `CentreSwitch`, the ExportSvg and PagesBlock switches, the 15 pressed toggles of I14 |
| `ToggleGroup` | `PanelRadioGroup`, `ModeSwitch`, AddPageGroup segments, `.library-kind`, ShapeToolbar mode, FixtureBody shape, MoveEntryChip switch |
| `Slider` | `RatioSlider`, `SizeSlider`, PlayBar's slider, `AdjustStrength`'s slider |
| `NumberField` | both `EntryField`s, `BarNumberField`, the inline bar fields of I18 |
| `TextField`, `SearchField`, `RenameField` | I20, I21, `sketchInput`, `.library-search`, `.library-source-form input` |
| `Tooltip` (`ui/tooltip.tsx`) | the 18 own Radix tooltips and the `title` attributes on controls |

## Questions for the customer

Frage 416 (SWITCH-LOOK): How an on/off setting in the panel looks once the switch is gone
- A: a full-width 244 x 28 button with symbol and label; on = blue ground, white symbol and label.
- B: the label at the left, a 28 x 28 symbol toggle at the right.
- C: keep the switch in the panel, toggle buttons only in the bars.
Recommendation: A (one look in panel and bars, the whole row is the target). Default if unanswered: A.

Frage 417 (BTN-SYMBOL): "All buttons with symbols": which buttons
- A: every button with a label gets a symbol before it, Cancel (`X`), OK (`Check`) and Add (`Plus`) included.
- B: action buttons get one; Cancel, OK and Done stay text only (outlined).
- C: bars icon-only with tooltips, panel symbol + label.
Recommendation: A (what was asked, and one rule). Default: A.

Frage 418 (HINT-KEEP): The four canvas hint chips (keys, handles, edit, Pen) and the slider help
line ("Click to set, double-click to type ...") are operating hints from before the no-hint rule
- A: keep them as they are (accepted behaviour).
- B: remove them all.
- C: remove the slider help line, keep the canvas chips.
Recommendation: A (not part of this request; removing accepted behaviour is your call). Default: A.

Frage 419 (OP-PILL): The operation symbols on the laser canvas
- A: one rounded pill per object holding all its symbols, centred.
- B: one chip per kind as now, but rounded (4 px) and centred.
- C: A plus a count when more than three kinds.
Recommendation: A (what "in a pill" reads as; at most six kinds, no count needed). Default: A.

Frage 420 (CHECK-TOGGLE): The option checks in the entry chips (Copy in the move, transform and
placement chips) and in the Sketch panel (Reference, Fix origin, Expose)
- A: become toggle buttons like the former switches.
- B: stay checkboxes, in the one checkbox look.
Recommendation: A (one way to show "on"; checkboxes remain only for ticking list rows). Default: A.

## Fix plan (Proposal, for the lead and the product owner)

Each step is one commit on `story/ui-consistency`, visible in the demo; the PR comes when all of
them are in. Order by what the customer named first.

1. **Components without call-site changes**: `Button`, `IconButton`, `ToggleButton` and the
   radius, focus and disabled rules in `docs/ui-elements.md`; unit tests for keyboard and ARIA.
2. **Buttons**: every primary, secondary and destructive button onto `Button`, with symbols
   (I3 to I8, I10, I11, I39); the library CSS rules go (I6).
3. **Switches to toggle buttons** (I12 to I15, I23 option checks if Frage 420 is A).
4. **Canvas symbol pill** in `curvyo-render-core/src/operation_symbol.rs` (I1), golden draw-list
   test of centre and radius.
5. **Fields**: `NumberField`, `TextField`, `SearchField`, `RenameField` (I18 to I22); the native
   select and steppers go (I19, I24).
6. **Segmented controls, sliders, pick lists** (I16, I17, I26).
7. **Tooltips, notices, bars, chips** (I27, I28, I30, I31, I32) and the token, type and icon
   clean-up (I33 to I38).
8. UX review in the browser against `docs/ui-elements.md` (one agent, foreground).
