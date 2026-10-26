# Font info and fonts from zip archives

Status: Ready (2026-10-26; Frage 41 unanswered, built with default A; UX notes applied, Info only from the mode switch as Proposal)
Priority: Should (R-FONT-008 for the embedding permission; R-FONT-006 for Add font)
Origin: Customer. (a) The font info view was `0077` Frage 25 C and its Out of scope ("licence, file, format, embedding permission (`fsType`, R-FONT-008), version"). (b) Request of 2026-10-23 (translated): Add font should take .zip archives, "then you don't have to unpack yourself; you often find the licence and credits there; we can simply keep those". Items marked **(Proposal)** are the product owner's; safe defaults the customer can veto at the demo.
Depends on: `0074-text-objects` (blocks, the Curvyo folder, font matching), `0075-font-library` (Add font, drop, de-duplication, the notice, Remove from Curvyo fonts, the browser's IndexedDB store), `0076-single-line-fonts` (SVG fonts, the hostile-file limits pattern of its criterion 23), `0077-font-preview` (the pane), `0082-glyph-table` (the mode switch and the subject face; whichever of `0082` and this story is built first brings the switch).

## User value

As a maker who buys or downloads fonts for a sign shop, I want to add the .zip I downloaded as it is, have Curvyo keep the licence and readme that came with it, and see for any font its licence, file, format, version, designer and embedding permission, so that I neither unpack archives by hand nor lose track of whether I may use a font for a commercial job.

## Field reference

| Tool | Font info | Archives |
|---|---|---|
| macOS Font Book | Info view: version, location, copyright, trademark, languages, embeddable or copy protected ([AppleInsider on Font Book 3](https://appleinsider.com/articles/11/02/28/inside_mac_os_x_10_7_lion_font_book_3_emoji_support)); the licence under Info > Usage ([Apple](https://support.apple.com/en-vn/guide/font-book/fntbk1014/mac)) | No: unzip first |
| FontBase | Font details per font | Add by drop or the +Add button for files or folders; no zip mentioned ([docs](https://fontba.se/learn/adding-fonts)) |
| FontExplorer X Pro | Name, style, format, version, copyright, embedding rights, languages ([listing](https://mac.softpedia.com/get/Font-Tools/FontExplorer-X-Pro.shtml)) | - |
| TransType | - | Accepts .zip archives with font files ([FontLab help](https://help.fontlab.com/transtype-5/articles/adding-fonts/)) |
| Inkscape, LightBurn, Affinity | None in the app | No: fonts are installed in the operating system first |
| FontDrop! (web) | Name and licence fields of one dropped file | No |

**What we do better.**
- **The download goes in as it is**: a .zip from a foundry or Google Fonts is added in one step, nested folders included, and the licence and readme stay with the fonts instead of being lost in the Downloads folder.
- **Licence next to the font where the maker picks it**: the info view of the font in use shows its licence text, the kept documents and the embedding permission in words. No tool in the field keeps the archive's licence files with the font.
- **Hostile archives are harmless**: zip bombs, `../` paths and symlinks are refused or skipped with stated limits, on the desktop and in the browser alike.
- **No verdict we cannot back**: the app shows what the file and the documents say; it never claims a font is "free for commercial use".

## Terms

- **Info view:** the third mode of the `0077` pane, beside Fonts and Glyphs (`0082` criterion 1). It shows the **subject face** of `0082` (the face of the text in edit mode or the one selected text, else the next-text face).
- **Archive:** a file with the extension `.zip` (any case) in the ZIP format.
- **Font entry:** an archive entry with the extension `.ttf`, `.otf`, `.ttc` or `.svg` that the Curvyo block's listing rules accept (`0075` criterion 32, `0076` criteria 1 and 23).
- **Document entry:** an archive entry kept beside the fonts: a base name starting (ignoring case) with `OFL`, `LICENSE`, `LICENCE`, `COPYING`, `COPYRIGHT`, `README`, `READ ME`, `CREDITS`, `AUTHORS`, `FONTLOG` or `EULA`, with no extension or the extension `.txt`, `.md` or `.pdf`; and every other entry with the extension `.txt`, `.md` or `.pdf`. (Proposal)
- **Text document:** a document entry with no extension, `.txt` or `.md`.
- **Font folder** (Frage 41, default A): one subfolder of the Curvyo folder per added archive, holding its fonts and documents.
- Notices at most 70 characters per part, in the slot and wording rules of `0075` criterion 22. Budgets on the plan's machine.

## Acceptance criteria

### Part A: the info view

1. Given the `0077` pane, then the mode switch of `0082` criterion 1 has a third cell, **Info**; Fonts, Glyphs and Info share the pane's split, full view, divider, Close and Escape rules (`0077` criteria 2 to 5, 27), and the pane reopens in the last mode (`0082` criterion 3). The subject line reads "Great Vibes Regular · Included".
2. Given Info mode and a readable subject face, then it shows these fields as label and value rows, in this order; a value the font does not state reads "Not stated", never an empty row:
   - **Family** and **Style** (as the Fonts list shows them), **Full name** (OpenType name ID 4).
   - **Format**: "TrueType (TTF)", "OpenType CFF (OTF)", "OpenType CFF2", "TrueType collection (TTC), face 2 of 4", "SVG font, single-line".
   - **File**: the file name and its size ("Lobster-Regular.ttf · 412 KB") and the block ("Curvyo fonts", "Included", "This computer", "Browser storage"); on the desktop for Curvyo fonts and This computer also the folder path, as selectable text.
   - **Version** (name ID 5).
   - **Designer** (name ID 9), **Vendor** (name ID 8, else the OS/2 `achVendID` code), **Copyright** (name ID 0), **Trademark** (name ID 7).
   - **Licence**: for an included font the licence of its `font.toml` in words ("SIL Open Font License 1.1", "Apache License 2.0", "Hershey notice") and its shipped licence text; for any other font the description of name ID 13 and the URL of name ID 14, as selectable text (no link: Curvyo opens no web page by itself, R-SYS-007).
   - **Embedding** (R-FONT-008), from OS/2 `fsType` in words: 0 "Installable", bit 1 "Restricted: may not be embedded", bit 2 "Preview and print", bit 3 "Editable", plus "no subsetting" (bit 8) and "bitmap only" (bit 9) when set; an SVG font: "Not stated (SVG font)". The value is information only: no permission ever blocks or changes the font's use, its conversion to paths or any output.
   - **Glyphs**: the face's glyph count (`maxp`; SVG: the number of `<glyph>` elements) and **Characters**: the count of `0082` criterion 4 ("412 characters").
   - **Documents**: criterion 4.
   Test: fixture faces with each `fsType` value, one with every name ID, one with none, an SVG font, a TTC; values as listed.
3. Given a long value (a copyright or licence description over 3 lines), then it shows its first 3 lines and a **Show all** text button that expands it in place; the expanded state is not kept. All values are selectable for copying. (Proposal)
4. Given a subject face from a font folder (criterion 9) that holds documents, then **Documents** lists them by file name, A to Z ignoring case. A press on a text document shows its text below the list, in place, as plain text with its line breaks, wrapped to the pane, at most the first 1 MiB (then "Shown: the first 1 MiB."); the text is never interpreted as Markdown or HTML. A press on a `.pdf` opens it in the system's PDF viewer on the desktop and in a new browser tab from the stored bytes in the browser build (no network request). A face outside a font folder has no Documents row. (Proposal for the PDF handling.)
5. Given the subject face changes, then the info view shows the new face within 100 ms (the fields are read from the face's tables, not from a second file read). Given the subject face is missing (`0074` criterion 29), then the view shows "Lobster is missing." and the family name only.
6. Given the browser build, then criteria 1 to 5 hold the same way for the fonts the browser lists; the path of criterion 2 is not shown there.

### Part B: Add font with .zip

7. Given Add font… (`0075` criterion 20, `0076` criterion 3), then its file filter also offers `.zip`, and archives may be chosen together with font files; a drop on the Fonts tab (`0075` criterion 24) accepts archives the same way.
8. Given an archive, then every entry is looked at, folders nested to any depth included: each font entry is read and accepted by the rules of a font file (`0075` criterion 21), each document entry is kept, and every other entry is left out. Entries under a `__MACOSX/` folder and entries whose base name starts with `._` or `.` are left out without being counted.
9. Given an archive with at least one font to add (Frage 41, default A), then a **font folder** named after the archive's base name (`Lobster.zip` gives `Lobster/`; a taken name, ignoring case, gives `Lobster-2/`, `-3`, ...) is created in the Curvyo folder and holds the archive's added fonts and its kept documents side by side, flat: the archive's inner folders are not recreated, and two entries with the same base name become `OFL.txt` and `OFL-2.txt`. File and folder names are made safe for Windows, macOS and Linux (characters `<>:"/\|?*` and controls replaced by `_`, reserved names such as `CON` prefixed with `_`, at most 120 characters, trailing dots and spaces removed).
10. Given the Curvyo folder (desktop) or the browser's font store, then the listing of `0074` criterion 19 lists the fonts directly in it **and in its subfolders one level deep** (a font folder of criterion 9, or a folder the maker made by hand); deeper folders and folders whose name starts with `.` are not read. This amends `0074` ("non-recursive") and `0075` Out of scope ("Fonts in sub-folders of the Curvyo folder"). A family keeps its star and tags whichever folder its file is in (`0075` criterion 11).
11. Given de-duplication (`0075` criterion 21), then a font entry whose bytes equal a font file anywhere in the Curvyo folder (top level or a subfolder) is "already there" and not copied; when an archive adds no font, no font folder is created and its documents are not kept. Inside one archive, the same face offered twice (the same family and style, for example `Lobster.otf` and `Lobster.ttf`) is added once, the first in the order TTF, OTF, TTC, SVG; the others count as "already there". (Proposal)
12. Given the archive is added, then it is added as a whole: the font folder appears complete or not at all (built under a temporary name starting with `.` in the Curvyo folder, then renamed); a cancelled, failed or interrupted add leaves no font folder and no partial file, and a leftover temporary folder is removed at the next start. The archive itself is not copied and stays where it was.
13. Given the result, then one notice (`0075` criterion 22) reports archives with the fonts and files: "Lobster.zip: 2 fonts added, 3 documents kept." / "Lobster.zip: 2 fonts were already there." / "Lobster.zip holds no font. Nothing was added." / "Pack.zip could not be read. Nothing was added." / "Pack.zip is over the size limit. Nothing was added."; counts of several archives and loose files are combined ("5 fonts added. 4 documents kept."). Left-out entries are not reported, skipped hostile entries (criterion 16) are counted as "2 files were skipped as unsafe.". The new families appear and are scrolled into view as `0075` criterion 22 says, and the tag rule of `0075` criterion 23 applies to them. Adding an archive of 20 fonts and 10 MB is done within 2 s.
14. Given **Remove from Curvyo fonts** (`0075` criterion 25) on a family whose files are in a font folder, then its files are deleted, and a font folder left with no font file is deleted together with its documents; the armed trash then says so (the inline confirm of `0081` criterion 3): its accessible name, tooltip and polite status read "Press again to remove Lobster from Curvyo fonts. Its 3 documents go too." plus, when texts use the family, " 2 texts here use it." ("1 document" in the singular). The documents sentence appears only when this removal takes the font folder's last font and that folder holds documents. A font folder that still holds another family's font keeps its documents.

### Part C: hostile archives

15. Given an archive, then these limits hold and are checked on the bytes actually read, never on sizes the archive declares: the archive file at most **256 MiB**; at most **10,000 entries**; at most **256 MiB** uncompressed in total for the entries kept (fonts and documents); a font entry at most 64 MiB (ADR 0020 §2), a document entry at most 16 MiB; at most **500 fonts** per archive. An archive over the file, entry or total limit is refused as a whole ("is over the size limit"); an entry over its own limit is skipped as unsafe. (Proposal: the numbers; the architect may lower them for the browser's memory.)
16. Given a hostile or damaged archive, then reading it never crashes, hangs or writes outside the font folder, and: an entry name with `..` as a path part, an absolute path, a drive letter or a UNC prefix is skipped as unsafe (names are never used as paths: only the sanitised base name of criterion 9 is); symlink entries and entries of other non-file types are skipped as unsafe; entries whose data overlap another entry's (the overlapping-file zip bomb) make the archive unreadable; an entry whose decompressed bytes exceed its declared size, or whose CRC-32 does not match, is skipped as unsafe; encrypted entries and compression methods other than stored and deflate are left out; an archive inside the archive (`.zip`, `.7z`, `.rar`, `.tar`, `.gz`) is never opened and is left out; a file that is not a ZIP is "could not be read". Entry names in UTF-8 (flag set) or CP437 are read for display; a name that decodes to neither is shown with `_` for the bad bytes. Golden fixtures in `tests/fixtures/fonts/zip/`: a plain archive, nested folders, `__MACOSX`, two families with the same `OFL.txt`, a `../` entry, an absolute entry, a symlink, a 42.zip-style nested bomb, an overlapping-entry bomb, a high-ratio single entry over the total limit, a wrong CRC, a declared size smaller than the data, an encrypted entry, 10,001 entries, a non-UTF-8 name, an archive with no font; and a fuzz target on the archive reader, as for fonts (ADR 0020 §2).
17. Given a font entry or document read from an archive, then the font is checked by the same reader and limits as a loose file (`0075` criterion 32, `0076` criterion 23) before anything is kept; a document's content is never executed or rendered beyond plain text (criterion 4).

### Part D: browser and the core rule

18. Given the browser build, then criteria 7 to 17 hold with the font folder kept in the browser's font store (`0075` adrs decision 7, IndexedDB): one archive is one transaction, so a quota error keeps nothing and shows the `0075` criterion 30 notice; nothing is uploaded and no network request is made (R-SYS-007). Fonts and documents survive a reload and a browser restart as `0075` criterion 29 states.
19. Given the core crates, then they read no file: reading an archive from its bytes (entry list, limits, the font, document and left-out classes, name sanitising, CRC check, decompression) and reading the info fields of criterion 2 from a face's bytes are pure functions in `*-core` crates that build for `wasm32-unknown-unknown`, so the desktop and the browser share one set of limits; creating the font folder, the atomic rename and the IndexedDB transaction are platform code (`CLAUDE.md` §6). No project `format_version` change and no change to the font library file.

## Out of scope

- Other archive formats (7z, RAR, tar, tar.gz), nested archives, password-protected archives (criterion 16 leaves them out).
- WOFF and WOFF2 inside archives (as `0075`: not a font for Curvyo); variable-font instances beyond what `0074` lists.
- Interpreting a licence: no "commercial use allowed" verdict, no licence detection from the documents' text, no warning for restrictive `fsType`; R-FONT-008 says the permission is shown and does not block use.
- Mapping documents to families inside a multi-family pack (every document of a font folder is shown for every font in it); recreating the archive's inner folders.
- Adding, editing or deleting single documents of a font folder in the app (the file manager can), documents for loose fonts in the Curvyo folder's top level (documents attach only to the fonts of their own subfolder), opening documents of other formats (`.html`, `.rtf`, images: left out).
- An Info button in the Text bar or the Select bar: Info is reached from the pane's mode switch only (Proposal, UX notes section 1; the `0084` second row is full).
- Info for a face that is not the subject face (see `0082` Frage 39), comparing the info of two fonts, languages or script coverage, the font's OpenType features, a "Show in folder" button.
- Exporting a font folder back to a zip, sync of font folders beyond what the data directory sync of `0075` gives.

## What the architect must decide (`adrs.md`)

1. **Zip reader**: which crate (pure Rust, deflate only, no `unsafe` beyond what `cargo deny` and ADR rules allow, permissive licence) or an own reader over a deflate crate; that it runs in a `*-core` crate on bytes for both targets (criterion 19), and whether that is a module of an existing crate or a new crate (new crate needs an ADR); how the limits of criterion 15 are enforced while decompressing (bounded reads, no trust in headers); the browser path (the same code in wasm; or the browser's `DecompressionStream` with JS, which would split the limits into two implementations: PO prefers one).
2. **Listing one level of subfolders** (criterion 10): `curvyo-storage-io`'s `fonts` module and the browser store's keys (`<folder>/<file>`); the face ids and the `0075` stamp for faces in subfolders; Remove's "directly in the Curvyo folder" check of `0075` decision 6 widened to one level.
3. **Atomic font folder**: temporary dot-folder, rename, cleanup at start (criterion 12) on Windows, macOS and Linux.
4. **Info fields**: a `curvyo-text-core` function from a face's bytes to the fields of criterion 2 (name IDs, `fsType`, `maxp`, format), including SVG font values; included fonts' `font.toml` licence in words.
5. **Opening a PDF** on the desktop (criterion 4): the opener the Tauri app already has or a plugin (dependency, licence); refusing anything but a `.pdf` inside a font folder.

## UX notes

See [`ux-notes.md`](ux-notes.md) (ux-engineer, 2026-10-26).

## Questions for the customer

Frage 41 (ZIP-FOLDER): Where do the fonts and the licence files from a .zip go?
- A: One folder per archive inside the Curvyo font folder (`fonts/Lobster/` with the fonts and OFL.txt, README, ...). Curvyo then also lists fonts in folders you make there yourself, one level deep. Remove deletes the folder with its documents when its last font goes.
- B: The fonts directly in the Curvyo font folder as today; the documents in a folder of their own per archive (`fonts/Lobster (documents)/`), linked to the fonts by name.
- C: The fonts directly in the Curvyo font folder; the documents are not kept (only the info the font file itself contains is shown).
- PO recommendation: A. Fonts and their licence stay together in one place you can also see in the file manager, and a folder of fonts you copy in by hand works the same way. B needs a link that breaks when you rename a file; C drops what you asked to keep.
- Default if unanswered: A.

## Links

Requirements: R-FONT-008 (embedding permission shown, does not block use), R-FONT-006 (own font folder, Add font), R-FONT-001, R-SYS-002, R-SYS-007
Depends on: `specs/0074-text-objects/`, `specs/0075-font-library/`, `specs/0076-single-line-fonts/`, `specs/0077-font-preview/`, `specs/0082-glyph-table/`
Changes to accepted specs: `0074` (the Curvyo folder is read one level deep, criterion 10), `0075` Out of scope ("Fonts in sub-folders of the Curvyo folder" moves into this story) and criterion 25 (Remove also deletes an emptied font folder, criterion 14)
ADRs: 0020 §2 and §4 (face limits, licence record), 0004 §7 (atomic writes), 0006 item 5 (font licences)
PR: -
