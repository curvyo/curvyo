# 0098 Page kinds: UX notes

ux-engineer, 2026-11-05. Checked against `specification.md` (criteria 1 to 21), `specs/0063-manufacturing-workbenches/concept-output-pages.md` §2.1, §2.2, §5.1, §7 and sketches a, b, f of `concept-sketches.html`, `docs/design-system.md` (rows "Properties panel: tabs", "Header row", "Machine block", "Inline delete confirm", "Bar overflow", "Hidden, not disabled", Interaction conventions), `PagesBlock.tsx`, `PageFrames.tsx`, `StatusBar.tsx`, `PropertiesPanel.tsx`, `WorkbenchSwitcher.tsx`, `MachineProfileList.tsx` and the UX notes of `0059`, `0066`, `0093`. Customer rules applied: no popups, no operating tips or hint labels (customer rule 2026-10-16: labels name a control or a state, nothing explains), no notice after a success, permit over refuse, symbols with tooltips only where there is no visible word. No new colour token: everything uses `--toolbar-icon`, `--panel-muted-fg`, `--editor-accent`, `--toolbar-bg`. Choices marked (Proposal) are vetoable at the demo.

## 1. The kind chip (one component, four places)

- **Anatomy.** A Lucide glyph at 12px, 1.5px absolute stroke, `currentColor`, 3px gap, then the kind word. Drawing `PenTool` "Drawing", Sketch `DraftingCompass` "Sketch", Laser `Zap` "Laser". Shape and word, never colour alone. The chip is static text: no hover, no focus stop, no tooltip, no press. There is no control that changes a kind (criterion 5), so nothing may look like one.
- **Places and sizes.**
  - Page list row: 11px word, directly after the name on line 1 (section 3).
  - Panel header: 12px word, 1px `--toolbar-icon` at 25% border, `rounded-md`, 2px 6px padding, at the right end of the header line, never truncated; the subject line before it truncates (section 4).
  - Canvas page label: 12px, after the name (section 5).
  - Status bar: 12px, first in the right block (section 6).
- **Accessible name.** The name cell of a row reads "<name>, <kind> page" ("Keychain, drawing page"); the chip glyph and word are `aria-hidden` there so nothing is read twice. In the header the chip is `role="img"` named "<kind> page"; on the canvas and in the status bar it is part of the text line.
- **States.** "Laser (off)": glyph `Zap` in `--panel-muted-fg`, word "Laser (off)". "Unknown kind": glyph `CircleHelp`, word "Unknown kind", name read as "<name>, unknown kind page". The word carries the state; the glyph colour only echoes it.

## 2. Add page

- **Opening.** The Add page button (`Plus`, unchanged place, tooltip and name "Add page") now opens an inline group directly under the page list and above the page-position fields: `role="group"` named "Add page", `--toolbar-bg`, 8px padding, 244px wide minus padding, no popup, no animation. It pushes everything below it down and closes again; the button is `aria-expanded` while it is open. One group at a time; pressing Add page again closes it (Cancel).
- **Content, top to bottom.**
  1. Label "Kind" (12px `--panel-muted-fg`) and the **Kind switch**: `role="radiogroup"` named "Kind", segments of equal width, 28px high (36px coarse), each glyph 14px plus word 13px (Drawing, Sketch, Laser), selected = `--toolbar-icon-active-bg` / `-fg`. One Tab stop, Left and Right move and select (the segmented rule). Only registered kinds appear; Laser is absent (not disabled) when its extension is off. More than three kinds wrap whole segments onto a second row (Bar overflow rule).
  2. Only with Laser selected: label "Machine" and the **profile list**: the inline list of `0026` (`role="listbox"` named "Machine", rows 44px with the name and the work area in 12px `--panel-muted-fg`, no Copy, Pencil or Trash buttons here), at most 4 rows then it scrolls inside itself. Up, Down, Home, End move and select; the last picked profile is preselected and scrolled into view, else Generic laser. Choosing another kind hides the list at once; the choice is kept if Laser is chosen again.
  3. A row of two 28px text buttons, right-aligned: **Cancel** and **Add** (the pair of the Formats inline form, `0045`).
- **Keys.** Focus starts on the selected Kind segment. Tab order: Kind, Machine (Laser only), Cancel, Add. Enter anywhere in the group adds, except on Cancel (it cancels). Escape or Cancel closes the group, changes nothing and returns focus to the Add page button. A press elsewhere in the panel or on the canvas leaves the group open with its draft (the tab-change rule of `0045`).
- **Remembered values (criterion 1).** The last added kind and the last picked machine are kept for the OS user (browser storage in the browser), not in the project file, like the panel-hidden setting. Unreadable storage, a kind that is no longer registered, or a deleted profile: Drawing, Generic laser.
- **Result.** Add closes the group, creates the page as criteria 2 to 4 say, makes it active, scrolls its row into view and moves focus to its name button (the row's one Tab stop). If the page lies wholly outside the canvas viewport the view pans to its centre at the current zoom, never zooms (the `0059` rule). No notice (no notice after a success). The History step reads "Add page", "Add sketch page" or "Add laser page"; one Ctrl+Z removes the page and, for Laser, its machine data.
- **Names.** "Page N", "Sketch N", "Laser N", N the next free number of that prefix (Proposal). Rename is F2 or double-click as before.
- **Presses.** Kind remembered: Add page, Add (2). Another kind: Add page, kind, Add (3). Laser with another machine: 4.

## 3. Page list row

- **Layout** (row 44px, 3px `--editor-accent` bar when active, semibold name, `aria-current`, as `0059`): grip, name cell, Move frame only switch, Delete page. Line 1 of the name cell: the name (truncates with ellipsis, full name in the tooltip as now) and the kind chip, which never shrinks. Line 2, 11px muted tabular: the size ("125 × 75 mm"); on a laser page the machine name first ("Generic laser · 400 × 300 mm"). Reserved columns as before, so nothing shifts on hover or activation.
- **Width check.** At 244px the name cell is about 140px; with the longest chip ("Unknown kind", 12px glyph and word, about 90px) the name keeps about 50px. Proposal: accept for "Laser (off)" and "Unknown kind" (rare, tooltip and header carry the full name), and for Drawing, Sketch, Laser the name keeps about 80px. If the demo finds that too short, the chip moves to line 2 in front of the size; nothing else changes.
- **Interactions unchanged.** Press, F2, double-click, Delete, Alt+Up and Alt+Down, drag by the grip, roving focus, the announcement "Page 3, position 2 of 8". Delete page on a laser page is one press, no confirm (reversible, one undo step restores the machine copy, fixtures and placements, criterion 16).
- **Frame-only switch.** Stays on every kind (Proposal; criterion 16 and `0059` say nothing against it).

## 4. Panel header and the removed switcher

- **Switcher gone (criterion 7).** The row above the tab strip, its Add manufacturing group, and the 32px-strip stack of workbench buttons are removed; The panel is as tall as the tab strip and body again (about 36px more body). The tab strip starts at the top as before the switcher existed. A session that had the Laser tab open shows Document.
- **Header line (row "Header row").** Tab name at the left as before. Right: the subject line (12px `--panel-muted-fg`, ellipsis, tooltip with the full text) followed by the kind chip. On Document the subject is the page name ("Document · Keychain · Drawing"; the matching-format subject of `0030` is dropped there, the pressed preset already shows it). The chip appears on the page-scoped tabs (Document, Style, Layers, Snapping, Grid, Sketches, Parameters) and not on File, History, Fonts and Share, which are project-wide (Proposal).

## 5. Canvas page label

- Text 12px `--toolbar-icon`, above the frame's top-left as now (`PageFrames`): name (semibold on the active page), 6px gap, the kind chip at normal weight. `aria-hidden`, `pointer-events: none` as built; the row carries the reading.
- Narrow frames (low zoom): the label is never wider than its frame. The name truncates first, the glyph stays, the word goes below a frame width of 96px, the whole label goes below 24px. Labels of neighbouring pages therefore never overlap.
- Sketch b of the concept shows the look with a laser page active; the other kinds use the same label.

## 6. Status bar

The right block keeps its place and tabular figures: kind chip first, then " · ", then the text from the session. Drawing and Sketch: "[glyph] Drawing · 125.0 × 75.0 mm". Laser: "[glyph] Laser · Generic laser · 400.0 × 300.0 mm". "Laser (off)" and "Unknown kind": the chip, then the size. The node count and its divider stay left of it. The block updates in the frame of the activation.

## 7. Tools and tabs follow the page

| | Drawing page | Laser page | Sketch page |
|---|---|---|---|
| Tool rail | vector rail, Text included | the same, no extra tool | sketch rail of `0066` (geometry and constraint cards) |
| Tabs | File, Document, Style, Layers, History, Fonts, Snapping, Grid, Share | the same; no Laser tab, no Operations tab yet (`0101`) | the tab set of the Sketch workbench of `0066` as built, Sketches and Parameters included |
| Document tab | Pages, Unit, size and format, Background | Pages, Unit, **Machine**, **Fixtures**, Background | as a drawing page (size is editable) |

- **Same frame.** Rail, strip, body, header chip, label and status bar change in one render from one read of the active page; no frame shows the old rail with the new tabs. No animation, no transition.
- **Tool on a change of kind.** If the active tool does not exist on the new rail, the new kind's Select tool becomes active (Proposal); a tool in progress ends as a tool change does today (text edit commits, an open Pen path ends). A press that changes the kind only activates the page: it draws nothing and starts no gesture. A press with the Select tool on an object also selects it, as any press.
- **Open tab (criterion 9).** The strip shows the new kind's tabs; if the open tab is gone, the panel shows the tab last used on a page of that kind, else Document. Session state, not saved. If the body is hidden, nothing opens.
- **Focus.** A focused control that disappears hands focus to the nearest survivor: a rail button to the canvas, a strip tab to the tab that became active. Focus never lands on `body`. A press on the canvas keeps focus on the canvas; a press on a row keeps it in the list.
- **Selection** is kept across activation (`0059`); the edit-context rule is `0100`.

## 8. Switching pages (criterion 8)

- **Ways:** a press inside the frame, a row press or Enter on it, `Ctrl+Page Down` and `Ctrl+Page Up` (Cmd on macOS). Page Down is the next row in list order, Page Up the previous; they stop at the ends (no wrap, no notice).
- **Scope.** The chord works with focus on the canvas or in the Pages list, not in a text field or during a drag, text edit or chip. With focus in the list it moves focus to the new active row; from the canvas focus stays on the canvas. The new page is revealed as in section 2 (pan, never zoom). A polite announcement reads "<name>, <kind> page, <n> of <total>". Not a history step, not a dirty change.
- **Plain Page Up and Page Down** (Bring forward and backward, Select tool) are untouched: the chord requires Ctrl or Cmd.
- **Browser (conflict, see report).** The design system says Ctrl+Page Up and Ctrl+Page Down cannot exist in the browser build (the browser takes them). Proposal: the chord is desktop only, as Shift+Ctrl+T for Fonts; the browser reaches the same result by the Pages list (Up and Down, Enter) and a press in the frame. No tooltip or label advertises it.

## 9. The laser page's Document tab (criteria 13 to 15)

- **Machine block** (the block of `0026`, moved here): the profile button row with `BookmarkPlus`, the state row "Differs from profile" with Update and **Reset from profile** (a reset is one undo step, criterion 14), then the **Work area** line, then the Origin corner control. The Name, Width and Height fields of the old block are gone.
- **Work area line.** A 28px row, plain text: label "Work area" at the left (12px `--panel-muted-fg`), value at the right (13px tabular, in the document display unit, "400 × 300 mm"). No field border, no lock glyph, no explanation, no Tab stop: it must not look editable. It changes in the frame a profile is picked or reset.
- **Profile list, work area edit (criterion 15).** A user profile's row gets a fourth action, `Ruler` 24 x 28 "Edit work area of <name>", before Trash (not a Tab stop, reached with Right on the row like the other actions). It opens an **in-place sub-row** under that row with two fields, "Width" and "Height" (28px, label inside the left edge, "mm" inside the right, the Bar number field look; range and clamp of `0026` criterion 1, a cut value shows the existing "Height limited to 10000 mm." Action notice). Enter or blur commits, Escape reverts and closes. The commit writes the library record: no History step, no change to the current page until Reset. Generic laser has no such button (hidden, not disabled).
- **Pages after a profile edit.** Every laser page using the profile shows the existing "Differs from profile" row; nothing else on the page changes until Reset from profile.
- **Fixtures block** and the **origin marker**: as built in `0093` and `0026`; the block sits after Machine. The Laser tab's header trash (Delete manufacturing) is gone; Delete page in the list replaces it.
- **Background and Unit** are document-wide (`0059`), so they stay on a laser page (Unit after the page list as today, Background last). Criterion 13 does not list them; see the report.

## 10. States

- **Laser workbench off (criterion 18 b).** The row reads "Laser (off)". Document tab: Pages, then one Notice block (the `0093` Notice look, 12px text, `TriangleAlert` 16px, `--toolbar-bg`, 1px border at 25%, `role="status"`) with the text "Laser workbench is switched off", in place of Machine and Fixtures. No size fields, no switch to turn it on here (that stays in File, Extensions). Rail and tabs are those of a drawing page, since its objects are ordinary (Proposal). The page can still be renamed, reordered and deleted. The File tab's Extensions group no longer lists disabled manufacturings with their own trash.
- **Unknown kind (criterion 18 c).** Neutral glyph, "Unknown kind". Document tab: Pages (plus Unit and Background), no size fields, no notice. Rail and tabs of a drawing page; rename, reorder, delete work.
- **Damaged kind register (criterion 20).** Reads as a drawing page: no mark of the damage anywhere.
- **No second kind available** (Laser off, nothing else registered): Kind still shows Drawing and Sketch; the group is never skipped.

## 11. A converted sketch page after opening an old file (criterion 19 b)

- **What the maker sees.** The page list gains a row "<page name> sketch" with the Sketch chip, same size, placed beside the drawing page with the page gap. The drawing page looks as before; its sketch now is a linked clone at its old place with the standard clone marks of `0049` (the `--clone-link` dashed outline and tag "Linked clone", Layers row with the link glyph), so the drawing is visually unchanged.
- **No message.** No notice, banner, dialog or "converted" label (no hints; nothing explains). The file is not marked changed by the conversion (it is part of reading, criterion 19) and it is written at the next save.
- **Active page and view.** The page that was active stays active; the opening view rule is unchanged, so the new sketch page may lie outside the first view and is reached by its row. A laser page from a format-36 manufacturing is listed after the pages in the plane order the conversion gave it, with its chip, Machine and Fixtures as in section 9.

## 12. Keyboard map and focus order (additions)

| Action | Key | Notes |
|---|---|---|
| Next page, previous page | Ctrl/Cmd+Page Down, Ctrl/Cmd+Page Up | desktop only, section 8 |
| Add page group | Enter adds, Escape closes | section 2 |
| Kind switch | Left, Right | segmented rule |
| Machine list | Up, Down, Home, End | selects as it moves |

Panel focus order in the Document tab: collapse control, tab strip, Pages (Add page, Move up, Move down, list, page position), the Add page group when open (it sits between the list and the position fields in screen order and in DOM order), Unit, Machine, Fixtures, Background.

## 13. Review checklist for the implementer

Contrast of the chip in light and dark (tokens only); hit targets 28px (36px coarse), rows 44px; focus ring on the Kind segments and Machine rows inside the box; empty and error states of section 10; undo of Add page in all three kinds removes the page, its focus returns to the previous page's row; with 20 pages of three kinds the row, label and status updates stay inside the 50ms budget of criterion 21; the 200-page list still scrolls and keeps the focused row.
