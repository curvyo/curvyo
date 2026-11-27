# ADRs for QR codes, barcodes and Data Matrix

Status: Proposed 2026-11-26 (architect), written against the Draft `specification.md` and `main` at
`2a2526899` (`curvyo-extension-core/src/extension.rs`, `param.rs`; `curvyo-document-core/src/
generated_codec.rs`, `generated_outline.rs`; `curvyo-ui-core/src/extensions.rs`, `instance.rs`;
`curvyo-generators-core/src/lib.rs`, `manifest.rs`). Follows Frage 319 A, 321 B, 320/322/323 at
their defaults. Decisions marked **(Proposal)** are the architect's and vetoable at the demo.
Customer questions carry a KURZ-ID only; the PO assigns the number.

- [ADR 0091](../../docs/adr/0091-qr-and-barcode-codes-extension.md) (new, Proposed 2026-11-26):
  decides this feature. New crate `curvyo-codes-core` (extension `curvyo.codes`), one kind per
  symbology (`qr` in slice 1), `qrcode` encoder and `rqrr` test decoder, `ParamKind::Text` and
  `Percent`, the refusal rule (no parts: stored parts kept, params written), traced Merged outlines and Separate
  modules in one part `code`, the codeword budget of the clear area, colours through the ordinary
  fill, budgets, SVG goldens, no `format_version` change. §8 (per-part paint) is
  `needs-customer`: CODE-PAINT.
- [ADR 0080](../../docs/adr/0080-generated-objects-extension-trait-and-parametric-instances.md):
  the generated object, its stored record and frame, unknown kinds kept and drawn (§4, §5), the
  instance mechanism, scale and Convert to paths (§6), history labels (§7). Used unchanged;
  extended by two parameter kinds and the refusal host rule.
- [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md) §1, §3, §5, §6: an extension
  is a crate and one registry entry; parameter keys and option ids are file format;
  `extensions.toml` can switch codes off. Unchanged.
- [ADR 0082](../../docs/adr/0082-generator-plate-joints-corner-rule-and-rounded-box.md) §10: part
  keys and per-part operations. The code has one part `code`, so Detach part is inert. Unchanged.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §2: gains `curvyo-codes-core`
  through ADR 0091.

## Spec changes the PO should take (from ADR 0091, all Proposal)

1. **Criterion 8:** the 4 / 9 / 15 / 20 % budget fails its own decoder test at small versions
   (measured largest readable share at version 3: 4.3 / 7.6 / 11.9 / 14.4 %, and L reads only
   2 % at version 1). Replace it with ADR 0091 §5: cleared non-function modules at most
   0.4 x 8 x the correctable codewords of the version and level.
2. **Criterion 15:** a Separate code converts to one compound path (one part); Break apart gives
   one path per module.
3. **Criterion 5:** no Symbology field in slice 1 (the kind is QR); switching in place comes with
   slice 2.
4. **Criterion 9:** Finder colour and Background wait for CODE-PAINT (default B); Fill and
   gradient and the contrast chip stay in slice 1.
5. **Criterion 1:** "a style change never re-encodes" becomes "encode and style are separate
   functions"; the generator runs both on each edit (no cache in a core crate; 7 ms at version 40).
6. **Criterion 16:** no `format_version` change in slice 1; an unknown symbology is an unknown kind.
7. **Criterion 2 (size):** the object's box is the symbol; the 29.0 mm is the Width parameter (it
   includes the quiet zone). The box with the quiet zone (UX notes §12.10) needs a model change.
8. **Criterion 13 (invalid content):** stored with the last valid paths (UX notes §12.3, adopted),
   drawn in the No solution look, as the sketch clone's fallback.

## Conflicts with UX notes revision 1 (for the lead)

- §9: Separate converts to a group of module paths; ADR 0091 §4: one compound path, Break apart.
- §7, §12.9: Finder colour and Background as own parts with own fills; ADR 0091 §8: CODE-PAINT,
  default B (not in slice 1), so the Colours group holds Inverted and the contrast line only.
- §12.10: box with the quiet zone; ADR 0091 §4: not in slice 1 (model change).

## Notes

- 2026-11-26: Spike results (scratch crate, not committed) are recorded in ADR 0091 Context and
  §5. `cargo deny check` passes for every candidate with this repo's `deny.toml`.
- 2026-11-26: Slice 2's human-readable text needs glyph outlines passed into a pure generator;
  ADR 0091 §9 defers that to a revision with slice 2.

## Slice 2 (1D barcodes), 2026-11-27

- [ADR 0092](../../docs/adr/0092-codes-slice-2-1d-barcodes-symbology-switch-and-human-readable-text.md)
  (new, Proposed): own 1D encoders in `linear/` (Code 128, Code 39, EAN-13, EAN-8, UPC-A), goldens
  from the standards, `rxing` 0.9 as the dev-only decoder (spiked: all five read, `cargo deny` passes);
  kinds per symbology in one `Family` (one "Code" segment, a Symbology group, `switch_kind` keeps
  values of equal key and lets an untouched default follow, UX notes §11.4); the human-readable text
  from a `GlyphTable` the host builds with text-core (`ParamKind::Font`, outline faces, default the
  included sans); `Resize::Free` and `Edit::Stretch` for the 1D edge handles (UX notes §11.10, QR
  unchanged); `Warning.param` and `subject`, `GeneratorKind.scanned` and the `instance.rs` split
  resolve the "QR knowledge in the host" debt; `ParamKind::Ratio`, `Percent` as `i32`; no
  `format_version` change. §3.6 is `needs-customer`: CODE-FONT-EMBED (a code's font embedded like a
  text's), default B (not in slice 2).
- ADR 0091 §2, §3, §9: extended by ADR 0092, decision text unchanged.
- ADR 0047 §2, §4: a code's font resolves as a text's (§4); embedding (§2) waits for CODE-FONT-EMBED.
- ADR 0080 §6 and `0141` UX notes §8: proportional-only stays the rule for `Resize::Proportional`
  kinds; 1D kinds are `Free`.

### Spec changes the PO should take (from ADR 0092, all Proposal)

1. **Criterion 11:** quiet zones per symbology (EAN-13 11 and 7, EAN-8 7 and 7, UPC-A 9 and 9, Code 128
   and 39 10; check against the GS1 General Specifications), no Quiet zone row for EAN and UPC; the
   guard bars reach into the text band; the font is an outline face, default Source Sans 3 Regular.
2. **Criterion 3:** "Check digit corrected." without the digit (criterion 13: no content in a notice).
3. **Criterion 14:** a 1D code resizes from edges and corners freely (edge x writes Module width and
   text Size, edge y Bar height); QR stays proportional.
4. **Criterion 16:** slice 2 changes no `format_version`; a slice-1 build opens a barcode as an
   unknown kind with its stored outline.
5. **Criterion 4:** the decoder reads the whole Bar width adjust range at 10 px per module (spiked
   -50 to +50 %), so it is no scannability proof for the adjust; the demo includes a phone check.

### Conflicts with UX notes revision 2 (for the lead)

- §13.2 proposes `quiet_zone` shared by the 1D kinds; ADR 0092 §2 uses `bar_quiet_zone`, because QR's
  shipped `quiet_zone` (in QR modules) would otherwise carry a changed value into Code 128, which
  §11.4 says must not cross. Same behaviour as §11.4, other key.
- §13.9 says "stored by name": the stored `Font` is a text's `FontRef` (name, style and file hash), so
  ADR 0047 §4's match order (bytes, then name) applies.
- §11.2 renames the shipped "QR code" segment to "Code" (accepted behaviour of PR #518): a Proposal
  for the PO and the demo, not an architecture decision.
- Edge resize (§11.10) amends `0141` §8 for the new 1D kinds only; QR, boxes and gears keep it.

### Architect review, 2026-11-27

- ADR 0092 accepted with the deviations and corrections of its "Architect review"; three technical-debt
  entries (glyph funnel fails open, 1D code keys in the host, no native-versus-wasm golden of the codes).
- For the PO: UPC-A's first and last symbol characters have guard-length bars in the GS1 layout; the build
  extends the guard patterns only (criterion 11 Proposal).

## Per-part paint (Finder colour, Background), 2026-11-27

- [ADR 0091](../../docs/adr/0091-qr-and-barcode-codes-extension.md) section "Per-part paint" (new,
  Proposed): Frage 406 CODE-PAINT **answered A by the customer on 2026-11-27**, superseding the default
  B of slices 1 to 3; replaces the §8 sketch. `Part.paint: Option<PartPaint { fill, job }>`; the record
  stores `part_paints` keyed by part key (absent = object style, written only when a part has a paint);
  part order is draw order; `PathSnapshot.paint_runs` and `paint_groups()` split a painted code into
  plain fills inside render-core's and svg-core's path painters (so no caller can miss the split);
  `job_subpaths()` keeps `job: false` parts out of every machine job; Convert to paths converts the
  other parts as today and puts the background as an unassigned sibling path below, so the job never
  changes; `ParamKind::Colour`, `ShownWhen::Off`; Background on all seven kinds, Finder colour QR only
  and ignored while Inverted; `format_version` the next free number at merge (69 is taken by
  `story/machine-families`, expect 70), goldens `generated_part_paint_v<N>.curvyo` and
  `generated_part_paint_damaged_v<N>.curvyo`, codes goldens `qr_background_finder.svg`,
  `ean13_background.svg`, `datamatrix_10_background.svg`.
- [ADR 0082](../../docs/adr/0082-generator-plate-joints-corner-rule-and-rounded-box.md) §10: part keys
  `background` and `finder` join `code`; Lifecycle (a key whose part is off is kept and ignored) and
  Detach part apply to `finder`; Detach refuses a `job: false` part.
- [ADR 0081](../../docs/adr/0081-print-and-cut-registration-marks-and-fit.md) §1: the Background is
  printed and exported, never burnt, like a registration mark.
- [ADR 0080](../../docs/adr/0080-generated-objects-extension-trait-and-parametric-instances.md) §5, §6:
  an unknown kind draws its paints from the record; Convert gives one path per part with its paint.
- ADR 0091 §4 amended: the box includes the quiet zone while Background is on.

### Spec changes the PO should take (Proposal unless a Frage is named)

1. **Criterion 9:** Finder colour (QR, switch plus colour, starts as the fill's colour, hidden and
   ignored while Inverted) and Background (every kind, switch plus colour, default white, one rectangle
   of symbol plus quiet zone, for 1D with the text band) are in; record Frage 406 answered A.
2. **Criterion 2:** with Background on, the box and W readout include the quiet zone (= Width).
3. **Machines:** a Background never runs on any output page (no look, no Cut-out around it); switching
   it on or off leaves the job unchanged; the finders run with the code's operation.
4. **Detach part:** offered for Finder, not for Background.
5. **Criterion 15:** Convert to paths gives the code as today (a path, or a group of code and finder)
   with its colours, plus an unassigned "Background" path directly below it; the job before and after
   is the same.
6. **Criterion 16:** the `format_version` bump; an older build refuses the file; a damaged
   `part_paints` is refused at open; older files open unchanged.
7. **Criterion 19:** SVG export writes one path per part in draw order (goldens above).
8. **Contrast line:** the light side is the Background when on.
9. **Criterion 4:** a code with Background and Finder colour, and Inverted with Background, still decodes.
10. **Criterion 17:** version 40 Dot with both on inside 100 ms.
11. A clone's fill override recolours the modules only; the Symbology switch keeps Background and its
    colour.
