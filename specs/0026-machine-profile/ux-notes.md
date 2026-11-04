# 0026 Machine profile: UX notes

ux-engineer, 2026-11-03. Checked against `specification.md` (criteria 1 to 14), `adrs.md`, ADR 0038 and 0039, the built Laser tab of `specs/0093-laser-workbench-shell/ux-notes.md` (sections 3 to 6) with `frontend/src/components/workbench/ManifestTab.tsx`, and `docs/design-system.md` (rows "Schema-rendered workbench tab", "Action notice", "Bar number field", "Reference control"; Interaction conventions "Inline delete confirm" (`0081`), "No notice after a success", "Segmented icon control", "Hidden, not disabled"). Customer rules applied: no popups, inline groups, symbol buttons with tooltips, no hint labels or operating tips, a reversible action never asks, permit over refuse, notices at most 70 characters. Choices marked (Proposal) are vetoable at the demo.

## 1. Where it lives

The Machine block replaces the read-only "Machine" display field of the Laser tab (`0093` section 4, item 2); the overhang notice stays above it, "Fixtures" stays below it. Nothing moves in the Properties panel strip, the toolbox or the switcher. The status bar's right block ("Generic laser · 400.0 × 300.0 mm", `0093` section 3) now shows the copy's name and the output page size, so a "Differs from profile" copy shows its own values. Panel width stays 244px, controls 28px high, 4px gaps, group headings 14px `--toolbar-icon`.

## 2. Block layout, top to bottom

Group "Machine":

1. **Library notice (only while it applies).** A `Notice` look (`TriangleAlert`, `role="status"`) at the top of the group, state not transient: "1 profile could not be read. It was left as it is." (50) and, on its own line, "1 profile is from a newer version. It was left as it is." (56); counts replace the 1 ("3 profiles could not be read..."). Gone after the next start when the records are fine. A notice at start would be missed if the Laser tab is not open, so it lives in the tab.
2. **Profile row (28px).** The **profile button** (flex, 212px: name at the left truncated with an ellipsis, `ChevronDown` 16px at the right, `ChevronUp` while open; `aria-expanded`, accessible name "Machine profile, K40 in the garage"; tooltip "Machine profile") and, 4px after it, the **Save as profile** icon button (28 x 28, `BookmarkPlus`, name and tooltip "Save as profile"). Always present for every state; it makes a new library profile from the values shown (section 5).
3. **State row (28px, only when a state applies).** 12px text `--toolbar-icon`, `role="status"`, at the left; no glyph, the words carry the state.
   - "Differs from profile": at the right two 28 x 28 icon buttons 4px apart, **Update profile** (`BookmarkCheck`) then **Reset from profile** (`RotateCcw`). Update is hidden (not disabled) for the built-in, because it cannot be overwritten (criterion 7). Reset is the only button there.
   - "Not in your profiles": text only, no buttons (Save as profile is in row 2; Update and Reset have nothing to act on, so they are hidden).
   - Neither state: the row is absent and nothing reserves its height.
4. **Profile list (inline, only while open).** Section 3.
5. **Fields**, 8px gap, in this order (the order a maker sets up a laser; schema order, new fields are appended, never inserted):
   1. **Name**: text field 244 x 28, "Name" inside the left edge, 1 to 60 characters.
   2. **Width** and **Height**: the two `0093` mm fields side by side, unchanged look, range 1 to 10,000 mm as in `0093`.
   3. **Origin corner**: label "Origin corner" (14px `--toolbar-icon`) at the left, a **2 x 2 radiogroup** at the right (section 6).
   4. **Max feed**: integer field, "Max feed" inside the left edge, "mm/min" inside the right (12px `--panel-muted-fg`), 1 to 100,000.
   5. **Max power**: integer field, "Max power" inside the left edge, no unit, 1 to 65,535. Its accessible name is "Max power (S-max)".
   6. **Laser mode**: a 28px row, label "Laser mode" at the left, a switch at the right (`ToolbarSwitch` look, `role="switch"`, `aria-checked`, no on/off text), default on.
6. **Group heading "Connection"** (sub-group of the block, same heading look, 12px above it), then:
   - **Connector address** (text, up to 200), **Controller port** (text, up to 200), **Baud rate** (integer 1 to 4,000,000, ADR spec issue 3, default 115200); labels inside the left edge.
   - **Unavailable** (the chosen profile is the built-in, or "Not in your profiles"): the three fields stay in place as read-only fields at 70% (the `0093` display look: `readOnly`, focusable, so the reason is reachable), empty or default, and one 12px `--panel-muted-fg` line under the heading says why: **"Only saved profiles have a connection."** (38; `aria-describedby` on the fields). The line is a state label, not a hint; it is absent when the fields are editable.
   - **Editable** (a user profile that exists in the library): normal fields. They edit the **library record**, not the copy: Enter or blur writes the file at once, Escape reverts, no History step and Ctrl+Z does not touch them (criteria 4, 9). That is why they are a group of their own, below everything that undoes.
   - Never a password, key, test button or status here (`0064`).

All fields except the Connection group edit the **manufacturing's copy**: Enter or blur commits, Escape reverts, one undo step per commit, the work area also moves the output page (`0093` criterion 3). Empty or non-numeric input reverts silently, as `MmField` does today. The History labels are "Pick profile", "Edit <field>" ("Edit max feed"), "Reset from profile" and "Save as profile".

## 3. Picking and managing profiles (inline list)

- **Open and close.** The profile button toggles the list **directly under the profile and state rows**, pushing the fields down (the inline-group exception the Add manufacturing group already uses). It closes on a pick, Escape, a second press of the button, a tab change, a workbench switch or a press on the canvas. Focus on open goes to the chosen row (the first row when the copy's profile is not in the library). No search field: 200 profiles scroll, and the list keeps its position.
- **List.** `role="listbox"` named "Machine profiles", at most 5 rows (220px) visible, then it scrolls inside its own region (no horizontal scroll). Order: Generic laser first, then user profiles by name, case-insensitive (Proposal). A rename or a duplicate moves a row; focus and scroll follow it.
- **Row (44px, List row look).** A 16px slot at the left holds `Check` on the chosen row (`aria-selected`; the slot is kept empty on other rows so names align). Line 1: the name, 14px, truncated, the full name in a tooltip. Line 2: the work area "400 × 300 mm", 12px `--panel-muted-fg`. At the right, right-aligned, three 24 x 28 icon buttons (`ICON_BUTTON` of `ManifestTab.tsx`): **Duplicate** (`Copy`), **Rename** (`Pencil`), **Delete** (`Trash2`). On Generic laser only Duplicate is rendered (hidden, not disabled; the slot is not reserved). Hover `--editor-accent-hover`. The buttons are mouse affordances, not Tab stops; every one has a key (below).
- **Pick.** Enter, Space or a press on the row. One undo step; picking the chosen row is a no-op and adds none (criterion 5). The list closes and focus returns to the profile button. No notice.
- **Duplicate.** Adds "<name> copy" (then "<name> copy 2", ...; cut to 60 characters, suffix kept) to the library, does not pick it, puts the new row into rename (text selected). Library change: no History step.
- **Rename.** Pencil or F2: the name line becomes a 28px text field in the row; Enter or blur commits, Escape reverts, empty reverts, over 60 characters is cut with the notice of section 7. Renaming a library profile makes copies that use it read "Differs from profile" (the name is a copied field, ADR 0039 §5); this is intended (spec criterion 7); Reset takes the new name.
- **Delete.** The `0081` Inline delete confirm, unchanged: first press arms (row `--destructive-soft`, filled `--destructive` trash, tooltip and name "Delete profile. Projects keep their copy."), second press after 400ms deletes, disarms after 4s, on Escape, on another press or when focus leaves the row. A manufacturing that used it keeps its copy and shows "Not in your profiles". The armed state is a polite status. Focus after a delete goes to the row that took its place, else the one above. It is the only confirm in the block because the library has no undo.
- **Keys on the focused row.** Up/Down move, Home/End jump, no wrap; Enter/Space pick; F2 rename; Ctrl+D duplicate (Proposal; the buttons are not Tab stops, so each needs a key); Delete arms, Delete or Enter on the armed row removes (the `0081` rule); Escape closes the list.

## 4. The two states

- **Differs from profile.** Derived (ADR 0039 §5). Shown whenever the library record exists and the name, corner, feed, power, laser mode or page size differ. It appears at once when a field commits and disappears when the values match again (typing the original value back clears it). No field is marked as the differing one in this slice (YAGNI); the maker compares with the list row, which shows the library name and work area.
  - **Update profile** overwrites the library record (id and connection stay), which has no undo, so it uses the armed press of `0081`: first press arms (button filled `--destructive`, name and tooltip "Update profile. Overwrites the saved profile.", polite status), second press after 400ms writes; it disarms after 4s, on Escape, on another press or when focus leaves the button. After the write the state row disappears, no notice, no History step. Other manufacturings, here or in other projects, using that record now read "Differs from profile".
  - **Reset from profile** changes only the manufacturing's copy (page size included), is one undo step and does not ask.
  - After either press the buttons unmount; focus moves to the profile button.
- **Not in your profiles.** The id is unknown (another computer, deleted, or a damaged id). The profile button shows the copy's name as usual, every field is editable (criterion 8), nothing is changed by opening. Row 2's Save as profile adds the copy to the library **under the same id**, so the state clears and the manufacturing is "in sync" (not Differs). The connection group is unavailable until then.
- **Built-in and Differs.** An edited copy of Generic laser shows "Differs from profile" with Reset only; **Save as profile** is how the maker keeps an edit (a new UUID, the manufacturing points at it).

## 5. Save as profile

- Makes a user profile from the values the block shows (copy values and page size; connection empty, default baud), adds it to the library, points the manufacturing at it (the id in the copy) in one History step "Save as profile". Undo restores the old id; the library record stays (adrs.md issue 8).
- Name: the copy's name, except when it equals the name of any profile in the library (the built-in included); then "<name> copy", and if taken "<name> copy 2", "<name> copy 3", ... (spec criterion 3; hand-typed names may still duplicate, but automatic ones never do, so rows stay distinguishable). The maker renames it in the Name field or the list row; because the name sits in the copy and the record, a rename in the **Name field** after saving shows "Differs from profile" until Update profile. That is the cost of the two-place name (copy and record); it is not hidden.
- Values outside the profile ranges (an old file with a 12,000 mm page) are cut to the range with the notice of section 7; the page itself is not changed.
- No success notice.

## 6. Origin corner control and marker

- **Radios.** `role="radiogroup"` named "Origin corner"; four 28 x 28 `role="radio"` buttons in a 2 x 2 grid, 4px gaps (60 x 60px), same order as the corners they name: top-left, top-right, bottom-left, bottom-right. Glyph (custom SVG, 16px): a 12px square outline, 1.5px, with a 5px filled square in the named corner, `currentColor`. Names "Top left", "Top right", "Bottom left", "Bottom right"; tooltip repeats the name. Chosen: `--toolbar-icon-active-bg` ground and `--toolbar-icon-active-fg` glyph (shape plus ground, not colour alone); hover `--editor-accent-hover`; focus ring 2px `--editor-accent` inside. One Tab stop, the chosen radio; arrows move in the grid and select (the segmented-control rule), Home goes to top-left, End to bottom-right. Each change is one undo step; the marker moves at once.
- **Marker on the Laser canvas** (view only, not output, not selectable, not hit-tested): at the chosen corner of the output page, a 6px filled square on the corner point and two 14px arms along the page's two edges at that corner, drawn inside the page, 2px screen lines in `--toolbar-icon` with a 1px `--selection-casing` (white) casing so it reads on the page and on the pasteboard (about 10:1 on white, 4.9:1 on `--pasteboard-bg`). Constant screen size, whatever the zoom; hidden while the page's shorter side is under 48 screen px; drawn above the placed drawing and fixtures, below nothing. It states the machine's origin only: the rulers still show 0 at the page's top-left and no content moves (criterion 11).
- No label, no legend and no tooltip on the marker.

## 7. Notices (Action notice, status look, 3s, 4px below the field, right-aligned, `role="status"`, none after a success)

A value changed to fit its range or rounded is a limit, so it earns a notice; the field shows the value that was set. Format "<Field> limited to <limit>." with the limit in the field's unit:

| Case | Text (characters) |
|---|---|
| Width or Height out of range | "Width limited to 10,000 mm." (27) / "Height limited to 1 mm." |
| Max feed | "Max feed limited to 100,000 mm/min." (35) |
| Max power | "Max power limited to 65,535." (28) |
| Baud rate | "Baud rate limited to 4,000,000." (31) |
| Fraction typed in feed, power, baud | "Max feed rounded to 1,250." (26), same pattern |
| Name over 60 | "Name limited to 60 characters." (30) |
| Address or port over 200 | "Address limited to 200 characters." (34) / "Port limited to 200 characters." |
| Library write failed | "Could not save the profile. Nothing was changed." (48), refusal look, `role="alert"` |

Over-long text is cut at the limit as typed or pasted (no native `maxLength` that drops characters silently). Empty or non-numeric: silent revert, no notice. Saving a profile from a copy with out-of-range values uses the same texts, anchored to the Save as profile button.

## 8. Add manufacturing

A new manufacturing starts with the last-picked profile (Generic laser at first run); its copy, the output page size and the marker are in the new tab with no extra step and no notice. Still one History step "Add manufacturing" (`0093` criterion 2).

## 9. Keyboard and focus

- **Tab order in the Laser tab:** Profile button, Save as profile, Update profile and Reset from profile (when shown), the open list (one stop: the chosen row), Name, Width, Height, Origin corner (one stop), Max feed, Max power, Laser mode, Connector address, Controller port, Baud rate, then the Fixtures group as in `0093`.
- Enter/Escape in every field as in section 2; Escape in the open list closes it and returns focus to the profile button; Escape in a field only reverts that field.
- Focus after a state-row button, a pick or a delete of the last row: profile button (rule in section 3). A button that unmounts never drops the focus to the document body.
- Live regions: the state row, the notices and the armed trash are polite statuses; the write-failure notice is an alert.
- Switch and radios use the existing `ToolbarSwitch` and radio focus rings; coarse pointer 36px as everywhere in the panel.

## 10. Checks for the review

- **Contrast:** all colours are existing tokens. 12px text on `--toolbar-bg` is the checked `--toolbar-icon` pair (8.2:1); `--panel-muted-fg` 5.0:1; the marker ink about 10:1 on white and 4.9:1 on `--pasteboard-bg`; the chosen radio uses the checked active pair (3.3:1 against the panel for the ground, shape carries the state). Hit targets 28px, the row actions 24 x 28 (SC 2.5.8).
- **Empty and error:** library with only the built-in (one row, no state row, the fields editable on the copy); damaged or newer records (item 1 of section 2); write failure (section 7); a profile deleted while chosen ("Not in your profiles", nothing else changes).
- **Undo:** pick, field edit, Reset, Save as profile are one step each; Update (armed press), Duplicate, Rename, Delete and the connection fields are library changes and no step (criterion 4); Reset is undoable and unarmed.
- **Large documents and libraries:** nothing here reads the drawing; 200 rows render within the 100 ms CPU budget (criterion 14), the list scrolls inside 220px.
- **Schema:** the block needs element kinds the manifest does not have (profile picker with list, text and integer fields, corner choice, switch). They are generic (no process name), so the Machine block is a host-drawn block kind `Block::MachineProfile` (`adrs.md` note 4); the manifest gets no new param kinds.

## Design-system rows added now

"Machine block (profile picker)" and "Origin corner control and marker", both new patterns, after the row "Schema-rendered workbench tab" in `docs/design-system.md`.
