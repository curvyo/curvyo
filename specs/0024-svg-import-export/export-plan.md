# Plan for SVG export (slice 4 of 0024)

Status: Accepted by the customer (2026-11-01), in progress on `story/0024-svg-export`.

**Customer decisions (2026-11-01):** SVG-PAGES A, with an "Active page only" option; SVG-TEXT A; SVG-MARKERS A; SVG-GUIDES B; SVG-OPTIONS A. **One PR** for the whole export (the customer prefers one larger PR over slices 4a and 4b): section 5 is built as one slice.

The export is specified in `specification.md`, section "Later: export" (criteria 4, 52 to 66, 71 and 72 export parts, written 2026-10-14 to 2026-10-16). Since then the document model has grown: pages, text, images, sketches, guides, page grids, arcs. This plan lists what the old criteria cover, what is missing, how the work is cut and what the customer has to decide first.

## 1. What exists

- **Reader:** `curvyo-svg-core` (ADR 0018) reads SVG into `SvgDrawing` (mm, `kurbo` paths, resolved styles, clones by index) and depends on no Curvyo crate. `curvyo-document-core` applies a drawing as one commit. `editor-wasm` and `frontend` have Open of `.svg` and Import SVG…, `storage-io`/`app` have the file dialogs. 127 import criteria, golden fixtures in `curvyo-svg-core/tests/fixtures/svg/`.
- **Writer:** none. No `export` function, no `*.exported.svg` goldens, no Export SVG… button. `curvyo_all.svg` (criterion 75 item 12) is not pinned.
- **Building blocks that the export can reuse:** text to outlines (`curvyo-ui-core/src/text_convert.rs`, needs the session's loaded faces), stroke to outline (`0085` stroke-to-path), resolved clone outlines (`0049`/`0057`), the per-page frames (`0059`), `Document::export_json` (determinism patterns), the import's number and unit rules.
- **R-SYS-006** ("move to and from Inkscape without hand-fixing geometry") is met only once the export is built.

## 2. What the old criteria already cover

Document root and units (52), layers, groups, names, hidden, locked, opacity, ids (53), paths and compound paths in absolute mm with node types (54, 58), rectangles, ellipses, polygons and stars with Inkscape star data (55), presentation-attribute styles (56), `curvyo:` shadow data for exact values (57), clones as `<use>` where they are an isometry without override, else a path (59), no personal data and byte-stable output (60, 72), optional namespaces (61), round trips of our own data, of Inkscape files and of hand edits (62 to 66), a budget (71: 1.5 s, 25 MB for 20,000 paths), the button, notices, busy state and default name (4, UX notes).

## 3. What is missing (model features added after 2026-10-16)

| Model feature | Gap | Proposal / question |
|---|---|---|
| Multiple pages (`0059`) | Criterion 52 knows one page (`width`, `height`, `viewBox`). | Frage 12 (SVG-PAGES). |
| Text objects (`0074`), text on path (`0080`), linked text clones, single-line fonts (`0076`) | Not in the export at all. Single-line (stick) fonts cannot be live text in another program. | Frage 13 (SVG-TEXT). Single-line texts always as open paths (Proposal). |
| Images (`0058`) | Not in the export. | `<image>` with the original bytes as a `data:` URI (PNG/JPEG/WebP as stored), placed by its frame matrix, `preserveAspectRatio="none"`; matches the import of slice 2 (Proposal). |
| Sketches (`0066`) | Not in the export; constraints have no SVG form. | Written as their solved geometry (paths/primitives) in a `<g inkscape:label>`; constraints and dimension symbols are not written; import gives plain objects (Proposal). |
| Ellipse arcs (`0021`), curved polygon/star sides | Criterion 55 knows full ellipses and straight-sided stars. | Arcs as `sodipodi:type="arc"` (`start`, `end`, `arc-type`) so the import of slice 3 reads them back; curved-side polygons/stars as `<path>` with `curvyo:kind`/`curvyo:frame` shadow data (Proposal). |
| Stroke markers (`0018`) | Criterion 59: only `curvyo:markers`, other programs draw no arrows. | Frage 14 (SVG-MARKERS). |
| Guides (`0046`) | Guides are "not output"; criterion 53 would write them as normal objects. | Frage 15 (SVG-GUIDES). |
| Page grid and origin corner (`0090`), snapping (`0060`) | View aids and settings. | Not exported (Proposal); SVG coordinates stay top-left, y down, whatever the origin corner. |
| Mirror / negative scale (`0056`), clone overrides (`0057`) | Covered by criterion 59 (flip is an isometry; override → path). | Nothing to add. |
| Export options (convert strokes, text to paths, selection only, visible only) | Recorded in "Later: export" without criteria or UI place. | Frage 16 (SVG-OPTIONS). |
| Desktop vs browser | Criterion 4 covers both (save dialog / download). | Reuse the import's dialog plumbing in `storage-io`/`app`. |

## 4. Architecture (for the architect to confirm in `adrs.md`)

- `curvyo-svg-core` gets `export(&SvgExport) -> String`: a pure writer. `SvgExport` is the writer's own input model (pages, layer/group tree, leaves with geometry in mm, resolved style, shadow values, clone references by index, images as bytes), mirroring `SvgDrawing`, so the crate keeps depending on no Curvyo crate (ADR 0018 boundary). No new dependency: the XML is written by hand with one escaping function.
- `curvyo-document-core` (or `curvyo-ui-core` where faces are needed) builds `SvgExport` from one document read: clone test of criterion 59, effective styles, page frames. Text outlines come from the session (faces are loaded there), so the text part is built in `curvyo-editor-wasm`.
- `curvyo-editor-wasm`: `export_svg(options) -> Vec<u8>`; `frontend`: Export SVG… in the File tab, save dialog / download, notices, busy state.
- Golden files: `name.exported.svg` beside each import fixture, `curvyo_all.svg` from the generated document, round-trip property test over 500 random documents (criteria 62, 63).
- No `format_version` change (the `.curvyo` format is untouched).

## 5. Slices

The export is one customer-visible feature; it is cut so that each PR can be tried end to end.

**Slice 4a: drawings (one PR).** Criteria 4, 52 to 61 with the gaps above for arcs, curved sides, images, sketches, guides and pages (per Frage 12, 14, 15); round trips 62 to 64 and 66; determinism 72; budget 71. Text objects are written as outlines in 4a when their faces are loaded, else skipped with the count in the notice (Proposal; live text follows in 4b). Hand check 65 with the current Inkscape by the tester.

**Slice 4b: text and options (one PR).** Live `<text>`/`<textPath>` per Frage 13, linked text clones, the options of Frage 16 and their UI place, round trips with text fixtures.

Tasks per slice follow the usual order: spec amendment with criteria numbers (PO), `adrs.md` amendment (architect), UX notes for the button, notices and options (UX), then build test-first: writer and number formatting in `curvyo-svg-core`, model mapping, wasm command, frontend, goldens, round trips, budget; tester pass (format-adjacent: full pass), UX review, full gate, PR.

## 6. Questions for the customer (each has a default)

Frage 12 (SVG-PAGES): A document with several pages is exported how?
- A: one SVG with all pages as Inkscape pages (Inkscape 1.2+ shows them as pages; other programs see one big canvas).
- B: only the active page, coordinates from its corner; objects outside it are left out.
- C: one SVG file per page.
- Recommendation and default: A, with "Active page only" as an option in slice 4b.

Frage 13 (SVG-TEXT): How is text written?
- A: live `<text>` with font name, size and lines (editable in Inkscape, needs the font installed there); a "Text as paths" option writes outlines.
- B: always outlines.
- Recommendation and default: A, option default off; single-line fonts always as open paths.

Frage 14 (SVG-MARKERS): Arrows and dots on strokes (`0018`):
- A: real SVG `<marker>` definitions, so every program draws them, plus `curvyo:markers` for the round trip.
- B: only `curvyo:markers` (as specified in 2026-10: other programs draw no arrows).
- C: expanded to plain paths.
- Recommendation and default: A.

Frage 15 (SVG-GUIDES): Guide paths (`0046`):
- A: not exported (they are never output).
- B: exported in a hidden, locked layer "Guides" so they come back on import.
- Recommendation and default: A.

Frage 16 (SVG-OPTIONS): Where do the export options live, and which ones now?
- A: an inline row of switches under Export SVG… in the File tab (no dialog), remembered per user: Text as paths, Strokes as paths, Selection only, Active page only.
- B: the same switches, stored in the document.
- Recommendation and default: A, all four in slice 4b.
