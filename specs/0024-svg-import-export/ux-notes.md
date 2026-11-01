# 0024 SVG import: UX notes (import slice)

ux-engineer, 2026-10-26. Condenses and amends the "UX notes" section of `specification.md` for the customer rules: no hint labels (2026-10-16, `docs/design-system.md`, "Interaction conventions"), no dialogs of our own, permit over refuse. Export notes stay in the spec ("Export UX notes"). Adds no scope.

## Where import lives
- **Two routes, both in the File tab, no drop.** **Open…** (Ctrl+O) reads `.curvyo` and `.svg` from one dialog; **Import SVG…** (text button, 118 x 28 px, 36 px coarse, look of Open…) adds the file to the open project. No drop of a file onto the canvas (out of scope; the Add font drop is on its tab body and is not extended). No menu, popover or dialog of ours; the OS file dialog is the only layer.
- **Row order:** Close, Template (`0046`, when merged), Import image… (`0058`, when merged), **Import SVG…**, 16 px, divider, 16 px, Quit, version line. Tab order is the visual order. Never dimmed, never hidden. About 306 px body at 800 x 600, no scroll.
- **Consistency with Template.** Both add outlines from another document as one undo step into the active layer, select the result, run no unsaved-changes check and keep no link to the file. Difference stays visible in the name: Template makes guides, Import SVG… makes artwork in a new layer. `0046` writes "Template" without an ellipsis while Open… and Import SVG… have one: finding for `0046`, add the ellipsis there (all three open an OS dialog).
- **Tooltip: the control's name only** ("Import SVG", `side="left"`, 400 ms and on keyboard focus), as Template's is "Template". The sentence in the spec's current note ("Adds the file to this drawing as a new layer, in the middle of the view.") is a hint label and is dropped.

## What the maker sees after Import SVG…
- One new layer named after the file ("poster", cut at 128), on top and active; the file's own layers are groups inside it. Its top-level children are selected, so S, arrows and Delete work at once. Content centred on the visible canvas centre, nothing scaled, view not moved or zoomed (a part larger than the view stays larger; Fit to content is one press away). The File tab stays open; no tab switch.
- One history step "Import SVG"; Ctrl+Z removes all of it and the selection with it. Names and glyphs in the Layers tab follow the spec ("Layers tab and the imported tree").
- **Open of an SVG:** replaces the project as an untitled, dirty project (title "*Curvyo"), empty history, content at the file's own coordinates, the view as after Open of a project (page fitted). The unsaved-changes block of `0051` comes first, unchanged.
- **Focus:** done, focus goes to the canvas. Cancelled dialog: focus returns to the button, nothing shown.

## Notices and refusals (one rule, no counts)
- **No message after a successful import**, whatever was left out. The objects appearing, selected, are the confirmation. No count, no list, no banner (customer 2026-10-16, REPORT-PLACE).
- **One notice** when nothing was imported: "Nothing in poster.svg could be imported." Canvas notice slot (File tab precedent: "Saved Poster.curvyo."), `role="status"`, 3 s, no focus. File name cut at 24 characters, ellipsis in the middle, extension kept.
- **Refusals** (five cases) use the existing error dialog, text per spec table, each ending "Nothing was changed."; focus on OK, Escape closes, focus returns to the button.

## Large files
- Parsing runs on the main thread. From 200 ms the cursor is `wait`, `aria-busy` on the app root, canvas, File buttons, Undo and chords are ignored; no progress bar, no Escape. One frame is painted before the work starts.
- Files of 1 MiB or more show first, in the notice slot, "Importing poster.svg..." ("Opening poster.svg..." for Open…), `role="status"`, no time limit, replaced by the result or removed. Smaller files show nothing.
- The result appears in one frame: layer, selection and the "nothing" notice together.

## Keyboard and accessibility
- No new chord (R-SYS-009 reviews all later). Tab to the button, Enter or Space. Visible text name equals the accessible name; 2 px `--editor-accent` focus ring; hit target 28 px, 36 px coarse. Nothing animates, so `prefers-reduced-motion` needs no case. Notice text 12 px `--toolbar-icon` on `--toolbar-bg`, contrast as the slot.
- Large documents: selecting 10,000 imported objects must not freeze the Layers tab (`0050` criteria 48 to 52 cover it).

## Rows for `docs/design-system.md` (written by the PR)
Import SVG… in the File tab (order, tooltip), the busy texts and 1 MiB threshold, the one notice, the refusal titles ("Can't open file", "Can't import SVG").

## Import button in the tool rail (customer change 2026-10-27)
Supersedes "Where import lives" for Import SVG… ("I don't like the position under File. I want an Import button as a tool, and it can do SVG and raster graphics.").
- **One Import button, shared with `0058`** (`specs/0058-bitmap-import/specification.md`, "Import trigger"): own card in column A under Path tools, an action (opens the OS picker at once, no active-tool state, the tool does not change), Lucide `Import` 20 px, name "Import", tooltip "Import (Ctrl+I)". One picker: `.svg`, `.png`, `.jpg`/`.jpeg`, `.webp`; routing by content.
- **"Import SVG…" and "Import image…" leave the File tab** (the rows and the row order above fall away). **Open…** stays and still opens `.svg` as a project (Ctrl+O, unsaved-changes block first).
- **SVG placement unchanged:** one layer named after the file, on top and active, content centred on the view centre, nothing scaled, one step "Import SVG" (a batch with images: "Import"). **New: drop.** An `.svg` dropped on the canvas lands centred on the drop point; the "no drop" rule above is withdrawn for the canvas (the panel, rail and rulers still refuse).
- **Ctrl+I (Cmd+I)** triggers it outside text edit mode only; in text edit mode it stays italic.
- No hint label, no dialog of ours; cancelled picker returns focus to the rail button.

# SVG export: UX notes (export slice)

ux-engineer, 2026-11-01. Replaces "Export UX notes" in `specification.md` ("Later: export") where they differ, and applies the accepted `export-plan.md` (Frage 16 A: an inline row of switches under Export SVG…, no dialog, remembered per user). Customer rules as above: no hint labels, tooltip = the control's name only, no dialog of ours, permit over refuse. Strings marked **(Proposal)** are vetoable at the demo.

## Placement in the File tab
- **Rows, top to bottom:** project block, the 100 px button block (New/Open…, Save/Save as…, Close/Template; Import SVG… and Import image… are gone since the tool rail Import button), 12 px, **Export SVG…**, 8 px, **the options**, then the divider, Quit (desktop) and the version line as today.
- **Export SVG…** sits outside the 100 px block, so the unsaved-changes check never replaces it and nothing below moves. Same look as Open… (outline text button, 118 x 28 px, 36 px coarse), own row, left-aligned with the other buttons. Never dimmed, never hidden, no chord.
- **Tooltip:** "Export SVG", `side="left"`, 400 ms and on keyboard focus. The spec's sentence "Saves this drawing as an SVG file." is a hint label and is dropped.

## The options row
- **Control type:** four switches (`role="switch"`, `aria-checked`), in a group with `aria-label="Export options"` and no visible caption. Each is one 28 px row (36 px coarse): a 24 x 14 px track with a round thumb, then the 12 px label in `--toolbar-icon`; the whole label-plus-track is the hit target. On = track `--toolbar-icon-active-bg`, thumb right; off = track outline only, thumb left (state is never colour alone). 2 px `--editor-accent` focus ring around the row.
- **Layout:** two columns of 118 px with an 8 px gap, same grid as the buttons above (about 244 px, no scroll at 800 x 600):

  | Left column | Right column |
  |---|---|
  | Text as paths | Selection only |
  | Strokes as paths | Active page only |

  Labels do not wrap or truncate. If "Strokes as paths" does not fit 118 px at 12 px in a real build, fall back to one column of four 24 px rows (36 px coarse), same order; do not shorten the label. The implementer measures once.
- **Labels (exact):** "Text as paths", "Strokes as paths", "Selection only", "Active page only". Order is the tab order (row by row).
- **Tooltips:** none. The visible label already is the name; a tooltip would repeat it.
- **Defaults:** all four off. Switching changes nothing in the drawing: no history step, no dirty flag.
- **Memory:** per user, not per document: desktop in the app settings, browser in `localStorage` (wrapped in try/catch; unreadable means the defaults). Survives restart and Open/New.
- **Busy:** while an export runs (below) the switches are inert like the buttons.

### States
- **Selection only with nothing selected:** stays operable and keeps its stored value (permit over refuse; the selection changes all the time). Pressing Export SVG… with it on and nothing selected writes no file and opens no save dialog: notice "Nothing is selected." (below).
- **Active page only with one page:** stays operable; with one page it changes nothing. Not dimmed, not `aria-disabled`: a disabled look would need an explanation, and the value must survive for the next multi-page document.
- **Selection only and Active page only together:** the selected objects on the active page.
- **Text as paths and single-line fonts:** single-line (stick) text is always written as open paths whatever the switch says. The switch does not change for it.

## Flow
1. Press Export SVG… (Enter or Space on keyboard). Desktop: the OS save dialog opens (title "Export SVG", filter `.svg`), then the writer runs. Browser: no dialog; the writer runs and the browser downloads the file.
2. **Default file name:** the source's name if the project came from `poster.svg` ("poster.svg"); else the project name with the extension replaced ("poster.curvyo" gives "poster.svg"); else "Untitled.svg". The OS dialog confirms an overwrite. Browser: the browser may add " (1)"; our notice shows our name.
3. **Cancelled dialog:** nothing shown, nothing written, focus stays on Export SVG….
4. **Focus after done:** a keyboard press leaves focus on Export SVG…; a mouse press hands focus to the canvas, as every File tab button does (`onReturnFocus` when `event.detail > 0`). Toggling a switch by mouse also hands focus to the canvas; by keyboard it stays on the switch.

## Busy state
- From 5,000 objects (after the save dialog has returned, desktop): the notice slot shows "Exporting poster.svg..." (`role="status"`, no time limit, replaced by the result), cursor `wait`, `aria-busy` on the app root and canvas, canvas input, Undo, chords, File buttons and switches are ignored. No progress bar, no Escape. One frame is painted before the work starts. Smaller documents show nothing.
- File name cut at 24 characters, ellipsis in the middle, extension kept (as for the import).

## Notices
Canvas notice slot (File tab precedent), 12 px `--toolbar-icon` on `--toolbar-bg`, `role="status"`, no focus change, 3 s unless noted.
- **Success, desktop:** "Exported poster.svg."
- **Success, browser:** "Downloaded poster.svg."
- **Empty drawing** (nothing in scope, including an empty active page): the file is still written. "Exported Untitled.svg. The drawing is empty." (browser: "Downloaded Untitled.svg. The drawing is empty.").
- **Nothing selected** (Selection only on): "Nothing is selected." No file, no dialog.
- **Text kept as text (lead decision, Proposal; spec criteria 136 and 148):** when outlines were requested and a face is missing, those texts are written as live `<text>` instead, never dropped, and the success notice gets a second sentence, 6 s: "Exported poster.svg. 2 texts kept as text because their fonts are missing." One text: "1 text kept as text because its font is missing."
- Nothing else is reported (no list of what the switches did, no object counts).

## Error dialog
- The existing error dialog, no new component. Title "Can't export drawing"; body: the system's reason, then "Nothing was written." Example: "Permission denied (os error 13). Nothing was written." Browser has no write failure of this kind.
- Focus on OK, Escape closes, focus returns to Export SVG…. Partial files are removed by the writer path, so "Nothing was written." is true.

## Keyboard and accessibility
- No new chord (R-SYS-009 reviews all later). Tab order: Export SVG…, Text as paths, Strokes as paths, Selection only, Active page only, then Quit. Space toggles a switch (Enter too, it is a button).
- Visible text equals the accessible name for the button and every switch. Group name "Export options". Switch state is `aria-checked`, not announced elsewhere.
- Hit targets 28 px, 36 px coarse (`pointer-coarse:`). Label text 12 px against `--toolbar-bg` meets 4.5:1 in both themes; the off track outline and thumb meet 3:1. Nothing animates beyond the thumb move; with `prefers-reduced-motion` the thumb jumps with no transition.
- Notices are `role="status"` (polite), never take focus.
- Large documents: the busy state above covers 20,000 paths (criterion 71 budget 1.5 s); the UI stays responsive before and after.

## Rows for `docs/design-system.md` (written by the PR)
Export SVG… in the File tab (placement outside the check block, tooltip), the switch control and its two-column layout, the stored options, the busy text and 5,000-object threshold, the notices and the "Can't export drawing" title.

## UI review checklist
1. Export SVG… is its own row below the button block, 118 x 28 px (36 coarse), tooltip "Export SVG", never dimmed, unaffected by the unsaved-changes check.
2. Four switches with exactly the labels above, `role="switch"`, `aria-checked`, group "Export options", two columns, no wrap, no scroll at 800 x 600.
3. All off by default, remembered across restart, not stored in the document, no undo step, no dirty flag.
4. Selection only with nothing selected: operable; Export shows "Nothing is selected." and opens no dialog. Active page only with one page: operable, no visual disabling.
5. Default name: source name, else project stem + ".svg", else "Untitled.svg".
6. Notices match the strings, 3 s (6 s with left-out texts), `role="status"`; cancelled dialog shows nothing.
7. Busy state at 5,000 objects: text, wait cursor, input ignored, switches inert; none below it.
8. Error dialog: title, reason, "Nothing was written.", focus on OK, focus back on the button.
9. Focus: keyboard stays on the control; mouse hands focus to the canvas; visible 2 px focus ring on all five controls; Tab order as listed.
10. Light and dark theme, coarse pointer, `prefers-reduced-motion`.
