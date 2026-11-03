# 0093 Laser workbench shell: UX notes

ux-engineer, 2026-11-02. Checked against `docs/design-system.md` (rows "Properties panel: tabs", "Toolbox card", "Rail tooltip and notice placement", "Action notice", "Status bar", the Interaction conventions "Inline delete confirm" (`0081`), "Bar overflow", "No notice after a success"), `PropertiesPanel.tsx` (today a `TracePresetMenu` named "Workbench", labels "Vector"/"Sketch", "V"/"S" when the body is hidden) and `0063` criteria 1 to 4. Customer rules applied: no popups, symbols with tooltips, no section appears for an operation, permit over refuse, no operating tips or hint labels. Everything in the Laser workbench is rendered from the extension's parameter schema (ADR 0034 block kinds `Params`, `FixtureList`, `Notice`); this file names only generic elements. Choices marked (Proposal).

## 1. Workbench switcher

- **Place and shape.** `role="toolbar"` named "Workbench", the top row of the Properties panel, above the tab strip and body, always visible whichever tab is open or whether the body is hidden. It replaces the "Workbench" menu. 28 x 28 icon buttons, `rounded-md`, 4px apart, 16px Lucide glyphs at 1.5px absolute stroke, `currentColor` (`--toolbar-icon`); same states as the tab strip: hover `--editor-accent-hover`, pressed `--toolbar-icon-active-bg` ground and `--toolbar-icon-active-fg` glyph (shape, not colour alone), focus ring 2px `--editor-accent` inside the box. Row padding 7px sides, 4px top and bottom as the old menu row. Coarse pointer: 36px.
- **Order.** Vector (first, always), Sketch, then the manufacturings in creation order, then the Add manufacturing button last (a new entry appends, so no earlier button moves). A divider (1px, 16px, `--toolbar-icon` at 25%) sits between the workbenches and Add manufacturing.
- **Glyphs (Proposal).** Vector `PenTool`, Sketch `DraftingCompass`, Add manufacturing `Plus`. A manufacturing shows the glyph its process declares in the manifest (Laser: `Zap`); two manufacturings of one process share a glyph and are told apart by name (tooltip and accessible name), never by colour.
- **Names.** Tooltip (Radix, 400ms, 300ms skip delay, `side="left"`, text only) and accessible name are the same text: "Vector", "Sketch", "Laser 1" (the manufacturing's own name), "Add manufacturing". No shortcut text until a chord exists. The panel is 244px wide, so the visible text label of the old menu is gone; the pressed state and the tooltip carry the name. Status bar (section 3) carries it as text.
- **Semantics.** Buttons are `aria-pressed` (exactly one pressed); roving tabindex, one Tab stop (the pressed one), Left and Right move the focus and activate (instant), Home and End jump, no wrap. Add manufacturing is `aria-expanded` while its group is open, not part of the roving set's activation (Left/Right reach it, Enter or Space opens it). Switching dismisses nothing else; open inline forms keep their drafts as in the tab strip rules.
- **Width.** Up to 7 buttons fit on one line at 244px; more wrap by whole buttons onto a second line (Bar overflow rule: no dropdown, no horizontal scroll). The row grows downward, nothing else moves while the count is stable.
- **Body hidden (32px strip).** The same buttons stack in a column above the tab strip, 28px each; the old "V"/"S" letters go. Above 6 entries the column scrolls inside its own region with the pressed button kept in view (Proposal; rare).
- **Migration from the menu.** No stored preference exists, so there is nothing to migrate; a project opens in the workbench it was last left in as today (session state). `TracePresetMenu name="Workbench"` and its `V`/`S` labels are removed in the same change. Existing shortcuts and `switchWorkbench` calls are unchanged; the registry (criterion 11) is the only source of entries, so a dummy workbench shows up with no shell edit.
- **Disabled extension.** A manufacturing whose process is not available (extension off, unknown process) stays in the row at 40% with `aria-disabled`, tooltip "Name (disabled)"; pressing it does nothing and it stays in the Tab order. It can still be deleted (section 7).

## 2. Add manufacturing (inline, no popup)

- Press Add manufacturing: a group opens **directly under the switcher row**, in the panel, `role="group"` named "Add manufacturing", on `--toolbar-bg`, 244px wide, 8px padding. It pushes the tabs and body down by its own height and closes again at once on choice; it is the one reserved-height exception the Inline confirm rule already allows (it grows the content, never floats).
- Content: one full-width text button per available process, 28px high, glyph 16px at the left, process name at 14px ("Laser"), in the manifest's order; a text button "Cancel" below it. No heading, no description, no hint text. Focus goes to the first process button; Escape or Cancel closes and returns focus to Add manufacturing; so do a press on any switcher button, a tab press and a canvas press.
- Choosing a process creates the manufacturing ("Laser 1", the next free number), switches to its workbench, closes the group, moves focus to the new switcher button, and writes one History step ("Add manufacturing"). No success notice. Undo removes manufacturing, output page and fixtures and returns to the workbench that was active before (criterion 2).
- **No process available** (the only extension is off): the Add manufacturing button is not rendered (hidden, not disabled, as for unavailable tabs). With exactly one process the group still shows (customer asked for a choice; consistent when a second process arrives).
- A project with no manufacturing shows only Vector, Sketch, Add (criterion 1): nothing else appears.

## 3. Laser workbench layout

- **Mode and machine always visible.** The pressed switcher button, and the status bar: the cursor readout on the left is unchanged; the right block, which shows the document size in Vector, shows "Generic laser · 400.0 × 300.0 mm" (machine name, work area; Proposal).
- **Canvas.** Shows only the output page: pasteboard `--pasteboard-bg`, page rectangle as the document rectangle look, the fixtures below the placed drawing, then the placed drawing. The drawing's own page borders, the Vector page list and the Vector pages bar are absent. Rulers stay (0 at the output page's top-left). Zoom and Pan are unchanged; the opening view fits the output page. Entering the workbench never changes the Vector view; each workbench keeps its own view.
- **Selection model.** Nothing on the output page is selectable: no handles, no marquee box, no hover highlight, cursor stays the default arrow. Select All and Delete do nothing except the read-only notice (section 5). A fixture is never hit.
- **Toolbox (rail).** One card, "View", `role="toolbar"`: Pan (H) and Zoom (Z) with the existing glyphs, shortcuts and tooltips, plus "Fit page" (the existing zoom-to-page command). No drawing tools, boolean, group or path cards; their shortcuts are inactive here (permit: no notice for tool keys, the keys simply do nothing). The schema adds no cards in this slice. Pan and Zoom stay active tools in the same sense as in Vector; the default tool on entry is the arrow-free view state (neither Pan nor Zoom pressed).
- **Select bar.** None (no selection exists).
- **Panel strip (Proposal).** Laser (first, glyph of the process, name "Laser"), then File and History, which are project-level and unchanged. Document, Style, Layers and Fonts are not rendered (they act on the drawing). Entering the workbench selects the Laser tab; leaving restores the last tab of the target workbench. Question for the customer in the lead's report: should File and History stay? Default: yes.

## 4. Laser tab content (all schema-rendered, generic elements)

Header row as for every tab: tab name "Laser", subject line "Laser 1". From the top:

1. **Notice (only when it applies)**, the overhang notice (section 6), a `Notice` block at the top of the tab.
2. **Group "Machine"** (group heading 14px `--toolbar-icon`, as the Document tab's group headings):
   - Value field "Machine", text, 244px minus padding; value "Generic laser" (display only until `0026`: rendered as a read-only field, 70% text, focusable).
   - Value fields "Width" and "Height" side by side, 28px high, 14px tabular right-aligned, fixed unit suffix "mm" inside the right edge (the "Bar number field" look); defaults 400 and 300; range 1 to 10000 mm, three decimals shown with trailing zeros trimmed. Enter or blur commits, Escape reverts; one undo step per commit (criterion 3). An out-of-range value is clamped and the field shows the clamped value (permit within bounds, no error text).
3. **Group "Fixtures"**: the `FixtureList` block.
   - Heading row: the group name at the left, an icon button `Plus` (28 x 28, tooltip and name "Add fixture") at the right. With one fixture kind the press adds an "Engraved grid" at once; with several kinds it opens the same inline kind group as Add manufacturing (section 2) below the heading row.
   - Empty list: no row, no text (a new manufacturing starts with none).
   - **Row** (List row look, 36px high, `--panel-bg`, hover `--editor-accent-hover`): at the left the kind glyph (16px) and the fixture's name ("Engraved grid", numbered when repeated: "Engraved grid 2"); at the right an **eye button** (24 x 28, `Eye` / `EyeOff` 16px, `aria-pressed` = hidden, name "Hide Engraved grid" / "Show Engraved grid", the Layers eye's look) and a **trash** button (24 x 28, `Trash2` 16px, name "Delete Engraved grid"); one press removes the fixture at once, no arming and no confirm: a reversible action never asks (`docs/design-system.md`), Ctrl+Z restores it (lead decision 2026-11-03, Proposal, vetoable at the demo). Hidden fixtures: name at 60%, `EyeOff`. Both actions are one undo step; no notice.
   - **Selected row expands in place** (one at a time; Enter or press on the row toggles, `aria-expanded`): value fields from the schema, each 28px high with label inside the left edge and unit inside the right (the Bar number field look):
     - "Spacing", mm, default 10, min 0.5 (Proposal; below that the grid is unreadable and slow), max 1000;
     - "Line width", mm, default 0.1, min 0.01, max 10;
     - "Origin X" and "Origin Y" (the origin offset), mm, default 0 and 0, any value within plus or minus the work area size.
     Edits commit on Enter or blur, one undo step each, the output page redraws at once.
   - Keyboard: the rows are one Tab stop (roving; the eye and trash are not Tab stops), Up and Down move between rows, Home and End jump, no wrap; Space on a row toggles its eye, Delete on a row removes the fixture at once and the focus moves to the row that took its place (else the one above, else Add fixture); Enter expands the row.
4. No further groups in this slice. New parameters from the schema are appended as new groups or fields, never inserted above existing ones.

## 5. Read-only notice

- Trigger: pressing on the output page's placed content or fixtures, or Delete, Backspace, Ctrl+A, a nudge key, or a paste while the canvas has focus.
- Text (29 characters): **"Read-only here. Edit in Vector."**
- Look: the Action notice (row "Rail tooltip and notice placement", "Action notice"): DOM, text only, no controls, `pointer-events: none`, no focus, instant in and out; anchored 12px up and right of the pointer for a pointer press, to the canvas centre bottom for a key; shown 2 seconds, repeated presses restart it; announced once through the polite live region. It is a refusal of an edit the maker tried, so it is allowed by "no notice after a success". No permanent banner and no hint label on the canvas.

## 6. Overhang notice

- Shows in the Laser tab whenever the placed page extends past the work area on any side: **"Extends past the work area by 40 x 12 mm"**, numbers are the overhang in x and y, one decimal at most, trailing ".0" dropped, in the display unit (mm shown as above; the sentence has no other variants). Shown when either value is above 0.01 mm (the geometric tolerance); a zero axis reads "0".
- Look: a `Notice` block, 12px text in `--toolbar-icon`, 16px `TriangleAlert` glyph in `--toolbar-icon` at the left (not `--destructive`: this is permitted, not an error), `--toolbar-bg` ground, 1px `--toolbar-icon` at 25% border, `rounded-md`, 8px padding, text wraps to 2 lines, `role="status"` (polite). It is state, so it appears and disappears with the geometry and nothing else in the tab moves except the groups beneath it; it updates live while the drawing or work area changes.
- Nothing is clipped, refused, snapped or moved; the drawn overhang is shown in full across the page edge.

## 7. Extension list entry

- Lives in the **File tab**, a group "Extensions" at its end (Proposal; no Preferences surface exists yet and this is app configuration, reached from the tab that already holds app-level actions). Row (List row look, 44px): name "Laser workbench" at 14px, version "0.1.0" below at 12px `--panel-muted-fg`, and at the right a **switch** (`ToolbarSwitch` look, `role="switch"`, `aria-checked`, name "Laser workbench", no on/off text). Toggling is immediate and a single press; it is not an undo step (app setting, not project data) and writes no notice.
- Off: the Laser switcher entries become disabled (section 1), Add manufacturing no longer offers Laser, the Laser tab and toolbox card are not rendered. Existing manufacturings stay in the file, shown disabled with their name, and save back unchanged. If the active workbench is a now-disabled manufacturing, the app switches to Vector.
- Persistence: for the OS user like the panel-hidden setting (browser storage in the browser), default on; not in the project file.

## 8. Delete manufacturing

- A trash symbol button (Trash2 16px, 24 x 28, name "Delete Laser 1") at the right of the **Laser tab header row**; a disabled manufacturing has it on its row in the File tab "Extensions" group (Proposal).
- One press deletes the manufacturing, its output page and fixtures in one History step, switches to Vector, moves focus to the Vector switcher button, writes no notice. No confirm: the design system says a reversible action never asks, and Ctrl+Z restores all of it and the Laser workbench (lead decision 2026-11-02, Proposal, vetoable at the demo).
- The drawing and its pages are untouched.

## Lead decisions on the open UX points (2026-11-02, Proposal)

- File and History stay in the Laser panel strip; Document, Style, Layers and Fonts are hidden there.
- The extension switch lives in the File tab "Extensions" group until a Preferences surface exists.
- The Vector workbench keeps the name "Vector".

## 9. States and checks for the review

- Keyboard: every control reachable; switcher one Tab stop; Add group focus order process buttons, Cancel; Laser tab order: Delete trash is not a Tab stop (keyboard route is `Delete` on the header), Machine, Width, Height, Add fixture, fixture rows.
- Contrast: all colours are existing tokens; text 12px on `--toolbar-bg` is the checked pair `--toolbar-icon` 8.2:1; hit targets 28px (24px minimum, SC 2.5.8).
- Empty states: no manufacturing (Add only), no fixtures (no row, no text), no placement (page empty; nothing shown).
- Large documents: switching to Laser does not scroll or reset the Vector view; the output page fit is computed from the work area, not from the drawing.
- Undo: add manufacturing, work area edit, fixture add/remove/hide/param edit, delete manufacturing are each one step; extension switch and switcher choice are not.

## Design-system rows to add on acceptance

None added now. After the build the implementer adds one row "Workbench switcher" (section 1), one "Add manufacturing group" (section 2), one "Schema-rendered Laser tab" (sections 4 to 6) and the Extensions group (section 7) to `docs/design-system.md`, and removes the "Workbench" menu mention.
