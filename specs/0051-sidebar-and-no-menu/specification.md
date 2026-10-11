# Sidebar: a vertical tab strip, a File tab, and no menu bar

Status: Draft (2026-10-11). The criteria are complete and testable. The ux-engineer confirmed and completed the UX notes on 2026-10-11 (two corrections to criteria 5 and 23, one to criterion 30). Before Ready: the architect answers the points in `adrs.md`, and the customer accepts or vetoes the items marked **Proposal** (all have a default, nothing blocks the build). One branch `story/sidebar-and-no-menu`, one PR, three milestones.
Priority: Should
Origin: Customer (request of 2026-10-11, paraphrased from German): the native menu bar at the top should go away in the long run, because menus make things hard to find and cost too many clicks; Open, Close and the other file actions move into the tabs on the right; the tabs become Blender-like and vertical (a strip of icon tabs along the panel's edge) because the horizontal strip leaves too little room; the collapse and expand symbol goes into that strip; the icons become smaller. Everything marked **Proposal** is mine: unsaved-changes protection, the header word, the version line, the "Saved" notice and the macOS exception. The strip sits on the inner edge next to the canvas, Blender's placement (lead decision, 2026-10-11).

## User value

As a maker I want one fixed column of small icon tabs along the edge of the right panel next to the canvas, with the file actions (New, Open, Save, Save as, Close, Quit) as buttons in its first tab and no menu bar at all, so that every command is one click or one key away, the right panel has room for more tabs than three, and pressing New or Open by mistake cannot throw my work away.

**What the reference tools do, and what we do differently** (from their manuals and behaviour as far as I know; check against the version the customer uses):

- *Blender:* the Properties editor has a column of icon tabs at its left edge, one tooltip each, and only the tabs that apply. Blender keeps its top menu bar (File, Edit, Render, Window, Help) and, on closing with changes, asks in a popup. We take the column and leave the menu bar and the popup.
- *Inkscape and LightBurn:* a menu bar first, File menu with Open, Save, Close; a modal "save changes?" dialog; Inkscape's dialogs sit in a notebook with tabs on top. Both are what the customer finds hard to use. We take their keys (Ctrl+O, Ctrl+S, Ctrl+Shift+S, Ctrl+W) and nothing else of the menu.
- *Illustrator:* a panel dock at the right window edge that collapses to a column of icons. We take the idea that the icon column remains when the panels are hidden: here the strip stays, only the body goes (and the strip then sits at the window edge).
- *What is better here:* no menu to hunt through (the file actions are buttons in a tab, the keys work from anywhere), no popup for unsaved changes (the question is asked inline in the tab, with the dirty state visible in the title, in the strip and in the tab), and a strip that scales to a dozen tabs, which the horizontal strip of `0043` cannot (five tabs of 36 px already fill 196 of 244 px).

**Decisions this spec reverses** (the customer's request is the reason for each): `0043` Question 1 (a horizontal strip in the header row, with the vertical strip "not recommended"), the design-system paragraph "Native OS menu: unchanged, stays native" (2026-10-05, which named moving File into the right panel as its one-line follow-up), the native-menu precedent of `0001` ("Window chrome and menu"), and `0020` criterion 8 (the native Edit menu).

## Words used below

- **Strip:** the fixed column of icon buttons on the inner (canvas-facing, left) edge of the Properties panel, as Blender's Properties editor has it. It holds the collapse control and the tabs.
- **Body:** the 280 px panel with the active tab's content, to the right of the strip. Its content width stays 244 px.
- **Tab:** one of File, Document, Style, Layers, History. **Active tab:** the one whose content the body shows.
- **Project:** the one open document with its file path, if it has one. **Untitled:** a project that was never saved.
- **Dirty:** the project has changes that no saved file holds (criterion 22). **Clean:** it has none.
- **Replacing action:** New, Open, Close, Quit and a window close: anything that ends the open project.
- **Chord:** a key combination with Ctrl (Cmd on macOS), such as Ctrl+S.

## Acceptance criteria

### The strip

1. Given the app window on Linux, Windows or in a browser, then the strip is the column next to the canvas: 32 px wide, from the top of the window body down to the status bar, and the body lies to the right of it, against the window edge. The horizontal tab strip of `0043` in the panel header and the 16 x 48 px collapse tab on the canvas-facing edge of the panel no longer exist. Test: bounding boxes of strip, body and canvas.
2. Given the strip, then from top to bottom it holds the collapse control (criterion 7), a 1 px divider, and the tabs **File, Document, Style, Layers, History** in this order. A tab whose feature has not shipped is not built (Layers until `0050` ships); the order never changes. Each tab is one button with a 28 x 28 px hit area, centred in the strip, 4 px apart, and a 16 px Lucide glyph with a 1.5 px stroke: File `FolderOpen`, Document `File`, Style `Droplet`, Layers `Layers`, History `History`. No visible text. (The glyphs are 2 px smaller and the hit areas 8 px narrower than in `0043`; 28 px is above the 24 px minimum of WCAG 2.2 2.5.8 and equals the height of every other control in the panel.)
3. Given `pointer: coarse` (a tablet), then each tab's hit area is 36 x 36 px and the strip is 40 px wide; the glyphs stay 16 px. Test: media emulation.
4. Given a tab, then its accessible name is "File", "Document", "Style", "Layers" or "History", and its tooltip (Radix, text only, after 400 ms and on keyboard focus, `side="left"`) reads "File", "Document (Shift+Ctrl+D)", "Style (Shift+Ctrl+F)", "Layers (Shift+Ctrl+L)", "History (Shift+Ctrl+H)" (Shift+Cmd on macOS). The File tab names no key: it has none (the keys of the file actions are in criterion 32).
5. Given the body shown, then the active tab is shown pressed by a shape (the `--toolbar-icon-active-bg` ground, as the active tool on the rail), not by colour alone, and no other tab is. Exactly one tab is active at any time. Given the body hidden, then no tab is shown pressed (the pressed ground says that the content is open) and `aria-selected` is false on all; the active tab is still remembered, stays the tab stop and is the one that Show panel restores. The dimmed Style tab (nothing selected, the Pen, the Node tool without a path) behaves as `0043` criterion 4 says: dimmed, `aria-disabled="true"`, not focusable, its tooltip names the reason.
6. Given the keyboard, then the tabs are `role="tablist"` with `aria-orientation="vertical"`, named "Panel"; each is `role="tab"` with `aria-selected` and `aria-controls`; the body is `role="tabpanel"` with `aria-labelledby`. The tablist is one Tab stop (the active tab). **Up** and **Down** move to the previous and next tab and activate it (the content is instant); Home and End jump to the first and last; there is no wrap; Left and Right do nothing; a dimmed Style tab is skipped; Escape returns focus to the canvas and never clears the selection. Tab order in the panel is the screen order: the collapse control, the tablist, the body.
7. Given the collapse control, then it is the first button of the strip, 28 x 28 px, glyph Lucide `PanelRightClose` while the body is shown and `PanelRightOpen` while it is hidden (16 px). Its accessible name and tooltip are "Hide panel" and "Show panel"; it has `aria-expanded` and `aria-controls` (the body). Pressing it shows or hides the body. Nothing else hides the body: Shift+Ctrl+D, F, H and L only show it.
8. Given the body hidden (collapsed), then it is not drawn, out of the tab order and out of the accessibility tree; the strip is still drawn with all tabs and the collapse control, now against the window edge (it moves right by exactly 280 px, its size is unchanged); the canvas region is exactly 280 px wider; and the document stays where it is on screen (the view keeps its origin, as `0007` criterion 39 says).
9. Given the body hidden, when the maker presses a tab with the pointer, then the body is shown with that tab active. Given the body shown, when the maker presses the active tab, then nothing happens (a tab never collapses the body). Given the body hidden and the keyboard, then Up, Down, Home and End only move the focus along the strip (nothing is activated, the body stays hidden), and Enter or Space on a focused tab shows the body with that tab active.
10. Given the maker hides or shows the body, then the choice is saved for the OS user (not for the project, not in the `.curvyo` file, no `format_version` change) and is applied at the next start. Given a first start with nothing saved, then the body is shown. Given New or Open, then the choice does not change. Given the place it is saved to cannot be read or written, then the body is shown, no error appears and nothing else is affected. The same rules hold in the browser build (browser storage). The active tab is not saved (`0043` Out of scope stays).
11. Given the tabs are taller than the strip (a browser window 160 px high), then the tabs scroll vertically inside the strip with a thin scrollbar; the collapse control and the divider stay at the top; a tab that gets keyboard focus is scrolled into view; every tab can be reached. At 800 x 600 all five tabs fit (about 203 px of 546) and nothing scrolls.
12. Given a window of 800 x 600, then strip and body together are 312 px wide, the canvas region is 488 px wide (the viewport 464 px beside the 24 px ruler; collapsed 768 and 744 px), and no component of the body changes size (content width 244 px). This is 32 px less than before (`0043` criterion 18: 520). The ux-engineer measures the Select, Node and shape bars at 464 px and reports before milestone 1 is accepted. Test: the figures, window 800 x 600.
13. **Proposal (the visible word):** given the body, then its header row is 24 px high (28 px in `0043`) and shows the active tab's name at the left (14 px, `--toolbar-icon`: "File", "Document", "Style", "Layers", "History", the word the header had before `0043`) and the subject line right-aligned in the rest (12 px `--panel-muted-fg`, ellipsis, full text in a tooltip). Reason: the icons are smaller and have tooltips only; the word says which tab is open. The File tab's subject line is "Unsaved changes" while the project is dirty and empty otherwise. Default yes; without it the header row goes and the body gains 28 px.
14. Given the rules of `0043` (which tab is active and when it switches: criteria 5 to 11, 13 to 17, 19 and 23 to 25) and the shortcuts of `0043`, `0020` and `0050` (Shift+Ctrl+D, F, H, L: expand, activate, focus), then they hold unchanged with the File tab added, with two additions: the File tab never switches by itself and no selection change leaves it (it is sticky like History and Layers); after New or Open, from any route, the active tab is Document or Style as `0043` criterion 5 says, so a successful Open hands the maker to the drawing.

### The File tab

15. Given the File tab active, then the body shows, from the top: the project block (criterion 16); the buttons **New**, **Open…**, **Save**, **Save as…** and **Close** as 28 px high outline text buttons, two per row at 118 px where they pair (New and Open…, Save and Save as…) and Close alone; a divider; **Quit** (desktop builds only); and a muted 12 px line "Curvyo <version>" with the application's version, for example "Curvyo 0.4.0" (**Proposal**: the one About-like fact a bug report needs; no licence text, no link, no dialog). The buttons are never dimmed or hidden by the state of the project: Save works on a clean project (`0001` criterion 4), Close and New work on an empty one. Absent in the browser build: Save as… and Quit.
16. Given the project block, then it shows the project's file name with its extension (14 px, ellipsis, the full path in a tooltip on desktop), or "Untitled" when it was never saved, and under it a 12 px muted status line: "Unsaved changes" while dirty, otherwise "No changes since the last save", or "Not saved yet" for an untitled clean project.
17. **New.** Given New, when the project is clean, then at once; when dirty, after the check of criterion 25. The result is an empty untitled project exactly as at launch (`0001` criterion 2): no file is written, both undo stacks are empty (`0020` criterion 25), the selection and tool are reset as today, the title is "Curvyo" (criterion 33), and the active tab follows criterion 14.
18. **Open…** Given Open…, when the project is clean, then the OS file dialog opens at once (the browser's file picker in the browser build); when dirty, the check comes first and the dialog opens after Discard or a successful Save. A cancelled dialog changes nothing, and **Discard changes nothing until the new project has opened**: Open…, Discard, cancel in the dialog leaves the project, its path and its dirty state as they were. A chosen file that cannot be opened is refused as `0001` criterion 7 says (the existing error dialog, text unchanged) and the open project stays untouched. A chosen file that opens replaces the project, which is clean, with its path and title set.
19. **Save.** Given a project with a path, then Save writes it there without a dialog (`0001` criterion 4); given an untitled project, then Save does what Save as… does. Given success, then the project is clean, the title loses its marker and the canvas notice slot says "Saved <file name>." (`role="status"`, 3 s; "Downloaded <file name>." in the browser build). Given a failed write, then the existing "Can't save project" dialog appears with its text unchanged and the project stays dirty.
20. **Save as…** Given Save as…, then the OS save dialog opens, the default name is the current file name or `Untitled.curvyo`, and on success the path, the title and the project block show the new file and the project is clean. A cancelled dialog changes nothing.
21. **Close.** Given Close, then the project is ended after the check of criterion 25 and the result is an empty untitled project, the state of a fresh launch, which is the result of New (the product has no start screen and no "no project open" state apart from that one). Ctrl+W does the same (criterion 32). **Question 2:** Close and New give the same result.
22. **Dirty.** Given a project, then it is dirty exactly when the document differs from what the last Save, Save as… or Open (or New) left: every step of `0020` (each row of its table, the document settings, the background, the history wipe) makes it dirty; selection, tool, pan, zoom, the entered group, the active tab, the panel's collapsed state and a held preview do not. Given a change made while a save is being written, then the project is dirty after the write. The safe direction is a rule: a project that differs from its saved file is never shown clean. **Proposal:** given a save, one step and Ctrl+Z, so that the content equals the saved content again, the project is shown clean (`0020` Out of scope expected this); if the architect finds that this costs more than a version comparison, it stays dirty, which is the safe direction.
23. **Dirty indicators.** Given a dirty project, then (a) the window title has the marker (criterion 33), (b) the File tab shows a 6 px filled dot in the glyph's colour with a casing in the ground colour (not `--accent`, which is the pressed ground), inside the button at its upper right, also while the body is hidden, with the accessible name "File, unsaved changes", (c) the File tab's subject line and the project block say "Unsaved changes". Given it becomes clean, all four go in the same frame.
24. Given the File tab, then it contains no popup, menu, `dialog`, `listbox` or `menu` role and no popover. The only layers over the window are the OS file dialogs (Open, Save as) and the two existing error dialogs of `0001`, whose text and behaviour are unchanged.

### Unsaved changes: the inline check

25. **The check.** Given a replacing action on a dirty project, then the five buttons of the File tab are replaced in place by an **inline confirm** (`docs/design-system.md`, "Inline confirm"): a group named by its sentence, "<name> has unsaved changes. Save them before you <start a new project | open another project | close it | quit>? Discarded changes cannot be brought back." ("This project has" for an untitled one), and three equal-width buttons, 76 px, 8 px apart: **Cancel** (left), **Discard** (`--destructive` look) and **Save** (the Apply look of the panel). Keyboard focus is on Cancel; Discard ignores presses for the first 400 ms; Escape and Cancel close the block and do nothing else; nothing else in the project changes while the block shows (the maker can keep editing).
26. Given the check, when the maker presses **Save**, then the project is saved as criterion 19 or 20 says (an untitled project gets the OS save dialog), and only after a successful save does the action go on. A cancelled save dialog leaves the block showing and the action pending, so the maker can choose again. A failed write drops the action: the project stays open and dirty, the block closes and the error dialog shows. Given **Discard**, the action goes on and the changes are dropped (criterion 18 for Open…).
27. Given any route to a replacing action, then it runs the same check, and a clean project runs none: the buttons of the File tab; the chords Ctrl+N, Ctrl+O, Ctrl+W and Ctrl+Q (criterion 32); the window's close button (X) and Alt+F4; Quit; a `.curvyo` that the operating system hands to the running app (macOS double-click or Open With, `0001` criterion 5); Cmd+Q and Quit in the macOS application menu. Given the window is closed while dirty, then the window stays open and the check shows; the app exits only after Discard, or after a successful Save. At launch, with a file handed over by the OS (Linux, Windows), there is no project to protect and no check.
28. Given the check was raised from outside the File tab (a chord, the window close, the OS handing over a file) while another tab is active or the body is hidden, then the body is shown, the File tab is active and the confirm has focus on Cancel; when the maker cancels, the previous tab and the previous hidden or shown state return; when the action completes, criterion 14 applies. Given a second request while a check shows, then it replaces the pending action and the sentence; Ctrl+S, Ctrl+Shift+S while it shows press its Save.
29. Given the browser build, then closing or reloading the page while dirty triggers the browser's own leave-page question (the one popup the page cannot avoid); Save, New, Open and Close behave as above with the file picker and a download in place of the OS dialogs.

### Window title

30. Given the project, then the window title is: "Curvyo" (untitled and clean), "<file name> — Curvyo" (saved or opened and clean; em dash, the file name without path as `0001` says), and the same with a leading asterisk when dirty: "*Curvyo", "*<file name> — Curvyo". The asterisk is Inkscape's and the Linux and Windows marker; a screen reader may say "star" or "asterisk" when the window gets focus, so the page carries the state in text too (criterion 23). macOS keeps the asterisk. In the browser build the page title follows the same rule. Test: edit, save, edit, undo the edit, reopen; the title after each step.
31. Given the first change after a save or open, then the title and the dot change within one frame of the commit (the same frame as the canvas).

### Keyboard

32. Given the chords of the table below with focus on the canvas, the tool rail, the strip, the body or a text field, then each does what its button does; the host takes no chord any more (criterion 38). In a text field the field commits its edit as it does when it loses focus, and then the action runs, so the saved file has the typed value. Test: type 300 into Width, press Ctrl+S without Enter, the saved file is 300 wide.

    | Chord | Action | Linux | Windows | macOS | Browser |
    |---|---|---|---|---|---|
    | Ctrl+N | New | yes | yes | Cmd+N | no (the browser keeps it) |
    | Ctrl+O | Open… | yes | yes | Cmd+O | yes |
    | Ctrl+S | Save | yes | yes | Cmd+S | yes |
    | Ctrl+Shift+S | Save as… | yes | yes | Cmd+Shift+S | yes (the same as Save) |
    | Ctrl+W | Close | yes | yes | Cmd+W | no (closes the tab) |
    | Ctrl+Q | Quit | yes | no (Alt+F4 and the window button) | Cmd+Q, native | no |

33. Given any chord of the table, then it is ignored (no action, no `preventDefault`) while a pointer drag runs in any tool, a typed-entry chip is open or a long operation runs (`aria-busy`); it is handled once per press (key repeat is ignored); it is matched by the typed character, not the key position (QWERTZ: `key: "s"`); and it calls `preventDefault` only when it acts, so the page's own Ctrl+S never opens a browser save dialog. An unfinished Pen path does not block Save (the path is not part of the document); Close, New, Open and Quit drop it as they do today.
34. Given the tooltip of a button in the File tab, then it names the chord that works on this build ("New (Ctrl+N)", "New (Cmd+N)" on macOS, "New" in the browser). The buttons carry no chord text of their own.
35. Given Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y (not macOS) with no menu, then they work as `0020` criteria 1 to 7 say; the History tab's Undo and Redo buttons (`0020` criterion 9) remain the only visible route. The native Edit menu of `0020` criterion 8 is withdrawn, together with its dedupe against a second handler.

### Removing the native menu

36. **Inventory (made first, milestone 2).** Given the items of the native menu in `curvyo-app/src/menu.rs`, then each has a home in the app before the menu goes, and a test lists the menu items and fails when one has none:

    | Menu item (today) | Home after this spec |
    |---|---|
    | File: New (Ctrl+N), Open… (Ctrl+O), Save (Ctrl+S), Save As… (Ctrl+Shift+S) | File tab buttons and the chords of criterion 32 |
    | File: Quit | Quit button of the File tab, Ctrl+Q (Linux), the window's close button; macOS application menu |
    | Edit: Undo, Redo (with the operation name) | Ctrl+Z, Ctrl+Shift+Z; the Undo and Redo buttons of the History tab, whose tooltips name the step (`0020` criterion 9) |
    | Not yet built: Undo and Redo of one object (`0041`), Select All (`0044`) | Ctrl+U and Ctrl+Shift+U, with the History tab's Object scope buttons as the fallback; Ctrl+A in the Select tool |
    | Close (new), Export… (none exists), Recent files (none exists) | Close in the File tab; Export and Recent are Out of scope |

    No Help, About or Settings entry exists today, and none is added except the version line of criterion 15.
37. Given Linux or Windows, then the window has no menu bar: the window body starts under the OS title bar, the Alt key opens nothing, and the canvas region is taller by the height the bar took (to be measured; about 28 px). The browser build loses its "Curvyo demo" bar (36 px, `BrowserBar`), whose New, Open and Save move to the File tab; the message "this browser cannot run the editor" stays, where the editor would be.
38. Given the host (`curvyo-app`), then it builds no menu on Linux and Windows and, on every OS, binds no accelerator that an editor action also uses (the only native accelerators are the platform's own on macOS, criterion 39), so each chord reaches the page once. Test: a press of each chord on each OS runs its action exactly once (no double fire, as `0020` criterion 8 had to check by hand).
39. **macOS (exception, Question 3).** Given macOS, then the menu bar cannot be removed; the app has only the menu the system requires: the application menu "Curvyo" with About Curvyo, Hide Curvyo, Hide Others, Show All and Quit Curvyo (Cmd+Q, which runs the check), and a minimal Edit menu with only what text fields need to work (Cut, Copy and Paste). It has no File menu, no Undo or Redo item, and no item with an accelerator that an editor action also uses. Given this menu, then checked by hand on a Mac (no Mac is in CI): Cmd+Z, Cmd+Shift+Z, Cmd+A with the canvas focused act on the document; Cmd+Z and Cmd+A inside a text field of the panel act on the text; Cmd+C, Cmd+V and Cmd+X work in text fields; Cmd+N, O, S, Shift+S and W do what criterion 32 says, once.
40. Given the documentation, then in the same PR: `docs/design-system.md` (the paragraph "Native OS menu: unchanged", "Properties panel: tabs", "`PropertiesPanel` width", "Viewport and rulers", the keyboard-shortcuts rows for New/Open/Save and the Edit menu, "Inline confirm" gets its three-button variant) and the specs listed in "Amendments" below are brought in line.

## Milestones (one branch, one PR; customer rule 2026-10-09)

1. **The strip.** Criteria 1 to 14 (the vertical strip, the collapse control, the persistence, the header row), with the tabs that exist at that point: Document, Style and History (Layers when `0050` has shipped). The File tab is not built yet, because a tab with nothing behind it is not built (`CLAUDE.md` §5). The panel's frame changes; no behaviour of any tab does. The ux-engineer's measurement of criterion 12 comes with it. Visible: run the app.
2. **File tab and unsaved changes.** Criteria 15 to 35 and 36 (inventory): the File tab with its buttons, dirty tracking, the title, the check, the chords in the frontend, the host's window-close handshake, Quit. The native menu still exists in this milestone but has **no accelerators** (they move to the frontend here, so nothing fires twice) and its items still work with the mouse.
3. **No menu.** Criteria 37 to 40: `menu.rs` and its dispatch go, the Edit menu with `HistoryItems`, `set_history_menu` and the `history-menu` listener go, the browser bar goes, the minimal macOS menu comes, the documentation and the amended specs are brought in line.

The customer sees the slice once, at the end; no stop between milestones.

## Amendments to other specs (made in this PR, each gets "amended by `0051`" in its Status line)

| Spec | What changes |
|---|---|
| `0043` | Criterion 1 (horizontal strip in the header row, 36 x 28 px, 18 px glyphs, three tabs) -> criteria 1 to 4 here; 12 (Left and Right) -> Up and Down, vertical tablist; 18 and 20 (panel 280 px, strip row 28 px, net 4 px of height) -> strip 32 px beside the 280 px body, header row 24 px; 21 (hit area 28 x 28, glyph 16 to 20) -> glyph 16 px; 22 (a collapsed panel shows the collapse tab only) -> criterion 8; Question 1 and "Out of scope: a vertical strip on the outer edge" reopened: the strip is vertical on the inner edge (lead decision, 2026-10-11, Blender's placement). Kept unchanged: 2 to 11, 13 to 17, 19, 23 to 25 |
| `0020` | Criterion 8 and the UX note "Edit menu" withdrawn (criterion 35 here); criterion 9, 25 and 26 unchanged; Out of scope "Unsaved-changes tracking" is delivered here (criterion 22) |
| `0050` | Criteria 1 and 4 (the Layers tab's hit area 36 x 28 px, "four tabs of 36 px ... 156 px", the subject line's 80 px) -> criteria 2 and 13 here; the UX notes' "strip row 36 px" and the list heights at 800 x 600 are recomputed from the 24 px header row; Tab order "collapse tab, strip" -> "collapse control, tablist" |
| `0041` | Criterion 5 and its UX note (Edit menu items for Ctrl+U and Ctrl+Shift+U) -> the fallback for a platform that eats Ctrl+Shift+U is the Undo and Redo buttons of the History tab's Object scope |
| `0044` | Criterion 14 and the line "the native Edit menu has Select All": no menu; Ctrl+A is the route |
| `0001` | "Window chrome and menu", "Keyboard shortcuts (accelerators on native items)", "Title bar states" (the dirty marker), "Precedent set here" and the Out of scope line on unsaved changes: superseded; the file stays as the record |

## Out of scope

- A start screen, a "no project open" state apart from the empty untitled project, recent files, templates, several open projects or windows. **Proposal for later:** a recent-files list in the File tab (needs a place that keeps paths, which is a privacy decision).
- Export and Import of drawings (SVG `0024`, G-code `0029`): each adds its rows to the File tab when it is specified. The formats-file Import and Export of `0045` stay in the Document tab: they are not project files.
- Autosave, crash recovery, a file lock, "file changed on disk" detection, named versions.
- Moving the existing error dialogs ("Can't open project", "Can't save project") inline; their text and behaviour stay.
- Remembering the active tab, the window size or position between sessions.
- A command palette, a `?` shortcut overlay, a Help, Settings or About page (a keyboard-help overlay is already a separate proposal in the design system).
- Reordering, hiding or detaching tabs; labels next to the icons; Ctrl+Wheel over the strip.
- A strip on the outer window edge (Question 1, option B) unless the customer chooses it; a second strip on the left of the canvas.
- Dark theme, mobile layouts beyond criterion 3.
- Any change to the content of the Document, Style, Layers and History tabs.

## Open questions (customer; each has a default, nothing blocks the build)

1. **Which edge of the panel is the strip on?** *A (default, decided by the lead on 2026-10-11 because the customer asked for "more Blender-like"):* the **inner** edge, the left edge of the panel next to the canvas: Blender's placement (its Properties editor has the tabs at its left edge), nearest to the canvas, and DOM order equals screen order. Cost: hiding the body moves the strip 280 px to the window edge, so the maker who wants it back makes a long mouse trip to the collapse control. *B:* the **outer** edge, at the window's right border. The strip never moves: Hide then Show is two clicks in one spot, like Illustrator's icon dock, and a window edge is an easy target for a mouse. Costs: the tabs do not touch the canvas, and the tab order and the visual order differ (strip first in the DOM, last on screen). B is a change of order in one flex row plus criteria 1 and 8; it can be made at the demo.
2. **Close and New (criterion 21).** The customer named Close. In this product Close and New end in the same empty project, so two buttons do one thing. *A (default):* build both, with Ctrl+W for Close. *B (my recommendation if the customer sees it as clutter):* drop Close and keep New; Ctrl+W stays unbound. *C:* give Close its own state (a start screen with Open and New), which is a larger feature.
3. **macOS (criterion 39).** The system draws a menu bar that no app can remove. *A (default):* a minimal native menu on macOS only: the application menu and an Edit menu with Cut, Copy and Paste, and nothing the editor also does. *B:* the same, plus Undo, Redo and Select All items with their accelerators, which keeps text-field Cmd+Z and Cmd+A working natively but needs the same dedupe `0020` criterion 8 needed. The architect decides the mechanism (`adrs.md`); the customer only needs to know that a Mac still has a menu bar with an application menu.
4. **The visible tab name in the header (criterion 13).** *A (default):* shown, 24 px row. *B:* no header row, the body gains 28 px, the subject line moves into the tab content. Recommendation A: the icons are small and have tooltips only.
5. **Unsaved-changes protection (criteria 22 to 29).** The customer asked for Open, Close and New as buttons; losing work to a mistaken click is the cost of that and today New and Open discard silently (`0001` left tracking out). *A (default):* built as written. *B:* no check, as today. Recommendation A.
6. **Quit as a button (criterion 15).** *A (default):* a Quit button in the File tab on desktop builds, below a divider. *B:* only Ctrl+Q and the window's close button. Recommendation A: the menu had Quit, and a maker who cannot find "exit" should find it in File.
7. **Ctrl+Q on Windows (criterion 32).** *A (default):* not bound (Windows apps use Alt+F4). *B:* bound as on Linux.

## UX notes

Confirmed and completed by the ux-engineer, 2026-10-11 (first draft: product owner). The numbers are in `docs/design-system.md`, "Properties panel: tabs" and "Properties panel: File tab". Reference: Blender's vertical tab column on the Properties editor's inner edge; Illustrator's icon dock for the collapsed state; VS Code's activity bar for "tabs stay, body hides".

### The strip

```
canvas | strip (32 px)          | body (280 px)
       |  [>]  collapse        | File                Unsaved changes   header row, 24 px
       |  ---                  |
       |  [F]  File (dot)      | (tab content, 244 px)
       |  [D]  Document        |
       |  [S]  Style           |
       |  [L]  Layers          |
       |  [H]  History         |
```

| Part | Rule |
|---|---|
| Strip | 32 px, `--panel-bg`, full height, a 1 px `--toolbar-icon` at 25% line on its canvas side (it replaces the body's old left border). The line takes pixel 0 of the 32; the 28 px buttons sit at x = 2 to 30, so the margins are 1 px (line side) and 2 px. Top padding **10 px**, not 8: the collapse control's centre is then 24 px below the top, the same line as the header row's centre (12 px body padding, 24 px row), so the control and the tab name read as one row. Order: collapse control (y 10 to 38), 4 px, a 16 px wide 1 px divider (`--toolbar-icon` at 25%), 4 px, then the tabs at a 32 px pitch (28 + 4). About **203 px** high with five tabs; twelve tabs would need about 427 px, which still fits the 546 px panel viewport, so "scales to a dozen" holds |
| Tab states | As `0043`: unpressed `--toolbar-icon` glyph (8.3:1), hover `--editor-accent-hover` ground, pressed `--toolbar-icon-active-bg` ground and `--toolbar-icon-active-fg` glyph (4.5:1 glyph on ground; the ground is 3.3:1 against the panel), dimmed Style at 40%. **Focus ring drawn inside the 28 px box**, not outside with a 1 px offset: the margin is 2 px and the strip scrolls (criterion 11), so an outside ring would be cut at the strip edge. Two tones, because the pressed ground has the colour of the ring: 2 px `--editor-accent` at the box edge, then 2 px `--toolbar-bg` inside it. On the panel and hover grounds the accent ring shows (3.3:1) and the light line vanishes; on the pressed ground the accent ring vanishes and the light line shows (3.3:1 against the blue). The glyph box (x 6 to 22) is clear of the 4 px band. **While the body is hidden no tab is shown pressed** (the pressed ground says "this content is open"); the remembered tab is still the tab stop and the one `Show panel` restores |
| Glyphs | Lucide, **16 px, 1.5 px absolute stroke** (`absoluteStrokeWidth`; without it a 1.5 stroke in the 24 grid renders as 1 px at 16 px and the glyphs lose contrast). File `FolderOpen`, Document `File`, Style `Droplet`, Layers `Layers`, History `History`; collapse `PanelRightClose` / `PanelRightOpen`. At 16 px the six silhouettes differ: a wide folder, a tall page with a folded corner, a drop, a stacked rhombus, a circle with an arrow, a frame with a chevron. The two nearest in meaning, File and Document, differ in outline (landscape against portrait), and File is the first tab and the only one that carries a dot |
| Dirty dot | 6 px filled disc at the upper right of the File button, centre 5 px from the button's top and right edge (so it is inside the 28 px box and never clipped by the strip's scroll area), with a 1.5 px casing in the ground colour (`--panel-bg`, or `--toolbar-icon-active-bg` while File is pressed). **Fill: the glyph colour (`currentColor`)**, 8.3:1 on the panel and 4.5:1 on the pressed ground. The draft's `--accent` dot is withdrawn: `--accent` equals `--toolbar-icon-active-bg`, so on the pressed File tab it was 1:1 and only the casing showed. The dot is a shape badge; the text cues are the accessible name "File, unsaved changes", the tab's subject line and the project block. The disc overlaps the glyph's box by 2 x 2 px; its casing may take a pixel off the glyph's corner, which is accepted and looked at in the build review |
| Tooltips | Radix, 400 ms, `side="left"` (text only), also on keyboard focus. After one tooltip has opened, the next one in the strip opens at once (Radix skip delay 300 ms), so scanning the icons costs one wait, not five. They open over the canvas edge, never over the body; z above the overlay row, so they may cover a bar's right end for a moment (as tooltips cover the rulers) |
| Body hidden | Strip only, 32 px, at the window's right edge; the dot stays. The canvas is 280 px wider and the view keeps its origin. Nothing animates (the whole panel is instant, so `prefers-reduced-motion` needs no case) |
| Touch | `pointer: coarse`: 36 px hit area, 40 px strip, same glyphs. Below the 44 px of Apple and WCAG 2.5.5 (AAA); it matches the Layers rows (40 px) and footer buttons (36 px), and a tablet window is wide enough for the 8 px. No hover on touch, so only the open tab has its name visible (header row); a long-press tooltip is not built and is part of the later touch pass |

**Target size, checked.** WCAG 2.2 SC 2.5.8 (AA) asks for 24 x 24 px or 24 px clear spacing: 28 x 28 with 4 px gaps passes either way. The design system's own scale: panel controls 28 px (value fields, buttons, `ToggleGroup`), row buttons 24 px, rail 40 px. 28 px is the panel's control height, so the strip is not the smallest thing in the panel, and a 36 px target would not fit 32 px. The customer asked for smaller icons; a 20 px glyph in a 28 px box would not shrink the strip. Fitts: the strip is a column of equal targets 4 px apart; with the body hidden it touches the window edge, which makes it faster to hit, not slower. Adjusted: only the focus ring (inside the box) and the dot (colour, position). Not adjusted: 28 and 16.

### Collapse control: the 280 px jump

Hide moves the control from the inner edge to the window edge; Show moves it back. Judged **acceptable**, with these reasons:

- The y position never changes (the top of the strip). Reaching the control after Hide is a horizontal sweep to the window's right edge, which needs no aim in x: the edge stops the pointer. That is the cheap half of Fitts' law.
- A second route does not need the control at all: any tab press, and Shift+Ctrl+D, F, H, L, show the body. Illustrator's dock and Photoshop's panel dock move their collapse chevron the same way when they fold.
- It cannot be double-clicked into a flicker: the control is gone from under the pointer after the first press.
- The alternative (outer edge, Question 1 B) has no jump but puts the tabs away from the canvas and makes DOM order differ from screen order. It stays a demo-time switch (one flex row plus criteria 1 and 8).

The one thing missing is a **key to hide** the body: criterion 7 says nothing else hides it, so a keyboard user has to Tab to the control. Recommended later addition, not in this spec: one chord, which must work on a QWERTZ layout (Ctrl+\ needs AltGr there, so it is no candidate) and in the browser build (Shift+Ctrl+P is Firefox's private window). Left to the customer with the shortcut review (R-SYS-009).

### The File tab (content width 244 px)

```
File                              Unsaved changes      header row, 24 px (name left, subject right)
Poster.curvyo                                          14 px, ellipsis, path in a tooltip
Unsaved changes                                        12 px muted
[ New        ] [ Open...    ]                          118 x 28, 8 px apart
[ Save       ] [ Save as... ]
[ Close      ]
-----------------------------
[ Quit       ]                                         desktop only
Curvyo 0.4.0                                           12 px muted
```

Gaps 8 px between rows, 16 px around the divider, 12 px under the project block and above the version line; about 270 px high with the header row, no scroll at 800 x 600 (a panel viewport of about 546 px; the browser build omits Save as, Quit and the divider). The buttons are the "Fit to content" outline button at 118 px (28 px; 36 px on touch). Hierarchy: no filled button; a filled one would call New the primary action, and Save is the frequent one. The project block is the only thing that tells state, so it is text, not a banner. The tooltips name the chords: "New (Ctrl+N)", "Open (Ctrl+O)", "Save (Ctrl+S)", "Save as (Ctrl+Shift+S)", "Close (Ctrl+W)", "Quit (Ctrl+Q)" (Linux); a button whose chord does not exist on the build names none. Tab order is reading order; there is no initial focus (the maker came by a click or a key on the strip).

### The inline check

The five buttons are replaced in place by the block (the design system's "Inline confirm", three-button variant):

```
Poster.curvyo has unsaved changes. Save them
before you open another project? Discarded
changes cannot be brought back.            12 px, up to four lines
[ Cancel ] [ Discard ] [ Save ]            76 x 28 each, 8 px apart (3 x 76 + 2 x 8 = 244)
```

Cancel on the left and focused (outline); Discard with the `--destructive` look and the 400 ms delay; **Save with the look of Apply** in the Offset section (`--toolbar-icon-active-bg` ground, white label, 4.5:1). Three different looks (outline, red outline, blue fill) so no button is picked by position alone; Discard and Save are neighbours, but Discard cannot be pressed for 400 ms and Save is the one that cannot lose work. The longest label "Discard" is about 50 px at 14 px, inside the 60 px left after 8 px padding. The block is a `role="group"` named by its sentence, not a dialog; the project block above it stays; nothing floats and nothing else moves (the block is reserved at 100 px, the height of the five buttons it replaces: four sentence lines of 16 px, 8 px, 28 px, so the divider, Quit and the version line do not move). After Cancel the five buttons come back with focus on the button that was pressed (a button) or on the canvas (a chord). A check raised from a chord or a window close while the body is hidden or another tab is open shows the body and takes focus (criterion 28); that is a response to the maker's own key, and Cancel restores the previous view.

### Texts

| Event | Text |
|---|---|
| Saved | "Saved Poster.curvyo." (canvas notice slot, 3 s, `role="status"`) |
| Browser | "Downloaded poster.curvyo." |
| Window close while dirty | the block with "...before you quit?" |
| Status line | "Unsaved changes" / "No changes since the last save" / "Not saved yet" |
| Dirty File tab, accessible name | "File, unsaved changes" |
| Collapse control | "Hide panel" / "Show panel" |

### Keyboard

- Tab order is screen order: the collapse control, the tablist (one stop: the active tab, or the last focused tab while the body is hidden), the body.
- **Roving tabindex**: the tab stop moves with focus. Up and Down move and, while the body is shown, activate (the content is instant, so automatic activation costs nothing); Home and End jump; no wrap; a dimmed Style tab is skipped; the arrow keys call `preventDefault` so the strip never scrolls the page. Left and Right do nothing (a vertical tablist).
- While the body is hidden the arrows only move focus (criterion 9); Enter or Space opens the focused tab.
- **No Ctrl+Tab, Ctrl+PageUp or Ctrl+PageDown** for tab cycling: the browser build cannot have them (the browser takes them for its own tabs), and a desktop-only key that the browser build lacks would split the documentation. Direct jumps exist: Shift+Ctrl+D, F, H, L (Shift+Cmd on macOS). The File tab has no jump key by design; its actions have their own chords.
- Escape from any strip control returns focus to the canvas and never clears the selection. Inside the inline check, Escape closes the block first.
- The chords of criterion 32 work from every focus. Shortcut discovery: the tooltips name them; a keyboard-help overlay is a separate proposal.

### How Layers and History fit

They are two more tabs of the same strip, nothing else changes: Layers (`0050`) is the fourth, History (`0020`) the fifth; the order never changes, and an unshipped tab is not built, so the strip is shorter, not gapped. The header row gives both 4 px more list height than `0043` did (24 px against 28 px); the removal of the menu bar adds about 28 px on Linux and Windows. Both stay sticky (criterion 14). Their glyphs, `Layers` and `History`, were already 18 px in `0043` and `0050` and are 16 px here; `docs/design-system.md` has the figures.

### Answers to the design questions

1. **280 px jump:** acceptable (see above). Outer edge only if the customer finds it wrong at the demo.
2. **Bars at 464 px:** the overlay row is 464 - 128 - 12 = **324 px** (356 px at 496 px before this spec). The Node bar (about 300 px) and the polygon/star bar (about 310 px) still fit, with 24 px and 14 px to spare; the Select bar already wraps (three rows at 356 px) and may need a fourth at 324 px. The Pencil bar's width is not known. Rule for milestone 1: no bar overflows or is clipped, and none wraps to more than three rows; if one does, it gets a wrap rule (the Select bar's groups wrap as whole groups), not a smaller control. The measurement stays with the build.
3. **The dot:** corrected, see "Dirty dot".
4. **Save in the three-button block:** the Apply look.
5. **Header row:** keep the name (Question 4, A). The icons are 16 px with tooltips only; on touch there is no tooltip, and the word is then the only name on screen.

### Risks for the build

- Clipping: the strip scrolls, so anything drawn outside a button's 28 px box (ring, dot) is cut. Both are kept inside.
- macOS (criterion 39): a webview without an Edit menu usually loses Cmd+C, Cmd+V, Cmd+X and may lose Cmd+Z and Cmd+A in text fields. The hand test decides; if undo and select-all fail in text fields, the fallback is Option B of Question 3, not a page-side workaround.
- The window-title asterisk is the Linux and Windows convention. macOS shows the document-edited dot in the close button; Tauri has no setting for it, so macOS keeps the asterisk until a platform call is added (architect).

## Links

Requirements: R-EDIT-032, R-SYS-010 (`docs/requirements.md`); R-SYS-009 (all shortcuts get one review at the end)
Builds on: `specs/0043-properties-tabs/`, `specs/0020-undo-redo/`, `specs/0001-project-file-foundation/`
Touches: `specs/0050-layers-and-objects-panel/` (the Layers tab joins the vertical strip), `specs/0041-object-history/`, `specs/0044-editing-quick-wins/`, `specs/0045-document-formats-library/` (its Import and Export stay in the Document tab)
ADRs: `adrs.md`
PR: -
