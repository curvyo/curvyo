# 0081 Inline delete confirm: UX notes

ux-engineer, 2026-10-23. Checked against `docs/design-system.md` (rows "Inline confirm", "List row", "Favourite star", "Action notice", the `0075` Fonts tab rows "Family row", "Tag strip", "Remove row", the colour tokens `--destructive`, `--field-invalid`, `--panel-bg`, `--panel-muted-fg`, `--editor-accent`), the `0075` UX notes (§3, A5, A7, A8, A11) and on `main` `FontTreeRows.tsx`, `FontTagStrip.tsx`, `FontRemoveRow.tsx`, `FormatList.tsx` (`InlineConfirm`), `index.css`.

The pattern adds **one token, one button per Curvyo family row, no popup, no hint label, no countdown**. The armed state is carried by the filled trash (shape) and the red ground (colour); the accessible name says what the next press does.

## 1. Tokens

| Token | Light | Use | Contrast (light, measured 2026-10-23) |
|---|---|---|---|
| `--destructive-soft` (new) | `#F7D3CF` | Ground of an armed family row or tag cell. Used by nothing else | `--toolbar-icon` on it 8.2:1, `--panel-muted-fg` 4.95:1, `--destructive` 4.7:1, `--editor-accent` (focus ring, current mark) 3.3:1. To `--panel-bg` 1.01:1 in luminance: the ground is the colour cue only, the filled trash is the boundary that reaches 3:1 |
| `--destructive` (existing, `#B3261E`) | unchanged | Fill of the armed trash | White glyph on it 6.5:1; it against `--destructive-soft` 4.7:1 |

The name pairs with the existing `--destructive` (spec example; not `--danger-*`, which would be a second name for the same meaning). Dark theme: the app is light only today (the `.dark` block in `index.css` is the unused shadcn default). Names are ready; proposed dark values, to be measured when a dark panel exists: `--destructive-soft` `#45231F`, `--destructive` `#F2685F` with a `#1E1E22` glyph (5.5:1).

## 2. Family row (Part A)

- **Place:** name line, between the current mark and the star: chevron 24, 4, name (flex), current mark, **trash 24 x 28**, 4, star 24 x 28, face count, 8. The star and the count keep their place in every block, so the star column lines up across blocks. Rows of other blocks reserve **no** slot (their name gets the 28 px); the trash is not in the tree there (row "Hidden, not disabled").
- **Idle:** `Trash2` 16 px, stroke 1.5, `--panel-muted-fg` (5.0:1 on `--panel-bg`), the same look as the outline star; hover `--editor-accent-hover` ground on the button, as the star. Always visible. Tooltip side left "Remove from Curvyo fonts"; accessible name "Remove Lobster from Curvyo fonts". `tabIndex={-1}`, `onMouseDown` prevents default (focus stays), `onClick` stops propagation (no pick, no expand).
- **Armed:** the whole row element (name line plus every tag line, wrapped lines included, inside the row's 5 px radius) takes `--destructive-soft`; hover ground is suppressed while armed. The trash shows a 22 x 22 square, radius 5 px, filled `--destructive`, the same `Trash2` 16 px in white. Text, chips, star and count keep their colours (all pass, §1). The focus ring stays as today on top of the ground.
- **No other change:** the row height, the name width and every position stay the same between idle and armed. The Remove row of `0075` and `FontRemoveRow.tsx` go.

## 3. Tag cell (Part B)

- Today's 20 x 20 `CELL_ICON` button with `Trash2` 12 px stays, beside the pencil; accessible name "Delete tag Wedding", tooltip "Delete" (as today).
- **Armed:** the cell (the `CELL` element, full 28 px height) takes `--destructive-soft`; the trash becomes a 20 x 20 square, radius 5 px, filled `--destructive`, white glyph. Cell label and pencil `--toolbar-icon` (8.2:1).
- **Armed while chosen:** the armed ground replaces the chosen `--toolbar-icon-active-bg` and the label turns `--toolbar-icon`; `aria-checked` stays true; after disarming the chosen look returns. (Two strong grounds in one cell would make the filled trash unreadable.)
- The Inline confirm block across the strip goes. The strip never reflows: no cell changes width.

## 4. Timing and motion

- Guard 400 ms, timeout 4 s from the arming press, one implementation for both uses.
- **No timeout feedback**: no progress bar, no fade-out countdown, no text. The timeout is a safety net in the safe direction (it disarms), not a deadline the maker has to watch. A second press after the timeout simply arms again.
- Arm and disarm: `background-color` and the trash fill change over 120 ms ease-out; under `prefers-reduced-motion` instantly. A press inside the 400 ms guard gives no feedback.
- The timeout keeps running while the row is scrolled out of the virtualised list; the armed state is held by family key (or tag id), not by the mounted row. Wheel scrolling does not disarm.

## 5. Keyboard and focus

| Where | Key | Result |
|---|---|---|
| Curvyo family row, or a face row of it | `Delete` | Arms the family row; focus stays on the focused row |
| Armed row (focus on it or its face row) | `Delete` or `Enter` | Removes (after the guard); focus to the next family row, else the previous one (`0075`) |
| Armed row | `Escape` | Disarms, consumed (does not also return focus to the canvas); a second Escape does what it does today |
| Armed row | Up, Down, Home, End, Page keys, Tab, F2 | Focus leaves the row, which disarms; the key then acts as today |
| Armed row | `Space`, `Ctrl+D` | Act as today (expand, star); the row stays armed unless the list changes (expand/collapse does disarm, criterion 5) |
| Row of another block | `Delete` | Nothing, no feedback |
| Own tag cell | `Delete` | Arms the cell; focus stays on the cell |
| Armed tag cell | `Delete` or `Enter` | Deletes (after the guard); focus to All when done by keyboard |
| Armed tag cell | arrows, Home, End | Move and choose as today; disarms |
| Built-in cell, All, Favourites | `Delete` | Nothing |

By mouse the focus never moves (press on either trash, first or second press). After a mouse removal the focus stays where it was; if that was inside the removed row, it goes to the next family row as for the keyboard (never to `body`).

## 6. Screen readers

- One `sr-only` `role="status"` `aria-live="polite"` region in the Fonts tab, shared by both uses; it is in the tree before the first arming (a region inserted with its text is not read).
- On arm it receives the armed accessible name: "Press again to remove Lobster from Curvyo fonts. 2 texts here use it." / "Press again to delete tag Wedding. Its 12 fonts stay in Curvyo."
- On removal: "Lobster removed from Curvyo fonts." / "Tag Wedding deleted." (sr-only; the visible notices stay those of criterion 8 and 15, so a plain removal shows nothing on screen).
- On disarm the region is cleared, nothing is announced.
- The armed trash's accessible name and tooltip switch to the armed text (criteria 3 and 11); the family row's own `aria-label` stays the family name. A face row with focus during keyboard arming is announced through the status region only.
- 4 s is short for a screen-reader user but fails safe (WCAG 2.2.1 concerns loss of data or of a session; here the timeout only cancels); re-arming is one key.

## 7. Notices

The TTC notice "Lobster is in a file with other fonts and was kept." is the existing Action notice (DOM, no controls, never takes focus, same dismissal as today), anchored under the family row's name line: top edge 4 px below the row's bottom, right edge level with the trash's right edge, clamped to the panel. The row stays (the file was kept) and returns to idle in the same frame.

## 8. Design-system text (applied in the build, criterion 17)

New bullet under "Interaction conventions", after "Inline confirm":

> - **Inline delete confirm** (2026-10-23, `0081`): how removing a listed item asks without a popup and without text. A `Trash2` icon button in the item's own row, always visible, not a Tab stop; the first press **arms** it: the row (or strip cell) takes `--destructive-soft` and the trash turns into a filled `--destructive` square with a white glyph, no width or height changes. The second press on the same trash removes; a press within 400 ms of arming is ignored. It disarms, without notice, after 4 s, on Escape, on any other pointer press, when the focus leaves the row and when the list changes (search, strip cell, expand or collapse, tab, panel collapsed). One trash is armed in the app at a time; arming another disarms the first. Keys: `Delete` on the focused row arms, `Delete` or `Enter` on the armed row removes; the focus stays put. The armed trash's name and tooltip say what the next press does, a polite status region repeats it. No countdown, no transition over 120 ms, none under `prefers-reduced-motion`. Used by: Remove from Curvyo fonts (family row), Delete tag (tag strip). Not for reversible actions (they never ask) and not where the consequence needs a sentence (that is the Inline confirm).

In the "Inline confirm" bullet nothing names the two uses today; "Used by" stays "Wipe history (document and object), Delete format and Delete group".

Colour token table, after `--destructive`:

> | `--destructive-soft` | `#F7D3CF` | `0081` (2026-10-23): ground of an armed Inline delete confirm row or strip cell. `--toolbar-icon` 8.2:1, `--panel-muted-fg` 4.95:1, `--destructive` 4.7:1, `--editor-accent` 3.3:1 on it; 1.01:1 to `--panel-bg` on purpose, the filled trash carries the state. A dark theme swaps the value |

Fonts tab table: row "Remove row" is replaced by

> | Remove trash (`0081`) | Curvyo-block family rows only, on the name line before the star: 24 x 28 button, `Trash2` 16px `--panel-muted-fg`; Inline delete confirm; Delete on the family row (or one of its face rows) arms it; other blocks reserve no slot |

and the "Tag strip" row's "Delete replaces the cell by the Inline confirm across the strip; after it the focus is on All" reads "Delete is an Inline delete confirm on the cell; after a keyboard delete the focus is on All".

Keyboard table:

> | Remove a Curvyo font | `Delete` on its family row, then `Delete` or `Enter` | `0081`; Curvyo fonts block only; not undoable (a file, not the document) |
> | Delete an own tag | `Delete` on its cell in the tag strip, then `Delete` or `Enter` | `0081`; built-in tags, All and Favourites ignore it |

and the row "Rename, delete an own tag" becomes "Rename an own tag | `F2` on its cell in the tag strip | `0075`; or its pencil".

## 9. Review checklist

Both uses share one state and one look; star column aligned across blocks; no layout shift on arm; contrast values of §1 on the rendered pixels; double-click never removes; Escape consumed once; status region pre-mounted; mouse presses never move focus; no Inline confirm left in the Fonts tab; the other Inline confirm uses unchanged.
