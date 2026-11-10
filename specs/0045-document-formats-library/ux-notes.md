# UX notes: Document formats library

The original UX notes (2026-10-10) are the section "UX notes" in `specification.md`; they stay valid except where this file amends them. This file holds the amendment below and everything after it. Numbers and component rules: `docs/design-system.md`, "Document formats library", "Inline delete confirm", "Properties panel: Fonts tab".

## Amendment 2026-11-10: Formats panel restyle (customer notes of 2026-11-06, items 1 to 6)

Written by the ux-engineer. Status: **not built.** Criteria 35 to 44 of the specification. Rules that apply to all of it: no popup, no dialog, no hint labels or operating tips, no notice for a successful operation (customer rules; the existing notices "Added <name>." and "Imported ..." are not touched here, see open question 12).

Scope of the customer notes: 1 restyle like the Fonts panel with armed delete and symbol buttons; 2 (added 2026-11-10, answer to the coordinator's question 143) page position fields leave the Pages block; 3 duplicate sizes allowed; 4 the lower bar goes; 5 Grid & Rulers origin picker; 6 no "Opens as", inline rename. Pages auto-layout and object-page ownership are separate stories and are not described here.

### 0. State today (read from the code, 2026-11-10)

| Where | Today |
|---|---|
| `frontend/src/components/panel/FormatList.tsx` | Row of 28 px: star (outline `--toolbar-icon`), apply button, then 14 px `Pencil` and `Trash2` icon buttons (reserved 52 px) that are roving stops. Trash opens `InlineConfirm` (sentence, Cancel, Delete) in place of the row. Group header: fold, name, count, `Pencil`, `Trash2`, Show switch; the pencil opens `GroupEditForm` (Name, "Opens as", Save, Cancel). |
| The lower bar | Under the groups, `FormatList.tsx` lines 588 to 620: a block of three full-width 28 px outline text buttons stacked with 8 px gaps (**Add format**, **Import formats**, **Export my formats**, about 100 px). The add form opens in the place of the first button. This is the bar identified for item 4 (open question 10 has the alternative). |
| `FormatForm.tsx` | Rows Name, Width, Height, Unit, Group (strip), for "New group..." also Group name and **Opens as** (Portrait / Landscape), the switch "Add to quick selection" (add only), Add or Save, Cancel. |
| `GridSection.tsx` | `OriginCornerPicker`: label "Origin", a 56 x 72 px page pictogram with four 14 px round radios on its corners, no tooltips, arrows cycle in reading order with wrap. |
| `PagesBlock.tsx` | `PagePosition`: two number inputs "Page X in mm" and "Page Y in mm" with the unit word, under the page list (design-system row "Add page group" says "above the page position fields"). |

### 1. Formats list like the Fonts tab (items 1, 4)

Only elements that have a Fonts counterpart are taken over. Everything else is unchanged. Items marked **(Proposal)** are my reading of "like the Fonts panel" and can be vetoed at the demo.

| Element | New rule | Fonts counterpart |
|---|---|---|
| Row height, selected look, hover | Unchanged: 28 px, selected row `--value-fill` with the 3 px `--accent` bar, hover `--editor-accent-hover`. | family row |
| Star | 28 x 28 hit, `Star` 16 px, 1.5 px stroke; off = outline `--panel-muted-fg`, on = filled `--toolbar-icon`. Tooltip, name and `aria-pressed` unchanged (criterion 17). | favourite star |
| Edit | `Pencil` 16 px, 1.5 px stroke, `--panel-muted-fg`, 24 x 28 hit (was 14 px). Remains a roving stop: it opens the form for size, unit and group. User formats only. | Add tag / tag cell pencil |
| Delete | **Armed delete**, exactly the `0081` pattern (design-system row "Inline delete confirm"): `Trash2` 16 px, `--panel-muted-fg`, 24 x 28 button, not a Tab stop, `onMouseDown` prevented so focus stays. First press arms: the whole row takes `--destructive-soft`, the trash becomes a 22 x 22 `--destructive` square with a white glyph; no size change. Second press after 400 ms deletes. Disarms silently after 4 s, on Escape (consumed), on a press anywhere but its trash, when focus leaves the row, and when the list changes (fold or unfold, Show switch, a form opens, another tab, panel collapsed). One armed trash in the app at a time. Transition 120 ms, none under `prefers-reduced-motion`. User formats only. | Remove trash of a Curvyo family row |
| Reserved width | 52 px at the row end (edit 24, trash 24, 4 gap) as today, so the size text does not move between built-in and user rows. | |
| Group header type **(Proposal)** | Name 12 px semibold `--panel-muted-fg` with the count at the right, as the Fonts block headers (was 14 px `--toolbar-icon`). Header height stays 28 px because of the Show switch. Fold chevron, Show switch and their names unchanged. | block header |
| Group header controls | Fold, name, count, trash (user groups), Show. The pencil is gone (item 6, section 5). The trash is the same armed trash on the header; the header takes `--destructive-soft` while armed. | |
| Section spacing | Group rows 4 px apart as today; 8 px between the last row and the rule under the list. | |

**Names, tooltips, status text** (the armed text is the name and tooltip, not a visible sentence):

| State | Accessible name and tooltip |
|---|---|
| Idle format | "Delete Key ring" |
| Armed format | "Press again to delete Key ring." |
| Idle group | "Delete group Laser" |
| Armed group | "Press again to delete group Laser. Its 2 formats go too." (singular "Its 1 format goes too."; an empty group: "Press again to delete group Laser.") |
| Status region | One `sr-only` `role="status"` `aria-live="polite"` region at the top of the list, in the tree before the first arming. On arm: the armed text. On delete: "Key ring deleted." / "Group Laser deleted." (sr-only; nothing visible, no notice). On disarm: cleared, silent. |

**Delete effects** are those of criterion 22: the id leaves the favourites, the file is written, the document is not changed. Deleting a group deletes its formats. There is no undo of a library change; the 400 ms guard and the armed look replace the sentence (open question 11 for the group case).

### 2. Symbol buttons replace the lower bar (items 1, 4)

The block under the list is removed completely: nothing follows the last group except the 8 px gap and the rule of the Document tab.

**Header row.** While the list is expanded, the "All formats" row becomes a 244 x 28 container (`div`, not a button; nested buttons are invalid). Left to right:

1. The disclosure button (flex, takes the rest): chevron 12 px, label "All formats" 14 px, muted count at its right, `aria-expanded`, `aria-controls`; hover `--editor-accent-hover` over its own width only.
2. 4 px, then up to three 28 x 28 outline icon buttons, 4 px apart: **Add format** (`Plus`), **Import formats...** (`FileInput`), **Export my formats...** (`FileOutput`). Lucide 16 px, 1.5 px stroke, 1 px `--toolbar-icon` at 60 % border, `rounded-[5px]`, `--toolbar-icon` glyph, hover `--editor-accent-hover`, focus ring 2 px `--editor-accent` (the look of "Add font..." in the Fonts search row). Names confirmed in the installed `lucide-react` (`Plus`, `FileInput`, `FileOutput`; `FilePlus` is not used because in the Fonts tab it means "a file comes in").
3. Tooltips: Radix `Tooltip`, 400 ms, `side="left"`, text only, also on focus. Text equals the accessible name: "Add format", "Import formats...", "Export my formats..." (the ellipsis marks the OS file dialog, as "Add font...").

**When they exist.** Only while the list is expanded; collapsed, the row is the disclosure alone, as today (the default tab looks as before). Not in the tree while the user file is broken (criterion 6). **Export** is not in the tree while there is no user format (it is the last button, so nothing else moves when it comes and goes). Order in the DOM and for Tab: disclosure, Add, Import, Export, then the list body (one Tab stop). Previously the three were stops after the list.

**Add.** Pressed: the add form opens as the first block of the expanded list, directly under the header row and above the first group; the panel scrolls it into view (its top at the top of the body); focus in Name. While the form is open the button shows the pressed look of the Fonts preview toggle (`--toolbar-icon-active-bg` ground, `--toolbar-icon-active-fg` glyph) and has `aria-expanded="true"` and `aria-controls` the form; pressing it again moves focus to Name (one form at a time, as today: it does not open a second one). Opening an edit form replaces the add form as today (the draft is dropped like Cancel). After Add (success) or Cancel the form closes and focus returns to the Add button. The edit form still replaces its row in place.

**Notices.** Failures and the existing notices anchor 4 px under the pressed button, right-aligned to the 244 px content edge, maximum 244 px wide, texts and lifetimes of criteria 20, 25 and 27. One anchor row instead of three.

### 3. Duplicate sizes (item 3)

The form and the list change in only two places:

- The check "Same size as <name>" is gone from Add and Save. The size rules (number, 1 to 100 000 mm) and the name rules (1 to 24 characters, "This group already has <name>") stay. The name is now the only thing that tells two formats of one size apart.
- **Which row is current when two formats match the document.** The first in list order, among groups that are on, is the current format (it is what the subject line names). Only its list row has the selected look and, if it is a favourite, only its quick-selection cell is pressed. A second format of the same size has the normal row look. This is the customer's earlier option B of Question 4, now decided. If the first match is not a favourite and a later one is, no cell is pressed (the rule of criterion 11).

Nothing else on screen changes: no warning, no badge, no "same size" hint.

### 4. Origin picker in Grid & Rulers (item 5)

The Grid & Rulers pane uses the same control as the laser Machine block (design-system row "Origin corner control and marker"; `MachineBlock.tsx` `CornerControl`, `CornerGlyph`, `machineBlockKeys.ts`). One implementation, extracted to a shared component; Grid keeps no copy of its own.

| Part | Rule |
|---|---|
| Row | Label "Origin" 14 px `--toolbar-icon` at the left (the Grid pane's word; the laser pane says "Origin corner"), the control at the right. 60 x 60 px: four 28 x 28 radios in a 2 x 2 grid, 4 px gaps. Replaces the 56 x 72 px pictogram. The row is 60 px high, 12 px less than before. |
| Radios | `role="radio"` in a `role="radiogroup"` named "Origin corner" (unchanged name). Glyph: a 12 px square outline with a filled square in the named corner. Chosen = `--toolbar-icon-active-bg` ground and `-fg` glyph; hover `--editor-accent-hover`; focus ring 2 px inside. Accessible names "Top left", "Top right", "Bottom left", "Bottom right". |
| Tooltip | Radix `Tooltip`, 400 ms, `side="left"`, text = the corner name ("Bottom right"), also on focus. |
| Keyboard | One Tab stop on the chosen corner. Left and Right move within a row, Up and Down within a column, Home = Top left, End = Bottom right; the arrows stop at the edge (no wrap); a move chooses and focuses. The old reading-order cycle with wrap goes. |
| Pointer | A pointer press returns focus to the canvas (Grid's rule, kept; the laser pane keeps focus because it is a form). The shared control takes an optional "return focus" callback. |
| Behaviour | Unchanged: works with the grid hidden, one History step "Change grid", disabled with the rest of the pane during a canvas drag. |
| Edges | Four corners only. The laser control has no edge choices, and neither does this one. |

### 5. The format forms and inline rename (item 6)

**"Opens as" is removed everywhere it is a row of its own:**

- `FormatForm`: choosing "New group..." now adds only the field **Group name** (unique ignoring case, 1 to 24 characters).
- `GroupEditForm` held Name and Opens as; with Opens as gone it would hold one field that inline rename covers. It and the group's pencil button are removed. A user group header has: fold, name, count, armed trash, Show.
- A group keeps its stored `default_orientation` (data unchanged, criteria 2 and 4, `0030` criterion 13: staying inside a group keeps the document's orientation, entering a group uses the group's default). A **new** group is made with `portrait`. Existing groups keep their value, built-in Slides stays landscape. The maker flips with the Portrait / Landscape buttons at the top of the tab. Open question 8 has the alternatives.

**Inline rename of user formats and user groups.**

| Part | Rule |
|---|---|
| Start | Double-click on the name of a user format row or user group header, or **F2** with focus on any control of that row (precedent: F2 in the machine profile list). Built-in formats and groups do nothing (no feedback). Starting disarms an armed trash. |
| Field | The name text is replaced in place by a 24 px text field (white ground, 14 px, 1 px `--editor-accent` border, text selected). On a format row the star, size text, edit and trash stay; the apply button is replaced only by the field. The row keeps 28 px. `aria-label` "Name of Key ring" / "Name of group Laser". |
| Enter | Validates. Valid and changed: writes the library once, the field closes, the name changes in the same frame in the list, the quick selection and the subject line. Unchanged: closes, nothing written. Focus goes to the row's apply button (group: its fold button). |
| Escape | Cancels, consumed (the first Escape never reaches the canvas), nothing written, focus as after Enter. |
| Blur | Valid and changed: commits, like Enter. Invalid: reverts without a notice. Unchanged: closes. (As the page rename and the tag rename: blur saves.) |
| Invalid | The field stays open on Enter: 2 px `--field-invalid` border, `aria-invalid`, the validation chip under it with the texts of criterion 20 ("Enter a name of 1 to 24 characters", "This group already has <name>") and for groups the text of criterion 24; the chip clears on the next keystroke. |
| Id and document | Renaming changes the name only; the id, the favourite state and the file position stay. The document is not changed. No notice. |
| Apply on the first click | The first click of a double-click on a format's name presses apply, as any click does. If the format's size differs from the document's, the document is resized once (one History step, Ctrl+Z reverts) and the rename opens. With the same size nothing is written (criterion 10). Open question 9. |

The add and edit forms keep their Name row (the form is also the way to change the size); renaming does not need the form.

### 6. Pages block: no position fields (item 2)

The two fields "Page X" and "Page Y" with the unit word (`PagePosition`) are removed from the Pages block of the Document tab. Pages will be placed automatically by a separate story (page auto-layout); this story only removes the fields.

- The Pages block shows: page rows (name, kind chip, size, delete, reorder handle), the Add page group. Nothing is added in the place of the fields; the Add page group is now the last element of the block and no line, label or hint refers to a position.
- **The layout does not change until the auto-layout story exists.** Pages keep their stored positions. They are arranged as today by the other two ways the code offers: dragging a page row out of the list onto the canvas (the placing drag, `rowDrag`) and moving the page frame on the canvas (`move_page_to`); reordering the list (drag handle or its keys) changes the list order only and moves nothing (`0059` criterion 5). A new page is placed where Add page places it today.
- The **Unit** row (mm, cm, in) under the Pages block stays: it still sets how Width, Height and every ruler and status value are shown. It was inside the customer's circle; this is a **(Proposal)**, open question 13 (default: stays).
- The session function `set_page_position` and its tests lose their only caller in the UI; whether the core keeps it for the auto-layout story is the implementer's and the architect's call.

### 7. Keyboard map (changed rows only)

| Where | Key | Result |
|---|---|---|
| Tab order | Tab | Disclosure, Add, Import, Export, list body (one stop), then the rest of the tab. |
| List body, format row | Left / Right | star, apply, edit (the trash is not in the set). User group header: fold, Show. |
| List body | Up / Down / Home / End | Between rows and headers as today. |
| Any control of a user format row or user group header | `Delete` | Arms the trash of the row or header (focus stays); a second `Delete` after 400 ms deletes. `Enter` and Space keep the meaning of the focused control (apply, star, edit, fold); they do not delete. After a keyboard delete focus goes to the next row's apply button, else the previous row's, else the disclosure. |
| Armed row | `Escape` | Disarms, consumed. |
| Armed row | Up, Down, Home, End, Tab | Focus leaves the row, which disarms; the key then acts as usual. |
| User format row or header | `F2` | Starts the inline rename. |
| Rename field | Enter / Escape / Tab | Confirm / cancel / blur (commit when valid). |
| Built-in row | `Delete`, `F2` | Nothing. |
| Grid origin picker | arrows, Home, End | Section 4. |

### 8. Accessible names (new or changed)

"Add format", "Import formats...", "Export my formats...", "Delete Key ring", "Press again to delete Key ring.", "Delete group Laser", "Press again to delete group Laser. Its 2 formats go too.", "Name of Key ring", "Name of group Laser", the origin radios "Top left" to "Bottom right" in the radiogroup "Origin corner". "Edit Key ring" and "Quick selection: A4" are unchanged; "Edit <group>" and "Opens as Portrait / Landscape" are gone.

### 9. Design-system rows that change

All edited in `docs/design-system.md` on 2026-11-10:

1. "Inline confirm", *Used by*: Delete format and Delete group removed (they are armed deletes now).
2. "Inline delete confirm", *Used by*: Delete format, Delete group added.
3. "Document formats library": rows *Group header*, *Format row*, *Add and Edit form*, *Delete*, *Import, Export*, *Keyboard* rewritten; *Header row with symbol buttons* and *Rename* added; "Opens as" removed from the form row.
4. "Origin corner control and marker": *Used by* adds the Grid & Rulers pane.
5. "Add page group": the words "above the page position fields" are replaced; row "Page position" added as removed.
6. `specs/0090-grid/ux-notes.md`: dated pointer under "Origin corner" to this file, section 4.

### 10. Tests that change by design

| Test | Change |
|---|---|
| `curvyo-document-core/tests/format_library.rs` (`fail_same_size.toml` cases near lines 221 and 284, 392, 398, 494) and the fixture `fail_same_size.toml` | The same-size rule no longer fails a user file or an add or edit; the fixture becomes a valid fixture "two formats of one size" (round trip keeps both). Name-in-group tests stay. |
| `curvyo-ui-core/src/format_form.rs` (line 464 and the `SameSizeAs` message) | The message "Same size as <name>" is removed with the reason. |
| `curvyo-editor-wasm/tests/document_formats_session.rs` (line 146) and `acceptance_0045_tester.rs` (lines 530, 672) | Duplicate sizes are accepted; the current format is the first in list order; a second row is not selected. |
| `curvyo-editor-wasm/tests/acceptance_0045_tester.rs` c24 (line 714) | Group edit no longer sets the orientation; rename of a user group stays (by the inline path). |
| Import fixtures (`import.toml`, `tester_import_in.toml`, `import.expected.toml`) | Skip rule: a format is skipped when its group, name and size match one already there; a same-size format with another name is added. |
| `frontend/tests/grid.test.ts` | The pictogram assertions go; new: 2 x 2 radiogroup "Origin corner", four radios with their names, tooltips, 2D arrow keys without wrap, Home and End. |
| New view tests (`ui-core`) | Rows have no pencil and trash in the roving set; the header row lists Add, Import, Export (Export only with a user format); no Add / Import / Export block under the list; the armed names; rename validation. |
| Frontend / browser check | Armed delete by mouse and by `Delete`; the add form opens under the header row; double-click and F2 rename; Pages block without X and Y at 800 x 600. |

### 11. Open questions (defaults in the specification, "Open questions" 8 to 14)

8 FMT-OPENS-AS, 9 FMT-RENAME-APPLY, 10 FMT-LOWER-BAR, 11 FMT-GROUP-ARM, 12 FMT-NOTICES, 13 FMT-PAGES-UNIT, 14 FMT-FILE-VERSION (architect).
