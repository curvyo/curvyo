# Document fonts: every used font travels inside the file

Status: Ready
Priority: Must
Origin: Customer, 2026-11-09: "All fonts that are used are always built into the `.curvyo` file, exactly like raster images. In the Fonts panel a new group 'Document Fonts' appears at the top, very similar to 'Curvyo fonts', but when a font is not installed, the star and the + (tags) are greyed out and instead of the trash can the install icon (the one in the top row) is shown. Document fonts never have a trash can; when the font is installed there is simply no 'Add font' symbol." Wording of texts is a Proposal until the demo.

## User value

As a maker who opens a file on another device or in a shared workspace I want the text to look as its author drew it without installing anything first, so that no "missing font" appears and a cut or a plot comes out as designed.

## The model

A font file that a text of the document uses is stored in the container like a picture: content-addressed by SHA-256 under `assets/<hash>.bin`, once per file, bytes unchanged. The document already names the face by that hash (`FontRef.face_hash`), so nothing in the document changes. The Fonts panel gets a group "Document Fonts" at the top that lists the fonts the document uses; each row says whether the font is installed on this device. Workspace and device sync carry the font like a picture (asset record), so a joined device has the font before or right after the text.

"Installed" means: a face with the same family and style is listed in Curvyo fonts, Included or This computer.

## Questions (defaults apply unless the customer answers)

- **FONT-LICENCE:** a font whose OS/2 `fsType` forbids embedding (bit "restricted licence", value 2). A: not embedded, the text shows as missing on devices without the font, as today (default; respects the font's licence). B: embedded anyway as the customer wrote ("all fonts").
- **FONT-SAME-NAME:** an installed font has the same family and style as an embedded one but other bytes. A: the embedded bytes draw the text (default; the file looks as authored). B: the installed font draws it, as before this feature.
- **FONT-INSTALL-MEANING:** the installed test above compares family and style names, not bytes (default). Alternative: compare bytes, but This computer faces have no hash before they are read.

## Acceptance criteria

Setup: font X is a TTF that is not installed on device B; text T in the document uses X on device A. "Installed" is as defined above.

### Embedding

1. Given T uses X, when the document is saved, then the container holds exactly one `assets/<hash of X's file>.bin` member whose bytes equal X's file bytes. Given three texts use X, then there is still one member. Given a font that no text uses, then no member for it is added.
2. Given a document that uses two faces of one font file (a collection), then the file is stored once.
3. Given T's font is changed to Y and the document saved, then Y's member is present; X's member stays until the document wipe (like a picture, `0058`), and a wipe keeps exactly the fonts a text in the tree uses.
4. Given a font file larger than the free room of the project's asset budget (`MAX_PROJECT_ASSET_BYTES`, shared with pictures), then it is not embedded, a notice names the font, and the text behaves as without this feature.
5. `format_version` does not change. A build without this feature opens the file, ignores the members and, when it saves, writes the file without them.

### Opening

6. Given a saved file with T in X and X not installed on B, when B opens it, then T is drawn in X (not in the fallback face), the Text bar shows no "missing" state for T, and a cut path of T equals the path on A to the document tolerance.
7. Given X is installed on B under the same bytes, then T is drawn in the installed face and the Fonts tab count of texts using the family (Remove prompt of `0075` criterion 25) counts T.
8. Given an installed font with the same family and style but other bytes (FONT-SAME-NAME A), then T is drawn in the embedded bytes.
9. Given a file from before this feature (no font members), then it opens as before; saving it embeds the used fonts that are available on the device.

### The Document Fonts group

10. The Fonts tab shows a group "Document Fonts" above "Curvyo fonts" with one family row per family used by any text of the open document and the count of those families. A document without text has no such group. The group follows search and the tag strip like the other groups.
11. Given the family is installed, then its row has the star and the + (tags) enabled, no trash can and no install icon.
12. Given the family is not installed, then the star and the + are greyed out (not operable, not focusable as buttons), the install icon (the icon of Add font in the top row) is shown where Curvyo fonts rows have the trash can, and the row's tooltip names the action ("Install font").
13. A Document Fonts row never has a trash can.
14. Given the install icon is pressed (mouse or keyboard), then the family's embedded file goes the way of Add font (its checks, archive handling and notices) into the Curvyo folder with bytes unchanged, the family appears in Curvyo fonts, and the Document Fonts row becomes an installed row (criterion 11) without reopening the tab. A font that cannot be installed shows the notice of Add font and keeps its row.
15. Given a row of a face is picked (click, Enter), then the selected texts get that face, as in the other groups.

### Sync (completes `0092`, `0095`, `0096`)

16. Given A edits T in X in a Synced workspace and B (X not installed) is in it, then B shows T in X within 10 s of the text arriving, B's Document Fonts shows X as not installed, and no text of B is "missing".
17. Given A types a new text in X later, then the font reaches B as one asset record and exactly once per device (a record already held changes nothing).
18. Given B is offline, A uses a new font, B comes back, then B receives the font like a picture placed offline (`0092` criterion on assets).

### Unchanged

19. Export to SVG, laser and cutting output, and the text engine give the same result as before for a font that is installed.
20. The Share, Fonts and Document tabs show no new explanatory sentences (customer dislikes hints); labels, counts and real errors only.
21. FONT-LICENCE A: given X has `fsType` restricted, then no member is written, the row of its family is not listed under Document Fonts, and T is missing on B as today.

## Out of scope

Font subsetting, web fonts, compressing the members (fonts are Stored like pictures; Deflate would save space and is a possible follow-up), a size limit of our own (reported in `docs/technical-debt.md` instead), embedding fonts used only by history.

## Touches earlier specs

`0075` (Fonts tab groups, count of texts using a family), `0074` (missing-font state), `0058` (asset budget shared), `0092`/`0095` (asset records).
