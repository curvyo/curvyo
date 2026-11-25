# 0152 Library browser drafts (ux-engineer, 2026-11-25)

Static screens for `specs/0152-library-browser/`; real tokens, built on the kit of
`../2026-11-library/` (`kit.css`, `kit.js`) plus `browser.css` and `browser.js` (the tile, card,
facet, header and detail-bar builders and the extra Lucide paths). Dashed outline = new or changed
control. The notes are `specs/0152-library-browser/ux-notes.md` (proposals U1 to U30, spec deltas
G1 to G13, Frage 374). Fragen 371 to 373 at their defaults A, Frage 374 at A. HTML only in this
pass: the PNG beside each is rendered at the next browser pass.

**Rework after the customer test of 2026-11-25** (UX notes sections 0.1 to 0.7, U31 to U48, H1 to
H14, Frage 376): two views **Items** | **Sets** as the header's first control, the history with
Back at the start of the subject line, a press replaces and Shift+press adds in the facet lists, no
Load, Import page, Load set, Load list or Add button (double-click, Enter or a drag), and Host your
own library…. Drafts 01 to 03 are redrawn for it.

- `01-split-view`: the narrow layout: the Items view with the compact detail bar (no Load), the
  Sets view with loaded and not loaded cards per source and Back, Lucide opened (Items narrowed to
  it, Back to Sets), the Filters band with a replaced and a Shift-added Category, Size 48 with the
  Enter outcome.
- `02-full-view`: the wide layout: the Items view with a template current (Show pages only); the
  Sets view with its facets, groups, a private catalogue's lock and the hosting link; a fuzzy
  search with Best match and Search results; the second level of a document at Size 160 with Back;
  the hosting guide as a place.
- `03-states`: empty library in both views (Show sets; the ready entry card), nothing matches with
  Clear all, every card state (no buttons), the tile states, the Page preview with Back after Pick
  elements…, the Add source form with Host your own library… and the token field of Frage 376 B,
  and Frage 374 B for comparison.
