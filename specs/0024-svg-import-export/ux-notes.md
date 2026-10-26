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
