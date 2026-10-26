# ADR 0018: SVG import and export crate, parser and limits

**Status:** Accepted 2026-10-26 by the architect (a crate and dependency decision, `CLAUDE.md` §3;
the customer can veto it in the story PR of `specs/0024-svg-import-export`). §5 records customer
decisions of 2026-10-16. Supersedes the words "SVG import/export" in the `document-core` row of
[ADR 0011](0011-workspace-and-crate-layout.md) §2 and extends the edge rule of its note of
2026-10-20 (`text-core → (external only)`). Amends
[ADR 0002](0002-document-model-units-and-svg-round-trip.md) §10. Asked by `specs/0024`.

## Context

`0024` adds the first importer: SVG bytes become groups, layers, paths, primitives and linked
clones of the open document (Open, or Import as one history step). The export is a later slice of
the same spec. `0046` stage 2 needs the same reader for template outlines, without the editor.

The proposal of 2026-10-16 (`specs/0024-svg-import-export/adrs.md`) put a new crate on top of
`document-core` and `geometry-core`. Since then `0076` (single-line fonts) merged a hostile-input
SVG reader into `curvyo-text-core`: `roxmltree` 0.20.0 (pinned), a byte scan of the prolog
that refuses any DOCTYPE internal subset, an element depth scan, node limits, and a complete
SVG 1.1 path data parser (`path_data.rs`, 461 lines, workspace `kurbo`, SVG's error rule). The
import must not grow a second copy of any of this. `text-core` does not depend on
`document-core` (ADR 0011, note 2026-10-20), so the shared code cannot live in a crate that does.

What is hard to change later: the crate edges, and the error behaviour of the parsers (golden
files and hostile fixtures pin it).

## Decision

1. **New crate `curvyo-svg-core`: a pure SVG reader with no Curvyo dependency.** Bytes in,
   `SvgDrawing { page, display unit, background, tree }` out, in millimetres, with `kurbo`
   paths, styles resolved per leaf, and clone leaves that name another leaf of the same tree
   by index and carry their matrix. `forbid(unsafe_code)`, builds for `wasm32-unknown-unknown`
   (picked up by the `core-wasm32` CI job by name). Like `text-core` and `crypto-core`, it
   never depends on `document-core`. The later export adds a writer from the same tree type to
   a string; there is no report type.
2. **The mapping to the document lives in `curvyo-ui-core`** (module `svg_import`, later
   `svg_export`): `SvgDrawing` to the snapshot types of `document-core`, primitive detection,
   the even-odd re-winding with `touching_outlines` and `outline_nesting` of `geometry-core`,
   and the `use` to clone resolution. `document-core` gets one write API that inserts such a
   tree in one commit (`insert_drawing`, the `*_write` pattern of `stroke_to_path_write.rs`).
   `0046` stage 2 reads closed outlines from `SvgDrawing` directly and needs neither.
3. **Shared hostile-XML guard and path data move into `svg-core`; `text-core` depends on it.**
   `svg_font.rs`'s `has_internal_subset` and `too_deep`, the `ParsingOptions`, and
   `path_data.rs` move to `svg-core` as `xml_guard` (one function: size, UTF-8, prolog and
   DOCTYPE scan, depth scan, then `roxmltree` with `allow_dtd` and a node limit; the caller
   passes its byte and node limits as plain values) and `path_data`. The DOCTYPE rule is one
   argument with two values, one per caller, because two accepted criteria differ: fonts keep
   "no internal subset at all" (`0076` criterion 23, its fixtures unchanged), the import takes
   the literal-entity rule of `0024` criterion 68 (at most 64 plain literal entities of at most
   256 bytes, at most 100,000 references; anything else refused), which older Illustrator files
   need. Both share the prolog scan that skips decoys in comments and instructions. Depth 256
   is one shared constant. Edge: `text-core → svg-core → (external only)`.
4. **Dependencies.** `roxmltree` =0.20.0 (already in the workspace). New: `svgtypes` =0.16.1
   (transform, colour, length, paint and number-list grammar; Apache-2.0 OR MIT, linebender,
   released 2026-01-09, `forbid(unsafe_code)`, depends on `kurbo` 0.13, the workspace's
   version, and `siphasher` 1.0, already in `Cargo.lock`) and `simplecss` =0.2.2 (the `<style>`
   subset of criterion 36; Apache-2.0 OR MIT, linebender, released 2025-01-06, no `unsafe`,
   depends on `log`, already in `Cargo.lock`). No new crate in the lock file besides these two;
   all licences are in `deny.toml`. Path data is **not** read with `svgtypes`: one path parser
   in the workspace, the one the fonts already use. Both are pinned with `=` because goldens
   depend on their error behaviour. Spike (scratch build, opt-level `s`, LTO): both build for
   wasm32 and add about 89 KB uncompressed, 40 KB gzip; the story PR reports the
   `editor-wasm` delta.
5. **Amendments to ADR 0002 §10 (customer, 2026-10-16).** (a) No passthrough bag: foreign
   attributes (`inkstitch:`, `onclick`) are dropped on import; a bag comes only if SVG becomes
   a native file format, with its own ADR and `format_version`. (b) No loss report: the
   "named, listed loss" becomes "unsupported content is ignored silently"; a file over a limit
   imports partially without a message.
6. **Limits** (criteria 6, 67 to 69). Before parsing: 50 MiB, UTF-8, gzip magic, the DOCTYPE
   rule of point 3, depth 256. While reading: 500,000 elements and 5,000,000 path commands
   (`use` copies count), then the import stops and keeps what it has. Every tree walk is
   iterative or bounded by depth 256. Nothing is fetched, nothing runs.
7. **Determinism** (criterion 72). Numbers are written with Rust's `f64` formatting (`{:.6}`
   for lengths, shortest round trip for `curvyo:` data); no `HashMap` is iterated. Same input,
   same target: same bytes. Across targets documents agree within 1e-9 mm (`sin`, `cos`,
   `atan2`, `tan` are library-defined); if the nightly job shows a difference, the fix is
   `libm` for those four functions, not a looser test.
8. **Text and images are not mapped in `0024`** (criteria 13, 49). When a later story asks:
   `svg-core` emits a text leaf (string after `xml:space`, family list, weight, style, em
   size in mm, anchor point, `text-anchor`, matrix, paint); `ui-core` maps a single-run
   `<text>` to a text node of ADR 0020 (cap-height size = em size × cap height / units per em
   of the resolved face, so the mapping needs `text-core` and the face list, which is why it is
   in `ui-core`), `<textPath>` to a text on a carrier (ADR 0022), and ignores the rest
   (per-glyph `x`/`rotate` lists, mixed-style runs). An `<image>` with a `data:` URI maps to
   an image node of ADR 0017 once `0058` is built. Neither needs a change to this ADR.

## Options considered

- **A. `svg-core` on top of `document-core` and `geometry-core` (proposal of 2026-10-16).**
  Lost: the guard and path parser could not be shared with `text-core` without pulling Loro
  into the font engine, so they would be duplicated; and `0046` would have to build a document
  to read an outline.
- **B. A module in `document-core` (ADR 0011's row).** Lost: the re-winding needs
  `geometry-core`, which depends on `document-core`; every crate would compile the parsers.
- **C. A separate `curvyo-xml-core` for the guard only, `svg-core` above the model.** Lost:
  two new crates where one serves; `path_data` would still need a home.
- **D. Keep the guard in `text-core` and export it.** Lost: the font engine would own the
  SVG import's security boundary; one responsibility per crate.
- **E. `usvg`** (normalises away `sodipodi:`, `inkscape:label` and `use`, resolves text, pulls
  font and image crates), **`quick-xml`** (no tree, no entity handling), **all grammars own**
  (about 800 more lines of solved code) were rejected in the 2026-10-16 proposal and still lose.

## Consequences

- `text-core` gains one internal edge; the move is a refactoring with the existing font tests
  and the `0076` hostile fixtures as the net, done as the first milestone commit of the `0024`
  branch.
- The export slice writes from `SvgDrawing`, so the document-to-SVG mapping is in `ui-core`
  next to the import mapping; criterion 59 (clone to `use`) is reviewed then against `0049`
  and `0057` as built.
- No `format_version` change (`0024` criterion 77; current format 24).
- Risks: the 2.0 s budget of criterion 71 is unproven (first plan task is a benchmark); a
  `svgtypes` or `simplecss` update is a reviewed PR that reruns the goldens and hostile tests.
