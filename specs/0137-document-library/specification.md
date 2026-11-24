# Document library: take a page, or just the elements of a page, from any of my documents as a plain copy

Status: Done (PR #470; Ready 2026-11-24; ADR 0084 accepted with PR #470; Frage 341 answered C 2026-11-24)
Priority: Should
Origin: Customer (2026-11-19, translated): a library of all workspaces, documents and pages so that a page, or only the elements of a page, can be imported easily; an import is a plain copy that carries a reference info "where it came from". Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As a maker with many documents (a workspace of box designs, a folder of logos, last year's jobs) I want to browse all of them in one place, see their pages as thumbnails and pull one page or a few elements into the document I am working on with one gesture, so that I reuse my own work without opening, selecting and copy-pasting between windows.

**What other tools do.** Inkscape: File > Import of a whole `.svg` file or copy and paste between windows; no browser of own documents. Illustrator: Libraries panel (cloud) and Bridge. Affinity: Assets panel, per document. Figma: team library of components (live-linked). LightBurn: Library of saved layouts (file based). Our own `0100` already brings content in as **live** linked clones or placed pages; this entry is the other half: a **dead copy**, independent after the drop.

## Words

- **Source:** a document the library knows, in a workspace (`0095`) or a folder the maker added. Read only: browsing never opens it for editing and never writes into it.
- **Copy import:** the pages or elements are copied into the open document as ordinary objects; nothing links them to the source afterwards (no clone, no placed page).
- **Came from:** a small info record on the copied page or object: source document name, page name, the item's name where it has one, and the date and time of the import. Plain data, shown in the Layers row's tooltip and in the Style tab's origin line; it never changes how the object draws or outputs. It survives save and reopen and copy and paste inside the document.

## How this relates to existing things

This idea is close to three existing things; the spec draws the lines (Fragen 268 to 270 are decided, see Decisions):

| Existing | Relation |
|---|---|
| `0100` bring-content-in (Done) | live clones and placed pages. This entry never creates a link. The library offers no link (Frage 269 A). |
| `0095` workspace (Done), slice 3 outline "workspace library" | that outline is the library of **shared records** (materials, fonts, palettes) inside a workspace; this entry is a library of **documents**. They share the browsing surface only. `0116` syncs the per-user libraries. |
| `0049`, `0057` clones | not touched. |
| R-ASSET-001 to -003 | assets are things the maker **saved on purpose** as templates; this library needs no saving step: every document is already in it. The templates of `0138` reuse the same surface. |

## Acceptance criteria

1. Given the Library tab in the Properties strip (after Fonts, **Proposal**, so it can stay open while drawing; UX notes), then it exists in the desktop **and** the browser build (Frage 341 C, answered 2026-11-24) and lists the sources of its build (Frage 268 B and C). **Desktop build:** the open workspace first, then the other workspaces of the recent list, each as a tree of its documents (the workspace tree of `0095`), then the folders the maker added by hand (an "Add folder" button; each folder is read recursively for `.curvyo` files; removing one is an inline second press and deletes nothing on disk; the list of added folders is kept per user in the app data folder, `R-ASSET-003`). Opening the list reads only file names and each document's embedded page pictures (criterion 2). **Browser build:** the documents stored in the browser itself (its own browser storage), each as a document with its pages like a workspace document; there are no workspaces, no recent list and no "Add folder" there, and the file-based sources do not appear (E1, **Proposal**). Everything else below (search, import, Came from, "Saved version") works the same on both builds; in the browser a file on the maker's disk is not read by the library.
2. Given a document in the list, then its row expands to its pages with a thumbnail per page (name, size, kind chip); the pictures are written into the file at save, 224 x 160 px, at most 32 pages, so a document saved before this slice lists its pages without pictures until it is saved once (E8, **Proposal**, ADR 0084); reading the pages does not mark anything dirty, does not touch the file's timestamp and does not take a lock (a document open in another window stays untouched).
3. Given a search field, then it filters documents and pages by name (substring, case-insensitive) across all sources, live, within 100 ms of thread CPU time for 1,000 documents from the index (the index is built in the background and kept per user, in the app data folder, not in the project **(Proposal)**).
4. Given a page row, when the maker drags it onto the canvas of the open document (or presses its Import page button), then the page is added as a **new page** of the open document after the last page (`0059`), with its size, kind (a laser or sketch page imports as such when the machine profile can be copied (`0039`), otherwise as a drawing page with a notice that names the reason), objects, layers and clone relations inside the page intact; one undo step "Import page"; the imported page becomes the active page (E5, **Proposal**). A placed page whose drawing page is not imported comes in as ordinary copies, and a laser page whose manufacturing data this build cannot read comes in as a drawing page (ADR 0084). The source is unchanged.
5. Given a page row opened in the page preview pane, then the maker picks **elements** of that page with the Select tool's click, Shift+click and marquee in the pane (E2, **Proposal**) and drags them (or press "Import elements") onto the active page of the open document: they land at the drop point, or, from the button, centred on the visible canvas **(Proposal)**, as an ordinary selection in the active layer of the active page (E6, **Proposal**), one undo step "Import elements"; groups and linked clones among them keep their structure; a linked clone whose original was not picked is imported as an ordinary unlinked object of its own kind, a rectangle stays a rectangle (the `0049` rule of a deleted original; wording 2026-11-24 after the acceptance test) and the notice says how many. Elements cannot be dropped onto a sketch page (the `0099` refusal; E4, **Proposal**): the drop is refused with the reason and nothing is imported. Importing from the document open in this window is allowed, from its saved file (E10, **Proposal**).
6. Given an imported page or elements, then every copied top-level page and object carries **Came from** (Words). Given a second import of the same page, then a second copy is made, never merged and never replaced.
7. Given the Layers row, the Style tab (its last block), the Document tab or the Pages list tooltip of an imported object or page, then the Came from line reads "From <document> › <page>" (E3, **Proposal**) and, if the source document still exists in a known source, a button "Open source" opens it and activates that page; the source is found by the page id stored in the record, so a moved or renamed document is still found, and with several copies the one with the same name wins (E9, **Proposal**, ADR 0084) (a normal open, with the unsaved-changes check of `0051`); if it no longer exists the line stays and the button is absent.
8. Given the imported objects' resources, then what they need is copied with them: gradients and fonts used in text are embedded as the document already embeds them (`0107` document fonts); a font that is not installed on this machine keeps its name and draws with the fallback of `0107`; document formats and material records are not copied.
9. Given the **Came from** info, then it is plain data in the document, `format_version` 58 (**next free at merge**; main writes 57 since `0145` slice 2; ADR 0084; the minimum: an optional record on page and object), a file without it opens unchanged, and SVG export writes nothing of it (**Proposal**, ADR 0084). A result of Boolean, Combine or Weld keeps Came from only when all its sources had the same one (**Proposal**, ADR 0084).
10. Given a library source that is unreadable (moved, offline drive, damaged, newer format), then its row shows "Cannot be read" with the reason and the other sources work; nothing else is affected.
11. Given a document that is open and has unsaved changes, then the library shows the **saved** file's state for it, not the open state, with the label "Saved version" **(Proposal)**; the label is certain only for the document open in this window (E7); for a document open in another window the library may show the saved file without the label, because it never reads another window's memory.
12. Given `R-SYS-007`, then the whole slice reads local data only (files on the desktop, the browser's own storage in the browser): no network access and no telemetry; a workspace folder that sync keeps current (`0096`) is read as the files on disk.
13. Given 500 documents with 8 pages each, then the list is virtualised and the first screen of rows renders within 200 ms of thread CPU time; thumbnails load lazily for visible rows only.

## Out of scope

- Live links to the source, "Update from source", "Replace with the source's current page", merge of later source changes (Frages 269 and 270 A; `0100` is the live route).
- Writing into the source (moving or copying **to** another document), editing a source from the library.
- Online sources in the Library tab (`0138`, which adds its Online section to both builds), browsing disk folders or files from the browser build,
- Templates over the network (`0138`), shared records of a workspace (`0095` slice 3), searching inside object contents or text.
- Importing from SVG, DXF or image files (`0024`, `0058` import from the file system).

## Decisions

Customer, 2026-11-19:

- **Frage 268 (DOCLIB-SCOPE): B and C.** The open workspace, every workspace of the recent list, plus single folders the maker adds by hand (criterion 1).
- **Frage 269 (DOCLIB-LINK): A.** Copies only; the library never creates a link. Live is `0100`.
- **Frage 270 (DOCLIB-REFRESH): A.** No refresh or replace command; the Came from line is all there is (criterion 7).

Customer, answered 2026-11-24:

- **Frage 341 (DOCLIB-WEB): C, answered 2026-11-24.** The browser build also has the Library tab and lists the documents stored in the browser itself (criterion 1); the tab is not desktop-only.

Open, with defaults (customer may veto at the demo):
- **ADR 0084** (document model, file format) is Proposed, `needs-customer`; accepted with the slice PR.

## Spec deltas of 2026-11-24

UX notes E1 to E10 and ADR 0084's notes are taken into the criteria above as **(Proposal)**; none changes a decision of 2026-11-19 (Fragen 268 to 270). E4 (the sketch-page refusal) is new behaviour inside the story and reversible in one PR.

## UX notes

See `ux-notes.md` (ux-engineer, 2026-11-24): the Library tab (after Fonts in the strip, both builds per Frage 341 C; the notes' desktop-only wording is superseded), the source tree with page rows, the page preview pane for picking elements, the drags and Import buttons, Came from line and Open source, Saved version, empty, loading and error states; proposals P1 to P25, spec deltas E1 to E10 (accepted above), Frage 341 (DOCLIB-WEB, answered C). The "Online" section of the same surface is in `../0138-template-library/ux-notes.md`. Drafts: `docs/design-drafts/2026-11-library/`.

## Links

Requirements: R-ASSET-006 (new), R-ASSET-001 to -003 (related), R-SYS-011, R-SYS-007
ADRs: `adrs.md`, ADR 0084
Depends on: `0095`, `0059`, `0049`, `0107`, `0051`
Related: `0100`, `0116`, `0138`
PR: none yet
