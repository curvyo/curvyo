# 0136 Autosave and restore: UX notes

ux-engineer, 2026-11-23. Written from `specification.md` (criteria 1 to 13; Fragen 265 B, 266 A+B,
267 A) and `docs/design-system.md`: "Properties panel: File tab", "Properties panel: tabs", "No
notice after a success", "No operating tips by default", "Hidden, not disabled", "Inline confirm"
(three-button variant), "Inline delete confirm", "List row", "Canvas notice slot". No new token, no
new component: the block is built from the File tab's project block, the workspace document row
(`0095`) and the armed trash of the formats list. **(Proposal)** items are vetoable at the demo;
section 11 lists them as P1 to P12, section 12 the spec deltas for the product owner, section 13
the one customer question. "Muted" is `--panel-muted-fg`; "column" is the File tab's 244 px
content width.

## 1. Principle

The app writes quietly and says nothing while it works. After a crash the work is simply there,
in the File tab, as rows that look like the files they are: a name, when it was last touched, how
big it is. One press brings it back as an unsaved project; one armed trash throws it away. No
dialog, no popup, no toast, no explanation.

## 2. When the block exists

- **Shown** only when the start found one or more recovery copies whose project is not open in
  another window or tab (criteria 4, 11). It is the first thing in the File tab body, above the
  project block (P1), 12 px above the project block's name. The Removed device alert of `0095`
  sits at the top for the same reason: what needs a decision comes first.
- **Read once, at start** (P2). The list is not polled: a copy that becomes offerable while the app
  runs (another window crashes) is offered at the next start.
- **Empty state: nothing.** No copies, no block, no heading, no "No unsaved work" line, no help
  line (criterion 9). When the last row goes (Restore or Discard), the block leaves in the same
  frame and the project block moves up by its height; nothing else moves.
- **Start view** (Frage 317, answered A): with the block present at start, the panel body is shown
  and the File tab is active, once, the way the unsaved-changes check already raises them. Focus
  stays on the canvas; afterwards the tab rules of "Properties panel: tabs" apply unchanged (the
  File tab never switches by itself again in that session).

## 3. Layout (column 244 px)

| # | Part | Size | Notes |
|---|---|---|---|
| 1 | Heading "Unsaved work from a crash", 14 px semibold `--toolbar-icon` | 20 | `h3`, names the section |
| 2 | Help line, 12 px muted, one line, ellipsis | 16 | desktop "Kept in ~/.local/share/curvyo/recovery" (the real app data path of the OS, cut at the start with a leading "...", full path in the tooltip); browser "Kept in this browser" (P3) |
| 3 | Row list, 4 px above | 44 per row, 60 with a third line | newest last step first; the newest copy plus up to 20 older ones, so at most 21 rows (criterion 12), no inner scroll, the body scrolls |
| 4 | 16 px, a 1 px divider (`--toolbar-icon` at 25 %), 16 px | 33 | separates the offer from the project block |

The help line is a state label (where the copies are), not a tip (criterion 9); it goes with the
block.

## 4. Row

Anatomy (the workspace document row of `0095` without thumbnail, P4): 8 px left padding, text block
(fills), then a 64 px action slot: **Restore** and **Discard**, 28 x 28 icon buttons (36 coarse),
Lucide 16 px at 1.5 px absolute stroke, `--toolbar-icon`, 4 px apart, 4 px right padding.

| Line | Text | Style |
|---|---|---|
| 1 | Name: the saved file's name ("Poster.curvyo") or "Untitled" | 14 px `--toolbar-icon`, ellipsis; desktop tooltip the file's full path (none for Untitled) |
| 2 | Time of the last step and page count: "12:03 · 1 page", "2 Nov 12:03 · 3 pages", "29 Sep 2025 12:03 · 3 pages" | 12 px muted, `tabular-nums`; the time format of the workspace document row |
| 3 | Only when criterion 6 applies: "File changed since" | 12 px muted; the row grows to 60 px |

- **Glyphs** (P5): Restore `ArchiveRestore` (not `RotateCcw`: that is "Restore original", a revert
  in place; this is an open), Discard `Trash2`.
- **Several "Untitled" rows** are told apart by their time and page count; no numbering (P6).
- **Damaged copy** (criterion 7): `TriangleAlert` 16 px muted in front of the name, line 2 reads
  "Cannot be restored: damaged." or "Cannot be restored: made by a newer Curvyo." (the open
  errors of `0001` criterion 7, same words), wrapping to a second line when needed (row 60 px). The
  Restore button is absent (Hidden, not disabled); Discard stays in its slot, so the trash lines
  up with the rows above.
- **States**: hover `--editor-accent-hover` on the row; focus 2 px `--editor-accent` ring inside
  the row; armed: the whole row `--destructive-soft` and the trash a filled `--destructive` square
  with a white glyph (Inline delete confirm, no size change). No selected state: nothing in the
  block is ever "current".
- **Pressing the row body does nothing** (P7). Restore replaces the open project, so it needs an
  aimed press on its button; this departs from the List row rule "a row is a button" for that
  reason.

## 5. Restore (criteria 5, 6)

- A press on Restore (or Enter on the row) opens the copy as the open project, exactly like Open:
  - **Current project clean** (also the blank project of a fresh start): replaced at once, no
    check.
  - **Current project dirty**: the project block's five buttons turn into the three-button Inline
    confirm of `0051` in place, sentence "Untitled has unsaved changes. Save them before you
    restore Poster.curvyo? Discarded changes cannot be brought back." (the restored row's name;
    "... before you restore Untitled from 12:03?" when the copy is Untitled, P8). Focus to Cancel;
    Cancel and Escape change nothing and return focus to the Restore button; Save, then Restore,
    as for Open.
- **Result**: the project opens dirty with no file path, named "Untitled" or "Poster (recovered)"
  (criterion 5); the project block's status line reads "Not saved yet"; the window title
  "* Poster (recovered) — Curvyo"; the File tab dot shows. Save goes straight to Save as with the
  default name "Poster (recovered).curvyo" (browser: the download). The row goes in the same
  frame. The File tab stays shown.
- **File changed since**: Restore behaves the same; the saved file is never touched. Nothing more
  is said after the restore (the name with "(recovered)" and "Not saved yet" are the feedback).
- **Busy**: from 1 MiB, "Restoring Poster.curvyo..." in the canvas notice slot until the project
  is shown, cursor `wait`, `aria-busy` on the root (the Open treatment of `0024`). A copy that
  fails while restoring keeps its row, which turns into the damaged look of section 4; the current
  project is left untouched (criterion 7).
- **No notice after success** (criterion 13): no "Restored ..." anywhere.
- **Focus after Restore**: the canvas, mouse or keyboard (the project changed, as after Open; P9).

## 6. Discard (criterion 4: inline second press)

The **Inline delete confirm** unchanged (`useDeleteArm`): first press arms, a press within 400 ms
is ignored, the second press deletes the copy for good; disarms after 4 s, on Escape, on any other
press, when focus leaves the row, when the list changes. One trash armed in the app at a time. No
sentence, no Cancel: the row itself is the item.

| | Unarmed | Armed |
|---|---|---|
| Accessible name and tooltip | "Discard Poster.curvyo" ("Discard Untitled from 12:03") | "Press again to discard Poster.curvyo for good." |
| Polite status region | | repeats the armed name once |

After the delete the row goes in the same frame; focus moves to the next row, else the previous
row, else the File tab button in the strip (the block is gone) (P10). No notice.

## 7. Keyboard

- The row list is **one Tab stop** with roving focus on the rows (List row): Up, Down, Home, End
  between rows, no wrap. Tab order: the row list, then the project block's buttons.
- On a row: **Enter** restores; **Right** moves to the Restore button, **Left** back to the row;
  **Delete** arms the trash, **Delete** or **Enter** on the armed row discards (the Inline delete
  confirm keys: once armed, Enter means Discard, and the armed name says so). The trash is not a
  roving stop (as in the formats list). **Escape** disarms; on an unarmed row it returns focus to
  the canvas (panel rule).
- On a damaged row: Enter does nothing (no Restore exists); Delete arms as above.
- No new chord; the File tab keeps having no key.

## 8. Accessibility

- `section` `aria-labelledby` the heading; the rows are a `ul` named "Unsaved work from a crash".
- Row: `li`, `tabindex` per roving focus, accessible name "Poster.curvyo, last change 12:03, 3
  pages", plus ", file changed since" or ", cannot be restored: damaged". The buttons inside are
  named "Restore Poster.curvyo" and as in section 6.
- While the block exists, the File tab button's accessible name gains ", unsaved work from a crash"
  (beside ", unsaved changes" of the dirty dot); no visual mark on the button (Frage 317 A).
- Appearing at start is not announced (no live region for static start content).
- Contrast: existing tokens only (`--toolbar-icon` 8.3:1, muted 5.0:1 on the panel; on
  `--destructive-soft` 8.2:1 and 4.95:1). Targets 28 px with 4 px gaps (WCAG 2.2 SC 2.5.8), 36
  coarse. No animation; the arming transition is at most 120 ms and none under
  `prefers-reduced-motion`.

## 9. Writing and its failure (criteria 1, 2, 8, 13)

- **A successful write shows nothing**: no notice, no spinner, no status line, no title change. The
  dirty marker keeps meaning "differs from the saved file", not "not yet in a recovery copy".
- **A failed write** (P11, reading of "the title area", delta E1):
  - **File tab**: a third line in the project block under the status line, 12 px `--field-invalid`,
    `role="alert"`, until the next successful write: "Recovery copy not written: the disk is
    full." / "Recovery copy not written: no permission for the app data folder." / browser
    "Recovery copy not written: the browser's storage is full." Other causes: "Recovery copy not
    written: <the system's reason>." Reserved only while present (the buttons below move by 16 px).
  - **Canvas notice slot**: the same sentence once as an alert (8 s) when the failure first
    happens. Not repeated on each retry; a new failure after a success shows it again.
- **Pruning** at start (criterion 12) is silent: no notice, no line.

## 10. Large documents and timing

The write runs off the main thread and has no visible state, so a drag on 10,000 objects shows
nothing of it (criterion 2). The block holds at most 21 rows (the newest copy plus 20 older, criterion 12); it is not virtualised. The page count
and time come from the copy's header, never by loading the project.

## 11. Proposals

| # | Proposal |
|---|---|
| P1 | The block sits at the top of the File tab, above the project block |
| P2 | The copy list is read once at start |
| P3 | Help line "Kept in <path>" / "Kept in this browser" |
| P4 | Row = the workspace document row without thumbnail, 44 / 60 px |
| P5 | Restore `ArchiveRestore`, Discard `Trash2` |
| P6 | Several Untitled rows are told apart by time and pages, not by numbers |
| P7 | Pressing the row body does nothing; Enter on the focused row restores |
| P8 | The dirty check's sentence names the copy being restored |
| P9 | Focus goes to the canvas after Restore |
| P10 | Focus after Discard: next row, previous row, File tab button |
| P11 | Failed write: persistent File tab line plus one canvas alert |
| P12 | The File tab button's accessible name mentions the waiting work |

## 12. Spec deltas for the product owner

Applied to the spec on 2026-11-23: E1 (criterion 13), E2 (criterion 4), E3 (criterion 9).

- **E1** (criterion 13): "the File tab and the title area" reads ambiguously. Proposed wording: "a
  line in the File tab's project block until the next successful write, and one canvas notice when
  the failure first happens" (section 9).
- **E2** (criterion 4): the page count is shown as "3 pages" on line 2, the time in the workspace
  row format; "File changed since" (criterion 6) is a third line.
- **E3** (criterion 9): the help line text is "Kept in <path>" / "Kept in this browser".

## 13. Customer answer

**Frage 317 (RESTORE-SHOW): How does the maker see the offer at start?** A new session opens on
the Document tab, the File tab never switches by itself, and the panel body may be hidden, so the
block alone would stay unseen.

- **A**: when copies are waiting at start, the panel body is shown and the File tab is active, once
  (like the unsaved-changes check does today); focus stays on the canvas; then the normal rules.
- **B**: no switch; a new mark on the File tab button (the dirty dot is taken, so a second cue).
- **C**: nothing; the block waits in the File tab until the maker looks.

Answered **A** by the customer on 2026-11-23 (the recommendation and default; the work is in front of the maker without a popup, and nothing
new has to be learnt).
