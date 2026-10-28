# 0087 Image clipboard, drop and import preview: UX notes

Author: ux-engineer. Date: 2026-10-28. Status: for Ready. Builds on `specification.md` and
`docs/design-system.md` (rows "Import button", "Action notice", "Refusal"). Own choices are marked
**(Proposal)**.

Customer rules applied: no dialogs, no permission prompts, no operating tips or hint labels (2026-10-16);
one rule for every way in (picker, drop, paste); direct manipulation; consistency replaces explanation.
There are no menus, buttons or panels in slices 1 and 2: the key chords and the canvas are the whole UI.

## 1. Where feedback appears

All notices use the **canvas notice slot** of the Import button (one notice, up to three lines,
`white-space: pre-line`, `pointer-events: none`, never takes focus), because no control caused them: the
key or the drop did. Looks are those of the Action notice row: success/status look (`--toolbar-bg` ground,
`role="status"`, 3 s; 5 s above 70 characters) and refusal look (`--popover` ground, 1 px `--field-invalid`
ring, alert glyph, `role="alert"`, 5 s like Import's refused-file alert). The live region exists from the
start; the announcement is the text change. A next key, press, selection change or tool change dismisses it.

## 2. Paste (Ctrl+V, Cmd+V)

| Situation | Feedback |
|---|---|
| Image pasted | No "pasted" notice: the image appears centred in the view, selected, Select tool, focus on the canvas, exactly as after Import. Only the `0058` notices that apply appear (fit, working limit, size). **(Proposal)** |
| Large clipboard (header 8 Mpx or more, or 5 MB or more) | Busy as Import: cursor `wait`, `aria-busy` on the root, one painted frame, status "Pasting image..." (the name "Pasted image" tells nothing, so no name). |
| No image on the clipboard | Status "Nothing to paste: the clipboard has no image." Document and history unchanged. |
| Invalid, damaged, too large | The refusal look with the `0058` strings; no history step. |
| Focus in a text field or the text editor | The element's own paste; no notice. |

- **Placement** (customer, Frage 67 B). Under the pointer with the `0058` clamp; pointer outside the canvas
  or never seen over it: view centre (spec criterion 2). No pointer-follow, no ghost, no drag-to-place.
  Two pastes with the pointer unmoved land on the same spot: the selection box and the new Layers row
  "Pasted image" are the feedback; no offset (YAGNI). **(Proposal)**
- **Undo.** One step "Paste image"; Ctrl+Z removes it and the selection returns to what it was.
- **Trace tool active.** The image is selected and the bar shows its preview at once; the tool does not
  change (criterion 9). No notice.
- **No hint.** Nothing in the UI says that Ctrl+V works; the shortcut table in `docs/design-system.md`
  carries it (criterion 18).

## 3. Copy (Ctrl+C, Cmd+C)

| Situation | Feedback |
|---|---|
| One image selected | Status "Image copied." (replaces "Copying..." in the same slot). |
| Takes over 400 ms | Status "Copying..." (criterion 14), cursor stays; the canvas keeps working. |
| Several images selected | "Copied 1 of 3 images." (topmost). One image among other objects: "Image copied." |
| No image in the selection | Nothing, no notice; the browser's copy in a panel field is not stopped. |
| Clipboard refused | Refusal look: "Could not copy the image: <short reason>." (at most 70 characters). |
| No `ClipboardItem` support | Refusal look: "Copying images needs a newer browser." |

The selection, tool and history do not change on copy. No undo exists for it, so no undo text.

## 4. Drop on the canvas (slice 2)

- **Hover.** The existing 2 px dashed `--accent` outline inside the viewport, no text, no animation, for
  every drag that carries files, a link or HTML with an image. A drag that carries only plain text gets
  drop effect `none` and no outline. The outline does not promise success: the page cannot tell a bare link
  from an image before the drop, and a refused link needs a drop to show its notice. **(Proposal)**
- **Elsewhere.** Panel, rail, rulers and bars: drop effect `none`, no outline, cursor "not allowed" as the
  browser draws it.
- **Drop.** Image centred at the drop point, clamped; selected; Select tool (Trace tool unchanged);
  focus to the canvas. Several images: the placement rule of `0058` criterion 4; one step "Import images".
  Busy text as Import: "Importing 3 of 12 images...".
- **Image data without a name:** object name "Dropped image". No notice on success.
- **Link only** (customer, Frage 68 A): refusal look, text "That drop is a link, not an image. Nothing was
  downloaded." No tip sentence (spec criterion 21 now has this string).
- **Mixed drop:** images import, one merged refusal for the rest, as `0058` criterion 5.
- Dropped `.curvyo`: nothing opens; the refusal notice of `0058`.

## 5. Import preview (slice 3, Frage 69)

**Superseded by the customer's answer (Frage 69 B, 2026-10-28; spec criteria 26 to 28):** the import dialog
shows the selected image with its pixel size, as the GTK file chooser's preview pane in GIMP and Inkscape
does, on all three desktop OSes, without a setting to switch on; in the browser if possible, else a
recorded deviation. The mechanism is in the architect's `adrs.md`; the option table below is the earlier
analysis and is kept for the layout notes of an own dialog.

The customer's complaint is that the maker picks files blind. UX view of the options:

| Option | UX effect | Verdict |
|---|---|---|
| A. Native dialog | No new UI; consistent with the OS; preview only where the OS offers it (Windows, macOS with the preview pane on; GTK via the plugin: no) | Take it where the spike shows it works |
| B. Own file browser (desktop) | Fixes the complaint everywhere; an in-app overlay replaces the OS dialog on the desktop | Only if A fails on Linux |
| C. Thumbnail strip after the pick | Adds a confirm step to every import, which the maker does 20 times a day, and does not help to find the file; the image on the canvas plus Ctrl+Z is already the preview | Not recommended |

**Recommendation:** run the spike; A where the native dialog previews; if the Linux dialog cannot, B (not C).
Default: A, as the spec says, so slices 1 and 2 are not blocked.

If B is built **(Proposal)**: an overlay sheet centred over the canvas, 640 x 440 px (it shrinks to the
viewport), a path bar with a folder up button, a grid of 96 px thumbnails with the file name under each
(cut at 24 characters), the last folder remembered, type to jump to a name, multi-select with Ctrl and
Shift (at most 20, as the picker), a text button "Import" (accent fill) and "Cancel". Enter imports, Escape
cancels, focus on the file grid at open and back on the Import button after (criterion 29). Files over the
ceilings show their name and "Too large" and are never decoded; an unreadable file shows its name only;
thumbnails fill in lazily and never block typing or scrolling (the 50 ms rule). No text for the sort or the
view; no modes. Light and dark theme from the tokens; a focus ring on the selected thumbnail, selection also
shown by the accent outline (not colour alone).

The browser build keeps the browser's picker; nothing is drawn, nothing is said in the app (D5).

## 6. Keyboard and accessibility

- Ctrl+V, Ctrl+C work with focus on the canvas or a non-editable bar or rail button; never in a text field,
  the text editor or a panel field (their own copy and paste).
- Every outcome that the canvas does not show is a live-region text (sections 2 to 4). Nothing relies on
  colour alone; the refusal look has a glyph and words.
- Contrast of the outline and notices as in the design system, both themes. Nothing animates.
- Large documents: paste and copy never block input for more than 50 ms on the main thread (criterion 14).

## 7. Mixed-in questions for the lead

Both earlier questions are answered: the link-drop string is the customer's (Frage 68 A); the preview is
Frage 69 B (section 5).
