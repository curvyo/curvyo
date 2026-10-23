# 0077 Font preview: UX notes

ux-engineer, 2026-10-23. Written for Frage 25 to 28 at their defaults (A, A, A, A). Checked against `docs/design-system.md` (rows "Regions", "Window minimum", "Viewport and rulers", "List row", "Slider", "Tag chip", "Favourite star", "Single-line name", "Action notice", "Hidden, not disabled", "Properties panel: Fonts tab" and its `0075`/`0076` amendments), the `0074`, `0075` and `0076` UX notes, and on `story/single-line-fonts` (`67dd251`) `FontSearchRow.tsx`, `FontTreeRows.tsx`, `RatioSlider.tsx`, `PanelStrip.tsx`. Existing rows are cited by name.

The preview adds **one pane, one toggle button, no popup, no key, no hint label and no panel section that comes and goes**. Every control in it reuses a row that already exists; the only new look is the tile.

## 0. Proposed changes to the specification

Status: all six applied by the product owner on 2026-10-23 (last column).

| # | Criterion | Change | Why | Applied |
|---|---|---|---|---|
| P1 | 3, 4 | While a split would leave the drawing narrower than 360px (criterion 3) the **Full view button is not in the tree** (row "Hidden, not disabled"); the pane is full view. This forced full view does not overwrite the stored split/full choice of criterion 6, and widening the window does not switch the view by itself. | A split at 488px would leave the drawing 168px; a button that offers it would offer nothing usable. | Applied in criteria 3 and 4 |
| P2 | 13 | "All tiles of one size are equally high" stays, but the sample area reserves **n lines (1 to 3)**, n computed once per sample, size and column width (§3), not always 3. A face that needs more than n lines ends with "…". | Always 3 lines leaves two empty lines under "Anna & Jonas 2026" in every tile: a grid of mostly white space. | Applied in criterion 13 (with W = max(240px, 5 x size) of §3); **superseded by the customer decision of 2026-10-23: one line per tile, cut off at the tile's edge, no "…"** |
| P3 | 12 | The Size slider's tooltip is the name and value only, "Size: 32 px", without the second "Click to set, double-click…" line of row "Slider". | Customer rule of 2026-10-16 (no hint labels). The older slider tooltips predate the rule; the lead may want a `chore/` to drop that line everywhere. | Applied in criterion 12 |
| P4 | 22 | The missing-glyph fact joins the tile's accessible description: "Anna & Jonas 2026. No glyph for: ä". | The marker is otherwise pointer-only. | Applied in criterion 22 |
| P5 | 5, 22 | Escape in the sample field closes the preview (it does not clear the field, unlike the Fonts tab search: the sample is kept by design). Outside the pane (customer decision 2026-10-23), Escape first takes the canvas's own step (selection, text edit, tool, group); the first Escape that finds nothing to cancel closes the preview. | One Escape meaning inside the pane; outside, the preview is the last thing Escape closes. | Applied in criteria 5 and 22 |
| P6 | new | Switching the Properties panel to another tab, or collapsing it, leaves the preview open; it keeps showing the Fonts tab's listed families. The header's subject line (§2) names that filter. | No popping pane; the maker may change the Fill colour while comparing fonts. | Applied as criterion 26 (appended in a Part F so criteria 1 to 25 keep their numbers) |

## 1. Pane, split and full

| Item | Decision |
|---|---|
| Place | A second pane in the canvas region (row "Regions"), to the right of the drawing's viewport, against the Properties panel strip. The drawing keeps its rulers, tool rail and bars' overlay row, all anchored to its own (narrower) viewport. |
| Divider | 1px `--toolbar-icon` at 25 % on the pane's left edge (the panel strip's edge-line convention). Draggable (criterion 27, customer decision 2026-10-23): a 7px hit area centred on the line, cursor `col-resize`, no other chrome; on hover, drag and keyboard focus a 3px `--editor-accent` line shows. Clamped to one tile column and a 360px drawing; the share of the canvas area is kept with the view state; double-click resets to the default. While a drag moves, the pane slides over the drawing, which keeps one width (beside the narrowest pane) until release; the drawing never resizes per frame (customer report 2026-10-23). `role="separator"`, `aria-orientation="vertical"`, "Font preview width", `aria-valuenow` the pane's share in percent (min one column, max the 360px drawing), `aria-valuetext` "420 px"; Left and Right move it 16px. |
| Open, close | Instant, no slide or fade (chrome never animates). The drawing's view is not re-zoomed: content at the left stays put, content under the pane is simply covered. The drawing's canvas element always spans the whole canvas region and is resized only by the window; the pane lies over its right part, and the session is told the visible width for fitting, centring and its visibility rules (customer report 2026-10-23: every canvas resize flickered on the Linux desktop). The rulers, rail and bars follow the visible part. |
| Grounds | Header `--panel-bg` with a 1px `--toolbar-icon` at 25 % bottom line; grid area `--canvas-bg`; tiles `--tile-bg` (§3). The pane reads as "a page lying on the canvas", not as a second Properties panel. |
| Split width | 35 % of the canvas region (customer decision 2026-10-23, was 30 %), or one full tile column at the current Size if wider: W (§3) + 2 x 12px grid padding + 18px (1px border, 17px scrollbar gutter). The pane widens and narrows with the Size slider. Customer decision 2026-10-23, replaces half the region, at least 320px. |
| Forced full | When a split would leave the drawing under 360px (the drawing's width at the old 720px threshold). |
| Full view | The pane fills the canvas region; rulers, rail and bars of the drawing are hidden with it (criterion 4). The panel stays. |
| Semantics | `role="region"`, `aria-label="Font preview"`. No visible title: the header controls and the tiles say what it is. |
| Focus | Opened by mouse: focus returns to the canvas (as every panel press). Opened by Enter or Space on the Preview button: focus goes to the sample field. Closed while the focus is inside: focus goes to the Preview button if the Fonts tab is shown, else to the canvas. |

## 2. Header

One 28px control height throughout, 8px padding, 8px between rows.

| Width of the pane | Layout |
|---|---|
| **Under 560px** (282px minimum, split at 1280 x 800 and 1920 x 1080 at small sizes) | Two rows, 80px with the line. **Row 1:** sample field (flex, at 339px: 259px), 4, Full view 28 x 28, 4, Close 28 x 28. **Row 2:** "Size" 12px label, 8, Size slider (96px, as built), 4, readout "32 px" 44px tabular; then 12px and the subject line (flex, ellipsis, may shrink to nothing but is never removed). |
| **560px and wider** (full view at 1280 x 800, split on wide windows or large sizes) | One row, 44px with the line: sample field (flex, 200 to 480px), 16, "Size" label, slider 120px, readout 44px, 16, subject line (flex, ellipsis), 8, Full view, 4, Close. Full view and Close stay at the right edge. |
| Sample field | The Fonts tab search field look without the icon: white ground, `rounded-[5px]`, 1px `ring-border`, 14px UI font, 8px inside left; `maxLength=200`; `aria-label="Sample text"`; no placeholder (an empty field shows the family names, criterion 11, which explains itself); a 24px "Clear sample" button (`X` 16px) inside the right edge while it holds text, as the search field. Printable characters typed in the grid land here. |
| Size slider | The Ratio slider (`RatioSlider.tsx`, row "Slider"): 4px track, 14px thumb, 24px target, `useSliderPointer`, double-click to type, `aria-label="Size"`, `aria-valuetext="32 pixels"`, Arrows 4px, Shift 16px, Home 16, End 160, Ctrl+Backspace resets to 32. Tooltip per P3. |
| Subject line | 12px `--panel-muted-fg`, the same words as the Fonts tab: the strip cell and the count, "All · 1,240", "Cuttable · 42", "Cuttable · 3 of 42" with a search. A state label, not a hint. |
| Buttons | 28 x 28 icon buttons, `rounded-[5px]`, glyph 16px, 1.5px stroke, `--toolbar-icon`, hover `--editor-accent-hover`, focus 2px `--editor-accent` ring (the Flip buttons' look). Tooltips Radix, 400 ms, `side="bottom"`. |

### Icons (Lucide, the set in `frontend/`)

| Control | Glyph | Name and tooltip |
|---|---|---|
| Preview (Fonts tab search row) | `LayoutGrid` | "Font preview"; toggle, `aria-pressed`; pressed look of the Link corners toggle (`--toolbar-icon-active-bg` ground, white glyph). Search row becomes: search field (flex, 180px at 244), 4, Preview 28, 4, Add font 28. |
| Full view / Split view | `Maximize2` in split, `Minimize2` in full | "Full view" / "Split view"; a command button whose glyph and name follow the state (the Link corners pattern), no `aria-pressed`. |
| Close | `X` | "Close preview". |
| Missing glyphs (criterion 18) | `TriangleAlert` 14px | §4. |

## 3. Tile

The look the customer asked for comes from type, space and restraint, not from effects: a white specimen card, the sample large and dark, everything else small and quiet.

| Item | Decision |
|---|---|
| Grid | 12px padding, 12px gap, equal columns (`repeat(auto-fill, minmax(W, 1fr))`), **W = max(240px, 5 x size)**, so a 160px size gives one column; virtualised by rows of fixed height (criterion 23). |
| Card | `--tile-bg` (white), 1px `--tile-edge`, `rounded-lg` (8px, the toolbox card radius), no shadow (shadow is reserved for floating chrome, `--panel-elevation-shadow`). Padding 16px left and right, 12px top, 8px bottom. |
| Sample area | **One line** (customer decision 2026-10-23, replaces the 1 to 3 reserved lines of P2): the sample set left aligned as far as the tile is wide, the rest cut off hard at the card's right edge (no wrap, no "…", no word rule; the area runs through the card's right padding to its edge). em = size / cap height of the face (0.7 when unknown); line box 1.2 em, the baseline 0.95 em below its top for every face. Ink `--toolbar-icon` (about 11:1 on white). No clipping of ascenders or descenders: the area extends 0.25 em above and below the line box (the Inkscape bug of cut-off previews). |
| Single-line sample | The row "Single-line name" at the tile's em: inline `<svg>`, `fill="none"`, `stroke="currentColor"`, 1 CSS px, round caps and joins, `vector-effect: non-scaling-stroke`. The hairline is deliberate: it is what the machine follows. |
| Footer | Two rows, 8px above them (customer request 2026-10-23: the tags get a row of their own). **Row 1**, 28px: family name 12px UI font `--panel-muted-fg`, ellipsis, and the star 24 x 28 at the right edge (row "Favourite star", same state, never moves the tile). **Row 2**, 24px, the list's tag line height: every tag as the list's chip with its x, "+N" by the list's fit rule in the tile's inner width, Add tag (+) at its end, the add field in its place while adding (customer decision 2026-10-23). The row is reserved on every tile, a family without tags shows only the +. "+N" lays all chips, wrapped, over the sample area on the tile's background until the pointer leaves the tile or Escape. |
| Height | 12 + sample area + 8 + 28 + 24 + 8, identical for every tile of one size. At 32px (default) about 158px; at 48px about 197px. |
| Hover | `--editor-accent-hover` ground (List row). Cursor `default`, no lift, no scale. |
| Current (criterion 19) | The List row selected look on the card: `--value-fill` ground and a 3px `--accent` bar on the inner left edge, clipped by the 8px radius; hover does not override it; `aria-selected="true"`. The Fonts tab marks the same family. |
| Focus | 2px `--editor-accent` ring inside the card (List row). |
| Loading (criterion 17) | Name and tags in the footer as usual; in the sample area a quiet bar `--tile-skeleton` (`--toolbar-icon` at 8 % on white), one cap height high on the baseline, 60 % of the tile wide, 3px radius; never the sample in another font (customer decision 2026-10-23, replaces the sample in the UI font). No shimmer, no pulse. Swapped in place, height unchanged. The faces of the listed families are read ahead while the page is idle once the Fonts tab lists them, the tiles in view first, so the preview usually opens with every visible tile drawn. |
| Empty sample (criterion 11) | The family name set in its own face in the sample area, one line cut off at the card's edge like a sample (customer decision 2026-10-23). |
| Empty grid | The Fonts tab's texts, 14px `--panel-muted-fg`, 24px from the top of the grid, centred: "Loading fonts…", "No font in Cuttable.", "No font in Cuttable matches "vibs"." No second line. |
| Semantics | `role="listbox"` "Fonts" with `aria-multiselectable="false"`, tiles `role="option"`; name and description per criterion 22 and P4. The star is a sibling button with `tabindex="-1"` as in the Fonts tab (Ctrl+D is its key route). |

## 4. Missing-glyph marker (criterion 18)

| Item | Decision |
|---|---|
| Place | Top right of the card, 8px in from top and right, inside a 24 x 24 hover target on the card's own ground (5px radius), so it stays legible where a long sample runs under it (the one-line sample has no right reserve since 2026-10-23). |
| Look | `TriangleAlert` 14px, 1.5px stroke, `--panel-muted-fg` (5.0:1 on white). Not `--field-invalid`: nothing is wrong yet, the font simply lacks a letter; red stays for "Missing" fonts and refusals. |
| Tooltip | "No glyph for: ä, ß" (at most 10 characters, then "…"), 400 ms, `side="bottom"`. Not focusable; the keyboard gets the same text through the description (P4). |
| Glyph drawing | Whatever the face draws (`.notdef`, `<missing-glyph>`, nothing): no recolouring of the missing letters. |

## 5. Keyboard

Tab order inside the pane: the divider (split view only; first, as it sits between the drawing and the pane), sample field, Size slider, Full view (when present), Close, grid (one stop, roving focus on the current tile, else the first). Keys as criterion 22. Ctrl+D in the grid toggles the star of the focused tile, as in the Fonts tab. No new global key (criterion 1).

## 6. Fit

Canvas region = window width less the panel (32px strip + 280px body); heights less the 24px status bar. Tile heights at size 48 (split rows updated for the 35 % rule, 2026-10-23).

| Window | View | Pane | Header | Grid |
|---|---|---|---|---|
| 800 x 600 | Full (forced, P1) | 488 x 576 | 2 rows, 81px | 1 column of 464px, about 3 tiles visible |
| 1280 x 800 | Split | 339 x 776 (size 32 and 48), 522 x 776 at size 96; drawing viewport 629px (446px at size 96) | 2 rows, 81px | 1 column |
| 1280 x 800 | Full | 968 x 776 | 1 row, 45px | 3 columns of about 306px, 3 rows, 9 to 12 tiles visible |
| 1920 x 1080 | Split | 563 x 1056 (sizes up to 64) | 1 row | 2 columns of about 254px (1 column from size 50) |

## 7. Design-system rows to add (by the implementer with the slice)

New section "Font preview (`0077`)" after "Properties panel: Fonts tab, font library":

| Item | Value |
|---|---|
| Tokens | `--tile-bg` `#FFFFFF`; `--tile-edge` `--toolbar-icon` at 25 % on white (`#CDCDCF`), decorative (the card is identified by its content, exempt from 3:1); `--tile-skeleton` `#EDEDEE`, the loading bar. Light theme only; names ready for a dark swap. |
| Pane, header, tile, marker, icons, keys, fit | §1 to §6 of these notes, one row each, as built. |
| Fonts tab layout (amends) | Search row: search field, 4, Preview 28 x 28 (`LayoutGrid`, toggle), 4, Add font 28 x 28. |
