# UX notes: Font info and fonts from zip archives (0083)

ux-engineer, 2026-10-26. Frage 41 is taken at its default A. Info is a third mode of the `0077` pane, not a new surface. No dialog (pattern of `0081`), no hint label (`docs/design-system.md`, "Interaction conventions"), no tooltip needed for any new control that has a visible name. Pane grounds, split, full view, divider, Close and Escape: `0077` and `0082` unchanged. Measures are at the 339px split width (282px minimum) and in full view.

## 1. Mode switch

- Three cells in the `0082` strip: **Fonts**, **Glyphs**, **Info**, about 143px, same look, `role="radiogroup"` "Preview mode", one Tab stop on the pressed cell, Left/Right choose without wrap, Home/End. The pane reopens in the last mode.
- Info has no sample field and no Size slider (they do nothing here). Row 1 at under 560px: switch 143, flex gap, Full view, Close; the subject line takes row 2 ("Great Vibes Regular · Included", 12px muted, ellipsis). At 560px and wider: one row, switch, subject line (flex), Full view, Close. Tab order: divider, switch, Full view, Close, then the content (Show all buttons, Documents rows, shown-text region).
- No Info button in the Text bar (0084 row 2 is full of groups; Info is reached from the switch only). Proposal.

## 2. Info layout

One scroll area (the body of the pane, thin scrollbar, `overscroll-behavior: contain`), 16px side padding, nothing sticky.

- **Header block**: the family name 28px and the style 16px muted below it, both drawn in the subject face (it is the preview), one line each, cut off at the edge with an ellipsis, no wrapping. A single-line (SVG) face draws as its engine outlines like its Fonts tile. Face not loaded yet: the name in the UI font, never in a wrong face (rule of `0077` criterion 17). Under it 16px gap, no rule.
- **Fact list**: two columns, a `dl`. Label column 96px (14px `--panel-muted-fg`, right edge ragged, top-aligned), 12px gap, value column flexible (14px `--panel-fg`, `user-select: text`, wraps by word, long tokens such as URLs break anywhere). 8px between rows, no zebra, no borders. At the 282px minimum the value column is 158px; below 560px nothing else changes.
- **Order and labels** as criterion 2: Full name, Format, File, Version, Designer, Vendor, Copyright, Trademark, Licence, Embedding, Glyphs, Characters, Documents. Family and Style are the header, not rows. Group breaks (16px instead of 8px): after Full name, after File, after Trademark, after Licence, after Characters. Rows the font does not state read "Not stated" in `--panel-muted-fg`.
- **File** value: line 1 "Lobster-Regular.ttf · 412 KB", line 2 the block ("Curvyo fonts"), line 3 the folder path (desktop; 12px muted, selectable, wraps anywhere). A font in a font folder shows its folder in that path; no extra "Folder" row.
- **Licence**: included fonts: the licence name in words, then its shipped text under Show all. Other fonts: the name ID 13 description, then the URL as plain selectable text (never a link, R-SYS-007).
- **Embedding**: the words only, as the spec lists them ("Installable", "Restricted: may not be embedded", ...). Same ground as the other values: no warning colour, no icon, no badge (it never blocks anything).
- **Show all** (criterion 3): a text button under the 3 clamped lines, 14px `--editor-accent`, 28px hit height, `aria-expanded`, becomes **Show less** when open. Clamp by lines (`-webkit-line-clamp: 3`), the button only if the text really is cut. Expanded state resets with the subject face.
- Full view (wider than 560px): same two columns, the list capped at 640px wide and left aligned under the header, the header may use the full width. No third column.
- Subject face changes (Text edit, selection): the content swaps in the same frame, no fade, scroll back to top.

## 3. Documents

- The Documents row sits at the end of the fact list as a label with the value column holding a stacked list of rows, A to Z (criterion 4). The row exists only for a face in a font folder; otherwise it is absent, not "Not stated".
- Each row: a 28px high text button, full value-column width: 16px Lucide icon (`FileText` for text documents, `FileType` for PDF; the icon says which behaviour follows), file name 14px with ellipsis in the middle-preserving extension (name cut, `.txt` kept), size 12px muted right-aligned ("12 KB"). Hover `--editor-accent-hover` ground, focus ring as list rows.
- **Text document**: press toggles it open **under the whole list**, in place: a block with 1px `--toolbar-icon` at 25% border, 8px padding, `--panel-bg` slightly inset, monospace 12px, `white-space: pre-wrap`, line breaks kept, plain text only. It grows to at most 320px and then scrolls inside (the one inner scroll area of this view; in full view up to 60% of the body). The open row is `aria-expanded` with a pressed look; a second press or a press on another document swaps/closes; the shown document is not kept across faces. The 1 MiB cut reads "Shown: the first 1 MiB." as the last line of that block, 12px muted.
- **PDF**: press hands the file to the system viewer; the row shows no pressed state. A second icon (`ExternalLink`) at the row end tells it leaves the app. If the viewer cannot start: notice "Lobster-License.pdf could not be opened." (`0075` slot).
- Unreadable document (file gone): the row stays, size reads "missing", press gives the notice "OFL.txt could not be read."

## 4. Add font… and drop with .zip

- The Add font button, its file filter and the drop outline are the `0075` ones (2px dashed `--accent` inset, button pressed look, no text). The filter adds `.zip`; no new button, no "Add archive".
- Nothing is asked: no dialog for which fonts of the archive to take, no progress dialog. Archives over a few hundred ms show the button's busy look of `0075` and the pane stays usable.
- **Report**: one notice in the `0075` slot (polite status, same timeout, 12px/14px as built), at most 70 characters per part, archive name first, counts only when above 0, no emoji, no exclamation:
  - "Lobster.zip: 2 fonts added, 3 documents kept."
  - "Lobster.zip: 2 fonts were already there."
  - "Lobster.zip holds no font. Nothing was added."
  - "Pack.zip could not be read. Nothing was added." (not a ZIP, damaged, empty)
  - "Pack.zip is over the size limit. Nothing was added."
  - Mixed: "5 fonts added. 4 documents kept." then a second part "1 archive could not be read." if the budget allows; otherwise the first refusal is named and the rest counted ("Pack.zip and 2 more could not be read."). Unsafe entries: "2 files were skipped as unsafe." Singular forms ("1 font", "1 document") in every part.
  - Several archives are summed; only when exactly one archive was dropped alone is its name used.
- Reasons are limited to the three above (could not be read, over the size limit, holds no font). Encrypted archives and nested archives read as "holds no font" or "could not be read"; no extra reason texts (YAGNI).
- New families appear and scroll into view as `0075` criterion 22 says; the Info pane, if open, is not switched.

## 5. The folder in the Fonts tab

The user does not see the folder. Families, rows, tags, star, search and the strip are exactly as before: no folder row, no tree, no folder badge, no group header. The block is still "Curvyo fonts". The folder surfaces in one place only: the path in Info's File row, and in the file manager. A hand-made folder is listed the same way (one level deep). Documents are not listed in the Fonts tab at all.

## 6. Remove from Curvyo fonts (inline confirm of `0081`)

- Nothing new: the trash in the family row, row turns soft red, press again after 400 ms. The text of `0075`/`0083` criterion 14 ("Remove Lobster from Curvyo fonts? Its 3 documents go too.") is superseded by the `0081` form, because the confirm sentence is no longer shown: the armed accessible name, tooltip and polite status read "Press again to remove Lobster from Curvyo fonts. Its 3 documents go too." and, when texts use it, " 2 texts here use it." after it. The documents sentence appears only when this removal takes the font folder's last font and that folder holds documents; counted as "1 document" / "3 documents".
- Disarm, keyboard (Delete arms, Delete/Enter confirms), focus and notices as `0081`. No history step, Ctrl+Z does not restore (unchanged).
- If the Info mode is showing the removed face: the pane shows the missing-face state of criterion 5.

## 7. Empty and error states

All in the `0077` empty look: 14px `--panel-muted-fg`, centred, 24px from the top of the body, one line.
- Info with no subject face at all (no text, nothing selected, no next-text face): not possible, the next-text face always exists; if its font is gone it is the missing state.
- Missing face: header block shows the family name in the UI font, one line under it "Lobster is missing.", no rows.
- Face that cannot be read: "Lobster Regular could not be read." under the header, no rows.
- Face without documents: no Documents row. Document list read but empty after a removal: row absent.
- Document text empty: the open block reads "This file is empty." (muted).

## 8. 800 x 600 fit

Split 339px, panel open, canvas overlay row 324px: header block 76px, 13 rows, about 2 to 4 lines each when long values clamp at 3, about 560px of fact list. The Info body scrolls as one area; header block and fact list never overflow sideways, the long names and URLs break. Row 1 of the header at 282px minimum: switch 143, Full view 28, Close 28, 8px gaps: fits with 66px spare. The Documents text block capped at 320px keeps the Close button and the switch in view (the header is not part of the scroll). No bar of the Text or Select bar gains a control (bar overflow rule of `0084` not touched).

## 9. Keyboard

- The pane's keys of `0077`/`0082` apply: Escape closes the pane or a shown text first (one step: a shown document closes before the pane), Tab order as section 1.
- Facts are text, not tab stops; the scroll area is focusable (`tabindex="0"`, `role="region"` "Font info") so Page Up/Down, Home/End and arrows scroll it. Show all and Documents rows are buttons (Tab, Enter, Space). Selecting and copying: Ctrl+C on a text selection, as everywhere.
- Rows of Documents: Up/Down move between rows once focus is in the list, Enter/Space opens. No new global shortcut; Info is reached by the switch (and Left/Right on the strip). Add font… keeps its key.

## 10. Design-system rows to add (when built)

"Font preview (`0077`)": rows "Info mode" (sections 2, 7), "Document rows" (3), amend "Mode switch" (143px) and "Notices" (zip reports). "Fonts tab": amend "Add font" (the filter has `.zip`) and "Remove" (documents sentence in the armed name).
