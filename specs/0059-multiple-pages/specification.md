# Multiple pages: several pages in one project, side by side on the canvas

Status: Done (PR #224; customer accepted and merged 2026-10-30)
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "In the page setup on the right side you can add a page like in Inkscape. It can have a different format, but it starts with the same as the last one. In the view it simply appears next to it with a bit of distance, similar to Inkscape. Especially interesting to put several coasters into one file for lasering, or, for a person who particularly likes an animal, a set of key rings, coasters and breakfast boards in one file. Very practical since fonts and graphics can be reused via linked clone, so you work very consistently."

This reverses the drop of 2026-10-05 ("not important enough for the MVP"; `0015`, `0046` out of scope, ADR 0012 rejected). Behaviour marked Proposal remains vetoable at the demo. ADR 0019 already settled that pages remain rectangular; templates are ordinary guide objects.

## User value

As a maker who cuts a whole family of products from one design (a coaster, a key ring and a breakfast board with the same animal), I want several pages in one project, each with its own size, so that I keep the shared parts (a logo, a font, an animal outline) once, reuse them as linked clones (`0049`, `0057`) on every page, and change them in one place.

As a maker who fills a sheet of material with identical or different parts, I want each part's page to be a page of its own, so that later each page can be placed on the machine's work area (`0063`).

**What other tools do.** Inkscape 1.2 and later: pages are rectangles on one canvas, drawn side by side; the page tool and a Pages dialog add, resize, rename and delete; objects belong to a page by position, not by a link; export writes one file per page or all. Affinity and Illustrator: artboards, the same idea, with names and an arrange command. Our model follows these: the canvas stays one plane, the pages are frames on it.

## Words used below

- **Page:** a named rectangle (position, width, height) on the canvas. Today's single document area (`0015`) becomes the first page.
- **Active page:** the page the Document tab shows and the rulers measure from. A view setting, per user, never undoable (ADR 0012 item 5 stands).
- **Pasteboard:** the canvas outside every page (`0015`).

## Acceptance criteria (as far as clear)

1. Given a new document, then it has exactly one page at 0, 0 with the default document size. Saving and reopening a current-format document preserves its pages. Compatibility with older files is not required (customer, 2026-10-29: unreleased SID development).
2. Given the Document tab, then it lists the pages (name, size) as rows under the existing size fields, and an **Add page** icon button (tooltip, accessible name). The row of the active page is marked. Selecting a row makes that page active and scrolls the view to it only if the page is not visible. No popup; nothing appears except the row and the page on the canvas.
3. Given Add page, then the new rectangular page has the size of the last page in the list (ADR 0019; guide objects are not copied), is placed to the right of the last page with a gap (Question 2), is named "Page N", becomes active, and is one history step "Add page".
4. Given the Width and Height fields of the Document tab, then they change the active page only; the other pages and their position do not move. Typed sizes follow `0015` and the formats of `0030` and `0045` apply to the active page.
5. Given a page, then it has a name (editable in its row), a **Delete page** icon button on its row, and a drag handle to reorder the list. Order is the order of the list only: reordering does not move pages on the canvas (Proposal).
6. Given Delete page, then the page frame goes and the objects that lay on it stay where they are on the pasteboard (permit over refuse; undo restores the frame). The last page cannot be deleted; the button is not shown on a single page.
7. Given the canvas, then every page is drawn with the document background (`0040`) and the document edge, the pasteboard around; the page name is drawn small above the top-left corner and the active page has the stronger edge. Nothing is clipped (as `0046` criterion 6).
8. Given the rulers, then their origin is the top-left corner of the active page (so a coaster's coordinates do not depend on where it sits in the row).
9. Given an object, then it belongs wholly or partly to one or several pages, by its position: every page whose rectangle its bounding box overlaps, or to no page (pasteboard). There is no stored membership (decided, customer 2026-10-17). Moving a page frame by its edge moves the objects that lie wholly on it with it, in one step; objects only partly on it stay; a switch in the row, off by default, moves the frame alone (Proposal).
10. Given a linked clone (`0049`) whose original is on one page and the clone on another, then it works as on one page: the original and clones are in one plane and one object tree.
11. Given `0050` layers, then layers are shared by all pages (a layer holds objects of any page, Question 1).
12. Given Select all (Ctrl+A, `0044`), then it selects the objects of the active page's context only when the pointer was last used on that page (Proposal; otherwise all pages).
13. Given undo, then page changes (add, delete, resize, rename, reorder, move) are steps in the document history (`0020`, `0042`) with operation names; the active page is not.
14. Given a project with 200 pages, then adding, switching and drawing do not stall (budget set by the implementer's plan).

## Implementation interpretation (2026-10-29)

- The customer requested this story; unanswered PAGE-LAYERS and PAGE-GAP use their stated defaults.
- Customer, 2026-10-29: old files do not matter during unreleased SID development. No backward compatibility or old-file migration requirement.
- [UX notes](ux-notes.md) define reversible interaction details and exact wrapping.
- [Architecture notes](adrs.md) preserve the accepted single-page resize behaviour. In a multi-page document, resize keeps frame origins fixed and recentres only wholly contained, non-shared objects by half the size delta; partial/shared objects stay (Proposal).
- Shapes in the original draft are superseded by customer-accepted ADR 0019: every page is rectangular; guides stay ordinary objects.

## Out of scope

- Printing and export of pages, SVG export per page (`0024` writes the whole drawing first; a page-per-file export is a later entry).
- Placing pages on a machine's work area, nesting (`0063`, R-MFG-004).
- Master pages, page numbers, page templates, a "duplicate page with content as linked clones" command (a natural follow-up, Proposal: it is exactly the key-ring-set workflow).
- Different units or different backgrounds per page (the background is one for all in this sketch).

## Architect notes (2026-10-17)

- The document model changes (a page list, a `format_version` bump; an older build would show one page and hide the others). The customer decided the page model (below), so the ADR needs only the page list. ADR 0012's content is reused: it rejected "pages are root nodes" for the MVP; this sketch takes the other shape.
- Coordinates stay global (millimetres, one plane). Page-local coordinates would break cross-page linked clones and the ruler rule above would be a view concern.
- The pages register is a list of `{id, name, x, y, width, height}` with per-field last-writer-wins (ADR 0009). The active page is ephemeral.

## Decided (customer, 2026-10-17)

- **PAGE-MODEL: A.** An object belongs wholly or partly to one or several pages, by its position; one object tree, no stored link (customer's wording). Linked clones work across pages. Criterion 9 is worded accordingly.

## Open questions (customer; each has a default)

Frage 1 (PAGE-LAYERS): Are layers shared by all pages or does each page have its own?
- A (default, recommended): shared; one Layers tab, a "Cut" layer holds the cut paths of every page, which is what a laser job wants (`0027`).
- B: per page; each page has its own "Cut" and "Engrave".
- Default if unanswered: A.

Frage 2 (PAGE-GAP): Where does a new page go, and how far away?
- A (default): to the right of the last page, gap 10 % of the larger side, at least 20 mm, and a row wraps after about 8 pages or 4 m (so a long row does not run off).
- B: below the last page.
- C: a fixed gap the maker sets in the Document tab.
- Default if unanswered: A.

## Links

Requirements: R-EDIT-038 (new)
Depends on: `specs/0015-document-size-and-rulers/`, `specs/0050-layers-and-objects-panel/`
Related: `specs/0043-properties-tabs/`, `specs/0045-document-formats-library/`, `specs/0046-document-shapes/` (shape per page, guides), `specs/0049-linked-clone/`, `specs/0057-clone-overrides/`, `specs/0042-history-branches/`, `specs/0024-svg-import-export/`, `specs/0063-manufacturing-workbenches/`; ADR 0012 (rejected, content reused), ADR 0009, ADR 0002
ADRs: [ADR 0024](../../docs/adr/0024-spatial-pages.md); see [adrs.md](adrs.md)
PR: -

**SVG export (2026-11-01):** Pages are written as Inkscape pages; Active page only exports one page (criteria 128 to 130); see `specs/0024-svg-import-export`, "Slice 4: export".
