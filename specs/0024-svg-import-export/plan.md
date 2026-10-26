# Plan for SVG import (first slice of 0024)

Implementer, 2026-10-26, branch `story/svg-import`, stacked on `story/stroke-to-path` (not merged;
customer allowed a stack of up to 3). Built under `adrs.md` (dated note 2026-10-26) and
[ADR 0018](../../docs/adr/0018-svg-import-crate-parser-and-limits.md). Import only; the export is
the later slice. No push, no PR in this run.

## Affected crates/modules

- **`curvyo-svg-core` (new, ADR 0018 §1):** a pure SVG reader that depends on no Curvyo crate.
  `xml_guard` (moved from `text-core/svg_font.rs`: size, gzip, UTF-8, prolog/DOCTYPE scan with the
  rule as an argument, depth scan, element cap, then `roxmltree`), `path_data` (moved from
  `text-core`, arcs split by tolerance for fonts or at most 45° for the import), `units`
  (lengths, viewBox and `preserveAspectRatio`, generator pixel sizes), `css` (the `<style>` subset
  on `simplecss`), `style` (cascade per leaf, colours, paints, gradients' middle colour),
  `reader` (tree walk, transforms baked, layers, groups, primitives with their matrix, `use` as
  clone or copy, limits), `drawing` (the output types `SvgDrawing`, `SvgNode`, `SvgLeaf`).
- **`curvyo-text-core`:** `svg_font.rs` and `limits.rs` use `curvyo_svg_core::{xml_guard,
  path_data}`; `path_data.rs` leaves. Font behaviour and fixtures unchanged (`0076` criterion 23).
- **`curvyo-document-core`:** `drawing_insert.rs` (new): the `Drawing` tree type (groups, layers,
  paths with outlines, primitives, clones by leaf index), `Document::insert_drawing` (one commit,
  one new top layer) and `Document::from_drawing` (a new document whose setup change holds the
  page, display unit, background and the content, so history and undo are empty);
  `DocumentVersion::never()` so an opened SVG is dirty.
- **`curvyo-ui-core`:** `svg_import.rs` (new): `SvgDrawing` to `Drawing`: primitives under the
  `0056` rules, node kinds (criterion 25), Inkscape stars (criterion 28), the even-odd re-winding
  (criteria 50, 51) with `outline_nesting`/`touching_outlines`, the placement translation of Import
  (criterion 3, clone matrices T·M·T⁻¹). `svg_import_style.rs` maps styles. `history_names.rs`:
  "Import SVG".
- **`curvyo-editor-wasm`:** `session/svg_import.rs` (Open of SVG as a new dirty session, Import
  SVG… as one step into a new top layer, selected, active), `wasm_svg_import.rs` (bindings
  `open_file`, `import_svg`), the refusal texts of the UX table.
- **`curvyo-app`:** `pick_project` takes the dialog kind (`open`: projects and SVG drawings plus
  All files; `svg`: SVG only; `project`: Template); `save_project_bytes` takes the suggested name.
- **`frontend`:** `useProject` (Open decides by content in the session, SVG opens untitled and
  dirty with the suggested save name, Import SVG… flow, busy notice from 1 MiB), `FileTab`
  (Import SVG… row below the block, outside the inline check), `host.ts` (browser picker accepts),
  `lib/svgImportText.ts` (notice and name cut).
- **Docs:** `docs/design-system.md` rows (File tab, notice, busy, refusal titles),
  `docs/svg-import.md` (the supported subset, criterion 76), `docs/technical-debt.md` "SVG
  round-trip is lossy", `specs/0054` criterion 27 note (EVENODD A), `deny.toml` if needed.

**Bytes across the Tauri boundary:** as the project open does today (`pick_project` returns
`bytes: Vec<u8>` in the JSON answer; the page passes a `Uint8Array` to the session). The 50 MiB
limit is checked in the session before the parse (criterion 6c); the host reads the file whole.

## Tasks

- [x] 1. Docs commit: spec Ready, UX amendments, ADR 0018, index, ADR 0011 note.
- [x] 2. Budget probe: an `#[ignore]` benchmark of `insert_drawing` over 20,000 paths with 400,000
  nodes in one commit (risk of `adrs.md`; AC 71). Restate the budget from the measurement if missed.
- [x] 3. New crate `curvyo-svg-core`; move `xml_guard` and `path_data` from `text-core` (font tests
  and `0076` hostile fixtures as the net); the literal-entity DOCTYPE rule and the element cap
  (AC 6, 68, 69).
- [x] 4. Reader, tests first: page and units (AC 15 to 20), path grammar and arcs (AC 21 to 24, 30),
  shapes (AC 26, 27, 28 data), transforms (AC 31 to 35), styles and CSS (AC 36 to 43), structure,
  layers, `use` (AC 44 to 49), ignored content (AC 13), limits (AC 69).
- [x] 5. Hostile input: refusals per case (AC 6), entity rules (AC 68), truncation and byte-change
  sweep of every fixture and a proptest over random bytes (AC 70 a), nothing fetched (AC 67).
- [x] 6. `document-core`: `Drawing`, `insert_drawing` one commit, `from_drawing` with empty history
  (AC 2, 3, 77).
- [x] 7. `ui-core::svg_import`: primitives and transforms (AC 26, 27, 33), node kinds (AC 25), star
  match (AC 28), even-odd (AC 50, 51), placement (AC 3), styles (AC 34, 38 to 42); goldens in
  `tests/fixtures/svg/` with `README.md` (AC 74, 75), determinism (AC 72), the 0046 call (AC 73).
- [x] 8. Session and bindings: Open by content (AC 1, 2), Import SVG… (AC 3), the notice (AC 8),
  refusals (AC 6), busy (AC 7), history name.
- [x] 9. Host and frontend: dialogs and filters, File tab row (AC 5), save name after Open (AC 2).
- [x] 10. Docs (AC 76), full gate, browser check with a screenshot.

## Validation

- Golden files: every fixture's imported `Drawing` as JSON, compared within 1e-9 mm (AC 72, 74).
  Inkscape-made and Illustrator samples are written by hand in the layout those programs write
  (no Inkscape on the build VM); the README says so per file. The tester replaces them with files
  saved by Inkscape 1.x when available (AC 75).
- Hostile fixtures and the truncation/byte-change sweep (AC 70 a); a proptest over arbitrary bytes.
  The 60 s nightly fuzz target (AC 70 b) is not built here: the sweep plus the proptest stand in,
  noted as an open point.
- `#[ignore]` budget tests (AC 71) run once in release; numbers in the report.
- Browser check: Import SVG… of a sample file in the browser build, screenshot.

## Notes for the lead (2026-10-26)

- **Permit over refuse, two readings.** A root `<svg>` without `xmlns` is read as SVG (criterion
  6 (a) refuses it as written); `<switch>` takes its first child that needs no
  `requiredExtensions`, so older Illustrator files (whose first child is a `foreignObject` with
  `requiredExtensions`) import (criterion 44 says "the first child element").
- **Budget 71** restated from the measurement (`adrs.md`, dated note 2026-10-26).
- **Not built here:** the 60 s nightly fuzz target of criterion 70 (b) (the truncation and
  byte-change sweep and two proptests stand in); the WASM timings and the `editor-wasm` size delta
  for the PR description; Inkscape-saved fixtures (the files are hand-written in Inkscape's
  layout, `tests/fixtures/svg/README.md`).
- **Existing tests touched:** the button lists of `frontend/tests/fileTab.test.ts` and
  `acceptance_0051_tester.test.ts` gain "Import SVG…" (criterion 5), as the Template button's
  commit did.
