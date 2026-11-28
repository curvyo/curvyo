# Plan for UI consistency (0164)

Implementer, 2026-11-27. All seven slices of [`fix-plan.md`](fix-plan.md) land in one PR on
`story/ui-consistency` (customer answer 2026-11-27); each slice is one or two commits (shared
component, then the swaps), scope = the area. This plan covers the build order; the tasks name the
findings (`F<n>`) of `fix-plan.md` and the criterion (1 to 7) of `specification.md` they close.
No `format_version` change, no document-model change, no new crate, no new dependency.

Answers built: Frage 452 A (full-width toggle button), 453 A (every labelled button has a
symbol), 455 A (one pill per object), 456 A (option checks become toggle buttons), 457 A (bars
wrap, never past the canvas), 458 A (tooltips keep their text), 459 A (pill centred 6 px above
the object; at the corner while the object shows handles, Proposal), 454 A (hint chips and the
slider help line stay; S7 lists verbose result notices instead).

## Affected crates/modules

- Frontend shared components (`frontend/src/components/ui/`): `button.tsx` (rewritten),
  `icon-button.tsx`, `text-link.tsx`, `toggle-button.tsx`, `checkbox.tsx` (new); later
  `bar.tsx`, `slider.tsx`, `number-field.tsx`, `text-field.tsx`, `search-field.tsx`,
  `rename-field.tsx`, tooltip and notice components.
- Frontend call sites: the panel tabs, the context bars, the Library and font preview, the
  Sketch panel, the workbench blocks, `index.css` (library button rules).
- `curvyo-render-core::operation_symbol` (S3: the symbol pill), `theme.rs` (token names);
  `curvyo-editor-wasm::session::operations::look_overlay` (S3: which items show handles).

## Tasks

### S1 Buttons and symbols (criterion 1)
- [x] 1.1 `ui/button.tsx`: `Button` with `primary`, `secondary`, `destructive`; `size` panel / bar;
  required `icon` for a labelled button; `busy`; `aria-disabled` dims and blocks the press.
  `ui/icon-button.tsx` (`plain` / `outlined`, 24 / 28 / 40), `ui/text-link.tsx` (F3, F5, F6, F7,
  F9, F13)
- [x] 1.2 Add page: framed inline form box, Cancel `X` secondary, Add `Plus` primary (F1, F10)
- [x] 1.3 Bar commands become secondary bar buttons with symbols; Close path group label in the
  muted group-label look (F2)
- [x] 1.4 Panel and Library buttons: outline copies, primary copies and destructive copies move to
  `Button`; library CSS button rules removed; Tiling Apply primary (F3, F4, F5, F6)
- [x] 1.5 Icon buttons: Add page `+`, Pages up/down, page-label toggles, Fonts tag `+`, library
  header actions use `IconButton` (F7)
- [x] 1.6 Every text-only action button gets its symbol (catalogue table), "×"/"+" glyphs go, OK
  not Done, "…" not "..." (F8)
- [x] 1.7 Text links that act (Clear all, Select, Show all) become small secondary buttons;
  real links use `TextLink` (F9)
- [x] 1.8 Panel header page-kind chip becomes plain heading text (F11)
- [x] 1.9 ErrorDialog: app tokens, primary OK with `Check` (F12)
- [x] 1.10 `docs/ui-elements.md` §0 to §5, §17, §19 and design-system rows (fix-plan §4)

### S2 Toggle buttons (criterion 2)
- [x] 2.1 `ui/toggle-button.tsx` on Radix `Toggle` (`aria-pressed`, Enter and Space, return focus,
  armed), panel full-width (Frage 452 A) and bar looks; `ui/checkbox.tsx` tri-state (F14, F18)
- [x] 2.2 Every switch call site (`PanelSwitch`, `ToolbarSwitch`, `CentreSwitch`, the export
  options, Move frame only) becomes a toggle button; the switch components go (F14)
- [x] 2.3 One pressed colour: Into slot caption, Fixture shape, the pale-blue selected toggles
  (F15); Snapping block as four toggle buttons with symbols (F16)
- [x] 2.4 Option checks of the entry chips and the Sketch panel become toggle buttons (Frage 456
  A) (F17); list ticks use `Checkbox` (F18)
- [x] 2.5 Tests that queried `role="switch"` move to `aria-pressed`; docs rows (§6, §8, "Switch")

### S3 Canvas symbol pill (criterion 3)
- [x] 3.1 Tests first in `curvyo-render-core`: pill width `6 + 16n + 4(n - 1)`, symbol centre on
  its cell centre within 0.1 px, pill centred 6 px above the object, corner anchor and its mirror
  while handles are shown, collision skip and cap per pill (F51)
- [x] 3.2 `operation_symbol.rs`: one rounded pill per item (22 px, full radius, 3 px padding, 16 px
  cells 4 px apart, outline inset 0.75 px), 1 px `--lane-line` edge; `ChipItem.handles` set by
  `look_overlay.rs` for the selected items (F51)
- [x] 3.3 Token names on the colour constants in `theme.rs` (I2, F51); canvas chip radii 8 px
  (F52); Measure readout and Aligned in one chip (F53)
- [x] 3.4 Docs: §13 pill, design-system "Spacing and sizing" row, `0101` ux-notes 13.3 pointer

### S3 fix (lead, browser check)
- [x] 3.5 The pill never lies over a transform handle: it moves past every handle glyph it would
  touch, 4 px clear (`chips_list_clear_of`, footprints from `theme.rs`); render-core tests for a
  selected object and a group selection

### S4 Bars, rail and canvas overlap (criterion 4, Frage 457 A)
- [x] 4.1 Empty groups take no room: no empty row, no divider after the last group (F38, F41)
- [x] 4.2 Compact Select bar: commands and scale toggles icon-only with tooltips; Place on…
  symbol; groups shrink and wrap inside the bar (F39, F40)
- [x] 4.3 Generators bar wraps inside its pill; Pencil bar on shared groups (F40, F41)
- [x] 4.4 Pills 8 px, rows 36 px; Node bar Sharp and Smooth with symbols (F44, F2)
- [x] 4.5 Clone tags below the bars (F42); rail scroller takes the wheel (F48)
- [ ] 4.6 One bar anchor (F43), Tile bar looks (F45), Gap names (F46), Test grid code (F47),
  Sketch rail cards (F49), page label under the bar (F50): not built in this pass

### S5 Segmented controls, sliders, pick lists (criterion 5)
- [x] 5.1 One segmented look (`SEGMENT_STRIP`, `SEGMENT_ITEM`): PanelRadioGroup, ModeSwitch, Add page
  Kind, library Items/Sets, Polygon/Star, fixture shape, Test grid strips (F19)
- [x] 5.2 Move chip Relative / Absolute is a radio group, not a switch (F20)
- [x] 5.3 One slider look: `SliderMarks` in the four sliders (F22)
- [x] 5.4 Print and cut Sheet: inline pick list instead of the native select (F23)
- [ ] 5.5 Origin corner ring (F21), Fonts filter layout (F19), pick-list chosen-row looks (F23):
  not built in this pass

### S6 Fields, focus, contrast, panel grid (criterion 6)
- [x] 6.1 Search and tag fields get the visible `--field-border` (F27); library search and source
  form white fields (F27, F28 partly)
- [x] 6.2 No browser steppers (F25); bar field label padding from the measured label (W19, F26 partly)
- [x] 6.3 Sketch field white (F29); Pages row focus ring (F30 partly); one Parameters heading (F36);
  laser Work area on the fields' grid
- [x] 6.4 `--row-hover` in the design-system colour table (F34)
- [ ] 6.5 NumberField / TextField / SearchField / RenameField components (F24, F28), radius and type
  token pass (F31, F32, F33), section headings and label column (F35), Layers row (F37): not built
  in this pass

### S7 Tooltips and notices (criterion 7, Fragen 454 A, 458 A)
- [x] 7.1 One tooltip look `TOOLTIP_CLASS` (shared tooltip, rail, command buttons, Import card, the
  bars' own Radix tooltips) (F54)
- [x] 7.2 One notice radius (8 px) on the canvas slot, action notices, Boolean notice, slot replace
  notice (F57 partly)
- [x] 7.3 Result notices without counts (customer 2026-11-27). **Changes the customer sees
  (Proposal):**
  - Close path: "Closed 3 paths." is gone (no notice on a clean close); when something was skipped
    only the reason shows ("1 path has fewer than 3 nodes and was not closed.").
  - Join: "Join: 2 more outlines moved into this compound path." becomes "Join: the other outlines
    moved into this compound path."
  - Centre lines Apply: "Centre lines: 18 converted, 2 kept filled, 3 skipped." is gone; only
    "Centre lines: some shapes were skipped." and "Linked clones became ordinary paths." remain,
    when they apply.
  - Kept (refusals and warnings, not chatter): every refusal, the skip reasons, the hint chips and
    the slider help line (454 A), operating text in tooltips (458 A).
  - **Found, not changed (Rust texts, for the customer's call):** "Put text on 3 paths."
    (`text_on_path_set.rs`), "Deleted Layer 1 and 5 objects." (`layer_text.rs`), "Set on 2 of 5
    outlines" (`path_options.rs`), "Own tabs on 2 of 5 items" (`tabs.rs`); the Centre lines Apply
    tooltip's preview counts ("70 paths, 1,200 nodes").
- [ ] 7.4 Native `title` attributes to `Tooltip`, `ActionNotice` and `AlertLine` components, Pen
  probe sheet (F55 n/a under 458 A, F56 n/a under 454 A, F58): not built in this pass

### Last pass (lead dispatch 2026-11-28)
- [x] L1 Browser check: the page label is never under a bar: a new view keeps the page top 72 px
  down (`curvyo-ui-core` viewport), and a label whose place a bar covers moves 4 px below the
  measured bar (`hooks/useBarBox.ts`, `pageLabelTop`, Proposal) (F50, criterion 4)
- [x] L2 Clone edge chips follow the measured bar bottom instead of 60 px (F42, criterion 4)
- [x] L3 One anchor for every bar: left, right of the tool rail (F43, Proposal, criterion 4)
- [x] L4 Stroke Width profile is a radio group: arrows move and choose, the preset checked (F19,
  criterion 5)
- [x] L5 Focus ring on the canvas container and the panel tab strip (F30, criterion 6)
- [x] L6 Laser Machine block: "Fixture sets in profile" inside the reserved state row, no blank
  44 px gap above Work area (criterion 6)
- [x] L7 Distribute "Gap" and "Grid gap" (F46); Test grid code with the caption "Test", Passes
  without "×", Crosshatch the shared toggle (F47); Tile bar on the shared strip, toggle, button
  and tooltip looks (F45); Sketch rail in three cards, tooltips "<Tool> tool" (F49); Origin corner
  one 2 x 2 strip, label at the top row (F21); Layers: no inset beside the scrollbar gutter, the
  name tooltip below the row, no native title beside it (F37) (criteria 4 to 7)
- [x] L8 Rust count texts (lead decision, Proposal, "the UI is not verbose"):
  - "Put text on 3 paths." (`text_on_path_set.rs`): gone, the canvas shows the text on the paths.
  - "Deleted layer Cut and 12 objects." (`layer_text.rs`): "Deleted layer Cut."
  - "Set on 2 of 5 outlines" (`path_options.rs`): "Set on some outlines".
  - "Own tabs on 2 of 5 items" (`tabs.rs`): "Own tabs on some items".
  `0050` criterion 36 and `0080` criterion 37 carry the amendment; the tests that asserted the old
  texts follow.
- [x] L9 Built by L11 to L14 in the follow-up pass (2026-11-28; the per-screen re-shoot is the
  lead's browser check): the
  `NumberField` / `TextField` / `SearchField` / `RenameField` components with the label inside
  and their call sites (F24, F28), the radius, type and icon token pass (F31 to F33), one panel
  label column (F35), the remaining native `title` attributes (about 100) to the shared tooltip,
  shared `ActionNotice` / `AlertLine` components (F57), the Pen probe sheet (F58).
- [x] L11 Fields: `NumberField`, `TextField`, `SearchField`, `RenameField` and their call sites
  (F24, F28, criterion 6)
- [x] L12 Radius, type and icon token pass (`GLYPH`, `SMALL_GLYPH`, `RAIL_GLYPH`), one section
  heading and one panel label column (F31 to F33, F35, criterion 6)
- [x] L13 Native `title` attributes on controls to the shared `Tooltip` (F54, criterion 7)
- [x] L14 `ActionNotice` and `AlertLine` in `ui/notice.tsx` and their call sites (F57); the Pen
  probe on the dialog look (F58) (criterion 7)
- [x] L10 Frage 460 B (customer 2026-11-28; criterion 4): kind groups only while every selected
  object is of their kind (`lib/selectBarGroups.ts`, `curvyo-ui-core` `selection_kind_codes`,
  wasm `selection_kinds`); one shared Gap field between Distribute and Grid, Rows and Columns
  with labels inside; Align as two groups of three (Proposal); the narrow class (usable bar
  width below 690 px, Proposal) hides Align, Distribute and Gap below two selected objects.
  Expected at 800 x 640 (324 px): nothing selected 1 row (Scale, Flip and Guide, Tile); a mixed
  selection with a path 4 rows (Scale, Flip and Guide, Align horizontally / Align vertically,
  Distribute / Gap, Grid / Tile, Stroke to path, Linked clone): the shared groups alone need
  about 840 px plus the gaps, so 3 rows is not reachable without dropping or merging a shared
  group. `0009` criteria 21, 22 and `0061` criteria 1, 18 carry the amendment.
- [x] L15 Panel tooltips open left (`docs/ui-elements.md` §15), Output checks on `AlertLine`, long
  font names end with an ellipsis, the full name in the tooltip (criterion 7)

## Decisions

- **(Proposal)** A labelled `Button` takes its symbol as a required `icon` prop (a Lucide
  component), so "every labelled button has a symbol" is enforced by the type checker.
- **(Proposal)** The pill's "handles shown" is the item being selected: `look_overlay.rs` already
  knows the selection, so no handle boxes cross into `curvyo-render-core` (no architect step).
- **(Proposal)** A centred pill that would leave the canvas at the left or right is moved inside
  it (clamped), the corner pill keeps its mirror.

## Validation

- S1, S2: `npx tsc -b --noEmit`, `npx oxlint`, `npm test` (node tests, about 30 s), source-text
  tests updated where they assert old class names or switch roles.
- S3: `cargo nextest run -p curvyo-render-core`, the operation-symbol tests of
  `curvyo-editor-wasm` by filter, clippy on both crates, `npm run build:wasm` once.
- Full gate and browser pass at 1440 x 900 and 800 x 640: the lead, once before the PR.
