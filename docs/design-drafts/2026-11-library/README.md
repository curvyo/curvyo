# 0137 / 0138 Library drafts (ux-engineer, 2026-11-24)

Static screens for `specs/0137-document-library/` and `specs/0138-template-library/`; real tokens,
280 px panel body (244 px column) plus the 32 px strip. Dashed outline = new or changed control.
The notes are `specs/0137-document-library/ux-notes.md` (proposals P1 to P25, spec deltas E1 to
E10, P26 to P28 for the browser build) and `specs/0138-template-library/ux-notes.md` (Q1 to Q22,
F1 to F8). HTML only in this pass:
the PNG beside each is rendered at the next browser pass.

- `01-library-tab`: the Library tab after Fonts in the strip; sources (open workspace semibold,
  recent workspaces, added folders); a document expanded to page rows with thumbnails and kind
  chips; the current page; "Saved version"; an armed Remove; read errors; reading pages; search
  with semibold matches; the empty state and the index count; the browser build (Frage 341 C):
  "In this browser" with its workspaces, no footer, its empty and unreadable states.
- `02-page-preview`: the page preview as the canvas area's side pane in Split view with three
  picks and the drag over the drawing (Copy badge); a page row dragged onto the canvas (New page
  badge); the sketch page refusal; the Came from line at the end of the Style tab and the Layers
  row tooltip.
- `03-online-section`: the Online group (0138): the Curvyo catalogue ready entry; template rows
  with licence links and Load; an Iconify icon set and its icon grid; the Add source form with a
  refusal and its "What is sent" line; offline per source and a failed fetch; the Credits block of
  the File tab; Enter in the search sent to the added Iconify sources (Frage 340 B): the Search
  online button, results rows, searching and Offline.

`kit.css` and `kit.js` hold the tokens, the row pieces and the Lucide paths used here. No new
colour token.
