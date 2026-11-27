# ADR 0092: Codes slice 2: 1D barcodes, the Symbology switch, glyph tables for the human-readable text, free resize and the code's host contract

**Status:** Accepted 2026-11-27 (architect, review of `story/barcodes` at `ee446ba6a`) with the deviations recorded under "Architect review" at the end (registry instead of an argument, cap-height glyph unit, no Code 128 set A, plus three corrections of this text). Extends [ADR 0091](0091-qr-and-barcode-codes-extension.md)
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

**Answered A by the customer, 2026-11-27** (see "Frage 407 answered" at the end). **CODE-FONT-EMBED: should a barcode's font travel in the file like a text's?** A: yes, the file
carries the font of a code's digits as it does for a text (the document reads one more place). B:
not in this slice; the code always looks as saved, and only an edit on a computer without that font
redraws the digits in the included font. C: as A, with a file-format change. Recommendation: A.
Default: B.

## Implementation notes (2026-11-27, slice 2 built)

The decisions above held. What the build added or did differently, all **(Proposal)**:

- **The glyph input is a registry the session sets, not a signature.** `instance::generate` stays the one
  funnel and reads the tables from a per-thread `GlyphRegistry` (`curvyo-ui-core/src/code_glyphs.rs`) that the
  session publishes before every write, as it already does for the active extension settings
  (`set_active_extension_settings`). It avoids a new argument on the 70 callers; the request still carries a plain
  `GlyphTable`, so the generators stay pure. A face missing on this computer maps to the table of the included
  sans, the stored `FontRef` is never rewritten.
- **One glyph unit is the face's cap height** (a text's Size is a cap height, `0074`), not an em: a code's text
  Size means what the Text bar's Size means. `glyph_table` raises nothing (the engine hands cubics) and
  normalises the winding by nesting.
- **Code 128 uses sets B and C only.** Spec criterion 22 allows printable ASCII, which set B holds entirely, so
  set A (control characters) is never reached; the shortest switching is a small search over B and C with a tie
  staying in the current set (`EXAMPLE-01` stays 145 modules).
- **Warning ids and words.** Besides the ids of §5 there is `too_many_characters` ("Too long for Code 128: 92
  characters, at most 80."); `not_encodable` carries the accepted characters in `Warning.subject`, the host
  words "{label} takes {subject} only."; a `Warning` that names a row is the lasting line only while the
  generator answers no parts, otherwise an ordinary notice (`check_digit_corrected`, `glyph_missing`).
  `noun()` keeps a label with a digit ("Code 128") like an acronym.
- **EAN and UPC digit places** (GS1 layout, GS1 General Specifications): leading digit centred 4.5 modules left
  of the first bar, UPC-A's trailing digit 4.5 modules right of the last; the digits stand under their 7-module
  characters. **UPC-A's quiet zones of 9 and 9 modules are from memory and not verified against the document**
  (the tester or the architect checks before the demo; a deviation changes the two numbers in
  `linear/ean.rs` only). Only the three guard patterns reach into the text band.
- **An inverted 1D code's ground** reaches 2 modules above and below what it holds, and the quiet zones to the
  left and right (`GROUND_PAD`).
- **Free resize.** `plan_resize` for a `Free` kind reads the factors in frame-local axes; one factor along both
  is `Edit::Scale`, two are `Edit::Stretch`; the frame is shifted so the point of the old box the drag left
  fixed keeps its relative place in the regenerated box (edge: 0 or 1, centre 0.5), and an axis the drag did not
  touch is not shifted, so the bars' centre stays. The live drag draws the regenerated outline with the readout
  of UX notes §11.10 ("Module width 0.40 mm", "Bar height 22.0 mm", the box for a corner).
- **Dependencies.** `rxing` 0.9.4 dev-only (`default-features = false`, `encoding_rs`, `decoders`, `oned`): the
  lockfile gains `rxing`, `rxing-one-d-proc-derive`, `codepage-437`, `core_detect`, `csv`, `csv-core`,
  `encoding_rs`, `multiversion_no_op`, `ryu`, `simdutf8`; `cargo deny check` passes with the existing
  `deny.toml`, `cargo tree -e normal` shows none of them.
- **CODE-FONT-EMBED stays at its default B**: nothing in `curvyo-document-core` changed, no `format_version`
  change (the nested `{"font": {...}}` value is stored by the existing params codec).

## Architect review (2026-11-27, `story/barcodes` at `ee446ba6a`)

The decisions hold; accepted with these deviations and corrections, all **(Proposal)**:

1. **Glyph registry (deviation, accepted).** A per-thread `GlyphRegistry` the session publishes in
   `recording_unguarded`, `generated_panel_json`, placing and switching, as `set_active_extension_settings`
   does. Deterministic: a table is a function of the face bytes (`BTreeMap`, only `+ - * /`, no
   transcendental float operations), so equal params and face give equal outlines native and wasm; the
   registry's `HashMap` is only looked up. Every write that regenerates a code has the code in
   `selected_generated()` (a group's selection holds its leaves) or forces the publish, and the
   Loading font gate covers rows, handles, the size entry, the switch and the Fonts pick (which waits for
   the face like the Text bar's). Undo, redo and sync apply stored records and never regenerate. The
   remaining weakness: `glyphs_for` answers the included sans for any face it does not hold, and a `Font`
   kind with no table at all draws without text, so a future writer that skips the publish stores wrong
   geometry silently. Follow-up (technical debt "A code's glyph funnel does not fail closed").
2. **Glyph unit is the cap height (deviation, accepted):** a code's text Size means the Text bar's Size.
3. **Code 128 sets B and C only (deviation, accepted):** criterion 22 allows printable ASCII only.
4. **Correction of §3.3:** the No solution memo keys on the params only and the ghost memo on the kind;
   both are right because a refusal depends on the content alone (an absent glyph is a notice, not a
   refusal) and the ghost has no text.
5. **Correction of §3.5:** no wasm test of an EAN-13 with text was added; native-versus-wasm equality
   rests on the argument in 1 (technical debt "No native-versus-wasm golden of the codes").
6. **Correction of Consequences:** the lock gains ten crates (`rxing`, `rxing-one-d-proc-derive`,
   `codepage-437`, `core_detect`, `csv`, `csv-core`, `encoding_rs`, `multiversion_no_op`, `ryu`,
   `simdutf8`), dev only (`cargo tree -e normal` has none, `cargo deny check` passes), no second `syn`.
7. **§5 is resolved in part:** `instance_resize::free_view` reads `module_width` and `bar_height` for the
   live readout and `instance_switch` treats `width` as derived, so the host still knows 1D keys
   (technical debt "1D code keys in the host").
8. **Quiet zones** (EAN-13 11 and 7, EAN-8 7 and 7, UPC-A 9 and 9 X; Code 128 and Code 39 10 X) agree with
   the GS1 General Specifications, ISO/IEC 15417 and ISO/IEC 16388 from the architect's knowledge, not
   checked against the documents: high confidence for EAN, UPC and Code 128, medium-high for Code 39
   (10 X is the ISO minimum, some application standards ask for more). Not in the build: GS1 extends the
   bars of UPC-A's first and last symbol characters (the number system and check digits, printed outside)
   to guard length; `linear/ean.rs` extends the three guard patterns only.

## Implementation notes, slice 3 (2026-11-27, Data Matrix, PR #523)

No new ADR; ADR 0091 §9 already chose the crate. All **(Proposal)**, vetoable at the demo.

1. **Encoder `datamatrix` 0.3.3** (Apache-2.0 OR MIT, `no_std`, optimizing encoder; `arrayvec` MIT OR
   Apache-2.0, `flagset` Apache-2.0; `cargo deny check` passes). Only `datamatrix/encode.rs` names it;
   `SymbolList::default().enforce_square()` picks the smallest of the 24 squares, 10 x 10 to 144 x 144,
   and the crate picks the encodation modes. Text that is not Latin-1 gets an initial UTF-8 ECI (some
   readers ignore it: a Latin-1 text is stored without one). The limit a refusal names is 1555 bytes,
   the most 144 x 144 holds of plain characters (measured; digits pack to 3116). `rxing` gets its `datamatrix` feature as a dev-dependency (the
   decoder of the round trip test, criterion 4); `cargo tree -e normal` shows no `rxing`.
2. **Kind `curvyo.codes/data-matrix`**, label "Data Matrix", icon `Grid3x3`, family `code`, manifest
   order second (after QR), `Resize::Proportional`. No `format_version` change (a new kind is additive,
   ADR 0091 §12). It reuses the QR schema rows (`Content`, `Size`, `Modules`, `Colours`, `Advanced`) with
   Quiet zone 1; the keys equal the QR keys, so a Symbology switch between QR and Data Matrix carries
   Content, Module size, the module style and Inverted.
3. **Drawing.** A symbol region's L finder (left column and bottom row), clock edge (top row and right
   column) and, from 32 x 32, the alignment patterns between regions are function cells: their dark
   cells are always solid squares and never rounded, shrunk or turned into dots. A Square Merged code
   is one trace of all dark cells; with Rounded the data modules are traced rounded and the function
   cells separately; Separate draws one shape per data module. Inverted cuts every dark cell, function
   cells included, out of one rectangle of symbol plus quiet zone, as QR.
4. **Not in slice 3:** rectangular sizes (Later), GS1 mode, a Shape row, a Finder group.
5. **Error text:** "Too long for a Data Matrix: 1700 bytes, at most 1555." (the QR sentence without the
   "at error correction" cause; `instance_notice::noun` keeps a label with a second capital).

## Frage 407 answered (2026-11-27): CODE-FONT-EMBED A

The customer chose A: a barcode's human-readable-text font travels in the file like a text's. Built as §3.6 A
describes, and it supersedes "stays at its default B" in the slice 2 notes: `curvyo-document-core/src/font_assets.rs`
`Document::used_fonts` also reads the stored `{"font": {family, style, face_hash, face_index}}` value of every
generated object's parameters (a `null` Font, the included sans, names no file). `embed_used_fonts`,
`refresh_document_fonts` (the Document Fonts block) and `prune_assets` follow from `used_fonts` unchanged, so a code's
face is embedded on save, listed, found again on a device without it and kept by the asset wipe. No `format_version`
change (the nested value was already stored by the params codec; an older build drops the asset on its next save,
ADR 0047 §1). A face whose licence forbids embedding stays out, as for a text. Tests: `document_fonts_tests.rs`
(an EAN-13 in Great Vibes embeds it, reopens with the same stored digits and redraws the same digits after an edit on
a device with no fonts; a QR code embeds nothing).
