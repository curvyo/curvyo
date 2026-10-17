# Fonts: a Fonts card, groups and favourites, an own font folder, and a preview page of the lettering in every font

Status: Draft (a sketch for the customer; not Ready: text objects do not exist yet and glyph licensing needs the customer; preview placement decided 2026-10-17)
Priority: Should (big; the customer called it "a great feature")
Origin: Customer (request of 2026-10-17, translated): "A great feature. We need our own card in the properties and a tool to add. The font selection like the document sizes: groupable, e.g. initials, script, sans, serif, cuttable (fonts whose strokes end up connected so they laser well). Mark with * as favourites. We need our own folder under .config (directories crate) to add fonts just for Curvyo; they are shown first in the whole list. Harder: I would like a preview for fonts as seen in PewPew Studio: a page where exactly the lettering is shown with all fonts (e.g. of a group). We said no popups and honestly I do not want one. Maybe we need a new fixed panel, or a split page. It may make sense to build this together with the output, since the main canvas must be extended/switched there anyway."

## User value

As a maker who cuts and engraves names, monograms and slogans, I want to type my text once and see it in every font of a group at the same time, pick one, and know which fonts cut cleanly (connected strokes), so that I choose a font by looking at my own lettering and not by scrolling a name list one font at a time.

**What other tools do.** Inkscape: a font-family dropdown with a preview of the name, one font at a time, no groups (the Text and Font dialog shows a sample text, not yours in all fonts). Illustrator and Affinity: a font list with a sample of the name, favourites (stars) and filters by class. **PewPew Studio** (the customer's reference): a page that shows your lettering in all fonts as a grid; click one to use it. LightBurn: a plain font list, plus "Welded" text and a single-line font option. Ink/Stitch: its own lettering fonts, picked from a list. The gap we fill: your own text in many fonts at once, groups that mean something for a machine, and a font folder that belongs to Curvyo alone.

## What exists today (checked 2026-10-17)

There are no text objects: the node kinds are layer, group, path and primitive. No font code, no text tool, no spec for a Text tool. `R-EDIT-004` ("object to path converts shape or text") has no text yet. `R-FONT-001` to `004` are the only requirements. ADR 0004 item 12: fonts are referenced from a user-managed font collection directory and not copied into the project by default, with an embed option on export. ADR 0006 item 5: a font's own licence governs redistribution. So this sketch cannot be built before a text object exists (Question 1).

## Stages (Proposal)

| Stage | Delivers | Needs first |
|---|---|---|
| 1 | Text object, Text tool, font by name, Object to path; a Fonts tab with the list | the text engine ADR; a spec for the Text tool (no number yet) |
| 2 | Groups, favourites, the Curvyo font folder, "cuttable" tag, adding fonts | stage 1 |
| 3 | The preview page (criteria 14 to 17), split and full view | stage 2 and the pane host of `0063` |

## Acceptance criteria (as far as clear)

### The Fonts card

1. Given the sidebar strip (`0051`), then **Fonts** is a tab of its own in the Properties panel (icon tab, tooltip). It is always there, not only with text selected, so the maker can look through fonts first.
2. Given the Fonts tab, then the font list works like the document formats of `0045`: the groups are inline buttons (**All**, **Favourites**, then built-in groups **Initials**, **Script**, **Sans**, **Serif**, **Cuttable**, and the maker's own), each group can be switched off, a group may be added and renamed; a font can be in several groups (Proposal). The list shows the font's name set in the font itself, one line per font, virtualised.
3. Given a font row, then a star button marks it as a favourite (the customer's "*"); favourites are one group and a click on a star never moves the list under the pointer (the row keeps its place, only the star changes).
4. Given a text object selected, then the card marks its font; picking a row sets the font of the selection, one history step. With no text selected, a pick sets the font for the next text.
5. Given a font that is missing (a project from another machine), then the text still shows (a fallback), the card shows the missing name in the row of the object, and nothing is refused. A font can be embedded in the project by an explicit switch (ADR 0004 item 12); the default stays "referenced".

### The Curvyo font folder and adding fonts

6. Given the desktop app, then Curvyo has its own font folder in the user's configuration directory (found with the `directories` crate, under the application's folder, `fonts/`). A font file placed there (TTF, OTF; WOFF2 is a Proposal) is available to Curvyo only and installed nowhere else (`R-FONT-001`).
7. Given the list, then fonts from the Curvyo folder are shown **first**, above the bundled and the system fonts, in every group they belong to.
8. Given adding, then an **Add font** button in the Fonts tab opens the OS file dialog (the only layer, as Open) and copies the chosen files into the Curvyo folder; the new fonts appear in the list at once. A dropped file does the same (Proposal). Removing a font deletes the copy in the folder only, after the inline confirmation pattern of `R-SYS-010`.
9. Given the browser build, then there is no folder; fonts the maker adds live in the browser's storage (origin-private) and are listed the same way. System fonts are listed on the desktop (Proposal); in the browser only where the platform offers them.
10. Given the core crates, then they read no file: a platform crate (`storage-io`) finds and reads the folder and passes the bytes in; the core classifies and lays out from bytes (`CLAUDE.md` §6).

### Groups and "cuttable"

11. Given the built-in groups, then a curated list ships with the app (`R-FONT-004`): each shipped font has its group tags in one data file. A maker can change the tags of any font, including the shipped ones, in the card (a per-user override file, as `0045`).
12. Given **Cuttable** (`R-FONT-002`), then it means: the font is a single-line font, or its glyph outlines overlap and join along a word so that the welded result is few closed pieces (a script font). Whether the program detects this or the maker tags it is Question 3. A first version tags by hand and by a curated list.
13. Given `R-FONT-003`, then a group can be the only one shown (the others switched off), without touching any other application's font list.

### The preview page

14. Given a text, then a **Preview** button in the Fonts tab shows a page where the given lettering is set in every font of the current group (or the favourites, or a search), as a scrolling grid of tiles. The lettering is the text of the selected text object, else a line the maker types into a field on the page (kept per user), else the last one.
15. Given a tile, then a click applies the font to the selected text (or sets it for the next), a star marks a favourite, the tile shows the font name small below; the tile size is adjustable; tiles are drawn on demand so a group of 500 fonts opens in under a second to the first screen (budget in the plan).
16. Given the preview, then it is not a popup and not a modal. It opens as a **split view** next to the drawing and can be switched to a **full view** that fills the whole canvas area (a symbol button in the preview, the same one switches back), so it works on a very wide monitor and on a normal one. The pane host of `0063` supports both split and full.
17. Given a tile of a font that cannot render the typed characters, then the tile shows what it can and a quiet marker; the font is not hidden (permit over refuse).

## Licensing and text-to-path (notes, not criteria)

- Shipped fonts must be OFL, Apache-2.0 or public domain; each gets a licence record (ADR 0006 item 5). A font the maker adds is the maker's responsibility; the app shows the embedding permission of the font (the `fsType` bits) in the row and does not block use, because a laser cut is not an embedding.
- Cut output uses outlines. Text-to-path at export or by "Object to path" gives paths in millimetres; the font file is not needed to open the result. Kerning, ligatures and shaping need a shaping library: an ADR for the text engine (candidates: `rustybuzz` with `ttf-parser` or `skrifa`; both permissive; wasm-friendly) before stage 1.

## Out of scope

- Text on a path, text in a shape, multi-line layout beyond a first slice, variable-font axes, colour fonts (each its own entry).
- A font marketplace or a download service (no provider identified; ADR 0007 question 3).
- Font editing, glyph editing.
- Stitch fonts for embroidery (`0063` embroidery workbench).

## Decided (customer, 2026-10-17)

- **FONT-PREVIEW: split view,** and it must also switch to a complete full view of the preview, so it works on very wide and on normal monitors. The pane host of `0063` must support split and full view (criterion 16).

## Open questions (customer; each has a default)

Frage 1 (FONT-TEXT): Do text objects and the Text tool come first, as their own spec?
- A (default, recommended): yes; a Text spec (typing, size, one line, Object to path) is written next and built before the Fonts stages; the Fonts card then has something to apply to.
- B: one big spec that includes the text object.
- Default if unanswered: A. The lead gives the Text spec a number.

Frage 2 (FONT-CUTTABLE): Is "cuttable" a tag the maker sets, or does the program detect it?
- A (default): a tag from a curated list, editable by the maker; no detection in the first version.
- B: detection: the program welds a sample word and counts the closed pieces; fonts under a threshold get the tag automatically (more work, can be wrong for decorative fonts).
- Default if unanswered: A.

## Links

Requirements: R-FONT-001 to R-FONT-004 (existing); R-FONT-005 to R-FONT-008 (new)
Depends on: a Text spec (no number), `specs/0043-properties-tabs/` and `specs/0051-sidebar-and-no-menu/` (tabs), `specs/0045-document-formats-library/` (the list pattern)
Related: `specs/0063-manufacturing-workbenches/`, `specs/0065-extensions-sdk/` (font packs as extensions), `specs/0025-raster-trace/research.md` (text recognition), `specs/0022-color-management/`; ADR 0004 item 12, ADR 0006 item 5, ADR 0007
ADRs: a text engine ADR (shaping, layout, text object), `needs-customer` (document model); the pane host (split and full view) is part of `0063`'s workbench ADR
PR: -
