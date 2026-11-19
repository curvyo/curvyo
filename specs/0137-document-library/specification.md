# Document library: take a page, or just the elements of a page, from any of my documents as a plain copy

Status: Draft (decisions of 2026-11-19 applied; not Ready: `adrs.md` and UX notes are missing)
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

1. Given the File tab or a Library tab (UX decides, **Proposal**: its own tab in the Properties strip, so it can stay open while drawing), then it lists the sources (Frage 268 B and C): the open workspace first, then the other workspaces of the recent list, each as a tree of its documents (the workspace tree of `0095`), then the folders the maker added by hand (an "Add folder" button; each folder is read recursively for `.curvyo` files; removing one is an inline second press and deletes nothing on disk; the list of added folders is kept per user in the app data folder, `R-ASSET-003`). Opening the list reads only file names and each document's embedded thumbnail data.
2. Given a document in the list, then its row expands to its pages with a thumbnail per page (name, size, kind chip); reading the pages does not mark anything dirty, does not touch the file's timestamp and does not take a lock (a document open in another window stays untouched).
3. Given a search field, then it filters documents and pages by name (substring, case-insensitive) across all sources, live, within 100 ms of thread CPU time for 1,000 documents from the index (the index is built in the background and kept per user, in the app data folder, not in the project **(Proposal)**).
4. Given a page row, when the maker drags it onto the canvas of the open document (or presses its Import page button), then the page is added as a **new page** of the open document after the last page (`0059`), with its size, kind (a laser or sketch page imports as such when the machine profile can be copied (`0039`), otherwise as a drawing page with a notice that names the reason), objects, layers and clone relations inside the page intact; one undo step "Import page". The source is unchanged.
5. Given a page's thumbnail and a marquee in the thumbnail preview (or Ctrl-click on objects of the opened page preview), then the maker can pick **elements** of that page and drag them (or press "Import elements") onto the active page of the open document: they land at the drop point, or, from the button, centred on the visible canvas **(Proposal)**, as an ordinary selection, one undo step "Import elements"; groups and linked clones among them keep their structure; a linked clone whose original was not picked is imported as an ordinary path (the `0049` rule of a deleted original) and the notice says how many.
6. Given an imported page or elements, then every copied top-level page and object carries **Came from** (Words). Given a second import of the same page, then a second copy is made, never merged and never replaced.
7. Given the Layers row or the Style tab of an imported object, then the Came from line reads "From <document> › <page>" and, if the source document still exists in a known source, a button "Open source" opens it and activates that page (a normal open, with the unsaved-changes check of `0051`); if it no longer exists the line stays and the button is absent.
8. Given the imported objects' resources, then what they need is copied with them: gradients and fonts used in text are embedded as the document already embeds them (`0107` document fonts); a font that is not installed on this machine keeps its name and draws with the fallback of `0107`; document formats and material records are not copied.
9. Given the **Came from** info, then it is plain data in the document, `format_version` change by the architect (the minimum: an optional record on page and object), a file without it opens unchanged, and it is not exported to SVG as geometry (a `data-` attribute or a comment is a **Proposal** for the exporter of `0024`).
10. Given a library source that is unreadable (moved, offline drive, damaged, newer format), then its row shows "Cannot be read" with the reason and the other sources work; nothing else is affected.
11. Given a document that is open and has unsaved changes, then the library shows the **saved** file's state for it, not the open state, with the label "Saved version" **(Proposal)**; the library never reads another window's memory.
12. Given `R-SYS-007`, then the whole slice reads local files only: no network access and no telemetry; a workspace folder that sync keeps current (`0096`) is read as the files on disk.
13. Given 500 documents with 8 pages each, then the list is virtualised and the first screen of rows renders within 200 ms of thread CPU time; thumbnails load lazily for visible rows only.

## Out of scope

- Live links to the source, "Update from source", "Replace with the source's current page", merge of later source changes (Frages 269 and 270 A; `0100` is the live route).
- Writing into the source (moving or copying **to** another document), editing a source from the library.
- Templates over the network (`0138`), shared records of a workspace (`0095` slice 3), searching inside object contents or text.
- Importing from SVG, DXF or image files (`0024`, `0058` import from the file system).

## Decisions

Customer, 2026-11-19:

- **Frage 268 (DOCLIB-SCOPE): B and C.** The open workspace, every workspace of the recent list, plus single folders the maker adds by hand (criterion 1).
- **Frage 269 (DOCLIB-LINK): A.** Copies only; the library never creates a link. Live is `0100`.
- **Frage 270 (DOCLIB-REFRESH): A.** No refresh or replace command; the Came from line is all there is (criterion 7).

## Still open before Ready

- `adrs.md` (architect: the document index, thumbnail read without opening, the `format_version` change for Came from).
- UX notes (the library surface, shared with `0138`).

## UX notes

(ux-engineer, before Ready.) Surface shared with `0138`.

## Links

Requirements: R-ASSET-006 (new), R-ASSET-001 to -003 (related), R-SYS-011, R-SYS-007
Depends on: `0095`, `0059`, `0049`, `0107`, `0051`
Related: `0100`, `0116`, `0138`
PR: none yet
