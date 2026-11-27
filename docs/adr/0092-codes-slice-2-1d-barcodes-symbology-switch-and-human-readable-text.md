# ADR 0092: Codes slice 2: 1D barcodes, the Symbology switch, glyph tables for the human-readable text, free resize and the code's host contract

**Status:** Proposed 2026-11-27 (architect). Extends [ADR 0091](0091-qr-and-barcode-codes-extension.md)
§2 (switching symbology in place), §3 (contract additions) and §9 (1D encoders, human-readable text),
whose decision text stays as accepted; amends `0141` UX notes §8 and ADR 0080 §6 ("generated objects
resize proportionally only") for kinds that declare free resize. **§3.6 (embedding a code's font) is
`needs-customer`** (document model, persistence): CODE-FONT-EMBED, default B (not embedded in slice
2). Everything else adds kinds and contract items through the route of ADR 0034 and ADR 0080 and
changes no file format. Spec: [`0144`](../../specs/0144-qr-and-barcodes/specification.md) slice 2
(criterion 11, Frage 323 A); UX notes revision 2, sections 11 and 13.

**In plain words (for the customer).** Barcodes are five more code types next to the QR code, and one
"Symbology" row switches a code between them in place, keeping what you typed. The digits under an
EAN code are real letter outlines from a font you pick; Curvyo hands the generator the letter shapes,
so the code comes out the same on every computer. A barcode can be stretched wider or taller from its
edges. Whether the file should also carry the barcode's font (as it does for a text) is your call
(CODE-FONT-EMBED).

## Context

Read against `story/barcodes` at `e0a47fa14`: `curvyo-extension-core/src/extension.rs`
(`GenerateRequest { kind, params, seed, edit }`, no font input; `Edit::{Param, Scale, Draft}`;
`Warning` has no row), `manifest.rs` (one bar segment per `GeneratorKind`), `curvyo-ui-core/src/
instance.rs` (`generate` is the one funnel; `plan_resize` takes the geometric mean),
`transform_handle_layout.rs` (`corners_only` for every generated selection), `curvyo-document-core/
src/font_assets.rs` (`used_fonts` walks text nodes only; `prune_assets` keeps the hashes a text or an
image names), `curvyo-editor-wasm/src/session/text_fonts.rs` (faces load on demand: `NeedsLoad`,
`wanted`). Host QR knowledge: the technical-debt entry "QR knowledge in the host".

**Spike (scratch crate, not committed, 2026-11-27).** `rxing 0.9.4` with `default-features = false,
features = ["encoding_rs", "decoders", "oned"]` decodes Code 128 "Curvyo", Code 39 "CURVYO", EAN-13
`4006381333931`, EAN-8 `96385074` and UPC-A `036000291452` from a plain luma buffer (no `image`
feature). Run lists rasterised with coverage at 10 px per module read at bar width adjust -50 % to
+50 % and at Code 39 wide:narrow 2.0, 2.5 and 3.0; at 3 to 4 px per module the decoder fails on grey
edges at +-30 %, so the test stays at 10 px, and the decoder is a weak guard for the adjust (EAN and
Code 128 decode edge to similar edge). `cargo deny check` with this repo's `deny.toml` passes
advisories, bans, licences and sources. New lockfile crates, dev only: about `rxing`,
`rxing-one-d-proc-derive`, `codepage-437`, `core_detect`, `encoding_rs`, `multiversion_no_op`,
`simdutf8` and `syn` 3 (a second major); the PR states the actual lock diff. Source 5.1 MB, release
build of the tree 10 s. rxing's writers are not used: their margins and their 2:1 Code 39 differ from
the spec's module counts, so goldens come from the standards.

### Options: glyph outlines into a pure generator (the human-readable text)

- **A. Host-composed text.** The generator returns text anchors, the host appends glyphs. Two owners
  of one outline; codes-core goldens miss the text; every writer must remember the step. Rejected.
- **B. A `Lettering` trait in extension-core**, called per glyph while generating. One product
  implementation plus a double, a borrowed `dyn` in a `Copy` request, host calls inside a pure
  function. Rejected for the plain data of E.
- **C. codes-core depends on text-core and receives face bytes.** The contract would carry text-core
  types and every generator crate the font engine. Rejected.
- **D. A built-in digits font in codes-core.** Pure, but criterion 11 picks the maker's fonts and
  Code 128 prints any ASCII. Rejected; an OCR-B face (GS1's recommendation) may come later as an
  included font, its licence to be checked (a customer licence question).
- **E. A glyph table in the request (chosen).** The host builds plain data with text-core; the
  generator places the glyphs itself (EAN digits sit per 7-module character, which only it knows).

### Options: the 1D resize (UX notes §11.10: eight handles, opposite edge fixed, corner free)

- **A. Parameter knobs** for Width and Bar height. No contract change, but centre-anchored knobs are
  not the edge handles of any path the UX notes ask for, and the Size chip's W and H stay coupled.
  Rejected.
- **B. A free-resize declaration and a two-factor edit (chosen).** Per kind, so QR, boxes and gears
  keep the proportional rule.

### Options: what crosses a symbology switch

- **A. The host rule of UX notes §11.4 (chosen):** equal key, and an untouched default follows the
  symbology. Generic, the generator is not involved, no new `Edit`.
- **B. `Edit::Switch { from }` and the generator decides.** Moves a schema rule into every generator.
  Rejected.

## Decision

1. **Encoders are ours, the decoder is `rxing` (dev only).** ADR 0091 §9 holds: `linear/mod.rs` (the
   run list: widths in modules, first a bar, plus the human-readable characters and their positions),
   `linear/code128.rs` (sets A, B, C with the shortest switching, mod-103 check; ISO/IEC 15417),
   `linear/code39.rs` (ISO/IEC 16388, `*` start and stop, no check character), `linear/ean.rs`
   (EAN-13, EAN-8, UPC-A as EAN-13 with a leading 0; GS1 General Specifications), `linear/style.rs`
   (runs, bar width adjust and glyphs to closed paths in part `code`). No new product dependency.
   Goldens: criterion 3's counts (EAN-13 95, Code 128 "Curvyo" 101, Code 39 "CURVYO" 127 at 3:1) and
   the standards' pattern tables; SVG fixtures `ean13.svg`, `code128.svg` as ADR 0091 §11.
   `rxing = { version = "0.9", default-features = false, features = ["encoding_rs", "decoders",
   "oned"] }` is a dev-dependency beside `rqrr` (slice 3 adds `datamatrix`); never in `cargo tree -e
   normal`. CI builds `*-core` tests natively only, so no target gate.
2. **Kinds, one family, the switch.**
   - Kinds `curvyo.codes/code-128`, `code-39`, `ean-13`, `ean-8`, `upc-a` (labels "Code 128",
     "Code 39", "EAN-13", "EAN-8", "UPC-A"); `qr` unchanged.
   - `GeneratorKind.family: Option<Family>`, `Family { id: "code", segment: "Code", label:
     "Symbology" }`: the Generators bar shows one segment per family (it places the session's last
     kind of the family, first the first in manifest order), the Properties block draws the
     Symbology group first, its options the family's kinds in manifest order (UX notes §11.2, §11.4).
   - `instance::switch_kind(document, settings, node, target)`, one step (label of UX notes §11.4).
     For each target parameter: a stored value of equal key is kept, clamped by the target schema
     (house clamp form), **unless it equals the old kind's default, then it takes the target's
     default**; keys the target lacks (the QR styles) are dropped, Undo restores them **(Proposal)**.
     A derived value (`width`) that crosses is recomputed, since a plain generation lets the stored
     truth win (ADR 0091 §4). Then a plain generation through `record_keeping`, so content the target cannot encode keeps the
     stored outline in the No solution look under the new kind (ADR 0091 §3). Same node id, frame
     origin and part key `code`, so operations on `#code` survive.
   - Keys (file format, ADR 0034 §3): shared by every code `content`, `inverted`; QR keeps its
     shipped `module_size` and `quiet_zone`; the 1D kinds share `module_width`, `bar_height`, `width`
     (derived, with the quiet zones), `text`, `font`, `text_size`, `bar_adjust`, and Code 128 and
     Code 39 `bar_quiet_zone` (a quiet zone in X dimensions, not in QR modules, so a QR's changed
     quiet zone does not cross, UX notes §11.4); Code 39 alone `wide_narrow`. EAN and UPC have no
     quiet zone row (fixed by the standard).
   - `ParamKind::Ratio { min, max, step, decimals }`, `ParamValue::Ratio(f64)`, stored `{"ratio": x}`
     (Wide:narrow 2.0 to 3.0, a unitless number shown with ": 1"). `ParamKind::Percent` bounds and
     `ParamValue::Percent` become `i32` (Bar width adjust -30 to +30; stored `{"pct": n}` unchanged).
3. **Human-readable text by glyph table.**
   1. `ParamKind::Font`, `ParamValue::Font(Option<FontRef>)`, stored `{"font": null}` or `{"font":
      {"family", "style", "face_hash", "face_index"}}` (a text's `FontRef`, outline kind); `None` is
      the included sans (Source Sans 3 Regular), so no font hash is a constant in a core crate.
      Outline faces only, since a `Part` holds closed paths: a single-line pick is refused with
      `0076`'s sentence (UX notes §11.9).
   2. `GenerateRequest.glyphs: Option<&GlyphTable>`, `GlyphTable { glyphs: BTreeMap<char, Glyph> }`,
      `Glyph { paths: Vec<ClosedPath>, advance: Length }` at a font size of 1 mm, baseline at y = 0,
      y down as the frame, outers wound as `trace.rs` outers and holes opposite (so NonZero and
      Inverted hold). It covers U+0020 to U+007E, everything Code 128 sets A and B, Code 39 and
      EAN/UPC print; no shaping or kerning. A character the face lacks is absent: the generator
      leaves its cell empty and warns `glyph_missing` (row `font`).
   3. ui-core builds the table (`code_glyphs.rs`: text-core outlines per character, quadratics
      raised to cubics exactly, winding normalised by nesting since TrueType and CFF wind
      oppositely); the session caches one per `(face hash, index)`; `instance::generate` gains the
      session's faces as one input and passes the table to a kind with a `Font` parameter. Every
      writer goes through that funnel. The preview memo (ADR 0080 §6) and the No solution memo
      (ADR 0091 notes) key on the `(face hash, index)` used as well as `(source, values)`.
   4. Resolution as a text's (ADR 0047 §4): exact bytes, then same name, else the included sans with
      the missing-face look and sentence of UX notes §11.9; a fallback never rewrites the stored
      `FontRef`, and stored paths stay as drawn until an edit. A pick writes `Font` once the face is
      loaded (as the Text bar's). **A loaded face is a precondition, never a refusal** (ADR 0091 §3's
      refusal stays "content cannot be encoded"): the session asks for the face of a selected code
      and for the included sans when the bar would place a kind with text on; until it arrives that
      code's rows, handles and Symbology group, and that Code segment, are inert with "Loading
      font…" and nothing is written.
   5. Determinism: the table is a function of the face bytes, so equal params and face give equal
      outlines native and wasm (criterion 1's wasm test adds an EAN-13 with text). The text lies in
      part `code` (one compound path, criterion 15). codes-core goldens use a test table
      (`tests/support/glyphs.rs`, box glyphs with a hole); a ui-core test covers the builder on an
      included face: winding, the counter of "0" turning to ink when Inverted, equal output twice.
   6. **Embedding the font (`needs-customer`, CODE-FONT-EMBED).** `used_fonts` sees texts only, so a
      code's font is neither embedded nor kept by `prune_assets`. **A:** document-core also reads
      `{"font": {...}}` values in generated params (one stored value shape stops being opaque, ADR
      0080 §4; no `format_version`; an older build drops the asset on its next save, ADR 0047 §1).
      **B:** not embedded in slice 2: the stored outline always draws; on a computer without the
      font an edit sets the text in the included sans with the notice; the default font is included
      everywhere. **C:** a `fonts` field in the generated record (`format_version` bump).
      Recommended A, default B (no model change; A can follow and embeds on the next save).
4. **Free resize for 1D kinds.** `GeneratorKind.resize: Resize::{Proportional, Free}` (every existing
   kind and QR `Proportional`) and `Edit::Stretch { x: ScaleFactor, y: ScaleFactor }`, the factors of
   the object's box along the frame's axes, sent only to `Free` kinds.
   - A selection of `Free` generated objects only, all with parallel frames, shows the eight resize
     handles and no skew handle; any `Proportional` one in it, or frames turned against each other
     (an edge drag would skew one), keeps `corners_only`, and every object gets `Edit::Scale`.
   - `plan_resize` reads the two factors from `generated_map` in frame-local axes; a map with skew or
     a turn in frame space is refused. The generator owns the mapping (UX notes §11.10): x writes
     `module_width` and `text_size`, y writes `bar_height` = new box height minus the text band;
     `Edit::Scale` scales all three. The host then places the frame so the regenerated box keeps the
     edge or corner opposite the handle (a clamp stops the dragged edge, never moves the fixed one).
     The Size chip's W and H go the same way. The origin stays at the bars' centre (UX notes §11.11).
5. **The host forgets QR (resolves the technical-debt entry).**
   - `Warning.param: Option<&'static str>`: the row the sentence stands under (replaces the
     `"content"` test in `instance_view`).
   - `Warning.subject: Option<&'static str>`: the words the generator names the cause with, as
     `Bound::DependentFor` ("at error correction M"), so `instance_warnings` stops reading `"ec"`; the
     kind's label ("EAN-13") is the host's. New ids: `wrong_length`, `digits_only`, `not_encodable`
     (the symbology's character set, never a character of the content, criterion 13),
     `check_digit_corrected` (no digit, UX notes §13.4), `glyph_missing`.
   - `GeneratorKind.scanned: bool`: a scanner reads the outline; it drives the contrast check
     instead of the `curvyo.codes/` prefix.
   - `instance.rs` split by arm and `session/generators.rs` brought under the limit as the first,
     behaviour-preserving commits on `story/barcodes` (no PR of their own). The gear's `GEAR`
     constant in `instance_resize.rs` stays (a generators matter, outside this entry).
6. **No `format_version` change.** Kinds are additive (a slice-1 build opens them as unknown kinds
   with the stored outline and ADR 0080 §5's notice); params stay opaque; `family`, `resize`,
   `scanned`, `Edit::Stretch` and the glyph table are run time. Only CODE-FONT-EMBED C would bump it.
7. **Contract changelog** (`curvyo-extension-core/CHANGELOG.md`, 0.x): `ParamKind::Font` and `Ratio`,
   `ParamValue::Font` and `Ratio`, `Percent` as `i32`, `GenerateRequest.glyphs`, `GlyphTable`,
   `Glyph`, `Edit::Stretch`, `GeneratorKind.family`, `resize`, `scanned`, `Family`, `Resize`,
   `Warning.param` and `subject`. Source-breaking for exhaustive matches and struct literals.

## Consequences

- One dev-dependency tree (rxing, about eight crates, a second `syn` major), no product dependency.
- The contract grows a font parameter and a data input any later generator with lettering (a name
  plate, a ruler) can use; the host builds one table per face, not per edit.
- Generated objects are no longer proportional-only everywhere: the handle layout and `plan_resize`
  branch on the kind's `resize`; QR, boxes and gears are unchanged.
- A code's font follows texts only once CODE-FONT-EMBED A is taken; until then a non-included font
  is a per-computer choice, visible only after an edit on another computer.
- The decoder reads the whole bar width adjust range, so the range is a maker's setting, not a
  scannability proof; a phone check stays part of the demo.
- Spec changes for the PO (Proposals): criterion 11's quiet zones per symbology (check against the
  GS1 General Specifications: EAN-13 11 and 7, EAN-8 7 and 7, UPC-A 9 and 9; Code 128 and 39 10),
  the EAN/UPC guard bars reach into the text band, the font is an outline face (default the
  included sans); criterion 3: "Check digit corrected." without the digit; criterion 14: 1D edges
  and corners resize freely, QR stays proportional; criterion 16: slice 2 changes no
  `format_version`.

## Customer question

**CODE-FONT-EMBED: should a barcode's font travel in the file like a text's?** A: yes, the file
carries the font of a code's digits as it does for a text (the document reads one more place). B:
not in this slice; the code always looks as saved, and only an edit on a computer without that font
redraws the digits in the included font. C: as A, with a file-format change. Recommendation: A.
Default: B.
