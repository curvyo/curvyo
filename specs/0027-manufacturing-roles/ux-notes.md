# 0027 Manufacturing roles: UX notes

ux-engineer, 2026-11-04. Checked against `specification.md` (criteria 1 to 24), `adrs.md`, ADR 0040, the built Laser tab (`specs/0093-laser-workbench-shell/ux-notes.md` sections 3 to 6, `specs/0026-machine-profile/ux-notes.md`, `ManifestTab.tsx`, `MachineBlock.tsx`) and `docs/design-system.md` (rows "Schema-rendered workbench tab", "Machine block", "Origin corner control and marker", "Properties panel: Layers tab"; Interaction conventions "Segmented icon control", "No popups in the properties panel", "Hidden, not disabled", "No notice after a success", "Casing over artwork"). Customer rules applied: no popups, inline groups, symbol buttons with tooltips, no operating tips or hint labels, a reversible action never asks, permit over refuse, notices at most 70 characters. Choices marked (Proposal) are vetoable at the demo. The Layers block is host-drawn (`Block::Layers`, `adrs.md` note 1); this file names its look, the schema is not involved.

## 1. Where it lives

- Third group of the Laser tab: **Machine, Layers, Fixtures** (spec criterion 1: below Machine). Nothing in the switcher, the strip, the toolbox or the status bar changes. The overhang notice stays on top. Panel width 244px, controls 28px high, 4px gaps, group heading 14px `--toolbar-icon`.
- The tab already scrolls as one column; the block adds no inner scroll. Machine is about 420px tall, so Layers starts near the fold at 800 x 600. Kept in the spec's order (Proposal; revisit at the demo if makers scroll to it every time).
- Layers carries **view state and manufacturing data only**: it never selects, activates, hides, locks, renames or reorders a layer. Those stay in Vector.

## 2. Block layout, top to bottom

1. **Heading** "Layers" (14px `--toolbar-icon`).
2. **Role view row** (28px): label "Role view" (14px) at the left, a `ToolbarSwitch` right after the label, the same look as the Laser mode row (`role="switch"`, `aria-checked`, name "Role view", no on/off text), default on. Toggling: instant, not an undo step, no notice. Same look as the Laser mode row.
3. **Legend** (16px row, only while Role view is on; hidden, not absent in the tab order, it has no controls): three items 12px `--toolbar-icon`, 12px apart: a 16px sample in the role's token followed by the word, "Cut" (1px solid line), "Engrave" (filled square), "Ignore" (dashed square). Ignore stands for Ignore and "No role", as the output page draws both alike; the rows tell which. `aria-hidden` (the row words and the role buttons carry the same facts for assistive technology). It is the key to the canvas, not a tip.
4. **Notice** (only while a count is above zero): the `Notice` look of the Laser tab (`TriangleAlert`, 12px `--toolbar-icon`, 1px border at 25%, `role="status"`, never `--destructive`). One box, one line per count, 8px below the legend and **above the rows** (Proposal; the spec says "below the rows", but with 200 layers a notice under the list is never seen; flagged in the report). Texts, singular and plural by count: "1 layer has no role and is not output." (38) / "3 layers have no role and are not output." (41), "1 image in a Cut layer is not output." (37) / "2 images in Cut layers are not output." (38). Counted as in criterion 15; the "Objects outside layers" row counts like a layer. It updates live; nothing else in the tab moves except the groups beneath it.
5. **Rows** (section 3), 8px below.
6. Fixtures group follows, 12px below.

**Empty:** heading and one 14px `--panel-muted-fg` line "The drawing has no layers yet." (30), nothing else; the Role view row, the legend and the notice are not in the tree (hidden, not disabled). Loose objects only: the single "Objects outside layers" row, with the rest of the block.

## 3. Layer row

One row per top-level layer, **front-most first** (the Layers tab's order), then, after a 1px `--toolbar-icon` at 25% line with 8px above and below, the "Objects outside layers" row while such objects exist. No zebra, no hover ground (the row is not clickable), no kind glyph, no per-layer colour (Curvyo layers have none, `0050`; the role colour belongs to the output page).

Row, 244px wide, two lines, 8px between rows:

- **Line 1 (28px).** The layer name (14px `--toolbar-icon`, ellipsis, tooltip with the whole name), flexible. At the right, in this order and only when they apply: the **locked glyph** (`Lock` 16px `--toolbar-icon`), the **hidden glyph** (`EyeOff` 16px `--toolbar-icon`; the name goes to `--panel-muted-fg` while the layer is hidden), then, **when the layer has no role**, the muted word "No role" (12px `--panel-muted-fg`). The glyphs are read only: not buttons, no hover, not Tab stops, `role="img"` with the names "Hidden in the drawing" and "Locked in the drawing", tooltip text only (Radix, 400ms, `side="left"`). The Layers tab owns eye and lock; here they only report. A hidden layer in the Layers tab uses the same two glyphs, so the vocabulary is one.
- **Line 2 (28px).** The **role control** (section 4), full width.
- **Caption line (16px, 12px `--panel-muted-fg`, only when it applies, under line 2, 4px gap).** "Hidden in the drawing: not output" (33) for a hidden layer. "Unknown role in file, treated as no role." (41) when the stored value is not cut, engrave or ignore; that row shows "No role" on line 1 as well and no role button is pressed. A row can carry both lines (+32px). Captions are state labels (they name the state and what follows from it), not operating tips.
- Row height 64px, 80px with one caption, 96px with two. Heights are known from the view data, so the list can be windowed (section 8).
- The locked glyph has no caption: locked changes nothing about output (criterion 13) and the role of a locked layer is as editable as any other. (Proposal; the spec lists only the hidden glyph. The lock is shown because the maker sees it in the Layers tab and would wonder whether it blocks the role. Vetoable.)
- **Outside row.** Name "Objects outside layers" (22), no glyphs ever, same role control and caption rules. Its role is stored under the reserved key; the row appears and disappears with the objects.

## 4. Role control

- `role="group"` named "Role, layer Walls" (outside row: "Role, objects outside layers"), described by the row's captions. Three buttons, **Cut**, **Engrave**, **Ignore**, equal width (78px each of 244px, 4px apart), 28px high (36px with a coarse pointer), `rounded-md`. Each shows a 16px glyph at the left, 4px, the word at 14px. Plain buttons with `aria-pressed`, not radios, because pressing the pressed one clears it.
- **Glyphs** (custom, 16px, 1.5px stroke, `currentColor`): Cut a 12px square outline, Engrave a 12px filled square, Ignore a 12px square outline dashed 3 on / 2 off. They are the shapes of the output page looks, so the control teaches the looks.
- **States.** None pressed (No role): all three in the panel's segmented outline look (1px `--toolbar-icon` at 60%). Pressed: `--toolbar-icon-active-bg` ground and `--toolbar-icon-active-fg` text and glyph (shape of the ground plus `aria-pressed`, not colour alone), the same as every segmented control. Hover `--editor-accent-hover`. Focus-visible 2px `--editor-accent` ring with 1px offset. The role colours are not used on the buttons: the pressed ground is the app's selection colour, and a role colour there would compete with the canvas.
- **Press.** An unpressed button sets that role; the pressed button clears it (Proposal of the spec). One undo step "Set role"; the output page shows it in the next frame; "No role" and the notice update with it. No notice for the action (success). Focus stays on the pressed button, also after a clear. Unknown role: pressing any button replaces the value (spec 18).
- The control never refuses: hidden and locked layers take roles.

## 5. The role look on the output page

Applies to every placed leaf while Role view is on, hidden leaves excepted (not drawn, spec 14). Tokens, each at least 3:1 against the output page ground (assumed `--canvas-bg`, see the report):

| Token | Light | On `--canvas-bg` | Dark (swap) | Use |
|---|---|---|---|---|
| `--role-cut` | `#C4122F` | 4.9:1 | `#FF6B6B` | Cut hairline |
| `--role-engrave` | `#8A4B08` | 5.6:1 | `#F0A04B` | Engrave area and line |
| `--role-ignore` | `#737379` | 3.9:1 | `#A0A0A8` | Ignore and No role, dashed |

Dark values measured 5 to 8:1 on `#2A2A2E`; re-measure against the real dark page when the dark theme lands. Red and amber are close for colour-blind makers, which is why the three looks differ by shape and by line weight first.

| Role | Look | Weight |
|---|---|---|
| Cut | Outline only, solid, no fill | 1 screen px at any zoom |
| Engrave, closed areas | Flat fill, no outline; overlaps merge into one area (no compositing, no opacity) | none |
| Engrave, all-open leaf (open path, single-line text) | Solid line | **2 screen px** (Proposal; `adrs.md` note 3 says 1px, which would make it indistinguishable from a Cut line but for the colour) |
| Ignore, No role | Outline only, dashed 4 on / 3 off (the `--dim-reference` pattern), no fill | 1 screen px |

- **Casing (Proposal).** Every Cut line is drawn over a 1px `--selection-casing` casing on each side (3px wide in total, the coloured line stays 1px), and **all Engrave areas are drawn first, then Ignore dashes, then Cut lines on top, whatever the stacking**. Reason: the canonical job is an Engrave layer and a Cut layer around the same shape, and a red hairline on an amber fill measures about 1.4:1 without the casing. Output geometry and order are untouched; this is the picture only (rule "Casing over artwork"). Ignore dashes get no casing.
- **Images.** Engrave: a grey-scale preview at the image's place, drawn with the Engrave areas. Cut, Ignore, No role: the image's frame as a closed outline in that role's look (a Cut frame is a hairline with casing, an Ignore frame is dashed); the image pixels are not shown.
- **Opacity** is ignored (a 0 % leaf is drawn at full strength, spec 13); **locked** looks like any other. Guides, dimensions and fixtures are not touched by the role look (fixtures keep the `0093` look).
- **Switch off:** the output page returns to the `0093` look, the legend leaves, nothing else changes. Switching back is one frame, no animation (nothing for reduced motion to change).
- Nothing on the page is selectable or hit-tested; a press still shows the Action notice "Read-only here. Edit in Vector." and the role is never set from the canvas.
- Dashed and 1px strokes keep their screen weight under a scaled placement (`adrs.md` note 3).

## 6. A selection carried from Vector

- The Vector selection, active layer and entered group are **kept as they are** while the Laser tab is open and come back untouched on return. Laser neither clears nor reads them (`0093` section 3: nothing on the output page is selectable, no select bar).
- The Layers block shows **no** trace of it: no highlighted row, no `aria-current`, no "Select contents", no diamond for a layer holding the selection. A row press does nothing and the name is plain text. Reason: the block answers "what will the laser do with this layer", not "what did I select"; a second selection indicator in a read-only world would read as a control. (Proposal; add a cue only if makers ask to find the layer of what they had selected.)
- Setting a role never changes the selection and, since it is not a drawing change, adds nothing to the Vector History beyond the one "Set role" step; the Vector selection survives undo of it.
- Hidden, locked or renamed in Vector while the Laser tab is open: the row follows at once (spec 16); the selection cannot be the cause of any change here.

## 7. Keyboard and focus

- **Tab order in the Laser tab:** ... Max power, Laser mode, Connection fields, **Role view**, **the role controls (one Tab stop)**, Add fixture, fixture rows. The glyphs, the legend and the notice take no focus.
- **The role controls are one Tab stop with roving focus** (like the fixture list): Tab lands on the button that last had focus, else the pressed button of the first row, else its first button. **Left and Right** move between Cut, Engrave and Ignore of the row and never press (the spec's arrows; a deliberate difference from the segmented rule, because arrows that select would write an undo step per key and the pressed button clears). **Up and Down** move to the same button of the previous or next row, Home and End jump to the first or last row, Page Up and Page Down by 5 rows; no wrap. **Space or Enter** presses (sets, or clears the pressed one). **Delete** or **Backspace** on any button clears the row's role. Tool letters are inactive in the block. Escape returns focus to the canvas and changes nothing. Ctrl+Z and Ctrl+Shift+Z work with focus here.
- **Focus follows the layer.** A reorder in Vector moves the row and the focus with it. If the focused layer is deleted, focus goes to the row that took its place, else the one above, else the Role view switch. A row appearing or disappearing above never moves the focus off its button. Focus is never dropped to the document body.
- **Semantics.** The rows are a `role="list"` named "Layer roles"; each row a `role="listitem"` with `aria-posinset` and `aria-setsize` and the group of section 4. Live regions: the notice is a polite status; a role change is announced by the pressed button itself, no extra region. The window (section 8) keeps the focused row's element in the tree.
- **Coarse pointer:** buttons 36px, rows 72px; nothing needs hover.

## 8. Many layers

- Rows have a fixed height per kind (64, 80, 96px), so the list is windowed inside the tab's scroll area: at most 60 rows in the DOM, spacers above and below, the focused row always kept, a move with Up, Down or Page keys scrolls the target into view. 200 layers stay within the 100 ms budget of criterion 21 because nothing renders off screen.
- No search, no filter, no collapse. Names with the same text are not merged; position tells them apart as in the Layers tab.
- The notice counts and the legend stay at the top while the rows scroll in the tab (they are above the rows, which is one more reason for item 4 of section 2).

## 9. States and checks for the review

- **Roles:** none (all three unpressed, "No role" on line 1), Cut, Engrave, Ignore, Unknown (nothing pressed, caption), stale entry for a deleted layer (nothing shown, nothing rewritten).
- **Layers:** hidden, locked, hidden and locked, outside row appearing and going, empty drawing, 1 layer, 200 layers, long and duplicate names, a layer renamed or deleted from Vector while the tab is open.
- **Output page:** all four looks, a Cut layer over an Engrave layer of one shape, a 5 mm stroke in a Cut layer (one hairline), an image under each role, a 0 % opacity leaf, a hidden layer (absent), Role view off.
- **Contrast:** `--toolbar-icon` 8.2:1 and `--panel-muted-fg` 5.0:1 on the panel (existing pairs); the three role tokens in section 5; the pressed pair is the checked active pair (3.3:1 ground, shape carries the state). Hit targets 78 x 28px, 24px minimum (SC 2.5.8).
- **Undo:** "Set role" is one step; the Role view switch is none; no pressed-again duplicate step.
- **Large documents:** 1,000 objects in 10 layers: the role press and the next frame within the 50 ms budget; the block itself reads no objects.

## Design-system rows added now

In `docs/design-system.md`: three tokens (`--role-cut`, `--role-engrave`, `--role-ignore`), the rows "Layers block (roles)", "Role control" and "Role look on the output page" after "Origin corner control and marker", and a clause in the Layers tab's "Eye, lock" row that the Laser tab now says what hidden does in output. The Layers tab itself still may not say it.
